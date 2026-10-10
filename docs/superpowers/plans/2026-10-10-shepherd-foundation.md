# Shepherd Foundation (Plan 1 of 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A GPUI window that shows herdr's focused tab with every pane rendered and typed into through a real libghostty surface, kept in sync with herdr's events; plus written answers to the spec's four spike questions.

**Architecture:** One crate. `herdr::client` speaks herdr's newline-delimited JSON API over its unix socket; `model` parses `session.snapshot` and turns a tab's layout into fractional pane frames; every herdr event just triggers a fresh snapshot. Each pane gets a libghostty surface hosted in our own `NSView` subclass placed under zui's GPUI overlay at the pane's bounds; the surface's process is `shepherd attach <pane>`, a shim between the surface PTY and `herdr terminal session control`.

**Tech Stack:** Rust 2024, GPUI (`zeronsh/zui@18a89af`), tokio, serde_json, base64, libc, objc2 0.6 / objc2-app-kit 0.3.2 / objc2-foundation 0.3.2, raw-window-handle 0.6, bindgen (build), libghostty (Zig 0.15.2, ghostty `efb35235919a38bb04ac8b8ef06537f0317b7045`), herdr 0.9.3 (protocol 22).

**Spec:** `docs/superpowers/specs/2026-10-10-shepherd-client-design.md`

Plan 2 (sidebar, tabs, splits UI, drag-and-drop, dialogs, saved machines, Ghostty action mapping, theme, bundling) is written after Task 7's spike report, because its shape depends on the answers.

## Global Constraints

- macOS only (14+, Apple Silicon first).
- Everything in the repository is in English: code, comments, docs, commit messages.
- No personal data in the repository: fixtures use made-up ids and `/tmp/demo` paths.
- GPUI deps pinned exactly as Collie's app: `gpui`, `gpui_platform` (`font-kit`, `runtime_shaders`), `gpui_tokio` from `https://github.com/zeronsh/zui` rev `18a89af04cfe079d55275b111ac349d5640d9cda`.
- Build the app outside nix with rustup cargo: `env -u MACOSX_DEPLOYMENT_TARGET cargo …` (GPUI needs Xcode's SDKs).
- herdr is the single source of truth for layout: Shepherd never edits layout locally.
- The terminal stream is `herdr terminal session control`, never `herdr terminal attach` (it reserves `ctrl+b`).

## Review Focus

1. Shepherd launched from inside a herdr pane must not attach a surface to its own pane (`HERDR_PANE_ID`): that pane shows a placeholder, never an infinite mirror. → Task 3 test `own_pane_is_not_attachable`.
2. herdr server restarts or drops the subscription (`events_lost`): the event stream ends cleanly so the app resubscribes and refetches. → Task 2 test `subscription_ends_when_server_closes`.
3. herdr not running: the error names the socket path instead of a raw `ENOENT`. → Task 2 test `missing_socket_names_the_path`.
4. A pane with a zero-sized rect (herdr layout during a resize, or a collapsed split) must not produce a zero-sized surface. → Task 3 test `zero_area_panes_are_skipped`.
5. A pane closed while attached: the shim exits on `terminal.closed` instead of hanging. → Task 4 test `shim_bridges_frames_input_and_exit`.

---

### Task 1: Crate scaffold, libghostty build, FFI smoke test

**Files:**
- Create: `Cargo.toml`, `build.rs`, `Makefile`, `.gitignore`, `src/main.rs`, `src/lib.rs`, `src/ghostty/mod.rs`, `src/ghostty/ffi.rs`, `tests/ghostty_link.rs`

**Interfaces:**
- Produces: `shepherd::ghostty::ffi::*` — bindgen output for `ghostty.h` (functions `ghostty_*`, types `ghostty_*`, constants `GHOSTTY_*` without enum-name prefixes).

- [ ] **Step 1: Install Zig 0.15.2**

Run: `zig version`
Expected: `0.15.2`. If missing or different, download `zig-aarch64-macos-0.15.2` from https://ziglang.org/download/ and put its `zig` on `PATH` (or pass `ZIG=/path/to/zig` to make).

- [ ] **Step 2: Write the Makefile and .gitignore**

`Makefile`:
```make
# libghostty at a pinned commit (the one flutter_ghostty builds with Zig 0.15.2).
GHOSTTY_REV := efb35235919a38bb04ac8b8ef06537f0317b7045
ZIG ?= zig
CARGO := env -u MACOSX_DEPLOYMENT_TARGET cargo

vendor/ghostty/libghostty.dylib:
	@$(ZIG) version | grep -qx 0.15.2 || { echo "need zig 0.15.2"; exit 1; }
	test -d .ghostty-src || git clone https://github.com/ghostty-org/ghostty .ghostty-src
	cd .ghostty-src && git fetch -q origin $(GHOSTTY_REV) && git checkout -q $(GHOSTTY_REV)
	cd .ghostty-src && $(ZIG) build -Dapp-runtime=none -Demit-xcframework=false -Demit-macos-app=false -Doptimize=ReleaseFast
	mkdir -p vendor/ghostty
	cp .ghostty-src/zig-out/lib/libghostty.dylib .ghostty-src/include/ghostty.h vendor/ghostty/
	install_name_tool -id @rpath/libghostty.dylib vendor/ghostty/libghostty.dylib

ghostty: vendor/ghostty/libghostty.dylib

build: ghostty
	$(CARGO) build

test: ghostty
	$(CARGO) test

run: ghostty
	$(CARGO) run

.PHONY: ghostty build test run
```

`.gitignore`:
```
/target
/.ghostty-src
/vendor
```

- [ ] **Step 3: Build libghostty**

Run: `make ghostty`
Expected: `vendor/ghostty/libghostty.dylib` and `vendor/ghostty/ghostty.h` exist (the build takes several minutes).

- [ ] **Step 4: Write Cargo.toml, build.rs and the FFI module**

`Cargo.toml`:
```toml
[package]
name = "shepherd"
version = "0.1.0"
edition = "2024"
publish = false

# Its own workspace, like collie/desktop/app.
[workspace]

[dependencies]
anyhow = "1"
base64 = "0.22"
futures = "0.3"
gpui = { git = "https://github.com/zeronsh/zui", rev = "18a89af04cfe079d55275b111ac349d5640d9cda" }
gpui_platform = { git = "https://github.com/zeronsh/zui", rev = "18a89af04cfe079d55275b111ac349d5640d9cda", features = ["font-kit", "runtime_shaders"] }
libc = "0.2"
objc2 = "0.6"
objc2-app-kit = { version = "0.3.2", features = ["NSEvent", "NSGraphics", "NSResponder", "NSView", "NSPasteboard", "NSWindow"] }
objc2-foundation = "0.3.2"
raw-window-handle = "0.6"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }

[build-dependencies]
bindgen = "0.72"

[dev-dependencies]
tempfile = "3"

# gpui is slow unoptimized (Zeron and Collie do the same).
[profile.dev.package."*"]
opt-level = 2
```

`build.rs`:
```rust
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/ghostty");
    let header = dir.join("ghostty.h");
    assert!(header.exists(), "run `make ghostty` first: {} is missing", header.display());
    println!("cargo:rerun-if-changed={}", header.display());
    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rustc-link-lib=dylib=ghostty");
    // ponytail: rpath to vendor/ for dev builds; the app bundle (plan 2) copies the dylib into Frameworks.
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", dir.display());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindgen::Builder::default()
        .header(header.to_str().unwrap())
        .allowlist_function("ghostty_.*")
        .allowlist_type("ghostty_.*")
        .allowlist_var("GHOSTTY_.*")
        .prepend_enum_name(false)
        .generate()
        .expect("bindgen ghostty.h")
        .write_to_file(out.join("ghostty.rs"))
        .unwrap();
}
```

`src/ghostty/ffi.rs`:
```rust
#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case, dead_code)]
include!(concat!(env!("OUT_DIR"), "/ghostty.rs"));
```

`src/ghostty/mod.rs`:
```rust
pub mod ffi;
```

`src/lib.rs`:
```rust
pub mod ghostty;
```

`src/main.rs`:
```rust
fn main() {}
```

- [ ] **Step 5: Write the failing link test**

`tests/ghostty_link.rs`:
```rust
use shepherd::ghostty::ffi;

#[test]
fn libghostty_links_and_initializes() {
    let rc = unsafe { ffi::ghostty_init(0, std::ptr::null_mut()) };
    assert_eq!(rc, 0, "ghostty_init failed");
    let info = unsafe { ffi::ghostty_info() };
    assert!(!info.version.is_null() && info.version_len > 0);
}
```

- [ ] **Step 6: Run it**

Run: `make test`
Expected: PASS `libghostty_links_and_initializes`. If cargo cannot resolve the git deps, copy Collie's lock first (`cp ../collie/desktop/app/Cargo.lock .`) so the same transitive versions are used, then rerun.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock build.rs Makefile .gitignore src tests
git commit -m "scaffold: crate, pinned libghostty build, bindgen FFI"
```

---

### Task 2: herdr JSON API client

**Files:**
- Create: `src/herdr/mod.rs`, `src/herdr/client.rs`, `tests/herdr_client.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces:
  - `shepherd::herdr::Client` (`Clone`): `Client::local() -> Client` (socket from `HERDR_SOCKET_PATH`, else `~/.config/herdr/herdr.sock`), `Client::at(path: PathBuf) -> Client`, `client.path() -> &Path`.
  - `async fn call(&self, method: &str, params: serde_json::Value) -> anyhow::Result<serde_json::Value>` — returns the response's `result`; an `error` becomes `Err("<code>: <message>")`.
  - `async fn subscribe(&self, types: &[&str]) -> anyhow::Result<tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>>` — yields each event envelope (`{"event": …, "data": …}`); the receiver ends when the server closes the connection or reports an error (`events_lost`). Must be called inside a tokio runtime.
  - `pub const EVENT_TYPES: &[&str]` — every lifecycle event that needs no `pane_id`.

- [ ] **Step 1: Write the failing tests**

`tests/herdr_client.rs`:
```rust
use serde_json::{Value, json};
use shepherd::herdr::Client;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

/// A fake herdr: answers each request line with `reply(request)`; returns the socket path.
fn fake_server(dir: &tempfile::TempDir, reply: fn(&Value) -> Vec<Value>) -> std::path::PathBuf {
    let path = dir.path().join("herdr.sock");
    let listener = UnixListener::bind(&path).unwrap();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let (r, mut w) = stream.into_split();
                let mut lines = BufReader::new(r).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let req: Value = serde_json::from_str(&line).unwrap();
                    for out in reply(&req) {
                        w.write_all(format!("{out}\n").as_bytes()).await.unwrap();
                    }
                }
            });
        }
    });
    path
}

#[tokio::test]
async fn call_returns_result() {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_server(&dir, |req| vec![json!({"id": req["id"], "result": {"type": "pong"}})]);
    let result = Client::at(path).call("ping", json!({})).await.unwrap();
    assert_eq!(result, json!({"type": "pong"}));
}

#[tokio::test]
async fn call_error_carries_code_and_message() {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_server(&dir, |req| {
        vec![json!({"id": req["id"], "error": {"code": "pane_not_found", "message": "no pane w9:p9"}})]
    });
    let err = Client::at(path).call("pane.get", json!({"pane_id": "w9:p9"})).await.unwrap_err();
    assert_eq!(err.to_string(), "pane_not_found: no pane w9:p9");
}

#[tokio::test]
async fn missing_socket_names_the_path() {
    let err = Client::at("/tmp/demo/no-such/herdr.sock".into()).call("ping", json!({})).await.unwrap_err();
    assert!(err.to_string().contains("/tmp/demo/no-such/herdr.sock"), "{err}");
}

#[tokio::test]
async fn subscription_yields_events() {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_server(&dir, |req| {
        assert_eq!(req["method"], "events.subscribe");
        vec![
            json!({"id": req["id"], "result": {"type": "subscription_started"}}),
            json!({"event": "tab.created", "data": {"type": "tab_created", "tab": {"tab_id": "w1:t2"}}}),
            json!({"event": "pane.closed", "data": {"type": "pane_closed", "pane_id": "w1:p2", "workspace_id": "w1"}}),
        ]
    });
    let mut rx = Client::at(path).subscribe(&["tab.created", "pane.closed"]).await.unwrap();
    assert_eq!(rx.recv().await.unwrap()["event"], "tab.created");
    assert_eq!(rx.recv().await.unwrap()["event"], "pane.closed");
}

#[tokio::test]
async fn subscription_ends_when_server_closes() {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_server(&dir, |req| {
        vec![
            json!({"id": req["id"], "result": {"type": "subscription_started"}}),
            json!({"id": req["id"], "error": {"code": "events_lost", "message": "fell behind"}}),
        ]
    });
    let mut rx = Client::at(path).subscribe(&["tab.created"]).await.unwrap();
    assert!(rx.recv().await.is_none());
}
```

Note: the fake keeps connections open after replying, so `subscription_ends_when_server_closes` proves the client ends the stream on an `error` line, which is what herdr sends before closing.

- [ ] **Step 2: Run them to verify they fail**

Run: `env -u MACOSX_DEPLOYMENT_TARGET cargo test --test herdr_client`
Expected: FAIL to compile, `unresolved import shepherd::herdr`.

- [ ] **Step 3: Implement the client**

`src/herdr/mod.rs`:
```rust
mod client;
pub use client::{Client, EVENT_TYPES};
```

`src/lib.rs`:
```rust
pub mod ghostty;
pub mod herdr;
```

`src/herdr/client.rs`:
```rust
use anyhow::{Context, anyhow};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;

/// Lifecycle events that need no `pane_id` filter (herdr rejects
/// `pane.agent_status_changed` and friends without one; `pane.updated` carries agent status).
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

    pub async fn subscribe(&self, types: &[&str]) -> anyhow::Result<mpsc::UnboundedReceiver<Value>> {
        let subscriptions: Vec<Value> = types.iter().map(|t| json!({"type": t})).collect();
        let (w, mut lines) = self.send("events.subscribe", json!({"subscriptions": subscriptions})).await?;
        let first = lines.next_line().await?.ok_or_else(|| anyhow!("herdr closed the connection"))?;
        into_result(serde_json::from_str(&first)?)?;
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let _w = w;
            while let Ok(Some(line)) = lines.next_line().await {
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
```

- [ ] **Step 4: Run the tests**

Run: `env -u MACOSX_DEPLOYMENT_TARGET cargo test --test herdr_client`
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/herdr tests/herdr_client.rs
git commit -m "herdr: JSON API client with calls and event subscriptions"
```

---

### Task 3: Snapshot model and pane frames

**Files:**
- Create: `src/model.rs`, `tests/fixtures/snapshot.json`, `tests/model.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: the `result` value of `session.snapshot` from `Client::call`.
- Produces:
  - `shepherd::model::Snapshot::from_result(result: &serde_json::Value) -> anyhow::Result<Snapshot>`
  - `Snapshot { focused_tab_id: Option<String>, workspaces: Vec<Workspace>, tabs: Vec<Tab>, panes: Vec<Pane>, layouts: Vec<Layout>, .. }`
  - `snapshot.pane_frames(tab_id: &str) -> Vec<PaneFrame>`; `PaneFrame { pane_id: String, frame: Frac, focused: bool }`; `Frac { x: f32, y: f32, w: f32, h: f32 }` in 0..=1 of the tab area.
  - `model::attachable(pane_id: &str, own_pane: Option<&str>) -> bool`
  - `AgentStatus` enum: `Idle | Working | Blocked | Done | Unknown`.

- [ ] **Step 1: Write the fixture**

`tests/fixtures/snapshot.json` (the `result` of `session.snapshot`, shape copied from herdr 0.9.3, data made up):
```json
{
  "type": "session_snapshot",
  "snapshot": {
    "version": "0.9.3",
    "protocol": 22,
    "focused_workspace_id": "w1",
    "focused_tab_id": "w1:t1",
    "focused_pane_id": "w1:p2",
    "workspaces": [
      {"workspace_id": "w1", "label": "demo", "number": 1, "active_tab_id": "w1:t1",
       "agent_status": "working", "focused": true, "pane_count": 3, "tab_count": 2}
    ],
    "tabs": [
      {"tab_id": "w1:t1", "workspace_id": "w1", "label": "main", "number": 1, "agent_status": "working", "focused": true, "pane_count": 2},
      {"tab_id": "w1:t2", "workspace_id": "w1", "label": "logs", "number": 2, "agent_status": "unknown", "focused": false, "pane_count": 1}
    ],
    "panes": [
      {"pane_id": "w1:p1", "tab_id": "w1:t1", "workspace_id": "w1", "terminal_id": "term_a", "agent_status": "unknown", "focused": false, "cwd": "/tmp/demo", "revision": 1},
      {"pane_id": "w1:p2", "tab_id": "w1:t1", "workspace_id": "w1", "terminal_id": "term_b", "agent_status": "working", "focused": true, "cwd": "/tmp/demo", "revision": 7,
       "agent": "claude", "terminal_title_stripped": "fixing tests"},
      {"pane_id": "w1:p3", "tab_id": "w1:t2", "workspace_id": "w1", "terminal_id": "term_c", "agent_status": "unknown", "focused": false, "cwd": "/tmp/demo", "revision": 0}
    ],
    "layouts": [
      {"workspace_id": "w1", "tab_id": "w1:t1", "zoomed": false, "focused_pane_id": "w1:p2",
       "area": {"x": 0, "y": 0, "width": 100, "height": 40},
       "panes": [
         {"pane_id": "w1:p1", "focused": false, "rect": {"x": 0, "y": 0, "width": 50, "height": 40}},
         {"pane_id": "w1:p2", "focused": true, "rect": {"x": 50, "y": 0, "width": 50, "height": 40}}
       ],
       "splits": [{"id": "s0", "direction": "right", "ratio": 0.5, "rect": {"x": 0, "y": 0, "width": 100, "height": 40}}]},
      {"workspace_id": "w1", "tab_id": "w1:t2", "zoomed": false, "focused_pane_id": "w1:p3",
       "area": {"x": 0, "y": 0, "width": 100, "height": 40},
       "panes": [
         {"pane_id": "w1:p3", "focused": true, "rect": {"x": 0, "y": 0, "width": 100, "height": 40}},
         {"pane_id": "w1:p9", "focused": false, "rect": {"x": 100, "y": 0, "width": 0, "height": 40}}
       ],
       "splits": []}
    ],
    "agents": []
  }
}
```

- [ ] **Step 2: Write the failing tests**

`tests/model.rs`:
```rust
use shepherd::model::{AgentStatus, Frac, Snapshot, attachable};

fn fixture() -> Snapshot {
    let value: serde_json::Value = serde_json::from_str(include_str!("fixtures/snapshot.json")).unwrap();
    Snapshot::from_result(&value).unwrap()
}

#[test]
fn parses_snapshot() {
    let s = fixture();
    assert_eq!(s.focused_tab_id.as_deref(), Some("w1:t1"));
    assert_eq!(s.workspaces[0].label, "demo");
    assert_eq!(s.tabs.len(), 2);
    let p2 = s.panes.iter().find(|p| p.pane_id == "w1:p2").unwrap();
    assert_eq!(p2.agent_status, AgentStatus::Working);
    assert_eq!(p2.terminal_title_stripped.as_deref(), Some("fixing tests"));
}

#[test]
fn pane_frames_are_fractions_of_the_tab_area() {
    let frames = fixture().pane_frames("w1:t1");
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].pane_id, "w1:p1");
    assert_eq!(frames[0].frame, Frac { x: 0.0, y: 0.0, w: 0.5, h: 1.0 });
    assert_eq!(frames[1].frame, Frac { x: 0.5, y: 0.0, w: 0.5, h: 1.0 });
    assert!(frames[1].focused);
}

#[test]
fn zoomed_tab_shows_only_the_focused_pane_full_size() {
    let mut s = fixture();
    s.layouts[0].zoomed = true;
    let frames = s.pane_frames("w1:t1");
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].pane_id, "w1:p2");
    assert_eq!(frames[0].frame, Frac { x: 0.0, y: 0.0, w: 1.0, h: 1.0 });
}

#[test]
fn zero_area_panes_are_skipped() {
    let frames = fixture().pane_frames("w1:t2");
    assert_eq!(frames.iter().map(|f| f.pane_id.as_str()).collect::<Vec<_>>(), ["w1:p3"]);
}

#[test]
fn unknown_tab_has_no_frames() {
    assert!(fixture().pane_frames("w7:t7").is_empty());
}

#[test]
fn own_pane_is_not_attachable() {
    assert!(!attachable("w1:p2", Some("w1:p2")));
    assert!(attachable("w1:p1", Some("w1:p2")));
    assert!(attachable("w1:p1", None));
}

#[test]
fn unknown_agent_status_parses_as_unknown() {
    let s: AgentStatus = serde_json::from_str("\"sleeping\"").unwrap();
    assert_eq!(s, AgentStatus::Unknown);
}
```

- [ ] **Step 3: Run them to verify they fail**

Run: `env -u MACOSX_DEPLOYMENT_TARGET cargo test --test model`
Expected: FAIL to compile, `unresolved import shepherd::model`.

- [ ] **Step 4: Implement the model**

`src/lib.rs`:
```rust
pub mod ghostty;
pub mod herdr;
pub mod model;
```

`src/model.rs`:
```rust
//! herdr's `session.snapshot`, parsed. Events only invalidate it: the app refetches
//! (herdr's docs: events have no shared sequence with snapshots, so never replay them onto one).
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Idle,
    Working,
    Blocked,
    Done,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Snapshot {
    pub focused_workspace_id: Option<String>,
    pub focused_tab_id: Option<String>,
    pub focused_pane_id: Option<String>,
    pub workspaces: Vec<Workspace>,
    pub tabs: Vec<Tab>,
    pub panes: Vec<Pane>,
    pub layouts: Vec<Layout>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    pub workspace_id: String,
    pub label: String,
    pub number: u32,
    pub active_tab_id: Option<String>,
    pub agent_status: AgentStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tab {
    pub tab_id: String,
    pub workspace_id: String,
    pub label: String,
    pub number: u32,
    pub agent_status: AgentStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pane {
    pub pane_id: String,
    pub tab_id: String,
    pub workspace_id: String,
    pub terminal_id: String,
    pub agent_status: AgentStatus,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub terminal_title_stripped: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Layout {
    pub tab_id: String,
    pub zoomed: bool,
    pub focused_pane_id: String,
    pub area: Rect,
    pub panes: Vec<LayoutPane>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutPane {
    pub pane_id: String,
    pub focused: bool,
    pub rect: Rect,
}

/// Terminal cells, in herdr's own client geometry.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frac {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaneFrame {
    pub pane_id: String,
    pub frame: Frac,
    pub focused: bool,
}

impl Snapshot {
    pub fn from_result(result: &serde_json::Value) -> anyhow::Result<Self> {
        Ok(Self::deserialize(&result["snapshot"])?)
    }

    // ponytail: fractions of herdr's cell rects; plan 2 builds a real split tree (dividers, set_split_ratio paths).
    pub fn pane_frames(&self, tab_id: &str) -> Vec<PaneFrame> {
        let Some(layout) = self.layouts.iter().find(|l| l.tab_id == tab_id) else { return Vec::new() };
        if layout.zoomed {
            let full = Frac { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
            return vec![PaneFrame { pane_id: layout.focused_pane_id.clone(), frame: full, focused: true }];
        }
        let (aw, ah) = (f32::from(layout.area.width), f32::from(layout.area.height));
        if aw == 0.0 || ah == 0.0 {
            return Vec::new();
        }
        layout
            .panes
            .iter()
            .filter(|p| p.rect.width > 0 && p.rect.height > 0)
            .map(|p| PaneFrame {
                pane_id: p.pane_id.clone(),
                frame: Frac {
                    x: f32::from(p.rect.x - layout.area.x) / aw,
                    y: f32::from(p.rect.y - layout.area.y) / ah,
                    w: f32::from(p.rect.width) / aw,
                    h: f32::from(p.rect.height) / ah,
                },
                focused: p.focused,
            })
            .collect()
    }
}

/// A Shepherd started inside a herdr pane must not mirror that pane into itself.
pub fn attachable(pane_id: &str, own_pane: Option<&str>) -> bool {
    own_pane != Some(pane_id)
}
```

- [ ] **Step 5: Run the tests**

Run: `env -u MACOSX_DEPLOYMENT_TARGET cargo test --test model`
Expected: 7 passed.

- [ ] **Step 6: Check the parser against the live server**

Append to `tests/model.rs`:
```rust
/// `herdr api snapshot | jq .result > /tmp/live.json && SHEPHERD_LIVE_SNAPSHOT=/tmp/live.json cargo test --test model -- --ignored`
#[test]
#[ignore]
fn parses_live_snapshot() {
    let path = std::env::var("SHEPHERD_LIVE_SNAPSHOT").expect("SHEPHERD_LIVE_SNAPSHOT");
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let s = Snapshot::from_result(&value).unwrap();
    for tab in &s.tabs {
        let _ = s.pane_frames(&tab.tab_id);
    }
}
```
Run: `herdr api snapshot | jq .result > /tmp/live.json && SHEPHERD_LIVE_SNAPSHOT=/tmp/live.json env -u MACOSX_DEPLOYMENT_TARGET cargo test --test model -- --ignored`
Expected: PASS. Never commit `/tmp/live.json` (real paths).

- [ ] **Step 7: Commit**

```bash
git add src/lib.rs src/model.rs tests/model.rs tests/fixtures/snapshot.json
git commit -m "model: session snapshot and fractional pane frames"
```

---

### Task 4: The attach shim

**Files:**
- Create: `src/attach.rs`, `tests/attach.rs`
- Modify: `src/lib.rs`, `src/main.rs`

**Interfaces:**
- Produces:
  - `shepherd::attach::run(pane: &str) -> anyhow::Result<()>` — the whole shim; blocks until the stream ends.
  - CLI: `shepherd attach <pane>`; the herdr binary is `SHEPHERD_HERDR` or `herdr` on `PATH`.
  - `shepherd::attach::command_line(exe: &Path, pane: &str) -> String` — the shell-quoted command a Ghostty surface runs.

- [ ] **Step 1: Write the failing test**

`tests/attach.rs`:
```rust
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

#[test]
fn shim_bridges_frames_input_and_exit() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("herdr");
    std::fs::write(
        &fake,
        r#"#!/bin/sh
echo "$@" > "$REC/args"
printf '%s\n' '{"type":"terminal.frame","seq":1,"encoding":"ansi","width":80,"height":24,"full":true,"bytes":"aGVsbG8="}'
cat > "$REC/stdin"
printf '%s\n' '{"type":"terminal.closed","reason":null}'
"#,
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_shepherd"))
        .args(["attach", "w1:p1"])
        .env("SHEPHERD_HERDR", &fake)
        .env("REC", dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"ls\r").unwrap(); // dropped here: EOF
    let out = child.wait_with_output().unwrap();

    assert!(out.status.success());
    assert_eq!(out.stdout, b"hello");
    let args = std::fs::read_to_string(dir.path().join("args")).unwrap();
    assert_eq!(args.trim(), "terminal session control w1:p1 --cols 80 --rows 24");
    let input = std::fs::read_to_string(dir.path().join("stdin")).unwrap();
    assert_eq!(input, "{\"type\":\"terminal.input\",\"bytes\":\"bHMN\"}\n");
}

#[test]
fn command_line_quotes_paths() {
    let line = shepherd::attach::command_line(std::path::Path::new("/Apps/My App/shepherd"), "w1:p1");
    assert_eq!(line, "'/Apps/My App/shepherd' attach 'w1:p1'");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `env -u MACOSX_DEPLOYMENT_TARGET cargo test --test attach`
Expected: FAIL to compile, `could not find attach in shepherd`.

- [ ] **Step 3: Implement the shim**

`src/lib.rs`:
```rust
pub mod attach;
pub mod ghostty;
pub mod herdr;
pub mod model;
```

`src/main.rs`:
```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let [_, cmd, pane] = args.as_slice() {
        if cmd == "attach" {
            if let Err(err) = shepherd::attach::run(pane) {
                eprintln!("shepherd attach: {err:#}");
                std::process::exit(1);
            }
            return;
        }
    }
}
```

`src/attach.rs`:
```rust
//! `shepherd attach <pane>`: the process inside a Ghostty surface. Bridges the surface PTY to
//! `herdr terminal session control` (JSON lines: base64 ANSI frames out, input/resize in).
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

type Sink = Arc<Mutex<Option<ChildStdin>>>;

pub fn command_line(exe: &Path, pane: &str) -> String {
    format!("{} attach {}", quote(&exe.to_string_lossy()), quote(pane))
}

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

pub fn run(pane: &str) -> anyhow::Result<()> {
    let herdr = std::env::var("SHEPHERD_HERDR").unwrap_or_else(|_| "herdr".into());
    let size = winsize().unwrap_or(Size { cols: 80, rows: 24, cell_w: 0, cell_h: 0 });
    let mut child = Command::new(herdr)
        .args(["terminal", "session", "control", pane])
        .args(["--cols", &size.cols.to_string(), "--rows", &size.rows.to_string()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let sink: Sink = Arc::new(Mutex::new(child.stdin.take()));
    let _raw = RawMode::enable();

    let input = sink.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        let mut stdin = std::io::stdin().lock();
        while let Ok(n @ 1..) = stdin.read(&mut buf) {
            send(&input, json!({"type": "terminal.input", "bytes": B64.encode(&buf[..n])}));
        }
        input.lock().unwrap().take(); // EOF: close herdr's stdin so it ends the session
    });

    watch_resize(sink.clone());

    let mut stdout = std::io::stdout().lock();
    for line in BufReader::new(child.stdout.take().unwrap()).lines() {
        let Ok(record) = serde_json::from_str::<Value>(&line?) else { continue };
        match record["type"].as_str() {
            Some("terminal.frame") => {
                stdout.write_all(&B64.decode(record["bytes"].as_str().unwrap_or(""))?)?;
                stdout.flush()?;
            }
            Some("terminal.closed") => break,
            _ => {}
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

fn send(sink: &Sink, record: Value) {
    if let Some(stdin) = sink.lock().unwrap().as_mut() {
        let _ = writeln!(stdin, "{record}");
    }
}

static WINCH: AtomicBool = AtomicBool::new(false);

extern "C" fn on_winch(_: libc::c_int) {
    WINCH.store(true, Ordering::Relaxed);
}

// ponytail: a flag polled every 50 ms; a self-pipe if resize latency ever matters.
fn watch_resize(sink: Sink) {
    unsafe { libc::signal(libc::SIGWINCH, on_winch as libc::sighandler_t) };
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(50));
        if WINCH.swap(false, Ordering::Relaxed) {
            if let Some(s) = winsize() {
                send(&sink, json!({"type": "terminal.resize", "cols": s.cols, "rows": s.rows,
                                   "cell_width_px": s.cell_w, "cell_height_px": s.cell_h}));
            }
        }
    });
}

struct Size {
    cols: u16,
    rows: u16,
    cell_w: u32,
    cell_h: u32,
}

fn winsize() -> Option<Size> {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    if unsafe { libc::ioctl(0, libc::TIOCGWINSZ, &mut ws) } != 0 || ws.ws_col == 0 {
        return None;
    }
    Some(Size {
        cols: ws.ws_col,
        rows: ws.ws_row,
        cell_w: u32::from(ws.ws_xpixel) / u32::from(ws.ws_col),
        cell_h: u32::from(ws.ws_ypixel) / u32::from(ws.ws_row.max(1)),
    })
}

/// Raw mode on the surface PTY so every byte (ctrl keys, escape sequences) reaches herdr untouched.
struct RawMode(libc::termios);

impl RawMode {
    fn enable() -> Option<Self> {
        unsafe {
            if libc::isatty(0) == 0 {
                return None;
            }
            let mut t: libc::termios = std::mem::zeroed();
            libc::tcgetattr(0, &mut t);
            let saved = t;
            libc::cfmakeraw(&mut t);
            libc::tcsetattr(0, libc::TCSANOW, &t);
            Some(Self(saved))
        }
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        unsafe { libc::tcsetattr(0, libc::TCSANOW, &self.0) };
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `env -u MACOSX_DEPLOYMENT_TARGET cargo test --test attach`
Expected: 2 passed.

- [ ] **Step 5: Try it in a real terminal**

In a Ghostty window outside herdr, with herdr running, pick a pane that is not the current one (`herdr pane list`) and run: `env -u MACOSX_DEPLOYMENT_TARGET cargo run -- attach w1:p1`
Expected: that pane's screen appears; typing reaches it; resizing the Ghostty window reflows it; closing the pane in herdr ends the command. Note anything wrong for the spike report (Task 7).

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/main.rs src/attach.rs tests/attach.rs
git commit -m "attach: shim between a surface PTY and herdr terminal session control"
```

---

### Task 5: Ghostty runtime and the surface view

**Files:**
- Create: `src/ghostty/app.rs`, `src/ghostty/surface.rs`, `src/ghostty/view.rs`
- Modify: `src/ghostty/mod.rs`

**Interfaces:**
- Consumes: `ffi::*` (Task 1), `attach::command_line` (Task 4).
- Produces:
  - `ghostty::App::init(cx: &mut gpui::App) -> ghostty::App` — calls `ghostty_init`, loads the user's Ghostty config, creates the app, and drives `ghostty_app_tick` from a GPUI foreground task on every wakeup. `App` is `Copy`.
  - `ghostty::Surface::new(app: App, window: &mut gpui::Window, pane_id: &str) -> anyhow::Result<Surface>` — creates the view under GPUI's overlay and a surface running `shepherd attach <pane_id>`.
  - `surface.set_bounds(bounds: gpui::Bounds<gpui::Pixels>, window: &gpui::Window)`; `surface.focus()`; `Drop` frees the surface and removes the view.

This task is the core of spike question 1. If an objc2 or GPUI signature differs from what is written here, follow the compiler for these pinned versions; the behavior described is what matters. Zeron's `crates/ui/src/browser/macos.rs` is the working reference for hosting a native view in a zui window.

- [ ] **Step 1: Write the app runtime**

`src/ghostty/mod.rs`:
```rust
mod app;
pub mod ffi;
mod surface;
mod view;

pub use app::App;
pub use surface::Surface;
```

`src/ghostty/app.rs`:
```rust
use super::ffi;
use futures::StreamExt;
use futures::channel::mpsc;
use std::ffi::c_void;
use std::sync::OnceLock;

static WAKE: OnceLock<mpsc::UnboundedSender<()>> = OnceLock::new();

#[derive(Clone, Copy)]
pub struct App(pub(crate) ffi::ghostty_app_t);

extern "C" fn wakeup(_: *mut c_void) {
    if let Some(tx) = WAKE.get() {
        let _ = tx.unbounded_send(());
    }
}

extern "C" fn action(_: ffi::ghostty_app_t, _: ffi::ghostty_target_s, action: ffi::ghostty_action_s) -> bool {
    // ponytail: plan 2 maps new_split / goto_split / close_surface / new_tab… to herdr calls here.
    eprintln!("ghostty action {}", action.tag);
    false
}

extern "C" fn read_clipboard(_: *mut c_void, _: ffi::ghostty_clipboard_e, _: *mut c_void) -> bool {
    false // paste: plan 2 (ghostty_surface_complete_clipboard_request with the pasteboard text)
}

extern "C" fn confirm_read_clipboard(_: *mut c_void, _: *const i8, _: *mut c_void, _: ffi::ghostty_clipboard_request_e) {}

extern "C" fn write_clipboard(
    _: *mut c_void,
    _: ffi::ghostty_clipboard_e,
    content: *const ffi::ghostty_clipboard_content_s,
    len: usize,
    _: bool,
) {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::NSString;
    if len == 0 || content.is_null() {
        return;
    }
    let data = unsafe { (*content).data };
    if data.is_null() {
        return;
    }
    let text = unsafe { std::ffi::CStr::from_ptr(data) }.to_string_lossy();
    unsafe {
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        pb.setString_forType(&NSString::from_str(&text), NSPasteboardTypeString);
    }
}

extern "C" fn close_surface(_: *mut c_void, _: bool) {}

impl App {
    pub fn init(cx: &mut gpui::App) -> Self {
        unsafe {
            assert_eq!(ffi::ghostty_init(0, std::ptr::null_mut()), 0, "ghostty_init");
            let config = ffi::ghostty_config_new();
            ffi::ghostty_config_load_default_files(config);
            ffi::ghostty_config_finalize(config);
            let runtime = ffi::ghostty_runtime_config_s {
                userdata: std::ptr::null_mut(),
                supports_selection_clipboard: false,
                wakeup_cb: Some(wakeup),
                action_cb: Some(action),
                read_clipboard_cb: Some(read_clipboard),
                confirm_read_clipboard_cb: Some(confirm_read_clipboard),
                write_clipboard_cb: Some(write_clipboard),
                close_surface_cb: Some(close_surface),
            };
            let raw = ffi::ghostty_app_new(&runtime, config);
            assert!(!raw.is_null(), "ghostty_app_new");
            let (tx, mut rx) = mpsc::unbounded();
            WAKE.set(tx).ok();
            let app = App(raw);
            cx.spawn(async move |_| {
                while rx.next().await.is_some() {
                    ffi::ghostty_app_tick(app.0);
                }
            })
            .detach();
            app
        }
    }
}
```

- [ ] **Step 2: Write the surface NSView (input)**

`src/ghostty/view.rs`:
```rust
//! Our NSView for a Ghostty surface: libghostty draws into its layer; we translate AppKit input.
use super::ffi;
use objc2::rc::Retained;
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect};
use std::cell::Cell;
use std::ffi::CString;

pub struct Ivars {
    pub surface: Cell<ffi::ghostty_surface_t>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "ShepherdSurfaceView"]
    #[ivars = Ivars]
    pub struct SurfaceView;

    impl SurfaceView {
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool { true }

        #[unsafe(method(becomeFirstResponder))]
        fn become_first_responder(&self) -> bool {
            self.with_surface(|s| unsafe { ffi::ghostty_surface_set_focus(s, true) });
            true
        }

        #[unsafe(method(resignFirstResponder))]
        fn resign_first_responder(&self) -> bool {
            self.with_surface(|s| unsafe { ffi::ghostty_surface_set_focus(s, false) });
            true
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            let action = if unsafe { event.isARepeat() } { ffi::GHOSTTY_ACTION_REPEAT } else { ffi::GHOSTTY_ACTION_PRESS };
            self.key(event, action, text_of(event));
        }

        #[unsafe(method(keyUp:))]
        fn key_up(&self, event: &NSEvent) { self.key(event, ffi::GHOSTTY_ACTION_RELEASE, None); }

        #[unsafe(method(flagsChanged:))]
        fn flags_changed(&self, event: &NSEvent) {
            // A modifier went down if its flag is now set; ghostty needs both edges.
            let flag = match unsafe { event.keyCode() } {
                0x38 | 0x3C => NSEventModifierFlags::Shift,
                0x3B | 0x3E => NSEventModifierFlags::Control,
                0x3A | 0x3D => NSEventModifierFlags::Option,
                0x37 | 0x36 => NSEventModifierFlags::Command,
                0x39 => NSEventModifierFlags::CapsLock,
                _ => return,
            };
            let down = unsafe { event.modifierFlags() }.contains(flag);
            self.key(event, if down { ffi::GHOSTTY_ACTION_PRESS } else { ffi::GHOSTTY_ACTION_RELEASE }, None);
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            if let Some(window) = self.window() { window.makeFirstResponder(Some(self)); }
            self.button(event, ffi::GHOSTTY_MOUSE_PRESS, ffi::GHOSTTY_MOUSE_LEFT);
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) { self.button(event, ffi::GHOSTTY_MOUSE_RELEASE, ffi::GHOSTTY_MOUSE_LEFT); }

        #[unsafe(method(rightMouseDown:))]
        fn right_down(&self, event: &NSEvent) { self.button(event, ffi::GHOSTTY_MOUSE_PRESS, ffi::GHOSTTY_MOUSE_RIGHT); }

        #[unsafe(method(rightMouseUp:))]
        fn right_up(&self, event: &NSEvent) { self.button(event, ffi::GHOSTTY_MOUSE_RELEASE, ffi::GHOSTTY_MOUSE_RIGHT); }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) { self.position(event); }

        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            let precise = unsafe { event.hasPreciseScrollingDeltas() };
            let (mut dx, mut dy) = unsafe { (event.scrollingDeltaX(), event.scrollingDeltaY()) };
            if precise { dx *= 2.0; dy *= 2.0; } // Ghostty's own AppKit view does the same
            self.with_surface(|s| unsafe { ffi::ghostty_surface_mouse_scroll(s, dx, dy, precise as i32) });
        }
    }
);

impl SurfaceView {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(Ivars { surface: Cell::new(std::ptr::null_mut()) });
        unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] }
    }

    fn with_surface(&self, f: impl FnOnce(ffi::ghostty_surface_t)) {
        let s = self.ivars().surface.get();
        if !s.is_null() {
            f(s);
        }
    }

    fn key(&self, event: &NSEvent, action: ffi::ghostty_input_action_e, text: Option<CString>) {
        let unshifted = unsafe { event.charactersIgnoringModifiers() }
            .and_then(|s| s.to_string().chars().next())
            .map_or(0, u32::from);
        let key = ffi::ghostty_input_key_s {
            action,
            mods: mods_of(event),
            consumed_mods: ffi::GHOSTTY_MODS_NONE,
            keycode: u32::from(unsafe { event.keyCode() }),
            text: text.as_ref().map_or(std::ptr::null(), |t| t.as_ptr()),
            unshifted_codepoint: unshifted,
            composing: false,
        };
        self.with_surface(|s| unsafe {
            ffi::ghostty_surface_key(s, key);
        });
    }

    fn button(&self, event: &NSEvent, state: ffi::ghostty_input_mouse_state_e, button: ffi::ghostty_input_mouse_button_e) {
        self.position(event);
        self.with_surface(|s| unsafe {
            ffi::ghostty_surface_mouse_button(s, state, button, mods_of(event));
        });
    }

    fn position(&self, event: &NSEvent) {
        let p: NSPoint = self.convertPoint_fromView(unsafe { event.locationInWindow() }, None);
        let height = self.frame().size.height;
        self.with_surface(|s| unsafe { ffi::ghostty_surface_mouse_pos(s, p.x, height - p.y, mods_of(event)) });
    }
}

// ponytail: no NSTextInputClient yet, so no IME composition (CJK, dead keys); plan 2 adds it.
fn text_of(event: &NSEvent) -> Option<CString> {
    let s = unsafe { event.characters() }?.to_string();
    let c = u32::from(s.chars().next()?);
    // Control characters and AppKit's function-key range: ghostty encodes these from keycode + mods.
    if c < 0x20 || (0xF700..=0xF8FF).contains(&c) {
        return None;
    }
    CString::new(s).ok()
}

fn mods_of(event: &NSEvent) -> ffi::ghostty_input_mods_e {
    let f = unsafe { event.modifierFlags() };
    let mut m = ffi::GHOSTTY_MODS_NONE;
    if f.contains(NSEventModifierFlags::Shift) { m |= ffi::GHOSTTY_MODS_SHIFT; }
    if f.contains(NSEventModifierFlags::Control) { m |= ffi::GHOSTTY_MODS_CTRL; }
    if f.contains(NSEventModifierFlags::Option) { m |= ffi::GHOSTTY_MODS_ALT; }
    if f.contains(NSEventModifierFlags::Command) { m |= ffi::GHOSTTY_MODS_SUPER; }
    if f.contains(NSEventModifierFlags::CapsLock) { m |= ffi::GHOSTTY_MODS_CAPS; }
    m
}
```

- [ ] **Step 3: Write the surface (placement under the GPUI overlay)**

`src/ghostty/surface.rs`:
```rust
use super::{App, ffi, view::SurfaceView};
use gpui::{Bounds, Pixels, Window};
use objc2::rc::Retained;
use objc2_app_kit::{NSView, NSWindowOrderingMode};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::ffi::CString;

pub struct Surface {
    view: Retained<SurfaceView>,
    parent: Retained<NSView>,
    raw: ffi::ghostty_surface_t,
}

/// GPUI's content NSView; native views go below its overlay so GPUI popovers stay on top.
fn parent_view(window: &Window) -> anyhow::Result<Retained<NSView>> {
    let RawWindowHandle::AppKit(handle) = window.window_handle()?.as_raw() else {
        anyhow::bail!("not an AppKit window");
    };
    unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) }.ok_or_else(|| anyhow::anyhow!("no NSView"))
}

impl Surface {
    pub fn new(app: App, window: &mut Window, pane_id: &str) -> anyhow::Result<Self> {
        let mtm = MainThreadMarker::new().ok_or_else(|| anyhow::anyhow!("not on the main thread"))?;
        window.enable_scene_overlay().map_err(|e| anyhow::anyhow!("{e}"))?;
        let parent = parent_view(window)?;
        let view = SurfaceView::new(mtm);
        parent.addSubview(&view);
        for sibling in parent.subviews() {
            if sibling.class().name() == c"GPUIOverlayView" {
                parent.addSubview_positioned_relativeTo(&view, NSWindowOrderingMode::Below, Some(&sibling));
                break;
            }
        }
        let command = CString::new(crate::attach::command_line(&std::env::current_exe()?, pane_id))?;
        let raw = unsafe {
            let mut config = ffi::ghostty_surface_config_new();
            config.platform_tag = ffi::GHOSTTY_PLATFORM_MACOS;
            config.platform.macos.nsview = Retained::as_ptr(&view) as *mut _;
            config.scale_factor = f64::from(window.scale_factor());
            config.command = command.as_ptr();
            ffi::ghostty_surface_new(app.0, &config)
        };
        anyhow::ensure!(!raw.is_null(), "ghostty_surface_new failed for {pane_id}");
        view.ivars().surface.set(raw);
        Ok(Self { view, parent, raw })
    }

    pub fn set_bounds(&self, bounds: Bounds<Pixels>, window: &Window) {
        let (x, y) = (f64::from(f32::from(bounds.origin.x)), f64::from(f32::from(bounds.origin.y)));
        let (w, h) = (f64::from(f32::from(bounds.size.width)).max(1.0), f64::from(f32::from(bounds.size.height)).max(1.0));
        let y = if self.parent.isFlipped() { y } else { self.parent.bounds().size.height - y - h };
        let frame = NSRect::new(NSPoint::new(x, y), NSSize::new(w, h));
        if self.view.frame() != frame {
            self.view.setFrame(frame);
            let scale = f64::from(window.scale_factor());
            unsafe {
                ffi::ghostty_surface_set_content_scale(self.raw, scale, scale);
                ffi::ghostty_surface_set_size(self.raw, (w * scale) as u32, (h * scale) as u32);
            }
        }
    }

    pub fn focus(&self) {
        if let Some(window) = self.view.window() {
            window.makeFirstResponder(Some(&self.view));
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.view.ivars().surface.set(std::ptr::null_mut());
        unsafe { ffi::ghostty_surface_free(self.raw) };
        self.view.removeFromSuperview();
    }
}
```

- [ ] **Step 4: Build**

Run: `env -u MACOSX_DEPLOYMENT_TARGET cargo build`
Expected: compiles (the window that uses these comes in Task 6). Fix signature mismatches against the compiler; do not change behavior.

- [ ] **Step 5: Commit**

```bash
git add src/ghostty
git commit -m "ghostty: app runtime and a surface NSView under the GPUI overlay"
```

---

### Task 6: The window: focused tab, live layout

**Files:**
- Create: `src/shell.rs`
- Modify: `src/main.rs`, `src/lib.rs`

**Interfaces:**
- Consumes: `herdr::{Client, EVENT_TYPES}`, `model::{Snapshot, attachable}`, `ghostty::{App, Surface}`.
- Produces: `shepherd::shell::Shell` (GPUI view) and `Shell::open(app: ghostty::App, rt: tokio::runtime::Handle, cx: &mut gpui::App)`.

- [ ] **Step 1: Write the shell view**

`src/lib.rs`:
```rust
pub mod attach;
pub mod ghostty;
pub mod herdr;
pub mod model;
pub mod shell;
```

`src/shell.rs`:
```rust
//! Plan 1's window: herdr's focused tab, one Ghostty surface per pane, refreshed on every herdr event.
use crate::ghostty::{self, Surface};
use crate::herdr::{Client, EVENT_TYPES};
use crate::model::{Snapshot, attachable};
use gpui::{
    Bounds, Context, IntoElement, ParentElement, Pixels, Render, Styled, Window, WindowOptions, canvas, div,
    prelude::*, px, relative, rgb, size,
};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

pub struct Shell {
    app: ghostty::App,
    snapshot: Option<Snapshot>,
    error: Option<String>,
    surfaces: HashMap<String, Surface>,
    own_pane: Option<String>,
    /// herdr's focused pane we last moved AppKit focus to; refocusing on every paint would undo clicks.
    focused: Option<String>,
}

impl Shell {
    pub fn open(app: ghostty::App, rt: tokio::runtime::Handle, cx: &mut gpui::App) {
        let options = WindowOptions {
            window_bounds: Some(gpui::WindowBounds::Windowed(Bounds::centered(None, size(px(1100.), px(720.)), cx))),
            ..Default::default()
        };
        cx.open_window(options, |_, cx| {
            cx.new(|cx| {
                let shell = Shell {
                    app,
                    snapshot: None,
                    error: None,
                    surfaces: HashMap::new(),
                    own_pane: std::env::var("HERDR_PANE_ID").ok(),
                    focused: None,
                };
                cx.spawn(async move |this, cx| sync_loop(this, rt, cx).await).detach();
                shell
            })
        })
        .expect("open window");
    }

    fn place(&mut self, pane_id: &str, focused: bool, bounds: Bounds<Pixels>, window: &mut Window) {
        if !self.surfaces.contains_key(pane_id) {
            match Surface::new(self.app, window, pane_id) {
                Ok(surface) => {
                    self.surfaces.insert(pane_id.to_string(), surface);
                }
                Err(err) => {
                    self.error = Some(format!("{err:#}"));
                    return;
                }
            }
        }
        let surface = &self.surfaces[pane_id];
        surface.set_bounds(bounds, window);
        if focused && self.focused.as_deref() != Some(pane_id) {
            surface.focus();
            self.focused = Some(pane_id.to_string());
        }
    }
}

/// Subscribe, then snapshot; every event (coalesced) triggers another snapshot. Reconnects forever.
async fn sync_loop(this: gpui::WeakEntity<Shell>, rt: tokio::runtime::Handle, cx: &mut gpui::AsyncApp) {
    let client = Client::local();
    loop {
        let sub = { let c = client.clone(); rt.spawn(async move { c.subscribe(EVENT_TYPES).await }) };
        let mut rx = match sub.await.expect("tokio") {
            Ok(rx) => rx,
            Err(err) => {
                let _ = this.update(cx, |s, cx| { s.error = Some(format!("{err:#}")); cx.notify() });
                cx.background_executor().timer(Duration::from_secs(1)).await;
                continue;
            }
        };
        loop {
            let c = client.clone();
            let result = rt.spawn(async move { c.call("session.snapshot", json!({})).await }).await.expect("tokio");
            let update = result.and_then(|r| Snapshot::from_result(&r));
            if this.update(cx, |s, cx| {
                match update {
                    Ok(snapshot) => { s.snapshot = Some(snapshot); s.error = None; }
                    Err(err) => s.error = Some(format!("{err:#}")),
                }
                cx.notify();
            }).is_err() {
                return; // window closed
            }
            // ponytail: refetch the whole snapshot per event burst; incremental apply only if this shows up in profiles.
            if rx.recv().await.is_none() {
                break; // server gone or events_lost: resubscribe
            }
            cx.background_executor().timer(Duration::from_millis(30)).await;
            while rx.try_recv().is_ok() {}
        }
    }
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let frames = self
            .snapshot
            .as_ref()
            .and_then(|s| s.focused_tab_id.as_deref().map(|t| s.pane_frames(t)))
            .unwrap_or_default();
        self.surfaces.retain(|id, _| frames.iter().any(|f| &f.pane_id == id));
        let view = cx.entity();
        let own = self.own_pane.clone();
        div()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .text_color(rgb(0xcccccc))
            .relative()
            .when_some(self.error.clone(), |el, err| el.child(div().p_4().child(err)))
            .children(frames.into_iter().map(move |f| {
                let view = view.clone();
                let cell = div()
                    .absolute()
                    .left(relative(f.frame.x))
                    .top(relative(f.frame.y))
                    .w(relative(f.frame.w))
                    .h(relative(f.frame.h));
                if !attachable(&f.pane_id, own.as_deref()) {
                    return cell.p_4().child(format!("{} is this Shepherd's own pane", f.pane_id));
                }
                cell.child(
                    canvas(|_, _, _| (), move |bounds, _, window, cx| {
                        view.update(cx, |shell, _| shell.place(&f.pane_id, f.focused, bounds, window));
                    })
                    .size_full(),
                )
            }))
    }
}
```

`src/main.rs`:
```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let [_, cmd, pane] = args.as_slice() {
        if cmd == "attach" {
            if let Err(err) = shepherd::attach::run(pane) {
                eprintln!("shepherd attach: {err:#}");
                std::process::exit(1);
            }
            return;
        }
    }
    let rt = tokio::runtime::Runtime::new().expect("tokio");
    let handle = rt.handle().clone();
    gpui_platform::application().run(move |cx| {
        let app = shepherd::ghostty::App::init(cx);
        shepherd::shell::Shell::open(app, handle.clone(), cx);
        cx.activate(true);
    });
    drop(rt);
}
```

- [ ] **Step 2: Build and run the tests**

Run: `make test`
Expected: all earlier tests still pass; the binary builds.

- [ ] **Step 3: Run it against your herdr (spike questions 1–3)**

Run from a terminal outside herdr: `make run`. Then check, writing each result down for Task 7:
1. The focused tab's panes appear at their places, rendered by Ghostty with your Ghostty font and theme.
2. Clicking a pane focuses it; typing (latin and Russian layouts, ctrl-c, arrows, alt-keys) reaches it; `vim` and Claude Code work full screen; mouse selection and scrolling behave.
3. Cmd+C / Cmd+V inside a surface: does GPUI's view swallow key equivalents before the surface sees them?
4. Resizing the window resizes the panes and reflows their content.
5. In another terminal, `herdr pane split <id> --direction right` and `herdr tab focus <id>`: the window follows within a moment.
6. `herdr server stop`, then start herdr again: the error appears, then the window recovers by itself.
7. With the herdr TUI open on the same tab at a different size: who wins the pane size, and does it fight?
8. Typing latency compared to the same pane in the TUI (subjective is fine; note anything noticeable).
9. Launch Shepherd from inside a herdr pane: its own pane shows the placeholder text.

- [ ] **Step 4: Commit**

```bash
git add src/lib.rs src/main.rs src/shell.rs
git commit -m "shell: herdr's focused tab with a Ghostty surface per pane, synced on events"
```

---

### Task 7: Spike report and spec update

**Files:**
- Create: `docs/spike-results.md`
- Modify: `docs/superpowers/specs/2026-10-10-shepherd-client-design.md` (only where the answers change it)

- [ ] **Step 1: Answer the spec's four spike questions**

`docs/spike-results.md`, one section per question from the spec's "Risks and first spike", each with: what was tried (task and step), what happened (concrete: works / fails, how), the decision (keep, fallback from the spec, or new approach). Include the nine checks from Task 6 Step 3 and the shim check from Task 4 Step 5.

- [ ] **Step 2: Update the spec**

Change only what the answers contradict (for example: moving terminals to child windows, switching the shim to herdr's binary protocol, a size-ownership rule). Leave the rest.

- [ ] **Step 3: Commit**

```bash
git add docs
git commit -m "docs: spike results for the libghostty + herdr approach"
```

Then write Plan 2 from the updated spec.
