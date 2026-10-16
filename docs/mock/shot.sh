#!/bin/sh
# Shepherd's screenshot (docs/screenshot.png, the site's) from the fixture in this folder, not from a live session:
# a debug Shepherd runs with SHEPHERD_MOCK and its own settings, the window is captured, then it quits. The fixture's
# repos are made under ~/.cache/shepherd-mock on first run (git runs for real there, so branches are real).
set -e
cd "$(dirname "$0")/../.."
mock=$PWD/docs/mock
base=$HOME/.cache/shepherd-mock
out=${1:-docs/screenshot.png}

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

# the window goes on the display the real settings last saved; without one it opens centered at the default size
display=$(python3 -c 'import json,os;print(json.load(open(os.path.expanduser("~/Library/Application Support/shepherd/settings.json")))["window"]["display"])' 2>/dev/null || true)
sed -e "s|\$HOME|$HOME|g" -e "s|\$DISPLAY|$display|" "$mock/settings.json" > "$base/settings.json"

# herdr's folder ($XDG_CONFIG_HOME/herdr) is the fixture's too: the socket there reads as the default session, and
# nothing live is on it
mkdir -p "$base/xdg/herdr" && : > "$base/xdg/herdr/config.toml"

cargo build -q
SHEPHERD_MOCK=$mock SHEPHERD_SETTINGS_FILE=$base/settings.json SHEPHERD_HERDR=$mock/bin/herdr \
  XDG_CONFIG_HOME=$base/xdg HERDR_SOCKET_PATH=$base/xdg/herdr/herdr.sock \
  scripts/run-bundled.sh target/debug/shepherd -ApplePersistenceIgnoreState YES &  # the real app's saved frame stays out
pid=$!
trap 'kill $pid 2>/dev/null' EXIT
sleep "${SHOT_WAIT:-5}"

# OmniWM tiles new windows: this one is floated (it has the focus, it was just activated) and sized by hand, since
# a floating window keeps the size it is given. Without OmniWM the frame in settings.json already holds.
omni=/Applications/OmniWM.app/Contents/MacOS/omniwmctl
if [ -x "$omni" ] && "$omni" ping >/dev/null 2>&1; then
  "$omni" query focused-window --format json | grep -q "\"pid\" : $pid" && "$omni" command toggle-focused-window-floating >/dev/null
  sleep 0.5
  osascript -e "tell application \"System Events\" to tell (first process whose unix id is $pid)
    set position of window 1 to {120, 90}
    set size of window 1 to {1360, 850}
  end tell"
  sleep 1
fi

# the window of this very process, not the real Shepherd's
id=$(python3 -c "
import Quartz
for w in Quartz.CGWindowListCopyWindowInfo(Quartz.kCGWindowListOptionOnScreenOnly | Quartz.kCGWindowListExcludeDesktopElements, Quartz.kCGNullWindowID):
    if w.get('kCGWindowOwnerPID') == $pid and w.get('kCGWindowLayer') == 0: print(w['kCGWindowNumber']); break")
[ -n "$id" ] || { echo "no window of the fixture's Shepherd" >&2; exit 1; }
screencapture -l"$id" -x "$out"
echo "$out"
