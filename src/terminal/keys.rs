//! Keystroke and paste encoding, from Zeron's crates/ui/src/terminal/view.rs (MIT, see assets/NOTICE).
use gpui::Modifiers;

/// The terminal's own shortcuts (copy, paste, find): Cmd on macOS. Elsewhere Ctrl+Shift, as GNOME Terminal, kitty
/// and Ghostty have it, since a terminal's Ctrl+letter belongs to the shell.
pub fn app_shortcut(mods: &Modifiers) -> bool {
    if cfg!(target_os = "macos") { mods.platform } else { mods.control && mods.shift && !mods.alt && !mods.platform }
}

/// Cmd+Shift+G steps the search back on macOS; elsewhere Shift is part of the shortcut, so it only goes forward.
pub fn search_back(mods: &Modifiers) -> bool {
    cfg!(target_os = "macos") && mods.shift
}

/// What turns a click into opening a link: Cmd on macOS, Ctrl elsewhere.
pub fn link_modifier(mods: &Modifiers) -> bool {
    if cfg!(target_os = "macos") { mods.platform } else { mods.control }
}

/// Encode a keystroke as PTY bytes. `None` means "not ours" — the event should
/// fall through (e.g. the platform-primary shortcuts that drive app actions).
///
/// `app_cursor` switches arrows/home/end from CSI to SS3 per DECCKM.
pub fn keystroke_bytes(
    key: &str,
    key_char: Option<&str>,
    mods: &Modifiers,
    app_cursor: bool,
) -> Option<Vec<u8>> {
    // Platform-primary combos (Cmd on macOS, the super key elsewhere) belong to
    // the app keymap, never the PTY.
    if mods.platform {
        // Except Cmd+Backspace: delete to the line's start (Ctrl+U), and Cmd+Left/Right: to the line's start or end
        // (Ctrl+A, Ctrl+E), as Terminal, iTerm2 and Ghostty do.
        return match key {
            "backspace" => Some(vec![0x15]),
            "left" => Some(vec![0x01]),
            "right" => Some(vec![0x05]),
            _ => None,
        };
    }
    if mods.alt {
        // Option+Left/Right: a word back or forward (ESC b, ESC f), which shells and readline know, as Ghostty sends
        match key {
            "left" if !mods.shift && !mods.control => return Some(b"\x1bb".to_vec()),
            "right" if !mods.shift && !mods.control => return Some(b"\x1bf".to_vec()),
            _ => {}
        }
        // macOS composes Option chords (Option+B → "∫"); send what was composed, as Ghostty does by default.
        if let Some(text) = key_char.filter(|c| !c.is_empty() && *c != key) {
            return Some(text.as_bytes().to_vec());
        }
        // Nothing composed (arrows, or a layout where Option adds nothing): Meta, ESC-prefixed.
        let inner = keystroke_bytes(
            key,
            None,
            &Modifiers {
                alt: false,
                ..*mods
            },
            app_cursor,
        )?;
        let mut out = vec![0x1b];
        out.extend(inner);
        return Some(out);
    }
    if mods.control {
        return control_bytes(key);
    }

    let seq = |csi: &[u8], ss3: &[u8]| {
        Some(if app_cursor {
            ss3.to_vec()
        } else {
            csi.to_vec()
        })
    };
    match key {
        "enter" => Some(b"\r".to_vec()),
        "backspace" => Some(vec![0x7f]),
        "tab" => Some(if mods.shift {
            b"\x1b[Z".to_vec()
        } else {
            b"\t".to_vec()
        }),
        "escape" => Some(vec![0x1b]),
        "space" => Some(b" ".to_vec()),
        "up" => seq(b"\x1b[A", b"\x1bOA"),
        "down" => seq(b"\x1b[B", b"\x1bOB"),
        "right" => seq(b"\x1b[C", b"\x1bOC"),
        "left" => seq(b"\x1b[D", b"\x1bOD"),
        "home" => seq(b"\x1b[H", b"\x1bOH"),
        "end" => seq(b"\x1b[F", b"\x1bOF"),
        "insert" => Some(b"\x1b[2~".to_vec()),
        "delete" => Some(b"\x1b[3~".to_vec()),
        "pageup" => Some(b"\x1b[5~".to_vec()),
        "pagedown" => Some(b"\x1b[6~".to_vec()),
        "f1" => Some(b"\x1bOP".to_vec()),
        "f2" => Some(b"\x1bOQ".to_vec()),
        "f3" => Some(b"\x1bOR".to_vec()),
        "f4" => Some(b"\x1bOS".to_vec()),
        "f5" => Some(b"\x1b[15~".to_vec()),
        "f6" => Some(b"\x1b[17~".to_vec()),
        "f7" => Some(b"\x1b[18~".to_vec()),
        "f8" => Some(b"\x1b[19~".to_vec()),
        "f9" => Some(b"\x1b[20~".to_vec()),
        "f10" => Some(b"\x1b[21~".to_vec()),
        "f11" => Some(b"\x1b[23~".to_vec()),
        "f12" => Some(b"\x1b[24~".to_vec()),
        _ => {
            // Printable: prefer the typed character (IME/shift-aware).
            let text = key_char.filter(|c| !c.is_empty()).or({
                // Fall back to single-char key names ("a", "/", …).
                if key.chars().count() == 1 {
                    Some(key)
                } else {
                    None
                }
            })?;
            Some(text.as_bytes().to_vec())
        }
    }
}

/// Ctrl-key encoding (caret notation).
fn control_bytes(key: &str) -> Option<Vec<u8>> {
    let mut chars = key.chars();
    let (c, rest) = (chars.next()?, chars.next());
    if rest.is_some() {
        return match key {
            "space" => Some(vec![0x00]),
            "backspace" => Some(vec![0x08]),
            "enter" => Some(b"\r".to_vec()),
            _ => None,
        };
    }
    match c {
        'a'..='z' => Some(vec![c as u8 - b'a' + 1]),
        '@' => Some(vec![0x00]),
        '[' => Some(vec![0x1b]),
        '\\' => Some(vec![0x1c]),
        ']' => Some(vec![0x1d]),
        '^' => Some(vec![0x1e]),
        '_' | '/' => Some(vec![0x1f]),
        '?' => Some(vec![0x7f]),
        _ => None,
    }
}

/// Wrap pasted text for the PTY (bracketed-paste aware; strips the one control
/// sequence a paste could inject).
pub fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    let sanitized = text.replace("\x1b[201~", "");
    if bracketed {
        let mut out = b"\x1b[200~".to_vec();
        out.extend_from_slice(sanitized.as_bytes());
        out.extend_from_slice(b"\x1b[201~");
        out
    } else {
        sanitized.into_bytes()
    }
}

/// Files dropped on a terminal, as text: each path shell-escaped with backslashes and space-separated, as Ghostty
/// does, so a shell or an agent TUI (Claude Code picks up image paths) reads them whole.
pub fn dropped_paths<P: AsRef<std::path::Path>>(paths: &[P]) -> String {
    let escape = |p: &P| {
        p.as_ref().to_string_lossy().chars().fold(String::new(), |mut out, c| {
            if c.is_whitespace() || "\\'\"`$&|;<>()[]{}*?!#~".contains(c) {
                out.push('\\');
            }
            out.push(c);
            out
        })
    };
    paths.iter().map(escape).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropped_paths_are_escaped_and_joined() {
        assert_eq!(dropped_paths(&["/tmp/a.png", "/Users/me/My Shot (1).png"]), r"/tmp/a.png /Users/me/My\ Shot\ \(1\).png");
    }

    fn mods() -> Modifiers {
        Modifiers::default()
    }

    #[test]
    fn printables_prefer_key_char() {
        assert_eq!(
            keystroke_bytes("a", Some("a"), &mods(), false),
            Some(b"a".to_vec())
        );
        assert_eq!(
            keystroke_bytes(
                "a",
                Some("A"),
                &Modifiers {
                    shift: true,
                    ..mods()
                },
                false
            ),
            Some(b"A".to_vec())
        );
        // Multi-byte characters pass through as UTF-8.
        assert_eq!(
            keystroke_bytes("e", Some("é"), &mods(), false),
            Some("é".as_bytes().to_vec())
        );
        // Named single-char keys fall back to the key name.
        assert_eq!(
            keystroke_bytes("/", None, &mods(), false),
            Some(b"/".to_vec())
        );
        // Unknown multi-char keys are not ours.
        assert_eq!(keystroke_bytes("capslock", None, &mods(), false), None);
    }

    #[test]
    fn control_keys_and_sequences() {
        assert_eq!(
            keystroke_bytes("enter", None, &mods(), false),
            Some(b"\r".to_vec())
        );
        assert_eq!(
            keystroke_bytes("backspace", None, &mods(), false),
            Some(vec![0x7f])
        );
        assert_eq!(
            keystroke_bytes("tab", None, &mods(), false),
            Some(b"\t".to_vec())
        );
        assert_eq!(
            keystroke_bytes(
                "tab",
                None,
                &Modifiers {
                    shift: true,
                    ..mods()
                },
                false
            ),
            Some(b"\x1b[Z".to_vec())
        );
        assert_eq!(
            keystroke_bytes("escape", None, &mods(), false),
            Some(vec![0x1b])
        );
        assert_eq!(
            keystroke_bytes("delete", None, &mods(), false),
            Some(b"\x1b[3~".to_vec())
        );
        assert_eq!(
            keystroke_bytes("pageup", None, &mods(), false),
            Some(b"\x1b[5~".to_vec())
        );
        assert_eq!(
            keystroke_bytes("f5", None, &mods(), false),
            Some(b"\x1b[15~".to_vec())
        );
    }

    #[test]
    fn arrows_respect_app_cursor_mode() {
        assert_eq!(
            keystroke_bytes("up", None, &mods(), false),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            keystroke_bytes("up", None, &mods(), true),
            Some(b"\x1bOA".to_vec())
        );
        assert_eq!(
            keystroke_bytes("home", None, &mods(), false),
            Some(b"\x1b[H".to_vec())
        );
        assert_eq!(
            keystroke_bytes("end", None, &mods(), true),
            Some(b"\x1bOF".to_vec())
        );
    }

    #[test]
    fn ctrl_combos_map_to_control_bytes() {
        let ctrl = Modifiers {
            control: true,
            ..mods()
        };
        assert_eq!(
            keystroke_bytes("c", Some("c"), &ctrl, false),
            Some(vec![0x03])
        );
        assert_eq!(keystroke_bytes("z", None, &ctrl, false), Some(vec![0x1a]));
        assert_eq!(
            keystroke_bytes("space", None, &ctrl, false),
            Some(vec![0x00])
        );
        assert_eq!(keystroke_bytes("[", None, &ctrl, false), Some(vec![0x1b]));
        assert_eq!(keystroke_bytes("_", None, &ctrl, false), Some(vec![0x1f]));
        // Ctrl+1 has no caret encoding — not ours.
        assert_eq!(keystroke_bytes("1", Some("1"), &ctrl, false), None);
    }

    #[test]
    fn alt_prefixes_escape() {
        let alt = Modifiers {
            alt: true,
            ..mods()
        };
        assert_eq!(
            keystroke_bytes("b", Some("b"), &alt, false),
            Some(vec![0x1b, b'b'])
        );
        let alt_ctrl = Modifiers {
            alt: true,
            control: true,
            ..mods()
        };
        assert_eq!(
            keystroke_bytes("c", None, &alt_ctrl, false),
            Some(vec![0x1b, 0x03])
        );
    }

    #[test]
    fn platform_primary_combos_fall_through() {
        let cmd = Modifiers {
            platform: true,
            ..mods()
        };
        assert_eq!(keystroke_bytes("j", Some("j"), &cmd, false), None);
        assert_eq!(keystroke_bytes("enter", None, &cmd, false), None);
        assert_eq!(keystroke_bytes("backspace", None, &cmd, false), Some(vec![0x15]));
    }

    #[test]
    fn paste_wraps_when_bracketed() {
        assert_eq!(paste_bytes("hi", false), b"hi".to_vec());
        assert_eq!(paste_bytes("hi", true), b"\x1b[200~hi\x1b[201~".to_vec());
        // Close-bracket injection is stripped.
        assert_eq!(
            paste_bytes("a\x1b[201~rm -rf", true),
            b"\x1b[200~arm -rf\x1b[201~".to_vec()
        );
    }

    #[test]
    fn option_sends_the_character_macos_composed() {
        // GPUI hands Option chords their composed key_char (Option+B → "∫" on a US layout); like Ghostty's
        // default (macos-option-as-alt = false), that character is what the user typed.
        let alt = Modifiers { alt: true, ..mods() };
        assert_eq!(keystroke_bytes("b", Some("∫"), &alt, false), Some("∫".as_bytes().to_vec()));
        assert_eq!(keystroke_bytes("l", Some("@"), &alt, false), Some(b"@".to_vec()));
    }

    #[test]
    fn option_without_a_character_is_meta() {
        let alt = Modifiers { alt: true, ..mods() };
        assert_eq!(keystroke_bytes("up", None, &alt, false), Some(b"\x1b\x1b[A".to_vec()));
        assert_eq!(keystroke_bytes("left", None, &alt, false), Some(b"\x1bb".to_vec()), "a word back");
        assert_eq!(keystroke_bytes("right", None, &alt, false), Some(b"\x1bf".to_vec()), "a word forward");
        let cmd = Modifiers { platform: true, ..mods() };
        assert_eq!(keystroke_bytes("left", None, &cmd, false), Some(vec![0x01]), "the line's start");
        assert_eq!(keystroke_bytes("right", None, &cmd, false), Some(vec![0x05]), "the line's end");
    }

    #[test]
    fn cyrillic_key_char_is_utf8() {
        assert_eq!(keystroke_bytes("ж", Some("ж"), &mods(), false), Some("ж".as_bytes().to_vec()));
    }
}
