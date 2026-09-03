# shepherd

A control plane over [Herdr](https://herdr.dev). It decides what coding agents work on and when,
and gives you one view across all projects. Herdr stays the agent runtime, git is the truth about
code, the tracker is the truth about tasks.

```
Task tracker → Orchestrator → Project → Herdr workspace → Agent → Git → MR/PR → Task tracker
```

## Install

```sh
bun install
bun run install:bin           # compiles a standalone binary into ~/.local/bin
shepherd init                 # writes ~/.config/shepherd/config.yaml (chmod 600)
shepherd doctor
```

It is `install:bin` and not `install` because an npm script named `install` is a lifecycle hook.
It would run on every `bun install`. Set `SHEPHERD_BIN_DIR` to change the target directory.

## Configuration

Lookup order: `$SHEPHERD_CONFIG`, then `./shepherd.yaml`, then `~/.config/shepherd/config.yaml`.
Keys live in the `env` section of that same file (mode 600). Environment variables win over it.

```yaml
env:
  LINEAR_API_KEY: lin_api_...
  # no GITHUB_TOKEN needed, shepherd uses the gh CLI auth

task_provider: # shared by every tracker
  assignee: me # "me" | email | "any" (the last one also picks up unassigned)
  # only the Todo column is pulled, Backlog is left alone

code_provider: # shared by every forge
  base_branch: main

orchestrator:
  max_concurrent_runs: 3
  max_attempts: 3 # failed runs per task before it needs a human
  poll_interval_ms: 5000 # how often Herdr is asked for agent state
  task_sync_interval_ms: 60000 # how often trackers are polled
  change_poll_interval_ms: 60000 # how often an open MR/PR is re-checked
  # max_review_rounds: 3         # rework rounds before the run gives up
  # auto_merge: true             # merge approved changes with green checks

projects:
  - name: Mochi
    repository: ~/Developer/dev/pet/mochi
    task_project: Mochi # project key on the tracker side
    agent: claude
    # validate: flutter test     # runs in the worktree, empty means validation is skipped
    # base_branch: develop       # overrides code_provider.base_branch
```

State lives in `~/.shepherd/state.db` (`SHEPHERD_DB`). Worktrees belong to herdr, which puts them
under `~/.herdr/worktrees/<repo>/<branch>`; set `orchestrator.worktrees` only to override that.

## Providers

There is no `type =` key anywhere. Every registered tracker is polled, and the git remote picks the
forge, so one shepherd can drive a Linear plus GitHub project and a Jira plus self-hosted GitLab one
side by side. A project joins a tracker through `task_project`.

Built in: Linear for tasks, GitHub (`gh`) and GitLab (`glab`) for changes. Per-provider settings go
into a section named after the provider:

```yaml
task_providers:
  jira:
    url: https://company.atlassian.net

code_providers:
  gitlab:
    url: https://ci.unitedline.net
    hosts: [ci.unitedline.net] # remotes that belong to this provider
    # transport: token           # skip glab, talk to the REST API with GITLAB_TOKEN
```

Remote matching goes through `hosts` first, then falls back to the host name containing `github` or
`gitlab`. Self-hosted installations need the `hosts` line, nothing else recognises them. GitLab uses
`glab` credentials when it is logged in and drops to `GITLAB_TOKEN` when it is not.

A tracker plugin is only polled when it has a `[task_providers.<name>]` section, since a plugin with
no settings has no host to talk to.

### Plugins

Files in `~/.config/shepherd/providers/` (and in a `providers/` folder next to a project-local
`shepherd.yaml`, or in `SHEPHERD_PROVIDERS`) are loaded at startup. A file that exports one factory
is registered under its own file name, so `jira.ts` becomes the `jira` provider and reads
`[task_providers.jira]`. A factory gets its whole config section.

```ts
// ~/.config/shepherd/providers/jira.ts
export const taskProvider = (settings) => new JiraTaskProvider(settings);
```

Export `taskProviders` or `codeProviders` maps instead when one file carries several. The core knows
nothing about plugins. A broken file does not take the CLI down, `shepherd doctor` reports it.

## Commands

```
shepherd init | doctor | cleanup
shepherd projects            tree of projects, tasks and agents
shepherd status              summary: agents / queue / needs attention
shepherd ui                  full-screen dashboard, see below
shepherd tasks | agents | runs
shepherd run                 orchestration loop (foreground)
shepherd daemon install      start at login via launchd
shepherd run <task-id>       a single run
shepherd review [run-id]     start review agents for runs waiting on review
shepherd stop|retry|open <run-id>
```

`shepherd ui` is the same state as a screen you can steer: projects and their tasks on the left,
the selected run on the right — its change, its approval and check marks, its last events — and
underneath, the tail of what the agent is printing right now.

```
j/k  move        tab  switch pane     r  retry     s  stop
o    open the workspace in Herdr      v  review    x  reset
S    sync the trackers (the only key that goes to the network)      q  quit
```

It reads SQLite every 1.5 s, so it costs the trackers nothing to leave open, and it decides nothing
on its own: every key calls what the matching `shepherd` command calls. One action runs at a time —
a push or a workspace teardown takes seconds — and the footer says how it ended.

Tasks can also be written, not only read. `shepherd task` talks to the tracker directly — the
orchestrator never creates or edits tasks itself.

```
shepherd task new <project> <title...>   file it in that project's tracker
shepherd task show <task-id>
shepherd task edit <task-id> [title...]  --body rewrites the description
shepherd task status <task-id> <todo|in_progress|in_review|done>
shepherd task comment <task-id> <body...>
shepherd task rm <task-id>               archived, not deleted
```

`--provider <name>` picks the tracker when several are registered; a task that has already been
synced remembers its own. `new`, `edit` and `rm` are optional for a provider — a plugin that only
implements the five methods the orchestrator needs answers "linear cannot create tasks" and nothing
breaks.

## Agent roles

```yaml
agents:
  plan:
    skill: superpowers:writing-plans # unset prompt and skill: no planning pass
  dev:
    prompt: "" # optional prefix in front of the task text
    skill: "" # named skill, put in front of everything else
  review:
    kind: claude
    prompt: /code-review # empty means no review agent is started
```

`skill` becomes a leading `Use the <skill> skill.` in that role's prompt, so a Claude agent reaches
for it before anything else; an agent without skills just reads it as one more instruction.

`plan` is a pass in front of the work, in the same agent: it is asked to read the code and publish
an implementation plan with `shepherd task comment`, and once it falls idle the same agent gets the
work prompt with the plan still in its context. The run sits in `planning` meanwhile. A task whose
change is being resumed skips it — that work is already done.

To override a role for one project, nest `agents` inside it:

```yaml
projects:
  - name: Fmc
    repository: ~/Developer/dev/fcm-group/fmc_frontend
    agent: codex # shorthand for the dev agent kind
    agents:
      review:
        prompt: /code-review --strict # kind is inherited from agents.review
```

`args` are the agent's own command-line flags, passed through to it by Herdr. Unset means the
kind's defaults, which for `claude` is `--dangerously-skip-permissions`: an agent that stops to ask
for permission never finishes a run nobody is watching. Set `args = []` to take that off.

Precedence: `projects[].agents.*`, then `project.agent`, then `agents.*`, then `codex`. Fields are
inherited one by one, not as a whole section. Set only `prompt` in a project and `kind` stays global.
Review runs with the same agent as development unless you say otherwise.

`prompt` is what goes in front of the task text: plain text, or a slash command the agent itself
understands. The dev agent works in the workspace root tab. The review agent comes up as a sibling
tab in the same workspace right after the change is created, and gets the link to it. It leaves its
notes as comments on the merge request. The orchestrator does not parse its answer. A failed review
does not fail the run.

Sending the task back to Todo in the tracker while the change is open means "rework": the
comments on the change (since it was opened, or since the last round) go to the dev agent, the task
returns to In Progress, and after validation the same change goes back to In Review with a fresh
review pass. `max_review_rounds` (default 3) caps that loop. A task in Todo whose branch already carries an open
change — after a restart, a fresh database or `shepherd retry` — is resumed on that change with its
comments, never redone from scratch. Once a human approves the change and
checks are green, shepherd merges it; moving the task to Done in the tracker says the same thing,
which is what you need on a repository where GitHub refuses an approval from the pull request's own
author. `auto_merge = false` leaves merging to a human as well. On GitHub
an approving review counts even without a required-review rule; on GitLab an approval by a person is
required, since `approved` is true by itself when a project has no approval rules.

## Daemon

```sh
shepherd daemon install    # macOS: ~/Library/LaunchAgents/dev.shepherd.orchestrator.plist (RunAtLoad + KeepAlive)
                           # Linux: ~/.config/systemd/user/dev.shepherd.orchestrator.service (enable --now)
shepherd daemon            # is the service installed, is the loop running, where the log is
shepherd daemon start|restart
shepherd daemon logs       # path to the log
shepherd daemon stop|uninstall
```

A systemd user unit stops when the login session ends; `loginctl enable-linger $USER` keeps it up.

The daemon holds config and code in memory from the moment it starts, so edit `config.yaml` and you
need `shepherd daemon start`. `bun run install:bin` does that for you: it rebuilds, installs the
binary, and restarts the daemon if one is installed.

The log is `~/.shepherd/shepherd.log`. Errors from command-line tools are cut to their first line:
a failing `jira` or `gh` answers with a page of help text, and logging that every minute is how a log
file reaches six megabytes. The loop is exclusive: the pid file `~/.shepherd/daemon.pid`
stops a second orchestrator from coming up, which would double `max_concurrent_runs`. A pid left by
a crashed process is cleaned up on its own. Read-only commands (`status`, `projects`, `agents`) do
not need the daemon, they read SQLite.

## Architecture

```
src/
├── core/           app wiring, config, SQLite
├── modules/
│   ├── orchestrator/  scheduler (when) + workflow (how) + policies (rules)
│   ├── providers/     registry, plugin loader, tasks/*, code/*
│   ├── herdr/         thin wrapper over the herdr CLI
│   ├── theme/         terminal palette → colour roles
│   ├── tui/           the dashboard (solid + opentui)
│   └── cli/           commands and the daemon
└── shared/         domain types, state view, actions, components, git, log
```

Run lifecycle:
`queued → starting → working → (blocked) → validating → creating_change → review → completed | failed`,
with `review → working` when a human sends the task back to Todo.
Agent state (`working/blocked/done/idle`) comes from Herdr in full. Nothing parses terminal output.

Guarantees: a partial unique index in SQLite keeps a task from being picked up twice and a run from
opening two changes. After a restart, state is restored from the database and the agents in Herdr
keep living.

## Tests

```sh
bun test            # node:test over tests/**, no framework to install
bun run typecheck
bun run fmt         # oxfmt
bun run lint        # oxlint
```

oxfmt and oxlint are dev dependencies, run through `bun run` (the local `node_modules/.bin` runner).

`tests/` covers the status logic, policies, SQLite invariants and provider parsing, plus the whole
run lifecycle in `workflow.test.ts` against fake Herdr and providers with a real git repository:
worktrees, commits and pushes are genuine, nothing touches the network.

## Not there yet

A TUI, Jira and Bitbucket as built-ins rather than plugins.
