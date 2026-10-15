//! Keeping the window in step with herdr: subscribe, snapshot, and start herdr's server when nothing answers.
use super::*;

/// Subscribe, then snapshot; every event (coalesced) triggers another snapshot. Reconnects forever.
pub(super) async fn sync_loop(this: gpui::WeakEntity<Shell>, rt: tokio::runtime::Handle, client: Client, cx: &mut gpui::AsyncApp) {
    // When nothing listens on the socket, Shepherd starts herdr's server itself: again after 10 s if it never came
    // up, after a minute if it could not be run at all.
    let (mut next_start, mut spawned, mut start_error) = (Instant::now(), None::<Instant>, None::<String>);
    // the panes whose agent status the subscription follows: herdr only sends that per pane
    let mut watched: Vec<String> = Vec::new();
    loop {
        let sub = {
            let (c, panes) = (client.clone(), watched.clone());
            rt.spawn(async move { c.subscribe(EVENT_TYPES, &panes).await })
        };
        let mut rx = match sub.await.expect("tokio") {
            Ok(rx) => {
                check_protocol(&this, &rt, &client, cx).await;
                rx
            }
            Err(err) => {
                let no_server = crate::herdr::no_server(&err);
                if no_server && Instant::now() >= next_start {
                    match crate::herdr::start_server() {
                        Ok(()) => (spawned, start_error, next_start) = (Some(Instant::now()), None, Instant::now() + Duration::from_secs(10)),
                        Err(e) => {
                            start_error = Some(format!("herdr isn't running and Shepherd couldn't start it: {e}. Install herdr or set its path in Settings."));
                            next_start = Instant::now() + Duration::from_secs(60);
                        }
                    }
                }
                let message = match (&start_error, spawned) {
                    (Some(e), _) if no_server => e.clone(),
                    (None, Some(at)) if no_server && at.elapsed() < Duration::from_secs(10) => "Starting herdr…".into(),
                    _ => format!("{err:#}"),
                };
                let alive = this.update(cx, |s, cx| {
                    s.error = Some(message);
                    cx.notify()
                });
                if alive.is_err() {
                    return;
                }
                cx.background_executor().timer(Duration::from_secs(1)).await;
                continue;
            }
        };
        loop {
            let c = client.clone();
            let result = rt.spawn(async move { c.call("session.snapshot", json!({})).await }).await.expect("tokio");
            let update = result.and_then(|r| Snapshot::from_result(&r));
            let alive = this.update(cx, |s, cx| {
                let mut panes = None;
                match update {
                    Ok(snapshot) => {
                        let mut ids: Vec<String> = snapshot.panes.iter().map(|p| p.pane_id.clone()).collect();
                        ids.sort(); // a reorder (a dragged tab) is the same set
                        panes = Some(ids);
                        // a workspace's agent that starts or stops has likely edited its worktree
                        for w in snapshot.workspaces.iter().filter(|w| {
                            s.snapshot.as_ref().is_some_and(|old| old.workspaces.iter().any(|o| o.workspace_id == w.workspace_id && o.agent_status != w.agent_status))
                        }) {
                            if let Some(t) = &w.worktree {
                                crate::ui::statusbar::refresh_changes(&crate::herdr::locate(&t.checkout_path), cx);
                            }
                        }
                        s.agents_changed(&snapshot, cx);
                        // herdr's focus moved (its TUI, another window, a command): Shepherd shows the same tab. Only a
                        // move counts, so a snapshot taken before your own switch reached herdr does not undo it.
                        let old = s.snapshot.as_ref().map(|o| o.focused_tab_id.as_deref());
                        if let Some(tab) = moved_focus(old, snapshot.focused_tab_id.as_deref()) {
                            s.selected_tab = Some(tab.into());
                        }
                        snapshot.track(&mut s.since, Instant::now());
                        s.snapshot = Some(snapshot);
                        s.error = None;
                        s.connecting = None;
                    }
                    Err(err) => s.error = Some(format!("{err:#}")),
                }
                cx.notify();
                panes
            });
            let Ok(panes) = alive else {
                return; // window closed
            };
            if let Some(panes) = panes.filter(|p| *p != watched) {
                watched = panes;
                break; // panes came or went: follow the new set's agents
            }
            // ponytail: refetch the whole snapshot per event burst; incremental apply only if this shows up in profiles.
            let Some(event) = rx.recv().await else {
                break; // server gone or events_lost: resubscribe
            };
            cx.background_executor().timer(Duration::from_millis(30)).await;
            let mut focused = event["event"] == "pane.focused";
            while let Ok(event) = rx.try_recv() {
                focused |= event["event"] == "pane.focused";
            }
            // ponytail: herdr's events do not name the client; a focus change Shepherd did not just cause is
            // another client's (its TUI in a terminal, over ssh), and the client used last sizes the panes
            if focused {
                let _ = this.update(cx, |s, cx| {
                    if s.touched.elapsed() > Duration::from_secs(2) {
                        cx.set_global(crate::terminal::pane::Yielded(true));
                    }
                });
            }
        }
    }
}

/// Asks the server which protocol it speaks. A change of verdict is told once, as a toast; the verdict stays in
/// [`crate::herdr::OutdatedServer`] for the panes whose connection it breaks.
async fn check_protocol(this: &gpui::WeakEntity<Shell>, rt: &tokio::runtime::Handle, client: &Client, cx: &mut gpui::AsyncApp) {
    let c = client.clone();
    let Ok(Ok(pong)) = rt.spawn(async move { c.call("ping", json!({})).await }).await else { return };
    let mismatch = crate::herdr::protocol_mismatch(&pong);
    let _ = this.update(cx, |_, cx| {
        let before = cx.try_global::<crate::herdr::OutdatedServer>().and_then(|o| o.0.clone());
        if before == mismatch {
            return;
        }
        if let Some(why) = &mismatch {
            crate::ui::toast::error(cx, why.clone());
        }
        cx.set_global(crate::herdr::OutdatedServer(mismatch));
        cx.refresh_windows();
    });
}

impl Shell {
    /// The other way: the tab you show gets herdr's focus, through the public API, which moves every client shell's
    /// tab with it (herdr's TUI, this window's shell, whose Kitty images are its own tab's only).
    pub(super) fn follow_shown_tab(&mut self) {
        let Some(tab) = self.shown_tab().map(|t| t.tab_id) else { return };
        if self.focused_tab.as_ref() == Some(&tab) {
            return;
        }
        if self.snapshot.as_ref().and_then(|s| s.focused_tab_id.as_ref()) != Some(&tab) {
            let (client, id) = (self.client.clone(), tab.clone());
            self.rt.spawn(async move { client.call("tab.focus", json!({"tab_id": id})).await });
        }
        self.focused_tab = Some(tab);
    }
}

/// herdr's focused tab when its focus moved since the last snapshot (`old`: none yet, or its focused tab).
fn moved_focus<'a>(old: Option<Option<&str>>, new: Option<&'a str>) -> Option<&'a str> {
    new.filter(|t| old.is_some_and(|o| o != Some(*t)))
}

#[cfg(test)]
mod tests {
    use super::moved_focus;

    #[test]
    fn follows_herdrs_focus_only_when_it_moves() {
        assert_eq!(moved_focus(Some(Some("w1:t1")), Some("w1:t2")), Some("w1:t2"));
        // unchanged: Shepherd's own choice stands (a snapshot from before its switch reached herdr)
        assert_eq!(moved_focus(Some(Some("w1:t1")), Some("w1:t1")), None);
        // the first snapshot: Shepherd's start-up tab stands
        assert_eq!(moved_focus(None, Some("w1:t1")), None);
        assert_eq!(moved_focus(Some(None), Some("w1:t1")), Some("w1:t1"));
        assert_eq!(moved_focus(Some(Some("w1:t1")), None), None);
    }
}
