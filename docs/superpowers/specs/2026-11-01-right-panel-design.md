# Right panel: browser and pull request beside the terminals

## Intent

A browser tab today replaces the panes, so a page and the agent working on it are never on screen together. The
pull request panel sits on the right but is a one-off. Shepherd gets one right panel, as Zeron and T3 Code have:
a column of tabs beside the terminals that holds the browser and the PR view, and later diffview and reviewer.

Zeron is the reference. Its code is in `~/Developer/forks/zeron` (MIT); port it nearly word for word and add the
line to `assets/NOTICE`. Where this document and Zeron disagree on look or feel, Zeron wins; where Zeron has what
Shepherd lacks (sessions, a composer, a route), this document says what to do instead.

## Order of work

Decided: browser annotations (SHE-10, branch `annotatsii-na-veb-stranitse-dlya`) land on `main` first, then this.
Building starts from that `main`, which must also have the work now uncommitted there: `Layout::has_neighbor`, which
`FocusRight` below relies on, and `menu_mark`. This document assumes both.

## Goals

- A page stays visible next to the panes, and typing in a terminal keeps working while it does.
- Browser tabs leave the tab bar; it holds herdr's tabs only.
- The PR view stops being a separate panel and becomes a tab.
- Adding a tool later means one enum variant, one render arm and one row in the `+` list.

## Non-goals

- A terminal, file, or side chat tab in the panel. Zeron has them; they can come later.
- Zeron's takeover (expand) button, and its files column.
- A plugin API. "Extensions" here means more built-in tab types.
- Restoring tabs or open state after a restart.
- Web pages from the left sidebar: they stay where they are and still open in place of the tabs and panes.

## Anti-goals

- Focus stolen from a terminal. Opening or switching a page must never take keystrokes from an agent the user is
  typing to, unless the user clicked into the page.
- The tab bar still showing, or reserving room for, browser tabs.

## The panel

Layout follows Zeron's `render_right_pane`: flush, full window height, a left hairline, `theme.surface` as today's
review panel. The status bar stays under the centre column, so the panel runs past it.

**Tab strip** (`render_right_tab_strip`, `surface_chrome::{tab, tab_chip, tab_chip_icon, toolbar}`): sits in the
titlebar band above the panel, `TITLEBAR_HEIGHT` tall. One 112 × 24 chip per tab: icon, truncated title, ✕ on hover.
The strip scrolls sideways with edge fades, chips reorder by drag, a `+` follows the last chip, and a ✕ at the right
end closes the panel, clear of `window_controls_w` off macOS.

- Browser chip: favicon, a spinner while loading, the page's title (today's `browser_label`).
- PR chip: PR icon, `PR #42` / `MR !42`, plain `Pull request` until known, a spinner while `fetching`.

**Empty panel** (`render_surface_picker`): rows of Browser and Pull request. `+` opens a menu with the same rows
(`right_plus`). The Pull request row only appears where the shown pane's repo has a remote (`has_remote`), as Zeron
gates git rows on `space_git_detected`.

**Lifecycle**, as in Zeron:

- The active tab is validated each frame; if it's gone, the most recently visited tab, else the picker.
- Closing the last tab closes the panel (`close_empty_right_pane`). Opening it again shows the picker.

**Width:** one width for the whole panel, dragged at its left edge with `GridHold` as the review panel's is, double
click for the default. `right_panel_width` in settings: default 520, min 360, max
`window − left sidebar − 300` (Zeron's `RIGHT_PANE_DEFAULT`, `RIGHT_PANE_MIN`, `CHAT_PANEL_MIN`). `review_width` and
`review_open` leave the settings; old files must still load, and neither is migrated.

A window too narrow for the panel's minimum hides it without losing its state; it returns when there is room. The
open-order ranking in Zeron's `HorizontalPanelFit` is not needed, there being one panel.

## Per workspace

`HashMap<workspace id, PanelState>` holding the tabs in strip order, the active tab, and open or closed. In memory
only, so a launch starts with every panel closed: this is a change for the PR panel, whose open state was saved.

Switching workspace swaps the whole panel; pages of other workspaces stay alive and hidden, as today. A workspace
gone takes its panel and pages with it (the `retain` in `sync_browser`). The existing limit stays: showing another
place (session, machine) closes this place's browser tabs. A worktree's workspace is a workspace like any other.

The panel hides while Settings or a sidebar Web page is shown, and keeps its state. *(My call: a sidebar page has no
workspace selected, so there is no panel to show, and it keeps one web view on screen at a time.)*

## Browser tabs

`BrowserTab` drops `after`; `Browser` drops `seen`. `follows`, `interleave`, `dropped`, `drop_tab`, `browser_tabs_of`
and their tests go, as does every `browser` branch in `tab_bar.rs` (`Shown.browser`, `loading`, `step_tab`'s) and
`browsing` in `render.rs`. The tab bar's `+` stays a new terminal; the chevron `new-tab-more` and `new_tab_menu` go.

`Pages.shown` means sidebar pages only. The panel's active browser tab is a second source of the page to show:
`sync_pages` makes visible whichever applies, never both. `render_browser_bar` and `render_web_area`, with what
annotations added to them, are reused under the strip.

**Annotations** come along with the browser tab; picking, the toolbar button, Esc and sending to an agent don't
change. Only the place of the notes does: decided, they sit in a block under the page instead of a 300 px column
beside it, since a 520 px panel would leave the page 220 px. *(My call)* The block takes the panel's full width, is
gone with no picks, and caps at 40% of the panel's height, scrolling inside. The page is what resizes.

**Links from a terminal** (`web_links_inside`): a new browser tab in the shown workspace's panel, which opens and
shows it. The keyboard stays in the terminal. Off macOS's layered pages, `overlay_open` and `popup_open` apply as
now; `right_plus` joins `popup_open`.

**Wayland** cannot embed a page, so *(my call)* Browser leaves the `+` list and picker there, and links go to the
system browser. Today's flow leaves an empty tab behind.

## Pull request tab

*(My call)* One per workspace, not several as in Zeron, and it keeps following the shown pane's branch as
`shown_review` does now: it's the same view, moved. `render_review` loses its header row: title and ✕ are the chip's
and the strip's, Refresh moves to a `surface_chrome::toolbar` row under the strip. `render_review`'s resize edge goes
to the panel. `poll_review` runs while a PR tab is active and the panel is shown, or the status bar needs it, as now.

A pane moved to a repo without a remote leaves the tab in place, showing what `Review::NoRemote` shows. The panel no
longer vanishes by itself.

## Opening and closing

- **`ToggleRightPanel`** (new): `cmd-alt-b` on macOS, `ctrl-alt-b` elsewhere, taken from `ToggleReview`. A button at
  the tab bar's right end while the panel is closed, in the spot the strip's ✕ holds while open.
- **`ToggleReview`** keeps its id, so a user's rebinding survives, and has no default chord. It opens the panel on
  the PR tab (creating it), or closes the panel if it's already there. The status bar's PR label runs it, and stays
  lit while the PR tab is shown.
- Opening with tabs present shows the last active one; with none, the picker.

*(My call)* Opening animates with the left sidebar's `motion` if the web view follows it cleanly each frame; if it
lags, the panel opens at once. Don't spend on masking it.

## Keyboard and focus

This is the one real behaviour change: today `sync_pages` hands the keyboard to any visible page, which would take
it from the terminal the moment a page is beside it.

- A panel page takes the keyboard only after a click in it or on its chip. The address field and an annotation's
  note hold it while edited, as `sync_pages` lets them now. On macOS the click is `PageEvent::Clicked`. A new blank
  tab focuses its address field.
- A click in a pane takes the keyboard back (`focus_parent`), and a pane becomes the target of pane commands.
- `Shell` keeps one bool, "focus in panel", as Zeron's `navigation_focus.in_right`. It decides the below.

| Action | Focus in panel | Otherwise |
| --- | --- | --- |
| `NextTab`, `PreviousTab` | panel tabs | herdr tabs |
| `CloseTab`, `ClosePane` | closes the active panel tab | as today |
| `NewTab`, `Tab1`…`Tab9` | herdr, always | herdr |
| `ReloadPage`, `FocusAddress`, `CopyPageUrl` | visible page | visible page, so `cmd-r` reloads beside a terminal |
| `SplitRight`, `SplitDown`, `ToggleZoom` | nothing | as today |

`pane_command`'s "a page hides the panes" check becomes `pages.shown` alone, which is what it now means.

*(My call)* `FocusRight` from the rightmost pane moves into the panel, `FocusLeft` from the panel returns to the last
pane, so it can be reached without the mouse.

Off macOS there is no `Clicked`. Find the equivalent, or accept focus in panel stays false there while the page
still gets its own keys, and record which. Check on the Windows VM.

## Code shape

New `src/shell/right_panel.rs`:

```rust
enum Surface { Browser(String), PullRequest }
struct PanelState { tabs: Vec<Surface>, active: Option<Surface>, open: bool } // active None is the picker
```

plus width, drag, `+` popup and focus-in-panel on `Shell`. Zeron's `RightSurface::{File, Diff, Terminal, ...}` aren't
ported. `surface_chrome.rs` comes across as a file. Ports: `render_right_pane`, `render_right_tab_strip`,
`render_surface_picker`, `right_surface_rows`, `reorder_right_tabs`, `close_right_surface`,
`resolved_right_active`, `SessionPanels::right_surface_fallback`, `shell/tabs.rs`, and the `+` menu at `shell.rs:12940`.

## Done means

- A page and two split panes on screen; typing in a pane reaches the pane while the page reloads.
- A terminal link opens beside the terminal and the terminal keeps the keyboard.
- Tab bar shows herdr's tabs only, in every state.
- PR view works as before from its tab: refresh, merge dialog, logs, selection and Cmd+C.
- Annotations work from a panel page: pick, note under the page, send to an agent.
- Two workspaces keep separate tabs; a closed workspace leaves nothing behind.
- Unit tests for the pure logic: active fallback on close, last tab closes the panel, reorder, width clamp.
- Old `settings.json` with `review_open` and `review_width` loads.
- UI checked in a `SHEPHERD_MOCK` instance, not the live one. `cargo clippy` clean, Windows cross-check clean.

## Left to the builder

The steps within the work, how `render_review` splits, whether Zeron's drag slide or Shepherd's `reorder.rs` fits the strip
better, and the right-click menu on chips (carry over whatever a browser tab had in the tab bar).
