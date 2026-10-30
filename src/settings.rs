//! Shepherd's settings: a JSON file every field of which has a default, so old, partial or future files load.
use crate::model::AgentSort;
use crate::theme::{AppearanceMode, FontSettings, Theme};
use gpui::{App, Bounds, Global, Pixels, Size, WindowAppearance, point, px, size};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub font_family: String,
    pub font_size: f32,
    /// Terminal row height as a multiple of the font size.
    pub line_height: f32,
    /// The theme variant (`ayu-light`, `ghostty-…`) in light appearance, and in dark.
    pub theme_light: String,
    pub theme_dark: String,
    /// Before the light and dark themes: one Zeron family for both. Read once, never written.
    #[serde(skip_serializing)]
    pub theme_family: Option<String>,
    pub appearance: AppearanceMode,
    /// The herdr binary; `None` is `herdr` on PATH.
    pub herdr_bin: Option<String>,
    /// herdr's API socket; `None` is herdr's own default.
    pub herdr_socket: Option<String>,
    /// The saved herdr machine (`herdr machine`) shown, by id; its relay's socket is `herdr_socket`.
    pub herdr_machine: Option<String>,
    /// The names and icons given to herdr sessions and machines in Shepherd, by the place's key (a session's socket,
    /// `machine:<id>`).
    pub place_looks: BTreeMap<String, PlaceLook>,
    /// The bar's order of herdr sessions and machines, by the place's key, as dragged; new ones go last.
    pub place_order: Vec<String>,
    /// The command Cmd+click opens `path:line:col` with (`zed`, `code -g`, `nvim` in a pane to the right); `None`
    /// finds Zed, Cursor or VS Code.
    pub editor: Option<String>,
    /// Action name → keystroke (`cmd-shift-d`); an empty string unbinds the action.
    pub keys: BTreeMap<String, String>,
    /// The workspaces sidebar's width; dragging its edge sets it.
    pub sidebar_width: f32,
    /// The review panel on the right: the shown branch's PR or MR and its checks.
    pub review_open: bool,
    /// Its width; dragging its edge sets it.
    pub review_width: f32,
    pub agent_sort: AgentSort,
    /// The sidebar's Agents section; General settings hide it.
    pub show_agents: bool,
    /// The bottom status bar; General settings hide it.
    pub show_status_bar: bool,
    /// Which of its labels the status bar shows: its right-click menu.
    pub status_bar_show: StatusBarShow,
    /// Whose plan limits the status bar shows; Usage settings pick it.
    pub usage_agent: UsageAgent,
    /// What a workspace row shows besides its name: the sidebar `…` menu's Show.
    pub workspace_show: WorkspaceShow,
    /// Shorter workspace rows.
    pub compact_workspaces: bool,
    /// One-line agent rows, without the task under the name.
    pub compact_agents: bool,
    /// How agent status is drawn: color dots, herdr's symbols, or Zeron's (a pulsing glyph while an agent works).
    pub status_icons: crate::theme::StatusIcons,
    /// The Agents list keeps only agents waiting on you or done and not looked at yet.
    pub attention_agents: bool,
    /// The Agents list's height cap; dragging the section's top edge sets it, `None` is 40 % of the sidebar.
    pub agents_height: Option<f32>,
    /// What the command panel ran last, newest first: an action's name, `workspace:<id>` or `tab:<id>`.
    pub recent_commands: Vec<String>,
    /// A macOS notification when an agent out of sight starts waiting or finishes.
    pub notify_agents: bool,
    /// Play a sound with those notifications.
    pub notify_sound: bool,
    /// The Dock icon counts agents waiting or done that you have not looked at.
    pub dock_badge: bool,
    /// A finished selection in a pane goes to the clipboard and clears, as in herdr.
    pub copy_on_select: bool,
    /// A jump of the cursor leaves a short trail behind it, as Ghostty's cursor_smear shader draws.
    pub animated_cursor: bool,
    /// macOS Secure Keyboard Entry while a Shepherd window is in front: other apps cannot read the keys typed.
    pub secure_input: bool,
    /// The window lets the desktop show through: blurred on macOS, and elsewhere as the compositor does it. On by
    /// default on macOS only, where it is how Shepherd has always looked.
    pub translucent_window: bool,
    /// Off macOS and Windows: the window's own minimize, maximize and close at the titlebar's right end. Off (when
    /// the window manager provides its own controls or tiles windows), the tabs take their room; the edges still
    /// resize the window, and a right-click on the titlebar opens the window manager's menu.
    pub window_buttons: bool,
    /// The workspaces sidebar is shown; `⌘B` hides it.
    pub sidebar_open: bool,
    /// Worktree groups folded in the sidebar, by workspace id.
    pub folded: Vec<String>,
    /// Projects folded in the agents list, by their top-level workspace's id.
    pub agents_folded: Vec<String>,
    /// Web pages listed in the sidebar above the workspaces and shown in place of the tabs and panes.
    pub web_entries: Vec<WebEntry>,
    /// Links to another site open in the system browser instead of in the page.
    pub web_links_outside: bool,
    /// Web links clicked in a terminal open in a browser tab beside it instead of in the system browser.
    pub web_links_inside: bool,
    /// Where the window was last; `None` opens it centered.
    pub window: Option<WindowFrame>,
}

/// An agent whose plan limits Shepherd reads. An unknown (future) value reads as Claude.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageAgent {
    Codex,
    #[default]
    #[serde(other)]
    Claude,
}

impl UsageAgent {
    pub const ALL: [UsageAgent; 2] = [UsageAgent::Claude, UsageAgent::Codex];

    /// Its key in the usage cache, and in the settings file.
    pub fn key(self) -> &'static str {
        match self {
            UsageAgent::Claude => "claude",
            UsageAgent::Codex => "codex",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            UsageAgent::Claude => "Claude",
            UsageAgent::Codex => "Codex",
        }
    }
}

/// A herdr session's or machine's look in Shepherd, as Arc's spaces have one: `None` keeps herdr's name, or the
/// name's first letter for the icon.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaceLook {
    pub name: Option<String>,
    /// A short text, an emoji or a letter or two.
    pub icon: Option<String>,
}

/// A web page in the sidebar: a local tool's name and URL, where it is listed and where its URL is opened.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WebEntry {
    /// Stable for the entry's life, so its page and favicon survive renames, edits and reordering.
    pub id: String,
    pub name: String,
    pub url: String,
    /// The place key (`Place::key`: a session's socket, `machine:<id>`) it is listed in; `None` is everywhere.
    pub show_in: Option<String>,
    pub open_from: OpenFrom,
}

impl WebEntry {
    /// An empty entry with an id of its own.
    pub fn new() -> WebEntry {
        use std::sync::atomic::{AtomicU64, Ordering};
        static MADE: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
        WebEntry { id: format!("{:x}{:x}", nanos, MADE.fetch_add(1, Ordering::Relaxed)), ..Default::default() }
    }
}

/// Where a web entry's URL is opened: on this computer as written, or on the shown place's machine through an SSH
/// forward of its port. An unknown (future) value reads as this computer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenFrom {
    Machine,
    #[default]
    #[serde(other)]
    Here,
}

/// The parts of a workspace row that can be hidden; all shown by default.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceShow {
    /// The folder, or a worktree's branch, before the name.
    pub icon: bool,
    /// The checked-out branch, on a line under the name (not when compact).
    pub branch: bool,
    /// A worktree's lines added and removed.
    pub changes: bool,
    /// A worktree's pull request state.
    pub pull_request: bool,
    /// The dot for the workspace's agents.
    pub status: bool,
}

impl Default for WorkspaceShow {
    fn default() -> Self {
        WorkspaceShow { icon: true, branch: true, changes: true, pull_request: true, status: true }
    }
}

/// The status bar's labels that can be hidden; all shown by default.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusBarShow {
    pub branch: bool,
    pub pull_request: bool,
    pub folder: bool,
    /// The shown agent's token spend.
    pub tokens: bool,
    /// Claude's plan limits.
    pub usage: bool,
}

impl Default for StatusBarShow {
    fn default() -> Self {
        StatusBarShow { branch: true, pull_request: true, folder: true, tokens: true, usage: true }
    }
}

/// The window's frame on its display, top-left origin, as moving or resizing it leaves it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowFrame {
    /// The display's UUID: the frame is relative to it, and goes with it.
    pub display: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub fullscreen: bool,
    /// Zoomed (the green button's fill); the frame is then the one un-zooming goes back to.
    pub maximized: bool,
}

impl WindowFrame {
    /// This frame zoomed: it keeps `last`'s frame on the same display, the one un-zooming goes back to.
    pub fn zoomed(self, last: Option<&WindowFrame>) -> WindowFrame {
        let base = last.filter(|l| l.display == self.display).cloned().unwrap_or(self);
        WindowFrame { maximized: true, fullscreen: false, ..base }
    }

    /// The frame on a display of `display` size, or `None` when it is bogus or less than 64 px of it is on screen.
    pub fn bounds_on(&self, display: Size<Pixels>) -> Option<Bounds<Pixels>> {
        let finite = [self.x, self.y, self.width, self.height].iter().all(|v| v.is_finite());
        if !finite || self.width <= 0.0 || self.height <= 0.0 {
            return None;
        }
        let bounds = Bounds::new(point(px(self.x), px(self.y)), size(px(self.width), px(self.height)));
        let seen = bounds.intersect(&Bounds::new(point(px(0.), px(0.)), display)).size;
        (seen.width >= px(64.) && seen.height >= px(64.)).then_some(bounds)
    }
}

/// Zeron's sidebar: 256 px by default, its edge drags it between 224 and 400.
pub const SIDEBAR_WIDTH: f32 = 256.0;
pub const SIDEBAR_MIN: f32 = 224.0;
pub const SIDEBAR_MAX: f32 = 400.0;
/// The review panel: 340 px by default, its edge drags it between 280 and 640.
pub const REVIEW_WIDTH: f32 = 340.0;
pub const REVIEW_MIN: f32 = 280.0;
pub const REVIEW_MAX: f32 = 640.0;

impl Default for Settings {
    fn default() -> Self {
        let font = FontSettings::default();
        Settings {
            font_family: font.family,
            font_size: font.size,
            line_height: font.line_height,
            theme_light: crate::theme::DEFAULT_LIGHT.into(),
            theme_dark: crate::theme::DEFAULT_DARK.into(),
            theme_family: None,
            appearance: AppearanceMode::System,
            herdr_bin: None,
            herdr_socket: None,
            herdr_machine: None,
            place_looks: BTreeMap::new(),
            place_order: Vec::new(),
            editor: None,
            keys: BTreeMap::new(),
            sidebar_width: SIDEBAR_WIDTH,
            review_open: false,
            review_width: REVIEW_WIDTH,
            agent_sort: AgentSort::default(),
            show_agents: true,
            show_status_bar: true,
            status_bar_show: StatusBarShow::default(),
            usage_agent: UsageAgent::default(),
            workspace_show: WorkspaceShow::default(),
            compact_workspaces: false,
            compact_agents: false,
            status_icons: crate::theme::StatusIcons::Dots,
            attention_agents: false,
            agents_height: None,
            recent_commands: Vec::new(),
            notify_agents: true,
            notify_sound: true,
            dock_badge: true,
            copy_on_select: true,
            animated_cursor: true,
            secure_input: false,
            translucent_window: cfg!(target_os = "macos"),
            window_buttons: true,
            sidebar_open: true,
            folded: Vec::new(),
            agents_folded: Vec::new(),
            web_entries: Vec::new(),
            web_links_outside: false,
            web_links_inside: false,
            window: None,
        }
    }
}

pub(crate) fn home() -> PathBuf {
    std::env::home_dir().unwrap_or_default()
}

/// Where the settings live: macOS's Application Support, Windows' roaming AppData, else the XDG config folder.
#[cfg(target_os = "macos")]
fn config_dir() -> PathBuf {
    home().join("Library/Application Support/Shepherd")
}

#[cfg(windows)]
fn config_dir() -> PathBuf {
    std::env::var_os("APPDATA").map_or_else(|| home().join("AppData").join("Roaming"), PathBuf::from).join("Shepherd")
}

#[cfg(not(any(target_os = "macos", windows)))]
fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map_or_else(|| home().join(".config"), PathBuf::from).join("shepherd")
}

/// Shepherd's caches: macOS's Library/Caches, Windows' local AppData, else the XDG cache folder. Short enough for a socket path under it
/// (104 bytes on macOS).
#[cfg(target_os = "macos")]
pub fn cache_dir() -> PathBuf {
    home().join("Library/Caches/Shepherd")
}

#[cfg(windows)]
pub fn cache_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map_or_else(|| home().join("AppData").join("Local"), PathBuf::from).join("Shepherd")
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME").map_or_else(|| home().join(".cache"), PathBuf::from).join("shepherd")
}

impl Settings {
    /// `SHEPHERD_SETTINGS_FILE` runs a second Shepherd with its own settings (a demo session, say).
    pub fn path() -> PathBuf {
        if let Some(file) = std::env::var_os("SHEPHERD_SETTINGS_FILE") {
            return file.into();
        }
        config_dir().join("settings.json")
    }

    /// Defaults when the file is missing; defaults and an error naming the file when it cannot be read. The file
    /// is never rewritten here, so a broken one survives until the user saves.
    pub fn load(path: &Path) -> (Settings, Option<String>) {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Settings::default(), None),
            Err(e) => return (Settings::default(), Some(format!("cannot read {}: {e}", path.display()))),
        };
        match serde_json::from_str::<Settings>(&text) {
            Ok(mut settings) => {
                // A hand-edited size of 0 would make zero-height rows and a 500×500 PTY in herdr.
                settings.font_size = settings.font_size.clamp(9.0, 32.0);
                settings.line_height = step_line_height(settings.line_height, 0.0);
                settings.sidebar_width = settings.sidebar_width.clamp(SIDEBAR_MIN, SIDEBAR_MAX);
                settings.review_width = settings.review_width.clamp(REVIEW_MIN, REVIEW_MAX);
                if let Some(family) = settings.theme_family.take() {
                    settings.adopt_family(&family);
                }
                (settings, None)
            }
            Err(e) => (Settings::default(), Some(format!("{} is not valid settings: {e}", path.display()))),
        }
    }

    /// Writes a temporary file next to `path`, then renames it over: a crash never leaves half a file.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// The settings after `change`, or `None` when it changes nothing (so nothing is saved).
    pub fn changed_by(&self, change: impl FnOnce(&mut Settings)) -> Option<Settings> {
        let mut next = self.clone();
        change(&mut next);
        (next != *self).then_some(next)
    }

    /// Binds `chord` to `action`; another action the user had put on that chord goes back to its default.
    pub fn record_chord(&mut self, action: &str, chord: &str) {
        let chord = crate::bindings::normalize(chord).unwrap_or_default();
        if !chord.is_empty() {
            self.keys.retain(|a, k| a == action || crate::bindings::normalize(k).as_deref() != Some(chord.as_str()));
        }
        self.keys.insert(action.to_string(), chord);
    }

    pub fn font(&self) -> FontSettings {
        FontSettings { family: self.font_family.clone(), size: self.font_size, line_height: self.line_height }
    }

    pub fn theme(&self, system: WindowAppearance) -> Theme {
        Theme { status_icons: self.status_icons, translucent: self.translucent_window, ..Theme::resolve(&self.theme_light, &self.theme_dark, self.appearance, system, &self.font()) }
    }

    /// The old single family as the light and dark themes: its variant of each appearance it has.
    fn adopt_family(&mut self, family: &str) {
        for v in crate::theme::variants().iter().filter(|v| v.family_id == family) {
            let slot = if v.appearance.is_dark() { &mut self.theme_dark } else { &mut self.theme_light };
            *slot = v.id.clone();
        }
    }
}

/// The terminal font size one step up or down, within 9…32.
pub fn step_font_size(size: f32, delta: f32) -> f32 {
    (size + delta).clamp(9.0, 32.0)
}

/// The terminal line height one 0.1 step up or down, within 1.0…2.0. Rounded so steps never drift (1.4 + 0.1 = 1.5).
pub fn step_line_height(v: f32, delta: f32) -> f32 {
    ((v + delta) * 10.0).round().clamp(10.0, 20.0) / 10.0
}

/// The loaded settings, the load error (shown in Settings) and where they live.
pub struct SettingsGlobal {
    pub settings: Settings,
    pub error: Option<String>,
    pub path: PathBuf,
}

impl Global for SettingsGlobal {}

impl SettingsGlobal {
    pub fn of(cx: &App) -> &SettingsGlobal {
        cx.global::<SettingsGlobal>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_is_translucent_by_default_on_macos_only() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.translucent_window, cfg!(target_os = "macos"));
        assert_eq!(s.theme(WindowAppearance::Dark).translucent, s.translucent_window);
        let on: Settings = serde_json::from_str(r#"{"translucent_window": true}"#).unwrap();
        let theme = on.theme(WindowAppearance::Dark);
        assert!(theme.glass().a < theme.surface.a, "the fill lets the desktop through");
    }

    #[test]
    fn the_window_buttons_show_by_default() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert!(s.window_buttons);
        assert!(!serde_json::from_str::<Settings>(r#"{"window_buttons": false}"#).unwrap().window_buttons);
    }

    fn tmp() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        (dir, path)
    }

    #[test]
    fn web_entries_default_to_none_and_round_trip() {
        assert!(Settings::default().web_entries.is_empty());
        assert!(!Settings::default().web_links_outside);
        let (_dir, path) = tmp();
        let mut s = Settings::default();
        s.web_entries.push(WebEntry {
            id: "a1".into(),
            name: "board".into(),
            url: "http://localhost:4317".into(),
            show_in: Some("machine:m1".into()),
            open_from: OpenFrom::Machine,
        });
        s.web_links_outside = true;
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path).0, s);
    }

    #[test]
    fn an_unknown_open_from_reads_as_here() {
        let e: WebEntry = serde_json::from_str(r#"{"open_from":"moon"}"#).unwrap();
        assert_eq!(e.open_from, OpenFrom::Here);
    }

    #[test]
    fn new_web_entries_get_distinct_ids() {
        assert_ne!(WebEntry::new().id, WebEntry::new().id);
    }

    #[test]
    fn missing_file_gives_defaults_without_error() {
        let (_d, path) = tmp();
        let (s, err) = Settings::load(&path);
        assert_eq!(s, Settings::default());
        assert!(err.is_none());
        assert_eq!(s.font_family, "Geist Mono");
        assert_eq!(s.font_size, 13.0);
        assert_eq!(s.theme_dark, "ayu-dark");
    }

    #[test]
    fn empty_object_and_unknown_fields_give_defaults() {
        let (_d, path) = tmp();
        std::fs::write(&path, r#"{"from_the_future": 1}"#).unwrap();
        let (s, err) = Settings::load(&path);
        assert_eq!(s, Settings::default());
        assert!(err.is_none());
    }

    #[test]
    fn a_partial_file_keeps_what_it_sets() {
        let (_d, path) = tmp();
        std::fs::write(&path, r#"{"font_size": 15, "appearance": "dark", "keys": {"NewTab": "cmd-n"}}"#).unwrap();
        let (s, _) = Settings::load(&path);
        assert_eq!(s.font_size, 15.0);
        assert_eq!(s.appearance, crate::theme::AppearanceMode::Dark);
        assert_eq!(s.keys.get("NewTab").map(String::as_str), Some("cmd-n"));
        assert_eq!(s.font_family, "Geist Mono");
    }

    #[test]
    fn a_broken_file_gives_defaults_an_error_and_stays_untouched() {
        let (_d, path) = tmp();
        std::fs::write(&path, "{not json").unwrap();
        let (s, err) = Settings::load(&path);
        assert_eq!(s, Settings::default());
        assert!(err.unwrap().contains(&path.display().to_string()));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{not json");
    }

    #[test]
    fn recording_a_chord_takes_it_from_other_user_bindings() {
        let mut s = Settings::default();
        s.keys.insert("NewTab".into(), "cmd-n".into());
        s.record_chord("SplitRight", "cmd-n");
        assert_eq!(s.keys.get("SplitRight"), crate::bindings::normalize("cmd-n").as_ref());
        assert_eq!(s.keys.get("NewTab"), None, "NewTab falls back to its default");
    }

    #[test]
    fn font_size_steps_within_bounds() {
        assert_eq!(step_font_size(13.0, 1.0), 14.0);
        assert_eq!(step_font_size(9.0, -1.0), 9.0);
        assert_eq!(step_font_size(32.0, 1.0), 32.0);
    }

    #[test]
    fn line_height_steps_within_bounds() {
        assert_eq!(step_line_height(1.4, 0.1), 1.5);
        assert_eq!(step_line_height(1.0, -0.1), 1.0);
        assert_eq!(step_line_height(2.0, 0.1), 2.0);
        assert_eq!(step_line_height(0.0, 0.0), 1.0, "a hand-edited 0 clamps up");
    }

    #[test]
    fn an_unchanged_value_is_no_change() {
        // Leaving a field without editing must not save (a broken file would be overwritten with defaults).
        let s = Settings::default();
        assert_eq!(s.changed_by(|s| s.font_family = "Geist Mono".into()), None);
        assert_eq!(s.changed_by(|s| s.font_family = "Menlo".into()).map(|n| n.font_family), Some("Menlo".into()));
    }

    #[test]
    fn unbinding_one_action_keeps_the_others_unbound() {
        let mut s = Settings::default();
        s.record_chord("ClosePane", "");
        s.record_chord("Quit", "");
        assert_eq!(s.keys.get("ClosePane").map(String::as_str), Some(""));
        assert_eq!(s.keys.get("Quit").map(String::as_str), Some(""));
    }

    #[test]
    fn recorded_chords_are_stored_normalized() {
        let mut s = Settings::default();
        s.keys.insert("NewTab".into(), "cmd-alt-left".into());
        s.record_chord("ToggleZoom", "alt-cmd-left");
        assert_eq!(s.keys.get("NewTab"), None, "same chord written another way");
    }

    #[test]
    fn a_hand_edited_font_size_is_clamped() {
        let (_d, path) = tmp();
        std::fs::write(&path, r#"{"font_size": 0}"#).unwrap();
        assert_eq!(Settings::load(&path).0.font_size, 9.0);
        std::fs::write(&path, r#"{"font_size": 400}"#).unwrap();
        assert_eq!(Settings::load(&path).0.font_size, 32.0);
    }

    #[test]
    fn an_unknown_agent_sort_reads_as_status() {
        let (_d, path) = tmp();
        std::fs::write(&path, r#"{"agent_sort": "workspace", "font_size": 15, "usage_agent": "gemini"}"#).unwrap();
        let (s, err) = Settings::load(&path);
        assert!(err.is_none());
        assert_eq!((s.agent_sort, s.font_size, s.usage_agent), (AgentSort::Status, 15.0, UsageAgent::Claude));
    }

    #[test]
    fn a_hand_edited_sidebar_width_is_clamped() {
        let (_d, path) = tmp();
        std::fs::write(&path, r#"{"sidebar_width": 10}"#).unwrap();
        assert_eq!(Settings::load(&path).0.sidebar_width, SIDEBAR_MIN);
        std::fs::write(&path, r#"{"review_width": 9000}"#).unwrap();
        assert_eq!(Settings::load(&path).0.review_width, REVIEW_MAX);
    }

    #[test]
    fn a_zoomed_window_keeps_the_frame_it_unzooms_to() {
        let last = WindowFrame { display: "d".into(), x: 100., y: 50., width: 1180., height: 760., ..Default::default() };
        let full = WindowFrame { x: 0., y: 0., width: 1512., height: 945., ..last.clone() };
        assert_eq!(full.clone().zoomed(Some(&last)), WindowFrame { maximized: true, ..last.clone() });
        let elsewhere = WindowFrame { display: "e".into(), ..last };
        assert_eq!(full.clone().zoomed(Some(&elsewhere)), WindowFrame { maximized: true, ..full });
    }

    #[test]
    fn a_window_frame_off_the_display_or_bogus_opens_centered() {
        let display = size(px(1512.), px(982.));
        let frame = WindowFrame { display: "d".into(), x: 100., y: 50., width: 1180., height: 760., ..Default::default() };
        assert!(frame.bounds_on(display).is_some());
        assert!(WindowFrame { x: 3000., ..frame.clone() }.bounds_on(display).is_none(), "on a display now gone");
        assert!(WindowFrame { x: 1490., ..frame.clone() }.bounds_on(display).is_none(), "a sliver on screen");
        assert!(WindowFrame { width: 0., ..frame.clone() }.bounds_on(display).is_none());
        assert!(WindowFrame { height: f32::INFINITY, ..frame }.bounds_on(display).is_none());
        let (_d, path) = tmp();
        std::fs::write(&path, r#"{"window": {"x": 10}, "font_size": 15}"#).unwrap();
        let (s, err) = Settings::load(&path);
        assert!(err.is_none());
        assert_eq!((s.window.unwrap().bounds_on(display), s.font_size), (None, 15.0));
    }

    #[test]
    fn save_then_load_round_trips() {
        let (_d, path) = tmp();
        let mut s = Settings {
            font_family: "IoskeleyMonoNL Nerd Font Mono".into(),
            herdr_bin: Some("/opt/homebrew/bin/herdr".into()),
            ..Default::default()
        };
        s.keys.insert("SplitRight".into(), "cmd-shift-right".into());
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), (s, None));
    }

    #[test]
    fn an_old_theme_family_becomes_the_light_and_dark_themes() {
        let (_d, path) = tmp();
        std::fs::write(&path, r#"{"theme_family": "nord"}"#).unwrap();
        let (s, _) = Settings::load(&path);
        assert_eq!((s.theme_light.as_str(), s.theme_dark.as_str()), ("ayu-light", "nord"));
        assert!(!serde_json::to_string(&s).unwrap().contains("theme_family"));
    }
}
