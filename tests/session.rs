use serde_json::{Value, json};
use shepherd::herdr::{Session, SessionEvent};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::mpsc::Receiver;
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
        std::env::set_var("REC", dir);
    }
    Session::spawn("w1:p1", 100, 30).unwrap()
}

static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn next(rx: &Receiver<SessionEvent>) -> SessionEvent {
    rx.recv_timeout(Duration::from_secs(5)).expect("no session event within 5 s")
}

#[test]
fn frames_and_commands() {
    let _l = LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    fake(
        dir.path(),
        r#"printf '%s\n' '{"type":"terminal.frame","seq":1,"encoding":"ansi","width":100,"height":30,"full":true,"bytes":"aGVsbG8="}'
cat > "$REC/stdin""#,
    );
    let (session, rx) = spawn(dir.path());
    assert!(matches!(next(&rx), SessionEvent::Frame(b) if b == b"hello"));
    session.input(b"ls\r");
    session.resize(120, 40);
    session.scroll(true, 3);
    session.mouse("down", "left", 4, 2, 0);
    drop(session); // closes stdin: the fake's cat ends

    let args = std::fs::read_to_string(dir.path().join("args")).unwrap();
    assert_eq!(args.trim(), "terminal session control w1:p1 --cols 100 --rows 30");
    let stdin = std::fs::read_to_string(dir.path().join("stdin")).unwrap();
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
    let _l = LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    fake(dir.path(), r#"printf '%s\n' '{"type":"terminal.closed","reason":"pane closed"}'"#);
    let (_session, rx) = spawn(dir.path());
    match next(&rx) {
        SessionEvent::Ended(why) => assert!(why.contains("pane closed"), "{why}"),
        SessionEvent::Frame(_) => panic!("expected the end"),
    }
    assert!(rx.recv_timeout(Duration::from_secs(5)).is_err(), "nothing after the end");
}

#[test]
fn herdr_errors_reach_the_end_reason() {
    let _l = LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    fake(dir.path(), "echo 'herdr: no pane w1:p1' >&2\nexit 1");
    let (_session, rx) = spawn(dir.path());
    match next(&rx) {
        SessionEvent::Ended(why) => assert!(why.contains("no pane w1:p1"), "{why}"),
        SessionEvent::Frame(_) => panic!("expected the end"),
    }
}

#[test]
fn dropping_session_kills_child() {
    let _l = LOCK.lock().unwrap();
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
    drop(session);
    let alive = std::process::Command::new("kill").args(["-0", &pid]).status().unwrap().success();
    assert!(!alive, "herdr child {pid} outlived its session");
}

/// Against a real herdr: `SHEPHERD_LIVE_PANE=<scratch pane id> cargo test --test session -- --ignored`.
#[test]
#[ignore]
fn live_echo() {
    let _l = LOCK.lock().unwrap();
    let pane = std::env::var("SHEPHERD_LIVE_PANE").expect("SHEPHERD_LIVE_PANE");
    let (session, rx) = Session::spawn(&pane, 80, 24).unwrap();
    session.input(b"echo shepherd-$((6*7))\r");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    while std::time::Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(SessionEvent::Frame(b)) => seen.extend(b),
            Ok(SessionEvent::Ended(why)) => panic!("ended: {why}"),
            Err(_) => {}
        }
        if String::from_utf8_lossy(&seen).contains("shepherd-42") {
            return;
        }
    }
    panic!("no shepherd-42 in {} bytes of frames", seen.len());
}
