# Web integrations: local web tools in Shepherd's window

## Intent

The user is building local web tools for herdr (dashboards and the like, served on `localhost`). They want them in
Shepherd's window: listed in the sidebar and shown in place of the tabs and panes, kept as they were left. They
should not live as separate apps or as tabs in a browser. "Integrations" means these pages: Shepherd shows them and
nothing more. The page does not need anything from Shepherd: no session, no theme, no pane.

## Goals

- In Settings → Integrations, below herdr's agent integrations, a list of entries, each a **name and a URL**. There
  can be any number. Each can be added, edited and removed there.
- Each entry has a **"Show in"** setting: **everywhere** (default), or **one place**, a herdr session or a machine
  from the session bar. An entry tied to a place is listed in the sidebar only while that place is shown.
- Each entry has an **"Open from"** setting: **this computer** (default) opens the URL as written. **The machine**
  opens it on the machine of the place shown: for a remote machine, the URL's host and port are reached through an
  SSH port forward, so `localhost:4317` means port 4317 on that machine. For a local herdr session it is the same as
  this computer.
- Each entry's icon is the page's **favicon**.
- The entries are listed in the sidebar **above the workspaces**, each as favicon + name.
- Clicking one shows the page in the **system web view** (WKWebView on macOS, WebView2 on Windows, WebKitGTK on
  Linux), through the Tauri team's **`wry`** crate. The page fills the whole area where the tabs and panes are now.
- The page **keeps its state** while Shepherd runs. Switching to a workspace hides it, and coming back shows it
  exactly as it was left: scroll, form input, open connections. Pages are not reloaded on switch.
- While a page is shown, the **tab bar row** shows **back, forward, reload** and the page's title instead of the
  tabs (mockup "A" in the brainstorm).
- A link to **another site** opens **inside Shepherd** by default. One setting in Settings → Integrations sends such
  links to the system browser instead.
- macOS, Windows and Linux (X11).

## Non-goals

- **Wayland.** wry can only embed into a GTK window there, and Shepherd's window isn't one. Deferred. Until then, on
  Wayland, clicking an entry opens its URL in the system browser. *(Builder's call, made in conversation, easy to
  change.)*
- Anything that passes Shepherd or herdr state to the page (query parameters, a JS bridge).
- An address bar, bookmarks, history, downloads UI, devtools.
- Reordering entries in the sidebar by drag (order is the settings list's).

## Anti-goals

- Shepherd's own overlays hidden under the page. The native web view sits above everything GPUI draws, so the
  command panel, settings, dialogs (image viewer, Markdown reader), menus and popups must not end up beneath it.
- Breaking Shepherd's keyboard shortcuts (⌘K, ⌘1…9, ⌘B, ⌘, …) while a page has focus, unless the platform makes
  it unavoidable. If so, that is a finding to bring back, not a silent loss.
- A page reloading or losing its state on a tab switch, a session switch, a sidebar toggle or a window resize.

## Constraints

- GPUI's `Window` implements `raw_window_handle::HasWindowHandle` (zui fork, `crates/gpui/src/window.rs`). wry's
  `WebViewBuilder::build_as_child` takes that handle on macOS, Windows and Linux X11.
- On Linux, WebKitGTK needs GTK initialized and its main loop pumped alongside GPUI's. Packages gain a runtime
  dependency on `libwebkit2gtk-4.1`, and the Docker build (`make linux`) gains its dev package.
- Windows needs the WebView2 runtime (present on Windows 11).
- Settings are `src/settings.rs` (`#[serde(default)]` struct, saved as JSON). The settings UI is `src/ui/settings.rs`,
  whose `Section::Integrations` today lists herdr's agent integrations.
- The sidebar is `src/shell/sidebar.rs` (`render_sidebar`, `workspace_rows`). The tab bar is `src/shell/tab_bar.rs`
  (`render_tab_bar`), placed in `src/shell.rs`'s render.
- `ureq` is already a dependency, for fetching favicons.
- Remote machines are reached over SSH in `src/herdr/remote.rs` (`ssh_options()`: a shared `ControlMaster`
  connection per target). The shown place is `remote::shown_machine()`; the bar's places are in
  `src/shell/sessions.rs`, keyed by the place's key (the same key `place_order` saves).

## Decided in conversation

- Name + URL entries in Settings → Integrations, any number. Favicon as icon. Listed above workspaces.
- Full system web view through wry, in place of tabs and panes, not a workspace or a tab.
- State kept across switches for as long as Shepherd runs.
- Back / forward / reload and title in the tab bar row (mockup A).
- Off-site links open inside by default. A setting sends them to the browser.
- Wayland deferred.
- No Shepherd context passed to pages.
- Each entry has two more settings: **where it shows** (everywhere by default, or one place from the session bar)
  and **where its URL opens** (this computer by default, or the machine of the place shown).

## Builder's calls (made by me, flagged for review)

- **Where in Settings:** a block in the existing Integrations section, under herdr's agent integrations. The links
  setting is one global switch there, not per entry.
- **Wayland fallback:** open the URL in the system browser.
- **Lifecycle:** a page's web view is created on its first open and lives until Shepherd quits or the entry is
  removed. Hidden pages are hidden (`set_visible(false)`), not destroyed.
- **Cookies and storage persist across restarts** (a login to a local tool survives): a data directory under
  Shepherd's cache dir on Windows/Linux, and the default persistent store on macOS.
- **Back / forward** show as disabled when there's no history (wry 0.57 has `can_go_back` / `can_go_forward`).
- **Server not running:** the engine's own error page shows, and reload retries. No custom screen.
- **Favicon:** fetched from the page (`<link rel="icon">`, else `/favicon.ico`) and cached. Until it arrives, or if
  it can't be had, the entry shows its name's first letter, like the session bar.
- **Overlays:** every Shepherd overlay hides the page while it is open: the command and agents panels, settings,
  dialogs, context menus, herdr's plugin popups, the closed sidebar's peek, and toasts. Menus are opened by a click,
  so hiding for them isn't flicker; toasts are rare (errors, the update note) and a click closes them. The bar's own
  buttons have no tooltips, which would open down over the page. The closed sidebar's peek keeps working: the web
  view leaves the few pixels at the window's left edge that trigger it. No page snapshots.
- `target=_blank` and `window.open` follow the same rule as off-site links: inside by default.
- **"Show in" stores the place's key.** If that place is gone (session stopped, machine removed), the entry stays in
  Settings, marked as such, and shows nowhere until it's changed.
- **"Open from: the machine" keeps one page per machine.** Switching the bar to another machine shows that
  machine's page, with its own state; going back shows the first one as it was left.
- **The forward:** a free local port forwarded over the machine's existing SSH control connection
  (`ssh -O forward -L`, or a separate `ssh -N -L` if that fails), opened on the page's first open there and closed
  when its page is dropped or Shepherd quits. The web view loads `http://localhost:<free port>` plus the URL's path.
  Off-site links aren't forwarded: they load from this computer.
- **Forward fails** (machine offline, SSH error): the page area shows the error and a retry button, no web view.
- **Favicon for "the machine" entries** is fetched through the same forward, on the first open; until then the
  letter.

## Left to the builder

- How the web view's bounds follow the main area: window resize, sidebar resize or toggle, review panel open.
- How the shown page is modelled next to the shown tab and workspace (Shell state), and how ⌘↑/⌘↓ or the swipe
  treat it.
- Favicon formats beyond PNG (`image` is built with `png` only; ICO/SVG may need features or a conversion).
- The GTK main-loop pumping on Linux, and keeping the X11 child window stacked correctly.
- Tests: the settings round-trip, the link policy (same-origin vs off-site), favicon `<link>` parsing, which entries
  a place lists, the forwarded URL rewrite.
