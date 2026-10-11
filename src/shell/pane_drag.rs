//! Dragging a pane by its grip: onto another pane's edge to split it there, onto its middle to swap the two,
//! or onto a tab to move it into that tab. herdr moves the pane's process along (`pane.move`, `pane.swap`).
use super::Shell;
use crate::commands::Command;
use crate::model::{Frac, PaneFrame};
use crate::theme::Theme;
use gpui::{
    Context, CursorStyle, Div, DragMoveEvent, Hsla, IntoElement, MouseButton, ParentElement, Render, SharedString,
    Stateful, Styled, Transformation, Window, div, prelude::*, px, radians, relative,
};

/// A pane being dragged, drawn under the pointer as a pill with its name.
#[derive(Clone)]
pub(super) struct DraggedPane {
    pane_id: String,
    tab_id: String,
    label: String,
    theme: Theme,
}

impl Render for DraggedPane {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let theme = &self.theme;
        div()
            .h(px(28.))
            .max_w(px(240.))
            .px(px(10.))
            .flex()
            .items_center()
            .gap(px(6.))
            .rounded(px(8.))
            .bg(theme.card)
            .border_1()
            .border_color(theme.border)
            .shadow_lg()
            .font_family(theme.font_ui.clone())
            .text_size(px(13.))
            .text_color(theme.text)
            .child(gpui::svg().path("icons/terminal.svg").size(px(14.)).flex_none().text_color(theme.text_muted))
            .child(div().truncate().child(self.label.clone()))
    }
}

/// The part of a pane a dragged one lands on: an edge splits the pane there, the middle swaps the two.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Zone {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

/// How far into a pane (as a fraction of its size) its edges reach.
const EDGE: f32 = 0.25;

/// The pane under `(x, y)` (fractions of the grid) and the zone of it the pointer is in.
pub(super) fn target_at(frames: &[PaneFrame], x: f32, y: f32) -> Option<(String, Zone)> {
    let f = frames.iter().find(|f| (f.frame.x..f.frame.x + f.frame.w).contains(&x) && (f.frame.y..f.frame.y + f.frame.h).contains(&y))?;
    let (u, v) = ((x - f.frame.x) / f.frame.w, (y - f.frame.y) / f.frame.h);
    let edges = [(u, Zone::Left), (1. - u, Zone::Right), (v, Zone::Top), (1. - v, Zone::Bottom)];
    let (depth, zone) = edges.into_iter().min_by(|a, b| a.0.total_cmp(&b.0))?;
    Some((f.pane_id.clone(), if depth > EDGE { Zone::Center } else { zone }))
}

/// The part of `frame` a pane dropped on `zone` would take.
fn landing(f: Frac, zone: Zone) -> Frac {
    let (w, h) = (f.w / 2., f.h / 2.);
    match zone {
        Zone::Left => Frac { w, ..f },
        Zone::Right => Frac { x: f.x + w, w, ..f },
        Zone::Top => Frac { h, ..f },
        Zone::Bottom => Frac { y: f.y + h, h, ..f },
        Zone::Center => f,
    }
}

/// The herdr calls that drop `source` on `zone` of `target`, both panes of `tab_id`. herdr refuses `pane.move`
/// within a tab (`same_tab`) and only splits right or down, so the pane hops out to a new tab and back (the
/// emptied tab closes), and a left or top split is a right or down one plus a swap.
// ponytail: the hop flashes a tab for a moment; one call once herdr moves within a tab and splits left/up.
pub(super) fn plan(source: &str, target: &str, tab_id: &str, zone: Zone) -> Vec<Command> {
    if source == target {
        return Vec::new();
    }
    let swap = Command::SwapPanes { source: source.into(), target: target.into() };
    let (down, then_swap) = match zone {
        Zone::Center => return vec![swap],
        Zone::Right => (false, false),
        Zone::Left => (false, true),
        Zone::Bottom => (true, false),
        Zone::Top => (true, true),
    };
    let mut plan = vec![
        Command::MovePaneToNewTab { pane_id: source.into() },
        Command::MovePane { pane_id: source.into(), tab_id: tab_id.into(), down, target: Some(target.into()) },
    ];
    if then_swap {
        plan.push(swap);
    }
    plan
}

/// A pane drag in progress: where it would land now.
#[derive(Default)]
pub(super) struct PaneDrop {
    target: Option<(String, Zone)>,
}

impl Shell {
    /// Runs `commands` one after another, stopping at the first error.
    fn send_all(&mut self, mut commands: Vec<Command>, cx: &mut Context<Self>) {
        if commands.is_empty() {
            return;
        }
        let first = commands.remove(0);
        self.send(first, cx, move |shell, _, cx| shell.send_all(commands, cx));
    }

    /// The handle a pane is dragged by, at the top middle of its cell, shown while `group` (the cell) is hovered.
    /// Off the terminal content, so mouse selection there keeps working.
    pub(super) fn pane_grip(&self, pane_id: &str, tab_id: &str, group: &SharedString, theme: &Theme, cx: &Context<Self>) -> Div {
        let pane = self.snapshot.as_ref().and_then(|s| s.panes.iter().find(|p| p.pane_id == pane_id));
        let label = pane
            .and_then(|p| p.agent.clone().or_else(|| p.terminal_title_stripped.clone()))
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| pane_id.to_string());
        let dragged = DraggedPane { pane_id: pane_id.into(), tab_id: tab_id.into(), label, theme: theme.clone() };
        let shell = cx.entity().downgrade();
        let hover = theme.hover;
        div().absolute().top_0().left_0().w_full().flex().justify_center().child(
            div()
                .id(SharedString::from(format!("pane-grip-{pane_id}")))
                .mt(px(3.))
                .w(px(36.))
                .h(px(14.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.))
                .bg(theme.card)
                .border_1()
                .border_color(theme.border)
                .opacity(0.)
                .group_hover(group.clone(), |s| s.opacity(1.))
                .hover(move |s| s.bg(hover))
                .cursor(CursorStyle::OpenHand)
                .occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_drag(dragged, move |d, _, _, cx| {
                    let _ = shell.update(cx, |shell, cx| {
                        shell.pane_drop = Some(PaneDrop::default());
                        cx.notify();
                    });
                    cx.new(|_| d.clone())
                })
                .child(
                    gpui::svg()
                        .path("icons/drag-handle.svg")
                        .size(px(12.))
                        .with_transformation(Transformation::rotate(radians(std::f32::consts::FRAC_PI_2)))
                        .text_color(theme.text_muted),
                ),
        )
    }

    /// Over the grid while a pane is dragged: finds the drop zone, highlights where the pane would land, and
    /// moves it there on release. Absent otherwise, so the terminals get the mouse.
    pub(super) fn pane_drop_overlay(
        &mut self,
        frames: &[PaneFrame],
        tab_id: &str,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        if !cx.has_active_drag() {
            self.pane_drop = None;
        }
        let drop = self.pane_drop.as_ref()?;
        let landing = drop
            .target
            .as_ref()
            .and_then(|(id, zone)| Some(landing(frames.iter().find(|f| &f.pane_id == id)?.frame, *zone)));
        let (frames, tab_id, accent) = (frames.to_vec(), tab_id.to_string(), theme.accent);
        Some(
            div()
                .id("pane-drop")
                .absolute()
                .inset_0()
                .occlude()
                .on_drag_move(cx.listener(move |this, e: &DragMoveEvent<DraggedPane>, _, cx| {
                    let (b, p) = (e.bounds, e.event.position);
                    let x = f32::from(p.x - b.origin.x) / f32::from(b.size.width);
                    let y = f32::from(p.y - b.origin.y) / f32::from(b.size.height);
                    let source = e.drag(cx).pane_id.clone();
                    let target = target_at(&frames, x, y).filter(|(id, _)| *id != source);
                    if let Some(drop) = this.pane_drop.as_mut().filter(|d| d.target != target) {
                        drop.target = target;
                        cx.notify();
                    }
                }))
                .on_drop(cx.listener(move |this, d: &DraggedPane, _, cx| {
                    if let Some((target, zone)) = this.pane_drop.take().and_then(|drop| drop.target) {
                        this.send_all(plan(&d.pane_id, &target, &tab_id, zone), cx);
                    }
                }))
                .children(landing.map(|f| {
                    div()
                        .absolute()
                        .left(relative(f.x))
                        .top(relative(f.y))
                        .w(relative(f.w))
                        .h(relative(f.h))
                        .p(px(4.))
                        .child(div().size_full().rounded(px(6.)).bg(accent.opacity(0.15)).border_2().border_color(accent.opacity(0.7)))
                })),
        )
    }
}

/// Lets a tab in the tab bar take a dragged pane: it moves into that tab, split right of the whole tab.
pub(super) fn accept_pane(el: Stateful<Div>, tab_id: &str, drop_bg: Hsla, cx: &Context<Shell>) -> Stateful<Div> {
    let (over, dropped) = (tab_id.to_string(), tab_id.to_string());
    el.drag_over::<DraggedPane>(move |s, d, _, _| if d.tab_id == over { s } else { s.bg(drop_bg) }).on_drop(cx.listener(
        move |this, d: &DraggedPane, _, cx| {
            this.pane_drop = None;
            if d.tab_id != dropped {
                let command = Command::MovePane { pane_id: d.pane_id.clone(), tab_id: dropped.clone(), down: false, target: None };
                this.send(command, cx, |_, _, _| {});
            }
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames() -> Vec<PaneFrame> {
        // p1 on the left half, p2 on the right
        let half = |x| Frac { x, y: 0., w: 0.5, h: 1. };
        vec![
            PaneFrame { pane_id: "w1:p1".into(), frame: half(0.), focused: true },
            PaneFrame { pane_id: "w1:p2".into(), frame: half(0.5), focused: false },
        ]
    }

    #[test]
    fn zones_follow_the_nearest_edge() {
        let at = |x, y| target_at(&frames(), x, y);
        assert_eq!(at(0.75, 0.5), Some(("w1:p2".into(), Zone::Center)));
        assert_eq!(at(0.52, 0.5), Some(("w1:p2".into(), Zone::Left)));
        assert_eq!(at(0.98, 0.5), Some(("w1:p2".into(), Zone::Right)));
        assert_eq!(at(0.75, 0.05), Some(("w1:p2".into(), Zone::Top)));
        assert_eq!(at(0.25, 0.95), Some(("w1:p1".into(), Zone::Bottom)));
        assert_eq!(at(1.2, 0.5), None);
    }

    #[test]
    fn landing_is_the_half_on_the_zone_side() {
        let f = Frac { x: 0.5, y: 0., w: 0.5, h: 1. };
        assert_eq!(landing(f, Zone::Right), Frac { x: 0.75, y: 0., w: 0.25, h: 1. });
        assert_eq!(landing(f, Zone::Top), Frac { x: 0.5, y: 0., w: 0.5, h: 0.5 });
        assert_eq!(landing(f, Zone::Center), f);
    }

    #[test]
    fn plans() {
        let (src, dst, tab) = ("w1:p1", "w1:p2", "w1:t1");
        let swap = Command::SwapPanes { source: src.into(), target: dst.into() };
        let hop = Command::MovePaneToNewTab { pane_id: src.into() };
        let back = |down| Command::MovePane { pane_id: src.into(), tab_id: tab.into(), down, target: Some(dst.into()) };
        assert_eq!(plan(src, dst, tab, Zone::Center), vec![swap.clone()]);
        assert_eq!(plan(src, dst, tab, Zone::Bottom), vec![hop.clone(), back(true)]);
        assert_eq!(plan(src, dst, tab, Zone::Left), vec![hop, back(false), swap]);
        assert!(plan(src, src, tab, Zone::Right).is_empty());
    }
}
