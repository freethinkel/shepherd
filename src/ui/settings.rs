//! The Settings page, shown in the main window in place of the sidebar and the panes (Zeron's settings route):
//! section tabs where the sidebar was, then a centered column of titled cards. Terminal font, appearance and
//! theme family, notifications, herdr's integrations and server settings, herdr's binary and socket, key
//! bindings. Every change is saved at once and applied to all windows; herdr's own settings go to its
//! `config.toml`, which herdr then reloads.
use crate::bindings::{ACTIONS, effective, rebind, recordable};
use crate::herdr::{Client, HerdrConfig, config_path, set_binary, set_socket};
use crate::settings::{Settings, SettingsGlobal, step_font_size, step_line_height};
use crate::theme::{Appearance, AppearanceMode, Theme, variants_for};
use crate::ui::input_style;
use crate::update::{self as updater, Updates};
use gpui::{
    App, ClickEvent, Context, Div, Entity, EventEmitter, FocusHandle, Focusable, FontWeight, Global, IntoElement,
    KeyDownEvent, Keystroke, KeystrokeEvent, ParentElement, Render, SharedString, Stateful, Styled, Subscription,
    Window, div, prelude::*, px,
};
use gpui_base::input::{Input, InputEvent, InputState};
use serde::Deserialize;
use serde_json::{Value, json};

/// The app's tokio runtime, for calls made from windows (the herdr connection test).
pub struct Tokio(pub tokio::runtime::Handle);

impl Global for Tokio {}

const TITLEBAR_HEIGHT: f32 = 38.0;
/// Zeron's page column: a centered measure with air on either side.
const PAGE_MAX_WIDTH: f32 = 760.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    General,
    Terminal,
    Appearance,
    Notifications,
    Integrations,
    Herdr,
    Shortcuts,
    About,
}

impl Section {
    const ALL: [Section; 8] = [
        Section::General,
        Section::Terminal,
        Section::Appearance,
        Section::Notifications,
        Section::Integrations,
        Section::Herdr,
        Section::Shortcuts,
        Section::About,
    ];

    fn label(self) -> &'static str {
        match self {
            Section::General => "General",
            Section::Terminal => "Terminal",
            Section::Appearance => "Appearance",
            Section::Notifications => "Notifications",
            Section::Integrations => "Integrations",
            Section::Herdr => "herdr",
            Section::Shortcuts => "Shortcuts",
            Section::About => "About",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Section::General => "icons/settings.svg",
            Section::Terminal => "icons/terminal.svg",
            Section::Appearance => "icons/tuning.svg",
            Section::Notifications => "icons/bell.svg",
            Section::Integrations => "icons/widget.svg",
            Section::Herdr => "icons/remote-server.svg",
            Section::Shortcuts => "icons/keyboard.svg",
            Section::About => "icons/info-circle.svg",
        }
    }
}

/// Back, or Escape: the window shows the panes again.
pub struct Close;

/// One of herdr's agent integrations (`integration.list`): hooks that report the agent's state to herdr.
#[derive(Debug, Clone, Deserialize)]
struct Integration {
    target: String,
    label: String,
    command: String,
    /// The agent's command is on PATH.
    available: bool,
    /// `not_installed`, `current` or `outdated`.
    state: String,
}

/// The Editor select's options: the command saved and its label. `None` finds Zed, Cursor or VS Code.
const EDITORS: [(Option<&str>, &str); 5] =
    [(None, "Auto"), (Some("zed"), "Zed"), (Some("cursor -g"), "Cursor"), (Some("code -g"), "VS Code"), (Some("nvim"), "Neovim")];

/// herdr's text settings shown as fields: the config key and what an empty field means.
const HERDR_FIELDS: [(&str, &str); 3] =
    [("terminal.default_shell", "$SHELL"), ("terminal.new_cwd", "follow"), ("worktrees.directory", "~/.herdr/worktrees")];

pub struct SettingsView {
    section: Section,
    font: Entity<InputState>,
    herdr_bin: Entity<InputState>,
    herdr_socket: Entity<InputState>,
    /// The open list of a select, by the select's id.
    open_select: Option<crate::ui::Popup<&'static str>>,
    /// The filter at the top of a long select's list.
    select_search: Entity<InputState>,
    /// herdr's `config.toml` as last read or written, or why it could not be.
    herdr_config: Result<HerdrConfig, String>,
    /// Fields for `HERDR_FIELDS`, in order.
    herdr_fields: Vec<Entity<InputState>>,
    /// The outcome of the last change to herdr's settings or integrations, when there is something to say.
    herdr_note: Option<String>,
    /// `None` until herdr answers `integration.list`.
    integrations: Option<Result<Vec<Integration>, String>>,
    /// The integration being installed or removed.
    integration_busy: Option<String>,
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
        let herdr_config = HerdrConfig::load(&config_path()).map_err(|e| format!("{e:#}"));
        let herdr_fields: Vec<_> = HERDR_FIELDS
            .iter()
            .map(|(key, empty)| {
                let value = herdr_config.as_ref().ok().and_then(|c| c.str(key)).map(str::to_string);
                field(value, empty, window, cx)
            })
            .collect();

        let select_search = field(None, "Search themes", window, cx);
        let mut subscriptions = vec![cx.subscribe(&select_search, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify()
            }
        })];
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
        for (input, (key, _)) in herdr_fields.iter().zip(HERDR_FIELDS) {
            subscriptions.push(cx.subscribe_in(input, window, move |v: &mut Self, input, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    let value = input.read(cx).value().trim().to_string();
                    let current = v.herdr_config.as_ref().ok().and_then(|c| c.str(key)).unwrap_or_default();
                    if value != current {
                        v.edit_herdr(key, (!value.is_empty()).then(|| value.into()), cx);
                    }
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
            .unwrap_or(Section::General);
        let mut view = SettingsView {
            section,
            font,
            herdr_bin,
            herdr_socket,
            open_select: None,
            select_search,
            herdr_config,
            herdr_fields,
            herdr_note: None,
            integrations: None,
            integration_busy: None,
            recording: None,
            ping: None,
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        };
        // SHEPHERD_SELECT=<select id>[:query] opens that list at launch, filtered, for screenshots.
        if let Ok(spec) = std::env::var("SHEPHERD_SELECT") {
            let (id, query) = spec.split_once(':').unwrap_or((spec.as_str(), ""));
            if let Some(id) = ["light-theme-select", "dark-theme-select", "editor-select"].into_iter().find(|s| *s == id) {
                view.open_select = Some(crate::ui::Popup::new(id));
                view.select_search.update(cx, |input, cx| {
                    input.set_value(query.to_string(), window, cx);
                    input.focus(window, cx);
                });
            }
        }
        view.load_integrations(cx);
        view
    }

    /// A request to the herdr Shepherd is connected to; `done` gets the result on the main thread.
    fn herdr_call(
        &self,
        method: &'static str,
        params: Value,
        cx: &mut Context<Self>,
        done: impl FnOnce(&mut Self, anyhow::Result<Value>) + 'static,
    ) {
        let socket = SettingsGlobal::of(cx).settings.herdr_socket.clone().filter(|p| !p.trim().is_empty());
        let client = socket.map_or_else(Client::local, |p| Client::at(p.into()));
        let call = cx.global::<Tokio>().0.spawn(async move { client.call(method, params).await });
        cx.spawn(async move |this, cx| {
            let result = call.await.unwrap_or_else(|e| Err(e.into()));
            let _ = this.update(cx, |v, cx| {
                done(v, result);
                cx.notify();
            });
        })
        .detach();
    }

    /// Changes one of herdr's settings (`None` puts back its default) and has herdr reload them. The file is read
    /// again first, so edits made elsewhere in the meantime are kept.
    fn edit_herdr(&mut self, key: &str, value: Option<toml_edit::Value>, cx: &mut Context<Self>) {
        let path = config_path();
        let saved = HerdrConfig::load(&path).and_then(|mut config| {
            config.set(key, value)?;
            config.save(&path)?;
            Ok(config)
        });
        match saved {
            Ok(config) => {
                self.herdr_config = Ok(config);
                self.herdr_note = None;
                self.herdr_call("server.reload_config", json!({}), cx, |v, result| {
                    v.herdr_note = result.err().map(|e| format!("Saved, but herdr did not reload it: {e:#}"));
                });
            }
            Err(e) => self.herdr_note = Some(format!("{e:#}")),
        }
        cx.notify();
    }

    fn load_integrations(&mut self, cx: &mut Context<Self>) {
        self.herdr_call("integration.list", json!({}), cx, |v, result| {
            v.integrations = Some(result.map_err(|e| format!("{e:#}")).and_then(|r| {
                let mut list: Vec<Integration> = serde_json::from_value(r["integrations"].clone()).map_err(|e| e.to_string())?;
                list.sort_by_key(|i| !i.available); // agents you have first, in herdr's order
                Ok(list)
            }));
        });
    }

    /// Installs (or updates) or uninstalls an integration, then lists them again.
    fn run_integration(&mut self, target: String, install: bool, cx: &mut Context<Self>) {
        let method = if install { "integration.install" } else { "integration.uninstall" };
        self.integration_busy = Some(target.clone());
        self.herdr_call(method, json!({"target": target}), cx, |v, result| {
            v.integration_busy = None;
            v.herdr_note = Some(match result {
                Ok(r) => r["messages"].as_array().into_iter().flatten().filter_map(Value::as_str).collect::<Vec<_>>().join("\n"),
                Err(e) => format!("{e:#}"),
            })
            .filter(|note| !note.is_empty());
        });
        self.load_integrations(cx);
        cx.notify();
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

impl SettingsView {
    /// A pill naming the current option; a click drops the list of `labels` under it (Zeron's select).
    /// `fallback` names a value that is none of the options; `leading` draws an option's swatch.
    #[allow(clippy::too_many_arguments)]
    fn select(
        &self,
        theme: &Theme,
        id: &'static str,
        labels: Vec<SharedString>,
        current: Option<usize>,
        fallback: SharedString,
        leading: impl Fn(usize) -> Option<gpui::AnyElement>,
        pick: impl Fn(usize, &mut App) + 'static,
        cx: &mut Context<Self>,
    ) -> Div {
        let hover = theme.wash(0.10);
        // A list longer than a screenful gets a filter.
        let searchable = labels.len() > 12;
        let label = current.and_then(|i| labels.get(i).cloned()).unwrap_or(fallback);
        let trigger = div()
            .id(id)
            .w(px(240.))
            .h(px(32.))
            .px(px(10.))
            .flex()
            .items_center()
            .gap(px(8.))
            .rounded(px(8.))
            .bg(theme.wash(0.06))
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .children(current.and_then(&leading))
            .child(div().flex_1().child(label))
            .child(gpui::svg().path("icons/alt-arrow-down.svg").size(px(14.)).text_color(theme.text_muted))
            .on_click(cx.listener(move |v, _: &ClickEvent, window, cx| {
                v.open_select = Some(crate::ui::Popup::new(id));
                if searchable {
                    v.select_search.update(cx, |input, cx| {
                        input.set_value("", window, cx);
                        input.focus(window, cx);
                    });
                }
                cx.notify();
            }));
        let pick = std::rc::Rc::new(pick);
        let menu = self.open_select.as_ref().filter(|m| m.value == id).map(|menu| {
            let query = if searchable { self.select_search.read(cx).value().trim().to_lowercase() } else { String::new() };
            let shown: Vec<usize> = (0..labels.len()).filter(|&i| labels[i].to_lowercase().contains(&query)).collect();
            let first = shown.first().copied();
            let rows = shown.into_iter().map(|i| {
                let pick = pick.clone();
                crate::shell::row(theme, false, (id, i))
                    .h(px(28.))
                    .flex_none()
                    .rounded(px(6.))
                    .gap(px(8.))
                    .children(leading(i))
                    .child(div().flex_1().child(labels[i].clone()))
                    .when(current == Some(i), |el| el.child(gpui::svg().path("icons/check.svg").size(px(14.)).text_color(theme.text_muted)))
                    .on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                        if let Some(m) = v.open_select.as_mut() { m.close() }
                        pick(i, cx);
                        cx.notify();
                    }))
            });
            let list = div()
                .id("select-list")
                .max_h(px(320.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .children(rows)
                .when(first.is_none(), |el| el.child(div().px(px(8.)).py(px(6.)).text_color(theme.text_muted).child("No matching themes")));
            let pick = pick.clone();
            let card = crate::shell::menu_card(theme, "select-menu", 240.)
                .w(px(if searchable { 280. } else { 240. }))
                .mt(px(4.))
                .flex()
                .flex_col()
                .on_mouse_down_out(cx.listener(|v, _, _, cx| {
                    if let Some(m) = v.open_select.as_mut() { m.close() }
                    cx.notify();
                }))
                // Enter takes the first match, Escape closes: caught before the field's own bindings.
                .capture_key_down(cx.listener(move |v, e: &KeyDownEvent, _, cx| match e.keystroke.key.as_str() {
                    "enter" | "escape" => {
                        if let Some(m) = v.open_select.as_mut() { m.close() }
                        if let (Some(i), "enter") = (first, e.keystroke.key.as_str()) {
                            pick(i, cx);
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }
                    _ => {}
                }))
                // Zeron's popup search: a borderless field on a faint wash, above the list.
                .when(searchable, |el| {
                    el.child(
                        div()
                            .mb(px(4.))
                            .h(px(32.))
                            .flex_none()
                            .px(px(10.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .rounded(px(6.))
                            .bg(theme.wash(0.04))
                            .child(gpui::svg().path("icons/palette-search.svg").size(px(16.)).flex_none().text_color(theme.text_muted))
                            .child(div().flex_1().min_w_0().flex().items_center().line_height(px(16.)).child(Input::new(&self.select_search))),
                    )
                })
                .child(list);
            // `anchored` is absolutely placed at its parent's origin: an in-flow wrapper puts that under the trigger.
            div().child(gpui::deferred(gpui::anchored().snap_to_window_with_margin(px(8.)).child(crate::ui::menu_motion("select-menu-in", menu, card))).with_priority(1))
        });
        div().flex_none().flex().flex_col().child(trigger).children(menu)
    }

    /// The themes of one appearance, each with its colors; picking one sets that appearance's theme.
    fn theme_select(&self, theme: &Theme, appearance: Appearance, current: &str, cx: &mut Context<Self>) -> Div {
        let (id, light) = match appearance {
            Appearance::Light => ("light-theme-select", true),
            Appearance::Dark => ("dark-theme-select", false),
        };
        let list = variants_for(appearance);
        let labels = list.iter().map(|(_, name)| SharedString::from(name.clone())).collect();
        let index = list.iter().position(|(id, _)| id == current);
        let ids: Vec<String> = list.into_iter().map(|(id, _)| id).collect();
        // Drawn only for the rows shown: the current one, and the list while it is open.
        let preview_ids = ids.clone();
        let font = crate::theme::FontSettings::default();
        self.select(
            theme,
            id,
            labels,
            index,
            current.to_string().into(),
            move |i| Theme::variant(&preview_ids[i], &font).map(|t| palette_preview(&t).into_any_element()),
            move |i, cx| {
                let id = ids[i].clone();
                update(cx, move |s| if light { s.theme_light = id } else { s.theme_dark = id })
            },
            cx,
        )
    }

    fn editor_select(&self, theme: &Theme, current: Option<&str>, cx: &mut Context<Self>) -> Div {
        let labels = EDITORS.iter().map(|(_, l)| SharedString::from(*l)).collect();
        let index = EDITORS.iter().position(|(cmd, _)| *cmd == current);
        // A command typed into settings.json by hand shows as itself.
        let fallback = SharedString::from(current.unwrap_or_default().to_string());
        self.select(theme, "editor-select", labels, index, fallback, |_| None, |i, cx| update(cx, |s| s.editor = EDITORS[i].0.map(String::from)), cx)
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

/// The shared switch (`crate::ui::switch`), clickable.
fn toggle(theme: &Theme, id: impl Into<SharedString>, on: bool) -> Stateful<Div> {
    let id = id.into();
    div().id(id.clone()).flex_none().cursor_pointer().child(crate::ui::switch(theme, on, id))
}

/// A card of the Color scheme row: a preview of the look, its icon and name under it (Zeron's option card).
fn option_card(theme: &Theme, icon: &'static str, label: &'static str, selected: bool, preview: impl IntoElement) -> Div {
    let tone = if selected { theme.accent } else { theme.text_muted };
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(8.))
        .cursor_pointer()
        .child(
            div()
                .h(px(112.))
                .w_full()
                .rounded(px(6.))
                .overflow_hidden()
                .border_1()
                .border_color(if selected { theme.accent } else { theme.border })
                .child(preview),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_color(tone)
                .when(selected, |el| el.font_weight(FontWeight::MEDIUM))
                .child(gpui::svg().path(icon).size(px(16.)).text_color(tone))
                .child(label),
        )
}

/// A sketch of the window in `theme`: a sidebar and a terminal pane, text as bars.
fn miniature(theme: &Theme) -> Div {
    let (line, strong) = (theme.text.opacity(0.22), theme.text.opacity(0.34));
    let bar = |fraction: f32, tone| div().h(px(5.)).w(gpui::relative(fraction)).rounded(px(3.)).bg(tone);
    div()
        .size_full()
        .flex()
        .bg(theme.surface)
        .child(div().w(px(40.)).flex_none().flex().flex_col().gap(px(7.)).px(px(8.)).pt(px(14.)).children([
            bar(0.7, strong),
            bar(1.0, line),
            bar(0.85, line),
        ]))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .my(px(8.))
                .mr(px(8.))
                .rounded(px(6.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.terminal.background)
                .flex()
                .flex_col()
                .gap(px(7.))
                .p(px(10.))
                .children([bar(0.62, strong), bar(0.88, line), bar(0.76, line), bar(0.52, line)]),
        )
}

/// A theme's three colors in a small chip: window, terminal, accent.
fn palette_preview(theme: &Theme) -> Div {
    div()
        .flex_none()
        .w(px(30.))
        .h(px(18.))
        .rounded(px(5.))
        .overflow_hidden()
        .border_1()
        .border_color(theme.border)
        .flex()
        .child(div().w_1_3().h_full().bg(theme.surface))
        .child(div().w_1_3().h_full().bg(theme.terminal.background))
        .child(div().w_1_3().h_full().bg(theme.accent))
}

/// One option of a row of choices: outlined, filled when selected.
fn choice(theme: &Theme, id: impl Into<gpui::ElementId>, label: &'static str, selected: bool) -> Stateful<Div> {
    let (selected_bg, hover) = (theme.wash(0.10), theme.hover);
    div()
        .id(id)
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
}

/// A muted line under a card: where a setting is saved, or what the last change did.
fn note(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div().mt(px(8.)).px(px(8.)).text_size(px(12.)).line_height(px(16.)).text_color(theme.text_muted).child(text.into())
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
        crate::ui::reap_popup(&mut self.open_select, window);
        let theme = Theme::of(cx).clone();
        let global = SettingsGlobal::of(cx);
        let settings = global.settings.clone();
        let error = global.error.clone();
        let herdr = self.herdr_config.as_ref().ok().cloned().unwrap_or_default();
        // Under herdr's cards: why its config cannot be read, what the last change did, else where it is saved.
        let herdr_note = self
            .herdr_config
            .as_ref()
            .err()
            .cloned()
            .or_else(|| self.herdr_note.clone())
            .unwrap_or_else(|| format!("Saved to {}. herdr reloads it right away.", config_path().display()));

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
                            if section == Section::Integrations && v.section != section {
                                v.load_integrations(cx); // installed from a terminal meanwhile
                            }
                            v.section = section;
                            v.recording = None;
                            cx.notify();
                        }))
                },
            )));

        let body = match self.section {
            Section::General => div().child(page_header(&theme, "General", "The window and what it shows.")).child(
                card(&theme)
                    .child(
                        card_row(&theme, true)
                            .child(row_text(&theme, "Agents", "Every agent herdr finds, at the bottom of the sidebar."))
                            .child(toggle(&theme, "show-agents", settings.show_agents).on_click(cx.listener(
                                |_, _: &ClickEvent, _, cx| update(cx, |s| s.show_agents = !s.show_agents),
                            ))),
                    )
                    .child(
                        card_row(&theme, false)
                            .child(row_text(&theme, "Status bar", "The branch, folder, focused agent and Claude usage along the bottom."))
                            .child(toggle(&theme, "show-status-bar", settings.show_status_bar).on_click(cx.listener(
                                |_, _: &ClickEvent, _, cx| update(cx, |s| s.show_status_bar = !s.show_status_bar),
                            ))),
                    )
                    .child(
                        card_row(&theme, false)
                            .child(row_text(&theme, "Copy on select", "A finished selection goes to the clipboard, then clears."))
                            .child(toggle(&theme, "copy-on-select", settings.copy_on_select).on_click(cx.listener(
                                |_, _: &ClickEvent, _, cx| update(cx, |s| s.copy_on_select = !s.copy_on_select),
                            ))),
                    ),
            ),
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
                        )
                        .child(
                            card_row(&theme, false)
                                .child(row_text(&theme, "Editor", if crate::terminal::pane::is_nvim(settings.editor.as_deref()) {
                                    "Cmd+click on file:line opens it in Neovim, in a pane to the right."
                                } else {
                                    "Cmd+click on file:line opens it in this editor."
                                }))
                                .child(self.editor_select(&theme, settings.editor.as_deref(), cx)),
                        ),
                ),
            Section::Appearance => {
                let modes = [
                    (AppearanceMode::System, "System", "icons/monitor.svg"),
                    (AppearanceMode::Light, "Light", "icons/sun.svg"),
                    (AppearanceMode::Dark, "Dark", "icons/moon.svg"),
                ];
                let font = settings.font();
                let light = Theme::resolve(&settings.theme_light, &settings.theme_dark, AppearanceMode::Light, window.appearance(), &font);
                let dark = Theme::resolve(&settings.theme_light, &settings.theme_dark, AppearanceMode::Dark, window.appearance(), &font);
                let cards = modes.into_iter().map(|(mode, label, icon)| {
                    let selected = settings.appearance == mode;
                    let preview = match mode {
                        AppearanceMode::System => div()
                            .size_full()
                            .flex()
                            .child(div().w_1_2().h_full().overflow_hidden().child(miniature(&light)))
                            .child(div().w_1_2().h_full().overflow_hidden().child(miniature(&dark))),
                        AppearanceMode::Light => miniature(&light),
                        AppearanceMode::Dark => miniature(&dark),
                    };
                    option_card(&theme, icon, label, selected, preview)
                        .id(SharedString::from(format!("mode-{label}")))
                        .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| update(cx, |s| s.appearance = mode)))
                }).collect::<Vec<_>>();
                let light_select = self.theme_select(&theme, Appearance::Light, &settings.theme_light, cx);
                let dark_select = self.theme_select(&theme, Appearance::Dark, &settings.theme_dark, cx);
                div()
                    .child(page_header(&theme, "Appearance", "Window and terminal colors."))
                    .child(section_label(&theme, "Color scheme"))
                    .child(div().flex().gap(px(16.)).children(cards))
                    .child(
                        card(&theme)
                            .child(card_row(&theme, true).child(row_text(&theme, "Light theme", "When the window is light.")).child(light_select))
                            .child(card_row(&theme, false).child(row_text(&theme, "Dark theme", "When the window is dark.")).child(dark_select)),
                    )
            }
            Section::Notifications => {
                let shepherd_rows: [(&'static str, &'static str, bool, fn(&mut crate::settings::Settings)); 3] = [
                    (
                        "Notify when an agent needs you",
                        "A macOS notification when an agent you are not looking at waits for you or finishes.",
                        settings.notify_agents,
                        |s| s.notify_agents = !s.notify_agents,
                    ),
                    ("Sound", "Play a sound with each notification.", settings.notify_sound, |s| s.notify_sound = !s.notify_sound),
                    (
                        "Dock badge",
                        "Count the agents waiting or done that you have not looked at yet.",
                        settings.dock_badge,
                        |s| s.dock_badge = !s.dock_badge,
                    ),
                ];
                let delivery = herdr.str("ui.toast.delivery").unwrap_or("off").to_string();
                let delay = herdr.int("ui.toast.delay_seconds", 1);
                let sounds = herdr.bool("ui.sound.enabled", true);
                div()
                    .child(page_header(&theme, "Notifications", "When an agent waits for you or finishes."))
                    .child(card(&theme).children(shepherd_rows.into_iter().enumerate().map(|(i, (title, description, on, flip))| {
                        card_row(&theme, i == 0)
                            .child(row_text(&theme, title, description))
                            .child(toggle(&theme, SharedString::from(format!("notify-{i}")), on).on_click(cx.listener(
                                move |_, _: &ClickEvent, _, cx| update(cx, flip),
                            )))
                    })).child({
                        let denied = cx.has_global::<crate::notify::Denied>();
                        let text = if denied {
                            row_text(&theme, "Turned off in macOS", "macOS blocks Shepherd's notifications, so none show up. Allow them in System Settings.")
                                .text_color(theme.danger)
                        } else {
                            row_text(&theme, "In macOS", "Banners, sounds and whether they show while you use other apps.")
                        };
                        card_row(&theme, false).child(text).child(
                            action(&theme, "notify-system-settings", "Open System Settings")
                                .on_click(|_: &ClickEvent, _, _| crate::notify::open_system_settings()),
                        )
                    }))
                    .child(section_label(&theme, "In herdr's terminal app"))
                    .child(
                        card(&theme)
                            .mt_0()
                            .child(
                                card_row(&theme, true).child(row_text(&theme, "Pop-ups", "Where herdr shows them.")).child(
                                    div().flex().gap(px(8.)).children(
                                        [("off", "Off"), ("herdr", "In herdr"), ("terminal", "Terminal"), ("system", "System")].into_iter().map(
                                            |(value, label)| {
                                                choice(&theme, SharedString::from(format!("delivery-{value}")), label, delivery == value)
                                                    .on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                                                        v.edit_herdr("ui.toast.delivery", Some(value.into()), cx)
                                                    }))
                                            },
                                        ),
                                    ),
                                ),
                            )
                            .child(
                                card_row(&theme, false)
                                    .child(row_text(&theme, "Delay", "Seconds before a pop-up shows."))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .child(action(&theme, "delay-down", "−").w(px(32.)).on_click(cx.listener(
                                                move |v, _: &ClickEvent, _, cx| {
                                                    v.edit_herdr("ui.toast.delay_seconds", Some((delay - 1).clamp(0, 30).into()), cx)
                                                },
                                            )))
                                            .child(div().w(px(40.)).flex().justify_center().child(format!("{delay}")))
                                            .child(action(&theme, "delay-up", "+").w(px(32.)).on_click(cx.listener(
                                                move |v, _: &ClickEvent, _, cx| {
                                                    v.edit_herdr("ui.toast.delay_seconds", Some((delay + 1).clamp(0, 30).into()), cx)
                                                },
                                            ))),
                                    ),
                            )
                            .child(
                                card_row(&theme, false)
                                    .child(row_text(&theme, "Sounds", "When an agent in a background workspace changes state."))
                                    .child(toggle(&theme, "herdr-sounds", sounds).on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                                        v.edit_herdr("ui.sound.enabled", Some((!sounds).into()), cx)
                                    }))),
                            ),
                    )
                    .child(note(&theme, herdr_note))
            }
            Section::Integrations => {
                let body = match &self.integrations {
                    None => note(&theme, "Asking herdr…").into_any_element(),
                    Some(Err(e)) => note(&theme, e.clone()).text_color(theme.danger).into_any_element(),
                    Some(Ok(list)) => card(&theme)
                        .children(list.iter().enumerate().map(|(i, integration)| {
                            let busy = self.integration_busy.as_deref() == Some(integration.target.as_str());
                            let (status, button) = match integration.state.as_str() {
                                "current" => ("Installed", "Uninstall"),
                                "outdated" => ("Update available", "Update"),
                                _ => ("Not installed", "Install"),
                            };
                            let status = match integration.available {
                                true => status.to_string(),
                                false => format!("{status} · `{}` is not on PATH", integration.command),
                            };
                            let (target, install) = (integration.target.clone(), button != "Uninstall");
                            card_row(&theme, i == 0)
                                .min_h(px(52.))
                                .py(px(8.))
                                .when(!integration.available, |el| el.text_color(theme.text_muted))
                                .child(row_text(&theme, integration.label.clone(), status))
                                .child(
                                    action(&theme, SharedString::from(format!("integration-{}", integration.target)), if busy { "…" } else { button })
                                        .when(!busy, |el| {
                                            el.on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                                                v.run_integration(target.clone(), install, cx)
                                            }))
                                        }),
                                )
                        }))
                        .into_any_element(),
                };
                div()
                    .child(page_header(
                        &theme,
                        "Integrations",
                        "Hooks that tell herdr what each agent is doing: working, waiting for you, done.",
                    ))
                    .child(body)
                    .children(self.herdr_note.clone().map(|text| note(&theme, text)))
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
                                .child(row_text(&theme, "API socket", "Shepherd switches to a new socket when it restarts."))
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
                )
                .child(section_label(&theme, "Server"))
                .child({
                    let resume = herdr.bool("session.resume_agents_on_restore", true);
                    let history = herdr.bool("experimental.pane_history", false);
                    let channel = herdr.str("update.channel").unwrap_or("stable").to_string();
                    let fields = [
                        ("Shell", "For new panes. Empty is $SHELL."),
                        ("New pane folder", "follow, home, current or a path. follow takes the folder of the pane it opens from."),
                        ("Worktrees folder", "Where new worktrees are checked out."),
                    ];
                    card(&theme)
                        .mt_0()
                        .children(fields.into_iter().zip(&self.herdr_fields).enumerate().map(|(i, ((title, description), input))| {
                            card_row(&theme, i == 0).child(row_text(&theme, title, description)).child(field(&theme, input))
                        }))
                        .child(
                            card_row(&theme, false)
                                .child(row_text(&theme, "Resume agents", "Reopen agent conversations after herdr restarts."))
                                .child(toggle(&theme, "herdr-resume", resume).on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                                    v.edit_herdr("session.resume_agents_on_restore", Some((!resume).into()), cx)
                                }))),
                        )
                        .child(
                            card_row(&theme, false)
                                .child(row_text(&theme, "Pane history", "Keep recent screen history across server restarts. Experimental."))
                                .child(toggle(&theme, "herdr-history", history).on_click(cx.listener(move |v, _: &ClickEvent, _, cx| {
                                    v.edit_herdr("experimental.pane_history", Some((!history).into()), cx)
                                }))),
                        )
                        .child(
                            card_row(&theme, false).child(row_text(&theme, "Updates", "herdr's release channel.")).child(
                                div().flex().gap(px(8.)).children([("stable", "Stable"), ("preview", "Preview")].into_iter().map(|(value, label)| {
                                    choice(&theme, SharedString::from(format!("channel-{value}")), label, channel == value).on_click(cx.listener(
                                        move |v, _: &ClickEvent, _, cx| v.edit_herdr("update.channel", Some(value.into()), cx),
                                    ))
                                })),
                            ),
                        )
                })
                .child(note(&theme, herdr_note)),
            Section::About => {
                let updates = Updates::of(cx).clone();
                let button = match &updates {
                    Updates::Available(r) => {
                        let r = r.clone();
                        action(&theme, "update-app", format!("Update to {}", r.version))
                            .on_click(move |_: &ClickEvent, _, cx| updater::install(r.clone(), cx))
                    }
                    _ => action(&theme, "check-updates", "Check for Updates").on_click(|_: &ClickEvent, _, cx| updater::check(cx, false)),
                };
                div().child(page_header(&theme, "About", "Shepherd's version. Updates come from its GitHub releases.")).child(
                    card(&theme).child(
                        card_row(&theme, true)
                            .child(row_text(&theme, format!("Shepherd {}", env!("CARGO_PKG_VERSION")), updates.line()))
                            .when(updater::app_bundle().is_some(), |row| row.child(button.when(updates.busy(), |b| b.opacity(0.5)))),
                    ),
                )
            }
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
