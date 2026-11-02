//! The ask for a GitHub star, T3 Code's card: once, after agents opened in five panes have run to done, and it stays
//! until answered.
use super::*;
use std::collections::HashSet;

const AFTER: u32 = 5;
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// The panes counted since launch, across windows: each window hears of the same agents.
#[derive(Default)]
struct Counted(HashSet<String>);

impl gpui::Global for Counted {}

/// The count once `pane`'s agent is done, if that moves it: a pane counts once a launch, and the count stops at the ask.
fn counted(done: u32, pane: &str, seen: &mut HashSet<String>) -> Option<u32> {
    (done < AFTER && seen.insert(pane.to_string())).then_some(done + 1)
}

// ponytail: by pane id, and ids compact, so a pane can count twice and the ask come a run early; harmless
pub(super) fn agent_done(pane: &str, cx: &mut App) {
    let done = SettingsGlobal::of(cx).settings.agents_done;
    let Some(done) = counted(done, pane, &mut cx.default_global::<Counted>().0) else { return };
    save_from_sidebar(cx, |s| s.agents_done = done);
    if done == AFTER {
        crate::ui::toast::sticky(cx, "star-prompt", Rc::new(card));
    }
}

fn card(theme: &Theme, close: Rc<dyn Fn(&mut App)>) -> gpui::AnyElement {
    let closes = |close: &Rc<dyn Fn(&mut App)>| {
        let close = close.clone();
        move |_: &ClickEvent, _: &mut Window, cx: &mut App| close(cx)
    };
    let star = {
        let close = close.clone();
        move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            cx.open_url(REPOSITORY);
            close(cx);
        }
    };
    let hover = theme.hover;
    let cross = div()
        .id("star-prompt-close")
        .occlude()
        .absolute()
        .top(px(-8.))
        .right(px(-8.))
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(theme.card)
        .border_1()
        .border_color(theme.border)
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(gpui::svg().path("icons/close.svg").size(px(12.)).text_color(theme.text_muted))
        .on_click(closes(&close));
    let title = div()
        .flex()
        .items_center()
        .gap(px(8.))
        .child(gpui::svg().path("icons/solar/star.svg").size(px(18.)).flex_none().text_color(theme.warning))
        .child(div().text_size(px(14.)).font_weight(gpui::FontWeight::MEDIUM).child("Enjoying Shepherd?"));
    let buttons = div()
        .mt(px(6.))
        .flex()
        .justify_end()
        .gap(px(4.))
        .child(ghost_button(theme, "star-prompt-dismiss", "Dismiss").text_color(theme.text).on_click(closes(&close)))
        .child(solid_button("star-prompt-star", "Star on GitHub", theme.accent, gpui::white()).on_click(star));
    div()
        .relative()
        .w(px(340.))
        .p(px(14.))
        .flex()
        .flex_col()
        .gap(px(6.))
        .rounded(px(10.))
        .bg(theme.card)
        .border_1()
        .border_color(theme.border)
        .shadow_lg()
        .text_size(px(13.))
        .text_color(theme.text)
        .child(title)
        .child(div().text_color(theme.text_muted).child("Give us a star on GitHub to let us know!"))
        .child(buttons)
        .child(cross)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_panes_reach_the_ask_and_it_comes_once() {
        let mut seen = HashSet::new();
        let mut done = 0;
        for (pane, moved) in [("1", true), ("1", false), ("2", true), ("3", true), ("4", true), ("5", true), ("6", false)] {
            let next = counted(done, pane, &mut seen);
            assert_eq!(next.is_some(), moved, "{pane}");
            done = next.unwrap_or(done);
        }
        assert_eq!(done, AFTER);
    }
}
