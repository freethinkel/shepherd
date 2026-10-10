//! One herdr pane as a GPUI terminal: a `herdr terminal session control` stream fed into Zeron's emulator and
//! painted by its grid element. Keys, wheel and mouse go back to herdr as session commands.
//! `GridGeometry` and the selection gestures are adapted from Zeron's crates/ui/src/terminal/panel.rs (MIT, see
//! assets/NOTICE).
use super::element::{SELECTION_DRAG_THRESHOLD, TERM_LINE_HEIGHT, TerminalElement, cell_at};
use super::emulator::{CellSnapshot, CursorSnapshot, Emulator, Side};
use super::keys::{keystroke_bytes, paste_bytes};
use crate::herdr::{Client, Session, SessionEvent};
use crate::theme::Theme;
use alacritty_terminal::index::Point as GridPoint;
use alacritty_terminal::selection::SelectionType;
use gpui::{
    App, AsyncApp, ClipboardItem, Context, FocusHandle, Focusable, IntoElement, KeyDownEvent, Modifiers,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Render, ScrollDelta,
    ScrollWheelEvent, Styled, Task, WeakEntity, Window, DispatchPhase, canvas, div, prelude::*, px,
};
use futures::StreamExt;
use futures::channel::mpsc::UnboundedReceiver;

/// Where the grid landed this frame, in window coordinates (from Zeron's panel.rs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridGeometry {
    pub bounds: gpui::Bounds<Pixels>,
    /// Top-left of the first glyph (bounds origin plus padding).
    pub origin: gpui::Point<Pixels>,
    pub cell_w: f32,
    pub line_h: f32,
    pub cols: u16,
    pub rows: u16,
}

pub struct GridSnapshot {
    pub lines: Vec<Vec<CellSnapshot>>,
    pub cursor: Option<CursorSnapshot>,
}

/// While true, panes keep their grid size when their bounds change (a divider is being dragged): the box resizes
/// live, the terminal and herdr's PTY resize once when the drag ends.
pub struct GridHold(pub bool);

impl gpui::Global for GridHold {}

/// herdr's socket for per-pane queries (scroll position); absent in tests.
pub struct PaneQueries(pub Client, pub tokio::runtime::Handle);

impl gpui::Global for PaneQueries {}

/// herdr's scrollback position; frames do not carry it, `pane.get` does.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Deserialize)]
pub struct ScrollState {
    pub offset_from_bottom: u32,
    pub max_offset_from_bottom: u32,
    pub viewport_rows: u32,
}

/// The scrollbar thumb's (top, height) in a track `track` px tall; `None` without history.
pub fn thumb(s: ScrollState, track: f32) -> Option<(f32, f32)> {
    if s.max_offset_from_bottom == 0 || track <= 0.0 {
        return None;
    }
    let total = (s.max_offset_from_bottom + s.viewport_rows) as f32;
    let h = (track * s.viewport_rows as f32 / total).clamp(SCROLLBAR_MIN_THUMB.min(track), track);
    let above = s.max_offset_from_bottom.saturating_sub(s.offset_from_bottom) as f32;
    Some(((track - h) * above / s.max_offset_from_bottom as f32, h))
}

const SCROLLBAR_MIN_THUMB: f32 = 24.0;

/// Lines per line-height of wheel/trackpad travel.
const WHEEL_SPEED: f32 = 1.5;

/// After our own scroll, herdr's reported offset lags behind it: the thumb keeps ours this long, then re-syncs.
const SCROLL_SETTLE: std::time::Duration = std::time::Duration::from_millis(250);

/// herdr's offset for a thumb grabbed at `start` and moved `dy` px down a track whose thumb can travel `travel` px.
pub fn dragged_offset(start: u32, max: u32, dy: f32, travel: f32) -> u32 {
    if travel <= 0.0 {
        return start;
    }
    (start as f32 - dy * max as f32 / travel).round().clamp(0.0, max as f32) as u32
}

/// herdr's offset that centers a `thumb_h` px thumb on `y` px down a `track` px track.
pub fn offset_at(y: f32, max: u32, track: f32, thumb_h: f32) -> u32 {
    let travel = track - thumb_h;
    if travel <= 0.0 {
        return 0;
    }
    let top = (y - thumb_h / 2.0).clamp(0.0, travel);
    (max as f32 * (1.0 - top / travel)).round() as u32
}

/// A grabbed scrollbar thumb: where the press started and the offset herdr was last asked for.
#[derive(Debug, Clone, Copy)]
struct ThumbDrag {
    start_y: Pixels,
    start_offset: u32,
    sent: u32,
}

/// A left-button press: armed once the pointer travels past the drag threshold.
#[derive(Debug, Clone, Copy)]
struct Drag {
    origin: gpui::Point<Pixels>,
    armed: bool,
}

pub struct PaneTerminal {
    pane_id: String,
    emulator: Emulator,
    session: Option<Session>,
    ended: Option<String>,
    geometry: Option<GridGeometry>,
    focus: FocusHandle,
    wheel_acc: f32,
    drag: Option<Drag>,
    scroll: ScrollState,
    /// A `pane.get` is out; `scroll_stale` asks for one more when it lands.
    scroll_fetching: bool,
    scroll_stale: bool,
    thumb_drag: Option<ThumbDrag>,
    /// Until then `scroll.offset_from_bottom` is our own prediction, not herdr's (stale) report.
    scroll_local_until: Option<std::time::Instant>,
    _scroll_settle: Option<Task<()>>,
    _pump: Option<Task<()>>,
}

impl PaneTerminal {
    pub fn new(pane_id: String, cx: &mut Context<Self>) -> Self {
        PaneTerminal {
            pane_id,
            emulator: Emulator::new(80, 24),
            session: None,
            ended: None,
            geometry: None,
            focus: cx.focus_handle(),
            wheel_acc: 0.0,
            drag: None,
            scroll: ScrollState::default(),
            scroll_fetching: false,
            scroll_stale: false,
            thumb_drag: None,
            scroll_local_until: None,
            _scroll_settle: None,
            _pump: None,
        }
        // Attaching waits for the first measured grid (`on_grid_metrics`): herdr resizes the real PTY to
        // whatever size the attach names.
    }

    pub fn pane_id(&self) -> &str {
        &self.pane_id
    }

    pub fn ended(&self) -> Option<&str> {
        self.ended.as_deref()
    }

    fn attach(&mut self, takeover: bool, cx: &mut Context<Self>) {
        let (cols, rows) = (self.emulator.cols() as u16, self.emulator.rows() as u16);
        match Session::spawn(&self.pane_id, cols, rows, takeover) {
            Ok((session, rx)) => {
                self.session = Some(session);
                self.ended = None;
                self._pump = Some(cx.spawn(async move |this, cx| pump(this, rx, cx).await));
            }
            Err(err) => self.ended = Some(format!("{err:#}")),
        }
    }

    /// `takeover` replaces another client that controls the pane.
    pub fn reattach(&mut self, takeover: bool, cx: &mut Context<Self>) {
        self.session = None;
        self.attach(takeover, cx);
        cx.notify();
    }

    /// Element hook: this frame's grid placement. A size change resizes the emulator and herdr's view.
    pub fn on_grid_metrics(&mut self, geometry: GridGeometry, cx: &mut Context<Self>) {
        let first = self.geometry.is_none();
        if !first && cx.try_global::<GridHold>().is_some_and(|hold| hold.0) {
            return; // a divider is moving: re-measured when it is let go
        }
        self.geometry = Some(geometry);
        let (cols, rows) = (geometry.cols, geometry.rows);
        let resized = self.emulator.cols() != cols as usize || self.emulator.rows() != rows as usize;
        if resized {
            self.emulator.resize(cols, rows);
        }
        if first && self.session.is_none() && self.ended.is_none() {
            self.attach(false, cx);
        } else if let (Some(session), true) = (&self.session, resized) {
            session.resize(cols, rows);
        }
    }

    /// Ask herdr where its view sits in the scrollback. One request at a time; a burst of frames costs one more.
    fn refresh_scroll(&mut self, cx: &mut Context<Self>) {
        let Some(PaneQueries(client, rt)) = cx.try_global::<PaneQueries>() else { return };
        if self.scroll_fetching {
            self.scroll_stale = true;
            return;
        }
        self.scroll_fetching = true;
        let client = client.clone();
        let pane_id = self.pane_id.clone();
        let call = rt.spawn(async move { client.call("pane.get", serde_json::json!({"pane_id": pane_id})).await });
        cx.spawn(async move |this, cx| {
            let scroll: Option<ScrollState> = call.await.ok().and_then(Result::ok).and_then(|r| serde_json::from_value(r["pane"]["scroll"].clone()).ok());
            let _ = this.update(cx, |pane, cx| {
                pane.scroll_fetching = false;
                if let Some(mut scroll) = scroll {
                    if pane.scroll_local_until.is_some_and(|t| std::time::Instant::now() < t) {
                        scroll.offset_from_bottom = pane.scroll.offset_from_bottom.min(scroll.max_offset_from_bottom);
                    }
                    if scroll != pane.scroll {
                        pane.scroll = scroll;
                        cx.notify();
                    }
                }
                if std::mem::take(&mut pane.scroll_stale) {
                    pane.refresh_scroll(cx);
                }
            });
        })
        .detach();
    }

    /// Scroll herdr by `delta` lines (positive = up, into history) and move the thumb now, not a round trip later.
    fn scroll_by(&mut self, delta: i64, cx: &mut Context<Self>) {
        let Some(session) = &self.session else { return };
        if delta == 0 {
            return;
        }
        session.scroll(delta > 0, delta.unsigned_abs().min(u16::MAX as u64) as u16);
        let max = self.scroll.max_offset_from_bottom as i64;
        self.scroll.offset_from_bottom = (self.scroll.offset_from_bottom as i64 + delta).clamp(0, max) as u32;
        self.scroll_local_until = Some(std::time::Instant::now() + SCROLL_SETTLE);
        self._scroll_settle = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SCROLL_SETTLE + std::time::Duration::from_millis(20)).await;
            let _ = this.update(cx, |pane, cx| pane.refresh_scroll(cx));
        }));
        cx.notify();
    }

    pub fn active_grid_snapshot(&self, _cx: &App) -> Option<GridSnapshot> {
        Some(GridSnapshot { lines: self.emulator.lines(), cursor: self.emulator.cursor() })
    }

    fn input(&self, bytes: &[u8]) {
        if let Some(session) = &self.session {
            session.input(bytes);
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let ks = &event.keystroke;
        let mods = &ks.modifiers;
        if ks.key == "v" && mods.platform {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                // ponytail: always bracketed; herdr's frames do not tell us whether the app asked for it.
                self.input(&paste_bytes(&text, true));
            }
            cx.stop_propagation();
            return;
        }
        if ks.key == "c" && mods.platform {
            if let Some(text) = self.emulator.selection_text() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                cx.stop_propagation();
            }
            return;
        }
        if let Some(bytes) = keystroke_bytes(&ks.key, ks.key_char.as_deref(), mods, false) {
            self.input(&bytes);
            cx.stop_propagation();
        }
    }

    fn on_scroll_wheel(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let line_h = self.geometry.map_or(TERM_LINE_HEIGHT, |g| g.line_h);
        let px = match event.delta {
            ScrollDelta::Pixels(d) => f32::from(d.y),
            ScrollDelta::Lines(d) => d.y * line_h,
        };
        let n = wheel_lines(&mut self.wheel_acc, px * WHEEL_SPEED, line_h);
        self.scroll_by(n as i64, cx);
    }

    fn on_thumb_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        let offset = self.scroll.offset_from_bottom;
        self.thumb_drag = Some(ThumbDrag { start_y: event.position.y, start_offset: offset, sent: offset });
        cx.stop_propagation(); // not a selection, not a click for herdr
        cx.notify();
    }

    /// A press on the track jumps the thumb under the pointer, then drags it from there.
    fn on_track_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        cx.stop_propagation();
        let Some(bounds) = self.geometry.map(|g| g.bounds) else { return };
        let track = f32::from(bounds.size.height);
        let Some((_, h)) = thumb(self.scroll, track) else { return };
        let target = offset_at(f32::from(event.position.y - bounds.top()), self.scroll.max_offset_from_bottom, track, h);
        self.scroll_by(target as i64 - self.scroll.offset_from_bottom as i64, cx);
        self.thumb_drag = Some(ThumbDrag { start_y: event.position.y, start_offset: target, sent: target });
        cx.notify();
    }

    /// Scrolls herdr by the difference between the thumb's new offset and the one asked for last.
    fn on_thumb_move(&mut self, y: Pixels, cx: &mut Context<Self>) {
        let Some(drag) = self.thumb_drag.as_mut() else { return };
        let track = self.geometry.map_or(0.0, |g| f32::from(g.bounds.size.height));
        let Some((_, h)) = thumb(self.scroll, track) else { return };
        let target = dragged_offset(drag.start_offset, self.scroll.max_offset_from_bottom, f32::from(y - drag.start_y), track - h);
        let delta = target as i64 - drag.sent as i64;
        drag.sent = target;
        self.scroll_by(delta, cx);
    }

    /// Window position → (viewport row, col) and the grid point/side for selections.
    fn hit(&self, position: gpui::Point<Pixels>) -> Option<(u16, u16, GridPoint, Side)> {
        let g = self.geometry?;
        let hit = cell_at(
            f32::from(position.x - g.origin.x),
            f32::from(position.y - g.origin.y),
            g.cell_w,
            g.line_h,
            g.cols as usize,
            g.rows as usize,
        );
        let point = self.emulator.grid_point(hit.row, hit.col);
        Some((hit.row as u16, hit.col as u16, point, hit.side))
    }

    fn mouse(&self, action: &str, position: gpui::Point<Pixels>, mods: &Modifiers) {
        if let (Some(session), Some((row, col, _, _))) = (&self.session, self.hit(position)) {
            session.mouse(action, "left", col, row, mouse_mods(mods));
        }
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        self.mouse("down", event.position, &event.modifiers);
        let Some((_, _, point, side)) = self.hit(event.position) else { return };
        match event.click_count {
            1 => {
                self.emulator.clear_selection();
                self.drag = Some(Drag { origin: event.position, armed: false });
            }
            n => {
                let ty = if n == 2 { SelectionType::Semantic } else { SelectionType::Lines };
                self.emulator.start_selection(ty, point, side);
                self.drag = Some(Drag { origin: event.position, armed: true });
            }
        }
        cx.notify();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !event.dragging() {
            return;
        }
        let Some(mut drag) = self.drag else { return };
        self.mouse("drag", event.position, &event.modifiers);
        if !drag.armed {
            let d = event.position - drag.origin;
            if f32::from(d.x).hypot(f32::from(d.y)) < SELECTION_DRAG_THRESHOLD {
                return;
            }
            let Some((_, _, anchor, side)) = self.hit(drag.origin) else { return };
            self.emulator.start_selection(SelectionType::Simple, anchor, side);
            drag.armed = true;
            self.drag = Some(drag);
        }
        if let Some((_, _, point, side)) = self.hit(event.position) {
            self.emulator.update_selection(point, side);
            cx.notify();
        }
    }

    fn on_mouse_up(&mut self, event: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        if self.drag.take().is_some() {
            self.mouse("up", event.position, &event.modifiers);
        }
    }
}

/// herdr allows one controller per terminal; its refusal names the flag that replaces the other one.
pub fn needs_takeover(reason: &str) -> bool {
    reason.contains("already has an attached client")
}

/// herdr's mouse modifier bits: Shift = 1, Ctrl = 2, Alt = 4.
fn mouse_mods(m: &Modifiers) -> u8 {
    u8::from(m.shift) | (u8::from(m.control) << 1) | (u8::from(m.alt) << 2)
}

/// Wheel/trackpad pixels → whole lines to scroll herdr by (positive = up, into history); the remainder is kept
/// so slow trackpad swipes still add up.
pub fn wheel_lines(acc: &mut f32, delta_px: f32, line_h: f32) -> i32 {
    if line_h <= 0.0 {
        return 0;
    }
    *acc += delta_px / line_h;
    let n = acc.trunc();
    *acc -= n;
    n as i32
}

/// Moves frames from the session's reader thread into the emulator. Wakes only when herdr sends something;
/// a burst of frames is applied in one update and one repaint.
async fn pump(this: WeakEntity<PaneTerminal>, mut rx: UnboundedReceiver<SessionEvent>, cx: &mut AsyncApp) {
    while let Some(first) = rx.next().await {
        let mut events = vec![first];
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        let mut ended = false;
        let alive = this.update(cx, |pane, cx| {
            for event in events {
                match event {
                    // Replies (DSR, DA…) are dropped: frames are herdr's re-render, and herdr already
                    // answered the app's own queries.
                    SessionEvent::Frame(bytes) => drop(pane.emulator.feed(&bytes)),
                    SessionEvent::Ended(why) => {
                        pane.ended = Some(why);
                        pane.session = None;
                        ended = true;
                        break;
                    }
                }
            }
            if !ended {
                pane.refresh_scroll(cx);
            }
            cx.notify();
        });
        if alive.is_err() || ended {
            return;
        }
    }
}

impl Focusable for PaneTerminal {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for PaneTerminal {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        if let Some(why) = self.ended.clone() {
            let takeover = needs_takeover(&why);
            return div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_3()
                .bg(theme.terminal.background)
                .text_color(theme.text_muted)
                .text_sm()
                .child(why)
                .child(
                    div()
                        .id("reattach")
                        .px_3()
                        .py_1()
                        .rounded_md()
                        .bg(theme.hover)
                        .text_color(theme.text)
                        .cursor_pointer()
                        .child(if takeover { "Take over" } else { "Reattach" })
                        .on_click(cx.listener(move |pane, _, _, cx| pane.reattach(takeover, cx))),
                )
                .into_any_element();
        }
        let focused = self.focus.is_focused(window);
        let track = self.geometry.map_or(0.0, |g| f32::from(g.bounds.size.height));
        let dragging = self.thumb_drag.is_some();
        // The hit zone is wider than the 6 px bar; the thumb's own press stops before the track's.
        let scrollbar = thumb(self.scroll, track).map(|(top, h)| {
            div()
                .id("scroll-track")
                .absolute()
                .top_0()
                .right_0()
                .w(px(12.))
                .h_full()
                .on_mouse_down(MouseButton::Left, cx.listener(Self::on_track_down))
                .child(
                    div()
                        .id("scroll-thumb")
                        .absolute()
                        .top(px(top))
                        .right_0()
                        .w_full()
                        .h(px(h))
                        .flex()
                        .justify_end()
                        .pr(px(2.))
                        .on_mouse_down(MouseButton::Left, cx.listener(Self::on_thumb_down))
                        .child(div().w(px(6.)).h_full().rounded_full().bg(theme.text_muted.opacity(if dragging { 0.6 } else { 0.35 }))),
                )
        });
        // While grabbed, the thumb follows the pointer anywhere in the window, not only over this pane.
        let thumb_tracking = dragging.then(|| {
            let this = cx.entity();
            canvas(|_, _, _| {}, move |_, _, window, _| {
                let moved = this.clone();
                window.on_mouse_event(move |e: &MouseMoveEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble {
                        moved.update(cx, |pane, cx| pane.on_thumb_move(e.position.y, cx));
                    }
                });
                window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble {
                        this.update(cx, |pane, cx| {
                            pane.thumb_drag = None;
                            cx.notify();
                        });
                    }
                });
            })
            .absolute()
            .size_0()
        });
        div()
            .id("terminal")
            .relative()
            .size_full()
            .bg(theme.terminal.background)
            .key_context("Terminal")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            .child(TerminalElement::new(cx.entity(), focused))
            .children(scrollbar)
            .children(thumb_tracking)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry(cols: u16, rows: u16) -> GridGeometry {
        GridGeometry {
            bounds: gpui::Bounds::default(),
            origin: gpui::Point::default(),
            cell_w: 8.0,
            line_h: 17.0,
            cols,
            rows,
        }
    }

    #[gpui::test]
    fn waits_for_its_size_before_attaching(cx: &mut gpui::TestAppContext) {
        // Attaching resizes herdr's real PTY: a placeholder 80×24 first would SIGWINCH every agent twice.
        unsafe { std::env::set_var("SHEPHERD_HERDR", "/nonexistent/herdr") };
        let pane = cx.new(|cx| PaneTerminal::new("w1:p1".into(), cx));
        pane.read_with(cx, |p, _| {
            assert!(p.session.is_none() && p.ended.is_none(), "attached before its size: {:?}", p.ended);
        });
        pane.update(cx, |p, cx| p.on_grid_metrics(geometry(120, 40), cx));
        pane.read_with(cx, |p, _| {
            assert_eq!((p.emulator.cols(), p.emulator.rows()), (120, 40));
            // The attach ran (at 120×40) and failed on the missing binary.
            assert!(p.ended.as_deref().is_some_and(|e| e.contains("/nonexistent/herdr")), "{:?}", p.ended);
        });
    }

    #[gpui::test]
    fn the_grid_holds_its_size_while_a_divider_moves(cx: &mut gpui::TestAppContext) {
        // Panes resize live under a divider drag; herdr's PTYs resize once, when the drag ends.
        unsafe { std::env::set_var("SHEPHERD_HERDR", "/nonexistent/herdr") };
        let pane = cx.new(|cx| PaneTerminal::new("w1:p1".into(), cx));
        pane.update(cx, |p, cx| p.on_grid_metrics(geometry(120, 40), cx));
        cx.update(|cx| cx.set_global(GridHold(true)));
        pane.update(cx, |p, cx| p.on_grid_metrics(geometry(90, 40), cx));
        pane.read_with(cx, |p, _| assert_eq!(p.emulator.cols(), 120, "resized during the drag"));
        cx.update(|cx| cx.set_global(GridHold(false)));
        pane.update(cx, |p, cx| p.on_grid_metrics(geometry(90, 40), cx));
        pane.read_with(cx, |p, _| assert_eq!(p.emulator.cols(), 90));
    }

    #[test]
    fn a_refused_attach_offers_takeover() {
        assert!(needs_takeover("herdr: terminal w1:p1 already has an attached client; retry with --takeover"));
        assert!(!needs_takeover("pane closed"));
    }

    #[test]
    fn the_thumb_tracks_herdrs_offset() {
        let s = |offset, max| ScrollState { offset_from_bottom: offset, max_offset_from_bottom: max, viewport_rows: 100 };
        assert_eq!(thumb(s(0, 0), 400.0), None); // no history, no bar
        assert_eq!(thumb(s(0, 300), 400.0), Some((300.0, 100.0))); // live: at the bottom, a quarter tall
        assert_eq!(thumb(s(300, 300), 400.0), Some((0.0, 100.0))); // oldest line: at the top
        assert_eq!(thumb(s(0, 100_000), 400.0), Some((376.0, 24.0))); // long history keeps a grabbable thumb
    }

    #[test]
    fn dragging_the_thumb_moves_through_history() {
        assert_eq!(dragged_offset(0, 300, -150.0, 300.0), 150); // half the travel up: half the history
        assert_eq!(dragged_offset(150, 300, 150.0, 300.0), 0); // back down to live
        assert_eq!(dragged_offset(0, 300, -1000.0, 300.0), 300); // clamped at the oldest line
        assert_eq!(dragged_offset(10, 300, 5.0, 0.0), 10); // a thumb as tall as the track cannot move
    }

    #[test]
    fn a_track_click_centers_the_thumb_on_it() {
        // 400 px track, 100 px thumb: 300 px of travel over 300 lines of history.
        assert_eq!(offset_at(50.0, 300, 400.0, 100.0), 300); // near the top: oldest line
        assert_eq!(offset_at(200.0, 300, 400.0, 100.0), 150); // middle
        assert_eq!(offset_at(390.0, 300, 400.0, 100.0), 0); // near the bottom: live
    }

    #[test]
    fn wheel_lines_accumulate_fractions() {
        let mut acc = 0.0;
        assert_eq!(wheel_lines(&mut acc, 6.0, 17.0), 0); // a trackpad tick under one line
        assert_eq!(wheel_lines(&mut acc, 6.0, 17.0), 0);
        assert_eq!(wheel_lines(&mut acc, 6.0, 17.0), 1); // 18 px ≥ one 17-px line
        assert_eq!(wheel_lines(&mut acc, -40.0, 17.0), -2);
    }
}
