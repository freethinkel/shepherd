//! HID client: owns the sdhid process, queues commands until it is ready.
//!
//! Threading: `send` runs on the caller's thread and, once the helper is `Ready`, writes
//! straight to its stdin under the state lock (so a flush of the queue and new commands
//! never interleave). The helper's stdout thread parses lines: `ready` flushes the queue
//! under the same lock; every message is then reported to `on_event` outside the lock (the
//! owner may call back into the client). Each helper process gets a generation number; lines
//! and exits from an older generation (after `restart`/`stop`) are ignored.
//!
//! `on_event` runs inside a [`CallbackGate`]: once `stop` (or drop) returns, no event is
//! being delivered or will be (a `stop` from inside `on_event` does not wait for itself).
//! `stop` is terminal: `restart` afterwards spawns nothing.
//!
//! Displacement does not gate sending: while displaced, sdhid itself drops everything but a
//! gesture start, and a gesture start is how it reclaims, so commands keep flowing.
use crate::error::Error;
use crate::helper::{CallbackGate, Supervised, lock};
use crate::protocol::{HidCmd, HidMsg, Phase, parse_hid_line};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Queue bound before `Ready` / while the helper restarts.
pub const QUEUE_CAP: usize = 256;

/// What the HID helper reports to the owner.
#[derive(Debug, Clone, PartialEq)]
pub enum HidEvent {
    Ready {
        w: f64,
        h: f64,
    },
    Displaced,
    Reclaimed,
    Warn(String),
    Error(String),
    /// A line sdhid wrote to stderr (diagnostics; not forwarded anywhere else).
    Note(String),
    /// The current helper exited (`code` is `None` if killed by a signal; `last_stderr` is
    /// its last stderr line). The owner decides whether and when to `restart`; commands
    /// queue meanwhile. Not reported for a helper replaced by `restart` or ended by `stop`.
    Exited {
        code: Option<i32>,
        last_stderr: Option<String>,
    },
}

fn is_move(cmd: &HidCmd) -> bool {
    matches!(
        cmd,
        HidCmd::Touch {
            phase: Phase::Move,
            ..
        } | HidCmd::Touch2 {
            phase: Phase::Move,
            ..
        }
    )
}

/// Pending commands (before `Ready` / while restarting). Bounded at [`QUEUE_CAP`] by
/// evicting the oldest move; with no move left to evict, new moves are refused.
///
/// The queue may exceed [`QUEUE_CAP`] only through non-moves: starts, ends, buttons, keys
/// and text are never dropped (losing a down or an up would leave a gesture half-done), so
/// once the queue holds no moves they are still appended.
#[derive(Default)]
struct Queue {
    q: VecDeque<HidCmd>,
}

impl Queue {
    /// False if `cmd` was refused.
    fn push(&mut self, cmd: HidCmd) -> bool {
        if self.q.len() >= QUEUE_CAP {
            if let Some(i) = self.q.iter().position(is_move) {
                self.q.remove(i);
            } else if is_move(&cmd) {
                return false;
            }
        }
        self.q.push_back(cmd);
        true
    }

    fn drain(&mut self) -> impl Iterator<Item = HidCmd> + '_ {
        self.q.drain(..)
    }
}

struct State {
    generation: u64,
    helper: Option<Supervised>,
    ready: bool,
    stopped: bool,
    displaced: bool,
    screen: Option<(f64, f64)>,
    queue: Queue,
}

impl State {
    /// Write the queue to the helper if it is ready and present. A failed write means the
    /// helper died (its `Exited` is on the way): stop there, keep the rest queued.
    fn flush(&mut self) {
        if let (true, Some(h)) = (self.ready, self.helper.as_ref()) {
            while let Some(cmd) = self.queue.q.front() {
                if !h.send_line(&cmd.to_line()) {
                    self.ready = false;
                    return;
                }
                self.queue.q.pop_front();
            }
        }
    }
}

struct Shared {
    bin: PathBuf,
    args: Vec<String>,
    state: Mutex<State>,
    /// Serializes `restart`/`stop` (spawning happens outside the state lock).
    lifecycle: Mutex<()>,
    gate: CallbackGate,
    on_event: Box<dyn Fn(HidEvent) + Send + Sync>,
}

impl Shared {
    fn emit(&self, ev: HidEvent) {
        self.gate.run(|| (self.on_event)(ev));
    }

    fn on_line(&self, generation: u64, line: &str) {
        let Some(msg) = parse_hid_line(line) else {
            return;
        };
        let ev = {
            let mut st = lock(&self.state);
            if st.generation != generation {
                return;
            }
            match msg {
                HidMsg::Ready { w, h } => {
                    st.ready = true;
                    st.displaced = false;
                    st.screen = Some((w, h));
                    st.flush();
                    HidEvent::Ready { w, h }
                }
                HidMsg::Displaced => {
                    st.displaced = true;
                    HidEvent::Displaced
                }
                HidMsg::Reclaimed => {
                    st.displaced = false;
                    HidEvent::Reclaimed
                }
                HidMsg::Warn(w) => HidEvent::Warn(w),
                HidMsg::Error(e) => HidEvent::Error(e),
            }
        };
        self.emit(ev);
    }

    fn on_stderr(&self, generation: u64, line: String) {
        if lock(&self.state).generation == generation {
            self.emit(HidEvent::Note(line));
        }
    }

    fn on_exit(&self, generation: u64, code: Option<i32>, last_stderr: Option<String>) {
        {
            let mut st = lock(&self.state);
            if st.generation != generation {
                return;
            }
            st.ready = false;
        }
        self.emit(HidEvent::Exited { code, last_stderr });
    }
}

/// Owns one sdhid process at a time (`restart` replaces it).
pub struct HidClient {
    shared: Arc<Shared>,
}

impl HidClient {
    /// Spawn `bin args…` (sdhid: `[udid]`). Commands queue until its `ready` line.
    pub fn spawn(
        bin: PathBuf,
        args: Vec<String>,
        on_event: impl Fn(HidEvent) + Send + Sync + 'static,
    ) -> Result<HidClient, Error> {
        let shared = Arc::new(Shared {
            bin,
            args,
            state: Mutex::new(State {
                generation: 0,
                helper: None,
                ready: false,
                stopped: false,
                displaced: false,
                screen: None,
                queue: Queue::default(),
            }),
            lifecycle: Mutex::new(()),
            gate: CallbackGate::new(),
            on_event: Box::new(on_event),
        });
        let client = HidClient { shared };
        client.restart()?;
        Ok(client)
    }

    /// Send now if the helper is ready, else queue (see [`Queue`]). A write that fails
    /// (the helper died, `Exited` not yet handled) queues the command for the next helper.
    /// Ignored after `stop`.
    pub fn send(&self, cmd: HidCmd) {
        let mut st = lock(&self.shared.state);
        if st.stopped {
            return;
        }
        if let (true, Some(h)) = (st.ready, st.helper.as_ref()) {
            if h.send_line(&cmd.to_line()) {
                return;
            }
            st.ready = false;
        }
        st.queue.push(cmd);
    }

    /// Write a raw line to the current helper, bypassing the queue (e.g. sdhid's test-only
    /// `{"t":"_invalidate"}`). Dropped if there is no helper.
    pub fn send_raw_line(&self, line: &str) {
        if let Some(h) = lock(&self.shared.state).helper.as_ref() {
            let _ = h.send_line(line);
        }
    }

    /// Device screen size in points, from the latest `ready`.
    pub fn screen_pt(&self) -> Option<(f64, f64)> {
        lock(&self.shared.state).screen
    }

    pub fn is_displaced(&self) -> bool {
        lock(&self.shared.state).displaced
    }

    pub fn pid(&self) -> Option<u32> {
        lock(&self.shared.state)
            .helper
            .as_ref()
            .map(Supervised::pid)
    }

    /// Stop the current helper (if any) and spawn a fresh one. Commands queue until its
    /// `ready`; queued ones survive the restart. After `stop` this does nothing (`Ok`).
    pub fn restart(&self) -> Result<(), Error> {
        let _l = lock(&self.shared.lifecycle);
        let (generation, old) = {
            let mut st = lock(&self.shared.state);
            if st.stopped {
                return Ok(());
            }
            st.generation += 1;
            st.ready = false;
            st.displaced = false;
            (st.generation, st.helper.take())
        };
        drop(old); // stop() outside the state lock: it may wait up to 250 ms
        let (s1, s2, s3) = (
            Arc::downgrade(&self.shared),
            Arc::downgrade(&self.shared),
            Arc::downgrade(&self.shared),
        );
        let helper = Supervised::spawn_with_stderr(
            self.shared.bin.clone(),
            self.shared.args.clone(),
            move |line| {
                if let Some(sh) = s1.upgrade() {
                    sh.on_line(generation, &line);
                }
            },
            move |line| {
                if let Some(sh) = s2.upgrade() {
                    sh.on_stderr(generation, line);
                }
            },
            move |code, last| {
                if let Some(sh) = s3.upgrade() {
                    sh.on_exit(generation, code, last);
                }
            },
        )?;
        let mut st = lock(&self.shared.state);
        if st.stopped || st.generation != generation {
            // stop() ran while we were spawning (it does not take the lifecycle lock).
            drop(st);
            drop(helper);
            return Ok(());
        }
        st.helper = Some(helper);
        st.flush(); // `ready` may have arrived before the helper was stored
        Ok(())
    }

    /// Terminal: stop the helper, drop queued commands, ignore later sends and restarts.
    /// Returns once no `on_event` is running (unless called from inside one) and none will
    /// start. Idempotent; never blocks on `restart` (it does not take the lifecycle lock).
    pub fn stop(&self) {
        let old = {
            let mut st = lock(&self.shared.state);
            st.generation += 1;
            st.ready = false;
            st.stopped = true;
            st.queue.drain().for_each(drop);
            st.helper.take()
        };
        self.shared.gate.close();
        drop(old);
    }
}

impl Drop for HidClient {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helper::{fake_helper, pid_gone};
    use crate::protocol::{Edge, Phase};
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::{Duration, Instant};

    const W: Duration = Duration::from_secs(5);

    fn touch(phase: Phase, x: f64) -> HidCmd {
        HidCmd::Touch {
            phase,
            x,
            y: 0.0,
            edge: Edge::None,
        }
    }

    fn touch2(phase: Phase, x: f64) -> HidCmd {
        HidCmd::Touch2 {
            phase,
            x1: x,
            y1: 0.0,
            x2: 0.0,
            y2: 0.0,
        }
    }

    struct Fake {
        dir: PathBuf,
        client: HidClient,
        events: mpsc::Receiver<HidEvent>,
    }

    impl Drop for Fake {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    impl Fake {
        fn new(tag: &str) -> Fake {
            let (tx, rx) = mpsc::channel();
            let tx = std::sync::Mutex::new(tx);
            let mut f = Fake::with(tag, move |e| {
                let _ = tx.lock().unwrap().send(e);
            });
            f.events = rx;
            f
        }

        fn with(tag: &str, on_event: impl Fn(HidEvent) + Send + Sync + 'static) -> Fake {
            let dir = std::env::temp_dir().join(format!("sdhid-fake-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let client = HidClient::spawn(
                fake_helper(),
                vec!["hid".into(), dir.to_string_lossy().into_owned()],
                on_event,
            )
            .unwrap();
            Fake {
                dir,
                client,
                events: mpsc::channel().1,
            }
        }

        fn go(&self) {
            std::fs::write(self.dir.join("go"), b"").unwrap();
        }

        fn expect(&self, want: HidEvent) {
            loop {
                match self.events.recv_timeout(W) {
                    Ok(HidEvent::Warn(_) | HidEvent::Note(_)) => continue,
                    got => {
                        assert_eq!(got.ok(), Some(want));
                        return;
                    }
                }
            }
        }

        fn log(&self) -> Vec<String> {
            std::fs::read_to_string(self.dir.join("log"))
                .unwrap_or_default()
                .lines()
                .map(String::from)
                .collect()
        }

        /// Send a sentinel and wait until the helper logged it: everything before is in.
        fn sync(&self, code: u32) -> Vec<String> {
            self.client.send(HidCmd::Key { code });
            let want = HidCmd::Key { code }.to_line();
            let t0 = Instant::now();
            loop {
                let log = self.log();
                if log.last() == Some(&want) {
                    return log[..log.len() - 1].to_vec();
                }
                assert!(
                    t0.elapsed() < W,
                    "sentinel never reached the helper; log has {} lines",
                    log.len()
                );
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }

    #[test]
    fn queue_policy_drops_oldest_moves_then_refuses_moves() {
        let mut q = Queue::default();
        assert!(q.push(touch(Phase::Down, 0.0)));
        for i in 1..=300 {
            assert!(q.push(touch(Phase::Move, i as f64)));
        }
        assert!(q.push(touch(Phase::Up, 0.0)));
        let v: Vec<HidCmd> = q.drain().collect();
        assert_eq!(v.len(), QUEUE_CAP);
        assert_eq!(v[0], touch(Phase::Down, 0.0));
        assert_eq!(v[QUEUE_CAP - 1], touch(Phase::Up, 0.0));
        let moves: Vec<HidCmd> = (47..=300).map(|i| touch(Phase::Move, i as f64)).collect();
        assert_eq!(
            &v[1..QUEUE_CAP - 1],
            &moves[..],
            "the newest 254 moves, in order"
        );

        // Full of non-moves: new moves are refused, starts/ends are still kept.
        let mut q = Queue::default();
        for i in 0..QUEUE_CAP {
            assert!(q.push(HidCmd::Key { code: i as u32 }));
        }
        assert!(
            !q.push(touch2(Phase::Move, 1.0)),
            "move refused when no move can be evicted"
        );
        assert!(q.push(touch2(Phase::Down, 1.0)), "starts are never dropped");
        assert!(q.push(touch(Phase::Up, 1.0)), "ends are never dropped");
        let v: Vec<HidCmd> = q.drain().collect();
        assert_eq!(v.len(), QUEUE_CAP + 2);
        assert_eq!(v[QUEUE_CAP], touch2(Phase::Down, 1.0));
        assert!(q.drain().next().is_none());
    }

    #[test]
    fn queue_overflow_drops_moves_first() {
        let f = Fake::new("overflow");
        f.client.send(touch(Phase::Down, 0.0));
        for i in 1..=300 {
            f.client.send(touch(Phase::Move, i as f64));
        }
        f.client.send(touch(Phase::Up, 0.0));
        assert_eq!(f.client.screen_pt(), None);
        std::thread::sleep(Duration::from_millis(50));
        assert!(
            f.log().is_empty(),
            "nothing may reach the helper before Ready"
        );

        f.go();
        f.expect(HidEvent::Ready { w: 393.0, h: 852.0 });
        assert_eq!(f.client.screen_pt(), Some((393.0, 852.0)));
        let got = f.sync(9999);
        let mut want = vec![touch(Phase::Down, 0.0).to_line()];
        want.extend((47..=300).map(|i| touch(Phase::Move, i as f64).to_line()));
        want.push(touch(Phase::Up, 0.0).to_line());
        assert_eq!(got.len(), QUEUE_CAP);
        assert_eq!(got, want, "down, the newest moves, up");
    }

    #[test]
    fn gesture_start_is_sent_while_displaced() {
        let f = Fake::new("displaced");
        f.go();
        f.expect(HidEvent::Ready { w: 393.0, h: 852.0 });
        f.client.send_raw_line(r#"{"t":"_invalidate"}"#);
        f.expect(HidEvent::Displaced);
        assert!(f.client.is_displaced());
        // Moves still go straight to the helper (it drops them itself), and the start reclaims.
        f.client.send(touch(Phase::Move, 1.0));
        f.client.send(touch(Phase::Down, 2.0));
        f.expect(HidEvent::Reclaimed);
        assert!(!f.client.is_displaced());
        let got = f.sync(9998);
        assert_eq!(
            got,
            vec![
                touch(Phase::Move, 1.0).to_line(),
                touch(Phase::Down, 2.0).to_line()
            ]
        );
    }

    #[test]
    fn queues_while_restarting() {
        let f = Fake::new("restart");
        f.go();
        f.expect(HidEvent::Ready { w: 393.0, h: 852.0 });
        let old = f.client.pid().unwrap();
        f.client.send_raw_line(r#"{"t":"_exit","code":5}"#);
        f.expect(HidEvent::Exited {
            code: Some(5),
            last_stderr: Some("fake_helper: exiting 5".into()),
        });
        f.client.send(touch(Phase::Down, 3.0));
        f.client.send(touch(Phase::Up, 3.0));
        f.client.restart().unwrap();
        assert_ne!(f.client.pid(), Some(old));
        f.expect(HidEvent::Ready { w: 393.0, h: 852.0 });
        let got = f.sync(9997);
        assert_eq!(
            got,
            vec![
                touch(Phase::Down, 3.0).to_line(),
                touch(Phase::Up, 3.0).to_line()
            ]
        );
    }

    #[test]
    fn no_event_after_stop() {
        let (in_tx, in_rx) = mpsc::channel();
        let in_tx = Mutex::new(in_tx);
        let (etx, erx) = mpsc::channel();
        let etx = Mutex::new(etx);
        let done = Arc::new(AtomicBool::new(false));
        let d = done.clone();
        let f = Fake::with("nolate", move |e| {
            let ready = matches!(e, HidEvent::Ready { .. });
            let _ = etx.lock().unwrap().send(e);
            if ready {
                let _ = in_tx.lock().unwrap().send(());
                std::thread::sleep(Duration::from_millis(300));
                d.store(true, Ordering::SeqCst);
            }
        });
        f.go();
        in_rx.recv_timeout(W).expect("on_event entered");
        f.client.stop();
        assert!(
            done.load(Ordering::SeqCst),
            "stop returned while on_event was still running"
        );
        assert!(matches!(erx.recv_timeout(W), Ok(HidEvent::Ready { .. })));
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(
            erx.try_recv().ok(),
            None,
            "on_event ran after stop (e.g. Exited)"
        );
    }

    #[test]
    fn stop_from_inside_on_event_does_not_deadlock() {
        let dir = std::env::temp_dir().join(format!("sdhid-fake-{}-selfstop", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let slot: Arc<Mutex<Option<std::sync::Weak<HidClient>>>> = Arc::new(Mutex::new(None));
        let (dtx, drx) = mpsc::channel();
        let dtx = Mutex::new(dtx);
        let s2 = slot.clone();
        let client = Arc::new(
            HidClient::spawn(
                fake_helper(),
                vec!["hid".into(), dir.to_string_lossy().into_owned()],
                move |e| {
                    if matches!(e, HidEvent::Ready { .. }) {
                        let c = s2
                            .lock()
                            .unwrap()
                            .as_ref()
                            .and_then(std::sync::Weak::upgrade);
                        if let Some(c) = c {
                            c.stop();
                            let _ = dtx.lock().unwrap().send(c.pid());
                        }
                    }
                },
            )
            .unwrap(),
        );
        *slot.lock().unwrap() = Some(Arc::downgrade(&client));
        let pid = client.pid().unwrap();
        std::fs::write(dir.join("go"), b"").unwrap();
        assert_eq!(
            drx.recv_timeout(W),
            Ok(None),
            "stop() from inside on_event returned, helper released"
        );
        assert!(pid_gone(pid));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn restart_after_stop_is_noop() {
        let f = Fake::new("stoprestart");
        let pid = f.client.pid().unwrap();
        f.client.stop();
        assert!(f.client.restart().is_ok());
        assert_eq!(
            f.client.pid(),
            None,
            "restart after stop must not spawn a helper"
        );
        assert!(pid_gone(pid));
    }

    #[test]
    fn stop_ends_helper_and_is_idempotent() {
        let f = Fake::new("stop");
        let pid = f.client.pid().unwrap();
        f.client.stop();
        assert!(pid_gone(pid));
        f.client.stop();
        f.client.send(touch(Phase::Down, 0.0)); // no helper: ignored, no panic
        assert!(!Path::new(&f.dir.join("log")).exists());
    }
}
