//! Agents that need you: a macOS notification when one starts waiting or finishes out of sight, and a Dock badge
//! counting those not looked at yet.
use crate::model::{AgentStatus, Pane, Snapshot};
use gpui::{App, SystemNotification, SystemNotificationAction};
#[cfg(target_os = "macos")]
use objc2::MainThreadMarker;
#[cfg(target_os = "macos")]
use objc2::AllocAnyThread;
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSApplication, NSImage};
#[cfg(target_os = "macos")]
use objc2::runtime::Bool;
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
    // ponytail: outside an .app UNUserNotificationCenter aborts (gpui makes it a no-op), so a bare binary shows none;
    // `cargo run` goes through scripts/run-bundled.sh, which runs it from inside Shepherd.app.
    if can_notify() {
        cx.show_system_notification(SystemNotification {
            // macOS replaces a delivered notification with the same tag without a banner: one tag per post
            tag: format!("{pane_id}{TAG_SEP}{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()).into(),
            title: title.into(),
            body: body.into(),
            actions: approve.then(|| SystemNotificationAction { id: APPROVE.into(), label: "Approve".into() }).into_iter().collect(),
        });
    }
    if sound {
        play_sound();
    }
}

#[cfg(target_os = "macos")]
fn can_notify() -> bool {
    NSBundle::mainBundle().bundleIdentifier().is_some()
}

/// The notification daemon (mako, dunst, …) takes them over D-Bus; gpui posts through it.
#[cfg(not(target_os = "macos"))]
fn can_notify() -> bool {
    true
}

#[cfg(target_os = "macos")]
fn play_sound() {
    run(std::process::Command::new("afplay").arg("/System/Library/Sounds/Glass.aiff"));
}

/// The freedesktop theme's bell through PipeWire/PulseAudio, when the sound theme is installed.
#[cfg(not(target_os = "macos"))]
fn play_sound() {
    const SOUND: &str = "/usr/share/sounds/freedesktop/stereo/complete.oga";
    if std::path::Path::new(SOUND).is_file() {
        run(std::process::Command::new("paplay").arg(SOUND));
    }
}

/// macOS has Shepherd's notifications turned off: none show up, and it will not ask again.
pub struct Denied;

impl gpui::Global for Denied {}

/// Asks macOS for notifications at launch; when they are off, sets [`Denied`] and shows a toast that opens Shepherd's
/// page in System Settings. gpui only asks on the first notification, and posts it before the answer, so that one is
/// lost and the prompt comes while you are away. Once answered, macOS answers again without a prompt.
#[cfg(not(target_os = "macos"))]
pub fn warn_if_denied(_: &mut App) {}

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
        let _ = cx.update(|cx| {
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

/// Shepherd's page in System Settings → Notifications: banners, sounds, and whether they show at all.
#[cfg(not(target_os = "macos"))]
pub fn open_system_settings() {}

#[cfg(target_os = "macos")]
pub fn open_system_settings() {
    let id = NSBundle::mainBundle().bundleIdentifier().map_or_else(|| "dev.freethinkel.shepherd".into(), |id| id.to_string());
    run(std::process::Command::new("open").arg(format!("x-apple.systempreferences:com.apple.Notifications-Settings.extension?id={id}")));
}

/// Runs the command off the main thread and reaps it, so no zombie is left behind.
fn run(command: &mut std::process::Command) {
    if let Ok(mut child) = command.spawn() {
        std::thread::spawn(move || child.wait());
    }
}

/// The Dock icon's badge: the count, or none at 0.
#[cfg(not(target_os = "macos"))]
pub fn set_badge(_count: usize) {}

#[cfg(target_os = "macos")]
pub fn set_badge(count: usize) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let label = (count > 0).then(|| NSString::from_str(&count.to_string()));
    NSApplication::sharedApplication(mtm).dockTile().setBadgeLabel(label.as_deref());
}

/// The Dock icon. A bare `cargo run` binary has no bundle to take it from.
#[cfg(not(target_os = "macos"))]
pub fn set_app_icon() {}

#[cfg(target_os = "macos")]
pub fn set_app_icon() {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let data = NSData::with_bytes(include_bytes!("../assets/app-icon/Shepherd.icns"));
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else { return };
    unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image)) };
}

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
