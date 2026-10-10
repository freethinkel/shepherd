//! App actions and their key bindings: defaults (Ghostty-like) overlaid by the user's settings.
use gpui::{App, KeyBinding, Keystroke, NoAction};
use std::collections::BTreeMap;

macro_rules! app_actions {
    ($($name:ident: $label:literal),+ $(,)?) => {
        gpui::actions!(shepherd, [$($name),+]);

        /// Every app action as (name in settings, label in the Keys settings).
        pub const ACTIONS: &[(&str, &str)] = &[$((stringify!($name), $label)),+];

        fn binding(action: &str, keystroke: &str) -> Option<KeyBinding> {
            match action {
                $(stringify!($name) => Some(KeyBinding::new(keystroke, $name, None)),)+
                _ => None,
            }
        }
    };
}

app_actions! {
    NewTab: "New tab",
    CloseTab: "Close tab",
    NewWorkspace: "New workspace",
    SplitRight: "Split right",
    SplitDown: "Split down",
    ClosePane: "Close pane",
    FocusLeft: "Focus pane left",
    FocusRight: "Focus pane right",
    FocusUp: "Focus pane up",
    FocusDown: "Focus pane down",
    ToggleZoom: "Zoom pane",
    Tab1: "Go to tab 1",
    Tab2: "Go to tab 2",
    Tab3: "Go to tab 3",
    Tab4: "Go to tab 4",
    Tab5: "Go to tab 5",
    Tab6: "Go to tab 6",
    Tab7: "Go to tab 7",
    Tab8: "Go to tab 8",
    Tab9: "Go to tab 9",
    PreviousWorkspace: "Previous workspace",
    NextWorkspace: "Next workspace",
    OpenSettings: "Settings",
    Quit: "Quit",
}

pub const DEFAULTS: &[(&str, &str)] = &[
    ("NewTab", "cmd-t"),
    ("CloseTab", "cmd-shift-w"),
    ("NewWorkspace", "cmd-shift-n"),
    ("SplitRight", "cmd-d"),
    ("SplitDown", "cmd-shift-d"),
    ("ClosePane", "cmd-w"),
    ("FocusLeft", "cmd-alt-left"),
    ("FocusRight", "cmd-alt-right"),
    ("FocusUp", "cmd-alt-up"),
    ("FocusDown", "cmd-alt-down"),
    ("ToggleZoom", "cmd-shift-enter"),
    ("Tab1", "cmd-1"),
    ("Tab2", "cmd-2"),
    ("Tab3", "cmd-3"),
    ("Tab4", "cmd-4"),
    ("Tab5", "cmd-5"),
    ("Tab6", "cmd-6"),
    ("Tab7", "cmd-7"),
    ("Tab8", "cmd-8"),
    ("Tab9", "cmd-9"),
    ("PreviousWorkspace", "cmd-up"),
    ("NextWorkspace", "cmd-down"),
    ("OpenSettings", "cmd-,"),
    ("Quit", "cmd-q"),
];

fn known(action: &str) -> bool {
    ACTIONS.iter().any(|(name, _)| *name == action)
}

/// A chord GPUI can bind: space-separated keystrokes that all parse.
pub fn valid_chord(chord: &str) -> bool {
    !chord.trim().is_empty() && chord.split_whitespace().all(|k| Keystroke::parse(k).is_ok())
}

/// The bindings in force: the user's chords first, then defaults for actions the user did not set, never two
/// actions on one chord (the user's wins). An empty user chord unbinds its action.
pub fn effective(user: &BTreeMap<String, String>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (action, chord) in user {
        if known(action) && valid_chord(chord) {
            out.retain(|(_, k)| k != chord);
            out.push((action.clone(), chord.clone()));
        }
    }
    for (action, chord) in DEFAULTS {
        if !user.contains_key(*action) && !out.iter().any(|(_, k)| k == chord) {
            out.push((action.to_string(), chord.to_string()));
        }
    }
    out
}

/// The bindings currently applied, so a settings change can unbind what it replaces.
pub struct ActiveBindings(pub Vec<(String, String)>);

impl gpui::Global for ActiveBindings {}

/// Applies the bindings `user` asks for, replacing the ones in force.
pub fn rebind(cx: &mut App, user: &BTreeMap<String, String>) {
    let previous = cx.try_global::<ActiveBindings>().map(|a| a.0.clone()).unwrap_or_default();
    let next = effective(user);
    apply(cx, &previous, &next);
    cx.set_global(ActiveBindings(next));
}

/// Binds `next`; chords of `previous` that `next` no longer uses are bound to `NoAction` (a later binding
/// wins in GPUI), so other bindings, such as gpui-base's input keys, are left alone.
pub fn apply(cx: &mut App, previous: &[(String, String)], next: &[(String, String)]) {
    let mut bindings: Vec<KeyBinding> = previous
        .iter()
        .filter(|(_, chord)| !next.iter().any(|(_, k)| k == chord))
        .map(|(_, chord)| KeyBinding::new(chord, NoAction, None))
        .collect();
    bindings.extend(next.iter().filter_map(|(action, chord)| binding(action, chord)));
    cx.bind_keys(bindings);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn user(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(a, k)| (a.to_string(), k.to_string())).collect()
    }

    fn chord<'a>(eff: &'a [(String, String)], action: &str) -> Option<&'a str> {
        eff.iter().find(|(a, _)| a == action).map(|(_, k)| k.as_str())
    }

    #[test]
    fn defaults_when_the_user_set_nothing() {
        let eff = effective(&user(&[]));
        assert_eq!(chord(&eff, "SplitRight"), Some("cmd-d"));
        assert_eq!(chord(&eff, "NewTab"), Some("cmd-t"));
        assert_eq!(chord(&eff, "OpenSettings"), Some("cmd-,"));
    }

    #[test]
    fn a_user_chord_replaces_the_default() {
        let eff = effective(&user(&[("SplitRight", "cmd-shift-right")]));
        assert_eq!(chord(&eff, "SplitRight"), Some("cmd-shift-right"));
    }

    #[test]
    fn rebinding_takes_the_chord_from_the_old_action() {
        let eff = effective(&user(&[("NewTab", "cmd-d")]));
        assert_eq!(chord(&eff, "NewTab"), Some("cmd-d"));
        assert_eq!(chord(&eff, "SplitRight"), None, "two actions on one chord");
    }

    #[test]
    fn an_empty_chord_unbinds() {
        let eff = effective(&user(&[("ClosePane", "")]));
        assert_eq!(chord(&eff, "ClosePane"), None);
    }

    #[test]
    fn unknown_actions_are_ignored() {
        let eff = effective(&user(&[("Teleport", "cmd-j")]));
        assert_eq!(chord(&eff, "Teleport"), None);
    }

    #[test]
    fn every_default_names_a_known_action() {
        for (action, _) in DEFAULTS {
            assert!(ACTIONS.iter().any(|(name, _)| name == action), "{action}");
        }
    }
}
