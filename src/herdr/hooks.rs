//! Worktree hooks: commands herdr runs in a new worktree of a project and after one is removed. They live in
//! `hooks.conf`, read by a small POSIX sh plugin (`assets/herdr-plugin`) that Shepherd writes into herdr's folder and
//! links, so they keep running with Shepherd closed or deleted, from any client.
use anyhow::{Context, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

pub const PLUGIN_ID: &str = "shepherd.worktree-hooks";
const MANIFEST: &str = include_str!("../../assets/herdr-plugin/herdr-plugin.toml");
const EVENT_SH: &str = include_str!("../../assets/herdr-plugin/event.sh");

/// One project of `hooks.conf`: its main checkout and the commands for its worktrees, one per line, run in order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Project {
    /// The herdr session whose worktrees it is for (`default`, a named one); `None` is every session's.
    pub session: Option<String>,
    pub name: String,
    pub dir: String,
    pub created: Vec<String>,
    pub removed: Vec<String>,
}

/// `hooks.conf` as event.sh reads it: `[session/name]` (or `[name]`, every session's) starts a project, then `dir = …`, `created = …`, `removed = …`, the value
/// verbatim after the `=` and its spaces. Comments, blank lines, unknown keys and keys before a `[name]` are skipped.
pub fn parse(text: &str) -> Vec<Project> {
    let mut projects: Vec<Project> = Vec::new();
    for line in text.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(section) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let (session, name) = match section.split_once('/') {
                Some((session, name)) => (Some(session.to_string()), name),
                None => (None, section),
            };
            projects.push(Project { session, name: name.to_string(), ..Project::default() });
            continue;
        }
        let (Some((key, value)), Some(project)) = (line.split_once('='), projects.last_mut()) else { continue };
        let value = value.trim_start_matches(' ').to_string();
        match key.trim_matches(' ') {
            "dir" => project.dir = value,
            "created" => project.created.push(value),
            "removed" => project.removed.push(value),
            _ => {}
        }
    }
    projects
}

pub fn render(projects: &[Project]) -> String {
    let mut text = String::from(
        "# Worktree hooks, one file for all projects. Shepherd → Settings → Worktrees edits it; so can you.\n\
         # created: run in a new worktree; removed: run from the main checkout. One command a line, in order; the\n\
         # first failure stops the rest. $PROJECT, $ROOT_PATH, $WORKTREE_PATH and $EVENT are set.\n",
    );
    for p in projects {
        let section = p.session.as_ref().map_or_else(|| p.name.clone(), |session| format!("{session}/{}", p.name));
        text.push_str(&format!("\n[{section}]\ndir = {}\n", p.dir));
        for c in &p.created {
            text.push_str(&format!("created = {c}\n"));
        }
        for c in &p.removed {
            text.push_str(&format!("removed = {c}\n"));
        }
    }
    text
}

/// `hooks.conf`, in the plugin's config dir.
pub fn conf_path() -> PathBuf {
    conf_in(&super::config::dir())
}

fn conf_in(herdr_dir: &Path) -> PathBuf {
    herdr_dir.join("plugins/config").join(PLUGIN_ID).join("hooks.conf")
}

fn root_in(herdr_dir: &Path) -> PathBuf {
    herdr_dir.join("plugins/local").join(PLUGIN_ID)
}

/// The projects in `hooks.conf`; none when there is no file yet.
pub fn load() -> anyhow::Result<Vec<Project>> {
    let path = conf_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(parse(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e).with_context(|| format!("cannot read {}", path.display())),
    }
}

/// Installs or updates the plugin, links it into herdr if it is not yet, and writes `projects` to `hooks.conf`.
pub fn save(projects: &[Project], stale: impl Fn() -> bool) -> anyhow::Result<()> {
    save_in(&super::config::dir(), &super::session::binary(), projects, &stale)
}

/// One save at a time, and only the latest: an older one let through after it would put back what it removed.
static SAVING: Mutex<()> = Mutex::new(());

fn save_in(herdr_dir: &Path, herdr: &str, projects: &[Project], stale: &dyn Fn() -> bool) -> anyhow::Result<()> {
    let _one = SAVING.lock().unwrap_or_else(|e| e.into_inner());
    if stale() {
        return Ok(());
    }
    let conf = conf_in(herdr_dir);
    let dir = conf.parent().expect("hooks.conf is in a folder");
    std::fs::create_dir_all(dir)?;
    // event.sh leaves worktrees older than this alone: they were there before the hooks
    let installed = dir.join(".installed");
    if !installed.exists() {
        std::fs::write(&installed, "")?;
    }
    // written whole and renamed into place, so event.sh never reads half of one
    let tmp = dir.join(".hooks.conf.tmp");
    std::fs::write(&tmp, render(&unique(projects)))?;
    std::fs::rename(&tmp, &conf).with_context(|| format!("cannot write {}", conf.display()))?;
    // after the file, so what was typed is kept when herdr cannot be run; the next save installs again
    install(herdr_dir, herdr)
}

/// Projects of one name in one session would be one to event.sh: the later ones get `-2`, `-3`.
fn unique(projects: &[Project]) -> Vec<Project> {
    let mut seen = std::collections::HashSet::new();
    projects
        .iter()
        .map(|p| {
            let mut p = p.clone();
            let base = p.name.clone();
            let mut n = 1;
            while !seen.insert((p.session.clone(), p.name.clone())) {
                n += 1;
                p.name = format!("{base}-{n}");
            }
            p
        })
        .collect()
}

/// Writes the plugin's files where they differ from this Shepherd's and links it into herdr: when herdr does not list
/// it, or again when its manifest changed, since herdr keeps the manifest's events from link time.
fn install(herdr_dir: &Path, herdr: &str) -> anyhow::Result<()> {
    let root = root_in(herdr_dir);
    std::fs::create_dir_all(&root)?;
    let manifest_changed = write_changed(&root.join("herdr-plugin.toml"), MANIFEST)?;
    write_changed(&root.join("event.sh"), EVENT_SH)?;
    let root = root.to_string_lossy();
    if !run(herdr, &["plugin", "list"])?.contains(&format!("- {PLUGIN_ID} ")) {
        run(herdr, &["plugin", "link", &root])?;
    } else if manifest_changed {
        run(herdr, &["plugin", "unlink", PLUGIN_ID])?;
        run(herdr, &["plugin", "link", &root])?;
    }
    Ok(())
}

/// Whether `path` had to be written.
fn write_changed(path: &Path, content: &str) -> anyhow::Result<bool> {
    if std::fs::read_to_string(path).is_ok_and(|old| old == content) {
        return Ok(false);
    }
    std::fs::write(path, content).with_context(|| format!("cannot write {}", path.display()))?;
    Ok(true)
}

/// herdr's output, or what it said when it failed.
fn run(herdr: &str, args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new(herdr).args(args).stdin(std::process::Stdio::null()).output().with_context(|| format!("cannot run {herdr}"))?;
    if !out.status.success() {
        return Err(anyhow!("herdr {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The start of a script run on a herdr machine: its herdr folder `$h`, binary `$herdr` (installed in ~/.local/bin,
/// which a non-interactive SSH shell may not have on PATH), the plugin's config `$c` and its files `$r`.
const REMOTE: &str = r#"h=${XDG_CONFIG_HOME:-$HOME/.config}/herdr
herdr=$(command -v herdr || echo "$HOME/.local/bin/herdr")
c=$h/plugins/config/shepherd.worktree-hooks
r=$h/plugins/local/shepherd.worktree-hooks
"#;

/// Splits what [`remote_load`] prints: its `hooks.conf`, then herdr's workspace list.
const SPLIT: &str = "\n\u{1}\n";

/// A saved herdr machine's projects (from its `hooks.conf`) and herdr's workspace list of its session `session`
/// (`Value::Null` while it does not run), in one SSH call.
pub fn remote_load(target: &str, session: &str) -> anyhow::Result<(Vec<Project>, serde_json::Value)> {
    let out = super::ssh(target, &load_script(session)).with_context(|| "cannot run ssh")?;
    anyhow::ensure!(out.status.success(), "{target}: {}", String::from_utf8_lossy(&out.stderr).trim());
    loaded(&String::from_utf8_lossy(&out.stdout)).ok_or_else(|| anyhow!("{target} answered something else"))
}

fn loaded(out: &str) -> Option<(Vec<Project>, serde_json::Value)> {
    let (conf, list) = out.split_once(SPLIT)?;
    Some((parse(conf), serde_json::from_str(list.trim()).unwrap_or_default()))
}

fn load_script(session: &str) -> String {
    format!(
        "{REMOTE}cat \"$c/hooks.conf\" 2>/dev/null\nprintf '\\n\\001\\n'\n\
         case {s} in default) k=$h/herdr.sock ;; *) k=$h/sessions/{s}/herdr.sock ;; esac\n\
         HERDR_SOCKET_PATH=$k \"$herdr\" workspace list 2>/dev/null || :\n",
        s = super::quote(session)
    )
}

/// Writes `projects` to the `hooks.conf` of the herdr machine at `target`, with the plugin's files, and links the
/// plugin into its herdr as [`save`] does here; herdr runs the commands there, with or without Shepherd.
pub fn save_remote(target: &str, projects: &[Project], stale: impl Fn() -> bool) -> anyhow::Result<()> {
    let _one = SAVING.lock().unwrap_or_else(|e| e.into_inner());
    if stale() {
        return Ok(());
    }
    let out = super::ssh(target, &save_script(projects)).with_context(|| "cannot run ssh")?;
    anyhow::ensure!(out.status.success(), "{target}: {}", String::from_utf8_lossy(&out.stderr).trim());
    Ok(())
}

// ponytail: the files go in the command line, which Linux caps at 128 KiB an argument; stdin if hooks.conf ever nears it
fn save_script(projects: &[Project]) -> String {
    let q = super::quote;
    format!(
        "set -e\n{REMOTE}mkdir -p \"$c\" \"$r\"\n\
         [ -e \"$c/.installed\" ] || : > \"$c/.installed\"\n\
         printf %s {conf} > \"$c/.hooks.conf.tmp\"\nmv \"$c/.hooks.conf.tmp\" \"$c/hooks.conf\"\n\
         old=$(cat \"$r/herdr-plugin.toml\" 2>/dev/null || :)\n\
         printf %s {manifest} > \"$r/herdr-plugin.toml\"\nprintf %s {event} > \"$r/event.sh\"\n\
         if ! \"$herdr\" plugin list | grep -qF -- {listed}; then \"$herdr\" plugin link \"$r\" >/dev/null\n\
         elif [ \"$old\" != \"$(cat \"$r/herdr-plugin.toml\")\" ]; then \"$herdr\" plugin unlink {PLUGIN_ID} >/dev/null; \"$herdr\" plugin link \"$r\" >/dev/null\n\
         fi\n",
        conf = q(&render(&unique(projects))),
        manifest = q(MANIFEST),
        event = q(EVENT_SH),
        listed = q(&format!("- {PLUGIN_ID} ")),
    )
}

/// At start: a plugin installed by an older Shepherd gets this one's files. Never installs one.
pub fn refresh() -> anyhow::Result<()> {
    refresh_in(&super::config::dir(), &super::session::binary())
}

fn refresh_in(herdr_dir: &Path, herdr: &str) -> anyhow::Result<()> {
    if !root_in(herdr_dir).exists() {
        return Ok(());
    }
    install(herdr_dir, herdr)
}

/// `~/…` for a path in the home folder, as hooks.conf is written by hand.
pub fn tilde(path: &str) -> String {
    match Path::new(path).strip_prefix(crate::settings::home()) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.to_string(),
    }
}

const HELP: &str = "\
shepherd hooks: the commands herdr runs in a new worktree and after one is removed (Settings → Worktrees)

  shepherd hooks                 print hooks.conf: every project and its commands
  shepherd hooks set [options]   set a project's commands; each of --created, --removed given replaces that list
  shepherd hooks rm [options]    remove a project
  shepherd hooks path            where hooks.conf is

  --created CMD    run in each new worktree, from it; repeat for more, run in order
  --removed CMD    run after a worktree is removed, from the main checkout; repeat for more
  --dir PATH       the project's main checkout (default: that of the git repo here)
  --name NAME      the project's name (default: the main checkout's folder name)
  --session NAME   the herdr session (default: this pane's, else default; on a machine, the machine's)
  --machine NAME   a saved herdr machine (herdr machine list): its hooks, over SSH; --dir is then needed

Commands run with sh -c and $ROOT_PATH (the main checkout), $WORKTREE_PATH (the worktree), $PROJECT and $EVENT
(created or removed) set; the first that fails stops the rest. herdr runs them, with or without Shepherd.
";

/// `shepherd hooks …`, for an agent asked to set the worktree hooks up: the exit code.
pub fn cli(args: &[String]) -> i32 {
    match cli_run(args) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("shepherd hooks: {e:#}");
            1
        }
    }
}

fn cli_run(args: &[String]) -> anyhow::Result<()> {
    // options alone, as `hooks --machine m`, list
    let (command, options) = match args.split_first() {
        Some((c, rest)) if !c.starts_with('-') => (c.as_str(), rest),
        _ => ("list", args),
    };
    let (mut created, mut removed, mut dir, mut name, mut session, mut machine) = (None, None, None, None, None, None);
    let mut options = options.iter();
    while let Some(flag) = options.next() {
        let mut value = || options.next().cloned().ok_or_else(|| anyhow!("{flag} needs a value"));
        match flag.as_str() {
            "--created" => created.get_or_insert_with(Vec::new).push(value()?),
            "--removed" => removed.get_or_insert_with(Vec::new).push(value()?),
            "--dir" => dir = Some(value()?),
            "--name" => name = Some(value()?),
            "--session" => session = Some(value()?),
            "--machine" => machine = Some(value()?),
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(());
            }
            _ => return Err(anyhow!("unknown option {flag}\n\n{HELP}")),
        }
    }
    let machine = match machine {
        Some(m) => Some(super::machines()?.into_iter().find(|x| [&x.target, &x.label, &x.id].contains(&&m)).ok_or_else(|| anyhow!("no herdr machine {m}: see herdr machine list"))?),
        None => None,
    };
    let load = || match &machine {
        Some(m) => remote_load(&m.target, &m.session).map(|(projects, _)| projects),
        None => load(),
    };
    let save = |projects: &[Project]| match &machine {
        Some(m) => save_remote(&m.target, projects, || false),
        None => save(projects, || false),
    };
    match command {
        "list" => print!("{}", render(&load()?)),
        "path" => println!("{}", conf_path().display()),
        "-h" | "--help" | "help" => print!("{HELP}"),
        "set" | "rm" => {
            let dir = match (dir, &machine) {
                // a machine's path is its own: taken as given
                (Some(dir), Some(_)) => PathBuf::from(dir.trim_end_matches('/')),
                (None, Some(m)) => return Err(anyhow!("give --dir: the project's main checkout on {}", m.target)),
                (Some(dir), None) => std::fs::canonicalize(&dir).with_context(|| format!("no folder {dir}"))?,
                (None, None) => main_checkout()?,
            };
            let name = name.unwrap_or_else(|| dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
            let session = session.or_else(|| machine.as_ref().map(|m| m.session.clone())).or_else(super::current_session).unwrap_or_else(|| "default".into());
            let dir = if machine.is_some() { dir.to_string_lossy().into_owned() } else { tilde(&dir.to_string_lossy()) };
            let mut projects = load()?;
            if command == "rm" {
                let at = find(&projects, &session, &name, &dir).ok_or_else(|| anyhow!("no project {name} ({dir}) in session {session}"))?;
                projects.remove(at);
            } else {
                anyhow::ensure!(created.is_some() || removed.is_some(), "nothing to set: give --created or --removed\n\n{HELP}");
                set(&mut projects, &session, &name, &dir, created, removed);
            }
            save(&projects)?;
            print!("{}", render(&load()?));
        }
        _ => return Err(anyhow!("unknown command {command}\n\n{HELP}")),
    }
    Ok(())
}

/// The main checkout of the git repo the current folder is in, also from a linked worktree of it.
fn main_checkout() -> anyhow::Result<PathBuf> {
    let out = Command::new("git").args(["rev-parse", "--path-format=absolute", "--git-common-dir"]).output().context("cannot run git")?;
    anyhow::ensure!(out.status.success(), "not in a git repo: give --dir");
    let git_dir = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    Ok(git_dir.parent().map(Path::to_path_buf).unwrap_or(git_dir))
}

/// The project of `session` for the checkout `dir`, or named `name`; a project of every session counts as default's.
fn find(projects: &[Project], session: &str, name: &str, dir: &str) -> Option<usize> {
    let of_session = |p: &Project| p.session.as_deref().unwrap_or("default") == session;
    projects.iter().position(|p| of_session(p) && p.dir == dir).or_else(|| projects.iter().position(|p| of_session(p) && p.name == name))
}

fn set(projects: &mut Vec<Project>, session: &str, name: &str, dir: &str, created: Option<Vec<String>>, removed: Option<Vec<String>>) {
    let at = find(projects, session, name, dir).unwrap_or_else(|| {
        projects.push(Project { name: name.into(), dir: dir.into(), ..Project::default() });
        projects.len() - 1
    });
    let p = &mut projects[at];
    p.session = Some(session.into());
    if let Some(created) = created {
        p.created = created;
    }
    if let Some(removed) = removed {
        p.removed = removed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str, dir: &str, created: &[&str], removed: &[&str]) -> Project {
        let lines = |l: &[&str]| l.iter().map(|s| s.to_string()).collect();
        Project { session: None, name: name.into(), dir: dir.into(), created: lines(created), removed: lines(removed) }
    }

    /// `script` as a machine runs it from [`super::super::ssh`] (`sh -c '…'` in its login shell), with `home` as
    /// `$HOME` and a stub herdr that logs its arguments and lists `listed` plugins.
    #[cfg(unix)]
    fn run_remote(script: &str, home: &Path, listed: &str) -> String {
        let bin = home.join(".local/bin");
        std::fs::create_dir_all(&bin).unwrap();
        let stub = r#"#!/bin/sh
echo "$*" >> "$HOME/herdr.log"
case $1$2 in
pluginlist) printf '%s\n' LISTED ;;
workspacelist) printf '{"result":{"socket":"%s"}}\n' "$HERDR_SOCKET_PATH" ;;
esac
"#
        .replace("LISTED", &super::super::quote(listed));
        std::fs::write(bin.join("herdr"), stub).unwrap();
        std::fs::set_permissions(bin.join("herdr"), std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("sh -c {}", super::super::quote(script)))
            .env("HOME", home)
            .env_remove("XDG_CONFIG_HOME")
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    #[cfg(unix)]
    #[test]
    fn a_machine_gets_the_files_and_reads_back_its_projects() {
        let home = tempfile::tempdir().unwrap();
        let projects = vec![Project { session: Some("work".into()), ..project("app", "/srv/app's", &[r#"ln -sf "$ROOT_PATH/.env" .env"#], &["echo 'bye'"]) }];
        run_remote(&save_script(&projects), home.path(), "");
        let h = home.path().join(".config/herdr");
        assert_eq!(std::fs::read_to_string(root_in(&h).join("event.sh")).unwrap(), EVENT_SH);
        assert_eq!(std::fs::read_to_string(root_in(&h).join("herdr-plugin.toml")).unwrap(), MANIFEST);
        assert!(conf_in(&h).with_file_name(".installed").exists());
        let log = std::fs::read_to_string(home.path().join("herdr.log")).unwrap();
        assert_eq!(log, format!("plugin list\nplugin link {}\n", root_in(&h).display()));
        // linked already, same manifest: nothing to do
        std::fs::remove_file(home.path().join("herdr.log")).unwrap();
        run_remote(&save_script(&projects), home.path(), &format!("- {PLUGIN_ID} (local)"));
        assert_eq!(std::fs::read_to_string(home.path().join("herdr.log")).unwrap(), "plugin list\n");

        let (back, list) = loaded(&run_remote(&load_script("work"), home.path(), "")).unwrap();
        assert_eq!(back, projects);
        assert_eq!(list["result"]["socket"], h.join("sessions/work/herdr.sock").display().to_string());
        let (_, list) = loaded(&run_remote(&load_script("default"), home.path(), "")).unwrap();
        assert_eq!(list["result"]["socket"], h.join("herdr.sock").display().to_string());
    }

    #[test]
    fn set_replaces_the_lists_given_and_adds_a_new_project() {
        let mut projects = vec![project("app", "~/app", &["a"], &["b"])];
        set(&mut projects, "default", "app", "~/app", Some(vec!["c".into()]), None);
        assert_eq!((projects[0].session.as_deref(), &projects[0].created, &projects[0].removed), (Some("default"), &vec!["c".to_string()], &vec!["b".to_string()]));
        set(&mut projects, "demo", "app", "~/app", None, Some(vec!["d".into()]));
        assert_eq!(projects.len(), 2);
        assert_eq!((projects[1].session.as_deref(), projects[1].created.len(), &projects[1].removed), (Some("demo"), 0, &vec!["d".to_string()]));
        assert_eq!(find(&projects, "default", "other", "~/app"), Some(0));
    }

    #[test]
    fn a_session_is_the_section_prefix() {
        let p = parse("[demo/app]\ndir = /a\n[app]\ndir = /b\n");
        assert_eq!((p[0].session.as_deref(), p[0].name.as_str()), (Some("demo"), "app"));
        assert_eq!((p[1].session.as_deref(), p[1].name.as_str()), (None, "app"));
        assert_eq!(parse(&render(&p)), p);
        assert!(render(&p).contains("\n[demo/app]\n"));
    }

    #[test]
    fn one_name_in_two_sessions_stays() {
        let mut a = project("app", "/a", &[], &[]);
        a.session = Some("default".into());
        let mut b = a.clone();
        b.session = Some("demo".into());
        let names: Vec<String> = unique(&[a, b]).into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["app", "app"]);
    }

    #[test]
    fn roundtrip_keeps_commands_verbatim() {
        let projects = vec![
            project(
                "callcenter",
                "~/Developer/sftpro/callcenter-frontend",
                &[
                    r#"ln -sf "$ROOT_PATH/.env" "$WORKTREE_PATH/.env""#,
                    r#"cp "$ROOT_PATH/CLAUDE.local.md" "$WORKTREE_PATH/CLAUDE.local.md" 2>/dev/null || true"#,
                    r#"[ "$(git rev-parse HEAD:package-lock.json)" = "$(git -C "$ROOT_PATH" rev-parse HEAD:package-lock.json)" ] || npm install --prefer-offline"#,
                ],
                &[],
            ),
            project(
                "comma",
                "~/Developer/apps/comma/comma",
                &[r#"[ -d "$ROOT_PATH/build" ] && { cp -Rc "$ROOT_PATH/build" "$WORKTREE_PATH/build" 2>/dev/null || cp -R "$ROOT_PATH/build" "$WORKTREE_PATH/build"; } || true"#],
                &["docker compose -p \"$(basename \"$WORKTREE_PATH\")\" down -v", "echo a=b"],
            ),
        ];
        assert_eq!(parse(&render(&projects)), projects);
    }

    #[test]
    fn parse_reads_spec_example() {
        let text = r#"# Worktree hooks
[callcenter]
dir = ~/Developer/sftpro/callcenter-frontend
created = ln -sf "$ROOT_PATH/.env" "$WORKTREE_PATH/.env"
created = cp -Rc "$ROOT_PATH/node_modules" node_modules 2>/dev/null || cp -R "$ROOT_PATH/node_modules" node_modules
removed = docker compose -p "$(basename "$WORKTREE_PATH")" down -v
"#;
        let p = parse(text);
        assert_eq!(p.len(), 1);
        assert_eq!((p[0].name.as_str(), p[0].dir.as_str()), ("callcenter", "~/Developer/sftpro/callcenter-frontend"));
        assert_eq!(p[0].created.len(), 2);
        assert_eq!(p[0].removed, vec![r#"docker compose -p "$(basename "$WORKTREE_PATH")" down -v"#.to_string()]);
    }

    #[test]
    fn parse_tolerates_junk() {
        let text = "dir = /outside\r\n\r\n# comment\r\n[app]\r\nnonsense\r\nunknown = 1\r\ndir=/a\r\ncreated =   make\r\n";
        assert_eq!(parse(text), vec![project("app", "/a", &["make"], &[])]);
    }

    #[test]
    fn render_starts_with_header_comment() {
        assert!(render(&[project("a", "/a", &[], &[])]).starts_with('#'));
    }

    /// A herdr that logs its arguments and lists the plugin once it was linked.
    #[cfg(unix)]
    fn stub_herdr(dir: &Path) -> String {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("herdr");
        let log = dir.join("calls.log");
        let script = format!(
            "#!/bin/sh\necho \"$*\" >> '{log}'\ncase \"$1 $2\" in \"plugin list\") grep -q '^plugin link' '{log}' && echo '- {PLUGIN_ID} (x) enabled' ;; esac\nexit 0\n",
            log = log.display()
        );
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path.display().to_string()
    }

    #[cfg(unix)]
    #[test]
    fn save_writes_files_and_conf() {
        let dir = tempfile::tempdir().unwrap();
        let herdr = stub_herdr(dir.path());
        let projects = vec![project("app", "/a", &["make"], &[])];
        save_in(dir.path(), &herdr, &projects, &|| false).unwrap();
        let root = root_in(dir.path());
        assert_eq!(std::fs::read_to_string(root.join("event.sh")).unwrap(), EVENT_SH);
        assert_eq!(std::fs::read_to_string(root.join("herdr-plugin.toml")).unwrap(), MANIFEST);
        assert_eq!(parse(&std::fs::read_to_string(conf_in(dir.path())).unwrap()), projects);
        let calls = || std::fs::read_to_string(dir.path().join("calls.log")).unwrap();
        assert!(calls().contains(&format!("plugin link {}", root.display())), "{}", calls());
        save_in(dir.path(), &herdr, &projects, &|| false).unwrap();
        assert_eq!(calls().matches("plugin link").count(), 1, "linked again: {}", calls());
    }

    #[cfg(unix)]
    #[test]
    fn a_changed_manifest_is_linked_again() {
        let dir = tempfile::tempdir().unwrap();
        let herdr = stub_herdr(dir.path());
        save_in(dir.path(), &herdr, &[], &|| false).unwrap();
        std::fs::write(root_in(dir.path()).join("herdr-plugin.toml"), "old").unwrap();
        refresh_in(dir.path(), &herdr).unwrap();
        let calls = std::fs::read_to_string(dir.path().join("calls.log")).unwrap();
        assert!(calls.contains(&format!("plugin unlink {PLUGIN_ID}")), "{calls}");
        assert_eq!(calls.matches("plugin link").count(), 2, "{calls}");
    }

    #[cfg(unix)]
    #[test]
    fn save_marks_the_install_once() {
        let dir = tempfile::tempdir().unwrap();
        let herdr = stub_herdr(dir.path());
        let marker = conf_in(dir.path()).with_file_name(".installed");
        save_in(dir.path(), &herdr, &[], &|| false).unwrap();
        let first = std::fs::metadata(&marker).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        save_in(dir.path(), &herdr, &[], &|| false).unwrap();
        assert_eq!(std::fs::metadata(&marker).unwrap().modified().unwrap(), first);
    }

    #[cfg(unix)]
    #[test]
    fn same_names_are_made_unique() {
        let dir = tempfile::tempdir().unwrap();
        let herdr = stub_herdr(dir.path());
        let projects = [project("frontend", "/a/frontend", &[], &[]), project("frontend", "/b/frontend", &[], &[])];
        save_in(dir.path(), &herdr, &projects, &|| false).unwrap();
        let names: Vec<String> = parse(&std::fs::read_to_string(conf_in(dir.path())).unwrap()).into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["frontend", "frontend-2"]);
    }

    #[test]
    fn a_stale_save_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        save_in(dir.path(), "/nonexistent/herdr", &[project("a", "/a", &[], &[])], &|| true).unwrap();
        assert!(!conf_in(dir.path()).exists());
    }

    #[test]
    fn hooks_are_kept_when_herdr_fails() {
        let dir = tempfile::tempdir().unwrap();
        let projects = vec![project("a", "/a", &["make"], &[])];
        assert!(save_in(dir.path(), "/nonexistent/herdr", &projects, &|| false).is_err());
        assert_eq!(parse(&std::fs::read_to_string(conf_in(dir.path())).unwrap()), projects);
    }

    #[test]
    fn refresh_never_installs() {
        let dir = tempfile::tempdir().unwrap();
        refresh_in(dir.path(), "/nonexistent/herdr").unwrap();
        assert!(!root_in(dir.path()).exists());
    }
}
