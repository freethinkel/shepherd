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
    ClientShellResize,
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
