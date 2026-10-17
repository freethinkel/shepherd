//! Zeron's loaders (`crates/ui/src/loaders.rs`): the gradient matrix spinner, a 3×3 grid tinted per row with a pulse
//! travelling up it, and its 2×3 variant for compact slots, with the pulse running round the edge: in one color, or
//! (an agent at work) a tint per row.
use gpui::{App, Div, EntityId, Hsla, ParentElement, Styled, Window, canvas, div, px};
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// One pulse across the cells (Zeron's `GRADIENT_SPIN`).
const PERIOD: f32 = 0.75;
/// How dim a cell rests between pulses (Zeron's `GSPIN_DIM`).
const DIM: f32 = 0.1;
/// An agent's spinner rests brighter, so its shape still reads as a glyph between pulses.
const AGENT_DIM: f32 = 0.45;
/// Zeron's sunrise gradient, a tint per row top to bottom.
const ROW_TINTS: [u32; 3] = [0xB6D3EF, 0xEDB185, 0xF888A0];
/// A wait shorter than this shows no loader, so a quick local connect doesn't flash one.
const GRACE: Duration = Duration::from_millis(300);

/// Whether a wait since `since` has gone on long enough to show a loader; asks for frames until it has.
pub fn due(since: Instant, window: &mut Window) -> bool {
    // a wait that starts later (the fixture's panes) asks none yet
    if since <= Instant::now() {
        request_slow_frame(window);
    }
    since.elapsed() >= GRACE
}

/// The 3×3 gradient spinner: each cell flares once a period, the bottom row first, the top middle last.
pub fn spinner(cell: f32, window: &mut Window) -> Div {
    grid(cell, 3, 3, |row, _| gpui::rgb(ROW_TINTS[row]).into(), |row, col| (3. - row as f32 + (col as f32 - 1.).abs()) / 4., window)
}

/// The 2×3 spinner in one color, the flare going round clockwise from the top left.
pub fn mini_spinner(cell: f32, tint: Hsla, window: &mut Window) -> Div {
    grid(cell, 3, 2, |_, _| tint, ring, window)
}

/// A spinner's pace: ~15 fps, not the display's 120, since every frame redraws the window and that cost a core for a
/// few dots.
const SLOW_FRAME: Duration = Duration::from_millis(66);

thread_local! {
    static SLOW: RefCell<HashSet<EntityId>> = RefCell::default();
}

/// Redraw the view being drawn in [`SLOW_FRAME`], where `request_animation_frame` redraws it on the display's next.
pub fn request_slow_frame(window: &Window) {
    SLOW.with(|s| s.borrow_mut().insert(window.current_view()));
}

/// For the app's life: every [`SLOW_FRAME`] the views that asked for one are redrawn.
pub fn drive_slow_frames(cx: &mut App) {
    cx.spawn(async move |cx| loop {
        cx.background_executor().timer(SLOW_FRAME).await;
        let views = SLOW.with(|s| std::mem::take(&mut *s.borrow_mut()));
        if !views.is_empty() {
            cx.update(|cx| views.into_iter().for_each(|view| cx.notify(view)));
        }
    })
    .detach();
}

/// Zeron's `mini_glyph_spinner`, an agent at work: [`mini_spinner`] with a tint per row, for a slot with no window: it
/// asks its slow frames as it is laid out.
pub fn glyph_spinner(cell: f32, rows: [Hsla; 3]) -> Div {
    cells(cell, 3, 2, |row, _| rows[row], ring, periods(), AGENT_DIM)
        .child(canvas(|_, window, _| request_slow_frame(window), |_, _, _, _| {}).absolute())
}

/// The 2×3 spinners' phases: clockwise round the edge from the top left.
fn ring(row: usize, col: usize) -> f32 {
    const RING: [[usize; 2]; 3] = [[0, 1], [5, 2], [4, 3]];
    RING[row][col] as f32 / 6.
}

fn grid(cell: f32, rows: usize, cols: usize, tint: impl Fn(usize, usize) -> Hsla, phase: impl Fn(usize, usize) -> f32, window: &mut Window) -> Div {
    request_slow_frame(window);
    cells(cell, rows, cols, tint, phase, periods(), DIM)
}

/// The periods since the first spinner showed: every spinner on one clock.
fn periods() -> f32 {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    EPOCH.get_or_init(Instant::now).elapsed().as_secs_f32() / PERIOD
}

/// The cells at `t` periods in, resting at `dim`.
fn cells(cell: f32, rows: usize, cols: usize, tint: impl Fn(usize, usize) -> Hsla, phase: impl Fn(usize, usize) -> f32, t: f32, dim: f32) -> Div {
    div().flex().flex_col().gap(px(cell / 2.)).children((0..rows).map(|row| {
        div().flex().gap(px(cell / 2.)).children((0..cols).map(|col| {
            div().size(px(cell)).rounded(px(cell / 2.)).bg(tint(row, col)).opacity(opacity(t + phase(row, col), dim))
        }))
    }))
}

/// A cell's opacity at `t` periods in: full at the start, down to `dim` by 45 %, resting there until 92 %, then
/// back up (Zeron's `gspin_opacity`).
fn opacity(t: f32, dim: f32) -> f32 {
    let t = t.rem_euclid(1.);
    if t < 0.45 {
        1. + (dim - 1.) * t / 0.45
    } else if t < 0.92 {
        dim
    } else {
        dim + (1. - dim) * (t - 0.92) / 0.08
    }
}

#[cfg(test)]
mod tests {
    use super::{DIM, opacity};

    #[test]
    fn a_cell_dims_rests_and_flares_once_a_period() {
        let opacity = |t| opacity(t, DIM);
        assert_eq!(opacity(0.), 1.);
        assert!((opacity(0.45) - 0.1).abs() < 1e-5);
        assert_eq!(opacity(0.7), 0.1);
        assert!((opacity(0.999) - 1.).abs() < 0.02);
        assert_eq!(opacity(1.25), opacity(0.25));
    }
}
