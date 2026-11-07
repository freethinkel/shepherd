//! Panes that float over a tab's split (Rex's floating layers): moved by their grip, resized by their edges and corners,
//! raised by a click, floated out of the split and docked back from the pane's menu.
use super::*;
use crate::backend::Float;
use crate::model::Frac;
use gpui::{AnyElement, CursorStyle, Transformation, radians};

/// The smallest a floating layer is resized to, as a fraction of the tab.
const MIN: f32 = 0.1;
/// A resize corner's side.
const CORNER: f32 = 14.;
/// A resize edge's thickness, half of it outside the card.
const EDGE: f32 = 6.;

/// What a drag on a floating layer takes: the whole layer, or the sides of it that a resize handle moves.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Grab {
    Move,
    Resize { left: bool, top: bool, right: bool, bottom: bool },
}

/// A floating layer being moved or resized.
pub(super) struct LayerDrag {
    pane_id: String,
    layer: String,
    from: Frac,
    now: Frac,
    start: gpui::Point<Pixels>,
    grab: Grab,
}

impl Shell {
    /// The layer's frame shown now: while dragged, or let go until the backend's snapshot catches up.
    pub(super) fn layer_override(&mut self, frames: &[PaneFrame]) -> Option<(String, Frac)> {
        if let Some((layer, to)) = &self.layer_settling
            && layer_frame(frames, layer).is_none_or(|now| close(now, *to))
        {
            self.layer_settling = None;
        }
        self.layer_drag.as_ref().map(|d| (d.layer.clone(), d.now)).or_else(|| self.layer_settling.clone())
    }

    fn start_layer_drag(&mut self, pane_id: String, layer: String, from: Frac, grab: Grab, start: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        cx.set_global(GridHold(true));
        self.layer_drag = Some(LayerDrag { pane_id, layer, from, now: from, start, grab });
        cx.notify();
    }

    pub(super) fn drag_layer(&mut self, position: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        let (Some(drag), Some(grid)) = (&mut self.layer_drag, self.grid_bounds.get()) else { return };
        let dx = f32::from(position.x - drag.start.x) / f32::from(grid.size.width).max(1.);
        let dy = f32::from(position.y - drag.start.y) / f32::from(grid.size.height).max(1.);
        drag.now = dragged(drag.from, dx, dy, drag.grab);
        cx.notify();
    }

    pub(super) fn end_layer_drag(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.layer_drag.take() else { return };
        cx.set_global(GridHold(false));
        cx.refresh_windows();
        if close(drag.now, drag.from) {
            return; // a click on the grip
        }
        self.layer_settling = Some((drag.layer, drag.now));
        self.float_pane(drag.pane_id, Float::Place(drag.now), cx);
        // the backend answers within a few frames; never hold a stale frame longer than a second
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let _ = this.update(cx, |shell, cx| {
                shell.layer_settling = None;
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn float_pane(&mut self, pane_id: String, change: Float, cx: &mut Context<Self>) {
        if crate::mock::dir().is_some() {
            return; // a fixture takes no commands
        }
        let backend = self.backend.clone();
        self.ask(async move { backend.mux.float(pane_id, change).await }, cx, |shell, result, cx| {
            if let Err(err) = result {
                shell.complain(err, cx);
            }
        });
    }

    /// Whether `pane_id` may float out of its tab's split: not the split's last pane, which would leave the tab
    /// nothing under its layers to dock them back into.
    pub(super) fn can_float(&self, pane_id: &str) -> bool {
        let Some(snapshot) = &self.snapshot else { return false };
        let Some(layout) = snapshot.layouts.iter().find(|l| l.panes.iter().any(|p| p.pane_id == pane_id)) else { return false };
        layout.panes.iter().any(|p| p.pane_id != pane_id && p.layer.is_none())
    }

    /// The floating layer `pane_id` is in, if it floats.
    pub(super) fn pane_layer(&self, pane_id: &str) -> Option<String> {
        let layouts = &self.snapshot.as_ref()?.layouts;
        layouts.iter().flat_map(|l| &l.panes).find(|p| p.pane_id == pane_id)?.layer.clone()
    }

    /// A floating pane's grip, which moves its layer, and the handles on its edges and corners, which resize it.
    pub(super) fn layer_handles(&self, f: &PaneFrame, frames: &[PaneFrame], group: &SharedString, theme: &Theme, cx: &Context<Self>) -> Vec<AnyElement> {
        let (Some(layer), Some(from)) = (f.layer.clone(), f.layer.as_deref().and_then(|l| layer_frame(frames, l))) else {
            return Vec::new();
        };
        let start = move |grab: Grab| {
            let (pane_id, layer) = (f.pane_id.clone(), layer.clone());
            cx.listener(move |this: &mut Shell, e: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                this.start_layer_drag(pane_id.clone(), layer.clone(), from, grab, e.position, cx);
            })
        };
        let grip = div().absolute().top_0().left_0().w_full().flex().justify_center().child(
            div()
                .id(SharedString::from(format!("layer-grip-{}", f.pane_id)))
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
                .hover(|s| s.opacity(1.).bg(theme.hover))
                .cursor(CursorStyle::OpenHand)
                .occlude()
                .on_mouse_down(MouseButton::Left, start(Grab::Move))
                .child(
                    gpui::svg()
                        .path("icons/drag-handle.svg")
                        .size(px(12.))
                        .with_transformation(Transformation::rotate(radians(std::f32::consts::FRAC_PI_2)))
                        .text_color(theme.text_muted),
                ),
        );
        // edges first, corners over them: (left, top, right, bottom) each handle moves
        let sides = [
            (true, false, false, false),
            (false, true, false, false),
            (false, false, true, false),
            (false, false, false, true),
            (true, true, false, false),
            (false, true, true, false),
            (true, false, false, true),
            (false, false, true, true),
        ];
        let handles = sides.into_iter().map(|(left, top, right, bottom)| {
            let corner = (left || right) && (top || bottom);
            let cursor = match (left, top, right, bottom) {
                _ if !corner && (left || right) => CursorStyle::ResizeLeftRight,
                _ if !corner => CursorStyle::ResizeUpDown,
                (true, true, ..) | (_, _, true, true) => CursorStyle::ResizeUpLeftDownRight,
                _ => CursorStyle::ResizeUpRightDownLeft,
            };
            let (thick, inset) = (px(if corner { CORNER } else { EDGE }), px(-EDGE / 2.));
            let el = div()
                .id(SharedString::from(format!("layer-edge-{}-{}{}{}{}", f.pane_id, left as u8, top as u8, right as u8, bottom as u8)))
                .absolute()
                .cursor(cursor)
                .occlude()
                .on_mouse_down(MouseButton::Left, start(Grab::Resize { left, top, right, bottom }));
            // along an edge between the corners; a corner square at its corner
            let el = if left { el.left(inset).w(thick) } else if right { el.right(inset).w(thick) } else { el.left(px(CORNER)).right(px(CORNER)) };
            let el = if top { el.top(inset).h(thick) } else if bottom { el.bottom(inset).h(thick) } else { el.top(px(CORNER)).bottom(px(CORNER)) };
            el.into_any_element()
        });
        std::iter::once(grip.into_any_element()).chain(handles).collect()
    }
}

/// The frame around a layer's panes.
pub(super) fn layer_frame(frames: &[PaneFrame], layer: &str) -> Option<Frac> {
    let mut panes = frames.iter().filter(|f| f.layer.as_deref() == Some(layer)).map(|f| f.frame);
    let first = panes.next()?;
    let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x + first.w, first.y + first.h);
    for f in panes {
        (x0, y0, x1, y1) = (x0.min(f.x), y0.min(f.y), x1.max(f.x + f.w), y1.max(f.y + f.h));
    }
    Some(Frac { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
}

/// `frames` with `layer`'s panes moved and scaled from its frame into `to`.
pub(super) fn place_layer(frames: &[PaneFrame], layer: &str, to: Frac) -> Vec<PaneFrame> {
    let Some(from) = layer_frame(frames, layer) else { return frames.to_vec() };
    let (sx, sy) = (to.w / from.w.max(f32::EPSILON), to.h / from.h.max(f32::EPSILON));
    frames
        .iter()
        .map(|f| match f.layer.as_deref() == Some(layer) {
            true => {
                let frame = Frac { x: to.x + (f.frame.x - from.x) * sx, y: to.y + (f.frame.y - from.y) * sy, w: f.frame.w * sx, h: f.frame.h * sy };
                PaneFrame { frame, ..f.clone() }
            }
            false => f.clone(),
        })
        .collect()
}

/// The layer at `from` moved by (`dx`, `dy`) of the tab, or the sides a resize grabbed moved by it; it stays in the
/// tab, no smaller than [`MIN`].
fn dragged(from: Frac, dx: f32, dy: f32, grab: Grab) -> Frac {
    let Grab::Resize { left, top, right, bottom } = grab else {
        return Frac { x: (from.x + dx).clamp(0., 1. - from.w), y: (from.y + dy).clamp(0., 1. - from.h), ..from };
    };
    let (mut x0, mut y0, mut x1, mut y1) = (from.x, from.y, from.x + from.w, from.y + from.h);
    if left {
        x0 = (x0 + dx).clamp(0., x1 - MIN);
    }
    if right {
        x1 = (x1 + dx).clamp(x0 + MIN, 1.);
    }
    if top {
        y0 = (y0 + dy).clamp(0., y1 - MIN);
    }
    if bottom {
        y1 = (y1 + dy).clamp(y0 + MIN, 1.);
    }
    Frac { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

fn close(a: Frac, b: Frac) -> bool {
    [a.x - b.x, a.y - b.y, a.w - b.w, a.h - b.h].iter().all(|d| d.abs() < 0.005)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(id: &str, layer: Option<&str>, frame: Frac) -> PaneFrame {
        PaneFrame { pane_id: id.into(), frame, focused: false, layer: layer.map(str::to_string) }
    }

    #[test]
    fn a_layer_moves_and_resizes_inside_the_tab_with_its_panes() {
        let tiled = frame("t", None, Frac { x: 0., y: 0., w: 1., h: 1. });
        let left = frame("a", Some("l"), Frac { x: 0.2, y: 0.2, w: 0.2, h: 0.4 });
        let right = frame("b", Some("l"), Frac { x: 0.4, y: 0.2, w: 0.2, h: 0.4 });
        let frames = [tiled.clone(), left, right];
        let from = layer_frame(&frames, "l").unwrap();
        assert!(close(from, Frac { x: 0.2, y: 0.2, w: 0.4, h: 0.4 }));

        let corner = Grab::Resize { left: false, top: false, right: true, bottom: true };
        assert!(close(dragged(from, 0.9, -0.5, Grab::Move), Frac { x: 0.6, y: 0., w: 0.4, h: 0.4 }), "kept inside");
        assert!(close(dragged(from, -0.9, 0.1, corner), Frac { x: 0.2, y: 0.2, w: MIN, h: 0.5 }), "no smaller than MIN");
        let top_left = Grab::Resize { left: true, top: true, right: false, bottom: false };
        assert!(close(dragged(from, -0.1, 0.1, top_left), Frac { x: 0.1, y: 0.3, w: 0.5, h: 0.3 }), "the far sides stay put");
        assert!(close(dragged(from, -0.5, -0.5, top_left), Frac { x: 0., y: 0., w: 0.6, h: 0.6 }), "kept inside");
        let left = Grab::Resize { left: true, top: false, right: false, bottom: false };
        assert!(close(dragged(from, 0.9, 0.3, left), Frac { x: 0.6 - MIN, y: 0.2, w: MIN, h: 0.4 }), "an edge moves one side");

        let placed = place_layer(&frames, "l", Frac { x: 0., y: 0., w: 0.8, h: 0.8 });
        assert_eq!(placed[0], tiled, "the split stays");
        assert!(close(placed[2].frame, Frac { x: 0.4, y: 0., w: 0.4, h: 0.8 }), "a pane keeps its place in the layer");
    }
}
