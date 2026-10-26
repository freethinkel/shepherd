# Worktree hooks: set up in Shepherd, run by herdr

## Intent

Commands that prepare a new git worktree (link `.env`, copy `node_modules`, `pnpm install`, `git submodule update`)
and clean up after a removed one now run from the user's own herdr plugin, `freethinkel.worktree-hooks`
(`~/Developer/pet/herdr-plugin-worktree-hooks`): Node.js and a hand-edited YAML file. Other Shepherd users have
neither. They should be able to set hooks up in Shepherd's settings, with nothing to install.

Once set up, the hooks belong to herdr, not to Shepherd: they run when the worktree is made from herdr's TUI, its
CLI, another client, or Shepherd, while Shepherd is closed, and after Shepherd is deleted.

## Goals

- A **Worktrees** section in Shepherd's settings: a list of projects, each a **name**, a **main checkout** (`dir`), the
  commands to run **after a worktree is created** and those **after one is removed**. Projects can be added, edited
  and removed there.
- Saving installs, once, a herdr plugin **written in POSIX `sh`** and writes the hooks file it reads. herdr runs the
  plugin on its own events; Shepherd plays no part at run time.
- The plugin needs only what every machine with herdr already has: `sh`, coreutils, `herdr`. `jq` is used when present
  and not required.
- The plugin and the hooks file live in herdr's config directory, outside `Shepherd.app`.
- The plugin behaves as `freethinkel.worktree-hooks` does today (see Behaviour).

## Non-goals

- Remote herdr machines. v1 installs into the herdr on this computer only. When Shepherd shows a machine, the section
  says the hooks apply to this computer. Rolling the plugin out to machines over SSH can come later; nothing in this
  design blocks it, since the plugin is plain `sh`.
- A helper binary, Node, or any runtime beyond `sh`. Discussed and dropped: a binary needs a build per OS and
  architecture, has to be copied to each machine and kept in step with Shepherd; the logic is ~100 lines.
- Hooks stored in the repo and shared with a team. The hooks are the user's, one file for all projects, as now.
- Windows. herdr plugins run on macOS and Linux only.
- An importer for the old YAML. Only this user has it; their file is converted once by hand on rollout (see Rollout).

## Anti-goals

- Hooks that stop working when Shepherd is closed, moved or deleted.
- A hook run twice for one worktree, or run when an old worktree is merely focused.
- Hooks running against the main checkout.

## Parts

### 1. The hooks file

`<herdr config dir>/plugins/config/shepherd.worktree-hooks/hooks.conf` — the directory herdr passes to the plugin as
`$HERDR_PLUGIN_CONFIG_DIR`. Line-based, so `sh` reads it with `while read` and nobody parses YAML:

```
# Worktree hooks, one file for all projects. Shepherd → Settings → Worktrees edits it; so can you.
[callcenter]
dir = ~/Developer/sftpro/callcenter-frontend
created = ln -sf "$MAIN/.env" "$WORKTREE/.env"
created = cp -Rc "$MAIN/node_modules" node_modules 2>/dev/null || cp -R "$MAIN/node_modules" node_modules
removed = docker compose -p "$(basename "$WORKTREE")" down -v
```

- `[name]` starts a project; `dir`, `created`, `removed` follow. `created` and `removed` repeat, one command per line,
  run in order. A command is the rest of the line after `= `, verbatim (no quoting or escaping of its own).
- Lines starting with `#` and blank lines are ignored. `~` at the start of `dir` is `$HOME`.
- Shepherd reads this file to fill the section and writes it back whole (temp file + rename, so the plugin never reads
  half of one). Comments in a hand-edited file are not kept on save. *(My call.)*
- One command per line means no multi-line commands. Every command in the current YAML is one line; a longer script
  goes in a file the command runs.

### 2. The plugin

Id `shepherd.worktree-hooks`, root `<herdr config dir>/plugins/local/shepherd.worktree-hooks/` (`dir()` in
`src/herdr/config.rs` gives the herdr config dir; herdr manages only `plugins/github/` and `plugins/config/`). Its files are embedded in Shepherd (`include_str!`) and kept in the
repo, e.g. `assets/herdr-plugin/`.

- `herdr-plugin.toml`: `platforms = ["macos", "linux"]`; events `worktree.created`, `workspace.created`,
  `workspace.focused`, `worktree.removed`, each `command = ["/bin/sh", "event.sh"]` (herdr runs commands with the
plugin root as cwd). The reason for four events is in the old
  plugin's README: herdr's UI "new worktree" emits only `workspace.focused`, the CLI emits the first two.
- `event.sh`: POSIX `sh` (`dash` on Debian must run it).

### 3. Install and update

On saving the section, Shepherd:

1. writes the plugin's files into its root if absent or different from the embedded ones;
2. writes `hooks.conf`;
3. if `herdr plugin list` does not show `shepherd.worktree-hooks`, runs `herdr plugin link <root>` (with the
   `herdr_bin` from settings).

A newer Shepherd with changed plugin files rewrites them on the next save and on start-up if the plugin is installed.
herdr reads `event.sh` afresh on each run, but keeps a copy of the manifest's events in `plugins.json` from link
time, so a changed manifest takes `herdr plugin unlink` + `link`.
Errors (no herdr, link failed) show inline in the section, as the herdr section shows its own.

## Behaviour of `event.sh`

Ported from `bin/event.mjs` and `src/core.mjs` of the old plugin; keep its semantics.

Input from herdr: `HERDR_PLUGIN_EVENT`, `HERDR_PLUGIN_EVENT_JSON`, `HERDR_PLUGIN_CONFIG_DIR`, `HERDR_PLUGIN_STATE_DIR`
(fallback `${TMPDIR:-/tmp}/shepherd-worktree-hooks`), `HERDR_BIN_PATH` (fallback `herdr`).

**Facts from the event JSON**, under `data` (or the root): workspace id `workspace.workspace_id` / `workspace.id` /
`workspace_id`; worktree = `worktree` or `workspace.worktree`, from which `checkout_path` (or `path`), `repo_root`,
`repo_name`, `is_linked_worktree`. Read with `jq` when it is on PATH; otherwise take the first `"key":"value"` of each
with `sed`. *ponytail: the sed path breaks on paths containing `"` or escapes; jq is the upgrade, already preferred.*

**Created** (any event without `remove` in its name):

1. No projects in `hooks.conf` → exit.
2. `workspace.focused` whose workspace is in `state/decided/` → exit (it fires on every focus). The decided key is the
   herdr session plus the workspace id, not the id alone: ids are short counters (`w1`, `wFA`), one state dir serves
   every session, and the old plugin's id-only key can silently skip a new worktree whose id was marked in another
   session. Which env var names the session (socket path or similar) is for the builder to find. *(My call.)*
3. If the payload lacks the checkout path or repo root, run `herdr workspace get <id>` (fallback: `workspace list`
   and find the id) and take the worktree facts plus `tab_count`, `pane_count`. *Fresh* = 1 tab and 1 pane; when the
   payload carried the facts, fresh is true.
4. Skip, marking the focused workspace decided, when: no checkout path; `is_linked_worktree` is false (the main
   checkout); not fresh; no project matches.
5. Match: a project whose `dir` (tilde-expanded, `cd -P` resolved) equals `repo_root`; else one whose
   `basename dir` equals `repo_name`.
6. Claim: `mkdir state/applied/<key>` where key = `cksum` of the checkout path; if it exists, already ran → exit.
   Unless the claim is stale: when the checkout's `.git` file is newer than the claim (`[ .git -nt claim ]`), the
   worktree was removed outside herdr and made again at the same path; drop the claim and go on. *(My call.)* Save the
   project's name to `state/map/<key>`. A worktree whose `.git` is not newer than `.installed` in the config dir
   (Shepherd writes it on the first save) was there before the hooks and is left alone: without it, the first focus
   or `workspace.created` on every existing worktree would run its `created` commands over its files.
7. Run each `created` command with `sh -c` in the checkout, env `PROJECT`, `MAIN` (the expanded `dir`), `WORKTREE`,
   `EVENT=created`; stop at the first non-zero exit. On failure remove the claim, so the next event retries.
8. Mark a focused workspace decided.

**Removed**: checkout path from the payload (none → log and exit). Project from `state/map/<key>`, else by matching
as above. Remove the claim and the map entry. Run each `removed` command from `MAIN` (the worktree is gone), env as
above with `EVENT=removed`, stopping at the first failure.

**PATH**: herdr's server, started by launchd or an app, runs with `PATH=/usr/bin:/bin:/usr/sbin:/sbin` (seen on this
Mac: it is why the old Node plugin fails every event with `node: No such file or directory`). Hook commands call
`npm`, `pnpm`, `git` from Homebrew and nvm, so before running any, `event.sh` takes the user's PATH from their login
shell once: `"$SHELL" -lic` printing `$PATH` after a marker (an interactive zsh may print other things first), stdin
from `/dev/null`. Empty or failed → keep the current PATH with `/opt/homebrew/bin:/usr/local/bin:$HOME/.local/bin`
appended. Commands still run under `sh -c`. *(My call.)*

**Logging**: each step and each command to stderr as `[worktree-hooks] …`; herdr keeps it
(`herdr plugin log list --plugin shepherd.worktree-hooks`).

## The settings section

A new section, **Worktrees**, after **herdr** in the sidebar. *(My call; the "Worktrees folder" field stays in the
herdr section.)* For each project: name, main checkout (a text field), and the two command lists, one command per line
— a multi-line field, or a list of single-line fields with add and remove if Shepherd's widgets have no multi-line
input (builder's call). Saved as other settings are, on change. A line under the title says the hooks run in herdr and
keep working without Shepherd, and lists `$MAIN`, `$WORKTREE`, `$PROJECT`.

## Checks

- `event.sh`: one `sh` test script beside it that runs it against a stub `herdr`, a temp `hooks.conf` and fake event
  JSON — created once, not twice for the second event, not for a stale focus, not for the main checkout; again for the
  same id in another session; again for a worktree re-made at the same path; removed runs from `MAIN`. Run under `sh` and, if present, `dash`.
- Shepherd: a Rust test that `hooks.conf` parses and writes back to the same projects.

## Rollout (this user)

Convert `~/.config/herdr/plugins/config/freethinkel.worktree-hooks/config.yml` into `hooks.conf` by hand (YAML
single-quoted commands lose their outer quotes), then `herdr plugin disable freethinkel.worktree-hooks`, so the hooks
do not run twice.

## Left to the builder

- Widget choice for command lists; layout of the section.
- Whether a manifest change needs a relink or reload in herdr.
- Exact layout of the plugin's files in the repo.

## Sessions (added 2026-10-26)

The user asked for the section to be grouped by herdr session, and for projects to be added only from the repos
open in herdr (the sidebar's).

- A project belongs to one herdr session: its section is `[session/name]`. A bare `[name]` (hand-written, or from
  before sessions) runs in every session; the section shows it under `default` and the next save writes it as
  `[default/name]`.
- `event.sh` knows its session from `$HERDR_SOCKET_PATH`: `<dir>/sessions/<name>/herdr.sock` is `<name>`, anything
  else `default`. Other sessions' sections are skipped. `$PROJECT` is the name without the session.
- Settings → Worktrees: a group per herdr session (`herdr session list`), named as the session bar names it; running
  sessions always, stopped ones only when they have projects. "Add project" lists the session's open repos
  (`workspace.list` on its socket: each repo's main checkout once), minus those already added; name and main
  checkout come from the repo, only the commands are typed. A stopped session's projects can be edited, not added
  to. Remote machines are still out of scope.
- The command fields grow with their text and never scroll themselves; the settings page scrolls with the app's
  scrollbar (the agents list's).
