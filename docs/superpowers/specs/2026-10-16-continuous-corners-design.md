# Continuous corners in the zui fork (macOS)

Date: 2026-10-16

## Goal

Every rounded corner Shepherd draws on macOS has Apple's continuous curve (`CALayer.cornerCurve = .continuous`,
Flutter's `RoundedSuperellipseBorder`) instead of a circular arc, so Shepherd's popups, cards, buttons and panels
match the system's. No API change: `.rounded(px(10.))` keeps its meaning (the radius Apple's `cornerRadius` takes)
and gets the continuous curve.

Success: for `r` = 8, 20 and 40 px, the coverage a gpui quad with radius `r` differs from a `CALayer` with
`cornerRadius = r` and the continuous curve by less than half of what the circular curve differs by (the spike
measured 0.33–0.41 at 1×); Shepherd shows no antialiasing or border artifacts at corners.

## Scope

In: the Metal renderer (`crates/gpui_macos`) of the fork `freethinkel/zui`, branched from
`metal-three-drawables`, and everything there that rounds by corner radius:
- quads: fill, uniform borders, borders of different widths per side, dashed borders;
- unblurred shadows and the shadow's element clip;
- sprites with corner radii (the rounded image alpha masks, #9);
- backdrop blur bounds.

Out:
- The wgpu renderer (Linux, Windows): its corners stay circular.
- Blurred shadows: `blur_along_x` integrates a circular corner analytically; under blur the two shapes differ by
  less than a pixel, so they keep the circle.
- An opt-out per element or app-wide. Add one when something needs a circle.

## The curve (from the spike)

Apple's corner of radius `r` is matched by a superellipse `|x|^n + |y|^n = s^n` in the corner's `s × s` square, with
`n = 2.7` and span `s = 1.28 r`. The spike fitted it against `CALayer` renders at 8× scale for `r` = 8, 12, 20
and 40: mean contour error 0.03–0.28 px, where a circle is off by up to 1.4–10.8 px. All four radii fit the same
pair.

Where the span would cross the middle of the shorter side (pills, `rounded_full`), the span is clamped to half that
side and `n` eases back to 2:

```
span = min(1.28 r, min(half_size))
n    = mix(2.0, 2.7, saturate((span - r) / (0.28 r)))
```

A full pill is then a circle. Apple's pill is a different shape again (about 8.7 px² per end at 200 × 40);
matching it is out of scope.

## Design

One place decides a corner's shape, and every SDF goes through it.

1. `corner_shape(float r, float2 half_size) -> float2 (span, n)` in `shaders.metal`, from the formula above.
2. `quad_sdf_impl(corner_center_to_point, span, n)` takes `n`. Inside the corner it returns
   `(‖p / span‖_n − 1) · span`, with `p = max(0, corner_center_to_point)`. `n = 2` keeps today's `length` path. An
   Lp norm with `n > 2` under-reports distance off the axes, by at most 14 % at 45°. The SDF is divided by its
   gradient's length, so the antialiasing band stays one pixel wide.
3. `quad_sdf(point, bounds, corner_radii)` calls `corner_shape` and so serves shadows, sprites and blur unchanged at
   their call sites.
4. `quad_fragment` computes `(span, n)` once, right after `pick_corner_radius`, and uses `span` wherever it uses
   `corner_radius` today (the corner zone, `corner_center_to_point`, the fast paths).
5. Borders:
   - uniform width: the inner edge stays `-(outer_sdf + border)`, the curve's offset, which is what a stroked
     continuous path gives;
   - different widths: `quarter_ellipse_sdf` becomes a superellipse with radii `span − border` and the same `n`
     (`n = 2` is the old ellipse);
   - dashed: a corner's arc length becomes `span · L(n)`, where `L(2) = π/2` and `L(2.7) = 1.6582` (interpolated
     linearly between them). The position along a corner keeps the angle parameter.
6. Rust and the instance layout don't change: corner radii are already in every primitive, and `n` is derived in the
   shader.

## Testing

In `metal_renderer.rs`, next to the existing headless render tests (`render_to_image`):

- `continuous_corner_matches_core_animation`: for `r` = 8, 20 and 40, render a white quad on black at 1× with
  `MetalRenderer::new_headless`, and render a `CALayer` of the same size with `cornerRadius = r` and the
  continuous curve into a `CGContext` (through `objc` and `core-graphics`, already dependencies), and once more with
  the circular curve. Summed coverage error against the continuous render < 0.5 × the circular render's.
- `pill_corner_is_a_circle`: a 200 × 40 quad with `r = 20` covers the same pixels as the circular SDF used to.
- `uniform_border_follows_the_curve`: a quad with `r = 20` and a 1 px border; the border's coverage in the 40 × 40
  corner square is `L(2.7) · 25.6 + 2 · (40 − 25.6) ≈ 71 px²` ± 10 % (a circular inner edge gives about 119).

The existing renderer tests must still pass. Shepherd is checked by eye: command panel, popups, sidebar
selection, switches (pills), images with rounded corners.

## Delivery

Shepherd's `[patch."https://github.com/zeronsh/zui"]` moves to the new fork revision. If the three-drawables
change is merged upstream before this one, this branch is rebased onto upstream's main.
