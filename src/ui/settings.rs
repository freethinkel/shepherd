//! The Settings window: terminal font, appearance and theme family, herdr's binary and socket, key bindings.
//! Every change is saved at once and applied to all windows.
use crate::bindings::{ACTIONS, effective, rebind, recordable};
use crate::herdr::{Client, set_binary};
use crate::settings::{Settings, SettingsGlobal, step_font_size};
use crate::theme::{AppearanceMode, Theme, families};
use crate::ui::input_style;
use gpui::{
    App, Bounds, ClickEvent, Context, Div, Entity, FocusHandle, Focusable, Global, IntoElement, KeystrokeEvent,
    ParentElement, Render, SharedString, Stateful, Styled, Subscription, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowOptions, div, point, prelude::*, px, size,
};
use gpui_base::input::{Input, InputEvent, InputState};

/// The app's tokio runtime, for calls made from windows (the herdr connection test).
pub struct Tokio(pub tokio::runtime::Handle);

impl Global for Tokio {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Terminal,
    Appearance,
    Herdr,
    Keys,
}

impl Section {
    const ALL: [Section; 4] = [Section::Terminal, Section::Appearance, Section::Herdr, Section::Keys];

    fn label(self) -> &'static str {
        match self {
            Section::Terminal => "Terminal",
            Section::Appearance => "Appearance",
            Section::Herdr => "herdr",
            Section::Keys => "Keys",
        }
    }
}

pub struct SettingsView {
    section: Section,
    font: Entity<InputState>,
    herdr_bin: Entity<InputState>,
    herdr_socket: Entity<InputState>,
    /// The action whose chord the next keystroke becomes.
    recording: Option<&'static str>,
    ping: Option<String>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

/// Opens Settings, or brings the open one forward.
pub fn open(cx: &mut App) {
    if let Some(window) = cx.windows().into_iter().find_map(|w| w.downcast::<SettingsView>()) {
        let _ = window.update(cx, |_, window, _| window.activate_window());
        return;
    }
    let options = WindowOptions {
        window_bounds: Some(gpui::WindowBounds::Windowed(Bounds::centered(None, size(px(720.), px(580.)), cx))),
        titlebar: Some(TitlebarOptions {
            title: Some("Settings".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(14.), px(14.))),
        }),
        window_background: WindowBackgroundAppearance::Blurred,
        ..Default::default()
    };
    let _ = cx.open_window(options, |window, cx| cx.new(|cx| SettingsView::new(window, cx)));
}

/// Changes the settings, saves them and applies them everywhere.
pub fn update(cx: &mut App, change: impl FnOnce(&mut Settings)) {
    // Nothing changed, nothing saved: leaving a field must not overwrite a file that failed to load.
    let Some(settings) = SettingsGlobal::of(cx).settings.changed_by(change) else { return };
    let keys_changed = settings.keys != SettingsGlobal::of(cx).settings.keys;
    let path = {
        let global = cx.global_mut::<SettingsGlobal>();
        global.settings = settings.clone();
        global.path.clone()
    };
    let saved = settings.save(&path);
    cx.global_mut::<SettingsGlobal>().error = saved.err().map(|e| format!("cannot save {}: {e:#}", path.display()));
    set_binary(settings.herdr_bin.clone());
    cx.set_global(settings.theme(cx.window_appearance()));
    if keys_changed {
        rebind(cx, &settings.keys);
    }
    cx.refresh_windows();
}

impl SettingsView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings = SettingsGlobal::of(cx).settings.clone();
        let theme = Theme::of(cx).clone();
        let field = |value: Option<String>, placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| {
                let mut input = InputState::new(window, cx).placeholder(placeholder).default_value(value.unwrap_or_default());
                input.set_editor_style(input_style(&theme));
                input
            })
        };
        let font = field(Some(settings.font_family.clone()), "Geist Mono", window, cx);
        let herdr_bin = field(settings.herdr_bin.clone(), "herdr on PATH", window, cx);
        let herdr_socket = field(settings.herdr_socket.clone(), "~/.config/herdr/herdr.sock", window, cx);

        let mut subscriptions = Vec::new();
        for (input, apply) in [
            (&font, (|s: &mut Settings, v: String| if !v.is_empty() { s.font_family = v }) as fn(&mut Settings, String)),
            (&herdr_bin, |s: &mut Settings, v: String| s.herdr_bin = (!v.is_empty()).then_some(v)),
            (&herdr_socket, |s: &mut Settings, v: String| s.herdr_socket = (!v.is_empty()).then_some(v)),
        ] {
            subscriptions.push(cx.subscribe_in(input, window, move |_, input, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    let value = input.read(cx).value().trim().to_string();
                    update(cx, |s| apply(s, value));
                }
            }));
        }
        // While recording, the next keystroke is the chord: intercepted before any binding (Cmd+Q included) runs.
        let view = cx.entity().downgrade();
        let settings_window = window.window_handle();
        subscriptions.push(cx.intercept_keystrokes(move |event: &KeystrokeEvent, window, cx| {
            // Only keys pressed in this window: typing in a terminal while Record is armed stays typing.
            if window.window_handle() != settings_window {
                return;
            }
            let Some(view) = view.upgrade() else { return };
            let Some(action) = view.read(cx).recording else { return };
            let keystroke = &event.keystroke;
            let bare = keystroke.modifiers == Default::default();
            match keystroke.key.as_str() {
                "escape" if bare => {}
                "backspace" if bare => update(cx, |s| s.record_chord(action, "")),
                _ if recordable(keystroke) => {
                    let chord = keystroke.unparse();
                    update(cx, |s| s.record_chord(action, &chord));
                }
                _ => return, // a bare key: keep waiting for a chord
            }
            cx.stop_propagation();
            view.update(cx, |v, cx| {
                v.recording = None;
                cx.notify();
            });
        }));
        // SHEPHERD_SETTINGS=<section> opens a section at launch, for screenshots (like Collie's COLLIE_APP_PAGE).
        let section = std::env::var("SHEPHERD_SETTINGS")
            .ok()
            .and_then(|name| Section::ALL.into_iter().find(|s| s.label().eq_ignore_ascii_case(&name)))
            .unwrap_or(Section::Terminal);
        SettingsView {
            section,
            font,
            herdr_bin,
            herdr_socket,
            recording: None,
            ping: None,
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }

    fn test_connection(&mut self, cx: &mut Context<Self>) {
        let socket = self.herdr_socket.read(cx).value().trim().to_string();
        let client = if socket.is_empty() { Client::local() } else { Client::at(socket.into()) };
        let rt = cx.global::<Tokio>().0.clone();
        let call = rt.spawn(async move { client.call("ping", serde_json::json!({})).await.map(|_| client) });
        self.ping = Some("Connecting…".into());
        cx.spawn(async move |this, cx| {
            let text = match call.await {
                Ok(Ok(client)) => format!("Connected to herdr at {}", client.path().display()),
                Ok(Err(err)) => format!("{err:#}"),
                Err(err) => format!("{err}"),
            };
            let _ = this.update(cx, |v, cx| {
                v.ping = Some(text);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

impl Focusable for SettingsView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

fn heading(theme: &Theme, text: &'static str) -> Div {
    div().text_size(px(11.)).text_color(theme.text_faint).mt(px(14.)).mb(px(6.)).child(text)
}

fn field_box(theme: &Theme) -> Div {
    div().h(px(32.)).px(px(10.)).flex().items_center().rounded(px(8.)).bg(theme.wash(0.06)).border_1().border_color(theme.border)
}

fn button(theme: &Theme, id: impl Into<gpui::ElementId>, label: impl Into<SharedString>) -> Stateful<Div> {
    let hover = theme.hover;
    div()
        .id(id)
        .h(px(28.))
        .px(px(10.))
        .flex()
        .items_center()
        .rounded(px(7.))
        .bg(theme.wash(0.08))
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(label.into())
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let global = SettingsGlobal::of(cx);
        let settings = global.settings.clone();
        let error = global.error.clone();

        let nav = div()
            .w(px(180.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(2.))
            .pt(px(44.))
            .px(px(8.))
            .bg(theme.wash(0.05))
            .border_r_1()
            .border_color(theme.border)
            .children(Section::ALL.into_iter().map(|section| {
                crate::shell::row(&theme, self.section == section, SharedString::from(section.label()))
                    .child(section.label())
                    .on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                        v.section = section;
                        cx.notify();
                    }))
            }));

        let body = match self.section {
            Section::Terminal => div()
                .child(heading(&theme, "Font family"))
                .child(field_box(&theme).child(Input::new(&self.font)))
                .child(
                    div()
                        .mt(px(6.))
                        .text_size(px(12.))
                        .text_color(theme.text_faint)
                        .child("Any installed monospace font. Icons missing from it come from installed Nerd Fonts."),
                )
                .child(heading(&theme, "Font size"))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(button(&theme, "size-down", "−").on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                            update(cx, |s| s.font_size = step_font_size(s.font_size, -1.0))
                        })))
                        .child(div().w(px(40.)).flex().justify_center().child(format!("{}", settings.font_size)))
                        .child(button(&theme, "size-up", "+").on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                            update(cx, |s| s.font_size = step_font_size(s.font_size, 1.0))
                        }))),
                ),
            Section::Appearance => {
                let modes = [(AppearanceMode::System, "System"), (AppearanceMode::Light, "Light"), (AppearanceMode::Dark, "Dark")];
                let system = window.appearance();
                div()
                    .child(heading(&theme, "Appearance"))
                    .child(div().flex().gap(px(4.)).children(modes.into_iter().map(|(mode, label)| {
                        let selected = settings.appearance == mode;
                        button(&theme, SharedString::from(format!("mode-{label}")), label)
                            .when(selected, |el| el.bg(theme.selected).border_1().border_color(theme.accent.opacity(0.6)))
                            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| update(cx, |s| s.appearance = mode)))
                    })))
                    .child(heading(&theme, "Theme"))
                    .child(div().flex().flex_col().gap(px(2.)).children(families().into_iter().map(|(id, name)| {
                        let preview = Theme::resolve(&id, settings.appearance, system, &settings.font());
                        let selected = settings.theme_family == id;
                        let swatches = std::iter::once(preview.terminal.background)
                            .chain(preview.terminal.ansi[1..7].iter().copied())
                            .map(|c| div().size(px(12.)).rounded(px(3.)).bg(c).border_1().border_color(theme.border));
                        crate::shell::row(&theme, selected, SharedString::from(format!("family-{id}")))
                            .child(div().flex_1().child(name))
                            .child(div().flex().gap(px(3.)).children(swatches))
                            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                                let id = id.clone();
                                update(cx, move |s| s.theme_family = id)
                            }))
                    })))
            }
            Section::Herdr => div()
                .child(heading(&theme, "herdr binary"))
                .child(field_box(&theme).child(Input::new(&self.herdr_bin)))
                .child(heading(&theme, "API socket"))
                .child(field_box(&theme).child(Input::new(&self.herdr_socket)))
                .child(
                    div()
                        .mt(px(6.))
                        .text_size(px(12.))
                        .text_color(theme.text_faint)
                        .child("A new socket is used after Shepherd restarts. Empty fields use herdr's defaults."),
                )
                .child(
                    div()
                        .mt(px(14.))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(button(&theme, "ping", "Test connection").on_click(cx.listener(|v, _: &ClickEvent, _, cx| v.test_connection(cx))))
                        .children(self.ping.clone().map(|text| div().text_size(px(12.)).text_color(theme.text_muted).child(text))),
                ),
            Section::Keys => {
                let bindings = effective(&settings.keys);
                div()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .child(heading(&theme, "Key bindings").flex_1())
                            .child(button(&theme, "reset-keys", "Reset to defaults").on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                                update(cx, |s| s.keys.clear())
                            }))),
                    )
                    .children(ACTIONS.iter().map(|(action, label)| {
                        let chord = bindings.iter().find(|(a, _)| a == action).map(|(_, k)| k.clone());
                        let recording = self.recording == Some(*action);
                        let action = *action;
                        div()
                            .h(px(34.))
                            .flex()
                            .items_center()
                            .border_b_1()
                            .border_color(theme.border)
                            .child(div().flex_1().child(*label))
                            .child(
                                div()
                                    .min_w(px(120.))
                                    .text_color(if recording { theme.accent } else { theme.text_muted })
                                    .child(if recording {
                                        "Press keys… (Esc cancels, ⌫ unbinds)".to_string()
                                    } else {
                                        chord.unwrap_or_else(|| "—".into())
                                    }),
                            )
                            .child(button(&theme, SharedString::from(format!("record-{action}")), "Record").on_click(cx.listener(
                                move |v, _: &ClickEvent, window, cx| {
                                    v.recording = Some(action);
                                    window.focus(&v.focus, cx);
                                    cx.notify();
                                },
                            )))
                    }))
            }
        };

        div()
            .id("settings")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .bg(theme.glass())
            .text_color(theme.text)
            .font_family(theme.font_ui.clone())
            .text_size(px(13.))
            .child(nav)
            .child(
                div()
                    .id("settings-body")
                    .flex_1()
                    .h_full()
                    .overflow_y_scroll()
                    .pt(px(30.))
                    .px(px(24.))
                    .pb(px(24.))
                    .bg(theme.card.opacity(0.6))
                    .children(error.map(|e| {
                        div().mb(px(10.)).px_3().py_2().rounded(px(8.)).border_1().border_color(theme.danger.opacity(0.5)).text_color(theme.danger).child(e)
                    }))
                    .child(body),
            )
    }
}
