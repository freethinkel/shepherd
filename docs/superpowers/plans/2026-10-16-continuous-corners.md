# Continuous Corners Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every rounded corner gpui's Metal renderer draws has Apple's continuous curve instead of a circular arc, with no API change.

**Architecture:** One shader function, `corner_shape(r, half_size) -> (span, n)`, turns a corner radius into a superellipse (`n = 2.7` over `1.28 r`, easing back to a circle where the span is clamped). The quad SDF (`quad_sdf_impl`) takes `(span, n)`; `quad_sdf` (shadows, sprites, blur) and `quad_fragment` (fills, borders, dashes) both go through it. Rust and the instance layout don't change.

**Tech Stack:** Metal Shading Language (`crates/gpui_macos/src/shaders.metal`), Rust tests on the headless `MetalRenderer`, Core Animation (`CALayer`) through `objc`/`core-graphics` as the test oracle.

**Spec:** `docs/superpowers/specs/2026-10-16-continuous-corners-design.md` (in Shepherd)

## Global Constraints

- Work happens in the fork `freethinkel/zui`, branch `continuous-corners` off `metal-three-drawables` (commit `98b1dd8`), cloned at `~/Developer/pet/zui`.
- Metal renderer only: `crates/gpui_wgpu` is not touched.
- Curve constants, verbatim: `span = min(1.28 r, min(half_size))`, `n = mix(2.0, 2.7, saturate((span - r) / (0.28 r)))`, `L(2) = π/2`, `L(2.7) = 1.6582`.
- No change to Rust primitives, their fields or the instance buffer layout.
- Blurred shadows (`blur_along_x`) stay circular.
- Tests run with `--features runtime_shaders` (this machine has no Metal Toolchain for precompiled shaders): `cargo test -p gpui_macos --lib --features runtime_shaders`.
- Commit messages end with:
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01AJRVE4ptTQby4VP165isQr
  ```

## Review Focus

1. Different radii per corner (`rounded_t`, `rounded_tl`): each corner takes its own span, square corners stay square. Test: `each_corner_takes_its_own_radius` (Task 2).
2. Small quads, where `1.28 r` passes the middle (a 12 × 12 badge with `r = 6`): a circle, no overshoot. Test: `small_quad_corner_stays_a_circle` (Task 2).
3. Sub-pixel radii (`r = 0.5`): no NaN from `pow`, the quad stays filled. Test: `half_pixel_radius_stays_filled` (Task 2).
4. A border wider than the corner (`r = 4`, border 6) or different widths per side: the inner edge's radii reach 0, and the fill has no holes. Tests: `thick_border_leaves_no_holes`, `uneven_border_leaves_no_holes` (Task 3).
5. Inset and unblurred shadows clip to the same curve as their element. Test: `unblurred_shadow_takes_the_same_curve` (Task 2).

---

### Task 1: Core Animation oracle and the failing corner test

**Files:**
- Create: `crates/gpui_macos/src/metal_renderer/continuous_corner_tests.rs`
- Modify: `crates/gpui_macos/src/metal_renderer.rs` (end of file: declare the module)

**Interfaces:**
- Produces (test-only, in `continuous_corner_tests`):
  - `fn ca_mask(w: usize, h: usize, r: f64, curve: &str) -> Vec<f32>`: Core Animation's coverage (0–1) of a white `w × h` rect with `cornerRadius = r`, `curve` = `"continuous"` or `"circular"`.
  - `fn gpui_mask(w: i32, h: i32, draw: impl FnOnce(&mut Scene)) -> Vec<f32>`: what `draw` paints in white over black, as coverage (red channel / 255).
  - `fn white_quad(b: Bounds<ScaledPixels>, radii: Corners<ScaledPixels>) -> Quad`
  - `fn radii(r: f32) -> Corners<ScaledPixels>`
  - `fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<ScaledPixels>`
  - `fn diff(a: &[f32], b: &[f32]) -> f32`: summed absolute difference, px².

- [ ] **Step 1: Clone the fork and branch**

```bash
git clone https://github.com/freethinkel/zui ~/Developer/pet/zui
cd ~/Developer/pet/zui
git checkout -b continuous-corners origin/metal-three-drawables
cargo test -p gpui_macos --lib --features runtime_shaders 2>&1 | grep "test result"
```

Expected: `test result: ok. 17 passed`.

- [ ] **Step 2: Declare the test module**

At the very end of `crates/gpui_macos/src/metal_renderer.rs`:

```rust
#[cfg(test)]
mod continuous_corner_tests;
```

- [ ] **Step 3: Write the oracle, the helpers and the failing test**

`crates/gpui_macos/src/metal_renderer/continuous_corner_tests.rs`:

```rust
//! Continuous corners: gpui's rounded quads against Core Animation's `cornerCurve = .continuous`.
//! Shapes here are symmetric, so the bitmap's row order (Core Graphics is y-up) doesn't matter.
use super::*;
use cocoa::base::{id, nil};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};
use core_graphics::{
    base::kCGImageAlphaNone, color::CGColor, color_space::CGColorSpace, context::CGContext,
};
use gpui::{BorderStyle, Corners, Edges, Hsla, solid_background, transparent_black};

pub(super) fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<ScaledPixels> {
    Bounds {
        origin: point(ScaledPixels(x), ScaledPixels(y)),
        size: size(ScaledPixels(w), ScaledPixels(h)),
    }
}

pub(super) fn radii(r: f32) -> Corners<ScaledPixels> {
    Corners::all(ScaledPixels(r))
}

fn gray(l: f32) -> Hsla {
    Hsla { h: 0., s: 0., l, a: 1. }
}

pub(super) fn white_quad(b: Bounds<ScaledPixels>, corner_radii: Corners<ScaledPixels>) -> Quad {
    Quad {
        order: 0,
        border_style: BorderStyle::default(),
        bounds: b,
        content_mask: ContentMask { bounds: b },
        background: solid_background(gray(1.)),
        border_color: transparent_black(),
        corner_radii,
        border_widths: Edges::default(),
        fade: gpui::EdgeFadeParams::default(),
    }
}

/// What `draw` paints in white over a black backdrop, as coverage: the red channel over 255.
pub(super) fn gpui_mask(w: i32, h: i32, draw: impl FnOnce(&mut Scene)) -> Vec<f32> {
    let mut scene = Scene::default();
    let mut backdrop = white_quad(bounds(0., 0., w as f32, h as f32), Corners::default());
    backdrop.background = solid_background(gray(0.));
    scene.insert_primitive(backdrop);
    draw(&mut scene);
    scene.finish();
    let mut renderer =
        MetalRenderer::new_headless(Arc::new(Mutex::new(InstanceBufferPool::default())));
    let image = renderer
        .render_scene_to_image(&scene, size(DevicePixels(w), DevicePixels(h)))
        .expect("render_scene_to_image failed");
    image.pixels().map(|p| p[0] as f32 / 255.).collect()
}

/// Core Animation's coverage of a white `w × h` rect with `cornerRadius = r` and the given
/// `cornerCurve` (`"continuous"` or `"circular"`), at 1×.
pub(super) fn ca_mask(w: usize, h: usize, r: f64, curve: &str) -> Vec<f32> {
    let mut ctx = CGContext::create_bitmap_context(
        None,
        w,
        h,
        8,
        w,
        &CGColorSpace::create_device_gray(),
        kCGImageAlphaNone,
    );
    unsafe {
        let layer: id = msg_send![class!(CALayer), layer];
        let frame = NSRect::new(NSPoint::new(0., 0.), NSSize::new(w as f64, h as f64));
        let _: () = msg_send![layer, setFrame: frame];
        let white = CGColor::rgb(1., 1., 1., 1.);
        let _: () = msg_send![layer, setBackgroundColor: white.as_concrete_TypeRef()];
        let _: () = msg_send![layer, setCornerRadius: r];
        let _: () = msg_send![layer, setCornerCurve: NSString::alloc(nil).init_str(curve)];
        let _: () = msg_send![layer, renderInContext: ctx.as_ptr()];
    }
    ctx.data().iter().map(|&v| v as f32 / 255.).collect()
}

/// Summed absolute coverage difference, in px².
pub(super) fn diff(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(a, b)| (a - b).abs()).sum()
}

#[test]
fn continuous_corner_matches_core_animation() {
    for r in [8.0f32, 20.0, 40.0] {
        let ours = gpui_mask(200, 200, |s| {
            s.insert_primitive(white_quad(bounds(0., 0., 200., 200.), radii(r)));
        });
        let apple = ca_mask(200, 200, r as f64, "continuous");
        let circle = ca_mask(200, 200, r as f64, "circular");
        let (e, base) = (diff(&ours, &apple), diff(&circle, &apple));
        assert!(e < 0.5 * base, "r = {r}: {e:.2} px² off Apple's curve, the circle is {base:.2}");
    }
}

#[test]
fn pill_corner_is_a_circle() {
    let ours = gpui_mask(200, 40, |s| {
        s.insert_primitive(white_quad(bounds(0., 0., 200., 40.), radii(20.)));
    });
    let circle = ca_mask(200, 40, 20., "circular");
    let apple = ca_mask(200, 40, 20., "continuous");
    let (e, base) = (diff(&ours, &circle), diff(&circle, &apple));
    assert!(e < 0.25 * base, "pill: {e:.2} px² off a circle, Apple's continuous pill is {base:.2} off");
}
```

If a `core-graphics` 0.24 name differs (`CGColor::rgb`, `CGContext::data`), use the crate's equivalent; the behavior stays as written.

- [ ] **Step 4: Run them**

Run: `cargo test -p gpui_macos --lib --features runtime_shaders continuous_corner_tests -- --nocapture`

Expected: `continuous_corner_matches_core_animation` FAILS with `r = 8: … px² off Apple's curve, the circle is …` (today's quad is the circle, so the ratio is about 1). `pill_corner_is_a_circle` PASSES: it guards the pill, which must stay as it is.

- [ ] **Step 5: Commit**

```bash
git add crates/gpui_macos/src/metal_renderer.rs crates/gpui_macos/src/metal_renderer/continuous_corner_tests.rs
git commit -m "Test rounded quads against Core Animation's continuous corners

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01AJRVE4ptTQby4VP165isQr"
```

---

### Task 2: The continuous curve in the quad SDF

**Files:**
- Modify: `crates/gpui_macos/src/shaders.metal` (declarations near line 26–30; `quad_fragment` near lines 137, 157, 188; `quad_sdf` and `quad_sdf_impl` near lines 1087–1115)
- Test: `crates/gpui_macos/src/metal_renderer/continuous_corner_tests.rs`

**Interfaces:**
- Consumes: Task 1's `gpui_mask`, `ca_mask`, `white_quad`, `radii`, `bounds`, `diff`.
- Produces (MSL):
  - `float2 corner_shape(float corner_radius, float2 half_size)`: `(span, n)`; `(0, 2)` for `corner_radius <= 0`.
  - `float corner_curve_sdf(float2 p, float span, float n)`: signed distance to the corner's curve, `p >= 0` relative to the corner's center.
  - `float quad_sdf_impl(float2 corner_center_to_point, float span, float n)` (was `(…, float corner_radius)`).
  - In `quad_fragment`: locals `float corner_span`, `float corner_n`, set right after `pick_corner_radius`.

- [ ] **Step 1: Write the review-focus tests**

Append to `continuous_corner_tests.rs`:

```rust
#[test]
fn each_corner_takes_its_own_radius() {
    let mut top_left_only = Corners::default();
    top_left_only.top_left = ScaledPixels(20.);
    let ours = gpui_mask(200, 200, |s| {
        s.insert_primitive(white_quad(bounds(0., 0., 200., 200.), top_left_only));
    });
    let apple = ca_mask(200, 200, 20., "continuous");
    let circle = ca_mask(200, 200, 20., "circular");
    // Only the top-left 50 × 50 is rounded; Apple's mask is symmetric, so its top-left is a reference.
    let corner = |m: &[f32]| -> Vec<f32> { (0..50).flat_map(|y| m[y * 200..y * 200 + 50].to_vec()).collect() };
    let (e, base) = (diff(&corner(&ours), &corner(&apple)), diff(&corner(&circle), &corner(&apple)));
    assert!(e < 0.5 * base, "top-left: {e:.2} px² off Apple's curve, the circle is {base:.2}");
    // The other corners are square: every pixel outside the top-left 50 × 50 is covered.
    for y in 0..200 {
        for x in 0..200 {
            if x >= 50 || y >= 50 {
                assert!(ours[y * 200 + x] > 0.99, "({x}, {y}) uncovered: {}", ours[y * 200 + x]);
            }
        }
    }
}

#[test]
fn small_quad_corner_stays_a_circle() {
    let ours = gpui_mask(12, 12, |s| s.insert_primitive(white_quad(bounds(0., 0., 12., 12.), radii(6.))));
    let circle = ca_mask(12, 12, 6., "circular");
    let e = diff(&ours, &circle);
    assert!(e < 1.5, "12 × 12 at r = 6: {e:.2} px² off a circle");
}

#[test]
fn half_pixel_radius_stays_filled() {
    let ours = gpui_mask(40, 40, |s| s.insert_primitive(white_quad(bounds(0., 0., 40., 40.), radii(0.5))));
    assert!(ours.iter().all(|c| c.is_finite()));
    for y in 1..39 {
        for x in 1..39 {
            assert!(ours[y * 40 + x] > 0.99, "({x}, {y}) uncovered: {}", ours[y * 40 + x]);
        }
    }
}

#[test]
fn unblurred_shadow_takes_the_same_curve() {
    let b = bounds(0., 0., 200., 200.);
    let quad = gpui_mask(200, 200, |s| s.insert_primitive(white_quad(b, radii(20.))));
    let shadow = gpui_mask(200, 200, |s| {
        s.insert_primitive(Shadow {
            order: 0,
            blur_radius: ScaledPixels(0.),
            bounds: b,
            corner_radii: radii(20.),
            content_mask: ContentMask { bounds: b },
            color: gray(1.),
            element_bounds: b,
            element_corner_radii: radii(20.),
            inset: 0,
            pad: 0,
        })
    });
    let e = diff(&quad, &shadow);
    assert!(e < 2.0, "shadow and quad corners differ by {e:.2} px²");
}
```

- [ ] **Step 2: Run them to see which fail**

Run: `cargo test -p gpui_macos --lib --features runtime_shaders continuous_corner_tests`

Expected: `continuous_corner_matches_core_animation` and `each_corner_takes_its_own_radius` FAIL (circle). The other three pass today; they guard the change.

- [ ] **Step 3: Add the curve functions**

In `shaders.metal`, replace the declaration

```metal
float quad_sdf_impl(float2 center_to_point, float corner_radius);
```

with

```metal
float2 corner_shape(float corner_radius, float2 half_size);
float corner_curve_sdf(float2 p, float span, float n);
float quad_sdf_impl(float2 corner_center_to_point, float span, float n);
```

Replace the definition of `quad_sdf_impl` (the whole function, from `// Implementation of quad signed distance field` to its closing brace) with:

```metal
// Apple's continuous corner (CALayer cornerCurve = .continuous) for a corner radius: a superellipse
// |x|^n + |y|^n = span^n with n = 2.7 over span = 1.28 r, fitted to Core Animation's renders. Where
// the span would pass the middle of the shorter side, it stops there and n eases back to 2, so pills
// stay circles. Returns (span, n); (0, 2) for a square corner.
float2 corner_shape(float corner_radius, float2 half_size) {
  if (corner_radius <= 0.0) {
    return float2(0.0, 2.0);
  }
  float span = min(corner_radius * 1.28, min(half_size.x, half_size.y));
  float n = mix(2.0, 2.7, saturate((span - corner_radius) / (corner_radius * 0.28)));
  return float2(span, n);
}

// Signed distance from the corner's curve, for p >= 0 measured from the corner's center. The
// superellipse's implicit distance is divided by its gradient's length, so the antialiasing band
// stays a pixel wide where the curve is off the axes. On an axis it is exactly p - span.
float corner_curve_sdf(float2 p, float span, float n) {
  if (n == 2.0) {
    return length(p) - span;
  }
  float2 q = p / span;
  float2 q_n1 = pow(q, float2(n - 1.0));
  float sum = q_n1.x * q.x + q_n1.y * q.y;
  if (sum <= 0.0) {
    return -span;
  }
  float norm = pow(sum, 1.0 / n);
  float gradient = length(q_n1) / pow(norm, n - 1.0);
  return (norm - 1.0) * span / gradient;
}

// Implementation of quad signed distance field
float quad_sdf_impl(float2 corner_center_to_point, float span, float n) {
  if (span == 0.0) {
    // Fast path for unrounded corners
    return max(corner_center_to_point.x, corner_center_to_point.y);
  }
  // Inside the quad inset by the span: 0 outside it, negative inside
  float inside = min(0.0, max(corner_center_to_point.x, corner_center_to_point.y));
  return corner_curve_sdf(max(float2(0.0), corner_center_to_point), span, n) + inside;
}
```

In `quad_sdf`, replace

```metal
    float corner_radius = pick_corner_radius(center_to_point, corner_radii);
    float2 corner_to_point = fabs(center_to_point) - half_size;
    float2 corner_center_to_point = corner_to_point + corner_radius;
    return quad_sdf_impl(corner_center_to_point, corner_radius);
```

with

```metal
    float2 corner = corner_shape(pick_corner_radius(center_to_point, corner_radii), half_size);
    float2 corner_to_point = fabs(center_to_point) - half_size;
    float2 corner_center_to_point = corner_to_point + corner.x;
    return quad_sdf_impl(corner_center_to_point, corner.x, corner.y);
```

- [ ] **Step 4: Use the curve in `quad_fragment`**

Replace

```metal
  // Radius of the nearest corner
  float corner_radius = pick_corner_radius(center_to_point, quad.corner_radii);
```

with

```metal
  // The nearest corner's curve: its span from the corner and its superellipse exponent
  float2 corner = corner_shape(pick_corner_radius(center_to_point, quad.corner_radii), half_size);
  float corner_span = corner.x;
  float corner_n = corner.y;
```

Then, in `quad_fragment` only:
- `float2 corner_center_to_point = corner_to_point + corner_radius;` → `float2 corner_center_to_point = corner_to_point + corner_span;`
- `float outer_sdf = quad_sdf_impl(corner_center_to_point, corner_radius);` → `float outer_sdf = quad_sdf_impl(corner_center_to_point, corner_span, corner_n);`

Every other `corner_radius` left in `quad_fragment` belongs to the borders and dashes: Tasks 3 and 4 replace them. Until then, make it compile by adding, right after `corner_n`:

```metal
  float corner_radius = corner_span; // Tasks 3 and 4 replace its remaining uses
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p gpui_macos --lib --features runtime_shaders`

Expected: every test passes, the 17 existing ones and all of `continuous_corner_tests`. If `continuous_corner_matches_core_animation` fails at one radius, print `e / base` per radius (`--nocapture`); the spike measured 0.41, 0.34 and 0.33 for r = 8, 20 and 40.

- [ ] **Step 6: Commit**

```bash
git add crates/gpui_macos/src/shaders.metal crates/gpui_macos/src/metal_renderer/continuous_corner_tests.rs
git commit -m "Round quads with Apple's continuous corner curve

A corner of radius r is a superellipse with n = 2.7 over 1.28 r, fitted to Core Animation's renders;
where that span passes the middle of the shorter side it eases back to a circle. quad_sdf (shadows,
sprites, blur) and quad_fragment both take the curve from corner_shape.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01AJRVE4ptTQby4VP165isQr"
```

---

### Task 3: Borders follow the curve

**Files:**
- Modify: `crates/gpui_macos/src/shaders.metal` (`quad_fragment`'s inner edge; `quarter_ellipse_sdf` near line 445 and its declaration near line 26)
- Test: `crates/gpui_macos/src/metal_renderer/continuous_corner_tests.rs`

**Interfaces:**
- Consumes: Task 2's `corner_span`, `corner_n` in `quad_fragment`; Task 1's helpers.
- Produces (MSL): `float quarter_superellipse_sdf(float2 point, float2 radii, float n)` replacing `quarter_ellipse_sdf` (same sign: positive inside).
- Produces (test): `fn bordered_quad(b: Bounds<ScaledPixels>, r: f32, widths: Edges<ScaledPixels>, fill: f32) -> Quad`.

- [ ] **Step 1: Write the tests**

Append to `continuous_corner_tests.rs`:

```rust
pub(super) fn bordered_quad(b: Bounds<ScaledPixels>, r: f32, widths: Edges<ScaledPixels>, fill: f32) -> Quad {
    let mut quad = white_quad(b, radii(r));
    quad.background = solid_background(gray(fill));
    quad.border_color = gray(1.);
    quad.border_widths = widths;
    quad
}

#[test]
fn uniform_border_follows_the_curve() {
    let b = bounds(0., 0., 200., 200.);
    let ours = gpui_mask(200, 200, |s| s.insert_primitive(bordered_quad(b, 20., Edges::all(ScaledPixels(1.)), 0.)));
    // The border's area in the 40 × 40 top-left square: the curve (L(2.7) · span) and the straight rest.
    let span = 1.28 * 20.;
    let expected = 1.6582 * span + 2. * (40. - span);
    let area: f32 = (0..40).flat_map(|y| ours[y * 200..y * 200 + 40].to_vec()).sum();
    assert!((area - expected).abs() < 0.1 * expected, "corner border area {area:.1} px², expected {expected:.1}");
}

#[test]
fn thick_border_leaves_no_holes() {
    let b = bounds(0., 0., 60., 60.);
    let filled = gpui_mask(60, 60, |s| s.insert_primitive(white_quad(b, radii(4.))));
    let bordered = gpui_mask(60, 60, |s| s.insert_primitive(bordered_quad(b, 4., Edges::all(ScaledPixels(6.)), 1.)));
    let e = diff(&filled, &bordered);
    assert!(e < 0.5, "a white border on a white fill differs from the fill by {e:.2} px²");
}

#[test]
fn uneven_border_leaves_no_holes() {
    let b = bounds(0., 0., 120., 120.);
    let widths = Edges { top: ScaledPixels(1.), right: ScaledPixels(4.), bottom: ScaledPixels(2.), left: ScaledPixels(8.) };
    let filled = gpui_mask(120, 120, |s| s.insert_primitive(white_quad(b, radii(20.))));
    let bordered = gpui_mask(120, 120, |s| s.insert_primitive(bordered_quad(b, 20., widths, 1.)));
    let e = diff(&filled, &bordered);
    assert!(e < 0.5, "a white uneven border on a white fill differs from the fill by {e:.2} px²");
}
```

- [ ] **Step 2: Run them**

Run: `cargo test -p gpui_macos --lib --features runtime_shaders continuous_corner_tests`

Expected: `uniform_border_follows_the_curve` may already pass (a uniform inner edge is `-(outer_sdf + border)`, the offset of the new curve). `uneven_border_leaves_no_holes` checks the ellipse path that this task replaces. Note which pass and which fail before changing the shader.

- [ ] **Step 3: Replace the quarter ellipse with a quarter superellipse**

Declaration: `float quarter_ellipse_sdf(float2 point, float2 radii);` → `float quarter_superellipse_sdf(float2 point, float2 radii, float n);`

Definition: replace the whole `quarter_ellipse_sdf` function with

```metal
// The inner edge of a corner whose borders differ in width: the corner's superellipse (n = 2 is an
// ellipse) with radii span - border. Positive inside. Radii reach 0 when a border is as wide as the
// corner; they are kept above 0 so the division stays finite.
float quarter_superellipse_sdf(float2 point, float2 radii, float n) {
  float2 q = max(float2(0.0), point) / max(radii, float2(1e-4));
  float norm = pow(pow(q.x, n) + pow(q.y, n), 1.0 / n);
  // Approximate up-scaling of the length by using the average of the radii.
  return (norm - 1.0) * (radii.x + radii.y) * -0.5;
}
```

In `quad_fragment`, replace

```metal
    float2 ellipse_radii = max(float2(0.0), float2(corner_radius) - reduced_border);
    inner_sdf = quarter_ellipse_sdf(corner_center_to_point, ellipse_radii);
```

with

```metal
    float2 inner_radii = max(float2(0.0), float2(corner_span) - reduced_border);
    inner_sdf = quarter_superellipse_sdf(corner_center_to_point, inner_radii, corner_n);
```

- [ ] **Step 4: Run all tests**

Run: `cargo test -p gpui_macos --lib --features runtime_shaders`

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/gpui_macos/src/shaders.metal crates/gpui_macos/src/metal_renderer/continuous_corner_tests.rs
git commit -m "Borders of different widths follow the continuous corner

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01AJRVE4ptTQby4VP165isQr"
```

---

### Task 4: Dashed borders measure the curve

**Files:**
- Modify: `crates/gpui_macos/src/shaders.metal` (`quad_fragment`'s rounded dashed branch, near lines 270–360; a new `corner_arc_length` and its declaration)
- Test: `crates/gpui_macos/src/metal_renderer/continuous_corner_tests.rs`

**Interfaces:**
- Consumes: Task 2's `corner_shape`, `corner_span`, `corner`; Task 3's `bordered_quad`.
- Produces (MSL): `float corner_arc_length(float2 corner)`: the curve's length for a `(span, n)`.

- [ ] **Step 1: Write the test**

Append to `continuous_corner_tests.rs`:

```rust
#[test]
fn dashed_border_keeps_its_dashes_around_the_corner() {
    let b = bounds(0., 0., 200., 200.);
    let solid = gpui_mask(200, 200, |s| s.insert_primitive(bordered_quad(b, 20., Edges::all(ScaledPixels(2.)), 0.)));
    let dashed = gpui_mask(200, 200, |s| {
        let mut quad = bordered_quad(b, 20., Edges::all(ScaledPixels(2.)), 0.);
        quad.border_style = BorderStyle::Dashed;
        s.insert_primitive(quad)
    });
    // Dashes are 2 widths on, 1 off: about 2/3 of the solid border, around the corners too.
    let (all_solid, all_dashed): (f32, f32) = (solid.iter().sum(), dashed.iter().sum());
    let ratio = all_dashed / all_solid;
    assert!((0.55..0.78).contains(&ratio), "dashed / solid = {ratio:.2}");
    let corner = |m: &[f32]| -> f32 { (0..26).flat_map(|y| m[y * 200..y * 200 + 26].to_vec()).sum() };
    let corner_ratio = corner(&dashed) / corner(&solid);
    assert!((0.3..0.95).contains(&corner_ratio), "corner dashed / solid = {corner_ratio:.2}");
    assert!(dashed.iter().all(|c| c.is_finite()));
}
```

- [ ] **Step 2: Run it**

Run: `cargo test -p gpui_macos --lib --features runtime_shaders dashed_border_keeps_its_dashes_around_the_corner -- --nocapture`

Expected: it may pass or fail before the change. The shader still measures corners as circles of `r`, while the shape now spans `1.28 r`. Note the ratios either way. The change below makes the dash layout consistent with the shape.

- [ ] **Step 3: Measure corners by their curve**

Declaration, next to `corner_shape`'s: `float corner_arc_length(float2 corner);`

Definition, after `corner_shape`:

```metal
// The length of a corner's curve for its (span, n): span · π/2 for a circle, span · 1.6582 for n = 2.7.
float corner_arc_length(float2 corner) {
  return corner.x * mix(M_PI_F / 2.0, 1.6582, (corner.y - 2.0) / 0.7);
}
```

In the rounded dashed branch, replace

```metal
        float r_tr = quad.corner_radii.top_right;
        float r_br = quad.corner_radii.bottom_right;
        float r_bl = quad.corner_radii.bottom_left;
        float r_tl = quad.corner_radii.top_left;
```

with

```metal
        float2 corner_tr = corner_shape(quad.corner_radii.top_right, half_size);
        float2 corner_br = corner_shape(quad.corner_radii.bottom_right, half_size);
        float2 corner_bl = corner_shape(quad.corner_radii.bottom_left, half_size);
        float2 corner_tl = corner_shape(quad.corner_radii.top_left, half_size);
        float r_tr = corner_tr.x;
        float r_br = corner_br.x;
        float r_bl = corner_bl.x;
        float r_tl = corner_tl.x;
```

replace

```metal
        float c_tr = r_tr * (M_PI_F / 2.0) * corner_dash_velocity_tr;
        float c_br = r_br * (M_PI_F / 2.0) * corner_dash_velocity_br;
        float c_bl = r_bl * (M_PI_F / 2.0) * corner_dash_velocity_bl;
        float c_tl = r_tl * (M_PI_F / 2.0) * corner_dash_velocity_tl;
```

with

```metal
        float c_tr = corner_arc_length(corner_tr) * corner_dash_velocity_tr;
        float c_br = corner_arc_length(corner_br) * corner_dash_velocity_br;
        float c_bl = corner_arc_length(corner_bl) * corner_dash_velocity_bl;
        float c_tl = corner_arc_length(corner_tl) * corner_dash_velocity_tl;
```

and replace

```metal
          float corner_t = radians * corner_radius;
```

with

```metal
          float corner_t = radians / (M_PI_F / 2.0) * corner_arc_length(corner);
```

- [ ] **Step 4: Remove the stand-in**

Delete the line Task 2 added:

```metal
  float corner_radius = corner_span; // Tasks 3 and 4 replace its remaining uses
```

The shader must compile without it. If a `corner_radius` remains in `quad_fragment`, it is a use these tasks missed: replace it with `corner_span`, or with the per-corner `r_*` inside the dashed branch.

- [ ] **Step 5: Run all tests**

Run: `cargo test -p gpui_macos --lib --features runtime_shaders`

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add crates/gpui_macos/src/shaders.metal crates/gpui_macos/src/metal_renderer/continuous_corner_tests.rs
git commit -m "Dashed borders lay their dashes along the continuous corner

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01AJRVE4ptTQby4VP165isQr"
```

---

### Task 5: Shepherd on the new revision

**Files:**
- Modify: Shepherd's `Cargo.toml` (the `[patch."https://github.com/zeronsh/zui"]` section), `Cargo.lock`

**Interfaces:**
- Consumes: the `continuous-corners` branch, pushed.

- [ ] **Step 1: Push the branch**

```bash
cd ~/Developer/pet/zui
git push -u origin continuous-corners
git rev-parse HEAD
```

- [ ] **Step 2: Point the patch at it**

In `~/Developer/pet/shepherd`, replace the revision in every line of the patch section (19 lines) with the new `HEAD`, and update the comment above it:

```bash
cd ~/Developer/pet/shepherd
NEW=$(git -C ~/Developer/pet/zui rev-parse HEAD)
sed -i '' "s/rev = \"98b1dd892d33770473bbce3d21014f4b5b37b1bc\"/rev = \"$NEW\"/" Cargo.toml
```

Comment above `[patch."https://github.com/zeronsh/zui"]`:

```toml
# freethinkel/zui@continuous-corners: three Metal drawables (with zui's two, a live window resize blocks in
# next_drawable and stutters as it starts) and Apple's continuous corners. Every zui crate is patched, so gpui-base
# and Shepherd share one gpui.
```

- [ ] **Step 3: Build and test**

```bash
cargo build --release 2>&1 | grep -E "^error" -A5
cargo test 2>&1 | grep -E "test result|FAILED"
cargo tree -i gpui_macos --depth 0
```

Expected: no errors; every Shepherd test passes; `gpui_macos` comes from `https://github.com/freethinkel/zui?rev=<NEW>`.

- [ ] **Step 4: Look at it**

`cargo run --release`, then check by eye at 2× and, if there is one, a 1× display: the command panel (⌘K), a popup menu, the sidebar's selected row, a switch in Settings (a pill: it must stay round-ended), an image with rounded corners, and the agents list's hover row. Corners must be smooth, with no dark fringe at a border and no notch where the curve meets a straight edge.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: zui with Apple's continuous corners: every rounded corner on macOS takes CALayer's continuous curve

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01AJRVE4ptTQby4VP165isQr"
```
