//! One `herdr terminal session control <pane>` child per shown pane: base64 ANSI frames in,
//! JSON command lines out (input, resize, scroll, mouse).
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

pub enum SessionEvent {
    /// Terminal bytes to feed the emulator.
    Frame(Vec<u8>),
    /// The stream is over: herdr's reason, its stderr, or why it could not run.
    Ended(String),
}

pub struct Session {
    child: Child,
    stdin: Mutex<Option<ChildStdin>>,
}

impl Session {
    pub fn spawn(pane: &str, cols: u16, rows: u16) -> anyhow::Result<(Session, Receiver<SessionEvent>)> {
        let herdr = std::env::var("SHEPHERD_HERDR").unwrap_or_else(|_| "herdr".into());
        let mut child = Command::new(&herdr)
            .args(["terminal", "session", "control", pane])
            .args(["--cols", &cols.to_string(), "--rows", &rows.to_string()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("cannot run {herdr}: {e}"))?;
        let (tx, rx) = channel();
        let stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let errors = std::thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        });
        std::thread::spawn(move || read_frames(stdout, errors, tx));
        let stdin = Mutex::new(child.stdin.take());
        Ok((Session { child, stdin }, rx))
    }

    pub fn input(&self, bytes: &[u8]) {
        self.send(json!({"type": "terminal.input", "bytes": B64.encode(bytes)}));
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        self.send(json!({"type": "terminal.resize", "cols": cols, "rows": rows}));
    }

    pub fn scroll(&self, up: bool, lines: u16) {
        let direction = if up { "up" } else { "down" };
        self.send(json!({"type": "terminal.scroll", "direction": direction, "lines": lines, "source": "wheel"}));
    }

    pub fn mouse(&self, action: &str, button: &str, column: u16, row: u16, modifiers: u8) {
        self.send(json!({"type": "terminal.mouse", "action": action, "button": button,
                         "column": column, "row": row, "modifiers": modifiers}));
    }

    fn send(&self, record: Value) {
        if let Some(stdin) = self.stdin.lock().unwrap().as_mut() {
            let _ = writeln!(stdin, "{record}");
        }
    }
}

fn read_frames(stdout: impl Read, errors: std::thread::JoinHandle<String>, tx: Sender<SessionEvent>) {
    for line in BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        let Ok(record) = serde_json::from_str::<Value>(&line) else { continue };
        match record["type"].as_str() {
            Some("terminal.frame") => {
                if let Ok(bytes) = B64.decode(record["bytes"].as_str().unwrap_or("")) {
                    if tx.send(SessionEvent::Frame(bytes)).is_err() {
                        return; // the pane view is gone
                    }
                }
            }
            Some("terminal.closed") => {
                let why = record["reason"].as_str().unwrap_or("the terminal closed").to_string();
                let _ = tx.send(SessionEvent::Ended(why));
                return;
            }
            _ => {}
        }
    }
    let stderr = errors.join().unwrap_or_default();
    let why = stderr.trim();
    let _ = tx.send(SessionEvent::Ended(if why.is_empty() { "herdr exited".into() } else { tail(why, 2000) }));
}

fn tail(s: &str, max: usize) -> String {
    let start = s.len().saturating_sub(max);
    let start = (start..=s.len()).find(|&i| s.is_char_boundary(i)).unwrap_or(s.len());
    s[start..].to_string()
}

impl Drop for Session {
    fn drop(&mut self) {
        // Closing stdin ends the session cleanly; a child that ignores it is killed after a short grace.
        self.stdin.lock().unwrap().take();
        let deadline = Instant::now() + Duration::from_millis(200);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
