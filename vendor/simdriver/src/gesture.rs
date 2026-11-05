//! Gesture router: turns raw pointer/scroll events into HID touch commands.
//!
//! Port of pip-mini `core/edges.ts` (native sink only). Pure: time is injected
//! and nothing here owns a timer; the driver calls [`Router::tick`] periodically.
//!
//! * A touch starting in an edge zone is held back until its intent is clear:
//!   inward travel replays it from 1 pt inside the edge with the edge flag on
//!   every sample (iOS decides home / switcher / control centre / back);
//!   sideways or outward travel releases it to the app unflagged; a tap is
//!   replayed plain on up.
//! * Option-drag is a two-finger pinch around the press point; Option+Shift-drag
//!   moves both fingers together.
//! * Trackpad scroll is a live one-finger pan, lifted on `Ended` or when idle.
//!
//! Coordinates are device points and every emitted point is clamped to the screen.

use std::time::Duration;

use crate::protocol::{Edge, HidCmd, Phase};

const EDGE_ZONE_BOTTOM_PT: f64 = 24.0;
const EDGE_ZONE_TOP_PT: f64 = 16.0;
const EDGE_ZONE_LEFT_PT: f64 = 10.0;
const EDGE_ZONE_RIGHT_PT: f64 = 10.0;
// Travel that decides a held edge touch. Small, because the screen doesn't follow the
// finger until then (16 pt felt like lag-then-jump on a home swipe); still enough to
// tell a tap or a sideways swipe from an inward one.
const COMMIT_PT: f64 = 6.0;
const PINCH_PT_PER_E: f64 = 150.0;
const PINCH_SCALE: (f64, f64) = (0.25, 4.0);
const PINCH_R0_PT: f64 = 50.0;
const PAN_IDLE_LIFT: Duration = Duration::from_millis(150);

/// Phase of a trackpad scroll gesture. Public API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollPhase {
    Began,
    Changed,
    Ended,
    Momentum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Stream,
    Pending,
    Edge,
    Pinch,
}

#[derive(Debug)]
struct Touch {
    edge: Edge,
    state: State,
    x0: f64,
    y0: f64,
    buffered: Vec<(f64, f64)>,
    pinch_pan: bool,
    pinch_last: Option<[f64; 4]>,
    /// Latest pointer position (clamped).
    last: (f64, f64),
}

#[derive(Debug)]
struct Pan {
    x: f64,
    y: f64,
    last: Duration,
}

#[derive(Debug)]
pub(crate) struct Router {
    w: f64,
    h: f64,
    touch: Option<Touch>,
    pan: Option<Pan>,
    /// Host-driven two-finger contact (`two_finger`): where its fingers last were.
    two: Option<[f64; 4]>,
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

/// Travel (into the screen, along the edge) from `edge`.
fn inward(edge: Edge, dx: f64, dy: f64) -> (f64, f64) {
    match edge {
        Edge::Bottom => (-dy, dx.abs()),
        Edge::Top => (dy, dx.abs()),
        Edge::Left => (dx, dy.abs()),
        Edge::Right => (-dx, dy.abs()),
        Edge::None => (0.0, 0.0),
    }
}

fn touch(phase: Phase, x: f64, y: f64, edge: Edge) -> HidCmd {
    HidCmd::Touch { phase, x, y, edge }
}

impl Router {
    pub(crate) fn new(screen_pt: (f64, f64)) -> Router {
        Router {
            w: screen_pt.0,
            h: screen_pt.1,
            touch: None,
            pan: None,
            two: None,
        }
    }

    fn cx(&self, x: f64) -> f64 {
        clamp(x, 0.0, self.w)
    }
    fn cy(&self, y: f64) -> f64 {
        clamp(y, 0.0, self.h)
    }

    /// The edge zone a touch starting at (x, y) belongs to. Bottom wins corners.
    fn edge_at(&self, x: f64, y: f64) -> Edge {
        if y >= self.h - EDGE_ZONE_BOTTOM_PT {
            Edge::Bottom
        } else if y <= EDGE_ZONE_TOP_PT {
            Edge::Top
        } else if x <= EDGE_ZONE_LEFT_PT {
            Edge::Left
        } else if x >= self.w - EDGE_ZONE_RIGHT_PT {
            Edge::Right
        } else {
            Edge::None
        }
    }

    /// One point inside the edge (Meta's edgeSwipe start).
    fn snap(&self, edge: Edge, x: f64, y: f64) -> (f64, f64) {
        match edge {
            Edge::Bottom => (x, self.h - 1.0),
            Edge::Top => (x, 1.0),
            Edge::Left => (1.0, y),
            Edge::Right => (self.w - 1.0, y),
            Edge::None => (x, y),
        }
    }

    fn pinch_fingers(&self, t: &Touch, x: f64, y: f64) -> [f64; 4] {
        let f = if t.pinch_pan {
            let (dx, dy) = (x - t.x0, y - t.y0);
            [
                t.x0 - PINCH_R0_PT + dx,
                t.y0 + dy,
                t.x0 + PINCH_R0_PT + dx,
                t.y0 + dy,
            ]
        } else {
            let scale = clamp(
                ((t.y0 - y) / PINCH_PT_PER_E).exp(),
                PINCH_SCALE.0,
                PINCH_SCALE.1,
            );
            let r = PINCH_R0_PT * scale;
            [t.x0 - r, t.y0, t.x0 + r, t.y0]
        };
        [self.cx(f[0]), self.cy(f[1]), self.cx(f[2]), self.cy(f[3])]
    }

    fn touch2(phase: Phase, f: [f64; 4]) -> HidCmd {
        HidCmd::Touch2 {
            phase,
            x1: f[0],
            y1: f[1],
            x2: f[2],
            y2: f[3],
        }
    }

    /// A held edge touch that was not an edge gesture: hand the app everything it
    /// missed, unflagged, then stream.
    fn release(t: &mut Touch, out: &mut Vec<HidCmd>) {
        t.state = State::Stream;
        out.push(touch(Phase::Down, t.x0, t.y0, Edge::None));
        for (x, y) in t.buffered.drain(..) {
            out.push(touch(Phase::Move, x, y, Edge::None));
        }
    }

    fn lift_two(&mut self, out: &mut Vec<HidCmd>) {
        if let Some(f) = self.two.take() {
            out.push(Self::touch2(Phase::Up, f));
        }
    }

    /// Two fingers exactly where the host puts them (rotation, off-centre pinch,
    /// scripted multi-touch). A down first lifts whatever else is touching; moves and
    /// ups without a down are ignored. Points are clamped to the screen.
    pub(crate) fn two_finger(&mut self, phase: Phase, f: [f64; 4]) -> Vec<HidCmd> {
        let f = [self.cx(f[0]), self.cy(f[1]), self.cx(f[2]), self.cy(f[3])];
        let mut out = Vec::new();
        match phase {
            Phase::Down => {
                self.lift_pan(&mut out);
                self.lift_two(&mut out);
                if let Some(t) = &self.touch {
                    if t.state == State::Pending {
                        self.touch = None; // never reached the device
                    } else {
                        let (lx, ly) = t.last;
                        out.extend(self.up(lx, ly, Duration::ZERO));
                    }
                }
                self.two = Some(f);
                out.push(Self::touch2(Phase::Down, f));
            }
            Phase::Move if self.two.is_some() => {
                self.two = Some(f);
                out.push(Self::touch2(Phase::Move, f));
            }
            Phase::Up if self.two.is_some() => {
                self.two = None;
                out.push(Self::touch2(Phase::Up, f));
            }
            _ => {}
        }
        out
    }

    fn lift_pan(&mut self, out: &mut Vec<HidCmd>) {
        if let Some(p) = self.pan.take() {
            out.push(touch(Phase::Up, p.x, p.y, Edge::None));
        }
    }

    /// `pinch`: Option held, the drag becomes two fingers; `pan` (Shift too) moves
    /// them together instead of spreading them.
    pub(crate) fn down(
        &mut self,
        x: f64,
        y: f64,
        pinch: bool,
        pan: bool,
        now: Duration,
    ) -> Vec<HidCmd> {
        let mut out = Vec::new();
        self.lift_pan(&mut out); // a click ends any trackpad pan in flight
        self.lift_two(&mut out);
        // A down without an up (the host lost one): lift the finger first. A held edge
        // touch never reached the device, so it is simply dropped.
        if let Some(t) = &self.touch {
            if t.state == State::Pending {
                self.touch = None;
            } else {
                let (lx, ly) = t.last;
                out.extend(self.up(lx, ly, now));
            }
        }
        let (x, y) = (self.cx(x), self.cy(y));
        let edge = if pinch {
            Edge::None
        } else {
            self.edge_at(x, y)
        };
        let state = if pinch {
            State::Pinch
        } else if edge != Edge::None {
            State::Pending
        } else {
            State::Stream
        };
        let mut t = Touch {
            edge,
            state,
            x0: x,
            y0: y,
            buffered: Vec::new(),
            pinch_pan: pan,
            pinch_last: None,
            last: (x, y),
        };
        match state {
            State::Stream => out.push(touch(Phase::Down, x, y, Edge::None)),
            State::Pinch => {
                let f = self.pinch_fingers(&t, x, y);
                t.pinch_last = Some(f);
                out.push(Self::touch2(Phase::Down, f));
            }
            _ => {}
        }
        self.touch = Some(t);
        out
    }

    pub(crate) fn move_to(&mut self, x: f64, y: f64, _now: Duration) -> Vec<HidCmd> {
        let (x, y) = (self.cx(x), self.cy(y));
        let mut out = Vec::new();
        let Some(mut t) = self.touch.take() else {
            return out;
        };
        t.last = (x, y);
        match t.state {
            State::Stream => out.push(touch(Phase::Move, x, y, Edge::None)),
            State::Pinch => {
                let f = self.pinch_fingers(&t, x, y);
                t.pinch_last = Some(f);
                out.push(Self::touch2(Phase::Move, f));
            }
            State::Edge => out.push(touch(Phase::Move, x, y, t.edge)),
            State::Pending => {
                t.buffered.push((x, y));
                let (into, along) = inward(t.edge, x - t.x0, y - t.y0);
                if into >= COMMIT_PT && into >= along {
                    // Inward: replay from the edge itself, flagged, then stream.
                    t.state = State::Edge;
                    let (sx, sy) = self.snap(t.edge, t.x0, t.y0);
                    out.push(touch(Phase::Down, sx, sy, t.edge));
                    for (bx, by) in t.buffered.drain(..) {
                        out.push(touch(Phase::Move, bx, by, t.edge));
                    }
                } else if along >= COMMIT_PT || into <= -COMMIT_PT {
                    Self::release(&mut t, &mut out);
                }
            }
        }
        self.touch = Some(t);
        out
    }

    pub(crate) fn up(&mut self, x: f64, y: f64, _now: Duration) -> Vec<HidCmd> {
        let (x, y) = (self.cx(x), self.cy(y));
        let mut out = Vec::new();
        let Some(mut t) = self.touch.take() else {
            return out;
        };
        match t.state {
            State::Pinch => {
                let f = t.pinch_last.unwrap_or_else(|| self.pinch_fingers(&t, x, y));
                out.push(Self::touch2(Phase::Up, f));
            }
            State::Edge => out.push(touch(Phase::Up, x, y, t.edge)),
            State::Pending => {
                // A tap in an edge zone: replay it plain.
                Self::release(&mut t, &mut out);
                out.push(touch(Phase::Up, x, y, Edge::None));
            }
            State::Stream => out.push(touch(Phase::Up, x, y, Edge::None)),
        }
        out
    }

    /// Trackpad scroll as a live one-finger pan. (x, y) is the pointer, (dx, dy) the
    /// scroll delta, both in device points. Momentum is ignored: iOS flings on its
    /// own from the release velocity.
    pub(crate) fn scroll(
        &mut self,
        x: f64,
        y: f64,
        dx: f64,
        dy: f64,
        phase: ScrollPhase,
        now: Duration,
    ) -> Vec<HidCmd> {
        let mut out = Vec::new();
        if self.touch.is_some() || self.two.is_some() {
            return out; // a mouse drag or a two-finger contact owns the fingers
        }
        match phase {
            ScrollPhase::Began => {
                self.lift_pan(&mut out);
                let (x, y) = (self.cx(x), self.cy(y));
                self.pan = Some(Pan { x, y, last: now });
                out.push(touch(Phase::Down, x, y, Edge::None));
            }
            ScrollPhase::Changed => {
                if let Some(p) = &self.pan {
                    // Natural scrolling: content follows the fingers, and so does the finger.
                    let nx = clamp(p.x + dx, 0.0, self.w);
                    let ny = clamp(p.y + dy, 0.0, self.h);
                    self.pan = Some(Pan {
                        x: nx,
                        y: ny,
                        last: now,
                    });
                    out.push(touch(Phase::Move, nx, ny, Edge::None));
                }
            }
            ScrollPhase::Ended => self.lift_pan(&mut out),
            ScrollPhase::Momentum => {}
        }
        out
    }

    /// Lift a pan finger if `Ended` never arrived. The driver calls this every ~50 ms.
    pub(crate) fn tick(&mut self, now: Duration) -> Vec<HidCmd> {
        let mut out = Vec::new();
        if self
            .pan
            .as_ref()
            .is_some_and(|p| now.saturating_sub(p.last) >= PAN_IDLE_LIFT)
        {
            self.lift_pan(&mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: Duration = Duration::ZERO;

    fn r() -> Router {
        Router::new((402.0, 874.0))
    }
    fn t(phase: Phase, x: f64, y: f64, edge: Edge) -> HidCmd {
        HidCmd::Touch { phase, x, y, edge }
    }
    fn t2(phase: Phase, x1: f64, y1: f64, x2: f64, y2: f64) -> HidCmd {
        HidCmd::Touch2 {
            phase,
            x1,
            y1,
            x2,
            y2,
        }
    }
    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn center_tap_streams_immediately() {
        let mut r = r();
        assert_eq!(
            r.down(200.0, 400.0, false, false, T0),
            vec![t(Phase::Down, 200.0, 400.0, Edge::None)]
        );
        assert_eq!(
            r.up(200.0, 400.0, T0),
            vec![t(Phase::Up, 200.0, 400.0, Edge::None)]
        );
    }

    #[test]
    fn bottom_edge_inward_replays_from_edge() {
        let mut r = r();
        assert!(r.down(200.0, 860.0, false, false, T0).is_empty());
        assert!(
            r.move_to(200.0, 856.0, T0).is_empty(),
            "4 pt up is still held"
        );
        assert_eq!(
            r.move_to(200.0, 850.0, T0),
            vec![
                t(Phase::Down, 200.0, 873.0, Edge::Bottom),
                t(Phase::Move, 200.0, 856.0, Edge::Bottom),
                t(Phase::Move, 200.0, 850.0, Edge::Bottom),
            ]
        );
        assert_eq!(
            r.move_to(200.0, 700.0, T0),
            vec![t(Phase::Move, 200.0, 700.0, Edge::Bottom)]
        );
        assert_eq!(
            r.up(200.0, 700.0, T0),
            vec![t(Phase::Up, 200.0, 700.0, Edge::Bottom)]
        );
    }

    #[test]
    fn status_bar_tap_is_replayed_plain() {
        let mut r = r();
        assert!(r.down(200.0, 8.0, false, false, T0).is_empty());
        assert_eq!(
            r.up(200.0, 8.0, T0),
            vec![
                t(Phase::Down, 200.0, 8.0, Edge::None),
                t(Phase::Up, 200.0, 8.0, Edge::None)
            ]
        );
    }

    #[test]
    fn edge_sideways_releases_to_app() {
        let mut r = r();
        assert!(r.down(200.0, 860.0, false, false, T0).is_empty());
        assert_eq!(
            r.move_to(220.0, 858.0, T0),
            vec![
                t(Phase::Down, 200.0, 860.0, Edge::None),
                t(Phase::Move, 220.0, 858.0, Edge::None)
            ]
        );
        assert_eq!(
            r.move_to(240.0, 858.0, T0),
            vec![t(Phase::Move, 240.0, 858.0, Edge::None)]
        );
        assert_eq!(
            r.up(240.0, 858.0, T0),
            vec![t(Phase::Up, 240.0, 858.0, Edge::None)]
        );
    }

    #[test]
    fn other_edges_commit_inward() {
        let mut r = r();
        assert!(r.down(5.0, 400.0, false, false, T0).is_empty());
        assert_eq!(
            r.move_to(25.0, 400.0, T0),
            vec![
                t(Phase::Down, 1.0, 400.0, Edge::Left),
                t(Phase::Move, 25.0, 400.0, Edge::Left)
            ]
        );
        r.up(25.0, 400.0, T0);
        assert!(r.down(200.0, 5.0, false, false, T0).is_empty());
        assert_eq!(
            r.move_to(200.0, 30.0, T0),
            vec![
                t(Phase::Down, 200.0, 1.0, Edge::Top),
                t(Phase::Move, 200.0, 30.0, Edge::Top)
            ]
        );
    }

    #[test]
    fn option_drag_pinches_live() {
        let mut r = r();
        assert_eq!(
            r.down(200.0, 400.0, true, false, T0),
            vec![t2(Phase::Down, 150.0, 400.0, 250.0, 400.0)]
        );
        let rad = 50.0 * 1f64.exp();
        assert_eq!(
            r.move_to(200.0, 250.0, T0),
            vec![t2(Phase::Move, 200.0 - rad, 400.0, 200.0 + rad, 400.0)]
        );
        assert_eq!(
            r.move_to(200.0, -1000.0, T0),
            vec![t2(Phase::Move, 0.0, 400.0, 400.0, 400.0)] // scale clamps to 4, r = 200
        );
        assert_eq!(
            r.up(200.0, -1000.0, T0),
            vec![t2(Phase::Up, 0.0, 400.0, 400.0, 400.0)]
        );
    }

    #[test]
    fn option_shift_drag_pans_two_fingers() {
        let mut r = r();
        assert_eq!(
            r.down(200.0, 400.0, true, true, T0),
            vec![t2(Phase::Down, 150.0, 400.0, 250.0, 400.0)]
        );
        assert_eq!(
            r.move_to(220.0, 380.0, T0),
            vec![t2(Phase::Move, 170.0, 380.0, 270.0, 380.0)]
        );
        assert_eq!(
            r.up(220.0, 380.0, T0),
            vec![t2(Phase::Up, 170.0, 380.0, 270.0, 380.0)]
        );
    }

    #[test]
    fn trackpad_pan_lifts_on_end_and_idle() {
        let mut r = r();
        assert_eq!(
            r.scroll(200.0, 400.0, 0.0, 0.0, ScrollPhase::Began, ms(0)),
            vec![t(Phase::Down, 200.0, 400.0, Edge::None)]
        );
        assert_eq!(
            r.scroll(200.0, 400.0, 10.0, -30.0, ScrollPhase::Changed, ms(16)),
            vec![t(Phase::Move, 210.0, 370.0, Edge::None)]
        );
        assert_eq!(
            r.scroll(200.0, 400.0, 500.0, 0.0, ScrollPhase::Changed, ms(32)),
            vec![t(Phase::Move, 402.0, 370.0, Edge::None)]
        );
        assert_eq!(
            r.scroll(0.0, 0.0, 0.0, 0.0, ScrollPhase::Ended, ms(48)),
            vec![t(Phase::Up, 402.0, 370.0, Edge::None)]
        );
        assert!(
            r.scroll(0.0, 0.0, 5.0, 5.0, ScrollPhase::Momentum, ms(64))
                .is_empty()
        );
        assert!(r.tick(ms(1000)).is_empty());

        // No Ended: idle lift via tick, measured from the last event.
        r.scroll(100.0, 100.0, 0.0, 0.0, ScrollPhase::Began, ms(2000));
        r.scroll(0.0, 0.0, 0.0, 10.0, ScrollPhase::Changed, ms(2040));
        assert!(r.tick(ms(2189)).is_empty());
        assert_eq!(
            r.tick(ms(2190)),
            vec![t(Phase::Up, 100.0, 110.0, Edge::None)]
        );
        assert!(r.tick(ms(3000)).is_empty());
        // Momentum while a pan is live emits nothing either.
        r.scroll(100.0, 100.0, 0.0, 0.0, ScrollPhase::Began, ms(4000));
        assert!(
            r.scroll(0.0, 0.0, 9.0, 9.0, ScrollPhase::Momentum, ms(4010))
                .is_empty()
        );
    }

    #[test]
    fn drag_outside_box_clamps() {
        let mut r = r();
        r.down(200.0, 400.0, false, false, T0);
        assert_eq!(
            r.move_to(-50.0, 1000.0, T0),
            vec![t(Phase::Move, 0.0, 874.0, Edge::None)]
        );
        assert_eq!(
            r.up(-50.0, 1000.0, T0),
            vec![t(Phase::Up, 0.0, 874.0, Edge::None)]
        );
    }

    #[test]
    fn scroll_ignored_while_mouse_down() {
        let mut r = r();
        r.down(200.0, 400.0, false, false, T0);
        assert!(
            r.scroll(200.0, 400.0, 0.0, 0.0, ScrollPhase::Began, T0)
                .is_empty()
        );
        assert!(
            r.scroll(200.0, 400.0, 5.0, 5.0, ScrollPhase::Changed, T0)
                .is_empty()
        );
        assert!(r.tick(ms(1000)).is_empty());
    }

    #[test]
    fn down_without_up_lifts_the_touch_in_flight() {
        let mut r = r();
        r.down(200.0, 400.0, false, false, T0);
        r.move_to(210.0, 410.0, T0);
        assert_eq!(
            r.down(100.0, 100.0, false, false, T0),
            vec![
                t(Phase::Up, 210.0, 410.0, Edge::None),
                t(Phase::Down, 100.0, 100.0, Edge::None)
            ]
        );
        // A pinch in flight lifts both fingers where they were.
        r.up(100.0, 100.0, T0);
        r.down(200.0, 400.0, true, false, T0);
        assert_eq!(
            r.down(100.0, 100.0, false, false, T0),
            vec![
                t2(Phase::Up, 150.0, 400.0, 250.0, 400.0),
                t(Phase::Down, 100.0, 100.0, Edge::None)
            ]
        );
        // A held edge touch never reached the device: dropped, nothing to lift.
        r.up(100.0, 100.0, T0);
        assert!(r.down(200.0, 860.0, false, false, T0).is_empty());
        assert_eq!(
            r.down(100.0, 100.0, false, false, T0),
            vec![t(Phase::Down, 100.0, 100.0, Edge::None)]
        );
    }

    #[test]
    fn touch2_places_both_fingers_and_owns_the_screen() {
        let mut r = r();
        // Moves and ups without a down do nothing.
        assert!(r.two_finger(Phase::Move, [1.0, 2.0, 3.0, 4.0]).is_empty());
        assert!(r.two_finger(Phase::Up, [1.0, 2.0, 3.0, 4.0]).is_empty());
        // A down lifts a one-finger touch in flight; points clamp to the screen.
        r.down(200.0, 400.0, false, false, T0);
        assert_eq!(
            r.two_finger(Phase::Down, [100.0, 400.0, 500.0, 400.0]),
            vec![
                t(Phase::Up, 200.0, 400.0, Edge::None),
                t2(Phase::Down, 100.0, 400.0, 402.0, 400.0)
            ]
        );
        // Rotation: each finger goes exactly where it is put.
        assert_eq!(
            r.two_finger(Phase::Move, [150.0, 350.0, 250.0, 450.0]),
            vec![t2(Phase::Move, 150.0, 350.0, 250.0, 450.0)]
        );
        // Scroll can't start a pan while two fingers are down.
        assert!(
            r.scroll(100.0, 100.0, 0.0, 0.0, ScrollPhase::Began, T0)
                .is_empty()
        );
        assert_eq!(
            r.two_finger(Phase::Up, [160.0, 340.0, 240.0, 460.0]),
            vec![t2(Phase::Up, 160.0, 340.0, 240.0, 460.0)]
        );
        // A mouse down lifts a two-finger contact left down.
        r.two_finger(Phase::Down, [100.0, 100.0, 200.0, 200.0]);
        assert_eq!(
            r.down(50.0, 400.0, false, false, T0),
            vec![
                t2(Phase::Up, 100.0, 100.0, 200.0, 200.0),
                t(Phase::Down, 50.0, 400.0, Edge::None)
            ]
        );
    }

    #[test]
    fn mouse_down_lifts_pan_in_flight() {
        let mut r = r();
        r.scroll(100.0, 100.0, 0.0, 0.0, ScrollPhase::Began, T0);
        assert_eq!(
            r.down(200.0, 400.0, false, false, T0),
            vec![
                t(Phase::Up, 100.0, 100.0, Edge::None),
                t(Phase::Down, 200.0, 400.0, Edge::None)
            ]
        );
    }
}
