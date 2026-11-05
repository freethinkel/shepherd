//! Helper process supervision: spawn, line I/O, stop. Restart policy lives in the owner.
use crate::error::Error;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

/// Env var that overrides where the helper binaries live.
pub const HELPER_DIR_ENV: &str = "SIMDRIVER_HELPER_DIR";

/// Helper directory: `opt`, else `$SIMDRIVER_HELPER_DIR`, else where the embedded helpers unpack.
pub fn helper_dir(opt: Option<&Path>) -> PathBuf {
    resolve_helper_dir(opt, std::env::var_os(HELPER_DIR_ENV))
}

fn resolve_helper_dir(opt: Option<&Path>, env: Option<OsString>) -> PathBuf {
    if let Some(p) = opt {
        return p.to_path_buf();
    }
    match env {
        Some(e) if !e.is_empty() => PathBuf::from(e),
        _ => crate::baked_helper_dir().to_path_buf(),
    }
}

/// Lock, ignoring poison: our critical sections never leave state half-updated, and the
/// stop/drop paths must not panic.
pub(crate) fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Env var: when set (non-empty), helper stderr is also copied to this process's stderr.
pub const DEBUG_ENV: &str = "SIMDRIVER_DEBUG";

/// Lets an owner promise "once `close` returns, no callback is running or will start".
///
/// Callbacks run inside [`CallbackGate::run`]; `close` marks the gate closed (later `run`s
/// do nothing) and waits for running callbacks to finish, except ones on the closing thread
/// itself (a callback that stops its owner must not wait for itself).
pub(crate) struct CallbackGate {
    st: Mutex<GateState>,
    cv: Condvar,
}

#[derive(Default)]
struct GateState {
    closed: bool,
    active: Vec<ThreadId>,
}

struct GateGuard<'a> {
    gate: &'a CallbackGate,
    me: ThreadId,
}

impl Drop for GateGuard<'_> {
    fn drop(&mut self) {
        let mut st = lock(&self.gate.st);
        if let Some(i) = st.active.iter().position(|t| *t == self.me) {
            st.active.swap_remove(i);
        }
        self.gate.cv.notify_all();
    }
}

impl CallbackGate {
    pub(crate) fn new() -> CallbackGate {
        CallbackGate {
            st: Mutex::new(GateState::default()),
            cv: Condvar::new(),
        }
    }

    /// Run `f` unless the gate is closed. Returns whether it ran.
    pub(crate) fn run(&self, f: impl FnOnce()) -> bool {
        let me = thread::current().id();
        let guard = {
            let mut st = lock(&self.st);
            if st.closed {
                return false;
            }
            st.active.push(me);
            GateGuard { gate: self, me }
        };
        f();
        drop(guard);
        true
    }

    /// True if the calling thread is inside a callback of this gate.
    pub(crate) fn running_here(&self) -> bool {
        let me = thread::current().id();
        lock(&self.st).active.contains(&me)
    }

    /// Close and wait for callbacks running on other threads. Idempotent.
    pub(crate) fn close(&self) {
        let me = thread::current().id();
        let mut st = lock(&self.st);
        st.closed = true;
        while st.active.iter().any(|t| *t != me) {
            st = self.cv.wait(st).unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// Grace period between SIGINT and SIGKILL in [`Supervised::stop`].
const KILL_AFTER: Duration = Duration::from_millis(250);

/// The child plus its exit status once reaped. Every reap happens under this lock, so a
/// signal sent while holding it can never hit a recycled pid.
struct Proc {
    child: Child,
    status: Option<ExitStatus>,
}

impl Proc {
    fn poll(&mut self) -> Option<ExitStatus> {
        if self.status.is_none()
            && let Ok(Some(st)) = self.child.try_wait()
        {
            self.status = Some(st);
        }
        self.status
    }
}

/// A running helper process.
///
/// Threads: one reads stdout and calls `on_line` per line; at stdout EOF it waits for the
/// process to be reaped (and for stderr to drain, bounded) and calls `on_exit` exactly once.
/// Another drains stderr: each line goes to `on_stderr` (if given) and is kept as
/// `last_stderr`; it is copied to this process's stderr only when `$SIMDRIVER_DEBUG` is set
/// (logging is the host's business). Nothing here panics on a failed write.
/// stdin stays open (piped) for the helper's lifetime: both helpers exit on stdin EOF,
/// which is what stops them if this process dies without running `stop`.
pub struct Supervised {
    pid: u32,
    proc_: Arc<Mutex<Proc>>,
    stdin: Mutex<Option<ChildStdin>>,
    last_stderr: Arc<Mutex<Option<String>>>,
    stopped: AtomicBool,
}

impl Supervised {
    /// Spawn `bin args…` with piped stdio. `on_line` gets each stdout line (no newline);
    /// `on_exit` gets the exit code (`None` if killed by a signal) once the process is gone,
    /// including after `stop`. Stderr is only kept (see [`Supervised::last_stderr`]).
    pub fn spawn(
        bin: PathBuf,
        args: Vec<String>,
        on_line: impl Fn(String) + Send + 'static,
        on_exit: impl Fn(Option<i32>) + Send + 'static,
    ) -> Result<Supervised, Error> {
        Supervised::spawn_with_stderr(bin, args, on_line, |_| {}, move |code, _| on_exit(code))
    }

    /// Like [`Supervised::spawn`], plus every non-empty stderr line goes to `on_stderr`, and
    /// `on_exit` also gets the last stderr line (stderr is drained, bounded at 500 ms, first).
    pub fn spawn_with_stderr(
        bin: PathBuf,
        args: Vec<String>,
        on_line: impl Fn(String) + Send + 'static,
        on_stderr: impl Fn(String) + Send + 'static,
        on_exit: impl Fn(Option<i32>, Option<String>) + Send + 'static,
    ) -> Result<Supervised, Error> {
        if !bin.is_file() {
            return Err(Error::HelperMissing(bin));
        }
        let name = bin
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut child = Command::new(&bin)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::Spawn(format!("{}: {e}", bin.display())))?;
        let pid = child.id();
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        // A write after the helper died must fail with EPIPE, not kill a host that does not
        // ignore SIGPIPE (C/Swift hosts don't).
        unsafe {
            libc::fcntl(stdin.as_raw_fd(), libc::F_SETNOSIGPIPE, 1);
        }
        let proc_ = Arc::new(Mutex::new(Proc {
            child,
            status: None,
        }));
        let last_stderr = Arc::new(Mutex::new(None));

        let (err_done_tx, err_done_rx) = mpsc::channel::<()>();
        let keep = last_stderr.clone();
        let debug = std::env::var_os(DEBUG_ENV).is_some_and(|v| !v.is_empty());
        thread::Builder::new()
            .name(format!("{name}-stderr"))
            .spawn(move || {
                let mut r = BufReader::new(stderr);
                let mut buf = Vec::new();
                loop {
                    buf.clear();
                    match r.read_until(b'\n', &mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                    let line = String::from_utf8_lossy(&buf).trim_end().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    if debug {
                        // Helpers prefix their own lines ("[sdframe] …"). Never panic on a
                        // closed/broken host stderr.
                        let _ = writeln!(std::io::stderr().lock(), "{line}");
                    }
                    *lock(&keep) = Some(line.clone());
                    on_stderr(line);
                }
                drop(err_done_tx);
            })
            .map_err(|e| Error::Spawn(format!("stderr thread: {e}")))?;

        let reap = proc_.clone();
        let keep = last_stderr.clone();
        thread::Builder::new()
            .name(format!("{name}-stdout"))
            .spawn(move || {
                let mut r = BufReader::new(stdout);
                let mut buf = Vec::new();
                loop {
                    buf.clear();
                    match r.read_until(b'\n', &mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                    let line = String::from_utf8_lossy(&buf);
                    on_line(line.trim_end_matches(['\n', '\r']).to_string());
                }
                // stdout closed: the helper is exiting. Reap it (stop() may beat us to it).
                let mut nap = Duration::from_millis(1);
                let status = loop {
                    if let Some(st) = lock(&reap).poll() {
                        break st;
                    }
                    thread::sleep(nap);
                    nap = (nap * 2).min(Duration::from_millis(50));
                };
                // Let the last stderr line land so the owner can read it from on_exit.
                let _ = err_done_rx.recv_timeout(Duration::from_millis(500));
                let last = lock(&keep).clone();
                on_exit(status.code(), last);
            })
            .map_err(|e| Error::Spawn(format!("stdout thread: {e}")))?;

        Ok(Supervised {
            pid,
            proc_,
            stdin: Mutex::new(Some(stdin)),
            last_stderr,
            stopped: AtomicBool::new(false),
        })
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Write one line (newline appended). False (and nothing else happens) once stdin is
    /// closed or the helper is gone.
    pub fn send_line(&self, line: &str) -> bool {
        let mut guard = lock(&self.stdin);
        let Some(w) = guard.as_mut() else {
            return false;
        };
        let mut buf = Vec::with_capacity(line.len() + 1);
        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
        w.write_all(&buf).is_ok()
    }

    /// Close our end of the helper's stdin (the helper then exits on EOF).
    pub fn close_stdin(&self) {
        drop(lock(&self.stdin).take());
    }

    /// The helper's most recent non-empty stderr line.
    pub fn last_stderr(&self) -> Option<String> {
        lock(&self.last_stderr).clone()
    }

    /// Close stdin, SIGINT, SIGKILL after 250 ms; returns once the process is reaped.
    /// Idempotent.
    pub fn stop(&self) {
        if self.stopped.swap(true, Ordering::SeqCst) {
            return;
        }
        self.close_stdin();
        {
            let mut p = lock(&self.proc_);
            if p.poll().is_some() {
                return;
            }
            unsafe {
                libc::kill(self.pid as libc::pid_t, libc::SIGINT);
            }
        }
        let deadline = Instant::now() + KILL_AFTER;
        while Instant::now() < deadline {
            if lock(&self.proc_).poll().is_some() {
                return;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let mut p = lock(&self.proc_);
        if p.poll().is_none() {
            let _ = p.child.kill();
            if let Ok(st) = p.child.wait() {
                p.status = Some(st);
            }
        }
    }
}

impl Drop for Supervised {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The `fake_helper` example binary (tests/support/fake_helper.rs).
///
/// `cargo test` builds examples next to the test binaries (`target/<profile>/examples`).
/// When that one is missing or older than its source (e.g. after `cargo test --lib`), it is
/// built once per test run into a separate target dir, so this never contends for the outer
/// cargo's build lock.
#[cfg(test)]
pub(crate) fn fake_helper() -> PathBuf {
    use std::sync::OnceLock;
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let exe = std::env::current_exe().expect("current_exe");
        let profile_dir = exe
            .parent()
            .and_then(Path::parent)
            .expect("target/<profile>/deps/<test>");
        let bin = profile_dir.join("examples").join("fake_helper");
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/fake_helper.rs");
        let mtime = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
        // Fresh if built after the last edit of its source (`cargo test --lib` does not
        // rebuild examples, so an older one may be stale).
        if let (Some(b), Some(s)) = (mtime(&bin), mtime(&src))
            && b >= s
        {
            return bin;
        }
        let target = profile_dir
            .parent()
            .expect("target dir")
            .join("fake-helper");
        let st = std::process::Command::new(env!("CARGO"))
            .args([
                "build",
                "--quiet",
                "--example",
                "fake_helper",
                "--manifest-path",
            ])
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .arg("--target-dir")
            .arg(&target)
            .status()
            .expect("run cargo build --example fake_helper");
        assert!(st.success(), "building fake_helper failed");
        let bin = target.join("debug").join("examples").join("fake_helper");
        assert!(bin.exists(), "{bin:?} missing after build");
        bin
    })
    .clone()
}

/// True once no process with `pid` exists (it exited and was reaped).
#[cfg(test)]
pub(crate) fn pid_gone(pid: u32) -> bool {
    unsafe { libc::kill(pid as libc::pid_t, 0) != 0 && *libc::__error() == libc::ESRCH }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    /// Spawn the fake with channels for its lines and its exit code.
    fn spawn_fake(
        a: &[&str],
    ) -> (
        Supervised,
        mpsc::Receiver<String>,
        mpsc::Receiver<Option<i32>>,
    ) {
        let (ltx, lrx) = mpsc::channel();
        let (etx, erx) = mpsc::channel();
        let s = Supervised::spawn(
            fake_helper(),
            args(a),
            move |l| {
                let _ = ltx.send(l);
            },
            move |c| {
                let _ = etx.send(c);
            },
        )
        .expect("spawn fake");
        (s, lrx, erx)
    }

    fn wait_gone(pid: u32, within: Duration) -> Option<Duration> {
        let t0 = Instant::now();
        while t0.elapsed() < within {
            if pid_gone(pid) {
                return Some(t0.elapsed());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        None
    }

    const W: Duration = Duration::from_secs(5);

    #[test]
    fn helper_dir_precedence() {
        let baked = crate::baked_helper_dir().to_path_buf();
        assert_eq!(resolve_helper_dir(None, None), baked);
        assert_eq!(
            resolve_helper_dir(None, Some("".into())),
            baked,
            "empty env is unset"
        );
        assert_eq!(
            resolve_helper_dir(None, Some("/env".into())),
            PathBuf::from("/env")
        );
        assert_eq!(
            resolve_helper_dir(Some(Path::new("/opt")), Some("/env".into())),
            PathBuf::from("/opt")
        );
        assert_eq!(
            resolve_helper_dir(Some(Path::new("/opt")), None),
            PathBuf::from("/opt")
        );
    }

    #[test]
    fn missing_binary_is_helper_missing() {
        let r = Supervised::spawn(
            PathBuf::from("/nonexistent/sdframe"),
            vec![],
            |_| {},
            |_| {},
        );
        assert!(
            matches!(r, Err(Error::HelperMissing(p)) if p == Path::new("/nonexistent/sdframe"))
        );
    }

    #[test]
    fn lines_then_exit_code() {
        let (_s, lines, exit) = spawn_fake(&["lines", "7", "a", "b c"]);
        assert_eq!(lines.recv_timeout(W).unwrap(), "a");
        assert_eq!(lines.recv_timeout(W).unwrap(), "b c");
        assert_eq!(exit.recv_timeout(W).unwrap(), Some(7));
        assert!(
            exit.recv_timeout(Duration::from_millis(200)).is_err(),
            "on_exit fires once"
        );
    }

    #[test]
    fn send_line_round_trips() {
        let (s, lines, _exit) = spawn_fake(&["echo"]);
        s.send_line(r#"{"t":"x"}"#);
        assert_eq!(lines.recv_timeout(W).unwrap(), r#"{"t":"x"}"#);
    }

    #[test]
    fn stdin_eof_ends_helper() {
        let (s, _lines, exit) = spawn_fake(&["echo"]);
        let pid = s.pid();
        s.close_stdin();
        assert_eq!(exit.recv_timeout(W).unwrap(), Some(0));
        assert!(
            wait_gone(pid, W).is_some(),
            "fake still around after stdin EOF"
        );
        s.send_line("after close is a no-op");
    }

    #[test]
    fn stop_escalates_to_sigkill() {
        let (s, lines, exit) = spawn_fake(&["stubborn"]);
        lines.recv_timeout(W).expect("stubborn is up");
        let pid = s.pid();
        let t0 = Instant::now();
        s.stop();
        let took = t0.elapsed();
        assert!(
            took >= Duration::from_millis(250),
            "SIGKILL only after 250 ms (took {took:?})"
        );
        assert!(took < Duration::from_secs(2), "stop took {took:?}");
        assert!(pid_gone(pid), "stubborn survived stop");
        assert_eq!(
            exit.recv_timeout(W).unwrap(),
            None,
            "killed by a signal: no exit code"
        );
        s.stop(); // idempotent
    }

    #[test]
    fn drop_stops_promptly() {
        let (s, _lines, exit) = spawn_fake(&["echo"]);
        let pid = s.pid();
        let t0 = Instant::now();
        drop(s);
        assert!(pid_gone(pid), "fake survived drop");
        assert!(
            t0.elapsed() < Duration::from_millis(250),
            "drop took {:?}",
            t0.elapsed()
        );
        assert!(
            exit.recv_timeout(W).is_ok(),
            "on_exit still fires after drop"
        );
    }

    #[test]
    fn stderr_goes_to_owner_and_exit() {
        let (stx, srx) = mpsc::channel();
        let (etx, erx) = mpsc::channel();
        let _s = Supervised::spawn_with_stderr(
            fake_helper(),
            args(&["nope"]),
            |_| {},
            move |l| {
                let _ = stx.send(l);
            },
            move |c, last| {
                let _ = etx.send((c, last));
            },
        )
        .unwrap();
        let want = r#"fake_helper: unknown mode Some("nope")"#.to_string();
        assert_eq!(srx.recv_timeout(W).unwrap(), want);
        assert_eq!(erx.recv_timeout(W).unwrap(), (Some(2), Some(want)));
    }

    #[test]
    fn gate_close_waits_for_running_callback_then_blocks_new_ones() {
        let gate = Arc::new(CallbackGate::new());
        let (in_tx, in_rx) = mpsc::channel();
        let done = Arc::new(AtomicBool::new(false));
        let (g, d) = (gate.clone(), done.clone());
        let t = thread::spawn(move || {
            g.run(|| {
                in_tx.send(()).unwrap();
                thread::sleep(Duration::from_millis(200));
                d.store(true, Ordering::SeqCst);
            })
        });
        in_rx.recv_timeout(W).unwrap();
        gate.close();
        assert!(
            done.load(Ordering::SeqCst),
            "close returned while a callback was running"
        );
        assert!(t.join().unwrap());
        assert!(!gate.run(|| panic!("ran after close")));
        // Closing from inside a callback does not wait for itself.
        let gate = CallbackGate::new();
        assert!(!gate.running_here());
        assert!(gate.run(|| {
            assert!(gate.running_here());
            gate.close()
        }));
        assert!(!gate.running_here());
    }

    #[test]
    fn last_stderr_is_kept() {
        let (s, _lines, exit) = spawn_fake(&["nope"]);
        assert_eq!(exit.recv_timeout(W).unwrap(), Some(2));
        assert_eq!(
            s.last_stderr().as_deref(),
            Some(r#"fake_helper: unknown mode Some("nope")"#)
        );
    }
}
