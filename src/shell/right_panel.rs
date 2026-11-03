//! The right panel, Zeron's: a column of tabs beside the panes, one set per workspace. The logic is
//! `crate::right_panel`'s, the strip is `tabs`', the picker and the `+` menu are ported from Zeron's shell.
use super::changes::Changes;
use super::review::ShownReview;
use crate::ui::review::Review;
use crate::ui::statusbar::{branch_of, fetching, pane_dir};
use super::tabs::{Leading, Tab, Tabs, new_tab_button, plain_tint, tab_element};
use super::*;
use crate::right_panel::{Offer, PanelState, Surface, clamp_width, fits, offered, slides};
use crate::settings::RIGHT_PANEL_WIDTH;
use gpui::AnyElement;

/// The tab an empty panel shows in its strip, whose `×` closes the panel.
const NEW: &str = "right-surface-new";
const DIFF_ICON: &str = "icons/solar/list.svg";

pub(super) struct RightPanel {
    /// By workspace id.
    pub(super) panels: HashMap<String, PanelState>,
    /// Saved in the settings when a drag ends.
    pub(super) width: f32,
    /// The panel's edge is being dragged.
    pub(super) drag: bool,
    /// The dragged edge's spring at a limit.
    pub(super) edge: EdgeSpring,
    /// The strip's `+` menu and where it opened.
    pub(super) plus: Option<crate::ui::Popup<gpui::Point<Pixels>>>,
    /// The workspace whose panel has the keyboard, not its panes.
    pub(super) focused: Option<String>,
    /// Its slide while it shows or hides: 0 hidden, 1 shown.
    pub(super) motion: Option<FoldMotion>,
    /// Its growth while it expands or collapses: 0 beside the panes, 1 in their place.
    pub(super) expand: Option<FoldMotion>,
    /// The workspace shown and whether its panel was open at the last render.
    seen: (Option<String>, bool),
    /// The same for whether it was expanded.
    seen_expanded: (Option<String>, bool),
    tabs: Tabs,
    /// What the picker and the `+` offered at the last render.
    offers: Vec<Offer>,
    /// The diff tabs of all workspaces, by their tab's key.
    pub(super) diffs: HashMap<String, Entity<Changes>>,
    /// How many diff tabs were made: their ids.
    pub(super) made_diffs: u64,
    /// The discard asked about, of which directory, at which capture.
    pub(super) discard: Option<crate::ui::Popup<super::changes::DiscardAsk>>,
    /// A diff's agent menu: of which tab, and where it opened.
    pub(super) agents: Option<crate::ui::Popup<(String, gpui::Point<Pixels>)>>,
    /// The agent picked for a diff tab, by its pane.
    pub(super) diff_agents: HashMap<String, String>,
    /// Each workspace's agents' turns, watched whether or not a diff is open: a turn can't be seen afterwards.
    pub(super) turns: HashMap<String, super::changes::Turn>,
}

impl RightPanel {
    pub(super) fn new(width: f32) -> Self {
        RightPanel {
            panels: HashMap::new(),
            width,
            drag: false,
            edge: None,
            plus: None,
            focused: None,
            motion: None,
            expand: None,
            seen: (None, false),
            seen_expanded: (None, false),
            tabs: Tabs::default(),
            offers: Vec::new(),
            diffs: HashMap::new(),
            made_diffs: 0,
            discard: None,
            agents: None,
            diff_agents: HashMap::new(),
            turns: HashMap::new(),
        }
    }
}

/// A 0-to-1 tween's value this frame, or where `on` rests. A tween heading elsewhere was another workspace's, left
/// behind by a switch.
fn tweened(motion: Option<FoldMotion>, on: bool) -> f32 {
    match motion.filter(|m| m.animating() && (m.to > 0.) == on) {
        Some(m) => m.current(),
        None if on => 1.,
        None => 0.,
    }
}

/// A 0-to-1 tween toward `on`, from wherever `motion` has got to.
fn retween(motion: Option<FoldMotion>, on: bool) -> FoldMotion {
    let to = if on { 1. } else { 0. };
    let from = motion.filter(|m| m.animating()).map_or(1. - to, FoldMotion::current);
    FoldMotion { epoch: 0, from, to, started: Instant::now() }
}

/// The pull request tab's title: the request's own once known.
fn pull_request_title(shown: &ShownReview) -> String {
    match &shown.review {
        Some(Review::Found { request: Some(r), .. }) if r.pr.gitlab => format!("MR !{}", r.pr.number),
        Some(Review::Found { request: Some(r), .. }) => format!("PR #{}", r.pr.number),
        _ => "Pull request".into(),
    }
}

fn offer_row(offer: Offer) -> (&'static str, &'static str) {
    match offer {
        Offer::Browser => ("icons/solar/global.svg", "Browser"),
        Offer::Diffs => (DIFF_ICON, "Diffs"),
        Offer::PullRequest => ("icons/git-pull-request.svg", "Pull request"),
    }
}

impl Shell {
    /// The shown workspace's panel.
    pub(super) fn panel(&self) -> Option<&PanelState> {
        self.right.panels.get(&self.shown_tab()?.workspace_id)
    }

    /// Changes the shown workspace's panel, made if it had none.
    pub(super) fn update_panel(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut PanelState)) {
        if let Some(tab) = self.shown_tab() {
            change(self.right.panels.entry(tab.workspace_id).or_default());
            cx.notify();
        }
    }

    /// Whether the window can show a panel: Settings and a sidebar page not in its place, and room for it.
    fn right_panel_fits(&self, window: &Window) -> bool {
        self.settings.is_none() && self.pages.shown.is_none() && fits(f32::from(window.viewport_size().width), self.sidebar_now())
    }

    fn right_panel_open(&self) -> bool {
        self.panel().is_some_and(|p| p.open)
    }

    /// Whether the panel is on screen and staying. One sliding out is not: it's only drawn, see `right_panel_now`.
    pub(super) fn right_panel_shown(&self, window: &Window) -> bool {
        self.right_panel_fits(window) && self.right_panel_open()
    }

    fn right_panel_expanded(&self) -> bool {
        self.panel().is_some_and(|p| p.expanded)
    }

    /// Whether it has left its place beside the panes, on its way or all the way.
    pub(super) fn right_panel_grown(&self) -> bool {
        tweened(self.right.expand, self.right_panel_expanded()) > 0.
    }

    /// The panel's slide this frame: 0 hidden, 1 shown.
    pub(super) fn right_panel_slide(&self, window: &Window) -> f32 {
        if self.right_panel_fits(window) { tweened(self.right.motion, self.right_panel_open()) } else { 0. }
    }

    /// The room the panes give up for the panel this frame.
    pub(super) fn right_panel_now(&self, window: &Window) -> f32 {
        self.right_panel_width(window) * self.right_panel_slide(window)
    }

    /// The panel's own width: beside the panes, or grown in their place to the sidebar.
    pub(super) fn right_panel_drawn(&self, window: &Window) -> f32 {
        let beside = self.right_panel_width(window);
        let full = f32::from(window.viewport_size().width) - self.sidebar_now();
        beside + (full - beside) * tweened(self.right.expand, self.right_panel_expanded())
    }

    /// Once per render, so every way of opening and closing it slides: holds the panes' grids until it lands, as the
    /// sidebar does.
    pub(super) fn follow_right_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ws = self.shown_tab().map(|t| t.workspace_id);
        let now = (ws.clone(), self.right_panel_open());
        let expanded = (ws.clone(), self.right_panel_expanded());
        let was = std::mem::replace(&mut self.right.seen, now.clone());
        let was_expanded = std::mem::replace(&mut self.right.seen_expanded, expanded.clone());
        let fit = self.right_panel_fits(window);
        if fit && slides(&was_expanded, &expanded) {
            self.right.expand = Some(retween(self.right.expand, expanded.1));
        }
        // the panes are clipped away: what's typed must not reach them unseen
        if expanded.1 && self.right_panel_shown(window) && self.right.focused != ws {
            self.focus_panel(true, window, cx);
        }
        if !fit || !slides(&was, &now) {
            return;
        }
        let motion = retween(self.right.motion, now.1);
        self.right.motion = Some(motion);
        cx.set_global(GridHold(true));
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(FOLD_DURATION + Duration::from_millis(120)).await;
            let _ = this.update(cx, |shell, cx| {
                if shell.right.motion.is_some_and(|m| m.started == motion.started) {
                    shell.right.motion = None;
                    shell.release_grids(cx);
                }
            });
        })
        .detach();
    }

    /// Whether the panel shows the pull request.
    pub(super) fn pull_request_shown(&self, window: &Window) -> bool {
        self.right_panel_shown(window) && self.panel().is_some_and(|p| p.active == Some(Surface::PullRequest))
    }

    /// The width the panel is drawn at, its edge's spring included.
    pub(super) fn right_panel_width(&self, window: &Window) -> f32 {
        clamp_width(self.right.width, f32::from(window.viewport_size().width), self.sidebar_now()) + spring_offset(self.right.edge).unwrap_or(0.)
    }

    pub(super) fn panel_focused(&self) -> bool {
        self.right.focused.is_some()
    }

    /// The keyboard to the panel, or back to the panes.
    pub(super) fn focus_panel(&mut self, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        match on {
            true => {
                self.right.focused = self.shown_tab().map(|t| t.workspace_id);
                self.sidebar_cursor = None;
                // a pane hands its keys over; `sync_pages` gives them to the page
                window.focus(&self.focus, cx);
            }
            false => self.release_panel(),
        }
        cx.notify();
    }

    /// The keyboard back to the panes: forgetting the pane focused has the next render focus it again.
    pub(super) fn release_panel(&mut self) {
        if self.right.focused.take().is_some() {
            self.focused = None;
        }
    }

    /// `FocusRight` past the rightmost pane; false when a pane lies that way, or there's no panel.
    pub(super) fn enter_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let tab = self.shown_tab().map(|t| t.tab_id);
        let layout = self.snapshot.as_ref().and_then(|s| s.layouts.iter().find(|l| Some(&l.tab_id) == tab.as_ref()));
        let pane = self.active_pane(window, cx);
        let beside = pane.is_some_and(|p| layout.is_some_and(|l| l.has_neighbor(&p, Direction::Right)));
        let enters = !beside && !self.panel_focused() && self.right_panel_shown(window);
        if enters {
            self.focus_panel(true, window, cx);
        }
        enters
    }

    /// `CloseTab` and `ClosePane` with the keyboard in the panel.
    pub(super) fn close_active_surface(&mut self, cx: &mut Context<Self>) {
        if let Some(surface) = self.panel().and_then(|p| p.active.clone()) {
            self.close_surface(&surface, cx);
        }
    }

    pub(super) fn toggle_right_panel(&mut self, window: &Window, cx: &mut Context<Self>) {
        let seen = self.right_panel_shown(window);
        self.update_panel(cx, |p| p.toggle_seen(seen));
    }

    /// The diff's key: as the pull request's, but a workspace may have no diff yet. In a repo, its first is made.
    /// The keyboard goes along: to the diff shown, back to the panes when it closes.
    pub(super) fn toggle_diff(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let seen = self.right_panel_shown(window);
        let mut missing = false;
        self.update_panel(cx, |p| missing = !p.toggle_diff(seen));
        let dir = pane_dir(self.snapshot.as_ref(), self.active_pane(window, cx).as_deref());
        if missing && dir.is_some_and(|d| branch_of(&d, cx).is_some()) {
            self.add_diff_surface(window, cx);
        }
        let on_diff = self.panel().is_some_and(|p| p.open && matches!(p.active, Some(Surface::Diff(_))));
        self.focus_panel(on_diff, window, cx);
    }

    /// Zeron's takeover, from the strip's button: the panel takes the panes' room, which clip and keep their grids, or
    /// goes back beside them with the keyboard back there too.
    fn toggle_right_panel_expand(&mut self, window: &Window, cx: &mut Context<Self>) {
        // a closing panel's strip is still drawn
        if self.right_panel_shown(window) {
            self.release_panel();
            self.update_panel(cx, |p| p.expanded = !p.expanded);
        }
    }

    pub(super) fn drag_right_panel(&mut self, x: Pixels, window: &Window, cx: &mut Context<Self>) {
        if self.right.drag {
            let window_w = f32::from(window.viewport_size().width);
            let sidebar = self.sidebar_now();
            let limit = |width| clamp_width(width, window_w, sidebar);
            let wanted = window_w - f32::from(x);
            spring_at_limit(&mut self.right.edge, wanted, limit(0.), limit(f32::INFINITY));
            self.right.width = limit(wanted);
            cx.notify();
        }
    }

    pub(super) fn end_right_panel_drag(&mut self, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.right.drag) {
            cx.set_global(GridHold(false));
            let width = self.right.width;
            save_from_sidebar(cx, |s| s.right_panel_width = width);
            cx.refresh_windows();
        }
    }

    /// A row of the picker or the `+` picked: its tab shown, made if it's the first.
    fn pick_surface(&mut self, offer: Offer, window: &mut Window, cx: &mut Context<Self>) {
        match offer {
            Offer::PullRequest => self.update_panel(cx, |p| p.add(Surface::PullRequest)),
            Offer::Diffs => self.add_diff_surface(window, cx),
            Offer::Browser => {
                if let Some(tab) = self.shown_tab() {
                    self.new_browser_tab(tab.workspace_id, None, true, window, cx);
                }
            }
        }
    }

    /// The strip's tabs, in order.
    fn surface_tabs(&self, shown: &ShownReview, theme: &Theme, window: &Window, cx: &mut App) -> Vec<Tab> {
        let Some(panel) = self.panel() else { return Vec::new() };
        let mark = |icon| Leading::Marks(vec![(icon, plain_tint(theme))]);
        let fetching_review = shown.key.as_deref().is_some_and(|k| fetching::<Review>(k, cx));
        let tab = |s: &Surface| {
            let (title, leading) = match s {
                Surface::Browser(id) => (self.browser_label(id), match self.browser_chip(id) {
                    (_, true) => Leading::Spinner,
                    (Some(image), false) => Leading::Image(image),
                    (None, false) => mark("icons/solar/global.svg"),
                }),
                Surface::Diff(id) => (self.right.diffs.get(id).map_or("Diff".to_string(), |c| c.read(cx).tab_title().to_string()), mark(DIFF_ICON)),
                Surface::PullRequest => (pull_request_title(shown), if fetching_review { Leading::Spinner } else { mark("icons/git-pull-request.svg") }),
            };
            Tab::titled(s.key(), title, leading, theme, window)
        };
        panel.tabs.iter().map(tab).collect()
    }

    pub(super) fn close_surface(&mut self, surface: &Surface, cx: &mut Context<Self>) {
        match surface {
            Surface::Browser(id) => self.close_browser_tab(id, cx),
            Surface::Diff(id) => {
                self.right.diffs.remove(id);
                self.update_panel(cx, |p| p.close(surface));
            }
            Surface::PullRequest => self.update_panel(cx, |p| p.close(surface)),
        }
    }

    /// The panel and its resize edge.
    pub(super) fn render_right_panel(&mut self, theme: &Theme, shown: &ShownReview, has_remote: bool, window: &mut Window, cx: &mut Context<Self>) -> [AnyElement; 2] {
        self.right.offers = offered(has_remote, shown.key.is_some(), super::web::embeds_pages(window));
        let width = self.right_panel_drawn(window);
        let now = width * self.right_panel_slide(window);
        let strip = self.render_right_strip(theme, shown, window, cx);
        let body = match self.panel().and_then(|p| p.active.clone()) {
            Some(Surface::PullRequest) => self.render_review(theme, shown, window, cx),
            Some(Surface::Diff(id)) if self.right.diffs.contains_key(&id) => self.render_diff(&id, theme, window, cx),
            Some(Surface::Browser(id)) => {
                let key = (id, None);
                let bar = self.render_browser_bar(&key, theme, window, cx);
                let page = div().flex_1().min_h_0().flex().flex_col().child(self.render_web_area(&key, theme, window, cx)).children(self.render_annotations(&key, theme, cx));
                // the page's own clicks reach `PageEvent::Clicked`; this takes the bar's and the notes'
                div()
                    .id("right-panel-browser")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .capture_any_mouse_down(cx.listener(|this, _: &MouseDownEvent, window, cx| this.focus_panel(true, window, cx)))
                    .child(bar)
                    .child(page)
                    .into_any_element()
            }
            Some(Surface::Diff(_)) | None => self.render_surface_picker(theme, cx).into_any_element(),
        };
        let panel = div().id("right-panel").w(px(width)).h_full().flex_none().flex().flex_col().overflow_hidden().bg(theme.wash(0.05)).border_l_1().border_color(theme.border).child(strip).child(body);
        // the sidebar's slide, mirrored but for the content, which rides the edge: a native page can't be clipped by
        // GPUI, only by the window
        let panel = div().w(px(now)).h_full().flex_none().overflow_hidden().child(panel);
        let grown = self.right_panel_grown();
        let edge = div()
            .id("right-panel-resize")
            .on_hover(cx.listener(|this, on: &bool, _, cx| this.hover_seam("right-panel-resize", *on, cx)))
            .absolute()
            .top(px(TITLEBAR_HEIGHT))
            .bottom_0()
            .right(px(now - 3.))
            .when(grown, |el| el.invisible())
            .w(px(6.))
            .cursor_col_resize()
            // the panel's 1 px border sits at x = 3 of the strip
            .child(resize_seam("right-panel-resize".into(), true, 3., theme.accent, self.right.drag, self.seam_hover.get("right-panel-resize").copied()))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.right.drag = true;
                    cx.set_global(GridHold(true));
                    cx.notify();
                }),
            )
            .on_click(cx.listener(|this, e: &ClickEvent, _, cx| {
                if e.click_count() == 2 {
                    this.right.width = RIGHT_PANEL_WIDTH;
                    save_from_sidebar(cx, |s| s.right_panel_width = RIGHT_PANEL_WIDTH);
                    cx.notify();
                }
            }));
        [panel.into_any_element(), edge.into_any_element()]
    }

    /// The titlebar band over the panel: its tabs and the `+`, as t3code's `RightPanelTabs`, and the toggle.
    fn render_right_strip(&mut self, theme: &Theme, shown: &ShownReview, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let mut tabs = self.surface_tabs(shown, theme, window, cx);
        let empty = tabs.is_empty();
        if empty {
            tabs.push(Tab::titled(NEW, "New".into(), Leading::Marks(vec![("icons/plus.svg", plain_tint(theme))]), theme, window));
        }
        let scope = self.shown_tab().map(|t| t.workspace_id);
        let active = self.panel().and_then(|p| p.active.as_ref()).map(|s| s.key().to_string());
        let placed = self.right.tabs.frame(scope.as_deref(), tabs, active.as_deref(), window, cx);
        let mut items = Vec::new();
        for p in &placed {
            if p.tab.id == NEW {
                let close = |this: &mut Shell, cx: &mut Context<Shell>| this.update_panel(cx, PanelState::shut);
                items.push(tab_element(p, theme, None, None, close, window, cx).into_any_element());
                continue;
            }
            let surface = Surface::of_key(&p.tab.id);
            let close = surface.clone();
            let tab = tab_element(p, theme, None, None, move |this, cx| this.close_surface(&close, cx), window, cx);
            if p.closing {
                items.push(tab.into_any_element());
                continue;
            }
            items.push(
                self.right
                    .tabs
                    .liftable(tab, p, theme, Shell::surface_dropped, cx)
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        this.update_panel(cx, |p| p.activate(&surface));
                        // a page takes the keyboard from a click on its tab; the pull request has no use for it
                        this.focus_panel(matches!(surface, Surface::Browser(_)), window, cx);
                    }))
                    .into_any_element(),
            );
        }
        let row = self.tabs_row(&self.right.tabs, "right-surface-strip", theme, items, Shell::surface_dropped, cx);
        let plus_open = self.right.plus.as_ref().is_some_and(|m| m.is_open());
        let expanded = self.right_panel_expanded();
        let slide = self.right_panel_slide(window);
        let left = f32::from(window.viewport_size().width) - self.right_panel_drawn(window) * slide;
        div()
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            // opaque, as the tab bar's, for the edge fades
            .bg(theme.surface)
            // grown to the window's edge with the sidebar closed, it clears the traffic lights and the sidebar toggle
            .pl(px((TITLEBAR_CLUSTER - left).max(8.)))
            .pr(px(8. + window_controls_w(cx)))
            .flex()
            .items_center()
            .gap(px(2.))
            .border_b_1()
            .border_color(theme.border)
            .child(row.flex_1())
            .when(!empty, |el| {
                el.child(
                    new_tab_button(theme, "right-surface-add")
                        .tooltip(tip("New tab", None))
                        .when(plus_open, |el| el.bg(theme.hover))
                        .on_click(cx.listener(|this, e: &ClickEvent, _, cx| {
                            let at = gpui::point(e.position().x - px(8.), px(TITLEBAR_HEIGHT - 2.));
                            this.right.plus = Some(crate::ui::Popup::new(at));
                            cx.notify();
                        })),
                )
            })
            .child(
                titlebar_button(theme, "right-panel-expand", if expanded { "icons/collapse-arrows.svg" } else { "icons/expand-arrows.svg" })
                    .tooltip(tip(if expanded { "Collapse panel" } else { "Expand panel" }, None))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_right_panel_expand(window, cx))),
            )
            .child(
                titlebar_button_with(theme, "right-panel-toggle-open", crate::ui::sidebar_glyph(slide, true, 16., theme.text_muted))
                    .tooltip(tip("Toggle right panel", Some("ToggleRightPanel")))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_right_panel(window, cx))),
            )
    }

    /// A tab let go of on the strip: it lands in the slot its gap holds.
    fn surface_dropped(&mut self, cx: &mut Context<Self>) {
        if let Some((from, to)) = self.right.tabs.drop_held(Instant::now()) {
            self.update_panel(cx, |p| p.reorder(from, to));
        }
        cx.notify();
    }

    /// Zeron's empty panel: a compact list of what it can show.
    fn render_surface_picker(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let (hover_bg, hover_border) = (theme.wash(0.05), theme.border_strong);
        let rows = self.right.offers.clone().into_iter().map(|offer| {
            let (icon, title) = offer_row(offer);
            div()
                .id(SharedString::from(format!("surface-card-{title}")))
                .w_full()
                .h(px(44.))
                .px(px(14.))
                .rounded(px(10.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.wash(0.02))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(10.))
                .cursor_pointer()
                .hover(move |s| s.bg(hover_bg).border_color(hover_border))
                .child(gpui::svg().path(icon).size(px(15.)).flex_none().text_color(theme.text_muted))
                .child(div().text_size(px(13.)).font_weight(gpui::FontWeight::MEDIUM).text_color(theme.text).child(title))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.pick_surface(offer, window, cx)))
        });
        div()
            .flex_1()
            .min_h_0()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .p(px(16.))
            .child(div().w_full().max_w(px(280.)).flex().flex_col().gap(px(8.)).children(rows))
    }

    /// The strip's `+` menu.
    pub(super) fn right_plus_menu(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.right.plus.as_ref()?;
        let at = menu.value;
        let items = self.right.offers.clone().into_iter().map(|offer| {
            let (icon, title) = offer_row(offer);
            row(theme, false, SharedString::from(format!("right-plus-{title}")))
                .h(px(28.))
                .rounded(px(6.))
                .child(menu_mark(icon, theme.text_muted))
                .child(title)
                .on_pick(
                    at,
                    cx.listener(move |this, _: &ClickEvent, window, cx| {
                        if let Some(m) = this.right.plus.as_mut() {
                            m.close()
                        }
                        this.pick_surface(offer, window, cx);
                    }),
                )
        });
        let card = menu_card(theme, "right-plus-menu", 168.)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if let Some(m) = this.right.plus.as_mut() {
                    m.close()
                }
                cx.notify();
            }))
            .children(items);
        let menu = crate::ui::menu_motion("right-plus-menu-in", menu, card);
        Some(gpui::deferred(gpui::anchored().position(at).anchor(gpui::Anchor::TopLeft).snap_to_window_with_margin(px(8.)).child(menu)).with_priority(1).into_any_element())
    }
}
