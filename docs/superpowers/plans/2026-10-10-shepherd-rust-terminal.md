# Shepherd: Rust Terminal and Zeron Look (Plan 2a) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the libghostty surfaces with GPUI-painted terminals (`alacritty_terminal` + Zeron's terminal code) fed straight from `herdr terminal session control`, with herdr-side scrolling, and restyle the window in Zeron's look as Collie does.

**Architecture:** Per visible pane the app spawns `herdr terminal session control <pane>`; a reader thread decodes `terminal.frame` lines into bytes; a `PaneTerminal` GPUI entity feeds them into Zeron's `Emulator` (no local scrollback) and paints it with Zeron's `TerminalElement`. Keys, wheel and mouse become `terminal.input` / `terminal.scroll` / `terminal.mouse` lines. Chrome uses Collie's Zeron tokens and Geist fonts.

**Tech Stack:** Rust 2024, GPUI (`zeronsh/zui@18a89af`), `alacritty_terminal` 0.26, tokio, serde_json, base64; herdr 0.9.3.

**Spec:** `docs/superpowers/specs/2026-10-10-shepherd-client-design.md` (revised: Approach, Architecture, Risks, Revision)

Out of this plan (next plan): management actions and their shortcuts (split, close, new tab/workspace, rename, drag-and-drop, worktrees, start agent), saved SSH machines, dividers that drag, `.app` bundling.

## Global Constraints

- Everything in the repository is in English; no personal data in fixtures.
- GPUI deps exactly as Collie's app: `gpui`, `gpui_platform` (`font-kit`, `runtime_shaders`) from `https://github.com/zeronsh/zui` rev `18a89af04cfe079d55275b111ac349d5640d9cda`.
- Code taken from Zeron (`/Users/freethinkel/Developer/forks/zeron`, MIT, Copyright (c) 2026 Wing) keeps a header comment naming its source file, and `assets/NOTICE` carries the MIT notice (copy Collie's).
- Build with `env -u MACOSX_DEPLOYMENT_TARGET cargo …`.
- herdr owns layout and terminal state: no local layout edits, no local scrollback.
- Never run a herdr command against the user's own panes in tests; live checks use a scratch tab created with `--no-focus` and closed afterwards.

## Review Focus

1. A pane whose session ends (pane closed, herdr down, attach refused) shows why and does not spin re-spawning children. → Task 5 test `closed_session_reports_end`; Task 7 renders the ended state.
2. Wheel deltas smaller than a line (trackpads) must accumulate, not be dropped or each turned into a full line. → Task 6 test `wheel_lines_accumulate_fractions`.
3. Typing Cyrillic and other non-ASCII text arrives as UTF-8 bytes; ctrl/alt chords produce the xterm bytes. → Task 4 ported tests plus `cyrillic_key_char_is_utf8`.
4. A session child must not outlive its pane view (no orphaned `herdr terminal session control` processes after switching tabs or quitting). → Task 5 test `dropping_session_kills_child`.
5. Shepherd started inside a herdr pane still shows its own pane as a placeholder, not a mirror. → kept from plan 1 (`model::attachable`), Task 7 keeps the check.

---

### Task 1: Remove the libghostty path

**Files:**
- Delete: `src/ghostty/`, `src/attach.rs`, `build.rs`, `tests/ghostty_link.rs`, `tests/attach.rs`, `Makefile`
- Modify: `Cargo.toml`, `src/lib.rs`, `src/main.rs`, `src/shell.rs`, `.gitignore`

**Interfaces:**
- Produces: a crate with `herdr`, `model`, `shell` only; `shell::Shell` renders panes as plain placeholder boxes until Task 7.

- [ ] **Step 1: Delete files and deps**

`git rm -r src/ghostty src/attach.rs build.rs tests/ghostty_link.rs tests/attach.rs Makefile`. In `Cargo.toml` remove `objc2`, `objc2-app-kit`, `objc2-foundation`, `raw-window-handle`, `libc`, `[build-dependencies]`/`bindgen`; add `alacritty_terminal = "0.26"`. In `.gitignore` remove `/.ghostty-src` and `/vendor`. `src/lib.rs`: `pub mod herdr; pub mod model; pub mod shell;`. `src/main.rs`: drop the `attach` dispatch and `ghostty::App::init`; `Shell::open(handle, cx)` takes no ghostty app.

- [ ] **Step 2: Make shell.rs compile without surfaces**

Remove `Surface`, `place`, `prune`'s surface map and the canvas; each pane cell renders `div().size_full().child(pane_id)` for now. Keep `prune`'s tests by making `prune` generic as it is and still used for the `focused` bookkeeping in Task 7 — if it becomes unused here, keep it with `#[allow(dead_code)]` until Task 7 wires it.

- [ ] **Step 3: Run the suite**

Run: `env -u MACOSX_DEPLOYMENT_TARGET cargo test`
Expected: model (11 + 1 ignored), herdr_client (5), shell (3) pass; no ghostty or attach tests remain.

- [ ] **Step 4: Commit** — `git commit -m "drop libghostty: the terminal moves into GPUI"`

---

### Task 2: Zeron theme and Geist fonts

**Files:**
- Create: `src/theme.rs`, `src/assets.rs`, `assets/fonts/` (Geist.ttf, Geist-Medium.ttf, Geist-SemiBold.ttf, GeistMono.ttf, Geist-OFL.txt copied from `../collie/desktop/app/assets/fonts/`), `assets/NOTICE`
- Modify: `src/lib.rs`, `src/main.rs`

**Interfaces:**
- Produces: `theme::Theme` (gpui `Global`) with `Theme::for_appearance(gpui::WindowAppearance) -> Theme`, `Theme::of(cx: &App) -> &Theme`, fields:
  `appearance: Appearance` (`Dark | Light`), `surface, card, text, text_muted, text_faint, accent, hover, border, border_strong, selected: Hsla`, `cursor: Hsla`, `font_ui: SharedString` ("Geist"), `font_terminal: SharedString` ("Geist Mono"), `terminal_font_size: f32` (13.0), `terminal: TerminalPalette { background, foreground, selection: Hsla, ansi: [Hsla; 16] }`.
  `rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32)` (Zeron's element needs it).
- Produces: `assets::register_fonts(cx: &mut App)`.

- [ ] **Step 1: Failing tests** in `src/theme.rs` `#[cfg(test)]`:

```rust
#[test]
fn dark_terminal_matches_zeron_dark() {
    let t = Theme::for_appearance(gpui::WindowAppearance::Dark);
    assert_eq!(t.terminal.background, hex(0x090909));
    assert_eq!(t.terminal.ansi[1], hex(0xf87171));
    assert_eq!(t.terminal.ansi[15], hex(0xfafafa));
}

#[test]
fn light_terminal_matches_zeron_light() {
    let t = Theme::for_appearance(gpui::WindowAppearance::Light);
    assert_eq!(t.terminal.background, hex(0xfafafa));
    assert_eq!(t.terminal.ansi[1], hex(0xdc2626));
    assert_eq!(t.appearance, Appearance::Light);
}
```
Run `cargo test --lib theme` → FAIL (no module).

- [ ] **Step 2: Implement** `src/theme.rs`: chrome tokens copied from Collie's `desktop/app/src/theme.rs` `dark()`/`light()` (surface, card, text, text_muted, text_faint, hover, border, border_strong, `selected()` as a field), accent = Zeron's (`#8b7cf6` dark, `#5b43e8` light); terminal from Zeron `crates/theme/src/builtins.rs`: background `#090909` / `#fafafa`, ANSI_DARK / ANSI_LIGHT (16 hex values each, copy verbatim), foreground = text, selection = accent at 0.30 alpha, cursor = text. `rgb_to_hsl` copied from Zeron `crates/ui/src/theme.rs`. Header comment: "Tokens from Collie (adapted from Zeron, MIT) and Zeron's terminal palette (crates/theme/src/builtins.rs)."
`src/assets.rs`: Collie's `FONTS` + `register_fonts` for the four Geist files (no icons yet).
`main.rs`: `assets::register_fonts(cx)`; set the `Theme` global from the window appearance and refresh it in `observe_window_appearance` exactly as Collie's `open_main_window` does.

- [ ] **Step 3: Run** `env -u MACOSX_DEPLOYMENT_TARGET cargo test --lib theme` → PASS (2).

- [ ] **Step 4: Commit** — `git commit -m "theme: Zeron tokens, terminal palette and Geist fonts as in Collie"`

---

### Task 3: The emulator

**Files:**
- Create: `src/terminal/mod.rs`, `src/terminal/emulator.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces: `terminal::emulator::{Emulator, CellSnapshot, CellColor, CursorSnapshot, Side, GridSize}` exactly as Zeron's `crates/ui/src/terminal/emulator.rs` (`Emulator::new(cols, rows)`, `feed(&[u8]) -> Vec<u8>`, `resize`, `cols`, `rows`, `title`, `lines`, `line`, `cursor`, `start_selection`, `update_selection`, `clear_selection`, `selection_text`, `has_selection`, `grid_point`, `history_lines`, …).

- [ ] **Step 1: Copy the file with its tests**

Copy Zeron's `emulator.rs` whole (code and its `#[cfg(test)] mod tests`), add the source header comment. Fix imports only (`crate::…` paths).

- [ ] **Step 2: Add the failing test for no local scrollback**

```rust
#[test]
fn keeps_no_local_scrollback() {
    // herdr owns history: lines scrolled off the top of a frame are gone locally.
    let mut emu = Emulator::new(10, 3);
    emu.feed(b"1\r\n2\r\n3\r\n4\r\n5\r\n");
    assert_eq!(emu.history_lines(), 0);
}
```
Run `cargo test --lib terminal::emulator` → FAIL on this test (Zeron keeps `SCROLLBACK_LINES`).

- [ ] **Step 3: Set `scrolling_history: 0`** in `Emulator::new`'s `Config` with a comment "herdr owns scrollback; scroll is a terminal.scroll request". Drop the `SCROLLBACK_LINES` const and fix any Zeron test that asserted on local scrollback by deleting that test (record each deleted test name in the commit message).

- [ ] **Step 4: Run** `env -u MACOSX_DEPLOYMENT_TARGET cargo test --lib terminal::emulator` → all PASS.

- [ ] **Step 5: Commit** — `git commit -m "terminal: Zeron's alacritty emulator without local scrollback"`

---

### Task 4: Key and paste encoding

**Files:**
- Create: `src/terminal/keys.rs`

**Interfaces:**
- Produces: `keys::keystroke_bytes(...) -> Option<Vec<u8>>` and `keys::paste_bytes(text: &str, bracketed: bool) -> Vec<u8>` with Zeron's signatures (`crates/ui/src/terminal/view.rs` lines ~201–330: `keystroke_bytes`, `control_bytes`, `paste_bytes`), and their tests from Zeron's `view.rs` test module (only the ones about these functions).

- [ ] **Step 1: Copy the functions and their tests** (header comment with source). Add:

```rust
#[test]
fn cyrillic_key_char_is_utf8() {
    let ks = gpui::Keystroke::parse("ж").unwrap();
    let ks = gpui::Keystroke { key_char: Some("ж".into()), ..ks };
    assert_eq!(keystroke_bytes(&ks, false).unwrap(), "ж".as_bytes());
}
```
(adapt the call to Zeron's exact signature).

- [ ] **Step 2: Run** `env -u MACOSX_DEPLOYMENT_TARGET cargo test --lib terminal::keys` — the ported tests pass; if `cyrillic_key_char_is_utf8` fails, fix `keystroke_bytes` to send `key_char` bytes for non-control text and rerun to PASS.

- [ ] **Step 3: Commit** — `git commit -m "terminal: Zeron's keystroke and paste encoding"`

---

### Task 5: herdr terminal sessions

**Files:**
- Create: `src/herdr/session.rs`, `tests/session.rs`
- Modify: `src/herdr/mod.rs`

**Interfaces:**
- Produces:
  - `herdr::Session::spawn(pane: &str, cols: u16, rows: u16) -> anyhow::Result<(Session, std::sync::mpsc::Receiver<SessionEvent>)>`; herdr binary from `SHEPHERD_HERDR` else `herdr`.
  - `enum SessionEvent { Frame(Vec<u8>), Ended(String) }` — `Ended` carries `terminal.closed`'s reason, herdr's stderr tail, or the spawn error.
  - `session.input(&[u8])`, `session.resize(cols, rows)`, `session.scroll(up: bool, lines: u16)`, `session.mouse(action: &str, button: &str, column: u16, row: u16, modifiers: u8)`.
  - `Drop` kills and reaps the child.

- [ ] **Step 1: Failing tests** `tests/session.rs` with a fake `herdr` script (pattern from plan 1's attach test):

```rust
// fake: records args, prints one frame ("hello"), copies stdin to $REC/stdin until EOF, then prints terminal.closed
#[test]
fn frames_and_commands() {
    // spawn("w1:p1", 100, 30); expect Frame(b"hello");
    // input(b"ls\r"); resize(120, 40); scroll(true, 3); mouse("down", "left", 4, 2, 0); drop(session)
    // args == "terminal session control w1:p1 --cols 100 --rows 30"
    // stdin lines == [
    //   {"type":"terminal.input","bytes":"bHMN"},
    //   {"type":"terminal.resize","cols":120,"rows":40},
    //   {"type":"terminal.scroll","direction":"up","lines":3,"source":"wheel"},
    //   {"type":"terminal.mouse","action":"down","button":"left","column":4,"row":2,"modifiers":0},
    // ]
}

#[test]
fn closed_session_reports_end() {
    // fake prints terminal.closed with reason "pane closed" right away
    // expect the receiver to yield Ended containing "pane closed", then disconnect
}

#[test]
fn dropping_session_kills_child() {
    // fake: `exec sleep 30`; spawn, read child pid via the fake writing $$ to $REC/pid, drop the session,
    // then assert `kill -0 <pid>` fails within 1 s
}
```
Write them in full (assert exact JSON strings; parse each stdin line with serde_json and compare `Value`s so key order does not matter). Run `cargo test --test session` → FAIL (no `Session`).

- [ ] **Step 2: Implement** `src/herdr/session.rs`: `Command` with piped stdin/stdout/stderr; a reader thread over stdout lines → `Frame(base64-decoded)` / on `terminal.closed` → `Ended(reason or "closed")` and stop; on EOF → `Ended(stderr tail or "herdr exited")`; a stderr thread keeps the last 2 KB. Commands are `writeln!` of `serde_json::json!` to the child's stdin behind a `Mutex<ChildStdin>`. `Drop`: `child.kill()`, `child.wait()`.

- [ ] **Step 3: Run** `env -u MACOSX_DEPLOYMENT_TARGET cargo test --test session` → 3 PASS.

- [ ] **Step 4: Live check** against a scratch tab (`herdr tab create --label shepherd-test --cwd /tmp --no-focus`): a small `#[ignore]` test `live_echo` driven by `SHEPHERD_LIVE_PANE` sends `echo shepherd-$((6*7))\r` and waits for a frame containing `shepherd-42`. Run it, then close the scratch tab.

- [ ] **Step 5: Commit** — `git commit -m "herdr: terminal sessions over terminal session control"`

---

### Task 6: The pane terminal view

**Files:**
- Create: `src/terminal/element.rs`, `src/terminal/pane.rs`
- Modify: `src/terminal/mod.rs`

**Interfaces:**
- Consumes: `Emulator` (Task 3), `keys` (Task 4), `Session`/`SessionEvent` (Task 5), `Theme` (Task 2).
- Produces:
  - `terminal::PaneTerminal` (GPUI entity): `PaneTerminal::new(pane_id: String, cx: &mut Context<Self>) -> Self`, `pane_id()`, `focus_handle()`, `ended() -> Option<&str>`, `reattach(cx)`; implements `Render` and `Focusable`.
  - `element::TerminalElement::new(pane: Entity<PaneTerminal>, focused: bool)` with `PaneTerminal::on_grid_metrics(GridGeometry, cx)` and `PaneTerminal::grid_snapshot() -> GridSnapshot` (`GridGeometry`, `GridSnapshot` as in Zeron's `panel.rs` lines ~176–198).
  - `pane::wheel_lines(accum: &mut f32, delta_px: f32, line_h: f32) -> i32` (positive = up).

- [ ] **Step 1: Failing test for wheel accumulation** in `pane.rs`:

```rust
#[test]
fn wheel_lines_accumulate_fractions() {
    let mut acc = 0.0;
    assert_eq!(wheel_lines(&mut acc, 6.0, 17.0), 0);  // a trackpad tick under one line
    assert_eq!(wheel_lines(&mut acc, 6.0, 17.0), 0);
    assert_eq!(wheel_lines(&mut acc, 6.0, 17.0), 1);  // 18 px ≥ one 17-px line
    assert_eq!(wheel_lines(&mut acc, -40.0, 17.0), -2);
}
```
Run → FAIL.

- [ ] **Step 2: Port the element** — copy Zeron's `TerminalElement` with `TerminalPrepaint`, `shape_row`, `resolve_color`, `extended_indexed_rgb`, `rgb8`, `cell_at`, `CellHit`, `SELECTION_DRAG_THRESHOLD`, `TERM_PADDING`, `TERM_LINE_HEIGHT_RATIO` from `view.rs` into `element.rs` (header comment), replacing `Entity<TerminalPanel>` with `Entity<PaneTerminal>`, `panel.on_grid_metrics` / `active_grid_snapshot` with the `PaneTerminal` methods, and Zeron's `Theme` with ours (`terminal_panel_bg` → `theme.terminal.background`, no glass). Copy the color tests from Zeron's `view.rs` test module that exercise these functions, adapting `Theme::for_selection(...)` to `Theme::for_appearance(...)`.

- [ ] **Step 3: Implement `PaneTerminal`** (`pane.rs`):
  - fields: `pane_id`, `emulator: Emulator` (80×24 until the first metrics), `session: Option<Session>`, `ended: Option<String>`, `geometry: Option<GridGeometry>`, `focus: FocusHandle`, `wheel_acc: f32`, `drag: Option<(Point, bool)>` (press origin, selecting).
  - `new`: spawn the session at 80×24 and a `cx.spawn` task that drains the receiver (poll `try_recv` in a loop with a 8 ms `background_executor().timer` between empty polls; ponytail comment: an async channel if latency shows) → `Frame` → `emulator.feed`, write back any bytes `feed` returns as `session.input`, `cx.notify()`; `Ended(why)` → `ended = Some(why)`, drop the session, notify, stop.
  - `on_grid_metrics`: store geometry; on cols/rows change `emulator.resize` and `session.resize`.
  - keys (`on_key_down`): Cmd+C copies `emulator.selection_text()` to the clipboard if any; Cmd+V pastes clipboard text via `paste_bytes(text, true)`; other Cmd chords are left to the app; everything else → `keystroke_bytes` → `session.input`, clearing the selection.
  - wheel (`on_scroll_wheel`): `delta.pixel_delta(line_h).y` through `wheel_lines`; `n > 0` → `session.scroll(true, n)`, `n < 0` → `session.scroll(false, -n)`.
  - mouse: down → focus, remember origin, `session.mouse("down", button, col, row, mods)`; move with button held → past `SELECTION_DRAG_THRESHOLD` start/update the local selection and send `"drag"`; up → `"up"`. Map positions with `cell_at` and the stored geometry.
  - render: when `ended` is set, a centered column with the reason (muted) and a "Reattach" button (`reattach` respawns the session at the current size and clears `ended`); else `div().track_focus(&self.focus).size_full().bg(theme.terminal.background)` + handlers + `TerminalElement::new(cx.entity(), focused)`.

- [ ] **Step 4: Run** `env -u MACOSX_DEPLOYMENT_TARGET cargo test --lib terminal` → PASS (wheel, element colors, emulator, keys).

- [ ] **Step 5: Commit** — `git commit -m "terminal: pane view painting herdr frames, with keys, wheel scroll and selection"`

---

### Task 7: The window in Zeron's look, wired to pane terminals

**Files:**
- Modify: `src/shell.rs`, `src/main.rs`

**Interfaces:**
- Consumes: `PaneTerminal` (Task 6), `Theme` (Task 2), `model` and `herdr::Client` (plan 1).

- [ ] **Step 1: Window options as Collie's** — transparent titlebar with traffic lights at (14, 14), `app_owns_titlebar_drag: true`, `WindowBackgroundAppearance::Blurred`, min size 820×560; root fill `theme.surface.opacity(0.80)` (Collie's `glass()`), font family `theme.font_ui`.

- [ ] **Step 2: Sidebar and tab bar restyle** — sidebar 240 px over the glass with a 38 px drag strip on top for the traffic lights, section label in `text_faint` 11 px, rows 28 px with `rounded_md`, `selected` / `hover` fills from the theme, status dots: working `accent`, blocked `warning`, done `success`, idle hollow `text_faint`. Tab bar 34 px on `card` with a bottom `border`; tab pills like the rows. Pane grid on `card`, 1 px gutters in `border`, the focused pane outlined in `accent` at 0.5 alpha.

- [ ] **Step 3: Pane entities** — `Shell.panes: HashMap<String, Entity<PaneTerminal>>`; render creates missing ones for shown, attachable panes, drops hidden ones (`prune`, now generic over `Entity<PaneTerminal>` with `exited = |_| false`), and focuses herdr's focused pane's `focus_handle` when it changes (same `focused` bookkeeping as before). Own pane keeps the placeholder.

- [ ] **Step 4: Run the suite** `env -u MACOSX_DEPLOYMENT_TARGET cargo test` → all PASS.

- [ ] **Step 5: Live checks** (scratch tab with two panes, `SHEPHERD_TAB=<tab>`; screenshot each):
  1. Both panes render in Geist Mono with Zeron colors; `ls --color` colors show.
  2. Typing, Cyrillic, ctrl-c, arrows, alt-b in the shell.
  3. `seq 1 500` then wheel up: older lines appear (herdr scrolled), wheel down returns to the bottom.
  4. `vim` in a pane: arrows move in normal and insert mode (risk 1 of the spec); `:q`.
  5. Drag-select text, Cmd+C, paste with Cmd+V into the other pane.
  6. Resize the window: content reflows.
  7. `herdr pane close` on one scratch pane: the window drops it; close the scratch tab after.
  Record results in `docs/spike-results.md` under a new "GPUI terminal" section.

- [ ] **Step 6: Commit** — `git commit -m "shell: Zeron look; panes are GPUI terminals"`
