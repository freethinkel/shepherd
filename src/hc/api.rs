//! [`Hc`] as a backend: Shepherd's layout document and what the core knows of its terminals.
use super::{Hc, app, syncing};
use super::conn::{Core, Item, Sink};
use super::tree::{Doc, OTHERS};
use crate::agents::{Report, plain_title};
use crate::backend::{Attached, Change, Kind, Made, Mode, Mux, Reply};
use crate::commands::Command;
use crate::model::{AREA, AgentStatus, Layout, LayoutPane, LayoutSplit, Pane, Rect, Snapshot, Tab, Workspace, Worktree};
use anyhow::{anyhow, bail};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::mpsc;

/// The core document the layout lives in. The core knows no workspaces: the Herdr app keeps its own beside it. Shared
/// with it ([`app`]), this holds what is Shepherd's own; apart, its terminals are taken in by their labels ([`tended`]).
const DOC: &str = "shepherd-2";
/// The document before other clients' terminals were taken in. A Shepherd that old counts a terminal in its layout
/// that it did not start as gone and ends it, so it must never read one: the layout is copied from here once, no more.
const OLD_DOC: &str = "shepherd";
/// Marks the terminals Shepherd started.
const LABEL: &str = "shepherd";

impl Mux for Hc {
    fn kind(&self) -> Kind {
        Kind::Hc
    }

    fn snapshot(&self) -> Reply<'_, Snapshot> {
        Box::pin(async move {
            let core = self.connected().await?;
            let _layout = self.layout.lock().await;
            let mut loaded = load(&core).await?;
            let terminals = loaded.terminals.clone();
            let shared = loaded.app.is_some();
            let changed = tended(&mut loaded.doc, &terminals, shared).is_some();
            let mut doc = loaded.doc.clone();
            if changed || loaded.unsent() {
                let (gone, saved) = self.edit(&core, |d| (tended(d, &terminals, shared), d.clone())).await?;
                doc = saved;
                ended(&core, &gone.unwrap_or_default()).await; // already gone: this only clears their records
            }
            if shared {
                for (terminal, space, tab) in app::misplaced(&doc, &terminals) {
                    core.call("terminal.label", json!({"terminal": terminal, "labels": {"herdr.space": space, "herdr.tab": tab}})).await?;
                }
            }
            Ok(snapshot(&doc, &terminals))
        })
    }

    /// The core's events and Shepherd's own saved edits.
    fn changes(&self, _agents: Vec<String>) -> Reply<'_, mpsc::UnboundedReceiver<Change>> {
        Box::pin(async move {
            let core: Arc<Core> = self.connected().await?;
            let (news, mut heard) = mpsc::unbounded_channel::<()>();
            let (mut pending, mut titles, mut news) = (Vec::new(), HashMap::new(), Some(news));
            let sink: Sink = Box::new(move |item| match item {
                Item::Data(bytes) => {
                    if lines(&mut pending, &bytes).iter().filter(|e| matters(e, &mut titles)).count() > 0
                        && let Some(news) = &news
                    {
                        let _ = news.send(());
                    }
                }
                _ => news = None,
            });
            let answer = core.ask("events.subscribe", json!({}), Some(sink)).await.map_err(|_| anyhow!("lost herdr-core"))??;
            let channel = answer["channel"].as_u64().ok_or_else(|| anyhow!("events.subscribe opened no stream"))? as u32;
            core.credit(channel, 1 << 20);
            let (tx, rx) = mpsc::unbounded_channel();
            let mut edits = self.edits.subscribe();
            tokio::spawn(async move {
                loop {
                    let event = tokio::select! {
                        event = heard.recv() => event,
                        _ = edits.changed() => Some(()),
                        _ = tx.closed() => None,
                    };
                    if !event.is_some_and(|()| tx.send(Change::Any).is_ok()) {
                        break;
                    }
                }
                core.send(channel, super::conn::CLOSE, &[]);
            });
            Ok(rx)
        })
    }

    fn run(&self, command: Command) -> Reply<'_, Made> {
        Box::pin(async move {
            let core = self.connected().await?;
            let core = core.as_ref();
            let _layout = self.layout.lock().await;
            Ok(match command {
                Command::NewWorkspace { cwd } => {
                    let cwd = cwd.unwrap_or_else(|| crate::settings::home().to_string_lossy().into_owned());
                    let terminal = run(core, &cwd).await?;
                    let (workspace, tab) = self.edit(core, |d| d.add_workspace(terminal.clone())).await?;
                    Made { workspace_id: Some(workspace), tab_id: Some(tab), pane_id: Some(terminal) }
                }
                Command::NewTab { workspace_id } => {
                    let doc = load(core).await?.doc;
                    let terminal = run(core, &workspace_cwd(core, &doc, &workspace_id).await).await?;
                    let Some(tab) = self.edit(core, |d| d.add_tab(&workspace_id, terminal.clone())).await? else {
                        ended(core, std::slice::from_ref(&terminal)).await;
                        bail!("no workspace {workspace_id}");
                    };
                    Made { tab_id: Some(tab), pane_id: Some(terminal), ..Made::default() }
                }
                Command::Split { pane_id, down, cwd } => {
                    let cwd = match cwd {
                        Some(cwd) => cwd,
                        None => cwd_of(core, Some(&pane_id)).await,
                    };
                    let terminal = run(core, &cwd).await?;
                    if !self.edit(core, |d| d.split(&pane_id, &terminal, down, true)).await? {
                        ended(core, std::slice::from_ref(&terminal)).await;
                        bail!("no pane {pane_id}");
                    }
                    Made { pane_id: Some(terminal), ..Made::default() }
                }
                Command::CloseWorkspace { workspace_id } => {
                    let terminals = self.edit(core, |d| d.close_workspace(&workspace_id)).await?;
                    ended(core, &terminals).await;
                    Made::default()
                }
                Command::CloseTab { tab_id } => {
                    let terminals = self.edit(core, |d| d.close_tab(&tab_id)).await?;
                    ended(core, &terminals).await;
                    Made::default()
                }
                Command::ClosePane { pane_id } => {
                    self.edit(core, |d| d.remove(&pane_id)).await?;
                    ended(core, std::slice::from_ref(&pane_id)).await;
                    Made::default()
                }
                Command::RenameWorkspace { workspace_id, label } => {
                    self.edit(core, |d| d.rename_workspace(&workspace_id, &label)).await?;
                    Made::default()
                }
                Command::RenameTab { tab_id, label } => {
                    self.edit(core, |d| d.rename_tab(&tab_id, &label)).await?;
                    Made::default()
                }
                Command::MoveTab { tab_id, insert_index } => {
                    self.edit(core, |d| d.move_tab(&tab_id, insert_index)).await?;
                    Made::default()
                }
                Command::MoveWorkspaces { workspace_ids, before } => {
                    self.edit(core, |d| d.move_workspaces(&workspace_ids, before.as_deref())).await?;
                    Made::default()
                }
                Command::FocusTab { tab_id } => {
                    self.edit(core, |d| d.focus_tab(&tab_id)).await?;
                    Made::default()
                }
                Command::Focus { pane_id } => {
                    self.edit(core, |d| d.focus_pane(&pane_id)).await?;
                    Made::default()
                }
                Command::FocusPane { pane_id, direction } => {
                    self.edit(core, |d| {
                        if let Some(next) = d.neighbour(&pane_id, direction.name()) {
                            d.focus_pane(&next);
                        }
                    })
                    .await?;
                    Made::default()
                }
                Command::Zoom { pane_id } => {
                    self.edit(core, |d| d.toggle_zoom(&pane_id)).await?;
                    Made::default()
                }
                Command::SwapPanes { source, target } => {
                    self.edit(core, |d| d.swap(&source, &target)).await?;
                    Made::default()
                }
                Command::MovePane { pane_id, tab_id, down, target } => {
                    let tab = self.edit(core, |d| d.move_pane(&pane_id, &tab_id, target.as_deref(), down).then(|| tab_id.clone())).await?;
                    Made { tab_id: tab, ..Made::default() }
                }
                Command::MovePaneToNewTab { pane_id, workspace_id } => {
                    let tab = self.edit(core, |d| d.pane_to_new_tab(&pane_id, workspace_id.as_deref())).await?;
                    Made { tab_id: tab, ..Made::default() }
                }
                Command::SetSplitRatio { tab_id, path, ratio } => {
                    self.edit(core, |d| d.set_ratio(&tab_id, &path, ratio)).await?;
                    Made::default()
                }
            })
        })
    }

    fn cwd(&self, pane: String) -> Reply<'_, Option<String>> {
        Box::pin(async move {
            let t = terminal(self.connected().await?.as_ref(), &pane).await?;
            Ok(t["foreground"]["cwd"].as_str().or(t["cwd"].as_str()).map(str::to_string))
        })
    }

    fn read(&self, pane: String, lines: usize, _ansi: bool) -> Reply<'_, String> {
        Box::pin(async move {
            let read = self.connected().await?.call("terminal.read", json!({"terminal": pane, "lines": lines})).await?;
            Ok(read["text"].as_str().unwrap_or_default().to_string())
        })
    }

    fn program(&self, pane: String) -> Reply<'_, Option<String>> {
        Box::pin(async move {
            let t = terminal(self.connected().await?.as_ref(), &pane).await?;
            let fg = &t["foreground"];
            Ok(if fg["pid"] == t["pid"] { None } else { fg["name"].as_str().map(str::to_string) })
        })
    }

    fn send(&self, pane: String, text: Option<String>) -> Reply<'_, ()> {
        Box::pin(async move {
            let core = self.connected().await?;
            match text {
                Some(text) => core.call("terminal.send", json!({"terminal": pane, "text": text, "force": true})).await.map(drop),
                None => core.call("terminal.keys", json!({"terminal": pane, "keys": ["enter"]})).await.map(drop),
            }
        })
    }

    fn attach(&self, pane: &str, cols: u16, rows: u16, cell_px: (u32, u32), mode: Mode) -> anyhow::Result<Attached> {
        let (attached, events) = Hc::attach(self, pane, cols, rows, cell_px, mode == Mode::Observe)?;
        Ok((Box::new(attached), events))
    }

    fn remember(&self, workspace: String, tree: Worktree) -> Reply<'_, ()> {
        Box::pin(async move {
            let core = self.connected().await?;
            let _layout = self.layout.lock().await;
            self.edit(&core, |d| d.set_worktree(&workspace, Some(tree.clone()))).await
        })
    }
}

impl Hc {
    /// Saves what is Shepherd's own, then the layout the app shares, whose version says whether it changed meanwhile:
    /// in that order, so that a conflict there leaves nothing to undo.
    async fn save(&self, core: &Core, mut loaded: Loaded) -> anyhow::Result<()> {
        let shared = loaded.app.take();
        let write = shared.as_ref().map(|(was, _)| loaded.written(was)).transpose()?;
        core.call("doc.set", json!({"name": DOC, "value": loaded.doc, "version": loaded.version})).await?;
        if let (Some((was, version)), Some(value)) = (shared, write)
            && was != value
        {
            if !self.backed_up.swap(true, Ordering::Relaxed) && core.call("doc.get", json!({"name": app::BACKUP})).await?["value"].is_null() {
                core.call("doc.set", json!({"name": app::BACKUP, "value": was, "version": 0})).await?;
            }
            core.call("doc.set", json!({"name": app::DOC, "value": value, "version": version})).await?;
        }
        self.edits.send_modify(|n| *n += 1);
        Ok(())
    }

    /// Applies `change` to the document as it is now, again if another writer (a second Shepherd, the app) got there first.
    async fn edit<T>(&self, core: &Core, mut change: impl FnMut(&mut Doc) -> T) -> anyhow::Result<T> {
        loop {
            let mut loaded = load(core).await?;
            let out = change(&mut loaded.doc);
            match self.save(core, loaded).await {
                Err(e) if e.to_string().starts_with("conflict") => continue,
                saved => return saved.map(|()| out),
            }
        }
    }
}

/// What a change works from.
struct Loaded {
    doc: Doc,
    version: u64,
    terminals: Vec<Value>,
    /// The app's document and its version, when the layout is shared.
    app: Option<(Value, u64)>,
}

impl Loaded {
    fn written(&mut self, was: &Value) -> anyhow::Result<Value> {
        let terminals = &self.terminals;
        let live = |t: &str| terminals.iter().any(|l| l["address"] == t);
        let cwd = |t: &str| terminals.iter().find(|l| l["address"] == t).and_then(|l| l["cwd"].as_str()).map(str::to_string);
        app::written(&mut self.doc, was, &live, &cwd)
    }

    /// Whether a workspace of Shepherd's has yet to reach the app.
    fn unsent(&self) -> bool {
        let Some((was, _)) = &self.app else { return false };
        let mut copy = Loaded { doc: self.doc.clone(), version: self.version, terminals: self.terminals.clone(), app: None };
        copy.written(was).is_ok_and(|now| now != *was)
    }
}

async fn load(core: &Core) -> anyhow::Result<Loaded> {
    let got = core.call("doc.get", json!({"name": DOC})).await?;
    let version = got["version"].as_u64().unwrap_or(0);
    let value = match got["value"].is_null() {
        true => core.call("doc.get", json!({"name": OLD_DOC})).await?["value"].clone(),
        false => got["value"].clone(),
    };
    let mut doc: Doc = if value.is_null() { Doc::default() } else { serde_json::from_value(value)? };
    let terminals = list(core).await?;
    let app = match syncing() {
        true => {
            let got = core.call("doc.get", json!({"name": app::DOC})).await?;
            let live = |t: &str| terminals.iter().any(|l| l["address"] == t);
            app::absorb(&mut doc, &got["value"], &live);
            Some((got["value"].clone(), got["version"].as_u64().unwrap_or(0)))
        }
        false => None,
    };
    Ok(Loaded { doc, version, terminals, app })
}

/// Every live terminal in the core, whoever started it.
async fn list(core: &Core) -> anyhow::Result<Vec<Value>> {
    let listed = core.call("terminal.list", json!({})).await?;
    Ok(listed["terminals"].as_array().cloned().unwrap_or_default())
}

/// Brings `doc` up to `terminals`: drops the gone, takes in what other clients started. `None`: nothing changed, else
/// the terminals of Shepherd's that are gone. When `shared`, the app places what its labels give a space, and the rest
/// (parked in the app, or no client's) are left for the others' workspace.
fn tended(doc: &mut Doc, terminals: &[Value], shared: bool) -> Option<Vec<String>> {
    let before = doc.adopted.len();
    let gone = doc.retain(|t| terminals.iter().any(|l| l["address"] == t));
    let mut changed = !gone.is_empty() || doc.adopted.len() != before;
    for t in terminals {
        let Some(address) = t["address"].as_str().filter(|a| doc.tab_of(a).is_none()) else { continue };
        let (space, tab) = (t["labels"]["herdr.space"].as_str().filter(|s| !s.is_empty()), t["labels"]["herdr.tab"].as_str());
        if if shared { space.is_none() } else { t["labels"][LABEL].is_null() } {
            doc.adopt(address, if shared { OTHERS } else { space.unwrap_or(OTHERS) }, tab.filter(|_| !shared));
            changed = true;
        }
    }
    changed.then_some(gone)
}

/// The space and tab a client names a terminal by, `where: dotfiles / claude`.
fn named(t: Option<&Value>) -> Option<(&str, &str)> {
    t?["labels"]["where"].as_str()?.split_once(" / ")
}

async fn terminal(core: &Core, address: &str) -> anyhow::Result<Value> {
    core.call("terminal.get", json!({"terminal": address})).await
}

/// A new shell in `cwd`, kept through a crash or a reboot as the Herdr app's panes are; its address.
async fn run(core: &Core, cwd: &str) -> anyhow::Result<String> {
    let env = json!({"TERM": "xterm-256color", "COLORTERM": "truecolor", "TERM_PROGRAM": "Shepherd"});
    let run = core.call("terminal.run", json!({"cwd": cwd, "env": env, "labels": {LABEL: "1", "herdr.seen": "0"}, "keep": true})).await?;
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

/// Ends `terminals`. A finished one refuses the kill and takes the removal.
async fn ended(core: &Core, terminals: &[String]) {
    for t in terminals {
        let _ = core.call("terminal.kill", json!({"terminal": t})).await;
        let _ = core.call("terminal.rm", json!({"terminal": t})).await;
    }
}

/// What the core knows of what a terminal runs and reports.
fn report(t: &Value) -> Report {
    let text = |value: &Value| value.as_str().map(str::to_string);
    Report { state: text(&t["status"]["state"]), app: text(&t["status"]["app"]), program: text(&t["foreground"]["argv"][0]), turn: t["turn"].as_u64().unwrap_or(0) }
}

/// The document with what the core knows of its terminals.
fn snapshot(doc: &Doc, terminals: &[Value]) -> Snapshot {
    let find = |address: &str| terminals.iter().find(|t| t["address"] == address);
    let rect = |r: super::tree::Rect| Rect { x: r.x, y: r.y, width: r.width, height: r.height };
    let text = |value: &Value| value.as_str().map(str::to_string);
    let mut out = Snapshot::default();
    for (w_at, w) in doc.workspaces.iter().enumerate() {
        for (t_at, tab) in w.tabs.iter().enumerate() {
            let (frames, splits) = tab.frames();
            for (address, _) in &frames {
                let t = find(address).unwrap_or(&Value::Null);
                out.panes.push(Pane {
                    pane_id: address.clone(),
                    tab_id: tab.id.clone(),
                    workspace_id: w.id.clone(),
                    terminal_id: address.clone(),
                    terminal_title_stripped: t["title"].as_str().map(|t| plain_title(t).to_string()),
                    cwd: text(&t["cwd"]),
                    foreground_cwd: text(&t["foreground"]["cwd"]),
                    report: Some(report(t)),
                    ..Pane::default()
                });
            }
            let name = tab.root.terminals().iter().find_map(|a| named(find(a))).map(|(_, tab)| tab.to_string());
            let label = if tab.label.is_empty() { name.unwrap_or_else(|| (t_at + 1).to_string()) } else { tab.label.clone() };
            out.tabs.push(Tab { tab_id: tab.id.clone(), workspace_id: w.id.clone(), label, number: t_at as u32 + 1, agent_status: AgentStatus::Unknown });
            out.layouts.push(Layout {
                tab_id: tab.id.clone(),
                zoomed: tab.zoomed,
                focused_pane_id: tab.focused.clone(),
                area: Rect { x: 0, y: 0, width: AREA, height: AREA },
                panes: frames.iter().map(|(a, r)| LayoutPane { pane_id: a.clone(), focused: *a == tab.focused, rect: rect(*r) }).collect(),
                splits: splits.iter().map(|s| LayoutSplit { id: s.id.clone(), direction: if s.right { "right" } else { "down" }.into(), ratio: s.ratio, rect: rect(s.rect) }).collect(),
            });
        }
        let first = w.tabs.first().map(|t| t.focused.as_str()).and_then(find);
        let folder = first.and_then(|t| t["foreground"]["cwd"].as_str().or(t["cwd"].as_str()));
        let folder = folder.and_then(|c| c.rsplit('/').find(|p| !p.is_empty())).unwrap_or("workspace");
        let space = w.tabs.iter().flat_map(|t| t.root.terminals()).find_map(|a| named(find(&a))).map(|(space, _)| space);
        let others = (w.origin.as_deref() == Some(OTHERS)).then_some("Other terminals");
        let label = if w.label.is_empty() { space.or(others).unwrap_or(folder) } else { &w.label };
        out.workspaces.push(Workspace {
            workspace_id: w.id.clone(),
            label: label.to_string(),
            number: w_at as u32 + 1,
            active_tab_id: Some(w.active_tab.clone()),
            agent_status: AgentStatus::Unknown,
            worktree: w.worktree.clone(),
        });
    }
    let shown = doc.shown_tab();
    out.focused_workspace_id = shown.and_then(|t| doc.workspace_of(&t.id)).map(|w| w.id.clone());
    out.focused_tab_id = shown.map(|t| t.id.clone());
    out.focused_pane_id = shown.map(|t| t.focused.clone());
    out
}

/// The core's events that change a snapshot; the `title` of an agent spinning does not, unless its words change.
fn matters(event: &Value, titles: &mut HashMap<String, String>) -> bool {
    match event["params"]["type"].as_str() {
        Some("title") => {
            let (address, title) = (event["params"]["terminal"].as_str().unwrap_or_default(), event["params"]["data"]["title"].as_str().unwrap_or_default());
            let title = plain_title(title).to_string();
            titles.insert(address.into(), title.clone()) != Some(title)
        }
        Some("turn" | "size" | "keep") => false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{Backend, Kind};
    use std::time::Duration;

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
        let s = snapshot(&doc(), &terminals);
        assert_eq!(s.workspaces[0].label, "repo", "the first tab's focused pane names it");
        assert_eq!(s.focused_pane_id.as_deref(), Some("t2"));
        assert_eq!(s.panes[1].terminal_title_stripped.as_deref(), Some("fixing it"));
        let told = Report { state: Some("working".into()), app: Some("Claude".into()), program: Some("claude".into()), turn: 3 };
        assert_eq!(s.panes[1].report.as_ref(), Some(&told));
        assert_eq!(s.panes[0].foreground_cwd.as_deref(), Some("/repo/shepherd/src"));
        let frames = s.pane_frames(&s.tabs[0].tab_id);
        assert_eq!((frames[0].frame.w, frames[1].frame.x), (0.5, 0.5));
        assert!(frames[1].focused);
        let splits = s.split_frames(&s.tabs[0].tab_id);
        assert_eq!((splits[0].path.clone(), splits[0].vertical), (vec![], true));
    }

    #[test]
    fn other_clients_terminals_show_under_their_names() {
        let app = |address: &str, tab: &str, place: &str| json!({"address": address, "labels": {"herdr.space": "sp1", "herdr.tab": tab, "where": place}});
        let terminals = [
            json!({"address": "t1", "labels": {"shepherd": "1"}, "cwd": "/mine"}),
            app("t2", "tb1", "dotfiles / claude"),
            app("t3", "tb1", "dotfiles / claude"),
            app("t4", "tb2", "dotfiles / 3"),
            json!({"address": "t5", "labels": {}, "cwd": "/x"}),
        ];
        let mut doc = Doc::default();
        doc.add_workspace("t1".into());
        assert_eq!(tended(&mut doc, &terminals, false), Some(vec![]));
        assert_eq!(tended(&mut doc, &terminals, false), None, "nothing to save the second time");
        let s = snapshot(&doc, &terminals);
        assert_eq!(s.workspaces.iter().map(|w| w.label.as_str()).collect::<Vec<_>>(), ["mine", "dotfiles", "Other terminals"]);
        assert_eq!(s.tabs.iter().map(|t| t.label.as_str()).collect::<Vec<_>>(), ["1", "claude", "3", "1"]);
        assert_eq!(s.pane_frames(&s.tabs[1].tab_id).len(), 2, "t2 and t3 share a tab");

        // one of Shepherd's and one of the app's end: only Shepherd's record is cleared
        let left = [terminals[2].clone(), terminals[3].clone(), terminals[4].clone()];
        assert_eq!(tended(&mut doc, &left, false), Some(vec!["t1".to_string()]));
    }

    #[test]
    fn a_worktree_sits_under_its_repos_checkout() {
        let mut doc = Doc::default();
        let tree = |checkout: &str, linked| Some(Worktree { repo_key: "/r/app/.git".into(), is_linked_worktree: linked, checkout_path: checkout.into(), repo_root: "/r/app".into() });
        let (linked, _) = doc.add_workspace("t1".into());
        doc.add_workspace("t2".into());
        let (main, _) = doc.add_workspace("t3".into());
        doc.set_worktree(&linked, tree("/w/app/fix", true));
        doc.set_worktree(&main, tree("/r/app", false));
        let s = snapshot(&doc, &[]);
        let order: Vec<_> = s.workspace_tree().into_iter().map(|(w, child)| (w.workspace_id.clone(), child)).collect();
        let ids: Vec<_> = order.iter().map(|(id, child)| (id.as_str(), *child)).collect();
        assert_eq!(ids[1..], [(main.as_str(), None), (linked.as_str(), Some(true))], "the worktree moved under its checkout");
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
        assert!(matters(&json!({"method": "event", "params": {"type": "labels"}}), &mut titles), "a tab renamed in another client");
        assert!(matters(&json!({"method": "missed", "params": {"seq": 2}}), &mut titles));
    }

    #[test]
    fn event_lines_survive_frame_boundaries() {
        let mut pending = Vec::new();
        assert!(lines(&mut pending, b"{\"a\":").is_empty());
        assert_eq!(lines(&mut pending, b"1}\n{\"b\":2}\n{\"c"), [json!({"a": 1}), json!({"b": 2})]);
        assert_eq!(lines(&mut pending, b"\":3}\n"), [json!({"c": 3})]);
    }

    /// Live, on a core of its own, a second client in the Herdr app's place: it reads and writes `herdr-app` as the app does.
    /// `HC_SESSION=<name> cargo test --lib hc::api::tests::synced -- --ignored`.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn synced() {
        assert!(std::env::var("HC_SESSION").is_ok_and(|s| s != "default"), "run it on an isolated core: it ends what it opens");
        super::super::set_sync(true);
        let hc = Hc::default();
        let core = hc.connected().await.unwrap();
        let shared = |core: Arc<Core>| async move { core.call("doc.get", json!({"name": app::DOC})).await.unwrap() };
        let cli = |args: Vec<String>| async move {
            tokio::task::spawn_blocking(move || Kind::Hc.run(&args.iter().map(String::as_str).collect::<Vec<_>>())).await.unwrap()
        };
        let said = cli(["run", "--bg", "--json", "--label", "herdr.space=sp1", "--label", "herdr.tab=tb1", "--keep"].map(String::from).to_vec()).await.unwrap();
        let theirs = serde_json::from_str::<Value>(&said).unwrap()["address"].as_str().unwrap().to_string();
        let first = json!({"spaces": [{"id": "sp1", "name": "", "folder": "/tmp", "tabs": [{"id": "tb1", "name": "claude", "root": {
            "type": "row", "id": "tb1", "children": [{"type": "block", "id": "p1", "content": {"terminal": theirs}}]}}]}]});
        core.call("doc.set", json!({"name": app::DOC, "value": first, "version": 0})).await.unwrap();

        let backend = Backend::of(Kind::Hc, None);
        let mux = &backend.mux;
        let mut changes = mux.changes(Vec::new()).await.unwrap();
        let shot = backend.snapshot().await.unwrap();
        assert_eq!(shot.workspaces.iter().map(|w| w.workspace_id.as_str()).collect::<Vec<_>>(), ["sp1"]);
        assert_eq!(shot.tabs[0].label, "claude");
        assert_eq!(shot.panes[0].pane_id, theirs);
        assert_eq!(shared(core.clone()).await["version"], 1, "reading writes nothing");

        // what Shepherd opens reaches the document, in the app's shape, and its terminal says where it is
        let made = mux.run(Command::NewWorkspace { cwd: Some("/tmp".into()) }).await.unwrap();
        let (workspace, tab, mine) = (made.workspace_id.unwrap(), made.tab_id.unwrap(), made.pane_id.unwrap());
        let doc = shared(core.clone()).await;
        let space = &doc["value"]["spaces"][1];
        assert_eq!(space["id"], workspace.as_str());
        assert!(space["folder"].as_str().is_some_and(|f| f.ends_with("/tmp")), "{space}");
        assert_eq!(space["tabs"][0]["root"]["id"], tab.as_str());
        assert_eq!(space["tabs"][0]["root"]["children"][0]["content"]["terminal"], mine.as_str());
        assert_eq!(core.call("doc.get", json!({"name": app::BACKUP})).await.unwrap()["value"], first, "a copy from before the first write");
        backend.snapshot().await.unwrap();
        let labels = terminal(&core, &mine).await.unwrap()["labels"].clone();
        assert_eq!((labels["herdr.space"].as_str(), labels["herdr.tab"].as_str()), (Some(workspace.as_str()), Some(tab.as_str())), "{labels}");

        // a split, a new tab and a rename in the app's space
        let split = mux.run(Command::Split { pane_id: theirs.clone(), down: false, cwd: Some("/tmp".into()) }).await.unwrap().pane_id.unwrap();
        let new_tab = mux.run(Command::NewTab { workspace_id: "sp1".into() }).await.unwrap().tab_id.unwrap();
        mux.run(Command::RenameTab { tab_id: "tb1".into(), label: "work".into() }).await.unwrap();
        let doc = shared(core.clone()).await;
        let tabs = doc["value"]["spaces"][0]["tabs"].as_array().unwrap();
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0]["name"], "work");
        assert_eq!(tabs[0]["root"]["children"][0]["id"], "p1", "its pane keeps its id");
        assert_eq!(tabs[0]["root"]["children"][1]["content"]["terminal"], split.as_str());
        assert_eq!(tabs[1]["id"], new_tab.as_str());

        // focus is Shepherd's own
        let version = doc["version"].clone();
        mux.run(Command::Focus { pane_id: theirs.clone() }).await.unwrap();
        assert_eq!(shared(core.clone()).await["version"], version, "the app's document is not touched");
        assert_eq!(backend.snapshot().await.unwrap().focused_pane_id.as_deref(), Some(theirs.as_str()));

        // the other client writes: the core says so, and Shepherd shows it
        while tokio::time::timeout(Duration::from_millis(300), changes.recv()).await.is_ok() {}
        let mut theirs_now = doc["value"].clone();
        theirs_now["spaces"][0]["tabs"][0]["name"] = json!("renamed there");
        core.call("doc.set", json!({"name": app::DOC, "value": theirs_now, "version": version})).await.unwrap();
        tokio::time::timeout(Duration::from_secs(3), changes.recv()).await.expect("the core says the document changed").unwrap();
        assert!(backend.snapshot().await.unwrap().tabs.iter().any(|t| t.label == "renamed there"));

        // a writer that got there first: Shepherd's change goes on top of it, not over it
        let doc = shared(core.clone()).await;
        let (mux2, tab2) = (mux.clone(), new_tab.clone());
        let renamer = tokio::spawn(async move {
            for at in 0..30 {
                mux2.run(Command::RenameTab { tab_id: tab2.clone(), label: format!("mine {at}") }).await.unwrap();
            }
        });
        let (mut value, mut version) = (doc["value"].clone(), doc["version"].as_u64().unwrap());
        let mut landed = 0;
        for at in 0..30 {
            value["spaces"][0]["name"] = json!(format!("space {at}"));
            match core.call("doc.set", json!({"name": app::DOC, "value": value, "version": version})).await {
                Ok(_) => landed += 1,
                Err(e) => assert!(e.to_string().starts_with("conflict"), "{e}"),
            }
            let now = shared(core.clone()).await;
            value = now["value"].clone();
            version = now["version"].as_u64().unwrap();
        }
        renamer.await.unwrap();
        let doc = shared(core.clone()).await;
        assert!(landed > 0);
        assert_eq!(doc["value"]["spaces"][0]["tabs"][1]["name"], "mine 29", "the last of Shepherd's");
        assert!(doc["value"]["spaces"][0]["name"].as_str().unwrap().starts_with("space "), "and the other's stayed: {doc}");

        // a terminal gone is hidden here and the document's to tidy; closing here is a change
        let version = doc["version"].clone();
        cli(vec!["kill".to_string(), split.clone()]).await.unwrap();
        for _ in 0..50 {
            if !backend.snapshot().await.unwrap().panes.iter().any(|p| p.pane_id == split) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(!backend.snapshot().await.unwrap().panes.iter().any(|p| p.pane_id == split));
        assert_eq!(shared(core.clone()).await["version"], version, "the app decides what became of it");
        mux.run(Command::CloseWorkspace { workspace_id: workspace.clone() }).await.unwrap();
        let doc = shared(core.clone()).await;
        assert!(!doc["value"]["spaces"].as_array().unwrap().iter().any(|s| s["id"] == workspace.as_str()));

        // off, the app's document is left alone and its terminals still show
        super::super::set_sync(false);
        let shot = backend.snapshot().await.unwrap();
        assert!(shot.panes.iter().any(|p| p.pane_id == theirs));
        mux.run(Command::NewWorkspace { cwd: Some("/tmp".into()) }).await.unwrap();
        assert_eq!(shared(core.clone()).await["version"], doc["version"], "nothing goes to the app");
        super::super::set_sync(true);
    }

    /// Live, on a core of its own: another client's terminals, as the Herdr app labels them.
    /// `HC_SESSION=<name> cargo test --lib hc::api::tests::others -- --ignored`.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn others() {
        assert!(std::env::var("HC_SESSION").is_ok_and(|s| s != "default"), "run it on an isolated core: it ends what it opens");
        super::super::set_sync(false);
        let started = |labels: &'static [&'static str]| async move {
            let mut args = vec!["run", "--bg", "--json"];
            args.extend(labels.iter().flat_map(|l| ["--label", *l]));
            tokio::task::spawn_blocking(move || Kind::Hc.run(&args)).await.unwrap().unwrap()
        };
        let address = |said: String| serde_json::from_str::<Value>(&said).unwrap()["address"].as_str().unwrap().to_string();
        // Shepherd's layout from before: carried over, and the old document left alone
        let hc = Hc::default();
        let core = hc.connected().await.unwrap();
        let mine = run(&core, "/tmp").await.unwrap();
        let mut old = Doc::default();
        old.add_workspace(mine.clone());
        core.call("doc.set", json!({"name": OLD_DOC, "value": old, "version": 0})).await.unwrap();

        let claude = address(started(&["herdr.space=sp1", "herdr.tab=tb1", "where=dots / claude"]).await);
        let second = address(started(&["herdr.space=sp1", "herdr.tab=tb1", "where=dots / claude"]).await);
        let third = address(started(&["herdr.space=sp1", "herdr.tab=tb2", "where=dots / logs"]).await);
        let stray = address(started(&[]).await);
        let backend = Backend::of(Kind::Hc, None);
        let mux = &backend.mux;
        let mut changes = mux.changes(Vec::new()).await.unwrap();
        let shot = backend.snapshot().await.unwrap();
        assert_eq!(shot.workspaces.iter().map(|w| w.label.as_str()).collect::<Vec<_>>(), ["tmp", "dots", "Other terminals"], "{:?}", shot.workspaces);
        assert_eq!(shot.tabs.iter().map(|t| t.label.as_str()).collect::<Vec<_>>(), ["1", "claude", "logs", "1"]);
        assert_eq!(shot.pane_frames(&shot.tabs[1].tab_id).len(), 2);
        let ids = |s: &Snapshot| s.workspaces.iter().map(|w| w.workspace_id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&backend.snapshot().await.unwrap()), ids(&shot), "the same ids the next time");
        assert_eq!(core.call("doc.get", json!({"name": OLD_DOC})).await.unwrap()["value"], serde_json::to_value(&old).unwrap(), "untouched");

        // the same terminal: typed into here, it answers in the core
        mux.prompt(claude.clone()).await.unwrap();
        mux.send(claude.clone(), Some("echo shepherd-$((40+2))".into())).await.unwrap();
        let mut read = String::new();
        for _ in 0..50 {
            read = mux.read(claude.clone(), 20, false).await.unwrap();
            if read.lines().any(|l| l.trim() == "shepherd-42") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(read.lines().any(|l| l.trim() == "shepherd-42"), "{read}");

        // what another client does shows without being asked
        while tokio::time::timeout(Duration::from_millis(300), changes.recv()).await.is_ok() {}
        let late = address(started(&["herdr.space=sp1", "herdr.tab=tb3", "where=dots / late"]).await);
        tokio::time::timeout(Duration::from_secs(3), changes.recv()).await.expect("the core says so").unwrap();
        assert!(backend.snapshot().await.unwrap().tabs.iter().any(|t| t.label == "late"));

        // one that ends over there leaves its record alone
        tokio::task::spawn_blocking({ let t = third.clone(); move || Kind::Hc.run(&["kill", &t]) }).await.unwrap().unwrap();
        for _ in 0..50 {
            if !backend.snapshot().await.unwrap().panes.iter().any(|p| p.pane_id == third) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(!backend.snapshot().await.unwrap().panes.iter().any(|p| p.pane_id == third));
        let all = tokio::task::spawn_blocking(|| Kind::Hc.run(&["ls", "--all", "--json"])).await.unwrap().unwrap();
        assert!(serde_json::from_str::<Value>(&all).unwrap()["terminals"].as_array().unwrap().iter().any(|t| t["address"] == third.as_str()), "still the other client's: {all}");

        for t in [mine, claude, second, late, stray] {
            let _ = tokio::task::spawn_blocking(move || Kind::Hc.run(&["kill", &t])).await.unwrap();
        }
    }

    /// Live, on a core of its own: `HC_SESSION=shepherd-live cargo test hc::api::tests::live -- --ignored --nocapture`.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn live() {
        assert!(std::env::var("HC_SESSION").is_ok_and(|s| s != "default"), "run it on an isolated core: it closes what it opens");
        let backend = Backend::of(Kind::Hc, None);
        let mux = &backend.mux;
        let snap = || async { backend.snapshot().await.unwrap() };
        let mut events = mux.changes(Vec::new()).await.unwrap();
        let made = mux.run(Command::NewWorkspace { cwd: Some("/tmp".into()) }).await.unwrap();
        let first = made.pane_id.clone().unwrap();
        let second = mux.run(Command::Split { pane_id: first.clone(), down: false, cwd: None }).await.unwrap().pane_id.unwrap();
        let shot = snap().await;
        assert_eq!(shot.panes.len(), 2);
        assert_eq!(shot.focused_pane_id.as_deref(), Some(second.as_str()));
        assert!(tokio::time::timeout(Duration::from_secs(2), events.recv()).await.unwrap().is_some(), "an event or an edit said so");

        let (hc, pane) = (mux.clone(), first.clone());
        let (attached, mut stream) = tokio::task::spawn_blocking(move || hc.attach(&pane, 80, 24, (10, 20), Mode::Control)).await.unwrap().unwrap();
        attached.input(b"seq 1 200; echo shepherd-$((40+2))\r");
        let mut emulator = crate::terminal::emulator::Emulator::raw(80, 24);
        let shown = |e: &crate::terminal::emulator::Emulator| (0..e.rows()).any(|row| e.row_text(row).trim() == "shepherd-42");
        let printed = tokio::time::timeout(Duration::from_secs(10), async {
            use futures::StreamExt;
            while let Some(event) = stream.next().await {
                if let crate::backend::SessionEvent::Frame(bytes) = event {
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

        assert!(mux.read(first.clone(), 50, false).await.unwrap().contains("shepherd-42"));
        mux.run(Command::ClosePane { pane_id: second }).await.unwrap();
        let shot = snap().await;
        assert_eq!(shot.panes.len(), 1, "its room went to the other");

        mux.run(Command::CloseWorkspace { workspace_id: shot.workspaces[0].workspace_id.clone() }).await.unwrap();
        assert!(snap().await.workspaces.is_empty());

        // a worktree, in the real `~/.herdr/worktrees/<a temporary repo>`, which goes at the end
        let dir = tempfile::tempdir().unwrap();
        let repo = std::fs::canonicalize(dir.path()).unwrap().to_string_lossy().into_owned();
        for args in [&["init", "-q"][..], &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "first"]] {
            assert!(std::process::Command::new("git").arg("-C").arg(&repo).args(args).status().unwrap().success());
        }
        let workspace = |shot: &Snapshot, id: &str| shot.workspaces.iter().find(|w| w.workspace_id == id).cloned();
        let main = mux.run(Command::NewWorkspace { cwd: Some(repo) }).await.unwrap().workspace_id.unwrap();
        let trees = &backend.worktrees;
        let made = trees.create(main.clone(), "fix/live".into()).await.unwrap();
        assert!(made.tab_id.is_some(), "the window selects its tab");
        let linked = made.workspace_id.unwrap();
        let shot = snap().await;
        let tree = workspace(&shot, &linked).unwrap().worktree.unwrap();
        assert!(tree.is_linked_worktree);
        assert!(tree.checkout_path.ends_with("/fix-live"), "{}", tree.checkout_path);
        assert_eq!(workspace(&shot, &linked).unwrap().label, "fix-live");
        assert_eq!(shot.workspace_tree().iter().map(|(_, child)| *child).collect::<Vec<_>>(), [None, Some(true)], "nested under the repo");

        let open: Vec<_> = trees.list(main.clone()).await.unwrap().into_iter().map(|w| w.open_workspace_id.unwrap()).collect();
        assert_eq!(open, [main.clone(), linked.clone()]);

        std::fs::write(std::path::Path::new(&tree.checkout_path).join("work"), "unsaved").unwrap();
        let refused = trees.remove(linked.clone(), false).await.unwrap_err().to_string();
        assert!(refused.starts_with("dirty_worktree_requires_force"), "{refused}");
        assert!(workspace(&snap().await, &linked).is_some(), "it stays open");
        assert!(trees.remove(main.clone(), true).await.is_err(), "the repo's own checkout is not a worktree");
        trees.remove(linked.clone(), true).await.unwrap();
        assert!(workspace(&snap().await, &linked).is_none());
        assert!(!std::path::Path::new(&tree.checkout_path).exists());

        mux.run(Command::CloseWorkspace { workspace_id: main }).await.unwrap();
        std::fs::remove_dir(std::path::Path::new(&tree.checkout_path).parent().unwrap()).unwrap();
    }
}
