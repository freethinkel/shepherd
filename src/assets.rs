//! Fonts compiled into the binary (licenses in assets/NOTICE). From Collie's assets.rs.
use gpui::App;
use std::borrow::Cow;

const FONTS: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/Geist.ttf"),
    include_bytes!("../assets/fonts/Geist-Medium.ttf"),
    include_bytes!("../assets/fonts/Geist-SemiBold.ttf"),
    include_bytes!("../assets/fonts/GeistMono.ttf"),
];

/// Adds Geist and Geist Mono; without them gpui falls back to the system font.
pub fn register_fonts(cx: &App) {
    if let Err(e) = cx.text_system().add_fonts(FONTS.iter().map(|f| Cow::Borrowed(*f)).collect()) {
        eprintln!("shepherd: cannot load the bundled fonts: {e:#}");
    }
}

/// Zeron's sidebar, settings and tab icons (agent marks included), served at `icons/<name>.svg`.
pub struct Assets;

macro_rules! icons {
    ($name:expr, $($icon:literal),+ $(,)?) => {
        match $name {
            $($icon => Some(Cow::Borrowed(include_bytes!(concat!("../assets/icons/", $icon, ".svg")).as_slice())),)+
            _ => None,
        }
    };
}

impl gpui::AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let Some(name) = path.strip_prefix("icons/").and_then(|p| p.strip_suffix(".svg")) else { return Ok(None) };
        if let Some(icon) = name.strip_prefix("solar/") {
            return Ok(solar_icon(icon).map(|svg| Cow::Owned(svg.into_bytes())));
        }
        Ok(icons!(
            name,
            "add-circle",
            "alt-arrow-down",
            "alt-arrow-left",
            "alt-arrow-right",
            "alt-arrow-up",
            "antigravity-mark",
            "arrow-down",
            "arrow-left",
            "arrow-right",
            "arrow-up",
            "bell",
            "bot",
            "bot-filled",
            "check",
            "checklist",
            "claude-mark",
            "close",
            "close-circle",
            "clock-circle",
            "cursor-mark",
            "devin-mark",
            "drag-handle",
            "expand-arrows",
            "folder",
            "git-branch",
            "git-pull-request",
            "grok-mark",
            "hermes-mark",
            "info-circle",
            "keyboard",
            "logout-2",
            "monitor",
            "moon",
            "more-horizontal",
            "openai-mark",
            "opencode-mark",
            "palette-search",
            "pi-mark",
            "plus",
            "prompt",
            "remote-server",
            "settings",
            "sidebar-minimalistic-left",
            "spinner",
            "split-columns",
            "split-rows",
            "sun",
            "terminal",
            "trash",
            "tuning",
            "widget",
        ))
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        Ok(Vec::new())
    }
}

/// The Solar icon set's Linear style (iconify's `solar` collection, `-linear` icons), by name: each icon's SVG body
/// on a 24 px canvas. The sessions' icons; licence in assets/NOTICE.
pub fn solar_names() -> impl Iterator<Item = &'static str> {
    solar().keys().map(String::as_str)
}

/// The Solar Linear icon `name` as a whole SVG, served at `icons/solar/<name>.svg`.
pub fn solar_icon(name: &str) -> Option<String> {
    solar().get(name).map(|body| format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">{body}</svg>"#))
}

// the bodies hold escaped quotes, so they are owned rather than borrowed from the JSON
fn solar() -> &'static std::collections::BTreeMap<String, String> {
    static SOLAR: std::sync::OnceLock<std::collections::BTreeMap<String, String>> = std::sync::OnceLock::new();
    SOLAR.get_or_init(|| serde_json::from_str(include_str!("../assets/solar-linear.json")).expect("assets/solar-linear.json"))
}
