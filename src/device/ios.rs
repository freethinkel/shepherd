//! iOS Simulators through `simdriver`: its helpers read the framebuffer's IOSurface and type into the digitizer, so
//! Shepherd never loads a private framework itself. It supports the Xcode it was built against (27.x) and says so
//! when started on another.
use super::{Button, Check, Device, Feed, Frame, Hinge, Platform, Touch, Wake, duo};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub(super) fn find() -> (Vec<Device>, Check) {
    let Ok(all) = simdriver::list_devices() else {
        let message = "Xcode and the iOS Simulator were not found. Install Xcode from the App Store.".into();
        return (Vec::new(), Check { ready: false, message });
    };
    let devices: Vec<Device> = all
        .into_iter()
        .filter(|d| d.runtime.starts_with("iOS"))
        .map(|d| Device { platform: Platform::Ios, version: version(&d.runtime), running: d.state == "Booted", id: d.udid, name: d.name })
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

/// Streams `udid` into `feed`. An iPhone Duo's `cover` (`Some(true)`) or inner screen (`Some(false)`), else its only one.
fn start(udid: &str, cover: Option<bool>, feed: Arc<Feed>, wake: Wake) -> anyhow::Result<simdriver::Driver> {
    let options = simdriver::Options {
        device: simdriver::DeviceSelector::Udid(udid.into()),
        screen_size: cover.filter(|c| *c).map(|_| duo::COVER),
        touch_target: cover.map_or(0, |c| duo::screen(c).into()),
        // the inner screen is mounted sideways
        rotate_cw: cover == Some(false),
        ..Default::default()
    };
    let screen = cover.map(duo::screen);
    Ok(simdriver::Driver::start(options, move |event| match event {
        simdriver::Event::Frame(f) => {
            // simdriver's frames are BGRA already, which is what GPUI draws
            feed.put(Frame { width: f.width, height: f.height, pixels: f.data, rgb: false, screen });
            wake();
        }
        simdriver::Event::State(simdriver::State::Restarting) => {
            feed.fail("The simulator restarted; reconnecting…".into());
            wake();
        }
        _ => {}
    })?)
}

pub struct Session {
    /// An iPhone Duo's is started over on the other screen when the fold hands over to it.
    driver: Arc<Mutex<simdriver::Driver>>,
    duo: Option<duo::Duo>,
}

impl Session {
    pub(super) fn open(udid: &str, feed: Arc<Feed>, wake: Wake) -> anyhow::Result<Session> {
        if !duo::is_duo(udid) {
            let driver = start(udid, None, feed, wake)?;
            return Ok(Session { driver: Arc::new(Mutex::new(driver)), duo: None });
        }
        let last = duo::last_settled(udid);
        let driver = Arc::new(Mutex::new(start(udid, Some(last.0), feed.clone(), wake.clone())?));
        let show = {
            let (udid, driver, feed, wake) = (udid.to_string(), driver.clone(), feed.clone(), wake.clone());
            move |cover| match start(&udid, Some(cover), feed.clone(), wake.clone()) {
                // starting one stops the old one, whose helpers let go of the device first
                Ok(new) => drop(driver.lock().map(|mut d| std::mem::replace(&mut *d, new))),
                Err(e) => {
                    feed.fail(format!("The other screen's video did not start: {e:#}"));
                    wake();
                }
            }
        };
        let duo = duo::Duo::open(udid, last, feed, wake, show);
        Ok(Session { driver, duo: Some(duo) })
    }

    fn driver(&self) -> std::sync::MutexGuard<'_, simdriver::Driver> {
        self.driver.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(super) fn hinge(&self, hinge: Hinge) {
        if let Some(duo) = &self.duo {
            duo.hinge(hinge);
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

    pub(super) fn scroll(&self, phase: gpui::TouchPhase, x: f64, y: f64, dx: f64, dy: f64) {
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
        // its hinge first: a fold that lands now must not start the stream again
        self.duo.take();
        self.driver().stop();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_versions_read_as_xcode_shows_them() {
        assert_eq!(super::version("iOS-27-0"), "iOS 27.0");
        assert_eq!(super::version("iOS-18-6"), "iOS 18.6");
    }
}
