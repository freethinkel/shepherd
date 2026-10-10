//! Split dividers: the ratio a drag asks herdr for, and how often it asks.
use std::time::{Duration, Instant};

/// The split ratio for a pointer at `pointer_px` along a split starting at `start_px` and `len_px` long,
/// clamped so neither side collapses.
pub fn drag_ratio(start_px: f32, len_px: f32, pointer_px: f32) -> f32 {
    if len_px <= 0.0 {
        return 0.5;
    }
    ((pointer_px - start_px) / len_px).clamp(0.05, 0.95)
}

/// At most one `layout.set_split_ratio` per 33 ms while dragging (about one per frame herdr re-lays out).
#[derive(Debug, Default)]
pub struct Throttle {
    last: Option<Instant>,
}

impl Throttle {
    pub fn ready(&mut self, now: Instant) -> bool {
        if self.last.is_some_and(|last| now.duration_since(last) < Duration::from_millis(33)) {
            return false;
        }
        self.last = Some(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn drag_ratio_follows_the_pointer_within_the_split() {
        assert_eq!(drag_ratio(100.0, 400.0, 300.0), 0.5);
        assert_eq!(drag_ratio(100.0, 400.0, 0.0), 0.05); // clamped: no pane collapses
        assert_eq!(drag_ratio(100.0, 400.0, 999.0), 0.95);
        assert_eq!(drag_ratio(100.0, 0.0, 50.0), 0.5); // a degenerate split stays put
    }

    #[test]
    fn throttle_lets_one_call_through_per_window() {
        let t0 = Instant::now();
        let mut t = Throttle::default();
        assert!(t.ready(t0));
        assert!(!t.ready(t0 + Duration::from_millis(10)));
        assert!(t.ready(t0 + Duration::from_millis(40)));
    }
}
