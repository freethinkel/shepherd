pub mod command_panel;
pub mod divider;
pub mod edge_fade;
pub mod loader;
pub mod project_icon;
pub mod review;
pub mod settings;
pub mod statusbar;
pub mod toast;

use crate::theme::Theme;
use gpui::{ElementId, px};
use gpui_base::input::InputEditorStyle;
use gpui_base::{Scrollbar, ScrollbarHandle, ScrollbarThumbStyle};

/// gpui-base inputs are unstyled: colors from the theme, as Collie does.
pub fn input_style(theme: &Theme) -> InputEditorStyle {
    InputEditorStyle {
        foreground: theme.text,
        muted_foreground: theme.text_faint,
        border: theme.border,
        selection: theme.accent.opacity(0.3),
        caret: theme.text,
        ..Default::default()
    }
}

/// The app's one scrollbar (terminals, the agents list): gpui-base's overlay bar, shown while scrolling and faded
/// out when idle as macOS does, with a 6 px round thumb 2 px off the edge in every state.
pub fn scrollbar<H: ScrollbarHandle + Clone>(id: impl Into<ElementId>, handle: &H, theme: &Theme) -> Scrollbar {
    let color = theme.text_muted;
    let thumb = move |t: ScrollbarThumbStyle, alpha: f32| {
        t.bg(color.opacity(alpha)).width(px(6.)).inset(px(2.)).radius(px(3.)).min_length(px(24.))
    };
    Scrollbar::vertical(handle).id(id).styles(|s| {
        s.track(|t| t.width(px(12.)))
            .track_hover(|t| t.width(px(12.)))
            .track_active(|t| t.width(px(12.)))
            .thumb(|t| thumb(t, 0.35))
            .thumb_hover(|t| thumb(t, 0.35))
            .thumb_active(|t| thumb(t, 0.6))
    })
}

const SWITCH_WIDTH: f32 = 44.8;
const SWITCH_HEIGHT: f32 = 28.8;
const SWITCH_TRACK_HEIGHT: f32 = 20.8;
const SWITCH_INSET: f32 = 1.6;
const SWITCH_THUMB_WIDTH: f32 = 24.0;
const SWITCH_THUMB_HEIGHT: f32 = SWITCH_TRACK_HEIGHT - 2.0 * SWITCH_INSET;
const SWITCH_MARK: f32 = 7.2;

/// Zeron's pill switch (`settings::widgets::toggle_switch`, frost tones left out): the on/off marks sit beneath a
/// thumb that slides over 180 ms. The caller owns the click; `key` keeps the slide across frames.
pub fn switch(theme: &Theme, on: bool, key: impl Into<gpui::SharedString>) -> SwitchVisual {
    SwitchVisual { theme: theme.clone(), on, key: key.into() }
}

#[derive(gpui::IntoElement)]
pub struct SwitchVisual {
    theme: Theme,
    on: bool,
    key: gpui::SharedString,
}

struct SwitchTravel {
    from: f32,
    target: f32,
    started: std::time::Instant,
}

impl SwitchTravel {
    fn value(&self, now: std::time::Instant) -> f32 {
        let t = (now.duration_since(self.started).as_secs_f32() / 0.18).min(1.0);
        self.from + (self.target - self.from) * (1.0 - (1.0 - t).powi(3))
    }
}

impl gpui::RenderOnce for SwitchVisual {
    fn render(self, window: &mut gpui::Window, _: &mut gpui::App) -> impl gpui::IntoElement {
        use gpui::{ParentElement, Styled, div};
        let now = std::time::Instant::now();
        let target = if self.on { 1.0 } else { 0.0 };
        let id = gpui::ElementId::Name(format!("switch-{}", self.key).into());
        let position = window.with_global_id(id, |id, window| {
            window.with_element_state(id, |previous: Option<SwitchTravel>, _| {
                let mut travel = previous.unwrap_or(SwitchTravel { from: target, target, started: now });
                if travel.target != target {
                    travel = SwitchTravel { from: travel.value(now), target, started: now };
                }
                (travel.value(now), travel)
            })
        });
        if (position - target).abs() > 0.001 {
            window.request_animation_frame();
        }
        let theme = &self.theme;
        let dark = theme.appearance == crate::theme::Appearance::Dark;
        let track = if self.on { theme.accent } else { theme.text.opacity(if dark { 0.18 } else { 0.10 }) };
        let mark_padding = (SWITCH_WIDTH - SWITCH_THUMB_WIDTH - SWITCH_INSET - SWITCH_MARK) / 2.0;
        let thumb_left = SWITCH_INSET + (SWITCH_WIDTH - SWITCH_THUMB_WIDTH - 2.0 * SWITCH_INSET) * position;
        let mark = |opacity: f32, glyph: gpui::Div| {
            div().size(px(SWITCH_MARK)).flex().items_center().justify_center().opacity(opacity).child(glyph)
        };
        div()
            .flex_none()
            .relative()
            .w(px(SWITCH_WIDTH))
            .h(px(SWITCH_HEIGHT))
            .child(
                div()
                    .absolute()
                    .top(px((SWITCH_HEIGHT - SWITCH_TRACK_HEIGHT) / 2.0))
                    .left_0()
                    .w(px(SWITCH_WIDTH))
                    .h(px(SWITCH_TRACK_HEIGHT))
                    .rounded_full()
                    .bg(track)
                    .border_1()
                    .border_color(if self.on { gpui::white().opacity(0.12) } else { theme.border })
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .px(px(mark_padding))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(mark(position, div().w(px(1.2)).h(px(7.2)).rounded_full().bg(gpui::white().opacity(0.96))))
                            .child(mark(
                                1.0 - position,
                                div().size(px(6.4)).rounded_full().border(px(1.0)).border_color(theme.text_muted),
                            )),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top(px((SWITCH_HEIGHT - SWITCH_THUMB_HEIGHT) / 2.0))
                    .left(px(thumb_left))
                    .w(px(SWITCH_THUMB_WIDTH))
                    .h(px(SWITCH_THUMB_HEIGHT))
                    .rounded_full()
                    .bg(gpui::white().opacity(if dark { 0.96 } else { 1.0 }))
                    .border_1()
                    .border_color(gpui::black().opacity(if dark { 0.10 } else { 0.08 })),
            )
    }
}

/// A popup that stays mounted after it closes while Zeron's exit (`motion::menu_out`) plays: it fades out and
/// lifts 2 px over 100 ms, deaf to the pointer.
pub struct Popup<T> {
    pub value: T,
    /// Keys the entrance, so a popup reopened during the previous one's exit plays it again.
    serial: usize,
    closing: Option<std::time::Instant>,
}

const MENU_OUT: std::time::Duration = std::time::Duration::from_millis(100);

impl<T> Popup<T> {
    pub fn new(value: T) -> Self {
        static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        Popup { value, serial: SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed), closing: None }
    }

    pub fn is_open(&self) -> bool {
        self.closing.is_none()
    }

    pub fn close(&mut self) {
        self.closing.get_or_insert_with(std::time::Instant::now);
    }

    /// How long its exit has played, while it plays.
    pub fn closed_for(&self) -> Option<std::time::Duration> {
        self.closing.map(|c| c.elapsed())
    }
}

/// Call once per frame: drops a popup whose exit has played, else asks for the next frame while it plays.
pub fn reap_popup<T>(popup: &mut Option<Popup<T>>, window: &mut gpui::Window) {
    if let Some(closing) = popup.as_ref().and_then(|p| p.closing) {
        if closing.elapsed() >= MENU_OUT {
            *popup = None;
        } else {
            window.request_animation_frame();
        }
    }
}

/// Zeron's popover motion: the entrance (`motion::menu_in`) fades up from 30 % and drops 2 px into place over
/// 140 ms; the exit runs it back out to nothing.
pub fn menu_motion<T>(name: impl Into<gpui::SharedString>, popup: &Popup<T>, menu: gpui::Stateful<gpui::Div>) -> gpui::AnyElement {
    popup_motion(name, popup, menu, -2.0, 0.3, 140)
}

/// Zeron's dialog entrance (`motion::dialog_in`): fades in from nothing over 180 ms, `rise` px below its place.
/// A backdrop passes 0 and only fades.
pub fn dialog_motion<T>(name: impl Into<gpui::SharedString>, popup: &Popup<T>, el: gpui::Stateful<gpui::Div>, rise: f32) -> gpui::AnyElement {
    popup_motion(name, popup, el, rise, 0.0, 180)
}

fn popup_motion<T>(
    name: impl Into<gpui::SharedString>,
    popup: &Popup<T>,
    el: gpui::Stateful<gpui::Div>,
    offset: f32,
    from_opacity: f32,
    ms: u64,
) -> gpui::AnyElement {
    use gpui::{AnimationExt, InteractiveElement, IntoElement, ParentElement, Styled};
    match popup.closing {
        Some(closing) => {
            let t = gpui::ease_in_out((closing.elapsed().as_secs_f32() / MENU_OUT.as_secs_f32()).min(1.0));
            el.relative()
                .opacity(1.0 - t)
                .top(px(offset * t))
                .child(gpui::div().absolute().inset_0().occlude())
                .into_any_element()
        }
        None => {
            let animation = gpui::Animation::new(std::time::Duration::from_millis(ms)).with_easing(gpui::ease_out_quint());
            el.with_animation(ElementId::NamedInteger(name.into(), popup.serial as u64), animation, move |el, t| {
                el.relative().opacity(from_opacity + (1.0 - from_opacity) * t).top(px(offset * (1.0 - t)))
            })
            .into_any_element()
        }
    }
}
