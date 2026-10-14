//! Zeron's project icon (`crates/ui/src/shell/project_icon.rs`): the repo's own artwork when it has one, else a
//! monogram on a tone picked by the repo's path.
use crate::theme::Theme;
use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, StyledImage, div, px};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Where repos keep their icon, best first.
const ICON_PATHS: &[&str] = &[
    "public/apple-touch-icon.png",
    "apple-touch-icon.png",
    "public/favicon.svg",
    "favicon.svg",
    "public/favicon.png",
    "public/icon.png",
    "public/logo.png",
    "favicon.png",
    "app/icon.png",
    "src/app/icon.png",
    "public/favicon.ico",
    "favicon.ico",
    "app/favicon.ico",
    "static/favicon.ico",
    "src-tauri/icons/icon.png",
    "assets/icon.png",
    "src/assets/icon.png",
];

/// The first of [`ICON_PATHS`] in the checkout holding `dir` (a linked worktree's `.git` is a file), else the
/// avatar of the GitHub account its `origin` belongs to: GitHub has no icon per repo. `dir` is located: a machine's
/// checkout is looked in there.
fn find_icon(dir: &str) -> Option<PathBuf> {
    if let (Some(target), path) = crate::herdr::split(dir) {
        return remote_icon(dir, target, path);
    }
    let dir = Path::new(dir).canonicalize().ok()?;
    let root = dir.ancestors().find(|p| p.join(".git").exists()).unwrap_or(&dir);
    ICON_PATHS.iter().map(|p| root.join(p)).find(|p| p.is_file()).or_else(|| github_avatar(&root.to_string_lossy()))
}

/// The icon of the checkout holding `path` on the machine at `target`: found there in one round trip and copied into
/// Shepherd's caches once, else the GitHub avatar of its `origin`'s owner.
fn remote_icon(dir: &str, target: &str, path: &str) -> Option<PathBuf> {
    let look = format!(
        "cd {} 2>/dev/null && cd \"$(git rev-parse --show-toplevel)\" && for p in {}; do [ -f \"$p\" ] && echo \"$PWD/$p\" && exit 0; done",
        crate::herdr::quote(path),
        ICON_PATHS.join(" ")
    );
    let found = String::from_utf8_lossy(&crate::herdr::ssh(target, &look).ok()?.stdout).trim().to_string();
    if found.is_empty() {
        return github_avatar(dir);
    }
    let ext = Path::new(&found).extension().and_then(|e| e.to_str()).unwrap_or("png");
    let name = format!("{:08x}.{ext}", fnv(&format!("{target}:{found}")));
    let cache = PathBuf::from(std::env::var_os("HOME")?).join("Library/Caches/Shepherd/remote-icons");
    let file = cache.join(name);
    if !file.is_file() {
        let out = crate::herdr::ssh(target, &format!("cat {}", crate::herdr::quote(&found))).ok()?;
        if !out.status.success() || out.stdout.is_empty() {
            return None;
        }
        std::fs::create_dir_all(&cache).ok()?;
        std::fs::write(&file, &out.stdout).ok()?;
    }
    Some(file)
}

/// FNV-1a of `s`, for a cache file's name.
fn fnv(s: &str) -> u32 {
    s.bytes().fold(2166136261u32, |h, b| (h ^ u32::from(b)).wrapping_mul(16777619))
}

/// The owner in a GitHub remote URL: `git@github.com:owner/repo.git`, `https://github.com/owner/repo`,
/// `ssh://git@github.com/owner/repo.git`.
fn github_owner(url: &str) -> Option<&str> {
    let rest = url.trim().split_once("github.com")?.1;
    let owner = rest.trim_start_matches([':', '/']).split('/').next()?;
    (!owner.is_empty() && owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')).then_some(owner)
}

// ponytail: downloaded once and kept; delete ~/Library/Caches/Shepherd/avatars to pick up a changed avatar.
/// The GitHub avatar of the owner of the located `dir`'s origin, downloaded once into Shepherd's caches.
fn github_avatar(dir: &str) -> Option<PathBuf> {
    let out = crate::herdr::git(dir, &["remote", "get-url", "origin"]).ok()?;
    let owner = github_owner(&String::from_utf8_lossy(&out.stdout))?.to_string();
    let dir = PathBuf::from(std::env::var_os("HOME")?).join("Library/Caches/Shepherd/avatars");
    let path = dir.join(format!("{owner}.png"));
    if !path.is_file() {
        std::fs::create_dir_all(&dir).ok()?;
        let url = format!("https://github.com/{owner}.png?size=64");
        let got = std::process::Command::new("curl").args(["-fsSL", "--max-time", "10", "-o"]).arg(&path).arg(url).status().ok()?;
        if !got.success() {
            let _ = std::fs::remove_file(&path);
            return None;
        }
    }
    Some(path)
}

// Zeron's curated badge tones, (dark appearance, light appearance); the order is kept so projects keep their color.
const MONOGRAM_PALETTE: [(u32, u32); 8] = [
    (0x94a3b8, 0x475569), // slate
    (0x93c5fd, 0x2563eb), // blue
    (0xc4b5fd, 0x7c3aed), // violet
    (0xfda4af, 0xbe123c), // rose
    (0xfcd34d, 0xa16207), // amber
    (0x6ee7b7, 0x047857), // emerald
    (0x5eead4, 0x0f766e), // teal
    (0xfdba74, 0xc2410c), // orange
];

/// `seed`'s entry in [`MONOGRAM_PALETTE`], by FNV-1a.
fn tone_index(seed: &str) -> usize {
    fnv(seed) as usize % MONOGRAM_PALETTE.len()
}

/// `name`'s first letter on its tone, washed brighter on the selected row.
fn monogram(name: &str, seed: &str, selected: bool, theme: &Theme) -> gpui::Div {
    let dark = theme.appearance == crate::theme::Appearance::Dark;
    let (on_dark, on_light) = MONOGRAM_PALETTE[tone_index(seed)];
    let tone = gpui::Hsla::from(gpui::rgb(if dark { on_dark } else { on_light }));
    let mut lit = tone;
    lit.l = if dark { (tone.l + 0.12).min(0.95) } else { (tone.l - 0.10).max(0.15) };
    let letter = name.trim().chars().next().unwrap_or('?').to_uppercase().to_string();
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(3.))
        .bg(tone.opacity(if selected { 0.24 } else { 0.08 }))
        .text_color(if selected { lit } else { tone.opacity(0.85) })
        .font_family("Geist Mono")
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_size(px(9.))
        .line_height(px(13.))
        .child(letter)
}

/// The icon of the project at `dir`, `size` px square: its artwork, looked for once off the main thread, else the
/// monogram of `name` on `seed`'s tone (and while the artwork loads or if it does not decode).
pub fn project_icon(name: &str, seed: &str, dir: Option<&str>, size: f32, selected: bool, theme: &Theme, cx: &mut App) -> AnyElement {
    // a machine's lookup can fail on a connection still coming up: asked again now and then, the last find kept
    let ttl = |d: &str| if crate::herdr::split(d).0.is_some() { Duration::from_secs(300) } else { Duration::MAX };
    let art = dir.and_then(|d| super::statusbar::cached(d, ttl(d), find_icon, cx));
    let frame = div().size(px(size)).flex_none();
    let mark = {
        let (name, seed, theme) = (name.to_string(), seed.to_string(), theme.clone());
        move || monogram(&name, &seed, selected, &theme).into_any_element()
    };
    match art {
        Some(path) => frame
            .child(gpui::img(path).size_full().rounded(px(3.)).object_fit(gpui::ObjectFit::Contain).with_fallback(mark.clone()).with_loading(mark))
            .into_any_element(),
        None => frame.child(mark()).into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_by_priority_from_the_checkout_root() {
        let dir = std::env::temp_dir().join(format!("shepherd-icon-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src/nested")).unwrap();
        std::fs::create_dir_all(dir.join("public")).unwrap();
        std::fs::write(dir.join(".git"), "gitdir: /unused").unwrap();
        let root = dir.canonicalize().unwrap();
        assert_eq!(find_icon(dir.to_str().unwrap()), None);
        std::fs::write(dir.join("favicon.ico"), b"").unwrap();
        assert_eq!(find_icon(dir.join("src/nested").to_str().unwrap()), Some(root.join("favicon.ico")));
        std::fs::write(dir.join("public/apple-touch-icon.png"), b"").unwrap();
        assert_eq!(find_icon(dir.to_str().unwrap()), Some(root.join("public/apple-touch-icon.png")));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn github_owners_from_remote_urls() {
        assert_eq!(github_owner("git@github.com:piashyos/collie.git\n"), Some("piashyos"));
        assert_eq!(github_owner("https://github.com/freethinkel/shepherd.git"), Some("freethinkel"));
        assert_eq!(github_owner("ssh://git@github.com/zeronsh/zeron.git"), Some("zeronsh"));
        assert_eq!(github_owner("ssh://git@ci.example.net:10022/callcenter/mono.git"), None);
    }

    #[test]
    fn a_seed_keeps_its_tone() {
        assert_eq!(tone_index("/repo/a"), tone_index("/repo/a"));
        assert!((0..20).map(|i| tone_index(&format!("/repo/{i}"))).collect::<std::collections::HashSet<_>>().len() > 1);
    }
}
