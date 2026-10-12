//! The window's small building blocks: status dots, menus, dialogs, buttons and sidebar rows.
use super::*;

pub(crate) fn status_color(status: AgentStatus, theme: &Theme) -> Option<Hsla> {
    match status {
        AgentStatus::Working => Some(theme.accent),
        AgentStatus::Blocked => Some(theme.warning),
        AgentStatus::Done => Some(theme.success.opacity(0.9)),
        AgentStatus::Idle | AgentStatus::Unknown => None,
    }
}

/// An agent's status in the theme's style (the settings' Status icons).
pub(crate) fn status_dot(status: AgentStatus, theme: &Theme) -> Div {
    status_icon(status, theme.status_symbols, theme)
}

/// Collie's status dot, filled for an agent state, a faint ring when idle. With `symbols`, herdr's
/// `status_indicators = "symbols"` shapes instead (× ◐ ✓ ○, a dashed ring for its `·`), in the same colors. Every
/// icon is drawn to the same 12 px box, so both styles are one size and switching never shifts a row.
pub(crate) fn status_icon(status: AgentStatus, symbols: bool, theme: &Theme) -> Div {
    let icon = match (symbols, status) {
        (false, AgentStatus::Idle | AgentStatus::Unknown) => "icons/status-idle.svg",
        (false, _) => "icons/status-dot.svg",
        (true, AgentStatus::Blocked) => "icons/status-blocked.svg",
        (true, AgentStatus::Working) => "icons/status-working.svg",
        (true, AgentStatus::Done) => "icons/status-done.svg",
        (true, AgentStatus::Idle) => "icons/status-idle.svg",
        (true, AgentStatus::Unknown) => "icons/status-unknown.svg",
    };
    let faint = if symbols { theme.text_faint } else { theme.text_faint.opacity(0.6) };
    let color = status_color(status, theme).unwrap_or(faint);
    div().size(px(12.)).flex_none().child(gpui::svg().path(icon).size(px(12.)).text_color(color))
}

/// A small square text button (`+`).
pub(super) fn icon_button(theme: &Theme, id: &'static str, glyph: &'static str) -> Stateful<Div> {
    let hover = theme.hover;
    div()
        .id(id)
        .size(px(22.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .text_size(px(15.))
        .text_color(theme.text_muted)
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(glyph)
}

/// Zeron's update strip at the sidebar's foot: an accent chip while there is an update to offer; a click installs it.
pub(super) fn update_strip(theme: &Theme, cx: &App) -> Option<Stateful<Div>> {
    let updates = crate::update::Updates::of(cx);
    let (label, release) = updates.strip()?;
    let tone = if matches!(updates, crate::update::Updates::Failed(..)) { theme.danger } else { theme.accent };
    let hover = tone.opacity(0.22);
    Some(
        div()
            .id("update-strip")
            .m(px(8.))
            .px(px(8.))
            .py(px(6.))
            .rounded(px(8.))
            .bg(tone.opacity(0.14))
            .text_size(px(11.))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(tone)
            .child(div().truncate().child(label))
            .when_some(release, |el, release| {
                el.cursor_pointer().hover(move |s| s.bg(hover)).on_click(move |_, _, cx| crate::update::install(release.clone(), cx))
            }),
    )
}

/// A popup menu's panel.
pub(crate) fn menu_card(theme: &Theme, id: &'static str, min_w: f32) -> Stateful<Div> {
    div()
        .id(id)
        .occlude()
        .min_w(px(min_w))
        .p(px(4.))
        .rounded(px(10.))
        .bg(theme.card)
        .border_1()
        .border_color(theme.border)
        .shadow_lg()
}

/// Zeron's modal: a 35 % scrim over the window, the 360 px dialog card centred on it; both fade with the popup.
pub(super) fn modal<T>(name: &'static str, popup: &crate::ui::Popup<T>, window: &Window, card: Stateful<Div>) -> impl IntoElement {
    let viewport = window.viewport_size();
    let scrim = div()
        .id(SharedString::from(format!("{name}-scrim")))
        .occlude()
        .w(viewport.width)
        .h(viewport.height)
        .bg(gpui::black().opacity(0.35))
        .flex()
        .items_center()
        .justify_center()
        .child(crate::ui::dialog_motion(format!("{name}-in"), popup, card, 2.));
    gpui::deferred(
        gpui::anchored()
            .position(point(px(0.), px(0.)))
            .child(crate::ui::dialog_motion(format!("{name}-scrim-in"), popup, scrim, 0.)),
    )
    .with_priority(2)
}

/// Zeron's dialog card (`popover::dialog_card`) with its title.
pub(super) fn dialog_card(theme: &Theme, id: &'static str, title: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .w(px(360.))
        .p(px(20.))
        .rounded(px(16.))
        .bg(theme.card)
        .border_1()
        .border_color(theme.border)
        .shadow_lg()
        .flex()
        .flex_col()
        .text_color(theme.text)
        .child(div().text_size(px(15.)).font_weight(gpui::FontWeight::SEMIBOLD).child(title))
}

/// Zeron's `btnGhost`: quiet text, a wash on hover.
pub(super) fn ghost_button(theme: &Theme, id: &'static str, label: &'static str) -> Stateful<Div> {
    let hover = theme.hover;
    div().id(id).px(px(12.)).py(px(6.)).rounded(px(8.)).text_color(theme.text_muted).cursor_pointer().hover(move |s| s.bg(hover)).child(label)
}

/// Zeron's `btnPrimary` (and `btnDestructive` with the danger fill).
pub(super) fn solid_button(id: &'static str, label: &'static str, fill: gpui::Hsla, text: gpui::Hsla) -> Stateful<Div> {
    div()
        .id(id)
        .px(px(12.))
        .py(px(6.))
        .rounded(px(8.))
        .bg(fill)
        .text_color(text)
        .font_weight(gpui::FontWeight::MEDIUM)
        .cursor_pointer()
        .hover(|s| s.opacity(0.9))
        .child(label)
}

/// Zeron's folded-section count: a muted badge with monospace digits, here the worktrees a folded repo holds.
pub(super) fn count_badge(theme: &Theme, count: usize) -> Div {
    div()
        .flex_none()
        .h(px(16.))
        .px(px(4.))
        .flex()
        .items_center()
        .rounded(px(4.))
        .bg(theme.wash(0.06))
        .text_size(px(10.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .font_family("Geist Mono")
        .text_color(theme.text_muted.opacity(0.7))
        .child(count.to_string())
}

/// A workspace row with its branch line under the name.
pub(super) const WS_TWO_LINES: f32 = 46.;

/// The agents list's orders: the Agents section's `…` menu.
pub(super) const SORTS: [(AgentSort, &str, &str, &str); 3] = [
    (AgentSort::Status, "sort-status", "icons/checklist.svg", "Status"),
    (AgentSort::Waiting, "sort-waiting", "icons/clock-circle.svg", "Waiting longest"),
    (AgentSort::Grouped, "sort-grouped", "icons/folder.svg", "Grouped"),
];

/// Native context menus: the right press that opened one can be held, dragged onto an item and let go there.
/// A release where the menu opened (`at`) is the press itself ending, not a pick.
pub(super) trait Pick: StatefulInteractiveElement + Sized {
    fn on_pick(self, at: gpui::Point<Pixels>, pick: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        let pick = Rc::new(pick);
        let on_release = pick.clone();
        self.on_click(move |e, w, cx| pick(e, w, cx)).on_mouse_up(MouseButton::Right, move |e, w, cx| {
            if (e.position - at).magnitude() > 4. {
                on_release(&ClickEvent::Mouse(gpui::MouseClickEvent { up: e.clone(), ..Default::default() }), w, cx);
            }
        })
    }
}

impl Pick for Stateful<Div> {}

/// A row of the sidebar's `…` menu: icon, label, a check on the current choice.
pub(super) fn menu_choice(theme: &Theme, id: &'static str, icon: &'static str, label: &'static str, checked: bool) -> Stateful<Div> {
    row(theme, false, id)
        .h(px(28.))
        .rounded(px(6.))
        .child(gpui::svg().path(icon).size(px(14.)).flex_none().text_color(theme.text_muted))
        .child(div().flex_1().child(label))
        .when(checked, |el| el.child(gpui::svg().path("icons/check.svg").size(px(14.)).text_color(theme.text_muted)))
}

/// Zeron's titlebar button: 24 px, a 16 px muted icon, a wash on hover. It occludes the drag strip under it.
pub(super) fn titlebar_button(theme: &Theme, id: &'static str, icon: &'static str) -> Stateful<Div> {
    let hover = theme.hover;
    div()
        .id(id)
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .child(gpui::svg().path(icon).size(px(16.)).text_color(theme.text_muted))
}

/// A 14 px spinner turning once every 900 ms.
pub(super) fn spinner(theme: &Theme, id: String) -> impl IntoElement {
    gpui::svg()
        .path("icons/spinner.svg")
        .size(px(14.))
        .flex_none()
        .text_color(theme.text_faint)
        .with_animation(
            SharedString::from(id),
            gpui::Animation::new(std::time::Duration::from_millis(900)).repeat(),
            |svg, t| svg.with_transformation(gpui::Transformation::rotate(gpui::percentage(t))),
        )
}

/// A worktree herdr is still creating: its branch with a spinner, not selectable.
pub(super) fn pending_worktree_row(theme: &Theme, branch: &str) -> Stateful<Div> {
    div()
        .id(SharedString::from(format!("ws-pending-{branch}")))
        .flex_shrink_0()
        .h(px(32.))
        .pl(px(30.))
        .pr(px(8.))
        .flex()
        .items_center()
        .gap(px(8.))
        .rounded(px(8.))
        .text_size(px(13.))
        .text_color(theme.text_faint)
        .child(spinner(theme, format!("ws-pending-spin-{branch}")))
        .child(div().flex_1().min_w_0().truncate().child(branch.to_string()))
}

/// Zeron's disclosure chevron: points right when folded, turns a quarter down as the group opens.
pub(super) fn fold_chevron(theme: &Theme, id: &str, open: bool, full: f32, tween: Option<FoldMotion>) -> Stateful<Div> {
    let chevron = gpui::svg().path("icons/alt-arrow-right.svg").size(px(12.)).text_color(theme.text_muted.opacity(0.5));
    let turn = |reveal: f32| gpui::Transformation::rotate(gpui::percentage(reveal * 0.25));
    div()
        .id(SharedString::from(format!("ws-fold-{id}")))
        .size(px(16.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .map(|el| match tween {
            Some(m) => el.child(chevron.with_animation(
                SharedString::from(format!("fold-chevron-{id}-{}", m.epoch)),
                fold_animation(),
                move |el, t| el.with_transformation(turn((m.from + (m.to - m.from) * t) / full)),
            )),
            None => el.child(chevron.with_transformation(turn(if open { 1. } else { 0. }))),
        })
}

/// Collie's sidebar row: 32 px, 8 px radius, a wash under the selected one.
pub(crate) fn row(theme: &Theme, selected: bool, id: impl Into<gpui::ElementId>) -> Stateful<Div> {
    let hover = theme.hover;
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .h(px(32.))
        .px(px(8.))
        .rounded(px(8.))
        .flex_shrink_0()
        .cursor_pointer()
        .when(selected, |el| el.bg(theme.selected))
        .when(!selected, |el| el.hover(move |s| s.bg(hover)))
}
