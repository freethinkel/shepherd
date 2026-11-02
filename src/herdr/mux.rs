//! herdr behind the backend's traits: it has all of it, so each is nearly one call of its API.
use super::{Client, EVENT_TYPES, Mode, Session};
use crate::backend::{Agents, Attached, Change, Kind, Made, Mux, Reply, Worktrees};
use crate::commands::Command;
use crate::model::{Pane, Snapshot, Worktree, WorktreeEntry};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;

fn change(event: &Value) -> Change {
    match event["event"].as_str() {
        Some("pane_focused") => Change::Focused,
        Some("pane_updated") => Pane::deserialize(&event["data"]["pane"]).map_or(Change::Any, |pane| Change::Pane(Box::new(pane))),
        _ => Change::Any,
    }
}

/// The program in front of the pane's shell, from `pane.process_info`.
fn foreground(info: &Value) -> Option<String> {
    let group = info["foreground_process_group_id"].as_u64()?;
    if info["shell_pid"].as_u64() == Some(group) {
        return None;
    }
    let leader = info["foreground_processes"].as_array()?.iter().find(|p| p["pid"].as_u64() == Some(group))?;
    Some(leader["name"].as_str()?.to_string())
}

impl Mux for Client {
    fn kind(&self) -> Kind {
        Kind::Herdr
    }

    fn snapshot(&self) -> Reply<'_, Snapshot> {
        Box::pin(async move { Snapshot::from_result(&self.call("session.snapshot", json!({})).await?) })
    }

    fn changes(&self, agents: Vec<String>) -> Reply<'_, mpsc::UnboundedReceiver<Change>> {
        Box::pin(async move {
            let mut events = self.subscribe(EVENT_TYPES, &agents).await?;
            let (tx, rx) = mpsc::unbounded_channel();
            tokio::spawn(async move {
                loop {
                    let event = tokio::select! {
                        event = events.recv() => event,
                        _ = tx.closed() => None, // hangs up now, not on the next event
                    };
                    if !event.is_some_and(|e| tx.send(change(&e)).is_ok()) {
                        break;
                    }
                }
            });
            Ok(rx)
        })
    }

    fn run(&self, command: Command) -> Reply<'_, Made> {
        Box::pin(async move {
            let (method, params) = command.request();
            Ok(Made::of(&self.call(method, params).await?))
        })
    }

    fn cwd(&self, pane: String) -> Reply<'_, Option<String>> {
        Box::pin(async move {
            let got = self.call("pane.get", json!({"pane_id": pane})).await?;
            let pane = &got["pane"];
            Ok(pane["foreground_cwd"].as_str().or(pane["cwd"].as_str()).map(str::to_string))
        })
    }

    fn read(&self, pane: String, lines: usize, ansi: bool) -> Reply<'_, String> {
        Box::pin(async move {
            let mut params = json!({"pane_id": pane, "source": "recent", "lines": lines});
            if ansi {
                params["format"] = json!("ansi");
                params["strip_ansi"] = json!(false);
            }
            let read = self.call("pane.read", params).await?;
            Ok(read["read"]["text"].as_str().unwrap_or_default().to_string())
        })
    }

    fn program(&self, pane: String) -> Reply<'_, Option<String>> {
        Box::pin(async move { Ok(foreground(&self.call("pane.process_info", json!({"pane_id": pane})).await?["process_info"])) })
    }

    fn send(&self, pane: String, text: Option<String>) -> Reply<'_, ()> {
        Box::pin(async move {
            let mut params = json!({"pane_id": pane, "keys": ["enter"]});
            if let Some(text) = text {
                params["text"] = json!(text);
            }
            self.call("pane.send_input", params).await.map(drop)
        })
    }

    fn prompt(&self, pane: String) -> Reply<'_, ()> {
        Box::pin(async move {
            let prompt = json!({"type": "regex", "value": "\\S"});
            let wait = json!({"pane_id": pane, "source": "recent", "match": prompt, "timeout_ms": 5000});
            self.call("pane.wait_for_output", wait).await.map(drop)
        })
    }

    fn attach(&self, pane: &str, cols: u16, rows: u16, cell_px: (u32, u32), mode: Mode) -> anyhow::Result<Attached> {
        let (session, events) = Session::spawn(pane, cols, rows, cell_px, mode)?;
        Ok((Box::new(session), events))
    }

    /// herdr knows its checkouts.
    fn remember(&self, _workspace: String, _tree: Worktree) -> Reply<'_, ()> {
        Box::pin(async { Ok(()) })
    }

    fn herdr(&self) -> Option<&Client> {
        Some(self)
    }
}

impl Worktrees for Client {
    fn list(&self, workspace: String) -> Reply<'_, Vec<WorktreeEntry>> {
        Box::pin(async move {
            let listed = self.call("worktree.list", json!({"workspace_id": workspace})).await?;
            Ok(serde_json::from_value(listed["worktrees"].clone())?)
        })
    }

    fn create(&self, workspace: String, branch: String) -> Reply<'_, Made> {
        Box::pin(async move { Ok(Made::of(&self.call("worktree.create", json!({"workspace_id": workspace, "branch": branch, "focus": false})).await?)) })
    }

    fn open(&self, workspace: String, path: String) -> Reply<'_, Made> {
        Box::pin(async move { Ok(Made::of(&self.call("worktree.open", json!({"workspace_id": workspace, "path": path, "focus": false})).await?)) })
    }

    fn remove(&self, workspace: String, force: bool) -> Reply<'_, ()> {
        Box::pin(async move { self.call("worktree.remove", json!({"workspace_id": workspace, "force": force})).await.map(drop) })
    }
}

impl Agents for Client {
    /// herdr's snapshot already has them.
    fn read(&self, _snapshot: &mut Snapshot) {}

    /// herdr's focus is how it marks a done agent seen. It is shared with its TUI clients, so it is sent only for a done
    /// agent you just looked at.
    fn seen(&self, pane: String) -> Reply<'_, ()> {
        Box::pin(async move { self.call("pane.focus", json!({"pane_id": pane})).await.map(drop) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pane_update_is_named_and_the_rest_only_said() {
        let pane = json!({"pane_id": "p", "tab_id": "t", "workspace_id": "w", "terminal_id": "x", "agent_status": "idle"});
        assert!(matches!(change(&json!({"event": "pane_updated", "data": {"pane": pane}})), Change::Pane(p) if p.pane_id == "p"));
        assert_eq!(change(&json!({"event": "pane_focused"})), Change::Focused);
        assert_eq!(change(&json!({"event": "tab_created"})), Change::Any);
        assert_eq!(change(&json!({"event": "pane_updated", "data": {}})), Change::Any, "one it cannot read");
    }

    /// Live, on a session of its own: `HERDR_SOCKET_PATH=<its socket> cargo test --lib herdr::mux::tests::live -- --ignored`.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn live() {
        let socket = std::env::var("HERDR_SOCKET_PATH").expect("run it on a session of its own: it closes what it opens");
        assert!(socket.contains("/sessions/"), "not herdr's default session");
        let backend = crate::backend::Backend::of(Kind::Herdr, Some(socket));
        let mux = &backend.mux;
        assert_eq!(mux.kind(), Kind::Herdr);
        assert!(mux.herdr().is_some());
        let mut changes = mux.changes(Vec::new()).await.unwrap();
        let repo = tempfile::tempdir().unwrap();
        let repo = std::fs::canonicalize(repo.path()).unwrap().to_string_lossy().into_owned();
        for args in [&["init", "-q"][..], &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "first"]] {
            assert!(std::process::Command::new("git").arg("-C").arg(&repo).args(args).status().unwrap().success());
        }
        let made = mux.run(Command::NewWorkspace { cwd: Some(repo.clone()) }).await.unwrap();
        let (workspace, tab, first) = (made.workspace_id.unwrap(), made.tab_id.unwrap(), made.pane_id.unwrap());
        let told = tokio::time::timeout(std::time::Duration::from_secs(3), changes.recv()).await.unwrap();
        assert!(told.is_some(), "herdr said so");
        let shot = backend.snapshot().await.unwrap();
        assert!(shot.workspaces.iter().any(|w| w.workspace_id == workspace));
        assert!(shot.tabs_of(&workspace).iter().any(|t| t.tab_id == tab));

        let second = mux.run(Command::Split { pane_id: first.clone(), down: false, cwd: Some(repo.clone()) }).await.unwrap().pane_id.unwrap();
        mux.prompt(second.clone()).await.unwrap();
        mux.send(second.clone(), Some("echo shepherd-$((40+2))".into())).await.unwrap();
        let mut read = String::new();
        for _ in 0..50 {
            read = mux.read(second.clone(), 20, false).await.unwrap();
            if read.lines().any(|l| l.trim() == "shepherd-42") {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert!(read.lines().any(|l| l.trim() == "shepherd-42"), "{read}");
        assert_eq!(mux.cwd(second.clone()).await.unwrap().as_deref(), Some(repo.as_str()));
        assert_eq!(mux.program(second.clone()).await.unwrap(), None, "a shell at its prompt");
        let (herdr, pane) = (mux.clone(), first.clone());
        let (attached, _events) = tokio::task::spawn_blocking(move || herdr.attach(&pane, 80, 24, (0, 0), Mode::Observe)).await.unwrap().unwrap();
        assert!(!attached.raw(), "herdr renders for its clients");
        drop(attached);
        backend.agents.seen(first.clone()).await.unwrap();
        mux.run(Command::FocusTab { tab_id: tab.clone() }).await.unwrap();

        let trees = &backend.worktrees;
        assert_eq!(trees.list(workspace.clone()).await.unwrap().len(), 1);
        let made = trees.create(workspace.clone(), "fix/herdr".into()).await.unwrap();
        assert!(made.tab_id.is_some(), "the window selects its tab");
        let linked = made.workspace_id.unwrap();
        let listed = trees.list(workspace.clone()).await.unwrap();
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().any(|w| w.is_linked_worktree && w.branch.as_deref() == Some("fix/herdr") && w.open_workspace_id.as_deref() == Some(linked.as_str())));
        let path = listed.iter().find(|w| w.is_linked_worktree).unwrap().path.clone();
        std::fs::write(std::path::Path::new(&path).join("work"), "unsaved").unwrap();
        let refused = trees.remove(linked.clone(), false).await.unwrap_err().to_string();
        assert!(refused.starts_with("dirty_worktree_requires_force"), "{refused}");
        trees.remove(linked, true).await.unwrap();
        mux.run(Command::CloseWorkspace { workspace_id: workspace.clone() }).await.unwrap();
        assert!(!backend.snapshot().await.unwrap().workspaces.iter().any(|w| w.workspace_id == workspace));
        let _ = std::fs::remove_dir(std::path::Path::new(&path).parent().unwrap());
    }

    #[test]
    fn the_shell_is_no_program_of_its_own() {
        let info = |group: u64| json!({"foreground_process_group_id": group, "shell_pid": 1, "foreground_processes": [{"pid": 2, "name": "nvim"}]});
        assert_eq!(foreground(&info(2)).as_deref(), Some("nvim"));
        assert_eq!(foreground(&info(1)), None);
    }
}
