//! herdr-core (`hc`) as Shepherd's backend, beside herdr. The core runs terminals and draws nothing: it has no
//! workspaces, tabs or panes, so the layout is Shepherd's ([`tree`]) and [`Hc`] is a [`crate::backend::Mux`] on it. Its
//! terminals stream their program's own bytes, which the pane's emulator reads as a terminal would.
mod api;
mod app;
mod conn;
pub mod facts;
pub(crate) mod tree;

use crate::backend::{SessionEvent, Terminal};
use conn::{Core, Item, Sink};
use futures::channel::mpsc::{UnboundedReceiver, unbounded};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

/// Lines of the core's history a pane is attached with.
const HISTORY: u32 = 10_000;

/// Whether the layout is shared with the Herdr app: the setting, read on every load so that it applies at once.
static SYNC: AtomicBool = AtomicBool::new(true);

pub fn set_sync(on: bool) {
    SYNC.store(on, Ordering::Relaxed);
}

fn syncing() -> bool {
    SYNC.load(Ordering::Relaxed)
}

/// The connection to the core, opened when first asked. Dropped, the core runs on.
pub struct Hc {
    core: Arc<Mutex<Option<Arc<Core>>>>,
    /// One layout change at a time: a snapshot's pruning never sees a terminal started and not yet placed.
    layout: tokio::sync::Mutex<()>,
    /// Counts saved edits: they are Shepherd's own, so no core event says to read the layout again.
    edits: watch::Sender<u64>,
    /// The app's document has its copy, or had one.
    backed_up: AtomicBool,
}

impl Default for Hc {
    fn default() -> Self {
        Self { core: Arc::default(), layout: tokio::sync::Mutex::const_new(()), edits: watch::channel(0).0, backed_up: AtomicBool::new(false) }
    }
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

/// What a program may ask of its terminal, which the core answers.
fn display(cell_px: (u32, u32)) -> Value {
    let (fg, bg) = crate::backend::colors().unzip();
    let cell = (cell_px.0 > 0 && cell_px.1 > 0).then_some([cell_px.0, cell_px.1]);
    json!({"fg": fg, "bg": bg, "cell": cell})
}

fn connect(held: &Mutex<Option<Arc<Core>>>) -> anyhow::Result<Arc<Core>> {
    let mut held = held.lock().unwrap();
    if let Some(core) = held.as_ref().filter(|c| !c.lost()) {
        return Ok(core.clone());
    }
    let binary = binary().ok_or_else(|| anyhow::anyhow!("herdr-core isn't installed: hc is not on PATH"))?;
    let core = Core::connect(&binary)?;
    *held = Some(core.clone());
    Ok(core)
}

/// A pane's stream on a terminal; dropped, the pane detaches and the program runs on.
struct Attached {
    core: Arc<Core>,
    channel: u32,
}

impl Terminal for Attached {
    fn input(&self, bytes: &[u8]) {
        self.core.send(self.channel, conn::DATA, bytes);
    }

    fn resize(&self, cols: u16, rows: u16, cell_px: (u32, u32)) {
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

impl Hc {
    async fn connected(&self) -> anyhow::Result<Arc<Core>> {
        let held = self.core.clone();
        tokio::task::spawn_blocking(move || connect(&held)).await?
    }

    /// Attaches to `terminal`: its screen and history first, then what its program writes.
    fn attach(&self, terminal: &str, cols: u16, rows: u16, cell_px: (u32, u32), observe: bool) -> anyhow::Result<(Attached, UnboundedReceiver<SessionEvent>)> {
        let core = connect(&self.core)?;
        let (tx, rx) = unbounded();
        let _ = tx.unbounded_send(SessionEvent::Restart); // the core sends its screen and history anew
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
}
