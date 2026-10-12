//! herdr's popup (a plugin's or a `[[keys.command]]` floating terminal) is no pane: only a client-rendered shell
//! hears of it, in the pane surfaces herdr sends it. One such connection per session, read only for the popup; the
//! popup's terminal itself is then controlled like a pane's ([`super::Session`] takes its terminal id).
use super::session::client_socket;
use super::wire::{self, ClientMessage, ServerMessage};
pub use super::wire::{PopupSize, PopupSurface};
use futures::channel::mpsc::{UnboundedReceiver, unbounded};
use std::os::unix::net::UnixStream;
use std::time::Duration;

/// What the client shell hears that Shepherd uses.
#[derive(Debug)]
pub enum Event {
    /// herdr's popup as it opens, changes and closes (`None`).
    Popup(Option<PopupSurface>),
    /// `[[keys.command]]` as herdr runs them (`command.invoke`): (command id, its key labels as in the config).
    Commands(Vec<(String, Vec<String>)>),
}

/// The client shell's [`Event`]s; the stream ends when herdr hangs up or refuses.
/// `cols`×`rows` is the shell's surface: herdr sizes the tab it shows to it while no other client does, so it
/// should be Shepherd's pane grid.
pub fn watch(cols: u16, rows: u16) -> anyhow::Result<UnboundedReceiver<Event>> {
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
    let (tx, rx) = unbounded();
    std::thread::spawn(move || {
        let mut last = None;
        loop {
            let event = match wire::read_message(&mut stream) {
                Ok(Some(ServerMessage::PaneSurface(frame))) if frame.popup != last => {
                    last = frame.popup;
                    Event::Popup(last.clone())
                }
                Ok(Some(ServerMessage::EndpointControl { kind, data })) if kind == "shell.snapshot.v1" => {
                    Event::Commands(commands(&data))
                }
                Ok(_) => continue,
                Err(_) => return,
            };
            if tx.unbounded_send(event).is_err() {
                return; // the watcher was dropped: hang up
            }
        }
    });
    Ok(rx)
}

fn commands(snapshot: &str) -> Vec<(String, Vec<String>)> {
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
    #[ignore]
    fn live() {
        let rx = watch(120, 40).unwrap();
        for popup in futures::executor::block_on_stream(rx).take(4) {
            println!("{popup:?}");
        }
    }
}
