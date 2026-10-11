# Shepherd's local build. `make help` lists the targets.
VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH    := $(shell uname -m)
APP     := target/release/Shepherd.app
ZIP     := target/release/Shepherd-$(VERSION)-$(ARCH).zip

# The keychain's Developer ID Application identity signs release apps (empty: ad hoc). Notarization uses the
# notarytool profile NOTARY_PROFILE, made once with `xcrun notarytool store-credentials shepherd-notary`,
# looked up in NOTARY_KEYCHAIN when set (CI's temporary keychain).
SIGN_IDENTITY  ?= $(shell security find-identity -v -p codesigning 2>/dev/null | sed -n 's/.*"\(Developer ID Application: .*\)"/\1/p' | head -1)
NOTARY_PROFILE ?= shepherd-notary
NOTARY_KEYCHAIN ?=
export SIGN_IDENTITY

.PHONY: help build run test app app-debug open install zip notarize version clean

help: ## This list
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  make %-10s %s\n", $$1, $$2}'

build: ## Debug binary
	cargo build

run: ## Debug app, run from target/debug/Shepherd.app (cargo runner)
	cargo run

test: ## All tests
	cargo test

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

zip: app ## Zip the release app for distribution
	ditto -c -k --keepParent $(APP) $(ZIP)
	@echo $(ZIP)

notarize: zip ## Notarize the zip with Apple, staple the ticket, re-zip
	xcrun notarytool submit $(ZIP) --keychain-profile $(NOTARY_PROFILE) $(if $(NOTARY_KEYCHAIN),--keychain $(NOTARY_KEYCHAIN)) --wait
	xcrun stapler staple $(APP)
	rm $(ZIP)
	ditto -c -k --keepParent $(APP) $(ZIP)
	spctl --assess --type execute -v $(APP)
	@echo $(ZIP)

version: ## Cargo.toml's version
	@echo $(VERSION)

clean: ## cargo clean
	cargo clean
