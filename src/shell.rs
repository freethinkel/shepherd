//! The window: workspaces sidebar, the tab bar, and one Ghostty surface per pane of the shown tab,
//! refreshed on every herdr event.
use crate::herdr::{Client, EVENT_TYPES};
use crate::model::{AgentStatus, PaneFrame, Snapshot, attachable};
use gpui::{
    Bounds, Context, Hsla, IntoElement, ParentElement, Render, SharedString, Styled, Window, WindowAppearance,
    WindowOptions, div, prelude::*, px, relative, rgb, size,
};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

pub struct Shell {
    snapshot: Option<Snapshot>,
    error: Option<String>,
    // ponytail: unit placeholder until Task 7 holds pane terminals here.
    surfaces: HashMap<String, ()>,
    own_pane: Option<String>,
    /// herdr's focused pane we last moved AppKit focus to; refocusing on every paint would undo clicks.
    focused: Option<String>,
    /// The tab Shepherd shows. Local on purpose: herdr's focus is shared with its TUI clients.
    selected_tab: Option<String>,
}

impl Shell {
    pub fn open(rt: tokio::runtime::Handle, cx: &mut gpui::App) {
        let options = WindowOptions {
            window_bounds: Some(gpui::WindowBounds::Windowed(Bounds::centered(None, size(px(1100.), px(720.)), cx))),
            ..Default::default()
        };
        cx.open_window(options, |_, cx| {
            cx.new(|cx| {
                let shell = Shell {
                    snapshot: None,
                    error: None,
                    surfaces: HashMap::new(),
                    own_pane: std::env::var("HERDR_PANE_ID").ok(),
                    focused: None,
                    selected_tab: std::env::var("SHEPHERD_TAB").ok(),
                };
                cx.spawn(async move |this, cx| sync_loop(this, rt, cx).await).detach();
                shell
            })
        })
        .expect("open window");
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

struct Palette {
    bg: Hsla,
    sidebar: Hsla,
    border: Hsla,
    hover: Hsla,
    selected: Hsla,
    text: Hsla,
    muted: Hsla,
}

// ponytail: two fixed palettes; plan 2 brings Zeron's theme as in Collie.
fn palette(appearance: WindowAppearance) -> Palette {
    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Palette {
            bg: rgb(0x1c1c1e).into(),
            sidebar: rgb(0x232326).into(),
            border: rgb(0x38383c).into(),
            hover: rgb(0x2e2e32).into(),
            selected: rgb(0x3a3a40).into(),
            text: rgb(0xe6e6e8).into(),
            muted: rgb(0x8e8e93).into(),
        },
        _ => Palette {
            bg: rgb(0xffffff).into(),
            sidebar: rgb(0xf4f4f5).into(),
            border: rgb(0xe0e0e3).into(),
            hover: rgb(0xe9e9ec).into(),
            selected: rgb(0xdcdce1).into(),
            text: rgb(0x1d1d1f).into(),
            muted: rgb(0x6e6e73).into(),
        },
    }
}

fn status_color(status: AgentStatus) -> Option<Hsla> {
    match status {
        AgentStatus::Working => Some(rgb(0x0a84ff).into()),
        AgentStatus::Blocked => Some(rgb(0xff9f0a).into()),
        AgentStatus::Done => Some(rgb(0x30d158).into()),
        AgentStatus::Idle | AgentStatus::Unknown => None,
    }
}

fn status_dot(status: AgentStatus, muted: Hsla) -> impl IntoElement {
    let color = status_color(status);
    div()
        .size(px(7.))
        .flex_none()
        .rounded_full()
        .when_some(color, |d, c| d.bg(c))
        .when(color.is_none(), |d| d.border_1().border_color(muted))
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette(window.appearance());
        let snapshot = self.snapshot.clone();
        let tab = snapshot.as_ref().and_then(|s| s.pick_tab(self.selected_tab.as_deref())).cloned();
        let frames = match (&snapshot, &tab) {
            (Some(s), Some(t)) => s.pane_frames(&t.tab_id),
            _ => Vec::new(),
        };
        prune(&mut self.surfaces, &mut self.focused, &frames, |_| false);

        let shown_ws = tab.as_ref().map(|t| t.workspace_id.clone());
        let shown_tab = tab.as_ref().map(|t| t.tab_id.clone());
        let mut workspaces = snapshot.as_ref().map(|s| s.workspaces.clone()).unwrap_or_default();
        workspaces.sort_by_key(|w| w.number);

        let sidebar = div()
            .w(px(220.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(2.))
            .p_2()
            .bg(p.sidebar)
            .border_r_1()
            .border_color(p.border)
            .child(div().px_2().pb_1().text_xs().text_color(p.muted).child("WORKSPACES"))
            .children(workspaces.into_iter().map(|w| {
                let selected = shown_ws.as_deref() == Some(w.workspace_id.as_str());
                let first_tab = snapshot.as_ref().and_then(|s| s.tabs_of(&w.workspace_id).first().map(|t| t.tab_id.clone()));
                let target = w.active_tab_id.clone().or(first_tab);
                div()
                    .id(SharedString::from(format!("ws-{}", w.workspace_id)))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .text_sm()
                    .cursor_pointer()
                    .when(selected, |d| d.bg(p.selected))
                    .when(!selected, |d| d.hover(|d| d.bg(p.hover)))
                    .child(status_dot(w.agent_status, p.muted))
                    .child(div().flex_1().overflow_hidden().whitespace_nowrap().text_ellipsis().child(w.label.clone()))
                    .child(div().text_xs().text_color(p.muted).child(w.number.to_string()))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_tab = target.clone();
                        cx.notify();
                    }))
            }));

        let tabs = match (&snapshot, &shown_ws) {
            (Some(s), Some(ws)) => s.tabs_of(ws).into_iter().cloned().collect(),
            _ => Vec::new(),
        };
        let tabbar = div()
            .id("tabbar")
            .overflow_x_scroll()
            .h(px(34.))
            .flex_none()
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .border_b_1()
            .border_color(p.border)
            .children(tabs.into_iter().map(|t| {
                let selected = shown_tab.as_deref() == Some(t.tab_id.as_str());
                let id = t.tab_id.clone();
                div()
                    .id(SharedString::from(format!("tab-{}", t.tab_id)))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .text_sm()
                    .cursor_pointer()
                    .when(selected, |d| d.bg(p.selected))
                    .when(!selected, |d| d.text_color(p.muted).hover(|d| d.bg(p.hover)))
                    .flex_none()
                    .max_w(px(220.))
                    .child(status_dot(t.agent_status, p.muted))
                    .child(div().overflow_hidden().whitespace_nowrap().text_ellipsis().child(t.label.clone()))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_tab = Some(id.clone());
                        cx.notify();
                    }))
            }));

        let own = self.own_pane.clone();
        let border = p.border;
        let panes = div()
            .flex_1()
            .relative()
            .overflow_hidden()
            .when_some(self.error.clone(), |el, err| el.child(div().p_4().child(err)))
            .when(snapshot.is_none() && self.error.is_none(), |el| el.child(div().p_4().text_color(p.muted).child("Connecting to herdr…")))
            .children(frames.into_iter().map(move |f| {
                // 1-px gutter in the border color stands in for split dividers until plan 2.
                let cell = div()
                    .absolute()
                    .left(relative(f.frame.x))
                    .top(relative(f.frame.y))
                    .w(relative(f.frame.w))
                    .h(relative(f.frame.h))
                    .p(px(0.5))
                    .bg(border);
                if !attachable(&f.pane_id, own.as_deref()) {
                    return cell.child(div().size_full().p_4().child(format!("{} is this Shepherd's own pane", f.pane_id)));
                }
                cell.child(div().size_full().bg(p.bg).p_2().text_color(p.muted).child(f.pane_id.clone()))
            }));

        div()
            .size_full()
            .flex()
            .bg(p.bg)
            .text_color(p.text)
            .child(sidebar)
            .child(div().flex_1().h_full().flex().flex_col().child(tabbar).child(panes))
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
