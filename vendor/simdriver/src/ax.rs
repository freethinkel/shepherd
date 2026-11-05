//! Accessibility snapshots through the `sdax` helper: spawned on first use and kept
//! running (its first snapshot is slow, later ones take tens of ms), one request at a time.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::helper::{Supervised, lock};

/// Bound on one snapshot, spawning sdax and the guest's cold start included.
const SNAPSHOT_WAIT: Duration = Duration::from_secs(15);

/// One element of the frontmost app's accessibility tree. `frame` is
/// `[x, y, width, height]` in screen points, top-left origin: the space of
/// [`crate::Driver::screen_points`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxNode {
    /// `AXApplication`, `AXButton`, `AXStaticText`, `AXTextField`, `AXSwitch`…
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// The app's `accessibilityIdentifier`, when it set one.
    #[serde(default, rename = "id", skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    pub frame: [f64; 4],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<AxNode>,
}

fn enabled_default() -> bool {
    true
}

impl AxNode {
    /// Centre of the frame in screen points.
    pub fn center(&self) -> (f64, f64) {
        (
            self.frame[0] + self.frame[2] / 2.0,
            self.frame[1] + self.frame[3] / 2.0,
        )
    }

    /// This node and every descendant, depth first.
    pub fn descendants(&self) -> Vec<&AxNode> {
        let mut out = vec![self];
        let mut i = 0;
        while i < out.len() {
            let n = out[i];
            out.splice(i + 1..i + 1, n.children.iter());
            i += 1;
        }
        out
    }
}

#[derive(Deserialize)]
struct Reply {
    t: String,
    #[serde(default)]
    id: u64,
    #[serde(default)]
    root: Option<AxNode>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    detail: Option<String>,
}

enum Msg {
    Line(String),
    Exit(Option<i32>),
}

struct Running {
    helper: Arc<Supervised>,
    rx: Receiver<Msg>,
}

pub(crate) struct Ax {
    helper: Mutex<(PathBuf, Vec<String>)>,
    /// Held for a whole snapshot: requests go one at a time.
    running: Mutex<Option<Running>>,
    /// The live sdax, reachable without `running` so `stop` never waits on a snapshot.
    current: Mutex<Option<Arc<Supervised>>>,
    next_id: AtomicU64,
    stopped: AtomicBool,
}

impl Ax {
    pub(crate) fn new(bin: PathBuf, args: Vec<String>) -> Ax {
        Ax {
            helper: Mutex::new((bin, args)),
            running: Mutex::new(None),
            current: Mutex::new(None),
            next_id: AtomicU64::new(0),
            stopped: AtomicBool::new(false),
        }
    }

    #[cfg(test)]
    pub(crate) fn set_helper(&self, bin: PathBuf, args: Vec<String>) {
        *lock(&self.helper) = (bin, args);
    }

    pub(crate) fn snapshot(&self) -> Result<AxNode, Error> {
        let mut run = lock(&self.running);
        if self.stopped.load(Ordering::SeqCst) {
            return Err(Error::Accessibility("driver stopped".into()));
        }
        if run.is_none() {
            *run = Some(self.spawn()?);
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let (result, keep) = ask(run.as_ref().expect("spawned above"), id);
        if !keep {
            if let Some(r) = run.take() {
                r.helper.stop();
            }
            lock(&self.current).take();
        }
        result
    }

    /// Kill sdax; a snapshot in flight returns an error at once. Later calls fail.
    pub(crate) fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        if let Some(h) = lock(&self.current).take() {
            h.stop();
        }
    }

    fn spawn(&self) -> Result<Running, Error> {
        let (bin, args) = lock(&self.helper).clone();
        let (tx, rx) = mpsc::channel();
        let exit_tx = tx.clone();
        let helper = Arc::new(Supervised::spawn(
            bin,
            args,
            move |l| {
                let _ = tx.send(Msg::Line(l));
            },
            move |code| {
                let _ = exit_tx.send(Msg::Exit(code));
            },
        )?);
        *lock(&self.current) = Some(helper.clone());
        // `stop` may have run between the check in `snapshot` and here.
        if self.stopped.load(Ordering::SeqCst) {
            helper.stop();
            lock(&self.current).take();
            return Err(Error::Accessibility("driver stopped".into()));
        }
        Ok(Running { helper, rx })
    }
}

/// Ask for snapshot `id`. The flag says whether this sdax is still usable.
fn ask(run: &Running, id: u64) -> (Result<AxNode, Error>, bool) {
    // If sdax is gone already, its exit (and maybe an error line) is on the way.
    let _ = run
        .helper
        .send_line(&format!(r#"{{"t":"snapshot","id":{id}}}"#));
    let deadline = Instant::now() + SNAPSHOT_WAIT;
    let mut detail: Option<String> = None;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match run.rx.recv_timeout(left) {
            Ok(Msg::Line(l)) => {
                let Ok(r) = serde_json::from_str::<Reply>(&l) else {
                    continue;
                };
                match r.t.as_str() {
                    "ax" if r.id == id => {
                        return match (r.root, r.error) {
                            (Some(root), _) => (Ok(root), true),
                            (None, e) => (
                                Err(Error::Accessibility(
                                    e.unwrap_or_else(|| "empty reply".into()),
                                )),
                                true,
                            ),
                        };
                    }
                    "error" => detail = r.detail,
                    _ => {}
                }
            }
            Ok(Msg::Exit(code)) => {
                let detail = detail.or_else(|| run.helper.last_stderr());
                let text = detail.unwrap_or_else(|| match code {
                    Some(c) => format!("sdax exited with code {c}"),
                    None => "sdax was killed".into(),
                });
                let e = match code {
                    Some(4) => Error::Unsupported(format!("accessibility: {text}")),
                    Some(3) => Error::Spawn(text),
                    _ => Error::Accessibility(text),
                };
                return (Err(e), false);
            }
            Err(RecvTimeoutError::Timeout) => {
                return (
                    Err(Error::Accessibility(format!(
                        "no snapshot within {}s",
                        SNAPSHOT_WAIT.as_secs()
                    ))),
                    false,
                );
            }
            Err(RecvTimeoutError::Disconnected) => {
                return (Err(Error::Accessibility("sdax went away".into())), false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helper::fake_helper;

    fn fake(mode: &[&str]) -> Ax {
        let mut args = vec!["ax".to_string()];
        args.extend(mode.iter().map(|s| s.to_string()));
        Ax::new(fake_helper(), args)
    }

    #[test]
    fn snapshots_parse_and_reuse_one_helper() {
        let ax = fake(&[]);
        let a = ax.snapshot().unwrap();
        assert_eq!(a.role, "AXApplication");
        assert_eq!(a.children[0].label.as_deref(), Some("Continue"));
        assert_eq!(a.children[0].identifier.as_deref(), Some("go"));
        assert_eq!(a.children[0].center(), (196.5, 725.0));
        assert_eq!(a.descendants().len(), 2);
        let pid = lock(&ax.current).as_ref().unwrap().pid();
        ax.snapshot().unwrap();
        assert_eq!(
            lock(&ax.current).as_ref().unwrap().pid(),
            pid,
            "sdax was respawned"
        );
        ax.stop();
        assert!(matches!(ax.snapshot(), Err(Error::Accessibility(_))));
    }

    #[test]
    fn exit_4_is_unsupported_and_names_the_missing_piece() {
        let ax = fake(&["unsupported"]);
        match ax.snapshot() {
            Err(Error::Unsupported(m)) => {
                assert!(m.contains("frontmostApplicationWithDisplayId"), "{m}")
            }
            other => panic!("{other:?}"),
        }
        assert!(lock(&ax.current).is_none(), "a dead sdax is dropped");
    }

    #[test]
    fn stop_unblocks_a_snapshot_in_flight() {
        let ax = Arc::new(fake(&["hang"]));
        let a = ax.clone();
        let t = std::thread::spawn(move || a.snapshot());
        while lock(&ax.current).is_none() {
            std::thread::sleep(Duration::from_millis(5));
        }
        let t0 = Instant::now();
        ax.stop();
        assert!(t.join().unwrap().is_err());
        assert!(t0.elapsed() < Duration::from_secs(2), "{:?}", t0.elapsed());
    }

    #[test]
    fn node_json_round_trips_compactly() {
        let n = AxNode {
            role: "AXButton".into(),
            label: Some("Go".into()),
            value: None,
            placeholder: None,
            identifier: None,
            enabled: true,
            frame: [1.0, 2.0, 3.0, 4.0],
            children: vec![],
        };
        let j = serde_json::to_string(&n).unwrap();
        assert_eq!(
            j,
            r#"{"role":"AXButton","label":"Go","enabled":true,"frame":[1.0,2.0,3.0,4.0]}"#
        );
        assert_eq!(serde_json::from_str::<AxNode>(&j).unwrap(), n);
    }
}
