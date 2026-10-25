//! Zeron's select: a pill naming the current option, its list dropped under it; the theme, editor and font ones.
use super::*;

impl SettingsView {
    /// A pill naming the current option; a click drops the list of `labels` under it (Zeron's select).
    /// `fallback` names a value that is none of the options; `leading` draws an option's swatch.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn select(
        &self,
        theme: &Theme,
        id: &'static str,
        labels: Vec<SharedString>,
        current: Option<usize>,
        fallback: SharedString,
        leading: impl Fn(usize) -> Option<gpui::AnyElement>,
        pick: impl Fn(usize, &mut App) + 'static,
        cx: &mut Context<Self>,
    ) -> Div {
        let hover = theme.wash(0.10);
        // A list longer than a screenful gets a filter.
        let searchable = labels.len() > 12;
        let label = current.and_then(|i| labels.get(i).cloned()).unwrap_or(fallback);
        let trigger = div()
            .id(id)
            .w_full()
            .h(px(32.))
            .px(px(10.))
            .flex()
            .items_center()
            .gap(px(8.))
            .rounded(px(8.))
            .bg(theme.wash(0.06))
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .children(current.and_then(&leading))
            .child(div().flex_1().child(label))
            .child(gpui::svg().path("icons/alt-arrow-down.svg").size(px(14.)).text_color(theme.text_muted))
            .on_click(cx.listener(move |v, _: &ClickEvent, window, cx| {
                v.open_select = Some(crate::ui::Popup::new(id));
                // the list opens on the current option, all of them shown
                v.select_cursor = current.unwrap_or(0);
                v.select_scroll.scroll_to_item(v.select_cursor);
                if searchable {
                    v.select_search.update(cx, |input, cx| {
                        input.set_value("", window, cx);
                        input.focus(window, cx);
                    });
                } else {
                    window.focus(&v.select_focus, cx);
                }
                cx.notify();
            }));
        let pick = std::rc::Rc::new(pick);
        let menu = self.open_select.as_ref().filter(|m| m.value == id).map(|menu| {
            let query = if searchable { self.select_search.read(cx).value().trim().to_lowercase() } else { String::new() };
            let shown: Vec<usize> = (0..labels.len()).filter(|&i| labels[i].to_lowercase().contains(&query)).collect();
            let cursor = self.select_cursor.min(shown.len().saturating_sub(1));
            let rows = shown.iter().enumerate().map(|(row, &i)| {
                let pick = pick.clone();
                crate::shell::row(theme, row == cursor, (id, i))
                    .h(px(28.))
                    .flex_none()
                    .rounded(px(6.))
                    .gap(px(8.))
                    .children(leading(i))
                    .child(div().flex_1().child(labels[i].clone()))
                    .when(current == Some(i), |el| el.child(gpui::svg().path("icons/check.svg").size(px(14.)).text_color(theme.text_muted)))
                    .on_mouse_move(cx.listener(move |v, _: &gpui::MouseMoveEvent, _, cx| {
                        if v.select_cursor != row {
                            v.select_cursor = row;
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |v, _: &ClickEvent, window, cx| {
                        v.close_select(window, cx);
                        pick(i, cx);
                    }))
            });
            let list = div()
                .id("select-list")
                .track_scroll(&self.select_scroll)
                .max_h(px(320.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .children(rows)
                .when(shown.is_empty(), |el| el.child(div().px(px(8.)).py(px(6.)).text_color(theme.text_muted).child("No matches")));
            let pick = pick.clone();
            let card = crate::shell::menu_card(theme, "select-menu", 240.)
                .w(px(if searchable { 280. } else { 240. }))
                .mt(px(4.))
                .flex()
                .flex_col()
                .track_focus(&self.select_focus)
                .on_mouse_down_out(cx.listener(|v, _, _, cx| {
                    if let Some(m) = v.open_select.as_mut() { m.close() }
                    cx.notify();
                }))
                // Up and Down (or Emacs's ctrl-p and ctrl-n, as in the command panel) move the highlight round the
                // list, Enter picks it, Escape closes: caught before the field's own bindings.
                .capture_key_down(cx.listener(move |v, e: &KeyDownEvent, window, cx| {
                    let n = shown.len() as isize;
                    let ctrl = e.keystroke.modifiers.control;
                    let key = match e.keystroke.key.as_str() {
                        "p" if ctrl => "up",
                        "n" if ctrl => "down",
                        key => key,
                    };
                    match key {
                        "up" | "down" if n > 0 => {
                            let by = if key == "up" { -1 } else { 1 };
                            v.select_cursor = (cursor as isize + by).rem_euclid(n) as usize;
                            crate::ui::reveal(&v.select_scroll, v.select_cursor..=v.select_cursor, 28.);
                            cx.notify();
                        }
                        "enter" => {
                            v.close_select(window, cx);
                            if let Some(&i) = shown.get(cursor) {
                                pick(i, cx);
                            }
                        }
                        "escape" => v.close_select(window, cx),
                        _ => return,
                    }
                    cx.stop_propagation();
                }))
                // Zeron's popup search: a borderless field on a faint wash, above the list.
                .when(searchable, |el| {
                    el.child(
                        div()
                            .mb(px(4.))
                            .h(px(32.))
                            .flex_none()
                            .px(px(10.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .rounded(px(6.))
                            .bg(theme.wash(0.04))
                            .child(gpui::svg().path("icons/palette-search.svg").size(px(16.)).flex_none().text_color(theme.text_muted))
                            .child(div().flex_1().min_w_0().flex().items_center().line_height(px(16.)).child(Input::new(&self.select_search))),
                    )
                })
                .child(list);
            // `anchored` is absolutely placed at its parent's origin: an in-flow wrapper puts that under the trigger.
            div().child(gpui::deferred(gpui::anchored().snap_to_window_with_margin(px(8.)).child(crate::ui::menu_motion("select-menu-in", menu, card))).with_priority(1))
        });
        // 240 px; a caller sets another width on this
        div().w(px(240.)).flex_none().flex().flex_col().child(trigger).children(menu)
    }

    /// Closes the open select and gives the keyboard back to the page, which Escape then closes.
    pub(super) fn close_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(m) = self.open_select.as_mut() {
            m.close()
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// The themes of one appearance, each with its colors; picking one sets that appearance's theme.
    pub(super) fn theme_select(&self, theme: &Theme, appearance: Appearance, current: &str, cx: &mut Context<Self>) -> Div {
        let (id, light) = match appearance {
            Appearance::Light => ("light-theme-select", true),
            Appearance::Dark => ("dark-theme-select", false),
        };
        let list = variants_for(appearance);
        let labels = list.iter().map(|(_, name)| SharedString::from(name.clone())).collect();
        let index = list.iter().position(|(id, _)| id == current);
        let ids: Vec<String> = list.into_iter().map(|(id, _)| id).collect();
        // Drawn only for the rows shown: the current one, and the list while it is open.
        let preview_ids = ids.clone();
        let font = crate::theme::FontSettings::default();
        self.select(
            theme,
            id,
            labels,
            index,
            current.to_string().into(),
            move |i| Theme::variant(&preview_ids[i], &font).map(|t| palette_preview(&t).into_any_element()),
            move |i, cx| {
                let id = ids[i].clone();
                update(cx, move |s| if light { s.theme_light = id } else { s.theme_dark = id })
            },
            cx,
        )
    }

    pub(super) fn editor_select(&self, theme: &Theme, current: Option<&str>, cx: &mut Context<Self>) -> Div {
        let labels = EDITORS.iter().map(|(_, l)| SharedString::from(*l)).collect();
        let index = EDITORS.iter().position(|(cmd, _)| *cmd == current);
        // A command typed into settings.json by hand shows as itself.
        let fallback = SharedString::from(current.unwrap_or_default().to_string());
        self.select(theme, "editor-select", labels, index, fallback, |_| None, |i, cx| update(cx, |s| s.editor = EDITORS[i].0.map(String::from)), cx)
    }

    /// Every installed family (Zed's font picker); a family named in settings.json that is not installed shows as itself.
    // ponytail: proportional families are listed too; filter by fixed advances (Zeron's fontdb check) if picking one bites.
    pub(super) fn font_select(&self, theme: &Theme, current: &str, cx: &mut Context<Self>) -> Div {
        let fonts = self.fonts.clone();
        let index = fonts.iter().position(|f| f == current);
        self.select(theme, "font-select", fonts.clone(), index, current.to_string().into(), |_| None, move |i, cx| {
            let family = fonts[i].to_string();
            update(cx, |s| s.font_family = family)
        }, cx)
    }
}
