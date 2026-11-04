# Shepherd

## Pull requests

Before opening a PR, and in its description:

- **Screenshots, before and after.** Capture the app on `main` (before) and on the branch (after), showing what the
  change touches, and attach both to the PR. `docs/mock/shot.sh <out.png>` shoots the window from the mock fixture, and
  `SHEPHERD_PANEL=commands|agents|workspaces|sessions` opens a panel for the shot. If a change has nothing to see
  (an internal refactor, a backend fix), say so instead.
- **What changed and why.** Explain in plain words what the user sees or can do now and what was wrong or missing
  before, then point to the main places in the code. Show it: the screenshots, plus a short clip or the steps to try
  it when one frame can't show the behavior (a swipe, a shortcut, an animation).
