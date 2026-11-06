//! iPhone Duo's hinge (macOS): what Xcode's Device Hub does to it, done the way serve-sim does. A small program in the
//! simulator (`duo-hid.m`, built on first use with the simulator SDK) registers the hinge and motion sensors and
//! reports what Shepherd tells it. SpringBoard says what came of it in its log: which screen is live and the angle,
//! as `COMPLETED: .cover:pri .inner:off | 0.0° …` once a fold hands over to the other screen, and how open the hinge
//! is (`CMMagicPoseEvent … hingeState:OpenFlat`) whenever it moves. That log is followed for as long as the session
//! lives, so a fold made elsewhere (Xcode's own window) shows too.
use super::{Feed, Fold, Hinge, Stand, Wake};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SOURCE: &str = include_str!("duo-hid.m");
/// The screens' pixels: the cover's, and the inner one's, which is the largest. Each stream names its own: one left to
/// pick the largest watches the main display's port, and stays dark when started as a fold hands over.
pub(super) const COVER: (u32, u32) = (1398, 2034);
pub(super) const INNER: (u32, u32) = (2007, 2853);
/// The simulator's TV-out screen's, which shows nothing.
pub(super) const TV_OUT: (u32, u32) = (720, 480);

/// The cover's screen id (`simctl io enumerate`), else the inner one's: also the touch target that lands on it,
/// CoreDevice's `DigitizerTarget.display<id>`.
pub(super) fn screen(cover: bool) -> u8 {
    if cover { 1 } else { 3 }
}
const SETTLED: &str = r#"process == "SpringBoard" AND ((category == "DisplayContentMode" AND eventMessage BEGINSWITH "COMPLETED") OR eventMessage BEGINSWITH "Sending MagicPose event")"#;

/// Whether simulator device type `device_type` (simctl's identifier) is iPhone Duo.
pub(super) fn is_duo(device_type: &str) -> bool {
    device_type.ends_with(".iPhone-Duo")
}

/// The guest program, built for this Mac's simulators into the cache the first time.
fn program() -> anyhow::Result<std::path::PathBuf> {
    let hash: String = <sha2::Sha256 as sha2::Digest>::digest(SOURCE.as_bytes()).iter().take(6).map(|b| format!("{b:02x}")).collect();
    let dir = crate::settings::cache_dir().join("device");
    let path = dir.join(format!("duo-hid-{hash}"));
    if path.is_file() {
        return Ok(path);
    }
    std::fs::create_dir_all(&dir)?;
    let source = dir.join(format!("duo-hid-{hash}.m"));
    std::fs::write(&source, SOURCE)?;
    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "x86_64" };
    let partial = path.with_extension("part");
    let out = crate::process::command("xcrun")
        .args(["--sdk", "iphonesimulator", "clang", "-arch", arch, "-mios-simulator-version-min=15.0", "-fobjc-arc", "-framework", "Foundation", "-Wl,-adhoc_codesign", "-o"])
        .arg(&partial)
        .arg(&source)
        .output()?;
    anyhow::ensure!(out.status.success(), "Building the Duo's hinge helper failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    std::fs::rename(&partial, &path)?;
    Ok(path)
}

/// The guest program running, its pipes.
struct Helper {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Helper {
    fn start(udid: &str) -> anyhow::Result<Helper> {
        let mut child = crate::process::command("xcrun")
            .args(["simctl", "spawn", udid])
            .arg(program()?)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let (input, output) = (child.stdin.take().expect("piped"), child.stdout.take().expect("piped"));
        Ok(Helper { child, input, output: BufReader::new(output) })
    }

    /// Sends `line`; whether the device took it.
    fn send(&mut self, line: &str) -> anyhow::Result<()> {
        writeln!(self.input, "{line}")?;
        let mut reply = String::new();
        self.output.read_line(&mut reply)?;
        anyhow::ensure!(reply.trim() == "OK", "The Duo did not take `{line}`");
        Ok(())
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `(cover, angle)` after SpringBoard's log line `line`, if it tells of the hinge: `COMPLETED: .cover:pri .inner:off |
/// 0.0° …` says both; a motion event's `hingeState:` only how open it is, which is enough to tell Book from Open (no
/// screen hands over between them, so nothing is COMPLETED).
fn settled(line: &str, (cover, angle): (bool, f64)) -> Option<(bool, f64)> {
    if let Some(rest) = line.split("COMPLETED:").nth(1) {
        let cover = rest.split(".cover:").nth(1)?.split_whitespace().next()? == "pri";
        let angle = rest.split("| ").nth(1)?.split('°').next()?.trim().parse().ok()?;
        return Some((cover, angle));
    }
    let angle = match line.split("hingeState:").nth(1)?.split_whitespace().next()? {
        "OpenFlat" => 180.,
        "Closed" => 0.,
        // part open: as far as it was, if it was
        _ if angle > 0. && angle < 180. => angle,
        _ => 90.,
    };
    Some((cover, angle))
}

/// The live screen's UI orientation from `simctl io enumerate`, by t3code's names: a phone's one screen reads as the
/// Duo's cover, `primary`.
pub(super) fn orientation(udid: &str, cover: bool) -> &'static str {
    let Ok(out) = crate::process::command("xcrun").args(["simctl", "io", udid, "enumerate"]).output() else { return "portrait" };
    let text = String::from_utf8_lossy(&out.stdout);
    let name = if cover { "primary" } else { "primary-1" };
    let mut current = "";
    for line in text.lines().map(str::trim) {
        if let Some(n) = line.strip_prefix("Device Name:") {
            current = n.trim();
        } else if let Some(o) = line.strip_prefix("UI Orientation:")
            && current == name
        {
            return match o.trim() {
                "Landscape Left" => "landscape_left",
                "Landscape Right" => "landscape_right",
                "Portrait Upside Down" => "portrait_upside_down",
                _ => "portrait",
            };
        }
    }
    "portrait"
}

struct Shared {
    udid: String,
    feed: Arc<Feed>,
    wake: Wake,
    /// Streams the screen that's live: the cover's, or the inner one's.
    show: Box<dyn Fn(bool) + Send + Sync>,
    helper: Mutex<Option<Helper>>,
    /// The move to make next: one is made at a time, and only the latest waiting one (a pinch sends many).
    next: Mutex<Option<Hinge>>,
    moving: AtomicBool,
    /// Moves made, and screen handovers SpringBoard has logged: a fold that hands over and isn't seen to is stuck.
    moves: AtomicU64,
    handovers: AtomicU64,
    /// The angle the hinge was last set to (or found at), where the next sweep starts: SpringBoard's motion events
    /// say only roughly where it is.
    at: Mutex<f64>,
    log: Mutex<Option<Child>>,
    closed: AtomicBool,
}

impl Shared {
    /// SpringBoard settled a fold: show the screen it left live.
    /// The hinge is at `angle` with the cover live, or not.
    fn settled(&self, cover: bool, angle: f64) {
        let was = self.feed.fold().map(|f| f.cover);
        if was != Some(cover) && !self.closed.load(Ordering::Relaxed) {
            (self.show)(cover);
        }
        let orientation = orientation(&self.udid, cover);
        self.feed.change_fold(|f| {
            // a stand is set at an angle of its own; another one means it was moved
            if f.stand.is_some_and(|s| (stand_angle(s) - angle).abs() > 1.) {
                f.stand = None;
            }
            (f.cover, f.angle, f.orientation) = (cover, angle, orientation);
        });
        (self.wake)();
    }

    /// Follows SpringBoard's log while the session lives.
    fn follow(self: Arc<Self>) {
        while !self.closed.load(Ordering::Relaxed) {
            let child = crate::process::command("xcrun")
                .args(["simctl", "spawn", &self.udid, "log", "stream", "--style", "compact", "--predicate", SETTLED])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn();
            let Ok(mut child) = child else { return };
            let lines = BufReader::new(child.stdout.take().expect("piped")).lines();
            if let Ok(mut log) = self.log.lock() {
                *log = Some(child);
            }
            for line in lines.map_while(Result::ok) {
                if line.contains("COMPLETED:") {
                    self.handovers.fetch_add(1, Ordering::Relaxed);
                }
                let was = self.feed.fold().map_or((true, 0.), |f| (f.cover, f.angle));
                if let Some(now) = settled(&line, was).filter(|now| *now != was) {
                    self.settled(now.0, now.1);
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    /// Sends the helper `lines` in turn, starting it if it isn't running.
    fn send(&self, lines: &[String]) -> anyhow::Result<()> {
        let mut helper = self.helper.lock().map_err(|_| anyhow::anyhow!("poisoned"))?;
        // a move still on its way when the tab closed starts no helper the session no longer stops
        anyhow::ensure!(!self.closed.load(Ordering::Relaxed), "The device was closed");
        if helper.is_none() {
            *helper = Some(Helper::start(&self.udid)?);
        }
        let result = lines.iter().try_for_each(|l| helper.as_mut().expect("started").send(l));
        if result.is_err() {
            // it's started over next time
            *helper = None;
        }
        result
    }

    /// Makes one move, and notes what came of it.
    fn make(self: &Arc<Self>, hinge: Hinge) {
        // the screen the fold will leave live, shown now as serve-sim does: SpringBoard says so seconds later, once
        // the screen has gone dark and lit again
        let cover = match hinge {
            Hinge::Angle(a) => Some(a <= 54.),
            Hinge::Stand(s) => Some(s == Stand::Tent),
            Hinge::Hold(_) => None,
        };
        let handover = cover.filter(|c| self.feed.fold().is_some_and(|f| f.cover != *c));
        if let Some(cover) = handover {
            (self.show)(cover);
            self.feed.change_fold(|f| f.cover = cover);
        }
        let (made, seen) = (self.moves.fetch_add(1, Ordering::Relaxed) + 1, self.handovers.load(Ordering::Relaxed));
        let from = self.at.lock().map_or(0., |a| *a);
        let sent = self.send(&commands(hinge, from));
        let to = match hinge {
            Hinge::Angle(a) => a.clamp(0., 180.),
            Hinge::Stand(s) => stand_angle(s),
            Hinge::Hold(_) => from,
        };
        if let (Ok(()), Ok(mut at)) = (&sent, self.at.lock()) {
            *at = to;
        }
        if let (Some(cover), Err(_)) = (handover, &sent) {
            // the hinge never moved: the screen shown ahead of it goes back
            (self.show)(!cover);
        }
        self.feed.change_fold(|f| {
            match (&sent, hinge) {
                (Err(e), _) => {
                    f.error = Some(format!("{e:#}"));
                    if let Some(cover) = handover {
                        f.cover = !cover;
                    }
                }
                // SpringBoard logs a fold only when it hands over to the other screen (Book to Open doesn't): the angle
                // the device took is its own
                (Ok(()), Hinge::Stand(s)) => (f.stand, f.angle) = (Some(s), stand_angle(s)),
                (Ok(()), Hinge::Angle(a)) => (f.stand, f.angle) = (None, a.clamp(0., 180.)),
                // turned in the hand, it stays in its stand
                (Ok(()), Hinge::Hold(_)) => {}
            }
        });
        (self.wake)();
        // SpringBoard now and then starts a handover too late to follow the hinge there, and waits for it to move
        // again (mostly a first fold, soon after it boots): unless it's seen to hand over, or another move
        // came, the hinge is moved a little and back
        if let (Some(cover), Ok(())) = (handover, &sent) {
            let shared = self.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(1000));
                let stuck = shared.handovers.load(Ordering::Relaxed) == seen && shared.moves.load(Ordering::Relaxed) == made;
                if stuck && !shared.closed.load(Ordering::Relaxed) {
                    let _ = shared.send(&nudge(to, cover));
                }
            });
        }
        // the UI turns a moment after it's held otherwise, and says so nowhere SpringBoard's log does; read off the
        // device by then, without holding up the next move
        let (udid, feed, wake) = (self.udid.clone(), self.feed.clone(), self.wake.clone());
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(700));
            let orientation = orientation(&udid, feed.fold().is_some_and(|f| f.cover));
            feed.change_fold(|f| f.orientation = orientation);
            wake();
        });
    }
}

/// The hinge angle a stand is set at: serve-sim's.
fn stand_angle(stand: Stand) -> f64 {
    match stand {
        Stand::Laptop => 90.,
        Stand::Tent => 80.,
    }
}

/// The stand the Duo is left in, read off how it is, for a tab opened onto it: Tent's angle is its own, and Laptop
/// shares Book's but holds the phone on its side, which leaves the inner screen's UI upright (Book's turns).
fn stand_at(angle: f64, cover: bool, orientation: &str) -> Option<Stand> {
    let at = |stand| (angle - stand_angle(stand)).abs() < 0.5;
    if at(Stand::Tent) {
        Some(Stand::Tent)
    } else if at(Stand::Laptop) && !cover && orientation == "portrait" {
        Some(Stand::Laptop)
    } else {
        None
    }
}

/// What the helper is told for `hinge`, the hinge at `from`: how the phone is held, then the hinge swept open or shut,
/// a fold across the whole of it in 0.4 s. Held upright first when it stood, so the fold lands in the screen's own
/// frame. Swept, as serve-sim's: SpringBoard's fold follows the hinge, and a jump now and then leaves it stuck, every
/// fold after ignored until the simulator restarts.
fn commands(hinge: Hinge, from: f64) -> Vec<String> {
    let (held, angle) = match hinge {
        Hinge::Angle(a) => ("portrait", a.clamp(0., 180.)),
        Hinge::Stand(Stand::Laptop) => ("landscape-left", stand_angle(Stand::Laptop)),
        Hinge::Stand(Stand::Tent) => ("facedown", stand_angle(Stand::Tent)),
        Hinge::Hold(o) => {
            let held = match o {
                "landscape_left" => "landscape-left",
                "landscape_right" => "landscape-right",
                "portrait_upside_down" => "pud",
                _ => "portrait",
            };
            return vec![format!("orientation {held}")];
        }
    };
    let ms = ((angle - from).abs() / 180. * 400.).round();
    vec![format!("orientation {held}"), format!("sweep {from} {angle} {ms}")]
}

/// The hinge at `angle` moved a little and back, on the way to the screen it hands over to (shut, for the cover), or
/// the other way at the end of its travel: further, and the way back would start a fold of its own.
fn nudge(angle: f64, cover: bool) -> [String; 2] {
    let way = if cover { -5. } else { 5. };
    let to = if (0. ..=180.).contains(&(angle + way)) { angle + way } else { angle - way };
    [format!("sweep {angle} {to} 60"), format!("sweep {to} {angle} 60")]
}

/// Where the Duo `udid`'s hinge was last left within `last` (`log show`'s: `1h`, `boot`): whether the cover is live,
/// and the angle; `None` when it wasn't moved then.
pub(super) fn last_settled(udid: &str, last: &str) -> Option<(bool, f64)> {
    let shown = crate::process::command("xcrun")
        .args(["simctl", "spawn", udid, "log", "show", "--last", last, "--style", "compact", "--predicate", SETTLED])
        .output();
    let text = shown.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    // one never opened since it booted is shut
    text.lines().fold(None, |was, line| settled(line, was.unwrap_or((true, 0.))).or(was))
}

/// The window of SpringBoard's log a tab opens with: the whole boot's takes 20 s on one booted for hours.
pub(super) const RECENT: &str = "1h";

/// A Duo's hinge, for as long as its session.
pub(super) struct Duo(Arc<Shared>);

impl Duo {
    /// Follows the Duo `udid`'s hinge into `feed`, from `last` (`last_settled` over the `RECENT` log; `None` when it
    /// says nothing, and the whole boot's is read behind it); `show` switches the stream to the cover (true) or the
    /// inner screen.
    pub(super) fn open(udid: &str, last: Option<(bool, f64)>, feed: Arc<Feed>, wake: Wake, show: impl Fn(bool) + Send + Sync + 'static) -> Duo {
        let (cover, angle) = last.unwrap_or((true, 0.));
        let orientation = orientation(udid, cover);
        if let Ok(mut fold) = feed.fold.lock() {
            let stand = stand_at(angle, cover, orientation);
            *fold = Some(Fold { duo: true, angle, cover, stand, orientation, busy: false, error: None });
        }
        let shared = Arc::new(Shared {
            udid: udid.into(),
            feed,
            wake,
            show: Box::new(show),
            helper: Mutex::default(),
            next: Mutex::default(),
            moving: AtomicBool::new(false),
            moves: AtomicU64::new(0),
            handovers: AtomicU64::new(0),
            at: Mutex::new(angle),
            log: Mutex::default(),
            closed: AtomicBool::new(false),
        });
        std::thread::spawn({
            let shared = shared.clone();
            move || shared.follow()
        });
        if last.is_none() {
            let shared = shared.clone();
            std::thread::spawn(move || {
                let opened = shared.feed.fold();
                let Some((cover, angle)) = last_settled(&shared.udid, "boot") else { return };
                // a fold seen or made since knows better
                if shared.feed.fold() != opened || shared.moves.load(Ordering::Relaxed) > 0 || shared.closed.load(Ordering::Relaxed) {
                    return;
                }
                if let Ok(mut at) = shared.at.lock() {
                    *at = angle;
                }
                shared.settled(cover, angle);
                shared.feed.change_fold(|f| f.stand = stand_at(f.angle, f.cover, f.orientation));
            });
        }
        // the helper started ahead of the first fold, which would wait a second on it
        std::thread::spawn({
            let shared = shared.clone();
            move || shared.send(&[])
        });
        Duo(shared)
    }

    /// Moves the hinge, off this thread; SpringBoard's log says where it ended up.
    pub(super) fn hinge(&self, hinge: Hinge) {
        let shared = self.0.clone();
        if let Ok(mut next) = shared.next.lock() {
            *next = Some(hinge);
        }
        if shared.moving.swap(true, Ordering::AcqRel) {
            return;
        }
        shared.feed.change_fold(|f| (f.busy, f.error) = (true, None));
        std::thread::spawn(move || {
            loop {
                if let Some(hinge) = shared.next.lock().ok().and_then(|mut n| n.take()) {
                    shared.make(hinge);
                    continue;
                }
                // not busy before letting go: a mover that takes over after sets it busy again, and no one clears
                // that but itself
                shared.feed.change_fold(|f| f.busy = false);
                (shared.wake)();
                shared.moving.store(false, Ordering::Release);
                // one asked for just now, after the take and before this
                let late = shared.next.lock().is_ok_and(|n| n.is_some());
                if !late || shared.moving.swap(true, Ordering::AcqRel) {
                    break;
                }
                shared.feed.change_fold(|f| f.busy = true);
            }
        });
    }
}

impl Drop for Duo {
    fn drop(&mut self) {
        self.0.closed.store(true, Ordering::Relaxed);
        if let Ok(Some(mut log)) = self.0.log.lock().map(|mut l| l.take()) {
            let _ = log.kill();
            let _ = log.wait();
        }
        // off this (the UI's) thread: a move or the first build of the helper can hold it for seconds
        let shared = self.0.clone();
        std::thread::spawn(move || {
            if let Ok(mut helper) = shared.helper.lock() {
                helper.take();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn springboard_settles() {
        let line = "2026-11-05 13:14:15.163 Df SpringBoard[41966:44f05] [com.apple.SpringBoard:DisplayContentMode] COMPLETED:     .cover:pri .inner:off | 0.0° (0.00 fold:0.00): fullyClosed [device:Pu]";
        assert_eq!(settled(line, (false, 180.)), Some((true, 0.)));
        let line = "… COMPLETED:     .cover:aux .inner:pri | 90.0° (0.50 fold:0.50): halfOpen [device:Pu]";
        assert_eq!(settled(line, (true, 0.)), Some((false, 90.)));
        assert_eq!(settled("STARTING: .cover:aux→pri .inner:pri→off | 0.0°", (true, 0.)), None);
        // Book to Open hands no screen over: only the motion event says it's flat
        let pose = "Sending MagicPose event to client: CMMagicPoseEvent deviceOrientation:Portrait hingeState:OpenFlat tabletopPose:Ambiguous";
        assert_eq!(settled(pose, (false, 90.)), Some((false, 180.)));
        assert_eq!(settled(&pose.replace("OpenFlat", "PartiallyOpen"), (false, 130.)), Some((false, 130.)));
        assert_eq!(settled(&pose.replace("OpenFlat", "PartiallyOpen"), (false, 180.)), Some((false, 90.)));
    }

    #[test]
    fn stands_read_off_the_device() {
        assert_eq!(stand_at(90., false, "portrait"), Some(Stand::Laptop));
        // Book: the same angle, held upright
        assert_eq!(stand_at(90., false, "landscape_left"), None);
        assert_eq!(stand_at(80., true, "portrait"), Some(Stand::Tent));
        assert_eq!(stand_at(180., false, "portrait"), None);
    }

    #[test]
    fn stands_are_held_then_folded() {
        assert_eq!(commands(Hinge::Stand(Stand::Laptop), 180.), ["orientation landscape-left", "sweep 180 90 200"]);
        assert_eq!(commands(Hinge::Angle(200.), 0.), ["orientation portrait", "sweep 0 180 400"]);
        assert_eq!(commands(Hinge::Hold("portrait_upside_down"), 90.), ["orientation pud"]);
        assert_eq!(nudge(180., false), ["sweep 180 175 60", "sweep 175 180 60"]);
        assert_eq!(nudge(90., false), ["sweep 90 95 60", "sweep 95 90 60"]);
        assert_eq!(nudge(0., true), ["sweep 0 5 60", "sweep 5 0 60"]);
    }
}
