//! One controlled herdr terminal per shown pane: ANSI frames in; input, resize, scroll and mouse out.
//!
//! Spoken over herdr's client socket ([`super::wire`]) when the installed herdr has the same protocol version, so
//! Shepherd learns when the pane's app captures the mouse (`MouseCapture`, which the JSON CLI drops). Otherwise —
//! another herdr version, no socket — it falls back to a `herdr terminal session control <pane>` child (base64
//! frames in, JSON command lines out), where mouse capture stays unknown.
use super::wire::{self, AttachScrollDirection, AttachScrollSource, ClientMessage, MouseButton, MouseKind, MousePosition, ServerMessage};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use std::sync::Mutex;
use std::time::{Duration, Instant};

static BINARY: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);
static SOCKET: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

/// The herdr binary sessions run (from settings); `None` is `herdr` on PATH. `SHEPHERD_HERDR` overrides it.
pub fn set_binary(path: Option<String>) {
    *BINARY.write().unwrap() = path.filter(|p| !p.trim().is_empty());
}

/// herdr's API socket (from settings); `None` is `HERDR_SOCKET_PATH`, else `~/.config/herdr/herdr.sock`.
pub fn set_socket(path: Option<String>) {
    *SOCKET.write().unwrap() = path.filter(|p| !p.trim().is_empty());
}

/// herdr's client socket, resolved as herdr does: `HERDR_CLIENT_SOCKET_PATH`, else `<api socket stem>-client.sock`
/// next to the API socket.
fn client_socket() -> PathBuf {
    if let Some(path) = std::env::var_os("HERDR_CLIENT_SOCKET_PATH") {
        return path.into();
    }
    let api = SOCKET.read().unwrap().clone().map(PathBuf::from).or_else(|| std::env::var_os("HERDR_SOCKET_PATH").map(PathBuf::from)).unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config/herdr/herdr.sock")
    });
    let stem = api.file_stem().and_then(|s| s.to_str()).unwrap_or("herdr").to_string();
    api.with_file_name(format!("{stem}-client.sock"))
}

fn binary() -> String {
    std::env::var("SHEPHERD_HERDR")
        .ok()
        .or_else(|| BINARY.read().unwrap().clone())
        .unwrap_or_else(|| "herdr".into())
}

pub enum SessionEvent {
    /// Terminal bytes to feed the emulator.
    Frame(Vec<u8>),
    /// Whether the pane's app asks for mouse reporting (socket sessions only).
    MouseCapture(bool),
    /// The stream is over: herdr's reason, its stderr, or why it could not run.
    Ended(String),
}

pub struct Session {
    transport: Transport,
}

enum Transport {
    Socket(Mutex<UnixStream>),
    Cli { child: Option<Child>, stdin: Mutex<Option<ChildStdin>> },
}

impl Session {
    /// `takeover` replaces another client's control of the pane (herdr allows one controller).
    pub fn spawn(pane: &str, cols: u16, rows: u16, takeover: bool) -> anyhow::Result<(Session, UnboundedReceiver<SessionEvent>)> {
        match Self::spawn_socket(pane, cols, rows, takeover) {
            Some(session) => Ok(session),
            None => Self::spawn_cli(pane, cols, rows, takeover),
        }
    }

    /// `None` when the socket is not there or speaks another protocol version.
    fn spawn_socket(pane: &str, cols: u16, rows: u16, takeover: bool) -> Option<(Session, UnboundedReceiver<SessionEvent>)> {
        let mut stream = UnixStream::connect(client_socket()).ok()?;
        stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
        let hello = ClientMessage::TerminalHello {
            version: wire::PROTOCOL_VERSION,
            cols,
            rows,
            cell_width_px: 0,
            cell_height_px: 0,
            pixel_mouse: false,
        };
        wire::write_message(&mut stream, &hello).ok()?;
        match wire::read_message(&mut stream).ok()?? {
            ServerMessage::Welcome { error: None, encoding: wire::RenderEncoding::TerminalAnsi, .. } => {}
            _ => return None,
        }
        stream.set_read_timeout(None).ok()?;
        wire::write_message(&mut stream, &ClientMessage::ControlTerminal { target: pane.into(), takeover }).ok()?;
        let reader = stream.try_clone().ok()?;
        let (tx, rx) = unbounded();
        std::thread::spawn(move || read_socket(reader, tx));
        Some((Session { transport: Transport::Socket(Mutex::new(stream)) }, rx))
    }

    fn spawn_cli(pane: &str, cols: u16, rows: u16, takeover: bool) -> anyhow::Result<(Session, UnboundedReceiver<SessionEvent>)> {
        let herdr = binary();
        let mut child = Command::new(&herdr)
            .envs(SOCKET.read().unwrap().clone().map(|p| ("HERDR_SOCKET_PATH", p)))
            .args(["terminal", "session", "control", pane])
            .args(["--cols", &cols.to_string(), "--rows", &rows.to_string()])
            .args(takeover.then_some("--takeover"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("cannot run {herdr}: {e}"))?;
        let (tx, rx) = unbounded();
        let stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let errors = std::thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        });
        std::thread::spawn(move || read_frames(stdout, errors, tx));
        let stdin = Mutex::new(child.stdin.take());
        Ok((Session { transport: Transport::Cli { child: Some(child), stdin } }, rx))
    }

    /// Whether this session reports [`SessionEvent::MouseCapture`] (the socket does, the CLI cannot).
    pub fn knows_mouse_capture(&self) -> bool {
        matches!(self.transport, Transport::Socket(_))
    }

    pub fn input(&self, bytes: &[u8]) {
        self.send(
            ClientMessage::Input { data: bytes.to_vec() },
            || json!({"type": "terminal.input", "bytes": B64.encode(bytes)}),
        );
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        self.send(
            ClientMessage::Resize { cols, rows, cell_width_px: 0, cell_height_px: 0, pixel_mouse: false },
            || json!({"type": "terminal.resize", "cols": cols, "rows": rows}),
        );
    }

    pub fn scroll(&self, up: bool, lines: u16) {
        let direction = if up { AttachScrollDirection::Up } else { AttachScrollDirection::Down };
        self.send(
            ClientMessage::AttachScroll { source: AttachScrollSource::Wheel, direction, lines, column: None, row: None, modifiers: 0 },
            || json!({"type": "terminal.scroll", "direction": if up { "up" } else { "down" }, "lines": lines, "source": "wheel"}),
        );
    }

    /// `action` is `down` / `up` / `drag` / `move`, `button` is `left` / `right` / `middle` (herdr's CLI names).
    pub fn mouse(&self, action: &str, button: &str, column: u16, row: u16, modifiers: u8) {
        let b = match button {
            "right" => MouseButton::Right,
            "middle" => MouseButton::Middle,
            _ => MouseButton::Left,
        };
        let kind = match action {
            "down" => MouseKind::Down(b),
            "up" => MouseKind::Up(b),
            "drag" => MouseKind::Drag(b),
            _ => MouseKind::Moved,
        };
        self.send(
            ClientMessage::AttachMouse { kind, position: MousePosition::Cell { column, row }, geometry: None, modifiers, lines: 1 },
            || json!({"type": "terminal.mouse", "action": action, "button": button,
                      "column": column, "row": row, "modifiers": modifiers}),
        );
    }

    fn send(&self, message: ClientMessage, record: impl FnOnce() -> Value) {
        match &self.transport {
            Transport::Socket(stream) => drop(wire::write_message(&mut *stream.lock().unwrap(), &message)),
            Transport::Cli { stdin, .. } => {
                if let Some(stdin) = stdin.lock().unwrap().as_mut() {
                    let _ = writeln!(stdin, "{}", record());
                }
            }
        }
    }
}

fn read_socket(mut stream: UnixStream, tx: UnboundedSender<SessionEvent>) {
    let why = loop {
        let event = match wire::read_message(&mut stream) {
            Ok(Some(ServerMessage::Terminal(frame))) => SessionEvent::Frame(frame.bytes),
            Ok(Some(ServerMessage::MouseCapture { enabled, .. })) => SessionEvent::MouseCapture(enabled),
            Ok(Some(ServerMessage::ServerShutdown { reason })) => break reason.unwrap_or_else(|| "the terminal closed".into()),
            Ok(_) => continue,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break "herdr closed the connection".into(),
            Err(e) => break format!("herdr connection failed: {e}"),
        };
        if tx.unbounded_send(event).is_err() {
            return; // the pane view is gone
        }
    };
    let _ = tx.unbounded_send(SessionEvent::Ended(why));
}

fn read_frames(stdout: impl Read, errors: std::thread::JoinHandle<String>, tx: UnboundedSender<SessionEvent>) {
    for line in BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        let Ok(record) = serde_json::from_str::<Value>(&line) else { continue };
        match record["type"].as_str() {
            Some("terminal.frame") => {
                if let Ok(bytes) = B64.decode(record["bytes"].as_str().unwrap_or("")) {
                    if tx.unbounded_send(SessionEvent::Frame(bytes)).is_err() {
                        return; // the pane view is gone
                    }
                }
            }
            Some("terminal.closed") => {
                let why = record["reason"].as_str().unwrap_or("the terminal closed").to_string();
                let _ = tx.unbounded_send(SessionEvent::Ended(why));
                return;
            }
            _ => {}
        }
    }
    let stderr = errors.join().unwrap_or_default();
    let why = stderr.trim();
    let _ = tx.unbounded_send(SessionEvent::Ended(if why.is_empty() { "herdr exited".into() } else { tail(why, 2000) }));
}

fn tail(s: &str, max: usize) -> String {
    let start = s.len().saturating_sub(max);
    let start = (start..=s.len()).find(|&i| s.is_char_boundary(i)).unwrap_or(s.len());
    s[start..].to_string()
}

impl Drop for Session {
    fn drop(&mut self) {
        let (child, stdin) = match &mut self.transport {
            Transport::Socket(stream) => {
                // Detach releases control at once; the shutdown also ends the reader thread.
                let stream = stream.get_mut().unwrap();
                let _ = wire::write_message(stream, &ClientMessage::Detach);
                let _ = stream.shutdown(std::net::Shutdown::Both);
                return;
            }
            Transport::Cli { child, stdin } => (child, stdin),
        };
        // Closing stdin ends the session cleanly. Waiting for herdr happens off this thread: drops run on
        // GPUI's main thread, several at once on a tab switch. A child that ignores the close is killed.
        stdin.lock().unwrap().take();
        let Some(mut child) = child.take() else { return };
        std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_millis(500);
            while Instant::now() < deadline {
                if let Ok(Some(_)) = child.try_wait() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let _ = child.kill();
            let _ = child.wait();
        });
    }
}
