//! The slice of herdr's client socket protocol a terminal session needs, from herdr's src/protocol/wire.rs
//! @ v0.9.4 (Apache-2.0, see assets/NOTICE): length-prefixed bincode-2 (`config::standard()`) messages.
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

/// herdr's `PaneSurfaceFrame`: read for its popup and its Kitty graphics.
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
    pub graphics: GraphicsScene,
}

/// herdr's `SurfaceGraphicsScene`: every placement shown on the surface, and only the image bytes this connection
/// has not been sent yet (herdr sends each image once; a later scene names it by its key).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GraphicsScene {
    pub assets: Vec<GraphicsAsset>,
    pub placements: Vec<GraphicsPlacement>,
    /// Images to keep although nothing shows them now (their pane is in another tab).
    pub retained_assets: Vec<AssetKey>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphicsAsset {
    pub key: AssetKey,
    #[serde(deserialize_with = "bytes")]
    pub data: Vec<u8>,
}

/// herdr writes image bytes as one byte string (length, then the bytes: `Vec<u8>`'s layout), read at once here
/// rather than byte by byte.
fn bytes<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    struct Bytes;
    impl serde::de::Visitor<'_> for Bytes {
        type Value = Vec<u8>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("image bytes")
        }
        fn visit_bytes<E>(self, v: &[u8]) -> Result<Vec<u8>, E> {
            Ok(v.to_vec())
        }
        fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Vec<u8>, E> {
            Ok(v)
        }
    }
    d.deserialize_bytes(Bytes)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct AssetKey {
    pub source: GraphicsSource,
    pub image_width: u32,
    pub image_height: u32,
    pub format: GraphicsFormat,
    pub data_len: u64,
    pub data_fingerprint: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub enum GraphicsSource {
    Terminal { target: GraphicsTarget, image_id: u32 },
    PaneLayer { pane_id: String, layer_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub enum GraphicsTarget {
    Pane { pane_id: String },
    Popup { terminal_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum GraphicsFormat {
    Rgb,
    Rgba,
    Png,
}

/// One image on a pane, already clipped to it: cells from the pane's top-left, the part of the image shown there
/// (`source_*`, image pixels), and a pixel offset into its first cell.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GraphicsPlacement {
    pub asset: AssetKey,
    pub logical_placement_id: u32,
    pub x: u16,
    pub y: u16,
    pub cols: u32,
    pub rows: u32,
    pub source_x: u32,
    pub source_y: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub x_offset: u32,
    pub y_offset: u32,
    pub z: i32,
    pub scrollback_offset: u32,
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
    pub sgr_pixel_mouse: bool,
    pub pixel_width: u32,
    pub pixel_height: u32,
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
    match bincode::serde::decode_from_slice(&payload, bincode::config::standard()) {
        Ok((msg, _)) => Ok(Some(msg)),
        Err(e) => {
            // a modelled message that does not decode: herdr's layout moved under the same protocol version
            static TOLD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if payload.first().is_some_and(|&i| i <= LAST_MODELLED) && !TOLD.swap(true, std::sync::atomic::Ordering::Relaxed) {
                eprintln!("shepherd: herdr message {} does not decode, so it is skipped (another herdr version?): {e}", payload[0]);
            }
            Ok(None)
        }
    }
}

/// The index of [`ServerMessage`]'s last variant.
const LAST_MODELLED: u8 = 20;

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

    /// herdr's scene as bincode lays it out: enum variants as varint indices, the image bytes as length + bytes.
    #[test]
    fn decodes_a_graphics_scene() {
        let target = (0u32, "w:p");
        let key = ((0u32, target, 7u32), 3u32, 1u32, 2u32, 4u64, 9u64);
        let scene = (vec![(key, vec![1u8, 2, 3, 4])], Vec::<u8>::new(), Vec::<u8>::new());
        let bytes = bincode::serde::encode_to_vec(scene, bincode::config::standard()).unwrap();
        let (scene, _): (GraphicsScene, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
        let asset = &scene.assets[0];
        assert_eq!(asset.data, [1, 2, 3, 4]);
        assert_eq!((asset.key.image_width, asset.key.image_height, asset.key.format), (3, 1, GraphicsFormat::Png));
        let GraphicsSource::Terminal { target: GraphicsTarget::Pane { pane_id }, image_id: 7 } = &asset.key.source else { panic!() };
        assert_eq!(pane_id, "w:p");
    }

    #[test]
    fn decodes_mouse_capture_and_skips_unmodelled_messages() {
        let mut framed = Vec::new();
        for payload in [vec![8u8, 1, 0], vec![21u8, 0]] {
            framed.extend((payload.len() as u32).to_le_bytes());
            framed.extend(payload);
        }
        let mut r = framed.as_slice();
        assert!(matches!(read_message(&mut r).unwrap(), Some(ServerMessage::MouseCapture { enabled: true, .. })));
        assert!(read_message(&mut r).unwrap().is_none());
    }
}
