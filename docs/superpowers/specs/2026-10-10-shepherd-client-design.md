# Shepherd: a native macOS client for herdr

Date: 2026-10-10. Status: draft for review.

## Goal

A desktop client for [herdr](https://github.com/herdrdev/herdr) (Apache-2.0, protocol 22, v0.9.3) that the
author fully controls, built on the same stack as Collie's app (GPUI from `zeronsh/zui`, `gpui-base`, Zeron's
palette), with terminals rendered by Ghostty itself through libghostty. It replaces daily use of the herdr TUI and
of the third-party Herdr GPUI app, and is the base for features those lack: first of all an ACP chat view for
agent panes (a separate spec, after this one).

Success: the author runs their herdr sessions, local and on saved SSH machines, from Shepherd every day: terminals
look and type exactly like their Ghostty, and everything the herdr TUI does for workspaces, tabs, panes,
worktrees and agents is reachable from the window.

## Scope

In:
- macOS only (14+, Apple Silicon first).
- Local herdr server and saved SSH machines (`herdr machine list`), each with its own connection.
- Sidebar: machines → workspaces (with worktree grouping) → agents and their status; tab bar; herdr's split layout.
- Terminals: one libghostty surface per visible pane, using the user's Ghostty config.
- Management: create / close / rename workspaces, tabs and panes; split; focus; resize splits by dragging;
  zoom; drag-and-drop of tabs and panes; create / open / remove worktrees; start agents.
- Chrome theme: Zeron light / dark following the system, as in Collie.

Out (later specs):
- ACP chat view for agent panes.
- Linux and Windows (a libghostty-vt + GPUI painter variant goes there).
- Named-session switching beyond what a saved machine profile targets (local uses the default session).
- Browser tabs, annotations, notifications UI beyond agent status, plugins UI, in-app settings.

## Approach

Terminals are native libghostty surfaces (Metal, `NSView`), one per visible pane, laid over the GPUI window at
rects GPUI computes. A surface's process is Shepherd's own shim, `shepherd attach <machine> <pane>`, which
bridges the surface's PTY to `herdr terminal session control <pane>` (JSON lines: `terminal.frame` with base64
ANSI out; `terminal.input`, `terminal.resize`, `terminal.scroll`, `terminal.mouse` in). The herdr server keeps
the authoritative terminal state (its own ghostty-vt); Ghostty re-parses the ANSI it emits, like a terminal
under tmux.

Why not `herdr terminal attach`: it reserves `ctrl+b` as a detach prefix and is not forwarded to saved machines.
Why not a GPUI-painted terminal (libghostty-vt + own renderer): that is writing a terminal renderer, which is
the main thing this project wants from Ghostty; it stays the plan for Linux and Windows.

herdr is the single source of truth for layout. Shepherd never changes its own layout model locally: every
action is an API call, and the window updates from the event that follows.

## Architecture

One crate, `shepherd`, in its own cargo workspace (like `collie/desktop/app`). GPUI dependencies pinned to the
same revs as Collie: `gpui`, `gpui_platform` (`font-kit`, `runtime_shaders`), `gpui_tokio` from
`zeronsh/zui@18a89af`, `gpui-base` from `zeronsh/gpui-component@103f13d`.

```
shepherd/
  Makefile            build libghostty (zig 0.15.2, pinned ghostty commit) → vendor/ghostty/{libghostty.a,ghostty.h}
  build.rs            link libghostty and its macOS frameworks
  src/
    main.rs           app entry; `shepherd attach …` subcommand dispatch
    herdr/
      transport.rs    a duplex byte stream: unix socket (local) | `ssh <target> herdr remote-api-bridge` stdio
      client.rs       JSON API requests/responses, `events.subscribe`, reconnect with backoff
      machines.rs     `herdr machine list --json`; the Local machine is implicit
    model.rs          Machine → Workspace → Tab → split tree → Pane (+ agent status); built from
                      `session.snapshot`, updated by events; pure, no GPUI
    ghostty/
      ffi.rs          hand-written bindings for the subset of ghostty.h used
      app.rs          ghostty app + config (`ghostty_config_load_default_files`), runtime callbacks
      surface.rs      surface lifecycle; `NSView` placement inside the GPUI window
      actions.rs      ghostty actions → herdr commands
    attach.rs         the shim: PTY ⇄ `herdr terminal session control` (local) or the same over ssh
    ui/
      window.rs       root view: sidebar | tab bar + pane grid
      sidebar.rs      machines, workspaces, worktree groups, agents with status
      tabs.rs         tab bar, drag-and-drop reorder / move
      panes.rs        split layout → rects, dividers (drag to resize), pane drag-and-drop, zoom
      dialogs.rs      rename, new worktree, start agent, confirm close/remove
      theme.rs        Zeron palette (light/dark), copied from Collie
```

### Units

- **transport** — gives `AsyncRead + AsyncWrite` for a machine. Local: the API socket path herdr uses
  (`herdr api` resolves it; mirror its logic). Remote: spawn `ssh` with herdr's shared ControlPath options for
  the profile's target, running `herdr remote-api-bridge`.
- **client** — speaks herdr's JSON API over a transport: request ids, responses, one `events.subscribe` stream.
  Exposes `call(method, params)` and a stream of events. On disconnect: exponential backoff up to 2 minutes,
  then `session.snapshot` again on reconnect.
- **model** — applies a snapshot and events (`workspace.*`, `tab.*`, `pane.*`, `layout.updated`,
  `pane.agent_status_changed`, …) to an immutable tree per machine. Unit-testable with recorded JSON.
- **ghostty** — owns the single `ghostty_app_t`, creates a surface per visible pane with `command` set to the
  shim, places its `NSView` at the pane rect, forwards focus, and routes actions.
- **attach (shim)** — runs as the surface's child process. Puts its stdin in raw mode, spawns
  `herdr terminal session control <pane> --cols C --rows R` (local) or `ssh -t <target> herdr --session <s>
  terminal session control …` (remote), writes decoded `terminal.frame` bytes to stdout, wraps stdin bytes as
  `terminal.input`, turns SIGWINCH into `terminal.resize`, exits on `terminal.closed`.
- **ui** — GPUI views over the model; every user action becomes a `client.call`.

## UI

- **Window**: left sidebar, right the selected workspace: a tab bar and the pane grid of the selected tab.
  Window title follows the focused pane (`GHOSTTY_ACTION_SET_TITLE` and herdr's pane title).
- **Sidebar**: machines as collapsible sections (Local first), each with its workspaces; worktree workspaces
  grouped under their repository's workspace; under each workspace its agents with a status dot (working,
  waiting / needs you, done, idle) from herdr's agent state. A disconnected machine is dimmed with a status
  badge (reconnecting / auth / error) whose click shows the last error and the CLI command to fix it.
  Context menu: rename, close, new tab, new worktree, start agent.
- **Tab bar**: tabs of the selected workspace; click to focus, double-click to rename, drag to reorder or onto a
  workspace in the sidebar to move; `+` makes a new tab.
- **Pane grid**: herdr's split tree rendered as nested flex rows/columns with 1-px dividers; dragging a divider
  calls `layout.set_split_ratio`; dragging a pane's header onto another pane's edge moves it (`pane.move`),
  onto its center swaps (`pane.swap`). The focused pane has an accent border. Zoom shows one pane full size.
- **Dialogs**: rename (inline text field), new worktree (branch name, base), start agent (kind from
  `server.agent_manifests`, optional prompt), confirm closing a pane with a running process and removing a
  worktree.
- **Keys**: inside a terminal, Ghostty handles keys with the user's Ghostty keybinds. Ghostty actions map to
  herdr: `new_split` → `pane.split` (direction kept), `goto_split` → `pane.focus_direction`, `resize_split` →
  `pane.resize`, `equalize_splits` → `layout.apply` of an equalized tree, `toggle_split_zoom` → `pane.zoom`,
  `close_surface` → `pane.close`, `new_tab` → `tab.create`, `goto_tab` → `tab.focus`, `close_tab` → `tab.close`,
  `new_window` → `workspace.create`. App-menu shortcuts cover what Ghostty has no action for: new workspace
  (Cmd+Shift+N), new worktree (Cmd+Shift+W), start agent (Cmd+Shift+A), previous / next machine-workspace
  (Cmd+Up / Cmd+Down), rename tab (Cmd+Shift+R).

### Surfaces and GPUI

GPUI lays out an empty placeholder element per pane and reports its bounds after layout. Shepherd keeps a
surface per pane id and moves its `NSView` to those bounds; views of panes no longer visible (other tab,
another workspace) are hidden, not destroyed, for a bounded set (the current tab plus the last 8 used tabs);
older ones are freed and recreated on return, which costs one reattach and a redraw from the server.
Popovers, menus and dialogs render above native views through the zui fork's deferred compositing over
native macOS views; anything that cannot is shown as a sheet or a separate window.

## Errors

- **herdr not installed or not running locally**: an empty state with the reason and a "Start herdr" button
  that runs `herdr server` in the background, then reconnects.
- **Machine unreachable / auth / incompatible**: the machine stays in the sidebar, dimmed, with cached state;
  input into its panes is disabled; badge click shows the error and the CLI command
  (`herdr machine reconnect <id>`, `herdr --remote <target>`). Shepherd never installs or restarts a remote
  server.
- **API call fails**: a toast with herdr's error message; the model is untouched (it only changes on events).
- **Shim fails or its stream closes**: the surface shows Ghostty's process-exited state; the pane gets a
  "Reattach" overlay. A pane herdr reports closed is removed by the next event anyway.
- **Controller conflict** (another client controls the terminal): attach without `--takeover`; on refusal show
  "Controlled elsewhere — Take over" on the pane, which reattaches with `--takeover`.

## Testing

- **model**: unit tests applying recorded `session.snapshot` and event JSON (captured from a real herdr 0.9.3)
  and asserting the tree; this is where most logic lives.
- **client**: tests against a fake JSON API server on a unix socket in a temp dir (request/response, event
  stream, reconnect).
- **attach**: a test that runs the shim against a fake `herdr` script on `PATH` that emits `terminal.frame`
  lines and records `terminal.input` / `terminal.resize`.
- **ui**: screenshots of the window against a fake herdr (Collie's `shot.sh` pattern) for sidebar, tabs,
  splits, disconnected machine, dialogs.
- **manual**: a checklist run against a real local herdr and one saved SSH machine.

## Risks and first spike

The plan starts with a throwaway spike that answers these before anything else is built:

1. A libghostty surface `NSView` inside a GPUI (zui) window: placement at GPUI rects, resize, keyboard focus
   moving between GPUI views and the surface, IME, z-order of GPUI popovers above it.
2. `herdr terminal session control` driven by the shim: latency of typing, full-screen apps (vim, Claude Code),
   mouse, scrollback (herdr-side `terminal.scroll` vs Ghostty's own scrollback), resize, local and over ssh.
3. Sizing when the herdr TUI or another client shows the same tab (herdr's shared-tab sizing rules).
4. Building libghostty with zig at a pinned commit and linking it into a cargo build (Collie's and
   `flutter_ghostty`'s experience apply).

If (1) fails, terminals move to separate child windows or the GPUI-painted variant; if (2) is too slow or lossy,
the shim switches to herdr's binary client protocol (`ControlTerminal` in `src/protocol/wire.rs`).
