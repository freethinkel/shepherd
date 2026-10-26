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
    pub name: String,
    pub dir: String,
    pub created: Vec<String>,
    pub removed: Vec<String>,
}

/// `hooks.conf` as event.sh reads it: `[name]` starts a project, then `dir = …`, `created = …`, `removed = …`, the value
/// verbatim after the `=` and its spaces. Comments, blank lines, unknown keys and keys before a `[name]` are skipped.
pub fn parse(text: &str) -> Vec<Project> {
    let mut projects: Vec<Project> = Vec::new();
    for line in text.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            projects.push(Project { name: name.to_string(), ..Project::default() });
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
         # first failure stops the rest. $PROJECT, $MAIN, $WORKTREE and $EVENT are set.\n",
    );
    for p in projects {
        text.push_str(&format!("\n[{}]\ndir = {}\n", p.name, p.dir));
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

fn save_in(herdr_dir: &Path, herdr: &str, projects: &[Project], stale: &dyn Fn() -> bool) -> anyhow::Result<()> {
    // one save at a time, and only the latest: an older one let through after it would put back what it removed
    static SAVING: Mutex<()> = Mutex::new(());
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

/// Projects of one name would be one to event.sh: the later ones get `-2`, `-3`.
fn unique(projects: &[Project]) -> Vec<Project> {
    let mut seen = std::collections::HashSet::new();
    projects
        .iter()
        .map(|p| {
            let mut p = p.clone();
            let base = p.name.clone();
            let mut n = 1;
            while !seen.insert(p.name.clone()) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str, dir: &str, created: &[&str], removed: &[&str]) -> Project {
        let lines = |l: &[&str]| l.iter().map(|s| s.to_string()).collect();
        Project { name: name.into(), dir: dir.into(), created: lines(created), removed: lines(removed) }
    }

    #[test]
    fn roundtrip_keeps_commands_verbatim() {
        let projects = vec![
            project(
                "callcenter",
                "~/Developer/sftpro/callcenter-frontend",
                &[
                    r#"ln -sf "$MAIN/.env" "$WORKTREE/.env""#,
                    r#"cp "$MAIN/CLAUDE.local.md" "$WORKTREE/CLAUDE.local.md" 2>/dev/null || true"#,
                    r#"[ "$(git rev-parse HEAD:package-lock.json)" = "$(git -C "$MAIN" rev-parse HEAD:package-lock.json)" ] || npm install --prefer-offline"#,
                ],
                &[],
            ),
            project(
                "comma",
                "~/Developer/apps/comma/comma",
                &[r#"[ -d "$MAIN/build" ] && { cp -Rc "$MAIN/build" "$WORKTREE/build" 2>/dev/null || cp -R "$MAIN/build" "$WORKTREE/build"; } || true"#],
                &["docker compose -p \"$(basename \"$WORKTREE\")\" down -v", "echo a=b"],
            ),
        ];
        assert_eq!(parse(&render(&projects)), projects);
    }

    #[test]
    fn parse_reads_spec_example() {
        let text = r#"# Worktree hooks
[callcenter]
dir = ~/Developer/sftpro/callcenter-frontend
created = ln -sf "$MAIN/.env" "$WORKTREE/.env"
created = cp -Rc "$MAIN/node_modules" node_modules 2>/dev/null || cp -R "$MAIN/node_modules" node_modules
removed = docker compose -p "$(basename "$WORKTREE")" down -v
"#;
        let p = parse(text);
        assert_eq!(p.len(), 1);
        assert_eq!((p[0].name.as_str(), p[0].dir.as_str()), ("callcenter", "~/Developer/sftpro/callcenter-frontend"));
        assert_eq!(p[0].created.len(), 2);
        assert_eq!(p[0].removed, vec![r#"docker compose -p "$(basename "$WORKTREE")" down -v"#.to_string()]);
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
