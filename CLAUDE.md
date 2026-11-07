# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# Shepherd

A native GPUI client for herdr (and, as experiments, herdr-core, Rex and tmux): sidebar, tabs, panes, agents.

## Build and test

`make help` lists the targets. libghostty-vt builds with Zig 0.16: an environment `ZIG` wins over the one-machine
path in `.cargo/config.toml`.

- `make test`, `make lint` (clippy, `-D warnings`, as CI). One test: `cargo test --lib <module>::tests::<name>`.
- `#[ignore]` tests are live: they drive a real backend server and close what they open, so give them an isolated
  one. Each one's doc comment names its env and command (`SHEPHERD_TMUX_SOCKET`, `REX_SERVER`, `HC_SESSION`).
- `cargo run` starts the app from inside `target/debug/Shepherd.app` (`scripts/run-bundled.sh`, the cargo runner)
  and attaches to the user's live herdr session. To look at UI yourself, run on the fixture: `SHEPHERD_MOCK=docs/mock`
  with `SHEPHERD_SETTINGS_FILE`, as `docs/mock/shot.sh` does.
- Releases: bumping `version` in `Cargo.toml` on `main` makes CI (`.github/workflows/release.yml`) build, sign,
  notarize and publish; `/make-new-release` does it.

## Architecture

- **Backends.** `src/backend.rs` is the seam: `Mux` (terminals and their layout, the one trait a new backend
  writes), `Terminal` (a pane's attached stream of bytes, fed to its emulator), `Worktrees`, `Agents`. herdr
  (`src/herdr`, JSON over its unix socket) has its own worktrees and agents, plus what only it has (plugins,
  popups, SSH machines), reached through `Mux::herdr`. `src/hc` keeps the layout itself (`hc/tree.rs`, optionally
  shared with the Herdr app); `src/rex` and `src/tmux` translate their server's sessions/windows/panes. Those three
  share `worktree::Git` and `agents::Own`.
- **Snapshot is the one truth.** `model::Snapshot` is the whole layout; `shell/sync.rs` subscribes to
  `Mux::changes` and refetches a snapshot on each change. Never apply an event onto a snapshot (herdr's events have
  no shared sequence with it).
- **UI.** `Shell` (`src/shell.rs` + `src/shell/`) is the window: sidebar, tab bar, panes, right panel (diffs,
  review, browser, device), popups. `src/ui` holds the reusable views (command panel, settings, status bar).
  `src/terminal` is a pane: libghostty-vt wrapped as a state machine (`emulator.rs`) and Shepherd's own glyph
  renderer (`element.rs`, `sprite/`).
- **gpui** is zeronsh/zui patched to the `freethinkel/zui` fork (`[patch]` in `Cargo.toml` says what the fork adds);
  a gpui change goes to that fork. Much UI code is ported from Zeron (MIT, `assets/NOTICE`).
- `shepherd hooks …` (`main.rs`) is the CLI behind the worktree hooks plugin Shepherd installs into herdr
  (`assets/herdr-plugin`).
- `SHEPHERD_*` env vars (`grep -rhoE 'SHEPHERD_[A-Z_]+' src`) override binaries (`SHEPHERD_HERDR`, `_HC`, `_REX`,
  `_TMUX`), the settings file, and open panels for screenshots.

Comments explain why, in full sentences; `ponytail:` marks a deliberate simplification and its ceiling.

## Pull requests

Before opening a PR, and in its description:

- **Screenshots, before and after.** Capture the app on `main` (before) and on the branch (after), showing what the
  change touches, and attach both to the PR. `docs/mock/shot.sh <out.png>` shoots the window from the mock fixture on
  macOS (`make linux-shot` on Linux, below), and
  `SHEPHERD_PANEL=commands|agents|workspaces|sessions|browser|diffs|usage|review[:summary|timeline|code|checks|menu]` opens a
  panel for the shot (`browser`, a blank browser tab in the right panel; `diffs`, the Diffs tab; `usage`, the limits popup;
  `device:ios:<udid>`, a device's tab, `:tools` and `:tools:overlay` after it its Tools panel and accessibility overlay,
  `:text-size` the rail's Text size menu). If a change has
  nothing to see (an internal refactor, a backend fix), say so instead.
  GitHub's API can't attach images: push them to the orphan branch `pr-assets` under `pr-<number>/` and link them as
  `https://raw.githubusercontent.com/freethinkel/shepherd/pr-assets/pr-<number>/<name>.png`.
- **What changed and why.** Explain in plain words what the user sees or can do now and what was wrong or missing
  before, then point to the main places in the code. Show it: the screenshots, plus a short clip or the steps to try
  it when one frame can't show the behavior (a swipe, a shortcut, an animation).

## Linux

No Rust toolchain is needed on the host, only Docker (the user in the `docker` group). Each target builds the images
first (`scripts/linux.Dockerfile`, then `scripts/dev.Dockerfile` on it), which takes a while once and is cached after:

- `make linux-test`, `make linux-lint`: `cargo test` and clippy with warnings as errors, as CI runs them.
- `make linux-shot [PANEL=sessions] [OUT=target/before.png]`: a debug build of this checkout on the mock fixture in
  Xvfb; the window goes to `OUT` (default `target/shot-linux.png`). Unset `PANEL` shows the plain window.

For the before shot, run it from a checkout of `main`, e.g. `git worktree add ../shepherd-main main`, then
`make -C ../shepherd-main linux-shot OUT=target/before.png`. Builds share the `shepherd-target` volume, so the
second one is quick.

If a shot comes out black, check `docs/mock/shot-linux.sh` still starts Xvfb with `-extension Composite` and
resizes the window after launch: GPUI draws nothing to a transparent window there, and starts drawing only on an
X event.
