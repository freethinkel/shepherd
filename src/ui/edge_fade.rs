//! Zeron's scroll-edge fade (`crates/ui/src/edge_fade.rs`): a scrolled list's rows fade out toward the edge they
//! scroll past, painted through gpui's `EdgeFade` scope, so nothing is overlaid on the
//! window's background.
use gpui::{AnyElement, App, Bounds, EdgeFade, Element, GlobalElementId, InspectorElementId, IntoElement, LayoutId, Pixels, ScrollHandle, Window, px};

/// Fades `child` within `band` px of its top and bottom edges, each only while `scroll` hides rows past it. The
/// handle is read at paint time, after this frame's scroll offset is clamped.
pub fn edge_faded(band: f32, scroll: &ScrollHandle, child: impl IntoElement) -> EdgeFaded {
    EdgeFaded { band, scroll: scroll.clone(), across: false, child: child.into_any_element() }
}

/// [`edge_faded`] for a strip that scrolls sideways: its left and right edges.
pub fn edge_faded_x(band: f32, scroll: &ScrollHandle, child: impl IntoElement) -> EdgeFaded {
    EdgeFaded { band, scroll: scroll.clone(), across: true, child: child.into_any_element() }
}

pub struct EdgeFaded {
    band: f32,
    scroll: ScrollHandle,
    across: bool,
    child: AnyElement,
}

impl IntoElement for EdgeFaded {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for EdgeFaded {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, window: &mut Window, cx: &mut App) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, _: Bounds<Pixels>, _: &mut (), window: &mut Window, cx: &mut App) {
        self.child.prepaint(window, cx);
    }

    fn paint(&mut self, _: Option<&GlobalElementId>, _: Option<&InspectorElementId>, bounds: Bounds<Pixels>, _: &mut (), _: &mut (), window: &mut Window, cx: &mut App) {
        let (offset, max) = if self.across {
            (self.scroll.offset().x, self.scroll.max_offset().x)
        } else {
            (self.scroll.offset().y, self.scroll.max_offset().y)
        };
        let scrolled = -f32::from(offset);
        let (start, end) = (scrolled > 1., scrolled < f32::from(max) - 1.);
        let fade = (start || end).then(|| EdgeFade {
            bounds,
            band: px(self.band),
            band_top: None,
            band_bottom: None,
            top: start && !self.across,
            bottom: end && !self.across,
            left: start && self.across,
            right: end && self.across,
        });
        window.with_edge_fade(fade, |window| self.child.paint(window, cx));
    }
}
