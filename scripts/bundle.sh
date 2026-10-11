#!/bin/sh
# Wraps the built binary in Shepherd.app: macOS delivers notifications (and their clicks) only to bundled apps.
# Usage: scripts/bundle.sh [release|debug|<built binary>]; prints the app's path. Open it with `open <path>`.
# A binary's path wraps that binary without building (scripts/run-bundled.sh does).
set -e
case $1 in
*/*) bin=$(cd "$(dirname "$1")" && pwd)/$(basename "$1") ;;
esac
cd "$(dirname "$0")/.."
if [ -z "$bin" ]; then
	profile=${1:-release}
	if [ "$profile" = release ]; then cargo build --release; else cargo build; fi
	bin="$PWD/target/$profile/shepherd"
fi
app="$(dirname "$bin")/Shepherd.app"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp assets/app-icon/Shepherd.icns "$app/Contents/Resources/Shepherd.icns"
cp "$bin" "$app/Contents/MacOS/shepherd"
cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleIdentifier</key><string>dev.freethinkel.shepherd</string>
	<key>CFBundleName</key><string>Shepherd</string>
	<key>CFBundleExecutable</key><string>shepherd</string>
	<key>CFBundleIconFile</key><string>Shepherd</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleVersion</key><string>$version</string>
	<key>CFBundleShortVersionString</key><string>$version</string>
	<key>LSMinimumSystemVersion</key><string>14.0</string>
	<key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF
# SIGN_IDENTITY ("Developer ID Application: …", the Makefile finds it) signs for distribution: hardened runtime
# and a secure timestamp, as notarization requires. Unset, the app is signed ad hoc and runs only here.
if [ -n "$SIGN_IDENTITY" ]; then
	codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "$app"
else
	codesign --force --sign - "$app" >/dev/null 2>&1
fi
echo "$app"
