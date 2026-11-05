//! A light, sampled tap on the frame pipeline, for recordings.
//!
//! Threading: [`TapFeeder::offer`] runs on the pipeline worker for every frame it reads,
//! before the host's `on_frame`. It never blocks: it updates the counters under a short
//! lock, decides whether the frame is due (`fps` a second, see below), and only then copies
//! it onto an unbounded channel whose depth is capped at [`QUEUE`] by an in-flight counter
//! checked *before* the copy, so a consumer that falls behind costs no copy at all, just a
//! `skipped_by_tap`. The [`FrameTap`] end lives on the consumer's thread and waits with
//! [`FrameTap::recv_timeout`]. `offer` returns false once the tap is dropped, which is how
//! the worker knows to prune its feeder; the tap sees [`TapRecv::Closed`] once its feeder
//! is gone (the pipeline dropped).
//!
//! Sampling follows a due-time schedule, so the long-run rate equals `fps` whatever the
//! input rate. With `interval = 1 s / fps` and `tol = 0.2 × interval`, the first frame is
//! always sampled; after that a frame is sampled when `at >= next_due - tol`, and each
//! sample sets `next_due = max(next_due + interval, at + interval - tol)`. The tolerance
//! lets a 60 Hz frame that lands just before the due time count (fps 30 takes every other
//! frame, fps 25 alternates every 2nd and 3rd); the `max` stops a burst of catch-up
//! samples after a pause.
use crate::helper::lock;
use crate::shm::Frame;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Arrival gaps this long or longer are the screen pausing (nothing changed, so sdframe
/// sent nothing), not stutters; the gap stats ignore them.
const PAUSE: Duration = Duration::from_millis(250);
/// Arrival gaps above this (two 60 Hz frames) count as stutters.
const STUTTER: Duration = Duration::from_millis(34);
/// Sampled frames queued for the consumer at most; more are skipped, not copied.
const QUEUE: usize = 3;

/// A sampled frame and when the pipeline read it.
#[derive(Debug, Clone)]
pub struct TapFrame {
    pub at: Instant,
    pub frame: Frame,
}

/// What a tap saw so far.
#[derive(Debug, Clone, Default)]
pub struct TapHealth {
    /// Frames offered to the tap.
    pub received: u64,
    /// Frames queued to the consumer.
    pub sampled: u64,
    /// Seq gaps: frames the pipeline coalesced away before the tap saw them.
    pub skipped_by_pipeline: u64,
    /// Sampled but dropped because the consumer was behind.
    pub skipped_by_tap: u64,
    /// Largest arrival gap shorter than a pause (250 ms).
    pub max_gap_ms: f32,
    /// Arrival gaps over 34 ms and shorter than a pause.
    pub gaps_over_34ms: u32,
    pub last_arrival: Option<Instant>,
}

/// Why [`FrameTap::recv_timeout`] returned no frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TapRecv {
    Timeout,
    /// Every feeder is gone (the pipeline dropped); no frame will come.
    Closed,
}

struct State {
    health: TapHealth,
    /// When the next sample is due; `None` until the first frame (always sampled).
    next_due: Option<Instant>,
    prev_seq: Option<u64>,
}

struct Shared {
    state: Mutex<State>,
    /// Frames sent and not yet received.
    in_flight: AtomicUsize,
}

/// The consumer end: sampled frames, oldest first. Dropping it detaches the tap.
pub struct FrameTap {
    rx: Receiver<TapFrame>,
    shared: Arc<Shared>,
}

/// The producer end, owned by the pipeline worker (or a test). A test seam: public so
/// the agent's tests and fakes can feed a tap, not part of the supported API.
#[doc(hidden)]
pub struct TapFeeder {
    tx: Sender<TapFrame>,
    shared: Arc<Shared>,
    interval: Duration,
}

impl FrameTap {
    /// A tap and its feeder, not attached to any pipeline: whoever holds the feeder
    /// offers the frames. `fps` is clamped to 1..=60. A test seam (see [`TapFeeder`]).
    #[doc(hidden)]
    pub fn manual(fps: u32) -> (FrameTap, TapFeeder) {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                health: TapHealth::default(),
                next_due: None,
                prev_seq: None,
            }),
            in_flight: AtomicUsize::new(0),
        });
        let interval = Duration::from_secs(1) / fps.clamp(1, 60);
        let feeder = TapFeeder {
            tx,
            shared: shared.clone(),
            interval,
        };
        (FrameTap { rx, shared }, feeder)
    }

    /// The next sampled frame, waiting up to `wait`. Frames already queued are delivered
    /// even after the feeder is gone; then [`TapRecv::Closed`].
    pub fn recv_timeout(&self, wait: Duration) -> Result<TapFrame, TapRecv> {
        match self.rx.recv_timeout(wait) {
            Ok(f) => {
                self.shared.in_flight.fetch_sub(1, Ordering::SeqCst);
                Ok(f)
            }
            Err(RecvTimeoutError::Timeout) => Err(TapRecv::Timeout),
            Err(RecvTimeoutError::Disconnected) => Err(TapRecv::Closed),
        }
    }

    pub fn health(&self) -> TapHealth {
        lock(&self.shared.state).health.clone()
    }
}

impl TapFeeder {
    /// Offer a frame read at `at`. Never blocks; copies it only if it is due and the
    /// queue has room. False once the [`FrameTap`] is dropped (stop offering).
    pub fn offer(&self, at: Instant, frame: &Frame) -> bool {
        // The FrameTap holds the only other reference.
        if Arc::strong_count(&self.shared) < 2 {
            return false;
        }
        {
            let mut s = lock(&self.shared.state);
            let State {
                health: h,
                next_due,
                prev_seq,
            } = &mut *s;
            h.received += 1;
            if let Some(prev) = h.last_arrival {
                let gap = at.saturating_duration_since(prev);
                if gap < PAUSE {
                    h.max_gap_ms = h.max_gap_ms.max(gap.as_secs_f32() * 1000.0);
                    if gap > STUTTER {
                        h.gaps_over_34ms += 1;
                    }
                }
            }
            h.last_arrival = Some(at);
            // sdframe seqs are even and advance by 2; going backwards is a new generation.
            if let Some(prev) = *prev_seq
                && frame.seq > prev
            {
                h.skipped_by_pipeline += ((frame.seq - prev) / 2).saturating_sub(1);
            }
            *prev_seq = Some(frame.seq);
            let tol = self.interval / 5;
            if next_due.is_some_and(|due| at + tol < due) {
                return true;
            }
            // A due frame takes its sampling slot even if the queue is full and it is
            // skipped, so the cadence holds when the consumer catches up.
            let after_this = at + self.interval - tol;
            *next_due = Some(match *next_due {
                Some(due) => (due + self.interval).max(after_this),
                None => after_this,
            });
            // Only this feeder adds to in_flight (under the lock); the consumer only
            // subtracts, so room seen here is still there after the unlock.
            if self.shared.in_flight.load(Ordering::SeqCst) >= QUEUE {
                h.skipped_by_tap += 1;
                return true;
            }
            self.shared.in_flight.fetch_add(1, Ordering::SeqCst);
            h.sampled += 1;
        }
        // The copy happens outside the lock, so health() never waits on it.
        let sent = TapFrame {
            at,
            frame: frame.clone(),
        };
        self.tx.send(sent).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    const W: Duration = Duration::from_secs(5);

    fn frame(seq: u64) -> Frame {
        Frame {
            width: 1,
            height: 1,
            source_width: 1,
            source_height: 1,
            seq,
            data: vec![seq as u8; 4],
        }
    }

    fn ms(n: f64) -> Duration {
        Duration::from_secs_f64(n / 1000.0)
    }

    /// Feeds `n` frames (seqs 2, 4, ...) `every_ms` apart, draining the tap after each
    /// offer so the queue never fills. Returns the tap's health.
    fn feed_steady(fps: u32, n: u64, every_ms: f64) -> TapHealth {
        let (tap, feeder) = FrameTap::manual(fps);
        let t0 = Instant::now();
        for i in 0..n {
            assert!(feeder.offer(t0 + ms(every_ms * i as f64), &frame(2 + 2 * i)));
            while tap.recv_timeout(Duration::ZERO).is_ok() {}
        }
        tap.health()
    }

    #[test]
    fn samples_60fps_input_at_about_30() {
        let h = feed_steady(30, 120, 16.7);
        assert_eq!(h.received, 120);
        assert!((58..=62).contains(&h.sampled), "sampled {}", h.sampled);
        assert_eq!((h.skipped_by_pipeline, h.skipped_by_tap), (0, 0));
    }

    #[test]
    fn fps_25_samples_about_25() {
        // 2.5 s of 60 Hz input.
        let h = feed_steady(25, 150, 16.7);
        assert!((60..=65).contains(&h.sampled), "sampled {}", h.sampled);
    }

    #[test]
    fn no_burst_after_a_pause() {
        let (tap, feeder) = FrameTap::manual(30);
        let t0 = Instant::now();
        let mut sampled = Vec::new();
        let mut offer = |at_ms: f64, seq: u64| {
            feeder.offer(t0 + ms(at_ms), &frame(seq));
            while let Ok(t) = tap.recv_timeout(Duration::ZERO) {
                sampled.push(t.frame.seq);
            }
        };
        // 0.5 s at 60 Hz (seqs 2..=60), a 1 s pause, then 60 Hz again (seqs 62..).
        for i in 0..30u64 {
            offer(16.7 * i as f64, 2 + 2 * i);
        }
        let resume = 16.7 * 29.0 + 1000.0;
        for i in 0..6u64 {
            offer(resume + 16.7 * i as f64, 62 + 2 * i);
        }
        assert!(
            sampled.contains(&62),
            "first frame after the pause: {sampled:?}"
        );
        assert!(
            !sampled.contains(&64),
            "no catch-up sample right after it: {sampled:?}"
        );
        assert!(sampled.contains(&66), "cadence resumes: {sampled:?}");
    }

    #[test]
    fn fps_is_clamped() {
        // fps 0 -> 1: one frame a second; fps 1000 -> 60: 120 Hz input gives about 60.
        assert_eq!(feed_steady(0, 100, 10.0).sampled, 2);
        let h = feed_steady(1000, 120, 1000.0 / 120.0);
        assert!((58..=62).contains(&h.sampled), "sampled {}", h.sampled);
    }

    #[test]
    fn seq_gaps_count_pipeline_skips() {
        let (tap, feeder) = FrameTap::manual(30);
        let t0 = Instant::now();
        for (i, seq) in [2, 4, 10].into_iter().enumerate() {
            feeder.offer(t0 + ms(40.0 * i as f64), &frame(seq));
        }
        assert_eq!(tap.health().skipped_by_pipeline, 2);
    }

    #[test]
    fn seq_reset_is_not_a_gap() {
        let (tap, feeder) = FrameTap::manual(30);
        let t0 = Instant::now();
        for (i, seq) in [10, 12, 2, 4].into_iter().enumerate() {
            feeder.offer(t0 + ms(40.0 * i as f64), &frame(seq));
        }
        assert_eq!(tap.health().skipped_by_pipeline, 0);
    }

    #[test]
    fn full_queue_skips_without_blocking() {
        let (tap, feeder) = FrameTap::manual(30);
        let t0 = Instant::now();
        let start = Instant::now();
        for i in 0..10u64 {
            assert!(feeder.offer(t0 + ms(40.0 * i as f64), &frame(2 + 2 * i)));
        }
        let took = start.elapsed();
        assert!(took < Duration::from_millis(50), "10 offers took {took:?}");
        let h = tap.health();
        assert_eq!((h.received, h.sampled, h.skipped_by_tap), (10, 3, 7));
        // The queue holds the first three; draining it makes room again.
        let seqs: Vec<u64> = (0..3)
            .map(|_| tap.recv_timeout(W).unwrap().frame.seq)
            .collect();
        assert_eq!(seqs, [2, 4, 6]);
        assert!(feeder.offer(t0 + ms(400.0), &frame(22)));
        assert_eq!(tap.recv_timeout(W).unwrap().frame.seq, 22);
    }

    #[test]
    fn pauses_are_not_stutters() {
        let (tap, feeder) = FrameTap::manual(30);
        let mut at = Instant::now();
        feeder.offer(at, &frame(2));
        for (i, gap) in [16.0, 40.0, 600.0].into_iter().enumerate() {
            at += ms(gap);
            feeder.offer(at, &frame(4 + 2 * i as u64));
        }
        let h = tap.health();
        assert!(
            (h.max_gap_ms - 40.0).abs() < 0.01,
            "max gap {}",
            h.max_gap_ms
        );
        assert_eq!(h.gaps_over_34ms, 1);
        assert_eq!(h.last_arrival, Some(at));
    }

    #[test]
    fn offer_returns_false_after_tap_drop() {
        let (tap, feeder) = FrameTap::manual(30);
        let t0 = Instant::now();
        assert!(feeder.offer(t0, &frame(2)));
        drop(tap);
        assert!(!feeder.offer(t0 + ms(40.0), &frame(4)));
        assert!(
            !feeder.offer(t0 + ms(41.0), &frame(6)),
            "not sampled, still false"
        );
    }

    #[test]
    fn recv_reports_closed_when_feeders_gone() {
        let (tap, feeder) = FrameTap::manual(30);
        assert_eq!(
            tap.recv_timeout(Duration::from_millis(10)).unwrap_err(),
            TapRecv::Timeout
        );
        feeder.offer(Instant::now(), &frame(2));
        drop(feeder);
        // Frames already queued are still delivered, then Closed.
        assert_eq!(tap.recv_timeout(W).unwrap().frame.seq, 2);
        let start = Instant::now();
        assert_eq!(tap.recv_timeout(W).unwrap_err(), TapRecv::Closed);
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "Closed is immediate"
        );
    }

    #[test]
    fn two_taps_both_receive() {
        let (a, fa) = FrameTap::manual(30);
        let (b, fb) = FrameTap::manual(30);
        let t0 = Instant::now();
        for i in 0..3u64 {
            let (at, f) = (t0 + ms(40.0 * i as f64), frame(2 + 2 * i));
            assert!(fa.offer(at, &f));
            assert!(fb.offer(at, &f));
        }
        for tap in [&a, &b] {
            let got: Vec<(Instant, u64)> = (0..3)
                .map(|_| {
                    let t = tap.recv_timeout(W).unwrap();
                    (t.at, t.frame.seq)
                })
                .collect();
            assert_eq!(got, [(t0, 2), (t0 + ms(40.0), 4), (t0 + ms(80.0), 6)]);
        }
    }
}
