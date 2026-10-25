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

const HELD_RESIZE: std::time::Duration = std::time::Duration::from_millis(80);

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

/// Cmd+F: herdr's whole history is searched (`pane.read`) to step through; matches on screen come from the grid.
/// What herdr's `pane.read` gives at most (it caps `lines`, with no paging), so what search sees of a history.
const READ_LINES: u32 = 1000;

struct Search {
    input: Entity<InputState>,
    /// Matches in herdr's history: line from the oldest one, char range.
    hits: Vec<(usize, Range<usize>)>,
    /// The history is longer than herdr reads (`READ_LINES`): only its newest lines were searched.
    partial: bool,
    current: Option<usize>,
    _events: Subscription,
    _read: Option<Task<()>>,
}

/// A row's chars and, per char, its cell (wide chars' spacers skipped).
fn row_chars(cells: &[CellSnapshot]) -> (Vec<char>, Vec<usize>) {
    cells.iter().enumerate().filter(|(_, c)| !c.wide_spacer).map(|(i, c)| (c.ch, i)).unzip()
}

/// A char range of `row_chars` as cells.
fn cell_range(cells: &[CellSnapshot], cols: &[usize], chars: Range<usize>) -> Range<usize> {
    let last = cols[chars.end - 1];
    cols[chars.start]..last + 1 + usize::from(cells[last].wide)
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
        if !first && resized && held && self.resized_at.is_some_and(|at| at.elapsed() < HELD_RESIZE) {
            return; // a drag is under way and the grid just reflowed: the next frame (or the drop) catches up
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

    fn search_marks(&self, lines: &[Vec<CellSnapshot>], cx: &App) -> Vec<(usize, Range<usize>, bool)> {
        let Some(search) = &self.search else { return Vec::new() };
        let query: Vec<char> = search.input.read(cx).value().chars().collect();
        // The current hit's row on screen: herdr's lines count from the oldest, the screen's top is `max - offset`.
        let current = search.current.and_then(|i| search.hits.get(i)).and_then(|(line, chars)| {
            let row = *line as i64 - (self.scroll.max_offset_from_bottom as i64 - self.scroll.offset_from_bottom as i64);
            usize::try_from(row).ok().map(|row| (row, chars.clone()))
        });
        let mut marks = Vec::new();
        for (row, cells) in lines.iter().enumerate() {
            let (chars, cols) = row_chars(cells);
            for hit in find_all(&chars, &query) {
                let now = current.as_ref().is_some_and(|(r, c)| *r == row && *c == hit);
                marks.push((row, cell_range(cells, &cols, hit), now));
            }
        }
        marks
    }

    fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.search.is_none() {
            let theme = Theme::of(cx).clone();
            let input = cx.new(|cx| {
                let mut input = InputState::new(window, cx).placeholder("Find");
                input.set_editor_style(crate::ui::input_style(&theme));
                input
            });
            let events = cx.subscribe_in(&input, window, |pane, _, event: &InputEvent, _, cx| match event {
                InputEvent::Change => pane.search_history(cx),
                InputEvent::PressEnter { shift, .. } => pane.step(!shift, cx),
                _ => {}
            });
            self.search = Some(Search { input, hits: Vec::new(), partial: false, current: None, _events: events, _read: None });
        }
        if let Some(search) = &self.search {
            search.input.update(cx, |input, cx| {
                input.focus(window, cx);
                input.select_all(window, cx);
            });
        }
        cx.notify();
    }

    fn close_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Reads herdr's history and finds the query in it; the current match becomes the newest one not below the
    /// screen.
    fn search_history(&mut self, cx: &mut Context<Self>) {
        let Some(search) = &mut self.search else { return };
        let query: Vec<char> = search.input.read(cx).value().chars().collect();
        search.hits.clear();
        search.current = None;
        search._read = None;
        cx.notify();
        let Some(PaneQueries(client, rt)) = cx.try_global::<PaneQueries>().filter(|_| !query.is_empty()) else { return };
        let (client, pane_id) = (client.clone(), self.pane_id.clone());
        let read = rt.spawn(async move {
            let read = client.call("pane.read", serde_json::json!({"pane_id": pane_id, "source": "recent", "lines": READ_LINES})).await;
            let text = read.ok().and_then(|r| r["read"]["text"].as_str().map(str::to_owned)).unwrap_or_default();
            let lines = text.lines().enumerate();
            let hits = lines.flat_map(|(i, l)| find_all(&l.chars().collect::<Vec<_>>(), &query).into_iter().map(move |r| (i, r)));
            (text.lines().count(), hits.collect::<Vec<_>>())
        });
        search._read = Some(cx.spawn(async move |this, cx| {
            let Ok((read, hits)) = read.await else { return };
            let _ = this.update(cx, |pane, cx| {
                let s = pane.scroll;
                let bottom = (s.max_offset_from_bottom - s.offset_from_bottom.min(s.max_offset_from_bottom)) as usize + pane.emulator.rows();
                // herdr reads the newest lines: the first one read is this far from the oldest
                let skipped = (s.max_offset_from_bottom as usize + pane.emulator.rows()).saturating_sub(read);
                let hits: Vec<_> = hits.into_iter().map(|(line, chars)| (line + skipped, chars)).collect();
                let Some(search) = &mut pane.search else { return };
                search.partial = skipped > 0;
                search.current = hits.iter().rposition(|(line, _)| *line < bottom).or(hits.len().checked_sub(1));
                search.hits = hits;
                pane.reveal(cx);
            });
        }));
    }

    /// The next match up into history (`older`) or back down, wrapping around.
    fn step(&mut self, older: bool, cx: &mut Context<Self>) {
        let Some(search) = &mut self.search else { return };
        let n = search.hits.len();
        if n == 0 {
            return;
        }
        search.current = Some(match search.current {
            None => n - 1,
            Some(i) if older => (i + n - 1) % n,
            Some(i) => (i + 1) % n,
        });
        self.reveal(cx);
    }

    /// Scrolls herdr to the current match unless it is on screen.
    fn reveal(&mut self, cx: &mut Context<Self>) {
        let line = self.search.as_ref().and_then(|s| s.current.and_then(|i| s.hits.get(i))).map(|(line, _)| *line);
        match line.and_then(|line| reveal_offset(self.scroll, line, self.emulator.rows())) {
            Some(offset) => self.scroll_by(offset as i64 - self.scroll.offset_from_bottom as i64, None, cx),
            None => cx.notify(),
        }
    }

    fn search_bar(&self, search: &Search, theme: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let count = match (search.current, search.hits.len()) {
            (Some(i), n) => format!("{}/{n}", i + 1),
            _ if search.input.read(cx).value().is_empty() => String::new(),
            _ => "No results".into(),
        };
        let count = if search.partial && !count.is_empty() { format!("{count} · last {READ_LINES}") } else { count };
        div()
            .id("search")
            .absolute()
            .top(px(8.))
            .right(px(20.))
            .w(px(300.))
            .flex()
            .items_center()
            .gap_2()
            .pl_2()
            .pr_3()
            .py_1()
            .rounded_md()
            .bg(theme.card)
            .border_1()
            .border_color(theme.border)
            .text_sm()
            .text_color(theme.text)
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .capture_key_down(cx.listener(|pane, e: &KeyDownEvent, window, cx| {
                let k = &e.keystroke;
                match k.key.as_str() {
                    "escape" => pane.close_search(window, cx),
                    "g" if app_shortcut(&k.modifiers) => pane.step(!search_back(&k.modifiers), cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .child(div().flex_1().child(Input::new(&search.input)))
            .child(div().flex_none().text_color(theme.text_muted).child(count))
            .into_any_element()
    }

    /// The link under a window position: its row, cells and target.
    fn link_under(&self, position: gpui::Point<Pixels>) -> Option<(usize, Range<usize>, Link)> {
        let g = self.geometry?;
        if !g.bounds.contains(&position) {
            return None;
        }
        let (row, col, _, _) = self.hit(position)?;
        // An OSC 8 hyperlink wins over the text under it.
        let osc8 = self.emulator.hyperlink_at(row as usize, col as usize);
        if let Some((cells, link)) = osc8.and_then(|(cells, uri)| Some((cells, hyperlink(&uri)?))) {
            return Some((row as usize, cells, link));
        }
        let cells = self.emulator.line(row as usize);
        let (chars, cols) = row_chars(&cells);
        let at = cols.iter().rposition(|&c| c <= col as usize)?;
        let (chars_range, link) = link_at(&chars, at)?;
        Some((row as usize, cell_range(&cells, &cols, chars_range), link))
    }

    fn update_hover(&mut self, cmd: bool, cx: &mut Context<Self>) {
        // No links under the image dialog: Cmd, still held from the click that opened it, would point at them.
        let hover = self.pointer.filter(|_| cmd && self.preview.is_none() && self.reader.is_none()).and_then(|p| self.link_under(p)).map(|(row, cells, _)| (row, cells));
        if hover != self.hover {
            self.hover = hover;
            cx.notify();
        }
    }

    /// URLs go to the browser (`file://` ones to their default app); paths that exist where the pane's shell runs
    /// (here, or on the machine shown) to the editor; images, either way, to the viewer over the pane, and Markdown
    /// files to the reader (its header opens them in the editor).
    fn open_link(&mut self, link: Link, cx: &mut Context<Self>) {
        let (path, line, col) = match link {
            // `ls --hyperlink` percent-encodes the path (a space is `%20`).
            Link::Url(url) => match url
                .strip_prefix("file://")
                .map(|p| percent_encoding::percent_decode_str(p).decode_utf8_lossy().into_owned())
                .filter(|p| is_image(Path::new(p)) || is_markdown(Path::new(p)))
            {
                Some(path) => (path, None, None),
                None => return cx.open_url(&url),
            },
            Link::File { path, line, col } => (path, line, col),
        };
        let Some(PaneQueries(client, rt)) = cx.try_global::<PaneQueries>() else { return };
        let editor = cx.try_global::<crate::settings::SettingsGlobal>().and_then(|s| s.settings.editor.clone());
        let machine = crate::herdr::shown_machine();
        let (client, pane_id) = (client.clone(), self.pane_id.clone());
        let opened = rt.spawn(async move {
            let pane = client.call("pane.get", serde_json::json!({"pane_id": pane_id})).await.map(|r| r["pane"].clone()).unwrap_or_default();
            let cwd = pane["foreground_cwd"].as_str().or(pane["cwd"].as_str()).unwrap_or("/").to_string();
            let file = File::find(machine, &path, &cwd).await?;
            if is_image(&file.path) {
                return file.local_copy().await.map(Opened::Image);
            }
            if !file.dir && is_markdown(&file.path) {
                let text = String::from_utf8_lossy(&file.bytes().await?).into_owned();
                let blocks = crate::ui::review::markdown(&text);
                return Some(Opened::Markdown { file, cwd, text, blocks });
            }
            // a folder here goes to Finder, not the editor
            if file.dir && file.machine.is_none() {
                let _ = tokio::process::Command::new("open").arg(&file.path).status().await;
                return None;
            }
            edit(&client, &pane_id, &cwd, editor.as_deref(), &file, line, col).await;
            None
        });
        cx.spawn(async move |this, cx| match opened.await {
            Ok(Some(Opened::Image(path))) => {
                let _ = this.update(cx, |pane, cx| pane.show_preview(path, cx));
            }
            Ok(Some(Opened::Markdown { file, cwd, text, blocks })) => {
                let _ = this.update(cx, |pane, cx| pane.show_reader(file, cwd, text, blocks, cx));
            }
            _ => {}
        })
        .detach();
    }

    fn show_reader(&mut self, file: File, cwd: String, text: String, blocks: Vec<crate::ui::review::Block>, cx: &mut Context<Self>) {
        let opened = std::time::Instant::now();
        let reader = Reader { file, cwd, text, blocks, images: Default::default(), scroll: gpui::ScrollHandle::new(), opened, copied: false };
        self.reader = Some(Popup::new(reader));
        self.hover = None;
        self.load_images(cx);
        cx.notify();
    }

    /// Fetches the reader's images into files here, a machine's over SSH and a web one over HTTPS, and shows each
    /// set as it arrives; until then their alt text stands in.
    fn load_images(&mut self, cx: &mut Context<Self>) {
        let Some(r) = self.reader.as_ref().map(|r| &r.value) else { return };
        let mut sources = Vec::new();
        image_sources(&r.blocks, &mut sources);
        let Some(PaneQueries(_, rt)) = cx.try_global::<PaneQueries>().filter(|_| !sources.is_empty()) else { return };
        let (dir, machine, opened) = (r.file.path.parent().map(Path::to_path_buf).unwrap_or_default(), r.file.machine.clone(), r.opened);
        // ponytail: one image after another; in parallel if a page with many over SSH ever loads slowly
        let loaded = rt.spawn(async move {
            let mut images = std::collections::HashMap::new();
            for source in sources {
                let path = if source.starts_with("https://") || source.starts_with("http://") {
                    download(&source).await
                } else {
                    let path = dir.join(percent_encoding::percent_decode_str(&source).decode_utf8_lossy().as_ref());
                    File { path, dir: false, machine: machine.clone() }.local_copy().await
                };
                if let Some(path) = path {
                    images.insert(source, path);
                }
            }
            images
        });
        cx.spawn(async move |this, cx| {
            let Ok(images) = loaded.await else { return };
            let _ = this.update(cx, |pane, cx| {
                if let Some(reader) = pane.reader.as_mut().filter(|r| r.value.opened == opened) {
                    reader.value.images = std::sync::Arc::new(images);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// A link clicked in the reader: a web one to the browser; a path, from the file's folder, opened as a Cmd+click
    /// opens it (another Markdown file in the reader, `#L12` at its line); a bare `#anchor` goes nowhere.
    fn follow_link(&mut self, url: &str, cx: &mut Context<Self>) {
        if url.contains("://") || url.starts_with("mailto:") {
            return cx.open_url(url);
        }
        let (path, anchor) = url.split_once('#').unwrap_or((url, ""));
        let Some(dir) = self.reader.as_ref().and_then(|r| r.value.file.path.parent().map(Path::to_path_buf)).filter(|_| !path.is_empty()) else {
            return;
        };
        let path = dir.join(percent_encoding::percent_decode_str(path).decode_utf8_lossy().as_ref());
        let line = anchor.strip_prefix('L').and_then(|l| l.split('-').next()?.parse().ok());
        self.open_link(Link::File { path: path.to_string_lossy().into_owned(), line, col: None }, cx);
    }

    fn close_reader(&mut self, cx: &mut Context<Self>) {
        if let Some(reader) = &mut self.reader {
            reader.close();
            crate::ui::markdown::forget(READER_KEY);
            cx.notify();
        }
    }

    /// The reader's Markdown to the clipboard, the button checked for a moment.
    fn copy_reader(&mut self, cx: &mut Context<Self>) {
        let Some(reader) = self.reader.as_mut() else { return };
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(reader.value.text.clone()));
        reader.value.copied = true;
        let opened = reader.value.opened;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_millis(1500)).await;
            let _ = this.update(cx, |pane, cx| {
                if let Some(reader) = pane.reader.as_mut().filter(|r| r.value.opened == opened) {
                    reader.value.copied = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// The reader's file in the editor.
    fn edit_reader(&mut self, cx: &mut Context<Self>) {
        let Some((file, cwd)) = self.reader.as_ref().map(|r| (r.value.file.clone(), r.value.cwd.clone())) else { return };
        self.close_reader(cx);
        let Some(PaneQueries(client, rt)) = cx.try_global::<PaneQueries>() else { return };
        let editor = cx.try_global::<crate::settings::SettingsGlobal>().and_then(|s| s.settings.editor.clone());
        let (client, pane_id) = (client.clone(), self.pane_id.clone());
        rt.spawn(async move { edit(&client, &pane_id, &cwd, editor.as_deref(), &file, None, None).await });
    }

    fn show_preview(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // Only the header: the image itself loads as the dialog draws it.
        let size = image::image_dimensions(&path).ok();
        self.preview = Some(Popup::new(Preview { path, size, zoom: 1., pan: (0., 0.), drag: None, opened: std::time::Instant::now() }));
        self.hover = None;
        cx.notify();
    }

    fn close_preview(&mut self, cx: &mut Context<Self>) {
        if let Some(preview) = &mut self.preview {
            preview.close();
            cx.notify();
        }
    }

    fn zoom_key(&mut self, factor: f32, window: &Window, cx: &mut Context<Self>) {
        if self.preview.as_ref().is_some_and(Popup::is_open) {
            self.zoom_preview(factor, None, window, cx);
        } else {
            cx.propagate();
        }
    }

    fn end_preview_drag(&mut self, cx: &mut Context<Self>) {
        if let Some(preview) = self.preview.as_mut().filter(|p| p.value.drag.is_some()) {
            preview.value.drag = None;
            cx.notify();
        }
    }

    /// Scales the open image by `factor` (0: back to the fit), keeping the point under `at` (window coordinates; none: the middle) in place.
    fn zoom_preview(&mut self, factor: f32, at: Option<gpui::Point<Pixels>>, window: &Window, cx: &mut Context<Self>) {
        let Some(preview) = self.preview.as_mut().filter(|p| p.is_open()) else { return };
        let frame = PreviewFrame::of(window, 1.);
        let p = &mut preview.value;
        if factor == 0. {
            (p.zoom, p.pan) = (1., (0., 0.));
        } else {
            p.zoom_about(p.zoom * factor, at.map_or((0., 0.), |at| frame.at(at)), &frame);
        }
        cx.notify();
    }

    /// The image dialog, about the window's size: the image fit inside it, scaled by a pinch, Cmd+scroll, Cmd+=/-/0
    /// or the header's buttons, and moved by scrolling or dragging. Zed's image viewer: no bounds, no glides.
    fn preview(&self, preview: &Popup<Preview>, theme: &Theme, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = &preview.value;
        let frame = PreviewFrame::of(window, grown(preview, p.opened, window));
        let name = p.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let (w, h) = p.shown(&frame);
        let percent = format!("{:.0}%", p.scale(&frame) * 100.);
        let card = dialog_card("image-preview", &frame, theme)
            .on_mouse_down_out(cx.listener(|pane, _, _, cx| pane.close_preview(cx)))
            .child(
                dialog_header(name, theme)
                    .child(dialog_button("image-zoom-out", "icons/minus.svg", theme).on_click(cx.listener(|pane, _, window, cx| pane.zoom_preview(1. / ZOOM_STEP, None, window, cx))))
                    .child(
                        div()
                            .id("image-zoom-fit")
                            .flex_none()
                            .w(px(40.))
                            .text_center()
                            .text_size(px(11.))
                            .text_color(theme.text_muted)
                            .cursor_pointer()
                            .child(percent)
                            .on_click(cx.listener(|pane, _, window, cx| pane.zoom_preview(0., None, window, cx))),
                    )
                    .child(dialog_button("image-zoom-in", "icons/plus.svg", theme).on_click(cx.listener(|pane, _, window, cx| pane.zoom_preview(ZOOM_STEP, None, window, cx))))
                    .child(dialog_button("image-preview-close", "icons/close.svg", theme).ml(px(6.)).on_click(cx.listener(|pane, _, _, cx| pane.close_preview(cx)))),
            )
            .child(
                div()
                    .id("image-preview-frame")
                    .relative()
                    .flex_1()
                    .overflow_hidden()
                    .rounded(px(6.))
                    .when(p.drag.is_some(), |el| el.cursor(gpui::CursorStyle::ClosedHand))
                    .on_pinch(cx.listener(|pane, e: &gpui::PinchEvent, window, cx| pane.zoom_preview(1. + e.delta, Some(e.position), window, cx)))
                    .on_scroll_wheel(cx.listener(|pane, e: &ScrollWheelEvent, window, cx| {
                        let d = e.delta.pixel_delta(px(20.));
                        if e.modifiers.platform || e.modifiers.control {
                            let dy = f32::from(d.y);
                            let factor = if dy > 0. { 1. + dy.abs() * 0.01 } else { 1. / (1. + dy.abs() * 0.01) };
                            return pane.zoom_preview(factor, Some(e.position), window, cx);
                        }
                        let Some(preview) = pane.preview.as_mut().filter(|p| p.is_open()) else { return };
                        preview.value.pan_by((f32::from(d.x), f32::from(d.y)), &PreviewFrame::of(window, 1.));
                        cx.notify();
                    }))
                    .on_mouse_down(MouseButton::Left, cx.listener(|pane, e: &MouseDownEvent, _, cx| {
                        if let Some(preview) = pane.preview.as_mut() {
                            preview.value.drag = Some(e.position);
                            cx.notify();
                        }
                    }))
                    .on_mouse_move(cx.listener(|pane, e: &MouseMoveEvent, window, cx| {
                        let Some(p) = pane.preview.as_mut().map(|p| &mut p.value) else { return };
                        // The button's own state, not the mouse up, which can go missing (seen after a Cmd+click).
                        if e.pressed_button != Some(MouseButton::Left) {
                            if p.drag.take().is_some() {
                                cx.notify();
                            }
                            return;
                        }
                        let Some(from) = p.drag.replace(e.position) else { return };
                        p.pan_by((f32::from(e.position.x - from.x), f32::from(e.position.y - from.y)), &PreviewFrame::of(window, 1.));
                        cx.notify();
                    }))
                    .on_mouse_up(MouseButton::Left, cx.listener(|pane, _, _, cx| pane.end_preview_drag(cx)))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(|pane, _, _, cx| pane.end_preview_drag(cx)))
                    // A double click goes between the fit and twice it, about the pointer.
                    .on_click(cx.listener(|pane, e: &gpui::ClickEvent, window, cx| {
                        if e.click_count() == 2 {
                            let zoomed = pane.preview.as_ref().is_some_and(|p| p.value.zoom != 1. || p.value.pan != (0., 0.));
                            pane.zoom_preview(if zoomed { 0. } else { 2. }, Some(e.position()), window, cx);
                        }
                    }))
                    .child(
                        gpui::img(p.path.clone())
                            .absolute()
                            .left(px((frame.area.0 - w) / 2. + p.pan.0))
                            .top(px((frame.area.1 - h) / 2. + p.pan.1))
                            .w(px(w))
                            .h(px(h))
                            .object_fit(gpui::ObjectFit::Contain)
                            .with_fallback(|| div().size_full().flex().items_center().justify_center().child("Can't show this image").into_any_element()),
                    ),
            );
        crate::shell::modal("image-preview", preview, window, card)
    }

    /// The Markdown dialog: the image dialog's card with the file rendered in a column, scrolled.
    fn reader(&self, reader: &Popup<Reader>, theme: &Theme, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let r = &reader.value;
        let frame = PreviewFrame::of(window, grown(reader, r.opened, window));
        let name = r.file.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let (images, pane) = (r.images.clone(), cx.entity().downgrade());
        let look = crate::ui::markdown::Look {
            document: true,
            image: Some(std::rc::Rc::new(move |source: &str| images.get(source).cloned())),
            link: Some(std::rc::Rc::new(move |url: &str, _: &mut Window, cx: &mut App| {
                let _ = pane.update(cx, |pane, cx| pane.follow_link(url, cx));
            })),
        };
        let card = dialog_card("markdown-reader", &frame, theme)
            .on_mouse_down_out(cx.listener(|pane, _, _, cx| pane.close_reader(cx)))
            .child(
                dialog_header(name, theme)
                    .child(
                        dialog_button("markdown-reader-copy", if r.copied { "icons/check.svg" } else { "icons/solar/copy.svg" }, theme)
                            .on_click(cx.listener(|pane, _, _, cx| pane.copy_reader(cx))),
                    )
                    .child(
                        div()
                            .id("markdown-reader-edit")
                            .flex_none()
                            .h(px(22.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .text_size(px(11.5))
                            .text_color(theme.text_muted)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.hover).text_color(theme.text))
                            .child("Open in editor")
                            .on_click(cx.listener(|pane, _, _, cx| pane.edit_reader(cx))),
                    )
                    .child(dialog_button("markdown-reader-close", "icons/close.svg", theme).ml(px(4.)).on_click(cx.listener(|pane, _, _, cx| pane.close_reader(cx)))),
            )
            .child(
                // the text fades where it scrolls under the edges, the app's overlay bar on the right
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(crate::ui::edge_fade::edge_faded(
                        12.,
                        &r.scroll,
                        div().id("markdown-reader-body").flex_1().min_h_0().overflow_y_scroll().track_scroll(&r.scroll).child(
                            div()
                                .max_w(px(800.))
                                .mx_auto()
                                .px(px(24.))
                                .pt(px(4.))
                                .pb(px(24.))
                                .flex()
                                .flex_col()
                                .gap(px(12.))
                                .text_size(px(14.))
                                .line_height(px(22.))
                                .child(crate::ui::markdown::frame_reset())
                                .children(crate::ui::markdown::render_with(&r.blocks, READER_KEY, theme, &look)),
                        ),
                    ))
                    .child(crate::ui::scrollbar("markdown-reader-scrollbar", &r.scroll, theme)),
            );
        crate::shell::modal("markdown-reader", reader, window, card)
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

pub struct Preview {
    path: PathBuf,
    /// In pixels, from the file's header; none for an SVG or a file that does not decode.
    size: Option<(u32, u32)>,
    /// Times the fit.
    zoom: f32,
    /// The image's middle from the frame's, in px.
    pan: (f32, f32),
    /// Where the pointer was, while a drag moves the image.
    drag: Option<gpui::Point<Pixels>>,
    opened: std::time::Instant,
}

/// Quick Look's entrance and exit for a dialog opened at `opened`: the card grows into place as it fades in, and
/// shrinks a little leaving. 1 is in place.
fn grown<T>(popup: &Popup<T>, opened: std::time::Instant, window: &Window) -> f32 {
    let opening = (opened.elapsed().as_secs_f32() / PREVIEW_IN.as_secs_f32()).min(1.);
    let closing = popup.closed_for().map_or(0., |d| (d.as_secs_f32() / 0.1).min(1.));
    if opening < 1. {
        window.request_animation_frame();
    }
    0.9 + 0.1 * (1. - (1. - opening).powi(3)) - 0.04 * closing
}

/// A Markdown file in the reader.
pub struct Reader {
    file: File,
    /// The pane's shell's folder, where Neovim opens beside it.
    cwd: String,
    /// The file's Markdown, for the copy button.
    text: String,
    blocks: Vec<crate::ui::review::Block>,
    /// The images fetched so far, by their source in the file.
    images: std::sync::Arc<std::collections::HashMap<String, PathBuf>>,
    scroll: gpui::ScrollHandle,
    opened: std::time::Instant,
    /// The copy button shows a check for a moment after a copy.
    copied: bool,
}

/// The images' sources in `blocks`, each once, in document order.
fn image_sources(blocks: &[crate::ui::review::Block], out: &mut Vec<String>) {
    use crate::ui::review::Block;
    let runs = |runs: &[crate::ui::review::InlineRun], out: &mut Vec<String>| {
        for image in runs.iter().filter_map(|r| r.style.image.as_ref()) {
            if !out.contains(&image.source) {
                out.push(image.source.clone());
            }
        }
    };
    for block in blocks {
        match block {
            Block::Paragraph { runs: r } | Block::Heading { runs: r, .. } => runs(r, out),
            Block::BlockQuote { children } => image_sources(children, out),
            Block::List { items, .. } => items.iter().for_each(|item| image_sources(item, out)),
            Block::Table { header, rows, .. } => header.iter().chain(rows.iter().flatten()).for_each(|cell| runs(cell, out)),
            _ => {}
        }
    }
}

/// A web image, downloaded into the cache under the extension its type has (a badge's URL often has none).
async fn download(url: &str) -> Option<PathBuf> {
    let url = url.to_string();
    let fetched = tokio::task::spawn_blocking(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(15))).build().into();
        let mut response = agent.get(&url).call().ok()?;
        let kind = response.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
        let bytes = response.body_mut().with_config().limit(32 << 20).read_to_vec().ok()?;
        let ext = ["svg", "png", "gif", "webp", "jpeg"].into_iter().find(|e| kind.contains(e)).unwrap_or("png");
        Some((bytes, format!("image.{ext}")))
    });
    let (bytes, name) = fetched.await.ok()??;
    cache_copy(&name, bytes).await
}

/// `bytes` as a file named `name` in the cache, under their hash: gpui keeps an image by its path, so a changed file
/// needs a new one.
// ponytail: copies pile up in the cache until it is cleared; prune by age if they ever matter.
async fn cache_copy(name: &str, bytes: Vec<u8>) -> Option<PathBuf> {
    let hash = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut h);
        h.finish()
    };
    let dir = crate::settings::cache_dir().join("files").join(format!("{hash:016x}"));
    tokio::fs::create_dir_all(&dir).await.ok()?;
    let copy = dir.join(name);
    tokio::fs::write(&copy, bytes).await.ok()?;
    Some(copy)
}

/// The reader's markdown selection keys start with it.
const READER_KEY: &str = "reader-md";

/// What a Cmd+click opens over the pane.
enum Opened {
    Image(PathBuf),
    Markdown { file: File, cwd: String, text: String, blocks: Vec<crate::ui::review::Block> },
}

/// A clicked file where the pane's shell runs: here, or on the machine at `machine` (its SSH target).
#[derive(Clone)]
struct File {
    path: PathBuf,
    dir: bool,
    machine: Option<String>,
}

impl File {
    async fn find(machine: Option<String>, path: &str, cwd: &str) -> Option<File> {
        let Some(target) = machine else {
            let path = resolve(path, Path::new(cwd))?;
            return Some(File { dir: path.is_dir(), path, machine: None });
        };
        let (t, path, cwd) = (target.clone(), path.to_string(), cwd.to_string());
        let (found, dir) = tokio::task::spawn_blocking(move || crate::herdr::find_file(&t, &cwd, &path)).await.ok()??;
        Some(File { path: PathBuf::from(found), dir, machine: Some(target) })
    }

    /// Its bytes, a machine's over SSH.
    async fn bytes(&self) -> Option<Vec<u8>> {
        let Some(target) = self.machine.clone() else { return tokio::fs::read(&self.path).await.ok() };
        let path = self.path.to_string_lossy().into_owned();
        tokio::task::spawn_blocking(move || crate::herdr::read_file(&target, &path)).await.ok()?.ok()
    }

    /// The file as one here for the image viewer: itself, or a machine's copied into the cache.
    async fn local_copy(&self) -> Option<PathBuf> {
        if self.machine.is_none() {
            return Some(self.path.clone());
        }
        let name = self.path.file_name()?.to_string_lossy().into_owned();
        cache_copy(&name, self.bytes().await?).await
    }
}

/// Opens `file` at its line in the user's editor: Neovim in a pane to the right of `pane_id`, where the pane's shell
/// runs; another editor here, over its SSH remoting for a machine's file.
async fn edit(client: &Client, pane_id: &str, cwd: &str, editor: Option<&str>, file: &File, line: Option<u32>, col: Option<u32>) {
    if is_nvim(editor) {
        // A terminal editor gets a pane to the right; its shell drops typeahead until the prompt is drawn.
        let params = serde_json::json!({"target_pane_id": pane_id, "direction": "right", "focus": true, "cwd": cwd});
        let Ok(r) = client.call("pane.split", params).await else { return };
        let Some(new_pane) = r["pane"]["pane_id"].as_str() else { return };
        let prompt = serde_json::json!({"type": "regex", "value": "\\S"});
        let wait = serde_json::json!({"pane_id": new_pane, "source": "recent", "match": prompt, "timeout_ms": 5000});
        let _ = client.call("pane.wait_for_output", wait).await;
        let text = nvim_command(editor.unwrap_or("nvim"), &file.path, line, col);
        let _ = client.call("pane.send_input", serde_json::json!({"pane_id": new_pane, "text": text, "keys": ["enter"]})).await;
        return;
    }
    let Some(argv) = editor_argv(editor, file.machine.as_deref(), &file.path, line, col) else { return };
    if let Ok(mut child) = tokio::process::Command::new(&argv[0]).args(&argv[1..]).spawn() {
        let _ = child.wait().await;
    }
}

const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 20.;
const ZOOM_STEP: f32 = 1.25;
const PREVIEW_IN: std::time::Duration = std::time::Duration::from_millis(220);
const PREVIEW_PAD: f32 = 8.;
const PREVIEW_HEADER: f32 = 22.;
const PREVIEW_GAP: f32 = 6.;

/// The image dialog's and the reader's card, `frame`'s size.
fn dialog_card(id: &'static str, frame: &PreviewFrame, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .w(px(frame.card.0))
        .h(px(frame.card.1))
        .p(px(PREVIEW_PAD))
        .rounded(px(12.))
        .bg(theme.card)
        .border_1()
        .border_color(theme.border)
        .shadow_lg()
        .flex()
        .flex_col()
        .gap(px(PREVIEW_GAP))
        .text_color(theme.text)
}

/// A dialog's header row: the file's name, then the controls added to it.
fn dialog_header(name: String, theme: &Theme) -> gpui::Div {
    div()
        .h(px(PREVIEW_HEADER))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(2.))
        .pl(px(6.))
        .child(div().flex_1().min_w_0().truncate().text_size(px(12.)).text_color(theme.text_muted).child(name))
}

/// A header's icon button, ghost until hovered.
fn dialog_button(id: &'static str, icon: &'static str, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .size(px(22.))
        .rounded(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(theme.hover))
        .child(gpui::svg().path(icon).size(px(14.)).text_color(theme.text_muted))
}

/// The dialog's geometry in a window: the card's size, its image frame's, and the frame's middle on the window.
struct PreviewFrame {
    card: (f32, f32),
    area: (f32, f32),
    center: (f32, f32),
}

impl PreviewFrame {
    /// The geometry `grown` (1: in place) of the way in.
    fn of(window: &Window, grown: f32) -> Self {
        let v = window.viewport_size();
        let (vw, vh) = (f32::from(v.width), f32::from(v.height));
        let card = ((vw * 0.85).max(320.) * grown, (vh * 0.85).max(240.) * grown);
        // the card's border is 1 px a side
        let area = (card.0 - 2. * PREVIEW_PAD - 2., card.1 - 2. * PREVIEW_PAD - 2. - PREVIEW_HEADER - PREVIEW_GAP);
        let center = (vw / 2., (vh - card.1) / 2. + 1. + PREVIEW_PAD + PREVIEW_HEADER + PREVIEW_GAP + area.1 / 2.);
        PreviewFrame { card, area, center }
    }

    /// A window point from the frame's middle.
    fn at(&self, p: gpui::Point<Pixels>) -> (f32, f32) {
        (f32::from(p.x) - self.center.0, f32::from(p.y) - self.center.1)
    }
}

impl Preview {
    /// The fit: inside the frame, never past the image's own pixels. An SVG fills the frame.
    fn fit(&self, f: &PreviewFrame) -> (f32, f32) {
        match self.size {
            Some((w, h)) => {
                let k = (f.area.0 / w as f32).min(f.area.1 / h as f32).min(1.);
                (w as f32 * k, h as f32 * k)
            }
            None => f.area,
        }
    }

    fn shown(&self, f: &PreviewFrame) -> (f32, f32) {
        let (w, h) = self.fit(f);
        (w * self.zoom, h * self.zoom)
    }

    /// Shown px per image pixel (per the fit, for an SVG).
    fn scale(&self, f: &PreviewFrame) -> f32 {
        self.size.map_or(self.zoom, |(w, _)| self.shown(f).0 / w as f32)
    }

    /// Scales to `zoom` keeping the image's point under `at` (from the frame's middle) in place, as far as
    /// the frame lets it.
    fn zoom_about(&mut self, zoom: f32, at: (f32, f32), f: &PreviewFrame) {
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        let k = zoom / self.zoom;
        let pan = (at.0 - (at.0 - self.pan.0) * k, at.1 - (at.1 - self.pan.1) * k);
        (self.zoom, self.pan) = (zoom, (0., 0.));
        self.pan_by(pan, f);
    }

    /// Moves the image by `d`, Preview's way: a larger one keeps the frame covered, a smaller one stays inside it.
    fn pan_by(&mut self, d: (f32, f32), f: &PreviewFrame) {
        let (w, h) = self.shown(f);
        let (mx, my) = ((w - f.area.0).abs() / 2., (h - f.area.1).abs() / 2.);
        self.pan = ((self.pan.0 + d.0).clamp(-mx, mx), (self.pan.1 + d.1).clamp(-my, my));
    }
}

/// Whether the viewer can show the file: the formats gpui decodes.
fn is_image(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
    matches!(ext.as_deref(), Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "svg" | "ico"))
}

/// Whether the reader shows the file.
fn is_markdown(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
    matches!(ext.as_deref(), Some("md" | "markdown" | "mdx"))
}

/// A clicked path as a file that exists: `~` is home, a relative path is the pane's shell's.
fn resolve(path: &str, cwd: &Path) -> Option<PathBuf> {
    let path = match path.strip_prefix("~/") {
        Some(rest) => std::env::home_dir()?.join(rest),
        None => cwd.join(path),
    };
    path.exists().then_some(path)
}

/// The command opening `path` at its line: the user's editor, else the first of Zed, Cursor and VS Code installed,
/// else macOS's default app for the file (which cannot take a line). A file on the machine at `remote` (its SSH
/// target) opens over the editor's SSH remoting, Zed's `ssh://` URL or VS Code's and Cursor's `--remote`; `None`
/// for an editor without it.
pub fn editor_argv(editor: Option<&str>, remote: Option<&str>, path: &Path, line: Option<u32>, col: Option<u32>) -> Option<Vec<String>> {
    let target = match (line, col) {
        (Some(l), Some(c)) => format!("{}:{l}:{c}", path.display()),
        (Some(l), None) => format!("{}:{l}", path.display()),
        _ => path.display().to_string(),
    };
    // An app opened from Finder gets launchd's PATH, without the usual homes of these CLIs.
    let path_env = std::env::var("PATH").unwrap_or_default();
    let installed = |name: &str| {
        let dirs = path_env.split(':').chain(["/usr/local/bin", "/opt/homebrew/bin"]);
        dirs.map(|d| Path::new(d).join(name)).find(|p| p.is_file()).map(|p| p.display().to_string())
    };
    let words = match editor.map(|e| e.split_whitespace().map(String::from).collect::<Vec<_>>()).filter(|w| !w.is_empty()) {
        Some(words) => {
            let program = if words[0].contains('/') { None } else { installed(&words[0]) };
            [vec![program.unwrap_or_else(|| words[0].clone())], words[1..].to_vec()].concat()
        }
        None => match (installed("zed"), installed("cursor").or_else(|| installed("code"))) {
            (Some(zed), _) => vec![zed],
            (None, Some(code)) => vec![code, "-g".into()],
            (None, None) => return remote.is_none().then(|| vec!["open".into(), path.display().to_string()]),
        },
    };
    let Some(host) = remote.map(|r| r.trim_start_matches("ssh://")) else { return Some([words, vec![target]].concat()) };
    match Path::new(&words[0]).file_name().and_then(|n| n.to_str()) {
        Some("zed") => Some([words, vec![format!("ssh://{host}{target}")]].concat()),
        Some("code" | "cursor" | "codium" | "windsurf") => {
            // `-g` wants a line; a folder or a whole file opens without it
            let flags = words[1..].iter().filter(|w| line.is_some() || *w != "-g").cloned();
            let remote = ["--remote".to_string(), format!("ssh-remote+{host}")];
            Some(words[..1].iter().cloned().chain(remote).chain(flags).chain([target]).collect())
        }
        _ => None,
    }
}

/// Whether the editor command runs nvim, which opens in a pane instead of its own window.
pub fn is_nvim(editor: Option<&str>) -> bool {
    editor.and_then(|e| e.split_whitespace().next()).is_some_and(|w| w.rsplit('/').next() == Some("nvim"))
}

/// The shell line opening `path` at its line and column in nvim; `exec`, so quitting nvim closes its pane.
pub fn nvim_command(editor: &str, path: &Path, line: Option<u32>, col: Option<u32>) -> String {
    let quoted = format!("'{}'", path.display().to_string().replace('\'', "'\\''"));
    let jump = match (line, col) {
        (Some(l), Some(c)) => format!(" '+call cursor({l},{c})'"),
        (Some(l), None) => format!(" +{l}"),
        _ => String::new(),
    };
    format!("exec {editor}{jump} {quoted}")
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

    #[test]
    fn preview_fits_and_zooms_about_the_pointer() {
        let frame = PreviewFrame { card: (0., 0.), area: (400., 300.), center: (0., 0.) };
        let mut p = Preview {
            path: "a.png".into(),
            size: Some((800, 400)),
            zoom: 1.,
            pan: (0., 0.),
            drag: None,
            opened: std::time::Instant::now(),
        };
        assert_eq!(p.fit(&frame), (400., 200.));
        // The point under the pointer stays: 100 px right of the middle, zoomed 2×, the image moves 100 px left.
        p.zoom_about(2., (100., 0.), &frame);
        assert_eq!((p.zoom, p.pan), (2., (-100., 0.)));
        assert_eq!(p.scale(&frame), 1.);
        // Dragged, it keeps the frame covered: 800×400 over 400×300 overhangs 200 and 50.
        p.pan_by((1000., 1000.), &frame);
        assert_eq!(p.pan, (200., 50.));
        // Smaller than the frame, it stays inside it: at 400×200 it fits exactly across, 50 px of room down.
        p.zoom_about(1., (0., 0.), &frame);
        p.pan_by((-1000., -1000.), &frame);
        assert_eq!(p.pan, (0., -50.));
        p.zoom_about(1000., (0., 0.), &frame);
        assert_eq!(p.zoom, MAX_ZOOM);
        // a small one is not blown up
        assert_eq!(Preview { size: Some((40, 20)), ..p }.fit(&frame), (40., 20.));
    }

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
