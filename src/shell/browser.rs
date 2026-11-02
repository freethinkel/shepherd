//! Browser tabs: web pages in a workspace's right panel beside the panes, opened from the panel or by a link clicked
//! in a terminal while the setting says so. They are Shepherd's own (herdr doesn't know them) and last until Shepherd
//! quits; the pages themselves are `web.rs`'s, as the sidebar's are.
use super::*;
use super::web::{Favicon, PageKey, nav_button};
use crate::right_panel::Surface;
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
}

#[derive(Default)]
pub(super) struct Browser {
    pub(super) tabs: Vec<BrowserTab>,
    made: u64,
    /// The shown tab's address field.
    address: Option<Address>,
    /// A link clicked in a terminal, opened by the next render, which has the window.
    pending: Option<String>,
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

/// A link clicked in a terminal: a web one to a browser tab in the right panel while the setting says so, anything
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
    /// A new browser tab in workspace `ws`'s panel, on `url` or empty, the panel opened on it; `focus_address` gives its
    /// address field the keyboard. A page that can't be made leaves no tab behind.
    pub(super) fn new_browser_tab(&mut self, ws: String, url: Option<String>, focus_address: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.browser.made += 1;
        let id = format!("{PREFIX}{}", self.browser.made);
        self.browser.tabs.push(BrowserTab { id: id.clone(), workspace: ws.clone(), url: url.unwrap_or_else(|| BLANK.into()) });
        if !self.ensure_page(&id, window, cx) {
            self.browser.tabs.retain(|t| t.id != id);
            return;
        }
        self.right.panels.entry(ws).or_default().add(Surface::Browser(id.clone()));
        if focus_address {
            self.focus_panel(true, window, cx);
            let input = self.address(&id, window, cx);
            input.update(cx, |input, cx| input.focus(window, cx));
        }
        cx.notify();
    }

    /// Closes browser tab `id` and its panel's tab. Its page goes in `sync_pages`.
    pub(super) fn close_browser_tab(&mut self, id: &str, cx: &mut Context<Self>) {
        let surface = Surface::Browser(id.to_string());
        for panel in self.right.panels.values_mut() {
            panel.close(&surface);
        }
        self.browser.tabs.retain(|t| t.id != id);
        cx.notify();
    }

    /// Once per render, before the pages sync: a link clicked in a terminal opens its tab, the keyboard staying in the
    /// terminal; workspaces gone take their tabs and panels along.
    pub(super) fn sync_browser(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(url) = self.browser.pending.take() {
            match self.shown_tab() {
                Some(tab) => self.new_browser_tab(tab.workspace_id, Some(url), false, window, cx),
                None => cx.open_url(&url),
            }
        }
        let Some(s) = self.snapshot.as_ref().filter(|s| !s.workspaces.is_empty()) else { return };
        // ponytail: another place shown (a session, a machine) closes this one's browser tabs; keyed by place if missed
        self.browser.tabs.retain(|t| s.workspaces.iter().any(|w| w.workspace_id == t.workspace));
        self.right.panels.retain(|ws, _| s.workspaces.iter().any(|w| w.workspace_id == *ws));
        for (ws, panel) in &mut self.right.panels {
            let live: Vec<&str> = self.browser.tabs.iter().filter(|t| t.workspace == *ws).map(|t| t.id.as_str()).collect();
            panel.retain_browsers(&live);
        }
    }

    /// Tab `id`'s label: its page's title, else its site, else New Tab.
    pub(super) fn browser_label(&self, id: &str) -> String {
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

    /// The panel's browser tab's address field takes the keyboard, its text selected; a sidebar page has none.
    pub(super) fn focus_address(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, _)) = self.panel_page(window) else { return };
        self.focus_panel(true, window, cx);
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

    /// Tab `id`'s site icon once fetched, and whether its page is loading.
    pub(super) fn browser_chip(&self, id: &str) -> (Option<std::sync::Arc<gpui::Image>>, bool) {
        let key = (id.to_string(), None);
        let icon = match self.pages.favicons.get(&key) {
            Some((_, Favicon::Image(image))) => Some(image.clone()),
            _ => None,
        };
        (icon, self.pages.live.get(&key).is_some_and(|p| p.loading))
    }

    /// The active browser tab's row over its page: back, forward, reload, the address and Open in Browser.
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
            // the address field's Esc ends a pick too
            .capture_key_down(cx.listener({
                let key = key.clone();
                move |this, e: &gpui::KeyDownEvent, _, cx| {
                    if e.keystroke.key == "escape" && this.annotating(&key) {
                        cx.stop_propagation();
                        this.stop_annotation(&key);
                        cx.notify();
                    }
                }
            }))
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
            .child(self.annotate_button(key, theme, cx))
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
}
