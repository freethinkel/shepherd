//! Simulators and emulators for the right panel's Device tabs: t3code's Device panel without its Node hub. iOS goes
//! through `simdriver` (the simulator's own framebuffer and digitizer, macOS only), Android through the emulator's gRPC,
//! and an Android phone on USB through scrcpy (macOS only).
//! What here is plain and blocking; the tabs are the Shell's (`shell/device.rs`).
use std::path::Path;
use std::sync::{Arc, Mutex};

mod android;
#[cfg(target_os = "macos")]
mod duo;
#[cfg(target_os = "macos")]
mod ios;
pub mod models;
#[cfg(target_os = "macos")]
mod scrcpy;
#[cfg(target_os = "macos")]
pub mod surface;
pub mod tools;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    Ios,
    Android,
}

impl Platform {
    fn tag(self) -> &'static str {
        match self {
            Platform::Ios => "ios",
            Platform::Android => "android",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub platform: Platform,
    /// The simulator's UDID, the virtual device's AVD id, or a phone's adb serial.
    pub id: String,
    pub name: String,
    /// `iOS 27.0`, `API 36`.
    pub version: String,
    pub running: bool,
    /// iPhone Duo, whose session follows its hinge and two screens.
    pub duo: bool,
    /// A real phone, on USB: always running, never powered off from here.
    pub usb: bool,
}

/// A Device tab's key starts with this; the bare prefix is the tab that lists them.
pub const KEY: &str = "device";

impl Device {
    /// Its tab's key: `device:ios:<udid>`.
    pub fn key(&self) -> String {
        format!("{KEY}:{}:{}", self.platform.tag(), self.id)
    }
}

/// Whether a platform can run here, and what to tell the user either way: t3code's `platformSetupStatus`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    pub ready: bool,
    pub message: String,
}

/// One look at the machine: its devices and both platforms' checks.
#[derive(Clone, Debug, Default)]
pub struct Found {
    pub devices: Vec<Device>,
    pub ios: Option<Check>,
    pub android: Option<Check>,
}

/// Lists the devices: running first, then by name, iOS before Android.
pub fn find() -> Found {
    let (ios_devices, ios) = find_ios();
    let (android_devices, android) = android::find();
    let mut devices: Vec<Device> = ios_devices.into_iter().chain(android_devices).collect();
    devices.sort_by(|a, b| (a.platform.tag() != "ios", !a.running, &a.name).cmp(&(b.platform.tag() != "ios", !b.running, &b.name)));
    Found { devices, ios: Some(ios), android: Some(android) }
}

#[cfg(target_os = "macos")]
fn find_ios() -> (Vec<Device>, Check) {
    ios::find()
}

#[cfg(not(target_os = "macos"))]
fn find_ios() -> (Vec<Device>, Check) {
    (Vec::new(), Check { ready: false, message: "iOS Simulators need macOS with Xcode.".into() })
}

/// Starts `device` and waits until it's up. Already running is success.
pub fn boot(device: &Device) -> anyhow::Result<()> {
    if device.usb {
        return Ok(());
    }
    match device.platform {
        #[cfg(target_os = "macos")]
        Platform::Ios => Ok(simdriver::boot(&device.id)?),
        Platform::Android => android::boot(&device.id),
        #[allow(unreachable_patterns)]
        _ => anyhow::bail!("iOS Simulators need macOS"),
    }
}

/// Powers `device` off.
pub fn power_off(device: &Device) -> anyhow::Result<()> {
    anyhow::ensure!(!device.usb, "A phone is powered off on the phone");
    match device.platform {
        #[cfg(target_os = "macos")]
        Platform::Ios => Ok(simdriver::shutdown(&device.id)?),
        Platform::Android => android::power_off(&device.id),
        #[allow(unreachable_patterns)]
        _ => anyhow::bail!("iOS Simulators need macOS"),
    }
}

/// Whether `device` is in dark mode; `None` when it can't say.
pub fn dark(device: &Device) -> Option<bool> {
    match device.platform {
        #[cfg(target_os = "macos")]
        Platform::Ios => ios::dark(&device.id),
        Platform::Android => android::dark(&device.id),
        #[allow(unreachable_patterns)]
        _ => None,
    }
}

pub fn set_dark(device: &Device, dark: bool) -> anyhow::Result<()> {
    match device.platform {
        #[cfg(target_os = "macos")]
        Platform::Ios => Ok(simdriver::set_appearance(&device.id, if dark { simdriver::Appearance::Dark } else { simdriver::Appearance::Light })?),
        Platform::Android => android::set_dark(&device.id, dark),
        #[allow(unreachable_patterns)]
        _ => anyhow::bail!("iOS Simulators need macOS"),
    }
}

/// A full-resolution PNG of the screen, saved at `to`.
pub fn screenshot(device: &Device, to: &Path) -> anyhow::Result<()> {
    match device.platform {
        #[cfg(target_os = "macos")]
        Platform::Ios => ios::screenshot(&device.id, to),
        Platform::Android => android::screenshot(&device.id, to),
        #[allow(unreachable_patterns)]
        _ => anyhow::bail!("iOS Simulators need macOS"),
    }
}

/// A screen's frame as it came: BGRA from a simulator, RGB from an emulator.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pixels: Vec<u8>,
    rgb: bool,
    /// iPhone Duo's screen it's from: 1 the cover, 3 the inner one.
    pub screen: Option<u8>,
    /// Quarter turns clockwise it shows the device's own screen turned, as it's held (`tools::ORIENTATIONS`' order): a
    /// turned emulator's come so, a turned simulator's are turned here, and the Duo's inner screen, mounted a quarter
    /// turn from the phone, comes turned once. Touches on it turn back (`device_point`).
    pub turn: u8,
    /// The same pixels in an IOSurface, which the window draws without a copy; `None` when Core Video gave none, and
    /// the frame is drawn as an image.
    #[cfg(target_os = "macos")]
    pub surface: Option<surface::Surface>,
}

impl Frame {
    /// A frame decoded into `surface` already (a phone's), with its BGRA `pixels`. scrcpy streams the phone's screen as
    /// it's held and takes touches in that frame, so it's never turned here.
    #[cfg(target_os = "macos")]
    fn decoded(width: u32, height: u32, pixels: Vec<u8>, surface: surface::Surface) -> Frame {
        Frame { width, height, pixels, rgb: false, screen: None, turn: 0, surface: Some(surface) }
    }

    /// A frame as it came, `turn` quarters from the device's own screen, its surface made here, on the stream's thread.
    pub fn new(width: u32, height: u32, pixels: Vec<u8>, rgb: bool, screen: Option<u8>, turn: u8) -> Frame {
        #[cfg(target_os = "macos")]
        let surface = surface::Surface::new(width, height, &pixels, rgb);
        Frame {
            width,
            height,
            pixels,
            rgb,
            screen,
            turn: turn % 4,
            #[cfg(target_os = "macos")]
            surface,
        }
    }

    /// Its pixels turned a quarter clockwise `turn` times (each pixel `bpp` bytes), and their new width and height.
    pub fn turn_pixels(pixels: &[u8], width: u32, height: u32, bpp: usize, turn: u8) -> (Vec<u8>, u32, u32) {
        let (w, h) = (width as usize, height as usize);
        let turn = turn % 4;
        if turn == 0 || pixels.len() < w * h * bpp {
            return (pixels.to_vec(), width, height);
        }
        let mut out = vec![0; w * h * bpp];
        for y in 0..h {
            for x in 0..w {
                let to = match turn {
                    1 => x * h + (h - 1 - y),
                    2 => (h - 1 - y) * w + (w - 1 - x),
                    _ => (w - 1 - x) * h + y,
                };
                out[to * bpp..(to + 1) * bpp].copy_from_slice(&pixels[(y * w + x) * bpp..(y * w + x + 1) * bpp]);
            }
        }
        if turn == 2 { (out, width, height) } else { (out, height, width) }
    }

    /// The same screen shown turned `turn` quarters instead.
    pub fn turned(&self, turn: u8) -> Frame {
        let bpp = if self.rgb { 3 } else { 4 };
        let (pixels, w, h) = Frame::turn_pixels(&self.pixels, self.width, self.height, bpp, (turn + 4 - self.turn) % 4);
        Frame::new(w, h, pixels, self.rgb, self.screen, turn)
    }

    /// Its pixels in BGRA, as GPUI draws an image: for a frame with no surface (Linux's). Converting happens where a
    /// frame is shown, for the latest only: converting each as it arrives falls behind a fast stream, which then
    /// queues up seconds of frames.
    pub fn bgra(&self) -> Vec<u8> {
        if !self.rgb {
            return self.pixels.clone();
        }
        let mut bgra = vec![255; self.pixels.len() / 3 * 4];
        for (o, p) in bgra.as_chunks_mut::<4>().0.iter_mut().zip(self.pixels.as_chunks::<3>().0) {
            (o[0], o[1], o[2]) = (p[2], p[1], p[0]);
        }
        bgra
    }

    /// Where a point on the frame (0 to 1 each way) is on the device's own screen.
    pub fn device_point(&self, x: f64, y: f64) -> (f64, f64) {
        match self.turn {
            1 => (y, 1. - x),
            2 => (1. - x, 1. - y),
            3 => (1. - y, x),
            _ => (x, y),
        }
    }

    /// Where a rect on the device's own screen (`[x, y, width, height]`, 0 to 1) shows on the frame: `device_point`'s
    /// way back.
    pub fn shown_rect(&self, [x, y, w, h]: [f64; 4]) -> [f64; 4] {
        match self.turn {
            1 => [1. - (y + h), x, h, w],
            2 => [1. - (x + w), 1. - (y + h), w, h],
            3 => [y, 1. - (x + w), h, w],
            _ => [x, y, w, h],
        }
    }

    /// A JPEG of it, for the 3D view's web page: encoded from the pixels as they came, no copy in between.
    pub fn jpeg(&self) -> anyhow::Result<Vec<u8>> {
        let mut out = Vec::new();
        let color = if self.rgb { jpeg_encoder::ColorType::Rgb } else { jpeg_encoder::ColorType::Bgra };
        jpeg_encoder::Encoder::new(&mut out, 85).encode(&self.pixels, u16::try_from(self.width)?, u16::try_from(self.height)?, color)?;
        Ok(out)
    }
}

/// What a session hands the window: its latest frame, numbered (older ones are dropped unseen), what went wrong, and
/// a foldable's hinge.
#[derive(Default)]
pub struct Feed {
    frame: Mutex<(u64, Option<Arc<Frame>>)>,
    arrived: std::sync::Condvar,
    error: Mutex<Option<String>>,
    fold: Mutex<Option<Fold>>,
}

impl Feed {
    /// The latest frame, when it's newer than frame `seen`.
    pub fn newer(&self, seen: u64) -> Option<(u64, Arc<Frame>)> {
        let f = self.frame.lock().ok()?;
        f.1.clone().filter(|_| f.0 > seen).map(|frame| (f.0, frame))
    }

    /// `newer`, waiting up to `timeout` for one.
    pub fn wait_newer(&self, seen: u64, timeout: std::time::Duration) -> Option<(u64, Arc<Frame>)> {
        let f = self.frame.lock().ok()?;
        let (f, _) = self.arrived.wait_timeout_while(f, timeout, |f| f.0 <= seen || f.1.is_none()).ok()?;
        f.1.clone().filter(|_| f.0 > seen).map(|frame| (f.0, frame))
    }

    /// The latest frame, seen or not.
    pub fn latest(&self) -> Option<Arc<Frame>> {
        self.newer(0).map(|(_, f)| f)
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().ok()?.clone()
    }

    fn put(&self, frame: Frame) {
        self.put_shared(Arc::new(frame));
    }

    fn put_shared(&self, frame: Arc<Frame>) {
        if let Ok(mut f) = self.frame.lock() {
            *f = (f.0 + 1, Some(frame));
        }
        self.arrived.notify_all();
        if let Ok(mut e) = self.error.lock() {
            e.take();
        }
    }

    fn fail(&self, why: String) {
        if let Ok(mut e) = self.error.lock() {
            *e = Some(why);
        }
    }

    /// A foldable's hinge; `None` for a phone that doesn't fold.
    pub fn fold(&self) -> Option<Fold> {
        self.fold.lock().ok()?.clone()
    }

    /// Changes the hinge's state, and numbers the latest frame anew so whoever waits for one hears of it.
    fn change_fold(&self, change: impl FnOnce(&mut Fold)) {
        if let Ok(mut fold) = self.fold.lock()
            && let Some(fold) = fold.as_mut()
        {
            change(fold);
        }
        if let Ok(mut f) = self.frame.lock()
            && f.1.is_some()
        {
            f.0 += 1;
        }
        self.arrived.notify_all();
    }
}

/// A foldable's hinge, as the device last told it.
#[derive(Clone, Debug, PartialEq)]
pub struct Fold {
    /// iPhone Duo, which takes any angle and stands; an Android foldable only folds and unfolds.
    pub duo: bool,
    /// 0 shut, 180 flat open.
    pub angle: f64,
    /// The cover screen is the live one.
    pub cover: bool,
    /// The stand it was set up in, until it's moved otherwise.
    pub stand: Option<Stand>,
    /// The live screen's UI orientation, by t3code's names: `portrait`, `landscape_left`…
    pub orientation: &'static str,
    /// A change on its way.
    pub busy: bool,
    /// Why the last change failed.
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stand {
    /// Half open on its side, the lower half flat.
    Laptop,
    /// Upside down on both edges, the cover outward.
    Tent,
}

/// What to do with the hinge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hinge {
    /// Open it this far, held upright.
    Angle(f64),
    Stand(Stand),
    /// iPhone Duo held the way the live screen's UI turns to, by t3code's names (`landscape_left`…): its 3D view
    /// turned to face another way.
    Hold(&'static str),
}

impl Fold {
    /// As the 3D view's page reads it.
    pub fn json(&self) -> String {
        let stand = match self.stand {
            Some(Stand::Laptop) => "\"laptop\"",
            Some(Stand::Tent) => "\"tent\"",
            None => "null",
        };
        format!(r#"{{"duo":{},"angle":{},"cover":{},"stand":{stand},"orientation":"{}"}}"#, self.duo, self.angle, self.cover, self.orientation)
    }
}

/// Called from the session's threads whenever the feed changed.
pub type Wake = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Touch {
    Down,
    Move,
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Home,
    /// Android's.
    Back,
    /// Android's.
    Recents,
}

/// What reads a session's device off the window's thread: an iOS session's driver, whose accessibility tree tells the
/// app in front and its elements. An emulator is read through adb, and needs none.
#[derive(Clone, Default)]
pub struct Probe {
    #[cfg(target_os = "macos")]
    driver: Option<Arc<simdriver::Driver>>,
}

/// A device's live screen and its input, until dropped.
pub enum Session {
    #[cfg(target_os = "macos")]
    Ios(ios::Session),
    Android(android::Session),
    /// A phone on USB.
    #[cfg(target_os = "macos")]
    Phone(scrcpy::Session),
}

impl Session {
    /// Streams `device`, which must be running, into `feed`. Blocks until the stream is up.
    pub fn open(device: &Device, rt: &tokio::runtime::Handle, feed: Arc<Feed>, wake: Wake) -> anyhow::Result<Session> {
        match device.platform {
            #[cfg(target_os = "macos")]
            Platform::Ios => Ok(Session::Ios(ios::Session::open(device, feed, wake)?)),
            #[cfg(target_os = "macos")]
            Platform::Android if device.usb => Ok(Session::Phone(scrcpy::Session::open(android::adb_path()?, &device.id, feed, wake)?)),
            Platform::Android => Ok(Session::Android(android::Session::open(&device.id, rt, feed, wake)?)),
            #[allow(unreachable_patterns)]
            _ => anyhow::bail!("iOS Simulators need macOS"),
        }
    }

    pub fn probe(&self) -> Probe {
        match self {
            #[cfg(target_os = "macos")]
            Session::Ios(s) => Probe { driver: Some(s.driver.clone()) },
            Session::Android(_) => Probe::default(),
            #[cfg(target_os = "macos")]
            Session::Phone(_) => Probe::default(),
        }
    }

    /// A finger at `x`, `y`, from 0 to 1 across the screen.
    pub fn touch(&self, phase: Touch, x: f64, y: f64) {
        match self {
            #[cfg(target_os = "macos")]
            Session::Ios(s) => s.touch(phase, x, y),
            Session::Android(s) => s.touch(phase, x, y),
            #[cfg(target_os = "macos")]
            Session::Phone(s) => s.touch(phase, x, y),
        }
    }

    /// A trackpad's scroll (or a mouse `wheel`'s, which has no phases) over `x`, `y`, by `dx`, `dy` points; iOS only,
    /// Android scrolls by dragging.
    pub fn scroll(&self, phase: gpui::TouchPhase, x: f64, y: f64, dx: f64, dy: f64, wheel: bool) {
        #[cfg(target_os = "macos")]
        if let Session::Ios(s) = self {
            s.scroll(phase, x, y, dx, dy, wheel);
        }
        let _ = (phase, x, y, dx, dy, wheel);
    }

    pub fn button(&self, button: Button) {
        match self {
            #[cfg(target_os = "macos")]
            Session::Ios(s) => s.button(button),
            Session::Android(s) => s.button(button),
            #[cfg(target_os = "macos")]
            Session::Phone(s) => s.button(button),
        }
    }

    /// A key by GPUI's name (`enter`, `backspace`, `left`…); false when the device has no such key.
    pub fn key(&self, name: &str) -> bool {
        match self {
            #[cfg(target_os = "macos")]
            Session::Ios(s) => s.key(name),
            Session::Android(s) => s.key(name),
            #[cfg(target_os = "macos")]
            Session::Phone(s) => s.key(name),
        }
    }

    pub fn text(&self, text: &str) {
        match self {
            #[cfg(target_os = "macos")]
            Session::Ios(s) => s.text(text),
            Session::Android(s) => s.text(text),
            #[cfg(target_os = "macos")]
            Session::Phone(s) => s.text(text),
        }
    }

    /// Turns a simulator to be held as `orientation` says (`tools::ORIENTATIONS`' names); its frames turn with it. An
    /// emulator turns through `tools::act`, off the window's thread.
    pub fn rotate(&self, orientation: &'static str) {
        #[cfg(target_os = "macos")]
        if let Session::Ios(s) = self {
            s.rotate(orientation);
        }
        let _ = orientation;
    }

    /// Moves a foldable's hinge (`Feed::fold` says whether it has one); its feed tells how it went.
    pub fn hinge(&self, hinge: Hinge) {
        match self {
            #[cfg(target_os = "macos")]
            Session::Ios(s) => s.hinge(hinge),
            Session::Android(s) => s.hinge(hinge),
            // a phone that folds is folded by hand
            #[cfg(target_os = "macos")]
            Session::Phone(_) => {}
        }
    }
}

/// GPUI's key names the devices take, and their W3C spelling (Android's emulator and simdriver both read it).
pub fn w3c_key(name: &str) -> Option<&'static str> {
    Some(match name {
        "enter" => "Enter",
        "backspace" => "Backspace",
        "delete" => "Delete",
        "tab" => "Tab",
        "escape" => "Escape",
        "left" => "ArrowLeft",
        "right" => "ArrowRight",
        "up" => "ArrowUp",
        "down" => "ArrowDown",
        "home" => "Home",
        "end" => "End",
        "pageup" => "PageUp",
        "pagedown" => "PageDown",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emulator_frames_turn_bgra() {
        let f = Frame::new(1, 2, vec![1, 2, 3, 4, 5, 6], true, None, 0);
        assert_eq!(f.bgra(), [3, 2, 1, 255, 6, 5, 4, 255]);
        assert!(f.jpeg().is_ok_and(|j| j.starts_with(&[0xff, 0xd8])));
    }

    #[test]
    fn turned_touches_turn_back() {
        let f = Frame::new(3, 2, vec![0; 24], false, Some(3), 1);
        // turned clockwise, the shown top-left was the screen's bottom-left, and the top-right its top-left
        assert_eq!(f.device_point(0., 0.), (0., 1.));
        assert_eq!(f.device_point(1., 0.), (0., 0.));
        // and back: the device's top-left corner shows at the top-right
        assert_eq!(f.shown_rect([0., 0., 0.1, 0.2]), [0.8, 0., 0.2, 0.1]);
        // every way round, a point and a rect go there and back
        for turn in 0..4 {
            let f = Frame::new(1, 1, vec![0; 4], false, None, turn);
            let [x, y, ..] = f.shown_rect([0.25, 0.5, 0., 0.]);
            assert_eq!(f.device_point(x, y), (0.25, 0.5), "turn {turn}");
        }
    }

    #[test]
    fn pixels_turn_as_the_points_do() {
        // a 3×2 frame, each pixel its own index
        let px: Vec<u8> = (0..6).collect();
        assert_eq!(Frame::turn_pixels(&px, 3, 2, 1, 1), (vec![3, 0, 4, 1, 5, 2], 2, 3));
        assert_eq!(Frame::turn_pixels(&px, 3, 2, 1, 2), (vec![5, 4, 3, 2, 1, 0], 3, 2));
        assert_eq!(Frame::turn_pixels(&px, 3, 2, 1, 3), (vec![2, 5, 1, 4, 0, 3], 2, 3));
        // turned once, and then to upside down, is turned twice
        let f = Frame::new(3, 2, (0..24).collect(), false, None, 0).turned(1).turned(2);
        assert_eq!((f.width, f.height, f.turn), (3, 2, 2));
        assert_eq!(f.pixels[..4], [20, 21, 22, 23]);
        // the device's top-left pixel shows where its point does
        let [x, y, ..] = f.shown_rect([0., 0., 0., 0.]);
        assert_eq!((x, y), (1., 1.));
    }

    #[test]
    fn a_feed_hands_out_only_newer_frames() {
        let feed = Feed::default();
        assert!(feed.newer(0).is_none());
        feed.put(Frame::new(1, 1, vec![0; 4], false, None, 0));
        assert_eq!(feed.newer(0).map(|(n, _)| n), Some(1));
        assert!(feed.newer(1).is_none());
        assert!(feed.wait_newer(1, std::time::Duration::from_millis(10)).is_none());
    }

    #[test]
    fn keys_and_tab_keys() {
        let d = Device { platform: Platform::Android, id: "Pixel_9".into(), name: "Pixel 9".into(), version: "API 36".into(), running: false, duo: false, usb: false };
        assert_eq!(d.key(), "device:android:Pixel_9");
        assert_eq!(w3c_key("left"), Some("ArrowLeft"));
        assert_eq!(w3c_key("a"), None);
    }
}
