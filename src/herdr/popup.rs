//! herdr's popup (a plugin's or a `[[keys.command]]` floating terminal) is no pane: only a client-rendered shell
//! hears of it, in the pane surfaces herdr sends it. One such connection per session, read only for the popup; the
//! popup's terminal itself is then controlled like a pane's ([`super::Session`] takes its terminal id).
//!
//! The same surfaces carry herdr's Kitty graphics, which attaches never get: the images on the panes of the shell's
//! tab ([`super::graphics`]), sent once herdr knows the cell size in pixels.
use super::session::client_socket;
use super::wire::{self, ClientMessage, HostThemeUpdate, ServerMessage, SurfaceSize};
pub use super::wire::{PopupSize, PopupSurface};
use futures::channel::mpsc::{UnboundedReceiver, unbounded};
use super::socket::{self, Stream};
use std::time::Duration;

/// What the client shell hears that Shepherd uses.
#[derive(Debug)]
pub enum Event {
    /// herdr's popup as it opens, changes and closes (`None`).
    Popup(Option<PopupSurface>),
    /// `[[keys.command]]` as herdr runs them (`command.invoke`): (command id, its key labels as in the config).
    Commands(Vec<(String, Vec<String>)>),
    /// The images on the panes of the shell's tab, when they change (see [`super::graphics`]).
    Images(super::graphics::PaneImages),
}

/// The client shell's surface, which herdr lays its focused tab out at while no other client does: kept at
/// Shepherd's pane grid, as herdr-gpui keeps its own.
pub struct Surface(Stream);

impl Surface {
    /// `cell_px`: a cell in device pixels, which herdr sizes images by.
    pub fn resize(&self, cols: u16, rows: u16, (cell_width_px, cell_height_px): (u32, u32)) {
        let surface_size = SurfaceSize { cols: cols.max(20), rows: rows.max(8) };
        let msg = ClientMessage::ClientShellResize { cell_width_px, cell_height_px, surface_size, pixel_mouse: false };
        let _ = wire::write_message(&mut &self.0, &msg); // a dead shell is noticed by its reader
    }


    /// Shepherd's colors for herdr's host theme. herdr keeps them with this shell and applies them whenever it is the
    /// client in front again (another client gone, or this one sending), so a TUI's theme does not outlast the TUI.
    pub fn theme(&self, updates: Vec<HostThemeUpdate>) {
        for update in updates {
            let _ = wire::write_message(&mut &self.0, &ClientMessage::ClientShellHostTheme { update });
        }
    }
}

/// The client shell's [`Event`]s, and its [`Surface`] (`cols`×`rows` of `cell_px` at first, (0, 0) while unknown);
/// the stream ends when herdr hangs up or refuses.
pub fn watch(cols: u16, rows: u16, cell_px: (u32, u32)) -> anyhow::Result<(UnboundedReceiver<Event>, Surface)> {
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
    let surface = Surface(stream.try_clone()?);
    let (tx, rx) = unbounded();
    std::thread::spawn(move || {
        let mut last = None;
        let images = tx.clone();
        let scenes = super::graphics::worker(move |i| images.unbounded_send(Event::Images(i)).is_ok());
        loop {
            let mut events = Vec::new();
            match wire::read_message(&mut stream) {
                Ok(Some(ServerMessage::PaneSurface(mut frame))) => {
                    let _ = scenes.send(std::mem::take(&mut frame.graphics));
                    let popup = frame.popup.as_ref().map(PopupSurface::from);
                    if popup != last {
                        last = popup;
                        events.push(Event::Popup(last.clone()));
                    }
                }
                Ok(Some(ServerMessage::EndpointControl { kind, data })) if kind == "shell.snapshot.v1" => {
                    events.push(Event::Commands(commands(&data)))
                }
                Ok(_) => continue,
                Err(_) => return,
            };
            for event in events {
                if tx.unbounded_send(event).is_err() {
                    return; // the watcher was dropped: hang up
                }
            }
        }
    });
    Ok((rx, surface))
}

pub(super) fn commands(snapshot: &str) -> Vec<(String, Vec<String>)> {
    #[derive(serde::Deserialize)]
    struct Command {
        command_id: String,
        binding_labels: Vec<String>,
    }
    #[derive(serde::Deserialize)]
    struct Snapshot {
        commands: Vec<Command>,
    }
    serde_json::from_str::<Snapshot>(snapshot).map_or(Vec::new(), |s| s.commands.into_iter().map(|c| (c.command_id, c.binding_labels)).collect())
}

/// The popup's outer size in cells inside `cols`×`rows`, as herdr fits it (`resolve_popup_geometry`): half the
/// area by default, at least 6×4, at most all of it.
pub fn size(width: Option<PopupSize>, height: Option<PopupSize>, cols: u16, rows: u16) -> Option<(u16, u16)> {
    let resolve = |size: Option<PopupSize>, avail: u16, min: u16| {
        let outer = match size {
            Some(PopupSize::Cells(n)) => n,
            Some(PopupSize::Percent(p)) => (avail as u32 * p as u32 / 100) as u16,
            None => (avail / 2).max(min),
        };
        outer.max(min).min(avail)
    };
    let (w, h) = (resolve(width, cols, 6), resolve(height, rows, 4));
    (w >= 6 && h >= 4).then_some((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_like_herdr() {
        assert_eq!(size(None, None, 100, 40), Some((50, 20)));
        assert_eq!(size(Some(PopupSize::Percent(80)), Some(PopupSize::Cells(10)), 100, 40), Some((80, 10)));
        assert_eq!(size(Some(PopupSize::Cells(300)), None, 100, 40), Some((100, 20)));
        assert_eq!(size(None, None, 5, 40), None);
    }

    /// Live: `cargo test popup::tests::live -- --ignored --nocapture`, then open a popup in herdr.
    #[test]
    fn resize_encodes_like_herdr() {
        let msg = ClientMessage::ClientShellResize { cell_width_px: 0, cell_height_px: 0, surface_size: SurfaceSize { cols: 120, rows: 40 }, pixel_mouse: false };
        assert_eq!(bincode::serde::encode_to_vec(&msg, bincode::config::standard()).unwrap(), [12, 0, 0, 120, 40, 0]);
    }

    #[test]
    #[ignore]
    fn live() {
        let (rx, _surface) = watch(120, 40, (0, 0)).unwrap();
        for popup in futures::executor::block_on_stream(rx).take(4) {
            println!("{popup:?}");
        }
    }

    /// Live: `PANE=<id> cargo test popup::tests::live_images -- --ignored --nocapture`, then show an image in that pane
    /// (`kitten icat`, `chafa -f kitty`). Holds the pane as Shepherd does (an attach, which gives herdr the pane's
    /// pixel size) and prints the images per pane as herdr's scenes change.
    #[test]
    #[ignore]
    fn live_images() {
        let pane = std::env::var("PANE").unwrap();
        let _held = super::super::Session::spawn(&pane, 100, 30, (16, 34), super::super::Mode::Control).unwrap();
        let (rx, _surface) = watch(120, 40, (16, 34)).unwrap();
        for event in futures::executor::block_on_stream(rx) {
            if let Event::Images(images) = event {
                println!("{images:?}");
            }
        }
    }
}
