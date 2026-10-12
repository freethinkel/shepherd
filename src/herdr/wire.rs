//! The slice of herdr's client socket protocol a terminal session needs, from herdr's src/protocol/wire.rs
//! @ v0.9.3 (Apache-2.0, see assets/NOTICE): length-prefixed bincode-2 (`config::standard()`) messages.
//!
//! Enums are matched by variant *index*, so every variant before the last one used is listed in herdr's order.
//! Variants Shepherd never sends or reads are unit placeholders: a placeholder decodes from just the index, so an
//! uninteresting server message still parses (its payload is left unread and ignored).
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

/// herdr's `PROTOCOL_VERSION`; the direct-terminal handshake requires an exact match.
pub const PROTOCOL_VERSION: u32 = 22;
/// herdr's `MAX_GRAPHICS_FRAME_SIZE`: the cap for server frames.
const MAX_FRAME_SIZE: usize = 32 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub enum ClientMessage {
    TerminalHello { version: u32, cols: u16, rows: u16, cell_width_px: u32, cell_height_px: u32, pixel_mouse: bool },
    Input { data: Vec<u8> },
    ClipboardImage,
    Resize { cols: u16, rows: u16, cell_width_px: u32, cell_height_px: u32, pixel_mouse: bool },
    Detach,
    AttachTerminal,
    AttachScroll {
        source: AttachScrollSource,
        direction: AttachScrollDirection,
        lines: u16,
        column: Option<u16>,
        row: Option<u16>,
        modifiers: u8,
    },
    ObserveTerminal { target: String },
    ControlTerminal { target: String, takeover: bool },
    GraphicsTransmissionResult,
    GraphicsTransmissionStarted,
    ClientShellHello,
    /// The client shell's whole pane grid in cells; herdr lays the shown tab's panes out in it.
    ClientShellResize { cell_width_px: u32, cell_height_px: u32, surface_size: SurfaceSize, pixel_mouse: bool },
    ClientShellPaneInput { pane_id: String, events: Vec<PaneInput> },
    /// Input for herdr's popup, which only client shells see.
    ClientShellPopupInput { terminal_id: String, events: Vec<PaneInput> },
    /// One API call (JSON `{id, method, params}`) on behalf of this client shell, e.g. `tab.focus` for its own view.
    ClientShellEndpointRequest { boot_id: String, request: String },
    AttachMouse {
        kind: MouseKind,
        position: MousePosition,
        geometry: Option<MouseGeometry>,
        modifiers: u8,
        lines: u16,
    },
    ClientShellHostTheme { update: HostThemeUpdate },
    ClientShellFocus { focused: bool },
    ClientShellMouseCapture,
    /// A named control message of herdr's client-shell ("endpoint") protocol, its data JSON.
    EndpointControl { kind: String, data: String },
}

#[derive(Debug, Clone, Copy, Serialize)]
#[allow(dead_code)]
pub enum AttachScrollDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub enum AttachScrollSource {
    Wheel,
    PageKey { input: Vec<u8> },
}

#[derive(Debug, Clone, Copy, Serialize)]
#[allow(dead_code)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[allow(dead_code)]
pub enum MouseKind {
    Down(MouseButton),
    Up(MouseButton),
    Drag(MouseButton),
    Moved,
    ScrollUp,
    ScrollDown,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[allow(dead_code)]
pub enum MousePosition {
    Cell { column: u16, row: u16 },
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct MouseGeometry {
    pub cols: u16,
    pub rows: u16,
    pub width_px: u32,
    pub height_px: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SurfaceSize {
    pub cols: u16,
    pub rows: u16,
}

/// herdr's `ClientPaneInputEvent`: keys stay semantic, so herdr encodes them for the pane app's keyboard protocol.
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub enum PaneInput {
    Key {
        code: KeyCode,
        /// crossterm's `KeyModifiers` bits: shift 1, control 2, alt 4, super 8.
        modifiers: u8,
        kind: KeyKind,
        repeat_count: u16,
        shifted_codepoint: Option<u32>,
        generated_text: Option<String>,
        tracks_release: bool,
        physical_key_id: Option<u32>,
        /// herdr's `WindowsKeyRecord`, always `None` here.
        windows_record: Option<()>,
    },
    /// Raw text for the PTY as it is.
    TextCommit(String),
    Mouse { kind: MouseKind, position: MousePosition, geometry: Option<MouseGeometry>, modifiers: u8, lines: u16 },
    /// Bracketed or not as the pane's app asked.
    Paste(String),
}

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub enum KeyCode {
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    Insert,
    Esc,
    Char(char),
    F(u8),
    Null,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[allow(dead_code)]
pub enum KeyKind {
    Press,
    Repeat,
    Release,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum DefaultColorKind {
    Foreground,
    Background,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum HostAppearance {
    Dark,
    Light,
}

/// The colors herdr answers the panes' OSC 10/11/4 queries with.
#[derive(Debug, Clone, Serialize)]
pub enum HostThemeUpdate {
    DefaultColor { kind: DefaultColorKind, color: Rgb },
    PaletteColors(Vec<(u8, Rgb)>),
    Appearance(HostAppearance),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderEncoding {
    SemanticFrame,
    TerminalAnsi,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub enum ServerMessage {
    Welcome { version: u32, encoding: RenderEncoding, error: Option<String> },
    Terminal(TerminalFrame),
    Graphics,
    ServerShutdown { reason: Option<String> },
    Notify,
    Clipboard,
    WindowTitle,
    ReloadSoundConfig,
    /// True while the controlled pane's app asks for mouse reporting.
    MouseCapture { enabled: bool, sgr_pixels: bool },
    TerminalBell,
    GraphicsFile,
    GraphicsTransmissionRetired,
    ClientShellSnapshot,
    /// The client shell's view of the shown tab, read only for its popup.
    PaneSurface(PaneSurfaceFrame),
    SemanticNotification,
    ClientShellError { message: String },
    DirectTerminalKeyboardProtocol,
    ClientShellKeyboardReportAll,
    ClientShellEndpointResponseChunk { boot_id: String, request_id: String, final_chunk: bool, data: Vec<u8> },
    PaneSurfacePatch,
    EndpointControl { kind: String, data: String },
}

/// herdr's `PaneSurfaceFrame` up to its popup; the fields after it are left unread.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PaneSurfaceFrame {
    pub boot_id: String,
    pub projection_revision: u64,
    pub surface_revision: u64,
    pub frame: FrameData,
    pub panes: Vec<PaneSurfacePane>,
    pub splits: Vec<PaneSurfaceSplit>,
    pub popup: Option<PopupFrame>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct FrameData {
    pub cells: Vec<CellData>,
    pub width: u16,
    pub height: u16,
    pub cursor: Option<CursorState>,
    pub hyperlinks: Vec<String>,
    pub graphics: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CellData {
    pub symbol: String,
    pub fg: u32,
    pub bg: u32,
    pub modifier: u16,
    pub skip: bool,
    pub hyperlink: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CursorState {
    pub x: u16,
    pub y: u16,
    pub visible: bool,
    pub shape: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SurfaceRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct PaneSurfacePane {
    pub pane_id: String,
    pub content_revision: u64,
    pub rect: SurfaceRect,
    pub inner_rect: SurfaceRect,
    pub scrollbar_rect: Option<SurfaceRect>,
    pub scroll: Option<(u64, u64, u64)>,
    pub focused: bool,
    pub mouse_reporting: bool,
    pub sgr_pixel_mouse: bool,
    pub alternate_screen_active: bool,
    pub pixel_width: u32,
    pub pixel_height: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PaneSurfaceSplit {
    pub direction: SplitDirection,
    pub pos: u16,
    pub area: SurfaceRect,
    pub hit_rect: SurfaceRect,
    pub path: Vec<bool>,
}

/// herdr's `ClientShellPopupSurface`: the popup's terminal, its size as asked, and its cells.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PopupFrame {
    pub terminal_id: String,
    pub title: String,
    pub width: Option<PopupSize>,
    pub height: Option<PopupSize>,
    pub frame: FrameData,
    pub mouse_reporting: bool,
}

/// What Shepherd keeps of herdr's popup to lay it out.
#[derive(Debug, Clone, PartialEq)]
pub struct PopupSurface {
    pub terminal_id: String,
    pub title: String,
    pub width: Option<PopupSize>,
    pub height: Option<PopupSize>,
}

impl From<&PopupFrame> for PopupSurface {
    fn from(p: &PopupFrame) -> Self {
        PopupSurface { terminal_id: p.terminal_id.clone(), title: p.title.clone(), width: p.width, height: p.height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub enum PopupSize {
    Cells(u16),
    Percent(u8),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TerminalFrame {
    pub seq: u64,
    pub width: u16,
    pub height: u16,
    pub full: bool,
    pub bytes: Vec<u8>,
}

pub fn write_message(w: &mut impl Write, msg: &ClientMessage) -> io::Result<()> {
    let payload = bincode::serde::encode_to_vec(msg, bincode::config::standard()).map_err(io::Error::other)?;
    w.write_all(&(payload.len() as u32).to_le_bytes())?;
    w.write_all(&payload)?;
    w.flush()
}

/// The next message; `Ok(None)` for one this module does not model (a later variant), which callers skip.
pub fn read_message(r: &mut impl Read) -> io::Result<Option<ServerMessage>> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len)?;
    let len = u32::from_le_bytes(len) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("herdr frame of {len} bytes")));
    }
    let mut payload = vec![0; len];
    r.read_exact(&mut payload)?;
    Ok(bincode::serde::decode_from_slice(&payload, bincode::config::standard()).ok().map(|(msg, _)| msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte layouts checked by hand against bincode 2 `standard()`: varint enum index, then the fields.
    #[test]
    fn encodes_like_herdr() {
        let bytes = |m: &ClientMessage| bincode::serde::encode_to_vec(m, bincode::config::standard()).unwrap();
        assert_eq!(bytes(&ClientMessage::Input { data: b"ab".to_vec() }), [1, 2, b'a', b'b']);
        assert_eq!(bytes(&ClientMessage::Detach), [4]);
        assert_eq!(
            bytes(&ClientMessage::ControlTerminal { target: "w:p".into(), takeover: true }),
            [8, 3, b'w', b':', b'p', 1]
        );
        assert_eq!(
            bytes(&ClientMessage::AttachMouse {
                kind: MouseKind::Drag(MouseButton::Left),
                position: MousePosition::Cell { column: 4, row: 2 },
                geometry: None,
                modifiers: 1,
                lines: 1,
            }),
            [16, 2, 0, 0, 4, 2, 0, 1, 1]
        );
    }

    #[test]
    fn decodes_mouse_capture_and_skips_unmodelled_messages() {
        let mut framed = Vec::new();
        for payload in [vec![8u8, 1, 0], vec![20u8, 0]] {
            framed.extend((payload.len() as u32).to_le_bytes());
            framed.extend(payload);
        }
        let mut r = framed.as_slice();
        assert!(matches!(read_message(&mut r).unwrap(), Some(ServerMessage::MouseCapture { enabled: true, .. })));
        assert!(read_message(&mut r).unwrap().is_none());
    }
}
