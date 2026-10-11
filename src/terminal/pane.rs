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
    ScrollWheelEvent, SharedString, Styled, Task, WeakEntity, Window, div, prelude::*, px,
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

/// herdr's offset from the bottom for a scroll offset `y` px down from the oldest line (`y` ≤ 0) at `row_px` a row.
pub fn offset_for(s: ScrollState, y: f32, row_px: f32) -> u32 {
    if row_px <= 0.0 {
        return s.offset_from_bottom;
    }
    let above = (-y / row_px).round().clamp(0.0, s.max_offset_from_bottom as f32) as u32;
    s.max_offset_from_bottom - above
}

/// herdr's scrollback as a gpui-base scroll handle, so terminals wear the app's one scrollbar (`ui::scrollbar`): a
/// row is the viewport's height over its rows, and a dragged thumb asks the pane to scroll herdr to its row.
#[derive(Clone)]
struct HerdrScroll(std::rc::Rc<HerdrScrollShared>);

struct HerdrScrollShared {
    state: std::cell::Cell<ScrollState>,
    bounds: std::cell::Cell<gpui::Bounds<Pixels>>,
    wanted: futures::channel::mpsc::UnboundedSender<u32>,
}

impl HerdrScroll {
    fn row_px(&self) -> f32 {
        f32::from(self.0.bounds.get().size.height) / self.0.state.get().viewport_rows.max(1) as f32
    }
}

impl gpui_base::ScrollbarHandle for HerdrScroll {
    fn viewport_bounds(&self) -> gpui::Bounds<Pixels> {
        self.0.bounds.get()
    }

    fn offset(&self) -> gpui::Point<Pixels> {
        let s = self.0.state.get();
        gpui::point(px(0.), px(-(s.max_offset_from_bottom.saturating_sub(s.offset_from_bottom) as f32) * self.row_px()))
    }

    fn set_offset(&self, offset: gpui::Point<Pixels>) {
        let _ = self.0.wanted.unbounded_send(offset_for(self.0.state.get(), f32::from(offset.y), self.row_px()));
    }

    fn content_size(&self) -> gpui::Size<Pixels> {
        let s = self.0.state.get();
        let rows = (s.max_offset_from_bottom + s.viewport_rows.max(1)) as f32;
        gpui::size(self.0.bounds.get().size.width, px(rows * self.row_px()))
    }
}

/// Lines per line-height of wheel/trackpad travel.
const WHEEL_SPEED: f32 = 1.5;

/// After our own scroll, herdr's reported offset lags behind it: the thumb keeps ours this long, then re-syncs.
const SCROLL_SETTLE: std::time::Duration = std::time::Duration::from_millis(250);

/// A left-button press: armed once the pointer travels past the drag threshold.
#[derive(Debug, Clone, Copy)]
struct Drag {
    origin: gpui::Point<Pixels>,
    armed: bool,
    /// Decided at the press, like Ghostty: the gesture goes to the pane's app, or selects here, or (when the
    /// session cannot say whether the app captures the mouse) both.
    to_app: bool,
    select: bool,
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
    /// The pane's app asks for mouse reporting (from herdr; false until it says otherwise).
    mouse_capture: bool,
    scroll: ScrollState,
    /// A `pane.get` is out; `scroll_stale` asks for one more when it lands.
    scroll_fetching: bool,
    scroll_stale: bool,
    scrollbar: HerdrScroll,
    _scrollbar_drag: Task<()>,
    /// Until then `scroll.offset_from_bottom` is our own prediction, not herdr's (stale) report.
    scroll_local_until: Option<std::time::Instant>,
    _scroll_settle: Option<Task<()>>,
    _pump: Option<Task<()>>,
}

impl PaneTerminal {
    pub fn new(pane_id: String, cx: &mut Context<Self>) -> Self {
        // The scrollbar's drags arrive here: the handle has no context to scroll herdr with.
        let (wanted, mut drags) = futures::channel::mpsc::unbounded::<u32>();
        let scrollbar_drag = cx.spawn(async move |this, cx| {
            while let Some(target) = drags.next().await {
                let moved = this.update(cx, |pane, cx| {
                    pane.scroll_by(target as i64 - pane.scroll.offset_from_bottom as i64, cx)
                });
                if moved.is_err() {
                    break;
                }
            }
        });
        let scrollbar = HerdrScroll(std::rc::Rc::new(HerdrScrollShared {
            state: Default::default(),
            bounds: Default::default(),
            wanted,
        }));
        PaneTerminal {
            pane_id,
            emulator: Emulator::new(80, 24),
            session: None,
            ended: None,
            geometry: None,
            focus: cx.focus_handle(),
            wheel_acc: 0.0,
            drag: None,
            mouse_capture: false,
            scroll: ScrollState::default(),
            scroll_fetching: false,
            scroll_stale: false,
            scrollbar,
            _scrollbar_drag: scrollbar_drag,
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
                self.mouse_capture = false;
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
        let Some((_, _, point, side)) = self.hit(event.position) else { return };
        // Ghostty: an app that captures the mouse gets it, Shift takes it back for a selection. A CLI session
        // cannot tell, so it keeps forwarding (herdr drops events the app did not ask for) and selects as well.
        let shift = event.modifiers.shift;
        let known = self.session.as_ref().is_some_and(Session::knows_mouse_capture);
        let (to_app, select) = if known { (self.mouse_capture && !shift, !self.mouse_capture || shift) } else { (!shift, true) };
        if to_app {
            self.mouse("down", event.position, &event.modifiers);
        }
        self.emulator.clear_selection();
        let armed = select && event.click_count > 1;
        if armed {
            let ty = if event.click_count == 2 { SelectionType::Semantic } else { SelectionType::Lines };
            self.emulator.start_selection(ty, point, side);
        }
        self.drag = Some(Drag { origin: event.position, armed, to_app, select });
        cx.notify();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !event.dragging() {
            return;
        }
        let Some(mut drag) = self.drag else { return };
        if drag.to_app {
            self.mouse("drag", event.position, &event.modifiers);
        }
        if !drag.select {
            return;
        }
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
        if self.drag.take().is_some_and(|d| d.to_app) {
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
                    SessionEvent::MouseCapture(enabled) => pane.mouse_capture = enabled,
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
        self.scrollbar.0.state.set(self.scroll);
        if let Some(g) = self.geometry {
            self.scrollbar.0.bounds.set(g.bounds);
        }
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
            .child(crate::ui::scrollbar(SharedString::from(format!("scrollbar-{}", self.pane_id)), &self.scrollbar, &theme))
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
        // No client socket and no binary: the attach fails without touching a real herdr.
        unsafe {
            std::env::set_var("SHEPHERD_HERDR", "/nonexistent/herdr");
            std::env::set_var("HERDR_CLIENT_SOCKET_PATH", "/nonexistent/herdr-client.sock");
        }
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
        // No client socket and no binary: the attach fails without touching a real herdr.
        unsafe {
            std::env::set_var("SHEPHERD_HERDR", "/nonexistent/herdr");
            std::env::set_var("HERDR_CLIENT_SOCKET_PATH", "/nonexistent/herdr-client.sock");
        }
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
    fn the_scrollbar_offset_maps_to_herdrs_rows() {
        // 300 lines of history at 4 px a row: the scroll offset counts down from the oldest line.
        let s = ScrollState { offset_from_bottom: 0, max_offset_from_bottom: 300, viewport_rows: 100 };
        assert_eq!(offset_for(s, 0.0, 4.0), 300); // top: the oldest line
        assert_eq!(offset_for(s, -600.0, 4.0), 150);
        assert_eq!(offset_for(s, -1200.0, 4.0), 0); // bottom: live
        assert_eq!(offset_for(s, -9999.0, 4.0), 0); // clamped
        assert_eq!(offset_for(s, -600.0, 0.0), 0); // no measured grid: stays where it is
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
