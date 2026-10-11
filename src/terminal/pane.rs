//! One herdr pane as a GPUI terminal: a `herdr terminal session control` stream fed into Zeron's emulator and
//! painted by its grid element. Keys, wheel and mouse go back to herdr as session commands.
//! `GridGeometry` and the selection gestures are adapted from Zeron's crates/ui/src/terminal/panel.rs (MIT, see
//! assets/NOTICE).
use super::element::{SELECTION_DRAG_THRESHOLD, TERM_LINE_HEIGHT, TerminalElement, cell_at};
use super::emulator::{CellSnapshot, CursorSnapshot, Emulator, Side};
use super::find::{Link, find_all, link_at};
use super::keys::{keystroke_bytes, paste_bytes};
use crate::herdr::{Client, Session, SessionEvent};
use crate::theme::Theme;
use alacritty_terminal::index::Point as GridPoint;
use alacritty_terminal::selection::SelectionType;
use gpui::{
    App, AsyncApp, ClipboardItem, Context, Entity, FocusHandle, Focusable, IntoElement, KeyDownEvent, Modifiers,
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
}

pub struct GridSnapshot {
    pub lines: Vec<Vec<CellSnapshot>>,
    pub cursor: Option<CursorSnapshot>,
    /// Search matches on screen: (row, cells, the current one).
    pub marks: Vec<(usize, Range<usize>, bool)>,
}

/// While true (a divider or the sidebar is being dragged), panes reflow at most every `HELD_RESIZE`, so the
/// terminal follows the drag without a SIGWINCH per pixel; the drag's end re-measures them all.
pub struct GridHold(pub bool);

const HELD_RESIZE: std::time::Duration = std::time::Duration::from_millis(80);

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
struct Search {
    input: Entity<InputState>,
    /// Matches in herdr's history: line from the oldest one, char range.
    hits: Vec<(usize, Range<usize>)>,
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
    /// The last grid resize, to pace them under `GridHold`.
    resized_at: Option<std::time::Instant>,
    focus: FocusHandle,
    wheel_acc: f32,
    drag: Option<Drag>,
    /// The pane's app asks for mouse reporting (from herdr; false until it says otherwise).
    mouse_capture: bool,
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
            resized_at: None,
            focus: cx.focus_handle(),
            wheel_acc: 0.0,
            drag: None,
            mouse_capture: false,
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

    /// Connects off the main thread: herdr's handshake takes tens of milliseconds, and attaching happens while a
    /// frame is laid out (a tab switch attaches every pane it shows).
    fn attach(&mut self, takeover: bool, cx: &mut Context<Self>) {
        let (cols, rows) = (self.emulator.cols() as u16, self.emulator.rows() as u16);
        let pane_id = self.pane_id.clone();
        let spawn = cx.background_executor().spawn(async move { Session::spawn(&pane_id, cols, rows, takeover) });
        self._pump = Some(cx.spawn(async move |this, cx| {
            let result = spawn.await;
            let rx = this.update(cx, |pane, cx| {
                cx.notify();
                match result {
                    Ok((session, rx)) => {
                        // The grid may have changed size while connecting.
                        let now = (pane.emulator.cols() as u16, pane.emulator.rows() as u16);
                        if now != (cols, rows) {
                            session.resize(now.0, now.1);
                        }
                        pane.session = Some(session);
                        pane.mouse_capture = false;
                        pane.ended = None;
                        Some(rx)
                    }
                    Err(err) => {
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

    /// Element hook: this frame's grid placement. A size change resizes the emulator and herdr's view.
    pub fn on_grid_metrics(&mut self, geometry: GridGeometry, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.scroll_due) {
            self.refresh_scroll(cx);
        }
        let first = self.geometry.is_none();
        let (cols, rows) = (geometry.cols, geometry.rows);
        let resized = self.emulator.cols() != cols as usize || self.emulator.rows() != rows as usize;
        let held = cx.try_global::<GridHold>().is_some_and(|hold| hold.0);
        if !first && resized && held && self.resized_at.is_some_and(|at| at.elapsed() < HELD_RESIZE) {
            return; // a drag is under way and the grid just reflowed: the next frame (or the drop) catches up
        }
        self.geometry = Some(geometry);
        if resized {
            self.emulator.resize(cols, rows);
            self.resized_at = Some(std::time::Instant::now());
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

    pub fn active_grid_snapshot(&self, cx: &App) -> Option<GridSnapshot> {
        let mut lines = self.emulator.lines();
        if let Some((row, cells)) = &self.hover {
            for cell in lines.get_mut(*row).into_iter().flat_map(|l| l[cells.clone()].iter_mut()) {
                cell.underline = true;
            }
        }
        let marks = self.search_marks(&lines, cx);
        Some(GridSnapshot { lines, cursor: self.emulator.cursor(), marks })
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
            self.search = Some(Search { input, hits: Vec::new(), current: None, _events: events, _read: None });
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
            let read = client.call("pane.read", serde_json::json!({"pane_id": pane_id, "source": "recent", "lines": u32::MAX})).await;
            let text = read.ok().and_then(|r| r["read"]["text"].as_str().map(str::to_owned)).unwrap_or_default();
            let lines = text.lines().enumerate();
            lines.flat_map(|(i, l)| find_all(&l.chars().collect::<Vec<_>>(), &query).into_iter().map(move |r| (i, r))).collect::<Vec<_>>()
        });
        search._read = Some(cx.spawn(async move |this, cx| {
            let Ok(hits) = read.await else { return };
            let _ = this.update(cx, |pane, cx| {
                let s = pane.scroll;
                let bottom = (s.max_offset_from_bottom - s.offset_from_bottom.min(s.max_offset_from_bottom)) as usize + pane.emulator.rows();
                let Some(search) = &mut pane.search else { return };
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
            Some(offset) => self.scroll_by(offset as i64 - self.scroll.offset_from_bottom as i64, cx),
            None => cx.notify(),
        }
    }

    fn search_bar(&self, search: &Search, theme: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let count = match (search.current, search.hits.len()) {
            (Some(i), n) => format!("{}/{n}", i + 1),
            _ if search.input.read(cx).value().is_empty() => String::new(),
            _ => "No results".into(),
        };
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
                    "g" if k.modifiers.platform => pane.step(!k.modifiers.shift, cx),
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
        let cells = self.emulator.line(row as usize);
        let (chars, cols) = row_chars(&cells);
        let at = cols.iter().rposition(|&c| c <= col as usize)?;
        let (chars_range, link) = link_at(&chars, at)?;
        Some((row as usize, cell_range(&cells, &cols, chars_range), link))
    }

    fn update_hover(&mut self, cmd: bool, cx: &mut Context<Self>) {
        let hover = self.pointer.filter(|_| cmd).and_then(|p| self.link_under(p)).map(|(row, cells, _)| (row, cells));
        if hover != self.hover {
            self.hover = hover;
            cx.notify();
        }
    }

    /// URLs go to the browser; paths that exist (relative to the pane's shell) to the editor.
    fn open_link(&self, link: Link, cx: &mut Context<Self>) {
        let (path, line, col) = match link {
            Link::Url(url) => return cx.open_url(&url),
            Link::File { path, line, col } => (path, line, col),
        };
        let Some(PaneQueries(client, rt)) = cx.try_global::<PaneQueries>() else { return };
        let editor = cx.try_global::<crate::settings::SettingsGlobal>().and_then(|s| s.settings.editor.clone());
        let (client, pane_id) = (client.clone(), self.pane_id.clone());
        rt.spawn(async move {
            let pane = client.call("pane.get", serde_json::json!({"pane_id": pane_id})).await.map(|r| r["pane"].clone()).unwrap_or_default();
            let cwd = pane["foreground_cwd"].as_str().or(pane["cwd"].as_str()).unwrap_or("/");
            let Some(path) = resolve(&path, Path::new(cwd)) else { return };
            if editor.as_deref().and_then(|e| e.split_whitespace().next()).is_some_and(|w| w.rsplit('/').next() == Some("nvim")) {
                // A terminal editor gets a pane to the right; its shell drops typeahead until the prompt is drawn.
                let params = serde_json::json!({"target_pane_id": pane_id, "direction": "right", "focus": true, "cwd": cwd});
                let Ok(r) = client.call("pane.split", params).await else { return };
                let Some(new_pane) = r["pane"]["pane_id"].as_str() else { return };
                let prompt = serde_json::json!({"type": "regex", "value": "\\S"});
                let wait = serde_json::json!({"pane_id": new_pane, "source": "recent", "match": prompt, "timeout_ms": 5000});
                let _ = client.call("pane.wait_for_output", wait).await;
                let text = nvim_command(editor.as_deref().unwrap_or("nvim"), &path, line, col);
                let _ = client.call("pane.send_input", serde_json::json!({"pane_id": new_pane, "text": text, "keys": ["enter"]})).await;
                return;
            }
            let argv = editor_argv(editor.as_deref(), &path, line, col);
            if let Ok(mut child) = tokio::process::Command::new(&argv[0]).args(&argv[1..]).spawn() {
                let _ = child.wait().await;
            }
        });
    }

    fn input(&self, bytes: &[u8]) {
        if let Some(session) = &self.session {
            session.input(bytes);
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.focus.is_focused(window) {
            return; // typing in the search bar
        }
        let ks = &event.keystroke;
        let mods = &ks.modifiers;
        if ks.key == "f" && mods.platform {
            self.open_search(window, cx);
            cx.stop_propagation();
            return;
        }
        if ks.key == "g" && mods.platform && self.search.is_some() {
            self.step(!mods.shift, cx);
            cx.stop_propagation();
            return;
        }
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
        if let Some((_, _, link)) = self.link_under(event.position).filter(|_| event.modifiers.platform) {
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
        self.pointer = Some(event.position);
        if !event.dragging() {
            self.update_hover(event.modifiers.platform, cx);
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

    fn on_mouse_up(&mut self, event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else { return };
        if drag.to_app {
            self.mouse("up", event.position, &event.modifiers);
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

/// A clicked path as a file that exists: `~` is home, a relative path is the pane's shell's.
fn resolve(path: &str, cwd: &Path) -> Option<PathBuf> {
    let path = match path.strip_prefix("~/") {
        Some(rest) => PathBuf::from(std::env::var_os("HOME")?).join(rest),
        None => cwd.join(path),
    };
    path.exists().then_some(path)
}

/// The command opening `path` at its line: the user's editor, else the first of Zed, Cursor and VS Code installed,
/// else macOS's default app for the file (which cannot take a line).
pub fn editor_argv(editor: Option<&str>, path: &Path, line: Option<u32>, col: Option<u32>) -> Vec<String> {
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
    if let Some(words) = editor.map(|e| e.split_whitespace().map(String::from).collect::<Vec<_>>()).filter(|w| !w.is_empty()) {
        let program = if words[0].contains('/') { None } else { installed(&words[0]) };
        return [vec![program.unwrap_or_else(|| words[0].clone())], words[1..].to_vec(), vec![target]].concat();
    }
    if let Some(zed) = installed("zed") {
        return vec![zed, target];
    }
    if let Some(code) = installed("cursor").or_else(|| installed("code")) {
        return vec![code, "-g".into(), target];
    }
    vec!["open".into(), path.display().to_string()]
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
        let search_bar = self.search.as_ref().map(|search| self.search_bar(search, &theme, cx));
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
            .on_modifiers_changed(cx.listener(|pane, e: &ModifiersChangedEvent, _, cx| pane.update_hover(e.modifiers.platform, cx)))
            .when(self.hover.is_some(), |el| el.cursor_pointer())
            .child(TerminalElement::new(cx.entity(), focused))
            .child(crate::ui::scrollbar(SharedString::from(format!("scrollbar-{}", self.pane_id)), &self.scrollbar, &theme))
            .children(search_bar)
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
        assert_eq!(editor_argv(Some("/bin/ed -g"), path, Some(3), Some(4)), ["/bin/ed", "-g", "/tmp/a.rs:3:4"]);
        assert_eq!(editor_argv(Some("/bin/ed"), path, Some(3), None), ["/bin/ed", "/tmp/a.rs:3"]);
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
