//! herdr's `[keys]` in Shepherd: its chords for the actions Shepherd has, prefix ones included, and its
//! `[[keys.command]]` (plugin actions, shell commands), so a keymap set up for herdr's TUI works here too.
use super::config::{HerdrConfig, str_list};
use toml_edit::Item;

/// A `[[keys.command]]`, run as herdr runs it.
#[derive(Clone, Debug, PartialEq, gpui::Action)]
#[action(namespace = shepherd, no_json)]
pub struct HerdrCommand {
    pub kind: CommandKind,
    /// A plugin action's id (`plugin.action`, or a bare `action`), or a shell command line.
    pub command: String,
    /// Its keys as the config spells them: how herdr's own list of these commands names it.
    pub keys: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CommandKind {
    Shell,
    Pane,
    Popup,
    PluginAction,
}

/// What a herdr chord runs: a Shepherd action by its name in `bindings::ACTIONS`, or a herdr command.
#[derive(Clone, Debug, PartialEq)]
pub enum Bind {
    App(&'static str),
    Command(HerdrCommand),
}

/// herdr's actions Shepherd has: herdr's name, its default (`herdr --default-config`), Shepherd's action.
const ACTIONS: &[(&str, &str, &str)] = &[
    ("settings", "prefix+s", "OpenSettings"),
    ("workspace_picker", "prefix+w", "CommandPanel"),
    ("new_workspace", "prefix+shift+n", "NewWorkspace"),
    ("previous_workspace", "", "PreviousWorkspace"),
    ("next_workspace", "", "NextWorkspace"),
    ("new_tab", "prefix+c", "NewTab"),
    ("previous_tab", "prefix+p", "PreviousTab"),
    ("next_tab", "prefix+n", "NextTab"),
    ("close_tab", "prefix+shift+x", "CloseTab"),
    ("focus_pane_left", "prefix+h", "FocusLeft"),
    ("focus_pane_down", "prefix+j", "FocusDown"),
    ("focus_pane_up", "prefix+k", "FocusUp"),
    ("focus_pane_right", "prefix+l", "FocusRight"),
    ("split_vertical", "prefix+v", "SplitRight"),
    ("split_horizontal", "prefix+minus", "SplitDown"),
    ("close_pane", "prefix+x", "ClosePane"),
    ("zoom", "prefix+z", "ToggleZoom"),
    ("toggle_sidebar", "prefix+b", "ToggleSidebar"),
];

const TABS: [&str; 9] = ["Tab1", "Tab2", "Tab3", "Tab4", "Tab5", "Tab6", "Tab7", "Tab8", "Tab9"];

/// Every chord herdr's config binds to something Shepherd runs, in GPUI's spelling.
pub fn bindings(config: &HerdrConfig) -> Vec<(Bind, String)> {
    let prefixes: Vec<String> =
        config.strs("keys.prefix").unwrap_or_else(|| vec!["ctrl+b".into()]).iter().filter_map(|p| keystroke(p)).collect();
    let chords = |label: &str| -> Vec<String> {
        match label.trim().strip_prefix("prefix+") {
            Some(rest) => keystroke(rest).map_or(Vec::new(), |k| prefixes.iter().map(|p| format!("{p} {k}")).collect()),
            None => keystroke(label).into_iter().collect(),
        }
    };
    let labels = |key: &str, default: &str| config.strs(&format!("keys.{key}")).unwrap_or_else(|| vec![default.into()]);
    let mut out = Vec::new();
    for (herdr, default, app) in ACTIONS {
        for label in labels(herdr, default) {
            out.extend(chords(&label).into_iter().map(|c| (Bind::App(app), c)));
        }
    }
    for label in labels("switch_tab", "prefix+1..9").iter().filter(|l| l.contains("1..9")) {
        for (i, app) in TABS.iter().enumerate() {
            out.extend(chords(&label.replace("1..9", &(i + 1).to_string())).into_iter().map(|c| (Bind::App(app), c)));
        }
    }
    let commands = config.0.get("keys").and_then(|k| k.get("command")).and_then(Item::as_array_of_tables);
    for table in commands.into_iter().flat_map(|a| a.iter()) {
        let kind = match table.get("type").and_then(Item::as_str).unwrap_or("shell") {
            "shell" => CommandKind::Shell,
            "pane" => CommandKind::Pane,
            "popup" => CommandKind::Popup,
            "plugin_action" => CommandKind::PluginAction,
            _ => continue,
        };
        let Some(command) = table.get("command").and_then(Item::as_str) else { continue };
        let keys = table.get("key").and_then(Item::as_value).map(str_list).unwrap_or_default();
        let command = HerdrCommand { kind, command: command.to_string(), keys: keys.clone() };
        for label in keys {
            out.extend(chords(&label).into_iter().map(|c| (Bind::Command(command.clone()), c)));
        }
    }
    out
}

/// What shift makes of a key on a US layout: GPUI matches the typed character, so `shift+1` is `!`.
const SHIFTED: &[(char, char)] = &[
    ('1', '!'), ('2', '@'), ('3', '#'), ('4', '$'), ('5', '%'), ('6', '^'), ('7', '&'), ('8', '*'), ('9', '('),
    ('0', ')'), ('-', '_'), ('=', '+'), ('[', '{'), (']', '}'), (';', ':'), ('\'', '"'), (',', '<'), ('.', '>'),
    ('/', '?'), ('\\', '|'), ('`', '~'),
];

/// One herdr key (`ctrl+shift+k`, `minus`, `%`) in GPUI's spelling; `None` for an empty or unknown one.
fn keystroke(label: &str) -> Option<String> {
    let (mut mods, mut shift, mut key) = (Vec::new(), false, None);
    for part in label.split('+').map(str::trim) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods.push("ctrl"),
            "alt" | "option" | "meta" => mods.push("alt"),
            "cmd" | "command" | "super" => mods.push("cmd"),
            "shift" => shift = true,
            _ if part.is_empty() || key.is_some() => return None,
            _ => key = Some(part),
        }
    }
    let key = key?;
    let named = match key.to_ascii_lowercase().as_str() {
        "space" | " " => "space",
        "enter" | "return" => "enter",
        "esc" | "escape" => "escape",
        "bs" | "backspace" => "backspace",
        "minus" => "-",
        "comma" => ",",
        "period" => ".",
        "slash" => "/",
        "backslash" => "\\",
        "quote" => "'",
        "double_quote" | "double-quote" => "\"",
        "semicolon" => ";",
        "colon" => ":",
        "percent" => "%",
        "ampersand" => "&",
        "backtick" => "`",
        "plus" => "+",
        _ => "",
    };
    let mut chars = key.chars();
    let key = match (named, chars.next(), chars.next()) {
        ("", Some(c), None) if c.is_ascii_uppercase() => {
            shift = true;
            c.to_ascii_lowercase().to_string()
        }
        ("", Some(c), None) => c.to_string(),
        ("", ..) => key.to_ascii_lowercase(), // tab, left, f5
        (named, ..) => named.to_string(),
    };
    let mut chars = key.chars();
    let key = match (chars.next(), chars.next()) {
        // a shifted digit or punctuation is the character it types; a letter or named key keeps shift
        (Some(c), None) if shift && !c.is_ascii_alphabetic() => {
            shift = false;
            SHIFTED.iter().find(|(k, _)| *k == c).map_or(c, |(_, s)| *s).to_string()
        }
        _ => key,
    };
    if shift {
        mods.push("shift");
    }
    mods.push(&key);
    crate::bindings::normalize(&mods.join("-"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(toml: &str) -> HerdrConfig {
        HerdrConfig(toml.parse().unwrap())
    }

    fn chord(binds: &[(Bind, String)], bind: Bind) -> Vec<&str> {
        binds.iter().filter(|(b, _)| *b == bind).map(|(_, c)| c.as_str()).collect()
    }

    #[test]
    fn herdr_keys_in_gpui_spelling() {
        let k = |s| keystroke(s);
        assert_eq!(k("ctrl+b").as_deref(), Some("ctrl-b"));
        assert_eq!(k("ctrl+alt+n"), crate::bindings::normalize("ctrl-alt-n"));
        assert_eq!(k("shift+k").as_deref(), Some("shift-k"));
        assert_eq!(k("K").as_deref(), Some("shift-k"));
        assert_eq!(k("minus").as_deref(), Some("-"));
        assert_eq!(k("%").as_deref(), Some("%"));
        assert_eq!(k("shift+5").as_deref(), Some("%"));
        assert_eq!(k("cmd+shift+}"), crate::bindings::normalize("cmd-}"));
        assert_eq!(k("shift+tab").as_deref(), Some("shift-tab"));
        assert_eq!(k("f5").as_deref(), Some("f5"));
        assert_eq!(k(""), None);
        assert_eq!(k("ctrl+a+b"), None);
    }

    #[test]
    fn defaults_hang_off_the_prefix() {
        let binds = bindings(&config(""));
        assert_eq!(chord(&binds, Bind::App("NewTab")), ["ctrl-b c"]);
        assert_eq!(chord(&binds, Bind::App("SplitDown")), ["ctrl-b -"]);
        assert_eq!(chord(&binds, Bind::App("Tab3")), ["ctrl-b 3"]);
        assert!(chord(&binds, Bind::App("NextWorkspace")).is_empty(), "unset by default");
    }

    #[test]
    fn the_users_keys_replace_the_defaults() {
        let binds = bindings(&config(
            r#"
[keys]
prefix = ["ctrl+a", "f12"]
split_vertical = "prefix+%"
next_tab = ["prefix+n", "ctrl+alt+]"]
close_pane = ""
toggle_sidebar = "cmd+b"
"#,
        ));
        assert_eq!(chord(&binds, Bind::App("SplitRight")), ["ctrl-a %", "f12 %"]);
        assert_eq!(chord(&binds, Bind::App("NextTab")).len(), 3);
        assert!(chord(&binds, Bind::App("ClosePane")).is_empty(), "empty unbinds");
        assert_eq!(chord(&binds, Bind::App("ToggleSidebar")), ["cmd-b"]);
    }

    #[test]
    fn commands_run_plugin_actions_and_shell_lines() {
        let binds = bindings(&config(
            r#"
[[keys.command]]
key = "ctrl+h"
type = "plugin_action"
command = "vim-herdr-navigation.left"

[[keys.command]]
key = "prefix+t"
command = "say hi"

[[keys.command]]
key = "prefix+y"
type = "teleport"
command = "x"
"#,
        ));
        let plugin = HerdrCommand { kind: CommandKind::PluginAction, command: "vim-herdr-navigation.left".into(), keys: vec!["ctrl+h".into()] };
        assert_eq!(chord(&binds, Bind::Command(plugin)), ["ctrl-h"]);
        let shell = HerdrCommand { kind: CommandKind::Shell, command: "say hi".into(), keys: vec!["prefix+t".into()] };
        assert_eq!(chord(&binds, Bind::Command(shell)), ["ctrl-b t"]);
        assert_eq!(binds.iter().filter(|(b, _)| matches!(b, Bind::Command(_))).count(), 2, "unknown types are skipped");
    }
}
