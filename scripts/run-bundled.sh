#!/bin/sh
# cargo's runner (.cargo/config.toml): `cargo run` starts the app from inside Shepherd.app, so its notifications come
# from Shepherd, with its icon, and a click brings the pane up. Tests and other binaries run as they are.
set -e
bin=$1
shift
[ "$(basename "$bin")" = shepherd ] || exec "$bin" "$@"
app=$("$(dirname "$0")/bundle.sh" "$bin")
exec "$app/Contents/MacOS/shepherd" "$@"
