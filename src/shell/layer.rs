//! Panes that float over a tab's split (Rex's floating layers): moved by their grip, resized by their corner,
//! raised by a click, floated out of the split and docked back from the pane's menu.
use super::*;
use crate::backend::Float;
use crate::model::Frac;
use gpui::{AnyElement, CursorStyle, Transformation, radians};

/// The smallest a floating layer is resized to, as a fraction of the tab.
const MIN: f32 = 0.1;
/// The resize corner's side.
const CORNER: f32 = 14.;

/// A floating layer being moved, or resized by its corner.
pub(super) struct LayerDrag {
    pane_id: String,
    layer: String,
    from: Frac,
    now: Frac,
    start: gpui::Point<Pixels>,
    resize: bool,
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

    fn start_layer_drag(&mut self, pane_id: String, layer: String, from: Frac, resize: bool, start: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        cx.set_global(GridHold(true));
        self.layer_drag = Some(LayerDrag { pane_id, layer, from, now: from, start, resize });
        cx.notify();
    }

    pub(super) fn drag_layer(&mut self, position: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        let (Some(drag), Some(grid)) = (&mut self.layer_drag, self.grid_bounds.get()) else { return };
        let dx = f32::from(position.x - drag.start.x) / f32::from(grid.size.width).max(1.);
        let dy = f32::from(position.y - drag.start.y) / f32::from(grid.size.height).max(1.);
        drag.now = dragged(drag.from, dx, dy, drag.resize);
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

    /// The floating layer `pane_id` is in, if it floats.
    pub(super) fn pane_layer(&self, pane_id: &str) -> Option<String> {
        let layouts = &self.snapshot.as_ref()?.layouts;
        layouts.iter().flat_map(|l| &l.panes).find(|p| p.pane_id == pane_id)?.layer.clone()
    }

    /// A floating pane's grip, which moves its layer, and its corner, which resizes it.
    pub(super) fn layer_handles(&self, f: &PaneFrame, frames: &[PaneFrame], group: &SharedString, theme: &Theme, cx: &Context<Self>) -> [AnyElement; 2] {
        let (Some(layer), Some(from)) = (f.layer.clone(), f.layer.as_deref().and_then(|l| layer_frame(frames, l))) else {
            return [div().into_any_element(), div().into_any_element()];
        };
        let start = move |resize: bool| {
            let (pane_id, layer) = (f.pane_id.clone(), layer.clone());
            cx.listener(move |this: &mut Shell, e: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                this.start_layer_drag(pane_id.clone(), layer.clone(), from, resize, e.position, cx);
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
                .on_mouse_down(MouseButton::Left, start(false))
                .child(
                    gpui::svg()
                        .path("icons/drag-handle.svg")
                        .size(px(12.))
                        .with_transformation(Transformation::rotate(radians(std::f32::consts::FRAC_PI_2)))
                        .text_color(theme.text_muted),
                ),
        );
        let corner = div()
            .id(SharedString::from(format!("layer-corner-{}", f.pane_id)))
            .absolute()
            .bottom_0()
            .right_0()
            .size(px(CORNER))
            .cursor(CursorStyle::ResizeUpLeftDownRight)
            .occlude()
            .on_mouse_down(MouseButton::Left, start(true));
        [grip.into_any_element(), corner.into_any_element()]
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

/// The layer at `from` moved by (`dx`, `dy`) of the tab, or resized by its bottom-right corner; it stays in the tab.
fn dragged(from: Frac, dx: f32, dy: f32, resize: bool) -> Frac {
    if resize {
        let (w, h) = ((from.w + dx).clamp(MIN, 1. - from.x), (from.h + dy).clamp(MIN, 1. - from.y));
        Frac { w, h, ..from }
    } else {
        Frac { x: (from.x + dx).clamp(0., 1. - from.w), y: (from.y + dy).clamp(0., 1. - from.h), ..from }
    }
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

        assert!(close(dragged(from, 0.9, -0.5, false), Frac { x: 0.6, y: 0., w: 0.4, h: 0.4 }), "kept inside");
        assert!(close(dragged(from, -0.9, 0.1, true), Frac { x: 0.2, y: 0.2, w: MIN, h: 0.5 }), "no smaller than MIN");

        let placed = place_layer(&frames, "l", Frac { x: 0., y: 0., w: 0.8, h: 0.8 });
        assert_eq!(placed[0], tiled, "the split stays");
        assert!(close(placed[2].frame, Frac { x: 0.4, y: 0., w: 0.4, h: 0.8 }), "a pane keeps its place in the layer");
    }
}
