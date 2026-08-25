# shepherd

A control plane on top of [Herdr](https://herdr.dev). It decides what coding agents work on and when,
and shows all autonomous work across projects in one place. Herdr stays the agent runtime, git stays
the source of truth for code, the tracker stays the source of truth for tasks.

```
Task tracker → Orchestrator → Project → Herdr workspace → Agent → Git → PR → Task tracker
```

## Install

```sh
nub install
nub run install:bin           # bundles and drops `shepherd` into ~/.local/bin
shepherd init                 # creates ~/.config/shepherd/config.toml (chmod 600)
shepherd doctor
```

The script is `install:bin`, not `install`, because npm treats a script named `install` as a
lifecycle hook and runs it on every `nub install`. Set `SHEPHERD_BIN_DIR` to install elsewhere.

## Configuration

Lookup order: `$SHEPHERD_CONFIG`, then `./shepherd.toml`, then `~/.config/shepherd/config.toml`.
Credentials live in the `[env]` section of that same file, which `init` creates with mode 600.
Environment variables override it.

```toml
[env]
LINEAR_API_KEY = "lin_api_..."

[task_provider]
type = "linear"
assignee = "me"   # "me" | email | "any". An unassigned task counts as unfinished.
# Only the Todo column is picked up. Backlog is left alone.

[orchestrator]
max_concurrent_runs = 3

[[projects]]
name = "Mochi"
repository = "~/Developer/dev/pet/mochi"
task_project = "Mochi"
agent = "claude"
# validate = "flutter test"
```

State lives in `~/.shepherd/state.db` (`SHEPHERD_DB`) and `~/.shepherd/worktrees/`.

## Commands

```
shepherd init | doctor | cleanup
shepherd projects            tree of projects, tasks and agents
shepherd status              agents, queue, what needs attention
shepherd tasks | agents | runs
shepherd run                 orchestration loop (foreground)
shepherd run <task-id>       a single run
shepherd stop|retry|open <run-id>
shepherd review [run-id]     start review agents for runs awaiting review
shepherd daemon install      autostart on login via launchd
```

## Custom providers

shepherd loads every file in `~/.config/shepherd/providers/` at startup. Override the folder with
`SHEPHERD_PROVIDERS`. Each file exports a map of name to factory, and a factory receives its whole
config section, so fields like `url` arrive without any schema change.

```ts
// ~/.config/shepherd/providers/jira.ts
export const taskProviders = {
  jira: (settings) => ({
    listTasks: async (filter) => { /* ... */ },
    getTask: async (id) => { /* ... */ },
    claimTask: async (id) => {},
    updateStatus: async (id, status) => {},
    addComment: async (id, body) => {},
  }),
};

export const codeProviders = {
  gitlab: (settings) => ({ createChange, getChange, mergeChange }),
};
```

```toml
[task_provider]
type = "jira"
url = "https://jira.example.com"

[code_provider]
type = "gitlab"
```

Node strips the types, so `.ts` files work as they are. If a plugin needs npm dependencies, put a
`package.json` next to it and install them there. Resolution follows the plugin's own path. A broken
plugin never takes the CLI down. It lands in the `shepherd doctor` error list while the rest load.
Secrets still come from `[env]` or the environment, never from provider code.

## Agent roles

```toml
[agents.dev]
prompt = ""              # optional prefix before the task text

[agents.review]
kind = "claude"
prompt = "/code-review"  # empty value starts no review agent
```

Override a role for one project with a nested table inside its `[[projects]]`:

```toml
[[projects]]
name = "Fmc"
repository = "~/Developer/dev/fcm-group/fmc_frontend"
agent = "codex"                        # shorthand for the dev agent kind

[projects.agents.review]
prompt = "/code-review --strict"       # kind is inherited from [agents.review]
```

Priority runs `[projects.agents.*]`, then `project.agent`, then `[agents.*]`, then `codex`.
Inheritance works per field rather than per section, so setting only `prompt` in a project keeps
`kind` global. Review defaults to the same agent kind as development.

`prompt` goes before the task text and can be plain text or the agent's own slash command. The dev
agent works in the workspace root tab. The review agent comes up in a sibling tab of the same
workspace right after the pull request is created and receives its URL. It leaves findings as PR
comments. The orchestrator never parses its answer and never decides on merging.

Every tick checks whether a run in `review` has its review agent, not just the moment the pull
request appears. Runs that started before the role existed, or before you changed the prompt, or
whose agent failed to come up, are caught up on their own. `shepherd review` does the same thing
right away. The `ReviewAgentStarted` event makes both paths idempotent, so a second reviewer never
appears, and a failing review never fails the run.

## Daemon

```sh
shepherd daemon install    # ~/Library/LaunchAgents/dev.shepherd.orchestrator.plist, RunAtLoad + KeepAlive
shepherd daemon            # is the agent installed, is the loop running, where is the log
shepherd daemon start      # restart (launchctl kickstart -k)
shepherd daemon stop|uninstall
```

The daemon reads config and code once at startup, so run `shepherd daemon start` after editing
`config.toml`. `nub run install:bin` already does it: rebuild, install, restart if the agent is
installed.

The log is `~/.shepherd/shepherd.log`. Only one loop may run at a time, because a second
orchestrator would double `max_concurrent_runs`. The pid file `~/.shepherd/daemon.pid` enforces
that, and a pid left by a dead process is cleaned up on the next check. Read-only commands
(`status`, `projects`, `agents`) read SQLite and work with the daemon stopped.

## Architecture

```
src/
├── domain/         types and status derivation (knows nothing about Linear/GitHub/Herdr)
├── orchestrator/   scheduler (when) + workflow (how) + policies (rules)
├── herdr/          thin wrapper over the herdr CLI
├── providers/      tasks/linear.ts, code/github.ts, load.ts for custom ones
├── repositories/   git worktrees, branches, validation
├── persistence/    SQLite, the source of truth for orchestration
├── view.ts         state for the CLI (and a future TUI)
└── cli/
```

Run lifecycle:
`queued → starting → working → (blocked) → validating → creating_change → review → completed | failed`.
Herdr owns agent state (`working/blocked/done/idle`). shepherd never parses terminal output to
guess it.

A task starts a run when four things hold at once. It sits in the tracker's Todo column, it matches
`assignee`, it has no run that did not fail, and `max_concurrent_runs` leaves a free slot. The
scheduler takes one task per project per pass, so a single project cannot hog the queue. A run
waiting in `review` keeps its slot until the pull request is merged or closed.

shepherd writes task status back to the tracker: In Progress on start, In Review once the pull
request exists, Done after somebody merges it.

A partial unique index in SQLite prevents claiming a task twice or creating two changes for one run.
After a restart shepherd rebuilds its state from the database, while the agents keep living in Herdr.

## Checks

```sh
nub run check       # status logic, policies, SQLite invariants
nub run typecheck
```

## Not built yet

A TUI, Jira/GitLab/Bitbucket/GitHub Issues providers, auto-merge. Adding a provider means writing one
file in `providers/`, either built in or in the config folder. The core stays untouched.
