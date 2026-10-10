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
