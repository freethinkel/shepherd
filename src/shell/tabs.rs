//! A strip of tabs, the tab bar's and the right panel's: Zeron's terminal-panel tabs (28 px, pills, a wash under
//! the active one), each with its favicon, sized to its label. A new tab grows in and a closed one stays mounted while
//! it shrinks out (the `Popup` recipe); a switch, a reorder and a drop take effect at once, as a tab sliding made it
//! hard to follow which one was which. What a tab shows and does is its owner's.
use super::*;
use gpui::AnyElement;
use std::cell::RefCell;

/// Zeron's `motion::COLLAPSE` for a tab growing in or shrinking out.
const ENTER: Duration = Duration::from_millis(180);
const EXIT: Duration = Duration::from_millis(150);
/// How long a dropped order is shown, at most, before herdr's own catches up.
const PENDING: Duration = Duration::from_millis(1500);

pub(super) const TAB_H: f32 = 28.;
/// The tabs' top inside the bar.
pub(super) const TAB_TOP: f32 = (TITLEBAR_HEIGHT - TAB_H) / 2.;
/// Space after each tab; it collapses with a closing tab so neighbours close in smoothly.
const GAP: f32 = 2.;
const PAD_L: f32 = 6.;
const PAD_R: f32 = 4.;
const INNER_GAP: f32 = 6.;
/// The favicon tile: Rex's shape, wider than tall.
const TILE_W: f32 = 20.;
const TILE_H: f32 = 16.;
/// How far each tile of a stack sits right of the one over it, and the gap the bar's color cuts around it.
const STACK_STEP: f32 = 15.;
const STACK_RING: f32 = 1.5;
/// The trailing slot: the status dot, swapped in place for `×` while the tab is hovered.
const SLOT: f32 = 20.;
const ZOOM_MARK: f32 = 12.;
const MIN_W: f32 = 96.;
const MAX_W: f32 = 220.;
const FONT_SIZE: f32 = 12.5;

/// A favicon: the mark and its tile and mark colors.
pub(super) type Mark = (&'static str, (Hsla, Hsla));

/// What leads a tab.
#[derive(Clone)]
pub(super) enum Leading {
    /// Tiles, overlapped, the first on top.
    Marks(Vec<Mark>),
    /// A site's own icon.
    Image(std::sync::Arc<gpui::Image>),
    /// Something on its way.
    Spinner,
}

/// One tab as drawn.
#[derive(Clone)]
pub(super) struct Tab {
    pub(super) id: String,
    /// The label as its owner has it: what a rename starts from.
    pub(super) label: String,
    /// The label as drawn and its faint ranges.
    pub(super) name: (String, Vec<std::ops::Range<usize>>),
    pub(super) leading: Leading,
    /// Fills the trailing slot until the tab is hovered.
    pub(super) status: Option<AgentStatus>,
    /// One of its panes is zoomed to fill it: the other panes are hidden.
    pub(super) zoomed: bool,
    /// The full width: label measured, clamped.
    pub(super) width: f32,
}

impl Tab {
    /// A tab with a plain title and one mark or none.
    pub(super) fn titled(id: &str, title: String, leading: Leading, theme: &Theme, window: &Window) -> Self {
        let width = tab_width(label_width(&title, theme, window), false, 0);
        Tab { id: id.to_string(), name: (title.clone(), Vec::new()), label: title, leading, status: None, zoomed: false, width }
    }
}

/// The tile of what has no color of its own.
pub(super) fn plain_tint(theme: &Theme) -> (Hsla, Hsla) {
    (theme.text_muted.opacity(0.55), theme.surface)
}

/// Strip state that outlives a frame: the scroll, the tabs as last drawn (closing ones included), and the tab being
/// dragged with its ghost's top-left in the window (written by the ghost as it follows the pointer).
#[derive(Default)]
pub(super) struct Tabs {
    pub(super) scroll: gpui::ScrollHandle,
    /// Whose tabs these are.
    pub(super) scope: Option<String>,
    pub(super) entries: Vec<Entry>,
    drag: Rc<RefCell<Option<(String, gpui::Point<Pixels>)>>>,
    /// The tab last scrolled into view; the strip follows a newly shown tab once, then leaves the scroll to you.
    revealed: Option<String>,
    /// The tab held in a drag and the slot its gap holds among the others, as the place bar's buttons do.
    pub(super) held: Option<(String, usize)>,
    /// The order a drop asked for, shown until its owner's is it.
    pending: Option<(Vec<String>, Instant)>,
}

/// A tab placed for this frame.
pub(super) struct Placed {
    pub(super) tab: Tab,
    /// Where it is drawn, from where the row lays it out.
    shift: f32,
    /// How far it is in: 0 collapsed, 1 full width.
    presence: f32,
    pub(super) closing: bool,
    selected: bool,
    /// In a drag: a placeholder keeps its gap and the ghost carries the tab.
    holding: bool,
}

#[derive(Clone)]
pub(super) struct Entry {
    pub(super) tab: Tab,
    enter: Option<Instant>,
    pub(super) exit: Option<Instant>,
    /// Where it is drawn: content x.
    drawn: f32,
}

impl Entry {
    fn new(tab: Tab, enter: Option<Instant>) -> Self {
        Entry { tab, enter, exit: None, drawn: 0. }
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
        self.exit.is_some_and(|s| running(s, EXIT)) || self.enter.is_some_and(|s| running(s, ENTER))
    }
}

fn progress(since: Instant, d: Duration, now: Instant) -> f32 {
    (now.duration_since(since).as_secs_f32() / d.as_secs_f32()).min(1.)
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

/// The width of `label` as a tab draws it.
pub(super) fn label_width(label: &str, theme: &Theme, window: &Window) -> f32 {
    let run = gpui::TextRun { len: label.len(), font: gpui::font(theme.font_ui.clone()), color: theme.text, background_color: None, underline: None, strikethrough: None };
    f32::from(window.text_system().shape_line(label.to_string().into(), px(FONT_SIZE), &[run], None).width)
}

/// A tab's name, its faint spans drawn so.
fn name_text(t: &Tab, theme: &Theme) -> gpui::StyledText {
    let faint = gpui::HighlightStyle { color: Some(theme.text_faint), ..Default::default() };
    gpui::StyledText::new(t.name.0.clone()).with_highlights(t.name.1.iter().map(|r| (r.clone(), faint)).collect::<Vec<_>>())
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

/// `more` tiles after the favicon widen the tab past [`MAX_W`], so they don't take the label's room.
pub(super) fn tab_width(label_w: f32, zoomed: bool, more: usize) -> f32 {
    let mark = if zoomed { ZOOM_MARK + INNER_GAP } else { 0. };
    let tiles = more as f32 * STACK_STEP;
    (PAD_L + TILE_W + tiles + INNER_GAP + label_w + mark + INNER_GAP + SLOT + PAD_R).clamp(MIN_W, MAX_W + tiles).ceil()
}

/// After the label of a tab with a zoomed pane, the zoom action's icon.
fn zoom_mark(theme: &Theme) -> gpui::Svg {
    gpui::svg().path("icons/expand-arrows.svg").size(px(ZOOM_MARK)).flex_none().text_color(theme.text_muted)
}

impl Tabs {
    /// Takes this frame's tabs; another scope's replace the old ones without motion.
    fn sync(&mut self, scope: Option<&str>, tabs: Vec<Tab>, now: Instant) {
        if self.scope.as_deref() != scope {
            self.scope = scope.map(str::to_string);
            self.entries = tabs.into_iter().map(|tab| Entry::new(tab, None)).collect();
            return;
        }
        let prev: Vec<&str> = self.entries.iter().map(|e| e.tab.id.as_str()).collect();
        let ids: Vec<&str> = tabs.iter().map(|t| t.id.as_str()).collect();
        let order = merge(&prev, &ids);
        let entries = order
            .into_iter()
            .filter_map(|(id, phase)| {
                let old = self.entries.iter().find(|e| e.tab.id == id);
                let fresh = || tabs.iter().find(|t| t.id == id).cloned();
                match phase {
                    Phase::New => Some(Entry::new(fresh()?, Some(now))),
                    Phase::Stay => Some(Entry { tab: fresh()?, exit: None, ..old?.clone() }),
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

    /// Each entry's slot x (where the row lays it out), full width and presence, and records where each is drawn:
    /// in the order shown, which is the list's, or with the tab `held` (its id, and its middle's x) in the slot its
    /// middle has reached, the others closing up around its gap, or a dropped order until the list's is it. True
    /// when the held tab took another slot.
    fn layout(&mut self, now: Instant, held: Option<(&str, f32)>) -> (Vec<(f32, f32, f32)>, bool) {
        let ks: Vec<f32> = self.entries.iter().map(|e| e.presence(now)).collect();
        let step = |i: usize| (self.entries[i].tab.width + GAP) * ks[i];
        let live: Vec<usize> = (0..self.entries.len()).filter(|i| self.entries[*i].exit.is_none()).collect();
        let list: Vec<String> = live.iter().map(|i| self.entries[*i].tab.id.clone()).collect();
        let mut slid = false;
        let shown: Vec<usize> = match held.and_then(|(id, mid)| Some((live.iter().position(|i| self.entries[*i].tab.id == id)?, mid))) {
            Some((from, mid)) => {
                let mut others = live.clone();
                let h = others.remove(from);
                let id = self.entries[h].tab.id.clone();
                // the others as drawn around its gap: one after the gap goes before it once the held tab's right edge
                // passes that one's middle, one before goes after once its left edge does
                let gap = self.held.as_ref().filter(|(was, _)| *was == id).map_or(from, |(_, at)| *at);
                let half = self.entries[h].tab.width / 2.;
                let (mut x, mut slot) = (0., 0);
                for (n, i) in others.iter().enumerate() {
                    if n == gap {
                        x += step(h);
                    }
                    let middle = x + self.entries[*i].tab.width / 2.;
                    slot += usize::from(if n < gap { mid - half >= middle } else { mid + half > middle });
                    x += step(*i);
                }
                slid = self.held.as_ref().is_some_and(|(was, at)| *was == id && *at != slot);
                self.held = Some((id, slot));
                others.insert(slot, h);
                others
            }
            None => {
                self.held = None;
                let asked = self.pending.as_ref().filter(|(order, at)| {
                    let (mut a, mut b) = (order.clone(), list.clone());
                    a.sort();
                    b.sort();
                    a == b && *order != list && now.saturating_duration_since(*at) < PENDING
                });
                match asked {
                    Some((order, _)) => order.iter().filter_map(|id| live.iter().copied().find(|i| self.entries[*i].tab.id == *id)).collect(),
                    None => {
                        self.pending = None;
                        live.clone()
                    }
                }
            }
        };
        // closing tabs keep their places in the row
        let mut order = shown.clone();
        for i in (0..self.entries.len()).filter(|i| self.entries[*i].exit.is_some()) {
            order.insert(i.min(order.len()), i);
        }
        let mut targets = vec![0.; self.entries.len()];
        let mut x = 0.;
        for i in order {
            targets[i] = x;
            x += step(i);
        }
        let mut natural = 0.;
        let layout = self
            .entries
            .iter_mut()
            .enumerate()
            .map(|(i, e)| {
                let at = natural;
                natural += (e.tab.width + GAP) * ks[i];
                e.drawn = targets[i];
                (at, e.tab.width, ks[i])
            })
            .collect();
        (layout, slid)
    }

    /// A drop let go of with the held tab in its slot: that order is shown until herdr's is it. The move it asks
    /// for, `(from, to)` among the tabs not closing.
    pub(super) fn drop_held(&mut self, now: Instant) -> Option<(usize, usize)> {
        let (id, slot) = self.held.take()?;
        let mut order: Vec<String> = self.entries.iter().filter(|e| e.exit.is_none()).map(|e| e.tab.id.clone()).collect();
        let from = order.iter().position(|t| *t == id)?;
        let tab = order.remove(from);
        order.insert(slot.min(order.len()), tab);
        (from != slot).then(|| {
            self.pending = Some((order, now));
            (from, slot)
        })
    }
}
impl Tabs {
    /// Takes this frame's tabs and places them, `shown` the selected one: the held tab's gap follows the pointer, and
    /// a newly shown tab scrolls into view, followed while it grows in (gpui clamps the offset to the content).
    pub(super) fn frame(&mut self, scope: Option<&str>, tabs: Vec<Tab>, shown: Option<&str>, window: &mut Window, cx: &App) -> Vec<Placed> {
        let now = Instant::now();
        self.sync(scope, tabs, now);

        let bar_x = f32::from(self.scroll.bounds().origin.x) + f32::from(self.scroll.offset().x);
        let dragging = cx.has_active_drag();
        let dragged = self.drag.borrow().clone();
        // the drag ended, dropped on a tab or anywhere
        if !dragging {
            self.drag.borrow_mut().take();
        }
        // the held tab's middle, where its ghost is, in the row
        let held = dragged.as_ref().filter(|_| dragging).and_then(|(id, at)| {
            let width = self.entries.iter().find(|e| e.tab.id == *id)?.tab.width;
            Some((id.clone(), f32::from(at.x) - bar_x + width / 2.))
        });
        let lifted = dragged.filter(|_| dragging).map(|(id, _)| id);

        let (layout, slid) = self.layout(now, held.as_ref().map(|(id, mid)| (id.as_str(), *mid)));
        if slid {
            haptic();
        }
        if let Some(entry) = self.entries.iter().find(|e| Some(e.tab.id.as_str()) == shown && e.exit.is_none())
            && (self.revealed.as_deref() != shown || entry.enter.is_some_and(|s| now.duration_since(s) < ENTER))
        {
            let view = f32::from(self.scroll.bounds().size.width);
            if view > 0. {
                if let Some(to) = reveal(-f32::from(self.scroll.offset().x), view, entry.drawn, entry.tab.width) {
                    self.scroll.set_offset(gpui::point(px(-to), self.scroll.offset().y));
                }
                self.revealed = shown.map(str::to_string);
            }
        }
        if self.entries.iter().any(|e| e.moving(now)) {
            window.request_animation_frame();
        }
        self.entries
            .iter()
            .zip(layout)
            .map(|(e, (slot_x, _, presence))| {
                let closing = e.exit.is_some();
                Placed {
                    tab: e.tab.clone(),
                    shift: e.drawn - slot_x,
                    presence,
                    closing,
                    selected: !closing && shown == Some(e.tab.id.as_str()),
                    holding: lifted.as_deref() == Some(e.tab.id.as_str()),
                }
            })
            .collect()
    }

    /// Lets `el`, tab `p`, be dragged along the strip; `dropped` lands it.
    pub(super) fn liftable(&self, el: Stateful<Div>, p: &Placed, theme: &Theme, dropped: fn(&mut Shell, &mut Context<Shell>), cx: &mut Context<Shell>) -> Stateful<Div> {
        let d = DraggedTab { tab: p.tab.clone(), theme: theme.clone(), grab: gpui::Point::default(), drag: self.drag.clone() };
        el.on_drag(d, |d, grab, _, cx| {
            let d = DraggedTab { grab, ..d.clone() };
            cx.new(|_| d)
        })
        // a tab's hitbox hides the row's from the drop: each takes it too
        .on_drop(cx.listener(move |this, _: &DraggedTab, _, cx| dropped(this, cx)))
    }
}

/// The favicon: the mark on a filled tile of its color, the same size for every kind; with more marks, a stack.
fn leading(t: &Tab, ring: Hsla, theme: &Theme, window: &mut Window) -> AnyElement {
    // the tile's room, so a label starts in the same place whatever leads it
    let slot = || div().w(px(TILE_W)).h(px(TILE_H)).flex_none().flex().items_center().justify_center();
    match &t.leading {
        Leading::Marks(marks) => match marks.as_slice() {
            [(icon, tint)] => tile(icon, *tint).into_any_element(),
            marks => stack(marks, ring).into_any_element(),
        },
        Leading::Image(image) => slot().child(gpui::img(image.clone()).size(px(16.)).rounded(px(3.))).into_any_element(),
        Leading::Spinner => slot().child(crate::ui::loader::mini_spinner(2., theme.text_faint, window)).into_any_element(),
    }
}

/// A tab's tiles overlapped, the first on top, each in a `ring` of what's behind them.
fn stack(marks: &[Mark], ring: Hsla) -> Div {
    let more = marks.len().saturating_sub(1) as f32;
    div().relative().flex_none().w(px(TILE_W + more * STACK_STEP)).h(px(TILE_H)).children(marks.iter().enumerate().rev().map(|(i, (icon, tint))| {
        div().absolute().left(px(i as f32 * STACK_STEP - STACK_RING)).top(px(-STACK_RING)).p(px(STACK_RING)).rounded(px(5. + STACK_RING)).bg(ring).child(tile(icon, *tint))
    }))
}

pub(super) fn tile(icon: &'static str, (bg, fg): (Hsla, Hsla)) -> Div {
    div()
        .w(px(TILE_W))
        .h(px(TILE_H))
        .flex_none()
        .rounded(px(5.))
        .flex()
        .items_center()
        .justify_center()
        // lit from above, as Rex's tiles are; the hairline keeps a dark tile's edge on a dark bar
        .bg(linear_gradient(180., linear_color_stop(bg.blend(gpui::white().opacity(0.07)), 0.), linear_color_stop(bg.blend(gpui::black().opacity(0.12)), 1.)))
        .border_1()
        .border_color(gpui::white().opacity(0.12))
        .child(gpui::svg().path(icon).size(px(11.)).text_color(fg))
}

/// A tab's face at full width: favicon, label, trailing slot.
fn face(theme: &Theme, t: &Tab, width: f32, selected: bool, window: &mut Window) -> Div {
    // ponytail: a hovered tab's wash and a dragged tab's card differ a shade from this; the group hover can follow if it shows
    let ring = if selected { theme.surface.blend(theme.selected) } else { theme.surface };
    div()
        .w(px(width))
        .h(px(TAB_H))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(INNER_GAP))
        .pl(px(PAD_L))
        .pr(px(PAD_R))
        .rounded_full()
        .font_family(theme.font_ui.clone())
        .text_size(px(FONT_SIZE))
        .text_color(if selected { theme.text } else { theme.text_muted })
        .child(leading(t, ring, theme, window))
}

/// The trailing slot: the status dot, if any.
fn dot_slot(theme: &Theme, t: &Tab) -> Div {
    div().size(px(SLOT)).flex_none().flex().items_center().justify_center().children(t.status.map(|s| status_dot(s, theme)))
}

/// A tab being dragged along its strip: its own face under the pointer, lifted on a card.
#[derive(Clone)]
pub(super) struct DraggedTab {
    tab: Tab,
    theme: Theme,
    /// Where the pointer grabbed the tab, from its top-left.
    grab: gpui::Point<Pixels>,
    drag: Rc<RefCell<Option<(String, gpui::Point<Pixels>)>>>,
}

impl Render for DraggedTab {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // gpui draws the ghost at the pointer less the grab: remember it, the held tab's gap follows it
        *self.drag.borrow_mut() = Some((self.tab.id.clone(), window.mouse_position() - self.grab));
        let theme = &self.theme;
        face(theme, &self.tab, self.tab.width, true, window)
            .bg(theme.card)
            .border_1()
            .border_color(theme.border)
            .shadow_lg()
            .child(div().flex_1().min_w_0().truncate().child(name_text(&self.tab, theme)))
            .when(self.tab.zoomed, |el| el.child(zoom_mark(theme)))
            .child(dot_slot(theme, &self.tab))
    }
}

/// Tab `p`: its face in its slot, `editing` in place of its label, its `×` and a middle click closing it. Its owner adds
/// what else it does.
pub(super) fn tab_element(
    p: &Placed,
    theme: &Theme,
    editing: Option<AnyElement>,
    close_key: Option<&'static str>,
    on_close: impl Fn(&mut Shell, &mut Context<Shell>) + Clone + 'static,
    window: &mut Window,
    cx: &mut Context<Shell>,
) -> Stateful<Div> {
    let (t, k, selected, closing, holding) = (&p.tab, p.presence, p.selected, p.closing, p.holding);
    let draggable = editing.is_none();
    let full_w = if draggable { t.width } else { MAX_W };
    let group = SharedString::from(format!("tab-row-{}", t.id));
    let middle = on_close.clone();
    // The face keeps its full width and is clipped while the tab grows or shrinks.
    let content = face(theme, t, full_w, selected, window)
        .map(|el| match editing {
            Some(field) => el.child(field),
            None => el.child(div().flex_1().min_w_0().truncate().child(name_text(t, theme))),
        })
        .when(t.zoomed && draggable, |el| el.child(zoom_mark(theme)))
        .child(
            div()
                .id(SharedString::from(format!("tab-x-{}", t.id)))
                .tooltip(tip("Close tab", close_key))
                .size(px(SLOT))
                .flex_none()
                .relative()
                .rounded_full()
                .hover(|s| s.bg(theme.hover))
                .child(dot_slot(theme, t).absolute().inset_0().group_hover(group.clone(), |s| s.opacity(0.)))
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
                    // the tab owns a drag: claim the press so it closes instead of starting one
                    el.on_mouse_down(MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        on_close(this, cx);
                    }))
                }),
        );
    div()
        .id(SharedString::from(format!("tab-{}", t.id)))
        .group(group)
        .relative()
        .left(px(p.shift))
        .flex_none()
        .h(px(TAB_H))
        .w(px(full_w * k))
        .mr(px(GAP * k))
        .overflow_hidden()
        .opacity(k)
        .when(selected && !holding, |el| el.bg(theme.selected))
        .when(holding, |el| el.border_1().border_color(theme.border).bg(theme.wash(0.04)))
        .rounded_full()
        .cursor_pointer()
        // off the titlebar's drag strip: a press here drags the tab, not the window; the wheel still scrolls the row
        .block_mouse_except_scroll()
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .when(!selected && !closing, |el| el.hover(|s| s.bg(theme.hover)))
        // Middle-click closes, like every tab strip (Zeron §1.10).
        .when(!closing, |el| el.on_mouse_down(MouseButton::Middle, cx.listener(move |this, _: &MouseDownEvent, _, cx| middle(this, cx))))
        .child(content.when(holding, |el| el.opacity(0.)))
}

/// The `+` beside a strip: it stays in view while the tabs scroll.
pub(super) fn new_tab_button(theme: &Theme, id: &'static str) -> Stateful<Div> {
    icon_button(theme, id, "+")
        .size(px(TAB_H))
        .text_size(px(19.))
        .block_mouse_except_scroll()
        .when(cfg!(windows), |el| el.occlude()) // as the tabs, off the drag strip
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
}

impl Shell {
    /// The strip's scrolling row of `items`, faded at the edges; `dropped` lands a tab let go of over it.
    pub(super) fn tabs_row(&self, tabs: &Tabs, id: &'static str, theme: &Theme, items: Vec<AnyElement>, dropped: fn(&mut Shell, &mut Context<Shell>), cx: &mut Context<Self>) -> Div {
        let scroll = &tabs.scroll;
        div()
            .min_w_0()
            .h_full()
            .relative()
            .child(
                div()
                    .id(id)
                    // GPUI tells Windows the control area of every hitbox under the pointer, scroll-only blockers
                    // included: without blocking here a press on a tab is the drag strip's, and Windows moves the
                    // window. The row still scrolls, being the blocker.
                    .when(cfg!(windows), |el| el.occlude())
                    .size_full()
                    .flex()
                    .items_center()
                    .overflow_x_scroll()
                    .track_scroll(scroll)
                    // the held tab's gap follows the pointer; let go of, it lands in that slot
                    .on_drag_move::<DraggedTab>(cx.listener(|_, _, _, cx| cx.notify()))
                    .on_drop(cx.listener(move |this, _: &DraggedTab, _, cx| dropped(this, cx)))
                    // re-render so the edge fades follow the scroll position
                    .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.notify()))
                    .children(items),
            )
            // ponytail: gpui has no alpha mask, so the fades are gradients of the strip's own color over the tabs
            .map(|el| {
                let (offset, max) = (-f32::from(scroll.offset().x), f32::from(scroll.max_offset().x));
                let fade = |angle: f32| {
                    div().absolute().top_0().bottom_0().w(px(32.)).bg(linear_gradient(angle, linear_color_stop(theme.surface, 0.), linear_color_stop(theme.surface.opacity(0.), 1.)))
                };
                el.when(offset > 0.5, |el| el.child(fade(90.).left_0())).when(offset < max - 0.5, |el| el.child(fade(270.).right_0()))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(id: &str) -> Tab {
        Tab { id: id.into(), label: id.into(), name: (id.into(), Vec::new()), leading: Leading::Spinner, status: None, zoomed: false, width: 100. }
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
    fn a_closing_tab_shrinks_out_then_goes() {
        let t0 = Instant::now();
        let mut bar = Tabs::default();
        bar.sync(Some("w"), vec![tab("a"), tab("b")], t0);
        bar.sync(Some("w"), vec![tab("b")], t0);
        assert_eq!(bar.entries.len(), 2);
        assert_eq!(bar.layout(t0, None).0[1].0, 102.); // `a` still holds its slot as it starts to close
        bar.sync(Some("w"), vec![tab("b")], t0 + EXIT);
        assert_eq!(bar.entries.len(), 1);
        // another workspace swaps the tabs without motion
        bar.sync(Some("v"), vec![tab("c")], t0 + EXIT);
        assert_eq!(bar.entries[0].presence(t0 + EXIT), 1.);
    }

    #[test]
    fn reordered_tabs_are_in_their_slots_at_once() {
        let t0 = Instant::now();
        let mut bar = Tabs::default();
        bar.sync(Some("w"), vec![tab("a"), tab("b")], t0);
        bar.layout(t0, None);
        bar.sync(Some("w"), vec![tab("b"), tab("a")], t0);
        bar.layout(t0, None);
        assert_eq!((bar.entries[0].drawn, bar.entries[1].drawn), (0., 102.));
        assert!(!bar.entries.iter().any(|e| e.moving(t0)));
    }

    #[test]
    fn a_held_tab_takes_the_slot_its_middle_reaches_and_lands_there() {
        let t0 = Instant::now();
        let mut bar = Tabs::default();
        bar.sync(Some("w"), vec![tab("a"), tab("b"), tab("c")], t0);
        bar.layout(t0, None);
        // `a` (middle 50) dragged right: its right edge (150) short of b's middle (152), it keeps its slot; past it,
        // it takes b's
        let (_, slid) = bar.layout(t0, Some(("a", 100.)));
        assert_eq!(bar.held, Some(("a".into(), 0)));
        assert!(!slid);
        let (_, slid) = bar.layout(t0, Some(("a", 110.)));
        assert_eq!(bar.held, Some(("a".into(), 1)));
        assert!(slid);
        // held there, it stays: its left edge (60) is past b's middle (50) now b has closed up
        let (_, slid) = bar.layout(t0, Some(("a", 110.)));
        assert!(!slid);
        // b is in a's place
        assert_eq!((bar.entries[0].drawn, bar.entries[1].drawn), (102., 0.));
        // let go: the move asked for, and that order kept until herdr's is it
        assert_eq!(bar.drop_held(t0), Some((0, 1)));
        bar.layout(t0, None);
        assert_eq!((bar.entries[0].drawn, bar.entries[1].drawn), (102., 0.));
        bar.sync(Some("w"), vec![tab("b"), tab("a"), tab("c")], t0);
        bar.layout(t0, None);
        assert_eq!((bar.entries[0].drawn, bar.entries[1].drawn), (0., 102.));
        assert!(bar.pending.is_none());
    }
}
