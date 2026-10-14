#!/bin/sh
# Builds Shepherd for Linux once and packs that one binary three ways into target/linux-dist: a tar.gz laid out as a
# prefix (unpack it into ~/.local or /usr/local), a .deb and an .rpm. CI runs it on each architecture's runner;
# `make linux` runs it in Docker. Needs cargo-deb and cargo-generate-rpm.
set -eu
cd "$(dirname "$0")/.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
target=${CARGO_TARGET_DIR:-target}
out=target/linux-dist

cargo build --release
strip "$target/release/shepherd"

rm -rf "$out" && mkdir -p "$out"
stage=$(mktemp -d)
install -Dm755 "$target/release/shepherd" "$stage/bin/shepherd"
install -Dm644 assets/linux/dev.freethinkel.shepherd.desktop "$stage/share/applications/dev.freethinkel.shepherd.desktop"
install -Dm644 assets/linux/dev.freethinkel.shepherd.png "$stage/share/icons/hicolor/512x512/apps/dev.freethinkel.shepherd.png"
tar -czf "$out/Shepherd-$version-linux-$(uname -m).tar.gz" -C "$stage" .
rm -rf "$stage"

cargo deb --no-build --no-strip --output "$out/"
cargo generate-rpm --target-dir "$target" --output "$out/"
ls -l "$out"
