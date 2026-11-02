//! Zeron's `file_icons.rs` (MIT): file identity icons from Symbols (assets/file-icons, licensed beside them). Resolution
//! preserves the VS Code theme's semantics: exact basenames win, then the longest matching compound extension, then
//! the generic file. The authored SVG colors are preserved in light mode; dark mode lifts the palette's darker accents
//! without replacing polychrome artwork with a tint.
use crate::theme::Appearance;
use gpui::{Img, SharedString, Styled as _, img};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::{borrow::Cow, collections::HashMap, sync::LazyLock};

const ASSET_PREFIX: &str = "file-icons/";

fn dark_icon_svg(svg: &str) -> String {
    // Lift shared palette accents, preserving bright colors, transparency, gradients, and the contrasting black/white
    // details within brand marks.
    let mut svg = svg.to_owned();
    for (original, brighter) in [
        ("#64748B", "#CBD5E1"),
        ("#71717A", "#D4D4D8"),
        ("#2563EB", "#60A5FA"),
        ("#EA580C", "#FB923C"),
        ("#16A34A", "#4ADE80"),
        ("#8B5CF6", "#A78BFA"),
        ("#A855F7", "#C084FC"),
    ] {
        svg = svg.replace(original, brighter).replace(&original.to_ascii_lowercase(), brighter);
    }
    svg
}

#[derive(RustEmbed)]
#[folder = "assets/file-icons"]
struct EmbeddedFileIcons;

/// `path`'s bytes when it's a file icon, for the app's asset source.
pub fn load(path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
    let Some(path) = path.strip_prefix(ASSET_PREFIX) else { return Ok(None) };
    if let Some(path) = path.strip_prefix("dark/") {
        return EmbeddedFileIcons::get(path)
            .map(|asset| {
                let svg = std::str::from_utf8(&asset.data)?;
                Ok(Cow::Owned(dark_icon_svg(svg).into_bytes()))
            })
            .transpose();
    }
    Ok(EmbeddedFileIcons::get(path).map(|asset| asset.data))
}

#[derive(Debug, Deserialize)]
struct Manifest {
    #[serde(rename = "iconDefinitions")]
    icon_definitions: HashMap<String, IconDefinition>,
    file: Option<String>,
    #[serde(default, rename = "fileExtensions")]
    file_extensions: HashMap<String, String>,
    #[serde(default, rename = "fileNames")]
    file_names: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct IconDefinition {
    #[serde(rename = "iconPath")]
    icon_path: String,
}

static MANIFEST: LazyLock<Manifest> = LazyLock::new(|| {
    let mut manifest: Manifest = serde_json::from_str(include_str!("../../assets/file-icons.json")).expect("bundled file-icon manifest must be valid");
    manifest.file_extensions = lowercase_keys(manifest.file_extensions);
    manifest.file_names = lowercase_keys(manifest.file_names);
    manifest
});

fn lowercase_keys(values: HashMap<String, String>) -> HashMap<String, String> {
    values.into_iter().map(|(key, value)| (key.to_ascii_lowercase(), value)).collect()
}

/// The asset of `path`'s icon, as registered with gpui.
pub fn asset_path(path: &str, appearance: Appearance) -> SharedString {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path).to_ascii_lowercase();
    let variant = if appearance == Appearance::Dark { "dark/" } else { "" };
    SharedString::from(format!("{ASSET_PREFIX}{variant}{}", resolve_file(&name)))
}

pub fn icon(path: &str, appearance: Appearance) -> Img {
    img(asset_path(path, appearance)).flex_none()
}

fn definition_asset(definition: &str) -> Option<&'static str> {
    // These two identifiers are referenced by the upstream manifest but do not have icon definitions. Its language
    // associations reveal the intended aliases, so resolve them rather than falling back to a generic document.
    let definition = match definition {
        "less" => "brackets-sky",
        "yml" => "yaml",
        definition => definition,
    };
    MANIFEST.icon_definitions.get(definition)?.icon_path.strip_prefix("./icons/")
}

fn resolve_file(name: &str) -> &'static str {
    if let Some(asset) = MANIFEST.file_names.get(name).and_then(|definition| definition_asset(definition)) {
        return asset;
    }
    for extension in name.match_indices('.').map(|(index, _)| &name[index + 1..]).filter(|extension| !extension.is_empty()) {
        if let Some(asset) = MANIFEST.file_extensions.get(extension).and_then(|definition| definition_asset(definition)) {
            return asset;
        }
    }
    MANIFEST.file.as_deref().and_then(definition_asset).unwrap_or("files/document.svg")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_resolved_icon_is_embedded() {
        for name in ["main.rs", "cargo.toml", "readme.md", "app.test.tsx", "dockerfile", "no-such-kind.xyz"] {
            let asset = asset_path(name, Appearance::Light);
            assert!(load(&asset).unwrap().is_some(), "{asset}");
            assert!(load(&asset_path(name, Appearance::Dark)).unwrap().is_some());
        }
    }
}
