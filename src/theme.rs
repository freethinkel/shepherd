//! Window and terminal colors from Zeron's built-in theme families (the `zeron-theme` crate, MIT, see
//! assets/NOTICE), laid out as Collie's tokens, with the terminal font from settings.
use gpui::{App, Global, Hsla, SharedString, WindowAppearance, hsla, rgb};
use serde::{Deserialize, Serialize};
use zeron_theme::{Color, ThemeRegistry, ThemeVariant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
}

/// Light, dark, or whatever the system shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppearanceMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontSettings {
    pub family: String,
    pub size: f32,
}

impl Default for FontSettings {
    fn default() -> Self {
        FontSettings { family: "Geist Mono".into(), size: 13.0 }
    }
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

#[allow(dead_code)]
pub(crate) fn hex(c: u32) -> Hsla {
    rgb(c).into()
}

fn color(c: Color) -> Hsla {
    let channel = |v: u8| f32::from(v) / 255.0;
    gpui::Rgba { r: channel(c.r), g: channel(c.g), b: channel(c.b), a: channel(c.a) }.into()
}

/// Built-in theme families as (id, name), in Zeron's order.
pub fn families() -> Vec<(String, String)> {
    ThemeRegistry::builtin().families.iter().map(|f| (f.id.clone(), f.name.clone())).collect()
}

impl Theme {
    /// Zeron's own theme for this appearance, with the default font.
    pub fn for_appearance(appearance: WindowAppearance) -> Theme {
        Theme::resolve("zeron", AppearanceMode::System, appearance, &FontSettings::default())
    }

    /// The variant of `family_id` for the mode (or the system's appearance); a family without that appearance
    /// uses its other variant, and an unknown family is Zeron's.
    pub fn resolve(family_id: &str, mode: AppearanceMode, system: WindowAppearance, font: &FontSettings) -> Theme {
        let dark = match mode {
            AppearanceMode::Dark => true,
            AppearanceMode::Light => false,
            AppearanceMode::System => matches!(system, WindowAppearance::Dark | WindowAppearance::VibrantDark),
        };
        let registry = ThemeRegistry::builtin();
        let family = registry
            .families
            .iter()
            .find(|f| f.id == family_id)
            .or_else(|| registry.families.iter().find(|f| f.id == "zeron"))
            .expect("Zeron's built-in family");
        let variant = family
            .variants
            .iter()
            .find(|v| v.appearance.is_dark() == dark)
            .or(family.variants.first())
            .expect("a family has variants");
        Theme::from_zeron(variant, font)
    }

    pub fn from_zeron(v: &ThemeVariant, font: &FontSettings) -> Theme {
        let c = &v.colors;
        let appearance = if v.appearance.is_dark() { Appearance::Dark } else { Appearance::Light };
        let text = color(c.text);
        Theme {
            appearance,
            surface: color(c.shell),
            card: color(c.card),
            text,
            text_muted: color(c.text_muted),
            text_faint: color(c.text_faint),
            accent: color(v.accent.primary),
            warning: color(c.warning),
            success: color(c.success),
            danger: color(c.danger),
            hover: color(c.hover),
            selected: color(c.active),
            border: color(c.border),
            border_strong: color(c.border_strong),
            // Zeron: translucent so the glyph under the block stays legible.
            cursor: text.opacity(if appearance == Appearance::Dark { 0.40 } else { 0.55 }),
            font_ui: "Geist".into(),
            font_terminal: font.family.clone().into(),
            terminal_font_size: font.size,
            terminal: TerminalPalette {
                background: color(v.terminal.background),
                foreground: color(v.terminal.foreground),
                selection: color(v.terminal.selection),
                ansi: v.terminal.ansi.map(color),
            },
        }
    }

    pub fn of(cx: &App) -> &Theme {
        cx.global::<Theme>()
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

    fn font() -> FontSettings {
        FontSettings { family: "Geist Mono".into(), size: 13.0 }
    }

    #[test]
    fn zeron_family_resolves_to_zeron_colors() {
        let t = Theme::resolve("zeron", AppearanceMode::System, gpui::WindowAppearance::Dark, &font());
        assert_eq!(t.terminal.background, hex(0x090909));
        assert_eq!(t.terminal.ansi[1], hex(0xf87171));
        assert_eq!(t.surface, hex(0x0d0d0d));
        assert_eq!(t.appearance, Appearance::Dark);
    }

    #[test]
    fn every_builtin_family_resolves_in_both_appearances() {
        for (id, _) in families() {
            for system in [gpui::WindowAppearance::Dark, gpui::WindowAppearance::Light] {
                let t = Theme::resolve(&id, AppearanceMode::System, system, &font());
                assert_ne!(t.terminal.background, t.terminal.foreground, "{id}");
            }
        }
        assert!(families().len() >= 10);
    }

    #[test]
    fn an_explicit_mode_beats_the_system() {
        let t = Theme::resolve("zeron", AppearanceMode::Light, gpui::WindowAppearance::Dark, &font());
        assert_eq!(t.appearance, Appearance::Light);
        assert_eq!(t.terminal.background, hex(0xfafafa));
    }

    #[test]
    fn unknown_family_falls_back_to_zeron() {
        let t = Theme::resolve("no-such-theme", AppearanceMode::Dark, gpui::WindowAppearance::Dark, &font());
        assert_eq!(t.terminal.background, hex(0x090909));
    }

    #[test]
    fn font_settings_reach_the_terminal() {
        let t = Theme::resolve("zeron", AppearanceMode::Dark, gpui::WindowAppearance::Dark, &FontSettings { family: "Menlo".into(), size: 16.0 });
        assert_eq!(t.font_terminal.as_ref(), "Menlo");
        assert_eq!(t.terminal_font_size, 16.0);
    }

    #[test]
    fn dark_terminal_matches_zeron_dark() {
        let t = Theme::for_appearance(gpui::WindowAppearance::Dark);
        assert_eq!(t.terminal.background, hex(0x090909));
        assert_eq!(t.terminal.ansi[1], hex(0xf87171));
        assert_eq!(t.terminal.ansi[15], hex(0xfafafa));
    }

    #[test]
    fn block_cursor_lets_the_glyph_under_it_show() {
        // Painted over the glyphs: opaque would hide the character in vim's normal mode.
        assert_eq!(Theme::for_appearance(gpui::WindowAppearance::Dark).cursor.a, 0.40);
        assert_eq!(Theme::for_appearance(gpui::WindowAppearance::Light).cursor.a, 0.55);
    }

    #[test]
    fn light_terminal_matches_zeron_light() {
        let t = Theme::for_appearance(gpui::WindowAppearance::Light);
        assert_eq!(t.terminal.background, hex(0xfafafa));
        assert_eq!(t.terminal.ansi[1], hex(0xdc2626));
        assert_eq!(t.appearance, Appearance::Light);
    }
}
