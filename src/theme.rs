//! Window and terminal colors from Zeron's built-in theme families (the `zeron-theme` crate, MIT, see
//! assets/NOTICE) and Ghostty's themes, laid out as Collie's tokens, with the terminal font from settings.
use gpui::{App, Global, Hsla, SharedString, WindowAppearance, hsla, rgb};
use serde::{Deserialize, Serialize};
use zeron_theme::{Color, ThemeRegistry, ThemeVariant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
}

/// Light, dark, or whatever the system shows.
/// How agent status is drawn (the settings' Status icons): herdr's two `status_indicators` styles, and Shepherd's
/// own, Zeron's: a pulsing glyph while an agent works, a check when done, a small dot otherwise.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StatusIcons {
    Symbols,
    Animated,
    /// Also what an unknown (future) value reads as.
    #[default]
    #[serde(other)]
    Dots,
}

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
    /// Terminal row height as a multiple of `size`.
    pub line_height: f32,
}

impl Default for FontSettings {
    fn default() -> Self {
        FontSettings { family: "Geist Mono".into(), size: 13.0, line_height: 1.4 }
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
    pub terminal_line_height: f32,
    /// How agent status is drawn (the settings' Status icons).
    pub status_icons: StatusIcons,
    /// The window lets the desktop through (the settings' Translucent window): its fill is `glass`.
    pub translucent: bool,
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

/// Every theme variant: the `zeron-theme` built-ins but Zeron's own family, then Ghostty's
/// (assets/themes/ghostty.conf) under names the built-ins do not have.
pub fn variants() -> &'static [ThemeVariant] {
    static VARIANTS: std::sync::OnceLock<Vec<ThemeVariant>> = std::sync::OnceLock::new();
    VARIANTS.get_or_init(|| {
        let mut all: Vec<ThemeVariant> = ThemeRegistry::builtin().families.iter().filter(|f| f.id != "zeron").flat_map(|f| f.variants.iter().cloned()).collect();
        for v in ghostty(include_str!("../assets/themes/ghostty.conf")) {
            if !all.iter().any(|b| b.name.eq_ignore_ascii_case(&v.name)) {
                all.push(v);
            }
        }
        all
    })
}

/// The variants of one appearance as (id, name): the built-ins first, then Ghostty's by name.
pub fn variants_for(appearance: Appearance) -> Vec<(String, String)> {
    let dark = appearance == Appearance::Dark;
    variants().iter().filter(|v| v.appearance.is_dark() == dark).map(|v| (v.id.clone(), v.name.clone())).collect()
}

/// The default variant for each appearance.
pub const DEFAULT_LIGHT: &str = "ayu-light";
pub const DEFAULT_DARK: &str = "ayu-dark";

/// Ghostty theme files, each after a `[Name]` line: `palette = N=#rrggbb`, `background`, `foreground`,
/// `selection-background`. A theme without a background or foreground is skipped.
fn ghostty(text: &str) -> Vec<ThemeVariant> {
    let mut out = Vec::new();
    let mut name: Option<&str> = None;
    let mut keys: Vec<(&str, &str)> = Vec::new();
    for line in text.lines().chain(std::iter::once("[]")) {
        let line = line.trim();
        if let Some(next) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if let Some(v) = name.and_then(|n| ghostty_variant(n, &keys)) {
                out.push(v);
            }
            (name, keys) = (Some(next), Vec::new());
        } else if let Some((key, value)) = line.split_once('=') {
            keys.push((key.trim(), value.trim()));
        }
    }
    out
}

/// One Ghostty theme as a Zeron variant: the terminal colors as they are, the window's derived from them the
/// way Zeron's built-ins derive theirs from a few seeds.
fn ghostty_variant(name: &str, keys: &[(&str, &str)]) -> Option<ThemeVariant> {
    use zeron_theme::{AccentRoles, SurfaceTreatment, TerminalPalette as Palette, ThemeColors, ThemeSource};
    let get = |k: &str| keys.iter().find(|(key, _)| *key == k).and_then(|(_, v)| v.parse::<Color>().ok());
    let (bg, fg) = (get("background")?, get("foreground")?);
    let mut ansi = [fg; 16];
    for (_, value) in keys.iter().filter(|(k, _)| *k == "palette") {
        if let Some((i, c)) = value.split_once('=')
            && let (Ok(i), Ok(c)) = (i.trim().parse::<usize>(), c.trim().parse::<Color>())
            && i < 16
        {
            ansi[i] = c
        }
    }
    let dark = bg.best_on_color() == Color::WHITE;
    let appearance = if dark { zeron_theme::Appearance::Dark } else { zeron_theme::Appearance::Light };
    let tone = if dark { Color::WHITE } else { Color::BLACK };
    let accent = AccentRoles::derive(ansi[4], appearance, bg);
    let raised = bg.mix(fg, 0.10);
    let card = bg.mix(fg, 0.04);
    let solid = if dark { Color::rgb(235, 235, 239) } else { Color::rgb(35, 35, 40) };
    let colors = ThemeColors {
        background: bg,
        shell: bg.mix(Color::BLACK, if dark { 0.18 } else { 0.04 }),
        raised,
        card,
        dialog: card.mix(raised, if dark { 0.18 } else { 0.04 }),
        overlay: card.mix(raised, if dark { 0.34 } else { 0.02 }),
        hover: tone.with_alpha(if dark { 0.11 } else { 0.06 }),
        active: accent.primary.with_alpha(if dark { 0.18 } else { 0.10 }),
        border: tone.with_alpha(if dark { 0.10 } else { 0.12 }),
        border_strong: tone.with_alpha(if dark { 0.18 } else { 0.22 }),
        text: fg,
        text_muted: fg.mix(bg, 0.30).ensure_contrast(bg, 4.5),
        text_faint: fg.mix(bg, 0.50),
        solid,
        on_solid: solid.best_on_color(),
        danger: ansi[1],
        danger_muted: ansi[1].mix(fg, 0.28),
        warning: ansi[3],
        warning_muted: ansi[3].mix(fg, 0.25),
        success: ansi[2],
        success_muted: ansi[2].mix(fg, 0.25),
        input: if dark { raised.with_alpha(0.72) } else { card },
        cursor: fg.with_alpha(if dark { 0.40 } else { 0.55 }),
        diff_add: ansi[2],
        diff_delete: ansi[1],
        diff_hunk: accent.primary.with_alpha(if dark { 0.08 } else { 0.07 }),
    };
    let slug: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    Some(ThemeVariant {
        id: format!("ghostty-{slug}"),
        family_id: "ghostty".into(),
        name: name.into(),
        appearance,
        recommended_surface_treatment: SurfaceTreatment::Opaque,
        colors,
        accent,
        syntax: Default::default(),
        terminal: Palette {
            background: bg,
            foreground: fg,
            selection: get("selection-background").unwrap_or(tone.with_alpha(if dark { 0.22 } else { 0.16 })),
            ansi,
        },
        source: ThemeSource {
            format: "ghostty".into(),
            url: "https://github.com/mbadolato/iTerm2-Color-Schemes".into(),
            revision: String::new(),
            license: "MIT".into(),
            asset_hash: String::new(),
        },
    })
}

impl Theme {
    /// The default theme for this appearance, with the default font.
    pub fn for_appearance(appearance: WindowAppearance) -> Theme {
        Theme::resolve(DEFAULT_LIGHT, DEFAULT_DARK, AppearanceMode::System, appearance, &FontSettings::default())
    }

    /// The light or the dark variant, whichever the mode (or the system's appearance) asks for; an unknown id is
    /// the default variant of that appearance.
    pub fn resolve(light: &str, dark: &str, mode: AppearanceMode, system: WindowAppearance, font: &FontSettings) -> Theme {
        let is_dark = match mode {
            AppearanceMode::Dark => true,
            AppearanceMode::Light => false,
            AppearanceMode::System => matches!(system, WindowAppearance::Dark | WindowAppearance::VibrantDark),
        };
        let (id, fallback) = if is_dark { (dark, DEFAULT_DARK) } else { (light, DEFAULT_LIGHT) };
        Theme::variant(id, font).unwrap_or_else(|| Theme::variant(fallback, font).expect("the default variants are built in"))
    }

    /// The variant `id`, if there is one.
    pub fn variant(id: &str, font: &FontSettings) -> Option<Theme> {
        variants().iter().find(|v| v.id == id).map(|v| Theme::from_zeron(v, font))
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
            // Zeron's `glass_selected_bg`: a neutral wash, so the selected row reads on any theme's accent
            selected: if v.appearance.is_dark() { hsla(0., 0., 0.92, 0.11) } else { hsla(0., 0., 0.10, 0.06) },
            border: color(c.border),
            border_strong: color(c.border_strong),
            // The theme's accent, opaque like Ghostty's: the cell under the block takes the background color.
            cursor: color(v.accent.primary),
            font_ui: "Geist".into(),
            font_terminal: font.family.clone().into(),
            terminal_font_size: font.size,
            terminal_line_height: font.line_height,
            status_icons: StatusIcons::Dots,
            translucent: cfg!(target_os = "macos"),
            terminal: TerminalPalette {
                background: color(v.terminal.background),
                foreground: color(v.terminal.foreground),
                // the accent, washed: a selection reads as the theme's, not as a grey film
                selection: color(v.accent.primary).opacity(if v.appearance.is_dark() { 0.30 } else { 0.20 }),
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

    /// The window's own fill, translucent over the blurred desktop (Collie's `glass`) when the window is
    /// translucent; else opaque, as a desktop that does not blur (GNOME) would show the windows behind as they are.
    pub fn glass(&self) -> Hsla {
        if self.translucent { self.surface.opacity(0.80) } else { self.surface }
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
        FontSettings::default()
    }

    fn resolve(mode: AppearanceMode, system: gpui::WindowAppearance) -> Theme {
        Theme::resolve(DEFAULT_LIGHT, DEFAULT_DARK, mode, system, &font())
    }

    #[test]
    fn default_resolves_to_ayu_colors() {
        let t = resolve(AppearanceMode::System, gpui::WindowAppearance::Dark);
        assert_eq!(t.terminal.background, hex(0x0d1017));
        assert_eq!(t.terminal.ansi[1], hex(0xf06b73));
        assert_eq!(t.surface, hex(0x0d1017));
        assert_eq!(t.appearance, Appearance::Dark);
    }

    #[test]
    fn every_variant_resolves_with_its_own_appearance() {
        for appearance in [Appearance::Light, Appearance::Dark] {
            let list = variants_for(appearance);
            assert!(list.len() >= 50, "{appearance:?}: {}", list.len());
            for (id, _) in list {
                let t = Theme::variant(&id, &font()).unwrap();
                assert_eq!(t.appearance, appearance, "{id}");
                assert_ne!(t.terminal.background, t.terminal.foreground, "{id}");
            }
        }
    }

    #[test]
    fn ghostty_themes_load_with_their_palette() {
        let t = Theme::variant("ghostty-catppuccin-frappe", &font()).unwrap();
        assert_eq!(t.terminal.background, hex(0x303446));
        assert_eq!(t.terminal.ansi[1], hex(0xe78284));
        // The built-in Catppuccin wins over Ghostty's of the same name.
        assert_eq!(variants().iter().filter(|v| v.name == "Catppuccin Mocha").count(), 1);
    }

    #[test]
    fn an_explicit_mode_beats_the_system() {
        let t = resolve(AppearanceMode::Light, gpui::WindowAppearance::Dark);
        assert_eq!(t.appearance, Appearance::Light);
        assert_eq!(t.terminal.background, hex(0xf8f9fa));
    }

    #[test]
    fn unknown_variant_falls_back_to_the_default() {
        // a saved Zeron theme is unknown now too
        let t = Theme::resolve("zeron-light", "zeron-dark", AppearanceMode::Dark, gpui::WindowAppearance::Dark, &font());
        assert_eq!(t.terminal.background, hex(0x0d1017));
    }

    #[test]
    fn font_settings_reach_the_terminal() {
        let t = Theme::resolve(DEFAULT_LIGHT, DEFAULT_DARK, AppearanceMode::Dark, gpui::WindowAppearance::Dark, &FontSettings { family: "Menlo".into(), size: 16.0, line_height: 1.2 });
        assert_eq!(t.font_terminal.as_ref(), "Menlo");
        assert_eq!(t.terminal_font_size, 16.0);
        assert_eq!(t.terminal_line_height, 1.2);
    }

    #[test]
    fn dark_terminal_matches_ayu_dark() {
        let t = Theme::for_appearance(gpui::WindowAppearance::Dark);
        assert_eq!(t.terminal.background, hex(0x0d1017));
        assert_eq!(t.terminal.ansi[1], hex(0xf06b73));
        assert_eq!(t.terminal.ansi[15], hex(0xffffff));
    }

    #[test]
    fn cursor_is_the_opaque_accent() {
        let t = Theme::for_appearance(gpui::WindowAppearance::Dark);
        assert_eq!(t.cursor, t.accent);
    }

    #[test]
    fn light_terminal_matches_ayu_light() {
        let t = Theme::for_appearance(gpui::WindowAppearance::Light);
        assert_eq!(t.terminal.background, hex(0xf8f9fa));
        assert_eq!(t.terminal.ansi[1], hex(0xf06b6c));
        assert_eq!(t.appearance, Appearance::Light);
    }
}
