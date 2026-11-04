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
use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;

use gpui::{
    App, Bounds, Entity, Focusable, GlobalElementId, Hsla, LayoutId, PaintQuad, Pixels, Point, RenderImage,
    ShapedLine, SharedString, Style, TextRun, Window, fill, font, outline, point, px, relative,
    size,
};

use crate::herdr::graphics::PaneImage;
use crate::settings::CursorEffect;
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

/// The corner radius of the selection and search marks as a share of the row height: Zed's.
const SELECTION_RADIUS: f32 = 0.15;

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
    /// Search matches' wash, under [`Self::selection`].
    mark_quads: Vec<PaintQuad>,
    /// Selection wash, a path per shape. Painted after [`Self::bg_quads`] and
    /// before the glyphs: it has to tint a cell's own background rather than
    /// replace it, and it must not wash out the text it is highlighting.
    selection: Vec<gpui::Path<Pixels>>,
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
    /// [`trail_shapes`], painted under the text.
    trail: Vec<(gpui::Path<Pixels>, gpui::Background)>,
    /// herdr's Kitty images, lowest z first, and whether each goes under the text.
    images: Vec<(Bounds<Pixels>, Arc<RenderImage>, bool)>,
    /// The focused block cursor is opaque, so it goes under the text (whose
    /// cell flips to the background color); the unfocused outline goes over.
    cursor_under_text: bool,
    /// The IME's composing text, underlined over the grid from the cursor cell.
    marked: Option<(Point<Pixels>, ShapedLine)>,
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
        // keyed so the cursor's animation state lives across frames
        Some(("terminal-grid", self.panel.entity_id()).into())
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
        id: Option<&GlobalElementId>,
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
                mark_quads: Vec::new(),
                selection: Vec::new(),
                lines: Vec::new(),
                glyphs: Vec::new(),
                stroke: 1.0,
                cell_w,
                line_h,
                cursor: None,
                trail: Vec::new(),
                images: Vec::new(),
                cursor_under_text: false,
                marked: None,
            };
        };

        let mut bg_quads = Vec::new();
        let mut mark_quads = Vec::new();
        let mut selected = Vec::new();
        let mut lines = Vec::with_capacity(snapshot.lines.len());
        let mut glyphs = Vec::with_capacity(snapshot.lines.len());
        let cursor_rect = snapshot.cursor.as_ref().map(|c| {
            let cell = Bounds::new(
                point(origin.x + cell_w * c.col as f32, origin.y + line_h * c.row as f32),
                size(cell_w, line_h),
            );
            // the shape the program set (DECSCUSR, passed on by herdr): vim's insert beam, an underline
            let bar = px(2.0);
            match (self.focused, c.shape) {
                (true, CursorShape::Beam) => Bounds::new(cell.origin, size(bar, line_h)),
                (true, CursorShape::Underline) => {
                    Bounds::new(point(cell.left(), cell.bottom() - bar), size(cell_w, bar))
                }
                _ => cell,
            }
        });
        // Ghostty's cursor_smear, focused only; the state is dropped on a frame that skips it, so
        // the cursor comes back without a trail after a blur or a hidden spell.
        let settings = &crate::settings::SettingsGlobal::of(cx).settings;
        let effect = settings.animated_cursor.then_some(settings.cursor_effect);
        let trail = match (id, cursor_rect, effect) {
            (Some(id), Some(rect), Some(effect)) if self.focused && !cx.reduce_motion() => {
                window.with_element_state(id, |trail: Option<CursorTrail>, window| {
                    let mut trail = trail.unwrap_or_default();
                    let shapes = trail
                        .track(rect, Instant::now())
                        .and_then(|(from, to, t)| trail_shapes(effect, from, to, t, theme.cursor, &theme.terminal.ansi));
                    if shapes.is_some() {
                        window.request_animation_frame();
                    }
                    (shapes, trail)
                })
            }
            _ => None,
        };
        // Ghostty's block cursor: the cell's text takes the background color.
        let block_cursor =
            snapshot.cursor.as_ref().filter(|c| self.focused && c.shape == CursorShape::Block).map(|c| (c.row, c.col));
        let cursor_text = Hsla { a: 1.0, ..theme.terminal.background };

        // Search matches wash their cells under the selection; the current one stands out.
        for (row, cells, current) in &snapshot.marks {
            let color = if *current { theme.accent.opacity(0.6) } else { theme.warning.opacity(0.3) };
            mark_quads.push(fill(
                Bounds::new(
                    point(origin.x + cell_w * cells.start as f32, origin.y + line_h * *row as f32),
                    size(cell_w * cells.len() as f32, line_h),
                ),
                color,
            )
            .corner_radii(line_h * SELECTION_RADIUS));
        }
        for (row_ix, row) in snapshot.lines.iter().enumerate() {
            let y = origin.y + line_h * row_ix as f32;
            // The emulator's selection is one run per row.
            if let (Some(start), Some(end)) =
                (row.iter().position(|cell| cell.selected), row.iter().rposition(|cell| cell.selected))
            {
                selected.push((row_ix, start..end + 1));
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

        let cursor = cursor_rect.map(|rect| {
            match self.focused {
                true => fill(rect, theme.cursor),
                false => outline(rect, theme.cursor, gpui::BorderStyle::Solid),
            }
            .corner_radii(px(1.))
        });
        let trail = trail
            .into_iter()
            .flatten()
            .filter_map(|(corners, fill)| {
                let mut path = gpui::PathBuilder::fill();
                path.move_to(corners.first()?.map(px));
                for corner in &corners[1..] {
                    path.line_to(corner.map(px));
                }
                path.close();
                path.build().ok().map(|path| (path, fill))
            })
            .collect();

        let selection = selection_outlines(&selected)
            .iter()
            .filter_map(|corners| {
                let at = |c: &Point<usize>| point(origin.x + cell_w * c.x as f32, origin.y + line_h * c.y as f32);
                rounded_path(&corners.iter().map(at).collect::<Vec<_>>(), line_h * SELECTION_RADIUS)
            })
            .collect();

        let images = snapshot
            .images
            .iter()
            .map(|i| (image_bounds(i, origin, cell_w, line_h, scale), i.image.clone(), i.z < 0))
            .collect();

        let marked = snapshot.marked.map(|(text, row, col)| {
            let color = theme.terminal.foreground;
            let underline = gpui::UnderlineStyle { color: Some(color), thickness: px(1.0), wavy: false };
            let run = TextRun {
                len: text.len(),
                font: mono.clone(),
                color,
                background_color: None,
                underline: Some(underline),
                strikethrough: None,
            };
            let line = window.text_system().shape_line(text.into(), font_size, &[run], None);
            (point(origin.x + cell_w * col as f32, origin.y + line_h * row as f32), line)
        });

        TerminalPrepaint {
            bg_quads,
            mark_quads,
            selection,
            lines,
            glyphs,
            // The font's underline thickness is ~0.05em; Ghostty strokes box
            // lines with it. ponytail: gpui doesn't expose the real
            // metric, so approximate it.
            stroke: theme.terminal_font_size * 0.05,
            cell_w,
            line_h,
            cursor,
            trail,
            images,
            cursor_under_text: block_cursor.is_some(),
            marked,
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
        // A selection drag that leaves the pane still follows the pointer: gpui's element move handlers only hear
        // moves over the element, so the window's go to the panel while a drag is out.
        if self.panel.read(cx).selecting() {
            let panel = self.panel.downgrade();
            window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, _, cx| {
                if phase == gpui::DispatchPhase::Bubble {
                    let _ = panel.update(cx, |panel, cx| panel.drag_outside(event.position, cx));
                }
            });
        }
        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            for quad in prepaint.bg_quads.drain(..) {
                window.paint_quad(quad);
            }
            for (bounds, image, _) in prepaint.images.iter().filter(|i| i.2) {
                let _ = window.paint_image(*bounds, Default::default(), image.clone(), 0, false);
            }
            for quad in prepaint.mark_quads.drain(..) {
                window.paint_quad(quad);
            }
            let selection = Theme::of(cx).terminal.selection;
            for path in prepaint.selection.drain(..) {
                window.paint_path(path, selection);
            }
            for (path, fill) in prepaint.trail.drain(..) {
                window.paint_path(path, fill);
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
            // over the cursor and the text under it, on the terminal's background
            if let Some((at, line)) = prepaint.marked.take() {
                let theme = Theme::of(cx);
                window.paint_quad(fill(Bounds::new(at, size(line.width, line_h)), theme.terminal.background));
                let _ = line.paint(at, line_h, gpui::TextAlign::Left, None, window, cx);
            }
        });
        let focus = self.panel.read(cx).focus_handle(cx);
        window.handle_input(&focus, gpui::ElementInputHandler::new(bounds, self.panel.clone()), cx);
    }
}

/// The shapes `rows` (each row with its selected columns, top down) make: every shape's corners
/// in grid lines, clockwise from the top right. Rows that share a column are one shape; a
/// selection from the end of a row to the start of the next shares none and is two.
// ponytail: a selection running past the viewport rounds at its edge too, as Zed's terminal does.
fn selection_outlines(rows: &[(usize, Range<usize>)]) -> Vec<Vec<Point<usize>>> {
    rows.chunk_by(|(a, x), (b, y)| a + 1 == *b && x.start < y.end && y.start < x.end)
        .map(|shape| {
            let side = |edge: fn(&Range<usize>) -> usize| {
                shape
                    .chunk_by(|(_, a), (_, b)| edge(a) == edge(b))
                    .flat_map(|run| [point(edge(&run[0].1), run[0].0), point(edge(&run[0].1), run[run.len() - 1].0 + 1)])
                    .collect::<Vec<_>>()
            };
            let mut corners = side(|cols| cols.end);
            corners.extend(side(|cols| cols.start).into_iter().rev());
            corners
        })
        .collect()
}

/// A filled path through `corners`, each one rounded by `radius`, those facing in as well as out.
fn rounded_path(corners: &[Point<Pixels>], radius: Pixels) -> Option<gpui::Path<Pixels>> {
    let mut path = gpui::PathBuilder::fill();
    for (ix, &at) in corners.iter().enumerate() {
        // the sides are square to the grid; half a side at most, so that a narrow one's two corners don't cross
        let toward = |to: Point<Pixels>| {
            let t = (radius / ((to.x - at.x).abs() + (to.y - at.y).abs())).min(0.5);
            point(at.x + (to.x - at.x) * t, at.y + (to.y - at.y) * t)
        };
        let enter = toward(corners[(ix + corners.len() - 1) % corners.len()]);
        match ix {
            0 => path.move_to(enter),
            _ => path.line_to(enter),
        }
        path.curve_to(toward(corners[(ix + 1) % corners.len()]), at);
    }
    path.close();
    path.build().ok()
}

/// Ghostty's cursor_smear shader (DURATION 0.2, TRAIL_SIZE 0.8, EaseOutCirc): the cursor jumps,
/// and a quad from where it was to where it is trails it, each corner easing in on its own clock:
/// the ones leading the move in DURATION·(1−TRAIL_SIZE), the trailing ones in DURATION, the sides
/// in between. A move under 1.5 cursor heights, typing, leaves no trail.
#[derive(Default)]
struct CursorTrail {
    /// The cursor's rect and since when it has been there.
    at: Option<(Bounds<Pixels>, Instant)>,
    /// Where it was before.
    from: Option<Bounds<Pixels>>,
}

impl CursorTrail {
    const DURATION: f32 = 0.2;
    const TRAIL_SIZE: f32 = 0.8;
    const MIN_DISTANCE: f32 = 1.5;
    const THICKNESS_X: f32 = 0.9;
    /// TL, TR, BR, BL as (x, y) signs; y grows down.
    const CORNERS: [(f32, f32); 4] = [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)];

    /// Notes the cursor at `rect`; the last move: from where, to where, how many seconds ago.
    fn track(&mut self, rect: Bounds<Pixels>, now: Instant) -> Option<(Bounds<Pixels>, Bounds<Pixels>, f32)> {
        if self.at.is_none_or(|(at, _)| at != rect) {
            self.from = self.at.map(|(at, _)| at);
            self.at = Some((rect, now));
        }
        let ((to, since), from) = (self.at?, self.from?);
        Some((from, to, now.duration_since(since).as_secs_f32()))
    }

    /// The warp trail's corners `t` seconds into a move, `None` when there is none to draw.
    fn warp(from: Bounds<Pixels>, to: Bounds<Pixels>, t: f32) -> Option<[Point<Pixels>; 4]> {
        let (to, from) = (Self::corners(to), Self::corners(from));
        let (dx, dy) = (to[2].x - from[2].x, to[2].y - from[2].y);
        let height = to[2].y - to[1].y;
        if t >= Self::DURATION || dx.hypot(dy) <= height * Self::MIN_DISTANCE {
            return None;
        }
        let sign = |v: f32| if v.abs() < 0.5 { 0.0 } else { v.signum() };
        let (sx, sy) = (sign(dx), sign(dy));
        let lead = Self::DURATION * (1.0 - Self::TRAIL_SIZE);
        Some(std::array::from_fn(|i| {
            let (cx, cy) = Self::CORNERS[i];
            // the shader's rails: the edge a sideways move pushes keeps both its corners leading
            let dot = if sx != 0.0 && cx == sx { 2.0 } else { cx * sx + cy * sy };
            let duration = match dot {
                d if d >= 0.5 => lead,
                d if d >= -0.5 => (lead + Self::DURATION) / 2.0,
                _ => Self::DURATION,
            };
            // EaseOutCirc
            let x = (t / duration).clamp(0.0, 1.0);
            let p = (1.0 - (x - 1.0).powi(2)).sqrt();
            point(px(from[i].x + (to[i].x - from[i].x) * p), px(from[i].y + (to[i].y - from[i].y) * p))
        }))
    }

    /// A rect's corners, TL, TR, BR, BL, narrowed by THICKNESS_X.
    fn corners(rect: Bounds<Pixels>) -> [Point<f32>; 4] {
        let c = rect.center();
        let (hw, hh) = (f32::from(rect.size.width) * 0.5 * Self::THICKNESS_X, f32::from(rect.size.height) * 0.5);
        Self::CORNERS.map(|(x, y)| Point::new(f32::from(c.x) + x * hw, f32::from(c.y) + y * hh))
    }
}

/// A filled polygon and what fills it.
type TrailShape = (Vec<Point<f32>>, gpui::Background);

/// Rex's cursor shaders `t` seconds into a move from `from` to `to`, `None` once done. GPUI runs no
/// custom shaders, so each SDF becomes the polygon it fills (the convex hull of the rects it spans),
/// and a glow becomes a grown copy under it, faint.
/// Rex's fixed colors come from the theme: the cursor's, and the ANSI palette's for Frozen, Gradient
/// and Rainbow.
// ponytail: Blaze's accent edge folds into the glow, Rainbow's plasma and Gradient's cycling into a
// two-stop gradient; a real shader pass if GPUI ever takes one.
fn trail_shapes(effect: CursorEffect, from: Bounds<Pixels>, to: Bounds<Pixels>, t: f32, color: Hsla, ansi: &[Hsla; 16]) -> Option<Vec<TrailShape>> {
    use CursorEffect::*;
    type P = Point<f32>;
    let quad = |r: Bounds<Pixels>| {
        let (o, s) = (r.origin.map(f32::from), r.size.map(f32::from));
        [o, point(o.x + s.width, o.y), point(o.x + s.width, o.y + s.height), point(o.x, o.y + s.height)]
    };
    let lerp = |a: P, b: P, k: f32| point(a.x + (b.x - a.x) * k, a.y + (b.y - a.y) * k);
    // a rect's corners pushed `d` out, TL TR BR BL
    let grow = |q: [P; 4], d: f32| std::array::from_fn::<P, 4, _>(|i| {
        let (x, y) = CursorTrail::CORNERS[i];
        point(q[i].x + x * d, q[i].y + y * d)
    });
    let cubic = |x: f32| 1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3);
    let (a, b) = (quad(from), quad(to));
    let (ca, cb) = (lerp(a[0], a[2], 0.5), lerp(b[0], b[2], 0.5));
    let travel = (cb.x - ca.x).hypot(cb.y - ca.y);
    let (w, h) = (f32::from(to.size.width), f32::from(to.size.height));
    // the cursor moved `k` of the way back to where it was
    let back = |k: f32| b.map(|p| point(p.x + (ca.x - cb.x) * k, p.y + (ca.y - cb.y) * k));
    // CSS's angle (0 up, clockwise) of the move, for a gradient from where it was to where it is
    let angle = (cb.x - ca.x).atan2(ca.y - cb.y).to_degrees();
    Some(match effect {
        Warp => vec![(CursorTrail::warp(from, to, t)?.map(|p| p.map(f32::from)).to_vec(), color.into())],
        // cursor_sweep: TRAIL_LENGTH 0.5, DURATION 0.2, EaseOutCubic
        Sweep if t < 0.2 && travel > h * 1.5 => vec![(hull(b.into_iter().chain(back(0.5 * (1.0 - cubic(t / 0.2))))), color.into())],
        // cursor_blaze and kin: DURATION 0.3, the trail reaching back (1−t)³ of the jump
        Blaze | BlazeTapered | BlazeNoTrail | Frozen if t < 0.3 => {
            let e = (1.0 - t / 0.3).powi(3);
            let (core, glow): (Hsla, Hsla) = match effect {
                // Rex's ice cyan edged in blue
                Frozen => (ansi[6], ansi[4]),
                _ => (color, color),
            };
            let tail: Vec<P> = match effect {
                BlazeNoTrail => Vec::new(),
                BlazeTapered => vec![lerp(cb, ca, e)],
                _ => back(e).to_vec(),
            };
            const HALO: f32 = 3.0;
            let halo = grow(b, HALO).into_iter().chain(tail.iter().flat_map(|&p| grow([p; 4], HALO)));
            let mut shapes = vec![(hull(halo), glow.opacity(0.5 * e).into())];
            if effect != BlazeNoTrail {
                shapes.push((hull(b.into_iter().chain(tail)), core.into()));
            }
            shapes
        }
        // Rex's comet: DURATION 0.32, a tail of WIDTH 0.42 heights that eases into the head
        Comet if t < 0.32 && travel >= w * 1.5 => {
            let age = t / 0.32;
            let tail = lerp(ca, cb, cubic(age));
            let r = h * 0.42;
            let tint = Hsla { l: color.l + (1.0 - color.l) * 0.25, ..color };
            let ramp = |alpha: f32| {
                gpui::linear_gradient(angle, gpui::linear_color_stop(tint.opacity(0.0), 0.0), gpui::linear_color_stop(tint.opacity(alpha), 1.0))
            };
            let fade = 1.0 - age;
            vec![(capsule(tail, cb, r * 0.45, r * 2.2), ramp(0.3 * fade)), (capsule(tail, cb, r * 0.2, r), ramp(fade))]
        }
        // cursor_rainbow / cursor_gradient: a hexagon whose trailing corner lags, the sides catching up twice as fast
        Rainbow | Gradient if t < if effect == Rainbow { 0.5 } else { 0.25 } => {
            let duration = if effect == Rainbow { 0.5 } else { 0.25 };
            let sign = |v: f32| if v.abs() < 0.5 { 0.0 } else { v.signum() };
            let (sx, sy) = (sign(cb.x - ca.x), sign(cb.y - ca.y));
            let dots = CursorTrail::CORNERS.map(|(x, y)| x * sx + y * sy);
            let least = dots.iter().copied().fold(f32::MAX, f32::min);
            let (slow, fast) = (cubic(t / duration), cubic(2.0 * t / duration));
            let lagging = (0..4).map(|i| lerp(a[i], b[i], if least < 0.0 && dots[i] == least { slow } else { fast }));
            let fill = match effect {
                Rainbow => {
                    // the theme's red, yellow, green, cyan, blue, magenta, turning with time and with where the cursor lands
                    const RING: [usize; 6] = [1, 3, 2, 6, 4, 5];
                    let at = (((cb.x + cb.y) / 600.0 + t * 2.0).fract() * 6.0) as usize;
                    gpui::linear_gradient(
                        angle,
                        gpui::linear_color_stop(ansi[RING[at % 6]].opacity(0.0), 0.0),
                        gpui::linear_color_stop(ansi[RING[(at + 2) % 6]], 1.0),
                    )
                }
                // Rex's gold to blue, top down
                _ => gpui::linear_gradient(
                    180.0,
                    gpui::linear_color_stop(ansi[3].opacity(0.8), 0.0),
                    gpui::linear_color_stop(ansi[4].opacity(0.8), 1.0),
                ),
            }
            .color_space(gpui::ColorSpace::Oklab);
            vec![(hull(b.into_iter().chain(lagging)), fill)]
        }
        _ => return None,
    })
}

/// The convex hull of `points` (Andrew's monotone chain).
fn hull(points: impl IntoIterator<Item = Point<f32>>) -> Vec<Point<f32>> {
    let mut points: Vec<_> = points.into_iter().collect();
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    if points.len() < 3 {
        return points;
    }
    let turn = |o: Point<f32>, a: Point<f32>, b: Point<f32>| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let mut hull: Vec<Point<f32>> = Vec::with_capacity(points.len() * 2);
    for &p in &points {
        while hull.len() >= 2 && turn(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(p);
    }
    let lower = hull.len() + 1;
    for &p in points[..points.len() - 1].iter().rev() {
        while hull.len() >= lower && turn(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
            hull.pop();
        }
        hull.push(p);
    }
    hull.pop();
    hull
}

/// A tapered capsule from a `tail` of radius `r0` to a `head` of radius `r1`, its ends round.
fn capsule(tail: Point<f32>, head: Point<f32>, r0: f32, r1: f32) -> Vec<Point<f32>> {
    let along = (head.y - tail.y).atan2(head.x - tail.x);
    let arc = |c: Point<f32>, r: f32, start: f32| {
        (0..=8).map(move |i| {
            let a = start + std::f32::consts::PI * i as f32 / 8.0;
            point(c.x + r * a.cos(), c.y + r * a.sin())
        })
    };
    let half = std::f32::consts::FRAC_PI_2;
    arc(head, r1, along - half).chain(arc(tail, r0, along + half)).collect()
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
    fn a_jump_trails_the_cursor_leading_corners_first() {
        let cell = |col: f32| Bounds::new(point(px(col * 10.), px(0.)), size(px(10.), px(20.)));
        let t0 = Instant::now();
        let at = |ms: u64| t0 + std::time::Duration::from_millis(ms);
        let mut trail = CursorTrail::default();
        let mut step = |rect, now| trail.track(rect, now).and_then(|(from, to, t)| CursorTrail::warp(from, to, t));
        assert!(step(cell(0.), t0).is_none(), "the first sighting has no trail");
        assert!(step(cell(1.), t0).is_none(), "nor does typing");
        assert!(step(cell(10.), t0).is_some(), "a jump does");
        let c = step(cell(10.), at(100)).unwrap();
        // moving right: the right edge has landed, at the center plus 0.9 of the half width
        assert_eq!((c[1].x, c[2].x), (px(109.5), px(109.5)));
        assert!(c[0].x > px(0.5) && c[0].x < px(100.5), "{c:?}");
        assert!(step(cell(10.), at(200)).is_none(), "and it is gone after DURATION");
    }

    #[test]
    fn every_cursor_effect_draws_a_jump_and_is_gone_after_it() {
        let cell = |col: f32, row: f32| Bounds::new(point(px(col * 10.), px(row * 20.)), size(px(10.), px(20.)));
        for effect in CursorEffect::ALL {
            let shapes = trail_shapes(effect, cell(0., 0.), cell(10., 3.), 0.05, gpui::red(), &[gpui::red(); 16]).unwrap_or_else(|| panic!("{effect:?}"));
            assert!(shapes.iter().all(|(p, _)| p.len() >= 3), "{effect:?}");
            assert!(trail_shapes(effect, cell(0., 0.), cell(10., 3.), 0.6, gpui::red(), &[gpui::red(); 16]).is_none(), "{effect:?}");
        }
        // two cells a diagonal apart: a hexagon, the corners each cell hides dropped
        let square = |x: f32| [(0., 0.), (1., 0.), (1., 1.), (0., 1.)].map(|(a, b)| point(a + x, b + x));
        assert_eq!(hull(square(0.).into_iter().chain(square(2.))).len(), 6);
    }

    #[test]
    fn a_selection_is_one_shape_while_its_rows_touch() {
        let corners = |rows: &[(usize, Range<usize>)]| {
            let shapes = selection_outlines(rows);
            shapes.iter().map(|s| s.iter().map(|c| (c.x, c.y)).collect::<Vec<_>>()).collect::<Vec<_>>()
        };
        assert_eq!(corners(&[(2, 3..7)]), [[(7, 2), (7, 3), (3, 3), (3, 2)]], "a row is a rectangle");
        assert_eq!(
            corners(&[(0, 3..10), (1, 0..10), (2, 0..5)]),
            [[(10, 0), (10, 2), (5, 2), (5, 3), (0, 3), (0, 1), (3, 1), (3, 0)]],
            "first and last rows step in, full rows leave no corners"
        );
        assert_eq!(
            corners(&[(0, 8..10), (1, 0..8)]),
            [[(10, 0), (10, 1), (8, 1), (8, 0)], [(8, 1), (8, 2), (0, 2), (0, 1)]],
            "rows that only meet at a corner are two shapes"
        );
    }

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
