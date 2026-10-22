//! Web integrations' pages: an entry clicked in the sidebar shows its page in the system web view (wry) in place of
//! the tabs and panes. A page lives until Shepherd quits or its entry goes; hidden, it keeps its state.
use super::*;
use super::tab_bar::{TAB_H, TAB_TOP};
use crate::settings::{OpenFrom, WebEntry};
use crate::web::{forwarded, label, listed, normalize, off_site, page_bounds, page_is_current};
use futures::StreamExt;
use std::sync::Arc;
use futures::channel::mpsc::UnboundedSender;
use wry::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};

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
    /// One of Shepherd's shortcuts, pressed in the page.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Key(gpui::Keystroke),
}

pub(super) struct Page {
    view: Rc<wry::WebView>,
    /// Told apart from an earlier page under its key, whose events may still be on their way.
    serial: u64,
    /// The normalized URL of the entry it was made for, so an edited URL makes a new page.
    entry_url: String,
    title: Option<String>,
    /// Whether it was last made visible, so it isn't told every frame.
    visible: bool,
    /// Whether it was last given the keyboard.
    focused: bool,
    /// Where it was last placed, so it isn't moved every frame.
    placed: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// On macOS, the view of its own the page sits in, under GPUI's overlay plane.
    #[cfg(target_os = "macos")]
    host: Option<Rc<host::Host>>,
    /// The SSH forward a page opened on a machine loads through; it closes with the page.
    tunnel: Option<crate::herdr::Tunnel>,
}

/// A machine's page whose forward isn't up: being made, or why it couldn't be; with the entry's URL it is for.
pub(super) enum Forward {
    Connecting,
    Failed(String),
}

/// An entry's sidebar icon.
pub(super) enum Favicon {
    Loading,
    Image(Arc<gpui::Image>),
    /// None to be had: the name's first letter stands in.
    None,
}

/// The live pages and which one is shown.
#[derive(Default)]
pub(super) struct Pages {
    /// The entry shown in place of the tabs and panes; `None` shows the tabs and panes.
    pub(super) shown: Option<String>,
    live: HashMap<PageKey, Page>,
    /// Pages opened on a machine still waiting for their forward, or whose forward failed.
    forwards: HashMap<PageKey, (String, Forward)>,
    /// Each page's favicon (a machine's page has its own) and the URL it was fetched for; in memory only, so fetched
    /// again each launch.
    favicons: HashMap<PageKey, (String, Favicon)>,
    /// Where pages keep their cookies and storage between launches; macOS keeps WKWebView's default store.
    #[cfg(not(target_os = "macos"))]
    context: Option<wry::WebContext>,
    events: Option<UnboundedSender<(PageKey, u64, PageEvent)>>,
    /// The last page's serial.
    made: u64,
    /// The setting that sends links to other sites to the browser, as the pages' handlers read it.
    links_outside: Rc<Cell<bool>>,
    /// The sidebar's rows of them, as dragged to reorder.
    reorder: Rc<std::cell::RefCell<reorder::Reorder>>,
    /// `SHEPHERD_WEB`'s entry was shown.
    hooked: bool,
    /// GPUI draws its deferred layer (menus, dialogs, panels, toasts) above the pages, so they needn't hide for it.
    layered: bool,
    /// Shepherd's chords in force, which the pages' key monitors take from the page.
    #[cfg(target_os = "macos")]
    shortcuts: Rc<std::cell::RefCell<Vec<String>>>,
    /// GTK's loop is pumped beside GPUI's.
    #[cfg(target_os = "linux")]
    gtk_pump: bool,
}

impl Shell {
    /// The key of the place shown in the bar (`Place::key`), once herdr has listed the places.
    pub(super) fn shown_place_key(&self) -> Option<String> {
        self.places.iter().find(|p| self.places_shown(p)).map(sessions::Place::key)
    }

    /// The entries the sidebar lists for the place shown.
    fn listed_entries(&self, cx: &App) -> Vec<WebEntry> {
        self.entries_in(self.shown_place_key().as_deref(), cx)
    }

    /// The entries the sidebar lists for the place with key `place`.
    fn entries_in(&self, place: Option<&str>, cx: &App) -> Vec<WebEntry> {
        listed(&SettingsGlobal::of(cx).settings.web_entries, place).into_iter().cloned().collect()
    }

    /// Fetches the favicons of the entries listed that have none yet, or whose URL changed.
    fn fetch_favicons(&mut self, cx: &mut Context<Self>) {
        for entry in self.listed_entries(cx) {
            // a page opened on a machine fetches its icon through its forward, once that is up
            if entry.open_from == OpenFrom::Machine {
                continue;
            }
            let url = normalize(&entry.url);
            let key = self.page_key(&entry);
            if self.pages.favicons.get(&key).is_some_and(|(fetched, _)| *fetched == url) {
                continue;
            }
            self.fetch_favicon(key, url.clone(), url, cx);
        }
    }

    /// Fetches page `key`'s favicon from `from` (its URL, or its forward's), kept as the icon of its URL `url`.
    fn fetch_favicon(&mut self, key: PageKey, url: String, from: String, cx: &mut Context<Self>) {
        self.pages.favicons.insert(key.clone(), (url.clone(), Favicon::Loading));
        let icon = cx.background_spawn(async move { crate::web::fetch_favicon(&from) });
        cx.spawn(async move |this, cx| {
            let icon = icon.await;
            let _ = this.update(cx, |s, cx| {
                // an edit since asked for another URL's icon
                let Some((_, slot)) = s.pages.favicons.get_mut(&key).filter(|(u, _)| *u == url) else { return };
                *slot = match icon {
                    Some((format, bytes)) => Favicon::Image(Arc::new(gpui::Image::from_bytes(format, bytes))),
                    None => Favicon::None,
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// The entry shown, with its page's key, while it is listed for the place shown.
    pub(super) fn shown_page(&self, cx: &App) -> Option<(PageKey, WebEntry)> {
        let id = self.pages.shown.as_ref()?;
        let entry = self.listed_entries(cx).into_iter().find(|e| &e.id == id)?;
        Some((self.page_key(&entry), entry))
    }

    /// An entry opened on the machine shown keys its page by that machine; anything else (a local session shown,
    /// or opened from this computer) by the entry alone.
    fn page_key(&self, entry: &WebEntry) -> PageKey {
        let machine = (entry.open_from == OpenFrom::Machine).then(crate::herdr::shown_machine).flatten();
        (entry.id.clone(), machine)
    }

    /// Whether anything Shepherd draws over the main area is open, sliding out included, that the page has to hide
    /// for: Settings, which takes the window; and where GPUI can't draw above native views (off macOS), every popup
    /// too, the page sitting above all of it.
    pub(super) fn overlay_open(&self, cx: &App) -> bool {
        self.settings.is_some()
            || (!self.pages.layered && (self.popup_open() || self.peek_now() > 0. || crate::ui::toast::any_open(cx)))
    }

    /// Whether a menu, dialog or panel is open, which takes the keyboard from the page. A new popup field belongs here.
    fn popup_open(&self) -> bool {
        let popups = [
            self.view_menu.is_some(),
            self.agents_menu.is_some(),
            self.ws_menu.is_some(),
            self.web_menu.is_some(),
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
        popups.into_iter().any(|open| open) || self.floating_open()
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
        #[cfg(target_os = "linux")]
        if let Err(e) = self.gtk_ready(cx) {
            crate::ui::toast::error(cx, format!("Web pages need GTK ({e}): {url} opens in the browser"));
            cx.open_url(&url);
            return;
        }
        let key = self.page_key(&entry);
        if let (None, Some(target)) = (self.pages.live.get(&key), key.1.clone()) {
            // a failed forward is tried again by Retry, not by every click
            if !self.pages.forwards.contains_key(&key) {
                self.open_forwarded(key, url, target, window, cx);
            }
        } else if !self.pages.live.contains_key(&key) {
            match self.make_page(key.clone(), &url, &url, window, cx) {
                Ok(page) => {
                    self.pages.live.insert(key, page);
                }
                Err(e) => {
                    crate::ui::toast::error(cx, format!("{}: {e}", label(&entry, None)));
                    // the tabs again, or the next render would try once more, and again
                    self.pages.shown = None;
                    return;
                }
            }
        }
        self.pages.shown = Some(id);
        cx.notify();
    }

    /// Forwards the entry's port from `target` (off the main thread), then makes its page on the forward; or keeps
    /// why it couldn't, for the page area to show.
    fn open_forwarded(&mut self, key: PageKey, url: String, target: String, window: &Window, cx: &mut Context<Self>) {
        let Some(((host, port), _)) = forwarded(&url, 0) else {
            let why = format!("{url} is not an http or https URL");
            self.pages.forwards.insert(key, (url, Forward::Failed(why)));
            return;
        };
        self.pages.forwards.insert(key.clone(), (url.clone(), Forward::Connecting));
        let local = crate::web::forward_port(&key.0, &target);
        let tunnel = cx.background_spawn(async move {
            let tunnel = crate::herdr::forward(&target, &host, port, local)?;
            match crate::web::answers(tunnel.port) {
                true => Ok(tunnel),
                false => Err(std::io::Error::other(format!("Nothing answers on {host}:{port} there."))),
            }
        });
        cx.spawn_in(window, async move |this, cx| {
            let tunnel = tunnel.await;
            let _ = this.update_in(cx, |s, window, cx| {
                // the entry went, or the attempt was dropped, meanwhile
                if !matches!(s.pages.forwards.get(&key), Some((for_url, Forward::Connecting)) if *for_url == url) {
                    return;
                }
                let made = tunnel.map_err(|e| e.to_string()).and_then(|tunnel| {
                    let Some((_, load)) = forwarded(&url, tunnel.port) else { return Err(format!("{url}: no host")) };
                    let mut page = s.make_page(key.clone(), &url, &load, window, cx).map_err(|e| e.to_string())?;
                    s.fetch_favicon(key.clone(), url.clone(), load, cx);
                    page.tunnel = Some(tunnel);
                    Ok(page)
                });
                match made {
                    Ok(page) => {
                        s.pages.forwards.remove(&key);
                        s.pages.live.insert(key.clone(), page);
                        s.watch_tunnel(key.clone(), cx);
                    }
                    Err(why) => {
                        s.pages.forwards.insert(key.clone(), (url.clone(), Forward::Failed(why)));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Looks at the page's forward every few seconds while the page lives: once its ssh has gone, a render lets
    /// `sync_pages` show why, with Retry.
    fn watch_tunnel(&self, key: PageKey, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(3)).await;
                let Ok(alive) = this.update(cx, |s, cx| {
                    let alive = s.pages.live.get_mut(&key).and_then(|p| p.tunnel.as_mut()).map(|t| t.alive());
                    if alive == Some(false) {
                        cx.notify();
                    }
                    alive == Some(true)
                }) else {
                    break;
                };
                if !alive {
                    break;
                }
            }
        })
        .detach();
    }

    /// `entry_url` is the entry's URL, which keeps the page while it is the same; `url` is the one loaded, the
    /// forward's for a page on a machine, and the site its links are measured against.
    fn make_page(&mut self, key: PageKey, entry_url: &str, url: &str, window: &Window, cx: &mut Context<Self>) -> wry::Result<Page> {
        let events = self.page_events(window, cx);
        self.pages.made += 1;
        let serial = self.pages.made;
        let send = move |event: PageEvent| {
            let _ = events.unbounded_send((key.clone(), serial, event));
        };
        let (title, loaded, opened, followed) = (send.clone(), send.clone(), send.clone(), send.clone());
        #[cfg(target_os = "macos")]
        let key_event = send;
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
            .with_devtools(true)
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
        // GPUI's deferred layer goes on a plane of its own above native views (macOS only, in this GPUI); the page
        // moves into a view of its own below it. Without it, the page hides under Shepherd's overlays instead.
        #[cfg(target_os = "macos")]
        let host = window
            .enable_scene_overlay()
            .ok()
            .and_then(|()| host::Host::new(&view, self.pages.shortcuts.clone(), move |k| key_event(PageEvent::Key(k))))
            .map(Rc::new);
        #[cfg(target_os = "macos")]
        {
            self.pages.layered = host.is_some();
        }
        Ok(Page {
            view: Rc::new(view),
            serial,
            entry_url: entry_url.to_string(),
            title: None,
            visible: false,
            focused: false,
            placed: Default::default(),
            #[cfg(target_os = "macos")]
            host,
            tunnel: None,
        })
    }

    /// WebKitGTK needs GTK set up on this thread, once, and its loop run: here a few iterations every frame's worth
    /// of time, beside GPUI's loop, while any page lives or waits for its forward.
    #[cfg(target_os = "linux")]
    fn gtk_ready(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        static INIT: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
        // GPUI is on X11 here (a Wayland window opens pages in the browser); GDK would take Wayland when it is there too,
        // and the page couldn't be put in GPUI's X11 window
        INIT.get_or_init(|| {
            gtk::gdk::set_allowed_backends("x11");
            gtk::init().map_err(|e| e.to_string())
        })
        .clone()?;
        if !std::mem::replace(&mut self.pages.gtk_pump, true) {
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(Duration::from_millis(16)).await;
                    while gtk::events_pending() {
                        gtk::main_iteration_do(false);
                    }
                    let Ok(live) = this.update(cx, |s, _| {
                        let live = !s.pages.live.is_empty() || !s.pages.forwards.is_empty();
                        s.pages.gtk_pump = live;
                        live
                    }) else {
                        break;
                    };
                    if !live {
                        break;
                    }
                }
            })
            .detach();
        }
        Ok(())
    }

    /// The channel the pages' handlers report on; its first use starts the loop that applies what they say.
    fn page_events(&mut self, window: &Window, cx: &mut Context<Self>) -> UnboundedSender<(PageKey, u64, PageEvent)> {
        if let Some(events) = &self.pages.events {
            return events.clone();
        }
        let (events, mut said) = futures::channel::mpsc::unbounded::<(PageKey, u64, PageEvent)>();
        cx.spawn_in(window, async move |this, cx| {
            while let Some((key, serial, event)) = said.next().await {
                if this.update_in(cx, |s, window, cx| s.page_event(key, serial, event, window, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        self.pages.events = Some(events.clone());
        events
    }

    fn page_event(&mut self, key: PageKey, serial: u64, event: PageEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.pages.live.get_mut(&key).filter(|p| p.serial == serial) else { return };
        match event {
            PageEvent::Title(title) => page.title = Some(title).filter(|t| !t.is_empty()),
            PageEvent::Loaded => {}
            PageEvent::Outside(url) => cx.open_url(&url),
            // dispatched as if pressed in Shepherd; a menu or dialog it opens takes the keyboard in `sync_pages`
            PageEvent::Key(keystroke) => window.defer(cx, move |window, cx| {
                window.dispatch_keystroke(keystroke, cx);
            }),
            PageEvent::Navigate(url) => {
                if let Err(e) = page.view.load_url(&url) {
                    crate::ui::toast::error(cx, format!("{url}: {e}"));
                }
            }
        }
        cx.notify();
    }

    /// Once per render: `SHEPHERD_WEB` shown at launch, once the place shown lists it; pages whose entry went or whose URL changed dropped; the shown
    /// one cleared when the place shown doesn't list it; only the shown page visible, and not while `covered`.
    pub(super) fn sync_pages(&mut self, covered: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.pages.hooked
            && let Ok(name) = std::env::var("SHEPHERD_WEB")
            && !name.is_empty()
            && let Some(entry) = self.listed_entries(cx).into_iter().find(|e| e.name == name)
        {
            self.pages.hooked = true;
            let key = self.page_key(&entry);
            self.show_web(entry.id, window, cx);
            // `SHEPHERD_WEB_INSPECT` opens its inspector too, as Inspect Element does (screenshots)
            if std::env::var_os("SHEPHERD_WEB_INSPECT").is_some()
                && let Some(page) = self.pages.live.get(&key)
            {
                page.view.open_devtools();
            }
        }
        self.pages.links_outside.set(SettingsGlobal::of(cx).settings.web_links_outside);
        #[cfg(target_os = "macos")]
        if let Some(active) = cx.try_global::<crate::bindings::ActiveBindings>() {
            let chords: Vec<String> = active.0.iter().map(|(_, chord)| chord.clone()).collect();
            if *self.pages.shortcuts.borrow() != chords {
                *self.pages.shortcuts.borrow_mut() = chords;
            }
        }
        self.fetch_favicons(cx);
        let entries = SettingsGlobal::of(cx).settings.web_entries.clone();
        // a page on a machine also goes when its entry no longer opens there
        let still_opened = |id: &String, machine: &Option<String>| {
            machine.is_none() || entries.iter().any(|e| &e.id == id && e.open_from == OpenFrom::Machine)
        };
        self.pages.live.retain(|(id, machine), page| page_is_current(&entries, id, &page.entry_url) && still_opened(id, machine));
        self.pages.forwards.retain(|(id, machine), (url, _)| page_is_current(&entries, id, url) && still_opened(id, machine));
        // a page whose forward closed shows why, with Retry
        let closed: Vec<PageKey> =
            self.pages.live.iter_mut().filter_map(|(k, p)| p.tunnel.as_mut().is_some_and(|t| !t.alive()).then(|| k.clone())).collect();
        for key in closed {
            if let Some(page) = self.pages.live.remove(&key) {
                let why = format!("The connection to {} closed.", key.1.as_deref().unwrap_or_default());
                self.pages.forwards.insert(key, (page.entry_url, Forward::Failed(why)));
            }
        }
        let shown = self.shown_page(cx).map(|(key, _)| key);
        if shown.is_none() {
            self.pages.shown = None;
        }
        // the page shown has no web view and no forward under its key now (the machine shown changed under it): one is
        // made, as a click makes it
        if let Some(key) = &shown
            && !self.pages.live.contains_key(key)
            && !self.pages.forwards.contains_key(key)
        {
            self.show_web(key.0.clone(), window, cx);
        }
        let popup = self.popup_open();
        for (key, page) in &mut self.pages.live {
            let visible = shown.as_ref() == Some(key) && !covered;
            if page.visible != visible {
                page.visible = visible;
                let _ = page.view.set_visible(visible);
                #[cfg(target_os = "macos")]
                if let Some(host) = &page.host {
                    host.show(visible);
                }
            }
            // the keyboard goes with the page, to a menu or dialog open over it, and back to Shepherd when it hides
            let focused = visible && !popup;
            if page.focused != focused {
                page.focused = focused;
                let _ = if focused { page.view.focus() } else { page.view.focus_parent() };
            }
        }
    }

    /// The page area, in place of the pane grid: a canvas that places the shown page's web view over itself.
    pub(super) fn render_web_area(&self, key: &PageKey, theme: &Theme, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let page = self.pages.live.get(key).map(|p| {
            #[cfg(target_os = "macos")]
            let host = p.host.clone();
            #[cfg(not(target_os = "macos"))]
            let host: Option<Rc<()>> = None;
            (p.view.clone(), p.placed.clone(), host)
        });
        let sidebar_closed = !self.sidebar_open;
        let target = key.1.clone().unwrap_or_default();
        // a page on a machine before its forward is up: connecting, or why it couldn't and a retry
        let waiting = match self.pages.forwards.get(key).map(|(_, f)| f) {
            Some(Forward::Connecting) => Some(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(12.))
                    .child(crate::ui::loader::spinner(2.5, window))
                    .child(div().text_size(px(12.)).text_color(theme.text_muted).child(format!("Connecting to {target}…"))),
            ),
            Some(Forward::Failed(why)) => {
                let (id, retry_key) = (key.0.clone(), key.clone());
                let hover = theme.wash(0.10);
                Some(
                    div()
                        .max_w(px(480.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(12.))
                        .child(div().text_color(theme.text).child(format!("Couldn't reach {target}")))
                        .child(div().text_size(px(12.)).text_color(theme.text_muted).text_center().child(why.clone()))
                        .child(
                            div()
                                .id("web-retry")
                                .h(px(32.))
                                .px(px(12.))
                                .flex()
                                .items_center()
                                .rounded(px(8.))
                                .bg(theme.wash(0.06))
                                .cursor_pointer()
                                .hover(move |s| s.bg(hover))
                                .child("Retry")
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    this.pages.forwards.remove(&retry_key);
                                    this.show_web(id.clone(), window, cx);
                                })),
                        ),
                )
            }
            None => None,
        };
        div()
            .flex_1()
            .min_h_0()
            .relative()
            .bg(theme.terminal.background)
            .when_some(waiting, |el, waiting| el.flex().items_center().justify_center().child(waiting))
            .when_some(page, |el, (view, placed, host)| {
            el.child(
                canvas(
                    move |area, _, _| page_bounds(area, sidebar_closed),
                    move |_, bounds, window, _| {
                        if placed.get() == Some(bounds) {
                            return;
                        }
                        match host.clone() {
                            // moved with the frame that shows the new layout, not before it; a paint that presents
                            // nothing (hit testing only) moves nothing, so the move is noted once it is made
                            #[cfg(target_os = "macos")]
                            Some(host) => {
                                let placed = placed.clone();
                                window.on_present(move || {
                                    host.place(bounds);
                                    placed.set(Some(bounds));
                                })
                            }
                            // in device pixels: wry scales logical ones by GTK's factor, which needn't be GPUI's
                            _ => {
                                placed.set(Some(bounds));
                                let device = |v: Pixels| (f32::from(v) * window.scale_factor()).round() as i32;
                                let _ = view.set_bounds(wry::Rect {
                                    position: PhysicalPosition::new(device(bounds.origin.x), device(bounds.origin.y)).into(),
                                    size: PhysicalSize::new(device(bounds.size.width), device(bounds.size.height)).into(),
                                });
                            }
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
        })
    }

    /// The tab bar's row while a page is shown: back, forward, reload and the page's title (the entry's name until
    /// the page has one, its host until then). No tooltips: they would open down over the page, under it.
    pub(super) fn render_web_bar(&self, key: &PageKey, entry: &WebEntry, theme: &Theme, sidebar_w: f32, cx: &mut Context<Self>) -> Div {
        let page = self.pages.live.get(key);
        let view = page.map(|p| p.view.clone());
        let can = |f: fn(&wry::WebView) -> wry::Result<bool>| view.as_deref().is_some_and(|v| f(v).unwrap_or(false));
        let title = page.and_then(|p| p.title.clone()).unwrap_or_else(|| label(entry, None));
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

    /// A web page's right-click menu in the sidebar, while it is up.
    pub(super) fn web_menu(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let menu = self.web_menu.as_ref()?;
        let (id, at) = menu.value.clone();
        let entry = SettingsGlobal::of(cx).settings.web_entries.iter().find(|e| e.id == id).cloned()?;
        let url = normalize(&entry.url);
        let view = self.pages.live.get(&self.page_key(&entry)).map(|p| p.view.clone());
        type Pick = Box<dyn Fn(&mut Shell, &mut Window, &mut Context<Shell>)>;
        let item = |key: &'static str, label: &'static str, pick: Pick| {
            row(theme, false, key).h(px(28.)).rounded(px(6.)).child(label).on_pick(
                at,
                cx.listener(move |this, _: &ClickEvent, window, cx| {
                    if let Some(m) = this.web_menu.as_mut() {
                        m.close()
                    }
                    pick(this, window, cx);
                    cx.notify();
                }),
            )
        };
        let (copied, removed) = (url.clone(), id.clone());
        let card = menu_card(theme, "web-menu", 160.)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if let Some(m) = this.web_menu.as_mut() {
                    m.close()
                }
                cx.notify();
            }))
            // a page not opened yet has nothing to reload
            .children(view.map(|view| {
                item("web-menu-reload", "Reload", Box::new(move |_, _, _| drop(view.reload())))
            }))
            .child(item("web-menu-browser", "Open in Browser", Box::new(move |_, _, cx| cx.open_url(&url))))
            .child(item(
                "web-menu-copy",
                "Copy URL",
                Box::new(move |_, _, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(copied.clone()))),
            ))
            .child(item("web-menu-edit", "Edit…", Box::new(|this, window, cx| this.open_web_settings(window, cx))))
            .child(div().mx(px(-4.)).my(px(4.)).h(px(1.)).bg(theme.border))
            .child(
                item(
                    "web-menu-remove",
                    "Remove",
                    Box::new(move |_, _, cx| {
                        let id = removed.clone();
                        save_from_sidebar(cx, move |s| s.web_entries.retain(|e| e.id != id))
                    }),
                )
                .text_color(theme.danger),
            );
        let menu = crate::ui::menu_motion("web-menu-in", menu, card);
        Some(
            gpui::deferred(gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(menu))
                .with_priority(1)
                .into_any_element(),
        )
    }

    /// The sidebar's rows of the entries listed for the place shown, above the workspaces, with a hairline under them.
    /// `place`'s rows; `live` for the sidebar itself, dragged to reorder them, not for the place a swipe shows beside
    /// it, whose rows are only drawn.
    pub(super) fn web_rows(&self, place: Option<&str>, live: bool, theme: &Theme, window: &mut Window, cx: &mut Context<Self>) -> Option<Div> {
        let entries = self.entries_in(place, cx);
        if entries.is_empty() {
            return None;
        }
        let shown = self.pages.shown.clone().filter(|_| live);
        let rows: Vec<(String, gpui::AnyElement)> = entries
            .into_iter()
            .map(|e| {
                let key = self.page_key(&e);
                let name = label(&e, self.pages.live.get(&key).and_then(|p| p.title.as_deref()));
                let selected = shown.as_deref() == Some(e.id.as_str());
                let id = e.id.clone();
                let icon = match self.pages.favicons.get(&key) {
                    Some((_, Favicon::Image(image))) => gpui::img(image.clone()).size(px(16.)).rounded(px(3.)).into_any_element(),
                    _ => sessions::place_mark(&name, None, false, theme.text_muted),
                };
                let menu_id = e.id.clone();
                let row = crate::shell::row(theme, selected, SharedString::from(format!("web-{}", e.id)))
                    .h(px(if SettingsGlobal::of(cx).settings.compact_workspaces { 26. } else { 32. }))
                    .child(div().size(px(16.)).flex_none().flex().items_center().justify_center().child(icon))
                    .child(div().flex_1().min_w_0().truncate().child(name))
                    .when(live, |el| {
                        reorder::draggable(el, &self.pages.reorder, &e.id)
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.show_web(id.clone(), window, cx)))
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                                    this.web_menu = Some(crate::ui::Popup::new((menu_id.clone(), e.position)));
                                    cx.notify();
                                }),
                            )
                    });
                (e.id, row.into_any_element())
            })
            .collect();
        let keys: Vec<String> = rows.iter().map(|(k, _)| k.clone()).collect();
        let column = match live {
            true => {
                let (column, dropped) = reorder::column(&self.pages.reorder, rows, 2., theme, window, cx.has_active_drag());
                // the dropped row takes the place of the one it was dropped on
                if let Some((from, to)) = dropped {
                    let (id, to) = (keys[from].clone(), keys[to].clone());
                    cx.defer(move |cx| save_from_sidebar(cx, move |s| crate::web::move_entry(&mut s.web_entries, &id, &to)));
                }
                column
            }
            false => div().flex().flex_col().gap(px(2.)).children(rows.into_iter().map(|(_, row)| row)),
        };
        Some(
            div()
                .flex_none()
                .px(px(8.))
                .pt(px(8.))
                .flex()
                .flex_col()
                .child(column)
                .child(div().mt(px(8.)).mx(px(8.)).h(px(1.)).bg(theme.border)),
        )
    }
}

/// macOS: a page's own view, holding its WKWebView and, docked, WebKit's inspector (which docks into the web view's
/// superview: in GPUI's view it took the whole window). It sits under GPUI's overlay plane, so menus, dialogs and
/// toasts draw over the page.
#[cfg(target_os = "macos")]
mod host {
    use gpui::{Bounds, Pixels};
    use objc2::rc::Retained;
    use objc2::{MainThreadMarker, MainThreadOnly, msg_send};
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSApplication, NSAutoresizingMaskOptions, NSEvent, NSEventMask, NSEventModifierFlags, NSView, NSWindowOrderingMode};
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use std::ptr::NonNull;
    use std::rc::Rc;
    use wry::WebViewExtMacOS;

    /// The page's view, and the key monitor that takes Shepherd's shortcuts from the page while it has the keyboard
    /// (WebKit would keep them, as Zeron's browser found).
    pub(super) struct Host(Retained<NSView>, Option<Retained<AnyObject>>);

    impl Host {
        /// Moves `view`'s WKWebView into a view of its own, just below GPUI's overlay plane; a key among `shortcuts`
        /// pressed in it goes to `key` instead.
        pub(super) fn new(
            view: &wry::WebView,
            shortcuts: Rc<std::cell::RefCell<Vec<String>>>,
            key: impl Fn(gpui::Keystroke) + 'static,
        ) -> Option<Host> {
            let mtm = MainThreadMarker::new()?;
            let web = view.webview();
            // SAFETY: a view's superview, read on the main thread
            let parent = unsafe { web.superview() }?;
            let host = NSView::initWithFrame(NSView::alloc(mtm), NSRect::ZERO);
            host.setAutoresizesSubviews(true);
            host.setHidden(true);
            web.removeFromSuperview();
            web.setFrame(host.bounds());
            web.setAutoresizingMask(NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable);
            host.addSubview(&web);
            let overlay = parent.subviews().iter().find(|v| v.class().name() == c"GPUIOverlayView");
            match overlay {
                Some(overlay) => parent.addSubview_positioned_relativeTo(&host, NSWindowOrderingMode::Below, Some(&overlay)),
                None => parent.addSubview(&host),
            }
            let watched = host.clone();
            let monitor = block2::RcBlock::new(move |event: NonNull<NSEvent>| {
                // SAFETY: AppKit hands the monitor a live event, on the main thread
                let e = unsafe { event.as_ref() };
                let in_page = watched
                    .window()
                    .filter(|w| e.window(mtm).as_ref() == Some(w))
                    .and_then(|w| w.firstResponder())
                    .and_then(|r| r.downcast::<NSView>().ok())
                    .is_some_and(|v| !watched.isHidden() && v.isDescendantOf(&watched));
                let flags = e.modifierFlags();
                let mods = crate::web::Mods {
                    cmd: flags.contains(NSEventModifierFlags::Command),
                    ctrl: flags.contains(NSEventModifierFlags::Control),
                    alt: flags.contains(NSEventModifierFlags::Option),
                    shift: flags.contains(NSEventModifierFlags::Shift),
                };
                // the key as ⌘ types it: Latin in a layout like Russian, whose own letters ⌘ doesn't use (Shift kept,
                // ⌥ left out, as `charactersIgnoringModifiers` would)
                let as_cmd = NSEventModifierFlags::Command | (flags & NSEventModifierFlags::Shift);
                let characters = e.charactersByApplyingModifiers(as_cmd).map(|c| c.to_string()).unwrap_or_default();
                if !in_page {
                    return event.as_ptr();
                }
                if let Some(keystroke) = crate::web::shepherds_key(mods, &characters, &shortcuts.borrow()) {
                    key(keystroke);
                    return std::ptr::null_mut();
                }
                // ⌘C, ⌘V and the rest go to the page's first responder, as an Edit menu's items would send them
                let edit = crate::web::edit_action(mods, &characters).map(objc2::runtime::Sel::register);
                // SAFETY: an action sent up the responder chain, on the main thread
                match edit.filter(|&sel| unsafe { NSApplication::sharedApplication(mtm).sendAction_to_from(sel, None, None) }) {
                    Some(_) => std::ptr::null_mut(),
                    None => event.as_ptr(),
                }
            });
            // SAFETY: the block lives as long as the monitor, which `Drop` removes
            let monitor = unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::KeyDown, &monitor) };
            Some(Host(host, monitor))
        }

        /// Puts it at `bounds`, in GPUI's top-left coordinates; its web view (and a docked inspector) follow.
        pub(super) fn place(&self, bounds: Bounds<Pixels>) {
            // SAFETY: as in `new`
            let Some(parent) = (unsafe { self.0.superview() }) else { return };
            let (x, y) = (f64::from(f32::from(bounds.origin.x)), f64::from(f32::from(bounds.origin.y)));
            let (w, h) = (f64::from(f32::from(bounds.size.width)).max(0.), f64::from(f32::from(bounds.size.height)).max(0.));
            let y = if parent.isFlipped() { y } else { parent.bounds().size.height - y - h };
            // SAFETY: CATransaction's class methods; no implicit animation of the move, which would trail the frame
            unsafe {
                let _: () = msg_send![objc2::class!(CATransaction), setDisableActions: true];
            }
            self.0.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(w, h)));
        }

        pub(super) fn show(&self, visible: bool) {
            self.0.setHidden(!visible);
        }
    }

    impl Drop for Host {
        fn drop(&mut self) {
            if let Some(monitor) = self.1.take() {
                // SAFETY: a monitor `new` added
                unsafe { NSEvent::removeMonitor(&monitor) };
            }
            self.0.removeFromSuperview();
        }
    }
}
