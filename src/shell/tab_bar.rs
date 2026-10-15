//! The tab bar: Zeron's terminal-panel tabs (28 px, 8 px radius, a wash under the active one) with what runs in
//! each tab as a colored favicon (Superlogical's tabs), sized to their labels. A new tab grows in, a closed one
//! stays mounted while it shrinks out (the `Popup` recipe), the active wash slides between tabs, a reorder
//! slides the tabs to their new places, and a dropped tab flies from where it was let go back into its slot.
use super::*;
use std::cell::RefCell;

/// Zeron's `motion::COLLAPSE` for a tab growing in or shrinking out; the wash and a reorder slide a bit quicker
/// than its `TAB_SLIDE`.
const ENTER: Duration = Duration::from_millis(180);
const EXIT: Duration = Duration::from_millis(150);
const SLIDE: Duration = Duration::from_millis(110);
/// A dropped tab's flight home.
const LAND: Duration = Duration::from_millis(220);

const TAB_H: f32 = 28.;
/// The tabs' top inside the bar.
const TAB_TOP: f32 = (TITLEBAR_HEIGHT - TAB_H) / 2.;
/// Space after each tab; it collapses with a closing tab so neighbours close in smoothly.
const GAP: f32 = 2.;
const PAD_L: f32 = 6.;
const PAD_R: f32 = 4.;
const INNER_GAP: f32 = 6.;
/// The favicon tile.
const ICON: f32 = 18.;
/// The trailing slot: the status dot, swapped in place for `×` while the tab is hovered.
const SLOT: f32 = 20.;
const ZOOM_MARK: f32 = 12.;
const MIN_W: f32 = 96.;
const MAX_W: f32 = 220.;
const FONT_SIZE: f32 = 12.5;

/// Tab-bar state that outlives a frame: the scroll, the tabs as last drawn (closing ones included), the wash,
/// and the tab being dragged with its ghost's top-left in the window (written by the ghost as it follows the pointer).
#[derive(Default)]
pub(super) struct TabBar {
    pub scroll: gpui::ScrollHandle,
    workspace: Option<String>,
    entries: Vec<Entry>,
    wash: Option<Wash>,
    drag: Rc<RefCell<Option<(String, gpui::Point<Pixels>)>>>,
    /// The tab last scrolled into view; the bar follows a newly shown tab once, then leaves the scroll to you.
    revealed: Option<String>,
}

/// One tab as drawn.
#[derive(Clone)]
struct Shown {
    id: String,
    label: String,
    status: AgentStatus,
    icon: &'static str,
    /// The favicon's tile and mark colors.
    tint: (Hsla, Hsla),
    /// One of its panes is zoomed to fill it: the other panes are hidden.
    zoomed: bool,
    /// The full width: label measured, clamped.
    width: f32,
}

#[derive(Clone)]
struct Entry {
    tab: Shown,
    enter: Option<Instant>,
    exit: Option<Instant>,
    /// Off its slot: sliding after a reorder, or flying home after a drop (`lifted`, drawn over everything).
    moved: Option<Move>,
    /// Where it was drawn last frame: content x and the offset from the bar's row.
    drawn: (f32, f32),
}

#[derive(Clone, Copy)]
struct Move {
    from: (f32, f32),
    started: Instant,
    duration: Duration,
    lifted: bool,
}

impl Entry {
    fn new(tab: Shown, enter: Option<Instant>) -> Self {
        Entry { tab, enter, exit: None, moved: None, drawn: (0., 0.) }
    }

    /// How far the tab is in: 0 collapsed, 1 full width.
    fn presence(&self, now: Instant) -> f32 {
        match (self.exit, self.enter) {
            (Some(since), _) => 1. - ease_out(progress(since, EXIT, now)),
            (None, Some(since)) => ease_out(progress(since, ENTER, now)),
            (None, None) => 1.,
        }
    }

    fn moving(&self, now: Instant) -> bool {
        let running = |since: Instant, d: Duration| now.duration_since(since) < d;
        self.exit.is_some_and(|s| running(s, EXIT))
            || self.enter.is_some_and(|s| running(s, ENTER))
            || self.moved.is_some_and(|m| running(m.started, m.duration))
    }

    /// Mid-flight home after a drop.
    fn lifted(&self, now: Instant) -> bool {
        self.moved.is_some_and(|m| m.lifted && now.duration_since(m.started) < m.duration)
    }
}

fn progress(since: Instant, d: Duration, now: Instant) -> f32 {
    (now.duration_since(since).as_secs_f32() / d.as_secs_f32()).min(1.)
}

/// The active wash: where it slides from, when it set off, and where it was last drawn (x, width).
struct Wash {
    tab: String,
    from: (f32, f32),
    started: Instant,
    shown: (f32, f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Stay,
    New,
    Gone,
}

/// The tabs to draw: `now` in herdr's order, with each tab gone since `prev` kept right after the tab it
/// followed (or first), so it shrinks out in place.
fn merge(prev: &[&str], now: &[&str]) -> Vec<(String, Phase)> {
    let mut out: Vec<(String, Phase)> =
        now.iter().map(|id| (id.to_string(), if prev.contains(id) { Phase::Stay } else { Phase::New })).collect();
    for (i, id) in prev.iter().enumerate() {
        if now.contains(id) {
            continue;
        }
        let anchor = prev[..i].iter().rev().find_map(|p| out.iter().position(|(o, _)| o == p));
        out.insert(anchor.map_or(0, |a| a + 1), (id.to_string(), Phase::Gone));
    }
    out
}

/// The icon for what runs in a tab: the agent's mark (Zeron's harness icons), a bot for agents without one,
/// a terminal for a plain shell.
fn agent_icon(agent: Option<&str>) -> &'static str {
    match agent.map(crate::ui::command_panel::agent_icon) {
        None => "icons/prompt.svg",
        Some("icons/bot.svg") => "icons/bot-filled.svg",
        Some(mark) => mark,
    }
}

/// The favicon's tile and mark: the brand's colors (black for monochrome brands, as their app icons are), an
/// inverted tile for agents without a mark, a muted one for a shell.
fn agent_tint(agent: Option<&str>, theme: &Theme) -> (Hsla, Hsla) {
    let brand = |bg: u32, fg: u32| (gpui::rgb(bg).into(), gpui::rgb(fg).into());
    match agent {
        None => (theme.text_muted.opacity(0.55), theme.surface),
        Some("claude") => brand(0xD97757, 0xFFFFFF),
        Some("codex") => brand(0x10A37F, 0xFFFFFF),
        Some("agy" | "gemini") => brand(0x4285F4, 0xFFFFFF),
        // cursor.com's signature orange
        Some("cursor") => brand(0xF54E00, 0xFFFFFF),
        // Cognition's Devin palette blue
        Some("devin") => brand(0x0294DE, 0xFFFFFF),
        // ponytail: pi.dev's yellow accent; pi has no published brand color
        Some("pi" | "omp") => brand(0xF1BE58, 0x1A1A1A),
        // ponytail: Qwen's logo purple, by eye
        Some("qwen") => brand(0x615CED, 0xFFFFFF),
        // monochrome brands
        Some("grok" | "opencode" | "copilot" | "droid" | "kimi" | "kilo" | "qoder" | "qodercli" | "mastra" | "mastracode") => {
            brand(0x0A0A0A, 0xFFFFFF)
        }
        Some(_) => (theme.text, theme.surface),
    }
}

/// `a` toward `b` by `t`, channel by channel.
fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
    let l = |x: f32, y: f32| x + (y - x) * t;
    gpui::hsla(l(a.h, b.h), l(a.s, b.s), l(a.l, b.l), l(a.a, b.a))
}

/// The scroll (px scrolled from the left) that brings the slot `x..x + w` into a `view`-wide bar, with a
/// fade's width to spare; `None` when it already shows.
fn reveal(scrolled: f32, view: f32, x: f32, w: f32) -> Option<f32> {
    const SPARE: f32 = 32.;
    if x < scrolled {
        Some((x - SPARE).max(0.))
    } else if x + w > scrolled + view {
        Some(x + w - view + SPARE)
    } else {
        None
    }
}

fn tab_width(label_w: f32, zoomed: bool) -> f32 {
    let mark = if zoomed { ZOOM_MARK + INNER_GAP } else { 0. };
    (PAD_L + ICON + INNER_GAP + label_w + mark + INNER_GAP + SLOT + PAD_R).clamp(MIN_W, MAX_W).ceil()
}

/// After the label of a tab with a zoomed pane, the zoom action's icon.
fn zoom_mark(theme: &Theme) -> gpui::Svg {
    gpui::svg().path("icons/expand-arrows.svg").size(px(ZOOM_MARK)).flex_none().text_color(theme.text_muted)
}

impl TabBar {
    /// Takes this frame's tabs; another workspace's tabs replace the old ones without motion. A reorder slides
    /// the tabs from where they were drawn.
    fn sync(&mut self, workspace: Option<&str>, tabs: Vec<Shown>, now: Instant) {
        if self.workspace.as_deref() != workspace {
            self.workspace = workspace.map(str::to_string);
            self.entries = tabs.into_iter().map(|tab| Entry::new(tab, None)).collect();
            self.wash = None;
            return;
        }
        let prev: Vec<&str> = self.entries.iter().map(|e| e.tab.id.as_str()).collect();
        let ids: Vec<&str> = tabs.iter().map(|t| t.id.as_str()).collect();
        let kept = |list: &[&str]| -> Vec<String> {
            list.iter().filter(|id| prev.contains(id) && ids.contains(id)).map(|id| id.to_string()).collect()
        };
        let reordered = kept(&prev) != kept(&ids);
        let order = merge(&prev, &ids);
        let entries = order
            .into_iter()
            .filter_map(|(id, phase)| {
                let old = self.entries.iter().find(|e| e.tab.id == id);
                let fresh = || tabs.iter().find(|t| t.id == id).cloned();
                match phase {
                    Phase::New => Some(Entry::new(fresh()?, Some(now))),
                    Phase::Stay => {
                        let old = old?;
                        let moved = if reordered {
                            let lifted = old.lifted(now);
                            let duration = if lifted { LAND } else { SLIDE };
                            Some(Move { from: old.drawn, started: now, duration, lifted })
                        } else {
                            old.moved
                        };
                        Some(Entry { tab: fresh()?, exit: None, moved, ..old.clone() })
                    }
                    Phase::Gone => {
                        let old = old?;
                        let exit = old.exit.unwrap_or(now);
                        (now.duration_since(exit) < EXIT).then(|| Entry { exit: Some(exit), ..old.clone() })
                    }
                }
            })
            .collect();
        self.entries = entries;
    }

    /// Sends the tab `id` flying home from `from` (content x, offset from the row).
    fn land(&mut self, id: &str, from: (f32, f32), now: Instant) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.tab.id == id) {
            e.moved = Some(Move { from, started: now, duration: LAND, lifted: true });
        }
    }

    /// Each entry's slot x, full width and presence, laid out left to right; records where each is drawn
    /// (its slot, or on its way there).
    fn layout(&mut self, now: Instant) -> Vec<(f32, f32, f32)> {
        let mut x = 0.;
        self.entries
            .iter_mut()
            .map(|e| {
                let k = e.presence(now);
                let at = x;
                x += (e.tab.width + GAP) * k;
                e.drawn = match e.moved {
                    Some(m) => {
                        let t = ease_out(progress(m.started, m.duration, now));
                        (m.from.0 + (at - m.from.0) * t, m.from.1 * (1. - t))
                    }
                    None => (at, 0.),
                };
                (at, e.tab.width, k)
            })
            .collect()
    }

    /// Where the active wash is drawn this frame (x, width): sliding from where it was toward `selected`.
    /// Hidden while the tab is dragged or flies home (the tab carries its own look), it then reappears in the
    /// slot without sliding.
    fn wash(&mut self, selected: Option<&str>, dragged: Option<&str>, layout: &[(f32, f32, f32)], now: Instant) -> Option<(f32, f32)> {
        let i = self.entries.iter().position(|e| Some(e.tab.id.as_str()) == selected && e.exit.is_none())?;
        if dragged == selected || self.entries[i].lifted(now) {
            self.wash = None;
            return None;
        }
        let (_, w, k) = layout[i];
        let target = (self.entries[i].drawn.0, w * k);
        let wash = match self.wash.take() {
            Some(w) if w.tab == self.entries[i].tab.id => w,
            Some(w) => Wash { tab: self.entries[i].tab.id.clone(), from: w.shown, started: now, shown: w.shown },
            None => Wash { tab: self.entries[i].tab.id.clone(), from: target, started: now - SLIDE, shown: target },
        };
        let t = ease_out(progress(wash.started, SLIDE, now));
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        let shown = (lerp(wash.from.0, target.0), lerp(wash.from.1, target.1));
        self.wash = Some(Wash { shown, ..wash });
        Some(shown)
    }

    fn sliding(&self, now: Instant) -> bool {
        self.wash.as_ref().is_some_and(|w| now.duration_since(w.started) < SLIDE)
    }
}

/// The favicon: the mark on a filled tile of its color, the same size for every kind.
fn favicon(t: &Shown) -> Div {
    tile(t.icon, t.tint)
}

/// A tab's favicon for `agent`, for lists of agents elsewhere.
pub(crate) fn agent_favicon(agent: Option<&str>, theme: &Theme) -> Div {
    tile(agent_icon(agent), agent_tint(agent, theme))
}

fn tile(icon: &'static str, (bg, fg): (Hsla, Hsla)) -> Div {
    div()
        .size(px(ICON))
        .flex_none()
        .rounded(px(5.))
        .flex()
        .items_center()
        .justify_center()
        .bg(bg)
        // a black tile would sink into a dark bar: a hairline keeps its edge
        .when(bg.l < 0.1, |el| el.border_1().border_color(gpui::white().opacity(0.14)))
        .child(gpui::svg().path(icon).size(px(11.)).text_color(fg))
}

/// A tab's face at full width: favicon, label, trailing slot.
fn face(theme: &Theme, t: &Shown, width: f32, selected: bool) -> Div {
    div()
        .w(px(width))
        .h(px(TAB_H))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(INNER_GAP))
        .pl(px(PAD_L))
        .pr(px(PAD_R))
        .rounded(px(8.))
        .font_family(theme.font_ui.clone())
        .text_size(px(FONT_SIZE))
        .text_color(if selected { theme.text } else { theme.text_muted })
        .child(favicon(t))
}

fn dot_slot(theme: &Theme, status: AgentStatus) -> Div {
    div().size(px(SLOT)).flex_none().flex().items_center().justify_center().child(status_dot(status, theme))
}

/// A tab being dragged along the tab bar: its own face under the pointer, lifted on a card.
#[derive(Clone)]
pub(super) struct DraggedTab {
    tab: Shown,
    index: usize,
    theme: Theme,
    /// Where the pointer grabbed the tab, from its top-left.
    grab: gpui::Point<Pixels>,
    drag: Rc<RefCell<Option<(String, gpui::Point<Pixels>)>>>,
}

impl Render for DraggedTab {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // gpui draws the ghost at the pointer less the grab: remember it, a drop flies home from there
        *self.drag.borrow_mut() = Some((self.tab.id.clone(), window.mouse_position() - self.grab));
        let theme = &self.theme;
        face(theme, &self.tab, self.tab.width, true)
            .bg(theme.card)
            .border_1()
            .border_color(theme.border)
            .shadow_lg()
            .child(div().flex_1().min_w_0().truncate().child(self.tab.label.clone()))
            .when(self.tab.zoomed, |el| el.child(zoom_mark(theme)))
            .child(dot_slot(theme, self.tab.status))
    }
}

impl Shell {
    pub(super) fn render_tab_bar(
        &mut self,
        theme: &Theme,
        shown_ws: Option<String>,
        shown_tab: Option<String>,
        sidebar_w: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let now = Instant::now();
        let font = gpui::font(theme.font_ui.clone());
        let tabs: Vec<Shown> = match (&self.snapshot, &shown_ws) {
            (Some(s), Some(ws)) => {
                let agents = s.agents();
                s.tabs_of(ws)
                    .into_iter()
                    .map(|t| {
                        let run = gpui::TextRun {
                            len: t.label.len(),
                            font: font.clone(),
                            color: theme.text,
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        };
                        let label_w = window.text_system().shape_line(t.label.clone().into(), px(FONT_SIZE), &[run], None).width;
                        let agent = agents.iter().find(|p| p.tab_id == t.tab_id).and_then(|p| p.agent.as_deref());
                        let zoomed = s.layouts.iter().any(|l| l.tab_id == t.tab_id && l.zoomed);
                        Shown {
                            id: t.tab_id.clone(),
                            label: t.label.clone(),
                            status: t.agent_status,
                            icon: agent_icon(agent),
                            tint: agent_tint(agent, theme),
                            zoomed,
                            width: tab_width(f32::from(label_w), zoomed),
                        }
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        self.tab_bar.sync(shown_ws.as_deref(), tabs, now);

        // The drag ended (dropped on a tab or anywhere): the tab flies home from where its ghost was let go.
        let bar = self.tab_bar.scroll.bounds();
        let (bar_x, bar_y) = (f32::from(bar.origin.x) + f32::from(self.tab_bar.scroll.offset().x), f32::from(bar.origin.y) + TAB_TOP);
        let dragging = cx.has_active_drag();
        let dragged = self.tab_bar.drag.borrow().clone();
        if let (false, Some((id, at))) = (dragging, &dragged) {
            self.tab_bar.drag.borrow_mut().take();
            self.tab_bar.land(id, (f32::from(at.x) - bar_x, f32::from(at.y) - bar_y), now);
        }
        let lifted_id = dragged.filter(|_| dragging).map(|(id, _)| id);

        let layout = self.tab_bar.layout(now);
        let wash = self.tab_bar.wash(shown_tab.as_deref(), lifted_id.as_deref(), &layout, now);
        // A newly shown tab scrolls into view, followed while it grows in (gpui clamps the offset to the content).
        if let Some(i) = self.tab_bar.entries.iter().position(|e| Some(&e.tab.id) == shown_tab.as_ref() && e.exit.is_none()) {
            let entry = &self.tab_bar.entries[i];
            if self.tab_bar.revealed.as_ref() != shown_tab.as_ref() || entry.enter.is_some_and(|s| now.duration_since(s) < ENTER) {
                let scroll = &self.tab_bar.scroll;
                let view = f32::from(scroll.bounds().size.width);
                if view > 0. {
                    if let Some(to) = reveal(-f32::from(scroll.offset().x), view, layout[i].0, entry.tab.width) {
                        scroll.set_offset(gpui::point(px(-to), scroll.offset().y));
                    }
                    self.tab_bar.revealed = shown_tab.clone();
                }
            }
        }
        if self.tab_bar.entries.iter().any(|e| e.moving(now)) || self.tab_bar.sliding(now) {
            window.request_animation_frame();
        }

        let mut index = 0;
        let mut items = Vec::new();
        let mut flying = Vec::new();
        for (entry, (slot_x, full_w, k)) in self.tab_bar.entries.clone().into_iter().zip(layout) {
            let t = entry.tab.clone();
            let closing = entry.exit.is_some();
            let selected = !closing && shown_tab.as_deref() == Some(t.id.as_str());
            let editing = (!closing).then(|| self.rename_field(&RenameTarget::Tab(t.id.clone()), cx)).flatten();
            let draggable = editing.is_none();
            let full_w = if draggable { full_w } else { MAX_W };
            // While dragged, the ghost carries the tab; while it flies home, a copy over everything does.
            let lifted = lifted_id.as_deref() == Some(t.id.as_str()) || entry.lifted(now);
            if entry.lifted(now) {
                let t_land = ease_out(entry.moved.map_or(1., |m| progress(m.started, m.duration, now)));
                let at = gpui::point(px(bar_x + entry.drawn.0), px(bar_y + entry.drawn.1));
                flying.push(
                    gpui::deferred(
                        gpui::anchored().position(at).child(
                            face(theme, &t, full_w, selected)
                                // the card settles into the tab's own look: the wash if active, else bare
                                .bg(mix(theme.card, if selected { theme.selected } else { theme.card.opacity(0.) }, t_land))
                                .border_1()
                                .border_color(theme.border.opacity(1. - t_land))
                                .child(div().flex_1().min_w_0().truncate().child(t.label.clone()))
                                .when(t.zoomed, |el| el.child(zoom_mark(theme)))
                                .child(dot_slot(theme, t.status)),
                        ),
                    )
                    .with_priority(1)
                    .into_any_element(),
                );
            }
            let group = SharedString::from(format!("tab-row-{}", t.id));
            // The face keeps its full width and is clipped while the tab grows or shrinks.
            let content = face(theme, &t, full_w, selected)
                .map(|el| match editing {
                    Some(field) => el.child(field),
                    None => el.child(div().flex_1().min_w_0().truncate().child(t.label.clone())),
                })
                .when(t.zoomed && draggable, |el| el.child(zoom_mark(theme)))
                .child(
                    div()
                        .id(SharedString::from(format!("tab-x-{}", t.id)))
                        .tooltip(tip("Close tab", Some("CloseTab")))
                        .size(px(SLOT))
                        .flex_none()
                        .relative()
                        .rounded(px(6.))
                        .hover(|s| s.bg(theme.hover))
                        .child(dot_slot(theme, t.status).absolute().inset_0().group_hover(group.clone(), |s| s.opacity(0.)))
                        .child(
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .opacity(0.)
                                .group_hover(group.clone(), |s| s.opacity(1.))
                                .child(gpui::svg().path("icons/close.svg").size(px(12.)).text_color(theme.text_muted)),
                        )
                        .when(!closing, |el| {
                            let id = t.id.clone();
                            // the tab owns a drag: claim the press so it closes instead of starting one
                            el.on_mouse_down(MouseButton::Left, |_, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                            })
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                cx.stop_propagation();
                                this.send(Command::CloseTab { tab_id: id.clone() }, cx, |_, _, _| {});
                            }))
                        }),
                );
            let tab = div()
                .id(SharedString::from(format!("tab-{}", t.id)))
                .group(group)
                .relative()
                .left(px(entry.drawn.0 - slot_x))
                .flex_none()
                .h(px(TAB_H))
                .w(px(full_w * k))
                .mr(px(GAP * k))
                .overflow_hidden()
                .opacity(if lifted { 0. } else { k })
                .rounded(px(8.))
                .cursor_pointer()
                // off the titlebar's drag strip: a press here drags the tab, not the window; the wheel still scrolls the bar
                .block_mouse_except_scroll()
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .when(!selected && !closing, |el| el.hover(|s| s.bg(theme.hover)))
                .child(content);
            if closing {
                items.push(tab.into_any_element());
                continue;
            }
            let i = index;
            index += 1;
            let id = t.id.clone();
            let dragged = draggable.then(|| DraggedTab {
                tab: t.clone(),
                index: i,
                theme: theme.clone(),
                grab: gpui::Point::default(),
                drag: self.tab_bar.drag.clone(),
            });
            let drop_bg = theme.accent.opacity(0.15);
            let label = t.label.clone();
            items.push(
                tab.on_mouse_down(MouseButton::Right, cx.listener({
                    let id = id.clone();
                    move |this, e: &MouseDownEvent, _, cx| {
                        this.tab_menu = Some(crate::ui::Popup::new((id.clone(), e.position)));
                        cx.notify();
                    }
                }))
                // Middle-click closes, like every tab strip (Zeron §1.10).
                .on_mouse_down(MouseButton::Middle, cx.listener({
                    let id = id.clone();
                    move |this, _: &MouseDownEvent, _, cx| {
                        this.send(Command::CloseTab { tab_id: id.clone() }, cx, |_, _, _| {});
                    }
                }))
                .when_some(dragged, |el, dragged| {
                    el.on_drag(dragged, |d, grab, _, cx| {
                        let d = DraggedTab { grab, ..d.clone() };
                        cx.new(|_| d)
                    })
                })
                .map(|el| pane_drag::accept_pane(el, &id, drop_bg, cx))
                .drag_over::<DraggedTab>(move |s, d, _, _| if d.index == i { s } else { s.bg(drop_bg) })
                // Dropped on a tab, the dragged one takes its place: herdr's slot is after it when moving right.
                .on_drop(cx.listener(move |this, d: &DraggedTab, _, cx| {
                    if d.index != i {
                        let insert_index = if d.index < i { i + 1 } else { i };
                        this.send(Command::MoveTab { tab_id: d.tab.id.clone(), insert_index }, cx, |_, _, _| {});
                    }
                    cx.notify();
                }))
                .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
                    if e.click_count() == 2 {
                        this.start_rename(RenameTarget::Tab(id.clone()), label.clone(), window, cx);
                        return;
                    }
                    this.selected_tab = Some(id.clone());
                    cx.notify();
                }))
                .into_any_element(),
            );
        }

        let scroll = &self.tab_bar.scroll;
        div()
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            // opaque, so the edge fades (same color) blend in exactly
            .bg(theme.surface)
            .flex()
            .items_center()
            .gap(px(2.))
            // the 28 px `+` sits as far from the right edge as from the top and bottom, and clear of the window's
            // buttons while no review panel sits under them
            .pr(px(TAB_TOP + if self.review_open { 0. } else { WINDOW_CONTROLS_W }))
            // clear the traffic lights and the sidebar toggle once the sidebar no longer sits under them
            .pl(px((TITLEBAR_CLUSTER - sidebar_w).max(8.)))
            .child(
                div()
                    // off macOS the `+` follows the last tab, apart from the window's buttons at the right end
                    .when(cfg!(target_os = "macos"), |el| el.flex_1())
                    .min_w_0()
                    .h_full()
                    .relative()
                    .child(
                        div()
                            .id("tabbar")
                            // GPUI tells Windows the control area of every hitbox under the pointer, scroll-only
                            // blockers included: without blocking here a press on a tab is the drag strip's, and
                            // Windows moves the window. The bar still scrolls, being the blocker.
                            .when(cfg!(windows), |el| el.occlude())
                            .size_full()
                            .flex()
                            .items_center()
                            .overflow_x_scroll()
                            .track_scroll(scroll)
                            // re-render so the edge fades follow the scroll position
                            .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.notify()))
                            // the active wash, drawn under the tabs, slides between them
                            .when_some(wash, |el, (x, w)| {
                                el.child(
                                    div()
                                        .absolute()
                                        .top(px(TAB_TOP))
                                        .left(px(x))
                                        .w(px(w))
                                        .h(px(TAB_H))
                                        .rounded(px(8.))
                                        .bg(theme.selected),
                                )
                            })
                            .children(items),
                    )
                    // ponytail: gpui has no alpha mask, so the fades are gradients of the bar's own color over the tabs
                    .map(|el| {
                        let (offset, max) = (-f32::from(scroll.offset().x), f32::from(scroll.max_offset().x));
                        let fade = |angle: f32| {
                            div().absolute().top_0().bottom_0().w(px(32.)).bg(linear_gradient(
                                angle,
                                linear_color_stop(theme.surface, 0.),
                                linear_color_stop(theme.surface.opacity(0.), 1.),
                            ))
                        };
                        el.when(offset > 0.5, |el| el.child(fade(90.).left_0()))
                            .when(offset < max - 0.5, |el| el.child(fade(270.).right_0()))
                    }),
            )
            .children(flying)
            // `+` stays in view; only the tabs scroll
            .when_some(shown_ws, |el, ws| {
                el.child(
                    icon_button(theme, "new-tab", "+")
                        .tooltip(tip("New tab", Some("NewTab")))
                        .size(px(TAB_H))
                        .rounded(px(8.))
                        .text_size(px(19.))
                        .block_mouse_except_scroll()
                        .when(cfg!(windows), |el| el.occlude()) // as the tabs, off the drag strip
                        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.new_tab(ws.clone(), cx))),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(id: &str) -> Shown {
        Shown { id: id.into(), label: id.into(), status: AgentStatus::Idle, icon: "", tint: (gpui::black(), gpui::white()), zoomed: false, width: 100. }
    }

    #[test]
    fn closed_tabs_stay_in_place_and_new_ones_join() {
        use Phase::*;
        let got = merge(&["a", "b", "c", "d"], &["a", "c", "d", "e"]);
        let want = [("a", Stay), ("b", Gone), ("c", Stay), ("d", Stay), ("e", New)];
        assert_eq!(got, want.map(|(id, p)| (id.to_string(), p)));
        // the first tab closing keeps the front; neighbours closing together keep their order
        let got = merge(&["a", "b", "c"], &["c"]);
        assert_eq!(got, [("a", Gone), ("b", Gone), ("c", Stay)].map(|(id, p)| (id.to_string(), p)));
        // a reorder is herdr's new order
        assert_eq!(merge(&["a", "b"], &["b", "a"]), [("b", Stay), ("a", Stay)].map(|(id, p)| (id.to_string(), p)));
    }

    #[test]
    fn the_shown_tab_scrolls_into_view() {
        assert_eq!(reveal(0., 300., 100., 100.), None);
        assert_eq!(reveal(0., 300., 400., 100.), Some(232.)); // off the right: its right edge plus a fade
        assert_eq!(reveal(500., 300., 100., 100.), Some(68.)); // off the left
        assert_eq!(reveal(50., 300., 10., 100.), Some(0.));
    }

    #[test]
    fn agents_get_their_mark() {
        assert_eq!(agent_icon(None), "icons/prompt.svg");
        assert_eq!(agent_icon(Some("claude")), "icons/claude-mark.svg");
        assert_eq!(agent_icon(Some("codex")), "icons/openai-mark.svg");
        assert_eq!(agent_icon(Some("kimi")), "icons/kimi-mark.svg");
        assert_eq!(agent_icon(Some("letta")), "icons/bot-filled.svg");
    }

    #[test]
    fn a_closing_tab_shrinks_out_then_goes() {
        let t0 = Instant::now();
        let mut bar = TabBar::default();
        bar.sync(Some("w"), vec![tab("a"), tab("b")], t0);
        bar.sync(Some("w"), vec![tab("b")], t0);
        assert_eq!(bar.entries.len(), 2);
        assert_eq!(bar.layout(t0)[1].0, 102.); // `a` still holds its slot as it starts to close
        bar.sync(Some("w"), vec![tab("b")], t0 + EXIT);
        assert_eq!(bar.entries.len(), 1);
        // another workspace swaps the tabs without motion
        bar.sync(Some("v"), vec![tab("c")], t0 + EXIT);
        assert_eq!(bar.entries[0].presence(t0 + EXIT), 1.);
    }

    #[test]
    fn reordered_and_dropped_tabs_slide_home() {
        let t0 = Instant::now();
        let mut bar = TabBar::default();
        bar.sync(Some("w"), vec![tab("a"), tab("b")], t0);
        bar.layout(t0);
        bar.sync(Some("w"), vec![tab("b"), tab("a")], t0);
        bar.layout(t0);
        // `a` starts where it was drawn and ends in its new slot
        assert_eq!(bar.entries[1].drawn, (0., 0.));
        bar.layout(t0 + SLIDE);
        assert_eq!(bar.entries[1].drawn, (102., 0.));
        // dropped 40 px below the bar, `b` flies home from there
        bar.land("b", (300., 40.), t0);
        bar.layout(t0);
        assert_eq!(bar.entries[0].drawn, (300., 40.));
        assert!(bar.entries[0].lifted(t0));
        bar.layout(t0 + LAND);
        assert_eq!(bar.entries[0].drawn, (0., 0.));
    }
}
