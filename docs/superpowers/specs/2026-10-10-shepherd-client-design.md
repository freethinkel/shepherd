# Shepherd: a native macOS client for herdr

Date: 2026-10-10, revised the same day after the libghostty spike (see "Revision" at the end).

## Goal

A desktop client for [herdr](https://github.com/herdrdev/herdr) (Apache-2.0, protocol 22, v0.9.3) that the
author fully controls, built on the same stack as Collie's app (GPUI from `zeronsh/zui`, `gpui-base`, Zeron's
palette and widgets, Geist fonts), with terminals emulated and painted in GPUI. It replaces daily use of the herdr TUI and
of the third-party Herdr GPUI app, and is the base for features those lack: first of all an ACP chat view for
agent panes (a separate spec, after this one).

Success: the author runs their herdr sessions, local and on saved SSH machines, from Shepherd every day: terminals
type, scroll and select like a normal terminal, the window looks like Zeron, and everything the herdr TUI does for workspaces, tabs, panes,
worktrees and agents is reachable from the window.

## Scope

In:
- macOS only (14+, Apple Silicon first).
- Local herdr server and saved SSH machines (`herdr machine list`), each with its own connection.
- Sidebar: machines → workspaces (with worktree grouping) → agents and their status; tab bar; herdr's split layout.
- Terminals: one GPUI terminal view per visible pane (`alacritty_terminal` emulation, Zeron's terminal painter).
- Management: create / close / rename workspaces, tabs and panes; split; focus; resize splits by dragging;
  zoom; drag-and-drop of tabs and panes; create / open / remove worktrees; start agents.
- Chrome theme: Zeron light / dark following the system, as in Collie.

Out (later specs):
- ACP chat view for agent panes.
- Linux and Windows builds (the terminal path is pure Rust + GPUI, so this is packaging, not a redesign).
- Named-session switching beyond what a saved machine profile targets (local uses the default session).
- Browser tabs, annotations, notifications UI beyond agent status, plugins UI, in-app settings.

## Approach

Each visible pane is a GPUI view that Shepherd paints itself. The app spawns `herdr terminal session control
<pane> --cols C --rows R` per pane (JSON lines: `terminal.frame` with base64 ANSI out; `terminal.input`,
`terminal.resize`, `terminal.scroll`, `terminal.mouse` in), feeds the frames into an `alacritty_terminal`
emulator and paints its grid with the terminal element ported from Zeron (`crates/ui/src/terminal`, MIT).

- **herdr owns the terminal state.** Frames are herdr's re-render of the pane's screen, so the local emulator keeps
  no scrollback (history 0). The wheel and Shift+PageUp/PageDown send `terminal.scroll`; herdr scrolls its own
  viewport and the next frames show it.
- **Input** goes through GPUI key events: Zeron's `keystroke_bytes` turns a keystroke into bytes sent as
  `terminal.input`. herdr's frames do not carry application modes (cursor keys, bracketed paste, mouse
  reporting), so Shepherd sends plain xterm encodings and relies on herdr re-encoding input for the pane's
  mode, as its own TUI client does (checked live in the plan).
- **Mouse**: a drag selects locally on the visible grid (Cmd+C copies); clicks and drags are also sent as
  `terminal.mouse`, which herdr drops unless the app asked for mouse reporting.
- **Colors and font**: Zeron's terminal palette (16 ANSI colors per appearance) and Geist Mono; the user's
  Ghostty config is not read.

Why not libghostty (the first spike, `docs/spike-results.md`): its surfaces own scrolling, selection and input in
a native `NSView` beside GPUI, so herdr-side scrollback could not be wired without hacks, every AppKit input path
had to be re-implemented by hand, and Zig 0.15.2 cannot build it on macOS 27.
Why not `herdr terminal attach`: it reserves `ctrl+b` as a detach prefix and is not forwarded to saved machines.

herdr is the single source of truth for layout. Shepherd never changes its own layout model locally: every
action is an API call, and the window updates from the event that follows.

## Architecture

One crate, `shepherd`, in its own cargo workspace (like `collie/desktop/app`). GPUI dependencies pinned to the
same revs as Collie: `gpui`, `gpui_platform` (`font-kit`, `runtime_shaders`), `gpui_tokio` from
`zeronsh/zui@18a89af`, `gpui-base` from `zeronsh/gpui-component@103f13d`.

```
shepherd/
  assets/             Geist + Geist Mono fonts, icons (licenses in assets/NOTICE)
  src/
    main.rs           app entry, window, menus
    herdr/
      transport.rs    a duplex byte stream: unix socket (local) | `ssh <target> herdr remote-api-bridge` stdio
      client.rs       JSON API requests/responses, `events.subscribe`, reconnect with backoff
      machines.rs     `herdr machine list --json`; the Local machine is implicit
      session.rs      one `herdr terminal session control` child per pane: frames in, commands out
    model.rs          Machine → Workspace → Tab → split tree → Pane (+ agent status); parsed from
                      `session.snapshot`, refetched on events; pure, no GPUI
    terminal/
      emulator.rs     alacritty_terminal Term + parser (from Zeron)
      keys.rs         keystroke → bytes, paste encoding (from Zeron)
      element.rs      the grid painter (from Zeron)
      pane.rs         a pane's view: emulator + session + input, scroll and selection handling
    theme.rs          Zeron tokens (light/dark) and terminal palette, as in Collie
    ui/
      window.rs       root view: sidebar | tab bar + pane grid
      sidebar.rs      machines, workspaces, worktree groups, agents with status
      tabs.rs         tab bar, drag-and-drop reorder / move
      panes.rs        split layout → rects, dividers (drag to resize), pane drag-and-drop, zoom
      dialogs.rs      rename, new worktree, start agent, confirm close/remove
```

### Units

- **transport** — gives `AsyncRead + AsyncWrite` for a machine. Local: the API socket path herdr uses
  (`herdr api` resolves it; mirror its logic). Remote: spawn `ssh` with herdr's shared ControlPath options for
  the profile's target, running `herdr remote-api-bridge`.
- **client** — speaks herdr's JSON API over a transport: request ids, responses, one `events.subscribe` stream.
  Exposes `call(method, params)` and a stream of events. On disconnect: exponential backoff up to 2 minutes,
  then `session.snapshot` again on reconnect.
- **model** — the parsed `session.snapshot` per machine. Events (`workspace.*`, `tab.*`, `pane.*`,
  `layout.updated`, …) are invalidation signals: a burst of them triggers one fresh snapshot, as herdr's docs
  require (events and snapshots share no sequence, so events are never replayed onto a snapshot).
  Unit-testable with recorded JSON.
- **session** — spawns `herdr terminal session control <pane> --cols C --rows R` (local) or
  `ssh <target> herdr --session <s> terminal session control …` (remote), decodes `terminal.frame` lines into
  bytes on a reader thread, and writes `terminal.input` / `resize` / `scroll` / `mouse` lines. Ends on
  `terminal.closed` or EOF.
- **terminal** — Zeron's emulator, key encoding and grid painter, plus a pane view that owns one session: frames
  feed the emulator, the element's size drives `resize`, keys / wheel / mouse become session commands.
- **ui** — GPUI views over the model; every user action becomes a `client.call`.

## UI

- **Window**: left sidebar, right the selected workspace: a tab bar and the pane grid of the selected tab.
  Window title follows the focused pane (herdr's pane title). Zeron look: Collie's tokens, Geist, a transparent
  titlebar over the sidebar.
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
- **Keys**: app shortcuts, Ghostty-like defaults: Cmd+D / Cmd+Shift+D split right / down (`pane.split`),
  Cmd+Option+arrows focus a neighbor (`pane.focus_direction`), Cmd+Shift+Enter zoom (`pane.zoom`), Cmd+W close
  pane (`pane.close`), Cmd+T new tab (`tab.create`), Cmd+1…9 go to tab, Cmd+Shift+N new workspace, Cmd+Shift+W
  new worktree, Cmd+Shift+A start agent, Cmd+Up / Cmd+Down previous / next workspace, Cmd+C / Cmd+V copy /
  paste. Every other key goes to the focused pane.

### Pane views

Each pane view is a GPUI entity keyed by pane id. Views of panes no longer visible (other tab, another
workspace) are dropped; returning re-spawns the session, and herdr's first frame is a full screen.

## Errors

- **herdr not installed or not running locally**: an empty state with the reason and a "Start herdr" button
  that runs `herdr server` in the background, then reconnects.
- **Machine unreachable / auth / incompatible**: the machine stays in the sidebar, dimmed, with cached state;
  input into its panes is disabled; badge click shows the error and the CLI command
  (`herdr machine reconnect <id>`, `herdr --remote <target>`). Shepherd never installs or restarts a remote
  server.
- **API call fails**: a toast with herdr's error message; the model is untouched (it only changes on events).
- **Session fails or its stream closes**: the pane shows why (herdr's stderr) with a "Reattach" button. A pane herdr reports closed is removed by the next event anyway.
- **Controller conflict** (another client controls the terminal): attach without `--takeover`; on refusal show
  "Controlled elsewhere — Take over" on the pane, which reattaches with `--takeover`.

## Testing

- **model**: unit tests parsing a `session.snapshot` fixture (shape from herdr 0.9.3, made-up data) and
  asserting the tree and pane frames; this is where most logic lives.
- **client**: tests against a fake JSON API server on a unix socket in a temp dir (request/response, event
  stream, reconnect).
- **session**: a test that runs a session against a fake `herdr` script that emits `terminal.frame` lines and
  records the command lines it gets.
- **terminal**: Zeron's emulator and key-encoding tests, ported with the code.
- **ui**: screenshots of the window against a fake herdr (Collie's `shot.sh` pattern) for sidebar, tabs,
  splits, disconnected machine, dialogs.
- **manual**: a checklist run against a real local herdr and one saved SSH machine.

## Risks

1. Input encoding without the pane's modes (cursor keys, bracketed paste): relies on herdr re-encoding input;
   checked live with vim and Claude Code. Fallback: herdr's binary client protocol (`ControlTerminal` in
   `src/protocol/wire.rs`), whose semantic key events herdr encodes for the pane.
2. Sizing when the herdr TUI or another client shows the same tab (herdr's shared-tab sizing rules).
3. Paint cost of large grids at 60 fps in GPUI (Zeron's element shapes one line per row; measure on a
   full-screen pane).

## Revision

The first plan embedded libghostty surfaces (`NSView`) under the GPUI overlay with a PTY shim in between. It
rendered correctly, but scrolling showed Ghostty's local scrollback of herdr's re-rendered frames instead of
herdr's history, and input, focus, IME and mouse all needed hand-written AppKit translation. The terminal moved
into GPUI (this spec); `docs/spike-results.md` keeps the findings.
