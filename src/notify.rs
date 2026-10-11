//! Agents that need you: a macOS notification when one starts waiting or finishes out of sight, and a Dock badge
//! counting those not looked at yet.
use crate::model::{AgentStatus, Pane, Snapshot};
use gpui::{App, SystemNotification};
use objc2::MainThreadMarker;
use objc2::AllocAnyThread;
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::{NSBundle, NSData, NSString};

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

/// Posts a notification tagged with the pane, so a click can bring that pane up.
pub fn show(cx: &App, pane_id: &str, title: String, body: String, sound: bool) {
    // ponytail: outside an .app UNUserNotificationCenter aborts (gpui makes it a no-op), so a bare binary shows none;
    // `cargo run` goes through scripts/run-bundled.sh, which runs it from inside Shepherd.app.
    if NSBundle::mainBundle().bundleIdentifier().is_some() {
        cx.show_system_notification(SystemNotification {
            tag: pane_id.to_string().into(),
            title: title.into(),
            body: body.into(),
            actions: Vec::new(),
        });
    }
    if sound {
        run(std::process::Command::new("afplay").arg("/System/Library/Sounds/Glass.aiff"));
    }
}

/// Runs the command off the main thread and reaps it, so no zombie is left behind.
fn run(command: &mut std::process::Command) {
    if let Ok(mut child) = command.spawn() {
        std::thread::spawn(move || child.wait());
    }
}

/// The Dock icon's badge: the count, or none at 0.
pub fn set_badge(count: usize) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let label = (count > 0).then(|| NSString::from_str(&count.to_string()));
    NSApplication::sharedApplication(mtm).dockTile().setBadgeLabel(label.as_deref());
}

/// The Dock icon. A bare `cargo run` binary has no bundle to take it from.
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
