//! Zeron's `surface_chrome`: the right panel's toolbars.
use crate::theme::Theme;
use gpui::{Div, div, prelude::*, px};

pub const HEADER_HEIGHT: f32 = 38.;
pub const CONTROL_SIZE: f32 = 24.;
pub const CONTROL_RADIUS: f32 = 6.;
pub const ICON_SIZE: f32 = 14.;
pub const CONTROL_GAP: f32 = 4.;
pub const EDGE_INSET: f32 = 8.;

/// A row of a surface's controls, under the strip.
pub fn toolbar(theme: &Theme) -> Div {
    div()
        .h(px(HEADER_HEIGHT))
        .w_full()
        .flex_none()
        .px(px(EDGE_INSET))
        .flex()
        .items_center()
        .gap(px(CONTROL_GAP))
        .border_b_1()
        .border_color(theme.border)
        .bg(theme.surface)
}
