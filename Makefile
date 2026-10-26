# Shepherd's local build. `make help` lists the targets.
VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
APP     := target/release/Shepherd.app
# Releases ship one universal (arm64 + x86_64) app, as a zip and a dmg.
DIST    := target/universal
DIST_APP := $(DIST)/Shepherd.app
ZIP     := $(DIST)/Shepherd-$(VERSION)-universal.zip
DMG     := $(DIST)/Shepherd-$(VERSION)-universal.dmg
NOTARY   = xcrun notarytool submit --keychain-profile $(NOTARY_PROFILE) $(if $(NOTARY_KEYCHAIN),--keychain $(NOTARY_KEYCHAIN)) --wait

# The keychain's Developer ID Application identity signs release apps (empty: ad hoc). Notarization uses the
# notarytool profile NOTARY_PROFILE, made once with `xcrun notarytool store-credentials shepherd-notary`,
# looked up in NOTARY_KEYCHAIN when set (CI's temporary keychain).
SIGN_IDENTITY  ?= $(shell security find-identity -v -p codesigning 2>/dev/null | sed -n 's/.*"\(Developer ID Application: .*\)"/\1/p' | head -1)
NOTARY_PROFILE ?= shepherd-notary
NOTARY_KEYCHAIN ?=
export SIGN_IDENTITY

.PHONY: help build run test lint linux app app-debug open install universal dist notarize version ghostty-themes deploy-site clean

help: ## This list
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  make %-10s %s\n", $$1, $$2}'

build: ## Debug binary
	cargo build

run: ## Debug app, run from target/debug/Shepherd.app (cargo runner)
	cargo run

test: ## All tests
	cargo test

lint: ## Clippy, warnings as errors
	cargo clippy --all-targets -- -D warnings

linux: ## Linux tar.gz, .deb and .rpm for this Mac's architecture, built in Docker, in target/linux-dist
	docker build -t shepherd-linux -f scripts/linux.Dockerfile scripts
	# the build goes to a volume: cargo-deb's temp folder can't be cleared on the Mac's bind-mounted target
	docker run --rm -v "$(CURDIR)":/src -v shepherd-cargo:/root/.cargo/registry -v shepherd-cargo-git:/root/.cargo/git \
		-v shepherd-target:/target -e CARGO_TARGET_DIR=/target -w /src shepherd-linux scripts/linux-dist.sh

app: ## Release Shepherd.app in target/release
	scripts/bundle.sh release

app-debug: ## Debug Shepherd.app in target/debug, signed ad hoc
	SIGN_IDENTITY= scripts/bundle.sh debug

open: app ## Build the release app and open it
	-pkill -f '[S]hepherd.app/Contents/MacOS/shepherd'
	open $(APP)

install: app ## Copy the release app to /Applications
	-pkill -f '[S]hepherd.app/Contents/MacOS/shepherd'
	rm -rf /Applications/Shepherd.app
	ditto $(APP) /Applications/Shepherd.app
	@echo /Applications/Shepherd.app

universal: ## Universal (arm64 + x86_64) Shepherd.app in target/universal
	cargo build --release --target aarch64-apple-darwin
	cargo build --release --target x86_64-apple-darwin
	mkdir -p $(DIST)
	lipo -create -output $(DIST)/shepherd \
		target/aarch64-apple-darwin/release/shepherd target/x86_64-apple-darwin/release/shepherd
	scripts/bundle.sh $(DIST)/shepherd

# The dmg: the app next to an Applications link, compressed, signed when an identity is around.
define make-dmg
	rm -rf $(DIST)/dmg $(DMG)
	mkdir $(DIST)/dmg
	ditto $(DIST_APP) $(DIST)/dmg/Shepherd.app
	ln -s /Applications $(DIST)/dmg/Applications
	hdiutil create -quiet -volname Shepherd -srcfolder $(DIST)/dmg -fs HFS+ -format UDZO $(DMG)
	rm -rf $(DIST)/dmg
	$(if $(SIGN_IDENTITY),codesign --force --timestamp --sign "$(SIGN_IDENTITY)" $(DMG))
endef

dist: universal ## Universal zip and dmg in target/universal
	rm -f $(ZIP)
	ditto -c -k --keepParent $(DIST_APP) $(ZIP)
	$(make-dmg)
	@echo $(ZIP) $(DMG)

notarize: dist ## dist, notarized by Apple with the tickets stapled to the app and the dmg
	$(NOTARY) $(ZIP)
	xcrun stapler staple $(DIST_APP)
	rm $(ZIP)
	ditto -c -k --keepParent $(DIST_APP) $(ZIP)
	$(make-dmg)
	$(NOTARY) $(DMG)
	xcrun stapler staple $(DMG)
	spctl --assess --type execute -v $(DIST_APP)
	spctl --assess --type open --context context:primary-signature -v $(DMG)
	@echo $(ZIP) $(DMG)

GHOSTTY_THEMES ?= /Applications/Ghostty.app/Contents/Resources/ghostty/themes
ghostty-themes: ## Re-import Ghostty's themes into assets/themes/ghostty.conf
	{ echo "# Ghostty's bundled themes (from iTerm2-Color-Schemes, MIT), one [Name] block each."; \
	  echo "# Regenerate: make ghostty-themes"; \
	  for f in "$(GHOSTTY_THEMES)"/*; do printf '\n[%s]\n' "$$(basename "$$f")"; \
	    grep -E '^(palette|background|foreground|selection-background) =' "$$f"; done; } > assets/themes/ghostty.conf

# shepherd.freethinkel.dev: the static site in site/, served by Caddy in kuzyo's pets container (infra/kuzyo pets/shepherd).
SITE_HOST ?= kuzyo
deploy-site: ## rsync site/ to shepherd.freethinkel.dev
	# ponytail: Cloudflare caches css/js/images for hours; a content hash in the query makes the fresh HTML fetch fresh files
	for f in style.css site.js assets/screenshot.webp; do sed -i '' "s|$$f?v=[0-9a-f]*|$$f?v=$$(md5 -q site/$$f | cut -c1-8)|" site/index.html; done
	# phone/ is the phone app's page, deployed by the mochi repo's make deploy-site
	rsync -rlpv --delete --exclude=/phone/ site/ $(SITE_HOST):/srv/pets/shepherd/site/

version: ## Cargo.toml's version
	@echo $(VERSION)

clean: ## cargo clean
	cargo clean
