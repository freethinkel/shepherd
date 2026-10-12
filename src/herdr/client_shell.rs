//! PoC (`SHEPHERD_CLIENT_SHELL=1`): Shepherd as one herdr client shell instead of one attach per shown pane.
//!
//! herdr lays the shown tab out in the shell's surface and sends its cells (`PaneSurface`, then `PaneSurfacePatch`
//! against the last one). Each pane's `inner_rect` is cut out of that grid and replayed as ANSI into the pane's own
//! emulator ([`super::SessionEvent`]), so the pane view, its selection, search and links work as they do over an
//! attach. Input goes back as herdr's semantic pane input (`ClientShellPaneInput`), the size as one
//! `ClientShellResize` for the whole grid, the shown tab as a `tab.focus` on the shell's own view.
use super::popup::{Event as PopupEvent, PopupSurface};
use super::session::{SessionEvent, client_socket};
use super::wire::{
    self, CellData, ClientMessage, CursorState, FrameData, KeyCode, KeyKind, PaneInput,
    PaneSurfaceFrame, PaneSurfacePane, ServerMessage, SurfaceRect, SurfaceSize,
};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use std::collections::HashMap;
use std::io::Write as _;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

pub use super::wire::{DefaultColorKind, HostAppearance, HostColor, HostThemeUpdate};

/// The PoC runs only when asked for.
pub fn enabled() -> bool {
    std::env::var_os("SHEPHERD_CLIENT_SHELL").is_some_and(|v| v == "1")
}

/// One pane of the shown tab as herdr laid it out, in surface cells.
#[derive(Debug, Clone, PartialEq)]
pub struct PaneRect {
    pub pane_id: String,
    /// (x, y, width, height), the pane with its border.
    pub rect: (u16, u16, u16, u16),
    /// Its terminal's cells.
    pub inner: (u16, u16, u16, u16),
    pub focused: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layout {
    pub cols: u16,
    pub rows: u16,
    pub panes: Vec<PaneRect>,
}

/// What the UI hears: the layout as it changes, herdr's popup and its `[[keys.command]]`.
#[derive(Debug)]
pub enum Event {
    Layout(Layout),
    Popup(PopupEvent),
}

#[derive(Default)]
struct State {
    boot_id: String,
    surface: Option<PaneSurfaceFrame>,
    /// The panes' views, each fed its cut of the surface.
    panes: HashMap<String, UnboundedSender<SessionEvent>>,
    /// The size each pane was last sent, to send `Size` only on a change.
    sizes: HashMap<String, (u16, u16)>,
    next_request: u64,
    /// The host's default fg/bg as herdr packs them: herdr resolves "default" to these RGBs in the surface, and
    /// the pane gets them back as default, so its view paints them with the theme (translucent background too).
    defaults: (u32, u32),
}

pub struct ClientShell {
    stream: Mutex<UnixStream>,
    state: Arc<Mutex<State>>,
}

impl ClientShell {
    /// Handshakes on herdr's client socket and starts reading. `cols`×`rows` is the pane grid.
    pub fn connect(cols: u16, rows: u16) -> anyhow::Result<(Arc<ClientShell>, UnboundedReceiver<Event>)> {
        let mut stream = UnixStream::connect(client_socket())?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        let hello = serde_json::json!({
            "generation": 1,
            "cell_width_px": 0,
            "cell_height_px": 0,
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

    /// The frames of `pane_id` from now on, the current one first.
    pub fn subscribe(&self, pane_id: &str) -> UnboundedReceiver<SessionEvent> {
        let (tx, rx) = unbounded();
        let mut state = self.state.lock().unwrap();
        state.sizes.remove(pane_id);
        if let Some(surface) = state.surface.take() {
            if let Some(pane) = surface.panes.iter().find(|p| p.pane_id == pane_id) {
                let defaults = state.defaults;
                send_pane(&mut state.sizes, &tx, &surface.frame, pane, surface.frame.cursor.as_ref(), defaults);
            }
            state.surface = Some(surface);
        }
        state.panes.insert(pane_id.into(), tx);
        rx
    }

    pub fn unsubscribe(&self, pane_id: &str) {
        let mut state = self.state.lock().unwrap();
        state.panes.remove(pane_id);
        state.sizes.remove(pane_id);
    }

    /// The whole pane grid, in cells.
    pub fn resize(&self, cols: u16, rows: u16) {
        self.send(&ClientMessage::ClientShellResize {
            cell_width_px: 0,
            cell_height_px: 0,
            surface_size: SurfaceSize { cols: cols.max(20), rows: rows.max(8) },
            pixel_mouse: false,
        });
    }

    pub fn input(&self, pane_id: &str, events: Vec<PaneInput>) {
        self.send(&ClientMessage::ClientShellPaneInput { pane_id: pane_id.into(), events });
    }

    pub fn theme(&self, updates: Vec<HostThemeUpdate>) {
        let packed = |c: &HostColor| 0x02_00_00_00 | (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32;
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

    pub fn focused(&self, focused: bool) {
        self.send(&ClientMessage::ClientShellFocus { focused });
    }

    /// Shows `tab_id` in this shell only: herdr keeps a view per client shell, other clients stay where they are.
    pub fn focus_tab(&self, tab_id: &str) {
        self.request("tab.focus", serde_json::json!({"tab_id": tab_id}));
    }

    /// One API call over the shell's command lane. herdr takes one at a time (`endpoint_busy` otherwise).
    // ponytail: fire and forget; the answer arrives as a ClientShellEndpointResponseChunk and is dropped
    fn request(&self, method: &str, params: serde_json::Value) {
        let (boot_id, id) = {
            let mut state = self.state.lock().unwrap();
            state.next_request += 1;
            (state.boot_id.clone(), format!("shepherd-{}", state.next_request))
        };
        let request = serde_json::json!({"id": id, "method": method, "params": params}).to_string();
        self.send(&ClientMessage::ClientShellEndpointRequest { boot_id, request });
    }
}

impl Drop for ClientShell {
    fn drop(&mut self) {
        let stream = self.stream.get_mut().unwrap();
        let _ = wire::write_message(stream, &ClientMessage::Detach);
        let _ = stream.shutdown(std::net::Shutdown::Both);
    }
}

fn read(mut stream: UnixStream, state: Arc<Mutex<State>>, tx: UnboundedSender<Event>) {
    let mut layout = Layout::default();
    let mut popup: Option<PopupSurface> = None;
    loop {
        let message = match wire::read_message(&mut stream) {
            Ok(Some(message)) => message,
            Ok(None) => continue,
            Err(_) => break,
        };
        let mut state = state.lock().unwrap();
        let state = &mut *state;
        match message {
            ServerMessage::PaneSurface(mut surface) => {
                state.boot_id = surface.boot_id.clone();
                for pane in &surface.panes {
                    if let Some(tx) = state.panes.get(&pane.pane_id) {
                        send_pane(&mut state.sizes, tx, &surface.frame, pane, surface.frame.cursor.as_ref(), state.defaults);
                    }
                }
                let next = layout_of(&surface);
                if next != layout {
                    layout = next;
                    let _ = tx.unbounded_send(Event::Layout(layout.clone()));
                }
                if surface.popup != popup {
                    popup = surface.popup.take();
                    let _ = tx.unbounded_send(Event::Popup(PopupEvent::Popup(popup.clone())));
                }
                state.surface = Some(surface);
            }
            ServerMessage::PaneSurfacePatch(patch) => {
                let Some(surface) = state.surface.as_mut() else { continue };
                if patch.base_surface_revision != surface.surface_revision || patch.boot_id != surface.boot_id {
                    continue; // ponytail: a full surface follows a lost baseline
                }
                let width = surface.frame.width as usize;
                for row in patch.rows {
                    let start = row.y as usize * width + row.x as usize;
                    if let Some(cells) = surface.frame.cells.get_mut(start..start + row.cells.len()) {
                        cells.clone_from_slice(&row.cells);
                    }
                }
                for updated in &patch.panes {
                    if let Some(pane) = surface.panes.iter_mut().find(|p| p.pane_id == updated.pane_id) {
                        *pane = updated.clone();
                    }
                }
                surface.frame.cursor = patch.cursor;
                surface.surface_revision = patch.surface_revision;
                // the panes whose content changed, and the focused one for the cursor
                for pane in surface.panes.iter().filter(|p| p.focused || patch.panes.iter().any(|u| u.pane_id == p.pane_id)) {
                    if let Some(tx) = state.panes.get(&pane.pane_id) {
                        send_pane(&mut state.sizes, tx, &surface.frame, pane, surface.frame.cursor.as_ref(), state.defaults);
                    }
                }
            }
            ServerMessage::EndpointControl { kind, data } if kind == "shell.snapshot.v1" => {
                let _ = tx.unbounded_send(Event::Popup(PopupEvent::Commands(super::popup::commands(&data))));
            }
            ServerMessage::ServerShutdown { .. } => break,
            _ => {}
        }
    }
    for pane in state.lock().unwrap().panes.values() {
        let _ = pane.unbounded_send(SessionEvent::Ended("herdr closed the client shell".into()));
    }
}

fn layout_of(surface: &PaneSurfaceFrame) -> Layout {
    let r = |r: &SurfaceRect| (r.x, r.y, r.width, r.height);
    Layout {
        cols: surface.frame.width,
        rows: surface.frame.height,
        panes: surface
            .panes
            .iter()
            .map(|p| PaneRect { pane_id: p.pane_id.clone(), rect: r(&p.rect), inner: r(&p.inner_rect), focused: p.focused })
            .collect(),
    }
}

fn send_pane(
    sizes: &mut HashMap<String, (u16, u16)>,
    tx: &UnboundedSender<SessionEvent>,
    frame: &FrameData,
    pane: &PaneSurfacePane,
    cursor: Option<&CursorState>,
    defaults: (u32, u32),
) {
    let size = (pane.inner_rect.width, pane.inner_rect.height);
    if sizes.insert(pane.pane_id.clone(), size) != Some(size) {
        let _ = tx.unbounded_send(SessionEvent::Size(size.0, size.1));
    }
    let cursor = cursor.filter(|_| pane.focused);
    let _ = tx.unbounded_send(SessionEvent::MouseCapture(pane.mouse_reporting));
    let _ = tx.unbounded_send(SessionEvent::Frame(pane_ansi(frame, &pane.inner_rect, cursor, defaults)));
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
        // Cmd+Backspace: delete to the line's start (Ctrl+U), as in the attach path
        return (key == "backspace").then(|| PaneInput::TextCommit("\x15".into()));
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
            if mods.alt && !mods.control {
                if let Some(text) = text.filter(|t| *t != key) {
                    return Some(PaneInput::TextCommit(text.into()));
                }
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
        let x = emu.line(0)[0];
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
        assert!(key_event("q", Some("q"), &gpui::Modifiers { platform: true, ..Default::default() }).is_none());
    }

    /// Live, against a running herdr: `cargo test client_shell::tests::live -- --ignored --nocapture`.
    /// Prints the layout, then the round trip of a typed char over the client shell and over an attach.
    #[test]
    #[ignore]
    fn live() {
        let (shell, rx) = ClientShell::connect(120, 40).unwrap();
        let mut rx = futures::executor::block_on_stream(rx);
        let layout = loop {
            if let Some(Event::Layout(l)) = rx.next() {
                break l;
            }
        };
        println!("{layout:#?}");
        let pane = std::env::var("PANE").unwrap_or_else(|_| layout.panes.iter().find(|p| p.focused).unwrap_or(&layout.panes[0]).pane_id.clone());
        for _ in 0..2 {
            bench("client shell", shell.subscribe(&pane), |c| shell.input(&pane, vec![PaneInput::TextCommit(c.into())]));
            shell.unsubscribe(&pane);
            let (session, frames) = super::super::Session::spawn(&pane, 57, 38, super::super::Mode::Control).unwrap();
            bench("attach", frames, |c| session.input(c.as_bytes()));
        }
    }

    fn bench(name: &str, mut frames: UnboundedReceiver<SessionEvent>, send: impl Fn(&str)) {
        use futures::StreamExt;
        let mut emu = crate::terminal::emulator::Emulator::new(120, 40);
        let apply = |emu: &mut crate::terminal::emulator::Emulator, event| match event {
            SessionEvent::Frame(bytes) => drop(emu.feed(&bytes)),
            SessionEvent::Size(c, r) => emu.resize(c, r),
            _ => {}
        };
        let screen = |emu: &crate::terminal::emulator::Emulator| (0..emu.rows()).map(|r| emu.row_text(r)).collect::<Vec<_>>();
        let mut took = Vec::new();
        for i in 0..40 {
            std::thread::sleep(Duration::from_millis(30));
            while let Ok(event) = frames.try_recv() {
                apply(&mut emu, event);
            }
            let before = screen(&emu);
            let start = std::time::Instant::now();
            send(["q", "w", "e", "r"][i % 4]);
            while screen(&emu) == before {
                let Some(event) = futures::executor::block_on(frames.next()) else { return };
                apply(&mut emu, event);
            }
            took.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        send("\x15"); // Ctrl+U: clear the typed line
        took.sort_by(f64::total_cmp);
        println!("{name}: keystroke → changed screen, ms: p50 {:.2}, p90 {:.2}, max {:.2}", took[took.len() / 2], took[took.len() * 9 / 10], took[took.len() - 1]);
    }
}

#[cfg(test)]
mod live_keys {
    use super::*;

    /// Live: types `echo hi`, Enter, then `sleep 9`, Enter, Ctrl+C as semantic keys, and prints the pane.
    #[test]
    #[ignore]
    fn semantic_keys_reach_the_shell() {
        let (shell, _rx) = ClientShell::connect(120, 40).unwrap();
        let pane = std::env::var("PANE").unwrap();
        let mut frames = shell.subscribe(&pane);
        let none = gpui::Modifiers::default();
        let ctrl = gpui::Modifiers { control: true, ..Default::default() };
        let mut keys: Vec<PaneInput> = "echo hi".chars().map(|c| key_event(&c.to_string(), Some(&c.to_string()), &none).unwrap()).collect();
        keys.push(key_event("enter", None, &none).unwrap());
        keys.extend("sleep 9".chars().map(|c| key_event(&c.to_string(), Some(&c.to_string()), &none).unwrap()));
        keys.push(key_event("enter", None, &none).unwrap());
        shell.input(&pane, keys);
        std::thread::sleep(Duration::from_millis(500));
        shell.input(&pane, vec![key_event("c", Some("c"), &ctrl).unwrap()]);
        std::thread::sleep(Duration::from_millis(500));
        let mut emu = crate::terminal::emulator::Emulator::new(120, 40);
        while let Ok(event) = frames.try_recv() {
            match event {
                SessionEvent::Frame(bytes) => drop(emu.feed(&bytes)),
                SessionEvent::Size(c, r) => emu.resize(c, r),
                _ => {}
            }
        }
        for r in 0..12 {
            println!("|{}", emu.row_text(r).trim_end());
        }
    }
}

#[cfg(test)]
mod live_tabs {
    use super::*;

    /// Live: the shell's view follows `tab.focus`; herdr's own focus (the API's) stays.
    #[test]
    #[ignore]
    fn the_shell_shows_the_tab_it_focuses() {
        let (shell, rx) = ClientShell::connect(120, 40).unwrap();
        let mut rx = futures::executor::block_on_stream(rx);
        let mut next_layout = || loop {
            if let Some(Event::Layout(l)) = rx.next() {
                break l.panes.iter().map(|p| (p.pane_id.clone(), p.rect, p.inner)).collect::<Vec<_>>();
            }
        };
        println!("first: {:?}", next_layout());
        shell.focus_tab(&std::env::var("TAB").unwrap());
        println!("after tab.focus: {:?}", next_layout());
    }
}

#[cfg(test)]
mod live_cells {
    use super::*;

    /// Live: the packed fg/bg herdr sends for a few cells of each pane.
    #[test]
    #[ignore]
    fn dump_cell_colors() {
        let mut stream = UnixStream::connect(client_socket()).unwrap();
        let hello = r#"{"generation":1,"cell_width_px":0,"cell_height_px":0,"surface_size":{"cols":120,"rows":40},"pixel_mouse":false,"direct_graphics":false,"endpoint_keybindings":false,"mouse_capture":false,"surface_active":true,"snapshot_codecs":["shell.snapshot.v1"],"surface_codecs":["shell.surface.v1"],"input_codecs":["shell.input.semantic.v1"],"blob_codecs":["shell.blob.v1"]}"#;
        wire::write_message(&mut stream, &ClientMessage::EndpointControl { kind: "endpoint.hello.v1".into(), data: hello.into() }).unwrap();
        loop {
            if let Ok(Some(ServerMessage::PaneSurface(s))) = wire::read_message(&mut stream) {
                for p in &s.panes {
                    let r = &p.inner_rect;
                    let at = |x: u16, y: u16| &s.frame.cells[(r.y + y) as usize * s.frame.width as usize + (r.x + x) as usize];
                    let cells: Vec<_> = [(0, 0), (1, 1), (r.width - 1, 3)].iter().map(|&(x, y)| { let c = at(x, y); (c.symbol.clone(), format!("{:08x}", c.fg), format!("{:08x}", c.bg)) }).collect();
                    println!("{} {:?} rect {:?} inner {:?} {cells:?}", p.pane_id, p.alternate_screen_active, (p.rect.x, p.rect.y, p.rect.width, p.rect.height), (r.x, r.y, r.width, r.height));
                }
                let border = &s.frame.cells[0];
                println!("surface cell 0,0 {:?} fg {:08x} bg {:08x}", border.symbol, border.fg, border.bg);
                return;
            }
        }
    }
}

#[cfg(test)]
mod live_attach_colors {
    /// Live: what an attach (observe) gives for the same cells, to compare with `dump_cell_colors`.
    #[test]
    #[ignore]
    fn attach_cell_colors() {
        let pane = std::env::var("PANE").unwrap();
        let (_s, rx) = crate::herdr::Session::spawn(&pane, 56, 38, crate::herdr::Mode::Observe).unwrap();
        let mut emu = crate::terminal::emulator::Emulator::new(56, 38);
        std::thread::sleep(std::time::Duration::from_millis(500));
        let mut rx = rx;
        while let Ok(crate::herdr::SessionEvent::Frame(b)) = rx.try_recv() {
            emu.feed(&b);
        }
        println!("{:?} {:?}", emu.line(1)[1].fg, emu.line(1)[1].bg);
    }
}

#[cfg(test)]
mod live_resize_scroll {
    use super::*;

    /// Live: `ClientShellResize` re-lays the tab out; a wheel event scrolls the pane's history.
    #[test]
    #[ignore]
    fn resize_and_wheel() {
        let (shell, rx) = ClientShell::connect(120, 40).unwrap();
        let mut rx = futures::executor::block_on_stream(rx);
        let mut next_layout = || loop {
            if let Some(Event::Layout(l)) = rx.next() {
                break l;
            }
        };
        let first = next_layout();
        shell.resize(100, 30);
        let resized = next_layout();
        println!("{}x{} → {}x{}, panes {:?}", first.cols, first.rows, resized.cols, resized.rows, resized.panes.iter().map(|p| p.inner).collect::<Vec<_>>());
        let pane = std::env::var("PANE").unwrap();
        let frames = shell.subscribe(&pane);
        std::thread::sleep(Duration::from_millis(200));
        let screen = |frames: &mut UnboundedReceiver<SessionEvent>, emu: &mut crate::terminal::emulator::Emulator| {
            while let Ok(e) = frames.try_recv() {
                match e {
                    SessionEvent::Frame(b) => drop(emu.feed(&b)),
                    SessionEvent::Size(c, r) => emu.resize(c, r),
                    _ => {}
                }
            }
            emu.row_text(0)
        };
        let (mut frames, mut emu) = (frames, crate::terminal::emulator::Emulator::new(10, 10));
        std::thread::sleep(Duration::from_millis(300));
        let before = screen(&mut frames, &mut emu);
        let wheel = PaneInput::Mouse { kind: wire::MouseKind::ScrollUp, position: wire::MousePosition::Cell { column: 2, row: 2 }, geometry: None, modifiers: 0, lines: 10 };
        shell.input(&pane, vec![wheel]);
        std::thread::sleep(Duration::from_millis(200));
        let after = screen(&mut frames, &mut emu);
        println!("row 0 before: {before:?}\nrow 0 after wheel up: {after:?}");
        let down = PaneInput::Mouse { kind: wire::MouseKind::ScrollDown, position: wire::MousePosition::Cell { column: 2, row: 2 }, geometry: None, modifiers: 0, lines: 10 };
        shell.input(&pane, vec![down]);
        std::thread::sleep(Duration::from_millis(200));
        println!("row 0 after wheel down: {:?}", screen(&mut frames, &mut emu));
    }
}
