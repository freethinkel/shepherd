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
    PreviousTab: "Previous tab",
    NextTab: "Next tab",
    PreviousWorkspace: "Previous workspace",
    NextWorkspace: "Next workspace",
    ToggleSidebar: "Toggle sidebar",
    OpenSettings: "Settings",
    CommandPanel: "Command panel",
    AgentsPanel: "Agents",
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
    ("PreviousTab", "cmd-{"),
    ("NextTab", "cmd-}"),
    ("PreviousWorkspace", "cmd-up"),
    ("NextWorkspace", "cmd-down"),
    ("ToggleSidebar", "cmd-b"),
    ("OpenSettings", "cmd-,"),
    ("CommandPanel", "cmd-k"),
    ("CommandPanel", "cmd-shift-p"),
    ("AgentsPanel", "cmd-shift-a"),
    ("Quit", "cmd-q"),
];

fn known(action: &str) -> bool {
    ACTIONS.iter().any(|(name, _)| *name == action)
}

/// The chord in GPUI's own spelling (`alt-cmd-left`), so the same keys written two ways compare equal.
pub fn normalize(chord: &str) -> Option<String> {
    let keys: Option<Vec<String>> = chord.split_whitespace().map(|k| Keystroke::parse(k).ok().map(|k| k.unparse())).collect();
    keys.filter(|k| !k.is_empty()).map(|k| k.join(" "))
}

/// A keystroke worth binding: with cmd, ctrl, alt or fn, or a function key. A bare letter would be taken from
/// every terminal.
pub fn recordable(keystroke: &Keystroke) -> bool {
    let m = &keystroke.modifiers;
    m.platform || m.control || m.alt || m.function || (keystroke.key.starts_with('f') && keystroke.key[1..].parse::<u8>().is_ok())
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
        if let (true, Some(chord)) = (known(action), normalize(chord)) {
            out.retain(|(_, k)| *k != chord);
            out.push((action.clone(), chord));
        }
    }
    for (action, chord) in DEFAULTS {
        let chord = normalize(chord).expect("defaults parse");
        if !user.contains_key(*action) && !out.iter().any(|(_, k)| *k == chord) {
            out.push((action.to_string(), chord));
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
    // macOS copies menu shortcuts when the menu is set: set it again so it shows (and only fires) the new ones.
    cx.set_menus(menus());
}

// Window-management items macOS apps carry; not rebindable, so outside `app_actions!`.
gpui::actions!(shepherd, [Hide, HideOthers, ShowAll, Minimize, Zoom, ToggleFullScreen]);

fn menus() -> Vec<gpui::Menu> {
    use gpui::{Menu, MenuItem as I, SystemMenuType};
    vec![
        Menu::new("Shepherd").items([
            I::action("Settings…", OpenSettings),
            I::separator(),
            I::os_submenu("Services", SystemMenuType::Services),
            I::separator(),
            I::action("Hide Shepherd", Hide),
            I::action("Hide Others", HideOthers),
            I::action("Show All", ShowAll),
            I::separator(),
            I::action("Quit Shepherd", Quit),
        ]),
        Menu::new("Shell").items([
            I::action("New Tab", NewTab),
            I::action("New Workspace", NewWorkspace),
            I::separator(),
            I::action("Split Right", SplitRight),
            I::action("Split Down", SplitDown),
            I::separator(),
            I::action("Close Pane", ClosePane),
            I::action("Close Tab", CloseTab),
        ]),
        Menu::new("View").items([
            I::action("Toggle Sidebar", ToggleSidebar),
            I::action("Zoom Pane", ToggleZoom),
            I::separator(),
            I::action("Command Panel", CommandPanel),
            I::action("Agents", AgentsPanel),
            I::separator(),
            I::action("Enter Full Screen", ToggleFullScreen),
        ]),
        Menu::new("Window").items([
            I::action("Minimize", Minimize),
            I::action("Zoom", Zoom),
            I::separator(),
            I::action("Previous Tab", PreviousTab),
            I::action("Next Tab", NextTab),
            I::action("Previous Workspace", PreviousWorkspace),
            I::action("Next Workspace", NextWorkspace),
            I::separator(),
            I::action("Focus Pane Left", FocusLeft),
            I::action("Focus Pane Right", FocusRight),
            I::action("Focus Pane Up", FocusUp),
            I::action("Focus Pane Down", FocusDown),
        ]),
    ]
}

/// Handlers for the window-management menu items.
pub fn init_menu_actions(cx: &mut App) {
    fn window(cx: &mut App, f: fn(&mut gpui::Window)) {
        if let Some(w) = cx.active_window() {
            let _ = w.update(cx, |_, window, _| f(window));
        }
    }
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &Minimize, cx| window(cx, |w| w.minimize_window()));
    cx.on_action(|_: &Zoom, cx| window(cx, |w| w.zoom_window()));
    cx.on_action(|_: &ToggleFullScreen, cx| window(cx, |w| w.toggle_fullscreen()));
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
    fn chords_written_differently_collide() {
        // GPUI's unparse writes alt before cmd; the defaults say cmd-alt-left.
        let eff = effective(&user(&[("NewTab", "alt-cmd-left")]));
        assert_eq!(chord(&eff, "FocusLeft"), None, "two actions on one chord");
        assert!(chord(&eff, "NewTab").is_some());
    }

    #[test]
    fn normalize_orders_modifiers_one_way() {
        assert_eq!(normalize("shift-cmd-d"), normalize("cmd-shift-d"));
        assert_eq!(normalize(""), None);
    }

    #[test]
    fn only_modified_or_function_keys_are_recordable() {
        let k = |s: &str| gpui::Keystroke::parse(s).unwrap();
        assert!(recordable(&k("cmd-d")));
        assert!(recordable(&k("ctrl-a")));
        assert!(recordable(&k("f5")));
        assert!(!recordable(&k("l")), "a bare letter would steal typing in every pane");
        assert!(!recordable(&k("shift-l")));
    }

    #[test]
    fn every_default_names_a_known_action() {
        for (action, _) in DEFAULTS {
            assert!(ACTIONS.iter().any(|(name, _)| name == action), "{action}");
        }
    }
}
