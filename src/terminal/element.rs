//! The terminal grid painter, from Zeron's crates/ui/src/terminal/view.rs (MIT, see assets/NOTICE).
//! Original notes:
//!
//! Terminal paint + input encoding.
//!
//! - theme-owned background/ANSI16 plus the xterm 256-color cube/grayscale;
//! - [`TerminalElement`] — a custom gpui element that measures cell metrics
//!   from the real mono font (the "font probe"), reports the resulting
//!   cols×rows back to the panel, and paints the grid: background quads for
//!   non-default cells, one `ShapedLine` per row (same font whatever the
//!   colors — paint never changes layout), and the cursor block.

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    App, Bounds, Entity, GlobalElementId, Hsla, LayoutId, PaintQuad, Pixels, RenderImage,
    ShapedLine, SharedString, Style, TextRun, Window, fill, font, outline, point, px, relative,
    size,
};

use crate::herdr::graphics::PaneImage;
use crate::theme::{Appearance, Theme, rgb_to_hsl};

use super::sprite;
use super::emulator::{CellColor, CellSnapshot, CursorShape, Side};
use super::pane::{GridGeometry, PaneTerminal};

/// Default terminal font metrics; the rendered values come from the theme.
pub const TERM_LINE_HEIGHT: f32 = 18.0;
// ponytail: a fixed list of common Nerd Font families; a setting when someone uses another.
const NERD_FALLBACKS: [&str; 3] = ["Symbols Nerd Font Mono", "IoskeleyMonoNL Nerd Font Mono", "FiraCode Nerd Font Mono"];
/// Inner padding of the grid area: Ghostty's default `window-padding-x/y`.
pub const TERM_PADDING: f32 = 2.0;

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

/// The panel fill behind the grid. On glass the opaque tone thins to a
/// translucent wash so the blurred desktop reads through like the rest of the
/// chrome (same move as [`Theme::card_glass_bg`]); opaque platforms keep the
/// true tone. Explicit cell backgrounds (vim colorschemes etc.) still paint
/// their own opaque quads on top.
pub fn terminal_panel_bg(theme: &Theme) -> Hsla {
    theme.terminal.background
}

fn rgb8(r: u8, g: u8, b: u8) -> Hsla {
    let (h, s, l) = rgb_to_hsl(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    gpui::hsla(h, s, l, 1.0)
}

/// xterm 256-color cube component levels.
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// Resolve the xterm extended-color range to RGB components.
///
/// Three ranges, treated differently on purpose:
///
/// - **0-15** are named slots and are resolved directly from `Theme::terminal`.
/// - **16-231** is the 6×6×6 cube: a program asking for index 196 is asking for
///   `#ff0000` by arithmetic, and remapping it would be inventing colors the
///   caller did not pick. Left alone in both appearances, same as iTerm/Apple
///   Terminal light themes.
/// - **232-255** is the grayscale ramp, which tools use for *de-emphasis*
///   rather than for a specific grey. Its dark→light direction only reads as
///   "dim" on a dark background, so light mode mirrors the ramp: index 232
///   stays the faintest and 255 the strongest in both. Without this the ramp is
///   the single biggest legibility hole on white, because its bright end —
///   where most "dim hint" text lands — is the end that vanishes.
fn extended_indexed_rgb(appearance: Appearance, index: u8) -> (u8, u8, u8) {
    match index {
        0..=15 => unreachable!("ANSI16 is resolved from the active theme"),
        16..=231 => {
            let n = index as usize - 16;
            (
                CUBE_LEVELS[n / 36],
                CUBE_LEVELS[(n / 6) % 6],
                CUBE_LEVELS[n % 6],
            )
        }
        232..=255 => {
            let step = index - 232;
            let step = match appearance {
                Appearance::Dark => step,
                Appearance::Light => 23 - step,
            };
            let v = 8 + 10 * step;
            (v, v, v)
        }
    }
}

/// Resolve a cell color to paint against the theme.
pub fn resolve_color(color: CellColor, theme: &Theme) -> Hsla {
    match color {
        CellColor::Foreground => theme.terminal.foreground,
        CellColor::Background => theme.terminal.background,
        CellColor::Indexed(ix @ 0..=15) => theme.terminal.ansi[ix as usize],
        CellColor::Indexed(ix) => {
            let (r, g, b) = extended_indexed_rgb(theme.appearance, ix);
            rgb8(r, g, b)
        }
        CellColor::Rgb(r, g, b) => rgb8(r, g, b),
    }
}

// ---------------------------------------------------------------------------
// Pointer → cell
// ---------------------------------------------------------------------------

/// Minimum pointer travel before a press turns into a selection.
///
/// Without it, the click that focuses the panel starts a one-cell selection if
/// the hand moves a pixel — and once anything copies on selection change, that
/// silently clobbers the clipboard. Matches the threshold zed uses for the same
/// reason (`SELECTION_DRAG_THRESHOLD`), which is gpui's own `div` drag
/// threshold.
pub const SELECTION_DRAG_THRESHOLD: f32 = 2.0;

/// Which cell a pointer landed on, and which edge of it a selection anchors to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellHit {
    pub row: usize,
    pub col: usize,
    /// Selections anchor to a cell *edge*, not a cell: pressing on the left
    /// half of a glyph includes it, the right half excludes it.
    pub side: Side,
}

/// Map a position *relative to the grid's top-left glyph* onto a cell.
///
/// Positions outside the grid clamp to the nearest cell rather than returning
/// `None`, because that is what a drag needs: the pointer routinely leaves the
/// panel mid-gesture, and the selection should extend to the edge it left
/// through instead of freezing at the last sample taken inside.
///
/// Overshoot also *forces the side*, which clamping alone does not give you.
/// Dragging past the bottom should take the last line whole, even when the
/// pointer drifted left of where it started — deriving the side from x there
/// would stop the selection mid-row. Same rule going up, mirrored. This is the
/// behaviour alacritty and zed's `grid_point_and_side` both implement.
pub fn cell_at(x: f32, y: f32, cell_w: f32, line_h: f32, cols: usize, rows: usize) -> CellHit {
    // Degenerate metrics (a zero-size grid, or a font probe that returned NaN)
    // would otherwise divide into garbage cell indices.
    let usable = |v: f32| v.is_finite() && v > 0.0;
    if cols == 0 || rows == 0 || !usable(cell_w) || !usable(line_h) {
        return CellHit {
            row: 0,
            col: 0,
            side: Side::Left,
        };
    }
    let x = if x.is_finite() { x } else { 0.0 };
    let y = if y.is_finite() { y } else { 0.0 };
    let last_col = cols - 1;
    let last_row = rows - 1;

    let raw_col = (x / cell_w).floor();
    let mut side = if x.max(0.0) % cell_w > cell_w / 2.0 {
        Side::Right
    } else {
        Side::Left
    };
    let col = if raw_col > last_col as f32 {
        side = Side::Right;
        last_col
    } else {
        raw_col.max(0.0) as usize
    };

    let raw_row = (y / line_h).floor();
    let row = if raw_row > last_row as f32 {
        side = Side::Right;
        last_row
    } else if raw_row < 0.0 {
        side = Side::Left;
        0
    } else {
        raw_row as usize
    };

    CellHit { row, col, side }
}

// ---------------------------------------------------------------------------
// Grid element
// ---------------------------------------------------------------------------

/// Paints the active tab's grid. Cell metrics come from the resolved mono font
/// each frame (font probe): `em_advance` for the cell width, the fixed line
/// height for rows. The measured cols×rows feed back into the panel, which
/// resizes the emulator and herdr's view of the pane.
pub struct TerminalElement {
    panel: Entity<PaneTerminal>,
    focused: bool,
}

impl TerminalElement {
    pub fn new(panel: Entity<PaneTerminal>, focused: bool) -> Self {
        Self { panel, focused }
    }
}

pub struct TerminalPrepaint {
    bg_quads: Vec<PaintQuad>,
    /// Selection wash. Painted after [`Self::bg_quads`] and before the glyphs:
    /// it has to tint a cell's own background rather than replace it, and it
    /// must not wash out the text it is highlighting.
    sel_quads: Vec<PaintQuad>,
    /// Per row, the shaped segments and the grid COLUMN each one starts at.
    /// Not one line per row: see [`shape_row`].
    lines: Vec<Vec<(usize, ShapedLine)>>,
    /// Per row, the cells drawn by [`sprite`] instead of the font.
    glyphs: Vec<Vec<CustomGlyph>>,
    /// Light line width for sprite glyphs, in logical pixels.
    stroke: f32,
    /// Grid cell advance, so paint can place segments by column.
    cell_w: Pixels,
    /// Grid row height, so paint places rows on the same baselines prepaint
    /// measured the grid with.
    line_h: Pixels,
    cursor: Option<PaintQuad>,
    /// herdr's Kitty images, lowest z first, and whether each goes under the text.
    images: Vec<(Bounds<Pixels>, Arc<RenderImage>, bool)>,
    /// The focused block cursor is opaque, so it goes under the text (whose
    /// cell flips to the background color); the unfocused outline goes over.
    cursor_under_text: bool,
}

impl gpui::IntoElement for TerminalElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl gpui::Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = TerminalPrepaint;

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let theme = Theme::of(cx).clone();
        // Ligatures stay on: `shape_row` puts every glyph back on its cell's
        // column, so a ligature narrower than its cells no longer pulls the
        // rest of the row off the grid.
        let mut mono = font(theme.font_terminal.clone());
        // Nerd Font icons (eza, prompts) are missing from Geist Mono; borrow them from installed Nerd Fonts.
        mono.fallbacks = Some(gpui::FontFallbacks::from_fonts(NERD_FALLBACKS.iter().map(|f| f.to_string()).collect()));
        // Font probe: measure the actual advance of the resolved mono font so
        // cols/rows track real glyph metrics, not a guessed aspect ratio.
        let font_size = px(theme.terminal_font_size);
        let font_id = window.text_system().resolve_font(&mono);
        let cell_w = window
            .text_system()
            .em_advance(font_id, font_size)
            .unwrap_or(px(theme.terminal_font_size * 0.6));
        let line_h = px((theme.terminal_font_size * theme.terminal_line_height).round());
        let scale = window.scale_factor();

        let inner_w = f32::from(bounds.size.width) - 2.0 * TERM_PADDING;
        let inner_h = f32::from(bounds.size.height) - 2.0 * TERM_PADDING;
        let cols = ((inner_w / f32::from(cell_w)).floor() as i64).clamp(2, 500) as u16;
        let rows = ((inner_h / f32::from(line_h)).floor() as i64).clamp(1, 500) as u16;

        // Report the measured grid, then snapshot for painting. Safe: the
        // panel entity is not borrowed during element prepaint.
        let origin = point(
            bounds.left() + px(TERM_PADDING),
            bounds.top() + px(TERM_PADDING),
        );
        let snapshot = self.panel.update(cx, |panel, cx| {
            panel.on_grid_metrics(
                GridGeometry {
                    bounds,
                    origin,
                    cell_w: f32::from(cell_w),
                    line_h: f32::from(line_h),
                    cols,
                    rows,
                    cell_px: ((f32::from(cell_w) * scale).round() as u32, (f32::from(line_h) * scale).round() as u32),
                },
                cx,
            );
            panel.active_grid_snapshot(cx)
        });
        let Some(snapshot) = snapshot else {
            return TerminalPrepaint {
                bg_quads: Vec::new(),
                sel_quads: Vec::new(),
                lines: Vec::new(),
                glyphs: Vec::new(),
                stroke: 1.0,
                cell_w,
                line_h,
                cursor: None,
                images: Vec::new(),
                cursor_under_text: false,
            };
        };

        let mut bg_quads = Vec::new();
        let mut sel_quads = Vec::new();
        let mut lines = Vec::with_capacity(snapshot.lines.len());
        let mut glyphs = Vec::with_capacity(snapshot.lines.len());
        // Ghostty's block cursor: the cell's text takes the background color.
        let block_cursor =
            snapshot.cursor.as_ref().filter(|c| self.focused && c.shape == CursorShape::Block).map(|c| (c.row, c.col));
        let cursor_text = Hsla { a: 1.0, ..theme.terminal.background };

        // Search matches wash their cells under the selection; the current one stands out.
        for (row, cells, current) in &snapshot.marks {
            let color = if *current { theme.accent.opacity(0.6) } else { theme.warning.opacity(0.3) };
            sel_quads.push(fill(
                Bounds::new(
                    point(origin.x + cell_w * cells.start as f32, origin.y + line_h * *row as f32),
                    size(cell_w * cells.len() as f32, line_h),
                ),
                color,
            ));
        }
        for (row_ix, row) in snapshot.lines.iter().enumerate() {
            let y = origin.y + line_h * row_ix as f32;
            // Selected runs, merged the same way background runs are: one quad
            // per contiguous span instead of one per cell.
            let mut sel_start: Option<usize> = None;
            for col in 0..=row.len() {
                let selected = row.get(col).is_some_and(|cell| cell.selected);
                match (sel_start, selected) {
                    (None, true) => sel_start = Some(col),
                    (Some(start), false) => {
                        sel_quads.push(fill(
                            Bounds::new(
                                point(origin.x + cell_w * start as f32, y),
                                size(cell_w * (col - start) as f32, line_h),
                            ),
                            theme.terminal.selection,
                        ));
                        sel_start = None;
                    }
                    _ => {}
                }
            }
            // Merge consecutive non-default background cells into quads.
            let mut run_start: Option<(usize, Hsla)> = None;
            for (col, color) in row
                .iter()
                .map(|cell| cell.display_colors().1)
                .chain(std::iter::once(CellColor::Background))
                .enumerate()
            {
                let paint = match color {
                    CellColor::Background => None,
                    other => Some(resolve_color(other, &theme)),
                };
                match (&run_start, paint) {
                    (None, Some(color)) => run_start = Some((col, color)),
                    (Some((start, current)), next) if next != Some(*current) => {
                        bg_quads.push(fill(
                            Bounds::new(
                                point(origin.x + cell_w * *start as f32, y),
                                size(cell_w * (col - *start) as f32, line_h),
                            ),
                            *current,
                        ));
                        run_start = next.map(|color| (col, color));
                    }
                    _ => {}
                }
            }
            let mut row_glyphs = Vec::new();
            let cursor_col = block_cursor.filter(|&(r, _)| r == row_ix).map(|(_, c)| (c, cursor_text));
            lines.push(shape_row(row, &theme, &mono, font_size, cell_w, cursor_col, &mut row_glyphs, window));
            glyphs.push(row_glyphs);
        }

        let cursor = snapshot.cursor.map(|c| {
            let cursor_bounds = Bounds::new(
                point(
                    origin.x + cell_w * c.col as f32,
                    origin.y + line_h * c.row as f32,
                ),
                size(cell_w, line_h),
            );
            // the shape the program set (DECSCUSR, passed on by herdr): vim's insert beam, an underline
            let bar = px(2.0);
            match (self.focused, c.shape) {
                (true, CursorShape::Block) => fill(cursor_bounds, theme.cursor),
                (true, CursorShape::Beam) => fill(Bounds::new(cursor_bounds.origin, size(bar, line_h)), theme.cursor),
                (true, CursorShape::Underline) => fill(
                    Bounds::new(point(cursor_bounds.left(), cursor_bounds.bottom() - bar), size(cell_w, bar)),
                    theme.cursor,
                ),
                _ => outline(cursor_bounds, theme.cursor, gpui::BorderStyle::Solid),
            }
        });

        let images = snapshot
            .images
            .iter()
            .map(|i| (image_bounds(i, origin, cell_w, line_h, scale), i.image.clone(), i.z < 0))
            .collect();

        TerminalPrepaint {
            bg_quads,
            sel_quads,
            lines,
            glyphs,
            // The font's underline thickness is ~0.05em; Ghostty strokes box
            // lines with it. ponytail: gpui doesn't expose the real
            // metric, so approximate it.
            stroke: theme.terminal_font_size * 0.05,
            cell_w,
            line_h,
            cursor,
            images,
            cursor_under_text: block_cursor.is_some(),
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _state: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let line_h = prepaint.line_h;
        let origin = point(
            bounds.left() + px(TERM_PADDING),
            bounds.top() + px(TERM_PADDING),
        );
        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            for quad in prepaint.bg_quads.drain(..) {
                window.paint_quad(quad);
            }
            for (bounds, image, _) in prepaint.images.iter().filter(|i| i.2) {
                let _ = window.paint_image(*bounds, Default::default(), image.clone(), 0, false);
            }
            for quad in prepaint.sel_quads.drain(..) {
                window.paint_quad(quad);
            }
            if prepaint.cursor_under_text
                && let Some(cursor) = prepaint.cursor.take()
            {
                window.paint_quad(cursor);
            }
            let cell_w = prepaint.cell_w;
            for (ix, segments) in prepaint.lines.iter().enumerate() {
                let y = origin.y + line_h * ix as f32;
                for (col, line) in segments {
                    let _ = line.paint(
                        point(origin.x + cell_w * *col as f32, y),
                        line_h,
                        gpui::TextAlign::Left,
                        None,
                        window,
                        cx,
                    );
                }
            }
            for (ix, row) in prepaint.glyphs.iter().enumerate() {
                let y = origin.y + line_h * ix as f32;
                for glyph in row {
                    let left = origin.x + cell_w * glyph.col as f32;
                    let bounds = Bounds::from_corners(
                        point(left, y),
                        point(left + cell_w * glyph.cells as f32, y + line_h),
                    );
                    paint_custom_glyph(glyph, bounds, prepaint.stroke, window, cx);
                }
            }
            for (bounds, image, _) in prepaint.images.drain(..).filter(|i| !i.2) {
                let _ = window.paint_image(bounds, Default::default(), image, 0, false);
            }
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        });
    }
}

/// Shape one grid row into segments placed by column.
///
/// A terminal is a fixed grid, but a shaped line places glyphs by their font
/// advances, and those drift off it: a glyph from FONT FALLBACK (arrows `→`,
/// emoji, CJK) advances by whatever its own font says, and a ligature (Geist
/// Mono's `--`, `->`) covers several cells with one narrower glyph. Laid out
/// as shaped, the rest of the row slides (user reports: "one of the pipes is
/// broken", `codex --yolo` rendering as `codex--yolo`), while backgrounds,
/// selection and the cursor stay put as quads at `cell_w * col`.
///
/// So, as Ghostty, kitty and WezTerm do, every glyph moves to the column of
/// the cell it came from (its cluster): a glyph narrower than its cells leaves
/// a gap, and nothing after it slides. Own segments remain only for sprite
/// glyphs (drawn by paint), right-to-left runs (CoreText reorders and joins
/// their glyphs, so they cannot be placed letter by letter) and the cursor
/// cell, which splits a ligature under the cursor as Ghostty does.
#[allow(clippy::too_many_arguments)]
fn shape_row(
    row: &[CellSnapshot],
    theme: &Theme,
    mono: &gpui::Font,
    font_size: Pixels,
    cell_w: Pixels,
    cursor: Option<(usize, Hsla)>,
    glyphs: &mut Vec<CustomGlyph>,
    window: &Window,
) -> Vec<(usize, ShapedLine)> {
    let mut segments: Vec<(usize, ShapedLine)> = Vec::new();
    let mut seg = Segment::default();
    let mut rtl_run = false;

    for (col, cell) in row.iter().enumerate() {
        if cell.wide_spacer {
            continue;
        }
        let ch = if cell.hidden { ' ' } else { cell.ch };
        let (fg, _) = cell.display_colors();
        let mut color = resolve_color(fg, theme);
        if cell.dim {
            color.a *= 0.6;
        }
        let at_cursor = cursor.is_some_and(|(c, _)| c == col);
        if let Some((_, cursor_text)) = cursor.filter(|_| at_cursor) {
            color = cursor_text;
        }
        // An RTL run ending here fits into the cells it spans.
        let rtl_fit = rtl_run.then(|| cell_w * (col - seg.col) as f32);
        if !ch.is_ascii() && sprite::has_char(ch) {
            // Drawn to the cell by paint, not shaped.
            seg.flush(rtl_fit, &mut segments, font_size, cell_w, window);
            rtl_run = false;
            glyphs.push(CustomGlyph {
                col,
                ch,
                cells: if cell.wide { 2 } else { 1 },
                color,
            });
            continue;
        }
        // Arabic/Hebrew letters (and the spaces between them) shape as ONE segment: alone, each letter gets its
        // isolated form and the words read left to right; together CoreText joins and reorders them.
        let rtl = is_rtl(ch) && !cell.wide;
        let joins = rtl_run && (rtl || ch == ' ');
        if !joins && (at_cursor || rtl || rtl_run) {
            seg.flush(rtl_fit, &mut segments, font_size, cell_w, window);
        }
        if seg.text.is_empty() {
            seg.col = col;
        }
        let weight = if cell.bold {
            gpui::FontWeight::BOLD
        } else {
            gpui::FontWeight::NORMAL
        };
        let style = if cell.italic {
            gpui::FontStyle::Italic
        } else {
            gpui::FontStyle::Normal
        };
        let underline = cell.underline.then_some(gpui::UnderlineStyle {
            color: Some(color),
            thickness: px(1.0),
            wavy: cell.undercurl,
        });
        let strikethrough = cell.strikethrough.then_some(gpui::StrikethroughStyle { color: Some(color), thickness: px(1.0) });
        // a cell's whole grapheme cluster (a flag, a skin tone, a letter and its marks) shapes together, on its column
        let mut buf = [0; 4];
        let text = cell.cluster.as_deref().filter(|_| !cell.hidden).unwrap_or_else(|| ch.encode_utf8(&mut buf));
        let len = text.len();
        seg.text.push_str(text);
        seg.cols.extend(std::iter::repeat_n(col, len));
        // every run is `mono` but for weight and style: comparing those spares a font clone and compare per cell
        match seg.runs.last_mut() {
            Some(last)
                if last.color == color && last.font.weight == weight && last.font.style == style && last.underline == underline
                    && last.strikethrough == strikethrough =>
            {
                last.len += len;
            }
            _ => seg.runs.push(TextRun {
                len,
                font: gpui::Font { weight, style, ..mono.clone() },
                color,
                background_color: None,
                underline,
                strikethrough,
            }),
        }
        rtl_run = rtl || joins;
        if at_cursor && !rtl_run {
            seg.flush(None, &mut segments, font_size, cell_w, window);
        }
    }
    let rtl_fit = rtl_run.then(|| cell_w * (row.len() - seg.col) as f32);
    seg.flush(rtl_fit, &mut segments, font_size, cell_w, window);
    segments
}

/// The row text being gathered into one shaped segment, with the grid column of each of its bytes.
#[derive(Default)]
struct Segment {
    text: String,
    cols: Vec<usize>,
    runs: Vec<TextRun>,
    col: usize,
}

impl Segment {
    /// Shape the gathered text into `out`. `rtl_fit` marks a right-to-left run and the width of its cells.
    fn flush(
        &mut self,
        rtl_fit: Option<Pixels>,
        out: &mut Vec<(usize, ShapedLine)>,
        font_size: Pixels,
        cell_w: Pixels,
        window: &Window,
    ) {
        if self.text.is_empty() {
            return;
        }
        let text = SharedString::from(std::mem::take(&mut self.text));
        let system = window.text_system();
        let mut shaped = system.shape_line(text.clone(), font_size, &self.runs, None);
        match rtl_fit {
            // A joined RTL run wider than its cells shrinks to fit them. Spacing glyphs apart instead would
            // break the joins between letters.
            // ponytail: narrower runs stay as they are; Arabic stretches with kashida (U+0640), not by spacing.
            Some(fit) if shaped.width > fit => {
                shaped = system.shape_line(text, font_size * (fit / shaped.width), &self.runs, None);
            }
            Some(_) => {}
            None => snap_to_cells(&mut shaped, &self.cols, self.col, cell_w),
        }
        out.push((self.col, shaped));
        self.runs.clear();
        self.cols.clear();
    }
}

/// Move every glyph of `line` to the column its cluster starts at. Glyphs of one cluster (a letter and its
/// combining marks) keep their offsets from the cluster's first glyph.
fn snap_to_cells(line: &mut ShapedLine, cols: &[usize], seg_col: usize, cell_w: Pixels) {
    let mut runs = line.runs.clone();
    let mut cluster = (usize::MAX, px(0.));
    for glyph in runs.iter_mut().flat_map(|run| run.glyphs.iter_mut()) {
        // a combining mark is its own glyph at its own index, but on its letter's column
        let col = cols.get(glyph.index).copied().unwrap_or(seg_col);
        if col != cluster.0 {
            cluster = (col, glyph.position.x);
        }
        glyph.position.x = cell_w * (col - seg_col) as f32 + (glyph.position.x - cluster.1);
    }
    **line = Arc::new(gpui::LineLayout {
        font_size: line.font_size,
        width: line.width,
        ascent: line.ascent,
        descent: line.descent,
        runs,
        len: line.len,
    });
}

/// Hebrew and Arabic blocks (letters, marks, presentation forms).
fn is_rtl(ch: char) -> bool {
    matches!(ch, '\u{0590}'..='\u{08FF}' | '\u{FB1D}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFC}')
}

/// Where a Kitty image lands: its cells from the grid's `origin`, less the device-pixel offset into the first one.
fn image_bounds(image: &PaneImage, origin: gpui::Point<Pixels>, cell_w: Pixels, line_h: Pixels, scale: f32) -> Bounds<Pixels> {
    let (dx, dy) = (px(image.x_offset as f32 / scale), px(image.y_offset as f32 / scale));
    Bounds::new(
        point(origin.x + cell_w * image.x as f32 + dx, origin.y + line_h * image.y as f32 + dy),
        size(cell_w * image.cols as f32 - dx, line_h * image.rows as f32 - dy),
    )
}

pub struct CustomGlyph {
    col: usize,
    ch: char,
    cells: usize,
    color: Hsla,
}

/// Drawn sprite glyphs, keyed by char, device-pixel cell size, line
/// thickness and color (images are full color, so the color is baked in).
#[derive(Default)]
struct CustomGlyphCache(HashMap<(char, u32, u32, u32, [u8; 4]), (Arc<RenderImage>, u32, u32)>);

impl gpui::Global for CustomGlyphCache {}

/// Paint one sprite glyph drawn at the exact device-pixel size of its cell,
/// so neighbouring cells tile without seams or resampling. The sprite is
/// padded on every side (a glyph may spill past its cell), so it is placed
/// up-left of the cell by the padding.
fn paint_custom_glyph(
    glyph: &CustomGlyph,
    bounds: Bounds<Pixels>,
    stroke: f32,
    window: &mut Window,
    cx: &mut App,
) {
    let scale = window.scale_factor();
    let device = |v: Pixels| (f32::from(v) * scale).round();
    let (left, top) = (device(bounds.left()), device(bounds.top()));
    let w = (device(bounds.right()) - left) as u32;
    let h = (device(bounds.bottom()) - top) as u32;
    let thickness = (stroke * scale).ceil().max(1.0) as u32;
    let rgba = gpui::Rgba::from(glyph.color);
    let color = [rgba.r, rgba.g, rgba.b, rgba.a].map(|c| (c * 255.0).round() as u8);
    let key = (glyph.ch, w, h, thickness, color);

    let cache = cx.default_global::<CustomGlyphCache>();
    let (image, pad_x, pad_y) = match cache.0.get(&key) {
        Some(hit) => hit.clone(),
        None => {
            let metrics = sprite::Metrics {
                cell_width: w / glyph.cells as u32,
                cell_height: h,
                box_thickness: thickness,
            };
            let Some(drawn) = sprite::render(glyph.ch, w, h, &metrics) else { return };
            // Coverage → straight-alpha BGRA in the glyph color.
            let [r, g, b, a] = color;
            let bgra: Vec<u8> =
                drawn.alpha.iter().flat_map(|&cov| [b, g, r, (cov as u16 * a as u16 / 255) as u8]).collect();
            let Some(buffer) = image::RgbaImage::from_raw(drawn.width, drawn.height, bgra) else { return };
            let image = Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]));
            // ponytail: drop everything past a cap (old sizes after a zoom,
            // truecolor gradients); an LRU if that ever thrashes. Images live
            // in this window's atlas, so other windows may keep stale copies.
            if cache.0.len() >= 4096 {
                for (_, (old, _, _)) in cache.0.drain() {
                    let _ = window.drop_image(old);
                }
            }
            let hit = (image, drawn.padding_x, drawn.padding_y);
            cache.0.insert(key, hit.clone());
            hit
        }
    };
    let size = image.size(0);
    let logical = |v: f32| px(v / scale);
    let bounds = Bounds::new(
        point(logical(left - pad_x as f32), logical(top - pad_y as f32)),
        gpui::size(logical(size.width.0 as f32), logical(size.height.0 as f32)),
    );
    let _ = window.paint_image(bounds, Default::default(), image, 0, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_covers_its_cells_less_the_offset_into_the_first() {
        let image = PaneImage {
            image: Arc::new(RenderImage::new(Vec::<image::Frame>::new())),
            x: 2,
            y: 1,
            cols: 3,
            rows: 2,
            x_offset: 4,
            y_offset: 0,
            z: 0,
        };
        // 2x display: 4 device pixels are 2 logical ones
        let b = image_bounds(&image, point(px(10.), px(20.)), px(8.), px(17.), 2.0);
        assert_eq!(b, Bounds::new(point(px(10. + 16. + 2.), px(20. + 17.)), size(px(24. - 2.), px(34.))));
    }

    #[test]
    fn cube_is_appearance_independent() {
        for appearance in [Appearance::Dark, Appearance::Light] {
            // 16 = cube origin (0,0,0); 231 = cube max (255,255,255).
            assert_eq!(extended_indexed_rgb(appearance, 16), (0, 0, 0));
            assert_eq!(extended_indexed_rgb(appearance, 231), (255, 255, 255));
            // 196 = pure red corner: 16 + 36*5.
            assert_eq!(extended_indexed_rgb(appearance, 196), (255, 0, 0));
            // 21 = pure blue corner.
            assert_eq!(extended_indexed_rgb(appearance, 21), (0, 0, 255));
        }
    }

    #[test]
    fn grayscale_ramp_mirrors_in_light() {
        // Dark: 232 → 8 (faintest), 255 → 238 (strongest).
        assert_eq!(extended_indexed_rgb(Appearance::Dark, 232), (8, 8, 8));
        assert_eq!(extended_indexed_rgb(Appearance::Dark, 255), (238, 238, 238));
        // Light reverses it so the faint end stays faint against white.
        assert_eq!(
            extended_indexed_rgb(Appearance::Light, 232),
            (238, 238, 238)
        );
        assert_eq!(extended_indexed_rgb(Appearance::Light, 255), (8, 8, 8));
    }

    fn hit(x: f32, y: f32) -> CellHit {
        cell_at(x, y, 10.0, 20.0, 8, 4)
    }

    #[test]
    fn pointer_maps_to_the_cell_it_is_over() {
        assert_eq!(
            hit(0.0, 0.0),
            CellHit {
                row: 0,
                col: 0,
                side: Side::Left
            }
        );
        assert_eq!(
            hit(25.0, 45.0),
            CellHit {
                row: 2,
                col: 2,
                side: Side::Left
            }
        );
        // Last cell, exactly.
        assert_eq!(
            hit(70.0, 60.0),
            CellHit {
                row: 3,
                col: 7,
                side: Side::Left
            }
        );
    }

    #[test]
    fn side_splits_the_cell_at_its_midpoint() {
        // Cell 2 spans x 20..30, so the midpoint is 25.
        assert_eq!(hit(21.0, 0.0).side, Side::Left);
        assert_eq!(
            hit(25.0, 0.0).side,
            Side::Left,
            "the midpoint itself is left"
        );
        assert_eq!(hit(26.0, 0.0).side, Side::Right);
        // The cell is unaffected by which half.
        assert_eq!(hit(21.0, 0.0).col, 2);
        assert_eq!(hit(29.0, 0.0).col, 2);
    }

    /// Dragging out of the panel must extend to the edge it left through, not
    /// freeze at the last sample inside.

    #[test]
    fn overshoot_clamps_into_the_grid() {
        assert_eq!(hit(9_999.0, 0.0).col, 7);
        assert_eq!(hit(0.0, 9_999.0).row, 3);
        assert_eq!(hit(-50.0, 0.0).col, 0);
        assert_eq!(hit(0.0, -50.0).row, 0);
    }

    /// The part clamping alone does not give you: past the right or bottom
    /// edge the side is forced Right so the last cell is *included*, and above
    /// the top it is forced Left. Dragging below-and-left must still take the
    /// bottom row whole.

    #[test]
    fn overshoot_forces_the_side_to_the_edge() {
        assert_eq!(hit(9_999.0, 10.0).side, Side::Right);
        // x sits in cell 0's left half, but the row overshot — Right wins.
        assert_eq!(hit(1.0, 9_999.0).side, Side::Right);
        assert_eq!(hit(1.0, 9_999.0).col, 0);
        // Above the top, mirrored.
        assert_eq!(hit(75.0, -50.0).side, Side::Left);
    }

    #[test]
    fn degenerate_metrics_do_not_panic() {
        assert_eq!(
            cell_at(5.0, 5.0, 0.0, 20.0, 8, 4),
            CellHit {
                row: 0,
                col: 0,
                side: Side::Left
            }
        );
        assert_eq!(
            cell_at(5.0, 5.0, 10.0, 20.0, 0, 0),
            CellHit {
                row: 0,
                col: 0,
                side: Side::Left
            }
        );
        assert_eq!(cell_at(f32::NAN, f32::INFINITY, 10.0, 20.0, 8, 4).col, 0);
    }

    #[test]
    fn drag_threshold_matches_the_gpui_default() {
        assert_eq!(SELECTION_DRAG_THRESHOLD, 2.0);
    }

    #[test]
    fn glyphs_snap_to_their_cluster_column() {
        let glyph = |index, x: f32| gpui::ShapedGlyph {
            id: gpui::GlyphId(0),
            position: point(px(x), px(0.)),
            index,
            is_emoji: false,
        };
        // `->x` from col 3: a ligature 7px wide for `->`, a combining mark 2px into its base `x`.
        let mut line = ShapedLine::default();
        *line = Arc::new(gpui::LineLayout {
            runs: vec![gpui::ShapedRun {
                font_id: gpui::FontId(0),
                glyphs: vec![glyph(0, 0.), glyph(2, 7.), glyph(2, 9.)],
            }],
            ..Default::default()
        });
        snap_to_cells(&mut line, &[3, 4, 5, 5], 3, px(10.));
        let xs: Vec<f32> = line.runs[0].glyphs.iter().map(|g| f32::from(g.position.x)).collect();
        assert_eq!(xs, [0., 20., 22.]);
    }
}
