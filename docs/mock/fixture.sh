# Sourced by shot.sh and shot-linux.sh, with $base set: the fixture's repos under $base (made on first run; git runs
# for real there, so branches are real) and its herdr folder.
repo() { # path, origin name
  [ -d "$1/.git" ] && return
  mkdir -p "$1"
  git -C "$1" init -q -b main
  git -C "$1" -c user.name=mock -c user.email=mock@example.com commit -q --allow-empty -m "init"
  git -C "$1" remote add origin "https://github.com/freethinkel/$2.git"
}
worktree() { # repo path, worktree path, branch
  [ -d "$2" ] && return
  mkdir -p "$(dirname "$2")"
  git -C "$1" worktree add -q -b "$3" "$2"
}
dev=$base/Developer
repo "$dev/pet/shepherd" shepherd
repo "$dev/pet/collie" collie
repo "$dev/pet/herdr" herdr
mkdir -p "$dev/infra/kuzyo"
worktree "$dev/pet/shepherd" "$dev/worktrees/shepherd/sidebar-polish" sidebar-polish
worktree "$dev/pet/shepherd" "$dev/worktrees/shepherd/review-panel" review-panel
worktree "$dev/pet/collie" "$dev/worktrees/collie/voice-local-models" voice-local-models
# no workspace has it open: the sidebar shows it with All worktrees on
worktree "$dev/pet/shepherd" "$dev/worktrees/shepherd/usage-rings" usage-rings

# herdr's folder ($XDG_CONFIG_HOME/herdr) is the fixture's too: the socket there reads as the default session, and
# nothing live is on it
mkdir -p "$base/xdg/herdr" && : > "$base/xdg/herdr/config.toml"

# files for the right panel's Files tab, committed on the review-panel branch (the focused pane's): its working tree,
# and so its diff, stays as it was; git's marks come from what's changed there
seed() { # worktree
  [ -f "$1/Cargo.toml" ] && return
  mkdir -p "$1/src/shell" "$1/docs" "$1/target/debug"
  printf '[package]\nname = "shepherd"\nversion = "1.7.0"\nedition = "2024"\n\n[dependencies]\ngpui = "0.2"\nserde = "1"\n' > "$1/Cargo.toml"
  printf 'target/\n' > "$1/.gitignore"
  printf '# Shepherd\n\nA native window for **herdr**: workspaces, tabs, panes and every coding agent at a glance.\n\n## What it does\n\n- Lists every agent with its status\n- Notifies you when one needs an answer\n- Shows diffs, pull requests and files beside the panes\n\n```sh\ncargo run\n```\n' > "$1/README.md"
  printf 'use shepherd::shell::Shell;\n\nfn main() {\n    // the window, and the herdr session behind it\n    let app = gpui::Application::new();\n    app.run(|cx| {\n        Shell::open(cx);\n    });\n}\n' > "$1/src/main.rs"
  printf 'pub mod files;\npub mod shell;\n' > "$1/src/lib.rs"
  printf 'pub mod right_panel;\n\npub struct Shell {\n    width: f32,\n}\n' > "$1/src/shell/mod.rs"
  printf '/// The right panel: a column of tabs beside the panes.\npub struct RightPanel {\n    pub open: bool,\n    pub width: f32,\n}\n\nimpl RightPanel {\n    pub fn toggle(&mut self) {\n        self.open = !self.open;\n    }\n}\n' > "$1/src/shell/right_panel.rs"
  cp docs/icon.png "$1/docs/icon.png"
  : > "$1/target/debug/shepherd"
  git -C "$1" add -- Cargo.toml .gitignore README.md src docs
  git -C "$1" -c user.name=mock -c user.email=mock@example.com commit -q -m "files" -- Cargo.toml .gitignore README.md src docs
}
seed "$dev/worktrees/shepherd/review-panel"
