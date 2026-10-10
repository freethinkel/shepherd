//! Window and terminal colors. Chrome tokens from Collie's theme (adapted from Zeron, MIT); the terminal palette
//! from Zeron's built-in `zeron_dark` / `zeron_light` (crates/theme/src/builtins.rs). See assets/NOTICE.
use gpui::{App, Global, Hsla, SharedString, WindowAppearance, hsla, rgb};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
}

#[derive(Debug, Clone)]
pub struct TerminalPalette {
    pub background: Hsla,
    pub foreground: Hsla,
    pub selection: Hsla,
    pub ansi: [Hsla; 16],
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub appearance: Appearance,
    pub surface: Hsla,
    pub card: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_faint: Hsla,
    pub accent: Hsla,
    pub warning: Hsla,
    pub success: Hsla,
    pub danger: Hsla,
    pub hover: Hsla,
    pub selected: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,
    pub cursor: Hsla,
    pub font_ui: SharedString,
    pub font_terminal: SharedString,
    pub terminal_font_size: f32,
    pub terminal: TerminalPalette,
}

impl Global for Theme {}

pub(crate) fn hex(c: u32) -> Hsla {
    rgb(c).into()
}

const ANSI_DARK: [u32; 16] = [
    0x242424, 0xf87171, 0x4ade80, 0xfacc15, 0x60a5fa, 0xc084fc, 0x22d3ee, 0xd4d4d8,
    0x52525b, 0xfca5a5, 0x86efac, 0xfde047, 0x93c5fd, 0xd8b4fe, 0x67e8f9, 0xfafafa,
];
const ANSI_LIGHT: [u32; 16] = [
    0x1f1f1f, 0xdc2626, 0x16a34a, 0xb45309, 0x2563eb, 0x9333ea, 0x0e7490, 0x3f3f46,
    0x71717a, 0xb91c1c, 0x15803d, 0x92400e, 0x1d4ed8, 0x7e22ce, 0x155e75, 0x18181b,
];

impl Theme {
    pub fn for_appearance(appearance: WindowAppearance) -> Theme {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Theme::dark(),
            WindowAppearance::Light | WindowAppearance::VibrantLight => Theme::light(),
        }
    }

    pub fn of(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    fn dark() -> Theme {
        let text = hex(0xe8e8ea);
        let accent = hex(0x8b7cf6);
        Theme {
            appearance: Appearance::Dark,
            surface: hex(0x0d0d0d),
            card: hex(0x0e0e0e),
            text,
            text_muted: hex(0xa9a9ae),
            text_faint: hex(0x85858a),
            accent,
            warning: hex(0xfacc15),
            success: hex(0x34d399),
            danger: hex(0xf87171),
            hover: hsla(0., 0., 1., 0.07),
            selected: hsla(0., 0., 0.92, 0.11),
            border: hsla(0., 0., 1., 0.10),
            border_strong: hsla(0., 0., 1., 0.18),
            cursor: text,
            font_ui: "Geist".into(),
            font_terminal: "Geist Mono".into(),
            terminal_font_size: 13.0,
            terminal: TerminalPalette {
                background: hex(0x090909),
                foreground: text,
                selection: accent.opacity(0.30),
                ansi: ANSI_DARK.map(hex),
            },
        }
    }

    fn light() -> Theme {
        let text = hex(0x303035);
        let accent = hex(0x5b43e8);
        Theme {
            appearance: Appearance::Light,
            surface: hex(0xf3f3f5),
            card: hex(0xffffff),
            text,
            text_muted: hex(0x62626a),
            text_faint: hex(0x797981),
            accent,
            warning: hex(0xa16207),
            success: hex(0x15803d),
            danger: hex(0xdc2626),
            hover: hsla(0., 0., 0., 0.04),
            selected: hsla(0., 0., 0.10, 0.06),
            border: hsla(0., 0., 0., 0.12),
            border_strong: hsla(0., 0., 0., 0.22),
            cursor: text,
            font_ui: "Geist".into(),
            font_terminal: "Geist Mono".into(),
            terminal_font_size: 13.0,
            terminal: TerminalPalette {
                background: hex(0xfafafa),
                foreground: text,
                selection: accent.opacity(0.22),
                ansi: ANSI_LIGHT.map(hex),
            },
        }
    }

    /// A tint that lifts a block off the page: light on dark, dark on light (Collie's `wash`).
    pub fn wash(&self, a: f32) -> Hsla {
        match self.appearance {
            Appearance::Dark => hsla(0., 0., 0.92, a),
            Appearance::Light => hsla(0., 0., 0.10, a),
        }
    }

    /// The window's own fill, translucent over the blurred desktop (Collie's `glass`).
    pub fn glass(&self) -> Hsla {
        self.surface.opacity(0.80)
    }
}

/// sRGB components 0..1 → HSL (gpui convention, all 0..1). From Zeron's crates/ui/src/theme.rs.
pub fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let delta = max - min;
    if delta < f32::EPSILON {
        return (0.0, 0.0, l);
    }
    let s = if l > 0.5 { delta / (2.0 - max - min) } else { delta / (max + min) };
    let h = if (max - r).abs() < f32::EPSILON {
        ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() < f32::EPSILON {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    } / 6.0;
    (h, s, l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_terminal_matches_zeron_dark() {
        let t = Theme::for_appearance(gpui::WindowAppearance::Dark);
        assert_eq!(t.terminal.background, hex(0x090909));
        assert_eq!(t.terminal.ansi[1], hex(0xf87171));
        assert_eq!(t.terminal.ansi[15], hex(0xfafafa));
    }

    #[test]
    fn light_terminal_matches_zeron_light() {
        let t = Theme::for_appearance(gpui::WindowAppearance::Light);
        assert_eq!(t.terminal.background, hex(0xfafafa));
        assert_eq!(t.terminal.ansi[1], hex(0xdc2626));
        assert_eq!(t.appearance, Appearance::Light);
    }
}
