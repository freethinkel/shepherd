//! [`Rex`] as a backend: a session is a workspace, a window a tab and a block a pane. Rex's ids name one thing, the
//! window's commands take only a pane or a tab: `session/block` and `session/window`.
use super::TERMINAL;
use super::Endpoint;
use super::conn::Control;
use crate::agents::{Report, plain_title};
use crate::backend::{Attached, Change, Float, Kind, Made, Mode, Mux, Reply};
use crate::commands::Command;
use crate::model::{AREA, AgentStatus, Frac, Layout, LayoutPane, LayoutSplit, Minimized, Pane, Rect, Snapshot, Tab, Workspace, Worktree};
use crate::worktree::Checkouts;
use anyhow::{anyhow, bail};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, watch};

const SHELL: &str = "com.superlogical.terminal.shell";

/// The connection to the server, opened when first asked. Dropped, the server runs on.
#[derive(Default)]
pub struct Rex {
    /// A host's ways to reach it, tried in turn; none for the server on this machine.
    servers: Vec<Endpoint>,
    control: Arc<Mutex<Option<Arc<Control>>>>,
    shared: Arc<Shared>,
}

impl Drop for Rex {
    fn drop(&mut self) {
        if let Some(core) = self.control.lock().unwrap().take() {
            core.close();
        }
    }
}

/// What the connection's reader and the backend share.
struct Shared {
    state: Mutex<State>,
    /// Counts what the server said that changes a snapshot.
    news: watch::Sender<u64>,
    checkouts: Checkouts,
}

impl Default for Shared {
    fn default() -> Self {
        Self { state: Mutex::default(), news: watch::channel(0).0, checkouts: Checkouts::default() }
    }
}

impl Shared {
    fn say(&self) {
        self.news.send_modify(|n| *n += 1);
    }
}

/// What the server's views leave out.
#[derive(Default)]
struct State {
    /// What each block's terminal runs and reports. Asked once, then kept by the block's events: a view has no room for
    /// it, and asking on each snapshot would be four calls per pane.
    blocks: HashMap<String, Block>,
    /// The session the window shows: Rex keeps the active window of each, not one for the server.
    focused: Option<String>,
}

/// Nerd Fonts' git branch before a worktree's `repo/name`, for Rex's own list of sessions. Shepherd nests a worktree
/// under its repo with its own mark, so it shows neither.
const WORKTREE_MARK: &str = "\u{f126} ";

/// `label` as Rex keeps it for a workspace.
fn kept_label(label: String, tree: Option<&Worktree>) -> String {
    match tree.filter(|t| t.is_linked_worktree).and_then(crate::worktree::repo_name) {
        Some(repo) => format!("{WORKTREE_MARK}{repo}/{label}"),
        None => label,
    }
}

/// A label Rex keeps as Shepherd shows it. A repo's folder has no `/`, so it ends at the first.
fn shown_label(label: &str) -> &str {
    let Some(marked) = label.strip_prefix(WORKTREE_MARK) else { return label };
    marked.split_once('/').map_or(marked, |(_, name)| name)
}

#[derive(Debug, Clone, Default)]
struct Block {
    title: String,
    cwd: Option<String>,
    report: Report,
    /// What it runs has changed: the next snapshot asks.
    stale: bool,
}

fn connect(held: &Mutex<Option<Arc<Control>>>, shared: &Arc<Shared>, servers: &[Endpoint]) -> anyhow::Result<Arc<Control>> {
    let mut held = held.lock().unwrap();
    if let Some(core) = held.as_ref().filter(|c| !c.lost()) {
        return Ok(core.clone());
    }
    let here = servers.is_empty();
    let servers = if here { Endpoint::local()? } else { servers.to_vec() };
    let open = || {
        let shared = shared.clone();
        Control::open(&servers, move |event| {
            if heard(event, &mut shared.state.lock().unwrap()) {
                shared.say();
            }
        })
    };
    let core = open().or_else(|e| {
        // a host's server is started on the host, not from here
        if !here || !servers.iter().all(Endpoint::is_local) || !crate::herdr::no_server(&e) {
            return Err(e);
        }
        super::start_server()?;
        open()
    })?;
    core.blocking_call("session.attach_all", json!({"event_scope": "all"}))?;
    shared.state.lock().unwrap().blocks.clear(); // the events since the last connection are lost
    *held = Some(core.clone());
    Ok(core)
}

impl Rex {
    /// The server at `servers`, the ways to reach a host in the order to try them.
    pub fn at(servers: Vec<Endpoint>) -> Self {
        Self { servers, control: Arc::default(), shared: Arc::default() }
    }

    async fn connected(&self) -> anyhow::Result<Arc<Control>> {
        let (held, shared, servers) = (self.control.clone(), self.shared.clone(), self.servers.clone());
        tokio::task::spawn_blocking(move || connect(&held, &shared, &servers)).await?
    }

    /// A new session on one shell in `cwd`.
    async fn open(&self, core: &Control, cwd: Option<&str>) -> anyhow::Result<Made> {
        let made = core.call("session.create", json!({"initial_windows": [{"layout": shell(cwd)}]})).await?;
        let session = text_of(&made["session_id"]);
        if session.is_empty() {
            bail!("session.create gave no session");
        }
        Ok(Made {
            workspace_id: Some(session.to_string()),
            tab_id: Some(format!("{session}/{}", text_of(&made["initial_windows"][0]["window_id"]))),
            pane_id: Some(format!("{session}/{}", text_of(&made["initial_windows"][0]["block_ids"][0]))),
        })
    }
}

impl Mux for Rex {
    fn kind(&self) -> Kind {
        Kind::Rex
    }

    fn snapshot(&self) -> Reply<'_, Snapshot> {
        Box::pin(async move {
            let core = self.connected().await?;
            let core = core.as_ref();
            let listed = core.call("session.list", json!({})).await?;
            let mut views = Vec::new();
            for session in listed["sessions"].as_array().map(Vec::as_slice).unwrap_or_default() {
                let id = text_of(&session["session_id"]);
                // gone since the list: the next snapshot won't have it
                let Ok(view) = view(core, id).await else { continue };
                let blocks = blocks(&view);
                for block in &blocks {
                    learn(core, &self.shared, id, block).await;
                }
                let first = blocks.first().and_then(|b| self.shared.state.lock().unwrap().blocks.get(b)?.cwd.clone());
                self.shared.checkouts.find(id, first).await;
                views.push(view);
            }
            let state = self.shared.state.lock().unwrap();
            Ok(snapshot(&views, &state, &self.shared.checkouts))
        })
    }

    /// The server's events that change a snapshot. Ends with the connection, or when the receiver goes.
    fn changes(&self, _agents: Vec<String>) -> Reply<'_, mpsc::UnboundedReceiver<Change>> {
        Box::pin(async move {
            let core = self.connected().await?;
            let (tx, rx) = mpsc::unbounded_channel();
            let mut news = self.shared.news.subscribe();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        changed = news.changed() => if changed.is_err() { break },
                        _ = tx.closed() => break,
                    }
                    if core.lost() || tx.send(Change::Any).is_err() {
                        break;
                    }
                }
            });
            Ok(rx)
        })
    }

    fn run(&self, command: Command) -> Reply<'_, Made> {
        Box::pin(async move {
            let core = self.connected().await?;
            let core = core.as_ref();
            Ok(match command {
                Command::NewWorkspace { cwd } => self.open(core, cwd.as_deref()).await?,
                Command::NewTab { workspace_id: session } => {
                    let cwd = session_cwd(core, &session).await;
                    let made = core.call("session.new_window", json!({"session_id": session, "layout": shell(cwd.as_deref()), "focus": false})).await?;
                    Made {
                        tab_id: Some(format!("{session}/{}", text_of(&made["window_id"]))),
                        pane_id: Some(format!("{session}/{}", text_of(&made["block_ids"][0]))),
                        ..Made::default()
                    }
                }
                Command::Split { pane_id, down, cwd } => {
                    let (session, block) = ids(&pane_id)?;
                    let cwd = match cwd {
                        Some(cwd) => Some(cwd),
                        None => cwd_of(core, &pane_id).await,
                    };
                    let direction = if down { "vertical" } else { "horizontal" };
                    let args = json!({"session_id": session, "anchor_id": block, "direction": direction, "side": "after", "layout": shell(cwd.as_deref()), "focus": true});
                    let made = core.call("session.new_split", args).await?;
                    Made { pane_id: Some(format!("{session}/{}", text_of(&made["block_ids"][0]))), ..Made::default() }
                }
                Command::CloseWorkspace { workspace_id } => {
                    core.call("session.destroy", json!({"session_id": workspace_id})).await?;
                    Made::default()
                }
                Command::CloseTab { tab_id } => {
                    let (session, window) = ids(&tab_id)?;
                    core.call("session.close_window", json!({"session_id": session, "window_id": window})).await?;
                    Made::default()
                }
                Command::ClosePane { pane_id } => {
                    let (session, block) = ids(&pane_id)?;
                    // The split's last pane, closed under floating ones, would leave the tab nothing to dock them
                    // into, and Rex has no way to start a split again there: the front floating pane takes its place.
                    // ponytail: a program in it that exits on its own still leaves them floating; Rex would have to
                    // place a block in an empty split for that.
                    let view = view(core, session).await?;
                    let windows = view["windows"].as_array().map(Vec::as_slice).unwrap_or_default();
                    let window = windows.iter().find(|w| frames(tree(w)).0.iter().any(|(b, _)| b == block));
                    if let Some(window) = window.filter(|w| frames(tree(w)).0.len() == 1) {
                        let floating: Vec<_> = layers(window).iter().filter(|l| l["kind"] == "floating").collect();
                        let front = floating.iter().rev().find(|l| l["state"] != "minimized").or(floating.last());
                        if let Some((other, _)) = front.and_then(|l| frames(&l["layout"]).0.into_iter().next()) {
                            core.call("session.swap_blocks", json!({"session_id": session, "block_id": block, "other_block_id": other})).await?;
                        }
                    }
                    core.call("block.close", json!({"session_id": session, "block_id": block})).await?;
                    Made::default()
                }
                Command::RenameWorkspace { workspace_id, label } => {
                    let label = kept_label(label, self.shared.checkouts.get(&workspace_id).as_ref());
                    core.call("session.set_label", json!({"session_id": workspace_id, "label": label})).await?;
                    Made::default()
                }
                Command::RenameTab { tab_id, label } => {
                    let (session, window) = ids(&tab_id)?;
                    core.call("session.set_window_label", json!({"session_id": session, "window_id": window, "label": label})).await?;
                    Made::default()
                }
                Command::MoveTab { tab_id, insert_index } => {
                    let (session, window) = ids(&tab_id)?;
                    let view = view(core, session).await?;
                    let windows = view["windows"].as_array().map(Vec::as_slice).unwrap_or_default();
                    // `insert_index` as herdr takes it: a place in the list before the move, 0 first, the tab count last
                    let before = windows.get(insert_index).map(|w| text_of(&w["window_id"]));
                    if before != Some(window) {
                        core.call("session.move_window", json!({"session_id": session, "window_id": window, "before_window_id": before})).await?;
                    }
                    Made::default()
                }
                Command::MoveWorkspaces { workspace_ids, before } => {
                    for session in workspace_ids {
                        core.call("session.move", json!({"session_id": session, "before_session_id": before})).await?;
                    }
                    Made::default()
                }
                Command::FocusTab { tab_id } => {
                    let (session, window) = ids(&tab_id)?;
                    self.shared.state.lock().unwrap().focused = Some(session.to_string());
                    core.call("session.focus_window", json!({"session_id": session, "window_id": window})).await?;
                    self.shared.say();
                    Made::default()
                }
                Command::Focus { pane_id } => {
                    let (session, block) = ids(&pane_id)?;
                    self.shared.state.lock().unwrap().focused = Some(session.to_string());
                    core.call("session.focus_block", json!({"session_id": session, "block_id": block})).await?;
                    self.shared.say();
                    Made::default()
                }
                Command::FocusPane { pane_id, direction } => {
                    core.call("session.focus_direction", json!({"session_id": ids(&pane_id)?.0, "direction": direction.name()})).await?;
                    Made::default()
                }
                Command::Zoom { pane_id } => {
                    let (session, block) = ids(&pane_id)?;
                    core.call("session.zoom_block", json!({"session_id": session, "block_id": block})).await?;
                    Made::default()
                }
                Command::SwapPanes { source, target } => {
                    let ((session, a), (other, b)) = (ids(&source)?, ids(&target)?);
                    if session != other {
                        bail!("Rex swaps panes only within a session");
                    }
                    core.call("session.swap_blocks", json!({"session_id": session, "block_id": a, "other_block_id": b})).await?;
                    Made::default()
                }
                Command::MovePane { pane_id, tab_id, down, target } => move_to_tab(core, &pane_id, &tab_id, down, target.as_deref()).await?,
                Command::MovePaneToNewTab { pane_id, workspace_id } => move_to_new_tab(core, &pane_id, workspace_id.as_deref()).await?,
                Command::SetSplitRatio { tab_id, path, ratio } => {
                    let (session, window) = ids(&tab_id)?;
                    let view = view(core, session).await?;
                    let found = window_of(&view, window).and_then(|w| border(tree(w), &path));
                    let (direction, before, after) = found.ok_or_else(|| anyhow!("no split there"))?;
                    core.call("session.resize_pane", json!({"session_id": session, "direction": direction, "before_block_id": before, "after_block_id": after, "ratio": ratio})).await?;
                    Made::default()
                }
            })
        })
    }

    fn float(&self, pane: String, change: Float) -> Reply<'_, ()> {
        Box::pin(async move {
            let core = self.connected().await?;
            let (session, block) = ids(&pane)?;
            let view = view(&core, session).await?;
            let windows = view["windows"].as_array().map(Vec::as_slice).unwrap_or_default();
            let holds = |l: &&Value| l["blocks"].as_array().is_some_and(|b| b.iter().any(|b| b["block_id"] == block));
            let (window, layer) = windows
                .iter()
                .find_map(|w| layers(w).iter().find(holds).map(|l| (w, l)))
                .ok_or_else(|| anyhow!("no pane {pane}"))?;
            let layer_id = text_of(&layer["layer_id"]);
            let floats = layer["kind"] == "floating";
            let at = |method: &'static str, args: Value| {
                let mut args = args;
                args["session_id"] = json!(session);
                core.call(method, args)
            };
            match change {
                // the split's last pane stays: a tab of layers alone has nothing to dock them back into
                Float::Out if !floats && frames(tree(window)).0.len() < 2 => bail!("the split's last pane can't float"),
                Float::Out if !floats => at("session.float_block", json!({"block_id": block, "bounds": OUT})).await?,
                Float::Dock if floats => {
                    let tiled = frames(tree(window)).0;
                    let focused = text_of(&window["focused_block_id"]);
                    let anchor = tiled.iter().find(|(b, _)| b == focused).or(tiled.first()).map(|(b, _)| b.clone());
                    // Rex places a block only beside another
                    let anchor = anchor.ok_or_else(|| anyhow!("the tab has no split to dock into"))?;
                    at("session.move_block", moved(session, block, &anchor, false)).await?
                }
                Float::Raise if floats => at("session.raise_layer", json!({"layer_id": layer_id})).await?,
                Float::Place(frame) if floats => at("session.resize_layer", json!({"layer_id": layer_id, "bounds": bounds(frame)})).await?,
                Float::Minimize if floats => at("session.set_layer_state", json!({"layer_id": layer_id, "state": "minimized"})).await?,
                Float::Restore if floats => at("session.set_layer_state", json!({"layer_id": layer_id, "state": "normal", "focus": true})).await?,
                _ => return Ok(()),
            };
            Ok(())
        })
    }

    fn cwd(&self, pane: String) -> Reply<'_, Option<String>> {
        Box::pin(async move { Ok(cwd_of(self.connected().await?.as_ref(), &pane).await) })
    }

    fn read(&self, pane: String, lines: usize, _ansi: bool) -> Reply<'_, String> {
        Box::pin(async move {
            let read = on_block(self.connected().await?.as_ref(), &pane, "format", json!({"format": "text", "trim": true})).await?;
            Ok(tail(text_of(&read["content"]), lines))
        })
    }

    fn program(&self, pane: String) -> Reply<'_, Option<String>> {
        Box::pin(async move {
            let process = on_block(self.connected().await?.as_ref(), &pane, "process", json!({})).await?;
            let fg = &process["foreground"];
            Ok(if fg["pid"] == *shell_pid(&process) { None } else { program_name(fg) })
        })
    }

    fn send(&self, pane: String, text: Option<String>) -> Reply<'_, ()> {
        Box::pin(async move {
            let core = self.connected().await?;
            if let Some(text) = text {
                on_block(&core, &pane, "write", json!({"data": crate::backend::typed(text)})).await?;
            }
            on_block(&core, &pane, "write", json!({"data": "\r"})).await.map(drop)
        })
    }

    fn attach(&self, pane: &str, cols: u16, rows: u16, cell_px: (u32, u32), mode: Mode) -> anyhow::Result<Attached> {
        // the stream goes to the address the backend's connection reached, not to each in turn again
        let server = connect(&self.control, &self.shared, &self.servers)?.server.clone();
        let (attached, events) = super::attach(&server, pane, cols, rows, cell_px, mode == Mode::Observe)?;
        Ok((Box::new(attached), events))
    }

    fn remember(&self, workspace: String, tree: Worktree) -> Reply<'_, ()> {
        self.shared.checkouts.set(workspace, tree);
        Box::pin(async { Ok(()) })
    }
}

/// `session/block` or `session/window` taken apart.
pub fn ids(id: &str) -> anyhow::Result<(&str, &str)> {
    id.split_once('/').ok_or_else(|| anyhow!("{id} is not one of Rex's"))
}

async fn on_block(core: &Control, pane: &str, method: &str, args: Value) -> anyhow::Result<Value> {
    let (session, block) = ids(pane)?;
    core.call(&format!("{TERMINAL}.{method}"), json!({"session_id": session, "block_id": block, "args": args})).await
}

/// A new shell in `cwd`, as a layout takes it.
fn shell(cwd: Option<&str>) -> Value {
    json!({"block": {"flavor": SHELL, "options": {"cwd": cwd.unwrap_or_default()}}})
}

/// Where `pane`'s program is now: a new pane beside it starts there.
async fn cwd_of(core: &Control, pane: &str) -> Option<String> {
    on_block(core, pane, "process", json!({})).await.ok()?["foreground"]["cwd"].as_str().map(str::to_string)
}

async fn view(core: &Control, session: &str) -> anyhow::Result<Value> {
    core.call("session.view", json!({"session_id": session})).await
}

fn window_of<'a>(view: &'a Value, window: &str) -> Option<&'a Value> {
    view["windows"].as_array()?.iter().find(|w| w["window_id"] == window)
}

/// Where a pane floated out of the split sits: centered, at most of the tab.
const OUT: [f64; 4] = [0.15, 0.1, 0.7, 0.75];

/// `frame` as Rex's bounds, kept inside the window, which Rex insists on.
fn bounds(frame: Frac) -> [f64; 4] {
    let (w, h) = (f64::from(frame.w).clamp(0.05, 1.), f64::from(frame.h).clamp(0.05, 1.));
    [f64::from(frame.x).clamp(0., 1. - w), f64::from(frame.y).clamp(0., 1. - h), w, h]
}

/// The split tree a window shows.
fn tree(window: &Value) -> &Value {
    layers(window).iter().find(|l| l["kind"] == "tiled").map_or(&Value::Null, |l| &l["layout"])
}

fn layers(window: &Value) -> &[Value] {
    window["layers"].as_array().map(Vec::as_slice).unwrap_or_default()
}

/// A window's panes: its split's, then each floating layer's, back to front, with the layer. A minimized layer's
/// blocks run on but aren't drawn.
fn laid(window: &Value) -> (Vec<(String, Rect, Option<String>)>, Vec<LayoutSplit>) {
    let (tiled, splits) = frames(tree(window));
    let mut panes: Vec<_> = tiled.into_iter().map(|(block, rect)| (block, rect, None)).collect();
    for layer in layers(window).iter().filter(|l| l["kind"] == "floating" && l["state"] != "minimized") {
        let id = text_of(&layer["layer_id"]).to_string();
        panes.extend(frames(&layer["layout"]).0.into_iter().map(|(block, rect)| (block, rect, Some(id.clone()))));
    }
    (panes, splits)
}

/// A window's minimized layers, front first: the first pane of each, named by its title or Rex's label.
fn minimized(window: &Value, session: &str, state: &State) -> Vec<Minimized> {
    let layers = layers(window).iter().rev().filter(|l| l["kind"] == "floating" && l["state"] == "minimized");
    layers
        .filter_map(|l| {
            let (block, _) = frames(&l["layout"]).0.into_iter().next()?;
            let title = state.blocks.get(&block).map(|k| plain_title(&k.title).to_string()).filter(|t| !t.is_empty());
            let label = title.unwrap_or_else(|| text_of(&l["blocks"][0]["label"]).to_string());
            Some(Minimized { pane_id: format!("{session}/{block}"), label })
        })
        .collect()
}

/// Where a session's focused pane is now.
async fn session_cwd(core: &Control, session: &str) -> Option<String> {
    let view = view(core, session).await.ok()?;
    let window = window_of(&view, view["active_window_id"].as_str()?)?;
    cwd_of(core, &format!("{session}/{}", window["focused_block_id"].as_str()?)).await
}

fn text_of(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

/// The shell a pane's program runs under: the outermost process past `login`, the foreground one at a prompt.
fn shell_pid(process: &Value) -> &Value {
    let ancestors = process["ancestors"].as_array().map(Vec::as_slice).unwrap_or_default();
    ancestors.iter().rev().find(|p| p["name"] != "login").map_or(&process["foreground"]["pid"], |shell| &shell["pid"])
}

/// A process's name as it was started: Claude Code's binary is named for its version (`2.1.319`) and run as `claude`.
fn program_name(process: &Value) -> Option<String> {
    let argv0 = process["argv0"].as_str().map(|a| a.rsplit('/').next().unwrap_or(a).trim_start_matches('-')).filter(|a| !a.is_empty());
    argv0.or(process["name"].as_str()).map(str::to_string)
}

fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.trim_end().lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// The move of a pane to the place beside `anchor`. Rex places a block only beside another, and within its session.
fn moved(session: &str, block: &str, anchor: &str, down: bool) -> Value {
    json!({"session_id": session, "block_id": block, "anchor_block_id": anchor, "direction": if down { "vertical" } else { "horizontal" }, "side": "after"})
}

async fn move_to_new_tab(core: &Control, pane: &str, workspace: Option<&str>) -> anyhow::Result<Made> {
    let (session, block) = ids(pane)?;
    if workspace.is_some_and(|w| w != session) {
        bail!("Rex moves a pane only within its session");
    }
    // a new window has to start with a block of its own, which holds the place and then goes
    let made = core.call("session.new_window", json!({"session_id": session, "layout": shell(None), "focus": false})).await?;
    let holder = text_of(&made["block_ids"][0]);
    core.call("session.move_block", moved(session, block, holder, false)).await?;
    core.call("block.close", json!({"session_id": session, "block_id": holder})).await?;
    Ok(Made { tab_id: Some(format!("{session}/{}", text_of(&made["window_id"]))), ..Made::default() })
}

async fn move_to_tab(core: &Control, pane: &str, tab: &str, down: bool, target: Option<&str>) -> anyhow::Result<Made> {
    let (session, block) = ids(pane)?;
    let (other, window) = ids(tab)?;
    if other != session {
        bail!("Rex moves a pane only within its session");
    }
    let anchor = match target {
        Some(target) => ids(target)?.1.to_string(),
        None => {
            let view = view(core, session).await?;
            text_of(&window_of(&view, window).ok_or_else(|| anyhow!("no tab {tab}"))?["focused_block_id"]).to_string()
        }
    };
    core.call("session.move_block", moved(session, block, &anchor, down)).await?;
    Ok(Made { tab_id: Some(tab.to_string()), ..Made::default() })
}

/// The block that ends the first branch at `node`.
fn first_block(node: &Value) -> Option<&str> {
    node["block_id"].as_str().or_else(|| first_block(&node["split"]["before"]))
}

/// The split at `path`, as `session.resize_pane` names it: its direction and a block on each side.
fn border<'a>(mut node: &'a Value, path: &[bool]) -> Option<(&'a str, &'a str, &'a str)> {
    for after in path {
        node = &node["split"][if *after { "after" } else { "before" }];
    }
    let split = &node["split"];
    Some((split["direction"].as_str()?, first_block(&split["before"])?, first_block(&split["after"])?))
}

/// Every block a view shows.
fn blocks(view: &Value) -> Vec<String> {
    let windows = view["windows"].as_array().map(Vec::as_slice).unwrap_or_default();
    windows.iter().flat_map(|w| laid(w).0).map(|(block, ..)| block).collect()
}

/// A tree's panes and splits as the window takes them. A split's id is its path, which [`crate::model::split_path`]
/// reads back and [`Command::SetSplitRatio`] brings here.
fn frames(tree: &Value) -> (Vec<(String, Rect)>, Vec<LayoutSplit>) {
    fn walk(node: &Value, path: &str, panes: &mut Vec<(String, Rect)>, splits: &mut Vec<LayoutSplit>) {
        if let Some(block) = node["block_id"].as_str() {
            return panes.push((block.to_string(), rect(&node["rect"])));
        }
        let split = &node["split"];
        if split.is_null() {
            return; // an empty layout
        }
        let id = format!("split_0_{}", if path.is_empty() { "root" } else { path });
        let direction = if split["direction"] == "horizontal" { "right" } else { "down" };
        splits.push(LayoutSplit { id, direction: direction.into(), ratio: split["ratio"].as_f64().unwrap_or(0.5) as f32, rect: rect(&node["rect"]) });
        walk(&split["before"], &format!("{path}0"), panes, splits);
        walk(&split["after"], &format!("{path}1"), panes, splits);
    }
    let (mut panes, mut splits) = (Vec::new(), Vec::new());
    walk(tree, "", &mut panes, &mut splits);
    (panes, splits)
}

/// `[x, y, w, h]` fractions on the window's grid. Rounded as edges, so neighbours meet.
fn rect(fractions: &Value) -> Rect {
    let at = |i: usize| fractions[i].as_f64().unwrap_or(0.);
    let grid = |f: f64| (f * f64::from(AREA)).round() as u16;
    let (x, y) = (grid(at(0)), grid(at(1)));
    Rect { x, y, width: grid(at(0) + at(2)) - x, height: grid(at(1) + at(3)) - y }
}

/// The block a window's keys go to, of those Shepherd shows: a minimized layer's are the layout's.
fn focused_block<'a>(window: &'a Value, frames: &'a [(String, Rect, Option<String>)]) -> &'a str {
    let focused = text_of(&window["focused_block_id"]);
    if frames.iter().any(|(block, ..)| block == focused) { focused } else { frames.first().map_or("", |(block, ..)| block) }
}

/// A window's name, unless it is the one Rex made from its id: then its place, as a tab of herdr's.
fn tab_label(window: &Value, number: usize) -> String {
    let (label, id) = (text_of(&window["label"]), text_of(&window["window_id"]));
    let made = label.strip_prefix("Window ").is_some_and(|hex| id.replace('-', "").contains(hex));
    if label.is_empty() || made { number.to_string() } else { label.to_string() }
}

/// The sessions' views with what is known of their blocks.
fn snapshot(views: &[Value], state: &State, checkouts: &Checkouts) -> Snapshot {
    let mut out = Snapshot::default();
    let shown: Vec<&Value> = views.iter().filter(|v| !v["windows"].as_array().is_none_or(Vec::is_empty)).collect();
    for (w_at, view) in shown.iter().enumerate() {
        let session = text_of(&view["session_id"]);
        for (t_at, window) in view["windows"].as_array().into_iter().flatten().enumerate() {
            let tab = format!("{session}/{}", text_of(&window["window_id"]));
            let (frames, splits) = laid(window);
            let focused = focused_block(window, &frames);
            let mut laid = Vec::new();
            // a minimized layer's panes aren't laid out, but run on: their agents and programs mark their buttons
            let resting = layers(window).iter().filter(|l| l["kind"] == "floating" && l["state"] == "minimized");
            let resting: Vec<String> = resting.flat_map(|l| self::frames(&l["layout"]).0).map(|(block, _)| block).collect();
            for block in frames.iter().map(|(block, ..)| block).chain(&resting) {
                let known = state.blocks.get(block).cloned().unwrap_or_default();
                let pane = format!("{session}/{block}");
                out.panes.push(Pane {
                    pane_id: pane.clone(),
                    tab_id: tab.clone(),
                    workspace_id: session.to_string(),
                    terminal_id: pane.clone(),
                    terminal_title_stripped: (!known.title.is_empty()).then(|| plain_title(&known.title).to_string()),
                    cwd: known.cwd.clone(),
                    foreground_cwd: known.cwd,
                    report: Some(known.report),
                    ..Pane::default()
                });
            }
            for (block, rect, layer) in &frames {
                laid.push(LayoutPane { pane_id: format!("{session}/{block}"), focused: block == focused, rect: *rect, layer: layer.clone() });
            }
            out.tabs.push(Tab { tab_id: tab.clone(), workspace_id: session.to_string(), label: tab_label(window, t_at + 1), number: t_at as u32 + 1, agent_status: AgentStatus::Unknown });
            out.layouts.push(Layout {
                tab_id: tab,
                zoomed: window["zoomed_block_id"].is_string(),
                focused_pane_id: format!("{session}/{focused}"),
                area: Rect { x: 0, y: 0, width: AREA, height: AREA },
                panes: laid,
                splits,
                minimized: minimized(window, session, state),
            });
        }
        out.workspaces.push(Workspace {
            workspace_id: session.to_string(),
            label: shown_label(text_of(&view["label"])).to_string(),
            number: w_at as u32 + 1,
            active_tab_id: Some(format!("{session}/{}", text_of(&view["active_window_id"]))),
            agent_status: AgentStatus::Unknown,
            worktree: checkouts.get(session),
        });
    }
    let focused = shown.iter().find(|v| state.focused.as_deref() == v["session_id"].as_str()).or(shown.first());
    let window = focused.and_then(|v| window_of(v, v["active_window_id"].as_str()?));
    let session = focused.map_or("", |v| text_of(&v["session_id"]));
    out.focused_workspace_id = focused.map(|_| session.to_string());
    out.focused_tab_id = focused.map(|v| format!("{session}/{}", text_of(&v["active_window_id"])));
    out.focused_pane_id = window.map(|w| format!("{session}/{}", focused_block(w, &laid(w).0)));
    out
}

/// The status a terminal reports.
fn reported(report: &mut Report, record: &Value) {
    report.state = record["state"].as_str().map(str::to_string);
    report.app = record.get("effective_app").unwrap_or(&record["app"]).as_str().map(str::to_string);
}

/// Asks what a block's events won't say: all of it once, then what it runs when that changes.
async fn learn(core: &Control, shared: &Shared, session: &str, block: &str) {
    let known = shared.state.lock().unwrap().blocks.get(block).cloned();
    if known.as_ref().is_some_and(|b| !b.stale) {
        return;
    }
    let pane = format!("{session}/{block}");
    let Ok(process) = on_block(core, &pane, "process", json!({})).await else { return };
    let fg = &process["foreground"];
    let mut known = match known {
        Some(known) => known,
        None => {
            let title = on_block(core, &pane, "title", json!({})).await.unwrap_or_default();
            let records = on_block(core, &pane, "program_status", json!({})).await.unwrap_or_default();
            let mut new = Block { title: text_of(&title["title"]).to_string(), ..Block::default() };
            if let Some(record) = records["records"].get(0) {
                reported(&mut new.report, record);
            }
            new
        }
    };
    known.cwd = fg["cwd"].as_str().map(str::to_string);
    known.report.program = fg.get("argv0").unwrap_or(&fg["name"]).as_str().map(str::to_string);
    known.stale = false;
    shared.state.lock().unwrap().blocks.insert(block.to_string(), known);
}

/// Takes in one of the server's events: whether a snapshot would show it. `Null`: the connection ended.
fn heard(event: &Value, state: &mut State) -> bool {
    match event["type"].as_str() {
        None | Some("session_created" | "session_destroyed" | "session_moved") => true,
        Some("session_event") => {
            let inner = &event["payload"]["event"];
            let fields = &inner["payload"];
            match inner["type"].as_str() {
                Some("session_view_changed" | "session_label_changed" | "window_label_changed") => true,
                Some("block_closed") => {
                    state.blocks.remove(text_of(&fields["block_id"]));
                    false // its view changes too, if it was in one
                }
                Some("block_event") => state.blocks.get_mut(text_of(&fields["block_id"])).is_some_and(|b| block_event(b, text_of(&fields["name"]), &fields["payload"])),
                _ => false,
            }
        }
        _ => false,
    }
}

/// One block's event on what is known of it: whether a snapshot would show it.
fn block_event(block: &mut Block, name: &str, fields: &Value) -> bool {
    match name {
        // the `title` of an agent spinning changes a few times a second, its words don't
        "title_changed" => {
            let new = text_of(&fields["title"]);
            let news = plain_title(&block.title) != plain_title(new);
            block.title = new.to_string();
            news
        }
        "pwd_changed" | "process_changed" => {
            block.stale = true;
            true
        }
        "program_status_changed" if fields["id"] == "" => {
            reported(&mut block.report, &fields["record"]);
            block.report.turn += 1;
            true
        }
        "program_status_removed" if fields["ids"].as_array().is_some_and(|ids| ids.iter().any(|id| id == "")) => {
            block.report.state = None;
            block.report.app = None;
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::Own;
    use crate::backend::{Agents, Backend, SessionEvent};
    use std::time::Duration;

    /// Two sessions as the server shows them, the first with a split and a second window.
    fn views() -> Vec<Value> {
        let block = |id: &str, rect: [f64; 4]| json!({"rect": rect, "block_id": id});
        let split = json!({"split_id": "split:1", "direction": "horizontal", "ratio": 0.25, "before": block("block:a", [0., 0., 0.25, 1.]), "after": block("block:b", [0.25, 0., 0.75, 1.])});
        let tiled = |layout: Value| json!([{"layer_id": "layer:1", "kind": "tiled", "bounds": [0, 0, 1, 1], "layout": layout}]);
        vec![
            json!({"revision": 3, "session_id": "session:1", "label": "shepherd", "active_window_id": "window:2", "windows": [
                {"window_id": "window:01a19222-319e", "label": "Term", "focused_block_id": "block:c", "layers": tiled(block("block:c", [0., 0., 1., 1.]))},
                {"window_id": "window:2", "label": "Window 2", "focused_block_id": "block:b", "layers": tiled(json!({"rect": [0, 0, 1, 1], "split": split}))},
            ]}),
            json!({"revision": 1, "session_id": "session:2", "label": "empty", "windows": []}),
            json!({"revision": 1, "session_id": "session:3", "label": "mono", "active_window_id": "window:3", "windows": [
                {"window_id": "window:3", "label": "w", "focused_block_id": "block:d", "layers": tiled(block("block:d", [0., 0., 1., 1.]))},
            ]}),
        ]
    }

    fn state() -> State {
        let mut state = State::default();
        let report = Report { state: Some("working".into()), app: Some("claude-code".into()), program: Some("claude".into()), turn: 2 };
        state.blocks.insert("block:b".into(), Block { title: "⠂ fixing it".into(), cwd: Some("/repo".into()), report, stale: false });
        state.blocks.insert("block:a".into(), Block { cwd: Some("/repo/src".into()), ..Block::default() });
        state
    }

    fn checkouts() -> Checkouts {
        let checkouts = Checkouts::default();
        checkouts.set("session:3".into(), Worktree { repo_key: "/r/.git".into(), is_linked_worktree: false, checkout_path: "/r".into(), repo_root: "/r".into() });
        checkouts
    }

    /// As the window reads them, with their agents.
    fn read(views: &[Value], state: &State) -> Snapshot {
        let mut shot = snapshot(views, state, &checkouts());
        Own::new(Arc::new(Rex::default())).read(&mut shot);
        shot
    }

    #[test]
    fn the_window_reads_rexs_sessions_as_herdrs() {
        let s = read(&views(), &state());
        assert_eq!(s.workspaces.iter().map(|w| w.label.as_str()).collect::<Vec<_>>(), ["shepherd", "mono"], "a session with no windows has nothing to show");
        assert_eq!(s.workspaces[0].agent_status, AgentStatus::Working);
        assert_eq!(s.tabs[0].label, "Term");
        assert_eq!(s.tabs[1].label, "2", "the name Rex made up from its id");
        assert_eq!(s.focused_tab_id.as_deref(), Some("session:1/window:2"), "the first session, as nothing focused one yet");
        assert_eq!(s.focused_pane_id.as_deref(), Some("session:1/block:b"));
        let pane = s.panes.iter().find(|p| p.pane_id == "session:1/block:b").unwrap();
        assert_eq!(pane.agent.as_deref(), Some("claude"));
        assert_eq!(pane.terminal_title_stripped.as_deref(), Some("fixing it"));
        assert_eq!(pane.foreground_cwd.as_deref(), Some("/repo"));
        let frames = s.pane_frames("session:1/window:2");
        assert_eq!((frames[0].frame.w, frames[1].frame.x), (0.25, 0.25));
        let splits = s.split_frames("session:1/window:2");
        assert_eq!((splits[0].path.clone(), splits[0].vertical), (vec![], true));
        assert!(s.workspaces[1].worktree.is_some());

        let mut zoomed = views();
        zoomed[0]["windows"][1]["zoomed_block_id"] = json!("block:b");
        let s = read(&zoomed, &state());
        assert_eq!(s.pane_frames("session:1/window:2").len(), 1, "the zoomed one fills it");
    }

    #[test]
    fn a_floating_layer_floats_over_the_split_until_minimized() {
        let mut floating = views();
        let layer = json!({"layer_id": "layer:f", "kind": "floating", "bounds": [0.2, 0.1, 0.6, 0.5], "layout": {"rect": [0.2, 0.1, 0.6, 0.5], "block_id": "block:float"}});
        floating[0]["windows"][1]["layers"].as_array_mut().unwrap().push(layer);
        floating[0]["windows"][1]["focused_block_id"] = json!("block:float");
        let s = read(&floating, &state());
        assert_eq!(s.focused_pane_id.as_deref(), Some("session:1/block:float"));
        let frames = s.pane_frames("session:1/window:2");
        let last = frames.last().unwrap();
        assert_eq!((last.pane_id.as_str(), last.layer.as_deref(), last.frame.x), ("session:1/block:float", Some("layer:f"), 0.2), "drawn last, over the split");
        assert_eq!(s.split_frames("session:1/window:2").len(), 1, "a layer's own splits aren't the tab's");
        let layout = s.layouts.iter().find(|l| l.tab_id == "session:1/window:2").unwrap();
        assert_eq!(layout.neighbor("session:1/block:a", crate::commands::Direction::Right), Some("session:1/block:b"), "past what floats");

        floating[0]["windows"][1]["layers"][1]["state"] = json!("minimized");
        let s = read(&floating, &state());
        assert_eq!(s.focused_pane_id.as_deref(), Some("session:1/block:a"), "a minimized layer leaves the keys to the layout");
        assert_eq!(s.pane_frames("session:1/window:2").len(), 2);
        assert!(s.panes.iter().any(|p| p.pane_id == "session:1/block:float"), "it runs on: its agent and program mark its button");
    }

    #[test]
    fn a_program_goes_by_the_name_it_was_started_as() {
        let claude = json!({"name": "2.1.319", "path": "/Users/me/.local/share/claude/versions/2.1.319", "argv0": "claude"});
        assert_eq!(program_name(&claude).as_deref(), Some("claude"));
        assert_eq!(program_name(&json!({"name": "nvim", "argv0": "/opt/homebrew/bin/nvim"})).as_deref(), Some("nvim"));
        assert_eq!(program_name(&json!({"name": "zsh", "argv0": "-zsh"})).as_deref(), Some("zsh"), "a login shell's dash");
        assert_eq!(program_name(&json!({"name": "htop"})).as_deref(), Some("htop"));
    }

    #[test]
    fn focusing_a_session_shows_it() {
        let mut state = state();
        state.focused = Some("session:3".into());
        let s = read(&views(), &state);
        assert_eq!(s.focused_workspace_id.as_deref(), Some("session:3"));
        assert_eq!(s.focused_pane_id.as_deref(), Some("session:3/block:d"));
    }

    #[test]
    fn only_a_linked_worktree_carries_the_mark() {
        let tree = |linked| Worktree { repo_key: "/r/.git".into(), is_linked_worktree: linked, checkout_path: "/r".into(), repo_root: "/r".into() };
        assert_eq!(kept_label("fix".into(), Some(&tree(true))), "\u{f126} r/fix");
        assert_eq!(shown_label("\u{f126} r/fix/two"), "fix/two", "a name may hold a slash");
        assert_eq!(shown_label("a/b"), "a/b", "only a worktree's is taken off");
        assert_eq!(kept_label("fix".into(), Some(&tree(false))), "fix", "the repo's own checkout");
        assert_eq!(kept_label("fix".into(), None), "fix");
    }

    #[test]
    fn a_splits_border_is_named_by_a_block_on_each_side() {
        let leaf = |b: &str| json!({"block_id": b});
        let inner = json!({"split": {"direction": "vertical", "before": leaf("b"), "after": leaf("c")}});
        let root = json!({"split": {"direction": "horizontal", "before": leaf("a"), "after": inner}});
        assert_eq!(border(&root, &[]), Some(("horizontal", "a", "b")));
        assert_eq!(border(&root, &[true]), Some(("vertical", "b", "c")));
        assert_eq!(border(&root, &[false]), None, "a pane has no border");
    }

    #[test]
    fn neighbours_meet_on_the_grid() {
        let (a, b) = (rect(&json!([0, 0, 0.3333, 1])), rect(&json!([0.3333, 0, 0.6667, 1])));
        assert_eq!(a.width + b.width, AREA);
        assert_eq!(a.width, b.x);
    }

    #[test]
    fn events_keep_a_block_up_to_date() {
        let mut state = state();
        let on = |name: &str, payload: Value| json!({"type": "session_event", "payload": {"session_id": "session:1", "event": {"type": "block_event", "payload": {"block_id": "block:b", "name": name, "payload": payload}}}});
        assert!(!heard(&on("title_changed", json!({"title": "⠐ fixing it"})), &mut state), "a spinning title");
        assert!(heard(&on("title_changed", json!({"title": "⠂ done"})), &mut state));
        assert!(heard(&on("process_changed", json!({})), &mut state));
        assert!(state.blocks["block:b"].stale, "the next snapshot asks");
        assert!(heard(&on("program_status_changed", json!({"id": "", "record": {"state": "done", "effective_app": "pi"}})), &mut state));
        assert_eq!((state.blocks["block:b"].report.app.as_deref(), state.blocks["block:b"].report.turn), (Some("pi"), 3));
        assert!(!heard(&on("program_status_changed", json!({"id": "build/test", "record": {"state": "working"}})), &mut state), "a part of the program's work");
        assert!(heard(&on("program_status_removed", json!({"ids": [""]})), &mut state));
        assert!(state.blocks["block:b"].report.state.is_none());
        assert!(!heard(&on("bell", json!({})), &mut state));

        let closed = json!({"type": "session_event", "payload": {"event": {"type": "block_closed", "payload": {"block_id": "block:b"}}}});
        assert!(!heard(&closed, &mut state));
        assert!(!state.blocks.contains_key("block:b"));
        assert!(heard(&json!({"type": "session_event", "payload": {"event": {"type": "session_view_changed", "payload": {"revision": 9}}}}), &mut state));
        assert!(heard(&json!({"type": "session_destroyed", "payload": {}}), &mut state));
        assert!(heard(&Value::Null, &mut state), "the connection ended");
        assert!(!heard(&json!({"type": "keymap_changed"}), &mut state));
    }

    #[test]
    fn the_shell_is_what_runs_under_login() {
        let login = json!({"pid": 1, "name": "login"});
        let at_prompt = json!({"foreground": {"pid": 2}, "ancestors": [login]});
        assert_eq!(shell_pid(&at_prompt), 2);
        let running = json!({"foreground": {"pid": 4}, "ancestors": [{"pid": 3, "name": "node"}, {"pid": 2, "name": "zsh"}, login]});
        assert_eq!(shell_pid(&running), 2);
    }

    #[test]
    fn a_read_keeps_the_last_lines() {
        assert_eq!(tail("a\nb\nc\n\n", 2), "b\nc");
        assert_eq!(tail("a", 5), "a");
    }

    /// Waits for what `done` wants of the snapshots.
    async fn until(backend: &Backend, what: &str, mut done: impl FnMut(&Snapshot) -> bool) -> Snapshot {
        for _ in 0..100 {
            let shot = backend.snapshot().await.unwrap();
            if done(&shot) {
                return shot;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("no {what}");
    }

    /// Live, on a server of its own: `REX_SERVER=unix://<socket> cargo test rex::api::tests::live -- --ignored --nocapture`.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn live() {
        assert!(std::env::var("REX_SERVER").is_ok(), "run it on an isolated server: it closes what it opens");
        let backend = Backend::of(Kind::Rex, None);
        let (b, mux) = (&backend, &backend.mux);
        let mut events = mux.changes(Vec::new()).await.unwrap();
        let before = until(b, "a snapshot", |_| true).await.workspaces.len();
        let repo = tempfile::tempdir().unwrap();
        let repo = std::fs::canonicalize(repo.path()).unwrap().to_string_lossy().into_owned();
        let made = mux.run(Command::NewWorkspace { cwd: Some(repo.clone()) }).await.unwrap();
        let (workspace, first, tab) = (made.workspace_id.unwrap(), made.pane_id.unwrap(), made.tab_id.unwrap());
        assert!(tokio::time::timeout(Duration::from_secs(2), events.recv()).await.unwrap().is_some(), "the server said so");

        let second = mux.run(Command::Split { pane_id: first.clone(), down: false, cwd: None }).await.unwrap().pane_id.unwrap();
        mux.run(Command::FocusTab { tab_id: tab.clone() }).await.unwrap();
        let shot = until(b, "two panes", |s| s.panes.iter().filter(|p| p.workspace_id == workspace).count() == 2).await;
        assert_eq!(shot.workspaces.len(), before + 1);
        assert_eq!(shot.focused_workspace_id.as_deref(), Some(workspace.as_str()));
        assert_eq!(shot.focused_pane_id.as_deref(), Some(second.as_str()), "a split takes the focus");
        let frames = shot.pane_frames(&tab);
        assert_eq!((frames.len(), shot.split_frames(&tab).len()), (2, 1));

        // what a pane does: a size, a snapshot of what is there, and a stream
        let (rex, pane) = (mux.clone(), first.clone());
        let (attached, mut stream) = tokio::task::spawn_blocking(move || rex.attach(&pane, 90, 20, (9, 18), Mode::Control)).await.unwrap().unwrap();
        attached.input(b"seq 1 200; echo shepherd-$((40+2))\r");
        let mut emulator = crate::terminal::emulator::Emulator::raw(90, 20);
        let shown = |e: &crate::terminal::emulator::Emulator| (0..e.rows()).any(|row| e.row_text(row).trim() == "shepherd-42");
        let printed = tokio::time::timeout(Duration::from_secs(10), async {
            use futures::StreamExt;
            while let Some(event) = stream.next().await {
                match event {
                    SessionEvent::Snapshot(bytes) => emulator = crate::terminal::emulator::Emulator::restored(&bytes, 90, 20).unwrap(),
                    SessionEvent::Frame(bytes) => {
                        emulator.feed(&bytes);
                        if shown(&emulator) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            false
        });
        assert!(printed.await.unwrap(), "the shell ran it and the emulator shows its output");
        assert!(emulator.history_lines() > 150, "what scrolled off is the emulator's own history: {}", emulator.history_lines());
        let probe = Rex::default();
        let core = probe.connected().await.unwrap();
        let size = on_block(&core, &first, "size", json!({})).await.unwrap();
        assert_eq!((size["columns"].as_u64(), size["rows"].as_u64()), (Some(90), Some(20)), "the pane's size is the terminal's");
        assert!(mux.read(first.clone(), 5, false).await.unwrap().contains("shepherd-42"));

        // a second look at the same block, as a second window has: the server replays it to it
        let (rex, pane) = (mux.clone(), first.clone());
        let (second_look, mut replay) = tokio::task::spawn_blocking(move || rex.attach(&pane, 90, 20, (9, 18), Mode::Observe)).await.unwrap().unwrap();
        let again = tokio::time::timeout(Duration::from_secs(5), async {
            use futures::StreamExt;
            while let Some(event) = replay.next().await {
                if let SessionEvent::Snapshot(bytes) = event {
                    return crate::terminal::emulator::Emulator::restored(&bytes, 90, 20).unwrap().history_lines();
                }
            }
            0
        });
        assert!(again.await.unwrap() > 150, "a snapshot holds the history");
        drop(second_look);

        // an agent's status, over OSC 7501 as Claude Code reports it
        attached.input(b"printf '\\033]7501;state=working:app=claude-code\\007'\r");
        let agent = until(b, "a reported status", |s| s.panes.iter().any(|p| p.pane_id == first && p.agent.as_deref() == Some("claude"))).await;
        assert_eq!(agent.panes.iter().find(|p| p.pane_id == first).unwrap().agent_status, AgentStatus::Working);
        drop(attached);

        let tab2 = mux.run(Command::NewTab { workspace_id: workspace.clone() }).await.unwrap().tab_id.unwrap();
        mux.run(Command::RenameTab { tab_id: tab2.clone(), label: "logs".into() }).await.unwrap();
        mux.run(Command::MoveTab { tab_id: tab2.clone(), insert_index: 0 }).await.unwrap();
        let shot = until(b, "a renamed tab", |s| s.tabs.iter().any(|t| t.label == "logs")).await;
        assert_eq!(shot.tabs_of(&workspace)[0].label, "logs");
        mux.run(Command::RenameWorkspace { workspace_id: workspace.clone(), label: "renamed".into() }).await.unwrap();
        mux.run(Command::SwapPanes { source: first.clone(), target: second.clone() }).await.unwrap();
        mux.run(Command::SetSplitRatio { tab_id: tab.clone(), path: Vec::new(), ratio: 0.25 }).await.unwrap();
        mux.run(Command::Zoom { pane_id: second.clone() }).await.unwrap();
        let shot = until(b, "a zoom", |s| s.layouts.iter().any(|l| l.tab_id == tab && l.zoomed)).await;
        assert_eq!(shot.workspaces.iter().find(|w| w.workspace_id == workspace).unwrap().label, "renamed");
        mux.run(Command::Zoom { pane_id: second.clone() }).await.unwrap();
        // a pane floats over the split, is moved and raised there, and docks back into it
        let floats = |s: &Snapshot| s.pane_frames(&tab).into_iter().find(|f| f.pane_id == second && f.layer.is_some());
        mux.float(second.clone(), Float::Out).await.unwrap();
        let shot = until(b, "a floating pane", |s| floats(s).is_some()).await;
        assert!(shot.split_frames(&tab).is_empty(), "the split is the other pane alone");
        mux.float(second.clone(), Float::Place(Frac { x: 0.5, y: 0.5, w: 0.4, h: 0.4 })).await.unwrap();
        mux.float(second.clone(), Float::Raise).await.unwrap();
        until(b, "a moved floating pane", |s| floats(s).is_some_and(|f| (f.frame.x - 0.5).abs() < 0.01 && (f.frame.w - 0.4).abs() < 0.01)).await;
        // minimized to a button at the tab's foot, and back
        let minimized = |s: &Snapshot| s.layouts.iter().find(|l| l.tab_id == tab).is_some_and(|l| l.minimized.iter().any(|m| m.pane_id == second));
        mux.float(second.clone(), Float::Minimize).await.unwrap();
        let shot = until(b, "a minimized pane", minimized).await;
        assert!(floats(&shot).is_none(), "not drawn while minimized");
        mux.float(second.clone(), Float::Restore).await.unwrap();
        until(b, "a restored pane", |s| !minimized(s) && floats(s).is_some()).await;
        mux.float(second.clone(), Float::Dock).await.unwrap();
        until(b, "a docked pane", |s| s.split_frames(&tab).len() == 1 && floats(s).is_none()).await;
        let moved = mux.run(Command::MovePaneToNewTab { pane_id: second.clone(), workspace_id: None }).await.unwrap().tab_id.unwrap();
        let shot = until(b, "a pane in its own tab", |s| s.panes.iter().any(|p| p.pane_id == second && p.tab_id == moved)).await;
        assert_eq!(shot.pane_frames(&moved).len(), 1, "the shell that held its place is gone");
        assert!(mux.run(Command::MovePaneToNewTab { pane_id: second.clone(), workspace_id: Some("session:other".into()) }).await.is_err());
        // the split's last pane closed under a floating one: that one takes its place, not a tab of layers alone
        let third = mux.run(Command::Split { pane_id: second.clone(), down: false, cwd: None }).await.unwrap().pane_id.unwrap();
        mux.float(third.clone(), Float::Out).await.unwrap();
        until(b, "a floating third pane", |s| s.pane_frames(&moved).iter().any(|f| f.pane_id == third && f.layer.is_some())).await;
        mux.run(Command::ClosePane { pane_id: second.clone() }).await.unwrap();
        until(b, "the floating pane docked", |s| s.pane_frames(&moved).iter().map(|f| (f.pane_id.as_str(), f.layer.is_none())).eq([(third.as_str(), true)])).await;

        mux.run(Command::ClosePane { pane_id: first.clone() }).await.unwrap();
        until(b, "a closed pane", |s| !s.panes.iter().any(|p| p.pane_id == first)).await;
        mux.run(Command::CloseTab { tab_id: tab2.clone() }).await.unwrap();
        until(b, "a closed tab", |s| !s.tabs.iter().any(|t| t.tab_id == tab2)).await;
        mux.run(Command::CloseWorkspace { workspace_id: workspace }).await.unwrap();
        until(b, "a closed workspace", |s| s.workspaces.len() == before).await;

        // a worktree, in the real `~/.herdr/worktrees/<a temporary repo>`, which goes at the end
        for args in [&["init", "-q"][..], &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "first"]] {
            assert!(std::process::Command::new("git").arg("-C").arg(&repo).args(args).status().unwrap().success());
        }
        let trees = &backend.worktrees;
        let main = mux.run(Command::NewWorkspace { cwd: Some(repo) }).await.unwrap().workspace_id.unwrap();
        // the repo is found from where the first pane is, once a snapshot has seen it
        until(b, "a repo", |s| s.workspaces.iter().any(|w| w.workspace_id == main && w.worktree.is_some())).await;
        let made = trees.create(main.clone(), "fix/rex".into()).await.unwrap();
        assert!(made.tab_id.is_some(), "the window selects its tab");
        let linked = made.workspace_id.unwrap();
        let shot = until(b, "a nested worktree", |s| s.workspace_tree().iter().any(|(w, child)| w.workspace_id == linked && child.is_some())).await;
        let tree = shot.workspaces.iter().find(|w| w.workspace_id == linked).unwrap().worktree.clone().unwrap();
        assert!(tree.is_linked_worktree);
        assert_eq!(shot.workspaces.iter().find(|w| w.workspace_id == linked).unwrap().label, "fix-rex");
        let core = tokio::task::spawn_blocking(|| Control::open(&Endpoint::local().unwrap(), |_| {})).await.unwrap().unwrap();
        let listed = core.call("session.list", json!({})).await.unwrap();
        let kept = format!("\u{f126} {}/fix-rex", crate::worktree::repo_name(&tree).unwrap());
        assert!(listed["sessions"].as_array().unwrap().iter().any(|s| s["label"] == kept.as_str()), "{listed}");
        let open: Vec<_> = trees.list(main.clone()).await.unwrap().into_iter().map(|w| w.open_workspace_id.unwrap_or_default()).collect();
        assert_eq!(open, [main.clone(), linked.clone()]);
        std::fs::write(std::path::Path::new(&tree.checkout_path).join("work"), "unsaved").unwrap();
        let refused = trees.remove(linked.clone(), false).await.unwrap_err().to_string();
        assert!(refused.starts_with("dirty_worktree_requires_force"), "{refused}");
        trees.remove(linked.clone(), true).await.unwrap();
        until(b, "a removed worktree", |s| !s.workspaces.iter().any(|w| w.workspace_id == linked)).await;
        assert!(!std::path::Path::new(&tree.checkout_path).exists());
        mux.run(Command::CloseWorkspace { workspace_id: main }).await.unwrap();
        std::fs::remove_dir(std::path::Path::new(&tree.checkout_path).parent().unwrap()).unwrap();
    }
}
