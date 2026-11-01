//! Zeron's `surface_chrome`: the metrics and chips of the right panel's strip and toolbars.
use crate::theme::Theme;
use gpui::{Div, div, prelude::*, px};

pub const HEADER_HEIGHT: f32 = 38.;
pub const CONTROL_SIZE: f32 = 24.;
pub const CONTROL_RADIUS: f32 = 6.;
pub const CONTROL_GAP: f32 = 4.;
pub const EDGE_INSET: f32 = 8.;

/// Fixed, as drag reordering (`drop_index`, `slide_offset`) assumes uniform chips.
pub const TAB_CHIP_WIDTH: f32 = 112.;
pub const TAB_CHIP_HEIGHT: f32 = 24.;
/// A chip plus the strip's own gap.
pub const TAB_CHIP_SLOT: f32 = TAB_CHIP_WIDTH + CONTROL_GAP;
/// Width of the fade over tabs scrolled out of the strip.
pub const TAB_STRIP_FADE: f32 = 36.;

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

/// A neutral navigation chip.
pub fn tab(id: impl Into<gpui::ElementId>, selected: bool, theme: &Theme) -> gpui::Stateful<Div> {
    let (wash, hover) = (theme.wash(0.10), theme.wash(0.06));
    div()
        .id(id)
        .h(px(CONTROL_SIZE))
        .flex_none()
        .rounded(px(CONTROL_RADIUS))
        .flex()
        .items_center()
        .gap(px(CONTROL_GAP))
        .cursor_pointer()
        .text_size(px(12.))
        .text_color(if selected { theme.text } else { theme.text_muted })
        .when(selected, move |el| el.bg(wash))
        .when(!selected, move |el| el.hover(move |s| s.bg(hover)))
}

/// One tab chip, as drawn in the titlebar band above the panel.
pub struct TabChip {
    /// Element id prefix.
    pub id: &'static str,
    /// Element id prefix of the close slot.
    pub close_id: &'static str,
    pub ix: usize,
    pub active: bool,
    pub title: gpui::SharedString,
    /// The leading 18 px slot's content, usually [`tab_chip_icon`].
    pub leading: gpui::AnyElement,
}

/// The leading icon, the truncated title, and a trailing ✕ that fades in on hover. Callers add activation, menus and
/// drag behaviour.
pub fn tab_chip(chip: TabChip, theme: &Theme, on_close: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static) -> gpui::Stateful<Div> {
    let TabChip { id, close_id, ix, active, title, leading } = chip;
    let group: gpui::SharedString = format!("{id}-{ix}").into();
    let close_hover = theme.wash(0.12);
    tab((id, ix), active, theme)
        .group(group.clone())
        .h(px(TAB_CHIP_HEIGHT))
        .w(px(TAB_CHIP_WIDTH))
        .px(px(4.))
        .gap(px(3.))
        // Not `.occlude()`: a blocking hitbox ends the hit test, and the strip behind the chips would never scroll.
        .block_mouse_except_scroll()
        .on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.prevent_default())
        .child(div().flex_none().size(px(18.)).flex().items_center().justify_center().child(leading))
        .child(div().flex_1().min_w_0().truncate().text_size(px(11.5)).text_color(if active { theme.text } else { theme.text_muted }).child(title))
        .child(
            div()
                .id((close_id, ix))
                .flex_none()
                .size(px(18.))
                .rounded(px(4.))
                .relative()
                .tooltip(crate::shell::tip("Close tab", None))
                .hover(move |s| s.bg(close_hover))
                // The tab owns a drag payload: the press is claimed here, or GPUI starts a drag instead of a click.
                .on_mouse_down(gpui::MouseButton::Left, |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_click(move |event, window, cx| {
                    cx.stop_propagation();
                    on_close(event, window, cx);
                })
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .opacity(0.)
                        .group_hover(group, |s| s.opacity(1.))
                        .child(gpui::svg().path("icons/close.svg").size(px(12.)).text_color(theme.text_muted)),
                ),
        )
}

/// A chip's leading glyph, quieter on inactive tabs.
pub fn tab_chip_icon(path: &'static str, active: bool, theme: &Theme) -> gpui::AnyElement {
    gpui::svg().path(path).size(px(12.)).text_color(if active { theme.text_muted } else { theme.text_muted.opacity(0.7) }).into_any_element()
}
