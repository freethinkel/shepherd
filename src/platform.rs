//! What differs between macOS and Linux: where files live, what opens a path, and which modifiers the terminal's
//! own shortcuts take. On macOS those are Cmd chords; on Linux Ctrl+Shift, as Linux terminals do, since Ctrl alone
//! belongs to the shell and Super to the window manager.
use gpui::Modifiers;
use std::path::PathBuf;

fn home() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
}

/// Where settings are kept: `~/Library/Application Support/Shepherd`, `$XDG_CONFIG_HOME/shepherd` on Linux.
pub fn config_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        return home().join("Library/Application Support/Shepherd");
    }
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(".config")).join("shepherd")
}

/// Where caches are kept: `~/Library/Caches/Shepherd`, `$XDG_CACHE_HOME/shepherd` on Linux.
pub fn cache_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        return home().join("Library/Caches/Shepherd");
    }
    std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(".cache")).join("shepherd")
}

/// The command that hands a path or URL to its default app: `open`, `xdg-open` on Linux.
pub const OPEN: &str = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };

/// The terminal's own shortcut for `key` (copy, paste, find): Cmd+key on macOS, Ctrl+Shift+key on Linux.
/// Linux also takes Ctrl+Insert to copy and Shift+Insert to paste, which Omarchy's Super+C and Super+V send.
pub fn terminal_chord(key: &str, mods: &Modifiers) -> Option<&'static str> {
    if cfg!(target_os = "macos") {
        return mods.platform.then_some(match key {
            "c" => "c",
            "v" => "v",
            "f" => "f",
            "g" => "g",
            _ => return None,
        });
    }
    match key {
        "insert" if mods.control && !mods.shift && !mods.alt => Some("c"),
        "insert" if mods.shift && !mods.control && !mods.alt => Some("v"),
        "c" | "v" | "f" | "g" if mods.control && mods.shift && !mods.alt && !mods.platform => {
            Some(match key {
                "c" => "c",
                "v" => "v",
                "f" => "f",
                _ => "g",
            })
        }
        _ => None,
    }
}

/// The modifier that turns a link under the pointer into a click target: Cmd on macOS, Ctrl on Linux.
pub fn link_modifier(mods: &Modifiers) -> bool {
    if cfg!(target_os = "macos") { mods.platform } else { mods.control }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(control: bool, shift: bool, platform: bool) -> Modifiers {
        Modifiers { control, shift, platform, ..Default::default() }
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn linux_terminal_chords() {
        assert_eq!(terminal_chord("c", &m(true, true, false)), Some("c"));
        assert_eq!(terminal_chord("c", &m(true, false, false)), None, "Ctrl+C is the shell's");
        assert_eq!(terminal_chord("insert", &m(true, false, false)), Some("c"));
        assert_eq!(terminal_chord("insert", &m(false, true, false)), Some("v"));
        assert_eq!(terminal_chord("v", &m(false, false, true)), None, "Super is the window manager's");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn mac_terminal_chords() {
        assert_eq!(terminal_chord("c", &m(false, false, true)), Some("c"));
        assert_eq!(terminal_chord("c", &m(true, true, false)), None);
    }
}
