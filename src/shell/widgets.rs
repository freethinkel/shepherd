//! The window's small building blocks: status dots, menus, dialogs, buttons and sidebar rows.
use super::*;

pub(crate) fn status_color(status: AgentStatus, theme: &Theme) -> Option<Hsla> {
    match status {
        AgentStatus::Working => Some(theme.accent),
        AgentStatus::Blocked => Some(theme.danger),
        AgentStatus::Done => Some(theme.success.opacity(0.9)),
        AgentStatus::Idle | AgentStatus::Unknown => None,
    }
}

/// An agent's status in the theme's style (the settings' Status icons).
pub(crate) fn status_dot(status: AgentStatus, theme: &Theme) -> Div {
    status_icon(status, theme.status_icons, theme)
}

/// Collie's status dot, filled for an agent state, a faint ring when idle; `Symbols` draws herdr's
/// `status_indicators = "symbols"` shapes instead (× ◐ ✓ ○, a dashed ring for its `·`), and `Animated` Zeron's: its
/// 2×3 glyph spinner while the agent works, a check when done, a question mark while it asks, a small dot otherwise. Every style fills the same
/// 12 px box in the same colors, so switching never shifts a row.
pub(crate) fn status_icon(status: AgentStatus, style: crate::theme::StatusIcons, theme: &Theme) -> Div {
    use crate::theme::StatusIcons::*;
    let faint = if style == Symbols { theme.text_faint } else { theme.text_faint.opacity(0.6) };
    let color = status_color(status, theme).unwrap_or(faint);
    let el = div().size(px(12.)).flex_none().flex().items_center().justify_center();
    // Animated is drawn a quarter up on the others, the spinner's cells 2.5 px: the check's drawing still fits the box
    let icon = match (style, status) {
        (Animated, AgentStatus::Working) => return el.child(crate::ui::loader::glyph_spinner(2.5, glyph_rows(color, theme))),
        (Animated, AgentStatus::Done) => return el.child(gpui::svg().path("icons/status-done.svg").flex_none().size(px(15.)).text_color(color)),
        (Animated, AgentStatus::Blocked) => return el.child(gpui::svg().path("icons/status-question.svg").flex_none().size(px(15.)).text_color(color)),
        (Animated, _) => return el.child(div().size(px(7.5)).rounded_full().bg(color)),
        (Dots, AgentStatus::Idle | AgentStatus::Unknown) => "icons/status-idle.svg",
        (Dots, _) => "icons/status-dot.svg",
        (Symbols, AgentStatus::Blocked) => "icons/status-blocked.svg",
        (Symbols, AgentStatus::Working) => "icons/status-working.svg",
        (Symbols, AgentStatus::Done) => "icons/status-done.svg",
        (Symbols, AgentStatus::Idle) => "icons/status-idle.svg",
        (Symbols, AgentStatus::Unknown) => "icons/status-unknown.svg",
    };
    el.child(gpui::svg().path(icon).size(px(12.)).text_color(color))
}

/// Zeron's `GlyphPalette::for_accent`: the spinner's rows, top to bottom, lighter, `tint` itself, deeper. The tint
/// is saturated a step past the accent and the light row kept nearer it, so a working agent stands out of a list.
fn glyph_rows(tint: Hsla, theme: &Theme) -> [Hsla; 3] {
    let mut tint = Hsla { s: (tint.s * 1.35).min(1.), ..tint };
    // a dark accent sinks into a dark sidebar: lift it toward the text
    if theme.appearance == crate::theme::Appearance::Dark {
        tint.l = tint.l.max(0.62);
    }
    let (mut light, mut deep) = (tint, tint);
    if theme.appearance == crate::theme::Appearance::Dark {
        (light.l, light.s) = ((light.l + 0.14).min(0.90), light.s * 0.9);
        deep.l = (deep.l - 0.08).max(0.45);
    } else {
        (light.l, light.s) = ((light.l + 0.11).min(0.76), light.s * 0.9);
        deep.l = (deep.l - 0.09).max(0.22);
    }
    [light, tint, deep]
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
pub(crate) fn modal<T>(name: &'static str, popup: &crate::ui::Popup<T>, window: &Window, card: Stateful<Div>) -> impl IntoElement {
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

/// Zeron's `TextTooltip`: a one-line chip under the pointer.
struct Tip(SharedString);

impl Render for Tip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx);
        div()
            .max_w(px(320.))
            .px(px(9.))
            .py(px(6.))
            .rounded(px(6.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.card)
            .text_size(px(11.))
            .text_color(theme.text_muted)
            .child(self.0.clone())
    }
}

/// `.tooltip(...)` with `text`, and `action`'s chord after it while one is bound ("Settings · ⌘,").
pub(crate) fn tip(text: impl Into<SharedString>, action: Option<&'static str>) -> impl Fn(&mut Window, &mut App) -> gpui::AnyView + 'static {
    let text: SharedString = text.into();
    move |_, cx| {
        let chord = action.and_then(|a| cx.try_global::<ActiveBindings>()?.0.iter().find(|(name, _)| name == a).map(|(_, keys)| keys.clone()));
        let text = match chord {
            Some(keys) => format!("{text} · {}", keys.split_whitespace().map(crate::ui::command_panel::glyphs).collect::<Vec<_>>().join(" ")).into(),
            None => text.clone(),
        };
        cx.new(|_| Tip(text)).into()
    }
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

/// Off macOS, the window's own buttons at the titlebar's right end, those the desktop offers: minimize, maximize (or
/// restore) and close.
pub(super) fn window_controls(theme: &Theme, window: &Window) -> Option<Div> {
    if cfg!(target_os = "macos") {
        return None;
    }
    let offered = window.window_controls();
    let maximize = if window.is_maximized() { "icons/window-restore.svg" } else { "icons/window-maximize.svg" };
    let button = |id, icon| titlebar_button(theme, id, icon).size(px(28.));
    Some(
        div()
            .absolute()
            .top(px(TRAFFIC_LIGHTS_CENTER - 14.))
            .right(px(8.))
            .flex()
            .gap(px(4.))
            .when(offered.minimize, |el| el.child(button("window-minimize", "icons/minus.svg").on_click(|_, window, _| window.minimize_window())))
            .when(offered.maximize, |el| el.child(button("window-maximize", maximize).on_click(|_, window, _| window.zoom_window())))
            .child(button("window-close", "icons/close.svg").on_click(|_, window, _| window.remove_window())),
    )
}

/// Off macOS the window draws its own frame, so its edges and corners resize it, until it's maximized or tiled.
pub(super) fn resize_edges(window: &Window) -> Vec<Stateful<Div>> {
    use gpui::{CursorStyle as C, ResizeEdge as E};
    if cfg!(target_os = "macos") || window.is_maximized() {
        return Vec::new();
    }
    const EDGE: f32 = 5.;
    const CORNER: f32 = 12.;
    let strip = |id: &'static str, edge: E, cursor: C| {
        div()
            .id(id)
            .absolute()
            .occlude()
            .cursor(cursor)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                cx.stop_propagation();
                window.start_window_resize(edge);
            })
    };
    vec![
        strip("resize-top", E::Top, C::ResizeUpDown).top_0().left(px(CORNER)).right(px(CORNER)).h(px(EDGE)),
        strip("resize-bottom", E::Bottom, C::ResizeUpDown).bottom_0().left(px(CORNER)).right(px(CORNER)).h(px(EDGE)),
        strip("resize-left", E::Left, C::ResizeLeftRight).left_0().top(px(CORNER)).bottom(px(CORNER)).w(px(EDGE)),
        strip("resize-right", E::Right, C::ResizeLeftRight).right_0().top(px(CORNER)).bottom(px(CORNER)).w(px(EDGE)),
        strip("resize-top-left", E::TopLeft, C::ResizeUpLeftDownRight).top_0().left_0().size(px(CORNER)),
        strip("resize-top-right", E::TopRight, C::ResizeUpRightDownLeft).top_0().right_0().size(px(CORNER)),
        strip("resize-bottom-left", E::BottomLeft, C::ResizeUpRightDownLeft).bottom_0().left_0().size(px(CORNER)),
        strip("resize-bottom-right", E::BottomRight, C::ResizeUpLeftDownRight).bottom_0().right_0().size(px(CORNER)),
    ]
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
