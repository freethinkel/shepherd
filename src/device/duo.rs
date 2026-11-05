//! iPhone Duo's hinge (macOS): what Xcode's Device Hub does to it, done the way serve-sim does. A small program in the
//! simulator (`duo-hid.m`, built on first use with the simulator SDK) registers the hinge and motion sensors and
//! reports what Shepherd tells it. SpringBoard says what came of it in its log: which screen is live and the angle,
//! as `COMPLETED: .cover:pri .inner:off | 0.0° …` once a fold has settled. That log is followed for as long as the
//! session lives, so a fold made elsewhere (Xcode's own window) shows too.
use super::{Feed, Fold, Hinge, Stand, Wake};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SOURCE: &str = include_str!("duo-hid.m");
/// The cover screen's pixels; the inner one is the largest, simdriver's own pick.
pub(super) const COVER: (u32, u32) = (1398, 2034);

/// The cover's screen id (`simctl io enumerate`), else the inner one's: also the touch target that lands on it,
/// CoreDevice's `DigitizerTarget.display<id>`.
pub(super) fn screen(cover: bool) -> u8 {
    if cover { 1 } else { 3 }
}
const SETTLED: &str = r#"process == "SpringBoard" AND category == "DisplayContentMode" AND eventMessage BEGINSWITH "COMPLETED""#;

pub(super) fn is_duo(udid: &str) -> bool {
    simdriver::list_devices().is_ok_and(|all| all.iter().any(|d| d.udid == udid && d.device_type.ends_with(".iPhone-Duo")))
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

/// `COMPLETED: .cover:pri .inner:off | 0.0° …`: whether the cover is live, and the angle.
fn settled(line: &str) -> Option<(bool, f64)> {
    let rest = line.split("COMPLETED:").nth(1)?;
    let cover = rest.split(".cover:").nth(1)?.split_whitespace().next()? == "pri";
    let angle = rest.split("| ").nth(1)?.split('°').next()?.trim().parse().ok()?;
    Some((cover, angle))
}

/// The live screen's UI orientation from `simctl io enumerate`, by t3code's names.
fn orientation(udid: &str, cover: bool) -> &'static str {
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
    log: Mutex<Option<Child>>,
    closed: AtomicBool,
}

impl Shared {
    /// SpringBoard settled a fold: show the screen it left live.
    fn settled(&self, cover: bool, angle: f64) {
        let was = self.feed.fold().map(|f| f.cover);
        // a session closing must not start its stream again
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
                if let Some((cover, angle)) = settled(&line) {
                    self.settled(cover, angle);
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    /// Sends the helper `lines` in turn, starting it if it isn't running.
    fn send(&self, lines: &[String]) -> anyhow::Result<()> {
        let mut helper = self.helper.lock().map_err(|_| anyhow::anyhow!("poisoned"))?;
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
    fn make(&self, hinge: Hinge) {
        let sent = self.send(&commands(hinge));
        // the UI turns a moment after it's held otherwise, and says so nowhere SpringBoard's log does
        std::thread::sleep(Duration::from_millis(700));
        let cover = self.feed.fold().is_some_and(|f| f.cover);
        let orientation = orientation(&self.udid, cover);
        self.feed.change_fold(|f| {
            f.orientation = orientation;
            match (&sent, hinge) {
                (Err(e), _) => f.error = Some(format!("{e:#}")),
                (Ok(()), Hinge::Stand(s)) => f.stand = Some(s),
                (Ok(()), Hinge::Angle(_)) => f.stand = None,
                // turned in the hand, it stays in its stand
                (Ok(()), Hinge::Hold(_)) => {}
            }
        });
        (self.wake)();
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

/// What the helper is told for `hinge`: how the phone is held, then how far it's open. Held upright first when it
/// stood, so the fold lands in the screen's own frame.
fn commands(hinge: Hinge) -> Vec<String> {
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
    vec![format!("orientation {held}"), format!("angle {angle}")]
}

/// Where this boot last left the Duo `udid`'s hinge: whether the cover is live, and the angle.
pub(super) fn last_settled(udid: &str) -> (bool, f64) {
    let shown = crate::process::command("xcrun")
        .args(["simctl", "spawn", udid, "log", "show", "--last", "boot", "--style", "compact", "--predicate", SETTLED])
        .output();
    let last = shown.ok().and_then(|o| String::from_utf8_lossy(&o.stdout).lines().rev().find_map(settled));
    // one never opened since it booted is shut
    last.unwrap_or((true, 0.))
}

/// A Duo's hinge, for as long as its session.
pub(super) struct Duo(Arc<Shared>);

impl Duo {
    /// Follows the Duo `udid`'s hinge into `feed`, from `(cover, angle)` (`last_settled`); `show` switches the stream to
    /// the cover (true) or the inner screen.
    pub(super) fn open(udid: &str, (cover, angle): (bool, f64), feed: Arc<Feed>, wake: Wake, show: impl Fn(bool) + Send + Sync + 'static) -> Duo {
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
            log: Mutex::default(),
            closed: AtomicBool::new(false),
        });
        std::thread::spawn({
            let shared = shared.clone();
            move || shared.follow()
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
                match shared.next.lock().ok().and_then(|mut n| n.take()) {
                    Some(hinge) => shared.make(hinge),
                    None => {
                        shared.moving.store(false, Ordering::Release);
                        // one asked for just now, after the take and before this
                        let late = shared.next.lock().is_ok_and(|n| n.is_some());
                        if !late || shared.moving.swap(true, Ordering::AcqRel) {
                            break;
                        }
                    }
                }
            }
            shared.feed.change_fold(|f| f.busy = false);
            (shared.wake)();
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
        if let Ok(mut helper) = self.0.helper.lock() {
            helper.take();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn springboard_settles() {
        let line = "2026-11-05 13:14:15.163 Df SpringBoard[41966:44f05] [com.apple.SpringBoard:DisplayContentMode] COMPLETED:     .cover:pri .inner:off | 0.0° (0.00 fold:0.00): fullyClosed [device:Pu]";
        assert_eq!(settled(line), Some((true, 0.)));
        let line = "… COMPLETED:     .cover:aux .inner:pri | 180.0° (1.00 fold:1.00): openFlat [device:Pu]";
        assert_eq!(settled(line), Some((false, 180.)));
        assert_eq!(settled("STARTING: .cover:aux→pri .inner:pri→off | 0.0°"), None);
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
        assert_eq!(commands(Hinge::Stand(Stand::Laptop)), ["orientation landscape-left", "angle 90"]);
        assert_eq!(commands(Hinge::Angle(200.)), ["orientation portrait", "angle 180"]);
        assert_eq!(commands(Hinge::Hold("portrait_upside_down")), ["orientation pud"]);
    }
}
