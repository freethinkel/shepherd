//! Browser tabs: web pages in a workspace's tab bar beside herdr's tabs, opened from `+`'s menu or by a link clicked
//! in a terminal while the setting says so. They are Shepherd's own (herdr doesn't know them) and last until Shepherd
//! quits; the pages themselves are `web.rs`'s, as the sidebar's are.
use super::*;
use super::web::{Favicon, PageKey, nav_button};
use crate::settings::WebEntry;

/// Browser tabs' ids start with this, so they never meet a web entry's.
const PREFIX: &str = "browser-";
/// An empty tab's page.
const BLANK: &str = "about:blank";

pub(super) struct BrowserTab {
    pub(super) id: String,
    pub(super) workspace: String,
    /// What it opened on; where its links lead from there is the page's business.
    url: String,
    /// The herdr tab it follows in the tab bar; `None` before them all.
    pub(super) after: Option<String>,
}

#[derive(Default)]
pub(super) struct Browser {
    pub(super) tabs: Vec<BrowserTab>,
    made: u64,
    /// The shown tab's address field.
    address: Option<Address>,
    /// A link clicked in a terminal, opened by the next render, which has the window.
    pending: Option<String>,
    /// Each workspace's herdr tabs as last seen, so a tab whose herdr tab closed follows the one before it.
    seen: HashMap<String, Vec<String>>,
}

struct Address {
    tab: String,
    input: Entity<InputState>,
    /// Being typed in: the page doesn't take the keyboard, and loads don't overwrite it.
    editing: bool,
    _events: Subscription,
}

impl Browser {
    /// The tabs as web entries, which is how the pages know them.
    pub(super) fn entries(&self) -> impl Iterator<Item = WebEntry> + '_ {
        self.tabs.iter().map(|t| WebEntry { id: t.id.clone(), url: t.url.clone(), ..Default::default() })
    }

    pub(super) fn editing(&self) -> bool {
        self.address.as_ref().is_some_and(|a| a.editing)
    }
}

/// Whether page `id` is a browser tab's.
pub(super) fn is_tab(id: &str) -> bool {
    id.starts_with(PREFIX)
}

/// The shell links clicked in a terminal go to.
pub(super) struct Links(pub(super) gpui::WeakEntity<Shell>);

impl gpui::Global for Links {}

/// A link clicked in a terminal: a web one to a browser tab beside the shown tab while the setting says so, anything
/// else to the system.
pub(crate) fn open_link(url: &str, cx: &mut App) {
    let web = url.starts_with("http://") || url.starts_with("https://");
    let shell = cx.try_global::<Links>().and_then(|l| l.0.upgrade()).filter(|_| web && SettingsGlobal::of(cx).settings.web_links_inside);
    match shell {
        Some(shell) => shell.update(cx, |s, cx| {
            s.browser.pending = Some(url.to_string());
            cx.notify();
        }),
        None => cx.open_url(url),
    }
}

/// `https://example.com/a?b` → `https://example.com`: what a tab's favicon is fetched again for.
fn site(url: &str) -> Option<String> {
    let uri: http::Uri = url.parse().ok()?;
    Some(format!("{}://{}", uri.scheme_str()?, uri.authority()?))
}

/// What's typed in the address field, as a URL: `localhost:3000` and `example.com` get `http://`, anything with a
/// space or without a dot or a port goes to a search.
fn typed_url(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if text.contains("://") || text.starts_with("about:") {
        return Some(text.to_string());
    }
    let host = text.split(['/', '?', '#']).next().unwrap_or_default();
    if !text.contains(' ') && (host.contains('.') || host.contains(':') || host == "localhost") {
        return Some(format!("http://{text}"));
    }
    let query: String = percent_encoding::utf8_percent_encode(text, percent_encoding::NON_ALPHANUMERIC).to_string();
    Some(format!("https://duckduckgo.com/?q={query}"))
}

impl Shell {
    /// A new browser tab in workspace `ws` on `url`, shown, after herdr tab `after` (and the browser tabs already
    /// there); without a URL, an empty tab with its address field to type in.
    pub(super) fn new_browser_tab(&mut self, ws: String, after: Option<String>, url: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.browser.made += 1;
        let id = format!("{PREFIX}{}", self.browser.made);
        let blank = url.is_none();
        self.browser.tabs.push(BrowserTab { id: id.clone(), workspace: ws, url: url.unwrap_or_else(|| BLANK.into()), after });
        self.show_web(id.clone(), window, cx);
        if blank {
            let input = self.address(&id, window, cx);
            input.update(cx, |input, cx| input.focus(window, cx));
        }
    }

    /// Closes browser tab `id`; shown, the workspace's tabs come back. Its page goes with it in `sync_pages`.
    pub(super) fn close_browser_tab(&mut self, id: &str, cx: &mut Context<Self>) {
        self.browser.tabs.retain(|t| t.id != id);
        if self.pages.shown.as_deref() == Some(id) {
            self.pages.shown = None;
        }
        cx.notify();
    }

    /// Closes tab `id` of the tab bar: a browser tab here, a herdr tab in herdr.
    pub(super) fn close_tab(&mut self, id: &str, browser: bool, cx: &mut Context<Self>) {
        match browser {
            true => self.close_browser_tab(id, cx),
            false => self.send(Command::CloseTab { tab_id: id.to_string() }, cx, |_, _, _| {}),
        }
    }

    /// Once per render, before the pages sync: a link clicked in a terminal opens its tab; tabs of workspaces gone
    /// close.
    pub(super) fn sync_browser(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(url) = self.browser.pending.take() {
            // beside the terminal it was clicked in
            match self.shown_tab() {
                Some(tab) => self.new_browser_tab(tab.workspace_id, Some(tab.tab_id), Some(url), window, cx),
                None => cx.open_url(&url),
            }
        }
        let Some(s) = self.snapshot.as_ref().filter(|s| !s.workspaces.is_empty()) else { return };
        // ponytail: another place shown (a session, a machine) closes this one's browser tabs; keyed by place if missed
        self.browser.tabs.retain(|t| s.workspaces.iter().any(|w| w.workspace_id == t.workspace));
        let Browser { tabs, seen, .. } = &mut self.browser;
        for t in tabs.iter_mut() {
            let now: Vec<String> = s.tabs_of(&t.workspace).into_iter().map(|h| h.tab_id.clone()).collect();
            t.after = follows(t.after.take(), seen.get(&t.workspace).map(Vec::as_slice).unwrap_or_default(), &now);
        }
        seen.clear();
        for t in tabs.iter() {
            seen.entry(t.workspace.clone()).or_insert_with(|| s.tabs_of(&t.workspace).into_iter().map(|h| h.tab_id.clone()).collect());
        }
    }

    /// A tab dragged in the tab bar from `from` to `to` in `order` (each tab's id, and whether it is a browser
    /// tab): herdr is told a herdr tab's new place, and every browser tab follows the herdr tab now before it.
    pub(super) fn drop_tab(&mut self, order: &[(String, bool)], from: usize, to: usize, cx: &mut Context<Self>) {
        let (moved, anchors) = dropped(order, from, to);
        if let Some((tab_id, insert_index)) = moved {
            self.send(Command::MoveTab { tab_id, insert_index }, cx, |_, _, _| {});
        }
        // in their new order, after the workspace's other browser tabs
        let ids: Vec<&String> = anchors.iter().map(|(id, _)| id).collect();
        let (mut kept, mut ours): (Vec<BrowserTab>, Vec<BrowserTab>) = std::mem::take(&mut self.browser.tabs).into_iter().partition(|t| !ids.contains(&&t.id));
        ours.sort_by_key(|t| ids.iter().position(|id| **id == t.id));
        for (t, (_, after)) in ours.iter_mut().zip(&anchors) {
            t.after = after.clone();
        }
        kept.extend(ours);
        self.browser.tabs = kept;
        cx.notify();
    }

    /// The browser tab shown, if a page shown is one.
    pub(super) fn shown_browser_tab(&self) -> Option<&BrowserTab> {
        let id = self.pages.shown.as_deref()?;
        self.browser.tabs.iter().find(|t| t.id == id)
    }

    /// Tab `id`'s label: its page's title, else its site, else New Tab.
    fn browser_label(&self, id: &str) -> String {
        let page = self.pages.live.get(&(id.to_string(), None));
        let title = page.and_then(|p| p.title.clone());
        let url = page.and_then(|p| p.view.url().ok()).filter(|u| u != BLANK);
        title.or_else(|| url.and_then(|u| u.parse::<http::Uri>().ok()?.host().map(str::to_string))).unwrap_or_else(|| "New Tab".into())
    }

    /// The address field of tab `id`, made for it if the field was another tab's.
    fn address(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        if let Some(a) = self.browser.address.as_ref().filter(|a| a.tab == id) {
            return a.input.clone();
        }
        let theme = Theme::of(cx).clone();
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Address or search"));
        input.update(cx, |input, _| input.set_editor_style(crate::ui::input_style(&theme)));
        let tab = id.to_string();
        let events = cx.subscribe_in(&input, window, move |shell, input, event: &InputEvent, window, cx| {
            let key: PageKey = (tab.clone(), None);
            match event {
                InputEvent::Focus => {
                    if let Some(a) = shell.browser.address.as_mut() {
                        a.editing = true;
                    }
                    input.update(cx, |input, cx| input.select_all(window, cx));
                }
                InputEvent::Blur => {
                    if let Some(a) = shell.browser.address.as_mut() {
                        a.editing = false;
                    }
                    shell.show_address(&key, window, cx);
                }
                InputEvent::PressEnter { .. } => {
                    let url = typed_url(&input.read(cx).value());
                    if let (Some(url), Some(page)) = (url, shell.pages.live.get(&key)) {
                        let _ = page.view.load_url(&url);
                    }
                    // the keyboard to the page: `sync_pages` hands it over once the field lets go
                    window.focus(&shell.focus, cx);
                }
                InputEvent::Change => {}
            }
            cx.notify();
        });
        self.browser.address = Some(Address { tab: id.to_string(), input: input.clone(), editing: false, _events: events });
        input
    }

    /// The shown browser tab's address field takes the keyboard, its text selected; a sidebar page has none.
    pub(super) fn focus_address(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown_browser_tab().map(|t| t.id.clone()) else { return };
        let input = self.address(&id, window, cx);
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
    }

    /// The address field shows the page's URL, unless it is being typed in.
    fn show_address(&mut self, key: &PageKey, window: &mut Window, cx: &mut Context<Self>) {
        let Some(a) = self.browser.address.as_ref().filter(|a| a.tab == key.0 && !a.editing) else { return };
        let url = self.pages.live.get(key).and_then(|p| p.view.url().ok()).filter(|u| u != BLANK).unwrap_or_default();
        if a.input.read(cx).value() != url {
            a.input.update(cx, |input, cx| input.set_value(url, window, cx));
        }
    }

    /// A browser tab's page loaded: its address shown, and its site's icon fetched when the site changed.
    pub(super) fn browser_loaded(&mut self, key: &PageKey, window: &mut Window, cx: &mut Context<Self>) {
        self.show_address(key, window, cx);
        let Some(url) = self.pages.live.get(key).and_then(|p| p.view.url().ok()) else { return };
        if let Some(site) = site(&url).filter(|s| self.pages.favicons.get(key).is_none_or(|(had, _)| had != s)) {
            self.fetch_favicon(key.clone(), site, url, cx);
        }
    }

    /// The browser tabs of workspace `ws` for the tab bar: id, label, the site's icon once fetched, the herdr tab each
    /// follows, and whether its page is loading.
    pub(super) fn browser_tabs_of(&self, ws: &str) -> Vec<(String, String, Option<std::sync::Arc<gpui::Image>>, Option<String>, bool)> {
        let icon = |id: &String| match self.pages.favicons.get(&(id.clone(), None)) {
            Some((_, Favicon::Image(image))) => Some(image.clone()),
            _ => None,
        };
        let loading = |id: &String| self.pages.live.get(&(id.clone(), None)).is_some_and(|p| p.loading);
        self.browser.tabs.iter().filter(|t| t.workspace == ws).map(|t| (t.id.clone(), self.browser_label(&t.id), icon(&t.id), t.after.clone(), loading(&t.id))).collect()
    }

    /// The shown browser tab's row under the tab bar: back, forward, reload, the address and Open in Browser.
    pub(super) fn render_browser_bar(&mut self, key: &PageKey, theme: &Theme, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let input = self.address(&key.0, window, cx);
        if !self.browser.editing() && input.read(cx).value().is_empty() {
            self.show_address(key, window, cx);
        }
        let view = self.pages.live.get(key).map(|p| p.view.clone());
        let can = |f: fn(&wry::WebView) -> wry::Result<bool>| view.as_deref().is_some_and(|v| f(v).unwrap_or(false));
        let outside = view.clone();
        div()
            .h(px(36.))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(2.))
            .px(px(6.))
            .bg(theme.surface)
            .border_b_1()
            .border_color(theme.border)
            .child(nav_button(theme, view.clone(), "browser-back", "icons/solar/alt-arrow-left.svg", can(wry::WebView::can_go_back), wry::WebView::go_back))
            .child(nav_button(theme, view.clone(), "browser-forward", "icons/solar/alt-arrow-right.svg", can(wry::WebView::can_go_forward), wry::WebView::go_forward))
            .child(self.reload_button(key, "browser-reload", theme, cx))
            .child(
                div()
                    .mx(px(4.))
                    .flex_1()
                    .min_w_0()
                    .h(px(28.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .rounded(px(8.))
                    .bg(theme.terminal.background)
                    .text_size(px(12.5))
                    .child(Input::new(&input)),
            )
            .child(
                div()
                    .id("browser-outside")
                    .size(px(28.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(8.))
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.hover))
                    .tooltip(tip("Open in Browser", None))
                    .child(gpui::svg().path("icons/solar/square-arrow-right-up.svg").size(px(16.)).text_color(theme.text_muted))
                    .on_click(move |_, _, cx| {
                        if let Some(url) = outside.as_deref().and_then(|v| v.url().ok()).filter(|u| u != BLANK) {
                            cx.open_url(&url);
                        }
                    }),
            )
    }

    /// `+`'s menu: a new terminal tab or a browser tab in workspace `ws`.
    pub(super) fn new_tab_menu(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let menu = self.new_tab_menu.as_ref()?;
        let (ws, at) = menu.value.clone();
        type Pick = Box<dyn Fn(&mut Shell, String, &mut Window, &mut Context<Shell>)>;
        let item = |key: &'static str, label: &'static str, pick: Pick| {
            let ws = ws.clone();
            row(theme, false, key).h(px(28.)).rounded(px(6.)).child(label).on_pick(
                at,
                cx.listener(move |this, _: &ClickEvent, window, cx| {
                    if let Some(m) = this.new_tab_menu.as_mut() {
                        m.close()
                    }
                    pick(this, ws.clone(), window, cx);
                    cx.notify();
                }),
            )
        };
        let card = menu_card(theme, "new-tab-menu", 160.)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if let Some(m) = this.new_tab_menu.as_mut() {
                    m.close()
                }
                cx.notify();
            }))
            .child(item("new-tab-terminal", "Terminal", Box::new(|this, ws, _, cx| this.new_tab(ws, cx))))
            .child(item("new-tab-browser", "Browser", Box::new(|this, ws, window, cx| {
                // at the end of the bar
                let last = this.snapshot.as_ref().and_then(|s| s.tabs_of(&ws).last().map(|t| t.tab_id.clone()));
                this.new_browser_tab(ws, last, None, window, cx)
            })));
        let menu = crate::ui::menu_motion("new-tab-menu-in", menu, card);
        Some(
            gpui::deferred(gpui::anchored().position(at).anchor(gpui::Anchor::TopRight).snap_to_window_with_margin(px(8.)).child(menu))
                .with_priority(1)
                .into_any_element(),
        )
    }
}

/// The herdr tab a browser tab follows once herdr's tabs are `now` (they were `before`): its own while there, else
/// the nearest before it that still is, else none (the bar's start).
fn follows(after: Option<String>, before: &[String], now: &[String]) -> Option<String> {
    let after = after?;
    if now.contains(&after) {
        return Some(after);
    }
    let at = before.iter().position(|id| *id == after)?;
    before[..at].iter().rev().find(|id| now.contains(id)).cloned()
}

/// The bar's tabs (id, browser or not) in the order shown: herdr's in their order, each browser tab after the herdr
/// tab it follows; one following a tab not there goes last.
pub(super) fn interleave<'a>(herdr: &[&'a str], browser: &[(&'a str, Option<&str>)]) -> Vec<(&'a str, bool)> {
    let after = |anchor: Option<&str>| -> Vec<(&'a str, bool)> { browser.iter().filter(|(_, a)| *a == anchor).map(|(id, _)| (*id, true)).collect() };
    let mut order = after(None);
    for h in herdr {
        order.push((h, false));
        order.extend(after(Some(h)));
    }
    order.extend(browser.iter().filter(|(_, a)| a.is_some_and(|a| !herdr.contains(&a))).map(|(id, _)| (*id, true)));
    order
}

/// The tab at `from` dropped on the one at `to`, taking its place: the herdr tab moved, with herdr's insert index
/// (counted before it is taken out), if herdr's order changed; and each browser tab, in the new order, with the herdr
/// tab before it.
fn dropped(order: &[(String, bool)], from: usize, to: usize) -> (Option<(String, usize)>, Vec<(String, Option<String>)>) {
    let mut now = order.to_vec();
    let tab = now.remove(from);
    now.insert(to, tab.clone());
    let herdr_at = |list: &[(String, bool)], i: usize| list[..i].iter().filter(|(_, b)| !b).count();
    let moved = (!tab.1).then(|| (herdr_at(order, from), herdr_at(&now, to))).filter(|(was, is)| was != is);
    let moved = moved.map(|(was, is)| (tab.0.clone(), if is > was { is + 1 } else { is }));
    let mut before = None;
    let mut anchors = Vec::new();
    for (id, browser) in now {
        match browser {
            true => anchors.push((id, before.clone())),
            false => before = Some(id),
        }
    }
    (moved, anchors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_addresses_become_urls_or_searches() {
        assert_eq!(typed_url("localhost:3000").as_deref(), Some("http://localhost:3000"));
        assert_eq!(typed_url("localhost").as_deref(), Some("http://localhost"));
        assert_eq!(typed_url(" docs.rs/gpui ").as_deref(), Some("http://docs.rs/gpui"));
        assert_eq!(typed_url("https://x.dev/a").as_deref(), Some("https://x.dev/a"));
        assert_eq!(typed_url("rust lifetimes").as_deref(), Some("https://duckduckgo.com/?q=rust%20lifetimes"));
        assert_eq!(typed_url("  "), None);
        assert_eq!(site("https://x.dev:8080/a?b").as_deref(), Some("https://x.dev:8080"));
    }

    #[test]
    fn browser_tabs_sit_after_their_herdr_tab() {
        let order = interleave(&["h1", "h2"], &[("b1", Some("h1")), ("b0", None), ("b2", Some("gone")), ("b3", Some("h1"))]);
        assert_eq!(order, [("b0", true), ("h1", false), ("b1", true), ("b3", true), ("h2", false), ("b2", true)]);
    }

    #[test]
    fn a_closed_herdr_tab_hands_its_browser_tabs_to_the_one_before() {
        let ids = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let (before, now) = (ids(&["h1", "h2", "h3"]), ids(&["h1", "h3"]));
        assert_eq!(follows(Some("h2".into()), &before, &now).as_deref(), Some("h1"));
        assert_eq!(follows(Some("h3".into()), &before, &now).as_deref(), Some("h3"));
        assert_eq!(follows(Some("h1".into()), &before, &ids(&["h3"])), None);
        assert_eq!(follows(None, &before, &now), None);
    }

    #[test]
    fn a_dropped_tab_moves_herdr_and_reanchors_browser_tabs() {
        let order: Vec<(String, bool)> = [("h1", false), ("b1", true), ("h2", false), ("h3", false)].iter().map(|(i, b)| (i.to_string(), *b)).collect();
        let anchors = |l: &[(&str, Option<&str>)]| l.iter().map(|(i, a)| (i.to_string(), a.map(str::to_string))).collect::<Vec<_>>();
        // a browser tab dropped on the last tab: herdr untouched
        assert_eq!(dropped(&order, 1, 3), (None, anchors(&[("b1", Some("h3"))])));
        // the first herdr tab dropped on h2, moving right: herdr's index counts it still in place
        assert_eq!(dropped(&order, 0, 2), (Some(("h1".into(), 2)), anchors(&[("b1", None)])));
        // h3 dropped on the browser tab, moving left: it lands between h1 and the browser tab
        assert_eq!(dropped(&order, 3, 1), (Some(("h3".into(), 1)), anchors(&[("b1", Some("h3"))])));
    }
}
