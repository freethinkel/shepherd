# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`README.md` is the user-facing manual (config keys, CLI commands, plugin format). Read it first —
this file only covers what the README does not.

## Commands

Package manager and runner is **nub** (not npm/pnpm). It executes `.ts` directly — there is no build
step for development.

```sh
nub install
nub run typecheck                        # tsc --noEmit
nub run test                             # node:test over tests/**/*.test.ts
nub --node --test tests/policies.test.ts # one file
nub --node --test --test-name-pattern "review falls back" tests/policies.test.ts
nub run fmt                              # oxfmt
nub run lint                             # oxlint
nub start -- status                      # run the CLI from source
nub run install:bin                      # bundle to ~/.local/bin/shepherd + restart the daemon
```

`install:bin` is the only way to ship a change to a running daemon: the daemon holds config and code
in memory, so editing `config.toml` or `src/` does nothing until it is rebuilt and kicked.

## Imports and types

Imports carry the `.ts` extension (`allowImportingTsExtensions`). `verbatimModuleSyntax` means type
imports must be written as `import type`. `exactOptionalPropertyTypes` is on, so optional fields are
declared `foo?: T | undefined` — dropping the explicit `| undefined` breaks the build.

## Architecture

Layering rule: `domain/` and `view.ts` know nothing about Linear, GitHub, GitLab or Herdr. Vendor
knowledge lives only in `providers/` and `herdr/`. Keep it that way when adding a tracker or forge.

```
config/schema.ts   zod + TOML, also the source of EXAMPLE_CONFIG shown by `shepherd init`
app.ts             wires config → db → registry → workflow → scheduler; every entry point uses it
domain/            types + status derivation (pure, no I/O)
orchestrator/      scheduler (when) · workflow (how, one method per run transition) · policies (pure rules)
providers/         registry (which provider) · load (plugin files) · tasks/* · code/*
herdr/client.ts    execFile over the `herdr` CLI, JSON in/out
repositories/git.ts  branch names, commit counting, push, validation command
persistence/db.ts  node:sqlite, the source of truth for orchestration
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
- Provider dispatch: a task remembers which tracker it came from (`tasks.provider`) so updates go
  back to the same one; a code provider is picked from the repository's git remote, never configured
  per project.
- Errors from CLI tools go through `briefError()` before reaching the log — `gh`/`glab`/`jira` answer
  failures with pages of help text and this loop runs every few seconds.

### Adding a provider

Prefer a plugin file in `~/.config/shepherd/providers/` (see README) — the core needs no changes.
A built-in goes into `providers/{tasks,code}/` plus one line in the `builtinTasks`/`builtinCode` map
in `registry.ts`. A code provider only has to implement `CodeProvider` from `domain/types.ts`; the
optional `check()` is a preflight so an agent does not work for an hour and then fail to open a PR.

## Tests

`node:test` + `node:assert`, no framework. `tests/workflow.test.ts` runs the full lifecycle against
fake Herdr and fake providers but a **real** git repo (bare origin + clone in a tmpdir) — worktrees,
commits and pushes are genuine, nothing touches the network. New orchestration behaviour belongs
there; pure rules belong in `policies.test.ts`.

## Style

Code comments are rare and one line. `ponytail:` comments mark deliberate simplifications and name
the upgrade path (in-memory poll timestamps, schema applied on open instead of migrations, polling
instead of a herdr event stream); respect them — they are decisions, not oversights. The _why_
behind a rule — the failure it prevents — lives in the invariants list above, not in a doc block
over the function. If a change needs a paragraph of rationale, add a bullet there.
