//! Plan 1's window: herdr's focused tab, one Ghostty surface per pane, refreshed on every herdr event.
use crate::ghostty::{self, Surface};
use crate::herdr::{Client, EVENT_TYPES};
use crate::model::{PaneFrame, Snapshot, attachable};
use gpui::{
    Bounds, Context, IntoElement, ParentElement, Pixels, Render, Styled, Window, WindowOptions, canvas, div,
    prelude::*, px, relative, rgb, size,
};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

pub struct Shell {
    app: ghostty::App,
    snapshot: Option<Snapshot>,
    error: Option<String>,
    surfaces: HashMap<String, Surface>,
    own_pane: Option<String>,
    /// herdr's focused pane we last moved AppKit focus to; refocusing on every paint would undo clicks.
    focused: Option<String>,
}

impl Shell {
    pub fn open(app: ghostty::App, rt: tokio::runtime::Handle, cx: &mut gpui::App) {
        let options = WindowOptions {
            window_bounds: Some(gpui::WindowBounds::Windowed(Bounds::centered(None, size(px(1100.), px(720.)), cx))),
            ..Default::default()
        };
        cx.open_window(options, |_, cx| {
            cx.new(|cx| {
                let shell = Shell {
                    app,
                    snapshot: None,
                    error: None,
                    surfaces: HashMap::new(),
                    own_pane: std::env::var("HERDR_PANE_ID").ok(),
                    focused: None,
                };
                cx.spawn(async move |this, cx| sync_loop(this, rt, cx).await).detach();
                shell
            })
        })
        .expect("open window");
    }

    fn place(&mut self, pane_id: &str, focused: bool, bounds: Bounds<Pixels>, window: &mut Window) {
        if !self.surfaces.contains_key(pane_id) {
            match Surface::new(self.app, window, pane_id) {
                Ok(surface) => {
                    self.surfaces.insert(pane_id.to_string(), surface);
                }
                Err(err) => {
                    self.error = Some(format!("{err:#}"));
                    return;
                }
            }
        }
        let surface = &self.surfaces[pane_id];
        surface.set_bounds(bounds, window);
        if focused && self.focused.as_deref() != Some(pane_id) {
            surface.focus();
            self.focused = Some(pane_id.to_string());
        }
    }
}

/// Keep surfaces of shown panes whose shim still runs; a dropped one is recreated (reattached) on the next
/// paint. Focus is forgotten with its surface so the recreated one gets AppKit focus again.
// ponytail: while herdr is down this reattaches once per refresh (about 1/s); add a backoff if that shows up.
fn prune<S>(surfaces: &mut HashMap<String, S>, focused: &mut Option<String>, shown: &[PaneFrame], exited: impl Fn(&S) -> bool) {
    surfaces.retain(|id, s| shown.iter().any(|f| &f.pane_id == id) && !exited(s));
    if focused.as_ref().is_some_and(|id| !surfaces.contains_key(id)) {
        *focused = None;
    }
}

/// Subscribe, then snapshot; every event (coalesced) triggers another snapshot. Reconnects forever.
async fn sync_loop(this: gpui::WeakEntity<Shell>, rt: tokio::runtime::Handle, cx: &mut gpui::AsyncApp) {
    let client = Client::local();
    loop {
        let sub = {
            let c = client.clone();
            rt.spawn(async move { c.subscribe(EVENT_TYPES).await })
        };
        let mut rx = match sub.await.expect("tokio") {
            Ok(rx) => rx,
            Err(err) => {
                let alive = this.update(cx, |s, cx| {
                    s.error = Some(format!("{err:#}"));
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
                match update {
                    Ok(snapshot) => {
                        s.snapshot = Some(snapshot);
                        s.error = None;
                    }
                    Err(err) => s.error = Some(format!("{err:#}")),
                }
                cx.notify();
            });
            if alive.is_err() {
                return; // window closed
            }
            // ponytail: refetch the whole snapshot per event burst; incremental apply only if this shows up in profiles.
            if rx.recv().await.is_none() {
                break; // server gone or events_lost: resubscribe
            }
            cx.background_executor().timer(Duration::from_millis(30)).await;
            while rx.try_recv().is_ok() {}
        }
    }
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let frames = self
            .snapshot
            .as_ref()
            // SHEPHERD_TAB pins a tab for spikes without moving herdr's own focus.
            .and_then(|s| std::env::var("SHEPHERD_TAB").ok().or(s.focused_tab_id.clone()).map(|t| s.pane_frames(&t)))
            .unwrap_or_default();
        prune(&mut self.surfaces, &mut self.focused, &frames, Surface::exited);
        let view = cx.entity();
        let own = self.own_pane.clone();
        div()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xcccccc))
            .relative()
            .when_some(self.error.clone(), |el, err| el.child(div().p_4().child(err)))
            .children(frames.into_iter().map(move |f| {
                let view = view.clone();
                let cell = div()
                    .absolute()
                    .left(relative(f.frame.x))
                    .top(relative(f.frame.y))
                    .w(relative(f.frame.w))
                    .h(relative(f.frame.h));
                if !attachable(&f.pane_id, own.as_deref()) {
                    return cell.p_4().child(format!("{} is this Shepherd's own pane", f.pane_id));
                }
                cell.child(
                    canvas(|_, _, _| (), move |bounds, _, window, cx| {
                        view.update(cx, |shell, _| shell.place(&f.pane_id, f.focused, bounds, window));
                    })
                    .size_full(),
                )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::prune;
    use crate::model::{Frac, PaneFrame};
    use std::collections::HashMap;

    fn shown(ids: &[&str]) -> Vec<PaneFrame> {
        let frame = Frac { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
        ids.iter().map(|id| PaneFrame { pane_id: id.to_string(), frame, focused: false }).collect()
    }

    /// Surfaces stand in as "has its process exited?".
    fn surfaces(entries: &[(&str, bool)]) -> HashMap<String, bool> {
        entries.iter().map(|(id, exited)| (id.to_string(), *exited)).collect()
    }

    #[test]
    fn hidden_panes_lose_their_surface_and_focus() {
        let mut s = surfaces(&[("w1:p1", false), ("w1:p2", false)]);
        let mut focused = Some("w1:p2".to_string());
        prune(&mut s, &mut focused, &shown(&["w1:p1"]), |exited| *exited);
        assert_eq!(s.keys().collect::<Vec<_>>(), ["w1:p1"]);
        assert_eq!(focused, None, "a recreated surface for w1:p2 must get focus again");
    }

    #[test]
    fn exited_surfaces_are_dropped_to_be_recreated() {
        let mut s = surfaces(&[("w1:p1", true)]);
        let mut focused = Some("w1:p1".to_string());
        prune(&mut s, &mut focused, &shown(&["w1:p1"]), |exited| *exited);
        assert!(s.is_empty());
        assert_eq!(focused, None);
    }

    #[test]
    fn live_shown_surfaces_keep_focus() {
        let mut s = surfaces(&[("w1:p1", false)]);
        let mut focused = Some("w1:p1".to_string());
        prune(&mut s, &mut focused, &shown(&["w1:p1"]), |exited| *exited);
        assert_eq!(s.len(), 1);
        assert_eq!(focused.as_deref(), Some("w1:p1"));
    }
}
