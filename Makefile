# libghostty at a pinned commit (the one flutter_ghostty builds with Zig 0.15.2).
GHOSTTY_REV := efb35235919a38bb04ac8b8ef06537f0317b7045
# Zig 0.15.2 from .tools/zig when present (ziglang.org tarball), else from PATH.
ZIG ?= $(firstword $(wildcard $(CURDIR)/.tools/zig/zig) zig)
CARGO := env -u MACOSX_DEPLOYMENT_TARGET cargo
# A folder with a libghostty.dylib already built from GHOSTTY_REV with the flags below, to skip Zig.
# ponytail: Zig 0.15.2 cannot link against the macOS 27 / CLT 26.5 SDKs (their libSystem.tbd lists
# arm64e only), so on such machines point this at an older build until Ghostty moves to a newer Zig.
GHOSTTY_PREBUILT ?=

ifneq ($(GHOSTTY_PREBUILT),)
vendor/ghostty/libghostty.dylib:
	test -d .ghostty-src || git clone https://github.com/ghostty-org/ghostty .ghostty-src
	cd .ghostty-src && git fetch -q origin $(GHOSTTY_REV) && git checkout -q $(GHOSTTY_REV)
	mkdir -p vendor/ghostty
	cp $(GHOSTTY_PREBUILT)/libghostty.dylib .ghostty-src/include/ghostty.h vendor/ghostty/
	install_name_tool -id @rpath/libghostty.dylib vendor/ghostty/libghostty.dylib
	codesign -f -s - vendor/ghostty/libghostty.dylib
else
vendor/ghostty/libghostty.dylib:
	@$(ZIG) version | grep -qx 0.15.2 || { echo "need zig 0.15.2"; exit 1; }
	test -d .ghostty-src || git clone https://github.com/ghostty-org/ghostty .ghostty-src
	cd .ghostty-src && git fetch -q origin $(GHOSTTY_REV) && git checkout -q $(GHOSTTY_REV)
	cd .ghostty-src && $(ZIG) build -Dapp-runtime=none -Demit-xcframework=false -Demit-macos-app=false -Doptimize=ReleaseFast
	mkdir -p vendor/ghostty
	cp .ghostty-src/zig-out/lib/libghostty.dylib .ghostty-src/include/ghostty.h vendor/ghostty/
	install_name_tool -id @rpath/libghostty.dylib vendor/ghostty/libghostty.dylib
	codesign -f -s - vendor/ghostty/libghostty.dylib
endif

ghostty: vendor/ghostty/libghostty.dylib

build: ghostty
	$(CARGO) build

test: ghostty
	$(CARGO) test

run: ghostty
	$(CARGO) run

.PHONY: ghostty build test run
