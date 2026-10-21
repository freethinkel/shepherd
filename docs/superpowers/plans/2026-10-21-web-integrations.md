# Web Integrations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Local web tools (name + URL entries in Settings) listed in the sidebar above the workspaces and shown in a
native web view in place of the tabs and panes, keeping their state, optionally scoped to one place and opened through
an SSH forward on that place's machine.

**Architecture:** Pure logic (which entries a place lists, link policy, URL normalising and forwarding, favicon
`<link>` parsing) lives in `src/web.rs` with unit tests. The Shell owns the live pages in `src/shell/web.rs`: one
`wry::WebView` per (entry, machine), created with `build_as_child` on GPUI's window, positioned by a canvas in the
main area, hidden with `set_visible(false)` when not shown or when any Shepherd overlay is open. Web view events (title,
loads, links to open outside, new-window requests) come back to the Shell over a `futures` channel.

**Tech Stack:** Rust 2024, GPUI (zui fork), `wry = "0.57"` (WKWebView / WebView2 / WebKitGTK), `gtk = "0.18"` on Linux
only, existing `ureq`, `serde`, `futures`.

**Spec:** `docs/superpowers/specs/2026-10-21-web-integrations-design.md`

## Global Constraints

- Platforms: macOS, Windows, Linux X11. On Wayland a click on an entry opens its URL in the system browser.
- Settings fields have defaults (`#[serde(default)]`); old settings files load unchanged.
- "Show in" default: everywhere. "Open from" default: this computer. Off-site links default: inside Shepherd.
- No Shepherd or herdr state passed to pages: no query parameters, no IPC handler, no initialization script.
- No address bar, bookmarks, history UI, downloads UI, devtools.
- Cookies and storage persist across restarts: `WebContext::new(Some(cache_dir().join("web")))` on Windows and Linux;
  macOS uses WKWebView's default persistent store.
- A page is never reloaded by a switch, a resize, a sidebar toggle or an overlay; only by its reload button, a URL
  edit, or an "Open from: the machine" page whose forward was re-made.
- The web view leaves 6 px at the window's left edge while the sidebar is closed (the peek strip is `w(px(6.))`).
- Comments and names follow the surrounding code: doc comments in plain sentences, `ponytail:` notes for deliberate
  shortcuts.
- The working tree holds the user's uncommitted work (`src/herdr/client_shell.rs`, `src/herdr/session.rs`,
  `src/shell.rs`, `src/shell/floating.rs`, `src/terminal/pane.rs`, staged deletion of `src/shell/client_shell.rs`) and
  the uncommitted session-bar reorder (`src/settings.rs` `place_order`, `src/shell/sessions.rs`, `src/shell/sidebar.rs`,
  `src/shell.rs` `place_motion`). Each task's commit stages only that task's own hunks (write them as a patch and
  `git apply --cached`), never the user's WIP. Ask the user whether to commit the session-bar reorder first.
- Every task ends green on `cargo test --lib` and `make lint` (clippy, warnings as errors).
- Never drive the user's mouse or keyboard. Visual checks use screenshots and env hooks (`SHEPHERD_WEB`, Task 4).

## Review Focus

1. **The main area changes size while a page is shown** (window resize, sidebar toggle or drag, review panel open):
   the web view must follow the area exactly, with no strip of stale page or gap. Pinned by `page_bounds` tests
   (Task 2) and the resize check in Task 4.
2. **An entry's URL is edited, or the entry removed, while its page is alive:** the old page goes, the new URL loads
   on the next show. Pinned by `a_page_is_current_only_for_its_entrys_url` (Task 2).
3. **A URL typed without a scheme** (`localhost:4317`, `127.0.0.1:8080/board`): it loads as `http://…`, not as a
   failed navigation. Pinned by `a_url_without_a_scheme_is_http` (Task 2).
4. **The bar switches to a place where the shown entry isn't listed:** the page hides and that place's tabs show;
   nothing keeps covering the panes. Pinned by `an_entry_tied_to_a_place_lists_only_there` (Task 2) and the switch
   check in Task 8.
5. **"Open from: the machine" when the machine can't be reached:** an error and a retry button within the SSH connect
   timeout (10 s), no hung UI, no blank web view. Pinned by `a_forward_that_cannot_connect_fails` (Task 8).

---

### Task 1: Settings model

**Files:**
- Modify: `src/settings.rs` (struct `Settings`, its `Default`, tests module)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
  #[serde(default)]
  pub struct WebEntry {
      /// Stable for the entry's life, so its page and favicon survive renames, edits and reordering.
      pub id: String,
      pub name: String,
      pub url: String,
      /// The place key (`Place::key`: a session's socket, `machine:<id>`) it is listed in; `None` is everywhere.
      pub show_in: Option<String>,
      pub open_from: OpenFrom,
  }
  #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
  #[serde(rename_all = "lowercase")]
  pub enum OpenFrom { #[default] #[serde(other)] Here, Machine }
  // in Settings:
  pub web_entries: Vec<WebEntry>,
  /// Links to another site open in the system browser instead of in the page.
  pub web_links_outside: bool,   // default false
  impl WebEntry { pub fn new() -> WebEntry }  // fresh id: SystemTime nanos as hex, everything else default
  ```

- [ ] **Step 1: Write the failing tests** in `src/settings.rs` tests:
  ```rust
  #[test]
  fn web_entries_default_to_none_and_round_trip() {
      assert!(Settings::default().web_entries.is_empty());
      assert!(!Settings::default().web_links_outside);
      let (_dir, path) = tmp();
      let mut s = Settings::default();
      s.web_entries.push(WebEntry { id: "a1".into(), name: "board".into(), url: "http://localhost:4317".into(),
          show_in: Some("machine:m1".into()), open_from: OpenFrom::Machine });
      s.web_links_outside = true;
      s.save(&path).unwrap();
      assert_eq!(Settings::load(&path).0, s);
  }
  #[test]
  fn an_unknown_open_from_reads_as_here() {
      let e: WebEntry = serde_json::from_str(r#"{"open_from":"moon"}"#).unwrap();
      assert_eq!(e.open_from, OpenFrom::Here);
  }
  #[test]
  fn new_web_entries_get_distinct_ids() {
      assert_ne!(WebEntry::new().id, WebEntry::new().id);
  }
  ```
- [ ] **Step 2: Run** `cargo test --lib settings::` → FAIL (types missing).
- [ ] **Step 3: Implement** the types and fields above (doc comments as shown). `WebEntry::new` must not repeat ids
  in a tight loop: mix the nanos with a process-wide `AtomicU64` counter.
- [ ] **Step 4: Run** `cargo test --lib settings::` → PASS; `make lint` → clean.
- [ ] **Step 5: Commit** `settings: web integrations' entries (name, URL, where shown, where opened) and the off-site
  links switch`.

### Task 2: Pure web logic

**Files:**
- Create: `src/web.rs` (add `pub mod web;` to `src/lib.rs`)

**Interfaces:**
- Consumes: `WebEntry`, `OpenFrom` (Task 1).
- Produces (all `pub`, in `src/web.rs`):
  ```rust
  /// `localhost:4317` → `http://localhost:4317`; a URL with a scheme is kept; surrounding space trimmed.
  pub fn normalize(url: &str) -> String
  /// The entries listed while the place with `place` key is shown, in the settings' order.
  pub fn listed<'a>(entries: &'a [WebEntry], place: Option<&str>) -> Vec<&'a WebEntry>
  /// Whether `to` leaves `page`'s site: another scheme, host or port. Unparsable `to` (about:, data:) is not off-site.
  pub fn off_site(page: &str, to: &str) -> bool
  /// The favicon's absolute URL from a page's HTML: the first `<link rel=…icon…>`'s href resolved against `page`,
  /// else `<origin>/favicon.ico`.
  pub fn favicon_url(html: &str, page: &str) -> String
  /// For a forward: the URL's (host, port) to reach on the machine (port defaults 80/443 by scheme), and the URL to
  /// load locally through `local_port` (scheme `http`, host `localhost`, same path and query).
  pub fn forwarded(url: &str, local_port: u16) -> Option<((String, u16), String)>
  /// Whether a live page, made for `url`, still belongs to entry `id`: the entry exists and its normalized URL is `url`.
  pub fn page_is_current(entries: &[WebEntry], id: &str, url: &str) -> bool
  /// The web view's rectangle inside the main area: the area itself, less `PEEK_EDGE` at its left when the sidebar
  /// is closed and the area starts at the window's left edge.
  pub fn page_bounds(area: gpui::Bounds<gpui::Pixels>, sidebar_closed: bool) -> gpui::Bounds<gpui::Pixels>
  pub const PEEK_EDGE: f32 = 6.;
  /// Image format by magic bytes, for GPUI's `Image::from_bytes`; `None` when unknown.
  pub fn image_format(bytes: &[u8]) -> Option<gpui::ImageFormat>
  ```
  URL parsing uses `wry::http::Uri` once Task 4 adds wry; until then add `http = "1"` (already in `Cargo.lock` via
  ureq) to `[dependencies]` and use `http::Uri`. Keep it: wry re-exports the same crate, one copy.

- [ ] **Step 1: Write the failing tests** in `src/web.rs`:
  ```rust
  #[test] fn a_url_without_a_scheme_is_http() {
      assert_eq!(normalize(" localhost:4317 "), "http://localhost:4317");
      assert_eq!(normalize("127.0.0.1:8080/board"), "http://127.0.0.1:8080/board");
      assert_eq!(normalize("https://x.dev/a"), "https://x.dev/a");
  }
  #[test] fn an_entry_tied_to_a_place_lists_only_there() {
      let e = |id: &str, show_in: Option<&str>| WebEntry { id: id.into(), show_in: show_in.map(Into::into), ..Default::default() };
      let entries = [e("a", None), e("b", Some("machine:m1")), e("c", Some("/tmp/s.sock"))];
      let ids = |p| listed(&entries, p).iter().map(|e| e.id.as_str()).collect::<Vec<_>>();
      assert_eq!(ids(Some("machine:m1")), ["a", "b"]);
      assert_eq!(ids(Some("/tmp/s.sock")), ["a", "c"]);
      assert_eq!(ids(None), ["a"]);
  }
  #[test] fn off_site_is_another_scheme_host_or_port() {
      let page = "http://localhost:4317/board";
      assert!(!off_site(page, "http://localhost:4317/other?q=1"));
      assert!(off_site(page, "http://localhost:3000/"));
      assert!(off_site(page, "https://localhost:4317/"));
      assert!(off_site(page, "https://github.com/x"));
      assert!(!off_site(page, "about:blank"));
  }
  #[test] fn the_favicon_is_the_pages_icon_link_else_favicon_ico() {
      let page = "http://localhost:4317/board/";
      assert_eq!(favicon_url(r#"<head><link rel="icon" href="/static/i.png"></head>"#, page), "http://localhost:4317/static/i.png");
      assert_eq!(favicon_url(r#"<link href='i.svg' rel='shortcut icon'>"#, page), "http://localhost:4317/board/i.svg");
      assert_eq!(favicon_url(r#"<link rel="icon" href="https://cdn.x/i.ico">"#, page), "https://cdn.x/i.ico");
      assert_eq!(favicon_url("<html></html>", page), "http://localhost:4317/favicon.ico");
  }
  #[test] fn a_forwarded_url_keeps_its_path_on_the_local_port() {
      assert_eq!(forwarded("http://localhost:4317/board?x=1", 50123),
          Some((("localhost".into(), 4317), "http://localhost:50123/board?x=1".into())));
      assert_eq!(forwarded("http://10.0.0.5/", 50123), Some((("10.0.0.5".into(), 80), "http://localhost:50123/".into())));
  }
  #[test] fn a_page_is_current_only_for_its_entrys_url() {
      let entries = [WebEntry { id: "a".into(), url: "localhost:1".into(), ..Default::default() }];
      assert!(page_is_current(&entries, "a", "http://localhost:1"));
      assert!(!page_is_current(&entries, "a", "http://localhost:2"));
      assert!(!page_is_current(&entries, "gone", "http://localhost:1"));
  }
  #[test] fn the_page_leaves_the_peek_edge_while_the_sidebar_is_closed() {
      use gpui::{point, px, size, Bounds};
      let area = Bounds::new(point(px(0.), px(38.)), size(px(800.), px(500.)));
      assert_eq!(page_bounds(area, false), area);
      assert_eq!(page_bounds(area, true), Bounds::new(point(px(6.), px(38.)), size(px(794.), px(500.))));
      let docked = Bounds::new(point(px(256.), px(38.)), size(px(600.), px(500.)));
      assert_eq!(page_bounds(docked, true), docked);
  }
  #[test] fn favicon_formats_by_magic_bytes() {
      assert_eq!(image_format(b"\x89PNG\r\n\x1a\n...."), Some(gpui::ImageFormat::Png));
      assert_eq!(image_format(&[0, 0, 1, 0, 1, 0]), Some(gpui::ImageFormat::Ico));
      assert_eq!(image_format(b"<svg xmlns=..."), Some(gpui::ImageFormat::Svg));
      assert_eq!(image_format(b"<?xml version=\"1.0\"?><svg"), Some(gpui::ImageFormat::Svg));
      assert_eq!(image_format(b"nope"), None);
  }
  ```
  (Also JPEG `FF D8 FF`, GIF `GIF8`, WebP `RIFF????WEBP` in `image_format`.)
- [ ] **Step 2: Run** `cargo test --lib web::` → FAIL.
- [ ] **Step 3: Implement** the functions. `favicon_url`: a plain scan of `<link …>` tags (case-insensitive), reading
  `rel` and `href` attributes in either order with `"` or `'` quotes; no HTML parser dependency. Resolve `href` as
  absolute (has scheme), protocol-relative (`//`), root-relative (`/`) or relative to `page`'s directory.
- [ ] **Step 4: Run** `cargo test --lib web::` → PASS; `make lint` → clean.
- [ ] **Step 5: Commit** `web: which entries a place lists, the link policy, favicon links and forwarded URLs`.

### Task 3: Settings UI

**Files:**
- Modify: `src/ui/settings.rs` (`SettingsView` fields and `new`, `Section::Integrations` render)

**Interfaces:**
- Consumes: `WebEntry`, `OpenFrom`, `Settings::web_entries`, `web_links_outside` (Task 1); `update(cx, …)`.
- Produces: nothing used by later tasks (the Shell reads settings).

Layout, under herdr's integrations card and its note, in the same section:
- `section_label(&theme, "Web pages")`, then a `card` with one `card_row` per entry:
  - name field (`field`, placeholder `"Name"`) and URL field (placeholder `"http://localhost:4317"`), each saved on
    Enter or blur like `herdr_bin` (an `InputEvent::PressEnter | Blur` subscription calling `update`);
  - **Show in**: a `select` with id `"web-show-select"` and options `Everywhere` + the places, labelled by
    `place_looks` name else herdr's name (sessions from `crate::herdr::sessions()`, enabled machines from
    `crate::herdr::machines()`, read once in a background task when the view opens, keyed by the same key as
    `Place::key`: a session's `socket_path`, `machine:<id>`). A saved key no longer among them shows
    `"<key> (gone)"` as the current label. Because `open_select` holds one `&'static str` id, add
    `web_select_entry: usize` naming which entry's list is open;
  - **Open from**: two `choice` chips, `This computer` / `The machine`;
  - a remove `action` ("Remove").
- Under the card: an `action` "Add page" pushing `WebEntry::new()`.
- A `card_row` with a `toggle` "Open links to other sites in the browser" bound to `web_links_outside`, with
  `row_text` description "Off: they open in the page, inside Shepherd."
- Field entities live in `web_fields: Vec<(String /*entry id*/, Entity<InputState>, Entity<InputState>)>`, rebuilt
  in `render` when the entry ids differ from the settings' (add/remove), keeping existing ones so typing isn't lost.

- [ ] **Step 1: Implement** the block as above. No unit test (rendering only; the stored values are Task 1's).
- [ ] **Step 2: Verify by screenshot:** add entries to `scratchpad/shot/settings-demo.json` by hand (one everywhere,
  one tied to a machine, `open_from: machine`), run
  `SHEPHERD_SETTINGS_FILE=<that file> SHEPHERD_SETTINGS=integrations cargo run`, screenshot the window with
  `screencapture`. Expected: both rows, their fields filled, the select showing the machine's name, the right chip
  lit, the toggle off.
- [ ] **Step 3:** `make lint` → clean.
- [ ] **Step 4: Commit** `settings: Integrations lists web pages: name, URL, where shown, where opened, and the
  off-site links switch`.

### Task 4: A page in place of the tabs and panes (this computer, macOS first)

**Files:**
- Modify: `Cargo.toml` (`wry = "0.57"`), `src/shell.rs` (field, render's main column, `selected_tab` writes),
  `src/shell/sidebar.rs` (entry rows), `src/shell/tab_bar.rs`, `src/shell/popups.rs`, `src/shell/pane_drag.rs`, `src/shell/workspaces.rs` (`selected_tab` writes)
- Create: `src/shell/web.rs` (`mod web;` in `src/shell.rs`)

**Interfaces:**
- Consumes: `web::{listed, normalize, page_is_current, page_bounds}` (Task 2); `Settings::web_entries`.
- Produces (in `src/shell/web.rs`):
  ```rust
  /// The live pages and which one is shown.
  #[derive(Default)]
  pub(super) struct Pages {
      /// The entry shown in place of the tabs and panes; `None` shows the tabs and panes.
      pub(super) shown: Option<String>,
      /// By (entry id, the machine's SSH target for an "Open from: the machine" page, else None).
      pub(super) live: HashMap<(String, Option<String>), Page>,
      pub(super) favicons: HashMap<String, Favicon>,   // Task 7
      #[cfg(not(target_os = "macos"))] context: Option<wry::WebContext>,
      events: Option<futures::channel::mpsc::UnboundedSender<(PageKey, PageEvent)>>,
  }
  pub(super) type PageKey = (String, Option<String>);
  pub(super) struct Page {
      pub(super) view: Rc<wry::WebView>,
      /// The normalized URL it was made for (`page_is_current`), or the local forwarded URL.
      pub(super) url: String,
      pub(super) title: Option<String>,
      /// Whether `set_visible(true)` was last applied, so it isn't called every frame.
      visible: bool,
      pub(super) _tunnel: Option<crate::herdr::Tunnel>,   // Task 8
  }
  pub(super) enum PageEvent { Title(String), Loaded, Navigate(String), Outside(String) }
  impl Shell {
      /// The entry clicked in the sidebar: its page shown (made on first show); on Wayland its URL in the browser.
      pub(super) fn show_web(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>);
      /// Every user selection of a tab goes through here; it leaves the page for the tabs.
      pub(super) fn select_tab(&mut self, tab: Option<String>);
      /// The key of the place shown in the bar (`Place::key`), if any.
      pub(super) fn shown_place_key(&self) -> Option<String>;
      /// The page area (replaces the pane grid while a page is shown): a canvas that sets the web view's bounds in
      /// prepaint, and nothing else drawn.
      pub(super) fn render_web_area(&mut self, key: &PageKey, sidebar_closed: bool, cx: &mut Context<Self>) -> Div;
      /// The tab bar row while a page is shown: ‹ › ⟳ and the page's title (entry name until it has one).
      pub(super) fn render_web_bar(&mut self, theme: &Theme, sidebar_w: f32, window: &mut Window, cx: &mut Context<Self>) -> Div;
      /// Applies visibility: only `shown`'s page visible, and only when `covered` is false; drops pages whose entry
      /// is gone or whose URL changed (`page_is_current`); clears `shown` when the place shown no longer lists it.
      pub(super) fn sync_pages(&mut self, covered: bool, cx: &mut Context<Self>);
  }
  ```

Details the implementer can't infer:
- Make the page with
  `wry::WebViewBuilder::new()` (`new_with_web_context(ctx)` off macOS) `.with_url(url).with_bounds(rect)
  .with_focused(true).with_document_title_changed_handler(..).with_on_page_load_handler(..)
  .with_navigation_handler(..).with_new_window_req_handler(..).build_as_child(window)`; `window` is GPUI's `&Window`
  (it implements `HasWindowHandle`). `Rect` from `wry::dpi::{LogicalPosition, LogicalSize}` with GPUI's logical px.
  The handlers only send `(key, PageEvent)` on the channel (they run without `cx`); one `cx.spawn` loop started with
  the first page receives them and updates the Shell (`Title` → `page.title`, `Loaded` → `cx.notify()` so back/forward
  re-read `can_go_back`/`can_go_forward`). Navigation/new-window policy is Task 6: until then navigation returns
  `true` and new-window `NewWindowResponse::Deny` + `Navigate(url)` → `view.load_url`.
- The web bar keeps `render_tab_bar`'s outer row (height, `theme.surface`, the `pl`/`pr` that clear the traffic
  lights, sidebar toggle and window buttons) and puts in it three `icon_button`s (`icons/solar/arrow-left.svg`,
  `arrow-right.svg`, `refresh.svg`, or the nearest Solar names in `assets/icons/solar`), 28 px like the `+`, then the
  title truncated in `theme.text_muted`. ‹ and › are dimmed and inert when `can_go_back()` / `can_go_forward()` is
  false (read each render); ⟳ calls `reload()`.
- Wayland: `match HasWindowHandle::window_handle(window).map(|h| h.as_raw())`, `RawWindowHandle::Wayland(_)` →
  `cx.open_url(&url)` and return without showing.
- The main column in `render`: when `self.pages.shown` is `Some` and its entry is listed for `shown_place_key()`,
  `tabbar` is `render_web_bar(...)` and `grid` is `render_web_area(...)`; the status bar stays. Pass `shown_ws: None`
  to `render_sidebar` then, so no workspace row looks selected.
- Sidebar rows: in `render_sidebar`, between the titlebar padding and the swiping list (outside the `left(list_offset)`
  container, so they don't move with a swipe), one row per `listed(...)` entry: 16 px favicon slot (Task 7; until then
  `place_mark(name, None, false, theme.text_muted)`), the name, selected style (`theme.selected`) when shown, click →
  `show_web`. A 1 px `theme.border` separator under them when any.
- `select_tab`: replace these assignments with it: `shell.rs` 643, 668, 673, 961, 971, 982, 988; `popups.rs` 793;
  `pane_drag.rs` 297, 446; `tab_bar.rs` 601; `sidebar.rs` 169, 337; `workspaces.rs` 73, 122. Leave `sync.rs` 103
  (herdr's own focus move) and `sessions.rs` 567 (a place switch: `sync_pages` decides) as plain assignments.
- Focus: showing a page calls `view.focus()`; leaving it (`select_tab`, an overlay) calls `view.focus_parent()` then
  focuses `self.focus` as render does when nothing has focus.
- `SHEPHERD_WEB=<entry name>`: on the first render, `show_web` for the entry with that name (screenshots), next to
  the other `SHEPHERD_*` hooks.

- [ ] **Step 1:** add `wry = "0.57"` to `[dependencies]`; `cargo build` → compiles on macOS.
- [ ] **Step 2: Implement** `src/shell/web.rs` and the hooks above (macOS path; Linux pumping is Task 9).
- [ ] **Step 3: Verify by screenshot.** Run a page server: `python3 -m http.server 4317 -d <scratchpad>/site` with an
  `index.html` holding a form and a long scrolling body. Settings file with an entry `board` → `http://localhost:4317`.
  `SHEPHERD_SETTINGS_FILE=… SHEPHERD_WEB=board cargo run`; screenshot. Expected: the sidebar row selected, the bar
  showing ‹ › ⟳ and the page title, the page filling exactly the area under the bar (no gap at its edges), status bar
  under it.
- [ ] **Step 4: Verify resize** (Review Focus 1): with the page shown, resize the window with
  `osascript -e 'tell application "System Events" to tell process "Shepherd" to set size of window 1 to {900, 600}'`
  (moves no pointer), and relaunch with `sidebar_open: false` in the settings file; screenshot both. Expected: the
  page edge flush with the area; with the sidebar closed a 6 px strip at its left.
- [ ] **Step 5: Verify state kept:** the test page writes `Date.now()` into its body on load. After the resize in
  Step 4 the timestamp in the screenshot is unchanged (no reload). Switching away and back needs clicks: list it for
  the user to check by hand rather than claiming it.
- [ ] **Step 6:** `cargo test --lib` and `make lint` → green.
- [ ] **Step 7: Commit** `web: an entry in the sidebar shows its page in a system web view in place of the tabs and
  panes, with back, forward, reload and its title in the tab bar row; pages keep their state while hidden`.

### Task 5: Overlays hide the page

**Files:**
- Modify: `src/shell/web.rs`, `src/shell.rs` (render computes `covered`), `src/ui/toast.rs`

**Interfaces:**
- Consumes: `sync_pages(covered, cx)` (Task 4).
- Produces: `pub fn any_open(cx: &App) -> bool` in `src/ui/toast.rs` (a toast whose `popup.is_open()`).

`covered` in `render` is true when any of these is open: `self.settings`, `self.command_panel`, `floating_open()`,
`peek_now() > 0.`, `toast::any_open(cx)`, and each `Popup` field (`is_open()`): `view_menu`, `agents_menu`, `ws_menu`,
`pane_menu`, `spent_menu`, `usage_menu`, `statusbar_menu`, `tab_menu`, `worktree_dialog`, `remove_confirm`,
`session_dialog`, `place_menu`, `place_edit`, `session_delete`. Put the list in one
`fn overlay_open(&self, cx: &App) -> bool` in `src/shell/web.rs` with a comment that a new popup field belongs in it.
The web bar's buttons get no `.tooltip(...)`.

- [ ] **Step 1: Implement.**
- [ ] **Step 2: Verify by screenshot:** `SHEPHERD_WEB=board SHEPHERD_PANEL=commands cargo run` → the command panel is
  visible over the main area with the page hidden behind it; `SHEPHERD_WEB=board SHEPHERD_SETTINGS=general` → settings
  shown, no page over them.
- [ ] **Step 3:** `make lint` → clean.
- [ ] **Step 4: Commit** `web: Shepherd's panels, dialogs, menus, popups, toasts and the sidebar's peek hide the
  page while open`.

### Task 6: Links

**Files:**
- Modify: `src/shell/web.rs`

**Interfaces:**
- Consumes: `web::off_site` (Task 2), `Settings::web_links_outside` (Task 1).

Navigation handler: `off_site(page.url, to) && links_outside` → send `Outside(to)` and return `false`; else `true`.
`links_outside` is read when the page is made and kept in an `Rc<Cell<bool>>` the Shell updates in `sync_pages`, so
flipping the switch applies to live pages. New-window handler: `Outside(url)` under the same condition, else
`Navigate(url)`; always `NewWindowResponse::Deny`. `Outside` → `cx.open_url`. `Navigate` → `view.load_url`.

- [ ] **Step 1: Implement.**
- [ ] **Step 2: Verify** with the test site: links to `https://example.com` (plain and `target=_blank`) and a
  `window.open` button. With `web_links_outside: false`, `SHEPHERD_WEB=board` and the page's own script clicking the
  `target=_blank` link after 2 s (`setTimeout(() => a.click(), 2000)`): screenshot shows example.com inside. With
  `true`: the system browser opens example.com and the page stays.
- [ ] **Step 3:** `make lint` → clean.
- [ ] **Step 4: Commit** `web: links to other sites and new windows open in the page, or in the browser when the
  setting says so`.

### Task 7: Favicons

**Files:**
- Modify: `src/shell/web.rs`, `src/shell/sidebar.rs` (row icon)

**Interfaces:**
- Consumes: `web::{favicon_url, image_format}` (Task 2).
- Produces: `pub(super) enum Favicon { Loading, Image(Arc<gpui::Image>), None }` in `Pages::favicons`, by entry id.

For "this computer" entries, fetched in a background task the first time the entry is listed: GET the URL with
`ureq` (5 s timeout), `favicon_url(body, url)`, GET it, `image_format`, `gpui::Image::from_bytes`. Any failure →
`Favicon::None`, letter shown, not retried until the URL changes or Shepherd restarts (`ponytail:` note: in memory
only, refetched each launch). "The machine" entries fetch through their forward when their page is first made
(Task 8). The row shows `img(image).size(px(16.)).rounded(px(3.))`, else the letter mark.

- [ ] **Step 1: Implement.**
- [ ] **Step 2: Verify by screenshot:** the test site serves `favicon.ico` (a PNG renamed), a second entry points at
  a site with `<link rel="icon" href="/i.svg">`; both rows show their icons; an entry for a dead port shows its letter.
- [ ] **Step 3:** `make lint` → clean.
- [ ] **Step 4: Commit** `web: an entry's sidebar icon is its page's favicon, else its name's first letter`.

### Task 8: Show in a place, open from the machine

**Files:**
- Modify: `src/herdr/remote.rs` (forward), `src/herdr/mod.rs` (re-export), `src/shell/web.rs`

**Interfaces:**
- Consumes: `web::{listed, forwarded}` (Task 2); `crate::herdr::shown_machine() -> Option<String>` (the SSH target).
- Produces (in `src/herdr/remote.rs`, re-exported as `crate::herdr::{forward, Tunnel}`):
  ```rust
  /// An SSH local forward; the ssh process is killed when it drops.
  pub struct Tunnel { child: std::process::Child, pub port: u16 }
  /// Forwards a free local port to `host:port` as seen from `target`, over a connection of its own (never the shared
  /// ControlMaster, whose forwards outlive the client), and returns once the port accepts a connection. Blocking: call
  /// it off the main thread. Fails when ssh exits first or nothing accepts within 15 s.
  pub fn forward(target: &str, host: &str, port: u16) -> std::io::Result<Tunnel>
  ```
  Split `ssh_options()` into `ssh_base_options()` (without the three ControlMaster/Persist/Path options) and
  `ssh_options()` = base + shared; `forward` uses the base plus `-N -o ExitOnForwardFailure=yes
  -L 127.0.0.1:<free>:<host>:<port> -- <target>`. The free port: bind `TcpListener` on `127.0.0.1:0`, read it, drop.

Shell side: an entry with `OpenFrom::Machine` while a machine is shown keys its page `(id, Some(target))`; with no
machine shown (a local session) it is a "this computer" page `(id, None)`. On first show the page area shows
"Connecting to <machine>…" (the `loader::spinner` as the grid's connecting state) while `forward` runs in
`cx.background_spawn`; on success the page is made with `forwarded(url, tunnel.port).1` and keeps the `Tunnel`; on
failure the area shows the error and a "Retry" `action`. Once the tunnel is up, the entry's favicon is fetched
through it as in Task 7 (Task 7's fetch, given the forwarded URL). `sync_pages` clears `shown` when `listed(entries,
shown_place_key())` no longer holds it (Review Focus 4).

- [ ] **Step 1: Write the failing test** in `src/herdr/remote.rs` tests:
  ```rust
  #[test]
  fn a_forward_that_cannot_connect_fails() {
      // a target that can't resolve: ssh exits at once, well inside the 15 s wait
      let at = std::time::Instant::now();
      assert!(forward("nobody@shepherd-no-such-host.invalid", "localhost", 4317).is_err());
      assert!(at.elapsed() < std::time::Duration::from_secs(15));
  }
  ```
- [ ] **Step 2: Run** `cargo test --lib remote::` → FAIL (no `forward`).
- [ ] **Step 3: Implement** `forward`, `Tunnel` (with `Drop` killing and reaping the child), the option split, and
  the Shell side.
- [ ] **Step 4: Run** `cargo test --lib remote::` → PASS.
- [ ] **Step 5: Verify against a real machine:** if the user has a herdr machine saved (`herdr machine list`), run
  the test site on it (`ssh <target> 'python3 -m http.server 4317 -d /tmp'`), an entry with `open_from: machine`,
  `show_in` that machine's key, settings `herdr_machine` set to it, `SHEPHERD_WEB=<name>`; screenshot shows the
  remote directory listing. Switch `herdr_machine` to none and relaunch: the entry is not listed (Review Focus 4).
  If no machine is saved, say so instead of claiming it works.
- [ ] **Step 6:** `make lint` → clean.
- [ ] **Step 7: Commit** `web: an entry can be listed in one place only, and opened on the shown machine through an
  SSH forward of its port`.

### Task 9: Linux and Windows

**Files:**
- Modify: `Cargo.toml` (`[target.'cfg(target_os = "linux")'.dependencies] gtk = "0.18"`), `src/shell/web.rs`,
  `scripts/linux.Dockerfile`, `.github/workflows/release.yml`, `Cargo.toml` `[package.metadata.generate-rpm.requires]`

**Interfaces:**
- Consumes: Task 4's page creation.

- Linux: before the first page, `gtk::init()` once (an `Err` → toast "Web pages need GTK: <e>" and open in the
  browser instead); while any page lives, a `cx.spawn` loop every 16 ms runs
  `while gtk::events_pending() { gtk::main_iteration_do(false); }`, stopping when the last page drops.
- Packages: add `libwebkit2gtk-4.1-dev libgtk-3-dev` to the Dockerfile's apt list and the release workflow's apt
  line; rpm requires `webkit2gtk4.1 = "*"`; the deb's `$auto` picks up libwebkit2gtk through shlibdeps.
- Windows: WebView2 needs the window thread in a COM STA; if `build_as_child` fails with a COM error, call
  `windows::Win32::System::Com::CoInitializeEx(None, COINIT_APARTMENTTHREADED)` once before the first page.

- [ ] **Step 1: Implement.**
- [ ] **Step 2: Linux:** `make linux` → builds; `make lint` inside the same container
  (`docker run … shepherd-linux make lint`) → clean.
- [ ] **Step 3: Windows:** clippy for `x86_64-pc-windows-msvc` from Linux Docker (xwin, as in the windows
  cross-check note) → clean; in the UTM Windows VM (`ssh egor@192.168.64.17`), build and run with `SHEPHERD_WEB` set
  and the test site served from the VM, screenshot from the VM session. Expected: the page in the main area.
- [ ] **Step 4: Linux X11 run:** if an X11 environment is reachable, the same screenshot check; otherwise say it was
  built but not run.
- [ ] **Step 5: Commit** `web: pages on Linux (X11, GTK pumped beside GPUI) and Windows (WebView2); packages depend
  on WebKitGTK`.

### Task 10: Keyboard check

**Files:** none unless a fix is found.

The spec forbids silently losing Shepherd's shortcuts while a page has focus. With `SHEPHERD_WEB=board` (the page
focused at show) send keystrokes via `osascript -e 'tell application "System Events" to keystroke "k" using command
down'` **only with the user's go-ahead** (it drives their keyboard): ⌘K, ⌘B, ⌘,, ⌘1, ⌘T, ⌘W. Report which reach
Shepherd. If some don't, propose the fix (a macOS menu item for the action, so AppKit's key equivalent reaches it
before the web view) and bring it back to the user rather than shipping a silent loss.

- [ ] **Step 1:** ask the user, run the check, report.
