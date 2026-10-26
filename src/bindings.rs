//! App actions and their key bindings: defaults (Ghostty-like) overlaid by the user's settings.
use gpui::{App, KeyBinding, Keystroke, NoAction};
use std::collections::BTreeMap;

macro_rules! app_actions {
    ($($name:ident: $label:literal),+ $(,)?) => {
        gpui::actions!(shepherd, [$($name),+]);

        /// Every app action as (name in settings, label in the Keys settings).
        pub const ACTIONS: &[(&str, &str)] = &[$((stringify!($name), $label)),+];

        fn binding(action: &str, keystroke: &str, context: Option<&str>) -> Option<KeyBinding> {
            match action {
                $(stringify!($name) => Some(KeyBinding::new(keystroke, $name, context)),)+
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
    ToggleReview: "Toggle pull request panel",
    FontSizeUp: "Bigger text",
    FontSizeDown: "Smaller text",
    FontSizeReset: "Default text size",
    OpenSettings: "Settings",
    ChangeTheme: "Change theme",
    ToggleSecureInput: "Secure keyboard entry",
    CommandPanel: "Command panel",
    AgentsPanel: "Agents",
    WorkspacesPanel: "Workspaces",
    ReloadPage: "Reload page",
    FocusAddress: "Focus page address",
    CopyPageUrl: "Copy page URL",
    Quit: "Quit",
}

#[cfg(target_os = "macos")]
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
    ("ToggleReview", "cmd-alt-b"),
    ("FontSizeUp", "cmd-="),
    ("FontSizeUp", "cmd-+"),
    ("FontSizeDown", "cmd--"),
    ("FontSizeReset", "cmd-0"),
    ("OpenSettings", "cmd-,"),
    ("CommandPanel", "cmd-k"),
    ("CommandPanel", "cmd-shift-p"),
    ("AgentsPanel", "cmd-shift-a"),
    ("ReloadPage", "cmd-r"),
    ("FocusAddress", "cmd-l"),
    ("CopyPageUrl", "cmd-shift-c"),
    ("Quit", "cmd-q"),
];

/// Off macOS: Ghostty's Linux defaults where it has one. A terminal's Ctrl+letter belongs to the shell, so the app's
/// chords take Ctrl+Shift, and Super belongs to the desktop.
#[cfg(not(target_os = "macos"))]
pub const DEFAULTS: &[(&str, &str)] = &[
    ("NewTab", "ctrl-shift-t"),
    ("CloseTab", "ctrl-alt-w"),
    ("NewWorkspace", "ctrl-shift-n"),
    ("SplitRight", "ctrl-shift-o"),
    ("SplitDown", "ctrl-shift-e"),
    ("ClosePane", "ctrl-shift-w"),
    ("FocusLeft", "ctrl-alt-left"),
    ("FocusRight", "ctrl-alt-right"),
    ("FocusUp", "ctrl-alt-up"),
    ("FocusDown", "ctrl-alt-down"),
    ("ToggleZoom", "ctrl-shift-enter"),
    ("Tab1", "alt-1"),
    ("Tab2", "alt-2"),
    ("Tab3", "alt-3"),
    ("Tab4", "alt-4"),
    ("Tab5", "alt-5"),
    ("Tab6", "alt-6"),
    ("Tab7", "alt-7"),
    ("Tab8", "alt-8"),
    ("Tab9", "alt-9"),
    ("PreviousTab", "ctrl-pageup"),
    ("NextTab", "ctrl-pagedown"),
    ("PreviousWorkspace", "ctrl-shift-pageup"),
    ("NextWorkspace", "ctrl-shift-pagedown"),
    ("ToggleSidebar", "ctrl-shift-b"),
    ("ToggleReview", "ctrl-alt-b"),
    ("FontSizeUp", "ctrl-="),
    ("FontSizeUp", "ctrl-+"),
    ("FontSizeDown", "ctrl--"),
    ("FontSizeReset", "ctrl-0"),
    ("OpenSettings", "ctrl-,"),
    ("CommandPanel", "ctrl-shift-p"),
    ("AgentsPanel", "ctrl-shift-a"),
    ("Quit", "ctrl-shift-q"),
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
    rebind_herdr(cx, &next);
    cx.set_global(ActiveBindings(next));
    // macOS copies menu shortcuts when the menu is set: set it again so it shows (and only fires) the new ones.
    set_menus(cx);
}

/// Sets the menu bar again, with Secure Keyboard Entry's check as the settings have it.
pub fn set_menus(cx: &mut App) {
    let secure = cx.try_global::<crate::settings::SettingsGlobal>().is_some_and(|s| s.settings.secure_input);
    cx.set_menus(menus(secure));
}

// Window-management items macOS apps carry; not rebindable, so outside `app_actions!`.
gpui::actions!(shepherd, [Hide, HideOthers, ShowAll, Minimize, Zoom, ToggleFullScreen]);

fn menus(secure_input: bool) -> Vec<gpui::Menu> {
    use gpui::{Menu, MenuItem as I, SystemMenuType};
    vec![
        Menu::new("Shepherd").items([
            I::action("Settings…", OpenSettings),
            I::action("Secure Keyboard Entry", ToggleSecureInput).checked(secure_input),
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
            I::action("Toggle Pull Request Panel", ToggleReview),
            I::action("Zoom Pane", ToggleZoom),
            I::separator(),
            I::action("Command Panel", CommandPanel),
            I::action("Agents", AgentsPanel),
            I::action("Workspaces", WorkspacesPanel),
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
    bindings.extend(next.iter().filter_map(|(action, chord)| binding(action, chord, None)));
    cx.bind_keys(bindings);
}

/// The chords of herdr's own keymap bound now, so the next rebind can unbind what it drops.
struct HerdrBindings(Vec<String>);

impl gpui::Global for HerdrBindings {}

/// The key context herdr's chords are bound in: herdr's keymap is for its panes, and a prefix waiting for its
/// second key would get in the way of typing elsewhere.
const TERMINAL: &str = "Terminal";

/// Binds herdr's keymap from its config.toml (`[keys]`, `[[keys.command]]`) in the panes, under Shepherd's
/// own: a chord `shepherd` uses stays Shepherd's.
fn rebind_herdr(cx: &mut App, shepherd: &[(String, String)]) {
    use crate::herdr::keys::Bind;
    // ponytail: read on rebind (start, Settings), not watched; a config.toml edit takes a restart
    let config = crate::herdr::HerdrConfig::load(&crate::herdr::config_path()).unwrap_or_default();
    let next: Vec<_> = crate::herdr::keys::bindings(&config)
        .into_iter()
        .filter(|(_, chord)| !shepherd.iter().any(|(_, k)| k == chord))
        .collect();
    let previous = cx.try_global::<HerdrBindings>().map(|b| b.0.clone()).unwrap_or_default();
    let mut bindings: Vec<KeyBinding> = previous
        .iter()
        .filter(|chord| !next.iter().any(|(_, k)| k == *chord))
        .map(|chord| KeyBinding::new(chord, NoAction, Some(TERMINAL)))
        .collect();
    bindings.extend(next.iter().filter_map(|(bind, chord)| match bind {
        Bind::App(action) => binding(action, chord, Some(TERMINAL)),
        Bind::Command(command) => Some(KeyBinding::new(chord, command.clone(), Some(TERMINAL))),
    }));
    cx.bind_keys(bindings);
    cx.set_global(HerdrBindings(next.into_iter().map(|(_, chord)| chord).collect()));
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
    #[cfg(target_os = "macos")]
    fn defaults_when_the_user_set_nothing() {
        let eff = effective(&user(&[]));
        assert_eq!(chord(&eff, "SplitRight"), Some("cmd-d"));
        assert_eq!(chord(&eff, "NewTab"), Some("cmd-t"));
        assert_eq!(chord(&eff, "OpenSettings"), Some("cmd-,"));
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn defaults_when_the_user_set_nothing() {
        let eff = effective(&user(&[]));
        assert_eq!(chord(&eff, "SplitRight"), Some("ctrl-shift-o"));
        assert_eq!(chord(&eff, "NewTab"), Some("ctrl-shift-t"));
        assert_eq!(chord(&eff, "OpenSettings"), Some("ctrl-,"));
    }

    #[test]
    fn a_user_chord_replaces_the_default() {
        let eff = effective(&user(&[("SplitRight", "cmd-shift-right")]));
        assert_eq!(chord(&eff, "SplitRight"), normalize("cmd-shift-right").as_deref());
    }

    #[test]
    fn rebinding_takes_the_chord_from_the_old_action() {
        let split = DEFAULTS.iter().find(|(a, _)| *a == "SplitRight").unwrap().1;
        let eff = effective(&user(&[("NewTab", split)]));
        assert_eq!(chord(&eff, "NewTab"), normalize(split).as_deref());
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
        // the default for FocusLeft (cmd-alt-left, or ctrl-alt-left) with its modifiers the other way round
        let default = DEFAULTS.iter().find(|(a, _)| *a == "FocusLeft").unwrap().1;
        let mut parts: Vec<&str> = default.split('-').collect();
        let key = parts.pop().unwrap();
        parts.reverse();
        let swapped = format!("{}-{key}", parts.join("-"));
        assert_ne!(swapped, default);
        let eff = effective(&user(&[("NewTab", swapped.as_str())]));
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
        for (action, chord) in DEFAULTS {
            assert!(ACTIONS.iter().any(|(name, _)| name == action), "{action}");
            assert!(normalize(chord).is_some(), "{chord}");
        }
    }
}
