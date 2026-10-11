# Shepherd's local build. `make help` lists the targets.
VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH    := $(shell uname -m)
APP     := target/release/Shepherd.app
ZIP     := target/release/Shepherd-$(VERSION)-$(ARCH).zip

.PHONY: help build run test app app-debug open install zip version clean

help: ## This list
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  make %-10s %s\n", $$1, $$2}'

build: ## Debug binary
	cargo build

run: ## Debug binary, run in place (notifications go through osascript)
	cargo run

test: ## All tests
	cargo test

app: ## Release Shepherd.app in target/release
	scripts/bundle.sh release

app-debug: ## Debug Shepherd.app in target/debug
	scripts/bundle.sh debug

open: app ## Build the release app and open it
	-pkill -f 'Shepherd.app/Contents/MacOS/shepherd'
	open $(APP)

install: app ## Copy the release app to /Applications
	-pkill -f 'Shepherd.app/Contents/MacOS/shepherd'
	rm -rf /Applications/Shepherd.app
	ditto $(APP) /Applications/Shepherd.app
	@echo /Applications/Shepherd.app

zip: app ## Zip the release app for distribution
	ditto -c -k --keepParent $(APP) $(ZIP)
	@echo $(ZIP)

version: ## Cargo.toml's version
	@echo $(VERSION)

clean: ## cargo clean
	cargo clean
