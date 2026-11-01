//! herdr-core (`hc`) as Shepherd's backend, beside herdr. The core runs terminals and draws nothing: it has no
//! workspaces, tabs or panes, so the layout is Shepherd's ([`tree`]), and [`api`] answers herdr's API from it. Its
//! terminals stream their program's own bytes, which the pane's emulator reads as a terminal would.
pub(crate) mod api;
mod conn;
pub(crate) mod tree;
pub use api::{call, subscribe};

use crate::herdr::SessionEvent;
use conn::{Core, Item, Sink};
use futures::channel::mpsc::{UnboundedReceiver, unbounded};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

/// Lines of the core's history a pane is attached with.
const HISTORY: u32 = 10_000;

static CORE: Mutex<Option<Arc<Core>>> = Mutex::new(None);
/// The terminal's colors, which the core answers a program's queries with.
static COLORS: Mutex<(Option<String>, Option<String>)> = Mutex::new((None, None));

/// Off: the connection goes; the core runs on.
pub fn disconnect() {
    *CORE.lock().unwrap() = None;
}

/// `hc`: `SHEPHERD_HC`, else on PATH, else where its installer puts it.
fn binary() -> Option<String> {
    let found = || crate::herdr::find_program("hc").map(|p| p.to_string_lossy().into_owned());
    std::env::var("SHEPHERD_HC").ok().or_else(found)
}

/// Whether herdr-core can be offered. Not in a fixture (`SHEPHERD_MOCK`), whose window shows only what it holds.
pub fn installed() -> bool {
    crate::mock::dir().is_none() && binary().is_some()
}

/// The foreground and background, as `#rrggbb`.
pub fn set_colors(foreground: String, background: String) {
    *COLORS.lock().unwrap() = (Some(foreground), Some(background));
}

fn display(cell_px: (u32, u32)) -> Value {
    let (fg, bg) = COLORS.lock().unwrap().clone();
    let cell = (cell_px.0 > 0 && cell_px.1 > 0).then_some([cell_px.0, cell_px.1]);
    json!({"fg": fg, "bg": bg, "cell": cell})
}

fn connect() -> anyhow::Result<Arc<Core>> {
    let mut held = CORE.lock().unwrap();
    if let Some(core) = held.as_ref().filter(|c| !c.lost()) {
        return Ok(core.clone());
    }
    let binary = binary().ok_or_else(|| anyhow::anyhow!("herdr-core isn't installed: hc is not on PATH"))?;
    let core = Core::connect(&binary)?;
    *held = Some(core.clone());
    Ok(core)
}

async fn connected() -> anyhow::Result<Arc<Core>> {
    tokio::task::spawn_blocking(connect).await?
}

/// A pane's stream on a terminal; dropped, the pane detaches and the program runs on.
pub struct Attached {
    core: Arc<Core>,
    channel: u32,
}

impl Attached {
    pub fn input(&self, bytes: &[u8]) {
        self.core.send(self.channel, conn::DATA, bytes);
    }

    pub fn resize(&self, cols: u16, rows: u16, cell_px: (u32, u32)) {
        let size: Vec<u8> = cols.to_le_bytes().into_iter().chain(rows.to_le_bytes()).collect();
        self.core.send(self.channel, conn::RESIZE, &size);
        self.core.send(self.channel, conn::FACTS, display(cell_px).to_string().as_bytes());
    }
}

impl Drop for Attached {
    fn drop(&mut self) {
        self.core.send(self.channel, conn::CLOSE, &[]);
    }
}

/// Attaches to `terminal`: its screen and history first, then what its program writes. Blocks: run it off the main
/// thread.
pub fn attach(terminal: &str, cols: u16, rows: u16, cell_px: (u32, u32), observe: bool) -> anyhow::Result<(Attached, UnboundedReceiver<SessionEvent>)> {
    let core = connect()?;
    let (tx, rx) = unbounded();
    let sink: Sink = Box::new(move |item| {
        let _ = tx.unbounded_send(match item {
            Item::Data(bytes) => SessionEvent::Frame(bytes),
            Item::Closed(why) => SessionEvent::Ended(why["reason"].as_str().unwrap_or("the terminal closed").to_string()),
            Item::Lost => SessionEvent::Ended("lost herdr-core".into()),
        });
    });
    let params = json!({"terminal": terminal, "cols": cols, "rows": rows, "history": HISTORY, "observe": observe, "display": display(cell_px)});
    let answer = core.ask("terminal.attach", params, Some(sink)).blocking_recv().map_err(|_| anyhow::anyhow!("lost herdr-core"))??;
    let channel = answer["channel"].as_u64().ok_or_else(|| anyhow::anyhow!("terminal.attach opened no stream"))? as u32;
    Ok((Attached { core, channel }, rx))
}
