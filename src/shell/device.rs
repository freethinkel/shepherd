//! Device tabs in the right panel, t3code's Device panel ported: its setup, the device list, the live screen and the
//! controls rail. The devices are `crate::device`'s (simulators through simdriver, emulators through their gRPC), not
//! a Node hub's. A tab keyed `device` lists them; opening one turns it into that device's tab.
use super::*;
use crate::device::{self, Button, Device, Feed, Found, Hinge, Platform, Session, Stand, Touch};
use crate::right_panel::Surface;
use gpui::{AnyElement, Bounds, RenderImage};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) mod three;
mod tools;

const PICKER: &str = device::KEY;
/// A 3D view's page id starts with this, then the device's key.
pub(super) const PAGE_PREFIX: &str = "device3d:";
const PHONE: &str = "icons/solar/smartphone.svg";

#[derive(Default)]
pub(super) struct Devices {
    /// The last look at the machine; `None` before the first.
    found: Option<Found>,
    finding: bool,
    /// A look asked for while one was under way, which may have seen the machine as it was: one more after it.
    find_again: bool,
    /// The device being started or opened, by key.
    pending: Option<Device>,
    error: Option<String>,
    /// The open devices' screens, by key; a device shown in two workspaces streams once.
    screens: HashMap<String, Screen>,
    /// The rail's `…` menu: of which device, and where.
    pub(super) menu: Option<crate::ui::Popup<(String, gpui::Point<Pixels>)>>,
    /// What `show_devices` opens with the device: its Tools panel, its overlay, the rail's Text size.
    open_with: (bool, bool, bool),
    /// The Tools panel's open select.
    pub(super) choice: Option<crate::ui::Popup<tools::Choice>>,
    /// A screen is on screen: frames of hidden ones wake nothing.
    shown: Arc<AtomicBool>,
    /// Hands the sessions' threads a way to redraw the window.
    wake: Option<device::Wake>,
}

struct Screen {
    device: Device,
    /// Shared with its 3D view's page, which sends it touches.
    session: Rc<Session>,
    feed: Arc<Feed>,
    /// The frame `image` was made from.
    seen: u64,
    image: Option<Arc<RenderImage>>,
    /// The frame `image` shows, which says where a point on it is on the device (an iPhone Duo's inner screen comes
    /// turned).
    shown: Option<Arc<device::Frame>>,
    /// Shown in 3D, t3code's way, by a page of its own instead of flat.
    three: bool,
    /// The hinge as the rail last drew it: in 3D the window redraws for nothing else.
    drawn_fold: Option<device::Fold>,
    /// Its 3D page has drawn the device; its loader shows until then.
    three_drawn: bool,
    /// Where the screen was painted last, which the pointer is read against.
    bounds: Rc<Cell<Bounds<Pixels>>>,
    /// A finger is down on it.
    pressed: bool,
    dark: Option<bool>,
    /// As the device last said, or as the rail last set it.
    text_size: Option<device::tools::TextSize>,
    /// A screenshot being saved.
    capturing: bool,
    /// How the rail's rotate last turned it (`ORIENTATIONS`' index), which its next press turns on from: an app that
    /// doesn't turn leaves its frames as they were.
    held: Option<usize>,
    /// Reads the device off the window's thread, for the Tools panel and the overlay.
    probe: device::Probe,
    /// The Tools panel beside the screen, while it's open.
    tools: Option<tools::Tools>,
    /// The frontmost app's elements outlined over the screen, while that's on.
    overlay: Option<tools::Overlay>,
}

impl Devices {
    /// Whether the panel shows a device's screen this frame.
    pub(super) fn set_shown(&self, on: bool) {
        self.shown.store(on, Ordering::Relaxed);
    }
}

fn platform_label(p: Platform) -> &'static str {
    match p {
        Platform::Ios => "iOS Simulators",
        Platform::Android => "Android",
    }
}

/// t3code's outline button, small.
fn outline_button(theme: &Theme, id: impl Into<gpui::ElementId>, icon: Option<&'static str>, label: &'static str) -> Stateful<Div> {
    let hover = theme.hover;
    div()
        .id(id)
        .h(px(28.))
        .px(px(10.))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(6.))
        .rounded(px(8.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.wash(0.02))
        .text_size(px(12.5))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(theme.text)
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .children(icon.map(|i| gpui::svg().path(i).size(px(14.)).text_color(theme.text_muted)))
        .child(label)
}

/// t3code's `DeviceLoadingView`: the device, what's happening, and which of the two steps (open, then video).
fn loading_view(theme: &Theme, id: &str, name: &str, description: &str, message: String, second_step: bool, error: bool) -> Div {
    let bar = |on: bool| div().h(px(4.)).flex_1().rounded_full().bg(if on { theme.text_muted } else { theme.wash(0.1) });
    div().size_full().flex().items_center().justify_center().px(px(24.)).child(
        div()
            .max_w(px(320.))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(12.))
            .text_center()
            .child(div().size(px(48.)).flex().items_center().justify_center().rounded(px(12.)).border_1().border_color(theme.border).bg(theme.wash(0.03)).child(gpui::svg().path(PHONE).size(px(24.)).text_color(theme.text_muted)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(div().text_size(px(13.)).font_weight(gpui::FontWeight::MEDIUM).child(name.to_string()))
                    .when(!description.is_empty(), |el| el.child(div().text_size(px(12.)).text_color(theme.text_muted).child(description.to_string()))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.))
                    .text_color(if error { theme.danger } else { theme.text_muted })
                    .when(!error, |el| el.child(spinner(theme, format!("device-loading-{id}"))))
                    .child(message),
            )
            .when(!error, |el| el.child(div().w(px(96.)).flex().gap(px(4.)).child(bar(true)).child(bar(second_step)))),
    )
}

/// t3code's `PlatformStatus`: a platform ready or why not.
fn platform_status(theme: &Theme, platform: &'static str, check: Option<&device::Check>) -> Div {
    let (ready, message) = check.map_or((false, "Checking…".to_string()), |c| (c.ready, c.message.clone()));
    div()
        .flex()
        .gap(px(8.))
        .rounded(px(8.))
        .border_1()
        .border_color(theme.border)
        .px(px(12.))
        .py(px(8.))
        .child(
            gpui::svg()
                .path(if ready { "icons/solar/check-circle.svg" } else { "icons/solar/danger-circle.svg" })
                .mt(px(1.))
                .size(px(16.))
                .flex_none()
                .text_color(if ready { theme.success } else { theme.text_muted }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_size(px(13.)).font_weight(gpui::FontWeight::MEDIUM).child(platform))
                .child(div().text_size(px(12.)).text_color(theme.text_muted).child(message)),
        )
}

impl Shell {
    /// `SHEPHERD_PANEL=devices`, or a device's key: the Device tab, on that device once it's found (screenshots);
    /// `:tools` after the key opens its Tools panel, `:overlay` its accessibility overlay too; `:text-size` the rail's
    /// Text size instead.
    pub fn show_devices(&mut self, key: &str, cx: &mut Context<Self>) {
        self.open_device_picker(cx);
        if key == PICKER {
            return;
        }
        let (key, text) = key.strip_suffix(":text-size").map_or((key, false), |k| (k, true));
        let (key, overlay) = key.strip_suffix(":overlay").map_or((key, false), |k| (k, true));
        let (key, tools) = key.strip_suffix(":tools").map_or((key, overlay), |k| (k, true));
        self.devices.open_with = (tools, overlay, text);
        let key = key.to_string();
        cx.spawn(async move |this, cx| {
            let found = cx.background_spawn(async { device::find() }).await;
            let _ = this.update(cx, |s, cx| {
                if let Some(d) = found.devices.iter().find(|d| d.key() == key) {
                    s.open_device(d.clone(), cx);
                }
                s.devices.found = Some(found);
            });
        })
        .detach();
    }

    /// The `Device` offer: the panel's list of devices, looked for the first time.
    pub(super) fn open_device_picker(&mut self, cx: &mut Context<Self>) {
        self.update_panel(cx, |p| p.add(Surface::Device(PICKER.into())));
        if self.devices.found.is_none() {
            self.find_devices(cx);
        }
    }

    pub(super) fn find_devices(&mut self, cx: &mut Context<Self>) {
        if std::mem::replace(&mut self.devices.finding, true) {
            self.devices.find_again = true;
            return;
        }
        cx.spawn(async move |this, cx| {
            let found = cx.background_spawn(async { device::find() }).await;
            let _ = this.update(cx, |s, cx| {
                s.devices.found = Some(found);
                s.devices.finding = false;
                if std::mem::take(&mut s.devices.find_again) {
                    s.find_devices(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The sessions' way to redraw: one channel, drained before each redraw, so a burst of frames is one.
    fn device_wake(&mut self, cx: &mut Context<Self>) -> device::Wake {
        if let Some(wake) = &self.devices.wake {
            return wake.clone();
        }
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<()>();
        let shown = self.devices.shown.clone();
        cx.spawn(async move |this, cx| {
            use futures::StreamExt;
            while rx.next().await.is_some() {
                while rx.try_recv().is_ok() {}
                let redraw = |s: &mut Self, cx: &mut Context<Self>| {
                    // a 3D view's page draws its frames, but its rail is the window's
                    if shown.load(Ordering::Relaxed) || s.devices.screens.values().any(|sc| sc.three && sc.feed.fold() != sc.drawn_fold) {
                        cx.notify();
                    }
                };
                if this.update(cx, redraw).is_err() {
                    break;
                }
            }
        })
        .detach();
        let wake: device::Wake = Arc::new(move || {
            let _ = tx.unbounded_send(());
        });
        self.devices.wake = Some(wake.clone());
        wake
    }

    /// Starts `device` if it's off, then streams it into its own tab in the shown workspace, in the list's place.
    fn open_device(&mut self, device: Device, cx: &mut Context<Self>) {
        let Some(ws) = self.shown_tab().map(|t| t.workspace_id) else { return };
        let key = device.key();
        if self.devices.screens.contains_key(&key) {
            self.show_device_tab(&ws, &key, cx);
            return;
        }
        if self.devices.pending.is_some() {
            return;
        }
        self.devices.pending = Some(device.clone());
        self.devices.error = None;
        let (rt, feed, wake) = (self.rt.clone(), Arc::new(Feed::default()), self.device_wake(cx));
        cx.spawn(async move |this, cx| {
            let opened = cx
                .background_spawn({
                    let (device, feed) = (device.clone(), feed.clone());
                    async move {
                        device::boot(&device)?;
                        let session = Session::open(&device, &rt, feed, wake)?;
                        anyhow::Ok((session, device::dark(&device)))
                    }
                })
                .await;
            let _ = this.update(cx, |s, cx| {
                s.devices.pending = None;
                match opened {
                    Ok((session, dark)) => {
                        let device = Device { running: true, ..device };
                        let screen = Screen {
                            device,
                            probe: session.probe(),
                            session: Rc::new(session),
                            feed,
                            seen: 0,
                            image: None,
                            shown: None,
                            three: false,
                            drawn_fold: None,
                            three_drawn: false,
                            bounds: Default::default(),
                            pressed: false,
                            dark,
                            text_size: None,
                            capturing: false,
                            held: None,
                            tools: None,
                            overlay: None,
                        };
                        s.devices.screens.insert(key.clone(), screen);
                        let (tools, overlay, text) = std::mem::take(&mut s.devices.open_with);
                        if tools {
                            s.toggle_device_tools(&key, cx);
                        }
                        if overlay {
                            s.toggle_device_overlay(&key, cx);
                        }
                        if text {
                            // once the rail's laid out, so the menu hangs from its button
                            let key = key.clone();
                            cx.spawn(async move |this, cx| {
                                cx.background_executor().timer(std::time::Duration::from_millis(500)).await;
                                let anchor = format!("device-rail-text-size-{key}");
                                let _ = this.update(cx, |s, cx| s.open_text_size_menu(&key, &anchor, Default::default(), cx));
                            })
                            .detach();
                        }
                        s.show_device_tab(&ws, &key, cx);
                        // it's running now
                        s.find_devices(cx);
                    }
                    Err(e) => s.devices.error = Some(format!("{e:#}")),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The device's tab in workspace `ws`, the list's tab giving it its place.
    fn show_device_tab(&mut self, ws: &str, key: &str, cx: &mut Context<Self>) {
        let panel = self.right.panels.entry(ws.to_string()).or_default();
        panel.add(Surface::Device(key.to_string()));
        panel.close(&Surface::Device(PICKER.into()));
        cx.notify();
    }

    /// Screens no tab shows any more stop streaming; their last frame leaves the atlas.
    pub(super) fn sync_devices(&mut self) {
        let panels = &self.right.panels;
        let gone: Vec<String> = self.devices.screens.keys().filter(|k| !panels.values().any(|p| p.tabs.iter().any(|s| s.key() == k.as_str()))).cloned().collect();
        for key in gone {
            if let Some(image) = self.devices.screens.remove(&key).and_then(|s| s.image) {
                self.retired_images.push(image);
            }
        }
    }

    /// Tab `key`'s title.
    pub(super) fn device_label(&self, key: &str) -> String {
        self.devices.screens.get(key).map_or_else(|| "Device".into(), |s| s.device.name.clone())
    }

    /// A key typed with the keyboard on a device's screen, sent to it.
    pub(super) fn device_key(&self, e: &gpui::KeyDownEvent) -> bool {
        let Some(Surface::Device(key)) = self.panel().and_then(|p| p.active.clone()) else { return false };
        let Some(screen) = self.devices.screens.get(&key) else { return false };
        let k = &e.keystroke;
        if device::w3c_key(&k.key).is_some() && screen.session.key(&k.key) {
            return true;
        }
        match &k.key_char {
            Some(text) => {
                screen.session.text(text);
                true
            }
            None => false,
        }
    }

    /// The clipboard's text typed into the device shown, as a paste would (simdriver pastes what isn't ASCII).
    pub(super) fn device_paste(&self, cx: &mut Context<Self>) -> bool {
        let Some(Surface::Device(key)) = self.panel().and_then(|p| p.active.clone()) else { return false };
        let Some(screen) = self.devices.screens.get(&key) else { return false };
        if let Some(text) = cx.read_from_clipboard().and_then(|c| c.text()).filter(|t| !t.is_empty()) {
            screen.session.text(&text);
        }
        true
    }

    fn device_action(&mut self, key: &str, cx: &mut Context<Self>, run: impl FnOnce(&Device) -> anyhow::Result<()> + Send + 'static) {
        let Some(device) = self.devices.screens.get(key).map(|s| s.device.clone()) else { return };
        cx.spawn(async move |_, cx| {
            if let Err(e) = cx.background_spawn(async move { run(&device) }).await {
                cx.update(|cx| crate::ui::toast::error(cx, format!("{e:#}")));
            }
        })
        .detach();
    }

    /// Turns the device to be held as `ORIENTATIONS[at]`: a simulator through its session, an emulator off this thread.
    pub(super) fn turn_device(&mut self, key: &str, at: usize, cx: &mut Context<Self>) {
        let Some(screen) = self.devices.screens.get_mut(key) else { return };
        let Some((orientation, _)) = device::tools::ORIENTATIONS.get(at) else { return };
        screen.held = Some(at);
        if screen.device.platform == Platform::Android {
            return self.device_act(key, device::tools::Action::Orientation(orientation), cx);
        }
        screen.session.rotate(orientation);
        cx.notify();
    }

    /// The rail's rotate: a quarter turn clockwise on from how it's held.
    fn rotate_device(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(screen) = self.devices.screens.get(key) else { return };
        let os = device::tools::ORIENTATIONS;
        // a Duo's from its live screen's UI, a phone's from its frames
        let shown = match screen.feed.fold().filter(|f| f.duo) {
            Some(fold) => os.iter().position(|(o, _)| *o == fold.orientation).unwrap_or(0),
            None => screen.feed.latest().map_or(0, |f| usize::from(f.turn)),
        };
        let next = (screen.held.unwrap_or(shown) + 1) % os.len();
        self.turn_device(key, next, cx);
    }

    fn toggle_device_dark(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(screen) = self.devices.screens.get(key) else { return };
        let dark = !screen.dark.unwrap_or(false);
        // the Tools panel reads it back, when it's open
        self.device_act(key, device::tools::Action::Dark(dark), cx);
    }

    /// A full-size screenshot to Downloads, as t3code's; the toast shows it in the Finder.
    fn save_device_screenshot(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(screen) = self.devices.screens.get_mut(key).filter(|s| !s.capturing) else { return };
        screen.capturing = true;
        let device = screen.device.clone();
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(std::path::PathBuf::from).unwrap_or_default();
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
        let name: String = device.name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' }).collect();
        let path = home.join("Downloads").join(format!("{name}-{stamp}.png"));
        let key = key.to_string();
        cx.spawn(async move |this, cx| {
            let saved = cx.background_spawn({
                let path = path.clone();
                async move { device::screenshot(&device, &path) }
            });
            let saved = saved.await;
            let _ = this.update(cx, |s, cx| {
                if let Some(screen) = s.devices.screens.get_mut(&key) {
                    screen.capturing = false;
                }
                match saved {
                    Ok(()) => {
                        let file = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
                        let reveal: Rc<dyn Fn(&mut App)> = Rc::new(move |cx| cx.reveal_path(&path));
                        crate::ui::toast::info(cx, format!("Saved {file} to Downloads"), Some(reveal));
                    }
                    Err(e) => crate::ui::toast::error(cx, format!("Screenshot failed: {e:#}")),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// t3code's 3D view of the device, or its flat screen again. The view is a page of the panel's: `sync_pages`
    /// makes it for the entry `device_pages` lists, and drops it once that's gone.
    fn show_device_3d(&mut self, key: &str, on: bool, cx: &mut Context<Self>) {
        if let Some(screen) = self.devices.screens.get_mut(key) {
            screen.three = on;
            screen.three_drawn = false;
            // the flat screen starts over on the latest frame when it comes back
            screen.seen = 0;
            cx.notify();
        }
    }

    fn reset_device_3d(&self, key: &str) {
        if let Some(page) = self.pages.live.get(&(format!("{PAGE_PREFIX}{key}"), None)) {
            let _ = page.view.evaluate_script("window.shepherdReset && window.shepherdReset()");
        }
    }

    /// The 3D views' pages, as entries of the panel's, as browser tabs are.
    pub(super) fn device_pages(&self, theme: &Theme) -> impl Iterator<Item = crate::settings::WebEntry> + '_ {
        let shown = self.devices.screens.iter().filter(|(_, s)| s.three);
        let backdrop = three::backdrop(theme);
        shown.map(move |(key, s)| crate::settings::WebEntry { id: format!("{PAGE_PREFIX}{key}"), url: three::url(&s.device, &backdrop), ..Default::default() })
    }

    /// The 3D pages still drawing their device, kept out of sight.
    pub(super) fn device_pages_drawing(&self) -> Vec<String> {
        self.devices.screens.iter().filter(|(_, s)| s.three && !s.three_drawn).map(|(key, _)| format!("{PAGE_PREFIX}{key}")).collect()
    }

    /// Page `id`, a 3D view's, has drawn its device.
    pub(super) fn device_page_ready(&mut self, id: &str) {
        if let Some(screen) = id.strip_prefix(PAGE_PREFIX).and_then(|key| self.devices.screens.get_mut(key)) {
            screen.three_drawn = true;
        }
    }

    /// The page of device tab `key`, while it's shown in 3D.
    pub(super) fn device_page_key(&self, key: &str) -> Option<String> {
        self.devices.screens.get(key).filter(|s| s.three).map(|_| format!("{PAGE_PREFIX}{key}"))
    }

    /// The session and feed page `id`, a 3D view's, shows.
    pub(super) fn device_page_session(&self, id: &str) -> Option<(Rc<Session>, Arc<Feed>)> {
        let key = id.strip_prefix(PAGE_PREFIX)?;
        self.devices.screens.get(key).map(|s| (s.session.clone(), s.feed.clone()))
    }

    /// Powers the device off and closes its tabs everywhere: it's gone.
    fn power_off_device(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(device) = self.devices.screens.get(key).map(|s| s.device.clone()) else { return };
        // the list looks again once it's off: a look now would see it running still
        cx.spawn(async move |this, cx| {
            let off = cx.background_spawn(async move { device::power_off(&device) }).await;
            let _ = this.update(cx, |s, cx| {
                if let Err(e) = off {
                    crate::ui::toast::error(cx, format!("{e:#}"));
                }
                s.find_devices(cx);
            });
        })
        .detach();
        if let Some(d) = self.devices.found.as_mut().and_then(|f| f.devices.iter_mut().find(|d| d.key() == key)) {
            d.running = false;
        }
        let surface = Surface::Device(key.to_string());
        self.right.panels.values_mut().for_each(|p| p.close(&surface));
        self.sync_devices();
        cx.notify();
    }

    /// The body of tab `key`: the setup, the list, or a device's screen.
    pub(super) fn render_device(&mut self, key: &str, theme: &Theme, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if key == PICKER {
            return match SettingsGlobal::of(cx).settings.devices_set_up {
                true => self.render_device_list(theme, cx).into_any_element(),
                false => self.render_device_setup(theme, cx).into_any_element(),
            };
        }
        let Some(screen) = self.devices.screens.get_mut(key) else {
            return loading_view(theme, key, "Device", "", "This device is closed.".into(), false, true).into_any_element();
        };
        if screen.three {
            let loading = (!screen.three_drawn).then(|| {
                let (name, version) = (screen.device.name.clone(), screen.device.version.clone());
                loading_view(theme, key, &name, &version, "Loading 3D view…".into(), false, false).absolute().inset_0()
            });
            // its page draws the frames: the window needn't redraw for them, only for its rail's hinge
            screen.drawn_fold = screen.feed.fold();
            self.devices.set_shown(false);
            let area = self.render_web_area(&(format!("{PAGE_PREFIX}{key}"), None), theme, window, cx).children(loading);
            // the rail keeps its strip: the page sits above what GPUI draws there
            return self.device_stage(key, theme, div().flex_1().min_w_0().h_full().pr(px(56.)).flex().child(area), None, window, cx);
        }
        if let Some((n, frame)) = screen.feed.newer(screen.seen) {
            screen.seen = n;
            #[cfg(target_os = "macos")]
            let surfaced = frame.surface.is_some();
            #[cfg(not(target_os = "macos"))]
            let surfaced = false;
            // a frame with a surface is drawn from it; one without, as an image made here
            if !surfaced
                && let Some(buffer) = image::RgbaImage::from_raw(frame.width, frame.height, frame.bgra())
                && let Some(old) = screen.image.replace(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
            {
                self.retired_images.push(old);
            }
            screen.shown = Some(frame);
        }
        let error = screen.feed.error();
        let (name, description) = (screen.device.name.clone(), screen.device.version.clone());
        let Some(frame) = screen.shown.clone() else {
            let (message, failed) = error.map_or(("Connecting video…".to_string(), false), |e| (e, true));
            return loading_view(theme, key, &name, &description, message, true, failed).into_any_element();
        };
        let image = screen.image.clone();
        let bounds = screen.bounds.clone();
        let overlay = screen.overlay.as_ref().map(|o| (o.frames.clone(), theme.accent));
        // the screen's own corners: an iPhone's are rounder than a Pixel's
        let round = if screen.device.platform == Platform::Ios { 0.13 } else { 0.07 };
        let painted = bounds.clone();
        let screen_canvas = gpui::canvas(
            |_, _, _| (),
            move |area, _, window, _| {
                let (w, h) = (frame.width as f32, frame.height as f32);
                if w == 0. || h == 0. {
                    return;
                }
                let scale = (f32::from(area.size.width) / w).min(f32::from(area.size.height) / h);
                let fitted = gpui::size(px(w * scale), px(h * scale));
                let origin = area.origin + gpui::point((area.size.width - fitted.width) / 2., (area.size.height - fitted.height) / 2.);
                let fitted = Bounds::new(origin, fitted);
                painted.set(fitted);
                let corners = gpui::Corners::all(fitted.size.width.min(fitted.size.height) * round);
                #[cfg(target_os = "macos")]
                let surfaced = frame.surface.as_ref().map(|surface| window.paint_surface(fitted, corners, surface.buffer())).is_some();
                #[cfg(not(target_os = "macos"))]
                let surfaced = false;
                if !surfaced && let Some(image) = image.clone() {
                    let _ = window.paint_image(fitted, corners, image, 0, false);
                }
                // the accessibility overlay: each element's frame, outlined where it's shown
                for f in overlay.iter().flat_map(|(frames, _)| frames) {
                    let [x, y, w, h] = frame.shown_rect(*f);
                    let at = fitted.origin + gpui::point(fitted.size.width * x as f32, fitted.size.height * y as f32);
                    let size = gpui::size(fitted.size.width * w as f32, fitted.size.height * h as f32);
                    let color = overlay.as_ref().map_or(gpui::black(), |(_, c)| *c);
                    window.paint_quad(gpui::outline(Bounds::new(at, size), color.opacity(0.8), gpui::BorderStyle::Solid));
                }
            },
        )
        .size_full();
        let shown = screen.shown.clone();
        let turn = move |x: f64, y: f64| shown.as_ref().map_or((x, y), |f| f.device_point(x, y));
        // where `at` falls on the device's screen, 0 to 1 each way
        let on_screen = move |at: gpui::Point<Pixels>| {
            let b = bounds.get();
            let x = f32::from(at.x - b.origin.x) / f32::from(b.size.width).max(1.);
            let y = f32::from(at.y - b.origin.y) / f32::from(b.size.height).max(1.);
            let (dx, dy) = turn(x as f64, y as f64);
            (dx, dy, (0. ..=1.).contains(&x) && (0. ..=1.).contains(&y))
        };
        // a scroll's way on the device: as a point's, from the frame's middle
        let delta = {
            let shown = screen.shown.clone();
            move |dx: f64, dy: f64| {
                let (x, y) = shown.as_ref().map_or((0.5 + dx, 0.5 + dy), |f| f.device_point(0.5 + dx, 0.5 + dy));
                (x - 0.5, y - 0.5)
            }
        };
        let (down, moved, up, wheel) = (on_screen.clone(), on_screen.clone(), on_screen.clone(), on_screen);
        let k = key.to_string();
        let stage = div()
            .id(SharedString::from(format!("device-stage-{key}")))
            .flex_1()
            .min_w_0()
            .h_full()
            .p(px(16.))
            .pr(px(64.))
            .child(screen_canvas)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener({
                    let k = k.clone();
                    move |this, e: &MouseDownEvent, window, cx| {
                        let (x, y, inside) = down(e.position);
                        if let Some(s) = this.devices.screens.get_mut(&k).filter(|_| inside) {
                            s.pressed = true;
                            s.session.touch(Touch::Down, x, y);
                        }
                        this.focus_panel(true, window, cx);
                    }
                }),
            )
            .on_mouse_move(cx.listener({
                let k = k.clone();
                move |this, e: &gpui::MouseMoveEvent, _, _| {
                    if let Some(s) = this.devices.screens.get(&k).filter(|s| s.pressed) {
                        let (x, y, _) = moved(e.position);
                        s.session.touch(Touch::Move, x, y);
                    }
                }
            }))
            .on_mouse_up(MouseButton::Left, {
                let up = up.clone();
                cx.listener({
                    let k = k.clone();
                    move |this, e: &gpui::MouseUpEvent, _, _| {
                        if let Some(s) = this.devices.screens.get_mut(&k).filter(|s| s.pressed) {
                            s.pressed = false;
                            let (x, y, _) = up(e.position);
                            s.session.touch(Touch::Up, x, y);
                        }
                    }
                })
            })
            // a drag let go of off the screen still lifts the finger
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener({
                    let k = k.clone();
                    move |this, e: &gpui::MouseUpEvent, _, _| {
                        if let Some(s) = this.devices.screens.get_mut(&k).filter(|s| s.pressed) {
                            s.pressed = false;
                            let (x, y, _) = up(e.position);
                            s.session.touch(Touch::Up, x, y);
                        }
                    }
                }),
            )
            .on_scroll_wheel(cx.listener({
                let k = k.clone();
                let size = screen.bounds.clone();
                move |this, e: &gpui::ScrollWheelEvent, _, _| {
                    let Some(s) = this.devices.screens.get(&k) else { return };
                    let (x, y, inside) = wheel(e.position);
                    let b = size.get().size;
                    let d = e.delta.pixel_delta(px(20.));
                    // a mouse wheel's turns are all Moved: each may start a scroll, so each must be over the screen
                    let wheel = matches!(e.delta, gpui::ScrollDelta::Lines(_));
                    if inside || (!wheel && e.touch_phase != gpui::TouchPhase::Started) {
                        let (dx, dy) = (f32::from(d.x) / f32::from(b.width).max(1.), f32::from(d.y) / f32::from(b.height).max(1.));
                        let (dx, dy) = delta(dx as f64, dy as f64);
                        s.session.scroll(e.touch_phase, x, y, dx, dy, wheel);
                    }
                }
            }));
        self.device_stage(key, theme, stage, error, window, cx)
    }

    /// The device's area: its screen, flat or in 3D, the rail beside it and a note of trouble under it; the Tools
    /// panel, when open, to the right of it all.
    fn device_stage(&mut self, key: &str, theme: &Theme, stage: impl IntoElement, error: Option<String>, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let tools = self.render_device_tools(key, theme, window, cx);
        let screen = div()
            .id(SharedString::from(format!("device-{key}")))
            .flex_1()
            .min_w_0()
            .h_full()
            .relative()
            .flex()
            .child(stage)
            .child(self.render_device_rail(key, theme, cx))
            .when_some(error, |el, e| {
                el.child(
                    div().absolute().bottom_0().left_0().right_0().flex().justify_center().p(px(8.)).child(
                        div().rounded(px(6.)).bg(theme.card).border_1().border_color(theme.border).px(px(8.)).py(px(4.)).text_size(px(12.)).text_color(theme.text_muted).child(e),
                    ),
                )
            });
        div().flex_1().min_h_0().flex().bg(theme.wash(0.03)).child(screen).children(tools).into_any_element()
    }

    /// t3code's `DeviceControlsRail`: a pill of buttons down the screen's right side.
    fn render_device_rail(&mut self, key: &str, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let Some(screen) = self.devices.screens.get(key) else { return div() };
        let android = screen.device.platform == Platform::Android;
        let dark = screen.dark;
        let capturing = screen.capturing;
        let three_d = screen.three;
        let tools_open = screen.tools.is_some();
        let menu_open = self.devices.menu.as_ref().is_some_and(|m| m.is_open() && m.value.0 == key);
        let text_anchor = format!("device-rail-text-size-{key}");
        let text_open = self.choice_open(&text_anchor);
        let hover = theme.hover;
        let button = |id: &str, icon: &'static str, label: String, enabled: bool| {
            div()
                .id(SharedString::from(format!("device-rail-{id}-{key}")))
                .size(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.))
                // a button whose menu is open has no tip over it
                .when(!label.is_empty(), |el| el.tooltip(tip(label, None)))
                .when(enabled, |el| el.cursor_pointer().hover(move |s| s.bg(hover)))
                .when(!enabled, |el| el.opacity(0.4))
                .child(gpui::svg().path(icon).size(px(16.)).text_color(theme.text_muted))
        };
        let divider = || div().my(px(4.)).h(px(1.)).w(px(20.)).bg(theme.border);
        let press = |b: Button| {
            let k = key.to_string();
            cx.listener(move |this: &mut Shell, _: &ClickEvent, _, _| {
                if let Some(s) = this.devices.screens.get(&k) {
                    s.session.button(b);
                }
            })
        };
        let k = key.to_string();
        let pill = div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(2.))
            .p(px(5.))
            .rounded(px(20.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.card)
            .shadow_sm()
            .child(button("home", "icons/solar/home-angle.svg", "Home".into(), true).on_click(press(Button::Home)))
            .when(android, |el| {
                el.child(button("back", "icons/alt-arrow-left.svg", "Back".into(), true).on_click(press(Button::Back)))
                    .child(button("recents", "icons/solar/three-squares.svg", "Recents".into(), true).on_click(press(Button::Recents)))
            })
            .child(divider())
            .child(button("rotate", "icons/solar/smartphone-rotate-orientation.svg", "Rotate".into(), true).on_click(cx.listener({
                let k = k.clone();
                move |this, _: &ClickEvent, _, cx| this.rotate_device(&k, cx)
            })))
            .child({
                let next = if dark == Some(true) { "light" } else { "dark" };
                let icon = if dark == Some(true) { "icons/solar/sun.svg" } else { "icons/solar/moon.svg" };
                button("appearance", icon, format!("Switch device to {next} mode"), dark.is_some()).on_click(cx.listener({
                    let k = k.clone();
                    move |this, _: &ClickEvent, _, cx| this.toggle_device_dark(&k, cx)
                }))
            })
            .child(
                button("text-size", "icons/solar/text-format.svg", if text_open { String::new() } else { "Text size".into() }, true)
                    .when(text_open, |el| el.bg(hover))
                    .relative()
                    .child(super::widgets::anchor_probe(&self.anchors, text_anchor.clone()))
                    .on_click(cx.listener({
                        let k = k.clone();
                        move |this, e: &ClickEvent, _, cx| this.open_text_size_menu(&k, &text_anchor, e.position(), cx)
                    })),
            )
            .child(
                button("screenshot", "icons/solar/camera.svg", if capturing { "Capturing screenshot".into() } else { "Save screenshot".into() }, !capturing).on_click(cx.listener({
                    let k = k.clone();
                    move |this, _: &ClickEvent, _, cx| this.save_device_screenshot(&k, cx)
                })),
            )
            .child(button("tools", "icons/solar/tuning-2.svg", "Tools".into(), true).when(tools_open, |el| el.bg(hover)).on_click(cx.listener({
                let k = k.clone();
                move |this, _: &ClickEvent, _, cx| this.toggle_device_tools(&k, cx)
            })))
            .child(button("more", "icons/solar/menu-dots.svg", if menu_open { String::new() } else { "More device actions".into() }, true)
                .when(menu_open, |el| el.bg(hover))
                .relative()
                .child(super::widgets::anchor_probe(&self.anchors, format!("device-more-{key}")))
                .on_click(cx.listener({
                let k = k.clone();
                move |this, e: &ClickEvent, _, cx| {
                    // the menu hangs under the button, its right edge at the button's
                    let at = this.anchors.borrow().get(format!("device-more-{k}").as_str()).map_or(e.position(), |b| b.bottom_right() + gpui::point(px(0.), px(4.)));
                    this.devices.menu = Some(crate::ui::Popup::new((k.clone(), at)));
                    cx.notify();
                }
            })))
            .child(divider())
                    .child(button("3d", "icons/solar/box.svg", "3D view".into(), true).when(three_d, |el| el.bg(hover)).on_click(cx.listener({
                        let k = k.clone();
                        move |this, _: &ClickEvent, _, cx| this.show_device_3d(&k, true, cx)
                    })))
                    .child(button("flat", PHONE, "Flat view".into(), true).when(!three_d, |el| el.bg(hover)).on_click(cx.listener({
                        let k = k.clone();
                        move |this, _: &ClickEvent, _, cx| this.show_device_3d(&k, false, cx)
                    })))
                    .when(three_d, |el| {
                        el.child(button("reset", "icons/solar/maximize.svg", "Restore 3D view".into(), true).on_click(cx.listener({
                            let k = k.clone();
                            move |this, _: &ClickEvent, _, _| this.reset_device_3d(&k)
                        })))
                    });
        let folds = self.render_fold_pills(key, theme, cx);
        div().absolute().top_0().bottom_0().right(px(10.)).flex().flex_col().items_center().justify_center().gap(px(8.)).child(pill).children(folds)
    }

    /// A foldable's hinge, under the rail as t3code's: iPhone Duo's fold shapes and, in 3D, where it shows how it
    /// stands, its stands; an Android's fold and unfold. A `!` says why the last change failed.
    fn render_fold_pills(&mut self, key: &str, theme: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some((fold, three)) = self.devices.screens.get(key).and_then(|s| Some((s.feed.fold()?, s.three))) else { return Vec::new() };
        let hover = theme.hover;
        let pill = || div().flex().flex_col().items_center().gap(px(2.)).p(px(5.)).rounded(px(20.)).border_1().border_color(theme.border).bg(theme.card).shadow_sm();
        let button = |id: &str, icon: &'static str, label: &'static str, pressed: bool, hinge: Hinge| {
            let k = key.to_string();
            div()
                .id(SharedString::from(format!("device-fold-{id}-{key}")))
                .size(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.))
                .tooltip(tip(label, None))
                .when(pressed, |el| el.bg(hover))
                .when(fold.busy, |el| el.opacity(0.4))
                .when(!fold.busy, |el| el.cursor_pointer().hover(move |s| s.bg(hover)))
                .child(gpui::svg().path(icon).size(px(18.)).text_color(theme.text_muted))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, _| {
                    if let Some(s) = this.devices.screens.get(&k).filter(|s| !s.feed.fold().is_some_and(|f| f.busy)) {
                        s.session.hinge(hinge);
                    }
                }))
        };
        let shape = if fold.angle <= 0. { "closed" } else if fold.angle >= 180. { "open" } else { "half" };
        let mut pills = Vec::new();
        if fold.duo {
            let flat = fold.stand.is_none();
            pills.push(
                pill()
                    .child(button("closed", "icons/fold-closed.svg", "Closed", flat && shape == "closed", Hinge::Angle(0.)))
                    .child(button("book", "icons/fold-book.svg", "Book", flat && shape == "half", Hinge::Angle(90.)))
                    .child(button("open", "icons/fold-open.svg", "Open", flat && shape == "open", Hinge::Angle(180.)))
                    .into_any_element(),
            );
            if three {
                pills.push(
                    pill()
                        .child(button("laptop", "icons/fold-laptop.svg", "Laptop stand", fold.stand == Some(Stand::Laptop), Hinge::Stand(Stand::Laptop)))
                        .child(button("tent", "icons/fold-tent.svg", "Tent stand", fold.stand == Some(Stand::Tent), Hinge::Stand(Stand::Tent)))
                        .into_any_element(),
                );
            }
        } else {
            pills.push(
                pill()
                    .child(button("fold", "icons/fold-closed.svg", "Fold device", fold.cover, Hinge::Angle(0.)))
                    .child(button("unfold", "icons/fold-open.svg", "Unfold device", !fold.cover, Hinge::Angle(180.)))
                    .into_any_element(),
            );
        }
        if let Some(error) = fold.error {
            pills.push(
                div()
                    .id(SharedString::from(format!("device-fold-error-{key}")))
                    .text_size(px(12.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.danger)
                    .tooltip(tip(error, None))
                    .child("!")
                    .into_any_element(),
            );
        }
        pills
    }

    /// The rail's `…` menu: close the tab, or power the device off.
    pub(super) fn device_menu(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.devices.menu.as_ref()?;
        let (key, at) = menu.value.clone();
        // a phone is powered off on the phone
        let usb = self.devices.screens.get(&key).is_some_and(|s| s.device.usb);
        let close = |this: &mut Shell| {
            if let Some(m) = this.devices.menu.as_mut() {
                m.close()
            }
        };
        let item = |id: &'static str, icon: &'static str, label: &'static str, color: Hsla| {
            row(theme, false, id).h(px(28.)).rounded(px(6.)).text_color(color).child(menu_mark(icon, color)).child(label)
        };
        let card = menu_card(theme, "device-menu", 190.)
            .on_mouse_down_out(cx.listener(move |this, _, _, cx| {
                close(this);
                cx.notify();
            }))
            .child(item("device-menu-close", "icons/close.svg", "Close device panel", theme.text).on_pick(
                at,
                cx.listener({
                    let key = key.clone();
                    move |this, _: &ClickEvent, _, cx| {
                        close(this);
                        this.close_surface(&Surface::Device(key.clone()), cx);
                    }
                }),
            ))
            .when(!usb, |el| {
                el.child(div().my(px(4.)).h(px(1.)).bg(theme.border)).child(item("device-menu-power", "icons/solar/power.svg", "Power off device", theme.danger).on_pick(
                    at,
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        close(this);
                        this.power_off_device(&key, cx);
                    }),
                ))
            });
        let menu = crate::ui::menu_motion("device-menu-in", menu, card);
        Some(gpui::deferred(gpui::anchored().position(at).anchor(gpui::Anchor::TopRight).snap_to_window_with_margin(px(8.)).child(menu)).with_priority(1).into_any_element())
    }

    /// t3code's `DeviceSetup`, its simulator step: what each platform needs, before the list is shown the first time.
    fn render_device_setup(&mut self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        if self.devices.found.is_none() {
            self.find_devices(cx);
        }
        let found = self.devices.found.clone().unwrap_or_default();
        let ready = found.ios.iter().chain(&found.android).any(|c| c.ready);
        let finding = self.devices.finding;
        let check = outline_button(theme, "device-setup-check", None, if finding { "Checking…" } else { "Check again" })
            .when(finding, |el| el.child(spinner(theme, "device-setup-spin".into())))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.find_devices(cx)));
        let done = solid_button("device-setup-done", "Done", theme.accent, theme.surface)
            .when(!ready, |el| el.opacity(0.4))
            .when(ready, |el| {
                el.on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                    save_from_sidebar(cx, |s| s.devices_set_up = true);
                    cx.refresh_windows();
                }))
            });
        let cancel = ghost_button(theme, "device-setup-cancel", "Cancel").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.close_surface(&Surface::Device(PICKER.into()), cx)));
        div().id("device-setup").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().justify_center().p(px(20.)).child(
            div()
                // a width of its own: a relative one has its wrapped lines measured unwrapped, and the buttons spill
                // over the padding
                .w(px(440.))
                .max_w_full()
                .mx_auto()
                .flex_none()
                .p(px(20.))
                .rounded(px(14.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.card)
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(div().text_size(px(15.)).font_weight(gpui::FontWeight::SEMIBOLD).child("Set up devices"))
                .child(div().text_size(px(12.5)).text_color(theme.text_muted).child("Review what runs on this machine before using simulators and emulators."))
                .child(div().mt(px(4.)).text_size(px(13.)).font_weight(gpui::FontWeight::MEDIUM).child("Check simulator support"))
                .child(platform_status(theme, "iOS", found.ios.as_ref()))
                .child(platform_status(theme, "Android", found.android.as_ref()))
                .child(div().text_size(px(12.)).text_color(theme.text_muted).child("You can use either platform. Fixing a missing platform does not block the other one."))
                .child(div().flex().child(check))
                .child(div().mt(px(8.)).flex().justify_end().gap(px(8.)).child(cancel).child(done)),
        )
    }

    /// t3code's `DevicePanel` with nothing open: the devices by platform, each to open or start.
    fn render_device_list(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        if let Some(d) = &self.devices.pending {
            let message = if d.running { "Opening device…" } else { "Starting device… This can take a minute." };
            return loading_view(theme, PICKER, &d.name, &d.version, message.into(), false, false).into_any_element();
        }
        let Some(found) = self.devices.found.clone() else {
            if !self.devices.finding {
                self.find_devices(cx);
            }
            return loading_view(theme, PICKER, "Devices", "", "Finding devices…".into(), false, false).into_any_element();
        };
        let error = self.devices.error.clone().map(|e| {
            div()
                .flex()
                .items_start()
                .gap(px(8.))
                .rounded(px(8.))
                .border_1()
                .border_color(theme.danger.opacity(0.3))
                .bg(theme.danger.opacity(0.06))
                .px(px(12.))
                .py(px(8.))
                .text_size(px(12.))
                .text_color(theme.danger)
                .child(gpui::svg().path("icons/solar/danger-circle.svg").mt(px(1.)).size(px(14.)).flex_none().text_color(theme.danger))
                .child(div().flex_1().min_w_0().child(e))
                .child(icon_button(theme, "device-error-dismiss", "×").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.devices.error = None;
                    cx.notify();
                })))
        });
        let groups = [Platform::Ios, Platform::Android].into_iter().filter_map(|platform| {
            let devices: Vec<Device> = found.devices.iter().filter(|d| d.platform == platform).cloned().collect();
            (!devices.is_empty()).then(|| {
                let rows = devices.into_iter().enumerate().map(|(i, d)| {
                    let status = format!("{} · {}", d.version, if d.running { "Running" } else { "Stopped" });
                    let (icon, label) = if d.running { ("icons/solar/square-arrow-right-up.svg", "Open") } else { ("icons/solar/play.svg", "Start") };
                    let open = outline_button(theme, SharedString::from(format!("device-open-{}", d.key())), Some(icon), label).on_click(cx.listener({
                        let d = d.clone();
                        move |this, _: &ClickEvent, _, cx| this.open_device(d.clone(), cx)
                    }));
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .px(px(12.))
                        .py(px(10.))
                        .when(i > 0, |el| el.border_t_1().border_color(theme.border))
                        .child(div().size(px(32.)).flex_none().flex().items_center().justify_center().rounded(px(6.)).border_1().border_color(theme.border).child(gpui::svg().path(PHONE).size(px(16.)).text_color(theme.text_muted)))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(div().truncate().text_size(px(13.)).font_weight(gpui::FontWeight::MEDIUM).child(d.name.clone()))
                                .child(div().truncate().text_size(px(12.)).text_color(theme.text_muted).child(status)),
                        )
                        .child(open)
                });
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.text_muted)
                            .child(gpui::svg().path(PHONE).size(px(16.)).text_color(theme.text_muted))
                            .child(platform_label(platform)),
                    )
                    .child(div().flex().flex_col().rounded(px(10.)).border_1().border_color(theme.border).bg(theme.wash(0.02)).children(rows))
            })
        });
        let groups: Vec<Div> = groups.collect();
        let empty = groups.is_empty();
        // a platform that can't run here says why, as t3code's discovery notes
        let notes = [found.ios.as_ref(), found.android.as_ref()].into_iter().flatten().filter(|c| !c.ready).map(|c| div().text_size(px(12.)).text_color(theme.text_muted).child(c.message.clone()));
        let refresh = outline_button(theme, "device-refresh", Some("icons/solar/refresh.svg"), if self.devices.finding { "Refreshing…" } else { "Refresh devices" })
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.find_devices(cx)));
        div()
            .id("device-list")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px(px(20.))
            .py(px(28.))
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .max_w(px(560.))
                    .mx_auto()
                    .when(empty, |el| el.my_auto().items_center().text_center())
                    .flex()
                    .flex_col()
                    .gap(px(20.))
                    .children(error)
                    .when(empty, |el| {
                        el.child(gpui::svg().path(PHONE).size(px(24.)).text_color(theme.text_faint))
                            .child(div().text_size(px(13.)).text_color(theme.text_muted).child("No simulators or emulators were found on this machine."))
                    })
                    .children(groups)
                    .children(notes)
                    .child(div().flex().when(empty, |el| el.justify_center()).child(refresh)),
            )
            .into_any_element()
    }
}
