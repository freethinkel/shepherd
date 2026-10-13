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
    /// The client shell's surface: herdr lays its tab out at `surface_size` (cols, rows).
    ClientShellResize { cell_width_px: u32, cell_height_px: u32, surface_size: (u16, u16), pixel_mouse: bool },
    ClientShellPaneInput,
    ClientShellPopupInput,
    ClientShellEndpointRequest,
    AttachMouse {
        kind: MouseKind,
        position: MousePosition,
        geometry: Option<MouseGeometry>,
        modifiers: u8,
        lines: u16,
    },
    ClientShellHostTheme { update: HostThemeUpdate },
    ClientShellFocus,
    ClientShellMouseCapture,
    /// A named control message of herdr's client-shell ("endpoint") protocol, its data JSON.
    EndpointControl { kind: String, data: String },
}

/// herdr's `ClientHostThemeUpdate`: a color or the appearance of the terminal a client shell runs in.
#[derive(Debug, Clone, Serialize)]
pub enum HostThemeUpdate {
    DefaultColor { kind: DefaultColorKind, color: Rgb },
    PaletteColors(Vec<(u8, Rgb)>),
    Appearance(HostAppearance),
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum DefaultColorKind {
    Foreground,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum HostAppearance {
    Dark,
    Light,
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
    ClientShellEndpointResponseChunk,
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
    pub popup: Option<PopupSurface>,
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

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CellData {
    pub symbol: String,
    pub fg: u32,
    pub bg: u32,
    pub modifier: u16,
    pub skip: bool,
    pub hyperlink: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CursorState {
    pub x: u16,
    pub y: u16,
    pub visible: bool,
    pub shape: u8,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SurfaceRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Deserialize)]
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

/// herdr's `ClientShellPopupSurface` up to its size; its rendered frame is left unread (the popup's terminal is
/// controlled directly instead).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PopupSurface {
    pub terminal_id: String,
    pub title: String,
    pub width: Option<PopupSize>,
    pub height: Option<PopupSize>,
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
        let bg = Rgb { r: 250, g: 250, b: 250 };
        let update = HostThemeUpdate::DefaultColor { kind: DefaultColorKind::Background, color: bg };
        assert_eq!(bytes(&ClientMessage::ClientShellHostTheme { update }), [17, 0, 1, 250, 250, 250]);
        let update = HostThemeUpdate::Appearance(HostAppearance::Light);
        assert_eq!(bytes(&ClientMessage::ClientShellHostTheme { update }), [17, 2, 1]);
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
