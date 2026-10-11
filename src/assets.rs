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

/// Zeron's sidebar and settings icons, served at `icons/<name>.svg`.
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
        Ok(icons!(
            name,
            "arrow-left",
            "alt-arrow-right",
            "bot",
            "check",
            "checklist",
            "folder",
            "git-branch",
            "keyboard",
            "more-horizontal",
            "remote-server",
            "settings",
            "sidebar-minimalistic-left",
            "spinner",
            "terminal",
            "tuning",
        ))
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        Ok(Vec::new())
    }
}
