//! A list of rows reordered by dragging, as the place bar's buttons are: the row lifts and follows the pointer, a
//! placeholder holds the slot it would land in, the others spring around it, and let go of, it flies into its slot.
//! Rows may differ in height (a worktree group, unfolded): each is measured as it is painted.
use super::motion::spring;
use super::*;
use std::cell::RefCell;
use std::collections::HashMap;

/// A row springing to a new slot, as a place's button does.
const SLIDE: Duration = Duration::from_millis(380);
/// How long a dropped order is shown, at most, before the list's own catches up (herdr's answer).
const PENDING: Duration = Duration::from_millis(1500);

/// What gpui drags: the list is told the row's key when the drag starts, and draws the row itself under the pointer.
#[derive(Clone)]
pub(super) struct DraggedRow {
    list: Rc<RefCell<Reorder>>,
    key: String,
}

impl Render for DraggedRow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// A row's top in the list going from `from` to `to` since `at`.
#[derive(Clone, Copy)]
struct Slide {
    from: f32,
    to: f32,
    at: Instant,
}

impl Slide {
    fn now(&self, now: Instant) -> f32 {
        self.from + (self.to - self.from) * spring(now.saturating_duration_since(self.at).as_secs_f32() / SLIDE.as_secs_f32())
    }

    fn done(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.at) >= SLIDE
    }
}

/// The row being dragged: where the pointer holds it from its top, the slot its placeholder holds, and its top.
struct Drag {
    key: String,
    grab: f32,
    slot: usize,
    top: f32,
}

/// A row as drawn this frame: moved `offset` from where the list lays it out, or, lifted, a placeholder there and
/// the row itself at `lifted` in the window.
pub(super) struct RowAt {
    pub(super) offset: f32,
    pub(super) lifted: Option<gpui::Point<Pixels>>,
}

#[derive(Default)]
pub(super) struct Reorder {
    /// Each row's height, and the list's top-left and width in the window, as last painted.
    heights: HashMap<String, f32>,
    origin: Option<gpui::Point<Pixels>>,
    pub(super) width: f32,
    /// Each row's top on its way to its slot, and its place in the order drawn.
    slides: HashMap<String, (Slide, usize)>,
    drag: Option<Drag>,
    /// The order a drop asked for, shown until the list's own is it.
    pending: Option<(Vec<String>, Instant)>,
    /// The row let go of, flying into its slot.
    landing: Option<String>,
}

impl Reorder {
    /// The rows of `keys` (`gap` apart) as drawn now, and whether any still moves; a drag that ended just now is
    /// also the move it asks for, `(from, to)` in `keys`.
    pub(super) fn layout(
        &mut self,
        keys: &[String],
        gap: f32,
        dragging: bool,
        pointer: gpui::Point<Pixels>,
        now: Instant,
    ) -> (Vec<RowAt>, bool, Option<(usize, usize)>) {
        let heights = &self.heights;
        let h = |k: &String| heights.get(k).copied().unwrap_or(0.);
        let origin = self.origin.unwrap_or_default();
        let mut dropped = None;
        let from = self.drag.as_ref().and_then(|d| keys.iter().position(|k| *k == d.key));
        match (self.drag.as_mut(), from) {
            (Some(d), Some(_)) if dragging => {
                d.top = f32::from(pointer.y - origin.y) - d.grab;
                // the others laid out without it: it goes after each whose middle is above its own
                let mid = d.top + h(&d.key) / 2.;
                let (mut y, mut slot) = (0., 0);
                for k in keys.iter().filter(|k| **k != d.key) {
                    slot += usize::from(y + h(k) / 2. < mid);
                    y += h(k) + gap;
                }
                if d.slot != slot {
                    haptic();
                }
                d.slot = slot;
            }
            (Some(_), Some(from)) => {
                let d = self.drag.take().expect("matched");
                if d.slot != from {
                    dropped = Some((from, d.slot));
                    let mut order: Vec<String> = keys.iter().filter(|k| **k != d.key).cloned().collect();
                    order.insert(d.slot, d.key.clone());
                    self.pending = Some((order, now));
                }
                // it flies from where it was let go
                if let Some((slide, _)) = self.slides.get_mut(&d.key) {
                    *slide = Slide { from: d.top, to: slide.to, at: now };
                }
                self.landing = Some(d.key);
            }
            _ => self.drag = None,
        }
        let mut same: Vec<&String> = keys.iter().collect();
        if let Some((order, at)) = &self.pending {
            same.sort();
            let mut asked: Vec<&String> = order.iter().collect();
            asked.sort();
            if order.as_slice() == keys || same != asked || now.saturating_duration_since(*at) > PENDING {
                self.pending = None;
            }
        }
        let order: Vec<String> = match (&self.drag, &self.pending) {
            (Some(d), _) => {
                let mut order: Vec<String> = keys.iter().filter(|k| **k != d.key).cloned().collect();
                order.insert(d.slot.min(order.len()), d.key.clone());
                order
            }
            (None, Some((order, _))) => order.clone(),
            (None, None) => keys.to_vec(),
        };
        let mut moving = false;
        let mut y = 0.;
        for (rank, k) in order.iter().enumerate() {
            let to = y;
            y += h(k) + gap;
            let (slide, was) = self.slides.entry(k.clone()).or_insert((Slide { from: to, to, at: now }, rank));
            if *was != rank {
                *slide = Slide { from: slide.now(now), to, at: now };
                *was = rank;
            } else if slide.to != to {
                // a row above grew or shrank: it moves along, without a spring
                let by = to - slide.to;
                slide.from += by;
                slide.to = to;
            }
            moving |= !slide.done(now);
        }
        self.slides.retain(|k, _| keys.contains(k));
        if self.landing.as_ref().is_some_and(|k| self.slides.get(k).is_none_or(|(s, _)| s.done(now))) {
            self.landing = None;
        }
        let lifted = self.drag.as_ref().map(|d| d.key.clone()).or(self.landing.clone());
        let mut y = 0.;
        let rows = keys
            .iter()
            .map(|k| {
                let top = self.slides.get(k).map_or(y, |(s, _)| s.now(now));
                let offset = top - y;
                y += h(k) + gap;
                let lifted = (lifted.as_ref() == Some(k)).then(|| {
                    let top = self.drag.as_ref().map_or(top, |d| d.top);
                    gpui::point(origin.x, origin.y + px(top))
                });
                RowAt { offset, lifted }
            })
            .collect();
        (rows, moving || dragging, dropped)
    }

    /// A row's height as last painted.
    pub(super) fn height(&self, key: &str) -> f32 {
        self.heights.get(key).copied().unwrap_or(0.)
    }
}

/// Makes `row` (key `key` in `list`) draggable, and measures it.
pub(super) fn draggable(row: Stateful<Div>, list: &Rc<RefCell<Reorder>>, key: &str) -> Stateful<Div> {
    let dragged = DraggedRow { list: list.clone(), key: key.to_string() };
    row.on_drag(dragged, |d, grab, _, cx| {
        let top = d.list.borrow().slides.get(&d.key).map_or(0., |(s, _)| s.to);
        let slot = d.list.borrow().slides.get(&d.key).map_or(0, |(_, rank)| *rank);
        d.list.borrow_mut().drag = Some(Drag { key: d.key.clone(), grab: f32::from(grab.y), slot, top });
        cx.new(|_| d.clone())
    })
}

/// Measures the row it is put in (which must be `relative`), as key `key` of `list`.
pub(super) fn measure(list: &Rc<RefCell<Reorder>>, key: &str) -> impl IntoElement + use<> {
    let (list, key) = (list.clone(), key.to_string());
    gpui::canvas(
        move |b, _, _| {
            list.borrow_mut().heights.insert(key, f32::from(b.size.height));
        },
        |_, _, _, _| {},
    )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
}

/// Measures where the list's rows start (put in their `relative` parent, which lays them out from its top).
pub(super) fn origin(list: &Rc<RefCell<Reorder>>) -> impl IntoElement + use<> {
    let list = list.clone();
    gpui::canvas(
        move |b, _, _| {
            let mut list = list.borrow_mut();
            list.origin = Some(b.origin);
            list.width = f32::from(b.size.width);
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// `rows` (each its key and element), `gap` apart in a column and moved as `list` has them; and the move a drag
/// that ended just now asks for, `(from, to)` in `rows`.
pub(super) fn column(
    list: &Rc<RefCell<Reorder>>,
    rows: Vec<(String, gpui::AnyElement)>,
    gap: f32,
    theme: &Theme,
    window: &mut Window,
    dragging: bool,
) -> (Div, Option<(usize, usize)>) {
    let keys: Vec<String> = rows.iter().map(|(k, _)| k.clone()).collect();
    let (at, moving, dropped) = list.borrow_mut().layout(&keys, gap, dragging, window.mouse_position(), Instant::now());
    if moving {
        window.request_animation_frame();
    }
    let width = list.borrow().width;
    let rows: Vec<_> = rows.into_iter().zip(at).map(|((key, row), at)| place(row, &at, list, &key, width, theme)).collect();
    let column = div()
        .relative()
        .flex()
        .flex_col()
        .gap(px(gap))
        // the lifted row follows the pointer
        .on_drag_move::<DraggedRow>(|_, window, _| window.refresh())
        .children(rows)
        .child(origin(list));
    (column, dropped)
}

/// `row`, drawn as `at` has it: in the list, moved; or lifted on a card at the pointer (`width` wide), a placeholder
/// of its height holding its slot.
fn place(row: gpui::AnyElement, at: &RowAt, list: &Rc<RefCell<Reorder>>, key: &str, width: f32, theme: &Theme) -> gpui::AnyElement {
    let slot = div().relative().top(px(at.offset));
    match at.lifted {
        None => slot.child(row).child(measure(list, key)).into_any_element(),
        Some(lifted) => slot
            .h(px(list.borrow().height(key)))
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border)
            .bg(theme.wash(0.04))
            .child(
                gpui::deferred(
                    gpui::anchored().position(lifted).child(
                        div()
                            .relative()
                            .w(px(width))
                            .rounded(px(8.))
                            .bg(theme.card)
                            .shadow_lg()
                            // drawn outside the window's root, it takes its text style itself
                            .text_color(theme.text)
                            .font_family(theme.font_ui.clone())
                            .text_size(px(13.))
                            .child(row)
                            .child(measure(list, key)),
                    ),
                )
                .with_priority(1),
            )
            .into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(heights: &[(&str, f32)]) -> (Reorder, Vec<String>) {
        let mut r = Reorder::default();
        for (k, h) in heights {
            r.heights.insert(k.to_string(), *h);
        }
        r.origin = Some(gpui::point(px(0.), px(100.)));
        (r, heights.iter().map(|(k, _)| k.to_string()).collect())
    }

    #[test]
    fn a_row_dragged_past_the_middle_of_the_next_takes_its_slot_and_is_moved_there_when_let_go() {
        // a tall group (b) between two rows, 2 px apart
        let (mut r, keys) = list(&[("a", 32.), ("b", 100.), ("c", 32.)]);
        let now = Instant::now();
        let at = |y: f32| gpui::point(px(10.), px(100. + y));
        r.layout(&keys, 2., false, at(0.), now);
        r.drag = Some(Drag { key: "a".into(), grab: 10., slot: 0, top: 0. });
        // held 20 px down, a's middle (36) is above b's (50): it stays
        let (rows, moving, _) = r.layout(&keys, 2., true, at(30.), now);
        assert_eq!(r.drag.as_ref().map(|d| d.slot), Some(0));
        assert!(rows[0].lifted.is_some() && moving);
        // 40 px down, past b's middle: after b, and b springs up into a's place
        r.layout(&keys, 2., true, at(50.), now);
        assert_eq!(r.drag.as_ref().map(|d| d.slot), Some(1));
        let (rows, ..) = r.layout(&keys, 2., true, at(50.), now + SLIDE);
        assert_eq!((rows[0].offset, rows[1].offset), (102., -34.));
        assert_eq!(rows[0].lifted, Some(gpui::point(px(0.), px(140.))));
        // let go: the move asked for, and the order kept until the list's is it
        let (_, _, dropped) = r.layout(&keys, 2., false, at(50.), now + SLIDE);
        assert_eq!(dropped, Some((0, 1)));
        let (rows, ..) = r.layout(&keys, 2., false, at(0.), now + SLIDE * 2);
        assert_eq!((rows[0].offset, rows[1].offset), (102., -34.));
        let moved = ["b", "a", "c"].map(String::from);
        let (rows, ..) = r.layout(&moved, 2., false, at(0.), now + SLIDE * 2);
        assert!(rows.iter().all(|r| r.offset == 0. && r.lifted.is_none()));
    }
}
