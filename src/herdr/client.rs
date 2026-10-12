use anyhow::{Context, anyhow};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;

/// Lifecycle events that need no `pane_id` filter (herdr rejects `pane.agent_status_changed` and friends without one,
/// and sends no `pane.updated` when only an agent's status changes: [`Client::subscribe`] follows that per pane).
pub const EVENT_TYPES: &[&str] = &[
    "workspace.created", "workspace.updated", "workspace.closed", "workspace.renamed",
    "workspace.moved", "workspace.reordered", "workspace.focused",
    "worktree.created", "worktree.opened", "worktree.removed",
    "tab.created", "tab.closed", "tab.renamed", "tab.moved", "tab.focused",
    "pane.created", "pane.closed", "pane.updated", "pane.focused", "pane.moved", "pane.exited",
    "pane.agent_detected", "layout.updated",
];

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct Client {
    path: PathBuf,
}

impl Client {
    /// Same resolution as herdr for the default session: `HERDR_SOCKET_PATH`, else `~/.config/herdr/herdr.sock`.
    pub fn local() -> Self {
        let path = std::env::var_os("HERDR_SOCKET_PATH").map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config/herdr/herdr.sock")
        });
        Self { path }
    }

    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    // ponytail: one connection per request, like herdr's own CLI; pool if latency ever shows up.
    /// The write half is returned so the caller keeps it alive: dropping it half-closes the socket.
    async fn send(&self, method: &str, params: Value) -> anyhow::Result<(OwnedWriteHalf, Lines<BufReader<OwnedReadHalf>>)> {
        let stream = UnixStream::connect(&self.path)
            .await
            .with_context(|| format!("herdr is not reachable at {}", self.path.display()))?;
        let (r, mut w) = stream.into_split();
        let id = format!("s{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
        let line = json!({"id": id, "method": method, "params": params});
        w.write_all(format!("{line}\n").as_bytes()).await?;
        Ok((w, BufReader::new(r).lines()))
    }

    pub async fn call(&self, method: &str, params: Value) -> anyhow::Result<Value> {
        let (_w, mut lines) = self.send(method, params).await?;
        let line = lines.next_line().await?.ok_or_else(|| anyhow!("herdr closed the connection"))?;
        into_result(serde_json::from_str(&line)?)
    }

    /// `types`, and `pane.agent_status_changed` for each of `panes`.
    pub async fn subscribe(&self, types: &[&str], panes: &[String]) -> anyhow::Result<mpsc::UnboundedReceiver<Value>> {
        let statuses = panes.iter().map(|p| json!({"type": "pane.agent_status_changed", "pane_id": p}));
        let subscriptions: Vec<Value> = types.iter().map(|t| json!({"type": t})).chain(statuses).collect();
        let (w, mut lines) = self.send("events.subscribe", json!({"subscriptions": subscriptions})).await?;
        let first = lines.next_line().await?.ok_or_else(|| anyhow!("herdr closed the connection"))?;
        into_result(serde_json::from_str(&first)?)?;
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let _w = w;
            loop {
                let line = tokio::select! {
                    line = lines.next_line() => line,
                    _ = tx.closed() => break, // the caller resubscribed: hang up now, not on the next event
                };
                let Ok(Some(line)) = line else { break };
                let Ok(value) = serde_json::from_str::<Value>(&line) else { break };
                // An error line (events_lost) ends the stream; the caller resubscribes and refetches.
                if value.get("error").is_some() || tx.send(value).is_err() {
                    break;
                }
            }
        });
        Ok(rx)
    }
}

fn into_result(mut response: Value) -> anyhow::Result<Value> {
    if let Some(error) = response.get("error") {
        let code = error["code"].as_str().unwrap_or("error");
        let message = error["message"].as_str().unwrap_or("");
        return Err(anyhow!("{code}: {message}"));
    }
    Ok(response["result"].take())
}
