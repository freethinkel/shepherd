//! One herdr pane as a GPUI terminal: a `herdr terminal session control` stream fed into Zeron's emulator and
//! painted by its grid element. Keys, wheel and mouse go back to herdr as session commands.
//! `GridGeometry` and the selection gestures are adapted from Zeron's crates/ui/src/terminal/panel.rs (MIT, see
//! assets/NOTICE).
use super::element::{SELECTION_DRAG_THRESHOLD, TERM_LINE_HEIGHT, TerminalElement, cell_at};
use super::emulator::{CellSnapshot, CursorSnapshot, Emulator, GridPoint, SelectionType, Side};
use super::find::{Link, find_all, hyperlink, link_at};
use super::keys::{app_shortcut, dropped_paths, keystroke_bytes, kitty_enter, link_modifier, paste_bytes, search_back};
use crate::herdr::graphics::PaneImage;
use crate::herdr::{Client, Mode, PaneInput, Session, SessionEvent};
use crate::theme::Theme;
use crate::ui::Popup;
use gpui::{
    App, AsyncApp, ClipboardItem, Context, Entity, ExternalPaths, FocusHandle, Focusable, IntoElement, KeyDownEvent, Modifiers,
    ModifiersChangedEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Render,
    ScrollDelta, ScrollWheelEvent, SharedString, Styled, Subscription, Task, WeakEntity, Window, div, prelude::*, px,
};
use gpui_base::input::{Input, InputEvent, InputState};
use std::ops::Range;
use std::path::{Path, PathBuf};
use futures::StreamExt;
use futures::channel::mpsc::UnboundedReceiver;

mod open;
mod search;
pub use open::is_nvim;
use open::*;
use search::*;

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
    /// A cell in device pixels, as herdr is told: it sizes Kitty images by it.
    pub cell_px: (u32, u32),
}

pub struct GridSnapshot {
    pub lines: Vec<Vec<CellSnapshot>>,
    pub cursor: Option<CursorSnapshot>,
    /// Search matches on screen: (row, cells, the current one).
    pub marks: Vec<(usize, Range<usize>, bool)>,
    pub images: Vec<PaneImage>,
}

/// While true (a divider or the sidebar is being dragged), panes reflow at most every `HELD_RESIZE`, so the
/// terminal follows the drag without a SIGWINCH per pixel; the drag's end re-measures them all.
pub struct GridHold(pub bool);

/// herdr draws a resized grid within a millisecond, and a shell, vim or Claude Code redraws in ~5–15 ms
/// (`client_shell::input_paths::resize_latency`): two frames at 60 Hz keeps up with a drag.
const HELD_RESIZE: std::time::Duration = std::time::Duration::from_millis(33);

impl gpui::Global for GridHold {}

/// While true, Shepherd only observes its panes and leaves their size to herdr's other clients (its TUI in a
/// terminal or over ssh). herdr gives one client a pane's size; the one used last takes it.
pub struct Yielded(pub bool);

impl gpui::Global for Yielded {}

fn yielded(cx: &App) -> bool {
    cx.try_global::<Yielded>().is_some_and(|y| y.0)
}

/// Shepherd is used again: it takes its panes' size back.
pub fn claim(cx: &mut App) {
    if yielded(cx) {
        cx.set_global(Yielded(false));
    }
}

/// The window's popup watch, which also carries typing to the panes it shows (see [`Surface::type_into`]).
///
/// [`Surface::type_into`]: crate::herdr::popup::Surface::type_into
pub struct TypingShell(pub std::sync::Arc<crate::herdr::popup::Surface>);

impl gpui::Global for TypingShell {}

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

/// herdr's line `line` (counted from the oldest) brought to mid-screen, as an offset from the bottom; `None` when it
/// is on screen already.
pub fn reveal_offset(s: ScrollState, line: usize, rows: usize) -> Option<u32> {
    let top = s.max_offset_from_bottom as i64 - s.offset_from_bottom as i64;
    let line = line as i64;
    if (top..top + rows as i64).contains(&line) {
        return None;
    }
    Some((s.max_offset_from_bottom as i64 + rows as i64 / 2 - line).clamp(0, s.max_offset_from_bottom as i64) as u32)
}

/// Lines per line-height of wheel/trackpad travel.
const WHEEL_SPEED: f32 = 1.5;
/// How much faster the wheel scrolls with Option held (iTerm's and VS Code's fast scroll).
const FAST_WHEEL: f32 = 5.0;

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
    button: MouseButton,
}

pub struct PaneTerminal {
    pane_id: String,
    /// What is shown: the live grid, or while scrolled back, a local copy of herdr's history.
    emulator: Emulator,
    /// While `emulator` shows the history copy: the live grid, which herdr's frames keep updating.
    live: Option<Emulator>,
    /// The lines scrolled while the history is being read.
    history_pending: Option<i64>,
    session: Option<Session>,
    /// The session (or the one connecting) only observes: see [`Yielded`].
    observing: bool,
    /// Typed while the pane was observed or connecting; sent once it is controlled.
    held: Vec<u8>,
    ended: Option<String>,
    /// When the pane was made or last began attaching: a loader covers it once the wait goes on.
    attaching: std::time::Instant,
    geometry: Option<GridGeometry>,
    /// The last grid resize, to pace them under `GridHold`.
    resized_at: Option<std::time::Instant>,
    focus: FocusHandle,
    wheel_acc: f32,
    drag: Option<Drag>,
    /// The pane's app asks for mouse reporting (from herdr; false until it says otherwise).
    mouse_capture: bool,
    /// The kitty keyboard flags the pane's app asked for (from herdr; 0, legacy keys, until it says otherwise).
    keyboard_flags: u16,
    scroll: ScrollState,
    /// A `pane.get` is out; `scroll_stale` asks for one more when it lands.
    scroll_fetching: bool,
    scroll_stale: bool,
    /// Output arrived since herdr was last asked for the scroll position; asked when the pane is next drawn, so
    /// hidden panes (kept attached for a quick tab switch) never ask.
    scroll_due: bool,
    scrollbar: HerdrScroll,
    _scrollbar_drag: Task<()>,
    /// Until then `scroll.offset_from_bottom` is our own prediction, not herdr's (stale) report.
    scroll_local_until: Option<std::time::Instant>,
    _scroll_settle: Option<Task<()>>,
    /// The frame that lays out a resize `HELD_RESIZE` held back.
    _resize_catchup: Option<Task<()>>,
    _pump: Option<Task<()>>,
    search: Option<Search>,
    /// The pointer, and the link under it while Cmd is held: (row, cells).
    pointer: Option<gpui::Point<Pixels>>,
    hover: Option<(usize, Range<usize>)>,
    /// The cell a hover was last reported in to an app that captures the mouse: one report per cell crossed.
    hover_cell: Option<(u16, u16)>,
    /// The image a Cmd+click opened, in a dialog over the window until Esc, Space or a click outside it.
    preview: Option<Popup<Preview>>,
    /// The Markdown file a Cmd+click opened, rendered in a dialog over the window until Esc or a click outside it.
    reader: Option<Popup<Reader>>,
    /// Always attaches by taking the terminal over (herdr's popup: whichever window shows it types into it).
    pub takes_over: bool,
    /// Kitty images herdr shows on the pane (from the window's client shell), drawn over the grid.
    images: Vec<PaneImage>,
}

impl PaneTerminal {
    pub fn new(pane_id: String, cx: &mut Context<Self>) -> Self {
        // The scrollbar's drags arrive here: the handle has no context to scroll herdr with.
        let (wanted, mut drags) = futures::channel::mpsc::unbounded::<u32>();
        let scrollbar_drag = cx.spawn(async move |this, cx| {
            while let Some(target) = drags.next().await {
                let moved = this.update(cx, |pane, cx| {
                    pane.scroll_by(target as i64 - pane.scroll.offset_from_bottom as i64, None, cx)
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
            live: None,
            history_pending: None,
            session: None,
            observing: false,
            held: Vec::new(),
            ended: None,
            attaching: std::time::Instant::now(),
            geometry: None,
            resized_at: None,
            focus: cx.focus_handle(),
            wheel_acc: 0.0,
            drag: None,
            mouse_capture: false,
            keyboard_flags: 0,
            scroll: ScrollState::default(),
            scroll_fetching: false,
            scroll_stale: false,
            scroll_due: false,
            scrollbar,
            _scrollbar_drag: scrollbar_drag,
            scroll_local_until: None,
            _scroll_settle: None,
            _resize_catchup: None,
            _pump: None,
            search: None,
            pointer: None,
            hover: None,
            hover_cell: None,
            preview: None,
            reader: None,
            takes_over: false,
            images: Vec::new(),
        }
        // Attaching waits for the first measured grid (`on_grid_metrics`): herdr resizes the real PTY to
        // whatever size the attach names.
    }

    pub fn pane_id(&self) -> &str {
        &self.pane_id
    }

    pub fn geometry(&self) -> Option<GridGeometry> {
        self.geometry
    }

    pub fn ended(&self) -> Option<&str> {
        self.ended.as_deref()
    }

    /// herdr's images on this pane now; returns those it no longer shows, for the window to free.
    pub fn set_images(&mut self, images: Vec<PaneImage>, cx: &mut Context<Self>) -> Vec<std::sync::Arc<gpui::RenderImage>> {
        if self.images.is_empty() && images.is_empty() {
            return Vec::new();
        }
        let old = std::mem::replace(&mut self.images, images);
        cx.notify();
        old.into_iter().map(|i| i.image).filter(|o| !self.images.iter().any(|i| std::sync::Arc::ptr_eq(&i.image, o))).collect()
    }

    /// Connects off the main thread: herdr's handshake takes tens of milliseconds, and attaching happens while a
    /// frame is laid out (a tab switch attaches every pane it shows).
    fn attach(&mut self, takeover: bool, cx: &mut Context<Self>) {
        if crate::mock::pane_text(&self.pane_id).is_some() {
            return; // the fixture's text is laid out by `on_grid_metrics`; there is no herdr to attach to
        }
        let (cols, rows) = (self.emulator.cols() as u16, self.emulator.rows() as u16);
        let px = self.geometry.map_or((0, 0), |g| g.cell_px);
        let pane_id = self.pane_id.clone();
        let mode = match (takeover || self.takes_over, yielded(cx)) {
            (true, _) => Mode::Takeover,
            (false, true) => Mode::Observe,
            (false, false) => Mode::Control,
        };
        self.observing = mode == Mode::Observe;
        self.attaching = std::time::Instant::now();
        let spawn = cx.background_executor().spawn(async move { Session::spawn(&pane_id, cols, rows, px, mode) });
        self._pump = Some(cx.spawn(async move |this, cx| {
            let result = spawn.await;
            let rx = this.update(cx, |pane, cx| {
                cx.notify();
                match result {
                    Ok((session, rx)) => {
                        // The grid may have changed size while connecting.
                        let now = (pane.emulator.cols() as u16, pane.emulator.rows() as u16);
                        let now_px = pane.geometry.map_or(px, |g| g.cell_px);
                        if (now, now_px) != ((cols, rows), px) {
                            session.resize(now.0, now.1, now_px);
                        }
                        if !pane.observing && !pane.held.is_empty() {
                            session.input(&std::mem::take(&mut pane.held));
                        }
                        pane.session = Some(session);
                        pane.mouse_capture = false;
                        pane.keyboard_flags = 0;
                        pane.ended = None;
                        Some(rx)
                    }
                    Err(err) => {
                        pane.held.clear();
                        pane.ended = Some(format!("{err:#}"));
                        None
                    }
                }
            });
            if let Ok(Some(rx)) = rx {
                pump(this, rx, cx).await;
            }
        }));
    }

    /// `takeover` replaces another client that controls the pane.
    pub fn reattach(&mut self, takeover: bool, cx: &mut Context<Self>) {
        self.session = None;
        self.attach(takeover, cx);
        cx.notify();
    }

    /// Re-attaches as [`Yielded`] now asks: observing, or controlling again.
    pub fn follow_yield(&mut self, cx: &mut Context<Self>) {
        if self.geometry.is_some() && self.ended.is_none() && self.observing != yielded(cx) {
            self.reattach(false, cx);
        }
    }

    /// Element hook: this frame's grid placement. A size change resizes the emulator and herdr's view.
    pub fn on_grid_metrics(&mut self, geometry: GridGeometry, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.scroll_due) {
            self.refresh_scroll(cx);
        }
        let first = self.geometry.is_none();
        // `SHEPHERD_READER=<file>`: the first pane laid out opens it as a Cmd+click does (screenshots)
        static READER_HOOK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if first && let Some(path) = std::env::var("SHEPHERD_READER").ok().filter(|_| !READER_HOOK.swap(true, std::sync::atomic::Ordering::Relaxed)) {
            self.open_link(Link::File { path, line: None, col: None }, cx);
        }
        let rescaled = self.geometry.is_some_and(|g| g.cell_px != geometry.cell_px);
        let (cols, rows) = (geometry.cols, geometry.rows);
        let resized = self.emulator.cols() != cols as usize || self.emulator.rows() != rows as usize;
        let held = cx.try_global::<GridHold>().is_some_and(|hold| hold.0);
        // the scrollbar follows the pane every frame, not the grid it held back
        self.scrollbar.0.bounds.set(geometry.bounds);
        let wait = self.resized_at.and_then(|at| HELD_RESIZE.checked_sub(at.elapsed())).filter(|w| !w.is_zero());
        if let Some(wait) = wait.filter(|_| !first && resized && held) {
            // a drag is under way and the grid just reflowed: a frame once the pace allows catches up, also when
            // the drag stops before it (a window's live resize has no drop to say so)
            self._resize_catchup = Some(cx.spawn(async move |this, cx| {
                cx.background_executor().timer(wait).await;
                let _ = this.update(cx, |_, cx| cx.notify());
            }));
            return;
        }
        self.geometry = Some(geometry);
        // the fixture's text (`SHEPHERD_MOCK`), laid out afresh at every size: there is no herdr to redraw it
        if let Some(text) = crate::mock::pane_text(&self.pane_id) {
            if first || resized {
                self.emulator = Emulator::new(cols, rows);
                self.emulator.feed(&text);
                // ponytail: an Instant a day ahead never comes due, so no loader covers the fixture
                self.attaching = std::time::Instant::now() + std::time::Duration::from_secs(86_400);
                cx.notify();
            }
            return;
        }
        if resized {
            self.leave_history(); // the copy is wrapped at the old width
            self.emulator.resize(cols, rows);
            self.resized_at = Some(std::time::Instant::now());
        }
        if first && self.session.is_none() && self.ended.is_none() {
            self.attach(false, cx);
        } else if let (Some(session), true) = (&self.session, resized || rescaled) {
            session.resize(cols, rows, geometry.cell_px);
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
                    if pane.live.is_some() {
                        scroll.offset_from_bottom = pane.emulator.display_offset() as u32; // herdr itself stays at its bottom
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
    /// `at` is the wheel's (col, row): an app reading the mouse (vim) gets the wheel there, herdr's default is 0,0.
    fn scroll_by(&mut self, delta: i64, at: Option<(u16, u16)>, cx: &mut Context<Self>) {
        if self.session.is_none() || delta == 0 {
            return;
        }
        if let Some(rest) = self.scroll_locally(delta, cx) {
            self.scroll_herdr(rest, at, cx);
        }
    }

    /// Scrolls through herdr's history without herdr: up from the live bottom reads it once (`pane.read`, at most
    /// herdr's 1000 lines), then each step is local, as fast as a terminal's own scrollback. Returns the lines left
    /// to herdr: all of them for an app that takes the wheel, the alternate screen, herdr scrolled already, a
    /// client-shell popup; those past the lines read.
    fn scroll_locally(&mut self, delta: i64, cx: &mut Context<Self>) -> Option<i64> {
        if let Some(pending) = &mut self.history_pending {
            *pending += delta;
            return None;
        }
        if self.live.is_some() {
            let target = self.emulator.display_offset() as i64 + delta;
            if target <= 0 {
                self.leave_history();
            } else if target as usize <= self.emulator.history_lines() {
                self.emulator.scroll_to_offset(target as usize);
                self.scroll.offset_from_bottom = target as u32;
            } else {
                self.leave_history();
                return Some(target); // herdr scrolls on from its bottom
            }
            cx.notify();
            return None;
        }
        let local = delta > 0 && self.scroll.offset_from_bottom == 0 && !self.mouse_capture && !self.emulator.alt_screen();
        let Some(PaneQueries(client, rt)) = cx.try_global::<PaneQueries>().filter(|_| local) else { return Some(delta) };
        self.history_pending = Some(delta);
        let (client, pane_id) = (client.clone(), self.pane_id.clone());
        let size = (self.emulator.cols(), self.emulator.rows());
        let params = serde_json::json!({"pane_id": pane_id, "source": "recent", "lines": 1000, "format": "ansi", "strip_ansi": false});
        let call = rt.spawn(async move { client.call("pane.read", params).await });
        cx.spawn(async move |this, cx| {
            let text = call.await.ok().and_then(Result::ok).and_then(|r| r["read"]["text"].as_str().map(str::to_owned));
            let _ = this.update(cx, |pane, cx| {
                let Some(delta) = pane.history_pending.take() else { return }; // a key or a resize came first
                // the grid must be the one the history was read at, or its lines land wrong
                match text.filter(|_| (pane.emulator.cols(), pane.emulator.rows()) == size) {
                    Some(text) => {
                        let history = Emulator::with_history(&text, size.0 as u16, size.1 as u16);
                        pane.live = Some(std::mem::replace(&mut pane.emulator, history));
                        pane.scroll_by(delta, None, cx);
                    }
                    None => pane.scroll_herdr(delta, None, cx),
                }
            });
        })
        .detach();
        None
    }

    /// Back to the live grid: herdr is at its bottom all along.
    // ponytail: the copy is not refreshed while it is shown; new output shows once you are back at the bottom
    fn leave_history(&mut self) {
        self.history_pending = None;
        if let Some(live) = self.live.take() {
            self.emulator = live;
            self.scroll.offset_from_bottom = 0;
        }
    }

    fn scroll_herdr(&mut self, delta: i64, at: Option<(u16, u16)>, cx: &mut Context<Self>) {
        let Some(session) = &self.session else { return };
        let (up, lines) = (delta > 0, delta.unsigned_abs().min(u16::MAX as u64) as u16);
        let (col, row) = at.unwrap_or((0, 0));
        if !self.via_shell(PaneInput::wheel(up, lines, col, row), cx) {
            session.scroll(up, lines, at);
        }
        let max = self.scroll.max_offset_from_bottom as i64;
        self.scroll.offset_from_bottom = (self.scroll.offset_from_bottom as i64 + delta).clamp(0, max) as u32;
        self.scroll_local_until = Some(std::time::Instant::now() + SCROLL_SETTLE);
        self._scroll_settle = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SCROLL_SETTLE + std::time::Duration::from_millis(20)).await;
            let _ = this.update(cx, |pane, cx| pane.refresh_scroll(cx));
        }));
        cx.notify();
    }

    pub fn active_grid_snapshot(&self, cx: &App) -> Option<GridSnapshot> {
        let mut lines = self.emulator.lines();
        if let Some((row, cells)) = &self.hover {
            // The hover was found on an older grid; output or a resize since may have shortened the row.
            for cell in lines.get_mut(*row).and_then(|l| l.get_mut(cells.clone())).into_iter().flatten() {
                cell.underline = true;
            }
        }
        let marks = self.search_marks(&lines, cx);
        // herdr places images for its live screen: scrolled back, they move down with the text
        let back = self.live.as_ref().map_or(0, |_| self.emulator.display_offset() as u16);
        let images = self.images.iter().map(|i| PaneImage { y: i.y.saturating_add(back), ..i.clone() }).collect();
        let cursor = if self.live.is_some() { None } else { self.emulator.cursor() };
        Some(GridSnapshot { lines, cursor, marks, images })
    }

    /// A key as herdr's semantic event over the window's client shell (see [`TypingShell`]), which encodes it for
    /// the app's keyboard protocol; false: the pane is not one to type into that way (see [`Self::input`]).
    fn type_key(&mut self, event: PaneInput, cx: &mut Context<Self>) -> bool {
        if !self.via_shell(event, cx) {
            return false;
        }
        self.typed(cx);
        true
    }

    /// Sends `event` over the window's client shell, which herdr acts on at once, where an attach's input waits out
    /// its 16 ms render cadence (a key's echo ~6 ms against ~23, the wheel in an app reading the mouse ~0.8 against
    /// ~20, `client_shell::input_paths`); false: send it over the attach.
    fn via_shell(&self, event: PaneInput, cx: &App) -> bool {
        let controlled = self.session.is_some() && !self.observing;
        controlled && cx.try_global::<TypingShell>().is_some_and(|t| t.0.pane_input(&self.pane_id, vec![event]))
    }

    /// Typing goes back to the live bottom, as in a terminal.
    fn typed(&mut self, cx: &mut Context<Self>) {
        if self.live.is_some() || self.history_pending.is_some() {
            self.leave_history();
            cx.notify();
        }
    }

    fn input(&mut self, bytes: &[u8], cx: &mut Context<Self>) {
        self.typed(cx);
        match &self.session {
            Some(session) if !self.observing => {
                if !cx.try_global::<TypingShell>().is_some_and(|t| t.0.type_into(&self.pane_id, bytes)) {
                    session.input(bytes)
                }
            }
            _ if self.ended.is_none() => {
                self.held.extend_from_slice(bytes);
                claim(cx);
            }
            _ => {}
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus.is_focused(window) {
            return; // typing in the search bar
        }
        let ks = &event.keystroke;
        // The reader takes the keys too: Esc closes it, Cmd+A (Ctrl+A off macOS) selects its text.
        if self.reader.as_ref().is_some_and(Popup::is_open) {
            let m = &ks.modifiers;
            if ks.key == "escape" {
                self.close_reader(cx);
            } else if ks.key == "a" && if cfg!(target_os = "macos") { m.platform } else { m.control } {
                crate::ui::markdown::select_all(READER_KEY);
                cx.notify();
            }
            return;
        }
        // The dialog takes the keys: Esc and Space (Quick Look's) close it, the rest go nowhere.
        if self.preview.as_ref().is_some_and(Popup::is_open) {
            if ks.key == "escape" || ks.key == "space" {
                self.close_preview(cx);
            }
            return;
        }
        let mods = &ks.modifiers;
        if ks.key == "f" && app_shortcut(mods) {
            self.open_search(window, cx);
            cx.stop_propagation();
            return;
        }
        if ks.key == "g" && app_shortcut(mods) && self.search.is_some() {
            self.step(!search_back(mods), cx);
            cx.stop_propagation();
            return;
        }
        if ks.key == "v" && app_shortcut(mods) {
            // ponytail: always bracketed; herdr's frames do not tell us whether the app asked for it.
            if let Some(item) = cx.read_from_clipboard() {
                match item.text() {
                    Some(text) => self.input(&paste_bytes(&text, true), cx),
                    // an image: ctrl-v, the key agent TUIs (Claude Code, Codex) read the clipboard's image on
                    None => self.input(b"\x16", cx),
                }
            }
            cx.stop_propagation();
            return;
        }
        if ks.key == "c" && app_shortcut(mods) {
            if let Some(text) = self.emulator.selection_text() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                cx.stop_propagation();
            }
            return;
        }
        if let Some(event) = crate::herdr::client_shell::key_event(&ks.key, ks.key_char.as_deref(), mods)
            && self.type_key(event, cx)
        {
            cx.stop_propagation();
            return;
        }
        let bytes = kitty_enter(&ks.key, mods, self.keyboard_flags).or_else(|| keystroke_bytes(&ks.key, ks.key_char.as_deref(), mods, false));
        if let Some(bytes) = bytes {
            self.input(&bytes, cx);
            cx.stop_propagation();
        }
    }

    fn on_scroll_wheel(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let line_h = self.geometry.map_or(TERM_LINE_HEIGHT, |g| g.line_h);
        let px = match event.delta {
            ScrollDelta::Pixels(d) => f32::from(d.y),
            ScrollDelta::Lines(d) => d.y * line_h,
        };
        claim(cx);
        let speed = if event.modifiers.alt { WHEEL_SPEED * FAST_WHEEL } else { WHEEL_SPEED };
        let n = wheel_lines(&mut self.wheel_acc, px * speed, line_h);
        let at = self.hit(event.position).map(|(row, col, _, _)| (col, row));
        self.scroll_by(n as i64, at, cx);
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

    fn mouse(&self, action: &str, button: MouseButton, position: gpui::Point<Pixels>, mods: &Modifiers, cx: &App) {
        let button = match button {
            MouseButton::Right => "right",
            MouseButton::Middle => "middle",
            _ => "left",
        };
        if let (Some(session), Some((row, col, _, _))) = (&self.session, self.hit(position))
            && !self.via_shell(PaneInput::mouse(action, button, col, row, mouse_mods(mods)), cx)
        {
            session.mouse(action, button, col, row, mouse_mods(mods));
        }
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.button != MouseButton::Left {
            // Right and middle go to an app that captures the mouse; else (or with Shift, as Ghostty) the right
            // button is the pane's menu, the shell's
            let captured = self.mouse_capture && self.session.as_ref().is_some_and(Session::knows_mouse_capture);
            if !captured || event.modifiers.shift {
                return;
            }
            cx.stop_propagation();
            window.focus(&self.focus, cx);
            claim(cx);
            self.mouse("down", event.button, event.position, &event.modifiers, cx);
            let drag = Drag { origin: event.position, armed: false, to_app: true, select: false, button: event.button };
            self.drag = Some(drag);
            return;
        }
        window.focus(&self.focus, cx);
        claim(cx);
        if let Some((_, _, link)) = self.link_under(event.position).filter(|_| link_modifier(&event.modifiers)) {
            self.open_link(link, cx);
            return;
        }
        let Some((_, _, point, side)) = self.hit(event.position) else { return };
        // Ghostty: an app that captures the mouse gets it, Shift takes it back for a selection. A CLI session
        // cannot tell, so it keeps forwarding (herdr drops events the app did not ask for) and selects as well.
        let shift = event.modifiers.shift;
        let known = self.session.as_ref().is_some_and(Session::knows_mouse_capture);
        let (to_app, select) = if known { (self.mouse_capture && !shift, !self.mouse_capture || shift) } else { (!shift, true) };
        if to_app {
            self.mouse("down", MouseButton::Left, event.position, &event.modifiers, cx);
        }
        self.emulator.clear_selection();
        let armed = select && event.click_count > 1;
        if armed {
            let ty = if event.click_count == 2 { SelectionType::Semantic } else { SelectionType::Lines };
            self.emulator.start_selection(ty, point, side);
        }
        self.drag = Some(Drag { origin: event.position, armed, to_app, select, button: MouseButton::Left });
        cx.notify();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.pointer = Some(event.position);
        // gpui's `dragging` is the left button's only
        if let Some(drag) = self.drag.filter(|d| d.button != MouseButton::Left) {
            self.mouse("drag", drag.button, event.position, &event.modifiers, cx);
            return;
        }
        if !event.dragging() {
            self.update_hover(link_modifier(&event.modifiers), cx);
            self.report_hover(event, cx);
            return;
        }
        let Some(mut drag) = self.drag else { return };
        if drag.to_app {
            self.mouse("drag", drag.button, event.position, &event.modifiers, cx);
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

    /// A move with no button held goes to an app that captures the mouse (herdr drops it unless the app asked for
    /// any-motion reporting); Shift keeps the mouse for a selection, as on a press.
    fn report_hover(&mut self, event: &MouseMoveEvent, cx: &App) {
        let captured = self.mouse_capture && self.session.as_ref().is_some_and(Session::knows_mouse_capture);
        let cell = self.hit(event.position).map(|(row, col, _, _)| (row, col)).filter(|_| captured && !event.modifiers.shift);
        if cell.is_some() && cell != self.hover_cell {
            self.mouse("move", MouseButton::Left, event.position, &event.modifiers, cx);
        }
        self.hover_cell = cell;
    }

    fn on_mouse_up(&mut self, event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take().filter(|d| d.button == event.button) else { return };
        if drag.to_app {
            self.mouse("up", drag.button, event.position, &event.modifiers, cx);
        }
        if !drag.armed || !crate::settings::SettingsGlobal::of(cx).settings.copy_on_select {
            return;
        }
        if let Some(text) = self.emulator.selection_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.emulator.clear_selection();
            cx.notify();
            crate::ui::toast::info(cx, "Copied to clipboard", None);
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
                    SessionEvent::Frame(bytes) => drop(pane.live.as_mut().unwrap_or(&mut pane.emulator).feed(&bytes)),
                    SessionEvent::MouseCapture(enabled) => pane.mouse_capture = enabled,
                    SessionEvent::KeyboardFlags(flags) => pane.keyboard_flags = flags,
                    SessionEvent::Size(cols, rows) => {
                        pane.leave_history();
                        pane.emulator.resize(cols, rows)
                    }
                    SessionEvent::Ended(why) => {
                        pane.ended = Some(why);
                        pane.session = None;
                        ended = true;
                        break;
                    }
                }
            }
            if !ended {
                pane.scroll_due = true;
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
                // keeps the pane focused, so the app's shortcuts still reach their handlers
                .track_focus(&self.focus)
                .on_mouse_down(MouseButton::Left, cx.listener(|pane, _, window, cx| window.focus(&pane.focus, cx)))
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
                // a server on another protocol is why: say so, and how to fix it
                .children(cx.try_global::<crate::herdr::OutdatedServer>().and_then(|o| o.0.clone()).map(|outdated| {
                    div().max_w(px(420.)).text_center().text_color(theme.danger).child(outdated)
                }))
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
        crate::ui::reap_popup(&mut self.preview, window);
        crate::ui::reap_popup(&mut self.reader, window);
        let focused = self.focus.is_focused(window);
        let search_bar = self.search.as_ref().map(|search| self.search_bar(search, &theme, cx));
        let loader = (self.session.is_none() && crate::ui::loader::due(self.attaching, window)).then(|| {
            div().absolute().inset_0().flex().items_center().justify_center().child(crate::ui::loader::spinner(2.5, window))
        });
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
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Right, cx.listener(Self::on_mouse_up))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
            // files from Finder: their paths, pasted
            .on_drop(cx.listener(|pane, paths: &ExternalPaths, window, cx| {
                window.focus(&pane.focus, cx);
                pane.input(&paste_bytes(&dropped_paths(paths.paths()), true), cx);
            }))
            // The text size keys scale the open image instead.
            .on_action(cx.listener(|pane, _: &crate::bindings::FontSizeUp, window, cx| pane.zoom_key(ZOOM_STEP, window, cx)))
            .on_action(cx.listener(|pane, _: &crate::bindings::FontSizeDown, window, cx| pane.zoom_key(1. / ZOOM_STEP, window, cx)))
            .on_action(cx.listener(|pane, _: &crate::bindings::FontSizeReset, window, cx| pane.zoom_key(0., window, cx)))
            .on_modifiers_changed(cx.listener(|pane, e: &ModifiersChangedEvent, _, cx| pane.update_hover(link_modifier(&e.modifiers), cx)))
            .when(self.hover.is_some(), |el| el.cursor_pointer())
            .child(TerminalElement::new(cx.entity(), focused))
            .child(crate::ui::scrollbar(SharedString::from(format!("scrollbar-{}", self.pane_id)), &self.scrollbar, &theme))
            .children(search_bar)
            .children(loader)
            .children(self.preview.as_ref().map(|preview| self.preview(preview, &theme, window, cx)))
            .children(self.reader.as_ref().map(|reader| self.reader(reader, &theme, window, cx)))
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
            cell_px: (16, 34),
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
        // The frame that measured the grid does not wait for herdr: the attach runs in the background.
        pane.read_with(cx, |p, _| assert!(p.session.is_none() && p.ended.is_none(), "attached on the main thread"));
        cx.run_until_parked();
        pane.read_with(cx, |p, _| {
            assert_eq!((p.emulator.cols(), p.emulator.rows()), (120, 40));
            // The attach ran (at 120×40) and failed on the missing binary.
            assert!(p.ended.as_deref().is_some_and(|e| e.contains("/nonexistent/herdr")), "{:?}", p.ended);
        });
    }

    #[gpui::test]
    fn the_grid_reflows_at_a_pace_while_a_divider_moves(cx: &mut gpui::TestAppContext) {
        // Under a divider drag the grid reflows at most every HELD_RESIZE, not on every frame.
        // No client socket and no binary: the attach fails without touching a real herdr.
        unsafe {
            std::env::set_var("SHEPHERD_HERDR", "/nonexistent/herdr");
            std::env::set_var("HERDR_CLIENT_SOCKET_PATH", "/nonexistent/herdr-client.sock");
        }
        let pane = cx.new(|cx| PaneTerminal::new("w1:p1".into(), cx));
        pane.update(cx, |p, cx| p.on_grid_metrics(geometry(120, 40), cx));
        cx.update(|cx| cx.set_global(GridHold(true)));
        pane.update(cx, |p, cx| p.on_grid_metrics(geometry(90, 40), cx));
        pane.read_with(cx, |p, _| assert_eq!(p.emulator.cols(), 120, "reflowed twice within HELD_RESIZE"));
        pane.read_with(cx, |p, _| assert!(p._resize_catchup.is_some(), "the held-back size waits for no further frame"));
        pane.update(cx, |p, cx| {
            p.resized_at = Some(std::time::Instant::now() - HELD_RESIZE);
            p.on_grid_metrics(geometry(90, 40), cx)
        });
        pane.read_with(cx, |p, _| assert_eq!(p.emulator.cols(), 90, "the drag reflows once the pace allows"));
        cx.update(|cx| cx.set_global(GridHold(false)));
        pane.update(cx, |p, cx| p.on_grid_metrics(geometry(100, 40), cx));
        pane.read_with(cx, |p, _| assert_eq!(p.emulator.cols(), 100));
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
    fn a_match_off_screen_is_brought_to_mid_screen() {
        // 100 lines of history above a 20-row screen at the live bottom: the screen shows lines 100..120.
        let s = ScrollState { offset_from_bottom: 0, max_offset_from_bottom: 100, viewport_rows: 20 };
        assert_eq!(reveal_offset(s, 110, 20), None);
        assert_eq!(reveal_offset(s, 50, 20), Some(60)); // top at 40, line 50 on row 10
        assert_eq!(reveal_offset(s, 2, 20), Some(100)); // clamped to the oldest line
        let up = ScrollState { offset_from_bottom: 60, ..s };
        assert_eq!(reveal_offset(up, 118, 20), Some(0));
    }

    #[test]
    fn the_editor_gets_the_line() {
        let path = Path::new("/tmp/a.rs");
        assert_eq!(editor_argv(Some("/bin/ed -g"), None, path, Some(3), Some(4)).unwrap(), ["/bin/ed", "-g", "/tmp/a.rs:3:4"]);
        assert_eq!(editor_argv(Some("/bin/ed"), None, path, Some(3), None).unwrap(), ["/bin/ed", "/tmp/a.rs:3"]);
    }

    #[test]
    fn a_machines_file_opens_over_the_editors_ssh() {
        let path = Path::new("/srv/app/a.rs");
        assert_eq!(editor_argv(Some("/bin/zed"), Some("box"), path, Some(3), None).unwrap(), ["/bin/zed", "ssh://box/srv/app/a.rs:3"]);
        assert_eq!(
            editor_argv(Some("/bin/code -g"), Some("me@box"), path, Some(3), Some(4)).unwrap(),
            ["/bin/code", "--remote", "ssh-remote+me@box", "-g", "/srv/app/a.rs:3:4"]
        );
        assert_eq!(editor_argv(Some("/bin/cursor -g"), Some("box"), path, None, None).unwrap(), ["/bin/cursor", "--remote", "ssh-remote+box", "/srv/app/a.rs"]);
        assert_eq!(editor_argv(Some("/bin/ed"), Some("box"), path, None, None), None);
    }

    #[test]
    fn nvim_gets_the_line_and_column() {
        assert_eq!(nvim_command("nvim", Path::new("/tmp/a b.rs"), Some(3), Some(4)), "exec nvim '+call cursor(3,4)' '/tmp/a b.rs'");
        assert_eq!(nvim_command("nvim", Path::new("/tmp/it's"), Some(3), None), "exec nvim +3 '/tmp/it'\\''s'");
        assert_eq!(nvim_command("nvim", Path::new("/tmp/a"), None, None), "exec nvim '/tmp/a'");
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
