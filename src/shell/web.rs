//! Web integrations' pages: an entry clicked in the sidebar shows its page in the system web view (wry) in place of
//! the tabs and panes. A page lives until Shepherd quits or its entry goes; hidden, it keeps its state.
use super::*;
use super::tab_bar::{TAB_H, TAB_TOP};
use crate::settings::WebEntry;
use crate::web::{listed, normalize, off_site, page_bounds, page_is_current};
use futures::StreamExt;
use futures::channel::mpsc::UnboundedSender;
use wry::dpi::{LogicalPosition, LogicalSize};

/// A page by its entry's id and, for one opened on a machine, that machine's SSH target.
pub(super) type PageKey = (String, Option<String>);

/// What a web view reports, sent from its handlers (which run without a `cx`) to the Shell.
pub(super) enum PageEvent {
    Title(String),
    /// A load started or finished: back and forward may have changed.
    Loaded,
    /// A new window asked for: it loads in the page instead.
    Navigate(String),
    /// A link to another site, for the system browser.
    Outside(String),
}

pub(super) struct Page {
    view: Rc<wry::WebView>,
    /// The normalized URL of the entry it was made for, so an edited URL makes a new page.
    entry_url: String,
    title: Option<String>,
    /// Whether it was last made visible, so it isn't told every frame.
    visible: bool,
    /// Where it was last placed, so it isn't moved every frame.
    placed: Rc<Cell<Option<Bounds<Pixels>>>>,
}

/// The live pages and which one is shown.
#[derive(Default)]
pub(super) struct Pages {
    /// The entry shown in place of the tabs and panes; `None` shows the tabs and panes.
    pub(super) shown: Option<String>,
    live: HashMap<PageKey, Page>,
    /// Where pages keep their cookies and storage between launches; macOS keeps WKWebView's default store.
    #[cfg(not(target_os = "macos"))]
    context: Option<wry::WebContext>,
    events: Option<UnboundedSender<(PageKey, PageEvent)>>,
    /// The setting that sends links to other sites to the browser, as the pages' handlers read it.
    links_outside: Rc<Cell<bool>>,
    /// `SHEPHERD_WEB` was looked at.
    hooked: bool,
}

impl Shell {
    /// The key of the place shown in the bar (`Place::key`), once herdr has listed the places.
    pub(super) fn shown_place_key(&self) -> Option<String> {
        self.places.iter().find(|p| self.places_shown(p)).map(sessions::Place::key)
    }

    /// The entries the sidebar lists for the place shown.
    fn listed_entries(&self, cx: &App) -> Vec<WebEntry> {
        listed(&SettingsGlobal::of(cx).settings.web_entries, self.shown_place_key().as_deref()).into_iter().cloned().collect()
    }

    /// The entry shown, with its page's key, while it is listed for the place shown.
    pub(super) fn shown_page(&self, cx: &App) -> Option<(PageKey, WebEntry)> {
        let id = self.pages.shown.as_ref()?;
        let entry = self.listed_entries(cx).into_iter().find(|e| &e.id == id)?;
        Some((self.page_key(&entry), entry))
    }

    fn page_key(&self, entry: &WebEntry) -> PageKey {
        (entry.id.clone(), None)
    }

    /// Whether anything Shepherd draws over the main area is open, sliding out included: the page, which sits above
    /// all of it, hides meanwhile. A new popup field belongs here.
    pub(super) fn overlay_open(&self, cx: &App) -> bool {
        let popups = [
            self.view_menu.is_some(),
            self.agents_menu.is_some(),
            self.ws_menu.is_some(),
            self.pane_menu.is_some(),
            self.spent_menu.is_some(),
            self.usage_menu.is_some(),
            self.statusbar_menu.is_some(),
            self.tab_menu.is_some(),
            self.worktree_dialog.is_some(),
            self.remove_confirm.is_some(),
            self.merge_confirm.is_some(),
            self.session_dialog.is_some(),
            self.place_menu.is_some(),
            self.place_edit.is_some(),
            self.session_delete.is_some(),
            self.command_panel.is_some(),
        ];
        popups.into_iter().any(|open| open)
            || self.settings.is_some()
            || self.floating_open()
            || self.peek_now() > 0.
            || crate::ui::toast::any_open(cx)
    }

    /// Every user selection of a tab goes through here; it leaves the page for the tabs.
    pub(super) fn select_tab(&mut self, tab: Option<String>) {
        self.selected_tab = tab;
        self.pages.shown = None;
    }

    /// The entry clicked in the sidebar: its page shown (made on first show); on Wayland, where a web view can't sit in
    /// Shepherd's window, its URL in the browser.
    pub(super) fn show_web(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = SettingsGlobal::of(cx).settings.web_entries.iter().find(|e| e.id == id).cloned() else { return };
        let url = normalize(&entry.url);
        use wry::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if matches!(HasWindowHandle::window_handle(window).map(|h| h.as_raw()), Ok(RawWindowHandle::Wayland(_))) {
            cx.open_url(&url);
            return;
        }
        let key = self.page_key(&entry);
        if !self.pages.live.contains_key(&key) {
            match self.make_page(key.clone(), &url, window, cx) {
                Ok(page) => {
                    self.pages.live.insert(key, page);
                }
                Err(e) => {
                    crate::ui::toast::error(cx, format!("{}: {e}", entry.name));
                    return;
                }
            }
        }
        self.pages.shown = Some(id);
        cx.notify();
    }

    fn make_page(&mut self, key: PageKey, url: &str, window: &Window, cx: &mut Context<Self>) -> wry::Result<Page> {
        let events = self.page_events(cx);
        let send = move |event: PageEvent| {
            let _ = events.unbounded_send((key.clone(), event));
        };
        let (title, loaded, opened, followed) = (send.clone(), send.clone(), send.clone(), send);
        // a link to another site goes to the browser when the setting says so; anything else stays in the page
        let (site, flag) = (url.to_string(), self.pages.links_outside.clone());
        let follows_outside: Rc<dyn Fn(&str) -> bool> = Rc::new(move |to| flag.get() && off_site(&site, to));
        let opens_outside = follows_outside.clone();
        #[cfg(not(target_os = "macos"))]
        let context = self.pages.context.get_or_insert_with(|| wry::WebContext::new(Some(crate::settings::cache_dir().join("web"))));
        #[cfg(not(target_os = "macos"))]
        let builder = wry::WebViewBuilder::new_with_web_context(context);
        #[cfg(target_os = "macos")]
        let builder = wry::WebViewBuilder::new();
        let view = builder
            .with_url(url)
            .with_visible(false)
            .with_bounds(wry::Rect { position: LogicalPosition::new(0., 0.).into(), size: LogicalSize::new(1., 1.).into() })
            .with_document_title_changed_handler(move |t| title(PageEvent::Title(t)))
            .with_on_page_load_handler(move |_, _| loaded(PageEvent::Loaded))
            .with_navigation_handler(move |to| {
                let stay = !follows_outside(&to);
                if !stay {
                    followed(PageEvent::Outside(to));
                }
                stay
            })
            .with_new_window_req_handler(move |to, _| {
                let event = if opens_outside(&to) { PageEvent::Outside(to) } else { PageEvent::Navigate(to) };
                opened(event);
                wry::NewWindowResponse::Deny
            })
            .build_as_child(window)?;
        Ok(Page { view: Rc::new(view), entry_url: url.to_string(), title: None, visible: false, placed: Default::default() })
    }

    /// The channel the pages' handlers report on; its first use starts the loop that applies what they say.
    fn page_events(&mut self, cx: &mut Context<Self>) -> UnboundedSender<(PageKey, PageEvent)> {
        if let Some(events) = &self.pages.events {
            return events.clone();
        }
        let (events, mut said) = futures::channel::mpsc::unbounded::<(PageKey, PageEvent)>();
        cx.spawn(async move |this, cx| {
            while let Some((key, event)) = said.next().await {
                if this.update(cx, |s, cx| s.page_event(key, event, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        self.pages.events = Some(events.clone());
        events
    }

    fn page_event(&mut self, key: PageKey, event: PageEvent, cx: &mut Context<Self>) {
        let Some(page) = self.pages.live.get_mut(&key) else { return };
        match event {
            PageEvent::Title(title) => page.title = Some(title).filter(|t| !t.is_empty()),
            PageEvent::Loaded => {}
            PageEvent::Outside(url) => cx.open_url(&url),
            PageEvent::Navigate(url) => {
                if let Err(e) = page.view.load_url(&url) {
                    crate::ui::toast::error(cx, format!("{url}: {e}"));
                }
            }
        }
        cx.notify();
    }

    /// Once per render: `SHEPHERD_WEB` shown at launch; pages whose entry went or whose URL changed dropped; the shown
    /// one cleared when the place shown doesn't list it; only the shown page visible, and not while `covered`.
    pub(super) fn sync_pages(&mut self, covered: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !std::mem::replace(&mut self.pages.hooked, true)
            && let Ok(name) = std::env::var("SHEPHERD_WEB")
            && let Some(entry) = SettingsGlobal::of(cx).settings.web_entries.iter().find(|e| e.name == name)
        {
            self.show_web(entry.id.clone(), window, cx);
        }
        self.pages.links_outside.set(SettingsGlobal::of(cx).settings.web_links_outside);
        let entries = SettingsGlobal::of(cx).settings.web_entries.clone();
        self.pages.live.retain(|(id, _), page| page_is_current(&entries, id, &page.entry_url));
        let shown = self.shown_page(cx).map(|(key, _)| key);
        if shown.is_none() {
            self.pages.shown = None;
        }
        for (key, page) in &mut self.pages.live {
            let visible = shown.as_ref() == Some(key) && !covered;
            if page.visible == visible {
                continue;
            }
            page.visible = visible;
            let _ = page.view.set_visible(visible);
            // the keyboard goes with the page, and back to Shepherd when it hides
            let _ = if visible { page.view.focus() } else { page.view.focus_parent() };
        }
    }

    /// The page area, in place of the pane grid: a canvas that places the shown page's web view over itself.
    pub(super) fn render_web_area(&self, key: &PageKey, theme: &Theme) -> Div {
        let page = self.pages.live.get(key).map(|p| (p.view.clone(), p.placed.clone()));
        let sidebar_closed = !self.sidebar_open;
        div().flex_1().min_h_0().relative().bg(theme.terminal.background).when_some(page, |el, (view, placed)| {
            el.child(
                canvas(
                    move |area, _, _| {
                        let bounds = page_bounds(area, sidebar_closed);
                        if placed.get() != Some(bounds) {
                            placed.set(Some(bounds));
                            let _ = view.set_bounds(wry::Rect {
                                position: LogicalPosition::new(f32::from(bounds.origin.x), f32::from(bounds.origin.y)).into(),
                                size: LogicalSize::new(f32::from(bounds.size.width), f32::from(bounds.size.height)).into(),
                            });
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
        })
    }

    /// The tab bar's row while a page is shown: back, forward, reload and the page's title (the entry's name until
    /// the page has one). No tooltips: they would open down over the page, under it.
    pub(super) fn render_web_bar(&self, key: &PageKey, entry: &WebEntry, theme: &Theme, sidebar_w: f32, cx: &mut Context<Self>) -> Div {
        let page = self.pages.live.get(key);
        let view = page.map(|p| p.view.clone());
        let can = |f: fn(&wry::WebView) -> wry::Result<bool>| view.as_deref().is_some_and(|v| f(v).unwrap_or(false));
        let title = page.and_then(|p| p.title.clone()).unwrap_or_else(|| entry.name.clone());
        let hover = theme.hover;
        let button = |id: &'static str, icon: &'static str, enabled: bool, act: fn(&wry::WebView) -> wry::Result<()>| {
            let view = view.clone();
            div()
                .id(id)
                .size(px(TAB_H))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.))
                .block_mouse_except_scroll()
                .when(cfg!(windows), |el| el.occlude()) // off the drag strip, as the tabs
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .child(gpui::svg().path(icon).size(px(16.)).text_color(if enabled { theme.text_muted } else { theme.text_faint }))
                .when(enabled, |el| {
                    el.cursor_pointer().hover(move |s| s.bg(hover)).on_click(move |_, _, _| {
                        if let Some(view) = &view {
                            let _ = act(view);
                        }
                    })
                })
        };
        div()
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            .bg(theme.surface)
            .flex()
            .items_center()
            .gap(px(2.))
            .pr(px(TAB_TOP + if self.review_open { 0. } else { window_controls_w(cx) }))
            .pl(px((TITLEBAR_CLUSTER - sidebar_w).max(8.)))
            .child(button("web-back", "icons/solar/alt-arrow-left.svg", can(wry::WebView::can_go_back), wry::WebView::go_back))
            .child(button("web-forward", "icons/solar/alt-arrow-right.svg", can(wry::WebView::can_go_forward), wry::WebView::go_forward))
            .child(button("web-reload", "icons/solar/restart.svg", view.is_some(), wry::WebView::reload))
            .child(div().ml(px(6.)).flex_1().min_w_0().truncate().text_color(theme.text_muted).child(title))
    }

    /// The sidebar's rows of the entries listed for the place shown, above the workspaces, with a hairline under them.
    pub(super) fn web_rows(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<Div> {
        let entries = self.listed_entries(cx);
        if entries.is_empty() {
            return None;
        }
        let shown = self.pages.shown.clone();
        let rows = entries.into_iter().map(|e| {
            let selected = shown.as_deref() == Some(e.id.as_str());
            let id = e.id.clone();
            crate::shell::row(theme, selected, SharedString::from(format!("web-{}", e.id)))
                .child(div().size(px(16.)).flex_none().flex().items_center().justify_center().child(sessions::place_mark(&e.name, None, false, theme.text_muted)))
                .child(div().flex_1().min_w_0().truncate().child(e.name.clone()))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.show_web(id.clone(), window, cx)))
        });
        Some(
            div()
                .flex_none()
                .px(px(8.))
                .pt(px(8.))
                .flex()
                .flex_col()
                .gap(px(2.))
                .children(rows)
                .child(div().mt(px(6.)).mx(px(8.)).h(px(1.)).bg(theme.border)),
        )
    }
}
