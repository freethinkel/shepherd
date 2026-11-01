# Right Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One right panel per workspace with flat tabs, holding the browser and the PR view beside the terminals.

**Architecture:** Plain logic in `src/right_panel.rs` (the `web.rs` / `shell/web.rs` split), the GPUI side in `src/shell/right_panel.rs`, chrome ported from Zeron into `src/ui/surface_chrome.rs`. `Pages.shown` shrinks to sidebar pages; the panel's active browser tab becomes a second, exclusive source of the visible page.

**Tech Stack:** Rust 2024, GPUI (zeronsh/zui), wry.

**Spec:** `docs/superpowers/specs/2026-11-01-right-panel-design.md`

## Global Constraints

- Starts from `main` at or after `5d9d9df`: annotations, `Layout::has_neighbor` and `menu_mark` are all there, so the spec's "Order of work" is met.
- Work in a worktree made with `herdr worktree create --branch right-panel --base main --label right-panel --no-focus`, never `git worktree add`.
- Zeron is at `~/Developer/forks/zeron`. Port nearly word for word; where Zeron and this plan disagree on look or feel, Zeron wins. Paths below are relative to its `crates/ui/src`.
- Widths: default `520.`, min `360.`, centre column kept `300.`. Chip `112. × 24.`, control `24.`, radius `6.`, gap `4.`.
- Icons: `icons/solar/global.svg` browser, `icons/git-pull-request.svg` PR, `icons/sidebar-minimalistic-right.svg` the toggle.
- Code comments in English, repo style. Commit subjects `area: what changed, in a sentence`, ending with the session's attribution lines.
- Every task ends green: `cargo test` and `cargo clippy --all-targets -- -D warnings`.

## Review Focus

1. A workspace closed while its panel holds pages: its state and pages go, and nothing stays on screen. *(Task 1, Task 4)*
2. A page that fails to build (`make_page` errors): the tab leaves the strip and the panel falls back; no tab stays with nothing behind it. *(Task 1, Task 4)*
3. A window shrunk below the saved width, then grown: the panel narrows or hides, and its saved width returns. *(Task 1)*
4. A terminal link with the panel closed: the panel opens on the new tab and the terminal keeps the keyboard. *(Task 1, Task 5)*
5. A tab dragged while another is active: order changes, the active tab doesn't. Out-of-range indices do nothing. *(Task 1)*

---

### Task 1: Panel logic

**Files:**
- Create: `src/right_panel.rs`
- Create: `tests/right_panel.rs`
- Modify: `src/lib.rs`, `src/settings.rs:258` (constants, field, clamp)

**Interfaces:**
- Produces, in `shepherd::right_panel`:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Surface { Browser(String), PullRequest }

#[derive(Debug, Default)]
pub struct PanelState { pub tabs: Vec<Surface>, pub active: Option<Surface>, pub open: bool /* + private visit history */ }

impl PanelState {
    pub fn add(&mut self, surface: Surface);              // appended unless present, activated, panel opened
    pub fn activate(&mut self, surface: &Surface);
    pub fn close(&mut self, surface: &Surface);           // fallback; the last one closes the panel
    pub fn reorder(&mut self, from: usize, to: usize);
    pub fn step(&mut self, forward: bool);                // wraps
    pub fn toggle(&mut self);
    pub fn toggle_pull_request(&mut self);
    pub fn retain_browsers(&mut self, live: &[&str]);     // drops Browser tabs not in `live`, with fallback
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Offer { Browser, PullRequest }

pub fn offered(has_remote: bool, embeds_pages: bool) -> Vec<Offer>;
pub fn fits(window: f32, sidebar: f32) -> bool;
pub fn clamp_width(width: f32, window: f32, sidebar: f32) -> f32;
pub fn drop_index(rel_x: f32, tab_w: f32, count: usize) -> usize;      // Zeron terminal/panel.rs
pub fn slide_offset(ix: usize, from: usize, over: usize) -> f32;       // Zeron terminal/panel.rs
```

- Produces, in `shepherd::settings`: `RIGHT_PANEL_WIDTH = 520.`, `RIGHT_PANEL_MIN = 360.`, `RIGHT_PANEL_CENTER_MIN = 300.`, and `Settings.right_panel_width: f32`, defaulting to `RIGHT_PANEL_WIDTH`, floored at `RIGHT_PANEL_MIN` on load.

- [ ] **Step 1: Write the failing tests in `tests/right_panel.rs`**

```rust
use shepherd::right_panel::{Offer, PanelState, Surface, clamp_width, drop_index, fits, offered, slide_offset};

fn b(id: &str) -> Surface { Surface::Browser(id.into()) }
fn panel(tabs: &[Surface]) -> PanelState { let mut p = PanelState::default(); tabs.iter().for_each(|s| p.add(s.clone())); p }

#[test]
fn add_opens_the_panel_on_the_tab() { // also pins a link clicked with the panel closed
    let mut p = PanelState::default();
    p.add(b("1"));
    assert!(p.open);
    assert_eq!(p.active, Some(b("1")));
    p.add(Surface::PullRequest);
    p.add(b("1")); // present: activated, not doubled
    assert_eq!(p.tabs, [b("1"), Surface::PullRequest]);
    assert_eq!(p.active, Some(b("1")));
}

#[test]
fn closing_the_active_tab_falls_back_to_the_last_visited() {
    let mut p = panel(&[b("1"), b("2"), b("3")]);
    p.activate(&b("1"));
    p.activate(&b("3"));
    p.close(&b("3"));
    assert_eq!(p.active, Some(b("1")));
}

#[test]
fn closing_another_tab_keeps_the_active_one() {
    let mut p = panel(&[b("1"), b("2")]);
    p.close(&b("1"));
    assert_eq!(p.active, Some(b("2")));
}

#[test]
fn closing_the_last_tab_closes_the_panel_and_the_next_open_shows_the_picker() {
    let mut p = panel(&[b("1")]);
    p.close(&b("1"));
    assert!(!p.open);
    assert_eq!(p.active, None);
    p.toggle();
    assert!(p.open);
    assert_eq!(p.active, None);
}

#[test]
fn reorder_leaves_the_active_tab_and_ignores_bad_indices() {
    let mut p = panel(&[b("1"), b("2"), b("3")]);
    p.activate(&b("2"));
    p.reorder(0, 2);
    assert_eq!(p.tabs, [b("2"), b("3"), b("1")]);
    assert_eq!(p.active, Some(b("2")));
    p.reorder(0, 9);
    p.reorder(9, 0);
    assert_eq!(p.tabs, [b("2"), b("3"), b("1")]);
}

#[test]
fn step_wraps_and_a_lone_tab_stays() {
    let mut p = panel(&[b("1"), b("2")]);
    p.step(true);
    assert_eq!(p.active, Some(b("1")));
    p.step(false);
    assert_eq!(p.active, Some(b("2")));
    let mut one = panel(&[b("1")]);
    one.step(true);
    assert_eq!(one.active, Some(b("1")));
    PanelState::default().step(true); // no tabs: no panic
}

#[test]
fn the_pull_request_toggle_opens_switches_and_closes() {
    let mut p = panel(&[b("1")]);
    p.toggle(); // closed
    p.toggle_pull_request();
    assert!(p.open);
    assert_eq!(p.active, Some(Surface::PullRequest));
    p.activate(&b("1"));
    p.toggle_pull_request();
    assert_eq!(p.active, Some(Surface::PullRequest));
    p.toggle_pull_request();
    assert!(!p.open);
    assert_eq!(p.tabs, [b("1"), Surface::PullRequest]); // the tab stays
}

#[test]
fn a_browser_tab_whose_page_is_gone_falls_back() {
    let mut p = panel(&[Surface::PullRequest, b("1"), b("2")]);
    p.retain_browsers(&["1"]);
    assert_eq!(p.tabs, [Surface::PullRequest, b("1")]);
    assert_eq!(p.active, Some(b("1")));
}

#[test]
fn offers_follow_the_remote_and_the_platform() {
    assert_eq!(offered(true, true), [Offer::Browser, Offer::PullRequest]);
    assert_eq!(offered(false, true), [Offer::Browser]);
    assert_eq!(offered(true, false), [Offer::PullRequest]);
    assert!(offered(false, false).is_empty());
}

#[test]
fn widths_keep_the_centre_column_and_the_saved_value_survives_a_small_window() {
    assert_eq!(clamp_width(520., 1400., 240.), 520.);
    assert_eq!(clamp_width(9000., 1400., 240.), 860.); // 1400 - 240 - 300
    assert_eq!(clamp_width(100., 1400., 240.), 360.);
    assert_eq!(clamp_width(520., 800., 0.), 500.);
    assert!(fits(660., 0.));
    assert!(!fits(659., 0.));
    assert!(!fits(800., 200.));
}

#[test]
fn tab_drags_quantise_and_slide() {
    assert_eq!(drop_index(-10., 116., 3), 0);
    assert_eq!(drop_index(116., 116., 3), 1);
    assert_eq!(drop_index(700., 116., 3), 2);
    assert_eq!(drop_index(50., 116., 0), 0);
    assert_eq!(slide_offset(1, 0, 2), -1.);
    assert_eq!(slide_offset(1, 2, 0), 1.);
    assert_eq!(slide_offset(0, 0, 2), 0.);
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test --test right_panel`
Expected: compile error, `right_panel` not found.

- [ ] **Step 3: Add the constants and `right_panel_width` to `src/settings.rs`, with a test beside `review_width`'s (`settings.rs:632`) that `{"right_panel_width": 10}` loads as `RIGHT_PANEL_MIN`**

- [ ] **Step 4: Implement `src/right_panel.rs` and `pub mod right_panel;` in `src/lib.rs`**

The fallback is Zeron's `right_surface_fallback` (`shell.rs:947`) without its "strip's first" case: `add` activates, so every tab is in the history. `activate` moves the surface to the end of a private `history: Vec<Surface>`; `close` and `retain_browsers` drop it from there, and the new active is the history's last. `clamp_width` must not panic when the max is below the min: `width.min(window - sidebar - CENTER_MIN).max(MIN)` for the first three asserts; the fourth pins that a window that fits narrows it (`800 - 0 - 300 = 500`). `fits` is `window - sidebar - CENTER_MIN >= MIN`.

- [ ] **Step 5: Run the tests**

Run: `cargo test --test right_panel && cargo test --lib settings`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/right_panel.rs tests/right_panel.rs src/lib.rs src/settings.rs
git commit -m "right panel: the logic of a workspace's tabs, its widths and what it offers"
```

---

### Task 2: The empty panel

An openable, resizable column with the strip and the picker. Tabs can be added and show an empty body.

**Files:**
- Create: `src/ui/surface_chrome.rs`, `src/shell/right_panel.rs`
- Modify: `src/ui/mod.rs`, `src/shell.rs`, `src/shell/render.rs`, `src/shell/tab_bar.rs:875`, `src/shell/web.rs:185`, `src/bindings.rs`, `src/ui/command_panel.rs:139`, `src/shell/popups.rs:481`

**Interfaces:**
- Consumes: Task 1.
- Produces, on `Shell` as field `right: right_panel::RightPanel`:

```rust
pub(super) struct RightPanel {
    pub(super) panels: HashMap<String, PanelState>, // by workspace id
    pub(super) width: f32,
    pub(super) drag: bool,
    pub(super) plus: Option<crate::ui::Popup<gpui::Point<Pixels>>>,
    pub(super) focused: bool,                       // focus in panel; Task 5 sets it
    pub(super) scroll: gpui::ScrollHandle,
    pub(super) tab_drag: Option<TabDrag>,           // Zeron's RightTabDragState
}

impl Shell {
    pub(super) fn panel(&self) -> Option<&PanelState>;                                    // the shown workspace's
    pub(super) fn update_panel(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut PanelState));
    pub(super) fn right_panel_shown(&self, window: &Window) -> bool;                      // open, no Settings, no `pages.shown`, `fits`
    pub(super) fn toggle_right_panel(&mut self, cx: &mut Context<Self>);
    pub(super) fn drag_right_panel(&mut self, x: Pixels, window: &Window, cx: &mut Context<Self>);
    pub(super) fn end_right_panel_drag(&mut self, cx: &mut Context<Self>);
    pub(super) fn render_right_panel(&mut self, theme: &Theme, shown: &ShownReview, window: &mut Window, cx: &mut Context<Self>) -> [AnyElement; 2]; // panel, edge
}
```

- Produces, in `crate::ui::surface_chrome`: `toolbar`, `tab`, `tab_chip`, `TabChip`, `tab_chip_icon`, `TAB_CHIP_WIDTH`, `TAB_CHIP_HEIGHT`, `TAB_CHIP_SLOT`, `TAB_STRIP_FADE`.
- Produces action `ToggleRightPanel`.

- [ ] **Step 1: Add `ToggleRightPanel: "Toggle right panel"` to `src/bindings.rs`, move the default chord to it**

`cmd-alt-b` and `ctrl-alt-b` go from `ToggleReview` to `ToggleRightPanel` in both `DEFAULTS`; `ToggleReview` stays in the action list with no default. View menu gets `I::action("Toggle Right Panel", ToggleRightPanel)` above the PR item. `command_panel.rs:139` gives it the toggle icon.

- [ ] **Step 2: Port `surface_chrome.rs`**

Copy Zeron's `surface_chrome.rs`, keeping only the Interfaces names. Adjust: `crate::theme::wash(x)` to `theme.wash(x)`, `crate::icons::icon(CLOSE)` to `gpui::svg().path("icons/close.svg")`, `typography::ui_rems` to `px`, `text_tooltip` to `tip`. `tab_strip_region` becomes `crate::ui::edge_fade::edge_faded_x(TAB_STRIP_FADE, scroll, strip)`. Drop the `is_glass` branches. Keep `role`, `aria_*` and `tab_index` if this GPUI rev has them.

- [ ] **Step 3: Write `src/shell/right_panel.rs`**

Port `render_right_tab_strip` (`shell.rs:12647`), `render_surface_picker` (`shell.rs:12477`), the `+` (`shell.rs:12940`) and the panel frame of `render_right_pane` (`shell.rs:12423`), minus every `RightSurface` arm the enum here lacks, the glass and takeover branches, and the window-corner code. Chips: `Browser` gets favicon or spinner and `browser_label`, wired in Task 4; `PullRequest` in Task 3; until then both show the icon and a plain name. With no tabs the strip shows one inactive `+ New` chip. Picker and `+` rows come from `offered(has_remote, embeds)`; pass `true` for `embeds` until Task 7. Picking Pull request calls `update_panel(|p| p.add(Surface::PullRequest))`; picking Browser calls today's `new_browser_tab` until Task 4. The strip's right end is a `titlebar_button` with `icons/close.svg` and `tip("Close", Some("ToggleRightPanel"))`, with `pr(12. + window_controls_w(cx))`. Middle click closes a chip. The resize edge is `review.rs:331` verbatim under the id `right-panel-resize`, double click resetting to `RIGHT_PANEL_WIDTH`, saved with `save_from_sidebar`; hold `GridHold` during the drag.

The body under the strip is `div().flex_1().min_h_0()` for both surfaces until Tasks 3 and 4.

- [ ] **Step 4: Wire it into the frame**

`render.rs`: `.children(right_panel)` after `.children(review…)`; `ToggleRightPanel` action; drag in the root's move and up handlers next to `drag_review`. `resize_cursor` and the idle check at `shell.rs:439` include `right.drag`. Tab bar: `pr` uses `right_panel_shown(window)` beside `review_open`. A `titlebar_button_with` for the toggle sits at the tab bar's right end, shown while the panel is closed, in the spot the strip's ✕ holds while open. `right.plus` joins `popup_open` and gets its `reap_popup` at `render.rs:17`.

- [ ] **Step 5: Run the checks**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 6: Look at it in the mock**

Run the mock per `docs/mock/shot.sh`, with `SHEPHERD_SETTINGS_FILE` set, on a copied bundle if `target/debug/Shepherd.app` is running. Send `cmd-alt-b` with `CGEvent.postToPid`, capture with `screencapture -l<window id>`, don't bring the window forward.
Expected: the column at 520 px with `+ New`, ✕ and the picker's rows; Browser only, as the mock has no remote.

- [ ] **Step 7: Commit**

```bash
git add -A src
git commit -m "right panel: a column beside the panes with Zeron's tab strip and picker, opened by a button and cmd-alt-b, resized at its edge"
```

---

### Task 3: The PR view becomes a tab

**Files:**
- Modify: `src/shell/review.rs:48-68,142-345`, `src/shell/right_panel.rs`, `src/shell/render.rs:252,287,294,364,378-396`, `src/shell.rs:133-137,439,496-498,1076`, `src/settings.rs:42-44,258-260,281-282,375,632-633`, `src/ui/statusbar.rs:783-791`, `src/shell/tab_bar.rs:875`, `src/shell/web.rs:704`

**Interfaces:**
- Consumes: `PanelState::toggle_pull_request`, `Surface::PullRequest`, `update_panel`, `surface_chrome::toolbar`.
- Produces: `Shell::pull_request_shown(&self, window: &Window) -> bool`, true while the panel shows the PR tab; `render_review(..) -> AnyElement`, the body only.

- [ ] **Step 1: Write the failing settings test in `src/settings.rs`**

```rust
#[test]
fn old_settings_with_the_review_panel_still_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, r#"{"review_open": true, "review_width": 400, "sidebar_width": 300}"#).unwrap();
    let (settings, _) = Settings::load(&path);
    assert_eq!(settings.sidebar_width, 300.);
    assert_eq!(settings.right_panel_width, RIGHT_PANEL_WIDTH);
}
```

Use whatever temp-dir helper the neighbouring tests use. Delete the `review_width` test at `settings.rs:632`.

- [ ] **Step 2: Run it**

Run: `cargo test --lib old_settings`
Expected: PASS already if unknown fields are ignored. If it fails, the removal in Step 3 must keep them loading. Either way it stays as the guard.

- [ ] **Step 3: Remove `review_open`, `review_width`, `REVIEW_WIDTH`, `REVIEW_MIN`, `REVIEW_MAX` from the settings, and `review_open`, `review_width`, `review_drag`, `drag_review`, `end_review_drag` from `Shell`**

- [ ] **Step 4: Rework `render_review` to return the body**

Delete its resize edge and its header. A `surface_chrome::toolbar` row on top holds the noun (`Pull request`, `Merge request`, `Pipeline`) and Refresh or its spinner. Everything else stays. `render.rs:294`'s `has_remote` gate goes: the tab stays and shows `Review::NoRemote`'s text. `markdown::forget("review-")` runs when `pull_request_shown` is false.

- [ ] **Step 5: Rewire the callers**

`toggle_review` becomes `update_panel(|p| p.toggle_pull_request())`. `shown` at `render.rs:252` is computed when `show_bar || pull_request_shown(window)`. The status bar's `review_open` argument takes `pull_request_shown`. `web.rs:704` becomes plain `window_controls_w(cx)`: a sidebar page hides the panel, so nothing sits under the window's buttons. `render_right_panel`'s `PullRequest` arm renders the body, its chip reads `PR #42`, `MR !42`, or `Pull request` until known, with a spinner while `fetching::<Review>`. The picker's `has_remote` is today's expression at `render.rs:291`.

- [ ] **Step 6: Run the checks**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 7: Commit**

```bash
git add -A src
git commit -m "review: the pull request panel is a tab of the right panel, one per workspace, and no longer remembers being open"
```

---

### Task 4: Browser tabs move in

**Files:**
- Modify: `src/shell/browser.rs`, `src/shell/web.rs:157-171,214-250,488-590`, `src/shell/tab_bar.rs`, `src/shell/render.rs:117-133`, `src/shell/right_panel.rs`, `src/shell/popups.rs:481`, `src/shell.rs:110-111,484`, `src/shell/sidebar.rs` if it reads `browsing`

**Interfaces:**
- Consumes: `update_panel`, `PanelState::{add, close, retain_browsers}`, `Surface::Browser`.
- Produces:

```rust
impl Shell {
    pub(super) fn ensure_page(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> bool; // makes the page; false when it couldn't
    pub(super) fn panel_page(&self, window: &Window) -> Option<PageKey>;                                 // the shown panel's active browser tab
    pub(super) fn visible_page(&self, window: &Window, cx: &App) -> Option<PageKey>;                     // sidebar's, else the panel's
}
```

- `new_browser_tab(ws: String, url: Option<String>, focus_address: bool, window, cx)`; `after` is gone.

- [ ] **Step 1: Delete what the move replaces, and let the compiler list the rest**

From `browser.rs`: `BrowserTab.after`, `Browser.seen`, `follows`, `interleave`, `dropped`, `drop_tab`, `browser_tabs_of`, `shown_browser_tab`, `new_tab_menu` and the three tests of these. From `tab_bar.rs`: `Shown.browser`, `Shown.image`, `Shown.loading`, the interleaving at `:681-694`, and the `browser` branches of `step_tab`, the drop and the close paths. From the tab bar's end: the `new-tab-more` chevron. From `shell.rs`: `new_tab_menu`. From `render.rs`: `browsing` and its branch; `tabbar` always gets `shown_tab`, `sidebar_ws` is `None` whenever `web` is some. `typed_url`'s test stays.

- [ ] **Step 2: Split `show_web`**

`ensure_page` is `show_web` up to and including the `make_page` match. On `Err` it toasts and returns false. `show_web` calls it and sets `pages.shown`. `shown_page` drops `.chain(self.browser.entries())`. Wayland and GTK fallbacks stay in `ensure_page`.

- [ ] **Step 3: Rewrite `new_browser_tab`**

Pushes the `BrowserTab`, `ensure_page`; on false drops it again and returns, so no tab stays behind (Review Focus 2). On true `update_panel(|p| p.add(Surface::Browser(id)))`. The picker's and `+`'s Browser row call it with `None, true`. `close_browser_tab` also does `PanelState::close` on the tab's workspace. `sync_browser`: `pending` opens with `focus_address: false` in the shown workspace, else `cx.open_url`; after the `retain`, drop panels whose workspace is gone and call `retain_browsers` on each with that workspace's live ids (Review Focus 1).

- [ ] **Step 4: Make the page visible from the panel**

In `sync_pages`, `shown` becomes `visible_page(window, cx)`, so the loop's `visible` needs no other change, and the "make it if missing" block applies to both. Keep `focused = visible && !popup` for now; Task 5 replaces it. `reload_shown_page`, `copy_page_url` and `focus_address` take `visible_page`. `render_right_panel`'s `Browser` arm: `render_browser_bar(key)` over the page and `render_annotations` in today's `flex` row, unchanged until Task 6. The `Browser` chip gets `browser_label`, the favicon from `pages.favicons`, and `loaders` spinner while `Page.loading`. Add `pub(super) fn browser_tab(&self, id: &str) -> Option<&BrowserTab>` if needed; `browser_label` and the favicon lookup become `pub(super)`. Whatever `tab_menu` offered a browser tab moves to a right click on its chip.

- [ ] **Step 5: Run the checks**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: PASS, no warnings; `tests/right_panel.rs` untouched; `grep -n "browser" src/shell/tab_bar.rs` finds nothing about browser tabs.

- [ ] **Step 6: Commit**

```bash
git add -A src
git commit -m "browser: its tabs live in the right panel beside the panes, the tab bar holds herdr's alone, and a link from a terminal opens there"
```

---

### Task 5: Keyboard and focus

**Files:**
- Modify: `src/shell/web.rs:478,554-590`, `src/shell/render.rs:96-104,322-371`, `src/shell.rs:900,912-918`, `src/shell/tab_bar.rs:618`, `src/shell/right_panel.rs`, `src/right_panel.rs`, `tests/right_panel.rs`

**Interfaces:**
- Consumes: `RightPanel.focused`, `panel_page`, `PanelState::{step, close}`, `Layout::has_neighbor`.
- Produces: `pub fn page_takes_keyboard(sidebar_page: bool, focus_in_panel: bool, popup: bool) -> bool` in `src/right_panel.rs`; `Shell::focus_panel(&mut self, on: bool, window: &mut Window, cx: &mut Context<Self>)`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_visible_page_keeps_its_hands_off_the_keyboard_until_asked() {
    use shepherd::right_panel::page_takes_keyboard as takes;
    assert!(takes(true, false, false));   // a sidebar page has the window to itself, as today
    assert!(!takes(false, false, false)); // a panel page beside a terminal
    assert!(takes(false, true, false));   // clicked into
    assert!(!takes(false, true, true));   // a popup, the address field or a note holds it
    assert!(!takes(true, false, true));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --test right_panel`
Expected: compile error, `page_takes_keyboard` not found.

- [ ] **Step 3: Implement `page_takes_keyboard` and use it in `sync_pages`**

`let focused = visible && page_takes_keyboard(self.pages.shown.is_some(), self.right.focused, popup);`. The existing `self.focus.contains_focused` hand-over stays.

- [ ] **Step 4: Track "focus in panel"**

`focus_panel(true)`: a chip click, `PageEvent::Clicked` when the key is the panel's page, a mouse down on the panel frame (`capture_any_mouse_down`, as Zeron's `right-pane-focus`). `focus_panel(false)`: a mouse down on the grid, a workspace switch, the panel closing, and a closed active tab with no tab left; `select_tab` has no window, so it only clears the flag. `focus_panel(false)` also refocuses the pane last focused, so keys land in the terminal, not on `self.focus`. `new_browser_tab` from `+` or the picker calls `focus_panel(true)`; from a link it doesn't (Review Focus 4). Off macOS, where `Clicked` doesn't fire, try the chip and frame paths alone; a click in the page still gives it its own keys natively. Record the outcome in the task's commit message.

- [ ] **Step 5: Route the actions**

Per the spec's table: `step_tab` steps the panel while `right.focused`. `CloseTab` and `ClosePane` close the active tab while `right.focused`; the old browser checks at `render.rs:328,340` go. `pane_command`'s guard stays `pages.shown`, plus an early return while `right.focused`. Reload, address and copy already take `visible_page`.

- [ ] **Step 6: Connect `FocusRight` and `FocusLeft`**

In the `FocusRight` handler, before `pane_command`: when the active pane has no neighbour to the right (`Layout::has_neighbor`) and the panel is shown, `focus_panel(true)`. `FocusLeft` while `right.focused`: `focus_panel(false)`.

- [ ] **Step 7: Run the checks**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 8: Commit**

```bash
git add -A src tests
git commit -m "right panel: a page beside the panes takes the keyboard only when clicked, and tab and page keys follow where focus is"
```

---

### Task 6: Annotations under the page

**Files:**
- Modify: `src/shell/annotations.rs:12,282-345`, `src/shell/right_panel.rs`

**Interfaces:**
- Consumes: `render_annotations(key, theme, cx) -> Option<Div>`, the `Browser` arm from Task 4.

- [ ] **Step 1: Stack the page and the block**

In the `Browser` arm: `div().flex_1().min_h_0().flex().flex_col()`, page first, block second. In `render_annotations`, remove `PANEL_W`; the block is `.w_full().flex_none().max_h(relative(0.4))` with a top border instead of a left one, its rows scrolling inside `overflow_y_scroll`. The page's canvas follows on its own.

- [ ] **Step 2: Run the checks**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 3: Pick, note and send in the mock**

Open a browser tab on a `file://` page, set focus with a temporary `SHEPHERD_WEB`-style env hook if a click is needed, and remove the hook after. Drive it by `postToPid` only.
Expected: the note list under the page, the page narrower in height only, and the note field keeps the keyboard.

- [ ] **Step 4: Commit**

```bash
git add src/shell/annotations.rs src/shell/right_panel.rs
git commit -m "annotations: their notes sit under the page, which needs the panel's width"
```

---

### Task 7: The edges and the finish

**Files:**
- Modify: `src/shell/right_panel.rs`, `src/shell/render.rs`, `assets/NOTICE:29-33`

**Interfaces:**
- Consumes: `fits`, `clamp_width`, `offered`.

- [ ] **Step 1: Window fit**

`render_right_panel` uses `clamp_width(right.width, window_w, sidebar_w)` for the drawn width, never writing it back, so a grown window restores it (Review Focus 3). `drag_right_panel` clamps with the same function.

- [ ] **Step 2: Wayland**

`embeds` for `offered` is false when `window.window_handle()` is `RawWindowHandle::Wayland`, the test `show_web` uses. Move that test into a `fn embeds_pages(window: &Window) -> bool` in `web.rs`, used by both. `sync_browser`'s `pending` goes to `cx.open_url` when it's false.

- [ ] **Step 3: Open and close animation**

Add a `FoldMotion` beside `sidebar_motion` on the panel width. Run it and watch the page in the mock. If the web view visibly lags the column, delete the motion and leave a `// ponytail: instant, the native page can't follow a slide` comment. Don't build a mask.

- [ ] **Step 4: NOTICE**

Add to the Zeron entry in `assets/NOTICE`: `the right panel's tab strip, picker and surface chrome (src/ui/surface_chrome.rs, src/shell/right_panel.rs)`.

- [ ] **Step 5: Whole-branch verification**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: PASS, no warnings. Then run the Windows cross-check per the `windows-cross-check` memory.

- [ ] **Step 6: Walk the spec's "Done means" in the mock**

Each line, by `postToPid` and `screencapture -l`, without activating the window. Report what could not be driven without a mouse rather than calling it passed. Off-macOS focus: try on the UTM Windows VM and state the result.

- [ ] **Step 7: Commit**

```bash
git add -A src assets/NOTICE
git commit -m "right panel: narrow windows hide it, Wayland offers no pages, and Zeron is credited"
```
