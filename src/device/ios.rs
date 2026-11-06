//! iOS Simulators through `simdriver`: its helpers read the framebuffer's IOSurface and type into the digitizer, so
//! Shepherd never loads a private framework itself. It supports the Xcode it was built against (27.x) and says so
//! when started on another.
use super::tools::{self, Action, Settings, Toggle};
use super::{Button, Check, Device, Feed, Frame, Hinge, Platform, Probe, Touch, Wake, duo};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

pub(super) fn find() -> (Vec<Device>, Check) {
    let Ok(all) = simdriver::list_devices() else {
        let message = "Xcode and the iOS Simulator were not found. Install Xcode from the App Store.".into();
        return (Vec::new(), Check { ready: false, message });
    };
    let devices: Vec<Device> = all
        .into_iter()
        .filter(|d| d.runtime.starts_with("iOS"))
        .map(|d| Device { platform: Platform::Ios, version: version(&d.runtime), running: d.state == "Booted", duo: duo::is_duo(&d.device_type), usb: false, id: d.udid, name: d.name })
        .collect();
    let check = match devices.is_empty() {
        true => Check { ready: false, message: "Xcode is installed, but no iOS Simulator is available. Install a runtime in Xcode Settings → Components.".into() },
        false => Check { ready: true, message: "Xcode and iOS Simulator are available.".into() },
    };
    (devices, check)
}

/// `iOS-27-0` → `iOS 27.0`.
fn version(runtime: &str) -> String {
    runtime.replacen('-', " ", 1).replace('-', ".")
}

pub(super) fn dark(udid: &str) -> Option<bool> {
    let out = crate::process::command("xcrun").args(["simctl", "ui", udid, "appearance"]).output().ok()?;
    match String::from_utf8_lossy(&out.stdout).trim() {
        "dark" => Some(true),
        "light" => Some(false),
        _ => None,
    }
}

pub(super) fn screenshot(udid: &str, to: &Path) -> anyhow::Result<()> {
    let out = crate::process::command("xcrun").args(["simctl", "io", udid, "screenshot"]).arg(to).output()?;
    crate::process::answer(out).map(drop)
}

/// `xcrun simctl <verb> <udid> <rest…>`: what it printed.
fn simctl(udid: &str, verb: &str, rest: &[&str]) -> anyhow::Result<String> {
    let out = crate::process::command("xcrun").args(["simctl", verb, udid]).args(rest).output()?;
    crate::process::answer(out)
}

/// sdaxsettings' keys for the toggles it sets.
fn ax_key(toggle: Toggle) -> Option<&'static str> {
    Some(match toggle {
        Toggle::ReduceMotion => "reduce-motion",
        Toggle::ReduceTransparency => "reduce-transparency",
        Toggle::ShowBorders => "show-borders",
        Toggle::VoiceOver => "voiceover",
        Toggle::IncreaseContrast | Toggle::Network => return None,
    })
}

/// The settings, each read at once: simctl takes a second a call.
pub(super) fn read_tools(device: &Device, probe: &Probe) -> Settings {
    let udid = device.id.as_str();
    let ui = |option: &str| simctl(udid, "ui", &[option]).ok().map(|o| o.trim().to_lowercase());
    let (appearance, size, contrast, ax, app) = std::thread::scope(|s| {
        let appearance = s.spawn(|| ui("appearance"));
        let size = s.spawn(|| ui("content_size"));
        let contrast = s.spawn(|| ui("increase_contrast"));
        let ax = s.spawn(|| simdriver::ax_settings(udid, &["status"]).ok());
        let app = s.spawn(|| front_app(udid, probe));
        let join = |h: std::thread::ScopedJoinHandle<'_, Option<String>>| h.join().ok().flatten();
        (join(appearance), join(size), join(contrast), join(ax), join(app))
    });
    let ax: std::collections::HashMap<String, String> = ax.and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default();
    let mut toggles = Vec::new();
    for toggle in [Toggle::ReduceMotion, Toggle::IncreaseContrast, Toggle::ReduceTransparency, Toggle::ShowBorders, Toggle::VoiceOver] {
        let on = match ax_key(toggle) {
            Some(key) => ax.get(key).and_then(|v| (v == "on" || v == "off").then(|| v == "on")),
            None => contrast.as_deref().and_then(|c| (c == "enabled" || c == "disabled").then(|| c == "enabled")),
        };
        toggles.extend(on.map(|on| (toggle, on)));
    }
    // Liquid Glass came with iOS 26: before it the setting reads but does nothing
    let glass = device.version.trim_start_matches("iOS ").split('.').next().and_then(|m| m.parse::<u32>().ok()).is_some_and(|m| m >= 26);
    Settings {
        dark: appearance.and_then(|a| (a == "dark" || a == "light").then(|| a == "dark")),
        text_size: size.and_then(|s| tools::text_size_from_ios(&s)),
        toggles,
        liquid_glass: ax.get("liquid-glass").filter(|_| glass).and_then(|v| tools::known(&tools::LIQUID_GLASS, v)),
        color_filter: ax.get("color-filter").and_then(|v| tools::known(&tools::COLOR_FILTERS, v)),
        app,
    }
}

/// The app in front, by bundle id: the accessibility tree names it as the home screen shows it, and the installed
/// apps say whose name that is.
// ponytail: two apps of one name read as the first; the app's pid from sdax would tell them apart
fn front_app(udid: &str, probe: &Probe) -> Option<String> {
    let name = probe.driver.as_ref()?.accessibility().ok()?.label?;
    type Apps = std::collections::HashMap<String, (std::time::Instant, Vec<simdriver::App>)>;
    static APPS: Mutex<Option<Apps>> = Mutex::new(None);
    let find = |apps: &[simdriver::App]| apps.iter().find(|a| a.name == name).map(|a| a.bundle_id.clone());
    let mut cache = APPS.lock().ok()?;
    let cache = cache.get_or_insert_default();
    if let Some((listed, apps)) = cache.get(udid) {
        // an app not listed may be newly installed: listed again, at most every half minute
        if let Some(id) = find(apps) {
            return Some(id);
        }
        if listed.elapsed() < std::time::Duration::from_secs(30) {
            return None;
        }
    }
    let apps = simdriver::list_apps(udid).ok()?;
    let id = find(&apps);
    cache.insert(udid.to_string(), (std::time::Instant::now(), apps));
    id
}

pub(super) fn act(udid: &str, action: &Action) -> anyhow::Result<()> {
    let ax = |key: &str, value: &str| simdriver::ax_settings(udid, &["set", key, value]).map(drop).map_err(anyhow::Error::from);
    match action {
        Action::Dark(dark) => simdriver::set_appearance(udid, if *dark { simdriver::Appearance::Dark } else { simdriver::Appearance::Light })?,
        Action::TextSize(size) => simdriver::set_content_size(udid, tools::ios_text_size(*size))?,
        Action::Toggle(Toggle::IncreaseContrast, on) => simctl(udid, "ui", &["increase_contrast", if *on { "enabled" } else { "disabled" }]).map(drop)?,
        Action::Toggle(toggle, on) => ax(ax_key(*toggle).ok_or_else(|| anyhow::anyhow!("{} is Android's", toggle.label()))?, if *on { "on" } else { "off" })?,
        Action::LiquidGlass(look) => ax("liquid-glass", look)?,
        Action::ColorFilter(filter) => ax("color-filter", filter)?,
        Action::Location(lat, lon) => simdriver::set_location(udid, *lat, *lon)?,
        Action::ClearLocation => simctl(udid, "location", &["clear"]).map(drop)?,
        Action::Permission { app, permission, decision } => simctl(udid, "privacy", &[decision.verb(), permission, app]).map(drop)?,
        Action::OpenUrl(url) => simdriver::open_url(udid, url)?,
        Action::Launch(app) => simdriver::launch(udid, app)?,
        Action::Terminate(app) => simdriver::terminate(udid, app)?,
        Action::Push { app, text } => {
            let payload = serde_json::json!({ "aps": { "alert": text } }).to_string();
            let mut child = crate::process::command("xcrun")
                .args(["simctl", "push", udid, app, "-"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                std::io::Write::write_all(&mut stdin, payload.as_bytes())?;
            }
            crate::process::answer(child.wait_with_output()?).map(drop)?
        }
        Action::Orientation(_) => anyhow::bail!("A simulator turns through its open screen"),
    }
    Ok(())
}

pub(super) fn elements(probe: &Probe) -> anyhow::Result<Vec<[f64; 4]>> {
    let driver = probe.driver.as_ref().ok_or_else(|| anyhow::anyhow!("The device is not open"))?;
    let root = driver.accessibility()?;
    let (w, h) = driver.screen_points().ok_or_else(|| anyhow::anyhow!("The screen's size is not known yet"))?;
    // the app itself covers the screen: its elements are what's outlined
    Ok(root.descendants().into_iter().skip(1).map(|n| [n.frame[0] / w, n.frame[1] / h, n.frame[2] / w, n.frame[3] / h]).collect())
}

fn options(udid: &str) -> simdriver::Options {
    simdriver::Options { device: simdriver::DeviceSelector::Udid(udid.into()), ..Default::default() }
}

/// simdriver's frames are BGRA already, which is what GPUI draws. The Duo's inner screen comes turned a quarter.
fn frame(f: simdriver::Frame, screen: Option<u8>) -> Frame {
    Frame::new(f.width, f.height, f.data, false, screen, u8::from(screen == Some(duo::screen(false))))
}

/// A phone's frame, turned as it's held (`turn` quarters): its framebuffer stays upright when it's turned, as
/// Simulator.app's does (it turns its window instead), unless it's turned already (t3code's way).
fn held(f: simdriver::Frame, turn: u8) -> Frame {
    let turn = if f.width > f.height { 0 } else { turn };
    let (pixels, w, h) = Frame::turn_pixels(&f.data, f.width, f.height, 4, turn);
    Frame::new(w, h, pixels, false, None, turn)
}

/// Streams `options`' screen; each frame to `on_frame`, and `on_back` once it's live again after a restart.
fn start(
    options: simdriver::Options,
    feed: Arc<Feed>,
    wake: Wake,
    on_frame: impl Fn(simdriver::Frame) + Send + Sync + 'static,
    on_back: impl Fn() + Send + Sync + 'static,
) -> anyhow::Result<simdriver::Driver> {
    let restarting = AtomicBool::new(false);
    Ok(simdriver::Driver::start(options, move |event| match event {
        simdriver::Event::Frame(f) => on_frame(f),
        simdriver::Event::State(simdriver::State::Restarting) => {
            restarting.store(true, Ordering::Relaxed);
            feed.fail("The simulator restarted; reconnecting…".into());
            wake();
        }
        simdriver::Event::State(simdriver::State::Live) if restarting.swap(false, Ordering::Relaxed) => on_back(),
        _ => {}
    })?)
}

/// How long a screen switched to may stay dark: a closed Duo's cover lights some seconds after the fold.
const LIGHTING: std::time::Duration = std::time::Duration::from_secs(10);

/// All but black: a screen not lit yet. Sampled, a few dozen pixels across it.
// ponytail: an app's own all-black screen reads blank too, so a hidden screen keeps an older frame of it, and a screen
// just switched to shows it only after LIGHTING; telling them apart needs the display's power state, which simctl
// doesn't give (both read On)
fn blank(frame: &Frame) -> bool {
    let (pixels, _) = frame.pixels.as_chunks::<4>();
    let step = (pixels.len() / 64).max(1);
    pixels.iter().step_by(step).all(|p| p[0] < 8 && p[1] < 8 && p[2] < 8)
}

/// iPhone Duo's two screens, as serve-sim's: each streamed all along, so a fold shows the other one's latest frame at
/// once, and its own as soon as it lights (a stream follows its screen from dark to lit).
struct Panels {
    cover: AtomicBool,
    /// Each screen's latest lit frame: the cover's, the inner one's.
    last: Mutex<[Option<Arc<Frame>>; 2]>,
    /// Both screens' streams, for as long as the session.
    streams: Mutex<Vec<simdriver::FrameStream>>,
    /// When the live screen was switched to, until it lights: it can be dark a few seconds more (a fold's screen
    /// lights late).
    shown_at: Mutex<Option<std::time::Instant>>,
    /// Why a screen's stream didn't start: said over the other one's frames, which would clear it.
    broken: Mutex<Option<String>>,
    udid: String,
    feed: Arc<Feed>,
    wake: Wake,
}

impl Panels {
    fn open(udid: &str, cover: bool, feed: Arc<Feed>, wake: Wake) -> anyhow::Result<Arc<Panels>> {
        let panels = Arc::new(Panels {
            cover: AtomicBool::new(cover),
            last: Mutex::default(),
            streams: Mutex::default(),
            shown_at: Mutex::new(Some(std::time::Instant::now())),
            broken: Mutex::default(),
            udid: udid.into(),
            feed,
            wake,
        });
        panels.stream()?;
        Ok(panels)
    }

    /// Starts both screens' streams, anew after the simulator restarted: they aren't restarted with it.
    fn stream(self: &Arc<Self>) -> anyhow::Result<()> {
        let udid = &self.udid;
        let mut streams = Vec::new();
        for cover in [true, false] {
            let options = match cover {
                true => simdriver::Options { screen_size: Some(duo::COVER), ..options(udid) },
                // mounted sideways
                false => simdriver::Options { screen_size: Some(duo::INNER), rotate_cw: true, ..options(udid) },
            };
            // weak: the streams' callbacks must not keep their own owner alive
            let p = Arc::downgrade(self);
            let started = simdriver::FrameStream::start(&options, move |f| {
                if let Some(p) = p.upgrade() {
                    p.frame(cover, frame(f, Some(duo::screen(cover))));
                }
            });
            streams.push(started.map_err(|e| anyhow::anyhow!("The {} screen's video did not start: {e:#}", if cover { "cover" } else { "inner" }))?);
        }
        if let Ok(mut old) = self.streams.lock() {
            *old = streams;
        }
        Ok(())
    }

    /// Back after the simulator restarted: both streams anew, or why not, kept on screen.
    fn restart(self: &Arc<Self>) {
        let broken = self.stream().err().map(|e| format!("{e:#}"));
        if let Some(why) = &broken {
            self.feed.fail(why.clone());
            (self.wake)();
        }
        if let Ok(mut b) = self.broken.lock() {
            *b = broken;
        }
    }

    fn frame(&self, cover: bool, frame: Frame) {
        let shown = self.cover.load(Ordering::Relaxed) == cover;
        let dark = blank(&frame);
        // a dark screen keeps its last lit frame: one folded away, or one just switched to and not lit yet, as t3code's
        // viewer does
        let lighting = !shown
            || self.shown_at.lock().is_ok_and(|mut at| {
                if !dark {
                    // lit: its own frames from now on, black or not
                    at.take();
                }
                at.is_some_and(|at| at.elapsed() < LIGHTING)
            });
        let frame = Arc::new(frame);
        if let Ok(mut last) = self.last.lock() {
            let slot = &mut last[usize::from(!cover)];
            if slot.is_some() && lighting && dark {
                return;
            }
            *slot = Some(frame.clone());
        }
        if shown {
            self.feed.put_shared(frame);
            if let Some(why) = self.broken.lock().ok().and_then(|b| b.clone()) {
                self.feed.fail(why);
            }
            (self.wake)();
        }
    }

    /// Shows the cover (or the inner screen): its last frame now, its own as they come.
    fn show(&self, cover: bool) {
        self.cover.store(cover, Ordering::Relaxed);
        if let Ok(mut at) = self.shown_at.lock() {
            *at = Some(std::time::Instant::now());
        }
        if let Some(frame) = self.last.lock().ok().and_then(|last| last[usize::from(!cover)].clone()) {
            self.feed.put_shared(frame);
            (self.wake)();
        }
    }
}

pub struct Session {
    /// The device's input, and its only screen's frames; an iPhone Duo's frames are its `Panels`'.
    pub(super) driver: Arc<simdriver::Driver>,
    /// An iPhone Duo's hinge, and its screens.
    duo: Option<(duo::Duo, Arc<Panels>)>,
    /// How a phone's held, in quarter turns (`tools::ORIENTATIONS`' order), which its frames are turned by.
    turn: Arc<AtomicU8>,
    feed: Arc<Feed>,
    wake: Wake,
    /// When a mouse wheel last scrolled: its finger is still down until simdriver lifts it, idle.
    scrolled: Mutex<Option<std::time::Instant>>,
}

/// How long a wheel's finger stays down between its turns: under simdriver's idle lift (150 ms), so a turn never moves
/// a finger already lifted.
const WHEEL_HELD: std::time::Duration = std::time::Duration::from_millis(100);

impl Session {
    pub(super) fn open(device: &Device, feed: Arc<Feed>, wake: Wake) -> anyhow::Result<Session> {
        let udid = device.id.as_str();
        if !device.duo {
            // as it was left: Simulator.app, or an earlier session, may have turned it
            let held_as = duo::orientation(udid, true);
            let turn = Arc::new(AtomicU8::new(tools::ORIENTATIONS.iter().position(|(o, _)| *o == held_as).unwrap_or(0) as u8));
            let (f, w, t) = (feed.clone(), wake.clone(), turn.clone());
            let on_frame = move |fr| {
                f.put(held(fr, t.load(Ordering::Relaxed)));
                w();
            };
            let driver = start(options(udid), feed.clone(), wake.clone(), on_frame, || {})?;
            return Ok(Session { driver: Arc::new(driver), duo: None, turn, feed, wake, scrolled: Default::default() });
        }
        let last = duo::last_settled(udid, duo::RECENT);
        let cover = last.is_none_or(|l| l.0);
        // the panels, and the driver itself, once they're up: a restarted simulator restarts neither their streams
        // nor the screen touches go to
        let back = Arc::new(std::sync::OnceLock::<(std::sync::Weak<Panels>, std::sync::Weak<simdriver::Driver>)>::new());
        let on_back = {
            let back = back.clone();
            move || {
                let Some((p, d)) = back.get().and_then(|(p, d)| p.upgrade().zip(d.upgrade())) else { return };
                d.set_touch_target(duo::screen(p.cover.load(Ordering::Relaxed)).into());
                // off the driver's thread: starting a stream lists the devices
                std::thread::spawn(move || p.restart());
            }
        };
        // the driver's own frames go unused: they're the TV-out's, which nothing shows and no other stream watches (two
        // streams of one screen leave one dark)
        let input = simdriver::Options { touch_target: duo::screen(cover).into(), screen_size: Some(duo::TV_OUT), fps: 1, scale: 8, ..options(udid) };
        let driver = Arc::new(start(input, feed.clone(), wake.clone(), |_| {}, on_back)?);
        let panels = Panels::open(udid, cover, feed.clone(), wake.clone())?;
        let _ = back.set((Arc::downgrade(&panels), Arc::downgrade(&driver)));
        let (p, d) = (panels.clone(), driver.clone());
        let duo = duo::Duo::open(udid, last, feed.clone(), wake.clone(), move |cover| {
            p.show(cover);
            d.set_touch_target(duo::screen(cover).into());
        });
        Ok(Session { driver, duo: Some((duo, panels)), turn: Default::default(), feed, wake, scrolled: Default::default() })
    }

    fn driver(&self) -> &simdriver::Driver {
        &self.driver
    }

    pub(super) fn hinge(&self, hinge: Hinge) {
        if let Some((duo, _)) = &self.duo {
            duo.hinge(hinge);
        }
    }

    /// Turns it to be held as `orientation` says (`tools::ORIENTATIONS`' names); an iPhone Duo as its hinge does.
    pub(super) fn rotate(&self, orientation: &'static str) {
        if let Some((duo, _)) = &self.duo {
            return duo.hinge(Hinge::Hold(orientation));
        }
        let turn = tools::ORIENTATIONS.iter().position(|(o, _)| *o == orientation).unwrap_or(0);
        // UIDeviceOrientation's: portrait, landscape right, upside down, landscape left
        self.driver().set_orientation([1, 3, 2, 4][turn]);
        self.turn.store(turn as u8, Ordering::Relaxed);
        // a screen that doesn't turn with it (an iPhone's home screen) sends no new frame: its last one turns now
        if let Some(last) = self.feed.latest().filter(|f| f.turn != turn as u8 && (f.turn != 0 || f.width < f.height)) {
            self.feed.put(last.turned(turn as u8));
            (self.wake)();
        }
    }

    pub(super) fn touch(&self, phase: Touch, x: f64, y: f64) {
        let phase = match phase {
            Touch::Down => simdriver::Phase::Down,
            Touch::Move => simdriver::Phase::Move,
            Touch::Up => simdriver::Phase::Up,
        };
        self.driver().pointer(phase, x, y, Default::default());
    }

    pub(super) fn scroll(&self, phase: gpui::TouchPhase, x: f64, y: f64, dx: f64, dy: f64, wheel: bool) {
        if wheel {
            // a mouse wheel's turns come with no start or end: each puts a finger down unless one still is, and
            // simdriver lifts it once the wheel stops
            let now = std::time::Instant::now();
            let held = self.scrolled.lock().ok().and_then(|mut at| at.replace(now)).is_some_and(|at| now - at < WHEEL_HELD);
            if !held {
                self.driver().scroll(simdriver::ScrollPhase::Began, x, y, 0., 0.);
            }
            self.driver().scroll(simdriver::ScrollPhase::Changed, x, y, dx, dy);
            return;
        }
        let phase = match phase {
            gpui::TouchPhase::Started => simdriver::ScrollPhase::Began,
            gpui::TouchPhase::Moved => simdriver::ScrollPhase::Changed,
            gpui::TouchPhase::Ended | gpui::TouchPhase::Cancelled => simdriver::ScrollPhase::Ended,
        };
        self.driver().scroll(phase, x, y, dx, dy);
    }

    pub(super) fn button(&self, button: Button) {
        if button == Button::Home {
            self.driver().button(simdriver::Button::Home, 1, 0.);
        }
    }

    pub(super) fn key(&self, name: &str) -> bool {
        self.driver().key(name)
    }

    pub(super) fn text(&self, text: &str) {
        self.driver().text(text);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // its hinge first, then its screens' streams
        if let Some((duo, panels)) = self.duo.take() {
            drop(duo);
            panels.streams.lock().map(|mut s| s.clear()).ok();
        }
        self.driver().stop();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dark_screens_read_blank() {
        let frame = |px: [u8; 4]| super::Frame::new(10, 10, px.repeat(100), false, Some(1), 0);
        assert!(super::blank(&frame([0, 0, 0, 255])));
        assert!(!super::blank(&frame([200, 200, 200, 255])));
    }

    #[test]
    fn a_screen_switched_to_keeps_its_last_frame_only_until_it_lights() {
        use super::*;
        let frame = |v: u8| Frame::new(10, 10, [v, v, v, 255].repeat(100), false, Some(1), 0);
        let feed = Arc::new(Feed::default());
        let panels = Panels {
            cover: AtomicBool::new(true),
            last: Mutex::default(),
            streams: Mutex::default(),
            shown_at: Mutex::default(),
            broken: Mutex::default(),
            udid: String::new(),
            feed: feed.clone(),
            wake: Arc::new(|| {}),
        };
        panels.frame(true, frame(200));
        panels.show(true);
        // dark, just switched to: the last lit frame stays
        panels.frame(true, frame(0));
        assert!(feed.newer(0).is_some_and(|(n, f)| n == 2 && !blank(&f)));
        // lit: from now on black is its own
        panels.frame(true, frame(200));
        panels.frame(true, frame(0));
        assert!(feed.newer(0).is_some_and(|(n, f)| n == 4 && blank(&f)));
    }

    #[test]
    fn runtime_versions_read_as_xcode_shows_them() {
        assert_eq!(super::version("iOS-27-0"), "iOS 27.0");
        assert_eq!(super::version("iOS-18-6"), "iOS 18.6");
    }
}
