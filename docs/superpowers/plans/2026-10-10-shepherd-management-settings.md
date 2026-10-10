# Shepherd: Management and Settings (Plan 2b) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Shepherd usable as the daily herdr client: drag split dividers, create / close / rename tabs and workspaces, split / close / navigate / zoom panes from the keyboard, and a Settings window for the terminal font, the theme, herdr's binary and socket, and key bindings.

**Architecture:** Every management action is a herdr API call built by a pure `commands` module (tested), sent by the shell, reflected back through events (herdr stays the single source of truth). Split dividers come from the snapshot's `splits` (their id encodes the tree path herdr's `layout.set_split_ratio` takes). Settings are a JSON file loaded into a GPUI global; the theme comes from Zeron's `zeron-theme` crate (its ~20 built-in families), key bindings are rebound from settings at startup and on change.

**Tech Stack:** Rust 2024, GPUI (`zeronsh/zui@18a89af`), `gpui-base` (`zeronsh/gpui-component@103f13d`, inputs), `zeron-theme` (git `zeronsh/comet`), serde_json; herdr 0.9.3.

**Spec:** `docs/superpowers/specs/2026-10-10-shepherd-client-design.md` (UI section: tab bar, sidebar, pane grid, keys; this plan adds in-app settings, which the spec listed as out — the user asked for them on 2026-10-10: terminal font, theme, herdr, key bindings).

Not in this plan: drag-and-drop of tabs and panes, worktrees, starting agents, saved SSH machines, `.app` bundling.

## Global Constraints

- Everything in the repository is in English; no personal data in fixtures.
- GPUI deps as Collie's; `gpui-base = { git = "https://github.com/zeronsh/gpui-component", rev = "103f13da91ff2c7c52941bdc2b1910f29716dc19" }` as Collie's.
- herdr owns layout: no local layout edits; every action is an API call and the window follows herdr's events.
- Live checks only on scratch tabs/workspaces created with `--no-focus`, closed afterwards.
- Build with `env -u MACOSX_DEPLOYMENT_TARGET cargo …`.

## Review Focus

1. A settings file that is missing, unreadable or from an older version must not stop the app: defaults load, the error shows in Settings, and the bad file is not overwritten until the user saves. → Task 6 tests.
2. A key binding the user records that collides with another action: the new one wins and the old action shows as unbound, never two actions on one chord. → Task 8 test `rebinding_takes_the_chord_from_the_old_action`.
3. Closing the last tab of a workspace or the last pane of a tab follows herdr (it closes the workspace / tab); the window must then show herdr's next focus, not a stale selection. → Task 4 keeps `pick_tab`'s fallback; Task 4 live check.
4. Dragging a divider sends a bounded number of `layout.set_split_ratio` calls (not one per mouse event) and clamps the ratio so no pane collapses to zero. → Task 3 tests.
5. Changing the font or font size re-measures every pane's grid and resizes herdr's view (no stale cols/rows). → Task 7 live check.

---

### Task 1: Split dividers in the model

**Files:** Modify `src/model.rs`, `tests/model.rs`, `tests/fixtures/snapshot.json`

**Interfaces:**
- Produces: `model::SplitFrame { path: Vec<bool>, vertical: bool, ratio: f32, frame: Frac }` (`vertical` = the divider line is vertical, i.e. herdr direction `right`); `Snapshot::split_frames(tab_id) -> Vec<SplitFrame>`; `model::split_path(id: &str) -> Option<Vec<bool>>` (`split_0_root` → `[]`, `split_0_01` → `[false, true]`).

- [ ] Step 1: change the fixture's split id `s0` to `split_0_root` (herdr's real format, `src/app/api/panes.rs::split_path_id`) and add failing tests:

```rust
#[test]
fn split_ids_decode_to_tree_paths() {
    assert_eq!(split_path("split_0_root"), Some(vec![]));
    assert_eq!(split_path("split_0_01"), Some(vec![false, true]));
    assert_eq!(split_path("s0"), None);
}

#[test]
fn split_frames_are_fractions_of_the_tab_area() {
    let splits = fixture().split_frames("w1:t1");
    assert_eq!(splits.len(), 1);
    assert_eq!(splits[0].path, Vec::<bool>::new());
    assert!(splits[0].vertical);
    assert_eq!(splits[0].ratio, 0.5);
    assert_eq!(splits[0].frame, Frac { x: 0.0, y: 0.0, w: 1.0, h: 1.0 });
}

#[test]
fn zoomed_tabs_have_no_dividers() {
    let mut s = fixture();
    s.layouts[0].zoomed = true;
    assert!(s.split_frames("w1:t1").is_empty());
}
```
Run `cargo test --test model` → FAIL.

- [ ] Step 2: add `splits: Vec<LayoutSplit>` (`id`, `direction`, `ratio`, `rect`) to `Layout` with `#[serde(default)]`, implement `split_path` and `split_frames` (same fraction math as `pane_frames`; skip splits whose id does not decode). Run → PASS. Commit `model: split dividers with their tree paths`.

---

### Task 2: herdr commands

**Files:** Create `src/commands.rs`; Modify `src/lib.rs`

**Interfaces:**
- Produces: `commands::Command` enum and `Command::request(&self) -> (&'static str, serde_json::Value)`:
  `NewTab { workspace_id }` → `tab.create {workspace_id, focus: false}`; `CloseTab { tab_id }` → `tab.close`; `RenameTab { tab_id, label }`; `NewWorkspace { cwd: Option<String> }` → `workspace.create {cwd, focus: false}`; `CloseWorkspace { workspace_id }` → `workspace.close {workspace_id, close_group: false}`; `RenameWorkspace { workspace_id, label }`; `Split { pane_id, down: bool }` → `pane.split {target_pane_id, direction: "right"|"down", focus: true}`; `ClosePane { pane_id }`; `FocusPane { pane_id, direction }` → `pane.focus_direction {pane_id, direction: "left"|"right"|"up"|"down"}`; `Zoom { pane_id }` → `pane.zoom {pane_id, mode: "toggle"}`; `SetSplitRatio { tab_id, path, ratio }` → `layout.set_split_ratio`.
  `commands::created_tab_id(result: &Value) -> Option<String>` (from `tab.create`'s `result.tab.tab_id`) and `created_workspace_tab(result: &Value) -> Option<String>` (the new workspace's active tab, read from the response; look at a real `workspace.create` response in the live step and pin the field in the test).

- [ ] Step 1: failing tests — one per variant asserting the exact `(method, params)` with `serde_json::json!`; `created_tab_id` against `{"type":"tab_created","tab":{"tab_id":"w1:t3",...},"root_pane":{...}}`.
- [ ] Step 2: implement; run → PASS.
- [ ] Step 3: live: on a scratch workspace (`herdr workspace create --label shepherd-test --cwd /tmp --no-focus`) call `workspace.create` through `Client::call` in an `#[ignore]` test, print the response, pin `created_workspace_tab`'s field in a unit test, close the scratch workspaces. Commit `commands: herdr requests for tab, workspace and pane management`.

---

### Task 3: Draggable split dividers

**Files:** Modify `src/shell.rs`; Create `src/ui/divider.rs` (pure math + element), `src/ui/mod.rs`

**Interfaces:**
- Produces: `divider::drag_ratio(start_px: f32, len_px: f32, pointer_px: f32) -> f32` (clamped to 0.05..=0.95); `divider::Throttle` (`ready(now) -> bool`, 33 ms).

- [ ] Step 1: failing tests:

```rust
#[test]
fn drag_ratio_follows_the_pointer_within_the_split() {
    assert_eq!(drag_ratio(100.0, 400.0, 300.0), 0.5);
    assert_eq!(drag_ratio(100.0, 400.0, 0.0), 0.05);   // clamped: no pane collapses
    assert_eq!(drag_ratio(100.0, 400.0, 999.0), 0.95);
}

#[test]
fn throttle_lets_one_call_through_per_window() {
    let t0 = std::time::Instant::now();
    let mut t = Throttle::default();
    assert!(t.ready(t0));
    assert!(!t.ready(t0 + std::time::Duration::from_millis(10)));
    assert!(t.ready(t0 + std::time::Duration::from_millis(40)));
}
```
- [ ] Step 2: implement; in the grid render each `SplitFrame` as a 6-px hit strip centered on the divider line (`cursor_col_resize` / `cursor_row_resize`, a 1-px line in `border`, `accent` while dragging). Mouse down starts a drag remembering the split's pixel start/length (from the grid bounds via a `canvas` prepaint, as the element does); mouse move → `drag_ratio` → when `Throttle::ready` send `Command::SetSplitRatio`; mouse up sends the final ratio unthrottled.
- [ ] Step 3: run tests → PASS; live check on a scratch tab with a vertical and a horizontal split: drag both, herdr's layout follows (`herdr pane layout`), panes reflow. Commit `ui: draggable split dividers`.

---

### Task 4: Tabs and workspaces from the window

**Files:** Modify `Cargo.toml` (gpui-base), `src/main.rs` (`gpui_base::init`), `src/shell.rs`

- [ ] Step 1: tab bar: a trailing `+` (new tab in the shown workspace; select the created tab via `created_tab_id`); each tab shows `×` on hover (close); double-click turns the label into a `gpui_base` `Input` (Enter renames, Escape cancels, blur cancels).
- [ ] Step 2: sidebar: `+` next to "Workspaces" (new workspace with no cwd, then select its tab via `created_workspace_tab`); rows get the same hover `×` and double-click rename. Close asks nothing for an empty shell but herdr may refuse; show herdr's error text in a strip at the top of the grid for 4 s.
- [ ] Step 3: shell keeps a `client: Client` and a `send(Command, cx)` helper that runs `client.call` on the tokio handle and reports errors to that strip.
- [ ] Step 4: live check on a scratch workspace: create, rename, close tabs and the workspace; closing the shown tab falls back to herdr's focus (`pick_tab`). Commit `ui: create, rename and close tabs and workspaces`.

---

### Task 5: Theme from Zeron's theme library

**Files:** Modify `Cargo.toml` (`zeron-theme = { git = "https://github.com/zeronsh/comet", rev = "9e17e401710412adca91fe485886d65aa2d7e3eb" }`), `src/theme.rs`

**Interfaces:**
- Produces: `Theme::from_zeron(variant: &zeron_theme::ThemeVariant, font: &FontSettings) -> Theme` mapping `colors.{shell→surface, card, text, text_muted, text_faint, hover, border, border_strong, warning, success, danger, cursor}`, `accent.primary → accent`, `terminal.{background, foreground, selection, ansi}`; `theme::families() -> Vec<(id, name)>` from `ThemeRegistry::builtin()`; `Theme::resolve(family_id, mode: AppearanceMode, system: WindowAppearance, font) -> Theme` (`AppearanceMode::{System, Light, Dark}`; a family without that appearance falls back to its other variant).

- [ ] Step 1: failing tests: Zeron family resolves to the current hard-coded values (keep the two existing tests green against `resolve("zeron", …)`); every built-in family resolves in both appearances without panicking and with 16 ANSI colors; `AppearanceMode::Light` on a dark system picks the light variant.
- [ ] Step 2: implement; delete the hard-coded palettes (Zeron's now come from the crate). Run → PASS. Commit `theme: Zeron's built-in theme families`.

---

### Task 6: Settings file

**Files:** Create `src/settings.rs`; Modify `src/lib.rs`, `src/main.rs`

**Interfaces:**
- Produces: `Settings { font_family: String ("Geist Mono"), font_size: f32 (13.0), theme_family: String ("zeron"), appearance: AppearanceMode (System), herdr_bin: Option<String>, herdr_socket: Option<String>, keys: BTreeMap<String /*action*/, String /*keystroke*/> }` with serde defaults for every field; `Settings::path() -> PathBuf` (`~/Library/Application Support/Shepherd/settings.json`); `Settings::load(path) -> (Settings, Option<String /*error*/>)`; `settings.save(path) -> anyhow::Result<()>` (write to a temp file then rename). GPUI global `SettingsGlobal { settings, error, path }`.

- [ ] Step 1: failing tests: missing file → defaults, no error; `{}` → defaults; unknown fields ignored; a partial file keeps given fields; garbage → defaults + error naming the path, and the file is left untouched; save then load round-trips.
- [ ] Step 2: implement; `main` loads settings before opening the window; theme resolves from them; `herdr_socket` feeds `Client` (else today's resolution), `herdr_bin` feeds `Session` (replacing the `SHEPHERD_HERDR` env lookup, which stays as an override for tests). Run → PASS. Commit `settings: a JSON settings file with defaults`.

---

### Task 7: Font settings reach the terminal

**Files:** Modify `src/terminal/element.rs`, `src/theme.rs`

- [ ] Step 1: the element reads `font_terminal` / `terminal_font_size` from the theme (already does); `Theme::resolve` takes them from `Settings`. A settings change sets the new theme global and `cx.refresh_windows()`: each pane's prepaint re-measures cols/rows and `on_grid_metrics` resizes herdr.
- [ ] Step 2: live check: change the size 13 → 16 and the family to a Nerd Font in the settings file, reload via the Settings window (Task 9) — grids re-measure, herdr re-renders at the new size. Commit `terminal: font family and size from settings`.

---

### Task 8: Key bindings

**Files:** Create `src/keys.rs` (app actions and bindings); Modify `src/main.rs`, `src/shell.rs`

**Interfaces:**
- Produces: GPUI actions `NewTab, CloseTab, NewWorkspace, SplitRight, SplitDown, ClosePane, FocusLeft, FocusRight, FocusUp, FocusDown, ToggleZoom, Tab1..Tab9, PreviousWorkspace, NextWorkspace, OpenSettings, Quit`; `keys::DEFAULTS: &[(&str /*action name*/, &str /*keystroke*/)]` — `cmd-t, cmd-shift-w, cmd-shift-n, cmd-d, cmd-shift-d, cmd-w, cmd-alt-left/right/up/down, cmd-shift-enter, cmd-1…cmd-9, cmd-up, cmd-down, cmd-,, cmd-q`; `keys::effective(user: &BTreeMap<String,String>) -> Vec<(String, String)>` (defaults overlaid by the user's; a chord bound twice keeps only the user's action); `keys::apply(cx, &effective)` (`cx.clear_key_bindings()` then `cx.bind_keys(...)`).

- [ ] Step 1: failing tests for `effective`: defaults when empty; a user override replaces the default chord; `rebinding_takes_the_chord_from_the_old_action` (user binds `cmd-d` to `NewTab` → `SplitRight` has no chord); an empty string unbinds.
- [ ] Step 2: implement; `Shell` handles the actions on its root (`on_action`): pane actions use the pane whose `PaneTerminal` focus handle is focused (else herdr's focused pane of the shown tab); tab/workspace actions use the shown ones; `Tab1..9` select by position; previous/next workspace walk the sorted list. Pane terminals already let `cmd` chords through (`keystroke_bytes` returns `None`). Run → PASS.
- [ ] Step 3: live: split, focus, zoom and close on a scratch tab by keyboard. Commit `keys: app actions with default bindings and user overrides`.

---

### Task 9: The Settings window

**Files:** Create `src/ui/settings.rs`; Modify `src/main.rs`

- [ ] Step 1: `OpenSettings` (Cmd+,) opens one Settings window (reuse it if open), 640×560, Collie's look: a left list of sections — Terminal, Appearance, herdr, Keys — and the section on the right.
  - Terminal: font family (`gpui_base` Input, applied on Enter / blur; below it the Nerd Font fallback note), size with − / + buttons (9…32).
  - Appearance: System / Light / Dark segmented control; a list of theme families, each row with a small swatch strip (background + 6 ANSI colors) of the variant it would show.
  - herdr: binary path and socket path inputs with "Default" placeholders; a "Test connection" button calling `ping` and showing the result.
  - Keys: one row per action (human label, current chord, "Record" → the next keystroke becomes the chord, Escape cancels, Backspace unbinds), and "Reset to defaults".
  - The settings file error (if any) as a strip on top.
- [ ] Step 2: every change updates `SettingsGlobal`, saves the file, re-resolves the theme or re-applies key bindings, and refreshes all windows.
- [ ] Step 3: live checks with screenshots of each section; change theme family and appearance and see the main window follow; record a binding and use it. Commit `ui: settings window`.
