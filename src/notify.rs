//! Agents that need you: a system notification when one starts waiting or finishes out of sight, and on macOS a
//! Dock badge counting those not looked at yet.
use crate::model::{AgentStatus, Pane, Snapshot};
use gpui::{App, SystemNotification, SystemNotificationAction};
#[cfg(target_os = "macos")]
use objc2::{AllocAnyThread, MainThreadMarker, runtime::Bool};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSApplication, NSImage};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSBundle, NSData, NSError, NSString};
#[cfg(target_os = "macos")]
use objc2_user_notifications::{UNAuthorizationOptions, UNUserNotificationCenter};

/// Waiting for the user, or finished: worth a look.
pub fn wants_attention(status: AgentStatus) -> bool {
    matches!(status, AgentStatus::Blocked | AgentStatus::Done)
}

/// Panes of `new` whose agent just started waiting or finished. A pane `old` did not have is no arrival, so a
/// reconnect or a new pane never sets off a burst.
pub fn arrivals<'a>(old: &'a Snapshot, new: &'a Snapshot) -> impl Iterator<Item = &'a Pane> {
    new.panes.iter().filter(|p| {
        wants_attention(p.agent_status)
            && old.panes.iter().any(|o| o.pane_id == p.pane_id && o.agent_status != p.agent_status)
    })
}

/// Between the pane and the post's time in a notification's tag.
const TAG_SEP: char = '#';

/// The pane a notification's tag names.
pub fn tag_pane(tag: &str) -> &str {
    tag.split(TAG_SEP).next().unwrap_or(tag)
}

/// The id of a waiting agent's Approve button.
pub const APPROVE: &str = "approve";

/// Posts a notification tagged with the pane, so a click can bring that pane up; `approve` adds a button that
/// answers the agent from the notification.
pub fn show(cx: &App, pane_id: &str, title: String, body: String, sound: bool, approve: bool) {
    if can_notify() {
        cx.show_system_notification(SystemNotification {
            // macOS replaces a delivered notification with the same tag without a banner: one tag per post
            tag: format!("{pane_id}{TAG_SEP}{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()).into(),
            title: title.into(),
            body: body.into(),
            actions: approve.then(|| SystemNotificationAction { id: APPROVE.into(), label: "Approve".into() }).into_iter().collect(),
        });
    }
    #[cfg(target_os = "macos")]
    if sound {
        run(std::process::Command::new("afplay").arg("/System/Library/Sounds/Glass.aiff"));
    }
    #[cfg(target_os = "linux")]
    if sound {
        play_theme_sound();
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let _ = sound;
}

/// The freedesktop sound theme's `complete`, the sound of a finished task. gpui posts the notification without
/// hints, so the notification server can't be asked to play it (`sound-name`): it plays here, through libcanberra's
/// `canberra-gtk-play` (which follows the user's sound theme), else, when that isn't installed, the theme's own file
/// through `paplay` (PulseAudio, or PipeWire's stand-in for it). With neither, nothing plays. A thread starts the
/// player and waits for it, so it is reaped.
#[cfg(target_os = "linux")]
fn play_theme_sound() {
    use std::process::{Command, Stdio};
    let home = std::env::home_dir();
    let dirs = data_dirs(std::env::var_os("XDG_DATA_HOME").as_deref(), std::env::var_os("XDG_DATA_DIRS").as_deref(), home.as_deref());
    let file = theme_sound(&dirs, "complete");
    std::thread::spawn(move || {
        let quiet = |c: &mut Command| -> std::io::Result<std::process::Child> { c.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() };
        let child = match quiet(Command::new("canberra-gtk-play").args(["--id", "complete", "--description", "Shepherd"])) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => match file {
                Some(file) => quiet(Command::new("paplay").arg(file)),
                None => return,
            },
            started => started,
        };
        if let Ok(mut child) = child {
            let _ = child.wait();
        }
    });
}

/// The folders XDG keeps data in, the user's first: `$XDG_DATA_HOME` (else `~/.local/share`), then
/// `$XDG_DATA_DIRS` (else `/usr/local/share` and `/usr/share`). Relative entries are not paths XDG allows: skipped.
#[cfg(target_os = "linux")]
fn data_dirs(data_home: Option<&std::ffi::OsStr>, data_dirs: Option<&std::ffi::OsStr>, home: Option<&std::path::Path>) -> Vec<std::path::PathBuf> {
    use std::path::PathBuf;
    let user = data_home.filter(|d| !d.is_empty()).map(PathBuf::from).filter(|d| d.is_absolute()).or_else(|| home.map(|h| h.join(".local/share")));
    let shared = data_dirs.filter(|d| !d.is_empty()).unwrap_or("/usr/local/share:/usr/share".as_ref());
    user.into_iter().chain(std::env::split_paths(shared).filter(|d| d.is_absolute())).collect()
}

/// The freedesktop theme's file for the sound `name`, in the first data folder that has it.
#[cfg(target_os = "linux")]
fn theme_sound(dirs: &[std::path::PathBuf], name: &str) -> Option<std::path::PathBuf> {
    dirs.iter().map(|d| d.join(format!("sounds/freedesktop/stereo/{name}.oga"))).find(|p| p.is_file())
}

/// Whether a notification can be posted at all.
#[cfg(target_os = "macos")]
fn can_notify() -> bool {
    // ponytail: outside an .app UNUserNotificationCenter aborts (gpui makes it a no-op), so a bare binary shows none;
    // `cargo run` goes through scripts/run-bundled.sh, which runs it from inside Shepherd.app.
    NSBundle::mainBundle().bundleIdentifier().is_some()
}

#[cfg(not(target_os = "macos"))]
fn can_notify() -> bool {
    true
}

/// macOS has Shepherd's notifications turned off: none show up, and it will not ask again.
pub struct Denied;

impl gpui::Global for Denied {}

/// Asks macOS for notifications at launch; when they are off, sets [`Denied`] and shows a toast that opens Shepherd's
/// page in System Settings. gpui only asks on the first notification, and posts it before the answer, so that one is
/// lost and the prompt comes while you are away. Once answered, macOS answers again without a prompt.
#[cfg(target_os = "macos")]
pub fn warn_if_denied(cx: &mut App) {
    if NSBundle::mainBundle().bundleIdentifier().is_none() {
        return;
    }
    let (tx, rx) = futures::channel::oneshot::channel();
    let tx = std::sync::Mutex::new(Some(tx));
    // called once, on a thread of macOS's choosing
    let answer = block2::RcBlock::new(move |granted: Bool, _error: *mut NSError| {
        if let Some(tx) = tx.lock().unwrap().take() {
            let _ = tx.send(!granted.as_bool());
        }
    });
    UNUserNotificationCenter::currentNotificationCenter()
        .requestAuthorizationWithOptions_completionHandler(UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound, &answer);
    cx.spawn(async move |cx| {
        if rx.await != Ok(true) {
            return;
        }
        cx.update(|cx| {
            cx.set_global(Denied);
            cx.refresh_windows();
            if crate::settings::SettingsGlobal::of(cx).settings.notify_agents {
                let open: std::rc::Rc<dyn Fn(&mut App)> = std::rc::Rc::new(|_| open_system_settings());
                crate::ui::toast::info(cx, "Notifications are off for Shepherd in macOS. Click to turn them on.", Some(open));
            }
        });
    })
    .detach();
}

/// Elsewhere nothing asks first.
#[cfg(not(target_os = "macos"))]
pub fn warn_if_denied(_: &mut App) {}

/// Shepherd's page in System Settings → Notifications: banners, sounds, and whether they show at all.
#[cfg(target_os = "macos")]
pub fn open_system_settings() {
    let id = NSBundle::mainBundle().bundleIdentifier().map_or_else(|| "dev.freethinkel.shepherd".into(), |id| id.to_string());
    run(std::process::Command::new("open").arg(format!("x-apple.systempreferences:com.apple.Notifications-Settings.extension?id={id}")));
}

#[cfg(not(target_os = "macos"))]
pub fn open_system_settings() {}

/// Starts the command and waits for it on a thread of its own, so it is reaped and no zombie is left behind.
#[cfg(target_os = "macos")]
fn run(command: &mut std::process::Command) {
    if let Ok(mut child) = command.spawn() {
        std::thread::spawn(move || child.wait());
    }
}

/// The Dock icon's badge: the count, or none at 0.
#[cfg(target_os = "macos")]
pub fn set_badge(count: usize) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let label = (count > 0).then(|| NSString::from_str(&count.to_string()));
    NSApplication::sharedApplication(mtm).dockTile().setBadgeLabel(label.as_deref());
}

#[cfg(not(target_os = "macos"))]
pub fn set_badge(_: usize) {}

/// The Dock icon. A bare `cargo run` binary has no bundle to take it from.
#[cfg(target_os = "macos")]
pub fn set_app_icon() {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let data = NSData::with_bytes(include_bytes!("../assets/app-icon/Shepherd.icns"));
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else { return };
    unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image)) };
}

#[cfg(not(target_os = "macos"))]
pub fn set_app_icon() {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn snapshot(panes: &[(&str, &str)]) -> Snapshot {
        let panes: Vec<_> = panes
            .iter()
            .map(|(id, status)| json!({"pane_id": id, "tab_id": "t1", "workspace_id": "w1", "terminal_id": id, "agent_status": status}))
            .collect();
        serde_json::from_value(json!({"workspaces": [], "tabs": [], "panes": panes, "layouts": []})).unwrap()
    }

    fn ids(old: &Snapshot, new: &Snapshot) -> Vec<String> {
        arrivals(old, new).map(|p| p.pane_id.clone()).collect()
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn the_data_folders_are_the_users_then_the_shared_ones_absolute_only() {
        use std::path::{Path, PathBuf};
        let home = Some(Path::new("/home/u"));
        let dirs = |h: Option<&str>, d: Option<&str>| data_dirs(h.map(std::ffi::OsStr::new), d.map(std::ffi::OsStr::new), home);
        let p = |s: &str| PathBuf::from(s);
        assert_eq!(dirs(None, None), [p("/home/u/.local/share"), p("/usr/local/share"), p("/usr/share")]);
        assert_eq!(dirs(Some("/data"), Some("/opt/share:rel:/usr/share")), [p("/data"), p("/opt/share"), p("/usr/share")]);
        assert_eq!(dirs(Some("rel"), Some("")), [p("/home/u/.local/share"), p("/usr/local/share"), p("/usr/share")]);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn the_theme_sound_is_the_first_folders_that_has_it() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let sound = b.path().join("sounds/freedesktop/stereo/complete.oga");
        std::fs::create_dir_all(sound.parent().unwrap()).unwrap();
        std::fs::write(&sound, b"").unwrap();
        assert_eq!(theme_sound(&[a.path().into(), b.path().into()], "complete"), Some(sound));
        assert_eq!(theme_sound(&[a.path().into()], "complete"), None);
    }

    #[test]
    fn a_tag_names_its_pane() {
        assert_eq!(tag_pane("w2:p1#1791794600123"), "w2:p1");
        assert_eq!(tag_pane("w2:p1"), "w2:p1");
    }

    #[test]
    fn waiting_and_finishing_arrive_once() {
        let old = snapshot(&[("p1", "working"), ("p2", "working"), ("p3", "idle")]);
        let new = snapshot(&[("p1", "blocked"), ("p2", "done"), ("p3", "working")]);
        assert_eq!(ids(&old, &new), ["p1", "p2"]);
        assert!(ids(&new, &new).is_empty(), "a status that stays is no news");
    }

    #[test]
    fn answered_then_done_arrives_again() {
        assert_eq!(ids(&snapshot(&[("p1", "blocked")]), &snapshot(&[("p1", "done")])), ["p1"]);
    }

    #[test]
    fn panes_new_to_the_snapshot_do_not_arrive() {
        assert!(ids(&snapshot(&[]), &snapshot(&[("p1", "done")])).is_empty());
    }

}
