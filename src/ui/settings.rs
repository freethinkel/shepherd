//! The Settings page, shown in the main window in place of the sidebar and the panes (Zeron's settings route):
//! section tabs where the sidebar was, then a centered column of titled cards. Terminal font, appearance and
//! theme family, herdr's binary and socket, key bindings. Every change is saved at once and applied to all windows.
use crate::bindings::{ACTIONS, effective, rebind, recordable};
use crate::herdr::{Client, set_binary, set_socket};
use crate::settings::{Settings, SettingsGlobal, step_font_size, step_line_height};
use crate::theme::{AppearanceMode, Theme, families};
use crate::ui::input_style;
use gpui::{
    App, ClickEvent, Context, Div, Entity, EventEmitter, FocusHandle, Focusable, FontWeight, Global, IntoElement,
    KeyDownEvent, Keystroke, KeystrokeEvent, ParentElement, Render, SharedString, Stateful, Styled, Subscription,
    Window, div, prelude::*, px,
};
use gpui_base::input::{Input, InputEvent, InputState};

/// The app's tokio runtime, for calls made from windows (the herdr connection test).
pub struct Tokio(pub tokio::runtime::Handle);

impl Global for Tokio {}

const TITLEBAR_HEIGHT: f32 = 38.0;
/// Zeron's page column: a centered measure with air on either side.
const PAGE_MAX_WIDTH: f32 = 760.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Terminal,
    Appearance,
    Herdr,
    Shortcuts,
}

impl Section {
    const ALL: [Section; 4] = [Section::Terminal, Section::Appearance, Section::Herdr, Section::Shortcuts];

    fn label(self) -> &'static str {
        match self {
            Section::Terminal => "Terminal",
            Section::Appearance => "Appearance",
            Section::Herdr => "herdr",
            Section::Shortcuts => "Shortcuts",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Section::Terminal => "icons/terminal.svg",
            Section::Appearance => "icons/tuning.svg",
            Section::Herdr => "icons/remote-server.svg",
            Section::Shortcuts => "icons/keyboard.svg",
        }
    }
}

/// Back, or Escape: the window shows the panes again.
pub struct Close;

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

impl EventEmitter<Close> for SettingsView {}

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
    set_socket(settings.herdr_socket.clone());
    cx.set_global(settings.theme(cx.window_appearance()));
    if keys_changed {
        rebind(cx, &settings.keys);
    }
    cx.refresh_windows();
}

impl SettingsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
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
            // Only keys pressed in this window: typing in another window's terminal while Record is armed stays typing.
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

/// Page headline and its muted subtitle.
fn page_header(theme: &Theme, title: &'static str, subtitle: &'static str) -> Div {
    div()
        .px(px(8.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(div().text_size(px(20.)).line_height(px(26.)).font_weight(FontWeight::MEDIUM).child(title))
        .child(div().text_color(theme.text_muted).line_height(px(17.)).child(subtitle))
}

/// The small plain label above a card.
fn section_label(theme: &Theme, label: &'static str) -> Div {
    div().mt(px(32.)).mb(px(8.)).px(px(8.)).text_color(theme.text_muted).child(label)
}

/// A filled, rounded block of related rows: a faint wash, no outline.
fn card(theme: &Theme) -> Div {
    div().mt(px(24.)).rounded(px(12.)).bg(theme.wash(0.045)).overflow_hidden().flex().flex_col()
}

/// One row of a card: text on the left, the control on the right, a hairline above all but the first.
fn card_row(theme: &Theme, first: bool) -> Div {
    div()
        .mx(px(16.))
        .py(px(12.))
        .min_h(px(60.))
        .when(!first, |el| el.border_t_1().border_color(theme.border.opacity(0.6)))
        .flex()
        .items_center()
        .gap(px(16.))
}

/// A row's title and, under it, its description.
fn row_text(theme: &Theme, title: impl Into<SharedString>, description: impl Into<SharedString>) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(div().font_weight(FontWeight::MEDIUM).line_height(px(17.)).child(title.into()))
        .child(div().text_size(px(12.)).line_height(px(16.)).text_color(theme.text_muted).child(description.into()))
}

/// Zeron's filled action: a borderless pill that lifts on hover.
fn action(theme: &Theme, id: impl Into<gpui::ElementId>, label: impl Into<SharedString>) -> Stateful<Div> {
    let lifted = theme.wash(0.10);
    div()
        .id(id)
        .flex_none()
        .min_h(px(32.))
        .px(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(8.))
        .bg(theme.wash(0.06))
        .text_size(px(12.5))
        .cursor_pointer()
        .hover(move |s| s.bg(lifted))
        .child(label.into())
}

/// A text field in a row: the filled pill of Zeron's selects.
fn field(theme: &Theme, input: &Entity<InputState>) -> Div {
    div().w(px(240.)).flex_none().h(px(32.)).px(px(10.)).flex().items_center().rounded(px(8.)).bg(theme.wash(0.06)).child(Input::new(input))
}

/// A chord as Zeron shows it: `cmd-shift-t` → `Cmd+Shift+T`.
pub(crate) fn display_chord(chord: &str) -> String {
    let Ok(k) = Keystroke::parse(chord) else { return chord.to_string() };
    let m = k.modifiers;
    let key = match k.key.as_str() {
        key if key.chars().count() == 1 => key.to_uppercase(),
        key => key[..1].to_uppercase() + &key[1..],
    };
    [(m.control, "Ctrl"), (m.alt, "Opt"), (m.shift, "Shift"), (m.platform, "Cmd"), (m.function, "Fn")]
        .into_iter()
        .filter_map(|(on, name)| on.then_some(name.to_string()))
        .chain(std::iter::once(key))
        .collect::<Vec<_>>()
        .join("+")
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let global = SettingsGlobal::of(cx);
        let settings = global.settings.clone();
        let error = global.error.clone();

        let nav = div()
            .w(px(settings.sidebar_width))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .pt(px(TITLEBAR_HEIGHT))
            .bg(theme.wash(0.05))
            .border_r_1()
            .border_color(theme.border)
            // Zeron's Back: a muted tab at the top of the column, an arrow and the word.
            .child(
                div().px(px(8.)).child(
                    crate::shell::row(&theme, false, "settings-back")
                        .text_color(theme.text_muted)
                        .child(gpui::svg().path("icons/arrow-left.svg").size(px(16.)).flex_none().text_color(theme.text_muted))
                        .child("Back")
                        .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(Close))),
                ),
            )
            .child(div().flex_1().min_h_0().flex().flex_col().gap(px(2.)).px(px(8.)).pt(px(12.)).children(Section::ALL.into_iter().map(
                |section| {
                    let selected = self.section == section;
                    crate::shell::row(&theme, selected, SharedString::from(format!("settings-{}", section.label())))
                        .text_color(if selected { theme.text } else { theme.text_muted })
                        .when(selected, |el| el.font_weight(FontWeight::MEDIUM))
                        .child(gpui::svg().path(section.icon()).size(px(16.)).flex_none().text_color(theme.text_muted))
                        .child(section.label())
                        .on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                            v.section = section;
                            v.recording = None;
                            cx.notify();
                        }))
                },
            )));

        let body = match self.section {
            Section::Terminal => div()
                .child(page_header(&theme, "Terminal", "The font every pane draws with."))
                .child(
                    card(&theme)
                        .child(
                            card_row(&theme, true)
                                .child(row_text(
                                    &theme,
                                    "Font family",
                                    "Any installed monospace font. Icons missing from it come from installed Nerd Fonts.",
                                ))
                                .child(field(&theme, &self.font)),
                        )
                        .child(
                            card_row(&theme, false)
                                .child(row_text(&theme, "Font size", "In points, for every pane."))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .child(action(&theme, "size-down", "−").w(px(32.)).on_click(cx.listener(
                                            |_, _: &ClickEvent, _, cx| update(cx, |s| s.font_size = step_font_size(s.font_size, -1.0)),
                                        )))
                                        .child(div().w(px(40.)).flex().justify_center().child(format!("{}", settings.font_size)))
                                        .child(action(&theme, "size-up", "+").w(px(32.)).on_click(cx.listener(
                                            |_, _: &ClickEvent, _, cx| update(cx, |s| s.font_size = step_font_size(s.font_size, 1.0)),
                                        ))),
                                ),
                        )
                        .child(
                            card_row(&theme, false)
                                .child(row_text(&theme, "Line height", "Row height as a multiple of the font size."))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .child(action(&theme, "line-height-down", "−").w(px(32.)).on_click(cx.listener(
                                            |_, _: &ClickEvent, _, cx| update(cx, |s| s.line_height = step_line_height(s.line_height, -0.1)),
                                        )))
                                        .child(div().w(px(40.)).flex().justify_center().child(format!("{:.1}", settings.line_height)))
                                        .child(action(&theme, "line-height-up", "+").w(px(32.)).on_click(cx.listener(
                                            |_, _: &ClickEvent, _, cx| update(cx, |s| s.line_height = step_line_height(s.line_height, 0.1)),
                                        ))),
                                ),
                        ),
                ),
            Section::Appearance => {
                let modes = [(AppearanceMode::System, "System"), (AppearanceMode::Light, "Light"), (AppearanceMode::Dark, "Dark")];
                let system = window.appearance();
                let (selected_bg, hover) = (theme.wash(0.10), theme.hover);
                div()
                    .child(page_header(&theme, "Appearance", "Window and terminal colors."))
                    .child(card(&theme).child(card_row(&theme, true).child(row_text(&theme, "Mode", "Follow the system, or keep one look.")).child(
                        div().flex().gap(px(8.)).children(modes.into_iter().map(|(mode, label)| {
                            let selected = settings.appearance == mode;
                            div()
                                .id(SharedString::from(format!("mode-{label}")))
                                .h(px(36.))
                                .px(px(14.))
                                .flex()
                                .items_center()
                                .rounded(px(8.))
                                .border_1()
                                .cursor_pointer()
                                .map(|el| match selected {
                                    true => el.bg(selected_bg).border_color(theme.border_strong).font_weight(FontWeight::MEDIUM),
                                    false => el.border_color(theme.border).text_color(theme.text_muted).hover(move |s| s.bg(hover)),
                                })
                                .child(label)
                                .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| update(cx, |s| s.appearance = mode)))
                        })),
                    )))
                    .child(section_label(&theme, "Theme"))
                    .child(card(&theme).mt_0().children(families().into_iter().enumerate().map(|(i, (id, name))| {
                        let preview = Theme::resolve(&id, settings.appearance, system, &settings.font());
                        let selected = settings.theme_family == id;
                        let swatches = std::iter::once(preview.terminal.background)
                            .chain(preview.terminal.ansi[1..7].iter().copied())
                            .map(|c| div().size(px(12.)).rounded(px(3.)).bg(c).border_1().border_color(theme.border));
                        let hover = theme.wash(0.03);
                        div()
                            .id(SharedString::from(format!("family-{id}")))
                            .cursor_pointer()
                            .hover(move |s| s.bg(hover))
                            .child(
                                card_row(&theme, i == 0)
                                    .min_h(px(44.))
                                    .py(px(10.))
                                    .child(
                                        div()
                                            .flex_1()
                                            .when(selected, |el| el.font_weight(FontWeight::MEDIUM))
                                            .when(!selected, |el| el.text_color(theme.text_muted))
                                            .child(name),
                                    )
                                    .child(div().flex().gap(px(3.)).children(swatches))
                                    .child(div().size(px(16.)).flex_none().when(selected, |el| {
                                        el.child(gpui::svg().path("icons/check.svg").size(px(16.)).text_color(theme.accent))
                                    })),
                            )
                            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                                let id = id.clone();
                                update(cx, move |s| s.theme_family = id)
                            }))
                    })))
            }
            Section::Herdr => div()
                .child(page_header(&theme, "herdr", "Where Shepherd finds herdr. Empty fields use herdr's defaults."))
                .child(
                    card(&theme)
                        .child(
                            card_row(&theme, true)
                                .child(row_text(&theme, "herdr binary", "A path to herdr, or the one on PATH."))
                                .child(field(&theme, &self.herdr_bin)),
                        )
                        .child(
                            card_row(&theme, false)
                                .child(row_text(&theme, "API socket", "A new socket is used after Shepherd restarts."))
                                .child(field(&theme, &self.herdr_socket)),
                        )
                        .child(
                            card_row(&theme, false)
                                .child(row_text(
                                    &theme,
                                    "Connection",
                                    self.ping.clone().unwrap_or_else(|| "Checks that herdr answers on this socket.".into()),
                                ))
                                .child(action(&theme, "ping", "Test connection").on_click(cx.listener(|v, _: &ClickEvent, _, cx| v.test_connection(cx)))),
                        ),
                ),
            Section::Shortcuts => {
                let bindings = effective(&settings.keys);
                div()
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .child(page_header(&theme, "Shortcuts", "Click a shortcut, then press the new chord. Esc cancels, ⌫ unbinds.").flex_1())
                            .child(action(&theme, "reset-keys", "Reset to defaults").on_click(cx.listener(|v, _: &ClickEvent, _, cx| {
                                v.recording = None;
                                update(cx, |s| s.keys.clear())
                            }))),
                    )
                    .child(card(&theme).children(ACTIONS.iter().enumerate().map(|(i, (action, label))| {
                        let chord = bindings.iter().find(|(a, _)| a == action).map(|(_, k)| display_chord(k));
                        let recording = self.recording == Some(*action);
                        let action = *action;
                        let hover = theme.border_strong;
                        card_row(&theme, i == 0).min_h(px(48.)).py(px(8.)).child(div().flex_1().child(*label)).child(
                            div()
                                .id(SharedString::from(format!("record-{action}")))
                                .h(px(28.))
                                .min_w(px(64.))
                                .px(px(10.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(6.))
                                .bg(theme.surface)
                                .border_1()
                                .font_family(theme.font_terminal.clone())
                                .text_size(px(12.))
                                .cursor_pointer()
                                .map(|el| match (recording, &chord) {
                                    (true, _) => el.border_color(theme.accent).text_color(theme.accent).child("Press keys…"),
                                    (false, Some(chord)) => el.border_color(theme.border).hover(move |s| s.border_color(hover)).child(chord.clone()),
                                    (false, None) => {
                                        el.border_color(theme.border).text_color(theme.text_faint).hover(move |s| s.border_color(hover)).child("—")
                                    }
                                })
                                .on_click(cx.listener(move |v, _: &ClickEvent, window, cx| {
                                    v.recording = Some(action);
                                    window.focus(&v.focus, cx);
                                    cx.notify();
                                })),
                        )
                    })))
            }
        };

        div()
            .id("settings")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .on_key_down(cx.listener(|v, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" && v.recording.is_none() {
                    cx.stop_propagation();
                    cx.emit(Close);
                }
            }))
            .child(nav)
            .child(
                div().id("settings-body").flex_1().min_w_0().h_full().overflow_y_scroll().child(
                    div()
                        .w_full()
                        .max_w(px(PAGE_MAX_WIDTH))
                        .mx_auto()
                        .px(px(40.))
                        .pt(px(TITLEBAR_HEIGHT + 16.))
                        .pb(px(48.))
                        .children(error.map(|e| {
                            div()
                                .mb(px(16.))
                                .px_3()
                                .py_2()
                                .rounded(px(12.))
                                .border_1()
                                .border_color(theme.danger.opacity(0.2))
                                .bg(theme.danger.opacity(0.06))
                                .text_color(theme.danger)
                                .child(e)
                        }))
                        .child(body),
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::display_chord;

    #[test]
    fn chords_read_as_zeron_shows_them() {
        assert_eq!(display_chord("cmd-shift-t"), "Shift+Cmd+T");
        assert_eq!(display_chord("ctrl-alt-space"), "Ctrl+Opt+Space");
        assert_eq!(display_chord("cmd-,"), "Cmd+,");
    }
}
