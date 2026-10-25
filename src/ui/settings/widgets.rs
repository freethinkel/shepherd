//! The pieces settings pages are built of: headers, cards, rows, toggles, choices and fields.
use super::*;

/// Page headline and its muted subtitle.
pub(super) fn page_header(theme: &Theme, title: &'static str, subtitle: &'static str) -> Div {
    div()
        .px(px(8.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(div().text_size(px(20.)).line_height(px(26.)).font_weight(FontWeight::MEDIUM).child(title))
        .child(div().text_color(theme.text_muted).line_height(px(17.)).child(subtitle))
}

/// The small plain label above a card.
pub(super) fn section_label(theme: &Theme, label: &'static str) -> Div {
    div().mt(px(32.)).mb(px(8.)).px(px(8.)).text_color(theme.text_muted).child(label)
}

/// A filled, rounded block of related rows: a faint wash, no outline.
pub(super) fn card(theme: &Theme) -> Div {
    div().mt(px(24.)).rounded(px(12.)).bg(theme.wash(0.045)).overflow_hidden().flex().flex_col()
}

/// One row of a card: text on the left, the control on the right, a hairline above all but the first.
pub(super) fn card_row(theme: &Theme, first: bool) -> Div {
    div()
        .mx(px(16.))
        .py(px(12.))
        .min_h(px(60.))
        .when(!first, |el| el.border_t_1().border_color(theme.border.opacity(0.6)))
        .flex()
        .items_center()
        .gap(px(16.))
}

/// One way to draw agent status: the waiting, working, done and idle icons in it, its name, and a check when chosen.
pub(super) fn status_choice(theme: &Theme, style: StatusIcons, label: &'static str, checked: bool) -> gpui::Stateful<Div> {
    use crate::model::AgentStatus::*;
    let hover = theme.wash(0.06);
    div()
        .id(label)
        .h(px(36.))
        .px(px(8.))
        .flex()
        .items_center()
        .gap(px(12.))
        .rounded(px(8.))
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(
            div()
                .w(px(72.))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(6.))
                .children([Blocked, Working, Done, Idle].map(|status| crate::shell::status_icon(status, style, theme))),
        )
        .child(div().flex_1().child(label))
        .when(checked, |el| el.child(gpui::svg().path("icons/check.svg").size(px(14.)).text_color(theme.accent)))
}

/// A row's title and, under it, its description.
pub(super) fn row_text(theme: &Theme, title: impl Into<SharedString>, description: impl Into<SharedString>) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(div().font_weight(FontWeight::MEDIUM).line_height(px(17.)).child(title.into()))
        .child(div().text_size(px(12.)).line_height(px(16.)).text_color(theme.text_muted).child(description.into()))
}

/// Zeron's filled action: a borderless pill that lifts on hover.
pub(super) fn action(theme: &Theme, id: impl Into<gpui::ElementId>, label: impl Into<SharedString>) -> Stateful<Div> {
    let lifted = theme.wash(0.10);
    div()
        .id(id)
        .flex_none()
        .min_h(px(32.))
        .px(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(8.))
        .bg(theme.wash(0.06))
        .text_size(px(12.5))
        .cursor_pointer()
        .hover(move |s| s.bg(lifted))
        .child(label.into())
}

/// A row the shared switch (`crate::ui::switch`) ends; a click anywhere on it flips the switch.
pub(super) fn toggle_row(
    theme: &Theme,
    first: bool,
    id: impl Into<SharedString>,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    on: bool,
) -> Stateful<Div> {
    let id = id.into();
    card_row(theme, first)
        .id(id.clone())
        .cursor_pointer()
        .child(row_text(theme, title, description))
        .child(div().flex_none().child(crate::ui::switch(theme, on, id)))
}

/// A card of the Color scheme row: a preview of the look, its icon and name under it (Zeron's option card).
pub(super) fn option_card(theme: &Theme, icon: &'static str, label: &'static str, selected: bool, preview: impl IntoElement) -> Div {
    let tone = if selected { theme.accent } else { theme.text_muted };
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(8.))
        .cursor_pointer()
        .child(
            div()
                .h(px(112.))
                .w_full()
                .rounded(px(6.))
                .overflow_hidden()
                .border_1()
                .border_color(if selected { theme.accent } else { theme.border })
                .child(preview),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_color(tone)
                .when(selected, |el| el.font_weight(FontWeight::MEDIUM))
                .child(gpui::svg().path(icon).size(px(16.)).text_color(tone))
                .child(label),
        )
}

/// A sketch of the window in `theme`: a sidebar and a terminal pane, text as bars.
pub(super) fn miniature(theme: &Theme) -> Div {
    let (line, strong) = (theme.text.opacity(0.22), theme.text.opacity(0.34));
    let bar = |fraction: f32, tone| div().h(px(5.)).w(gpui::relative(fraction)).rounded(px(3.)).bg(tone);
    div()
        .size_full()
        .flex()
        .bg(theme.surface)
        .child(div().w(px(40.)).flex_none().flex().flex_col().gap(px(7.)).px(px(8.)).pt(px(14.)).children([
            bar(0.7, strong),
            bar(1.0, line),
            bar(0.85, line),
        ]))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .my(px(8.))
                .mr(px(8.))
                .rounded(px(6.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.terminal.background)
                .flex()
                .flex_col()
                .gap(px(7.))
                .p(px(10.))
                .children([bar(0.62, strong), bar(0.88, line), bar(0.76, line), bar(0.52, line)]),
        )
}

/// A theme's three colors in a small chip: window, terminal, accent.
pub(super) fn palette_preview(theme: &Theme) -> Div {
    div()
        .flex_none()
        .w(px(30.))
        .h(px(18.))
        .rounded(px(5.))
        .overflow_hidden()
        .border_1()
        .border_color(theme.border)
        .flex()
        .child(div().w_1_3().h_full().bg(theme.surface))
        .child(div().w_1_3().h_full().bg(theme.terminal.background))
        .child(div().w_1_3().h_full().bg(theme.accent))
}

/// One option of a row of choices: outlined, filled when selected.
pub(super) fn choice(theme: &Theme, id: impl Into<gpui::ElementId>, label: &'static str, selected: bool) -> Stateful<Div> {
    let (selected_bg, hover) = (theme.wash(0.10), theme.hover);
    div()
        .id(id)
        .h(px(36.))
        .px(px(14.))
        .flex()
        .items_center()
        .rounded(px(8.))
        .border_1()
        .cursor_pointer()
        .map(|el| match selected {
            true => el.bg(selected_bg).border_color(theme.border_strong).font_weight(FontWeight::MEDIUM),
            false => el.border_color(theme.border).text_color(theme.text_muted).hover(move |s| s.bg(hover)),
        })
        .child(label)
}

/// A muted line under a card: where a setting is saved, or what the last change did.
pub(super) fn note(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div().mt(px(8.)).px(px(8.)).text_size(px(12.)).line_height(px(16.)).text_color(theme.text_muted).child(text.into())
}

/// A text field in a row: the filled pill of Zeron's selects.
/// A short field for a number, between its − and + buttons.
/// The number fields, font size and line height: a setting's value, how it is set (clamped), and one step.
pub(super) const NUMBERS: [(fn(&Settings) -> f32, fn(&mut Settings, f32), f32); 2] = [
    (|s| s.font_size, |s, v| s.font_size = step_font_size(v, 0.0), 1.0),
    (|s| s.line_height, |s, v| s.line_height = step_line_height(v, 0.0), 0.1),
];

/// The font size and line height as their fields show them.
pub(super) fn numbers(s: &Settings) -> [String; 2] {
    [s.font_size.to_string(), format!("{:.1}", s.line_height)]
}

/// Sets number field `i`'s setting to what the field shows, typed or not, moved `steps` steps; then the field shows
/// the setting, clamped. A field that doesn't parse moves from the setting.
pub(super) fn set_number(i: usize, input: &Entity<InputState>, steps: f32, window: &mut Window, cx: &mut App) {
    let (get, set, step) = NUMBERS[i];
    let typed = input.read(cx).value().trim().replace(',', ".").parse::<f32>().ok();
    if typed.is_some() || steps != 0.0 {
        update(cx, |s| set(s, typed.unwrap_or(get(s)) + steps * step));
    }
    let shown = numbers(&SettingsGlobal::of(cx).settings)[i].clone();
    input.update(cx, |input, cx| input.set_value(shown, window, cx));
}

/// Number field `i`, with stepper chevrons at its right as `<input type="number">` has.
pub(super) fn number_field(theme: &Theme, i: usize, input: &Entity<InputState>) -> Div {
    let hover = theme.wash(0.10);
    let chevron = |dir: f32, icon: &'static str| {
        div()
            .id(("number-step", i * 2 + (dir > 0.) as usize))
            .w(px(16.))
            .h(px(12.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(3.))
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .child(gpui::svg().path(icon).size(px(10.)).text_color(theme.text_muted))
            .on_click({
                let input = input.clone();
                move |_, window, cx| set_number(i, &input, dir, window, cx)
            })
    };
    div()
        .w(px(72.))
        .flex_none()
        .h(px(32.))
        .pl(px(10.))
        .pr(px(4.))
        .flex()
        .items_center()
        .gap(px(2.))
        .rounded(px(8.))
        .bg(theme.wash(0.06))
        .child(div().flex_1().min_w_0().child(Input::new(input)))
        .child(div().flex_none().flex().flex_col().child(chevron(1., "icons/alt-arrow-up.svg")).child(chevron(-1., "icons/alt-arrow-down.svg")))
}

pub(super) fn field(theme: &Theme, input: &Entity<InputState>) -> Div {
    div().w(px(240.)).flex_none().h(px(32.)).px(px(10.)).flex().items_center().rounded(px(8.)).bg(theme.wash(0.06)).child(Input::new(input))
}
