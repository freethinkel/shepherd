#!/bin/sh
# Wraps the built binary in Shepherd.app: macOS delivers notifications (and their clicks) only to bundled apps.
# Usage: scripts/bundle.sh [release|debug]; prints the app's path. Open it with `open <path>`.
set -e
cd "$(dirname "$0")/.."
profile=${1:-release}
if [ "$profile" = release ]; then cargo build --release; else cargo build; fi
app="target/$profile/Shepherd.app"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
mkdir -p "$app/Contents/MacOS"
cp "target/$profile/shepherd" "$app/Contents/MacOS/shepherd"
cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleIdentifier</key><string>dev.freethinkel.shepherd</string>
	<key>CFBundleName</key><string>Shepherd</string>
	<key>CFBundleExecutable</key><string>shepherd</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleVersion</key><string>$version</string>
	<key>CFBundleShortVersionString</key><string>$version</string>
	<key>LSMinimumSystemVersion</key><string>14.0</string>
	<key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF
codesign --force --sign - "$app" >/dev/null 2>&1
echo "$app"
