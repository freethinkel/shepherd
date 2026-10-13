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
use std::path::{Path, PathBuf};
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
pub(super) fn client_socket() -> PathBuf {
    if let Some(path) = std::env::var_os("HERDR_CLIENT_SOCKET_PATH") {
        return path.into();
    }
    let api = api_socket();
    let stem = api.file_stem().and_then(|s| s.to_str()).unwrap_or("herdr").to_string();
    api.with_file_name(format!("{stem}-client.sock"))
}

/// herdr's API socket: from settings, else `HERDR_SOCKET_PATH`, else `~/.config/herdr/herdr.sock`.
fn api_socket() -> PathBuf {
    SOCKET.read().unwrap().clone().map(PathBuf::from).or_else(|| std::env::var_os("HERDR_SOCKET_PATH").map(PathBuf::from)).unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config/herdr/herdr.sock")
    })
}

/// Tells herdr the colors Shepherd paints terminals with. herdr bakes its host theme into every frame and answers
/// apps' color queries (OSC 10/11, `?997`) with it, but takes one only from the client shell in front: so a client
/// shell connects for a moment, sends it and leaves. With no other client shell, herdr keeps it; herdr's own TUI
/// takes the theme back while it is the one in front.
// ponytail: the moment's client shell claims the layout of herdr's active tab at 80x24; panes Shepherd shows are
// locked to its own sizes, a hidden one reflows until Shepherd shows it. A lasting client shell would avoid it.
pub fn push_host_theme(fg: wire::Rgb, bg: wire::Rgb, ansi: [wire::Rgb; 16], light: bool) -> std::io::Result<()> {
    use wire::{DefaultColorKind, HostAppearance, HostThemeUpdate};
    let mut stream = UnixStream::connect(client_socket())?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let hello = json!({
        "generation": 1, "cell_width_px": 8, "cell_height_px": 16, "surface_size": {"cols": 80, "rows": 24},
        "pixel_mouse": false, "direct_graphics": false, "endpoint_keybindings": false, "mouse_capture": false,
        "surface_active": true, "snapshot_codecs": ["shell.snapshot.v1"], "surface_codecs": ["shell.surface.v1"],
        "input_codecs": ["shell.input.semantic.v1"], "blob_codecs": ["shell.blob.v1"],
    });
    wire::write_message(&mut stream, &ClientMessage::EndpointControl { kind: "endpoint.hello.v1".into(), data: hello.to_string() })?;
    match wire::read_message(&mut stream)? {
        Some(ServerMessage::EndpointControl { kind, .. }) if kind == "endpoint.welcome.v1" => {}
        other => return Err(std::io::Error::other(format!("herdr refused the client shell: {other:?}"))),
    }
    let appearance = if light { HostAppearance::Light } else { HostAppearance::Dark };
    for update in [
        HostThemeUpdate::DefaultColor { kind: DefaultColorKind::Foreground, color: fg },
        HostThemeUpdate::DefaultColor { kind: DefaultColorKind::Background, color: bg },
        HostThemeUpdate::PaletteColors((0..16).map(|i| (i as u8, ansi[i])).collect()),
        HostThemeUpdate::Appearance(appearance),
    ] {
        wire::write_message(&mut stream, &ClientMessage::ClientShellHostTheme { update })?;
    }
    Ok(())
}

pub(super) fn binary() -> String {
    std::env::var("SHEPHERD_HERDR").ok().or_else(|| BINARY.read().unwrap().clone()).unwrap_or_else(find_herdr)
}

/// `herdr` on PATH, else where its installers put it: an app opened from Finder gets launchd's bare PATH.
fn find_herdr() -> String {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .chain([home.join(".local/bin"), home.join(".cargo/bin"), "/opt/homebrew/bin".into(), "/usr/local/bin".into()])
        .map(|dir| dir.join("herdr"))
        .find(|p| p.is_file())
        .map_or_else(|| "herdr".into(), |p| p.to_string_lossy().into_owned())
}

/// Nothing listens on herdr's socket: it is missing, or a server died and left it behind.
pub fn no_server(err: &anyhow::Error) -> bool {
    err.chain().any(|e| {
        e.downcast_ref::<std::io::Error>()
            .is_some_and(|e| matches!(e.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused))
    })
}

/// The herdr session whose server listens on `socket`: `Some(None)` for the default one, `Some(Some(name))` for
/// `herdr --session <name>`, `None` for any other socket.
fn session_of(socket: &Path, home: &Path) -> Option<Option<String>> {
    let config = home.join(".config/herdr");
    if socket == config.join("herdr.sock") {
        return Some(None);
    }
    let dir = socket.strip_prefix(config.join("sessions")).ok()?;
    let mut parts = dir.iter();
    match (parts.next(), parts.next(), parts.next()) {
        (Some(name), Some(file), None) if file == "herdr.sock" => Some(Some(name.to_string_lossy().into_owned())),
        _ => None,
    }
}

/// Runs herdr's headless server (`herdr server`) for the socket Shepherd uses, in its own process group so it
/// outlives Shepherd, as one started from a terminal would. Only herdr's own sessions: a server pointed at another
/// socket would still load the default session's state, and two servers would share it.
pub fn start_server() -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let socket = api_socket();
    let Some(session) = session_of(&socket, &home) else {
        return Err(std::io::Error::other(format!("{} is not a herdr session's socket, so Shepherd leaves starting it to you", socket.display())));
    };
    let mut command = Command::new(binary());
    if let Some(name) = session {
        command.args(["--session", &name]);
    }
    command.arg("server").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).process_group(0);
    // Shepherd may itself run inside a herdr pane: the server must not take that pane's context.
    for var in ["HERDR_ENV", "HERDR_PANE_ID", "HERDR_TAB_ID", "HERDR_WORKSPACE_ID", "HERDR_SOCKET_PATH", "HERDR_CLIENT_SOCKET_PATH"] {
        command.env_remove(var);
    }
    let mut child = command.spawn()?;
    std::thread::spawn(move || child.wait()); // reaped whenever it exits
    Ok(())
}

/// One of herdr's sessions, as `herdr session list --json` lists it.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SessionInfo {
    pub name: String,
    pub running: bool,
    pub socket_path: String,
    #[serde(default)]
    pub default: bool,
}

/// herdr's sessions, from `herdr session list --json`.
pub fn sessions() -> anyhow::Result<Vec<SessionInfo>> {
    let out = Command::new(binary()).args(["session", "list", "--json"]).stdin(Stdio::null()).output()?;
    anyhow::ensure!(out.status.success(), "herdr session list: {}", String::from_utf8_lossy(&out.stderr).trim());
    parse_sessions(&out.stdout)
}

fn parse_sessions(json: &[u8]) -> anyhow::Result<Vec<SessionInfo>> {
    #[derive(serde::Deserialize)]
    struct List {
        sessions: Vec<SessionInfo>,
    }
    Ok(serde_json::from_slice::<List>(json)?.sessions)
}

/// The name of the herdr session Shepherd talks to: `default`, a named one, or `None` for a socket of its own.
pub fn current_session() -> Option<String> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    session_of(&api_socket(), &home).map(|name| name.unwrap_or_else(|| "default".into()))
}

/// herdr's rule for a session's name: ASCII letters, digits, `.`, `_` and `-`.
pub fn valid_session_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// The session `name` where herdr keeps it. herdr has no command to make one: its server makes it on first start,
/// which Shepherd does when nothing answers on the socket.
pub fn new_session(name: &str) -> SessionInfo {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let socket = home.join(".config/herdr/sessions").join(name).join("herdr.sock");
    SessionInfo { name: name.into(), running: false, socket_path: socket.to_string_lossy().into_owned(), default: false }
}

/// Stops `name`'s server when it runs, then deletes the session with its workspaces.
pub fn delete_session(name: &str, running: bool) -> anyhow::Result<()> {
    if running {
        session_command(&["stop", name])?;
    }
    // ponytail: the server may take a moment to let go of its socket after stop answers; ten tries, 200 ms apart
    let mut tries = 0;
    loop {
        match session_command(&["delete", name]) {
            Err(e) if tries < 10 && format!("{e:#}").contains("is running") => tries += 1,
            done => return done,
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// `herdr session <args> --json`: its error message when it answers with one.
fn session_command(args: &[&str]) -> anyhow::Result<()> {
    let out = Command::new(binary()).arg("session").args(args).arg("--json").stdin(Stdio::null()).output()?;
    let answer: Value = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
    if let Some(message) = answer["error"]["message"].as_str() {
        anyhow::bail!("{message}");
    }
    anyhow::ensure!(out.status.success(), "herdr session {}: {}", args[0], String::from_utf8_lossy(&out.stderr).trim());
    Ok(())
}

pub enum SessionEvent {
    /// Terminal bytes to feed the emulator.
    Frame(Vec<u8>),
    /// Whether the pane's app asks for mouse reporting (socket sessions only).
    MouseCapture(bool),
    /// The grid herdr renders the terminal at (a client shell's popup only): the frames that follow are this size.
    Size(u16, u16),
    /// The stream is over: herdr's reason, its stderr, or why it could not run.
    Ended(String),
}

pub struct Session {
    transport: Transport,
}

/// How a session holds its pane. herdr allows one controller, which types into the pane and sets its size;
/// observers only watch, so herdr's other clients size the pane.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Control,
    /// Control, replacing another controller.
    Takeover,
    Observe,
}

enum Transport {
    Socket(Mutex<UnixStream>),
    /// herdr's popup as Shepherd's client shell shows it (PoC, [`super::client_shell`]): frames from its surface,
    /// input as `ClientShellPopupInput`.
    Popup { shell: std::sync::Arc<super::client_shell::ClientShell>, terminal_id: String },
    Cli { child: Option<Child>, stdin: Mutex<Option<ChildStdin>> },
}

impl Session {
    pub fn spawn(pane: &str, cols: u16, rows: u16, mode: Mode) -> anyhow::Result<(Session, UnboundedReceiver<SessionEvent>)> {
        match Self::spawn_socket(pane, cols, rows, mode) {
            Some(session) => Ok(session),
            None => Self::spawn_cli(pane, cols, rows, mode),
        }
    }

    /// herdr's popup `terminal_id` over `shell`: no connection of its own.
    pub fn over_popup(shell: std::sync::Arc<super::client_shell::ClientShell>, terminal_id: &str) -> (Session, UnboundedReceiver<SessionEvent>) {
        let rx = shell.subscribe_popup(terminal_id);
        (Session { transport: Transport::Popup { shell, terminal_id: terminal_id.into() } }, rx)
    }

    /// Semantic input for a client-shell popup; `false` (nothing sent) for the others, which take bytes.
    pub fn shell_input(&self, events: Vec<wire::PaneInput>) -> bool {
        let Transport::Popup { shell, terminal_id } = &self.transport else { return false };
        shell.popup_input(terminal_id, events);
        true
    }

    pub fn is_shell(&self) -> bool {
        matches!(self.transport, Transport::Popup { .. })
    }

    /// `None` when the socket is not there or speaks another protocol version.
    fn spawn_socket(pane: &str, cols: u16, rows: u16, mode: Mode) -> Option<(Session, UnboundedReceiver<SessionEvent>)> {
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
        let hold = match mode {
            Mode::Observe => ClientMessage::ObserveTerminal { target: pane.into() },
            _ => ClientMessage::ControlTerminal { target: pane.into(), takeover: mode == Mode::Takeover },
        };
        wire::write_message(&mut stream, &hold).ok()?;
        let reader = stream.try_clone().ok()?;
        let (tx, rx) = unbounded();
        std::thread::spawn(move || read_socket(reader, tx));
        Some((Session { transport: Transport::Socket(Mutex::new(stream)) }, rx))
    }

    fn spawn_cli(pane: &str, cols: u16, rows: u16, mode: Mode) -> anyhow::Result<(Session, UnboundedReceiver<SessionEvent>)> {
        let herdr = binary();
        let mut child = Command::new(&herdr)
            .envs(SOCKET.read().unwrap().clone().map(|p| ("HERDR_SOCKET_PATH", p)))
            .args(["terminal", "session", if mode == Mode::Observe { "observe" } else { "control" }, pane])
            .args(["--cols", &cols.to_string(), "--rows", &rows.to_string()])
            .args((mode == Mode::Takeover).then_some("--takeover"))
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
        matches!(self.transport, Transport::Socket(_) | Transport::Popup { .. })
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

    pub fn scroll(&self, up: bool, lines: u16, at: Option<(u16, u16)>) {
        let direction = if up { AttachScrollDirection::Up } else { AttachScrollDirection::Down };
        let (column, row) = (at.map(|a| a.0), at.map(|a| a.1));
        self.send(
            ClientMessage::AttachScroll { source: AttachScrollSource::Wheel, direction, lines, column, row, modifiers: 0 },
            || json!({"type": "terminal.scroll", "direction": if up { "up" } else { "down" }, "lines": lines, "source": "wheel",
                      "column": column, "row": row}),
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
            Transport::Popup { shell, terminal_id } => {
                // herdr sizes the popup from the shell's surface: the view's own resize means nothing there
                let event = match message {
                    ClientMessage::Input { data } => wire::PaneInput::TextCommit(String::from_utf8_lossy(&data).into_owned()),
                    ClientMessage::AttachScroll { direction, lines, column, row, .. } => wire::PaneInput::Mouse {
                        kind: if matches!(direction, AttachScrollDirection::Up) { MouseKind::ScrollUp } else { MouseKind::ScrollDown },
                        position: MousePosition::Cell { column: column.unwrap_or(0), row: row.unwrap_or(0) },
                        geometry: None,
                        modifiers: 0,
                        lines,
                    },
                    ClientMessage::AttachMouse { kind, position, geometry, modifiers, lines } => {
                        wire::PaneInput::Mouse { kind, position, geometry, modifiers, lines }
                    }
                    _ => return,
                };
                shell.popup_input(terminal_id, vec![event]);
            }
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
            Transport::Popup { shell, .. } => return shell.unsubscribe_popup(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_herdrs_own_sessions_are_started() {
        let home = Path::new("/Users/me");
        let at = |p: &str| session_of(Path::new(p), home);
        assert_eq!(at("/Users/me/.config/herdr/herdr.sock"), Some(None));
        assert_eq!(at("/Users/me/.config/herdr/sessions/pets/herdr.sock"), Some(Some("pets".into())));
        assert_eq!(at("/Users/me/.config/herdr/sessions/pets/other.sock"), None);
        assert_eq!(at("/Users/me/.config/herdr/sessions/a/b/herdr.sock"), None);
        assert_eq!(at("/tmp/herdr.sock"), None);
    }

    #[test]
    fn session_names_herdr_takes() {
        assert!(valid_session_name("pets") && valid_session_name("client.v2_old-1"));
        assert!(!valid_session_name("") && !valid_session_name("Bad Name") && !valid_session_name("a/b") && !valid_session_name("пэт"));
    }

    #[test]
    fn sessions_from_herdrs_list() {
        let json = br#"{"sessions":[{"default":true,"name":"default","running":true,"session_dir":"/h","socket_path":"/h/herdr.sock"},
            {"default":false,"name":"pets","running":false,"session_dir":"/h/s/pets","socket_path":"/h/s/pets/herdr.sock"}]}"#;
        let list = parse_sessions(json).unwrap();
        assert_eq!(list.iter().map(|s| (s.name.as_str(), s.running, s.default)).collect::<Vec<_>>(), [("default", true, true), ("pets", false, false)]);
        assert_eq!(list[1].socket_path, "/h/s/pets/herdr.sock");
    }
}
