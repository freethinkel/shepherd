//! herdr's socket API as Shepherd calls it, answered by Rex's server: a session is a workspace, a window a tab and a
//! block a pane, so the window runs on any backend unchanged. [`crate::herdr::Client`] comes here instead of herdr's
//! socket. Rex's ids name one thing, herdr's calls take only a pane or a tab: `session/block` and `session/window`.
use super::TERMINAL;
use super::conn::Control;
use crate::hc::api::{agent, folder, loudest, plain_title, status};
use crate::hc::tree::AREA;
use crate::model::Worktree;
use crate::worktree;
use anyhow::{anyhow, bail};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, watch};

const SHELL: &str = "com.superlogical.terminal.shell";

static CORE: Mutex<Option<Arc<Control>>> = Mutex::new(None);
static STATE: LazyLock<Mutex<State>> = LazyLock::new(Default::default);
/// Counts what the server said that changes a snapshot.
static NEWS: LazyLock<watch::Sender<u64>> = LazyLock::new(|| watch::channel(0).0);

/// What the server's views leave out.
#[derive(Default)]
struct State {
    /// By block: what its terminal runs and reports, as herdr-core describes a terminal, which [`agent`] and [`status`]
    /// read. Asked once, then kept by the block's events: a view has no room for it, and asking on each snapshot
    /// would be four calls per pane.
    blocks: HashMap<String, Value>,
    /// The turn each finished agent was looked at in, which reads as idle after that (herdr's done → idle).
    seen: HashMap<String, u64>,
    /// The session the window shows: Rex keeps the active window of each, not one for the server.
    focused: Option<String>,
    /// The checkout each session is on. Only git knows, and Rex has nowhere to keep it, so it is asked once a run.
    checkouts: HashMap<String, Option<Worktree>>,
}

fn connect() -> anyhow::Result<Arc<Control>> {
    let mut held = CORE.lock().unwrap();
    if let Some(core) = held.as_ref().filter(|c| !c.lost()) {
        return Ok(core.clone());
    }
    let open = || {
        Control::open(|event| {
            if heard(event, &mut STATE.lock().unwrap()) {
                NEWS.send_modify(|n| *n += 1);
            }
        })
    };
    let core = open().or_else(|e| {
        if !crate::herdr::no_server(&e) {
            return Err(e);
        }
        super::start_server()?;
        open()
    })?;
    core.blocking_call("session.attach_all", json!({"event_scope": "all"}))?;
    STATE.lock().unwrap().blocks.clear(); // the events since the last connection are lost
    *held = Some(core.clone());
    Ok(core)
}

/// Off: the connection goes; the server runs on.
pub fn disconnect() {
    if let Some(core) = CORE.lock().unwrap().take() {
        core.close();
    }
}

async fn connected() -> anyhow::Result<Arc<Control>> {
    tokio::task::spawn_blocking(connect).await?
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

/// The split tree a window shows. Rex's floating layers have no place in Shepherd's panes.
fn tree(window: &Value) -> &Value {
    let layers = window["layers"].as_array().map(Vec::as_slice).unwrap_or_default();
    layers.iter().find(|l| l["kind"] == "tiled").map_or(&Value::Null, |l| &l["layout"])
}

/// Where a session's focused pane is now.
async fn session_cwd(core: &Control, session: &str) -> Option<String> {
    let view = view(core, session).await.ok()?;
    let window = window_of(&view, view["active_window_id"].as_str()?)?;
    cwd_of(core, &format!("{session}/{}", window["focused_block_id"].as_str()?)).await
}

/// A new session on one shell in `cwd`, on the checkout that is in.
async fn open(core: &Control, cwd: Option<&str>) -> anyhow::Result<Value> {
    let mut args = json!({"initial_windows": [{"layout": shell(cwd)}]});
    if let Some(cwd) = cwd {
        args["label"] = json!(folder(cwd));
    }
    let made = core.call("session.create", args).await?;
    let session = made["session_id"].as_str().ok_or_else(|| anyhow!("session.create gave no session"))?;
    let (tab, pane) = (format!("{session}/{}", text_of(&made["initial_windows"][0]["window_id"])), format!("{session}/{}", text_of(&made["initial_windows"][0]["block_ids"][0])));
    if let Some(cwd) = cwd.map(str::to_string) {
        let tree = tokio::task::spawn_blocking(move || worktree::of(&cwd)).await?;
        STATE.lock().unwrap().checkouts.insert(session.to_string(), tree);
    }
    Ok(json!({"workspace": {"workspace_id": session, "active_tab_id": tab}, "tab": {"tab_id": tab}, "root_pane": {"pane_id": pane}}))
}

fn text_of(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

/// The checkout a session is on. One opened elsewhere may have `cd`ed into a repo: that is its checkout from now on.
async fn checkout(core: &Control, session: &str) -> anyhow::Result<Worktree> {
    if let Some(tree) = STATE.lock().unwrap().checkouts.get(session).cloned().flatten() {
        return Ok(tree);
    }
    let cwd = session_cwd(core, session).await.unwrap_or_default();
    let found = cwd.clone();
    let tree = tokio::task::spawn_blocking(move || worktree::of(&found)).await?.ok_or_else(|| anyhow!("not_git_worktree: {cwd}"))?;
    STATE.lock().unwrap().checkouts.insert(session.to_string(), Some(tree.clone()));
    Ok(tree)
}

/// One request, herdr's method and params, herdr's result shape.
pub async fn call(method: &str, p: Value) -> anyhow::Result<Value> {
    let core = connected().await?;
    let core = core.as_ref();
    let text = |key: &str| p[key].as_str().map(str::to_string).ok_or_else(|| anyhow!("{method}: no {key}"));
    Ok(match method {
        "ping" => json!({"type": "pong"}),
        "session.snapshot" => {
            let listed = core.call("session.list", json!({})).await?;
            let mut views = Vec::new();
            for session in listed["sessions"].as_array().map(Vec::as_slice).unwrap_or_default() {
                let id = text_of(&session["session_id"]);
                // gone since the list: the next snapshot won't have it
                let Ok(view) = view(core, id).await else { continue };
                for block in blocks(&view) {
                    learn(core, id, &block).await;
                }
                find_checkout(id, &view).await;
                views.push(view);
            }
            json!({"snapshot": snapshot(&views, &STATE.lock().unwrap())})
        }
        "pane.get" => {
            let pane = text("pane_id")?;
            let cwd = cwd_of(core, &pane).await;
            json!({"pane": {"pane_id": pane, "cwd": cwd, "foreground_cwd": cwd}})
        }
        "pane.read" => {
            let read = on_block(core, &text("pane_id")?, "format", json!({"format": "text", "trim": true})).await?;
            let lines = p["lines"].as_u64().unwrap_or(0) as usize;
            json!({"read": {"text": tail(text_of(&read["content"]), lines)}})
        }
        "pane.process_info" => {
            let process = on_block(core, &text("pane_id")?, "process", json!({})).await?;
            let fg = &process["foreground"];
            json!({"process_info": {"foreground_process_group_id": fg["pid"], "shell_pid": shell_pid(&process), "foreground_processes": [{"pid": fg["pid"], "name": fg["name"]}]}})
        }
        "pane.send_input" => {
            let pane = text("pane_id")?;
            if let Some(text) = p["text"].as_str() {
                on_block(core, &pane, "write", json!({"data": text})).await?;
            }
            // the one key herdr's callers name
            if p["keys"].as_array().is_some_and(|keys| keys.iter().any(|k| k == "enter")) {
                on_block(core, &pane, "write", json!({"data": "\r"})).await?;
            }
            json!({})
        }
        // ponytail: a fixed wait for the prompt of the shell just started, as on herdr-core
        "pane.wait_for_output" => {
            tokio::time::sleep(Duration::from_millis(400)).await;
            json!({})
        }
        "workspace.create" => open(core, p["cwd"].as_str()).await?,
        "tab.create" => {
            let session = text("workspace_id")?;
            let cwd = session_cwd(core, &session).await;
            let made = core.call("session.new_window", json!({"session_id": session, "layout": shell(cwd.as_deref()), "focus": false})).await?;
            json!({"tab": {"tab_id": format!("{session}/{}", text_of(&made["window_id"]))}, "root_pane": {"pane_id": format!("{session}/{}", text_of(&made["block_ids"][0]))}})
        }
        "pane.split" => {
            let target = text("target_pane_id")?;
            let (session, block) = ids(&target)?;
            let cwd = match p["cwd"].as_str() {
                Some(cwd) => Some(cwd.to_string()),
                None => cwd_of(core, &target).await,
            };
            let direction = if p["direction"] == "down" { "vertical" } else { "horizontal" };
            let args = json!({"session_id": session, "anchor_id": block, "direction": direction, "side": "after", "layout": shell(cwd.as_deref()), "focus": p["focus"].as_bool().unwrap_or(true)});
            let made = core.call("session.new_split", args).await?;
            json!({"pane": {"pane_id": format!("{session}/{}", text_of(&made["block_ids"][0]))}})
        }
        "workspace.close" => core.call("session.destroy", json!({"session_id": text("workspace_id")?})).await?,
        "tab.close" => {
            let tab = text("tab_id")?;
            let (session, window) = ids(&tab)?;
            core.call("session.close_window", json!({"session_id": session, "window_id": window})).await?
        }
        "pane.close" => {
            let pane = text("pane_id")?;
            let (session, block) = ids(&pane)?;
            core.call("block.close", json!({"session_id": session, "block_id": block})).await?
        }
        "workspace.rename" => core.call("session.set_label", json!({"session_id": text("workspace_id")?, "label": text("label")?})).await?,
        "tab.rename" => {
            let tab = text("tab_id")?;
            let (session, window) = ids(&tab)?;
            core.call("session.set_window_label", json!({"session_id": session, "window_id": window, "label": text("label")?})).await?
        }
        "tab.move" => {
            let tab = text("tab_id")?;
            let (session, window) = ids(&tab)?;
            let view = view(core, session).await?;
            let windows = view["windows"].as_array().map(Vec::as_slice).unwrap_or_default();
            // `slot` as herdr takes it: a place in the list before the move, 0 first, the tab count last
            let before = windows.get(p["insert_index"].as_u64().unwrap_or(0) as usize).map(|w| text_of(&w["window_id"]));
            if before == Some(window) {
                return Ok(json!({}));
            }
            core.call("session.move_window", json!({"session_id": session, "window_id": window, "before_window_id": before})).await?
        }
        "workspace.move_block" => {
            let sessions: Vec<String> = serde_json::from_value(p["workspace_ids"].clone())?;
            for session in sessions {
                core.call("session.move", json!({"session_id": session, "before_session_id": p["before_workspace_id"]})).await?;
            }
            json!({})
        }
        "tab.focus" => {
            let tab = text("tab_id")?;
            let (session, window) = ids(&tab)?;
            STATE.lock().unwrap().focused = Some(session.to_string());
            core.call("session.focus_window", json!({"session_id": session, "window_id": window})).await?;
            news();
            json!({})
        }
        "pane.focus" => {
            let pane = text("pane_id")?;
            let (session, block) = ids(&pane)?;
            {
                let mut state = STATE.lock().unwrap();
                state.focused = Some(session.to_string());
                if let Some(turn) = state.blocks.get(block).and_then(|t| t["turn"].as_u64()) {
                    state.seen.insert(block.to_string(), turn);
                }
            }
            core.call("session.focus_block", json!({"session_id": session, "block_id": block})).await?;
            news();
            json!({})
        }
        "pane.focus_direction" => {
            let pane = text("pane_id")?;
            core.call("session.focus_direction", json!({"session_id": ids(&pane)?.0, "direction": text("direction")?})).await?
        }
        "pane.zoom" => {
            let pane = text("pane_id")?;
            let (session, block) = ids(&pane)?;
            core.call("session.zoom_block", json!({"session_id": session, "block_id": block})).await?
        }
        "pane.swap" => {
            let (a, b) = (text("source_pane_id")?, text("target_pane_id")?);
            let ((session, a), (other, b)) = (ids(&a)?, ids(&b)?);
            if session != other {
                bail!("Rex swaps panes only within a session");
            }
            core.call("session.swap_blocks", json!({"session_id": session, "block_id": a, "other_block_id": b})).await?
        }
        "pane.move" => move_pane(core, &text("pane_id")?, &p["destination"]).await?,
        "layout.set_split_ratio" => {
            let tab = text("tab_id")?;
            let (session, window) = ids(&tab)?;
            let path: Vec<bool> = serde_json::from_value(p["path"].clone())?;
            let view = view(core, session).await?;
            let found = window_of(&view, window).and_then(|w| border(tree(w), &path));
            let (direction, before, after) = found.ok_or_else(|| anyhow!("no split there"))?;
            core.call("session.resize_pane", json!({"session_id": session, "direction": direction, "before_block_id": before, "after_block_id": after, "ratio": p["ratio"]})).await?
        }
        "worktree.list" => {
            let tree = checkout(core, &text("workspace_id")?).await?;
            let open: HashMap<String, String> = {
                let state = STATE.lock().unwrap();
                state.checkouts.iter().filter_map(|(s, t)| Some((t.as_ref()?.checkout_path.clone(), s.clone()))).collect()
            };
            tokio::task::spawn_blocking(move || worktree::list(&tree.checkout_path, |path| open.get(path).cloned())).await??
        }
        "worktree.create" => {
            let branch = text("branch")?;
            let source = checkout(core, &text("workspace_id")?).await?;
            let from = source.clone();
            let path = tokio::task::spawn_blocking(move || worktree::create(&from, &branch)).await??;
            let opened = open(core, Some(&path)).await?;
            worktree::created(&source, &path);
            opened
        }
        "worktree.open" => open(core, Some(&text("path")?)).await?,
        "worktree.remove" => {
            let session = text("workspace_id")?;
            let tree = STATE.lock().unwrap().checkouts.get(&session).cloned().flatten().filter(|t| t.is_linked_worktree);
            let tree = tree.ok_or_else(|| anyhow!("not_linked_worktree: {session}"))?;
            let (gone, force) = (tree.clone(), p["force"].as_bool().unwrap_or(false));
            tokio::task::spawn_blocking(move || worktree::remove(&gone, force)).await??;
            core.call("session.destroy", json!({"session_id": session})).await?;
            worktree::removed(&tree);
            json!({})
        }
        // plugins, popups and herdr's own commands: what Rex has no word for
        _ => bail!("Rex has no {method}"),
    })
}

fn news() {
    NEWS.send_modify(|n| *n += 1);
}

/// The shell a pane's program runs under: the outermost process past `login`, the foreground one at a prompt.
fn shell_pid(process: &Value) -> Value {
    let ancestors = process["ancestors"].as_array().map(Vec::as_slice).unwrap_or_default();
    ancestors.iter().rev().find(|p| p["name"] != "login").map_or(&process["foreground"]["pid"], |shell| &shell["pid"]).clone()
}

fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.trim_end().lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// herdr's `pane.move`. Rex places a block only beside another, and within its session.
async fn move_pane(core: &Control, pane: &str, to: &Value) -> anyhow::Result<Value> {
    let (session, block) = ids(pane)?;
    let across = || anyhow!("Rex moves a pane only within its session");
    let args = |anchor: &str, down: bool| json!({"session_id": session, "block_id": block, "anchor_block_id": anchor, "direction": if down { "vertical" } else { "horizontal" }, "side": "after"});
    let tab = if to["type"] == "new_tab" {
        if to["workspace_id"].as_str().is_some_and(|w| w != session) {
            return Err(across());
        }
        // a new window has to start with a block of its own, which holds the place and then goes
        let made = core.call("session.new_window", json!({"session_id": session, "layout": shell(None), "focus": false})).await?;
        let holder = text_of(&made["block_ids"][0]);
        core.call("session.move_block", args(holder, false)).await?;
        core.call("block.close", json!({"session_id": session, "block_id": holder})).await?;
        format!("{session}/{}", text_of(&made["window_id"]))
    } else {
        let tab = text_of(&to["tab_id"]);
        let (other, window) = ids(tab)?;
        if other != session {
            return Err(across());
        }
        let anchor = match to["target_pane_id"].as_str() {
            Some(target) => ids(target)?.1.to_string(),
            None => {
                let view = view(core, session).await?;
                text_of(&window_of(&view, window).ok_or_else(|| anyhow!("no tab {tab}"))?["focused_block_id"]).to_string()
            }
        };
        core.call("session.move_block", args(&anchor, to["split"] == "down")).await?;
        tab.to_string()
    };
    Ok(json!({"move_result": {"pane": {"tab_id": tab}}}))
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
    windows.iter().flat_map(|w| frames(tree(w)).0).map(|(block, _)| block).collect()
}

type Frame = (String, Value);

/// A tree's panes and splits, as herdr lists them. A split's id is its path, which [`crate::model::split_path`]
/// reads back and `layout.set_split_ratio` brings here.
fn frames(tree: &Value) -> (Vec<Frame>, Vec<Value>) {
    fn walk(node: &Value, path: &str, panes: &mut Vec<Frame>, splits: &mut Vec<Value>) {
        if let Some(block) = node["block_id"].as_str() {
            return panes.push((block.to_string(), rect(&node["rect"])));
        }
        let split = &node["split"];
        if split.is_null() {
            return; // an empty layout
        }
        let id = format!("split_0_{}", if path.is_empty() { "root" } else { path });
        let direction = if split["direction"] == "horizontal" { "right" } else { "down" };
        splits.push(json!({"id": id, "direction": direction, "ratio": split["ratio"], "rect": rect(&node["rect"])}));
        walk(&split["before"], &format!("{path}0"), panes, splits);
        walk(&split["after"], &format!("{path}1"), panes, splits);
    }
    let (mut panes, mut splits) = (Vec::new(), Vec::new());
    walk(tree, "", &mut panes, &mut splits);
    (panes, splits)
}

/// `[x, y, w, h]` fractions on herdr's grid. Rounded as edges, so neighbours meet.
fn rect(fractions: &Value) -> Value {
    let at = |i: usize| fractions[i].as_f64().unwrap_or(0.);
    let grid = |f: f64| (f * f64::from(AREA)).round() as u16;
    let (x, y) = (grid(at(0)), grid(at(1)));
    json!({"x": x, "y": y, "width": grid(at(0) + at(2)) - x, "height": grid(at(1) + at(3)) - y})
}

/// The block a window's keys go to, of those Shepherd shows: a floating one's are the layout's.
fn focused_block<'a>(window: &'a Value, frames: &'a [Frame]) -> &'a str {
    let focused = text_of(&window["focused_block_id"]);
    if frames.iter().any(|(block, _)| block == focused) { focused } else { frames.first().map_or("", |(block, _)| block) }
}

/// A window's name, unless it is the one Rex made from its id: then its place, as a tab of herdr's.
fn tab_label(window: &Value, number: usize) -> String {
    let (label, id) = (text_of(&window["label"]), text_of(&window["window_id"]));
    let made = label.strip_prefix("Window ").is_some_and(|hex| id.replace('-', "").contains(hex));
    if label.is_empty() || made { number.to_string() } else { label.to_string() }
}

/// `session.snapshot`'s result: the sessions' views with what is known of their blocks.
fn snapshot(views: &[Value], state: &State) -> Value {
    let (mut workspaces, mut tabs, mut panes, mut layouts) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let shown = views.iter().filter(|v| !v["windows"].as_array().is_none_or(Vec::is_empty));
    let shown: Vec<&Value> = shown.collect();
    for (w_at, view) in shown.iter().enumerate() {
        let session = text_of(&view["session_id"]);
        let mut w_statuses = Vec::new();
        for (t_at, window) in view["windows"].as_array().into_iter().flatten().enumerate() {
            let tab = format!("{session}/{}", text_of(&window["window_id"]));
            let (frames, splits) = frames(tree(window));
            let focused = focused_block(window, &frames);
            let mut t_statuses = Vec::new();
            let mut laid = Vec::new();
            for (block, rect) in &frames {
                let t = state.blocks.get(block).unwrap_or(&Value::Null);
                let (agent, status) = (agent(t), status(t, &state.seen));
                if agent.is_some() {
                    t_statuses.push(status);
                }
                let pane = format!("{session}/{block}");
                panes.push(json!({
                    "pane_id": pane, "tab_id": tab, "workspace_id": session, "terminal_id": pane,
                    "agent_status": status, "agent": agent,
                    "terminal_title_stripped": t["title"].as_str().filter(|t| !t.is_empty()).map(plain_title),
                    "cwd": t["cwd"], "foreground_cwd": t["cwd"],
                }));
                laid.push(json!({"pane_id": pane, "focused": block == focused, "rect": rect}));
            }
            let status = loudest(t_statuses.iter().copied());
            tabs.push(json!({"tab_id": tab, "workspace_id": session, "label": tab_label(window, t_at + 1), "number": t_at + 1, "agent_status": status}));
            w_statuses.push(status);
            layouts.push(json!({
                "tab_id": tab, "zoomed": window["zoomed_block_id"].is_string(), "focused_pane_id": format!("{session}/{focused}"),
                "area": {"x": 0, "y": 0, "width": AREA, "height": AREA}, "panes": laid, "splits": splits,
            }));
        }
        workspaces.push(json!({
            "workspace_id": session, "label": text_of(&view["label"]), "number": w_at + 1,
            "active_tab_id": format!("{session}/{}", text_of(&view["active_window_id"])),
            "agent_status": loudest(w_statuses.iter().copied()), "worktree": state.checkouts.get(session).cloned().flatten(),
        }));
    }
    let focused = shown.iter().find(|v| state.focused.as_deref() == v["session_id"].as_str()).or(shown.first());
    let window = focused.and_then(|v| window_of(v, v["active_window_id"].as_str()?));
    let session = focused.map_or("", |v| text_of(&v["session_id"]));
    json!({
        "focused_workspace_id": focused.map(|v| &v["session_id"]),
        "focused_tab_id": focused.map(|v| format!("{session}/{}", text_of(&v["active_window_id"]))),
        "focused_pane_id": window.map(|w| format!("{session}/{}", focused_block(w, &frames(tree(w)).0))),
        "workspaces": workspaces, "tabs": tabs, "panes": panes, "layouts": layouts,
    })
}

/// The status a terminal reports, as [`status`] reads it.
fn reported(record: &Value) -> Value {
    json!({"state": record["state"], "app": record.get("effective_app").unwrap_or(&record["app"])})
}

/// Asks what a block's events won't say: all of it once, then what it runs when that changes.
async fn learn(core: &Control, session: &str, block: &str) {
    let known = STATE.lock().unwrap().blocks.get(block).cloned();
    if known.as_ref().is_some_and(|t| t["stale"] != true) {
        return;
    }
    let pane = format!("{session}/{block}");
    let Ok(process) = on_block(core, &pane, "process", json!({})).await else { return };
    let fg = &process["foreground"];
    let mut t = match known {
        Some(t) => t,
        None => {
            let title = on_block(core, &pane, "title", json!({})).await.unwrap_or_default();
            let records = on_block(core, &pane, "program_status", json!({})).await.unwrap_or_default();
            let status = records["records"].get(0).map_or(Value::Null, reported);
            json!({"address": block, "title": title["title"], "status": status, "turn": 0})
        }
    };
    t["cwd"] = fg["cwd"].clone();
    t["foreground"] = json!({"name": fg["name"], "argv": [fg.get("argv0").unwrap_or(&fg["name"])]});
    t["stale"] = json!(false);
    STATE.lock().unwrap().blocks.insert(block.to_string(), t);
}

/// Asks git for the checkout of a session not seen yet, from where its first pane is.
async fn find_checkout(session: &str, view: &Value) {
    let first = blocks(view).into_iter().next();
    let cwd = {
        let state = STATE.lock().unwrap();
        if state.checkouts.contains_key(session) {
            return;
        }
        first.and_then(|b| state.blocks.get(&b)?["cwd"].as_str().map(str::to_string))
    };
    // none yet: its shell has only just started, and the next snapshot asks again
    let Some(cwd) = cwd else { return };
    let Ok(tree) = tokio::task::spawn_blocking(move || worktree::of(&cwd)).await else { return };
    STATE.lock().unwrap().checkouts.insert(session.to_string(), tree);
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
                    let block = text_of(&fields["block_id"]);
                    state.blocks.remove(block);
                    state.seen.remove(block);
                    false // its view changes too, if it was in one
                }
                Some("block_event") => state.blocks.get_mut(text_of(&fields["block_id"])).is_some_and(|t| block_event(t, text_of(&fields["name"]), &fields["payload"])),
                _ => false,
            }
        }
        _ => false,
    }
}

/// One block's event on what is known of it: whether a snapshot would show it.
fn block_event(t: &mut Value, name: &str, fields: &Value) -> bool {
    match name {
        // the `title` of an agent spinning changes a few times a second, its words don't
        "title_changed" => {
            let new = text_of(&fields["title"]);
            let news = plain_title(text_of(&t["title"])) != plain_title(new);
            t["title"] = json!(new);
            news
        }
        "pwd_changed" | "process_changed" => {
            t["stale"] = json!(true);
            true
        }
        "program_status_changed" if fields["id"] == "" => {
            t["status"] = reported(&fields["record"]);
            t["turn"] = json!(t["turn"].as_u64().unwrap_or(0) + 1);
            true
        }
        "program_status_removed" if fields["ids"].as_array().is_some_and(|ids| ids.iter().any(|id| id == "")) => {
            t["status"] = Value::Null;
            true
        }
        _ => false,
    }
}

/// The server's events that change a snapshot, which is all herdr's event stream tells the window. Ends with the
/// connection, or when the receiver goes.
pub async fn subscribe() -> anyhow::Result<mpsc::UnboundedReceiver<Value>> {
    let core = connected().await?;
    let (tx, rx) = mpsc::unbounded_channel();
    let mut news = NEWS.subscribe();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                changed = news.changed() => if changed.is_err() { break },
                _ = tx.closed() => break,
            }
            if core.lost() || tx.send(Value::Null).is_err() {
                break;
            }
        }
    });
    Ok(rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AgentStatus, Snapshot};

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
        let agent = json!({"address": "block:b", "title": "⠂ fixing it", "cwd": "/repo", "turn": 2, "foreground": {"name": "node", "argv": ["claude"]}, "status": {"state": "working", "app": "claude-code"}});
        state.blocks.insert("block:b".into(), agent);
        state.blocks.insert("block:a".into(), json!({"address": "block:a", "title": "", "cwd": "/repo/src", "foreground": {"name": "zsh", "argv": ["-zsh"]}}));
        state.checkouts.insert("session:3".into(), Some(Worktree { repo_key: "/r/.git".into(), is_linked_worktree: false, checkout_path: "/r".into() }));
        state
    }

    #[test]
    fn the_window_reads_rexs_sessions_as_herdrs() {
        let s = Snapshot::from_result(&json!({"snapshot": snapshot(&views(), &state())})).unwrap();
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
        let s = Snapshot::from_result(&json!({"snapshot": snapshot(&zoomed, &state())})).unwrap();
        assert_eq!(s.pane_frames("session:1/window:2").len(), 1, "the zoomed one fills it");
    }

    #[test]
    fn a_floating_block_leaves_the_keys_to_the_layout() {
        let mut floating = views();
        floating[0]["windows"][1]["focused_block_id"] = json!("block:float");
        let s = Snapshot::from_result(&json!({"snapshot": snapshot(&floating, &state())})).unwrap();
        assert_eq!(s.focused_pane_id.as_deref(), Some("session:1/block:a"));
    }

    #[test]
    fn focusing_a_session_shows_it() {
        let mut state = state();
        state.focused = Some("session:3".into());
        let s = Snapshot::from_result(&json!({"snapshot": snapshot(&views(), &state)})).unwrap();
        assert_eq!(s.focused_workspace_id.as_deref(), Some("session:3"));
        assert_eq!(s.focused_pane_id.as_deref(), Some("session:3/block:d"));
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
        assert_eq!(a["width"].as_u64().unwrap() as u16 + b["width"].as_u64().unwrap() as u16, AREA);
        assert_eq!(a["width"], b["x"]);
    }

    #[test]
    fn events_keep_a_block_up_to_date() {
        let mut state = state();
        let on = |name: &str, payload: Value| json!({"type": "session_event", "payload": {"session_id": "session:1", "event": {"type": "block_event", "payload": {"block_id": "block:b", "name": name, "payload": payload}}}});
        assert!(!heard(&on("title_changed", json!({"title": "⠐ fixing it"})), &mut state), "a spinning title");
        assert!(heard(&on("title_changed", json!({"title": "⠂ done"})), &mut state));
        assert!(heard(&on("process_changed", json!({})), &mut state));
        assert_eq!(state.blocks["block:b"]["stale"], true, "the next snapshot asks");
        assert!(heard(&on("program_status_changed", json!({"id": "", "record": {"state": "done", "effective_app": "pi"}})), &mut state));
        assert_eq!((&state.blocks["block:b"]["status"]["app"], &state.blocks["block:b"]["turn"]), (&json!("pi"), &json!(3)));
        assert!(!heard(&on("program_status_changed", json!({"id": "build/test", "record": {"state": "working"}})), &mut state), "a part of the program's work");
        assert!(heard(&on("program_status_removed", json!({"ids": [""]})), &mut state));
        assert!(state.blocks["block:b"]["status"].is_null());
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
    async fn until(what: &str, mut done: impl FnMut(&Snapshot) -> bool) -> Snapshot {
        for _ in 0..100 {
            let shot = Snapshot::from_result(&call("session.snapshot", json!({})).await.unwrap()).unwrap();
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
        let mut events = subscribe().await.unwrap();
        let before = until("a snapshot", |_| true).await.workspaces.len();
        let repo = tempfile::tempdir().unwrap();
        let repo = std::fs::canonicalize(repo.path()).unwrap().to_string_lossy().into_owned();
        let made = call("workspace.create", json!({"cwd": repo})).await.unwrap();
        let (workspace, first) = (text_of(&made["workspace"]["workspace_id"]).to_string(), text_of(&made["root_pane"]["pane_id"]).to_string());
        assert!(tokio::time::timeout(Duration::from_secs(2), events.recv()).await.unwrap().is_some(), "the server said so");

        let second = call("pane.split", json!({"target_pane_id": first, "direction": "right"})).await.unwrap();
        let second = text_of(&second["pane"]["pane_id"]).to_string();
        call("tab.focus", json!({"tab_id": made["tab"]["tab_id"]})).await.unwrap();
        let shot = until("two panes", |s| s.panes.iter().filter(|p| p.workspace_id == workspace).count() == 2).await;
        assert_eq!(shot.workspaces.len(), before + 1);
        assert_eq!(shot.focused_workspace_id.as_deref(), Some(workspace.as_str()));
        assert_eq!(shot.focused_pane_id.as_deref(), Some(second.as_str()), "a split takes the focus");
        let tab = text_of(&made["tab"]["tab_id"]).to_string();
        let frames = shot.pane_frames(&tab);
        assert_eq!((frames.len(), shot.split_frames(&tab).len()), (2, 1));

        // what a pane does: a size, a snapshot of what is there, and a stream
        let pane = first.clone();
        let (attached, mut stream) = tokio::task::spawn_blocking(move || crate::rex::attach(&pane, 90, 20, (9, 18), false)).await.unwrap().unwrap();
        attached.input(b"seq 1 200; echo shepherd-$((40+2))\r");
        let mut emulator = crate::terminal::emulator::Emulator::raw(90, 20);
        let shown = |e: &crate::terminal::emulator::Emulator| (0..e.rows()).any(|row| e.row_text(row).trim() == "shepherd-42");
        let printed = tokio::time::timeout(Duration::from_secs(10), async {
            use futures::StreamExt;
            while let Some(event) = stream.next().await {
                match event {
                    crate::herdr::SessionEvent::Snapshot(bytes) => emulator = crate::terminal::emulator::Emulator::restored(&bytes, 90, 20).unwrap(),
                    crate::herdr::SessionEvent::Frame(bytes) => {
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
        let core = connected().await.unwrap();
        let size = on_block(&core, &first, "size", json!({})).await.unwrap();
        assert_eq!((size["columns"].as_u64(), size["rows"].as_u64()), (Some(90), Some(20)), "the pane's size is the terminal's");
        let read = call("pane.read", json!({"pane_id": first, "lines": 5})).await.unwrap();
        assert!(text_of(&read["read"]["text"]).contains("shepherd-42"));

        // a second look at the same block, as a second window has: the server replays it to it
        let pane = first.clone();
        let (second_look, mut replay) = tokio::task::spawn_blocking(move || crate::rex::attach(&pane, 90, 20, (9, 18), true)).await.unwrap().unwrap();
        let again = tokio::time::timeout(Duration::from_secs(5), async {
            use futures::StreamExt;
            while let Some(event) = replay.next().await {
                if let crate::herdr::SessionEvent::Snapshot(bytes) = event {
                    return crate::terminal::emulator::Emulator::restored(&bytes, 90, 20).unwrap().history_lines();
                }
            }
            0
        });
        assert!(again.await.unwrap() > 150, "a snapshot holds the history");
        drop(second_look);

        // an agent's status, over OSC 7501 as Claude Code reports it
        attached.input(b"printf '\\033]7501;state=working:app=claude-code\\007'\r");
        let agent = until("a reported status", |s| s.panes.iter().any(|p| p.pane_id == first && p.agent.as_deref() == Some("claude"))).await;
        assert_eq!(agent.panes.iter().find(|p| p.pane_id == first).unwrap().agent_status, AgentStatus::Working);
        drop(attached);

        let tab2 = call("tab.create", json!({"workspace_id": workspace})).await.unwrap();
        let tab2_id = text_of(&tab2["tab"]["tab_id"]).to_string();
        call("tab.rename", json!({"tab_id": tab2_id, "label": "logs"})).await.unwrap();
        call("tab.move", json!({"tab_id": tab2_id, "insert_index": 0})).await.unwrap();
        let shot = until("a renamed tab", |s| s.tabs.iter().any(|t| t.label == "logs")).await;
        assert_eq!(shot.tabs_of(&workspace)[0].label, "logs");
        call("workspace.rename", json!({"workspace_id": workspace, "label": "renamed"})).await.unwrap();
        call("pane.swap", json!({"source_pane_id": first, "target_pane_id": second})).await.unwrap();
        call("layout.set_split_ratio", json!({"tab_id": tab, "path": [], "ratio": 0.25})).await.unwrap();
        call("pane.zoom", json!({"pane_id": second})).await.unwrap();
        let shot = until("a zoom", |s| s.layouts.iter().any(|l| l.tab_id == tab && l.zoomed)).await;
        assert_eq!(shot.workspaces.iter().find(|w| w.workspace_id == workspace).unwrap().label, "renamed");
        call("pane.zoom", json!({"pane_id": second})).await.unwrap();
        let moved = call("pane.move", json!({"pane_id": second, "destination": {"type": "new_tab"}})).await.unwrap();
        let moved = text_of(&moved["move_result"]["pane"]["tab_id"]).to_string();
        let shot = until("a pane in its own tab", |s| s.panes.iter().any(|p| p.pane_id == second && p.tab_id == moved)).await;
        assert_eq!(shot.pane_frames(&moved).len(), 1, "the shell that held its place is gone");
        assert!(call("pane.move", json!({"pane_id": second, "destination": {"type": "new_tab", "workspace_id": "session:other"}})).await.is_err());

        call("pane.close", json!({"pane_id": first})).await.unwrap();
        until("a closed pane", |s| !s.panes.iter().any(|p| p.pane_id == first)).await;
        call("tab.close", json!({"tab_id": tab2_id})).await.unwrap();
        until("a closed tab", |s| !s.tabs.iter().any(|t| t.tab_id == tab2_id)).await;
        call("workspace.close", json!({"workspace_id": workspace})).await.unwrap();
        until("a closed workspace", |s| s.workspaces.len() == before).await;

        // a worktree, in the real `~/.herdr/worktrees/<a temporary repo>`, which goes at the end
        for args in [&["init", "-q"][..], &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "first"]] {
            assert!(std::process::Command::new("git").arg("-C").arg(&repo).args(args).status().unwrap().success());
        }
        let main = call("workspace.create", json!({"cwd": repo})).await.unwrap();
        let main = text_of(&main["workspace"]["workspace_id"]).to_string();
        let made = call("worktree.create", json!({"workspace_id": main, "branch": "fix/rex", "focus": false})).await.unwrap();
        assert!(crate::commands::created_tab_id(&made).is_some(), "the window selects its tab");
        let linked = text_of(&made["workspace"]["workspace_id"]).to_string();
        let shot = until("a nested worktree", |s| s.workspace_tree().iter().any(|(w, child)| w.workspace_id == linked && child.is_some())).await;
        let tree = shot.workspaces.iter().find(|w| w.workspace_id == linked).unwrap().worktree.clone().unwrap();
        assert!(tree.is_linked_worktree);
        assert_eq!(shot.workspaces.iter().find(|w| w.workspace_id == linked).unwrap().label, "fix-rex");
        let listed = call("worktree.list", json!({"workspace_id": main})).await.unwrap();
        let open: Vec<_> = listed["worktrees"].as_array().unwrap().iter().map(|w| text_of(&w["open_workspace_id"])).collect();
        assert_eq!(open, [main.as_str(), linked.as_str()]);
        std::fs::write(std::path::Path::new(&tree.checkout_path).join("work"), "unsaved").unwrap();
        let refused = call("worktree.remove", json!({"workspace_id": linked, "force": false})).await.unwrap_err().to_string();
        assert!(refused.starts_with("dirty_worktree_requires_force"), "{refused}");
        call("worktree.remove", json!({"workspace_id": linked, "force": true})).await.unwrap();
        until("a removed worktree", |s| !s.workspaces.iter().any(|w| w.workspace_id == linked)).await;
        assert!(!std::path::Path::new(&tree.checkout_path).exists());
        call("workspace.close", json!({"workspace_id": main})).await.unwrap();
        std::fs::remove_dir(std::path::Path::new(&tree.checkout_path).parent().unwrap()).unwrap();
    }
}
