//! Frame/input statistics, with the definitions of pip-mini's `core/recorder.ts`.
//!
//! Pure and clock-free: callers pass `now` (time since an arbitrary epoch).
//!
//! Definitions:
//! - **active**: a finger is down, or it lifted at most 500 ms ago (inclusive).
//! - **fps**: frames with timestamp in the window `(now - 1 s, now]`, per second.
//! - **active_fps / idle_fps**: the frames of that same window, split by whether
//!   the instant of the frame was active, each divided by the seconds of the
//!   window that were active (resp. idle). A class with no time in the window
//!   reports `0.0`. (The window is always 1 s long, even in the first second.)
//! - **last_response_ms**: latest `Down` to the first frame after it; `None`
//!   while no frame has arrived since the latest `Down` (or before any `Down`).
//! - **max_active_gap_ms**: over the whole session, the largest frame-to-frame
//!   gap during which at least one `Move` arrived (gaps without a move say
//!   nothing about speed).
//! - **frames**: total frames seen.

use std::collections::VecDeque;
use std::time::Duration;

use crate::protocol::Phase;

const ACTIVE_TAIL: Duration = Duration::from_millis(500);
const WINDOW: Duration = Duration::from_secs(1);

/// Snapshot of driver statistics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    /// Frames per second over the last 1 s.
    pub fps: f32,
    /// Frames per active second, over the last 1 s.
    pub active_fps: f32,
    /// Frames per idle second, over the last 1 s.
    pub idle_fps: f32,
    /// Latest touch-down to first frame after it; `None` if none arrived yet.
    pub last_response_ms: Option<f32>,
    /// Largest frame gap (session-wide) that contained a finger move.
    pub max_active_gap_ms: f32,
    /// Total frames.
    pub frames: u64,
}

/// An active span: finger down at `start`; `end` is `None` while still down,
/// else lift time + tail (inclusive).
#[derive(Debug, Clone, Copy)]
struct Span {
    start: Duration,
    end: Option<Duration>,
}

impl Span {
    fn contains(&self, t: Duration) -> bool {
        t >= self.start && self.end.is_none_or(|e| t <= e)
    }
}

#[derive(Debug, Default)]
pub(crate) struct StatsTracker {
    frames: u64,
    recent: VecDeque<Duration>,
    spans: Vec<Span>,
    last_frame: Option<Duration>,
    moved_since_frame: bool,
    max_gap: Duration,
    down_at: Option<Duration>,
    response: Option<Duration>,
}

impl StatsTracker {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn on_frame(&mut self, now: Duration) {
        self.frames += 1;
        if let Some(prev) = self.last_frame
            && self.moved_since_frame
        {
            self.max_gap = self.max_gap.max(now.saturating_sub(prev));
        }
        self.last_frame = Some(now);
        self.moved_since_frame = false;
        if let (Some(down), None) = (self.down_at, self.response) {
            self.response = Some(now.saturating_sub(down));
        }
        self.recent.push_back(now);
        self.prune(now);
    }

    pub(crate) fn on_input(&mut self, phase: Phase, now: Duration) {
        match phase {
            Phase::Down => {
                self.down_at = Some(now);
                self.response = None;
                // Re-grab inside the tail: extend the previous span.
                match self.spans.last_mut() {
                    Some(s) if s.end.is_some_and(|e| now <= e) => s.end = None,
                    Some(s) if s.end.is_none() => {}
                    _ => self.spans.push(Span {
                        start: now,
                        end: None,
                    }),
                }
            }
            Phase::Move => self.moved_since_frame = true,
            Phase::Up => {
                if let Some(s) = self.spans.last_mut()
                    && s.end.is_none()
                {
                    s.end = Some(now + ACTIVE_TAIL);
                }
            }
        }
        self.prune(now);
    }

    pub(crate) fn snapshot(&self, now: Duration) -> Stats {
        let start = now.saturating_sub(WINDOW);
        let (mut total, mut active) = (0u32, 0u32);
        for &t in self.recent.iter().filter(|&&t| t > start && t <= now) {
            total += 1;
            if self.spans.iter().any(|s| s.contains(t)) {
                active += 1;
            }
        }
        let active_secs: f32 = self
            .spans
            .iter()
            .map(|s| {
                let lo = s.start.max(start);
                let hi = s.end.unwrap_or(now).min(now);
                hi.saturating_sub(lo).as_secs_f32()
            })
            .sum::<f32>()
            .min(1.0);
        let idle_secs = 1.0 - active_secs;
        let rate = |n: u32, secs: f32| if secs > 0.0 { n as f32 / secs } else { 0.0 };
        Stats {
            fps: total as f32,
            active_fps: rate(active, active_secs),
            idle_fps: rate(total - active, idle_secs),
            last_response_ms: self.response.map(|d| d.as_secs_f32() * 1000.0),
            max_active_gap_ms: self.max_gap.as_secs_f32() * 1000.0,
            frames: self.frames,
        }
    }

    /// Drop data that can no longer fall inside a window ending at or after `now`.
    fn prune(&mut self, now: Duration) {
        let start = now.saturating_sub(WINDOW);
        while self.recent.front().is_some_and(|&t| t <= start) {
            self.recent.pop_front();
        }
        self.spans.retain(|s| s.end.is_none_or(|e| e >= start));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn fps_counts_frames_in_last_second_only() {
        let mut s = StatsTracker::new();
        for t in [100, 600, 1100, 1500] {
            s.on_frame(ms(t));
        }
        let st = s.snapshot(ms(1500));
        assert_eq!(st.fps, 3.0); // 100 ms is out of (500, 1500]; 600 is in
        assert_eq!(st.frames, 4);
        // boundary: a frame exactly 1 s old is excluded
        assert_eq!(s.snapshot(ms(1600)).fps, 2.0); // 600 is excluded, 1100 + 1500 in
    }

    #[test]
    fn active_idle_split_with_500ms_tail() {
        let mut s = StatsTracker::new();
        s.on_input(Phase::Down, ms(1000));
        s.on_frame(ms(1200)); // finger down: active
        s.on_input(Phase::Up, ms(1400));
        s.on_frame(ms(1900)); // exactly 500 ms after lift: still active
        s.on_frame(ms(1901)); // idle
        let st = s.snapshot(ms(2000));
        // window (1000, 2000]: active 1000..1900 = 0.9 s, idle 0.1 s
        assert_eq!(st.fps, 3.0);
        assert!((st.active_fps - 2.0 / 0.9).abs() < 1e-3);
        assert!((st.idle_fps - 1.0 / 0.1).abs() < 1e-2);
    }

    #[test]
    fn idle_only_window_has_zero_active_fps() {
        let mut s = StatsTracker::new();
        s.on_frame(ms(100));
        s.on_frame(ms(200));
        let st = s.snapshot(ms(900));
        assert_eq!(st.active_fps, 0.0);
        assert_eq!(st.idle_fps, 2.0);
    }

    #[test]
    fn last_response_is_latest_down_to_first_frame_after() {
        let mut s = StatsTracker::new();
        assert_eq!(s.snapshot(ms(0)).last_response_ms, None);
        s.on_input(Phase::Down, ms(100));
        assert_eq!(s.snapshot(ms(150)).last_response_ms, None); // no frame yet
        s.on_frame(ms(116));
        s.on_frame(ms(200)); // later frames don't change it
        assert_eq!(s.snapshot(ms(300)).last_response_ms, Some(16.0));
        s.on_input(Phase::Up, ms(300));
        s.on_input(Phase::Down, ms(1000)); // new down, no frame after it
        assert_eq!(s.snapshot(ms(1100)).last_response_ms, None);
        s.on_frame(ms(1040));
        assert_eq!(s.snapshot(ms(1100)).last_response_ms, Some(40.0));
    }

    #[test]
    fn max_active_gap_counts_only_gaps_containing_a_move() {
        let mut s = StatsTracker::new();
        s.on_frame(ms(0));
        s.on_frame(ms(500)); // 500 ms gap, no move: ignored
        s.on_input(Phase::Down, ms(510));
        s.on_input(Phase::Move, ms(520));
        s.on_frame(ms(570)); // 70 ms gap with a move
        s.on_frame(ms(870)); // 300 ms gap, finger held still: ignored
        s.on_input(Phase::Move, ms(880));
        s.on_input(Phase::Move, ms(890));
        s.on_frame(ms(930)); // 60 ms gap with moves
        let st = s.snapshot(ms(930));
        assert_eq!(st.max_active_gap_ms, 70.0);
        assert_eq!(st.frames, 5);
    }

    #[test]
    fn frames_is_total_beyond_window() {
        let mut s = StatsTracker::new();
        for i in 0..50 {
            s.on_frame(ms(i * 100));
        }
        let st = s.snapshot(ms(4900));
        assert_eq!(st.frames, 50);
        assert_eq!(st.fps, 10.0);
    }
}
