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
