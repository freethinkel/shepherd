# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`README.md` is the user-facing manual (config keys, CLI commands, plugin format). Read it first —
this file only covers what the README does not.

## Commands

Package manager and runner is **bun** (not npm/pnpm). It executes `.ts` directly — there is no build
step for development. `@opentui/core` is native and needs Bun >= 1.3: that is what pins the runtime.

```sh
bun install
bun run typecheck                        # tsc --noEmit
bun test                                 # node:test over tests/**/*.test.ts, run by bun
bun test tests/policies.test.ts          # one file
bun test -t "review falls back"          # one test
bun run fmt                              # oxfmt
bun run lint                             # oxlint
bun run start -- status                  # run the CLI from source
bun run start -- ui                      # the TUI from source
bun run install:bin                      # compile to ~/.local/bin/shepherd + restart the daemon
```

`install:bin` is the only way to ship a change to a running daemon: the daemon holds config and code
in memory, so editing `config.yaml` or `src/` does nothing until it is rebuilt and kicked.

## Imports and types

Imports carry the `.ts` extension (`allowImportingTsExtensions`). `verbatimModuleSyntax` means type
imports must be written as `import type`. `exactOptionalPropertyTypes` is on, so optional fields are
declared `foo?: T | undefined` — dropping the explicit `| undefined` breaks the build.

## Architecture

Layering rule: `shared/` and `core/` know nothing about Linear, GitHub, GitLab or Herdr. Vendor
knowledge lives only in `modules/providers/` and `modules/herdr/`. Keep it that way when adding a
tracker or forge.

`core/` is the wiring and the state everything stands on, `modules/` are the parts that could be
swapped out, `shared/` is what more than one module needs. A module owns its own `ui/`, `helpers/`,
`constants/` and `types/` when it grows them — `modules/theme` and `modules/tui` do.

```
core/app.ts              wires config → db → registry → workflow → scheduler; every entry point uses it
core/config/schema.ts    zod + YAML, also the source of EXAMPLE_CONFIG shown by `shepherd init`
core/persistence/db.ts   bun:sqlite, the source of truth for orchestration
modules/orchestrator/    scheduler (when) · workflow (how, one method per run transition) · policies (pure rules)
modules/providers/       registry (which provider) · load (plugin files) · tasks/* · code/*
modules/herdr/client.ts  execFile over the `herdr` CLI, JSON in/out
modules/theme/           terminal palette → colour roles; the UI never names a hex
modules/tui/             the dashboard: solid components over shared/view + shared/actions
modules/cli/             commands, daemon, plain-text rendering
shared/domain/           types + status derivation (pure, no I/O)
shared/view.ts           one state view for the CLI and the TUI
shared/actions.ts        retry/stop/open/review/reset — what a human asks for, CLI and TUI share it
shared/components/       Text · List · Scroll, all painted from the palette
shared/git.ts            branch names, commit counting, push, validation command
```

### The two loops

`Scheduler.loop` ticks every `poll_interval_ms`: re-sync tasks when due → `workflow.advance(run)` for
every active run → `dispatch()` starts new runs up to `max_concurrent_runs`. Everything else (CLI
commands, daemon) just calls into the same scheduler/workflow.

`Workflow` is a state machine driven by observation, never by parsing agent output. `advance()`
switches on `run.status` and does one transition per call; `transition()` writes SQLite, appends an
event, and pushes the derived status back to the tracker. Agent state comes verbatim from
`herdr agent get` and is only normalized in `domain/status.ts`.

### Invariants worth knowing before touching orchestration

- **One run per task** is enforced by a partial unique index (`one_active_run_per_task`), not by
  application code. Same for one change per run (`changes.run_id UNIQUE`).
- **Events are state.** `ValidationRejected` is the retry counter for `max_validation_rounds`.
  Adding a "did we already do X" flag means appending an event, not adding a column.
- **Todo during `review` means rework.** `checkChange` reads the synced task status (our own
  `fail()` also parks tasks in Todo, but by then the run is `failed`, so that branch cannot fire).
  `ReviewRejected` is the round counter for `max_review_rounds`, appended only after the prompt
  reached the agent, and the newest of `ReviewRejected`/`ChangeCreated` is the cutoff for which
  comments are sent — round two must not re-fix round one. `ensureReviewAgent` allows one pass per
  round by comparing per-run counts (`ReviewAgentStarted` vs `ReviewRejected`), never timestamps
  or task-scoped events: a retried run must get its own reviewer.
- **An open change on the branch is resumed, not redone.** `start()` asks the forge for a change on
  the task's branch (`findChange`); an open one is recorded on the new run (`ChangeResumed`) and
  its review comments go into the first prompt. That is how a task sent back to Todo is picked up
  on a fresh database, or after `max_attempts` and `shepherd retry`, without opening a second PR.
- **The orchestrator is the only thing that pushes.** `createChange` pushes before its
  existing-change short-circuit because a rework round comes back through it with the change already
  recorded, and the dev agent is told never to push. The push is `--force-with-lease`: rework rounds
  may rebase or amend, a plain push would reject and fail the run.
- **The change row follows the live run.** `recordChange` is an upsert on `(provider, id)`: a
  retried run reopens the same change, and a row still pointing at the dead run makes `checkChange`
  find nothing and loop `review ⇄ creating_change` every tick, pushing and commenting each time.
- **Acceptance is a human's word, from either side.** An approval on the forge or the task moved to
  Done in the tracker (`TaskAccepted`, read live — the sync only lists Todo) both mean yes; GitHub
  refuses an approval from the author, so on a solo repository the tracker is the only channel.
- **Merge is executed, never decided.** `approved && checks === "success"` from `getChange` calls
  `mergeChange`. `approved` means a _person_ approved: on GitHub `reviewDecision` is empty without a
  required-review rule, so `latestReviews` decide; on GitLab `approvals.approved` is `true` with zero
  approval rules (the free-tier default), so `approved_by` must be non-empty. `checks` is `success`
  when there is no CI at all; SKIPPED/NEUTRAL do not block a GitHub merge either. A `ChangeMerged`
  event short-circuits later attempts (a forge reports `open` for a while after a merge — queue,
  API lag — and a second press fails with a false "merge failed" comment). `MergeFailed` is capped at
  three, the run stays in `review` for a human. A failing `/approvals` read degrades to
  "not approved" rather than failing the run.
- **Idle needs two consecutive polls** before a run moves to `validating` — a single idle tick is a
  pause, not completion.
- **A run in `review` never times out**; every other status is killed after `run_timeout_ms`.
- **Failure is not terminal for a task.** `fail()` puts the task back in the tracker's Todo column
  and comments why; `isTaskAvailable` re-queues it until `max_attempts` runs have failed.
- **A miss in SQLite is `null`, not `undefined`.** `bun:sqlite` answers an empty `.get()` with
  `null`, so `hasEvent` compares against `null`; the `!== undefined` it used to use reported every
  event as already present, which silently disables every "did we already do X" guard above.
- Provider dispatch: a task remembers which tracker it came from (`tasks.provider`) so updates go
  back to the same one; a code provider is picked from the repository's git remote, never configured
  per project.
- Errors from CLI tools go through `briefError()` before reaching the log — `gh`/`glab`/`jira` answer
  failures with pages of help text and this loop runs every few seconds.

### The TUI

Solid + JSX over `@opentui/solid`, the same shape as the `glui` project. The JSX transform is
registered by `@opentui/solid/preload`: `bun run start` passes it with `--preload`, and `build.ts`
registers the same plugin for the compiled binary. A `bunfig.toml` would do it too, but the
compiled binary reads the one in the current directory and dies on a preload it does not contain.

Colour is never a hex. `modules/theme` asks the terminal for its real palette (`getPalette`) and
maps it to roles — `fg`, `muted`, `border`, `selectionBg`, `accent`, `danger`, `success`,
`warning` — pushing every role that carries text to a legible contrast ratio. Without an answer it
falls back to ANSI indices, which the terminal resolves itself. opentui's own default is white,
which is invisible on a light background: that is the bug this exists to prevent.

The layout is adaptive: under `WIDE_COLS` (100) the two columns become one, and the run detail
moves to a screen of its own behind `enter`. `useTerminalDimensions` drives it, so resizing the
window switches layout without a restart.

`overview()` takes the configured project ids: a project dropped from the config keeps its rows —
`runs` and `events` are history — but disappears from the dashboard and from `status`.

Two rules the dashboard was rewritten to obey:

- **A frame never waits on I/O.** `refresh()` reads SQLite and nothing else; the agent log arrives
  through its own effect. Drawing behind `herdr read` showed empty panes and swallowed ctrl-c.
- **Quitting hands stdin back first.** `renderer.destroy()` blocks forever while stdin is in raw
  mode, so `helpers/quit.ts` clears raw mode, then destroys, then exits — a `herdr read` still in
  flight would otherwise keep the terminal hostage.

### Adding a provider

Prefer a plugin file in `~/.config/shepherd/providers/` (see README) — the core needs no changes.
A built-in goes into `providers/{tasks,code}/` plus one line in the `builtinTasks`/`builtinCode` map
in `registry.ts`. A code provider only has to implement `CodeProvider` from `domain/types.ts`; the
optional `check()` is a preflight so an agent does not work for an hour and then fail to open a PR.

## Tests

`node:test` + `node:assert`, run by `bun test`, no framework. `tests/workflow.test.ts` runs the full lifecycle against
fake Herdr and fake providers but a **real** git repo (bare origin + clone in a tmpdir) — worktrees,
commits and pushes are genuine, nothing touches the network. New orchestration behaviour belongs
there; pure rules belong in `policies.test.ts`.

## Style

Code comments are rare and one line. `ponytail:` comments mark deliberate simplifications and name
the upgrade path (in-memory poll timestamps, schema applied on open instead of migrations, polling
instead of a herdr event stream); respect them — they are decisions, not oversights. The _why_
behind a rule — the failure it prevents — lives in the invariants list above, not in a doc block
over the function. If a change needs a paragraph of rationale, add a bullet there.
