#!/bin/sh
# shot.sh on Linux, in the shepherd-dev container (`make linux-shot`): a debug Shepherd on the fixture in Xvfb, its
# window saved to $1 (relative to the checkout), then it quits. SHEPHERD_PANEL opens a panel for the shot; HOST_UID
# and HOST_GID own the file.
set -e
cd "$(dirname "$0")/../.."
mock=$PWD/docs/mock
base=$HOME/.cache/shepherd-mock
out=${1:-target/shot-linux.png}

. "$mock/fixture.sh"
# no saved display here: the frame in settings.json is not applied, and the window is sized below
sed -e "s|\$HOME|$HOME|g" -e "s|\$DISPLAY||" "$mock/settings.json" > "$base/settings.json"

cargo build -q

# Without Composite, Xvfb offers no 32-bit visual, so GPUI's window is opaque; a transparent one's frames never reach
# the screen there and the shot comes out black.
Xvfb :99 -screen 0 1600x1000x24 -nolisten tcp -extension Composite >/dev/null 2>&1 &
export DISPLAY=:99
export XDG_RUNTIME_DIR=$(mktemp -d)
sleep 1
SHEPHERD_MOCK=$mock SHEPHERD_SETTINGS_FILE=$base/settings.json SHEPHERD_HERDR=$mock/bin/herdr \
  XDG_CONFIG_HOME=$base/xdg HERDR_SOCKET_PATH=$base/xdg/herdr/herdr.sock \
  "${CARGO_TARGET_DIR:-target}/debug/shepherd" >/tmp/shepherd.log 2>&1 &
pid=$!
trap 'kill $pid 2>/dev/null' EXIT
sleep "${SHOT_WAIT:-6}"
kill -0 $pid 2>/dev/null || { echo "Shepherd quit:" >&2; tail -20 /tmp/shepherd.log >&2; exit 1; }

win=$(xdotool search --name '^Shepherd$' | head -1)
[ -n "$win" ] || { echo "no window of the fixture's Shepherd" >&2; exit 1; }
# GPUI starts drawing on an X event the bare server never sends by itself: a resize is one, at shot.sh's size
xdotool windowsize "$win" 1360 850
sleep 3
[ -d "$(dirname "$out")" ] || { mkdir -p "$(dirname "$out")" && chown "${HOST_UID:-0}:${HOST_GID:-0}" "$(dirname "$out")"; }
import -window "$win" "$out"
chown "${HOST_UID:-0}:${HOST_GID:-0}" "$out"
echo "$out"
