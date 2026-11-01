//! herdr's socket API as Shepherd calls it, answered from herdr-core and the layout document, so the window runs on
//! either backend unchanged: [`crate::herdr::Client`] comes here instead of herdr's socket.
use super::conn::{Core, Item, Sink};
use super::tree::{AREA, Doc};
use crate::model::Worktree;
use crate::worktree;
use anyhow::{anyhow, bail};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, watch};

/// The core document the layout lives in. The Herdr app keeps its own beside it; the two don't share spaces.
const DOC: &str = "shepherd";
/// Marks the terminals Shepherd started: the core is shared, and others' terminals (the Herdr app's) are not its own.
const LABEL: &str = "shepherd";
/// Agents known by their program's name, for one that does not report itself over OSC 7501.
const AGENTS: [&str; 9] = ["claude", "codex", "pi", "opencode", "gemini", "amp", "droid", "copilot", "kimi"];

/// One layout change at a time: a snapshot's pruning never sees a terminal started and not yet placed.
static LAYOUT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
/// Counts saved edits: they are Shepherd's own, so no core event says to read the layout again.
static EDITS: LazyLock<watch::Sender<u64>> = LazyLock::new(|| watch::channel(0).0);
/// The turn each finished agent was looked at in, which reads as idle after that (herdr's done → idle).
// ponytail: in memory, so a restart shows finished agents as done again; a doc field if that bites
static SEEN: LazyLock<Mutex<HashMap<String, u64>>> = LazyLock::new(Default::default);

/// One request, herdr's method and params, herdr's result shape.
pub async fn call(method: &str, params: Value) -> anyhow::Result<Value> {
    let core = super::connected().await?;
    let text = |key: &str| params[key].as_str().map(str::to_string).ok_or_else(|| anyhow!("{method}: no {key}"));
    Ok(match method {
        "ping" => json!({"type": "pong"}),
        "pane.get" => {
            let t = terminal(&core, &text("pane_id")?).await?;
            json!({"pane": {"pane_id": t["address"], "cwd": t["cwd"], "foreground_cwd": t["foreground"]["cwd"]}})
        }
        "pane.read" => {
            let lines = params["lines"].as_u64().unwrap_or(0);
            let read = core.call("terminal.read", json!({"terminal": text("pane_id")?, "lines": lines})).await?;
            json!({"read": {"text": read["text"]}})
        }
        "pane.process_info" => {
            let t = terminal(&core, &text("pane_id")?).await?;
            let fg = &t["foreground"];
            json!({"process_info": {"foreground_process_group_id": fg["pid"], "shell_pid": t["pid"], "foreground_processes": [{"pid": fg["pid"], "name": fg["name"]}]}})
        }
        "pane.send_input" => {
            let pane = text("pane_id")?;
            match params["text"].as_str() {
                Some(text) => core.call("terminal.send", json!({"terminal": pane, "text": text, "force": true})).await?,
                None => core.call("terminal.keys", json!({"terminal": pane, "keys": params["keys"]})).await?,
            }
        }
        // ponytail: a fixed wait for the prompt of the shell just started, which only the editor's split asks for;
        // `terminal.wait --output` matches only what comes after it starts
        "pane.wait_for_output" => {
            tokio::time::sleep(Duration::from_millis(400)).await;
            json!({})
        }
        // git can take a while: the layout waits only for what comes after it
        "worktree.list" => {
            let (doc, _) = load(&core).await?;
            let tree = checkout(&core, &doc, &text("workspace_id")?).await?;
            tokio::task::spawn_blocking(move || worktree::list(&tree.checkout_path, |path| doc.workspace_at(path))).await??
        }
        "worktree.create" => {
            let branch = text("branch")?;
            let (doc, _) = load(&core).await?;
            let source = checkout(&core, &doc, &text("workspace_id")?).await?;
            let from = source.clone();
            let path = tokio::task::spawn_blocking(move || worktree::create(&from, &branch)).await??;
            let _layout = LAYOUT.lock().await;
            let opened = open(&core, &path, folder(&path)).await?;
            worktree::created(&source, &path);
            opened
        }
        "worktree.open" => {
            let path = text("path")?;
            let _layout = LAYOUT.lock().await;
            open(&core, &path, folder(&path)).await?
        }
        "worktree.remove" => {
            let id = text("workspace_id")?;
            let (doc, _) = load(&core).await?;
            let tree = doc.workspaces.iter().find(|w| w.id == id).and_then(|w| w.worktree.clone()).filter(|t| t.is_linked_worktree);
            let tree = tree.ok_or_else(|| anyhow!("not_linked_worktree: {id}"))?;
            let (gone, force) = (tree.clone(), params["force"].as_bool().unwrap_or(false));
            tokio::task::spawn_blocking(move || worktree::remove(&gone, force)).await??;
            let _layout = LAYOUT.lock().await;
            let terminals = edit(&core, |d| d.close_workspace(&id)).await?;
            ended(&core, &terminals).await;
            worktree::removed(&tree);
            json!({})
        }
        _ => {
            let _layout = LAYOUT.lock().await;
            layout_call(&core, method, &params).await?
        }
    })
}

async fn layout_call(core: &Core, method: &str, p: &Value) -> anyhow::Result<Value> {
    let text = |key: &str| p[key].as_str().map(str::to_string).ok_or_else(|| anyhow!("{method}: no {key}"));
    Ok(match method {
        "session.snapshot" => {
            let (mut doc, version) = load(core).await?;
            let terminals = list(core).await?;
            let gone = doc.retain(|t| terminals.iter().any(|l| l["address"] == t));
            if !gone.is_empty() {
                save(core, &doc, version).await?;
                ended(core, &gone).await; // already gone: this only clears their records
            }
            json!({"snapshot": snapshot(&doc, &terminals, &SEEN.lock().unwrap())})
        }
        "workspace.create" => {
            let cwd = p["cwd"].as_str().map_or_else(|| crate::settings::home().to_string_lossy().into_owned(), str::to_string);
            open(core, &cwd, "").await?
        }
        "tab.create" => {
            let workspace = text("workspace_id")?;
            let (doc, _) = load(core).await?;
            let terminal = run(core, &workspace_cwd(core, &doc, &workspace).await).await?;
            let Some(tab) = edit(core, |d| d.add_tab(&workspace, terminal.clone())).await? else {
                ended(core, std::slice::from_ref(&terminal)).await;
                bail!("no workspace {workspace}");
            };
            json!({"tab": {"tab_id": tab}, "root_pane": {"pane_id": terminal}})
        }
        "pane.split" => {
            let target = text("target_pane_id")?;
            let cwd = match p["cwd"].as_str() {
                Some(cwd) => cwd.to_string(),
                None => cwd_of(core, Some(&target)).await,
            };
            let terminal = run(core, &cwd).await?;
            let (down, focus) = (p["direction"] == "down", p["focus"].as_bool().unwrap_or(true));
            if !edit(core, |d| d.split(&target, &terminal, down, focus)).await? {
                ended(core, std::slice::from_ref(&terminal)).await;
                bail!("no pane {target}");
            }
            json!({"pane": {"pane_id": terminal}})
        }
        "workspace.close" => {
            let id = text("workspace_id")?;
            let terminals = edit(core, |d| d.close_workspace(&id)).await?;
            ended(core, &terminals).await;
            json!({})
        }
        "tab.close" => {
            let id = text("tab_id")?;
            let terminals = edit(core, |d| d.close_tab(&id)).await?;
            ended(core, &terminals).await;
            json!({})
        }
        "pane.close" => {
            let pane = text("pane_id")?;
            edit(core, |d| d.remove(&pane)).await?;
            ended(core, std::slice::from_ref(&pane)).await;
            json!({})
        }
        "workspace.rename" => {
            let (id, label) = (text("workspace_id")?, text("label")?);
            edit(core, |d| d.rename_workspace(&id, &label)).await?;
            json!({})
        }
        "tab.rename" => {
            let (id, label) = (text("tab_id")?, text("label")?);
            edit(core, |d| d.rename_tab(&id, &label)).await?;
            json!({})
        }
        "tab.move" => {
            let (id, slot) = (text("tab_id")?, p["insert_index"].as_u64().unwrap_or(0) as usize);
            edit(core, |d| d.move_tab(&id, slot)).await?;
            json!({})
        }
        "workspace.move_block" => {
            let ids: Vec<String> = serde_json::from_value(p["workspace_ids"].clone())?;
            edit(core, |d| d.move_workspaces(&ids, p["before_workspace_id"].as_str())).await?;
            json!({})
        }
        "tab.focus" => {
            let id = text("tab_id")?;
            edit(core, |d| d.focus_tab(&id)).await?;
            json!({})
        }
        "pane.focus" => {
            let pane = text("pane_id")?;
            let turn = terminal(core, &pane).await?["turn"].as_u64().unwrap_or(0);
            SEEN.lock().unwrap().insert(pane.clone(), turn);
            edit(core, |d| d.focus_pane(&pane)).await?;
            json!({})
        }
        "pane.focus_direction" => {
            let (pane, direction) = (text("pane_id")?, text("direction")?);
            edit(core, |d| {
                if let Some(next) = d.neighbour(&pane, &direction) {
                    d.focus_pane(&next);
                }
            })
            .await?;
            json!({})
        }
        "pane.zoom" => {
            let pane = text("pane_id")?;
            edit(core, |d| d.toggle_zoom(&pane)).await?;
            json!({})
        }
        "pane.swap" => {
            let (a, b) = (text("source_pane_id")?, text("target_pane_id")?);
            edit(core, |d| d.swap(&a, &b)).await?;
            json!({})
        }
        "pane.move" => {
            let pane = text("pane_id")?;
            let to = &p["destination"];
            let tab = match to["type"].as_str() {
                Some("new_tab") => edit(core, |d| d.pane_to_new_tab(&pane, to["workspace_id"].as_str())).await?,
                _ => {
                    let tab = to["tab_id"].as_str().unwrap_or_default();
                    edit(core, |d| d.move_pane(&pane, tab, to["target_pane_id"].as_str(), to["split"] == "down").then(|| tab.to_string())).await?
                }
            };
            json!({"move_result": {"pane": {"tab_id": tab}}})
        }
        "layout.set_split_ratio" => {
            let tab = text("tab_id")?;
            let path: Vec<bool> = serde_json::from_value(p["path"].clone())?;
            let ratio = p["ratio"].as_f64().unwrap_or(0.5) as f32;
            edit(core, |d| d.set_ratio(&tab, &path, ratio)).await?;
            json!({})
        }
        // plugins, popups and herdr's own commands: what the core has no word for
        _ => bail!("herdr-core has no {method}"),
    })
}

async fn load(core: &Core) -> anyhow::Result<(Doc, u64)> {
    let got = core.call("doc.get", json!({"name": DOC})).await?;
    let doc = if got["value"].is_null() { Doc::default() } else { serde_json::from_value(got["value"].clone())? };
    Ok((doc, got["version"].as_u64().unwrap_or(0)))
}

async fn save(core: &Core, doc: &Doc, version: u64) -> anyhow::Result<()> {
    core.call("doc.set", json!({"name": DOC, "value": doc, "version": version})).await?;
    EDITS.send_modify(|n| *n += 1);
    Ok(())
}

/// Applies `change` to the document as it is now, again if another writer (a second Shepherd) got there first.
async fn edit<T>(core: &Core, mut change: impl FnMut(&mut Doc) -> T) -> anyhow::Result<T> {
    loop {
        let (mut doc, version) = load(core).await?;
        let out = change(&mut doc);
        match save(core, &doc, version).await {
            Err(e) if e.to_string().starts_with("conflict") => continue,
            saved => return saved.map(|()| out),
        }
    }
}

async fn list(core: &Core) -> anyhow::Result<Vec<Value>> {
    let listed = core.call("terminal.list", json!({"labels": {LABEL: "1"}})).await?;
    Ok(listed["terminals"].as_array().cloned().unwrap_or_default())
}

async fn terminal(core: &Core, address: &str) -> anyhow::Result<Value> {
    core.call("terminal.get", json!({"terminal": address})).await
}

/// A new shell in `cwd`, kept through a crash or a reboot as the Herdr app's panes are; its address.
async fn run(core: &Core, cwd: &str) -> anyhow::Result<String> {
    let env = json!({"TERM": "xterm-256color", "COLORTERM": "truecolor", "TERM_PROGRAM": "Shepherd"});
    let run = core.call("terminal.run", json!({"cwd": cwd, "env": env, "labels": {LABEL: "1"}, "keep": true})).await?;
    run["address"].as_str().map(str::to_string).ok_or_else(|| anyhow!("terminal.run gave no address"))
}

/// Where `terminal`'s program is now: a new pane beside it starts there. Home without one.
async fn cwd_of(core: &Core, terminal_address: Option<&str>) -> String {
    let t = match terminal_address {
        Some(address) => terminal(core, address).await.unwrap_or_default(),
        None => Value::Null,
    };
    let cwd = t["foreground"]["cwd"].as_str().or(t["cwd"].as_str());
    cwd.map_or_else(|| crate::settings::home().to_string_lossy().into_owned(), str::to_string)
}

/// Where a workspace's focused pane is now.
async fn workspace_cwd(core: &Core, doc: &Doc, id: &str) -> String {
    let focused = doc.workspaces.iter().find(|w| w.id == id).and_then(|w| w.tabs.iter().find(|t| t.id == w.active_tab)).map(|t| t.focused.clone());
    cwd_of(core, focused.as_deref()).await
}

/// A new workspace in `dir`, on the checkout that is in, `label`led (empty: after its pane's folder as that changes).
async fn open(core: &Core, dir: &str, label: &str) -> anyhow::Result<Value> {
    let terminal = run(core, dir).await?;
    let found = dir.to_string();
    let tree = tokio::task::spawn_blocking(move || worktree::of(&found)).await?;
    let (workspace, tab) = edit(core, |d| {
        let (workspace, tab) = d.add_workspace(terminal.clone());
        d.set_worktree(&workspace, tree.clone());
        d.rename_workspace(&workspace, label);
        (workspace, tab)
    })
    .await?;
    Ok(json!({"workspace": {"workspace_id": workspace, "active_tab_id": tab}, "tab": {"tab_id": tab}, "root_pane": {"pane_id": terminal}}))
}

/// The checkout a workspace is on. One opened elsewhere may have `cd`ed into a repo: that is its checkout from now on.
async fn checkout(core: &Core, doc: &Doc, id: &str) -> anyhow::Result<Worktree> {
    if let Some(tree) = doc.workspaces.iter().find(|w| w.id == id).and_then(|w| w.worktree.clone()) {
        return Ok(tree);
    }
    let cwd = workspace_cwd(core, doc, id).await;
    let found = cwd.clone();
    let tree = tokio::task::spawn_blocking(move || worktree::of(&found)).await?.ok_or_else(|| anyhow!("not_git_worktree: {cwd}"))?;
    edit(core, |d| d.set_worktree(id, Some(tree.clone()))).await?;
    Ok(tree)
}

pub(crate) fn folder(path: &str) -> &str {
    path.rsplit('/').find(|p| !p.is_empty()).unwrap_or(path)
}

/// Ends `terminals` and forgets them. A finished one refuses the kill and takes the removal.
async fn ended(core: &Core, terminals: &[String]) {
    for t in terminals {
        let _ = core.call("terminal.kill", json!({"terminal": t})).await;
        let _ = core.call("terminal.rm", json!({"terminal": t})).await;
        SEEN.lock().unwrap().remove(t);
    }
}

/// A title without the spinner or glyph an agent keeps in front of it, which changes a few times a second.
pub(crate) fn plain_title(title: &str) -> &str {
    title.trim_start_matches(|c: char| !c.is_alphanumeric()).trim_end()
}

/// The agent a terminal runs, by the name Shepherd knows it by: what it reports itself as (`claude-code`), else a
/// known name in its foreground program's argv, else `agent` when it reports without a name.
pub(crate) fn agent(t: &Value) -> Option<String> {
    let reported = t["status"]["app"].as_str().filter(|a| !a.is_empty()).map(|a| {
        let a = a.to_lowercase();
        a.strip_suffix("-code").map_or_else(|| a.clone(), str::to_string)
    });
    let program = t["foreground"]["argv"][0].as_str().and_then(|p| p.rsplit('/').next()).filter(|p| AGENTS.contains(p)).map(str::to_string);
    reported.or(program).or_else(|| t["status"].is_object().then(|| "agent".into()))
}

/// herdr's word for a terminal's status. An error needs you as a block does; `seen` turns a done agent idle.
pub(crate) fn status(t: &Value, seen: &HashMap<String, u64>) -> &'static str {
    match t["status"]["state"].as_str() {
        Some("working") => "working",
        Some("blocked" | "error") => "blocked",
        Some("done") if t["address"].as_str().and_then(|a| seen.get(a)).copied() != t["turn"].as_u64() => "done",
        Some(_) => "idle",
        None => "unknown",
    }
}

/// The one that needs you first: what a tab or workspace of these shows.
pub(crate) fn loudest<'a>(statuses: impl Iterator<Item = &'a str>) -> &'a str {
    statuses.min_by_key(|s| ["blocked", "done", "working", "idle"].iter().position(|o| o == s).unwrap_or(4)).unwrap_or("unknown")
}

/// `session.snapshot`'s result: the document with what the core knows of its terminals.
fn snapshot(doc: &Doc, terminals: &[Value], seen: &HashMap<String, u64>) -> Value {
    let find = |address: &str| terminals.iter().find(|t| t["address"] == address);
    let rect = |r: super::tree::Rect| json!({"x": r.x, "y": r.y, "width": r.width, "height": r.height});
    let (mut workspaces, mut tabs, mut panes, mut layouts) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (w_at, w) in doc.workspaces.iter().enumerate() {
        let mut w_statuses = Vec::new();
        for (t_at, tab) in w.tabs.iter().enumerate() {
            let (frames, splits) = tab.frames();
            let mut t_statuses = Vec::new();
            for (address, _) in &frames {
                let t = find(address).unwrap_or(&Value::Null);
                let (agent, status) = (agent(t), status(t, seen));
                if agent.is_some() {
                    t_statuses.push(status);
                }
                panes.push(json!({
                    "pane_id": address, "tab_id": tab.id, "workspace_id": w.id, "terminal_id": address,
                    "agent_status": status, "agent": agent,
                    "terminal_title_stripped": t["title"].as_str().map(plain_title),
                    "cwd": t["cwd"], "foreground_cwd": t["foreground"]["cwd"],
                }));
            }
            let label = if tab.label.is_empty() { (t_at + 1).to_string() } else { tab.label.clone() };
            let status = loudest(t_statuses.iter().copied());
            tabs.push(json!({"tab_id": tab.id, "workspace_id": w.id, "label": label, "number": t_at + 1, "agent_status": status}));
            w_statuses.push(status);
            let panes = frames.iter().map(|(a, r)| json!({"pane_id": a, "focused": *a == tab.focused, "rect": rect(*r)})).collect::<Vec<_>>();
            let splits = splits.iter().map(|s| json!({"id": s.id, "direction": if s.right { "right" } else { "down" }, "ratio": s.ratio, "rect": rect(s.rect)})).collect::<Vec<_>>();
            layouts.push(json!({
                "tab_id": tab.id, "zoomed": tab.zoomed, "focused_pane_id": tab.focused,
                "area": {"x": 0, "y": 0, "width": AREA, "height": AREA}, "panes": panes, "splits": splits,
            }));
        }
        let first = w.tabs.first().map(|t| t.focused.as_str()).and_then(find);
        let folder = first.and_then(|t| t["foreground"]["cwd"].as_str().or(t["cwd"].as_str()));
        let folder = folder.and_then(|c| c.rsplit('/').find(|p| !p.is_empty())).unwrap_or("workspace");
        let label = if w.label.is_empty() { folder } else { &w.label };
        workspaces.push(json!({
            "workspace_id": w.id, "label": label, "number": w_at + 1, "active_tab_id": w.active_tab,
            "agent_status": loudest(w_statuses.iter().copied()), "worktree": w.worktree,
        }));
    }
    let shown = doc.shown_tab();
    json!({
        "focused_workspace_id": shown.and_then(|t| doc.workspace_of(&t.id)).map(|w| &w.id),
        "focused_tab_id": shown.map(|t| &t.id), "focused_pane_id": shown.map(|t| &t.focused),
        "workspaces": workspaces, "tabs": tabs, "panes": panes, "layouts": layouts,
    })
}

/// The core's events that change a snapshot; the `title` of an agent spinning does not, unless its words change.
fn matters(event: &Value, titles: &mut HashMap<String, String>) -> bool {
    match event["params"]["type"].as_str() {
        Some("title") => {
            let (address, title) = (event["params"]["terminal"].as_str().unwrap_or_default(), event["params"]["data"]["title"].as_str().unwrap_or_default());
            let title = plain_title(title).to_string();
            titles.insert(address.into(), title.clone()) != Some(title)
        }
        Some("turn" | "size" | "labels" | "keep") => false,
        _ => event["method"] != "heartbeat",
    }
}

/// Whole JSON lines out of `bytes`: DATA frame boundaries can fall anywhere in one.
fn lines(pending: &mut Vec<u8>, bytes: &[u8]) -> Vec<Value> {
    pending.extend_from_slice(bytes);
    let Some(end) = pending.iter().rposition(|b| *b == b'\n') else { return Vec::new() };
    let whole: Vec<u8> = pending.drain(..=end).collect();
    whole.split(|b| *b == b'\n').filter_map(|l| serde_json::from_slice(l).ok()).collect()
}

/// The core's events for Shepherd's terminals and its own saved edits: herdr's event stream, which only says when
/// to take another snapshot. Ends with the connection, or when the receiver goes.
pub async fn subscribe() -> anyhow::Result<mpsc::UnboundedReceiver<Value>> {
    let core: Arc<Core> = super::connected().await?;
    let (news, mut heard) = mpsc::unbounded_channel::<Value>();
    let (mut pending, mut titles, mut news) = (Vec::new(), HashMap::new(), Some(news));
    let sink: Sink = Box::new(move |item| match item {
        Item::Data(bytes) => {
            for event in lines(&mut pending, &bytes).into_iter().filter(|e| matters(e, &mut titles)) {
                if let Some(news) = &news {
                    let _ = news.send(event);
                }
            }
        }
        _ => news = None,
    });
    let answer = core.ask("events.subscribe", json!({"labels": {LABEL: "1"}}), Some(sink)).await.map_err(|_| anyhow!("lost herdr-core"))??;
    let channel = answer["channel"].as_u64().ok_or_else(|| anyhow!("events.subscribe opened no stream"))? as u32;
    core.credit(channel, 1 << 20);
    let (tx, rx) = mpsc::unbounded_channel();
    let mut edits = EDITS.subscribe();
    tokio::spawn(async move {
        loop {
            let event = tokio::select! {
                event = heard.recv() => event,
                _ = edits.changed() => Some(Value::Null),
                _ = tx.closed() => None,
            };
            if !event.is_some_and(|e| tx.send(e).is_ok()) {
                break;
            }
        }
        core.send(channel, super::conn::CLOSE, &[]);
    });
    Ok(rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AgentStatus, Snapshot};

    fn doc() -> Doc {
        let mut doc = Doc::default();
        doc.add_workspace("t1".into());
        doc.split("t1", "t2", false, true);
        doc
    }

    #[test]
    fn the_window_reads_a_snapshot_as_herdrs() {
        let terminals = [
            json!({"address": "t1", "cwd": "/repo/shepherd", "title": "zsh", "foreground": {"name": "zsh", "argv": ["zsh"], "cwd": "/repo/shepherd/src"}}),
            json!({"address": "t2", "cwd": "/repo", "title": "⠂ fixing it", "turn": 3, "foreground": {"name": "2.1.0", "argv": ["claude"], "cwd": "/repo"},
                   "status": {"state": "working", "app": "Claude"}}),
        ];
        let result = json!({"snapshot": snapshot(&doc(), &terminals, &HashMap::new())});
        let s = Snapshot::from_result(&result).unwrap();
        assert_eq!(s.workspaces[0].label, "repo", "the first tab's focused pane names it");
        assert_eq!(s.workspaces[0].agent_status, AgentStatus::Working);
        assert_eq!(s.tabs[0].agent_status, AgentStatus::Working);
        assert_eq!(s.focused_pane_id.as_deref(), Some("t2"));
        assert_eq!(s.panes[0].agent, None);
        assert_eq!(s.panes[1].agent.as_deref(), Some("claude"));
        assert_eq!(s.panes[1].terminal_title_stripped.as_deref(), Some("fixing it"));
        assert_eq!(s.panes[0].foreground_cwd.as_deref(), Some("/repo/shepherd/src"));
        let frames = s.pane_frames(&s.tabs[0].tab_id);
        assert_eq!((frames[0].frame.w, frames[1].frame.x), (0.5, 0.5));
        assert!(frames[1].focused);
        let splits = s.split_frames(&s.tabs[0].tab_id);
        assert_eq!((splits[0].path.clone(), splits[0].vertical), (vec![], true));
    }

    #[test]
    fn a_worktree_sits_under_its_repos_checkout() {
        let mut doc = Doc::default();
        let tree = |checkout: &str, linked| Some(Worktree { repo_key: "/r/app/.git".into(), is_linked_worktree: linked, checkout_path: checkout.into() });
        let (linked, _) = doc.add_workspace("t1".into());
        doc.add_workspace("t2".into());
        let (main, _) = doc.add_workspace("t3".into());
        doc.set_worktree(&linked, tree("/w/app/fix", true));
        doc.set_worktree(&main, tree("/r/app", false));
        assert_eq!(doc.workspace_at("/w/app/fix"), Some(linked.clone()));
        let s = Snapshot::from_result(&json!({"snapshot": snapshot(&doc, &[], &HashMap::new())})).unwrap();
        let order: Vec<_> = s.workspace_tree().into_iter().map(|(w, child)| (w.workspace_id.clone(), child)).collect();
        let ids: Vec<_> = order.iter().map(|(id, child)| (id.as_str(), *child)).collect();
        assert_eq!(ids[1..], [(main.as_str(), None), (linked.as_str(), Some(true))], "the worktree moved under its checkout");
    }

    #[test]
    fn statuses_read_as_herdrs() {
        let t = |state: &str| json!({"address": "t1", "turn": 2, "status": {"state": state}});
        let none = HashMap::new();
        assert_eq!(status(&t("working"), &none), "working");
        assert_eq!(status(&t("error"), &none), "blocked");
        assert_eq!(status(&t("done"), &none), "done");
        assert_eq!(status(&t("done"), &HashMap::from([("t1".to_string(), 2)])), "idle", "seen this turn");
        assert_eq!(status(&t("done"), &HashMap::from([("t1".to_string(), 1)])), "done", "seen before it finished again");
        assert_eq!(status(&json!({"address": "t1"}), &none), "unknown");
    }

    #[test]
    fn agents_are_named_by_themselves_or_their_program() {
        assert_eq!(agent(&json!({"status": {"state": "idle", "app": "Pi"}})).as_deref(), Some("pi"));
        assert_eq!(agent(&json!({"status": {"state": "idle", "app": "claude-code"}})).as_deref(), Some("claude"));
        assert_eq!(agent(&json!({"foreground": {"argv": ["/usr/local/bin/codex", "resume"]}})).as_deref(), Some("codex"));
        assert_eq!(agent(&json!({"status": {"state": "idle"}})).as_deref(), Some("agent"));
        assert_eq!(agent(&json!({"foreground": {"argv": ["vim"]}})), None);
    }

    #[test]
    fn a_tab_shows_what_needs_you_first() {
        assert_eq!(loudest(["idle", "working", "done"].into_iter()), "done");
        assert_eq!(loudest(std::iter::empty()), "unknown");
    }

    #[test]
    fn a_spinning_title_is_not_news() {
        let title = |t: &str| json!({"method": "event", "params": {"type": "title", "terminal": "t1", "data": {"title": t}}});
        let mut titles = HashMap::new();
        assert!(matters(&title("⠂ fixing it"), &mut titles));
        assert!(!matters(&title("⠐ fixing it"), &mut titles));
        assert!(matters(&title("⠂ done"), &mut titles));
        assert!(!matters(&json!({"method": "event", "params": {"type": "size"}}), &mut titles));
        assert!(!matters(&json!({"method": "heartbeat", "params": {"seq": 1}}), &mut titles));
        assert!(matters(&json!({"method": "event", "params": {"type": "finished"}}), &mut titles));
        assert!(matters(&json!({"method": "missed", "params": {"seq": 2}}), &mut titles));
    }

    #[test]
    fn event_lines_survive_frame_boundaries() {
        let mut pending = Vec::new();
        assert!(lines(&mut pending, b"{\"a\":").is_empty());
        assert_eq!(lines(&mut pending, b"1}\n{\"b\":2}\n{\"c"), [json!({"a": 1}), json!({"b": 2})]);
        assert_eq!(lines(&mut pending, b"\":3}\n"), [json!({"c": 3})]);
    }

    /// Live, on a core of its own: `HC_SESSION=shepherd-live cargo test hc::api::tests::live -- --ignored --nocapture`.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn live() {
        assert!(std::env::var("HC_SESSION").is_ok_and(|s| s != "default"), "run it on an isolated core: it closes what it opens");
        let mut events = subscribe().await.unwrap();
        let made = call("workspace.create", json!({"cwd": "/tmp"})).await.unwrap();
        let first = made["root_pane"]["pane_id"].as_str().unwrap().to_string();
        let second = call("pane.split", json!({"target_pane_id": first, "direction": "right"})).await.unwrap();
        let second = second["pane"]["pane_id"].as_str().unwrap().to_string();
        let shot = Snapshot::from_result(&call("session.snapshot", json!({})).await.unwrap()).unwrap();
        assert_eq!(shot.panes.len(), 2);
        assert_eq!(shot.focused_pane_id.as_deref(), Some(second.as_str()));
        assert!(tokio::time::timeout(Duration::from_secs(2), events.recv()).await.unwrap().is_some(), "an event or an edit said so");

        let pane = first.clone();
        let (attached, mut stream) = tokio::task::spawn_blocking(move || crate::hc::attach(&pane, 80, 24, (10, 20), false)).await.unwrap().unwrap();
        attached.input(b"seq 1 200; echo shepherd-$((40+2))\r");
        let mut emulator = crate::terminal::emulator::Emulator::raw(80, 24);
        let shown = |e: &crate::terminal::emulator::Emulator| (0..e.rows()).any(|row| e.row_text(row).trim() == "shepherd-42");
        let printed = tokio::time::timeout(Duration::from_secs(10), async {
            use futures::StreamExt;
            while let Some(event) = stream.next().await {
                if let crate::herdr::SessionEvent::Frame(bytes) = event {
                    emulator.feed(&bytes);
                    if shown(&emulator) {
                        return true;
                    }
                }
            }
            false
        });
        assert!(printed.await.unwrap(), "the shell ran it and the emulator shows its output");
        assert!(emulator.history_lines() > 150, "what scrolled off is the emulator's own history: {}", emulator.history_lines());
        drop(attached);

        let read = call("pane.read", json!({"pane_id": first, "lines": 50})).await.unwrap();
        assert!(read["read"]["text"].as_str().unwrap().contains("shepherd-42"));
        call("pane.close", json!({"pane_id": second})).await.unwrap();
        let shot = Snapshot::from_result(&call("session.snapshot", json!({})).await.unwrap()).unwrap();
        assert_eq!(shot.panes.len(), 1, "its room went to the other");

        call("workspace.close", json!({"workspace_id": shot.workspaces[0].workspace_id})).await.unwrap();
        let shot = Snapshot::from_result(&call("session.snapshot", json!({})).await.unwrap()).unwrap();
        assert!(shot.workspaces.is_empty());
        assert!(list(&super::super::connected().await.unwrap()).await.unwrap().is_empty(), "no terminal left behind");

        // a worktree, in the real `~/.herdr/worktrees/<a temporary repo>`, which goes at the end
        let dir = tempfile::tempdir().unwrap();
        let repo = std::fs::canonicalize(dir.path()).unwrap().to_string_lossy().into_owned();
        for args in [&["init", "-q"][..], &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "first"]] {
            assert!(std::process::Command::new("git").arg("-C").arg(&repo).args(args).status().unwrap().success());
        }
        let workspace = |shot: &Snapshot, id: &str| shot.workspaces.iter().find(|w| w.workspace_id == id).cloned();
        let snap = || async { Snapshot::from_result(&call("session.snapshot", json!({})).await.unwrap()).unwrap() };
        let main = call("workspace.create", json!({"cwd": repo})).await.unwrap();
        let main = main["workspace"]["workspace_id"].as_str().unwrap().to_string();
        let made = call("worktree.create", json!({"workspace_id": main, "branch": "fix/live", "focus": false})).await.unwrap();
        assert!(crate::commands::created_tab_id(&made).is_some(), "the window selects its tab");
        let linked = made["workspace"]["workspace_id"].as_str().unwrap().to_string();
        let shot = snap().await;
        let tree = workspace(&shot, &linked).unwrap().worktree.unwrap();
        assert!(tree.is_linked_worktree);
        assert!(tree.checkout_path.ends_with("/fix-live"), "{}", tree.checkout_path);
        assert_eq!(workspace(&shot, &linked).unwrap().label, "fix-live");
        assert_eq!(shot.workspace_tree().iter().map(|(_, child)| *child).collect::<Vec<_>>(), [None, Some(true)], "nested under the repo");

        let listed = call("worktree.list", json!({"workspace_id": main})).await.unwrap();
        let open: Vec<_> = listed["worktrees"].as_array().unwrap().iter().map(|w| w["open_workspace_id"].as_str().unwrap()).collect();
        assert_eq!(open, [main.as_str(), linked.as_str()]);

        std::fs::write(std::path::Path::new(&tree.checkout_path).join("work"), "unsaved").unwrap();
        let refused = call("worktree.remove", json!({"workspace_id": linked, "force": false})).await.unwrap_err().to_string();
        assert!(refused.starts_with("dirty_worktree_requires_force"), "{refused}");
        assert!(workspace(&snap().await, &linked).is_some(), "it stays open");
        assert!(call("worktree.remove", json!({"workspace_id": main, "force": true})).await.is_err(), "the repo's own checkout is not a worktree");
        call("worktree.remove", json!({"workspace_id": linked, "force": true})).await.unwrap();
        assert!(workspace(&snap().await, &linked).is_none());
        assert!(!std::path::Path::new(&tree.checkout_path).exists());

        call("workspace.close", json!({"workspace_id": main})).await.unwrap();
        std::fs::remove_dir(std::path::Path::new(&tree.checkout_path).parent().unwrap()).unwrap();
    }
}
