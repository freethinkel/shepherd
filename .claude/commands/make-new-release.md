---
description: Bump Shepherd's version and push it to main, so CI tags and publishes the release
argument-hint: "[patch|minor|major|X.Y.Z]"
---

Release a new Shepherd version. Pushing a new `version` in `Cargo.toml` to `main` is the whole release:
`.github/workflows/release.yml` then builds, tags `v<version>` and publishes the zip and dmg, with the commits since
the last tag as the notes (`docs:` and `release:` commits left out).

Argument: `$ARGUMENTS`. Empty means `patch`; `minor` and `major` bump that part; `X.Y.Z` is the exact version.

1. Check the state; stop and say why if any of these fail:
   - the branch is `main` and `git fetch origin` shows it is not behind `origin/main`;
   - `Cargo.toml` and `Cargo.lock` have no uncommitted changes (the bump commit must hold only the version);
   - the new version is greater than the current one (`make -s version`) and `v<new>` is not already a tag.
   Other uncommitted files are fine: leave them alone, do not stash or commit them.
2. Show the notes the release will get: `git log --no-merges --format='- %s' $(git describe --tags --abbrev=0)..HEAD | grep -vE '^- (docs|release):'`.
   If the list is empty, stop: there is nothing to release.
3. Set `version = "<new>"` in `Cargo.toml`, then `cargo update --workspace --offline` to update `Cargo.lock`.
4. `make test`. If it fails, revert the two files and stop with the output.
5. Commit only `Cargo.toml` and `Cargo.lock` as `release: <new>`, then `git push origin main`.
6. Find the run with `gh run list --workflow release.yml --limit 1` and `gh run watch <id> --exit-status` in the
   background. When it ends, report the release URL (`gh release view v<new> --json url -q .url`), or the failing step's log.
