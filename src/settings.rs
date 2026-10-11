//! Shepherd's settings: a JSON file every field of which has a default, so old, partial or future files load.
use crate::model::AgentSort;
use crate::theme::{AppearanceMode, FontSettings, Theme};
use gpui::{App, Global, WindowAppearance};
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
    pub theme_family: String,
    pub appearance: AppearanceMode,
    /// The herdr binary; `None` is `herdr` on PATH.
    pub herdr_bin: Option<String>,
    /// herdr's API socket; `None` is herdr's own default.
    pub herdr_socket: Option<String>,
    /// Action name → keystroke (`cmd-shift-d`); an empty string unbinds the action.
    pub keys: BTreeMap<String, String>,
    /// The workspaces sidebar's width; dragging its edge sets it.
    pub sidebar_width: f32,
    pub agent_sort: AgentSort,
    /// The sidebar's Agents section; the `…` menu's switch hides it.
    pub show_agents: bool,
    /// The Agents list's height cap; dragging the section's top edge sets it, `None` is 40 % of the sidebar.
    pub agents_height: Option<f32>,
}

/// Zeron's sidebar: 256 px by default, its edge drags it between 224 and 400.
pub const SIDEBAR_WIDTH: f32 = 256.0;
pub const SIDEBAR_MIN: f32 = 224.0;
pub const SIDEBAR_MAX: f32 = 400.0;

impl Default for Settings {
    fn default() -> Self {
        let font = FontSettings::default();
        Settings {
            font_family: font.family,
            font_size: font.size,
            line_height: font.line_height,
            theme_family: "zeron".into(),
            appearance: AppearanceMode::System,
            herdr_bin: None,
            herdr_socket: None,
            keys: BTreeMap::new(),
            sidebar_width: SIDEBAR_WIDTH,
            agent_sort: AgentSort::default(),
            show_agents: true,
            agents_height: None,
        }
    }
}

impl Settings {
    pub fn path() -> PathBuf {
        PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join("Library/Application Support/Shepherd/settings.json")
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
        Theme::resolve(&self.theme_family, self.appearance, system, &self.font())
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

    fn tmp() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        (dir, path)
    }

    #[test]
    fn missing_file_gives_defaults_without_error() {
        let (_d, path) = tmp();
        let (s, err) = Settings::load(&path);
        assert_eq!(s, Settings::default());
        assert!(err.is_none());
        assert_eq!(s.font_family, "Geist Mono");
        assert_eq!(s.font_size, 13.0);
        assert_eq!(s.theme_family, "zeron");
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
        assert_eq!(s.keys.get("SplitRight").map(String::as_str), Some("cmd-n"));
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
        std::fs::write(&path, r#"{"agent_sort": "workspace", "font_size": 15}"#).unwrap();
        let (s, err) = Settings::load(&path);
        assert!(err.is_none());
        assert_eq!((s.agent_sort, s.font_size), (AgentSort::Status, 15.0));
    }

    #[test]
    fn a_hand_edited_sidebar_width_is_clamped() {
        let (_d, path) = tmp();
        std::fs::write(&path, r#"{"sidebar_width": 10}"#).unwrap();
        assert_eq!(Settings::load(&path).0.sidebar_width, SIDEBAR_MIN);
    }

    #[test]
    fn save_then_load_round_trips() {
        let (_d, path) = tmp();
        let mut s = Settings::default();
        s.font_family = "IoskeleyMonoNL Nerd Font Mono".into();
        s.herdr_bin = Some("/opt/homebrew/bin/herdr".into());
        s.keys.insert("SplitRight".into(), "cmd-shift-right".into());
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), (s, None));
    }
}
