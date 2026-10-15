//! Updating Shepherd.app from its GitHub releases (Collie's updater): the latest release from GitHub's API, its
//! `Shepherd-<V>-universal.zip` downloaded and unpacked next to the running app, checked to be signed by the
//! running app's team, swapped in, then a relaunch. Checked at launch and every few hours; the sidebar offers
//! what it finds (Zeron's update strip), Settings → About checks by hand. On Linux, where there is no signature to
//! check a download against, it only tells: the offer opens the release's page, and the build or the package
//! manager installs it.
use anyhow::{Context as _, bail};
use futures::StreamExt;
use gpui::{App, AppContext as _, Global};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const LATEST: &str = "https://api.github.com/repos/freethinkel/shepherd/releases/latest";
const EVERY: Duration = Duration::from_secs(6 * 3600);
/// The release zips' arch, as `uname -m` names it in the Makefile.
const ARCH: &str = if cfg!(target_arch = "aarch64") { "arm64" } else { "x86_64" };

/// A release's version and its zip for this Mac.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub size: u64,
}

/// Where the app is with its updates.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Updates {
    #[default]
    Unknown,
    Checking,
    UpToDate,
    Available(Release),
    Downloading { version: String, percent: u64 },
    Installing,
    /// Why the check or the update failed; with the release when it was the update, to try again.
    Failed(String, Option<Release>),
}

impl Global for Updates {}

impl Updates {
    pub fn of(cx: &App) -> &Updates {
        cx.global::<Updates>()
    }

    /// The line under "Shepherd <version>" in Settings → About.
    pub fn line(&self) -> String {
        match self {
            Updates::Unknown => if cfg!(target_os = "macos") { "A herdr client for macOS" } else { "A herdr client for Linux" }.into(),
            Updates::Checking => "Checking for updates…".into(),
            Updates::UpToDate => "Up to date".into(),
            Updates::Available(r) => format!("Shepherd {} is available", r.version),
            Updates::Downloading { percent, .. } => format!("Downloading · {percent}%"),
            Updates::Installing => "Installing, Shepherd restarts in a moment…".into(),
            Updates::Failed(why, _) => format!("Update failed: {why}"),
        }
    }

    pub fn busy(&self) -> bool {
        matches!(self, Updates::Checking | Updates::Downloading { .. } | Updates::Installing)
    }

    /// The sidebar strip: its label and the release a click installs. `None` while there is nothing to offer.
    pub fn strip(&self) -> Option<(String, Option<Release>)> {
        match self {
            Updates::Available(r) if INSTALLS => Some((format!("Update to v{}", r.version), Some(r.clone()))),
            Updates::Available(r) => Some((format!("Shepherd v{} is out", r.version), Some(r.clone()))),
            Updates::Downloading { version, percent } => Some((format!("Downloading v{version}… {percent}%"), None)),
            Updates::Installing => Some(("Installing, Shepherd restarts…".into(), None)),
            Updates::Failed(why, Some(r)) => Some((format!("Update failed: {why}"), Some(r.clone()))),
            _ => None,
        }
    }
}

/// The `.app` this release build runs from; none for debug builds (`cargo run` runs from a bundle too, which
/// must not be swapped for a release).
pub fn app_bundle() -> Option<PathBuf> {
    if cfg!(debug_assertions) {
        return None;
    }
    bundle_of(&std::env::current_exe().ok()?)
}

/// `…/Shepherd.app/Contents/MacOS/shepherd` → `…/Shepherd.app`.
fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent().filter(|d| d.ends_with("Contents/MacOS"))?;
    let app = macos.parent()?.parent()?;
    (app.extension()? == "app").then(|| app.to_path_buf())
}

/// Whether this build installs updates itself: a macOS app bundle. Elsewhere an offer opens the release's page.
pub const INSTALLS: bool = cfg!(target_os = "macos");

/// Whether this build looks for updates: a release build in an app bundle on macOS, any release build elsewhere.
pub fn checks() -> bool {
    if INSTALLS { app_bundle().is_some() } else { !cfg!(debug_assertions) }
}

/// Sets up the state and, where [`checks`], checks now and every few hours.
pub fn start(cx: &mut App) {
    cx.set_global(Updates::default());
    if !checks() {
        return;
    }
    cx.spawn(async move |cx| {
        loop {
            let _ = cx.update(|cx| check(cx, true));
            cx.background_executor().timer(EVERY).await;
        }
    })
    .detach();
}

/// Asks GitHub for the latest release. A `quiet` check (the timer's) keeps what was shown when it fails and
/// leaves an offered update alone.
pub fn check(cx: &mut App, quiet: bool) {
    let before = Updates::of(cx).clone();
    if before.busy() || quiet && matches!(before, Updates::Available(_) | Updates::Failed(_, Some(_))) {
        return;
    }
    set(cx, Updates::Checking);
    cx.spawn(async move |cx| {
        let found = cx.background_spawn(async { latest() }).await;
        let _ = cx.update(|cx| {
            set(
                cx,
                match found {
                    Ok(r) if is_newer(&r.version, env!("CARGO_PKG_VERSION")) => Updates::Available(r),
                    Ok(_) => Updates::UpToDate,
                    Err(_) if quiet => before,
                    Err(e) => Updates::Failed(format!("{e:#}"), None),
                },
            )
        });
    })
    .detach();
}

/// Downloads `release`, swaps the app for it and relaunches; where Shepherd does not install, opens its page.
pub fn install(release: Release, cx: &mut App) {
    if !INSTALLS {
        if let Ok(mut child) = Command::new(crate::platform::OPEN).arg(&release.url).spawn() {
            std::thread::spawn(move || child.wait());
        }
        return;
    }
    let Some(app) = app_bundle() else { return };
    if Updates::of(cx).busy() {
        return;
    }
    set(cx, Updates::Downloading { version: release.version.clone(), percent: 0 });
    // The percent comes from the download's thread; the stream ends when the download drops its sender.
    let (tx, mut rx) = futures::channel::mpsc::unbounded::<u64>();
    let version = release.version.clone();
    let progress = cx.spawn(async move |cx| {
        while let Some(percent) = rx.next().await {
            let _ = cx.update(|cx| set(cx, Updates::Downloading { version: version.clone(), percent }));
        }
    });
    cx.spawn(async move |cx| {
        let r = release.clone();
        let zip = cx
            .background_spawn(async move { download(&r, &std::env::temp_dir().join("shepherd-update"), |p| drop(tx.unbounded_send(p))) })
            .await;
        progress.await;
        let done = match zip {
            Ok(zip) => {
                let _ = cx.update(|cx| set(cx, Updates::Installing));
                let target = app.clone();
                cx.background_spawn(async move { replace(&zip, &target) }).await.and_then(|()| relaunch(&app))
            }
            Err(e) => Err(e),
        };
        let _ = cx.update(|cx| match done {
            Ok(()) => cx.quit(),
            Err(e) => set(cx, Updates::Failed(format!("{e:#}"), Some(release))),
        });
    })
    .detach();
}

fn set(cx: &mut App, updates: Updates) {
    cx.set_global(updates);
    cx.refresh_windows();
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .build()
        .into()
}

/// The latest release and its zip for this Mac.
fn latest() -> anyhow::Result<Release> {
    let body = agent()
        .get(LATEST)
        .header("Accept", "application/vnd.github+json")
        .call()
        .context("cannot reach GitHub")?
        .body_mut()
        .read_to_string()?;
    let json: serde_json::Value = serde_json::from_str(&body)?;
    if !INSTALLS {
        return release_page(&json).context("the latest release has no version tag");
    }
    release_of(&json).with_context(|| format!("the latest release has no universal or {ARCH} Shepherd zip"))
}

/// A release from GitHub's JSON as an offer to read about: `v0.2.0` and its page.
fn release_page(json: &serde_json::Value) -> Option<Release> {
    let version = json["tag_name"].as_str()?.strip_prefix('v')?.to_string();
    Some(Release { url: json["html_url"].as_str()?.into(), size: 0, version })
}

/// A release from GitHub's JSON: `v0.2.0` with `Shepherd-0.2.0-universal.zip` (or, from before universal builds,
/// `Shepherd-0.2.0-arm64.zip`) among its assets.
fn release_of(json: &serde_json::Value) -> Option<Release> {
    let version = json["tag_name"].as_str()?.strip_prefix('v')?.to_string();
    let assets = json["assets"].as_array()?;
    let asset = ["universal", ARCH].iter().find_map(|kind| {
        let name = format!("Shepherd-{version}-{kind}.zip");
        assets.iter().find(|a| a["name"] == name.as_str())
    })?;
    Some(Release { url: asset["browser_download_url"].as_str()?.into(), size: asset["size"].as_u64()?, version })
}

/// `latest` is a later `x.y.z` than `current`; anything unreadable is not newer.
fn is_newer(latest: &str, current: &str) -> bool {
    let parts = |v: &str| v.split('.').map(|p| p.parse::<u64>().ok()).collect::<Option<Vec<_>>>();
    matches!((parts(latest), parts(current)), (Some(l), Some(c)) if l > c)
}

/// The release's zip into `dir`, with its percent as it comes.
fn download(release: &Release, dir: &Path, mut progress: impl FnMut(u64)) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let zip = dir.join(format!("Shepherd-{}.zip", release.version));
    let response = agent().get(&release.url).call().with_context(|| format!("cannot download {}", release.url))?;
    let mut body = response.into_body().into_with_config().limit(release.size + 1).reader();
    let mut file = std::fs::File::create(&zip).with_context(|| zip.display().to_string())?;
    let (mut buf, mut done, mut shown) = (vec![0; 1 << 16], 0u64, 0);
    loop {
        let n = body.read(&mut buf).context("the download broke off")?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        done += n as u64;
        let percent = done * 100 / release.size.max(1);
        if percent != shown {
            shown = percent;
            progress(percent);
        }
    }
    if done != release.size {
        bail!("the download stopped at {done} of {} bytes", release.size);
    }
    Ok(zip)
}

/// Unpacks `zip` beside `app` and puts its `.app` in its place: the old one is renamed aside first and comes back
/// if the new one cannot move in. The running process keeps working from the renamed bundle.
fn replace(zip: &Path, app: &Path) -> anyhow::Result<()> {
    let stage = app.with_extension("update");
    let _ = std::fs::remove_dir_all(&stage);
    let out = Command::new("ditto").args(["-x", "-k"]).arg(zip).arg(&stage).output().context("spawn ditto")?;
    if !out.status.success() {
        bail!("cannot unpack {}: {}", zip.display(), String::from_utf8_lossy(&out.stderr).trim());
    }
    let result = (|| {
        let unpacked = std::fs::read_dir(&stage)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x == "app") && p.join("Contents/MacOS").is_dir())
            .with_context(|| format!("{} holds no app", zip.display()))?;
        verify(&unpacked, app)?;
        let old = app.with_extension("old.app");
        let _ = std::fs::remove_dir_all(&old);
        std::fs::rename(app, &old).with_context(|| format!("move {} aside", app.display()))?;
        if let Err(e) = std::fs::rename(&unpacked, app) {
            let _ = std::fs::rename(&old, app);
            return Err(e).with_context(|| format!("move the new app to {}", app.display()));
        }
        let _ = std::fs::remove_dir_all(&old);
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&stage);
    let _ = std::fs::remove_file(zip);
    result
}

/// The new app's signature holds and, when the running app is signed by a team, it is that team's.
fn verify(new: &Path, old: &Path) -> anyhow::Result<()> {
    let out = Command::new("codesign").args(["--verify", "--deep", "--strict"]).arg(new).output().context("spawn codesign")?;
    if !out.status.success() {
        bail!("the new app's signature is broken: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    // ponytail: an ad hoc signed app (released without the signing secrets) takes any intact signature.
    if let Some(want) = team(old)
        && team(new).as_deref() != Some(want.as_str())
    {
        bail!("the new app is not signed by team {want}");
    }
    Ok(())
}

/// The signing team of `app`; none when ad hoc signed.
fn team(app: &Path) -> Option<String> {
    let out = Command::new("codesign").arg("-dv").arg(app).output().ok()?;
    let info = String::from_utf8_lossy(&out.stderr);
    info.lines().find_map(|l| l.strip_prefix("TeamIdentifier=")).filter(|t| *t != "not set").map(str::to_string)
}

/// Opens `app` once this process has quit; the caller quits right after.
fn relaunch(app: &Path) -> anyhow::Result<()> {
    Command::new("sh")
        .args(["-c", r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done; exec open "$2""#, "_"])
        .arg(std::process::id().to_string())
        .arg(app)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("spawn the relauncher")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_release_for_this_mac() {
        let json = serde_json::json!({
            "tag_name": "v0.2.0",
            "assets": [
                { "name": "Shepherd-0.2.0-other.zip", "size": 1, "browser_download_url": "https://x/other" },
                { "name": format!("Shepherd-0.2.0-{ARCH}.zip"), "size": 7, "browser_download_url": "https://x/zip" },
            ],
        });
        assert_eq!(release_of(&json), Some(Release { version: "0.2.0".into(), url: "https://x/zip".into(), size: 7 }));
        assert_eq!(release_of(&serde_json::json!({ "tag_name": "v0.2.0", "assets": [] })), None);
        // A universal zip runs on every Mac and wins over an arch's.
        let json = serde_json::json!({
            "tag_name": "v0.3.0",
            "assets": [
                { "name": format!("Shepherd-0.3.0-{ARCH}.zip"), "size": 7, "browser_download_url": "https://x/arch" },
                { "name": "Shepherd-0.3.0-universal.zip", "size": 9, "browser_download_url": "https://x/universal" },
                { "name": "Shepherd-0.3.0-universal.dmg", "size": 9, "browser_download_url": "https://x/dmg" },
            ],
        });
        assert_eq!(release_of(&json), Some(Release { version: "0.3.0".into(), url: "https://x/universal".into(), size: 9 }));
    }

    #[test]
    fn newer_versions() {
        assert!(is_newer("0.2.0", "0.1.1"));
        assert!(is_newer("0.1.10", "0.1.9"));
        assert!(!is_newer("0.1.1", "0.1.1"));
        assert!(!is_newer("0.1.0", "0.1.1"));
        assert!(!is_newer("0.2.0-beta", "0.1.1"));
    }

    #[test]
    fn a_release_page_needs_no_assets() {
        let json = serde_json::json!({ "tag_name": "v1.3.0", "html_url": "https://github.com/o/r/releases/tag/v1.3.0", "assets": [] });
        assert_eq!(release_page(&json), Some(Release { version: "1.3.0".into(), url: "https://github.com/o/r/releases/tag/v1.3.0".into(), size: 0 }));
        assert_eq!(release_page(&serde_json::json!({ "tag_name": "nightly", "html_url": "x" })), None);
    }

    #[test]
    fn the_app_bundle_of_the_executable() {
        assert_eq!(bundle_of(Path::new("/Applications/Shepherd.app/Contents/MacOS/shepherd")), Some("/Applications/Shepherd.app".into()));
        assert_eq!(bundle_of(Path::new("/r/target/debug/shepherd")), None);
    }

    /// An ad hoc signed bundle whose `Contents/Resources/which` says which one it is.
    #[cfg(target_os = "macos")]
    fn fake_app(app: &Path, which: &str) {
        std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        std::fs::create_dir_all(app.join("Contents/Resources")).unwrap();
        std::fs::copy("/usr/bin/true", app.join("Contents/MacOS/shepherd")).unwrap();
        std::fs::write(app.join("Contents/Resources/which"), which).unwrap();
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>CFBundleExecutable</key><string>shepherd</string><key>CFBundleIdentifier</key><string>test.shepherd</string></dict></plist>"#;
        std::fs::write(app.join("Contents/Info.plist"), plist).unwrap();
        assert!(Command::new("codesign").args(["-s", "-", "--force"]).arg(app).status().unwrap().success());
    }

    #[test]
    #[cfg(target_os = "macos")] // ditto and codesign
    fn the_new_app_replaces_the_old_one() {
        let tmp = tempfile::tempdir().unwrap();
        let (apps, build) = (tmp.path().join("Applications"), tmp.path().join("build"));
        let app = apps.join("Shepherd.app");
        fake_app(&app, "old");
        fake_app(&build.join("Shepherd.app"), "new");
        let zip = tmp.path().join("Shepherd-0.2.0.zip");
        let zipped = Command::new("ditto").args(["-c", "-k", "--keepParent"]).arg(build.join("Shepherd.app")).arg(&zip).status().unwrap();
        assert!(zipped.success());
        replace(&zip, &app).unwrap();
        assert_eq!(std::fs::read_to_string(app.join("Contents/Resources/which")).unwrap(), "new");
        assert_eq!(std::fs::read_dir(&apps).unwrap().count(), 1, "no stage or old app left behind");

        // A tampered app leaves the installed one alone.
        std::fs::write(build.join("Shepherd.app/Contents/Resources/which"), "evil").unwrap();
        let zipped = Command::new("ditto").args(["-c", "-k", "--keepParent"]).arg(build.join("Shepherd.app")).arg(&zip).status().unwrap();
        assert!(zipped.success());
        let e = replace(&zip, &app).unwrap_err().to_string();
        assert!(e.contains("signature"), "{e}");
        assert_eq!(std::fs::read_to_string(app.join("Contents/Resources/which")).unwrap(), "new");
    }

    /// The latest release from GitHub over the app at `SHEPHERD_UPDATE_APP` (network; run by hand).
    #[test]
    #[ignore]
    fn updates_an_app_from_github() {
        let app = PathBuf::from(std::env::var("SHEPHERD_UPDATE_APP").expect("SHEPHERD_UPDATE_APP"));
        let release = latest().unwrap();
        let mut seen = Vec::new();
        let zip = download(&release, &std::env::temp_dir().join("shepherd-update-test"), |p| seen.push(p)).unwrap();
        assert_eq!(seen.last(), Some(&100));
        replace(&zip, &app).unwrap();
    }
}
