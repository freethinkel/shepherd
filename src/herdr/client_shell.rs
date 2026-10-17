//! PoC (`SHEPHERD_CLIENT_SHELL=1`), the hybrid: Shepherd's panes stay on their attach connections, and one lasting
//! herdr client shell per window does what an attach cannot: tell herdr Shepherd's colors (`ClientShellHostTheme`,
//! which herdr takes only from the client shell in front), show herdr's popup (no pane: it reaches client shells
//! only, in their `PaneSurface`) and hear the `[[keys.command]]` list (the shell snapshot).
//!
//! The shell's surface is the window's pane grid, so herdr fits the popup to it; the surface itself is not drawn.
//! The popup's cells are replayed as ANSI into the popup view's emulator ([`super::SessionEvent`]); its input goes
//! back as `ClientShellPopupInput`.
//!
//! herdr's Kitty graphics reach client shells only (attaches get cells), with the surface: the images on the panes
//! of the shell's tab go to their views ([`super::graphics`]). herdr sends them once it knows the cell size in pixels.
use super::popup::Event as PopupEvent;
use super::session::{SessionEvent, client_socket};
use super::wire::{self, CellData, ClientMessage, CursorState, FrameData, KeyCode, KeyKind, PaneInput, PopupFrame, PopupSurface, ServerMessage, SurfaceRect, SurfaceSize};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use std::io::Write as _;
use super::socket::{self, Stream};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

pub use super::wire::{DefaultColorKind, HostAppearance, HostThemeUpdate, Rgb};

/// The PoC runs only when asked for.
pub fn enabled() -> bool {
    std::env::var_os("SHEPHERD_CLIENT_SHELL").is_some_and(|v| v == "1")
}

#[derive(Default)]
struct State {
    /// The popup view's frames: (its terminal id, the channel).
    popup_view: Option<(String, UnboundedSender<SessionEvent>)>,
    /// herdr's popup as last sent, to replay it to a view that subscribes late.
    popup: Option<PopupFrame>,
    /// The popup size last sent to the view.
    popup_size: Option<(u16, u16)>,
    /// The host's default fg/bg as herdr packs them: herdr resolves "default" to these RGBs in the popup's cells, and
    /// the view gets them back as default, so it paints them with the theme.
    defaults: (u32, u32),
    /// herdr's run, from its surfaces: an endpoint request names it.
    boot_id: String,
    /// The endpoint request waited on: its id, and who to tell once herdr has answered it.
    // ponytail: one at a time; a newer request leaves the older one to its timeout
    answer: Option<(String, std::sync::mpsc::Sender<()>)>,
}

pub struct ClientShell {
    stream: Mutex<Stream>,
    state: Arc<Mutex<State>>,
}

impl ClientShell {
    /// Handshakes on herdr's client socket and starts reading. `cols`×`rows` is the pane grid, `cell_px` a cell in
    /// device pixels ((0, 0) while unknown). The shell is active: herdr sends popups and takes a theme only from an
    /// active client shell, and makes it the one in front.
    pub fn connect(cols: u16, rows: u16, cell_px: (u32, u32)) -> anyhow::Result<(Arc<ClientShell>, UnboundedReceiver<PopupEvent>)> {
        let mut stream = socket::connect(&client_socket())?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        let hello = serde_json::json!({
            "generation": 1,
            "cell_width_px": cell_px.0,
            "cell_height_px": cell_px.1,
            "surface_size": {"cols": cols.max(20), "rows": rows.max(8)},
            "pixel_mouse": false,
            "direct_graphics": false,
            "endpoint_keybindings": false,
            "mouse_capture": false,
            "surface_active": true,
            "snapshot_codecs": ["shell.snapshot.v1"],
            "surface_codecs": ["shell.surface.v1"],
            "input_codecs": ["shell.input.semantic.v1"],
            "blob_codecs": ["shell.blob.v1"],
        });
        wire::write_message(&mut stream, &ClientMessage::EndpointControl { kind: "endpoint.hello.v1".into(), data: hello.to_string() })?;
        let Some(ServerMessage::EndpointControl { data, .. }) = wire::read_message(&mut stream)? else {
            anyhow::bail!("herdr did not welcome the client shell");
        };
        let welcome: serde_json::Value = serde_json::from_str(&data)?;
        if let Some(error) = welcome.get("error").filter(|e| !e.is_null()) {
            anyhow::bail!("herdr refused the client shell: {}", error["message"]);
        }
        stream.set_read_timeout(None)?;
        let state = Arc::new(Mutex::new(State::default()));
        let (tx, rx) = unbounded();
        let reader = stream.try_clone()?;
        let shared = state.clone();
        std::thread::spawn(move || read(reader, shared, tx));
        Ok((Arc::new(ClientShell { stream: Mutex::new(stream), state }), rx))
    }

    fn send(&self, message: &ClientMessage) {
        let _ = wire::write_message(&mut *self.stream.lock().unwrap(), message);
    }

    /// The popup `terminal_id`'s frames from now on, the current one first.
    pub fn subscribe_popup(&self, terminal_id: &str) -> UnboundedReceiver<SessionEvent> {
        let (tx, rx) = unbounded();
        let mut state = self.state.lock().unwrap();
        state.popup_size = None;
        let defaults = state.defaults;
        if let Some(popup) = state.popup.take() {
            if popup.terminal_id == terminal_id {
                send_popup(&mut state.popup_size, &tx, &popup, defaults);
            }
            state.popup = Some(popup);
        }
        state.popup_view = Some((terminal_id.into(), tx));
        rx
    }

    pub fn unsubscribe_popup(&self) {
        self.state.lock().unwrap().popup_view = None;
    }

    /// The pane grid, in cells, and a cell in device pixels: herdr fits its popup to the grid and sizes images by
    /// the cell.
    // ponytail: also lays herdr's hidden tabs out at this size while no other client shell is attached (the same as
    // the popup watch in main)
    pub fn resize(&self, cols: u16, rows: u16, cell_px: (u32, u32)) {
        self.send(&ClientMessage::ClientShellResize {
            cell_width_px: cell_px.0,
            cell_height_px: cell_px.1,
            surface_size: SurfaceSize { cols: cols.max(20), rows: rows.max(8) },
            pixel_mouse: false,
        });
    }

    /// Points this shell's tab, and with it herdr's focus, at `tab_id`; returns once herdr has done it (or after a
    /// second). A popup herdr opens from then on belongs to that tab, so it comes to this shell. Other client shells
    /// (herdr's TUI) keep their own tab.
    pub fn focus_tab(&self, tab_id: &str) {
        let (tx, rx) = std::sync::mpsc::channel();
        let boot_id = {
            let mut state = self.state.lock().unwrap();
            state.answer = Some(("tab.focus".into(), tx));
            state.boot_id.clone()
        };
        let request = serde_json::json!({"id": "tab.focus", "method": "tab.focus", "params": {"tab_id": tab_id}}).to_string();
        self.send(&ClientMessage::ClientShellEndpointRequest { boot_id, request });
        let _ = rx.recv_timeout(Duration::from_secs(1));
    }

    pub fn popup_input(&self, terminal_id: &str, events: Vec<PaneInput>) {
        self.send(&ClientMessage::ClientShellPopupInput { terminal_id: terminal_id.into(), events });
    }

    /// Shepherd's colors for herdr's host theme. Sending also makes this shell the one in front again.
    pub fn theme(&self, updates: Vec<HostThemeUpdate>) {
        let packed = |c: &Rgb| 0x02_00_00_00 | (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32;
        {
            let mut state = self.state.lock().unwrap();
            for update in &updates {
                match update {
                    HostThemeUpdate::DefaultColor { kind: DefaultColorKind::Foreground, color } => state.defaults.0 = packed(color),
                    HostThemeUpdate::DefaultColor { kind: DefaultColorKind::Background, color } => state.defaults.1 = packed(color),
                    _ => {}
                }
            }
        }
        for update in updates {
            self.send(&ClientMessage::ClientShellHostTheme { update });
        }
    }
}

impl Drop for ClientShell {
    fn drop(&mut self) {
        let stream = self.stream.get_mut().unwrap();
        let _ = wire::write_message(stream, &ClientMessage::Detach);
        let _ = stream.shutdown(std::net::Shutdown::Both);
    }
}

fn read(mut stream: Stream, state: Arc<Mutex<State>>, tx: UnboundedSender<PopupEvent>) {
    let mut shown: Option<PopupSurface> = None;
    let images = tx.clone();
    let scenes = super::graphics::worker(move |i| images.unbounded_send(PopupEvent::Images(i)).is_ok());
    loop {
        let message = match wire::read_message(&mut stream) {
            Ok(Some(message)) => message,
            Ok(None) => continue,
            Err(_) => break,
        };
        match message {
            // ponytail: the whole surface is decoded for its popup; the panes' cells are dropped unread by Shepherd
            ServerMessage::PaneSurface(mut surface) => {
                let _ = scenes.send(std::mem::take(&mut surface.graphics));
                let mut state = state.lock().unwrap();
                let state = &mut *state;
                state.boot_id.clone_from(&surface.boot_id);
                let surface_popup = surface.popup.as_ref().map(PopupSurface::from);
                if surface_popup != shown {
                    shown = surface_popup;
                    let _ = tx.unbounded_send(PopupEvent::Popup(shown.clone()));
                }
                if let (Some(popup), Some((id, view))) = (&surface.popup, &state.popup_view)
                    && &popup.terminal_id == id
                {
                    send_popup(&mut state.popup_size, view, popup, state.defaults);
                }
                state.popup = surface.popup;
            }
            ServerMessage::Clipboard { data } => {
                if let Some(text) = super::popup::clipboard(&data) {
                    let _ = tx.unbounded_send(PopupEvent::Clipboard(text));
                }
            }
            ServerMessage::EndpointControl { kind, data } if kind == "shell.snapshot.v1" => {
                let _ = tx.unbounded_send(PopupEvent::Commands(super::popup::commands(&data)));
            }
            ServerMessage::ClientShellEndpointResponseChunk { request_id, final_chunk: true, .. } => {
                let mut state = state.lock().unwrap();
                if state.answer.as_ref().is_some_and(|(id, _)| *id == request_id) {
                    let _ = state.answer.take().map(|(_, tx)| tx.send(()));
                }
            }
            ServerMessage::ServerShutdown { .. } => break,
            _ => {}
        }
    }
    if let Some((_, view)) = &state.lock().unwrap().popup_view {
        let _ = view.unbounded_send(SessionEvent::Ended("herdr closed the client shell".into()));
    }
}

fn send_popup(size: &mut Option<(u16, u16)>, tx: &UnboundedSender<SessionEvent>, popup: &PopupFrame, defaults: (u32, u32)) {
    let frame = &popup.frame;
    if *size != Some((frame.width, frame.height)) {
        *size = Some((frame.width, frame.height));
        let _ = tx.unbounded_send(SessionEvent::Size(frame.width, frame.height));
    }
    let _ = tx.unbounded_send(SessionEvent::MouseCapture(popup.mouse_reporting));
    let area = SurfaceRect { x: 0, y: 0, width: frame.width, height: frame.height };
    let _ = tx.unbounded_send(SessionEvent::Frame(pane_ansi(frame, &area, frame.cursor.as_ref(), defaults)));
}

/// `area` of `frame` as a full repaint for an emulator of exactly its size: every row from its first column, the
/// styles as SGR, links as OSC 8, then the cursor (hidden unless it is in `area`). `defaults` (packed fg, bg) go
/// out as the emulator's default colors.
pub fn pane_ansi(frame: &FrameData, area: &SurfaceRect, cursor: Option<&CursorState>, defaults: (u32, u32)) -> Vec<u8> {
    let mut out = Vec::with_capacity(area.width as usize * area.height as usize * 2);
    out.extend_from_slice(b"\x1b[?25l\x1b[0m");
    let width = frame.width as usize;
    let mut link: Option<u32> = None;
    for row in 0..area.height {
        let _ = write!(out, "\x1b[{};1H", row + 1);
        let mut style = None;
        let mut covered = 0; // the cells after a wide char: the emulator fills them itself
        for col in 0..area.width {
            let Some(cell) = frame.cells.get((area.y + row) as usize * width + (area.x + col) as usize) else { break };
            if covered > 0 {
                covered -= 1;
                continue;
            }
            if style != Some((cell.fg, cell.bg, cell.modifier)) {
                style = Some((cell.fg, cell.bg, cell.modifier));
                sgr(&mut out, cell, defaults);
            }
            if cell.hyperlink != link {
                link = cell.hyperlink;
                let uri = link.and_then(|i| frame.hyperlinks.get(i as usize)).map_or("", String::as_str);
                let _ = write!(out, "\x1b]8;;{uri}\x1b\\");
            }
            let symbol = if cell.symbol.is_empty() { " " } else { cell.symbol.as_str() };
            out.extend_from_slice(symbol.as_bytes());
            covered = symbol.width().saturating_sub(1);
        }
    }
    if link.is_some() {
        out.extend_from_slice(b"\x1b]8;;\x1b\\");
    }
    out.extend_from_slice(b"\x1b[0m");
    if let Some(c) = cursor.filter(|c| c.visible) {
        let inside = (area.x..area.x + area.width).contains(&c.x) && (area.y..area.y + area.height).contains(&c.y);
        if inside {
            let _ = write!(out, "\x1b[{};{}H\x1b[?25h", c.y - area.y + 1, c.x - area.x + 1);
        }
    }
    out
}

/// herdr's packed colors and ratatui modifier bits as one SGR.
fn sgr(out: &mut Vec<u8>, cell: &CellData, (fg, bg): (u32, u32)) {
    out.extend_from_slice(b"\x1b[0");
    for (bit, code) in [(1u16, "1"), (2, "2"), (4, "3"), (8, "4"), (16, "5"), (64, "7"), (128, "8"), (256, "9")] {
        if cell.modifier & bit != 0 {
            let _ = write!(out, ";{code}");
        }
    }
    color(out, if cell.fg == fg { 0 } else { cell.fg }, 38);
    color(out, if cell.bg == bg { 0 } else { cell.bg }, 48);
    out.push(b'm');
}

/// `0x00_00_00_nn`: reset (0) or ratatui's named colors 1..=16 (ANSI 0..=15); `0x01_..`: indexed; `0x02_..`: RGB.
fn color(out: &mut Vec<u8>, packed: u32, base: u8) {
    let _ = match packed >> 24 {
        0x00 if (1..=16).contains(&(packed & 0xFF)) => write!(out, ";{base};5;{}", (packed & 0xFF) - 1),
        0x01 => write!(out, ";{base};5;{}", packed & 0xFF),
        0x02 => write!(out, ";{base};2;{};{};{}", (packed >> 16) & 0xFF, (packed >> 8) & 0xFF, packed & 0xFF),
        _ => Ok(()),
    };
}

/// gpui's keystroke as herdr's semantic key; `None` for the platform chords that drive Shepherd itself.
pub fn key_event(key: &str, key_char: Option<&str>, mods: &gpui::Modifiers) -> Option<PaneInput> {
    if mods.platform {
        // Cmd+Backspace: delete to the line's start (Ctrl+U), Cmd+Left/Right: to its start or end (Ctrl+A, Ctrl+E),
        // as in the attach path
        let text = match key {
            "backspace" => "\x15",
            "left" => "\x01",
            "right" => "\x05",
            _ => return None,
        };
        return Some(PaneInput::TextCommit(text.into()));
    }
    // Option+Left/Right: a word back or forward (ESC b, ESC f), as Ghostty sends them
    if mods.alt && !mods.shift && !mods.control && matches!(key, "left" | "right") {
        return Some(PaneInput::TextCommit(if key == "left" { "\x1bb" } else { "\x1bf" }.into()));
    }
    let code = match key {
        "enter" => KeyCode::Enter,
        "backspace" => KeyCode::Backspace,
        "tab" if mods.shift => KeyCode::BackTab,
        "tab" => KeyCode::Tab,
        "escape" => KeyCode::Esc,
        "space" => KeyCode::Char(' '),
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "insert" => KeyCode::Insert,
        "delete" => KeyCode::Delete,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        f if f.len() > 1 && f.starts_with('f') && f[1..].parse::<u8>().is_ok() => KeyCode::F(f[1..].parse().ok()?),
        _ => {
            // Option chords macOS composed (Option+B → "∫") go as the text, as Ghostty does by default.
            let text = key_char.filter(|c| !c.is_empty());
            if mods.alt
                && !mods.control
                && let Some(text) = text.filter(|t| *t != key)
            {
                return Some(PaneInput::TextCommit(text.into()));
            }
            let mut chars = key.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => KeyCode::Char(c),
                _ => return text.map(|t| PaneInput::TextCommit(t.into())),
            }
        }
    };
    let modifiers = mods.shift as u8 | (mods.control as u8) << 1 | (mods.alt as u8) << 2;
    let generated_text = key_char.filter(|c| !c.is_empty() && !mods.control && !mods.alt).map(str::to_string);
    Some(PaneInput::Key {
        code,
        modifiers,
        kind: KeyKind::Press,
        repeat_count: 1,
        shifted_codepoint: None,
        generated_text,
        tracks_release: false,
        physical_key_id: None,
        windows_record: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(symbol: &str, fg: u32, modifier: u16) -> CellData {
        CellData { symbol: symbol.into(), fg, bg: 0, modifier, skip: false, hyperlink: None }
    }

    #[test]
    fn a_pane_is_cut_out_of_the_surface_and_replayed_into_an_emulator() {
        // 4×2 surface; the pane is its right half; a wide char takes two cells
        let cells = vec![
            cell("a", 0, 0), cell("b", 0, 0), cell("x", 0x02_ff_00_00, 1), cell("y", 0, 0),
            cell("c", 0, 0), cell("d", 0, 0), cell("你", 0, 0), cell(" ", 0, 0),
        ];
        let frame = FrameData { cells, width: 4, height: 2, cursor: None, hyperlinks: vec![], graphics: vec![] };
        let area = SurfaceRect { x: 2, y: 0, width: 2, height: 2 };
        let bytes = pane_ansi(&frame, &area, Some(&CursorState { x: 3, y: 0, visible: true, shape: 0 }), (0x02_ff_00_00, 0));
        let mut emu = crate::terminal::emulator::Emulator::new(2, 2);
        emu.feed(&bytes);
        assert_eq!(emu.row_text(0), "xy");
        assert_eq!(emu.row_text(1).trim_end(), "你");
        // red is the host's default foreground here: it goes back as default
        let x = emu.line(0)[0].clone();
        assert!(x.bold && x.fg == crate::terminal::emulator::CellColor::Foreground);
        assert_eq!(emu.cursor().map(|c| (c.row, c.col)), Some((0, 1)));
    }

    #[test]
    fn keys_go_semantic_and_composed_text_as_text() {
        let none = gpui::Modifiers::default();
        let ctrl = gpui::Modifiers { control: true, ..Default::default() };
        let alt = gpui::Modifiers { alt: true, ..Default::default() };
        assert!(matches!(key_event("c", Some("c"), &ctrl), Some(PaneInput::Key { code: KeyCode::Char('c'), modifiers: 2, generated_text: None, .. })));
        assert!(matches!(key_event("enter", None, &none), Some(PaneInput::Key { code: KeyCode::Enter, .. })));
        assert!(matches!(key_event("b", Some("∫"), &alt), Some(PaneInput::TextCommit(t)) if t == "∫"));
        // as Ghostty sends them, not as the kitty or legacy encoding of an arrow would
        assert!(matches!(key_event("left", None, &alt), Some(PaneInput::TextCommit(t)) if t == "\x1bb"));
        let cmd = gpui::Modifiers { platform: true, ..Default::default() };
        assert!(matches!(key_event("right", None, &cmd), Some(PaneInput::TextCommit(t)) if t == "\x05"));
        assert!(key_event("j", Some("j"), &cmd).is_none());
        assert!(key_event("q", Some("q"), &gpui::Modifiers { platform: true, ..Default::default() }).is_none());
    }
}

/// Live probe for the hybrid's side effects: `cargo test --lib client_shell::probe -- --ignored --nocapture` with
/// `ACTIVE=0|1 COLS ROWS SECS` (hello), `RESIZE=c,r` / `FOCUS=0|1` / `THEME=1` sent after a second. Prints what
/// herdr sent, per message kind, each second.
#[cfg(test)]
mod probe {
    use super::*;
    use std::io::Read;

    fn env(name: &str, default: &str) -> String {
        std::env::var(name).unwrap_or_else(|_| default.into())
    }

    #[test]
    #[ignore]
    fn probe() {
        let active = env("ACTIVE", "1") == "1";
        let (cols, rows): (u16, u16) = (env("COLS", "80").parse().unwrap(), env("ROWS", "24").parse().unwrap());
        let mut stream = socket::connect(&client_socket()).unwrap();
        let hello = serde_json::json!({
            "generation": 1, "cell_width_px": 0, "cell_height_px": 0, "surface_size": {"cols": cols, "rows": rows},
            "pixel_mouse": false, "direct_graphics": false, "endpoint_keybindings": false, "mouse_capture": false,
            "surface_active": active, "snapshot_codecs": ["shell.snapshot.v1"], "surface_codecs": ["shell.surface.v1"],
            "input_codecs": ["shell.input.semantic.v1"], "blob_codecs": ["shell.blob.v1"],
        });
        wire::write_message(&mut stream, &ClientMessage::EndpointControl { kind: "endpoint.hello.v1".into(), data: hello.to_string() }).unwrap();
        let mut writer = stream.try_clone().unwrap();
        let boot_id = Arc::new(Mutex::new(String::new()));
        let boot = boot_id.clone();
        let secs: u64 = env("SECS", "5").parse().unwrap();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(1));
            if let Ok(size) = std::env::var("RESIZE") {
                let (c, r) = size.split_once(',').unwrap();
                let surface_size = SurfaceSize { cols: c.parse().unwrap(), rows: r.parse().unwrap() };
                wire::write_message(&mut writer, &ClientMessage::ClientShellResize { cell_width_px: 0, cell_height_px: 0, surface_size, pixel_mouse: false }).unwrap();
                println!("sent resize {size}");
            }
            if let Ok(f) = std::env::var("FOCUS") {
                wire::write_message(&mut writer, &ClientMessage::ClientShellFocus { focused: f == "1" }).unwrap();
                println!("sent focus {f}");
            }
            // PULSE=1: switch the surface on, send the theme, switch it off again
            let surface = |w: &mut Stream, active: bool, id: &str| {
                let request = serde_json::json!({"id": id, "method": "client_shell.surface.set", "params": {"active": active}}).to_string();
                wire::write_message(w, &ClientMessage::ClientShellEndpointRequest { boot_id: boot.lock().unwrap().clone(), request }).unwrap();
            };
            let pulse = std::env::var("PULSE").is_ok();
            if pulse {
                surface(&mut writer, true, "on");
                println!("sent surface on");
            }
            if std::env::var("THEME").is_ok() || pulse {
                let red = Rgb { r: 0x40, g: 0x00, b: 0x00 };
                let update = HostThemeUpdate::DefaultColor { kind: DefaultColorKind::Background, color: red };
                wire::write_message(&mut writer, &ClientMessage::ClientShellHostTheme { update }).unwrap();
                println!("sent theme bg #400000");
            }
            if pulse {
                surface(&mut writer, false, "off");
                println!("sent surface off");
            }
        });
        const NAMES: [&str; 21] = [
            "Welcome", "Terminal", "Graphics", "ServerShutdown", "Notify", "Clipboard", "WindowTitle", "ReloadSoundConfig",
            "MouseCapture", "TerminalBell", "GraphicsFile", "GraphicsTransmissionRetired", "ClientShellSnapshot", "PaneSurface",
            "SemanticNotification", "ClientShellError", "DirectTerminalKeyboardProtocol", "ClientShellKeyboardReportAll",
            "ClientShellEndpointResponseChunk", "PaneSurfacePatch", "EndpointControl",
        ];
        stream.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
        let start = std::time::Instant::now();
        let mut second = 0;
        let mut tally: std::collections::BTreeMap<String, (u32, usize)> = Default::default();
        while start.elapsed() < Duration::from_secs(secs) {
            let mut len = [0u8; 4];
            if stream.read_exact(&mut len).is_ok() {
                let mut payload = vec![0; u32::from_le_bytes(len) as usize];
                stream.read_exact(&mut payload).unwrap();
                let mut name = NAMES.get(payload[0] as usize).copied().unwrap_or("?").to_string();
                if let Ok((ServerMessage::EndpointControl { kind, data }, _)) = bincode::serde::decode_from_slice::<ServerMessage, _>(&payload, bincode::config::standard()) {
                    if kind == "shell.snapshot.v1" {
                        let snapshot: serde_json::Value = serde_json::from_str(&data).unwrap();
                        *boot_id.lock().unwrap() = snapshot["boot_id"].as_str().unwrap_or_default().into();
                    }
                    name = format!("EndpointControl {kind}");
                }
                if let Ok((ServerMessage::PaneSurface(s), _)) = bincode::serde::decode_from_slice::<ServerMessage, _>(&payload, bincode::config::standard()) {
                    let panes: Vec<_> = s.panes.iter().map(|p| format!("{} {}x{}", p.pane_id, p.inner_rect.width, p.inner_rect.height)).collect();
                    name = format!("PaneSurface {}x{} {panes:?} popup={:?}", s.frame.width, s.frame.height, s.popup.as_ref().map(|p| &p.terminal_id));
                }
                if let Ok((ServerMessage::ClientShellEndpointResponseChunk { data, .. }, _)) = bincode::serde::decode_from_slice::<ServerMessage, _>(&payload, bincode::config::standard()) {
                    name = format!("response {}", String::from_utf8_lossy(&data));
                }
                let entry = tally.entry(name).or_default();
                entry.0 += 1;
                entry.1 += 4 + payload.len();
            }
            if start.elapsed().as_secs() > second {
                for (name, (n, bytes)) in std::mem::take(&mut tally) {
                    println!("t={second}s {n:>4} × {name}: {bytes} B");
                }
                second = start.elapsed().as_secs();
            }
        }
        for (name, (n, bytes)) in tally {
            println!("t={second}s {n:>4} × {name}: {bytes} B");
        }
    }

    /// Holds attach controllers like Shepherd's shown panes: `HOLD=w1:p1=70x30,w1:p2=50x30 SECS=60`.
    #[test]
    #[ignore]
    fn hold() {
        let mut held = Vec::new();
        for spec in env("HOLD", "").split(',').filter(|s| !s.is_empty()) {
            let (pane, size) = spec.split_once('=').unwrap();
            let (c, r) = size.split_once('x').unwrap();
            let (session, rx) = crate::herdr::Session::spawn(pane, c.parse().unwrap(), r.parse().unwrap(), (0, 0), crate::herdr::Mode::Control).unwrap();
            held.push((session, rx));
            println!("holding {pane} at {size}");
        }
        std::thread::sleep(Duration::from_secs(env("SECS", "60").parse().unwrap()));
    }
}

/// Live: the popup's frames reach its view and `ClientShellPopupInput` types into it. Open a popup first (an editor):
/// types `ihello` + Esc, prints the popup, then `:q!` + Enter.
#[cfg(test)]
mod live_popup {
    use super::*;

    #[test]
    #[ignore]
    fn popup_frames_and_input() {
        let (shell, rx) = ClientShell::connect(100, 35, (0, 0)).unwrap();
        let mut rx = futures::executor::block_on_stream(rx);
        let popup = loop {
            if let Some(PopupEvent::Popup(Some(p))) = rx.next() {
                break p;
            }
        };
        println!("popup {:?} {:?}x{:?} {}", popup.terminal_id, popup.width, popup.height, popup.title);
        let mut frames = shell.subscribe_popup(&popup.terminal_id);
        let none = gpui::Modifiers::default();
        let mut keys: Vec<PaneInput> = "ihello".chars().map(|c| key_event(&c.to_string(), Some(&c.to_string()), &none).unwrap()).collect();
        keys.push(key_event("escape", None, &none).unwrap());
        shell.popup_input(&popup.terminal_id, keys);
        std::thread::sleep(Duration::from_millis(800));
        let mut emu = crate::terminal::emulator::Emulator::new(10, 10);
        while let Ok(event) = frames.try_recv() {
            match event {
                SessionEvent::Frame(b) => drop(emu.feed(&b)),
                SessionEvent::Size(c, r) => {
                    println!("size {c}x{r}");
                    emu.resize(c, r)
                }
                _ => {}
            }
        }
        for r in 0..emu.rows().min(6) {
            println!("|{}", emu.row_text(r).trim_end());
        }
        let mut quit: Vec<PaneInput> = ":q!".chars().map(|c| key_event(&c.to_string(), Some(&c.to_string()), &none).unwrap()).collect();
        quit.push(key_event("enter", None, &none).unwrap());
        shell.popup_input(&popup.terminal_id, quit);
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Keystroke → changed screen on one attach, the key sent over the attach (`Input`) or over a client shell
/// (`ClientShellPaneInput`): `cargo test --lib client_shell::input_paths -- --ignored --nocapture`.
#[cfg(test)]
mod input_paths {
    use super::*;
    use crate::herdr::session::{Mode, Session};
    use futures::StreamExt;

    #[test]
    #[ignore]
    fn attach_echo_latency() {
        let mut stream = socket::connect(&client_socket()).unwrap();
        let hello = serde_json::json!({
            "generation": 1, "cell_width_px": 0, "cell_height_px": 0, "surface_size": {"cols": 80, "rows": 24},
            "pixel_mouse": false, "direct_graphics": false, "endpoint_keybindings": false, "mouse_capture": false,
            "surface_active": true, "snapshot_codecs": ["shell.snapshot.v1"], "surface_codecs": ["shell.surface.v1"],
            "input_codecs": ["shell.input.semantic.v1"], "blob_codecs": ["shell.blob.v1"],
        });
        wire::write_message(&mut stream, &ClientMessage::EndpointControl { kind: "endpoint.hello.v1".into(), data: hello.to_string() }).unwrap();
        let pane = loop {
            if let Ok(Some(ServerMessage::PaneSurface(s))) = wire::read_message(&mut stream) {
                break s.panes.iter().find(|p| p.focused).unwrap_or(&s.panes[0]).pane_id.clone();
            }
        };
        let mut reader = stream.try_clone().unwrap();
        std::thread::spawn(move || while wire::read_message(&mut reader).is_ok() {});
        let shell = Mutex::new(stream);
        let (session, mut frames) = Session::spawn(&pane, 80, 24, (0, 0), Mode::Control).unwrap();
        let mut emu = crate::terminal::emulator::Emulator::new(80, 24);
        // `RUN`: a command typed into the pane first, e.g. a program that echoes each key after a delay
        if let Ok(run) = std::env::var("RUN") {
            session.input(format!("{run}\r").as_bytes());
            std::thread::sleep(Duration::from_secs(1));
            drain(&mut frames, &mut emu);
        }
        // what Shepherd types through: the popup watch, once its surface names the pane
        let (_events, watch) = super::super::popup::watch(80, 24, (0, 0)).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        println!("pane {pane}");
        for _ in 0..2 {
            bench("attach Input", &mut frames, &mut emu, |c| session.input(c.as_bytes()));
            bench("popup::Surface::type_into", &mut frames, &mut emu, |c| assert!(watch.type_into(&pane, c.as_bytes())));
            bench("ClientShellPaneInput", &mut frames, &mut emu, |c| {
                let input = ClientMessage::ClientShellPaneInput { pane_id: pane.clone(), events: vec![PaneInput::TextCommit(c.into())] };
                wire::write_message(&mut *shell.lock().unwrap(), &input).unwrap();
            });
        }
        session.input(b"\x03");
    }

    fn drain(frames: &mut UnboundedReceiver<SessionEvent>, emu: &mut crate::terminal::emulator::Emulator) {
        while let Ok(event) = frames.try_recv() {
            if let SessionEvent::Frame(bytes) = event {
                emu.feed(&bytes);
            }
        }
    }

    fn bench(
        name: &str,
        frames: &mut UnboundedReceiver<SessionEvent>,
        emu: &mut crate::terminal::emulator::Emulator,
        send: impl Fn(&str),
    ) {
        let screen = |emu: &crate::terminal::emulator::Emulator| (0..emu.rows()).map(|r| emu.row_text(r)).collect::<Vec<_>>();
        let mut took = Vec::new();
        for i in 0..40 {
            std::thread::sleep(Duration::from_millis(50));
            drain(frames, emu);
            let before = screen(emu);
            let start = std::time::Instant::now();
            send(["q", "w", "e", "r"][i % 4]);
            while screen(emu) == before {
                match futures::executor::block_on(frames.next()) {
                    Some(SessionEvent::Frame(bytes)) => drop(emu.feed(&bytes)),
                    Some(_) => {}
                    None => return,
                }
            }
            took.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        send("\x15"); // Ctrl+U: clear the typed line
        std::thread::sleep(Duration::from_millis(100));
        took.sort_by(f64::total_cmp);
        println!("{name}: keystroke → changed screen, ms: p50 {:.2}, p90 {:.2}, max {:.2}", took[took.len() / 2], took[took.len() * 9 / 10], took[took.len() - 1]);
    }
}

/// `TAB=<tab id> cargo test --lib client_shell::focus_tab -- --ignored --nocapture`: how soon herdr answers the
/// shell's `tab.focus`, and whether it did (no answer waits the full second).
#[cfg(test)]
mod focus_tab {
    use super::*;

    #[test]
    #[ignore]
    fn answered() {
        let (shell, events) = ClientShell::connect(80, 24, (0, 0)).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        let start = std::time::Instant::now();
        shell.focus_tab(&std::env::var("TAB").unwrap());
        println!("tab.focus returned after {:?}", start.elapsed());
        // `OPEN`: a `herdr` command that opens a popup through the public API; it should come to this shell
        let Ok(open) = std::env::var("OPEN") else { return };
        std::process::Command::new("sh").args(["-c", &open]).status().unwrap();
        let popup = futures::executor::block_on_stream(events).take(20).find_map(|e| match e {
            PopupEvent::Popup(Some(p)) => Some(p.terminal_id),
            _ => None,
        });
        println!("popup here: {popup:?}");
    }
}
