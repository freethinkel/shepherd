//! The public driver: one booted simulator, its two helpers, and what a host calls.
//!
//! Threads:
//! - **supervisor** (one per driver): every lifecycle decision — state changes, helper
//!   restarts with backoff, the 50 ms gesture tick — happens here. Helper callbacks only
//!   post a [`Msg`] to it, so reader threads never block on the host and the inner
//!   clients are never stopped from two callback threads at once.
//! - **frame worker** (`FramePipeline`): delivers `Event::Frame` directly (latest wins).
//! - **caller threads**: input goes through the gesture router under one lock, straight to
//!   the HID client (which queues while sdhid is not ready).
//!
//! Every event goes through one [`CallbackGate`]: once `stop` returns, `on_event` is not
//! running and never will again, and the last event it saw was `State::Stopped`. `stop`
//! from inside `on_event` does not wait for itself.
use crate::ax::{Ax, AxNode};
use crate::devices::{DeviceSelector, list_devices};
use crate::error::Error;
use crate::frames::FramePipeline;
use crate::gesture::{Router, ScrollPhase};
use crate::grid::Grid;
use crate::helper::{CallbackGate, Supervised, helper_dir, lock};
use crate::hid::{HidClient, HidEvent, QUEUE_CAP};
use crate::protocol::{HidCmd, Phase};
use crate::shm::Frame;
use crate::stats::{Stats, StatsTracker};
use crate::tap::FrameTap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Condvar, Mutex, PoisonError, Weak};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Gesture router tick (lifts an idle trackpad pan).
const TICK: Duration = Duration::from_millis(50);
/// How long `start` waits for frames and input (or an early helper failure) before
/// returning anyway, still `Starting`.
const START_WAIT: Duration = Duration::from_secs(15);
const BACKOFF_MIN: Duration = Duration::from_millis(100);
const BACKOFF_MAX: Duration = Duration::from_secs(2);

/// GPUI key names, lowercase, DOM spellings too → HID usage (pip-mini `core/hid.ts` HID_KEYS).
const HID_KEYS: &[(&str, u32)] = &[
    ("enter", 40),
    ("return", 40),
    ("escape", 41),
    ("backspace", 42),
    ("tab", 43),
    ("space", 44),
    ("delete", 76),
    ("home", 74),
    ("end", 77),
    ("pageup", 75),
    ("pagedown", 78),
    ("right", 79),
    ("arrowright", 79),
    ("left", 80),
    ("arrowleft", 80),
    ("down", 81),
    ("arrowdown", 81),
    ("up", 82),
    ("arrowup", 82),
];

fn key_code(name: &str) -> Option<u32> {
    let name = name.to_ascii_lowercase();
    HID_KEYS
        .iter()
        .find(|(k, _)| *k == name)
        .map(|(_, c)| *c)
        .or_else(|| match name.as_bytes() {
            [b'a'..=b'z'] => Some(u32::from(name.as_bytes()[0] - b'a') + 4),
            [b'1'..=b'9'] => Some(u32::from(name.as_bytes()[0] - b'1') + 30),
            [b'0'] => Some(39),
            _ => None,
        })
}

/// How to start a [`Driver`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub device: DeviceSelector,
    /// Frame rate cap.
    pub fps: u32,
    /// Frame downscale factor from device pixels (2 = half width and height).
    pub scale: u32,
    /// Where `sdframe`/`sdhid` live; default `$SIMDRIVER_HELPER_DIR`, else the build's own.
    pub helper_dir: Option<PathBuf>,
    /// Stream the screen of this pixel size (either way round) instead of the largest: a foldable's other panel.
    pub screen_size: Option<(u32, u32)>,
    /// The digitizer target touches go to: 0 for the main screen, else that screen's (a foldable's other panel).
    pub touch_target: u64,
    /// Frames turned a quarter clockwise: a foldable's panel mounted sideways, shown as it's held.
    pub rotate_cw: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            device: DeviceSelector::FirstBootedOrIphone,
            fps: 60,
            scale: 2,
            helper_dir: None,
            screen_size: None,
            touch_target: 0,
            rotate_cw: false,
        }
    }
}

/// Keyboard modifiers held during a pointer press. Alt makes the drag a two-finger pinch
/// around the press point; Alt+Shift moves both fingers together. Ctrl/Meta are unused.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub alt: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub meta: bool,
}

/// Pauses around the movement portion of a drag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragTiming {
    pub source_hold: Duration,
    pub movement: Duration,
    pub destination_hold: Duration,
}

/// A hardware button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Home,
    Lock,
    SideButton,
    Siri,
    VolumeUp,
    VolumeDown,
}

impl Button {
    fn wire(self) -> &'static str {
        match self {
            Button::Home => "HOME",
            Button::Lock => "LOCK",
            Button::SideButton => "SIDE_BUTTON",
            Button::Siri => "SIRI",
            Button::VolumeUp => "VOLUME_UP",
            Button::VolumeDown => "VOLUME_DOWN",
        }
    }
}

/// Driver state, reported through `Event::State` on every change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Helpers coming up: frames may already arrive; input queues.
    Starting,
    /// Frames streaming and input connected.
    Live,
    /// Something else took the touch input; frames keep streaming. The next gesture start
    /// (touch down, button, key, text) takes it back.
    InputDisplaced,
    /// A helper exited (e.g. the simulator rebooted) and is being restarted.
    Restarting,
    Stopped,
}

/// What the driver reports to the host.
#[derive(Debug, Clone)]
pub enum Event {
    Frame(Frame),
    State(State),
    /// Diagnostics (helper notes, warnings, exits). Free text.
    Note(String),
}

/// Raw host input, kept until the screen size is known (sdhid's first `ready`).
enum Raw {
    Pointer {
        phase: Phase,
        x: f64,
        y: f64,
        pinch: bool,
        pan: bool,
    },
    Scroll {
        phase: ScrollPhase,
        x: f64,
        y: f64,
        dx: f64,
        dy: f64,
    },
    TwoFinger {
        phase: Phase,
        f: [f64; 4],
    },
    Cmd(HidCmd),
}

impl Raw {
    fn is_move(&self) -> bool {
        matches!(
            self,
            Raw::Pointer {
                phase: Phase::Move,
                ..
            } | Raw::Scroll {
                phase: ScrollPhase::Changed,
                ..
            } | Raw::TwoFinger {
                phase: Phase::Move,
                ..
            }
        )
    }
}

#[derive(Default)]
struct Input {
    /// Device screen in points; `None` until sdhid's first `ready`.
    screen: Option<(f64, f64)>,
    router: Option<Router>,
    /// Input that arrived before the screen size was known; bounded like the HID queue
    /// (oldest move evicted first; starts and ends always kept).
    early: VecDeque<Raw>,
}

impl Input {
    fn hold(&mut self, raw: Raw) {
        if self.early.len() >= QUEUE_CAP {
            if let Some(i) = self.early.iter().position(Raw::is_move) {
                self.early.remove(i);
            } else if raw.is_move() {
                return;
            }
        }
        self.early.push_back(raw);
    }
}

/// Posted to the supervisor by helper callbacks.
enum Msg {
    Hid(HidEvent),
    /// sdframe process `gen` exited.
    FrameExit {
        generation: u64,
        code: Option<i32>,
        last: Option<String>,
    },
    /// First frame since sdframe was (re)spawned.
    FrameUp,
    Note(String),
    Stop,
}

/// What callbacks need; owns no helper (so callbacks holding it make no cycle).
struct Shared {
    on_event: Box<dyn Fn(Event) + Send + Sync>,
    gate: CallbackGate,
    state: Mutex<State>,
    epoch: Instant,
    stats: Mutex<StatsTracker>,
    awaiting_frame: AtomicBool,
    tx: Sender<Msg>,
}

impl Shared {
    fn emit(&self, ev: Event) {
        self.gate.run(|| (self.on_event)(ev));
    }

    fn note(&self, n: String) {
        self.emit(Event::Note(n));
    }

    fn set_state(&self, s: State) {
        {
            let mut cur = lock(&self.state);
            if *cur == s || *cur == State::Stopped {
                return;
            }
            *cur = s;
        }
        self.emit(Event::State(s));
    }

    /// Stats clock: `now` is taken under the tracker lock, so concurrent frame and input
    /// threads always feed it non-decreasing times.
    fn stats_input(&self, phase: Phase) {
        let mut st = lock(&self.stats);
        st.on_input(phase, self.epoch.elapsed());
    }

    fn on_frame(&self, f: Frame) {
        {
            let mut st = lock(&self.stats);
            st.on_frame(self.epoch.elapsed());
        }
        if self.awaiting_frame.swap(false, Ordering::SeqCst) {
            let _ = self.tx.send(Msg::FrameUp);
        }
        self.emit(Event::Frame(f));
    }
}

struct FrameSide {
    pipeline: Option<FramePipeline>,
    helper: Option<Supervised>,
    generation: u64,
}

#[derive(Default)]
struct Startup {
    outcome: Option<Result<(), Error>>,
    /// `start` has returned (with the outcome or after `START_WAIT`): later events are
    /// ordinary restarts.
    settled: bool,
}

struct Inner {
    udid: String,
    shared: Arc<Shared>,
    hid: HidClient,
    /// Accessibility snapshots; sdax is spawned on the first one.
    ax: Ax,
    input: Mutex<Input>,
    frame: Mutex<FrameSide>,
    frame_bin: PathBuf,
    frame_args: Vec<String>,
    startup: Mutex<Startup>,
    startup_cv: Condvar,
    stopping: AtomicBool,
    stopped: Mutex<bool>,
    stopped_cv: Condvar,
    supervisor: Mutex<Option<JoinHandle<()>>>,
}

/// One live driver per UDID per process: starting another stops this one.
static REGISTRY: Mutex<Vec<(String, Weak<Inner>)>> = Mutex::new(Vec::new());

/// Starts for one UDID run one at a time, so two never spawn sdhid against the same
/// device at once (the later connect would displace the earlier one's input).
static STARTING: Mutex<Vec<(String, Arc<Mutex<()>>)>> = Mutex::new(Vec::new());

fn start_lock(udid: &str) -> Arc<Mutex<()>> {
    let mut all = lock(&STARTING);
    all.retain(|(_, m)| Arc::strong_count(m) > 1); // drop locks nobody holds
    if let Some((_, m)) = all.iter().find(|(u, _)| u == udid) {
        return m.clone();
    }
    let m = Arc::new(Mutex::new(()));
    all.push((udid.to_string(), m.clone()));
    m
}

/// Remove and return the live drivers registered for `udid`.
fn registry_take(udid: &str) -> Vec<Arc<Inner>> {
    let mut reg = lock(&REGISTRY);
    let mut out = Vec::new();
    reg.retain(|(u, w)| {
        if u != udid {
            return w.strong_count() > 0;
        }
        out.extend(w.upgrade());
        false
    });
    out
}

impl Inner {
    /// Spawn a fresh sdframe feeding the (long-lived) pipeline. No-op once stopped.
    fn spawn_frame(&self) -> Result<(), Error> {
        let mut f = lock(&self.frame);
        let Some(p) = f.pipeline.as_ref() else {
            return Ok(());
        };
        let (sink, stderr_sink) = (p.sink(), p.stderr_sink());
        f.generation += 1;
        let generation = f.generation;
        let tx = self.shared.tx.clone();
        self.shared.awaiting_frame.store(true, Ordering::SeqCst);
        let helper = Supervised::spawn_with_stderr(
            self.frame_bin.clone(),
            self.frame_args.clone(),
            sink,
            stderr_sink,
            move |code, last| {
                let _ = tx.send(Msg::FrameExit {
                    generation,
                    code,
                    last,
                });
            },
        )?;
        let old = f.helper.replace(helper);
        drop(f);
        drop(old); // already exited; reaps
        Ok(())
    }

    fn send(&self, cmd: HidCmd) {
        if let HidCmd::Touch { phase, .. } | HidCmd::Touch2 { phase, .. } = &cmd {
            self.shared.stats_input(*phase);
        }
        self.hid.send(cmd);
    }

    /// Route one input through the gesture router (screen known) and send the result.
    fn apply(&self, inp: &mut Input, raw: Raw) {
        let (Some((w, h)), Some(r)) = (inp.screen, inp.router.as_mut()) else {
            return;
        };
        let now = self.shared.epoch.elapsed();
        let cmds = match raw {
            Raw::Pointer {
                phase,
                x,
                y,
                pinch,
                pan,
            } => {
                let (x, y) = (x * w, y * h);
                match phase {
                    Phase::Down => r.down(x, y, pinch, pan, now),
                    Phase::Move => r.move_to(x, y, now),
                    Phase::Up => r.up(x, y, now),
                }
            }
            Raw::Scroll {
                phase,
                x,
                y,
                dx,
                dy,
            } => r.scroll(x * w, y * h, dx * w, dy * h, phase, now),
            Raw::TwoFinger { phase, f } => {
                r.two_finger(phase, [f[0] * w, f[1] * h, f[2] * w, f[3] * h])
            }
            Raw::Cmd(c) => vec![c],
        };
        for c in cmds {
            self.send(c);
        }
    }

    fn input(&self, raw: Raw) {
        if self.stopping.load(Ordering::SeqCst) {
            return;
        }
        let mut inp = lock(&self.input);
        if inp.router.is_some() {
            self.apply(&mut inp, raw);
        } else {
            inp.hold(raw);
        }
    }

    /// sdhid is ready (first time or after a restart): a fresh router, so a touch in
    /// flight under the old sdhid is dropped rather than continued on one that never saw
    /// it go down. Then replay early input.
    fn set_screen(&self, w: f64, h: f64) {
        let mut inp = lock(&self.input);
        inp.screen = Some((w, h));
        inp.router = Some(Router::new((w, h)));
        let early = std::mem::take(&mut inp.early);
        for raw in early {
            self.apply(&mut inp, raw);
        }
    }

    fn tick(&self) {
        let mut inp = lock(&self.input);
        let now = self.shared.epoch.elapsed();
        let cmds = inp.router.as_mut().map(|r| r.tick(now)).unwrap_or_default();
        for c in cmds {
            self.send(c);
        }
    }

    fn startup_ok(&self) {
        let mut s = lock(&self.startup);
        if !s.settled && s.outcome.is_none() {
            s.outcome = Some(Ok(()));
            self.startup_cv.notify_all();
        }
    }

    /// Fail `start` with `e` if it is still waiting. False if `start` already returned.
    fn startup_fail(&self, e: Error) -> bool {
        let mut s = lock(&self.startup);
        if s.settled || s.outcome.is_some() {
            return false;
        }
        s.outcome = Some(Err(e));
        self.startup_cv.notify_all();
        true
    }

    fn wait_startup(&self, within: Duration) -> Result<(), Error> {
        let s = lock(&self.startup);
        let (mut s, _) = self
            .startup_cv
            .wait_timeout_while(s, within, |s| s.outcome.is_none())
            .unwrap_or_else(PoisonError::into_inner);
        s.settled = true;
        s.outcome.take().unwrap_or(Ok(()))
    }

    fn stop(&self) {
        if self.stopping.swap(true, Ordering::SeqCst) {
            // Another thread is stopping. From inside a callback we must not wait: that
            // thread may be waiting for this callback to return.
            if !self.shared.gate.running_here() {
                let done = lock(&self.stopped);
                drop(self.stopped_cv.wait_while(done, |d| !*d));
            }
            return;
        }
        // No event is being delivered on another thread, and none will start.
        self.shared.gate.close();
        self.ax.stop();
        let _ = self.shared.tx.send(Msg::Stop);
        if let Some(h) = lock(&self.supervisor).take()
            && h.thread().id() != thread::current().id()
        {
            let _ = h.join();
        }
        self.hid.stop();
        let (helper, pipeline) = {
            let mut f = lock(&self.frame);
            (f.helper.take(), f.pipeline.take())
        };
        drop(helper); // sdframe stopped and reaped
        drop(pipeline); // joins the frame worker (unless we are on it)
        *lock(&self.shared.state) = State::Stopped;
        (self.shared.on_event)(Event::State(State::Stopped));
        lock(&REGISTRY).retain(|(_, w)| w.strong_count() > 0 && !std::ptr::eq(w.as_ptr(), self));
        *lock(&self.stopped) = true;
        self.stopped_cv.notify_all();
    }
}

/// sdframe exits that mean "this device or Xcode won't work" (not found, no surface).
fn is_fatal(code: Option<i32>) -> bool {
    matches!(code, Some(3 | 4))
}

/// The same for sdhid, plus 1: dtuhidd never came up, which is how an unknown Xcode
/// build shows up on the input side.
fn is_fatal_hid(code: Option<i32>) -> bool {
    matches!(code, Some(1 | 3 | 4))
}

fn exit_text(name: &str, code: Option<i32>, why: &Option<String>) -> String {
    let how = code.map_or("was killed by a signal".to_string(), |c| {
        format!("exited with code {c}")
    });
    match why {
        Some(w) => format!("{name} {how}: {w}"),
        None => format!("{name} {how}"),
    }
}

struct Supervisor {
    inner: Arc<Inner>,
    rx: Receiver<Msg>,
    framed: bool,
    hid_ready: bool,
    displaced: bool,
    restarting: bool,
    frame_retry: Option<Instant>,
    frame_backoff: Duration,
    hid_retry: Option<Instant>,
    hid_backoff: Duration,
    /// sdhid reports startup failures on stdout (`error`), not stderr.
    last_hid_error: Option<String>,
}

impl Supervisor {
    fn run(mut self) {
        let mut next_tick = Instant::now() + TICK;
        while !self.inner.stopping.load(Ordering::SeqCst) {
            let wake = [Some(next_tick), self.frame_retry, self.hid_retry]
                .into_iter()
                .flatten()
                .min();
            let wait = wake.map_or(TICK, |t| t.saturating_duration_since(Instant::now()));
            match self.rx.recv_timeout(wait) {
                Ok(Msg::Stop) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(msg) => self.handle(msg),
                Err(RecvTimeoutError::Timeout) => {}
            }
            if self.inner.stopping.load(Ordering::SeqCst) {
                return;
            }
            let now = Instant::now();
            if now >= next_tick {
                self.inner.tick();
                next_tick = now + TICK;
            }
            if self.frame_retry.is_some_and(|t| now >= t) {
                self.frame_retry = None;
                if let Err(e) = self.inner.spawn_frame() {
                    self.inner
                        .shared
                        .note(format!("sdframe restart failed: {e}"));
                    self.frame_retry = Some(self.backoff(true));
                }
            }
            if self.hid_retry.is_some_and(|t| now >= t) {
                self.hid_retry = None;
                if let Err(e) = self.inner.hid.restart() {
                    self.inner.shared.note(format!("sdhid restart failed: {e}"));
                    self.hid_retry = Some(self.backoff(false));
                }
            }
        }
    }

    /// When to retry, doubling that helper's backoff for the next time.
    fn backoff(&mut self, frame: bool) -> Instant {
        let b = if frame {
            &mut self.frame_backoff
        } else {
            &mut self.hid_backoff
        };
        let at = Instant::now() + *b;
        *b = (*b * 2).min(BACKOFF_MAX);
        at
    }

    fn handle(&mut self, msg: Msg) {
        let shared = self.inner.shared.clone();
        match msg {
            Msg::Stop => {}
            Msg::Note(n) => shared.note(n),
            Msg::FrameUp => {
                self.framed = true;
                self.frame_backoff = BACKOFF_MIN;
                self.update();
            }
            Msg::FrameExit {
                generation,
                code,
                last,
            } => {
                if generation != lock(&self.inner.frame).generation {
                    return;
                }
                let text = exit_text("sdframe", code, &last);
                if is_fatal(code)
                    && self
                        .inner
                        .startup_fail(Error::Spawn(last.unwrap_or_else(|| text.clone())))
                {
                    return;
                }
                shared.note(text);
                self.framed = false;
                self.restarting = true;
                self.frame_retry = Some(self.backoff(true));
                self.update();
            }
            Msg::Hid(ev) => match ev {
                HidEvent::Ready { w, h } => {
                    self.hid_ready = true;
                    self.displaced = false;
                    self.hid_backoff = BACKOFF_MIN;
                    self.last_hid_error = None;
                    self.inner.set_screen(w, h);
                    self.update();
                }
                HidEvent::Displaced => {
                    self.displaced = true;
                    self.update();
                }
                HidEvent::Reclaimed => {
                    self.displaced = false;
                    self.update();
                }
                HidEvent::Warn(w) => shared.note(format!("sdhid: {w}")),
                HidEvent::Error(e) => {
                    shared.note(format!("sdhid error: {e}"));
                    self.last_hid_error = Some(e);
                }
                HidEvent::Note(n) => shared.note(n),
                HidEvent::Exited { code, last_stderr } => {
                    // HidClient checks the generation apart from emitting, so an exit of a
                    // helper we already replaced can still arrive: the live one is younger.
                    if self.inner.hid.pid().is_some_and(|p| !pid_gone(p)) {
                        return;
                    }
                    let why = self.last_hid_error.take().or(last_stderr);
                    let text = exit_text("sdhid", code, &why);
                    if is_fatal_hid(code)
                        && self
                            .inner
                            .startup_fail(Error::Spawn(why.unwrap_or_else(|| text.clone())))
                    {
                        return;
                    }
                    shared.note(text);
                    self.hid_ready = false;
                    self.displaced = false;
                    self.restarting = true;
                    self.hid_retry = Some(self.backoff(false));
                    self.update();
                }
            },
        }
    }

    fn update(&mut self) {
        let s = if self.framed && self.hid_ready {
            self.restarting = false;
            self.inner.startup_ok(); // `start` returns once frames and input are both up
            if self.displaced {
                State::InputDisplaced
            } else {
                State::Live
            }
        } else if self.restarting {
            State::Restarting
        } else {
            State::Starting
        };
        self.inner.shared.set_state(s);
    }
}

/// True once no process with `pid` exists (exited and reaped).
fn pid_gone(pid: u32) -> bool {
    unsafe { libc::kill(pid as libc::pid_t, 0) != 0 && *libc::__error() == libc::ESRCH }
}

/// Drives one booted simulator: streams its screen and injects input. `Send + Sync`.
/// Dropping it stops it.
pub struct Driver {
    inner: Arc<Inner>,
}

impl Driver {
    /// Start on a **booted** device (`Error::NotBooted` otherwise; booting is the host's
    /// call, see [`crate::boot`]). A live driver for the same device in this process is
    /// stopped first. Returns once frames stream and input is connected (`Live`), or
    /// after a bounded wait, still `Starting`. Fails with `Error::Spawn` if, before that,
    /// a helper cannot find the device or its screen (exit 3/4) or sdhid cannot reach the
    /// digitizer (exit 1, e.g. an unsupported Xcode). Events go to `on_event` from
    /// internal threads, possibly concurrently.
    pub fn start(
        opts: Options,
        on_event: impl Fn(Event) + Send + Sync + 'static,
    ) -> Result<Driver, Error> {
        let device = opts.device.resolve(&list_devices()?)?;
        if device.state != "Booted" {
            return Err(Error::NotBooted(device.udid));
        }
        let dir = helper_dir(opts.helper_dir.as_deref());
        if let Err(why) = crate::embedded::dir()
            && dir == crate::baked_helper_dir()
        {
            return Err(Error::Spawn(format!(
                "{why}; set {} to a directory with the helpers",
                crate::helper::HELPER_DIR_ENV
            )));
        }
        let mut frame_args = vec![
            device.udid.clone(),
            "--fps".into(),
            opts.fps.to_string(),
            "--scale".into(),
            opts.scale.to_string(),
        ];
        if opts.rotate_cw {
            frame_args.extend(["--rotate-cw".into(), "1".into()]);
        }
        if let Some((w, h)) = opts.screen_size {
            frame_args.extend(["--screen-size".into(), format!("{w}x{h}")]);
        }
        let hid_args = vec![device.udid.clone(), "--target".into(), opts.touch_target.to_string()];
        Driver::start_with(
            device.udid,
            (dir.join("sdframe"), frame_args),
            (dir.join("sdhid"), hid_args),
            Box::new(on_event),
            START_WAIT,
        )
    }

    /// `start` after the device checks, with explicit helper commands (tests use fakes).
    fn start_with(
        udid: String,
        frame: (PathBuf, Vec<String>),
        hid: (PathBuf, Vec<String>),
        on_event: Box<dyn Fn(Event) + Send + Sync>,
        start_wait: Duration,
    ) -> Result<Driver, Error> {
        for bin in [&frame.0, &hid.0] {
            if !bin.is_file() {
                return Err(Error::HelperMissing(bin.clone()));
            }
        }
        let starting = start_lock(&udid);
        let _one_start = lock(&starting);
        for old in registry_take(&udid) {
            old.stop(); // its sdhid must let go before ours connects
        }
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Shared {
            on_event,
            gate: CallbackGate::new(),
            state: Mutex::new(State::Stopped),
            epoch: Instant::now(),
            stats: Mutex::new(StatsTracker::new()),
            awaiting_frame: AtomicBool::new(false),
            tx: tx.clone(),
        });
        *lock(&shared.state) = State::Starting;
        shared.emit(Event::State(State::Starting));
        let hid_tx = tx;
        // sdax sits next to sdhid; it is spawned on the first snapshot, not here.
        let hid_bin = hid.0.clone();
        let hid = match HidClient::spawn(hid.0, hid.1, move |e| {
            let _ = hid_tx.send(Msg::Hid(e));
        }) {
            Ok(h) => h,
            Err(e) => {
                shared.gate.close();
                *lock(&shared.state) = State::Stopped;
                (shared.on_event)(Event::State(State::Stopped));
                return Err(e);
            }
        };
        let pipeline = {
            let (f, n) = (shared.clone(), shared.clone());
            FramePipeline::new(
                move |fr| f.on_frame(fr),
                move |note| {
                    let _ = n.tx.send(Msg::Note(note));
                },
            )
        };
        let inner = Arc::new(Inner {
            ax: Ax::new(hid_bin.with_file_name("sdax"), vec![udid.clone()]),
            udid,
            shared,
            hid,
            input: Mutex::new(Input::default()),
            frame: Mutex::new(FrameSide {
                pipeline: Some(pipeline),
                helper: None,
                generation: 0,
            }),
            frame_bin: frame.0,
            frame_args: frame.1,
            startup: Mutex::new(Startup::default()),
            startup_cv: Condvar::new(),
            stopping: AtomicBool::new(false),
            stopped: Mutex::new(false),
            stopped_cv: Condvar::new(),
            supervisor: Mutex::new(None),
        });
        let driver = Driver {
            inner: inner.clone(),
        };
        // From here on, dropping `driver` on an error path stops everything.
        driver.inner.spawn_frame()?;
        let sup = Supervisor {
            inner,
            rx,
            framed: false,
            hid_ready: false,
            displaced: false,
            restarting: false,
            frame_retry: None,
            frame_backoff: BACKOFF_MIN,
            hid_retry: None,
            hid_backoff: BACKOFF_MIN,
            last_hid_error: None,
        };
        let handle = thread::Builder::new()
            .name("simdriver-supervisor".into())
            .spawn(move || sup.run())
            .map_err(|e| Error::Spawn(format!("supervisor thread: {e}")))?;
        *lock(&driver.inner.supervisor) = Some(handle);
        driver.inner.wait_startup(start_wait)?;
        lock(&REGISTRY).push((driver.inner.udid.clone(), Arc::downgrade(&driver.inner)));
        Ok(driver)
    }

    pub fn udid(&self) -> &str {
        &self.inner.udid
    }

    /// Pointer at (`x`, `y`), normalized 0..1 inside the box (outside clamps to the screen
    /// edge). `mods` matter on `Down`: Alt = pinch, Alt+Shift = two-finger pan.
    pub fn pointer(&self, phase: Phase, x: f64, y: f64, mods: Modifiers) {
        self.inner.input(Raw::Pointer {
            phase,
            x,
            y,
            pinch: mods.alt,
            pan: mods.alt && mods.shift,
        });
    }

    /// Trackpad scroll as a one-finger pan at (`x`, `y`) (0..1); `dx`, `dy` are fractions
    /// of the box.
    pub fn scroll(&self, phase: ScrollPhase, x: f64, y: f64, dx: f64, dy: f64) {
        self.inner.input(Raw::Scroll {
            phase,
            x,
            y,
            dx,
            dy,
        });
    }

    /// Two fingers at (`x1`, `y1`) and (`x2`, `y2`), normalized 0..1 inside the box,
    /// placed exactly as given: rotation, an off-centre pinch, scripted multi-touch.
    /// `Down` first lifts any other touch in progress; send `Move`s, then `Up`.
    pub fn touch2(&self, phase: Phase, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.inner.input(Raw::TwoFinger {
            phase,
            f: [x1, y1, x2, y2],
        });
    }

    /// The device screen in points (e.g. 402×874 for an iPhone 18 Pro), known once
    /// input is connected; `None` before that. Hosts use it to show the device at its
    /// point-accurate size.
    pub fn screen_points(&self) -> Option<(f64, f64)> {
        lock(&self.inner.input).screen
    }

    /// A copy of the newest frame (what the screen shows now), or `None` if none comes
    /// within `wait`. For one still image (an agent screenshot) without copying every
    /// frame; the stream itself keeps going to `on_event`.
    pub fn capture(&self, wait: Duration) -> Option<Frame> {
        let capturer = lock(&self.inner.frame).pipeline.as_ref()?.capturer();
        capturer.capture(wait)
    }

    /// A sampled tap on the frame stream, at most `fps` frames a second (clamped to
    /// 1..=60), for recording what the screen does; `None` without a frame pipeline
    /// (the driver has stopped). It never slows the stream to `on_event`: frames the tap's
    /// consumer is too slow for are skipped, not queued. Reports [`crate::TapRecv::Closed`]
    /// once the driver stops.
    pub fn frame_tap(&self, fps: u32) -> Option<FrameTap> {
        Some(lock(&self.inner.frame).pipeline.as_ref()?.tap(fps))
    }

    /// A tap at (`x`, `y`), normalized 0..1 inside the box: a pointer down and up.
    pub fn tap(&self, x: f64, y: f64) {
        self.pointer(Phase::Down, x, y, Modifiers::default());
        self.pointer(Phase::Up, x, y, Modifiers::default());
    }

    /// Tap twice at (`x`, `y`) with a short pause so iOS recognizes a double tap.
    pub fn double_tap(&self, x: f64, y: f64) {
        self.tap(x, y);
        thread::sleep(Duration::from_millis(100));
        self.tap(x, y);
    }

    /// Press at (`x`, `y`), normalized 0..1 inside the box, hold for `hold`, then lift —
    /// iOS's long press (context menus, rearrange mode). The hold sleeps on the calling
    /// thread; the driver keeps serving input from other threads meanwhile.
    pub fn long_press(&self, x: f64, y: f64, hold: Duration) {
        self.pointer(Phase::Down, x, y, Modifiers::default());
        thread::sleep(hold);
        self.pointer(Phase::Up, x, y, Modifiers::default());
    }

    /// Drag from (`x1`, `y1`) to (`x2`, `y2`), each 0..1 in the box: down, hold `hold`
    /// first (so iOS treats it as a drag, not a scroll), move over `duration`, then
    /// lift. Sleeps on the calling thread.
    pub fn drag(&self, x1: f64, y1: f64, x2: f64, y2: f64, hold: Duration, duration: Duration) {
        self.drag_with_holds(
            (x1, y1),
            (x2, y2),
            DragTiming {
                source_hold: hold,
                movement: duration,
                destination_hold: Duration::ZERO,
            },
        );
    }

    /// Drag with optional pauses at the source and destination.
    pub fn drag_with_holds(&self, from: (f64, f64), to: (f64, f64), timing: DragTiming) {
        let (x1, y1) = from;
        let (x2, y2) = to;
        self.pointer(Phase::Down, x1, y1, Modifiers::default());
        thread::sleep(timing.source_hold);
        let steps = (timing.movement.as_millis() / 16).max(2) as u32;
        for i in 1..=steps {
            thread::sleep(timing.movement / steps);
            let t = f64::from(i) / f64::from(steps);
            self.pointer(
                Phase::Move,
                x1 + (x2 - x1) * t,
                y1 + (y2 - y1) * t,
                Modifiers::default(),
            );
        }
        thread::sleep(timing.destination_hold);
        self.pointer(Phase::Up, x2, y2, Modifiers::default());
    }

    /// The frontmost app's accessibility tree (system UI such as the home screen
    /// included), frames in screen points. Blocks: the first call starts the `sdax`
    /// helper and takes 1–2 s while the guest warms up; later calls take tens of ms.
    /// Fails with `Error::Unsupported` on an Xcode without the private API it uses.
    /// Web content inside Safari shows up only as an opaque element.
    pub fn accessibility(&self) -> Result<AxNode, Error> {
        self.inner.ax.snapshot()
    }

    /// A [`Grid`] over this device's screen with `cols` columns, once input is
    /// connected (the screen size comes from sdhid).
    pub fn grid(&self, cols: u32) -> Option<Grid> {
        self.screen_points().map(|s| Grid::new(s, cols))
    }

    /// Press and release a named key (enter/return, escape, backspace, tab, space, delete,
    /// arrows; DOM spellings like "ArrowLeft" too; case-insensitive). False if unknown.
    pub fn key(&self, name: &str) -> bool {
        if name.contains('+') {
            let parts: Vec<&str> = name.split('+').collect();
            return self.key_combo(&parts);
        }
        let Some(code) = key_code(name) else {
            return false;
        };
        self.inner.input(Raw::Cmd(HidCmd::Key { code }));
        true
    }

    /// Press the final key while holding cmd, shift, alt, or ctrl modifiers.
    /// Example: `key_combo(&["cmd", "a"])` sends Command+A.
    pub fn key_combo(&self, keys: &[&str]) -> bool {
        if keys.len() < 2 {
            return false;
        }
        let mut mods = Vec::with_capacity(keys.len() - 1);
        for modifier in &keys[..keys.len() - 1] {
            let code = match modifier.to_ascii_lowercase().as_str() {
                "cmd" | "command" | "meta" => 227,
                "shift" => 225,
                "alt" | "option" => 226,
                "ctrl" | "control" => 224,
                _ => return false,
            };
            if !mods.contains(&code) {
                mods.push(code);
            }
        }
        let Some(code) = key_code(keys[keys.len() - 1]) else {
            return false;
        };
        self.inner.input(Raw::Cmd(HidCmd::KeyCombo { code, mods }));
        true
    }

    /// Type text. Non-ASCII uses the simulator pasteboard, so the app sees a paste action.
    pub fn text(&self, s: &str) {
        if s.is_ascii() {
            if !s.is_empty() {
                self.inner.input(Raw::Cmd(HidCmd::Text(s.to_string())));
            }
        } else {
            match crate::devices::clipboard_write(&self.inner.udid, s) {
                Ok(()) => {
                    self.key_combo(&["cmd", "v"]);
                }
                Err(e) => self
                    .inner
                    .shared
                    .note(format!("non-ASCII paste failed: {e}")),
            }
        }
    }

    /// Read text from this simulator's pasteboard.
    pub fn clipboard_read(&self) -> Result<String, Error> {
        crate::devices::clipboard_read(&self.inner.udid)
    }

    /// Write text to this simulator's pasteboard. An empty string clears it.
    pub fn clipboard_write(&self, text: &str) -> Result<(), Error> {
        crate::devices::clipboard_write(&self.inner.udid, text)
    }

    /// Open a URL on the device (`simctl openurl`): Safari for web links, or the app
    /// that handles a custom scheme. iOS may ask "Open in …?" first.
    pub fn open_url(&self, url: &str) -> Result<(), Error> {
        crate::devices::open_url(&self.inner.udid, url)
    }

    /// Launch an installed app (`simctl launch`); brings it to the front.
    pub fn launch(&self, bundle_id: &str) -> Result<(), Error> {
        crate::devices::launch(&self.inner.udid, bundle_id)
    }

    /// Terminate a running app (`simctl terminate`).
    pub fn terminate(&self, bundle_id: &str) -> Result<(), Error> {
        crate::devices::terminate(&self.inner.udid, bundle_id)
    }

    /// Simulate the device being at `lat`, `lon` (`simctl location set`).
    pub fn set_location(&self, lat: f64, lon: f64) -> Result<(), Error> {
        crate::devices::set_location(&self.inner.udid, lat, lon)
    }

    /// Switch the system between light and dark mode.
    pub fn set_appearance(&self, appearance: crate::devices::Appearance) -> Result<(), Error> {
        crate::devices::set_appearance(&self.inner.udid, appearance)
    }

    /// Grant `bundle_id` a privacy `service` (`location`, `photos`, `contacts`, …) via
    /// `simctl privacy`, so its prompt never shows. A running app may cache the old
    /// answer until it relaunches.
    pub fn grant_permission(&self, service: &str, bundle_id: &str) -> Result<(), Error> {
        crate::devices::grant_permission(&self.inner.udid, service, bundle_id)
    }

    /// Deny `bundle_id` a privacy `service`.
    pub fn revoke_permission(&self, service: &str, bundle_id: &str) -> Result<(), Error> {
        crate::devices::revoke_permission(&self.inner.udid, service, bundle_id)
    }

    /// Forget `bundle_id`'s answer for a privacy `service`, so the app asks again.
    pub fn reset_permission(&self, service: &str, bundle_id: &str) -> Result<(), Error> {
        crate::devices::reset_permission(&self.inner.udid, service, bundle_id)
    }

    /// Dynamic Type size: `extra-small` … `accessibility-extra-extra-extra-large`, or
    /// `increment` / `decrement`.
    pub fn set_content_size(&self, size: &str) -> Result<(), Error> {
        crate::devices::set_content_size(&self.inner.udid, size)
    }

    /// A clean status bar for screenshots: 9:41, full battery, charged.
    pub fn status_bar_override(&self) -> Result<(), Error> {
        crate::devices::status_bar_override(&self.inner.udid)
    }

    /// Undo [`Driver::status_bar_override`].
    pub fn status_bar_clear(&self) -> Result<(), Error> {
        crate::devices::status_bar_clear(&self.inner.udid)
    }

    /// Deliver the APNs payload in the JSON file at `payload_path` to `bundle_id`.
    pub fn push_notification(&self, bundle_id: &str, payload_path: &str) -> Result<(), Error> {
        crate::devices::push_notification(&self.inner.udid, bundle_id, payload_path)
    }

    /// Erase the simulator's keychain.
    pub fn reset_keychain(&self) -> Result<(), Error> {
        crate::devices::reset_keychain(&self.inner.udid)
    }

    /// List installed apps on this simulator, including system apps.
    pub fn apps(&self) -> Result<Vec<crate::devices::App>, Error> {
        crate::devices::list_apps(&self.inner.udid)
    }

    /// Explicitly erase an app's saved data and terminate it first.
    pub fn clear_app_data(&self, bundle_id: &str) -> Result<(), Error> {
        crate::devices::clear_app_data(&self.inner.udid, bundle_id)
    }

    /// Press `b` `presses` times (2 = double press), holding each `hold_secs` (0 = a tap).
    pub fn button(&self, b: Button, presses: u32, hold_secs: f32) {
        self.inner.input(Raw::Cmd(HidCmd::Button {
            name: b.wire(),
            presses,
            hold_secs,
        }));
    }

    /// The current state (the last one delivered as an event).
    pub fn state(&self) -> State {
        *lock(&self.inner.shared.state)
    }

    /// Frame-rate and input-latency counters since start.
    pub fn stats(&self) -> Stats {
        let st = lock(&self.inner.shared.stats);
        st.snapshot(self.inner.shared.epoch.elapsed())
    }

    /// Stop both helpers. Returns once `on_event` is not running and never will run again
    /// (from inside `on_event` it does not wait for itself); the last event delivered is
    /// `State::Stopped`. Idempotent; `Drop` calls it.
    pub fn stop(&self) {
        self.inner.stop();
    }

    /// Test seam: make sdhid act as if another client took the touch input.
    #[doc(hidden)]
    pub fn __invalidate_input_for_test(&self) {
        self.inner.hid.send_raw_line(r#"{"t":"_invalidate"}"#);
    }

    /// Test seam: pids of the running helpers (sdframe, sdhid).
    #[doc(hidden)]
    pub fn __helper_pids(&self) -> Vec<u32> {
        let frame = lock(&self.inner.frame).helper.as_ref().map(Supervised::pid);
        frame.into_iter().chain(self.inner.hid.pid()).collect()
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        self.inner.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helper::fake_helper;
    use crate::shm::{FrameHeader, ShmWriter};

    const W: Duration = Duration::from_secs(5);

    #[test]
    fn key_names() {
        assert_eq!(key_code("enter"), Some(40));
        assert_eq!(key_code("Return"), Some(40));
        assert_eq!(key_code("ArrowLeft"), Some(80));
        assert_eq!(key_code("UP"), Some(82));
        assert_eq!(key_code("delete"), Some(76));
        assert_eq!(key_code("Home"), Some(74));
        assert_eq!(key_code("pageup"), Some(75));
        assert_eq!(key_code("a"), Some(4));
        assert_eq!(key_code("f1"), None);
        assert_eq!(key_code(""), None);
    }

    #[test]
    fn options_default() {
        let o = Options::default();
        assert_eq!(
            (o.device, o.fps, o.scale, o.helper_dir),
            (DeviceSelector::FirstBootedOrIphone, 60, 2, None)
        );
    }

    /// Everything a driver reported, in order.
    #[derive(Clone, Default)]
    struct Log(Arc<Mutex<Vec<Event>>>);

    impl Log {
        fn push(&self, e: Event) {
            self.0.lock().unwrap().push(e);
        }
        fn states(&self) -> Vec<State> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .filter_map(|e| {
                    if let Event::State(s) = e {
                        Some(*s)
                    } else {
                        None
                    }
                })
                .collect()
        }
        fn frames(&self) -> usize {
            self.0
                .lock()
                .unwrap()
                .iter()
                .filter(|e| matches!(e, Event::Frame(_)))
                .count()
        }
        fn len(&self) -> usize {
            self.0.lock().unwrap().len()
        }
        fn last(&self) -> Option<String> {
            self.0.lock().unwrap().last().map(|e| format!("{e:?}"))
        }
        /// Wait until `n` states equal to `want` have been reported.
        fn wait_state(&self, want: State, n: usize) {
            let t0 = Instant::now();
            while self.states().iter().filter(|s| **s == want).count() < n {
                assert!(
                    t0.elapsed() < W,
                    "waiting for {want:?} #{n}; states {:?}",
                    self.states()
                );
                thread::sleep(Duration::from_millis(5));
            }
        }
    }

    /// A fake sdframe (over a filled segment) and a fake sdhid (ready once `go` exists).
    struct Rig {
        dir: PathBuf,
        _shm: ShmWriter,
        frame: (PathBuf, Vec<String>),
        hid: (PathBuf, Vec<String>),
    }

    impl Drop for Rig {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    impl Rig {
        fn new(tag: &str, hid_ready: bool) -> Rig {
            let dir =
                std::env::temp_dir().join(format!("simdriver-drv-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            if hid_ready {
                std::fs::write(dir.join("go"), b"").unwrap();
            }
            let name = format!("/sdd-{}-{tag}", std::process::id());
            let shm = ShmWriter::create(&name, 3, 16).unwrap();
            let h = FrameHeader {
                width: 2,
                height: 2,
                source_width: 4,
                source_height: 4,
            };
            for (slot, seq) in [(0, 2), (1, 4), (2, 6)] {
                shm.write(slot, seq, &h, &[seq as u8; 16]);
            }
            let frame = (
                fake_helper(),
                vec!["frames".into(), name, "3".into(), "16".into()],
            );
            let hid = (
                fake_helper(),
                vec!["hid".into(), dir.to_string_lossy().into_owned()],
            );
            Rig {
                dir,
                _shm: shm,
                frame,
                hid,
            }
        }

        fn start(&self, udid: &str, log: &Log) -> Result<Driver, Error> {
            self.start_within(udid, log, W)
        }

        fn start_within(&self, udid: &str, log: &Log, wait: Duration) -> Result<Driver, Error> {
            let l = log.clone();
            Driver::start_with(
                udid.into(),
                self.frame.clone(),
                self.hid.clone(),
                Box::new(move |e| l.push(e)),
                wait,
            )
        }

        fn hid_log(&self) -> Vec<String> {
            std::fs::read_to_string(self.dir.join("log"))
                .unwrap_or_default()
                .lines()
                .map(String::from)
                .collect()
        }

        fn wait_hid_log(&self, n: usize) -> Vec<String> {
            let t0 = Instant::now();
            loop {
                let l = self.hid_log();
                if l.len() >= n {
                    return l;
                }
                assert!(
                    t0.elapsed() < W,
                    "hid log has {} of {n} lines: {l:?}",
                    l.len()
                );
                thread::sleep(Duration::from_millis(5));
            }
        }
    }

    fn gone(pids: &[u32]) -> bool {
        pids.iter().all(|p| pid_gone(*p))
    }

    #[test]
    fn starts_live_streams_and_stops_last() {
        let rig = Rig::new("live", true);
        let log = Log::default();
        let d = rig.start("fake-live", &log).unwrap();
        log.wait_state(State::Live, 1);
        assert_eq!(log.states()[0], State::Starting);
        assert_eq!(d.state(), State::Live);
        let t0 = Instant::now();
        while log.frames() < 10 {
            assert!(t0.elapsed() < W, "frames stopped at {}", log.frames());
            thread::sleep(Duration::from_millis(5));
        }
        assert!(d.stats().frames >= 10);
        let pids = d.__helper_pids();
        assert_eq!(pids.len(), 2);
        d.stop();
        assert!(gone(&pids), "helpers survived stop");
        assert_eq!(log.last().as_deref(), Some("State(Stopped)"));
        let n = log.len();
        thread::sleep(Duration::from_millis(200));
        assert_eq!(log.len(), n, "events after stop");
        d.stop(); // idempotent
        assert_eq!(log.len(), n);
        assert_eq!(d.state(), State::Stopped);
        d.pointer(Phase::Down, 0.5, 0.5, Modifiers::default()); // ignored, no panic
    }

    #[test]
    fn input_before_ready_is_replayed_in_device_points() {
        let rig = Rig::new("early", false);
        let log = Log::default();
        // sdhid never gets ready here, so `start` returns after its bounded wait.
        let d = rig
            .start_within("fake-early", &log, Duration::from_millis(300))
            .unwrap();
        assert_eq!(d.state(), State::Starting, "sdhid is not ready yet");
        d.pointer(Phase::Down, 0.5, 0.25, Modifiers::default());
        d.pointer(Phase::Move, 0.6, 0.25, Modifiers::default());
        d.pointer(Phase::Up, 2.0, -1.0, Modifiers::default()); // outside the box: clamps
        assert!(d.key("Enter"));
        assert!(d.key("cmd+a"));
        assert!(d.key_combo(&["shift", "a"]));
        assert!(!d.key("nope"));
        d.button(Button::Home, 2, 0.0);
        d.text("hi");
        thread::sleep(Duration::from_millis(100));
        assert!(
            rig.hid_log().is_empty(),
            "nothing reaches sdhid before ready"
        );
        std::fs::write(rig.dir.join("go"), b"").unwrap();
        log.wait_state(State::Live, 1);
        let got = rig.wait_hid_log(8);
        let touch = |p, x: f64, y: f64| {
            HidCmd::Touch {
                phase: p,
                x,
                y,
                edge: crate::protocol::Edge::None,
            }
            .to_line()
        };
        assert_eq!(
            got,
            vec![
                touch(Phase::Down, 196.5, 213.0),
                touch(Phase::Move, 393.0 * 0.6, 213.0),
                touch(Phase::Up, 393.0, 0.0),
                HidCmd::Key { code: 40 }.to_line(),
                HidCmd::KeyCombo {
                    code: 4,
                    mods: vec![227]
                }
                .to_line(),
                HidCmd::KeyCombo {
                    code: 4,
                    mods: vec![225]
                }
                .to_line(),
                HidCmd::Button {
                    name: "HOME",
                    presses: 2,
                    hold_secs: 0.0
                }
                .to_line(),
                HidCmd::Text("hi".into()).to_line(),
            ]
        );
        // Once ready, input goes straight through.
        d.pointer(
            Phase::Down,
            0.5,
            0.5,
            Modifiers {
                alt: true,
                ..Modifiers::default()
            },
        );
        let got = rig.wait_hid_log(9);
        assert_eq!(
            got[8],
            HidCmd::Touch2 {
                phase: Phase::Down,
                x1: 146.5,
                y1: 426.0,
                x2: 246.5,
                y2: 426.0
            }
            .to_line()
        );
    }

    #[test]
    fn helper_exits_restart_with_restarting_then_live() {
        let rig = Rig::new("restart", true);
        let log = Log::default();
        let d = rig.start("fake-restart", &log).unwrap();
        log.wait_state(State::Live, 1);

        let old = d.__helper_pids();
        lock(&d.inner.frame)
            .helper
            .as_ref()
            .unwrap()
            .send_line(r#"{"t":"_exit","code":7}"#);
        log.wait_state(State::Restarting, 1);
        log.wait_state(State::Live, 2);
        let now = d.__helper_pids();
        assert_ne!(now[0], old[0], "sdframe was not respawned");
        assert_eq!(now[1], old[1], "sdhid untouched");

        d.inner.hid.send_raw_line(r#"{"t":"_exit","code":1}"#);
        log.wait_state(State::Restarting, 2);
        log.wait_state(State::Live, 3);
        assert_ne!(d.__helper_pids()[1], old[1], "sdhid was not respawned");

        // Displacement and reclaim map to states; frames keep coming meanwhile.
        d.__invalidate_input_for_test();
        log.wait_state(State::InputDisplaced, 1);
        d.pointer(Phase::Down, 0.5, 0.5, Modifiers::default());
        log.wait_state(State::Live, 4);
        assert_eq!(
            log.states(),
            vec![
                State::Starting,
                State::Live,
                State::Restarting,
                State::Live,
                State::Restarting,
                State::Live,
                State::InputDisplaced,
                State::Live
            ]
        );
        let notes: Vec<String> = log
            .0
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| {
                if let Event::Note(n) = e {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            notes
                .iter()
                .any(|n| n == "sdframe exited with code 7: fake_helper: exiting 7"),
            "{notes:?}"
        );
    }

    #[test]
    fn exit_3_or_4_before_first_frame_fails_start() {
        let rig = Rig::new("fail", true);
        let log = Log::default();
        let frame = (
            fake_helper(),
            vec!["fail".into(), "3".into(), "device X not found".into()],
        );
        let l = log.clone();
        let r = Driver::start_with(
            "fake-fail".into(),
            frame,
            rig.hid.clone(),
            Box::new(move |e| l.push(e)),
            W,
        );
        assert!(
            matches!(&r, Err(Error::Spawn(m)) if m == "device X not found"),
            "{:?}",
            r.err()
        );
        assert_eq!(log.states(), vec![State::Starting, State::Stopped]);

        // sdhid failing before ready (sdframe silent: the `echo` fake never frames).
        let rig = Rig::new("hidfail", false);
        let log = Log::default();
        let hid = (
            fake_helper(),
            vec!["fail".into(), "4".into(), "no main screen".into()],
        );
        let l = log.clone();
        let r = Driver::start_with(
            "fake-hidfail".into(),
            (fake_helper(), vec!["echo".into()]),
            hid,
            Box::new(move |e| l.push(e)),
            W,
        );
        assert!(
            matches!(&r, Err(Error::Spawn(m)) if m == "no main screen"),
            "{:?}",
            r.err()
        );
        assert_eq!(log.states(), vec![State::Starting, State::Stopped]);
        drop(rig);

        // Frames already streaming do not settle `start`: sdhid's exit 1 (dtuhidd never
        // came up — an unsupported Xcode) still fails it.
        let rig = Rig::new("hidfail1", false);
        let log = Log::default();
        // sdhid fails 300 ms in, well after the fake sdframe's first frame.
        let hid = (
            fake_helper(),
            vec![
                "fail".into(),
                "1".into(),
                "dtuhidd did not come up".into(),
                "300".into(),
            ],
        );
        let l = log.clone();
        let r = Driver::start_with(
            "fake-hidfail1".into(),
            rig.frame.clone(),
            hid,
            Box::new(move |e| l.push(e)),
            W,
        );
        assert!(
            matches!(&r, Err(Error::Spawn(m)) if m == "dtuhidd did not come up"),
            "{:?}",
            r.err()
        );
        assert_eq!(log.states(), vec![State::Starting, State::Stopped]);
        assert!(log.frames() > 0, "frames were streaming when sdhid failed");
        drop(rig);

        // Any other early exit is a restart, not a failure.
        let rig = Rig::new("fail1", true);
        let log = Log::default();
        let frame = (
            fake_helper(),
            vec!["fail".into(), "1".into(), "transient".into()],
        );
        let l = log.clone();
        let short = Duration::from_millis(300);
        let r = Driver::start_with(
            "fake-fail1".into(),
            frame,
            rig.hid.clone(),
            Box::new(move |e| l.push(e)),
            short,
        );
        let d = r.expect("exit 1 is not fatal");
        log.wait_state(State::Restarting, 1);
        d.stop();
    }

    #[test]
    fn touch2_and_screen_points() {
        let rig = Rig::new("two", true);
        let log = Log::default();
        let d = rig.start("fake-two", &log).unwrap();
        log.wait_state(State::Live, 1);
        assert_eq!(d.screen_points(), Some((393.0, 852.0)));
        d.touch2(Phase::Down, 0.25, 0.5, 0.75, 0.5);
        d.touch2(Phase::Move, 0.5, 0.25, 0.5, 0.75); // a quarter turn
        d.touch2(Phase::Up, 0.5, 0.25, 0.5, 0.75);
        let t2 = |phase, x1, y1, x2, y2| {
            HidCmd::Touch2 {
                phase,
                x1,
                y1,
                x2,
                y2,
            }
            .to_line()
        };
        assert_eq!(
            rig.wait_hid_log(3),
            vec![
                t2(Phase::Down, 98.25, 426.0, 294.75, 426.0),
                t2(Phase::Move, 196.5, 213.0, 196.5, 639.0),
                t2(Phase::Up, 196.5, 213.0, 196.5, 639.0),
            ]
        );
    }

    #[test]
    fn long_press_and_drag_emit_ordered_pointer_phases() {
        let rig = Rig::new("long-press-drag", true);
        let log = Log::default();
        let d = rig.start("fake-long-press-drag", &log).unwrap();
        log.wait_state(State::Live, 1);
        d.long_press(0.25, 0.5, Duration::ZERO);
        d.drag(0.25, 0.5, 0.75, 0.5, Duration::ZERO, Duration::ZERO);
        let touch = |phase, x, y| {
            HidCmd::Touch {
                phase,
                x,
                y,
                edge: crate::protocol::Edge::None,
            }
            .to_line()
        };
        let got = rig.wait_hid_log(6);
        assert_eq!(got[0], touch(Phase::Down, 98.25, 426.0));
        assert_eq!(got[1], touch(Phase::Up, 98.25, 426.0));
        assert_eq!(got[2], touch(Phase::Down, 98.25, 426.0));
        assert_eq!(got[3], touch(Phase::Move, 196.5, 426.0));
        assert_eq!(got[4], touch(Phase::Move, 294.75, 426.0));
        assert_eq!(got[5], touch(Phase::Up, 294.75, 426.0));
    }

    #[test]
    fn double_tap_emits_two_taps_at_the_same_point() {
        let rig = Rig::new("double-tap", true);
        let log = Log::default();
        let d = rig.start("fake-double-tap", &log).unwrap();
        log.wait_state(State::Live, 1);
        d.double_tap(0.25, 0.5);
        let touch = |phase| {
            HidCmd::Touch {
                phase,
                x: 98.25,
                y: 426.0,
                edge: crate::protocol::Edge::None,
            }
            .to_line()
        };
        assert_eq!(
            rig.wait_hid_log(4),
            vec![
                touch(Phase::Down),
                touch(Phase::Up),
                touch(Phase::Down),
                touch(Phase::Up)
            ]
        );
    }

    #[test]
    fn concurrent_starts_for_one_device_leave_one_live() {
        let rig = Rig::new("concurrent", true);
        let logs: Vec<Log> = (0..3).map(|_| Log::default()).collect();
        let drivers: Vec<Driver> = thread::scope(|s| {
            let hs: Vec<_> = logs
                .iter()
                .map(|l| {
                    let (frame, hid, l) = (rig.frame.clone(), rig.hid.clone(), l.clone());
                    s.spawn(move || {
                        Driver::start_with(
                            "fake-concurrent".into(),
                            frame,
                            hid,
                            Box::new(move |e| l.push(e)),
                            W,
                        )
                        .unwrap()
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let live: Vec<_> = drivers
            .iter()
            .filter(|d| d.state() == State::Live)
            .collect();
        assert_eq!(
            live.len(),
            1,
            "{:?}",
            drivers.iter().map(|d| d.state()).collect::<Vec<_>>()
        );
        for d in drivers.iter().filter(|d| d.state() != State::Live) {
            assert_eq!(d.state(), State::Stopped);
        }
        // Each start ran alone: every replaced driver saw Live before it was stopped.
        for l in &logs {
            assert!(l.states().contains(&State::Live), "{:?}", l.states());
        }
    }

    #[test]
    fn sdhid_restart_drops_the_touch_in_flight() {
        let rig = Rig::new("inflight", true);
        let log = Log::default();
        let d = rig.start("fake-inflight", &log).unwrap();
        log.wait_state(State::Live, 1);
        d.pointer(Phase::Down, 0.5, 0.5, Modifiers::default());
        rig.wait_hid_log(1);
        d.inner.hid.send_raw_line(r#"{"t":"_exit","code":1}"#);
        log.wait_state(State::Restarting, 1);
        log.wait_state(State::Live, 2);
        // The new sdhid never saw that finger go down: the rest of the drag is dropped.
        d.pointer(Phase::Move, 0.6, 0.5, Modifiers::default());
        d.pointer(Phase::Up, 0.6, 0.5, Modifiers::default());
        d.pointer(Phase::Down, 0.25, 0.25, Modifiers::default());
        d.pointer(Phase::Up, 0.25, 0.25, Modifiers::default());
        let touch = |p, x: f64, y: f64| {
            HidCmd::Touch {
                phase: p,
                x,
                y,
                edge: crate::protocol::Edge::None,
            }
            .to_line()
        };
        let got = rig.wait_hid_log(3);
        assert_eq!(
            got,
            vec![
                touch(Phase::Down, 196.5, 426.0),
                touch(Phase::Down, 98.25, 213.0),
                touch(Phase::Up, 98.25, 213.0)
            ]
        );
    }

    #[test]
    fn accessibility_tap_and_grid() {
        let rig = Rig::new("ax", true);
        let log = Log::default();
        let d = rig.start("fake-ax", &log).unwrap();
        log.wait_state(State::Live, 1);
        // The real sdax would sit next to the fake sdhid; point it at the fake one.
        d.inner.ax.set_helper(fake_helper(), vec!["ax".into()]);
        let root = d.accessibility().unwrap();
        let button = root
            .descendants()
            .into_iter()
            .find(|n| n.role == "AXButton")
            .unwrap();
        let (sw, sh) = d.screen_points().unwrap();
        let (cx, cy) = button.center();
        d.tap(cx / sw, cy / sh);
        let touch = |p, x: f64, y: f64| {
            HidCmd::Touch {
                phase: p,
                x,
                y,
                edge: crate::protocol::Edge::None,
            }
            .to_line()
        };
        assert_eq!(
            rig.wait_hid_log(2),
            vec![
                touch(Phase::Down, 196.5, 725.0),
                touch(Phase::Up, 196.5, 725.0)
            ]
        );
        let g = d.grid(Grid::DEFAULT_COLS).unwrap();
        assert_eq!(g, Grid { rows: 13, cols: 6 });
        let f = d.capture(W).expect("capture");
        assert_eq!((f.width, f.height, f.data.len()), (2, 2, 16));
        d.stop();
        assert!(matches!(d.accessibility(), Err(Error::Accessibility(_))));
    }

    #[test]
    fn second_start_replaces_first() {
        let rig = Rig::new("second", true);
        let (la, lb) = (Log::default(), Log::default());
        let a = rig.start("fake-second", &la).unwrap();
        let pids = a.__helper_pids();
        let b = rig.start("fake-second", &lb).unwrap();
        assert_eq!(a.state(), State::Stopped);
        assert_eq!(la.last().as_deref(), Some("State(Stopped)"));
        assert!(gone(&pids));
        lb.wait_state(State::Live, 1);
        drop(a);
        assert_eq!(b.state(), State::Live);
        let other = rig.start("fake-other", &Log::default()).unwrap();
        assert_eq!(b.state(), State::Live, "a different device is not replaced");
        drop(other);
    }

    #[test]
    fn stop_from_inside_on_event_does_not_deadlock() {
        // Every event (frames on the frame worker, states on the supervisor) stops the
        // driver, so stops race from two callback threads.
        let rig = Rig::new("selfstop", true);
        let slot: Arc<Mutex<Option<Weak<Driver>>>> = Arc::new(Mutex::new(None));
        let log = Log::default();
        let (s2, l2) = (slot.clone(), log.clone());
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let done_tx = Mutex::new(done_tx);
        let d = Arc::new(
            Driver::start_with(
                "fake-selfstop".into(),
                rig.frame.clone(),
                rig.hid.clone(),
                Box::new(move |e| {
                    let stop = matches!(e, Event::Frame(_) | Event::State(State::Live));
                    l2.push(e);
                    if stop {
                        let d = s2.lock().unwrap().as_ref().and_then(Weak::upgrade);
                        if let Some(d) = d {
                            d.stop();
                            let _ = done_tx.lock().unwrap().send(());
                        }
                    }
                }),
                W,
            )
            .unwrap(),
        );
        let pids = d.__helper_pids();
        *slot.lock().unwrap() = Some(Arc::downgrade(&d));
        done_rx
            .recv_timeout(W)
            .expect("stop() from inside on_event returned");
        d.stop(); // waits for the stop in progress
        assert_eq!(d.state(), State::Stopped);
        assert!(gone(&pids));
        assert_eq!(log.last().as_deref(), Some("State(Stopped)"));
        // Dropping the last handle from inside a callback is fine too.
        let rig = Rig::new("selfdrop", true);
        let slot: Arc<Mutex<Option<Driver>>> = Arc::new(Mutex::new(None));
        let s2 = slot.clone();
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let done_tx = Mutex::new(done_tx);
        let d = Driver::start_with(
            "fake-selfdrop".into(),
            rig.frame.clone(),
            rig.hid.clone(),
            Box::new(move |e| {
                if matches!(e, Event::Frame(_))
                    && let Some(d) = s2.lock().unwrap().take()
                {
                    drop(d);
                    let _ = done_tx.lock().unwrap().send(());
                }
            }),
            W,
        )
        .unwrap();
        let pids = d.__helper_pids();
        *slot.lock().unwrap() = Some(d);
        done_rx
            .recv_timeout(W)
            .expect("drop from inside on_event returned");
        let t0 = Instant::now();
        while !gone(&pids) {
            assert!(
                t0.elapsed() < W,
                "helpers survived a drop from inside on_event"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn missing_helper_is_an_error() {
        let rig = Rig::new("missing", true);
        let log = Log::default();
        let l = log.clone();
        let r = Driver::start_with(
            "fake-missing".into(),
            (PathBuf::from("/nonexistent/sdframe"), vec![]),
            rig.hid.clone(),
            Box::new(move |e| l.push(e)),
            W,
        );
        assert!(matches!(r, Err(Error::HelperMissing(_))));
        assert!(log.states().is_empty(), "nothing started, nothing reported");
    }
}
