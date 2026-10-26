//! Settings → Web pages: an editable row per page, saved on Enter or when a field is left.
use super::*;

/// One web page's fields, saved on Enter or when left.
pub(super) struct WebFields {
    id: String,
    name: Entity<InputState>,
    url: Entity<InputState>,
    _events: [Subscription; 2],
}

/// Changes the web page `id`, and saves.
pub(super) fn edit_web(cx: &mut App, id: &str, change: impl FnOnce(&mut WebEntry)) {
    update(cx, |s| {
        if let Some(e) = s.web_entries.iter_mut().find(|e| e.id == id) {
            change(e)
        }
    });
}

/// The id of web page `id`'s `which` select (Show in, Open from): selects are keyed by a `&'static str`.
// ponytail: two leaked strings per web page shown in this run; key selects by SharedString if that ever matters
pub(super) fn web_select_id(which: &str, id: &str) -> &'static str {
    static IDS: std::sync::Mutex<std::collections::BTreeMap<String, &'static str>> = std::sync::Mutex::new(std::collections::BTreeMap::new());
    let key = format!("web-{which}-{id}");
    IDS.lock().unwrap().entry(key.clone()).or_insert_with(|| Box::leak(key.into_boxed_str()))
}

impl SettingsView {
    /// Fields for the web pages in the settings: kept for the ones still there, made for new ones.
    pub(super) fn sync_web_fields(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let entries = SettingsGlobal::of(cx).settings.web_entries.clone();
        if self.web_fields.iter().map(|f| &f.id).eq(entries.iter().map(|e| &e.id)) {
            return;
        }
        let theme = Theme::of(cx).clone();
        let mut old = std::mem::take(&mut self.web_fields);
        for entry in entries {
            if let Some(at) = old.iter().position(|f| f.id == entry.id) {
                self.web_fields.push(old.remove(at));
                continue;
            }
            let input = |value: &str, placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
                cx.new(|cx| {
                    let mut input = InputState::new(window, cx).placeholder(placeholder).default_value(value.to_string());
                    input.set_editor_style(input_style(&theme));
                    input
                })
            };
            let name = input(&entry.name, "The page's title", window, cx);
            let url = input(&entry.url, "http://localhost:4317", window, cx);
            let setters: [(&Entity<InputState>, fn(&mut WebEntry, String)); 2] = [(&name, |e, v| e.name = v), (&url, |e, v| e.url = v)];
            let events = setters.map(|(field, set)| {
                let id = entry.id.clone();
                cx.subscribe_in(field, window, move |_, field, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                        let value = field.read(cx).value().trim().to_string();
                        edit_web(cx, &id, |e| set(e, value));
                    }
                })
            });
            self.web_fields.push(WebFields { id: entry.id, name, url, _events: events });
        }
    }

    /// Settings → Web pages, Zeron's way: each page a block of labelled fields on a two-column grid
    /// (name and URL, where it is listed and where it opens) with a red bin to remove it, "Add page" by the section's
    /// label, and the switch for links to other sites.
    pub(super) fn web_pages(&self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let settings = &SettingsGlobal::of(cx).settings;
        let (entries, outside, inside) = (settings.web_entries.clone(), settings.web_links_outside, settings.web_links_inside);
        let muted = theme.text_muted;
        let labelled = |label: &'static str, control: Div| labelled(theme, label, control);
        let pair = |left: Div, right: Div| div().flex().gap(px(12.)).child(left).child(right);
        let rows: Vec<Div> = self
            .web_fields
            .iter()
            .zip(&entries)
            .enumerate()
            .map(|(i, (fields, entry))| {
                let id = entry.id.clone();
                let places = self.places.clone().unwrap_or_default();
                let labels = std::iter::once(SharedString::from("Everywhere")).chain(places.iter().map(|(_, n)| SharedString::from(n.clone()))).collect();
                let current = match &entry.show_in {
                    None => Some(0),
                    Some(key) => places.iter().position(|(k, _)| k == key).map(|at| at + 1),
                };
                // a place no longer there (stopped, removed) shows as its key; until herdr answers, nothing
                let fallback = match (&entry.show_in, &self.places) {
                    (Some(key), Some(_)) => format!("{key} (gone)"),
                    _ => String::new(),
                };
                let keys: Vec<Option<String>> = std::iter::once(None).chain(places.into_iter().map(|(k, _)| Some(k))).collect();
                let pick_place = {
                    let id = id.clone();
                    move |i: usize, cx: &mut App| {
                        let key = keys[i].clone();
                        edit_web(cx, &id, |e| e.show_in = key)
                    }
                };
                let show_in = self.select(theme, web_select_id("show", &id), labels, current, fallback.into(), |_| None, pick_place, cx);
                pub(super) const FROM: [(OpenFrom, &str); 2] = [(OpenFrom::Here, "This computer"), (OpenFrom::Machine, "The machine shown")];
                let pick_from = {
                    let id = id.clone();
                    move |i: usize, cx: &mut App| edit_web(cx, &id, |e| e.open_from = FROM[i].0)
                };
                let from = FROM.iter().position(|(f, _)| *f == entry.open_from);
                let open_from = self.select(theme, web_select_id("from", &id), FROM.iter().map(|(_, l)| SharedString::from(*l)).collect(), from, "".into(), |_| None, pick_from, cx);
                let remove = remove_button(theme, format!("web-remove-{id}")).on_click(move |_, _, cx| update(cx, |s| s.web_entries.retain(|e| e.id != id)));
                div()
                    .mx(px(16.))
                    .py(px(14.))
                    .when(i > 0, |el| el.border_t_1().border_color(theme.border.opacity(0.6)))
                    .flex()
                    .items_start()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .child(pair(labelled("Name", field(theme, &fields.name).w_full()), labelled("URL", field(theme, &fields.url).w_full())))
                            .child(pair(labelled("Show in", show_in.w_full()), labelled("Open from", open_from.w_full()))),
                    )
                    .child(remove)
            })
            .collect();
        let empty = rows.is_empty();
        div()
            .child(
                div()
                    .mt(px(32.))
                    .mb(px(8.))
                    .pl(px(8.))
                    .flex()
                    .items_center()
                    .child(div().flex_1().text_color(muted).child("Pages"))
                    .child(
                        action(theme, "web-add", "Add page")
                            .gap(px(6.))
                            .child(gpui::svg().path("icons/solar/add-circle.svg").size(px(14.)).text_color(muted))
                            .on_click(|_, _, cx| update(cx, |s| s.web_entries.push(WebEntry::new()))),
                    ),
            )
            .child(
                card(theme).mt(px(0.)).children(rows).when(empty, |el| {
                    el.child(card_row(theme, true).child(row_text(
                        theme,
                        "No web pages yet",
                        "Add one with its URL: a dev server, a dashboard, a tool on a machine.",
                    )))
                }),
            )
            .child(
                card(theme).mt(px(12.)).child(
                    toggle_row(theme, true, "web-links-outside", "Open links to other sites in the browser", "Off: they open in the page, inside Shepherd.", outside)
                        .on_click(|_, _, cx| update(cx, |s| s.web_links_outside = !s.web_links_outside)),
                )
                .child(
                    toggle_row(theme, false, "web-links-inside", "Open terminal links in a browser tab", "Cmd+click on a web link opens it beside the terminal. Off: the system browser.", inside)
                        .on_click(|_, _, cx| update(cx, |s| s.web_links_inside = !s.web_links_inside)),
                ),
            )
    }
}
