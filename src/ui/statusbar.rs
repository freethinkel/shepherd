//! The window's bottom bar (Zeron's footer labels): the shown pane's branch, directory and agent, and how many
//! agents across the workspaces work or wait for you. Git runs off the main thread and is cached per directory.
use crate::model::{AgentStatus, Snapshot};
use crate::theme::Theme;
use gpui::{App, ClickEvent, Global, IntoElement, ParentElement, Styled, Window, div, prelude::*, px};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

pub const HEIGHT: f32 = 24.0;
/// How old a directory's git state may get before a render asks git again.
const GIT_TTL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Git {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub dirty: bool,
}

/// `git status --porcelain=v2 --branch`, read; `None` outside a repo (git printed nothing).
pub fn parse_git(out: &str) -> Option<Git> {
    let mut git = Git::default();
    let mut oid = None;
    for line in out.lines() {
        if let Some(head) = line.strip_prefix("# branch.head ") {
            git.branch = head.to_string();
        } else if let Some(o) = line.strip_prefix("# branch.oid ") {
            oid = Some(o.chars().take(7).collect::<String>());
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            let mut it = ab.split(' ').map(|n| n.trim_start_matches(['+', '-']).parse().unwrap_or(0));
            (git.ahead, git.behind) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
        } else if !line.starts_with('#') && !line.is_empty() {
            git.dirty = true;
        }
    }
    if git.branch.is_empty() {
        return None;
    }
    if git.branch == "(detached)" {
        git.branch = oid.unwrap_or(git.branch);
    }
    Some(git)
}

/// `path` with the home directory as `~`; deeper than three parts, only the last two after `…`.
pub fn short_path(path: &str, home: Option<&Path>) -> String {
    let (root, rest) = match home.and_then(|h| Path::new(path).strip_prefix(h).ok()) {
        Some(rest) => ("~", rest.to_string_lossy().into_owned()),
        None => ("", path.to_string()),
    };
    let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
    match (root, parts.len()) {
        ("~", 0) => "~".into(),
        (_, n) if n > 3 => format!("{root}/…/{}", parts[n - 2..].join("/")),
        _ => format!("{root}/{}", parts.join("/")),
    }
}

/// Agents working, blocked (waiting for input) and done, across every workspace.
pub fn agent_counts(snapshot: &Snapshot) -> (usize, usize, usize) {
    let count = |s| snapshot.panes.iter().filter(|p| p.agent.is_some() && p.agent_status == s).count();
    (count(AgentStatus::Working), count(AgentStatus::Blocked), count(AgentStatus::Done))
}

/// The tab of the blocked agent after `current` (cycling), or the first blocked one.
pub fn next_waiting<'a>(snapshot: &'a Snapshot, current: Option<&str>) -> Option<&'a str> {
    let waiting: Vec<_> = snapshot.agents().into_iter().filter(|p| p.agent_status == AgentStatus::Blocked).collect();
    let next = match waiting.iter().position(|p| Some(p.pane_id.as_str()) == current) {
        Some(i) => waiting.get((i + 1) % waiting.len()),
        None => waiting.first(),
    };
    next.map(|p| p.tab_id.as_str())
}

fn status_label(status: AgentStatus) -> &'static str {
    match status {
        AgentStatus::Working => "working",
        AgentStatus::Blocked => "needs input",
        AgentStatus::Done => "done",
        AgentStatus::Idle | AgentStatus::Unknown => "idle",
    }
}

#[derive(Default)]
struct GitCache(HashMap<String, GitEntry>);

#[derive(Default)]
struct GitEntry {
    git: Option<Git>,
    fetched: Option<Instant>,
    busy: bool,
}

impl Global for GitCache {}

// ponytail: one entry per directory ever shown, never evicted; prune if people cd through thousands of dirs.
/// The cached git state of `dir`; asks git in the background when it is missing or stale.
fn git_of(dir: &str, cx: &mut App) -> Option<Git> {
    let entry = cx.default_global::<GitCache>().0.entry(dir.to_string()).or_default();
    let git = entry.git.clone();
    if entry.busy || entry.fetched.is_some_and(|t| t.elapsed() < GIT_TTL) {
        return git;
    }
    entry.busy = true;
    let dir = dir.to_string();
    cx.spawn(async move |cx| {
        let path = dir.clone();
        let fresh = cx
            .background_spawn(async move {
                // --no-optional-locks: never take index.lock from under the agents' own git.
                let out = std::process::Command::new("git")
                    .args(["--no-optional-locks", "-C", &path, "status", "--porcelain=v2", "--branch"])
                    .output()
                    .ok()?;
                parse_git(&String::from_utf8_lossy(&out.stdout))
            })
            .await;
        let _ = cx.update(|cx| {
            let entry = cx.default_global::<GitCache>().0.entry(dir).or_default();
            let changed = entry.git != fresh;
            *entry = GitEntry { git: fresh, fetched: Some(Instant::now()), busy: false };
            if changed {
                cx.refresh_windows();
            }
        });
    })
    .detach();
    git
}

fn label(theme: &Theme, icon: &'static str) -> gpui::Div {
    div()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(6.))
        .px(px(8.))
        .child(gpui::svg().path(icon).size(px(12.)).flex_none().text_color(theme.text_muted.opacity(0.7)))
}

fn dot(theme: &Theme, status: AgentStatus) -> gpui::Div {
    let dot = div().size(px(6.)).rounded_full().flex_none();
    match crate::shell::status_color(status, theme) {
        Some(c) => dot.bg(c),
        None => dot.border_1().border_color(theme.text_faint.opacity(0.6)),
    }
}

/// The bar for the pane `pane_id` of `snapshot`. `on_waiting` runs on a click on the agents summary while
/// some agent waits for input.
pub fn bar(
    snapshot: Option<&Snapshot>,
    pane_id: Option<&str>,
    theme: &Theme,
    on_waiting: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &mut App,
) -> gpui::AnyElement {
    let pane = snapshot.zip(pane_id).and_then(|(s, id)| s.panes.iter().find(|p| p.pane_id == id));
    let dir = pane.and_then(|p| p.foreground_cwd.clone().or_else(|| p.cwd.clone())).filter(|d| !d.is_empty());
    let git = dir.as_deref().and_then(|d| git_of(d, cx));
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let (working, waiting, done) = snapshot.map(agent_counts).unwrap_or_default();
    let muted = theme.text_muted.opacity(0.7);
    div()
        .flex_none()
        .h(px(HEIGHT))
        .px(px(4.))
        .flex()
        .items_center()
        .overflow_hidden()
        .border_t_1()
        .border_color(theme.border)
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(muted)
        .children(git.map(|g| {
            label(theme, "icons/git-branch.svg")
                .flex_shrink_0()
                .max_w(px(220.))
                .child(div().min_w_0().truncate().child(g.branch))
                .when(g.dirty, |el| el.child(div().flex_none().text_color(theme.warning).child("●")))
                .when(g.ahead > 0, |el| el.child(div().flex_none().child(format!("↑{}", g.ahead))))
                .when(g.behind > 0, |el| el.child(div().flex_none().child(format!("↓{}", g.behind))))
        }))
        .children(dir.map(|d| {
            label(theme, "icons/folder.svg").child(div().min_w_0().truncate().child(short_path(&d, home.as_deref())))
        }))
        .children(pane.map(|p| match &p.agent {
            Some(agent) => label(theme, "icons/bot.svg")
                .flex_shrink_0()
                .child(agent.clone())
                .child(dot(theme, p.agent_status))
                .child(status_label(p.agent_status)),
            None => label(theme, "icons/terminal.svg")
                .child(div().min_w_0().truncate().child(p.terminal_title_stripped.clone().unwrap_or_default())),
        }))
        .child(div().flex_1())
        .when(working + waiting + done > 0, |el| {
            let count = |status, n: usize, what: &str| {
                (n > 0).then(|| div().flex().items_center().gap(px(5.)).child(dot(theme, status)).child(format!("{n} {what}")))
            };
            let hover = theme.hover;
            el.child(
                div()
                    .id("statusbar-agents")
                    .flex_none()
                    .h(px(20.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .rounded(px(6.))
                    .children(count(AgentStatus::Blocked, waiting, "waiting"))
                    .children(count(AgentStatus::Working, working, "working"))
                    .children(count(AgentStatus::Done, done, "done"))
                    .when(waiting > 0, |el| {
                        el.cursor_pointer().hover(move |s| s.bg(hover)).on_click(on_waiting)
                    }),
            )
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_tracking_and_changes() {
        let out = "# branch.oid 7d9be72aaaa\n# branch.head statusbar\n# branch.upstream origin/statusbar\n# branch.ab +2 -1\n1 .M N... 100644 100644 100644 a b src/x.rs\n";
        assert_eq!(parse_git(out), Some(Git { branch: "statusbar".into(), ahead: 2, behind: 1, dirty: true }));
        let clean = "# branch.oid 7d9be72aaaa\n# branch.head main\n";
        assert_eq!(parse_git(clean), Some(Git { branch: "main".into(), ..Git::default() }));
        let detached = "# branch.oid 7d9be72aaaa\n# branch.head (detached)\n? new.txt\n";
        assert_eq!(parse_git(detached), Some(Git { branch: "7d9be72".into(), dirty: true, ..Git::default() }));
        assert_eq!(parse_git(""), None);
    }

    #[test]
    fn shortens_paths_under_home() {
        let home = Some(Path::new("/Users/me"));
        assert_eq!(short_path("/Users/me", home), "~");
        assert_eq!(short_path("/Users/me/dev/shepherd", home), "~/dev/shepherd");
        assert_eq!(short_path("/Users/me/a/b/c", home), "~/a/b/c");
        assert_eq!(short_path("/Users/me/.herdr/worktrees/shepherd/statusbar", home), "~/…/shepherd/statusbar");
        assert_eq!(short_path("/Users/meow/x", home), "/Users/meow/x");
        assert_eq!(short_path("/opt/a/b/c/d", home), "/…/c/d");
        assert_eq!(short_path("/", None), "/");
    }

    #[test]
    fn counts_agents_and_cycles_through_the_waiting_ones() {
        let pane = |id: &str, tab: &str, status: &str, agent: bool| {
            serde_json::json!({"pane_id": id, "tab_id": tab, "workspace_id": "w", "terminal_id": "t",
                "agent_status": status, "agent": if agent { Some("claude") } else { None }})
        };
        let snapshot: Snapshot = serde_json::from_value(serde_json::json!({
            "focused_workspace_id": null, "focused_tab_id": null, "focused_pane_id": null,
            "workspaces": [], "tabs": [], "layouts": [],
            "panes": [pane("p1", "t1", "blocked", true), pane("p2", "t2", "working", true),
                      pane("p3", "t3", "blocked", true), pane("p4", "t4", "blocked", false)],
        }))
        .unwrap();
        assert_eq!(agent_counts(&snapshot), (1, 2, 0));
        assert_eq!(next_waiting(&snapshot, None), Some("t1"));
        assert_eq!(next_waiting(&snapshot, Some("p2")), Some("t1"));
        assert_eq!(next_waiting(&snapshot, Some("p1")), Some("t3"));
        assert_eq!(next_waiting(&snapshot, Some("p3")), Some("t1"));
    }
}
