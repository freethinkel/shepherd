# Worktree Hooks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Worktree hooks set up in Shepherd's settings and run by a POSIX `sh` herdr plugin that keeps working without
Shepherd.

**Architecture:** The plugin (`herdr-plugin.toml`, `event.sh`) lives in the repo under `assets/herdr-plugin/` and is
embedded in Shepherd with `include_str!`. `src/herdr/hooks.rs` owns the `hooks.conf` format, writes the plugin into
herdr's config dir and links it. Settings → Worktrees edits the projects through that module.

**Tech Stack:** POSIX `sh` (must run under `dash`), optional `jq`; Rust, gpui + gpui-base `InputState`.

**Spec:** `docs/superpowers/specs/2026-10-26-worktree-hooks-design.md`

## Global Constraints

- Plugin id `shepherd.worktree-hooks`; `platforms = ["macos", "linux"]`; `min_herdr_version = "0.7.0"`.
- Plugin root `<herdr dir>/plugins/local/shepherd.worktree-hooks/`; hooks file
  `<herdr dir>/plugins/config/shepherd.worktree-hooks/hooks.conf`; `<herdr dir>` is `crate::herdr::config::dir()`.
- Event commands `["/bin/sh", "event.sh"]` on `worktree.created`, `workspace.created`, `workspace.focused`,
  `worktree.removed`.
- `event.sh` needs only `sh`, coreutils and `$HERDR_BIN_PATH`; `jq` used when on PATH, never required.
- Hook env: `PROJECT`, `MAIN` (expanded `dir`), `WORKTREE`, `EVENT` (`created` / `removed`). Commands run with
  `sh -c`, in order, stopping at the first non-zero exit.
- Log lines to stderr prefixed `[worktree-hooks] `.
- Local herdr only; nothing is installed on remote machines.
- Commit messages follow the repo's `area: what changed` style.

## Review Focus

1. **herdr's bare PATH** (`/usr/bin:/bin:/usr/sbin:/sbin`): `npm`/`pnpm` from Homebrew or nvm must still be found →
   Task 1 test `path_from_login_shell`.
2. **Commands with quotes, `$(…)`, `||`, `2>/dev/null`** (all in the user's real config) must reach `sh -c` byte for
   byte → Task 1 test `command_verbatim`, Task 2 test `roundtrip_keeps_commands_verbatim`.
3. **A focus storm**: `workspace.focused` on an old, arranged worktree must not run anything and must not call herdr
   twice → Task 1 test `focus_on_arranged_worktree_skips`.
4. **A path with spaces** in `dir` or the checkout → Task 1 test `paths_with_spaces`.
5. **Hand-edited `hooks.conf`** with comments, CRLF line ends, keys before the first `[section]`: parse without
   panicking, ignore what isn't understood → Task 2 test `parse_tolerates_junk`.

---

### Task 1: The `sh` plugin

**Files:**
- Create: `assets/herdr-plugin/herdr-plugin.toml`
- Create: `assets/herdr-plugin/event.sh`
- Create: `assets/herdr-plugin/test.sh`
- Create: `tests/herdr_plugin.rs`

**Interfaces:**
- Produces: the two plugin files (embedded by Task 2); `hooks.conf` format (spec §1) that `event.sh` reads from
  `$HERDR_PLUGIN_CONFIG_DIR/hooks.conf`.

`test.sh` builds a temp dir per case with: a `hooks.conf`; a stub `herdr` (a script that prints canned JSON for
`workspace get <id>` / `workspace list` and appends each call to `calls.log`); a fake main checkout and worktree (a
`mkdir` with a `.git` file); hook commands that append to `ran.log` (e.g. `created = echo "$EVENT $PROJECT $PWD" >> "$LOG"`).
It runs `event.sh` with `HERDR_PLUGIN_EVENT`, `HERDR_PLUGIN_EVENT_JSON`, `HERDR_PLUGIN_CONFIG_DIR`,
`HERDR_PLUGIN_STATE_DIR`, `HERDR_BIN_PATH`, `HERDR_SOCKET_PATH`, `SHELL=/bin/sh`, `PATH=/usr/bin:/bin`, prints
`ok <name>` / `FAIL <name>: <why>` per case, exits non-zero on any failure. Run each case twice when `jq` is present:
with it, and with a `PATH` that hides it, so the `sed` path is covered.

- [ ] **Step 1: Write `test.sh` with these cases**

| case | event(s) | expect |
|---|---|---|
| `created_runs_once` | `worktree.created` then `workspace.created` (payload has worktree facts) | `ran.log` has one `created <project> <worktree>` line, cwd is the worktree |
| `focus_path_queries_herdr` | `workspace.focused` (id only); stub returns 1 tab, 1 pane, linked | runs once; `calls.log` has one `workspace get` |
| `focus_on_arranged_worktree_skips` | `workspace.focused` twice; stub returns 2 panes | nothing ran; second event makes no herdr call |
| `main_checkout_skips` | `worktree.created` with `is_linked_worktree: false` | nothing ran |
| `same_id_other_session_runs` | focus `w1` under socket A (decided), then a new worktree as `w1` under socket B | runs for B |
| `remade_at_same_path_runs` | created; touch the worktree's `.git` newer than the claim; created again | ran twice |
| `failure_releases_claim` | first command `false`; created twice | second run retries (two attempts in `ran.log` from a command before `false`) |
| `stops_at_first_failure` | `created = false`, `created = echo after >> "$LOG"` | `after` absent |
| `removed_runs_from_main` | created, then `worktree.removed` | `removed <project> <main>`; claim and map gone |
| `match_by_basename` | `repo_root` absent, `repo_name` = basename of `dir` | runs |
| `command_verbatim` | `created = [ "$(printf '%s' "a b")" = "a b" ] && echo q >> "$LOG" \|\| true` | `q` in `ran.log` |
| `paths_with_spaces` | `dir` and checkout both contain a space | runs, `$PWD` correct |
| `path_from_login_shell` | `SHELL` = a stub that prints junk, then the marker and `/stub/bin:/usr/bin:/bin`; command `command -v fakepnpm >> "$LOG"` with `fakepnpm` in `/stub/bin` | found |
| `no_projects_exits_quietly` | empty `hooks.conf` | exit 0, no herdr call |

- [ ] **Step 2: Write `tests/herdr_plugin.rs`**

```rust
#[cfg(unix)]
#[test]
fn event_sh() {
    for shell in ["sh", "dash"] {
        if shell == "dash" && std::process::Command::new("dash").arg("-c").arg(":").status().is_err() { continue }
        let out = std::process::Command::new(shell).arg("assets/herdr-plugin/test.sh").output().unwrap();
        assert!(out.status.success(), "{shell}:\n{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    }
}
```

`test.sh` runs `event.sh` with the shell it was itself started by (`$0`'s interpreter: pass it as `SH=${SH:-sh}` from
the Rust test via env, and invoke `"$SH" event.sh`).

- [ ] **Step 3: Run it, expect failure**

Run: `cargo test --test herdr_plugin`
Expected: FAIL (`event.sh` missing).

- [ ] **Step 4: Write `herdr-plugin.toml`**

Fields from Global Constraints; `name = "Worktree hooks (Shepherd)"`; `version = "1.0.0"`; `description` says it is
managed by Shepherd → Settings → Worktrees and that `hooks.conf` can be edited by hand.

- [ ] **Step 5: Write `event.sh`**

Behaviour exactly as spec §"Behaviour of `event.sh`", including the decided key (cksum of `$HERDR_SOCKET_PATH` +
workspace id), the stale-claim check (`[ "$checkout/.git" -nt "$claim" ]`), and PATH from the login shell (spec
§PATH), resolved only when commands are about to run. Before calling `workspace get` on the focus path, try the
worktree facts in `$HERDR_PLUGIN_CONTEXT_JSON` (herdr documents worktree fields there); keep the query as the
fallback. State keys: `printf %s "$x" | cksum | cut -d' ' -f1`. Read `hooks.conf` with `while IFS= read -r line`,
strip a trailing `\r`. POSIX only: no `local`, arrays, `[[`, `$'…'`, `pipefail`.

- [ ] **Step 6: Run tests, expect pass**

Run: `cargo test --test herdr_plugin`
Expected: PASS, under `sh` and (if installed) `dash`. Also `sh assets/herdr-plugin/test.sh` prints all `ok`.

- [ ] **Step 7: Commit**

```bash
git add assets/herdr-plugin tests/herdr_plugin.rs
git commit -m "worktree hooks: a herdr plugin in POSIX sh that runs a project's commands when herdr makes or removes its worktree"
```

### Task 2: `hooks.conf` and installing the plugin

**Files:**
- Create: `src/herdr/hooks.rs`
- Modify: `src/herdr/mod.rs` (add `pub mod hooks;`)
- Modify: `src/main.rs` (call `refresh` once at start, off the main thread)

**Interfaces:**
- Consumes: `assets/herdr-plugin/{herdr-plugin.toml,event.sh}` (Task 1) via `include_str!`; `config::dir()`;
  `session::binary()`.
- Produces:
  ```rust
  pub const PLUGIN_ID: &str = "shepherd.worktree-hooks";
  #[derive(Debug, Clone, Default, PartialEq)]
  pub struct Project { pub name: String, pub dir: String, pub created: Vec<String>, pub removed: Vec<String> }
  pub fn parse(text: &str) -> Vec<Project>;
  pub fn render(projects: &[Project]) -> String;
  pub fn conf_path() -> PathBuf;                       // <dir>/plugins/config/shepherd.worktree-hooks/hooks.conf
  pub fn load() -> anyhow::Result<Vec<Project>>;       // missing file → empty
  pub fn save(projects: &[Project]) -> anyhow::Result<()>; // install/update plugin, link if needed, write conf
  pub fn refresh() -> anyhow::Result<()>;              // if the plugin root exists: rewrite changed files, relink on manifest change
  ```

- [ ] **Step 1: Write the failing tests (in `hooks.rs`, `#[cfg(test)] mod tests`)**

- `roundtrip_keeps_commands_verbatim`: two projects, commands taken from the user's real config (with `'`, `"`,
  `$(…)`, `||`, `2>/dev/null`, `{ …; }`); `parse(&render(&p)) == p`.
- `parse_reads_spec_example`: the block in spec §1 → one project `callcenter` with two `created`, one `removed`.
- `parse_tolerates_junk`: CRLF, `#` comments, blank lines, `dir = x` before any section, an unknown key, a line
  without `=` → those are ignored, the valid project still parses.
- `render_starts_with_header_comment`: output's first line starts with `#`.
- `save_writes_files_and_conf` (using a temp herdr dir via `XDG_CONFIG_HOME` and `SHEPHERD_HERDR` pointing at a stub
  script that logs its args): plugin files present and equal to the embedded ones, `hooks.conf` written, the stub
  saw `plugin link <root>`; a second `save` with unchanged files does not link again (stub's `plugin list` prints the
  id). Serialize env-mutating tests with a `static Mutex`.

- [ ] **Step 2: Run, expect failure**

Run: `cargo test --lib herdr::hooks`
Expected: FAIL (module missing).

- [ ] **Step 3: Implement**

`render` writes `[name]`, `dir = …`, then each `created = …`, each `removed = …`, a blank line between projects.
`save`: create root and config dirs; write each embedded file if content differs, remembering whether the manifest
changed; then if `herdr plugin list` output lacks `- shepherd.worktree-hooks ` → `plugin link <root>`; else if the
manifest changed → `plugin unlink <id>` then `plugin link <root>`; write `hooks.conf` to a temp file in the same dir
and rename. Errors carry herdr's stderr. `refresh` does the file/relink part only, and nothing when the root is
absent (never installs on its own). `main.rs`: `std::thread::spawn(|| if let Err(e) = shepherd::herdr::hooks::refresh() { eprintln!("shepherd: worktree hooks: {e:#}") });`
after `set_binary`.

- [ ] **Step 4: Run, expect pass**

Run: `cargo test --lib herdr::hooks && cargo clippy --all-targets -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 5: Commit**

```bash
git add src/herdr/hooks.rs src/herdr/mod.rs src/main.rs
git commit -m "worktree hooks: hooks.conf read and written, the plugin written into herdr's folder and linked; a newer Shepherd refreshes it at start"
```

### Task 3: Settings → Worktrees

**Files:**
- Create: `src/ui/settings/worktrees.rs`
- Modify: `src/ui/settings.rs` (`Section::Worktrees` after `Herdr` in `ALL`, label `"Worktrees"`, icon
  `"icons/git-branch.svg"`; `mod worktrees;`; fields on `SettingsView`)
- Modify: `src/ui/settings/render.rs` (the section's page; call the sync like `sync_web_fields`)

**Interfaces:**
- Consumes: `hooks::{Project, load, save}` (Task 2).
- Produces: nothing used elsewhere.

Model it on `src/ui/settings/web.rs`, with one difference: the projects are not in `Settings` (Shepherd's JSON) but in
`hooks.conf`, so `SettingsView` holds `worktree_projects: Option<Vec<Project>>` (loaded with `hooks::load()` the first
time the section renders), the per-project field entities, and `worktree_note: Option<String>`.

- [ ] **Step 1: Write the failing test** (`worktrees.rs`, `#[cfg(test)]`)

`lines_to_commands`: `fn commands(text: &str) -> Vec<String>` — splits on `\n`, trims each line's ends, drops empty
lines: `"a\n\n  b  \r\n"` → `["a", "b"]`. And `fn text(commands: &[String]) -> String` joins with `\n`.

- [ ] **Step 2: Run, expect failure**

Run: `cargo test --lib worktrees`
Expected: FAIL.

- [ ] **Step 3: Implement the section**

- Header: `page_header(&theme, "Worktrees", "Commands herdr runs in a new worktree and after one is removed. They run in herdr, so they keep working with Shepherd closed or deleted, from any client.")`.
- A muted line under it: `$MAIN — the project's main checkout · $WORKTREE — the new worktree · $PROJECT — the project's name`.
- When `settings.herdr_machine` is `Some`: a `note` saying the hooks are installed in the herdr on this computer, not
  on the machine shown.
- Per project (layout as a web page's block, with the red bin): `Name`, `Main checkout` single-line fields; `After
  creating` and `After removing` as `InputState::new(..).auto_grow(2, 12)`, one command per line, placeholder for
  created `ln -sf "$MAIN/.env" .env`, for removed `docker compose down`.
- "Add project" by the list's label; an empty list shows a row "No projects yet" / "Add one with its main checkout
  and the commands a new worktree needs."
- Save on `InputEvent::Blur` (and `PressEnter` for the single-line fields): rebuild `Vec<Project>` from the fields,
  drop projects whose name and dir are both empty, run `hooks::save` on a background thread
  (`cx.background_spawn`), put an error into `worktree_note`, clear it on success. Removing a project saves at once.

- [ ] **Step 4: Run, expect pass, and look at it**

Run: `cargo test --lib worktrees && cargo clippy --all-targets -- -D warnings`
Expected: PASS. Then `cargo run` (memory: the bundle is refreshed only by `cargo run`), open Settings → Worktrees,
add a project, leave the field; check `~/.config/herdr/plugins/config/shepherd.worktree-hooks/hooks.conf` and
`herdr plugin list` shows `shepherd.worktree-hooks enabled`. Screenshot the section for the user.

- [ ] **Step 5: Commit**

```bash
git add src/ui/settings.rs src/ui/settings/render.rs src/ui/settings/worktrees.rs
git commit -m "settings: Worktrees, the commands herdr runs in a new worktree and after removing one, per project"
```

### Task 4: Rollout on this Mac (not code; ask the user before step 2)

- [ ] **Step 1:** Convert `~/.config/herdr/plugins/config/freethinkel.worktree-hooks/config.yml` into
  `hooks.conf` (YAML single-quoted commands lose their outer quotes, `''` becomes `'`; comments dropped). Load it in
  Settings → Worktrees and check every project and command is there.
- [ ] **Step 2:** `herdr plugin disable freethinkel.worktree-hooks` (it fails on every event anyway: no `node` on
  herdr's PATH).
- [ ] **Step 3:** End to end: `herdr worktree create --branch hooks-smoke` in a configured repo; `herdr plugin log list --plugin shepherd.worktree-hooks`
  shows the commands ran and `.env` is linked in the new worktree; then `herdr worktree remove` it and delete the
  branch.
- [ ] **Step 4:** Tell the user their `~/.claude/CLAUDE.md` still names `freethinkel.worktree-hooks` (Subagents
  section) and offer to change it to `shepherd.worktree-hooks`.
