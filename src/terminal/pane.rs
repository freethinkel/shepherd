//! One herdr pane as a GPUI terminal: a `herdr terminal session control` stream fed into Zeron's emulator and
//! painted by its grid element. Keys, wheel and mouse go back to herdr as session commands.
use super::element::{SELECTION_DRAG_THRESHOLD, TERM_LINE_HEIGHT, TerminalElement, cell_at};
use super::emulator::{CellSnapshot, CursorSnapshot, Emulator, Side};
use super::keys::{keystroke_bytes, paste_bytes};
use crate::herdr::{Session, SessionEvent};
use crate::theme::Theme;
use alacritty_terminal::index::Point as GridPoint;
use alacritty_terminal::selection::SelectionType;
use gpui::{
    App, AsyncApp, ClipboardItem, Context, FocusHandle, Focusable, IntoElement, KeyDownEvent, Modifiers,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Render, ScrollDelta,
    ScrollWheelEvent, Styled, Task, WeakEntity, Window, div, prelude::*,
};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

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
    _pump: Option<Task<()>>,
}

impl PaneTerminal {
    pub fn new(pane_id: String, cx: &mut Context<Self>) -> Self {
        let mut this = PaneTerminal {
            pane_id,
            emulator: Emulator::new(80, 24),
            session: None,
            ended: None,
            geometry: None,
            focus: cx.focus_handle(),
            wheel_acc: 0.0,
            drag: None,
            _pump: None,
        };
        this.attach(cx);
        this
    }

    pub fn pane_id(&self) -> &str {
        &self.pane_id
    }

    pub fn ended(&self) -> Option<&str> {
        self.ended.as_deref()
    }

    fn attach(&mut self, cx: &mut Context<Self>) {
        let (cols, rows) = (self.emulator.cols() as u16, self.emulator.rows() as u16);
        match Session::spawn(&self.pane_id, cols, rows) {
            Ok((session, rx)) => {
                self.session = Some(session);
                self.ended = None;
                self._pump = Some(cx.spawn(async move |this, cx| pump(this, rx, cx).await));
            }
            Err(err) => self.ended = Some(format!("{err:#}")),
        }
    }

    pub fn reattach(&mut self, cx: &mut Context<Self>) {
        self.session = None;
        self.attach(cx);
        cx.notify();
    }

    /// Element hook: this frame's grid placement. A size change resizes the emulator and herdr's view.
    pub fn on_grid_metrics(&mut self, geometry: GridGeometry, _cx: &mut Context<Self>) {
        self.geometry = Some(geometry);
        let (cols, rows) = (geometry.cols, geometry.rows);
        if self.emulator.cols() == cols as usize && self.emulator.rows() == rows as usize {
            return;
        }
        self.emulator.resize(cols, rows);
        if let Some(session) = &self.session {
            session.resize(cols, rows);
        }
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

    fn on_scroll_wheel(&mut self, event: &ScrollWheelEvent, _: &mut Window, _: &mut Context<Self>) {
        let line_h = self.geometry.map_or(TERM_LINE_HEIGHT, |g| g.line_h);
        let px = match event.delta {
            ScrollDelta::Pixels(d) => f32::from(d.y),
            ScrollDelta::Lines(d) => d.y * line_h,
        };
        let n = wheel_lines(&mut self.wheel_acc, px, line_h);
        if let (Some(session), true) = (&self.session, n != 0) {
            session.scroll(n > 0, n.unsigned_abs().min(u16::MAX as u32) as u16);
        }
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

/// Moves frames from the session's reader thread into the emulator.
// ponytail: polls every 8 ms when idle; an async channel if input latency ever shows.
async fn pump(this: WeakEntity<PaneTerminal>, rx: Receiver<SessionEvent>, cx: &mut AsyncApp) {
    loop {
        let mut frames = Vec::new();
        let mut ended = None;
        loop {
            match rx.try_recv() {
                Ok(SessionEvent::Frame(bytes)) => frames.push(bytes),
                Ok(SessionEvent::Ended(why)) => {
                    ended = Some(why);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    ended = Some("the session closed".into());
                    break;
                }
            }
        }
        if !frames.is_empty() || ended.is_some() {
            let alive = this.update(cx, |pane, cx| {
                for frame in &frames {
                    // Replies (DSR, DA…) are dropped: frames are herdr's re-render, and herdr already
                    // answered the app's own queries.
                    let _ = pane.emulator.feed(frame);
                }
                if let Some(why) = ended.clone() {
                    pane.ended = Some(why);
                    pane.session = None;
                }
                cx.notify();
            });
            if alive.is_err() || ended.is_some() {
                return;
            }
        }
        cx.background_executor().timer(Duration::from_millis(8)).await;
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
                        .child("Reattach")
                        .on_click(cx.listener(|pane, _, _, cx| pane.reattach(cx))),
                )
                .into_any_element();
        }
        let focused = self.focus.is_focused(window);
        div()
            .id("terminal")
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
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel_lines_accumulate_fractions() {
        let mut acc = 0.0;
        assert_eq!(wheel_lines(&mut acc, 6.0, 17.0), 0); // a trackpad tick under one line
        assert_eq!(wheel_lines(&mut acc, 6.0, 17.0), 0);
        assert_eq!(wheel_lines(&mut acc, 6.0, 17.0), 1); // 18 px ≥ one 17-px line
        assert_eq!(wheel_lines(&mut acc, -40.0, 17.0), -2);
    }
}
