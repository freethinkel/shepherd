use serde_json::{Value, json};
use shepherd::herdr::{Mode, Session, SessionEvent};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use futures::channel::mpsc::{TryRecvError, UnboundedReceiver as Receiver};
use std::time::Duration;

/// A fake `herdr` for `terminal session control`; `$REC` is where it records.
fn fake(dir: &Path, body: &str) {
    let path = dir.join("herdr");
    std::fs::write(&path, format!("#!/bin/sh\necho \"$@\" > \"$REC/args\"\necho $$ > \"$REC/pid\"\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn spawn(dir: &Path) -> (Session, Receiver<SessionEvent>) {
    // Each test points SHEPHERD_HERDR at its own fake; tests in this file run one at a time via the lock.
    unsafe {
        std::env::set_var("SHEPHERD_HERDR", dir.join("herdr"));
        // No client socket there: these tests exercise the CLI fallback.
        std::env::set_var("HERDR_CLIENT_SOCKET_PATH", dir.join("no-client.sock"));
        std::env::set_var("REC", dir);
    }
    Session::spawn("w1:p1", 100, 30, Mode::Control).unwrap()
}

static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn recv(rx: &mut Receiver<SessionEvent>, within: Duration) -> Option<SessionEvent> {
    let deadline = std::time::Instant::now() + within;
    while std::time::Instant::now() < deadline {
        match rx.try_recv() {
            Ok(event) => return Some(event),
            Err(TryRecvError::Closed) => return None,
            Err(TryRecvError::Empty) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    None
}

fn next(rx: &mut Receiver<SessionEvent>) -> SessionEvent {
    recv(rx, Duration::from_secs(5)).expect("no session event within 5 s")
}

#[test]
fn frames_and_commands() {
    let _l = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    fake(
        dir.path(),
        r#"printf '%s\n' '{"type":"terminal.frame","seq":1,"encoding":"ansi","width":100,"height":30,"full":true,"bytes":"aGVsbG8="}'
cat > "$REC/stdin""#,
    );
    let (session, mut rx) = spawn(dir.path());
    assert!(matches!(next(&mut rx), SessionEvent::Frame(b) if b == b"hello"));
    session.input(b"ls\r");
    session.resize(120, 40);
    session.scroll(true, 3);
    session.mouse("down", "left", 4, 2, 0);
    drop(session); // closes stdin: the fake's cat ends

    let args = std::fs::read_to_string(dir.path().join("args")).unwrap();
    assert_eq!(args.trim(), "terminal session control w1:p1 --cols 100 --rows 30");
    // The fake finishes writing after the drop returns (the drop does not wait for herdr).
    let stdin_path = dir.path().join("stdin");
    let mut stdin = String::new();
    for _ in 0..50 {
        stdin = std::fs::read_to_string(&stdin_path).unwrap_or_default();
        if stdin.lines().count() == 4 {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let lines: Vec<Value> = stdin.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(
        lines,
        [
            json!({"type": "terminal.input", "bytes": "bHMN"}),
            json!({"type": "terminal.resize", "cols": 120, "rows": 40}),
            json!({"type": "terminal.scroll", "direction": "up", "lines": 3, "source": "wheel"}),
            json!({"type": "terminal.mouse", "action": "down", "button": "left", "column": 4, "row": 2, "modifiers": 0}),
        ]
    );
}

#[test]
fn closed_session_reports_end() {
    let _l = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    fake(dir.path(), r#"printf '%s\n' '{"type":"terminal.closed","reason":"pane closed"}'"#);
    let (_session, mut rx) = spawn(dir.path());
    match next(&mut rx) {
        SessionEvent::Ended(why) => assert!(why.contains("pane closed"), "{why}"),
        _ => panic!("expected the end"),
    }
    assert!(recv(&mut rx, Duration::from_secs(1)).is_none(), "nothing after the end");
}

#[test]
fn herdr_errors_reach_the_end_reason() {
    let _l = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    fake(dir.path(), "echo 'herdr: no pane w1:p1' >&2\nexit 1");
    let (_session, mut rx) = spawn(dir.path());
    match next(&mut rx) {
        SessionEvent::Ended(why) => assert!(why.contains("no pane w1:p1"), "{why}"),
        _ => panic!("expected the end"),
    }
}

#[test]
fn dropping_session_kills_child() {
    let _l = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    fake(dir.path(), "exec sleep 30");
    let (session, _rx) = spawn(dir.path());
    let pid_file = dir.path().join("pid");
    for _ in 0..50 {
        if pid_file.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let pid = std::fs::read_to_string(&pid_file).unwrap().trim().to_string();
    let started = std::time::Instant::now();
    drop(session);
    // Drops run on GPUI's main thread (tab switches drop several at once): never wait there.
    assert!(started.elapsed() < Duration::from_millis(50), "drop blocked for {:?}", started.elapsed());
    let gone = (0..50).any(|_| {
        std::thread::sleep(Duration::from_millis(20));
        !std::process::Command::new("kill").args(["-0", &pid]).status().unwrap().success()
    });
    assert!(gone, "herdr child {pid} outlived its session");
}

/// The client socket path, against a fake herdr server speaking protocol 22 (bytes per bincode 2 `standard()`).
#[test]
fn socket_session_reports_mouse_capture() {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    let _l = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("herdr-client.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let server = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        fn read(s: &mut std::os::unix::net::UnixStream) -> Vec<u8> {
            let mut len = [0u8; 4];
            s.read_exact(&mut len).unwrap();
            let mut payload = vec![0; u32::from_le_bytes(len) as usize];
            s.read_exact(&mut payload).unwrap();
            payload
        }
        fn send(s: &mut std::os::unix::net::UnixStream, payload: &[u8]) {
            s.write_all(&(payload.len() as u32).to_le_bytes()).unwrap();
            s.write_all(payload).unwrap();
        }
        let hello = read(&mut s);
        // Welcome { version: 22, encoding: TerminalAnsi, error: None }.
        send(&mut s, &[0, 22, 1, 0]);
        let control = read(&mut s);
        // Terminal(seq 1, 100×30, full, b"hello"), then MouseCapture { enabled: true, sgr_pixels: false }.
        send(&mut s, &[1, 1, 100, 30, 1, 5, b'h', b'e', b'l', b'l', b'o']);
        send(&mut s, &[8, 1, 0]);
        let input = read(&mut s);
        // ServerShutdown { reason: Some("bye") }.
        send(&mut s, &[3, 1, 3, b'b', b'y', b'e']);
        (hello, control, input)
    });
    unsafe { std::env::set_var("HERDR_CLIENT_SOCKET_PATH", &path) };
    let (session, mut rx) = Session::spawn("w1:p1", 100, 30, Mode::Control).unwrap();
    assert!(session.knows_mouse_capture());
    assert!(matches!(next(&mut rx), SessionEvent::Frame(b) if b == b"hello"));
    assert!(matches!(next(&mut rx), SessionEvent::MouseCapture(true)));
    session.input(b"ls\r");
    assert!(matches!(next(&mut rx), SessionEvent::Ended(why) if why == "bye"));
    let (hello, control, input) = server.join().unwrap();
    // TerminalHello { version: 22, cols: 100, rows: 30, cell px 0×0, pixel_mouse: false }.
    assert_eq!(hello, [0, 22, 100, 30, 0, 0, 0]);
    // ControlTerminal { target: "w1:p1", takeover: false }.
    assert_eq!(control, [8, 5, b'w', b'1', b':', b'p', b'1', 0]);
    // Input { data: b"ls\r" }.
    assert_eq!(input, [1, 3, b'l', b's', b'\r']);
}

/// Against a real herdr: `SHEPHERD_LIVE_PANE=<scratch pane id> cargo test --test session -- --ignored`.
#[test]
#[ignore]
fn live_echo() {
    let _l = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let pane = std::env::var("SHEPHERD_LIVE_PANE").expect("SHEPHERD_LIVE_PANE");
    let (session, mut rx) = Session::spawn(&pane, 80, 24, Mode::Control).unwrap();
    session.input(b"echo shepherd-$((6*7))\r");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    while std::time::Instant::now() < deadline {
        match recv(&mut rx, Duration::from_millis(200)) {
            Some(SessionEvent::Frame(b)) => seen.extend(b),
            Some(SessionEvent::Ended(why)) => panic!("ended: {why}"),
            Some(SessionEvent::MouseCapture(_)) | None => {}
        }
        if String::from_utf8_lossy(&seen).contains("shepherd-42") {
            return;
        }
    }
    panic!("no shepherd-42 in {} bytes of frames", seen.len());
}

#[test]
fn takeover_is_passed_to_herdr() {
    // herdr refuses a second controller ("already has an attached client; retry with --takeover").
    let _l = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    fake(dir.path(), "cat > /dev/null");
    unsafe {
        std::env::set_var("SHEPHERD_HERDR", dir.path().join("herdr"));
        // otherwise the real herdr's client socket answers and the fake never runs
        std::env::set_var("HERDR_CLIENT_SOCKET_PATH", dir.path().join("no-client.sock"));
        std::env::set_var("REC", dir.path());
    }
    let (session, _rx) = Session::spawn("w1:p1", 100, 30, Mode::Takeover).unwrap();
    let args_path = dir.path().join("args");
    let args = (0..50)
        .find_map(|_| {
            std::thread::sleep(Duration::from_millis(20));
            std::fs::read_to_string(&args_path).ok().filter(|a| !a.is_empty())
        })
        .unwrap();
    drop(session);
    assert_eq!(args.trim(), "terminal session control w1:p1 --cols 100 --rows 30 --takeover");
}

#[test]
fn an_observer_runs_herdrs_observe_session() {
    let _l = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    fake(dir.path(), "cat > /dev/null");
    unsafe {
        std::env::set_var("SHEPHERD_HERDR", dir.path().join("herdr"));
        // otherwise the real herdr's client socket answers and the fake never runs
        std::env::set_var("HERDR_CLIENT_SOCKET_PATH", dir.path().join("no-client.sock"));
        std::env::set_var("REC", dir.path());
    }
    let (session, _rx) = Session::spawn("w1:p1", 100, 30, Mode::Observe).unwrap();
    let args_path = dir.path().join("args");
    let args = (0..50)
        .find_map(|_| {
            std::thread::sleep(Duration::from_millis(20));
            std::fs::read_to_string(&args_path).ok().filter(|a| !a.is_empty())
        })
        .unwrap();
    drop(session);
    assert_eq!(args.trim(), "terminal session observe w1:p1 --cols 100 --rows 30");
}

#[test]
fn the_configured_binary_is_used_without_the_env_override() {
    let _l = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    fake(dir.path(), "cat > /dev/null");
    unsafe {
        std::env::remove_var("SHEPHERD_HERDR");
        std::env::set_var("HERDR_CLIENT_SOCKET_PATH", dir.path().join("no-client.sock"));
        std::env::set_var("REC", dir.path());
    }
    shepherd::herdr::set_binary(Some(dir.path().join("herdr").display().to_string()));
    let (session, _rx) = Session::spawn("w1:p1", 100, 30, Mode::Control).unwrap();
    let args_path = dir.path().join("args");
    let ran = (0..50).any(|_| {
        std::thread::sleep(Duration::from_millis(20));
        args_path.exists()
    });
    drop(session);
    shepherd::herdr::set_binary(None);
    assert!(ran, "the configured herdr binary did not run");
}

/// Against a real herdr, over its client socket: an app turning on mouse reporting reaches the session.
/// `SHEPHERD_LIVE_PANE=<scratch pane id> cargo test --test session -- --ignored`.
#[test]
#[ignore]
fn live_mouse_capture() {
    let _l = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let pane = std::env::var("SHEPHERD_LIVE_PANE").expect("SHEPHERD_LIVE_PANE");
    let (session, mut rx) = Session::spawn(&pane, 80, 24, Mode::Control).unwrap();
    assert!(session.knows_mouse_capture(), "the socket handshake fell back to the CLI");
    session.input(b"printf '\\033[?1000h'; sleep 2; printf '\\033[?1000l'\r");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    while std::time::Instant::now() < deadline {
        match recv(&mut rx, Duration::from_millis(200)) {
            Some(SessionEvent::MouseCapture(on)) => seen.push(on),
            Some(SessionEvent::Ended(why)) => panic!("ended: {why}"),
            _ => {}
        }
        if seen.ends_with(&[true, false]) {
            return;
        }
    }
    panic!("mouse capture went {seen:?}, expected on then off");
}

/// How long a pane takes to show up after Shepherd attaches to it, as on every tab switch (network: a live herdr;
/// `SHEPHERD_PERF_PANE=w1:p1 HERDR_SOCKET_PATH=… cargo test --test session attach_timing -- --ignored --nocapture`).
#[test]
#[ignore]
fn attach_timing() {
    let pane = std::env::var("SHEPHERD_PERF_PANE").expect("SHEPHERD_PERF_PANE");
    for _ in 0..5 {
        let t0 = std::time::Instant::now();
        let (session, mut rx) = Session::spawn(&pane, 120, 40, Mode::Control).unwrap();
        let spawned = t0.elapsed();
        let (mut frames, mut bytes, mut first) = (0, 0, None);
        while let Some(event) = recv(&mut rx, Duration::from_millis(300)) {
            if let SessionEvent::Frame(data) = event {
                first.get_or_insert(t0.elapsed());
                frames += 1;
                bytes += data.len();
            }
        }
        println!("spawn {spawned:?}, first frame {first:?}, {frames} frames / {bytes} bytes in the first 300 ms of quiet");
        drop(session);
        std::thread::sleep(Duration::from_millis(200));
    }
}
