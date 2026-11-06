//! The Tools panel beside a device's screen, t3code's `DeviceToolsPanel`: the app in front, the device's settings,
//! the accessibility overlay, a fake location, an app's permissions and, on iOS, a test push. Each control is an
//! action (`device::tools::act`) whose value is then read back from the device: what's kept here is the last read.
use super::*;
use crate::device::tools::{self as dt, Action, Decision, Settings};
use gpui::{Subscription, Task};
use gpui_base::input::{Input, InputEvent, InputState};
use std::time::Duration;

const WIDTH: f32 = 280.;
/// How often an open panel reads the device again: what's in front changes as it's used.
// ponytail: a fixed poll, about a second of simctl's work each on iOS; the device's own change notices would make it free
const POLL: Duration = Duration::from_secs(4);
/// How often the overlay reads the elements again.
const OVERLAY_POLL: Duration = Duration::from_millis(1500);

pub(super) struct Tools {
    /// `None` before the first read.
    settings: Option<Settings>,
    /// Actions under way.
    acting: usize,
    /// Bumped by each action: a read begun before it is stale, and dropped.
    serial: u64,
    error: Option<String>,
    fields: Option<Fields>,
    /// The permission the Grant and Revoke buttons give.
    permission: &'static str,
    _poll: Task<()>,
}

struct Fields {
    url: Entity<InputState>,
    launch: Entity<InputState>,
    lat: Entity<InputState>,
    lon: Entity<InputState>,
    app: Entity<InputState>,
    push: Entity<InputState>,
    /// What the app field says when empty: the app in front.
    app_placeholder: String,
    _events: Vec<Subscription>,
}

/// The accessibility overlay: the elements last read (0 to 1 across the screen), and what reads them again.
pub(super) struct Overlay {
    pub(super) frames: Vec<[f64; 4]>,
    error: Option<String>,
    _poll: Task<()>,
}

/// A select's options, open under its button.
pub(in crate::shell) struct Choice {
    /// The button it hangs from, its anchor's name.
    anchor: String,
    at: gpui::Point<Pixels>,
    /// Which of its corners is at `at`.
    corner: gpui::Anchor,
    options: Vec<SharedString>,
    current: Option<usize>,
    pick: Rc<dyn Fn(&mut Shell, usize, &mut Window, &mut Context<Shell>)>,
}

/// What an action shows at once, before the device's read confirms it.
fn expect(settings: &mut Settings, action: &Action) {
    match action {
        Action::Dark(dark) => settings.dark = Some(*dark),
        Action::TextSize(size) => settings.text_size = Some(*size),
        Action::Toggle(toggle, on) => settings.toggles.iter_mut().filter(|(t, _)| t == toggle).for_each(|(_, v)| *v = *on),
        Action::LiquidGlass(look) => settings.liquid_glass = Some(look),
        Action::ColorFilter(filter) => settings.color_filter = Some(filter),
        _ => {}
    }
}

impl Shell {
    pub(super) fn toggle_device_tools(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(screen) = self.devices.screens.get_mut(key) else { return };
        screen.tools = match screen.tools.take() {
            Some(_) => None,
            None => Some(Tools { settings: None, acting: 0, serial: 0, error: None, fields: None, permission: "camera", _poll: poll_tools(key.to_string(), cx) }),
        };
        cx.notify();
    }

    /// The device's read, if it's still the latest: an action since makes it stale.
    fn settle_tools(&mut self, key: &str, serial: u64, settings: Settings, cx: &mut Context<Self>) {
        let Some(screen) = self.devices.screens.get_mut(key) else { return };
        let Some(tools) = screen.tools.as_mut().filter(|t| t.serial == serial) else { return };
        screen.dark = settings.dark.or(screen.dark);
        screen.text_size = settings.text_size.or(screen.text_size);
        tools.settings = Some(settings);
        cx.notify();
    }

    /// Runs `action` on the device, then reads it back; what it should show, at once.
    pub(super) fn device_act(&mut self, key: &str, action: Action, cx: &mut Context<Self>) {
        let Some(screen) = self.devices.screens.get_mut(key) else { return };
        match action {
            Action::Dark(dark) => screen.dark = Some(dark),
            Action::TextSize(size) => screen.text_size = Some(size),
            _ => {}
        }
        let (device, probe) = (screen.device.clone(), screen.probe.clone());
        let Some(tools) = screen.tools.as_mut() else {
            // the rail's appearance button or text size, with the panel shut: nothing to read back into
            self.device_action(key, cx, move |d| dt::act(d, &action));
            cx.notify();
            return;
        };
        if let Some(settings) = tools.settings.as_mut() {
            expect(settings, &action);
        }
        tools.serial += 1;
        tools.acting += 1;
        tools.error = None;
        let (serial, key) = (tools.serial, key.to_string());
        cx.spawn(async move |this, cx| {
            let (done, settings) = cx.background_spawn(async move { (dt::act(&device, &action), dt::read(&device, &probe)) }).await;
            let _ = this.update(cx, |s, cx| {
                if let Some(tools) = s.devices.screens.get_mut(&key).and_then(|sc| sc.tools.as_mut()) {
                    tools.acting -= 1;
                    tools.error = done.err().map(|e| format!("{e:#}"));
                }
                s.settle_tools(&key, serial, settings, cx);
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn toggle_device_overlay(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(screen) = self.devices.screens.get_mut(key) else { return };
        screen.overlay = match screen.overlay.take() {
            Some(_) => None,
            None => Some(Overlay { frames: Vec::new(), error: None, _poll: poll_overlay(key.to_string(), cx) }),
        };
        cx.notify();
    }

    /// A field's text, trimmed, when there's any: the field is emptied.
    fn take_field(&self, key: &str, field: fn(&Fields) -> &Entity<InputState>, window: &mut Window, cx: &mut Context<Self>) -> Option<String> {
        let input = field(self.devices.screens.get(key)?.tools.as_ref()?.fields.as_ref()?).clone();
        let text = input.read(cx).value().trim().to_string();
        (!text.is_empty()).then(|| {
            input.update(cx, |i, cx| i.set_value("", window, cx));
            text
        })
    }

    fn field_text(&self, key: &str, field: fn(&Fields) -> &Entity<InputState>, cx: &App) -> String {
        let fields = self.devices.screens.get(key).and_then(|s| s.tools.as_ref()?.fields.as_ref());
        fields.map(|f| field(f).read(cx).value().trim().to_string()).unwrap_or_default()
    }

    fn submit_tools_field(&mut self, key: &str, which: Submit, window: &mut Window, cx: &mut Context<Self>) {
        let app = self.devices.screens.get(key).and_then(|s| s.tools.as_ref()?.settings.as_ref()?.app.clone());
        let action = match which {
            Submit::Url => self.take_field(key, |f| &f.url, window, cx).map(Action::OpenUrl),
            Submit::Launch => self.take_field(key, |f| &f.launch, window, cx).map(Action::Launch),
            Submit::Push => app.filter(|_| !self.field_text(key, |f| &f.push, cx).is_empty()).and_then(|app| Some(Action::Push { app, text: self.take_field(key, |f| &f.push, window, cx)? })),
            Submit::Location => {
                let (lat, lon) = (self.field_text(key, |f| &f.lat, cx).parse::<f64>(), self.field_text(key, |f| &f.lon, cx).parse::<f64>());
                lat.ok().zip(lon.ok()).filter(|(lat, lon)| lat.abs() <= 90. && lon.abs() <= 180.).map(|(lat, lon)| Action::Location(lat, lon))
            }
        };
        if let Some(action) = action {
            self.device_act(key, action, cx);
        }
    }

    /// The app the Permissions section is for: the one typed, else the one in front.
    fn permission_app(&self, key: &str, cx: &App) -> Option<String> {
        let typed = self.field_text(key, |f| &f.app, cx);
        let front = self.devices.screens.get(key).and_then(|s| s.tools.as_ref()?.settings.as_ref()?.app.clone());
        Some(typed).filter(|t| !t.is_empty()).or(front)
    }

    fn decide_permission(&mut self, key: &str, decision: Decision, cx: &mut Context<Self>) {
        let Some(permission) = self.devices.screens.get(key).and_then(|s| Some(s.tools.as_ref()?.permission)) else { return };
        if let Some(app) = self.permission_app(key, cx) {
            self.device_act(key, Action::Permission { app, permission, decision }, cx);
        }
    }

    fn make_fields(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let ios = self.devices.screens.get(key).is_some_and(|s| s.device.platform == Platform::Ios);
        let style = crate::ui::input_style(Theme::of(cx));
        let field = |placeholder: &str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| {
                let mut input = InputState::new(window, cx).placeholder(placeholder.to_string());
                input.set_editor_style(style.clone());
                input
            })
        };
        let url = field("https://… or myapp://", window, cx);
        let launch = field(if ios { "Bundle ID to launch" } else { "Package name to launch" }, window, cx);
        let lat = field("Latitude", window, cx);
        let lon = field("Longitude", window, cx);
        let app = field("App ID", window, cx);
        let push = field("Alert text", window, cx);
        let mut events = Vec::new();
        for (input, submit) in [(&url, Some(Submit::Url)), (&launch, Some(Submit::Launch)), (&lat, Some(Submit::Location)), (&lon, Some(Submit::Location)), (&app, None), (&push, Some(Submit::Push))] {
            let key = key.to_string();
            events.push(cx.subscribe_in(input, window, move |this, _, e: &InputEvent, window, cx| match e {
                InputEvent::PressEnter { .. } => {
                    if let Some(submit) = submit {
                        this.submit_tools_field(&key, submit, window, cx);
                    }
                }
                // the buttons beside it wake as it fills
                InputEvent::Change => cx.notify(),
                _ => {}
            }));
        }
        if let Some(tools) = self.devices.screens.get_mut(key).and_then(|s| s.tools.as_mut()) {
            tools.fields = Some(Fields { url, launch, lat, lon, app, push, app_placeholder: "App ID".into(), _events: events });
        }
    }

    /// Opens a select's options under its button.
    fn open_choice(&mut self, anchor: &str, options: Vec<SharedString>, current: Option<usize>, pick: impl Fn(&mut Shell, usize, &mut Window, &mut Context<Shell>) + 'static, press: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        let at = super::super::widgets::under(&self.anchors, anchor, press);
        self.devices.choice = Some(crate::ui::Popup::new(Choice { anchor: anchor.to_string(), at, corner: gpui::Anchor::TopLeft, options, current, pick: Rc::new(pick) }));
        cx.notify();
    }

    /// Whether the options open hang from button `anchor`.
    pub(super) fn choice_open(&self, anchor: &str) -> bool {
        self.devices.choice.as_ref().is_some_and(|c| c.is_open() && c.value.anchor == anchor)
    }

    /// The rail's Text size, t3code's: the sizes left of the button, the current one checked, read again as it opens.
    pub(super) fn open_text_size_menu(&mut self, key: &str, anchor: &str, press: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        let Some(screen) = self.devices.screens.get(key) else { return };
        let (device, known) = (screen.device.clone(), screen.text_size);
        let index = |size: Option<dt::TextSize>| size.and_then(|s| dt::TEXT_SIZES.iter().position(|(t, _)| *t == s));
        let at = self.anchors.borrow().get(anchor).map_or(press, |b| b.bottom_right() + gpui::point(px(0.), px(4.)));
        let k = key.to_string();
        let pick = move |s: &mut Shell, i: usize, _: &mut Window, cx: &mut Context<Shell>| s.device_act(&k, Action::TextSize(dt::TEXT_SIZES[i].0), cx);
        let options = dt::TEXT_SIZES.iter().map(|(_, label)| SharedString::from(*label)).collect();
        let choice = Choice { anchor: anchor.to_string(), at, corner: gpui::Anchor::TopRight, options, current: index(known), pick: Rc::new(pick) };
        self.devices.choice = Some(crate::ui::Popup::new(choice));
        let (key, anchor) = (key.to_string(), anchor.to_string());
        cx.spawn(async move |this, cx| {
            let Some(size) = cx.background_spawn(async move { dt::text_size(&device) }).await else { return };
            let _ = this.update(cx, |s, cx| {
                if let Some(screen) = s.devices.screens.get_mut(&key) {
                    screen.text_size = Some(size);
                }
                if s.choice_open(&anchor)
                    && let Some(c) = s.devices.choice.as_mut()
                {
                    c.value.current = index(Some(size));
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// The open select's options.
    pub(in crate::shell) fn device_choice(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.devices.choice.as_ref()?;
        let close = |this: &mut Shell| {
            if let Some(m) = this.devices.choice.as_mut() {
                m.close()
            }
        };
        let (at, corner) = (menu.value.at, menu.value.corner);
        let items = menu.value.options.iter().enumerate().map(|(i, label)| {
            let pick = menu.value.pick.clone();
            row(theme, false, SharedString::from(format!("device-choice-{i}")))
                .h(px(28.))
                .rounded(px(6.))
                .text_size(px(12.5))
                .child(div().flex_1().child(label.clone()))
                .when(menu.value.current == Some(i), |el| el.child(gpui::svg().path("icons/check.svg").size(px(14.)).text_color(theme.text_muted)))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    close(this);
                    pick(this, i, window, cx);
                    cx.notify();
                }))
        });
        let card = menu_card(theme, "device-choice", 160.)
            .on_mouse_down_out(cx.listener(move |this, _, _, cx| {
                close(this);
                cx.notify();
            }))
            .children(items);
        let menu = crate::ui::menu_motion("device-choice-in", menu, card);
        Some(gpui::deferred(gpui::anchored().position(at).anchor(corner).snap_to_window_with_margin(px(8.)).child(menu)).with_priority(1).into_any_element())
    }

    /// The panel, while it's open.
    pub(super) fn render_device_tools(&mut self, key: &str, theme: &Theme, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let screen = self.devices.screens.get(key)?;
        let tools = screen.tools.as_ref()?;
        if tools.fields.is_none() {
            self.make_fields(key, window, cx);
        }
        let screen = self.devices.screens.get(key)?;
        let tools = screen.tools.as_ref()?;
        let fields = tools.fields.as_ref()?;
        let ios = screen.device.platform == Platform::Ios;
        let settings = tools.settings.clone();
        let s = settings.clone().unwrap_or_default();
        let busy = tools.acting > 0 || settings.is_none();
        let app = s.app.clone();
        let permission = tools.permission;
        let overlay = screen.overlay.as_ref().map(|o| o.error.clone());
        // the permissions' app field says whose they'll be when left empty
        let placeholder = app.clone().unwrap_or_else(|| "App ID".into());
        if fields.app_placeholder != placeholder {
            fields.app.update(cx, |i, cx| i.set_placeholder(placeholder.clone(), window, cx));
            if let Some(f) = self.devices.screens.get_mut(key).and_then(|s| s.tools.as_mut()?.fields.as_mut()) {
                f.app_placeholder = placeholder;
            }
        }
        let screen = self.devices.screens.get(key)?;
        let tools = screen.tools.as_ref()?;
        let fields = tools.fields.as_ref()?;
        let (url, launch, lat, lon, app_field, push) = (fields.url.clone(), fields.launch.clone(), fields.lat.clone(), fields.lon.clone(), fields.app.clone(), fields.push.clone());
        let filled = |input: &Entity<InputState>| !input.read(cx).value().trim().is_empty();
        let (url_filled, launch_filled, push_filled) = (filled(&url), filled(&launch), filled(&push));
        let location_ok = self.field_text(key, |f| &f.lat, cx).parse::<f64>().is_ok_and(|l| l.abs() <= 90.) && self.field_text(key, |f| &f.lon, cx).parse::<f64>().is_ok_and(|l| l.abs() <= 180.);
        let permission_app = self.permission_app(key, cx).is_some();
        let error = tools.error.clone();

        let k = key.to_string();

        let mut body = div().flex().flex_col();
        if let Some(e) = error {
            body = body.child(div().px(px(12.)).py(px(8.)).border_b_1().border_color(theme.border).bg(theme.danger.opacity(0.08)).text_size(px(12.)).text_color(theme.danger).child(e));
        }
        if settings.is_none() {
            body = body.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .px(px(12.))
                    .py(px(10.))
                    .text_size(px(12.))
                    .text_color(theme.text_muted)
                    .child(spinner(theme, format!("device-tools-reading-{key}")))
                    .child("Reading device settings…"),
            );
        }

        // App
        let mut app_section = section(theme, "App").child(row_of(theme, "Foreground", div().truncate().font_family("Geist Mono").text_size(px(12.)).child(app.clone().unwrap_or_else(|| "–".into()))));
        if let Some(app) = &app {
            app_section = app_section.child(
                div()
                    .flex()
                    .gap(px(6.))
                    .child(small_button(theme, format!("tools-terminate-{key}"), "Terminate", true).on_click(on_act(cx, key, Action::Terminate(app.clone()))))
                    .child(small_button(theme, format!("tools-relaunch-{key}"), "Relaunch", true).on_click(on_act(cx, key, Action::Launch(app.clone())))),
            );
        }
        app_section = app_section
            .child(submit_row(theme, &url, small_button(theme, format!("tools-open-{key}"), "Open", url_filled).when(url_filled, |el| el.on_click(on_submit(cx, key, Submit::Url)))))
            .child(submit_row(theme, &launch, small_button(theme, format!("tools-launch-{key}"), "Launch", launch_filled).when(launch_filled, |el| el.on_click(on_submit(cx, key, Submit::Launch)))));
        body = body.child(app_section);

        // the device's own settings
        let mut device_section = section(theme, if ios { "Simulator" } else { "Emulator" });
        if let Some(dark) = s.dark {
            device_section = device_section.child(row_of(
                theme,
                "Appearance",
                segmented(theme, &format!("tools-appearance-{key}"), &[("Light", !dark), ("Dark", dark)], {
                    let k = k.clone();
                    cx.listener(move |this, i: &usize, _, cx| this.device_act(&k, Action::Dark(*i == 1), cx))
                }),
            ));
        }
        if let Some(size) = s.text_size {
            let current = dt::TEXT_SIZES.iter().position(|(t, _)| *t == size);
            let options = dt::TEXT_SIZES.iter().map(|(_, l)| SharedString::from(*l)).collect();
            device_section = device_section.child(row_of(theme, "Text size", self.select(theme, key, "text-size", options, current, None, |s, k, i, _, cx| s.device_act(k, Action::TextSize(dt::TEXT_SIZES[i].0), cx), cx)));
        }
        if let Some(look) = s.liquid_glass {
            let options: Vec<(&str, bool)> = dt::LIQUID_GLASS.iter().map(|(v, l)| (*l, *v == look)).collect();
            device_section = device_section.child(row_of(
                theme,
                "Liquid Glass",
                segmented(theme, &format!("tools-glass-{key}"), &options, {
                    let k = k.clone();
                    cx.listener(move |this, i: &usize, _, cx| this.device_act(&k, Action::LiquidGlass(dt::LIQUID_GLASS[*i].0), cx))
                }),
            ));
        }
        if let Some(filter) = s.color_filter {
            let current = dt::COLOR_FILTERS.iter().position(|(v, _)| *v == filter);
            let options = dt::COLOR_FILTERS.iter().map(|(_, l)| SharedString::from(*l)).collect();
            device_section = device_section.child(row_of(theme, "Color filter", self.select(theme, key, "color-filter", options, current, None, |s, k, i, _, cx| s.device_act(k, Action::ColorFilter(dt::COLOR_FILTERS[i].0), cx), cx)));
        }
        if settings.is_some() {
            // t3code's too: the orientation isn't read back, the select turns it
            let options = dt::ORIENTATIONS.iter().map(|(_, l)| SharedString::from(*l)).collect();
            device_section = device_section.child(row_of(theme, "Orientation", self.select(theme, key, "orientation", options, None, Some("Rotate to…"), |s, k, i, _, cx| s.turn_device(k, i, cx), cx)));
        }
        for (toggle, on) in s.toggles.iter().copied() {
            device_section = device_section.child(switch_row(theme, format!("tools-{toggle:?}-{key}"), toggle.label(), on).on_click(on_act(cx, key, Action::Toggle(toggle, !on))));
        }
        if settings.is_some() {
            body = body.child(device_section);
        }

        body = body.child(
            section(theme, "Accessibility")
                .child(switch_row(theme, format!("tools-overlay-{key}"), "Overlay element frames", overlay.is_some()).on_click({
                    let k = k.clone();
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle_device_overlay(&k, cx))
                }))
                .children(overlay.flatten().map(|e| div().text_size(px(12.)).text_color(theme.text_muted).child(e))),
        );

        // Location
        let presets = dt::LOCATIONS.iter().map(|(l, ..)| SharedString::from(*l)).collect();
        let preset = self.select(
            theme,
            key,
            "location",
            presets,
            None,
            Some("Preset…"),
            |s, k, i, window, cx| {
                let (_, lat, lon) = dt::LOCATIONS[i];
                // the fields show where it was put
                if let Some(f) = s.devices.screens.get(k).and_then(|sc| sc.tools.as_ref()?.fields.as_ref()) {
                    let (la, lo) = (f.lat.clone(), f.lon.clone());
                    la.update(cx, |i, cx| i.set_value(lat.to_string(), window, cx));
                    lo.update(cx, |i, cx| i.set_value(lon.to_string(), window, cx));
                }
                s.device_act(k, Action::Location(lat, lon), cx);
            },
            cx,
        );
        body = body.child(
            section(theme, "Location").child(div().flex().gap(px(6.)).child(field(theme, &lat)).child(field(theme, &lon))).child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.))
                    .child(preset)
                    .child(small_button(theme, format!("tools-location-set-{key}"), "Set", location_ok).when(location_ok, |el| el.on_click(on_submit(cx, key, Submit::Location))))
                    .when(ios, |el| {
                        el.child(ghost_small(theme, format!("tools-location-clear-{key}"), "Clear").on_click({
                            let k = k.clone();
                            cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.take_field(&k, |f| &f.lat, window, cx);
                                this.take_field(&k, |f| &f.lon, window, cx);
                                this.device_act(&k, Action::ClearLocation, cx);
                            })
                        }))
                    }),
            ),
        );

        // Permissions
        let list = dt::permissions(screen.device.platform);
        let current = list.iter().position(|(v, _)| *v == permission);
        let options = list.iter().map(|(_, l)| SharedString::from(*l)).collect();
        let pick = self.select(
            theme,
            key,
            "permission",
            options,
            current,
            None,
            move |s, k, i, _, cx| {
                if let Some(t) = s.devices.screens.get_mut(k).and_then(|sc| sc.tools.as_mut()) {
                    t.permission = list[i].0;
                }
                cx.notify();
            },
            cx,
        );
        body = body.child(
            section(theme, "Permissions").child(field(theme, &app_field)).child(row_of(theme, "Permission", pick)).child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.))
                    .child(small_button(theme, format!("tools-grant-{key}"), "Grant", permission_app).when(permission_app, |el| el.on_click(on_decide(cx, key, Decision::Grant))))
                    .child(small_button(theme, format!("tools-revoke-{key}"), "Revoke", permission_app).when(permission_app, |el| el.on_click(on_decide(cx, key, Decision::Revoke))))
                    .when(ios, |el| el.child(ghost_small(theme, format!("tools-reset-{key}"), "Reset").when(permission_app, |el| el.on_click(on_decide(cx, key, Decision::Reset))))),
            ),
        );

        if ios {
            body = body.child(
                section(theme, "Push notification")
                    .child(submit_row(theme, &push, small_button(theme, format!("tools-push-{key}"), "Send", push_filled && app.is_some()).when(push_filled && app.is_some(), |el| el.on_click(on_submit(cx, key, Submit::Push)))))
                    .when(app.is_none(), |el| el.child(div().text_size(px(12.)).text_color(theme.text_muted).child("Open an app first."))),
            );
        }

        let header = div()
            .h(px(36.))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(12.))
            .border_b_1()
            .border_color(theme.border)
            .child(div().text_size(px(13.)).font_weight(gpui::FontWeight::MEDIUM).child("Tools"))
            .when(busy, |el| el.child(spinner(theme, format!("device-tools-busy-{key}"))))
            .child(div().flex_1())
            .child(icon_button(theme, "device-tools-close", "×").tooltip(tip("Close tools", None)).on_click({
                let k = k.clone();
                cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle_device_tools(&k, cx))
            }));
        let panel = div()
            .w(px(WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .text_color(theme.text)
            .child(header)
            .child(div().id(SharedString::from(format!("device-tools-{key}"))).flex_1().min_h_0().overflow_y_scroll().child(body));
        // slides out from the screen's right edge
        Some(
            div()
                .flex_none()
                .h_full()
                .overflow_hidden()
                .child(panel)
                .with_animation(SharedString::from(format!("device-tools-in-{key}")), gpui::Animation::new(Duration::from_millis(180)).with_easing(super::super::motion::ease_out), |el, t| el.w(px(WIDTH * t)))
                .into_any_element(),
        )
    }

    /// A select: its button shows the current option (or `placeholder`), a press lists them all.
    #[allow(clippy::too_many_arguments)]
    fn select(
        &self,
        theme: &Theme,
        key: &str,
        name: &str,
        options: Vec<SharedString>,
        current: Option<usize>,
        placeholder: Option<&'static str>,
        pick: impl Fn(&mut Shell, &str, usize, &mut Window, &mut Context<Shell>) + Clone + 'static,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let anchor = format!("device-select-{name}-{key}");
        let label = current.and_then(|i| options.get(i).cloned());
        let hover = theme.hover;
        let k = key.to_string();
        div()
            .id(SharedString::from(anchor.clone()))
            .relative()
            .h(px(24.))
            .w(px(150.))
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(4.))
            .rounded(px(6.))
            .border_1()
            .border_color(theme.border)
            .text_size(px(12.))
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .child(super::super::widgets::anchor_probe(&self.anchors, anchor.clone()))
            .child(match label {
                Some(l) => div().flex_1().min_w_0().truncate().child(l),
                None => div().flex_1().min_w_0().truncate().text_color(theme.text_muted).child(placeholder.unwrap_or("Unknown")),
            })
            .child(gpui::svg().path("icons/alt-arrow-down.svg").size(px(12.)).flex_none().text_color(theme.text_muted))
            .on_click(cx.listener(move |this, e: &ClickEvent, _, cx| {
                let (k, pick) = (k.clone(), pick.clone());
                this.open_choice(&anchor, options.clone(), current, move |s, i, window, cx| pick(s, &k, i, window, cx), e.position(), cx);
            }))
    }
}

fn on_act(cx: &Context<Shell>, key: &str, action: Action) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    let key = key.to_string();
    cx.listener(move |this: &mut Shell, _: &ClickEvent, _, cx| this.device_act(&key, action.clone(), cx))
}

fn on_submit(cx: &Context<Shell>, key: &str, which: Submit) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    let key = key.to_string();
    cx.listener(move |this: &mut Shell, _: &ClickEvent, window, cx| this.submit_tools_field(&key, which, window, cx))
}

fn on_decide(cx: &Context<Shell>, key: &str, decision: Decision) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    let key = key.to_string();
    cx.listener(move |this: &mut Shell, _: &ClickEvent, _, cx| this.decide_permission(&key, decision, cx))
}

#[derive(Clone, Copy)]
enum Submit {
    Url,
    Launch,
    Location,
    Push,
}

/// Reads the device every `POLL` while its panel is open.
fn poll_tools(key: String, cx: &mut Context<Shell>) -> Task<()> {
    cx.spawn(async move |this, cx| {
        loop {
            let job = this.update(cx, |s, _| {
                let sc = s.devices.screens.get(&key)?;
                Some((sc.tools.as_ref()?.serial, sc.device.clone(), sc.probe.clone()))
            });
            let Ok(Some((serial, device, probe))) = job else { break };
            let settings = cx.background_spawn(async move { dt::read(&device, &probe) }).await;
            if this.update(cx, |s, cx| s.settle_tools(&key, serial, settings, cx)).is_err() {
                break;
            }
            cx.background_executor().timer(POLL).await;
        }
    })
}

/// Reads the elements every `OVERLAY_POLL` while the overlay's on.
fn poll_overlay(key: String, cx: &mut Context<Shell>) -> Task<()> {
    cx.spawn(async move |this, cx| {
        loop {
            let job = this.update(cx, |s, _| s.devices.screens.get(&key).filter(|sc| sc.overlay.is_some()).map(|sc| (sc.device.clone(), sc.probe.clone())));
            let Ok(Some((device, probe))) = job else { break };
            let read = cx.background_spawn(async move { dt::elements(&device, &probe) }).await;
            let settled = this.update(cx, |s, cx| {
                if let Some(o) = s.devices.screens.get_mut(&key).and_then(|sc| sc.overlay.as_mut()) {
                    match read {
                        Ok(frames) => (o.frames, o.error) = (frames, None),
                        Err(e) => o.error = Some(format!("{e:#}")),
                    }
                    cx.notify();
                }
            });
            if settled.is_err() {
                break;
            }
            cx.background_executor().timer(OVERLAY_POLL).await;
        }
    })
}

fn section(theme: &Theme, title: &'static str) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .px(px(12.))
        .py(px(10.))
        .border_b_1()
        .border_color(theme.border)
        .child(div().text_size(px(12.)).font_weight(gpui::FontWeight::MEDIUM).text_color(theme.text_muted).child(title))
}

fn row_of(theme: &Theme, label: &'static str, control: impl IntoElement) -> Div {
    div()
        .min_h(px(26.))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .child(div().flex_none().text_size(px(12.)).text_color(theme.text_muted).child(label))
        .child(div().min_w_0().flex().justify_end().child(control))
}

fn switch_row(theme: &Theme, id: String, label: &'static str, on: bool) -> Stateful<Div> {
    let id = SharedString::from(id);
    div()
        .id(id.clone())
        .min_h(px(26.))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .cursor_pointer()
        .child(div().text_size(px(12.)).text_color(theme.text_muted).child(label))
        .child(crate::ui::switch(theme, on, id))
}

/// t3code's small outline button; a dimmed one does nothing.
fn small_button(theme: &Theme, id: String, label: &'static str, enabled: bool) -> Stateful<Div> {
    let hover = theme.hover;
    let el = div()
        .id(SharedString::from(id))
        .h(px(24.))
        .px(px(8.))
        .flex_none()
        .flex()
        .items_center()
        .rounded(px(6.))
        .border_1()
        .border_color(theme.border)
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .child(label);
    match enabled {
        true => el.cursor_pointer().hover(move |s| s.bg(hover)),
        false => el.opacity(0.4),
    }
}

fn ghost_small(theme: &Theme, id: String, label: &'static str) -> Stateful<Div> {
    let hover = theme.hover;
    div().id(SharedString::from(id)).h(px(24.)).px(px(8.)).flex().items_center().rounded(px(6.)).text_size(px(12.)).text_color(theme.text_muted).cursor_pointer().hover(move |s| s.bg(hover)).child(label)
}

fn field(theme: &Theme, input: &Entity<InputState>) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .h(px(26.))
        .px(px(8.))
        .flex()
        .items_center()
        .rounded(px(6.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.wash(0.03))
        .font_family("Geist Mono")
        .text_size(px(12.))
        .child(Input::new(input))
}

fn submit_row(theme: &Theme, input: &Entity<InputState>, button: Stateful<Div>) -> Div {
    div().flex().items_center().gap(px(6.)).child(field(theme, input)).child(button)
}

/// t3code's toggle group: the options side by side, the current one filled; a press gives its index.
fn segmented(theme: &Theme, id: &str, options: &[(&'static str, bool)], pick: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Div {
    let pick = Rc::new(pick);
    let (on_bg, hover) = (theme.wash(0.10), theme.hover);
    div().flex().p(px(2.)).gap(px(2.)).rounded(px(7.)).border_1().border_color(theme.border).children(options.iter().enumerate().map(|(i, (label, on))| {
        let pick = pick.clone();
        div()
            .id(SharedString::from(format!("{id}-{i}")))
            .h(px(20.))
            .px(px(8.))
            .flex()
            .items_center()
            .rounded(px(5.))
            .text_size(px(12.))
            .cursor_pointer()
            .map(|el| match on {
                true => el.bg(on_bg).font_weight(gpui::FontWeight::MEDIUM),
                false => el.text_color(theme.text_muted).hover(move |s| s.bg(hover)),
            })
            .child(*label)
            .on_click(move |_, window, cx| pick(&i, window, cx))
    }))
}
