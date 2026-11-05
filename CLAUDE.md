# Shepherd

## Pull requests

Before opening a PR, and in its description:

- **Screenshots, before and after.** Capture the app on `main` (before) and on the branch (after), showing what the
  change touches, and attach both to the PR. `docs/mock/shot.sh <out.png>` shoots the window from the mock fixture on
  macOS (`make linux-shot` on Linux, below), and `SHEPHERD_PANEL=commands|agents|workspaces|sessions|browser|usage` opens a
  panel for the shot (`browser`, a blank browser tab in the right panel; `usage`, the limits popup). If a change has nothing to see (an internal refactor, a backend fix), say so instead.
  GitHub's API can't attach images: push them to the orphan branch `pr-assets` under `pr-<number>/` and link them as
  `https://raw.githubusercontent.com/freethinkel/shepherd/pr-assets/pr-<number>/<name>.png`.
- **What changed and why.** Explain in plain words what the user sees or can do now and what was wrong or missing
  before, then point to the main places in the code. Show it: the screenshots, plus a short clip or the steps to try
  it when one frame can't show the behavior (a swipe, a shortcut, an animation).

## Linux

No Rust toolchain is needed on the host, only Docker (the user in the `docker` group). Each target builds the images
first (`scripts/linux.Dockerfile`, then `scripts/dev.Dockerfile` on it), which takes a while once and is cached after:

- `make linux-test`, `make linux-lint`: `cargo test` and clippy with warnings as errors, as CI runs them.
- `make linux-shot [PANEL=sessions] [OUT=target/before.png]`: a debug build of this checkout on the mock fixture in
  Xvfb; the window goes to `OUT` (default `target/shot-linux.png`). Unset `PANEL` shows the plain window.

For the before shot, run it from a checkout of `main`, e.g. `git worktree add ../shepherd-main main`, then
`make -C ../shepherd-main linux-shot OUT=target/before.png`. Builds share the `shepherd-target` volume, so the
second one is quick.

If a shot comes out black, check `docs/mock/shot-linux.sh` still starts Xvfb with `-extension Composite` and
resizes the window after launch: GPUI draws nothing to a transparent window there, and starts drawing only on an
X event.
