//! Adapted from Zeron's crates/ui/src/terminal/emulator.rs (MIT, see assets/NOTICE).
//! The terminal emulator core: Ghostty's libghostty-vt (the `ghostty-vt` crate, built from Ghostty's source with
//! Zig 0.16) wrapped as a pure state machine.
//!
//! Bytes in ([`Emulator::feed`] — the decoded `SubscribeTerminal` Data frames),
//! grid snapshots out ([`Emulator::line`], [`Emulator::cursor`]). No I/O, no
//! timers, no gpui: the panel owns RPC and scheduling, the view owns paint.
//! That split makes the whole escape-sequence surface unit-testable with
//! scripted byte strings.
//!
//! Selection lives here too ([`Emulator::start_selection`] and friends): the
//! terminal holds the installed selection and keeps it on its text as output
//! moves the grid; word and line bounds are Ghostty's own rules.
//!
//! API notes for the pinned `ghostty-vt 0.7.0` (Ghostty `b40acce`):
//! - Snapshots read through a `RenderState` (the C API's render path); the
//!   handles are `!Send`, which suits an emulator living inside a gpui entity.
//! - Query responses (DSR/DA/…) surface as `Event::PtyWrite` after `vt_write`;
//!   [`Emulator::feed`] returns them so the panel can write them back.

use std::cell::RefCell;

use ghostty_vt::render::{CellIterator, Dirty, RowIterator};
use ghostty_vt::screen::{CellContentTag, CellWide, GridRef, Screen};
use ghostty_vt::selection::{FormatOptions, SelectLineOptions, SelectWordBetweenOptions, Selection};
use ghostty_vt::style::{Style, StyleColor, Underline};
use ghostty_vt::terminal::{Mode, Point, PointCoordinate, PointSpace, ScrollViewport};
use ghostty_vt::{Error, Event, RenderState, Terminal, TerminalOptions};

/// A cell in viewport coordinates (row 0 = top). herdr owns scrollback, so the
/// viewport is the whole local grid and a point needs no history offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GridPoint {
    pub row: usize,
    pub col: usize,
}

/// Which half of a cell the pointer is on: a drag starting on a cell's right
/// half leaves that cell out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// Selection granularity: a drag, a double-click word, a triple-click row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionType {
    Simple,
    Semantic,
    Lines,
}

/// A cell's paint color, decoupled from the palette: the view resolves these
/// against the theme (default fg/bg, 256-color index, or direct RGB).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellColor {
    /// Default foreground.
    Foreground,
    /// Default background.
    Background,
    /// Indexed color: 0-15 ANSI, 16-231 color cube, 232-255 grayscale ramp.
    Indexed(u8),
    /// Direct 24-bit color.
    Rgb(u8, u8, u8),
}

fn map_color(color: StyleColor, default: CellColor) -> CellColor {
    match color {
        StyleColor::None => default,
        StyleColor::Palette(ix) => CellColor::Indexed(ix.0),
        StyleColor::Rgb(c) => CellColor::Rgb(c.r, c.g, c.b),
    }
}

/// One rendered cell: char + colors + the flags paint cares about.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellSnapshot {
    pub ch: char,
    pub fg: CellColor,
    pub bg: CellColor,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub hidden: bool,
    /// A double-width char (occupies this cell plus the next spacer cell).
    pub wide: bool,
    /// The spacer half of a wide char — never shaped, only background-painted.
    pub wide_spacer: bool,
    /// Inside the active selection: the view paints a wash over this cell.
    pub selected: bool,
}

impl CellSnapshot {
    /// Effective paint colors after INVERSE/HIDDEN resolution.
    pub fn display_colors(&self) -> (CellColor, CellColor) {
        let (fg, bg) = if self.inverse {
            (self.bg, self.fg)
        } else {
            (self.fg, self.bg)
        };
        if self.hidden { (bg, bg) } else { (fg, bg) }
    }
}

/// Cursor position in viewport coordinates (row 0 = top of the visible grid).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorSnapshot {
    pub row: usize,
    pub col: usize,
}

/// The render-path handles, reused across snapshots. Behind a `RefCell`
/// because updating them mutates, while snapshots are `&self` reads.
struct Render {
    state: RenderState,
    rows: RowIterator,
    cells: CellIterator,
    /// The rows as last read: a row Ghostty did not mark dirty since is taken from here, not read cell by cell again.
    lines: Vec<Vec<CellSnapshot>>,
}

/// The selection being made: its granularity and where it started.
#[derive(Debug, Clone, Copy)]
struct Anchor {
    ty: SelectionType,
    point: GridPoint,
    side: Side,
}

/// The emulator: a pure fold of PTY bytes into a renderable grid.
pub struct Emulator {
    term: Terminal,
    render: RefCell<Render>,
    anchor: Option<Anchor>,
    bell: bool,
}

impl Emulator {
    pub fn new(cols: u16, rows: u16) -> Self {
        // herdr owns scrollback; scrolling is a terminal.scroll request to it, or a local copy ([`Self::with_history`]).
        Self::sized(cols, rows, 0)
    }

    /// herdr's history as `pane.read` gives it (`source: recent`, `format: ansi`): the lines above its screen, then
    /// the screen, wrapped at this width, no newline after the last. The grid holds them all, viewed at the live
    /// bottom, so [`Self::scroll_to_offset`] walks back through them without asking herdr.
    pub fn with_history(text: &str, cols: u16, rows: u16) -> Self {
        // Ghostty's default scrollback cap (bytes, an estimate): far more than herdr's 1000 lines read
        let mut e = Self::sized(cols, rows, 10_000_000);
        for (i, line) in text.split('\n').enumerate() {
            if i > 0 {
                e.feed(b"\r\n");
            }
            e.feed(b"\x1b[0m");
            e.feed(line.strip_suffix('\r').unwrap_or(line).as_bytes());
        }
        e
    }

    /// `max_scrollback`: bytes of history the terminal keeps (0: none).
    fn sized(cols: u16, rows: u16, max_scrollback: usize) -> Self {
        let term = Terminal::new(TerminalOptions { cols: cols.max(2), rows: rows.max(1), max_scrollback })
            .expect("libghostty-vt terminal");
        let render = Render {
            state: RenderState::new().expect("libghostty-vt render state"),
            rows: RowIterator::new().expect("libghostty-vt row iterator"),
            cells: CellIterator::new().expect("libghostty-vt cell iterator"),
            lines: Vec::new(),
        };
        Self {
            term,
            render: RefCell::new(render),
            anchor: None,
            bell: false,
        }
    }

    /// Advance the state machine over decoded PTY output. Returns bytes the
    /// terminal wants written back to the PTY (DSR/DA query responses etc.).
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.term.vt_write(bytes);
        let mut responses = Vec::new();
        for event in self.term.events() {
            match event {
                Event::PtyWrite(bytes) => responses.extend_from_slice(&bytes),
                Event::Bell => self.bell = true,
                _ => {}
            }
        }
        responses
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        // ponytail: no pixel size; only CSI 14/16 t size reports would want it.
        // No reflow: herdr repaints the whole grid at the new size, and until then a reflow shows its frame at the
        // old width rewrapped (Ghostty reflows only under wraparound).
        let wrap = self.term.mode(Mode::WRAPAROUND).unwrap_or(true);
        let _ = self.term.set_mode(Mode::WRAPAROUND, false);
        let _ = self.term.resize(cols.max(2), rows.max(1), 0, 0);
        let _ = self.term.set_mode(Mode::WRAPAROUND, wrap);
    }

    pub fn cols(&self) -> usize {
        self.term.cols().map_or(0, usize::from)
    }

    pub fn rows(&self) -> usize {
        self.term.rows().map_or(0, usize::from)
    }

    /// OSC title, if the running program set one.
    pub fn title(&self) -> Option<&str> {
        self.term.title().ok().filter(|t| !t.is_empty())
    }

    /// True once a BEL arrived; reading clears it.
    pub fn take_bell(&mut self) -> bool {
        std::mem::take(&mut self.bell)
    }

    /// Arrow keys should send SS3 (`ESC O A`) instead of CSI.
    pub fn app_cursor_mode(&self) -> bool {
        self.term.mode(Mode::DECCKM).unwrap_or(false)
    }

    /// The app is on the alternate screen (vim, less): it has no history of its own.
    pub fn alt_screen(&self) -> bool {
        self.term.active_screen().is_ok_and(|s| s == Screen::Alternate)
    }

    /// Pastes should be wrapped in `ESC [200~` / `ESC [201~`.
    pub fn bracketed_paste_mode(&self) -> bool {
        self.term.mode(Mode::BRACKETED_PASTE).unwrap_or(false)
    }

    /// Lines kept above the screen (0 but in a [`Self::with_history`] copy: herdr owns history).
    pub fn history_lines(&self) -> usize {
        self.term.scrollback_rows().unwrap_or(0)
    }

    /// Lines the view is scrolled back from the live bottom.
    pub fn display_offset(&self) -> usize {
        self.term.scrollbar().map_or(0, |s| s.total.saturating_sub(s.offset + s.len) as usize)
    }

    /// Scroll the view to `offset` lines back from the live bottom (clamped to the history).
    pub fn scroll_to_offset(&mut self, offset: usize) {
        let top = self.history_lines().saturating_sub(offset);
        self.term.scroll_viewport(ScrollViewport::Row(top));
    }

    // ---- selection ----
    //
    // The terminal owns the installed selection and moves it with its text
    // as output scrolls the grid. What stays here is the anchor of the one
    // being made, so each pointer move rebuilds it from anchor to pointer.

    fn grid_ref(&self, point: GridPoint) -> Option<GridRef<'_>> {
        let (x, y) = (u16::try_from(point.col).ok()?, u32::try_from(point.row).ok()?);
        self.term.grid_ref(Point::Viewport(PointCoordinate { x, y })).ok()
    }

    fn point_of(&self, grid_ref: &GridRef<'_>) -> Option<GridPoint> {
        let c = self.term.point_from_grid_ref(grid_ref, PointSpace::Viewport).ok()??;
        Some(GridPoint { row: c.y as usize, col: c.x as usize })
    }

    /// A selection's ends, top-left first.
    fn bounds(&self, selection: Option<Selection<'_>>) -> Option<(GridPoint, GridPoint)> {
        let selection = selection?;
        let (a, b) = (self.point_of(&selection.start())?, self.point_of(&selection.end())?);
        Some((a.min(b), a.max(b)))
    }

    /// The grid point under a viewport cell (row 0 = top of the visible area).
    pub fn grid_point(&self, viewport_row: usize, col: usize) -> GridPoint {
        GridPoint { row: viewport_row, col: col.min(self.cols().saturating_sub(1)) }
    }

    /// Begin a selection. `ty` picks the granularity: [`SelectionType::Simple`]
    /// for a drag, `Semantic` for a double-click word, `Lines` for a triple-
    /// click row.
    pub fn start_selection(&mut self, ty: SelectionType, point: GridPoint, side: Side) {
        self.anchor = Some(Anchor { ty, point, side });
        self.update_selection(point, side);
    }

    /// Extend the in-progress selection to `point`. No-op without one.
    pub fn update_selection(&mut self, point: GridPoint, side: Side) {
        let Some(anchor) = self.anchor else { return };
        let range = match anchor.ty {
            SelectionType::Simple => simple_range((anchor.point, anchor.side), (point, side), self.cols()),
            // Ghostty's double-click drag: the nearest word from each end toward the other.
            SelectionType::Semantic => {
                let word = |from, to| {
                    let options = SelectWordBetweenOptions::new(self.grid_ref(from)?, self.grid_ref(to)?);
                    self.bounds(self.term.select_word_between(options).ok()?)
                };
                union(word(anchor.point, point), word(point, anchor.point))
            }
            SelectionType::Lines => {
                let line = |at| self.bounds(self.term.select_line(SelectLineOptions::new(self.grid_ref(at)?)).ok()?);
                union(line(anchor.point), line(point))
            }
        };
        let selection = range.and_then(|(start, end)| Some(Selection::new(self.grid_ref(start)?, self.grid_ref(end)?, false)));
        let _ = self.term.set_selection(selection.as_ref());
    }

    pub fn clear_selection(&mut self) {
        self.anchor = None;
        let _ = self.term.set_selection(None);
    }

    /// The selected text, or `None` when there is no selection or it covers
    /// nothing (a click without a drag leaves an empty one behind).
    pub fn selection_text(&self) -> Option<String> {
        // Ghostty's copy: plain text, soft wraps joined, trailing blanks trimmed.
        let options = FormatOptions::new().with_unwrap(true).with_trim(true);
        let bytes = self.term.format_selection_alloc(options).ok()??;
        let mut text = String::from_utf8_lossy(&bytes).into_owned();
        // A row selection pastes as a row, line break included (Ghostty's trim drops it).
        if self.anchor.is_some_and(|a| a.ty == SelectionType::Lines) && !text.is_empty() {
            text.push('\n');
        }
        Some(text).filter(|s| !s.is_empty())
    }

    /// Whether a non-empty selection is active — drives the copy action and
    /// the "clear it" branch on the next click.
    pub fn has_selection(&self) -> bool {
        self.term.selection().is_ok_and(|s| s.is_some())
    }

    /// Snapshot one viewport row (0 = top).
    // ponytail: snapshots the whole viewport; only hover and tests ask for one row.
    pub fn line(&self, viewport_row: usize) -> Vec<CellSnapshot> {
        self.lines().into_iter().nth(viewport_row).unwrap_or_default()
    }

    /// All viewport rows, top to bottom.
    pub fn lines(&self) -> Vec<Vec<CellSnapshot>> {
        self.read_lines().unwrap_or_default()
    }

    fn read_lines(&self) -> Result<Vec<Vec<CellSnapshot>>, Error> {
        let mut render = self.render.borrow_mut();
        let Render { state, rows, cells, lines: cache } = &mut *render;
        let plain = Style::default();
        let snapshot = state.update(&self.term)?;
        let full = snapshot.dirty()? == Dirty::Full;
        let mut lines = Vec::with_capacity(self.rows());
        {
            let mut row_iter = rows.update(&snapshot)?;
            while let Some(row) = row_iter.next() {
                let selected = row.selection()?.map(|s| s.start_x..=s.end_x);
                // the selection is taken afresh for a clean row too: a changed one may leave the row clean
                if let Some(mut line) = cache.get(lines.len()).filter(|l| !full && l.len() == self.cols() && !row.dirty().unwrap_or(true)).cloned() {
                    for (col, cell) in line.iter_mut().enumerate() {
                        cell.selected = selected.as_ref().is_some_and(|r| r.contains(&(col as u16)));
                    }
                    lines.push(line);
                    continue;
                }
                let mut line = Vec::with_capacity(self.cols());
                let mut cell_iter = cells.update(row)?;
                while let Some(cell) = cell_iter.next() {
                    let raw = cell.raw_cell()?;
                    let style = if cell.has_styling()? { cell.style()? } else { plain };
                    // An erase with a background set stores the color in the cell, not a style.
                    let bg = match raw.content_tag()? {
                        CellContentTag::BgColorPalette => CellColor::Indexed(raw.bg_color_palette()?.0),
                        CellContentTag::BgColorRgb => {
                            let c = raw.bg_color_rgb()?;
                            CellColor::Rgb(c.r, c.g, c.b)
                        }
                        _ => map_color(style.bg_color, CellColor::Background),
                    };
                    let wide = raw.wide()?;
                    let col = line.len() as u16;
                    line.push(CellSnapshot {
                        // ponytail: the base codepoint only; combining marks are dropped, as alacritty's view did.
                        ch: char::from_u32(raw.codepoint()?).filter(|&c| c != '\0').unwrap_or(' '),
                        fg: map_color(style.fg_color, CellColor::Foreground),
                        bg,
                        bold: style.bold,
                        dim: style.faint,
                        italic: style.italic,
                        underline: style.underline != Underline::None,
                        inverse: style.inverse,
                        hidden: style.invisible,
                        wide: wide == CellWide::Wide,
                        wide_spacer: matches!(wide, CellWide::SpacerTail | CellWide::SpacerHead),
                        selected: selected.as_ref().is_some_and(|r| r.contains(&col)),
                    });
                }
                lines.push(line);
            }
        }
        snapshot.clean()?;
        cache.clone_from(&lines);
        Ok(lines)
    }

    /// Cursor in viewport coordinates; `None` when hidden.
    pub fn cursor(&self) -> Option<CursorSnapshot> {
        if !self.term.is_cursor_visible().ok()? {
            return None;
        }
        Some(CursorSnapshot {
            row: self.term.cursor_y().ok()? as usize,
            col: self.term.cursor_x().ok()? as usize,
        })
    }

    fn hyperlink(&self, row: usize, col: usize) -> Option<String> {
        let grid_ref = self.grid_ref(GridPoint { row, col })?;
        let mut buf = vec![0; 512];
        let len = match grid_ref.hyperlink_uri(&mut buf) {
            Err(Error::OutOfSpace { required }) => {
                buf.resize(required, 0);
                grid_ref.hyperlink_uri(&mut buf).ok()?
            }
            other => other.ok()?,
        };
        (len > 0).then(|| String::from_utf8_lossy(&buf[..len]).into_owned())
    }

    /// The OSC 8 hyperlink at a viewport cell: the cells around it on that row with the same URI, and the URI.
    // ponytail: one row, like text links; a hyperlink wrapped onto the next row underlines one row at a time.
    pub fn hyperlink_at(&self, viewport_row: usize, col: usize) -> Option<(std::ops::Range<usize>, String)> {
        let uri = self.hyperlink(viewport_row, col)?;
        let same = |c: &usize| self.hyperlink(viewport_row, *c).as_ref() == Some(&uri);
        let start = (0..col).rev().take_while(same).last().unwrap_or(col);
        let end = (col + 1..self.cols()).take_while(same).last().unwrap_or(col) + 1;
        Some((start..end, uri))
    }

    /// Test/diagnostic helper: a viewport row as trimmed text (wide-char
    /// spacers skipped).
    pub fn row_text(&self, viewport_row: usize) -> String {
        let mut text: String = self
            .line(viewport_row)
            .iter()
            .filter(|c| !c.wide_spacer)
            .map(|c| c.ch)
            .collect();
        while text.ends_with(' ') {
            text.pop();
        }
        text
    }
}

/// A drag's cells, top-left first: the side of each end says whether its cell is in.
fn simple_range(a: (GridPoint, Side), b: (GridPoint, Side), cols: usize) -> Option<(GridPoint, GridPoint)> {
    let ((mut start, start_side), (mut end, end_side)) = if a.0 <= b.0 { (a, b) } else { (b, a) };
    if start_side == Side::Right {
        start = if start.col + 1 < cols { GridPoint { col: start.col + 1, ..start } } else { GridPoint { row: start.row + 1, col: 0 } };
    }
    if end_side == Side::Left {
        end = match end.col {
            0 => GridPoint { row: end.row.checked_sub(1)?, col: cols.saturating_sub(1) },
            col => GridPoint { col: col - 1, ..end },
        };
    }
    (start <= end).then_some((start, end))
}

/// The span covering both ranges.
fn union(a: Option<(GridPoint, GridPoint)>, b: Option<(GridPoint, GridPoint)>) -> Option<(GridPoint, GridPoint)> {
    match (a, b) {
        (Some(a), Some(b)) => Some((a.0.min(b.0), a.1.max(b.1))),
        (a, b) => a.or(b),
    }
}

impl std::fmt::Debug for Emulator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Emulator")
            .field("cols", &self.cols())
            .field("rows", &self.rows())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_no_local_scrollback() {
        // herdr owns history: lines scrolled off the top of a frame are gone locally.
        let mut emu = Emulator::new(10, 3);
        emu.feed(b"1\r\n2\r\n3\r\n4\r\n5\r\n");
        assert_eq!(emu.history_lines(), 0);
    }

    fn emu(cols: u16, rows: u16) -> Emulator {
        Emulator::new(cols, rows)
    }

    #[test]
    fn a_reread_takes_changed_rows_and_the_selection_afresh() {
        let mut e = emu(10, 3);
        e.feed(b"one\r\ntwo\r\nthree");
        assert_eq!(e.row_text(1), "two");
        e.feed(b"\x1b[2;1HTWO");
        assert_eq!((e.row_text(0), e.row_text(1)), ("one".into(), "TWO".into()));
        e.start_selection(SelectionType::Simple, e.grid_point(0, 0), Side::Left);
        e.update_selection(e.grid_point(0, 2), Side::Right);
        assert!(e.line(0)[1].selected);
        e.clear_selection();
        assert!(!e.line(0)[1].selected);
        e.resize(6, 3);
        assert_eq!(e.line(0).len(), 6);
    }

    #[test]
    fn plain_text_lands_on_row_zero() {
        let mut e = emu(20, 5);
        e.feed(b"hello");
        assert_eq!(e.row_text(0), "hello");
        assert_eq!(e.cursor(), Some(CursorSnapshot { row: 0, col: 5 }));
    }

    #[test]
    fn crlf_moves_lines_and_cr_returns_to_column_zero() {
        let mut e = emu(20, 5);
        e.feed(b"one\r\ntwo\r\nthree");
        assert_eq!(e.row_text(0), "one");
        assert_eq!(e.row_text(1), "two");
        assert_eq!(e.row_text(2), "three");
        e.feed(b"\rXX");
        assert_eq!(e.row_text(2), "XXree");
    }

    #[test]
    fn long_line_wraps_at_the_grid_width() {
        let mut e = emu(10, 4);
        e.feed(b"abcdefghijKLM");
        assert_eq!(e.row_text(0), "abcdefghij");
        assert_eq!(e.row_text(1), "KLM");
    }

    #[test]
    fn sgr_colors_and_attributes() {
        let mut e = emu(40, 4);
        e.feed(b"\x1b[31mred\x1b[0m plain \x1b[1;44mboldbg\x1b[0m");
        let line = e.line(0);
        assert_eq!(line[0].fg, CellColor::Indexed(1));
        assert_eq!(line[0].bg, CellColor::Background);
        // After reset: defaults.
        assert_eq!(line[4].fg, CellColor::Foreground);
        // Bold + blue background segment starts at col 10 ("red plain " = 10).
        let bold_cell = line[10];
        assert!(bold_cell.bold);
        assert_eq!(bold_cell.bg, CellColor::Indexed(4));
    }

    #[test]
    fn bright_256_and_truecolor_sgr() {
        let mut e = emu(40, 2);
        e.feed(b"\x1b[95mA\x1b[38;5;196mB\x1b[38;2;10;20;30mC");
        let line = e.line(0);
        assert_eq!(line[0].fg, CellColor::Indexed(13)); // bright magenta
        assert_eq!(line[1].fg, CellColor::Indexed(196));
        assert_eq!(line[2].fg, CellColor::Rgb(10, 20, 30));
    }

    #[test]
    fn inverse_and_hidden_resolve_in_display_colors() {
        let mut e = emu(10, 2);
        e.feed(b"\x1b[7mI\x1b[0m\x1b[8mH");
        let inv = e.line(0)[0];
        assert!(inv.inverse);
        assert_eq!(
            inv.display_colors(),
            (CellColor::Background, CellColor::Foreground)
        );
        let hid = e.line(0)[1];
        assert!(hid.hidden);
        let (fg, bg) = hid.display_colors();
        assert_eq!(fg, bg, "hidden text paints foreground as background");
    }

    #[test]
    fn cursor_addressing_and_relative_moves() {
        let mut e = emu(20, 6);
        e.feed(b"\x1b[3;5Hx");
        // CSI H is 1-based; cell written at row 2, col 4; cursor advanced by 1.
        assert_eq!(e.line(2)[4].ch, 'x');
        assert_eq!(e.cursor(), Some(CursorSnapshot { row: 2, col: 5 }));
        e.feed(b"\x1b[2D"); // left twice
        assert_eq!(e.cursor(), Some(CursorSnapshot { row: 2, col: 3 }));
        e.feed(b"\x1b[A"); // up
        assert_eq!(e.cursor(), Some(CursorSnapshot { row: 1, col: 3 }));
    }

    #[test]
    fn clear_screen_and_home() {
        let mut e = emu(20, 4);
        e.feed(b"aaa\r\nbbb\r\nccc");
        e.feed(b"\x1b[2J\x1b[H");
        for row in 0..4 {
            assert_eq!(e.row_text(row), "");
        }
        assert_eq!(e.cursor(), Some(CursorSnapshot { row: 0, col: 0 }));
        e.feed(b"fresh");
        assert_eq!(e.row_text(0), "fresh");
    }

    #[test]
    fn an_erase_with_a_background_paints_the_erased_cells() {
        // Ghostty keeps such a background in the cell, not in a style.
        let mut e = emu(10, 2);
        e.feed(b"ab\x1b[44m\x1b[K\x1b[48;2;1;2;3m\r\n\x1b[K");
        assert_eq!(e.line(0)[1].bg, CellColor::Background);
        assert_eq!(e.line(0)[5].bg, CellColor::Indexed(4));
        assert_eq!(e.line(1)[0].bg, CellColor::Rgb(1, 2, 3));
    }

    #[test]
    fn erase_line_variants() {
        let mut e = emu(20, 2);
        e.feed(b"abcdef\x1b[3D\x1b[K"); // erase from cursor (col 3) to end
        assert_eq!(e.row_text(0), "abc");
    }

    #[test]
    fn a_read_history_scrolls_back_without_herdr() {
        // 30 lines, the screen 10 of them: the last 10 show, the 20 above are history; styles carry over
        let text = (1..=30).map(|i| if i == 25 { "\x1b[1mbold".to_string() } else { format!("l{i}") }).collect::<Vec<_>>().join("\r\n");
        let mut e = Emulator::with_history(&text, 20, 10);
        assert_eq!((e.history_lines(), e.row_text(0).as_str(), e.row_text(9).as_str()), (20, "l21", "l30"));
        assert!(e.line(4)[0].bold && !e.line(5)[0].bold, "each line starts unstyled");
        e.scroll_to_offset(5);
        assert_eq!((e.display_offset(), e.row_text(0).as_str()), (5, "l16"));
        e.scroll_to_offset(100);
        assert_eq!((e.display_offset(), e.row_text(0).as_str()), (20, "l1"));
        // a full-width line ends where its newline does, not a row later
        let e = Emulator::with_history("abcd\r\nef", 4, 2);
        assert_eq!((e.row_text(0).as_str(), e.row_text(1).as_str()), ("abcd", "ef"));
    }

    #[test]
    fn alt_screen_restores_primary_content() {
        let mut e = emu(20, 4);
        e.feed(b"primary");
        // Enter the alt screen; 1049 keeps the cursor position, so home first.
        e.feed(b"\x1b[?1049h\x1b[H");
        e.feed(b"alt-content");
        assert_eq!(e.row_text(0), "alt-content");
        e.feed(b"\x1b[?1049l"); // leave
        assert_eq!(e.row_text(0), "primary");
    }

    #[test]
    fn dsr_cursor_report_produces_pty_response() {
        let mut e = emu(20, 4);
        e.feed(b"\x1b[2;3H");
        let responses = e.feed(b"\x1b[6n");
        assert_eq!(String::from_utf8_lossy(&responses), "\x1b[2;3R");
    }

    #[test]
    fn osc_title_and_bell() {
        let mut e = emu(20, 2);
        assert_eq!(e.title(), None);
        e.feed(b"\x1b]0;my title\x07");
        assert_eq!(e.title(), Some("my title"));
        assert!(!e.take_bell());
        e.feed(b"\x07");
        assert!(e.take_bell());
        assert!(!e.take_bell(), "bell reads clear it");
    }

    #[test]
    fn app_cursor_and_bracketed_paste_modes_toggle() {
        let mut e = emu(10, 2);
        assert!(!e.app_cursor_mode());
        e.feed(b"\x1b[?1h");
        assert!(e.app_cursor_mode());
        e.feed(b"\x1b[?1l");
        assert!(!e.app_cursor_mode());
        e.feed(b"\x1b[?2004h");
        assert!(e.bracketed_paste_mode());
    }

    #[test]
    fn hidden_cursor_mode() {
        let mut e = emu(10, 2);
        e.feed(b"\x1b[?25l");
        assert_eq!(e.cursor(), None);
        e.feed(b"\x1b[?25h");
        assert!(e.cursor().is_some());
    }

    #[test]
    fn resize_preserves_content_and_reflows_cursor() {
        let mut e = emu(20, 5);
        e.feed(b"keepme\r\nsecond");
        e.resize(30, 3);
        assert_eq!(e.cols(), 30);
        assert_eq!(e.rows(), 3);
        assert_eq!(e.row_text(0), "keepme");
        assert_eq!(e.row_text(1), "second");
    }

    #[test]
    fn resize_clips_rows_instead_of_rewrapping_them() {
        // herdr's frames place every row with CUP; its repaint at the new size follows the resize
        let mut e = emu(6, 3);
        e.feed(b"\x1b[1;1Habcdef\x1b[2;1Hgh");
        e.resize(4, 3);
        assert_eq!((e.row_text(0).as_str(), e.row_text(1).as_str()), ("abcd", "gh"));
        assert!(e.term.mode(Mode::WRAPAROUND).unwrap());
    }

    #[test]
    fn wide_chars_occupy_two_cells_with_spacer() {
        let mut e = emu(10, 2);
        e.feed("宽w".as_bytes());
        let line = e.line(0);
        assert!(line[0].wide);
        assert_eq!(line[0].ch, '宽');
        assert!(line[1].wide_spacer);
        assert_eq!(line[2].ch, 'w');
        assert_eq!(e.row_text(0), "宽w");
        assert_eq!(e.cursor(), Some(CursorSnapshot { row: 0, col: 3 }));
    }

    #[test]
    fn simple_selection_yields_its_text_and_marks_its_cells() {
        let mut e = emu(20, 3);
        e.feed(b"hello world");
        assert!(!e.has_selection());
        assert_eq!(e.selection_text(), None);

        // Drag across "hello".
        e.start_selection(SelectionType::Simple, e.grid_point(0, 0), Side::Left);
        e.update_selection(e.grid_point(0, 4), Side::Right);
        assert!(e.has_selection());
        assert_eq!(e.selection_text().as_deref(), Some("hello"));

        let line = e.line(0);
        assert!(line[..5].iter().all(|c| c.selected));
        assert!(!line[5].selected, "the space past the drag is not selected");

        e.clear_selection();
        assert!(!e.has_selection());
        assert!(e.line(0).iter().all(|c| !c.selected));
    }

    /// Double-click granularity: the anchor expands to the whole word without
    /// the caller computing any boundaries.
    #[test]
    fn semantic_selection_expands_to_the_word() {
        let mut e = emu(30, 2);
        e.feed(b"alpha beta gamma");
        e.start_selection(SelectionType::Semantic, e.grid_point(0, 7), Side::Left);
        assert_eq!(e.selection_text().as_deref(), Some("beta"));
    }

    /// Triple-click granularity. The trailing newline is part of the copy —
    /// pasting a line-selection should reproduce the line break, the way it
    /// does in every other terminal.
    #[test]
    fn line_selection_takes_the_whole_row() {
        let mut e = emu(30, 3);
        e.feed(b"first row\r\nsecond row");
        e.start_selection(SelectionType::Lines, e.grid_point(1, 3), Side::Left);
        assert_eq!(e.selection_text().as_deref(), Some("second row\n"));
    }

    /// A selection made across a line break keeps the newline, so pasting the
    /// copy reproduces the rows.
    #[test]
    fn selection_spans_rows_with_a_newline() {
        let mut e = emu(10, 3);
        e.feed(b"ab\r\ncd");
        e.start_selection(SelectionType::Simple, e.grid_point(0, 0), Side::Left);
        e.update_selection(e.grid_point(1, 1), Side::Right);
        assert_eq!(e.selection_text().as_deref(), Some("ab\ncd"));
    }

    /// A click with no drag selects nothing, and must not report a selection —
    /// otherwise the copy action fires on every bare click.
    #[test]
    fn a_click_without_a_drag_selects_nothing() {
        let mut e = emu(20, 2);
        e.feed(b"hello");
        e.start_selection(SelectionType::Simple, e.grid_point(0, 2), Side::Left);
        assert_eq!(e.selection_text(), None);
        assert!(!e.has_selection());
    }

    #[test]
    fn utf8_split_across_feeds_reassembles() {
        let mut e = emu(10, 2);
        let bytes = "é".as_bytes();
        e.feed(&bytes[..1]);
        e.feed(&bytes[1..]);
        assert_eq!(e.row_text(0), "é");
    }

    #[test]
    fn an_osc8_hyperlink_spans_its_cells() {
        // A style change inside the link keeps it one span.
        let mut e = emu(30, 2);
        e.feed(b"see \x1b]8;;https://a.b\x1b\\li\x1b[1mnk\x1b]8;;\x1b\\ ok");
        assert_eq!(e.hyperlink_at(0, 6), Some((4..8, "https://a.b".into())));
        assert_eq!(e.hyperlink_at(0, 4).map(|h| h.0), Some(4..8));
        assert_eq!(e.hyperlink_at(0, 2), None);
        assert_eq!(e.hyperlink_at(0, 9), None);
    }
}
