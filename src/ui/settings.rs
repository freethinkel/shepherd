//! The Settings page, shown in the main window in place of the sidebar and the panes (Zeron's settings route):
//! section tabs where the sidebar was, then a centered column of titled cards. Terminal font, appearance and
//! theme family, notifications, herdr's integrations and server settings, herdr's binary and socket, key
//! bindings. Every change is saved at once and applied to all windows; herdr's own settings go to its
//! `config.toml`, which herdr then reloads.
use crate::bindings::{ACTIONS, effective, rebind, recordable};
use crate::herdr::{Client, HerdrConfig, config_path, set_binary, set_socket};
use crate::settings::{OpenFrom, Settings, SettingsGlobal, WebEntry, step_font_size, step_line_height};
use crate::theme::{Appearance, AppearanceMode, StatusIcons, Theme, variants_for};
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

mod render;
mod select;
mod web;
mod worktrees;
mod widgets;
use web::*;
use widgets::*;
use worktrees::ProjectFields;

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
    Usage,
    Integrations,
    WebPages,
    Herdr,
    Worktrees,
    Shortcuts,
    About,
}

impl Section {
    const ALL: [Section; 11] = [
        Section::General,
        Section::Terminal,
        Section::Appearance,
        Section::Notifications,
        Section::Usage,
        Section::Integrations,
        Section::WebPages,
        Section::Herdr,
        Section::Worktrees,
        Section::Shortcuts,
        Section::About,
    ];

    fn label(self) -> &'static str {
        match self {
            Section::General => "General",
            Section::Terminal => "Terminal",
            Section::Appearance => "Appearance",
            Section::Notifications => "Notifications",
            Section::Usage => "Usage",
            Section::Integrations => "Integrations",
            Section::WebPages => "Web pages",
            Section::Herdr => "herdr",
            Section::Worktrees => "Worktrees",
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
            Section::Usage => "icons/clock-circle.svg",
            Section::Integrations => "icons/widget.svg",
            Section::WebPages => "icons/solar/global.svg",
            Section::Herdr => "icons/solar/server-2.svg",
            Section::Worktrees => "icons/git-branch.svg",
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
    /// Installed font families, read once: the font select's options.
    fonts: Vec<SharedString>,
    /// Font size and line height as typed; between edits they follow the setting.
    font_size: Entity<InputState>,
    line_height: Entity<InputState>,
    /// The font size and line height the fields last showed, so a step while one is typed in still shows.
    numbers_shown: [String; 2],
    herdr_bin: Entity<InputState>,
    herdr_socket: Entity<InputState>,
    /// The open list of a select, by the select's id.
    open_select: Option<crate::ui::Popup<&'static str>>,
    /// The filter at the top of a long select's list.
    select_search: Entity<InputState>,
    /// The highlighted row of the open select, as an index into the rows shown; Enter picks it.
    select_cursor: usize,
    /// Holds the keyboard for a short select's list, which has no search field to.
    select_focus: FocusHandle,
    select_scroll: gpui::ScrollHandle,
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
    /// The web pages' name and URL fields, in the settings' order; made as entries come, dropped as they go.
    web_fields: Vec<WebFields>,
    /// The places a web page can be listed in, as (key, name), once herdr has listed them.
    places: Option<Vec<(String, String)>>,
    /// Each worktree hooks project's fields, once Worktrees was shown; why `hooks.conf` could not be read.
    worktrees: Option<Result<Vec<ProjectFields>, String>>,
    /// Why the worktree hooks were not saved.
    worktree_note: Option<String>,
    /// Saves of the worktree hooks started, so one overtaken by a later one is dropped.
    worktree_saves: std::sync::Arc<std::sync::atomic::AtomicU64>,
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
    let secure_changed = settings.secure_input != SettingsGlobal::of(cx).settings.secure_input;
    let translucent_changed = settings.translucent_window != SettingsGlobal::of(cx).settings.translucent_window;
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
    if translucent_changed {
        let background = crate::shell::window_background(settings.translucent_window);
        // deferred: the click that got us here is inside an update of the window being changed, and gpui takes that
        // window out while it updates, so a nested update of it would fail
        cx.defer(move |cx| {
            for window in cx.windows() {
                if let Err(e) = window.update(cx, |_, window, _| window.set_background_appearance(background)) {
                    eprintln!("shepherd: cannot change a window's background: {e:#}");
                }
            }
        });
    }
    if keys_changed {
        rebind(cx, &settings.keys);
    } else if secure_changed {
        crate::bindings::set_menus(cx);
    }
    cx.refresh_windows();
}

impl SettingsView {
    /// Turns to the Usage page.
    pub fn show_usage(&mut self, cx: &mut Context<Self>) {
        self.section = Section::Usage;
        cx.notify();
    }

    pub fn show_web_pages(&mut self, cx: &mut Context<Self>) {
        self.section = Section::WebPages;
        cx.notify();
    }

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
        let font_size = field(Some(settings.font_size.to_string()), "13", window, cx);
        let line_height = field(Some(format!("{:.1}", settings.line_height)), "1.4", window, cx);
        let herdr_bin = field(settings.herdr_bin.clone(), "herdr on PATH", window, cx);
        let herdr_socket = field(settings.herdr_socket.clone(), if cfg!(windows) { r"%APPDATA%\herdr\herdr.sock" } else { "~/.config/herdr/herdr.sock" }, window, cx);
        let herdr_config = HerdrConfig::load(&config_path()).map_err(|e| format!("{e:#}"));
        let herdr_fields: Vec<_> = HERDR_FIELDS
            .iter()
            .map(|(key, empty)| {
                let value = herdr_config.as_ref().ok().and_then(|c| c.str(key)).map(str::to_string);
                field(value, empty, window, cx)
            })
            .collect();

        let select_search = field(None, "Search", window, cx);
        let mut subscriptions = vec![cx.subscribe(&select_search, |v: &mut Self, search, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                // a query highlights its first match; the emptied field of an opening select keeps the current option
                if !search.read(cx).value().trim().is_empty() {
                    v.select_cursor = 0;
                }
                cx.notify()
            }
        })];
        for (input, apply) in [
            (&herdr_bin, (|s: &mut Settings, v: String| s.herdr_bin = (!v.is_empty()).then_some(v)) as fn(&mut Settings, String)),
            (&herdr_socket, |s: &mut Settings, v: String| s.herdr_socket = (!v.is_empty()).then_some(v)),
        ] {
            subscriptions.push(cx.subscribe_in(input, window, move |_, input, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    let value = input.read(cx).value().trim().to_string();
                    update(cx, |s| apply(s, value));
                }
            }));
        }
        for (i, input) in [&font_size, &line_height].into_iter().enumerate() {
            // Enter leaves the field, which applies it. (Render runs before the blur arrives, so it can't.)
            subscriptions.push(cx.subscribe_in(input, window, move |v: &mut Self, input, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => window.focus(&v.focus, cx),
                InputEvent::Blur => set_number(i, input, 0.0, window, cx),
                _ => {}
            }));
        }
        // Up and down step the focused number field; intercepted, since the field's own bindings move its cursor.
        let numbers_window = window.window_handle();
        let number_inputs = [font_size.clone(), line_height.clone()];
        subscriptions.push(cx.intercept_keystrokes(move |event: &KeystrokeEvent, window, cx| {
            let dir = match event.keystroke.key.as_str() {
                "up" => 1.0,
                "down" => -1.0,
                _ => return,
            };
            if window.window_handle() != numbers_window || event.keystroke.modifiers != Default::default() {
                return;
            }
            if let Some(i) = number_inputs.iter().position(|input| input.read(cx).focus_handle(cx).is_focused(window)) {
                set_number(i, &number_inputs[i], dir, window, cx);
                cx.stop_propagation();
            }
        }));
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
            // Dot-prefixed families are macOS's private UI fonts.
            fonts: cx.text_system().all_font_names().into_iter().filter(|n| !n.starts_with('.')).map(SharedString::from).collect(),
            numbers_shown: numbers(&settings),
            font_size,
            line_height,
            herdr_bin,
            herdr_socket,
            open_select: None,
            select_search,
            select_cursor: 0,
            select_focus: cx.focus_handle(),
            select_scroll: gpui::ScrollHandle::new(),
            herdr_config,
            herdr_fields,
            herdr_note: None,
            integrations: None,
            integration_busy: None,
            recording: None,
            web_fields: Vec::new(),
            places: None,
            worktrees: None,
            worktree_note: None,
            worktree_saves: Default::default(),
            ping: None,
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        };
        // SHEPHERD_SELECT=<select id>[:query] opens that list at launch, filtered, for screenshots.
        if let Ok(spec) = std::env::var("SHEPHERD_SELECT") {
            let (id, query) = spec.split_once(':').unwrap_or((spec.as_str(), ""));
            if let Some(id) = ["light-theme-select", "dark-theme-select", "editor-select", "font-select"].into_iter().find(|s| *s == id) {
                view.open_select = Some(crate::ui::Popup::new(id));
                view.select_search.update(cx, |input, cx| {
                    input.set_value(query.to_string(), window, cx);
                    input.focus(window, cx);
                });
            }
        }
        view.load_integrations(cx);
        view.load_places(cx);
        view
    }

    /// The herdr sessions and machines a web page can be listed in, named as the session bar names them.
    fn load_places(&mut self, cx: &mut Context<Self>) {
        let list = cx.background_spawn(async { (crate::herdr::sessions().unwrap_or_default(), crate::herdr::machines().unwrap_or_default()) });
        cx.spawn(async move |v, cx| {
            let (sessions, machines) = list.await;
            let _ = v.update(cx, |v, cx| {
                let looks = &SettingsGlobal::of(cx).settings.place_looks;
                let name = |key: &str, herdr: &str| looks.get(key).and_then(|l| l.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| herdr.to_string());
                let sessions = sessions.iter().map(|s| (s.socket_path.clone(), name(&s.socket_path, &s.name)));
                let machines = machines.iter().filter(|m| m.enabled).map(|m| {
                    let key = format!("machine:{}", m.id);
                    let label = name(&key, &m.label);
                    (key, label)
                });
                v.places = Some(sessions.chain(machines).collect());
                cx.notify();
            });
        })
        .detach();
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


impl Focusable for SettingsView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// A chord as Zeron shows it: `cmd-shift-t` → `Cmd+Shift+T`.
pub(crate) fn display_chord(chord: &str) -> String {
    let Ok(k) = Keystroke::parse(chord) else { return chord.to_string() };
    let m = k.modifiers;
    let key = match k.key.as_str() {
        key if key.chars().count() == 1 => key.to_uppercase(),
        key => key[..1].to_uppercase() + &key[1..],
    };
    let (alt, platform) = if cfg!(target_os = "macos") { ("Opt", "Cmd") } else { ("Alt", "Super") };
    [(m.control, "Ctrl"), (m.alt, alt), (m.shift, "Shift"), (m.platform, platform), (m.function, "Fn")]
        .into_iter()
        .filter_map(|(on, name)| on.then_some(name.to_string()))
        .chain(std::iter::once(key))
        .collect::<Vec<_>>()
        .join("+")
}

#[cfg(test)]
mod tests {
    use super::display_chord;

    #[test]
    #[cfg(target_os = "macos")]
    fn chords_read_as_zeron_shows_them() {
        assert_eq!(display_chord("cmd-shift-t"), "Shift+Cmd+T");
        assert_eq!(display_chord("ctrl-alt-space"), "Ctrl+Opt+Space");
        assert_eq!(display_chord("cmd-,"), "Cmd+,");
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn chords_read_in_words() {
        assert_eq!(display_chord("ctrl-shift-t"), "Ctrl+Shift+T");
        assert_eq!(display_chord("ctrl-alt-space"), "Ctrl+Alt+Space");
    }
}
