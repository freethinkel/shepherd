//! Toasts: herdr's errors and Shepherd's own notes, stacked in the window's top-right corner. Each slides in from
//! the right, stays a few seconds (a click closes it sooner) and slides back out.
use super::Popup;
use crate::theme::Theme;
use gpui::{AnimationExt, App, ElementId, Global, InteractiveElement, IntoElement, ParentElement, SharedString, StatefulInteractiveElement, Styled, Window, div, px};
use std::rc::Rc;
use std::time::Duration;

const STAY: Duration = Duration::from_secs(5);
const IN: Duration = Duration::from_millis(240);
const OUT: Duration = Duration::from_millis(180);
/// How far right of its place a toast starts and ends.
const SLIDE: f32 = 24.0;

struct Toast {
    message: SharedString,
    error: bool,
    /// Runs on a click, before the toast closes.
    action: Option<Rc<dyn Fn(&mut App)>>,
    /// Draws a card of the caller's in place of the message.
    content: Option<Content>,
    popup: Popup<()>,
}

/// A card of its own, given the theme and what closes it.
pub type Content = Rc<dyn Fn(&Theme, Rc<dyn Fn(&mut App)>) -> gpui::AnyElement>;

#[derive(Default)]
struct Toasts(Vec<Toast>);

impl Global for Toasts {}

pub fn error(cx: &mut App, message: impl Into<SharedString>) {
    push(cx, message.into(), true, None, None);
}

/// A note; `action`, when given, runs on a click.
pub fn info(cx: &mut App, message: impl Into<SharedString>, action: Option<Rc<dyn Fn(&mut App)>>) {
    push(cx, message.into(), false, action, None);
}

/// A card drawn by `content`, in the stack until it closes itself.
pub fn sticky(cx: &mut App, key: impl Into<SharedString>, content: Content) {
    push(cx, key.into(), false, None, Some(content));
}

fn push(cx: &mut App, message: SharedString, error: bool, action: Option<Rc<dyn Fn(&mut App)>>, content: Option<Content>) {
    let toasts = &mut cx.default_global::<Toasts>().0;
    // the same message again replaces the open one, so a repeated failure does not pile up
    toasts.retain(|t| t.message != message || !t.popup.is_open());
    let popup = Popup::new(());
    let serial = popup.serial;
    let stays = content.is_some();
    toasts.push(Toast { message, error, action, content, popup });
    cx.refresh_windows();
    if stays {
        return;
    }
    cx.spawn(async move |cx| {
        cx.background_executor().timer(STAY).await;
        cx.update(|cx| close(cx, serial));
    })
    .detach();
}

/// Whether a toast is on screen, sliding out included.
pub fn any_open(cx: &App) -> bool {
    cx.try_global::<Toasts>().is_some_and(|t| !t.0.is_empty())
}

fn close(cx: &mut App, serial: usize) {
    if let Some(t) = cx.default_global::<Toasts>().0.iter_mut().find(|t| t.popup.serial == serial) {
        t.popup.close();
        cx.refresh_windows();
    }
}

/// The stack, for the window's root. Call once per frame: it drops toasts whose exit has played.
pub fn stack(theme: &Theme, top: f32, window: &mut Window, cx: &mut App) -> gpui::AnyElement {
    let toasts = &mut cx.default_global::<Toasts>().0;
    toasts.retain(|t| t.popup.closing.is_none_or(|c| c.elapsed() < OUT));
    if toasts.iter().any(|t| !t.popup.is_open()) {
        window.request_animation_frame();
    }
    let cards: Vec<_> = toasts.iter().map(|t| card(theme, t)).collect();
    gpui::deferred(
        div().absolute().top(px(top)).right(px(12.)).w(px(340.)).flex().flex_col().items_end().gap(px(8.)).children(cards),
    )
    .into_any_element()
}

fn card(theme: &Theme, toast: &Toast) -> gpui::AnyElement {
    let serial = toast.popup.serial;
    let action = toast.action.clone();
    let (tone, bg) = if toast.error { (theme.danger, theme.card.blend(theme.danger.opacity(0.14))) } else { (theme.text, theme.card) };
    let el = div().id(("toast", serial)).occlude().relative().max_w_full();
    let el = match &toast.content {
        Some(content) => el.child(content(theme, Rc::new(move |cx| close(cx, serial)))),
        None => el
            .px_3()
            .py_2()
            .rounded(px(10.))
            .bg(bg)
            .shadow_lg()
            .text_size(px(12.))
            .text_color(tone)
            .cursor_pointer()
            .child(toast.message.clone())
            .on_click(move |_, _, cx| {
                if let Some(action) = &action {
                    action(cx);
                }
                close(cx, serial);
            }),
    };
    match toast.popup.closing {
        Some(closing) => {
            let t = gpui::ease_in_out((closing.elapsed().as_secs_f32() / OUT.as_secs_f32()).min(1.0));
            el.opacity(1.0 - t).left(px(SLIDE * t)).into_any_element()
        }
        None => el
            .with_animation(ElementId::NamedInteger("toast-in".into(), serial as u64), gpui::Animation::new(IN).with_easing(gpui::ease_out_quint()), |el, t| {
                el.opacity(t).left(px(SLIDE * (1.0 - t)))
            })
            .into_any_element(),
    }
}
