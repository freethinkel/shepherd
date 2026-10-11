<h1 align="center"><img src="docs/icon.png" width="48" height="48" align="absmiddle" alt=""> Shepherd</h1>

A native macOS window for [herdr](https://herdr.dev). herdr runs your shells and coding agents in workspaces, tabs and panes. Shepherd shows the same session with a sidebar, real tabs and a list of every agent, so you can see which one is waiting on you without cycling through panes.

It's written in Rust on [GPUI](https://github.com/zeronsh/zui) and styled after [Zeron](https://github.com/zeronsh/zeron). The terminal is alacritty_terminal with its own glyph renderer.

![Shepherd with a split tab, the workspace sidebar and four agents](docs/screenshot.png)

## What it does

- The sidebar lists workspaces. A repo's worktrees fold under it, and right-click creates or removes a worktree with the branch name you type.
- The agents list at the bottom shows every agent herdr detects in any workspace, with its status (working, blocked, done, idle) and the task in its title. Click one to jump to its pane.
- When an agent you can't see starts waiting or finishes, macOS shows a notification and the Dock icon counts it until you look at that tab.
- Tabs show the agent or shell running in them. Drag tabs to reorder them. Drag a pane by its grip onto another pane to split or swap, or onto a tab to move it there.
- Dividers between panes snap to the cell grid herdr lays out, and the terminal reflows while you drag.
- `⌘K` opens a command panel with every action, workspace and tab. `⇧⌘A` opens the agents panel.
- The status bar shows the branch, the current folder, the focused agent and your Claude usage.
- Settings cover fonts, themes and key bindings. They also edit herdr's own `config.toml` for notifications and integrations, keeping your comments.

## Install

Download `Shepherd-<version>-universal.dmg` from [Releases](https://github.com/freethinkel/shepherd/releases), open it and drag Shepherd to Applications. One build runs on Apple silicon and Intel Macs. It's signed and notarized, so Gatekeeper opens it without a prompt. The same app is also there as a zip.

Shepherd needs macOS 14 or later and herdr. It talks to the herdr server over `~/.config/herdr/herdr.sock`, so herdr has to be installed and running. You can point it at another binary or socket in Settings.

## Keys

| Keys | Action |
|---|---|
| `⌘T`, `⇧⌘W` | New tab, close tab |
| `⌘D`, `⇧⌘D` | Split right, split down |
| `⌘W` | Close pane |
| `⌥⌘` + arrows | Focus the pane in that direction |
| `⇧⌘↩` | Zoom the pane |
| `⌘1` … `⌘9`, `⇧⌘[`, `⇧⌘]` | Go to a tab, previous or next tab |
| `⌘↑`, `⌘↓` | Previous or next workspace |
| `⇧⌘N` | New workspace |
| `⌘B` | Toggle the sidebar |
| `⌘K`, `⇧⌘A` | Command panel, agents panel |
| `⌘,` | Settings |

Every binding can be changed in Settings.

## Build

You need a recent stable Rust toolchain and Xcode's command line tools.

```sh
make run      # debug build, run as Shepherd.app
make test
make app      # target/release/Shepherd.app
make install  # copy it to /Applications
make dist     # universal zip and dmg in target/universal
make help     # everything else
```

`make app` signs with the Developer ID certificate in your keychain if it finds one, and ad hoc otherwise. Releases come from CI: bumping `version` in `Cargo.toml` on `main` builds, signs, notarizes and publishes a new release.

## License

MIT, see [LICENSE](LICENSE). Code and assets taken from Zeron (MIT) and the Geist fonts (SIL OFL 1.1) are listed in [assets/NOTICE](assets/NOTICE).
