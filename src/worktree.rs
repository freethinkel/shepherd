//! Worktrees for a backend with no word for them: herdr's `worktree.*` done with git, in herdr's folder, and the
//! commands of `hooks.conf`, which herdr's plugin runs on herdr and nothing else would. Blocking: run it off the async
//! threads.
use crate::herdr::hooks::{self, Project};
use crate::model::Worktree;
use anyhow::{Context, bail};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// `<repo>/<branch slug>` go under it in the home folder.
// ponytail: herdr's default; its `[worktrees] directory` isn't read, so a moved folder isn't followed here
const FOLDER: &str = ".herdr/worktrees";

fn git(dir: &str, args: &[&str]) -> anyhow::Result<String> {
    let out = crate::herdr::git(dir, args).context("cannot run git")?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

/// The checkout `dir` is in. `None`: it is not in a repo.
pub fn of(dir: &str) -> Option<Worktree> {
    let out = git(dir, &["rev-parse", "--path-format=absolute", "--git-common-dir", "--git-dir", "--show-toplevel"]).ok()?;
    let [common, own, top] = out.lines().collect::<Vec<_>>()[..] else { return None };
    Some(Worktree { repo_key: common.into(), is_linked_worktree: common != own, checkout_path: top.into() })
}

/// The main checkout of `tree`'s repo.
fn root(tree: &Worktree) -> &str {
    tree.repo_key.strip_suffix("/.git").unwrap_or(&tree.repo_key)
}

/// Checks `branch` out in a new folder, as a new branch from `source`'s HEAD if there is none: the folder.
pub fn create(source: &Worktree, branch: &str) -> anyhow::Result<String> {
    let repo = Path::new(root(source)).file_name().context("a repo at the root of the disk")?;
    let path = crate::settings::home().join(FOLDER).join(repo).join(crate::commands::branch_path_slug(branch));
    add(&source.checkout_path, branch, &path)
}

fn add(checkout: &str, branch: &str, path: &Path) -> anyhow::Result<String> {
    let path = path.to_string_lossy();
    let known = git(checkout, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).is_ok();
    let args: &[&str] = if known { &["worktree", "add", &path, branch] } else { &["worktree", "add", "-b", branch, &path] };
    git(checkout, args)?;
    Ok(path.into_owned())
}

/// `worktree.list`'s result for the repo `dir` is in; `open` names the workspace on a checkout.
pub fn list(dir: &str, open: impl Fn(&str) -> Option<String>) -> anyhow::Result<Value> {
    let listed = git(dir, &["worktree", "list", "--porcelain"])?;
    let entries = listed.split("\n\n").enumerate().filter_map(|(at, entry)| {
        let field = |key: &str| entry.lines().find_map(|l| l.strip_prefix(key));
        let path = field("worktree ")?;
        Some(json!({
            "path": path, "branch": field("branch refs/heads/"), "open_workspace_id": open(path), "is_linked_worktree": at > 0,
            "is_bare": field("bare").is_some(), "is_prunable": field("prunable").is_some(),
        }))
    });
    Ok(json!({"worktrees": entries.collect::<Vec<_>>()}))
}

/// Deletes the checkout. Git refuses one with changes, and that is herdr's word for asking again with `force`.
// ponytail: it refuses a repo with submodules the same way, so a clean checkout of one asks twice
pub fn remove(tree: &Worktree, force: bool) -> anyhow::Result<()> {
    let path = tree.checkout_path.as_str();
    let args: &[&str] = if force { &["worktree", "remove", "--force", path] } else { &["worktree", "remove", path] };
    match git(&tree.repo_key, args) {
        Err(e) if !force && e.to_string().contains("--force") => bail!("dirty_worktree_requires_force: {e}"),
        done => done.map(drop),
    }
}

/// Runs the repo's `created` commands in the new checkout, which opens without waiting for them, as on herdr.
pub fn created(source: &Worktree, path: &str) {
    fire("created", root(source).to_string(), path.to_string());
}

/// Runs the repo's `removed` commands from its main checkout.
pub fn removed(tree: &Worktree) {
    fire("removed", root(tree).to_string(), tree.checkout_path.clone());
}

fn fire(event: &'static str, root: String, worktree: String) {
    std::thread::spawn(move || {
        if let Err(e) = run_hooks(event, &root, &worktree) {
            eprintln!("shepherd: worktree hooks: {e:#}");
        }
    });
}

fn run_hooks(event: &str, root: &str, worktree: &str) -> anyhow::Result<()> {
    let Some(project) = project(hooks::load()?, root) else { return Ok(()) };
    let (commands, cwd) = if event == "created" { (&project.created, worktree) } else { (&project.removed, root) };
    if commands.is_empty() {
        return Ok(());
    }
    let mut env = vec![("PROJECT", project.name.clone()), ("ROOT_PATH", root.into()), ("WORKTREE_PATH", worktree.into()), ("EVENT", event.into())];
    env.extend(login_path().map(|path| ("PATH", path)));
    run(commands, cwd, &env)
}

/// The project of the repo at `root`, in whichever session of `hooks.conf`: a core has no sessions.
fn project(projects: Vec<Project>, root: &str) -> Option<Project> {
    let root = std::fs::canonicalize(root).ok()?;
    projects.into_iter().find(|p| std::fs::canonicalize(expanded(&p.dir)).is_ok_and(|dir| dir == root))
}

fn expanded(dir: &str) -> PathBuf {
    dir.strip_prefix("~/").map_or_else(|| dir.into(), |rest| crate::settings::home().join(rest))
}

/// The user's PATH: an app opened from the Dock has launchd's bare one, and hooks want Homebrew's, nvm's, pnpm's.
/// An interactive login shell may print other things first, hence the marker.
fn login_path() -> Option<String> {
    const MARK: &str = "__SHEPHERD_PATH__=";
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let script = format!("printf '\\n{MARK}%s\\n' \"$PATH\"");
    let out = crate::process::command(shell).args(["-lic", &script]).stdin(Stdio::null()).output().ok()?;
    String::from_utf8_lossy(&out.stdout).lines().rev().find_map(|l| l.strip_prefix(MARK)).map(str::to_string)
}

/// Runs `commands` in `cwd`, stopping at the first failure.
fn run(commands: &[String], cwd: &str, env: &[(&str, String)]) -> anyhow::Result<()> {
    for line in commands {
        let mut command = crate::process::shell(line);
        let out = command.current_dir(cwd).envs(env.iter().map(|(k, v)| (*k, v))).stdin(Stdio::null()).output().with_context(|| format!("cannot run {line}"))?;
        if !out.status.success() {
            bail!("{line}: {}: {}", out.status, String::from_utf8_lossy(&out.stderr).trim());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn repo() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap().join("app");
        std::fs::create_dir(&root).unwrap();
        let root = root.to_string_lossy().into_owned();
        for args in [&["init", "-q"][..], &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "first"]] {
            assert!(Command::new("git").arg("-C").arg(&root).args(args).status().unwrap().success());
        }
        (dir, root)
    }

    #[test]
    fn a_worktree_is_made_listed_and_removed() {
        let (dir, root) = repo();
        let main = of(&root).unwrap();
        assert!(!main.is_linked_worktree);
        assert_eq!(super::root(&main), root);
        assert!(of(&dir.path().to_string_lossy()).is_none(), "outside git");

        let path = dir.path().join("wt/fix-login");
        let made = add(&root, "fix/login", &path).unwrap();
        let linked = of(&made).unwrap();
        assert!(linked.is_linked_worktree);
        assert_eq!(linked.repo_key, main.repo_key, "the two group as one repo");

        let listed = list(&root, |p| (p == linked.checkout_path).then(|| "ws2".to_string())).unwrap();
        let w = listed["worktrees"].as_array().unwrap();
        assert_eq!((w[0]["path"].as_str(), w[0]["is_linked_worktree"].as_bool()), (Some(root.as_str()), Some(false)));
        assert_eq!((w[1]["branch"].as_str(), w[1]["open_workspace_id"].as_str()), (Some("fix/login"), Some("ws2")));

        std::fs::write(path.join("new"), "work").unwrap();
        let refused = remove(&linked, false).unwrap_err().to_string();
        assert!(refused.starts_with("dirty_worktree_requires_force"), "{refused}");
        assert!(path.join("new").exists(), "nothing was lost");
        remove(&linked, true).unwrap();
        assert!(!path.exists());

        let again = add(&root, "fix/login", &path).unwrap();
        assert_eq!(of(&again).unwrap().repo_key, main.repo_key, "the branch stayed and checks out again");
    }

    #[test]
    fn a_clean_worktree_goes_without_asking() {
        let (dir, root) = repo();
        let made = add(&root, "tidy", &dir.path().join("tidy")).unwrap();
        remove(&of(&made).unwrap(), false).unwrap();
        assert!(!Path::new(&made).exists());
    }

    #[test]
    fn hooks_stop_at_the_first_failure_and_see_their_worktree() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = dir.path().to_string_lossy();
        let commands = ["printf %s \"$PROJECT $EVENT\" > ran".to_string(), "echo no >&2; exit 3".into(), "touch never".into()];
        let env = [("PROJECT", "app".to_string()), ("EVENT", "created".to_string())];
        let failed = run(&commands, &cwd, &env).unwrap_err().to_string();
        assert!(failed.contains("no"), "{failed}");
        assert_eq!(std::fs::read_to_string(dir.path().join("ran")).unwrap(), "app created");
        assert!(!dir.path().join("never").exists());
    }

    #[test]
    fn a_project_is_found_by_its_main_checkout() {
        let (_dir, root) = repo();
        let named = |name: &str, dir: &str, session: Option<&str>| Project { name: name.into(), dir: dir.into(), session: session.map(str::to_string), ..Project::default() };
        let projects = vec![named("other", "/nowhere", None), named("app", &root, Some("work"))];
        assert_eq!(project(projects.clone(), &root).unwrap().name, "app");
        assert!(project(projects, "/tmp").is_none());
    }
}
