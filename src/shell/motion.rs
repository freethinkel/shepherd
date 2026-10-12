//! Tweens and springs: the sidebar's slide and edge nudge, worktree folds, and the seams that light up on hover.
use super::*;

/// Zeron's sidebar disclosure (`motion::COLLAPSE`): 180 ms CSS ease-out.
pub(super) const FOLD_DURATION: Duration = Duration::from_millis(180);

/// CSS `ease-out`, cubic-bezier(0, 0, 0.58, 1), solved for x by bisection.
pub(super) fn ease_out(x: f32) -> f32 {
    if x <= 0. || x >= 1. {
        return x.clamp(0., 1.);
    }
    let bezier = |p2: f32, t: f32| 3. * p2 * t * t * (1. - t) + t * t * t;
    let (mut lo, mut hi) = (0f32, 1f32);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.;
        if bezier(0.58, mid) < x { lo = mid } else { hi = mid }
    }
    bezier(1., (lo + hi) / 2.).clamp(0., 1.)
}

/// A damped spring (ζ 0.7) over its duration: a quick rise, about 5 % past the end, then home.
pub(super) fn spring(t: f32) -> f32 {
    damped_spring(0.7, t)
}

/// A spring damped by `zeta` over its duration: the closer to 1, the less it goes past the end.
pub(super) fn damped_spring(zeta: f32, t: f32) -> f32 {
    if t >= 1. {
        return 1.;
    }
    let omega = 14f32;
    let damped = omega * (1. - zeta * zeta).sqrt();
    1. - (-zeta * omega * t).exp() * ((damped * t).cos() + zeta * omega / damped * (damped * t).sin())
}

pub(super) const SPRING_DURATION: Duration = Duration::from_millis(240);

/// Zeron's resize-edge spring (`motion::resize_bounce_offset`): 5 px out over the first 32 % of 220 ms, then
/// home over the rest, smoothstep both ways.
pub(super) const EDGE_SPRING: Duration = Duration::from_millis(220);

pub(super) fn edge_nudge(t: f32) -> f32 {
    let smooth = |t: f32| {
        let t = t.clamp(0., 1.);
        t * t * (3. - 2. * t)
    };
    let out = 0.32;
    5. * if t < out { smooth(t / out) } else { 1. - smooth((t - out) / (1. - out)) }
}

/// Zeron's resize seam (`Shell::resize_handle`): a 1 px line (the accent) `at` px into the hit strip that
/// fades out toward both ends. Shown while dragged; `hover` (hovered, flip count) fades it in and out like Zeron's
/// `HOVER_FADE`, restarting on every flip.
pub(super) fn resize_seam(
    id: SharedString,
    vertical: bool,
    at: f32,
    color: Hsla,
    dragging: bool,
    hover: Option<(bool, u64)>,
) -> gpui::AnyElement {
    let clear = color.opacity(0.);
    let angle = if vertical { 180. } else { 90. };
    let half = |from, to| {
        div().flex_1().bg(gpui::linear_gradient(
            angle,
            gpui::linear_color_stop(from, 0.),
            gpui::linear_color_stop(to, 1.),
        ))
    };
    let line = div().absolute().flex().child(half(clear, color)).child(half(color, clear));
    let line = if vertical {
        line.flex_col().top_0().bottom_0().left(px(at)).w(px(1.))
    } else {
        line.flex_row().left_0().right_0().top(px(at)).h(px(1.))
    };
    match (dragging, hover) {
        (true, _) => line.into_any_element(),
        (false, Some((on, flips))) => line
            .with_animation(
                SharedString::from(format!("{id}-fade-{flips}")),
                gpui::Animation::new(SEAM_FADE).with_easing(ease_out),
                move |el, t| el.opacity(if on { t } else { 1. - t }),
            )
            .into_any_element(),
        (false, None) => line.invisible().into_any_element(),
    }
}

pub(super) const SEAM_FADE: Duration = Duration::from_millis(150);

/// A pane divider's hit strip, centred on the panes' shared edge.
pub(super) const PANE_SEAM_HIT: f32 = 10.;

/// A pane divider: a 1 px `rest` line always on the panes' shared edge (the strip's centre), and over it the
/// sidebar's `lit` seam (`resize_seam`, fading toward both ends) while hovered or dragged.
pub(super) fn pane_seam(id: SharedString, vertical: bool, rest: Hsla, lit: Hsla, dragging: bool, hover: Option<(bool, u64)>) -> Div {
    let at = (PANE_SEAM_HIT - 1.) / 2.;
    let line = div().absolute().bg(rest);
    let line = if vertical { line.top_0().bottom_0().left(px(at)).w(px(1.)) } else { line.left_0().right_0().top(px(at)).h(px(1.)) };
    div().absolute().inset_0().child(line).child(resize_seam(id, vertical, at, lit, dragging, hover))
}

pub(super) fn fold_animation() -> gpui::Animation {
    gpui::Animation::new(FOLD_DURATION).with_easing(ease_out)
}

/// A group body's height tween; `epoch` restarts the animation on every toggle.
#[derive(Clone, Copy)]
pub(super) struct FoldMotion {
    pub(super) epoch: u64,
    pub(super) from: f32,
    pub(super) to: f32,
    pub(super) started: Instant,
}

impl FoldMotion {
    pub(super) fn current(self) -> f32 {
        let t = (self.started.elapsed().as_secs_f32() / FOLD_DURATION.as_secs_f32()).min(1.);
        self.from + (self.to - self.from) * ease_out(t)
    }

    /// A little past the end, so the last frame lands before the static height takes over.
    pub(super) fn animating(self) -> bool {
        self.started.elapsed() < FOLD_DURATION + Duration::from_millis(120)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spring_overshoots_and_settles() {
        assert_eq!(spring(0.), 0.);
        let peak = (0..100).map(|i| spring(i as f32 / 100.)).fold(0f32, f32::max);
        assert!(peak > 1.02 && peak < 1.08, "{peak}");
        assert!((spring(0.99) - 1.).abs() < 0.001);
    }

    #[test]
    fn edge_nudge_springs_out_and_home() {
        assert_eq!(edge_nudge(0.), 0.);
        assert_eq!(edge_nudge(0.32), 5.);
        assert_eq!(edge_nudge(1.), 0.);
    }

    #[test]
    fn ease_out_matches_css() {
        assert_eq!(ease_out(0.), 0.);
        assert_eq!(ease_out(1.), 1.);
        // CSS ease-out at x = 0.5 is ≈ 0.684
        assert!((ease_out(0.5) - 0.684).abs() < 0.01);
    }
}
