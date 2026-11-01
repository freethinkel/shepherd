//! The right panel, Zeron's: a column of tabs beside the panes, one set per workspace. The logic is
//! `crate::right_panel`'s; the strip, the picker and the `+` here are ported from Zeron's shell.
use super::review::ShownReview;
use crate::ui::review::Review;
use crate::ui::statusbar::fetching;
use super::*;
use crate::right_panel::{Offer, PanelState, Surface, clamp_width, drop_index, fits, offered, slide_offset};
use crate::settings::RIGHT_PANEL_WIDTH;
use gpui::AnyElement;
use crate::ui::edge_fade::edge_faded_x;
use crate::ui::surface_chrome::{TAB_CHIP_HEIGHT, TAB_CHIP_SLOT, TAB_CHIP_WIDTH, TAB_STRIP_FADE, TabChip, tab_chip, tab_chip_icon};

/// A chip sliding aside for the one dragged over it.
const TAB_SLIDE: Duration = Duration::from_millis(150);

pub(super) struct RightPanel {
    /// By workspace id.
    pub(super) panels: HashMap<String, PanelState>,
    /// Saved in the settings when a drag ends.
    pub(super) width: f32,
    /// The panel's edge is being dragged.
    pub(super) drag: bool,
    /// The strip's `+` menu and where it opened.
    pub(super) plus: Option<crate::ui::Popup<gpui::Point<Pixels>>>,
    /// The workspace whose panel has the keyboard, not its panes.
    pub(super) focused: Option<String>,
    scroll: gpui::ScrollHandle,
    tab_drag: Option<TabDrag>,
    /// What the picker and the `+` offered at the last render.
    offers: Vec<Offer>,
}

impl RightPanel {
    pub(super) fn new(width: f32) -> Self {
        RightPanel { panels: HashMap::new(), width, drag: false, plus: None, focused: None, scroll: Default::default(), tab_drag: None, offers: Vec::new() }
    }
}

/// A chip on the move: where it left, and the slot it hovers over; `epoch` restarts the slide as that changes.
struct TabDrag {
    from: usize,
    over: usize,
    epoch: usize,
    prev_over: usize,
}

/// What a dragged chip carries.
#[derive(Clone)]
struct DraggedChip {
    workspace: String,
    from: usize,
    title: SharedString,
}

/// The chip that follows the pointer.
struct ChipGhost(SharedString);

impl Render for ChipGhost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx);
        div()
            .h(px(TAB_CHIP_HEIGHT))
            .w(px(TAB_CHIP_WIDTH))
            .px(px(8.))
            .flex()
            .items_center()
            .rounded(px(6.))
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border_strong)
            .text_size(px(11.5))
            .text_color(theme.text)
            .child(div().truncate().child(self.0.clone()))
    }
}

/// The pull request chip's title: the request's own once known.
fn pull_request_title(shown: &ShownReview) -> SharedString {
    match &shown.review {
        Some(Review::Found { request: Some(r), .. }) if r.pr.gitlab => format!("MR !{}", r.pr.number).into(),
        Some(Review::Found { request: Some(r), .. }) => format!("PR #{}", r.pr.number).into(),
        _ => "Pull request".into(),
    }
}

fn offer_row(offer: Offer) -> (&'static str, &'static str) {
    match offer {
        Offer::Browser => ("icons/solar/global.svg", "Browser"),
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

    /// Whether the panel is on screen: open, Settings and a sidebar page not in its place, and room for it.
    pub(super) fn right_panel_shown(&self, window: &Window) -> bool {
        let room = fits(f32::from(window.viewport_size().width), self.sidebar_now());
        self.settings.is_none() && self.pages.shown.is_none() && room && self.panel().is_some_and(|p| p.open)
    }

    /// Whether the panel shows the pull request.
    pub(super) fn pull_request_shown(&self, window: &Window) -> bool {
        self.right_panel_shown(window) && self.panel().is_some_and(|p| p.active == Some(Surface::PullRequest))
    }

    /// The width the panel is drawn at.
    pub(super) fn right_panel_width(&self, window: &Window) -> f32 {
        clamp_width(self.right.width, f32::from(window.viewport_size().width), self.sidebar_now())
    }

    pub(super) fn panel_focused(&self) -> bool {
        self.right.focused.is_some()
    }

    /// The keyboard to the panel, or back to the panes.
    pub(super) fn focus_panel(&mut self, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        match on {
            true => {
                self.right.focused = self.shown_tab().map(|t| t.workspace_id);
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

    pub(super) fn toggle_right_panel(&mut self, cx: &mut Context<Self>) {
        self.update_panel(cx, PanelState::toggle);
    }

    pub(super) fn drag_right_panel(&mut self, x: Pixels, window: &Window, cx: &mut Context<Self>) {
        if self.right.drag {
            let window_w = f32::from(window.viewport_size().width);
            self.right.width = clamp_width(window_w - f32::from(x), window_w, self.sidebar_now());
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
            Offer::Browser => {
                if let Some(tab) = self.shown_tab() {
                    self.new_browser_tab(tab.workspace_id, None, true, window, cx);
                }
            }
        }
    }

    /// The strip's tabs with their titles, in order.
    fn surface_rows(&self, shown: &ShownReview) -> Vec<(Surface, SharedString)> {
        let Some(panel) = self.panel() else { return Vec::new() };
        panel
            .tabs
            .iter()
            .map(|s| {
                let title: SharedString = match s {
                    Surface::Browser(id) => self.browser_label(id).into(),
                    Surface::PullRequest => pull_request_title(shown),
                };
                (s.clone(), title)
            })
            .collect()
    }

    pub(super) fn close_surface(&mut self, surface: &Surface, cx: &mut Context<Self>) {
        match surface {
            Surface::Browser(id) => self.close_browser_tab(id, cx),
            Surface::PullRequest => self.update_panel(cx, |p| p.close(surface)),
        }
    }

    /// The panel and its resize edge.
    pub(super) fn render_right_panel(&mut self, theme: &Theme, shown: &ShownReview, has_remote: bool, window: &mut Window, cx: &mut Context<Self>) -> [AnyElement; 2] {
        self.right.offers = offered(has_remote, true);
        let width = self.right_panel_width(window);
        let strip = self.render_right_strip(theme, shown, window, cx);
        let body = match self.panel().and_then(|p| p.active.clone()) {
            Some(Surface::PullRequest) => self.render_review(theme, shown, window, cx),
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
            None => self.render_surface_picker(theme, cx).into_any_element(),
        };
        let panel = div().id("right-panel").w(px(width)).h_full().flex_none().flex().flex_col().overflow_hidden().bg(theme.wash(0.05)).border_l_1().border_color(theme.border).child(strip).child(body);
        let edge = div()
            .id("right-panel-resize")
            .on_hover(cx.listener(|this, on: &bool, _, cx| this.hover_seam("right-panel-resize", *on, cx)))
            .absolute()
            .top(px(TITLEBAR_HEIGHT))
            .bottom_0()
            .right(px(width - 3.))
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

    /// The titlebar band over the panel: one chip per tab and the `+`, as t3code's `RightPanelTabs`, and the ✕.
    fn render_right_strip(&mut self, theme: &Theme, shown: &ShownReview, window: &mut Window, cx: &mut Context<Self>) -> Div {
        // the pointer let go outside the strip
        if self.right.tab_drag.is_some() && !cx.has_active_drag() {
            self.right.tab_drag = None;
        }
        let rows = self.surface_rows(shown);
        let fetching_review = shown.key.as_deref().is_some_and(|k| fetching::<Review>(k, cx));
        let count = rows.len();
        let active = self.panel().and_then(|p| p.active.clone());
        let workspace = self.shown_tab().map(|t| t.workspace_id).unwrap_or_default();
        let drag = self.right.tab_drag.as_ref().map(|d| (d.from, d.over, d.epoch, d.prev_over));
        let scroll = self.right.scroll.clone();
        let over_workspace = workspace.clone();
        let drop_workspace = workspace.clone();
        let mut strip = div()
            .id("right-surface-strip")
            .h_full()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(4.))
            .min_w_0()
            .overflow_x_scroll()
            .track_scroll(&self.right.scroll)
            .when(cfg!(windows), |el| el.occlude()) // as the tab bar's, off the drag strip
            .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.notify()))
            .on_drag_move::<DraggedChip>(cx.listener(move |this, e: &gpui::DragMoveEvent<DraggedChip>, _, cx| {
                let chip = e.drag(cx);
                if chip.workspace != over_workspace {
                    return;
                }
                let from = chip.from;
                let x = f32::from(e.event.position.x) - f32::from(e.bounds.left()) - f32::from(scroll.offset().x);
                let over = drop_index(x, TAB_CHIP_SLOT, count);
                this.drag_chip_over(from, over, cx);
            }))
            .on_drop::<DraggedChip>(cx.listener(move |this, chip: &DraggedChip, _, cx| this.chip_dropped(chip, &drop_workspace, cx)));
        for (ix, (surface, title)) in rows.into_iter().enumerate() {
            let is_active = active.as_ref() == Some(&surface);
            let (activate, close, middle) = (surface.clone(), surface.clone(), surface.clone());
            let icon = match surface {
                Surface::Browser(_) => "icons/solar/global.svg",
                Surface::PullRequest => "icons/git-pull-request.svg",
            };
            let spinner = |window: &mut Window| crate::ui::loader::mini_spinner(2., theme.text_faint, window).into_any_element();
            let leading = match &surface {
                Surface::PullRequest if fetching_review => spinner(window),
                Surface::PullRequest => tab_chip_icon(icon, is_active, theme),
                Surface::Browser(id) => match self.browser_chip(id) {
                    (_, true) => spinner(window),
                    (Some(image), false) => gpui::img(image).size(px(12.)).rounded(px(2.)).into_any_element(),
                    (None, false) => tab_chip_icon(icon, is_active, theme),
                },
            };
            let drop_workspace = workspace.clone();
            let chip = tab_chip(
                TabChip { id: "right-surface-tab", close_id: "right-surface-close", ix, active: is_active, title: title.clone(), leading },
                theme,
                cx.listener(move |this, _, _, cx| this.close_surface(&close, cx)),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                this.update_panel(cx, |p| p.activate(&activate));
                // a page takes the keyboard from a click on its tab; the pull request has no use for it
                this.focus_panel(matches!(activate, Surface::Browser(_)), window, cx);
            }))
            .on_mouse_down(MouseButton::Middle, cx.listener(move |this, _: &MouseDownEvent, _, cx| this.close_surface(&middle, cx)))
            .on_drag(DraggedChip { workspace: workspace.clone(), from: ix, title }, |chip, _, _, cx| {
                cx.stop_propagation();
                let title = chip.title.clone();
                cx.new(|_| ChipGhost(title))
            })
            // the chips tile the strip and block the mouse: the drop has to land on one
            .on_drop::<DraggedChip>(cx.listener(move |this, chip: &DraggedChip, _, cx| this.chip_dropped(chip, &drop_workspace, cx)));
            // sliding aside for the chip dragged over; the dragged one leaves a gap, the ghost carrying it
            let placed = match drag {
                Some((from, over, epoch, prev_over)) if ix != from => {
                    let (to, start) = (slide_offset(ix, from, over) * TAB_CHIP_SLOT, slide_offset(ix, from, prev_over) * TAB_CHIP_SLOT);
                    div()
                        .relative()
                        .child(chip.with_animation(("right-tab-slide", (ix as u64) | ((epoch as u64) << 32)), gpui::Animation::new(TAB_SLIDE), move |el, t| el.left(px(start + (to - start) * t))))
                        .into_any_element()
                }
                Some((from, ..)) if ix == from => div().w(px(TAB_CHIP_WIDTH)).h(px(TAB_CHIP_HEIGHT)).flex_none().into_any_element(),
                _ => chip.into_any_element(),
            };
            strip = strip.child(placed);
        }
        if count == 0 {
            strip = strip.child(
                tab_chip(
                    TabChip { id: "right-surface-new", close_id: "right-surface-new-close", ix: 0, active: false, title: "New".into(), leading: tab_chip_icon("icons/plus.svg", false, theme) },
                    theme,
                    cx.listener(|this, _, _, cx| this.toggle_right_panel(cx)),
                ),
            );
        } else {
            let plus_open = self.right.plus.as_ref().is_some_and(|m| m.is_open());
            strip = strip.child(
                icon_button(theme, "right-surface-add", "")
                    .size(px(24.))
                    .tooltip(tip("New tab", None))
                    .when(plus_open, |el| el.bg(theme.hover))
                    .child(gpui::svg().path("icons/plus.svg").size(px(13.)).text_color(theme.text_muted))
                    .block_mouse_except_scroll()
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(cx.listener(|this, e: &ClickEvent, _, cx| {
                        let at = gpui::point(e.position().x - px(8.), px(TITLEBAR_HEIGHT - 2.));
                        this.right.plus = Some(crate::ui::Popup::new(at));
                        cx.notify();
                    })),
            );
        }
        let scroll = self.right.scroll.clone();
        div()
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            .pl(px(8.))
            .pr(px(8. + window_controls_w(cx)))
            .flex()
            .items_center()
            .gap(px(4.))
            .border_b_1()
            .border_color(theme.border)
            .child(div().flex_1().min_w_0().h_full().child(edge_faded_x(TAB_STRIP_FADE, &scroll, strip).bottom_only()))
            .child(
                titlebar_button(theme, "right-panel-close", "icons/close.svg")
                    .tooltip(tip("Close", Some("ToggleRightPanel")))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_right_panel(cx))),
            )
    }

    fn drag_chip_over(&mut self, from: usize, over: usize, cx: &mut Context<Self>) {
        match &mut self.right.tab_drag {
            Some(drag) if drag.over != over => {
                drag.prev_over = drag.over;
                drag.over = over;
                drag.epoch += 1;
                cx.notify();
            }
            Some(_) => {}
            None => {
                self.right.tab_drag = Some(TabDrag { from, over, epoch: 0, prev_over: from });
                cx.notify();
            }
        }
    }

    /// A chip let go of on the strip: it lands in the slot it was over, in the panel it came from.
    fn chip_dropped(&mut self, chip: &DraggedChip, workspace: &str, cx: &mut Context<Self>) {
        let to = self.right.tab_drag.take().map_or(chip.from, |d| d.over);
        if chip.workspace == workspace {
            self.update_panel(cx, |p| p.reorder(chip.from, to));
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
