//! Panes that float over a tab's split (Rex's floating layers): moved by their grip, resized by their edges and corners,
//! raised by a click, floated out of the split and docked back from the pane's menu.
use super::*;
use crate::backend::Float;
use crate::model::Frac;
use gpui::{AnyElement, Bounds, CursorStyle, Transformation, radians};

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

/// How long a layer's spring into its button at the tab's foot, or back out of it, plays (Rex's capsule): it's most
/// of the way in the first quarter, then settles a little past the end.
const MORPH: Duration = Duration::from_millis(260);
/// How far a minimized layer's button sits in from the tab's bottom-right corner.
pub(super) const PILL_FOOT: f32 = 10.;
const PILL_H: f32 = super::tabs::CHIP_H;

/// A floating layer shrinking into its button at the tab's foot, or growing back out of it: a bare card drawn
/// between the two while the layer and the button are hidden.
pub(super) struct Morph {
    /// The layer's panes; the first is the one its button restores.
    panes: Vec<String>,
    layer: Option<String>,
    /// The layer's frame in the tab: known on the way down, found once the layer shows again on the way up.
    card: Option<Frac>,
    /// The button, in the window, while it was drawn.
    pill: Option<Bounds<Pixels>>,
    down: bool,
    asked: Instant,
    /// Playing since: at once on the way down, when the backend shows the layer again on the way up.
    started: Option<Instant>,
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

    /// Rex's Float Pane: the focused pane out of the split, or a floating one back into it.
    pub(super) fn toggle_float(&mut self, window: &Window, cx: &mut Context<Self>) {
        let Some(pane_id) = self.keyed_pane(window, cx) else { return };
        let change = match self.pane_layer(&pane_id) {
            Some(_) => Float::Dock,
            None if self.can_float(&pane_id) => Float::Out,
            None => return,
        };
        self.float_pane(pane_id, change, cx);
    }

    /// Rex's Minimize Floating Window: the focused floating pane's layer down to its button, else the front button's
    /// layer back. `all`, Minimize Floating Windows: every layer of the tab down, or with none up, every one back.
    pub(super) fn toggle_minimize(&mut self, all: bool, window: &Window, cx: &mut Context<Self>) {
        let Some(focused) = self.keyed_pane(window, cx) else { return };
        let (Some(snapshot), Some(tab)) = (&self.snapshot, self.shown_tab()) else { return };
        let Some(layout) = snapshot.layouts.iter().find(|l| l.tab_id == tab.tab_id) else { return };
        let up: Vec<String> = layout.minimized.iter().map(|m| m.pane_id.clone()).take(if all { usize::MAX } else { 1 }).collect();
        let frames = snapshot.pane_frames(&tab.tab_id);
        let mut down: Vec<String> = Vec::new();
        for f in &frames {
            if let Some(layer) = f.layer.as_ref().filter(|l| !down.contains(l) && (all || f.pane_id == focused)) {
                down.push(layer.clone());
            }
        }
        if down.is_empty() {
            // back to front: each one shown comes in front
            for pane_id in up.into_iter().rev() {
                self.restore_layer(pane_id, cx);
            }
        }
        for layer in down {
            self.minimize_layer(&frames, layer, cx);
        }
    }

    /// The floating layer `pane_id` is in down to its button at the tab's foot, shrunk into it.
    pub(super) fn minimize_pane(&mut self, pane_id: &str, cx: &mut Context<Self>) {
        let Some(tab) = self.snapshot.as_ref().and_then(|s| s.layouts.iter().find(|l| l.panes.iter().any(|p| p.pane_id == pane_id))) else { return };
        let frames = self.snapshot.as_ref().map(|s| s.pane_frames(&tab.tab_id)).unwrap_or_default();
        if let Some(layer) = self.pane_layer(pane_id) {
            self.minimize_layer(&frames, layer, cx);
        }
    }

    fn minimize_layer(&mut self, frames: &[PaneFrame], layer: String, cx: &mut Context<Self>) {
        let panes: Vec<String> = frames.iter().filter(|f| f.layer.as_ref() == Some(&layer)).map(|f| f.pane_id.clone()).collect();
        let (Some(pane_id), Some(card)) = (panes.first().cloned(), layer_frame(frames, &layer)) else { return };
        let now = Instant::now();
        self.morphs.push(Morph { panes, layer: Some(layer), card: Some(card), pill: None, down: true, asked: now, started: Some(now) });
        self.float_pane(pane_id, Float::Minimize, cx);
    }

    /// The minimized layer `pane_id` is in back where it was, grown out of its button.
    pub(super) fn restore_layer(&mut self, pane_id: String, cx: &mut Context<Self>) {
        let pill = self.pill_bounds.borrow().get(&pane_id).copied();
        let panes = vec![pane_id.clone()];
        self.morphs.push(Morph { panes, layer: None, card: None, pill, down: false, asked: Instant::now(), started: None });
        self.float_pane(pane_id, Float::Restore, cx);
    }

    /// The pane the keys are in, unless they are in a page or the right panel.
    fn keyed_pane(&self, window: &Window, cx: &App) -> Option<String> {
        (self.pages.shown.is_none() && !self.panel_focused()).then(|| self.active_pane(window, cx)).flatten()
    }

    /// The morphs playing over `frames`: the layers and the buttons (by pane) to hide meanwhile, and the cards to draw.
    pub(super) fn morph_cards(&mut self, frames: &[PaneFrame], theme: &Theme, window: &mut Window, cx: &mut Context<Self>) -> (Vec<String>, Vec<String>, Vec<AnyElement>) {
        let now = Instant::now();
        for m in self.morphs.iter_mut().filter(|m| m.started.is_none()) {
            // the backend shows the layer again: it grows from the button to where it is
            if let Some(layer) = frames.iter().find(|f| f.pane_id == m.panes[0]).and_then(|f| f.layer.clone()) {
                m.card = layer_frame(frames, &layer);
                m.panes = frames.iter().filter(|f| f.layer.as_ref() == Some(&layer)).map(|f| f.pane_id.clone()).collect();
                m.layer = Some(layer);
                m.started = Some(now);
            }
        }
        // a backend that never answers leaves no card behind
        let (playing, done): (Vec<Morph>, Vec<Morph>) = std::mem::take(&mut self.morphs).into_iter().partition(|m| match m.started {
            Some(started) => started.elapsed() < MORPH,
            None => m.asked.elapsed() < Duration::from_secs(1),
        });
        self.morphs = playing;
        // a pane hidden while its card played draws its grid afresh, not the empty frame it last cached
        for pane in done.iter().flat_map(|m| &m.panes).filter_map(|id| self.panes.get(id)) {
            pane.update(cx, |_, cx| cx.notify());
        }
        let (Some(grid), false) = (self.grid_bounds.get(), self.morphs.is_empty()) else { return Default::default() };
        window.request_animation_frame();
        let pills = self.pill_bounds.borrow();
        let (mut layers, mut buttons, mut cards) = (Vec::new(), Vec::new(), Vec::new());
        for m in &self.morphs {
            layers.extend(m.layer.clone());
            buttons.extend(m.panes.iter().cloned());
            // a button not drawn yet goes in the tab's bottom-right corner
            let pill = m.pill.or_else(|| m.panes.iter().find_map(|p| pills.get(p).copied())).unwrap_or_else(|| {
                let (w, h) = (px(160.), px(PILL_H));
                let origin = gpui::point(grid.right() - px(PILL_FOOT) - w, grid.bottom() - px(PILL_FOOT) - h);
                Bounds { origin, size: gpui::size(w, h) }
            });
            let pill = Bounds { origin: pill.origin - grid.origin, ..pill };
            let card = m.card.map_or(pill, |c| Bounds {
                origin: gpui::point(grid.size.width * c.x, grid.size.height * c.y),
                size: gpui::size(grid.size.width * c.w, grid.size.height * c.h),
            });
            // how far toward the button: all of it until the layer shows on the way up
            let t = m.started.map_or(1., |s| super::motion::damped_spring(0.72, s.elapsed().as_secs_f32() / MORPH.as_secs_f32()));
            // past 1 or below 0 while the spring overshoots: the size goes past, the corners and color don't
            let p = if m.down { t } else { 1. - t };
            let mix = |a: Pixels, b: Pixels| (a + (b - a) * p).max(px(0.));
            let shape = p.clamp(0., 1.);
            let radius = crate::terminal::pane::CARD_RADIUS + (PILL_H / 2. - crate::terminal::pane::CARD_RADIUS) * shape;
            cards.push(
                div()
                    .absolute()
                    .left(mix(card.origin.x, pill.origin.x))
                    .top(mix(card.origin.y, pill.origin.y))
                    .w(mix(card.size.width, pill.size.width))
                    .h(mix(card.size.height, pill.size.height))
                    .rounded(px(radius))
                    .bg(theme.terminal.background.blend(theme.card.opacity(shape)))
                    .border_1()
                    .border_color(theme.border_strong)
                    .shadow_lg()
                    .into_any_element(),
            );
        }
        (layers, buttons, cards)
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
