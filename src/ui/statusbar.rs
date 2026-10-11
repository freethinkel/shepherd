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
/// Claude's usage endpoint is polled as often as the dotfiles' statusbar does.
const USAGE_TTL: Duration = Duration::from_secs(300);

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

/// Background lookups by key, each asked again once older than its TTL.
struct Cache<T>(HashMap<String, Entry<T>>);

struct Entry<T> {
    value: Option<T>,
    fetched: Option<Instant>,
    busy: bool,
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Cache(HashMap::new())
    }
}

impl<T: 'static> Global for Cache<T> {}

// ponytail: one entry per key ever shown, never evicted; prune if people cd through thousands of dirs.
/// The cached `fetch(key)`; runs it on a background thread when missing or older than `ttl`. A failed fetch
/// keeps the last value.
fn cached<T: Clone + PartialEq + Send + 'static>(key: &str, ttl: Duration, fetch: fn(&str) -> Option<T>, cx: &mut App) -> Option<T> {
    let entry = cx
        .default_global::<Cache<T>>()
        .0
        .entry(key.to_string())
        .or_insert(Entry { value: None, fetched: None, busy: false });
    let value = entry.value.clone();
    if entry.busy || entry.fetched.is_some_and(|t| t.elapsed() < ttl) {
        return value;
    }
    entry.busy = true;
    let key = key.to_string();
    cx.spawn(async move |cx| {
        let k = key.clone();
        let fresh = cx.background_spawn(async move { fetch(&k) }).await;
        let _ = cx.update(|cx| {
            let Some(entry) = cx.default_global::<Cache<T>>().0.get_mut(&key) else { return };
            (entry.fetched, entry.busy) = (Some(Instant::now()), false);
            if fresh.is_some() && entry.value != fresh {
                entry.value = fresh;
                cx.refresh_windows();
            }
        });
    })
    .detach();
    value
}

fn read_git(dir: &str) -> Option<Git> {
    // --no-optional-locks: never take index.lock from under the agents' own git.
    let out = std::process::Command::new("git")
        .args(["--no-optional-locks", "-C", dir, "status", "--porcelain=v2", "--branch"])
        .output()
        .ok()?;
    parse_git(&String::from_utf8_lossy(&out.stdout))
}

/// Claude's plan usage, in percent of each window.
#[derive(Debug, Clone, PartialEq)]
pub struct Usage {
    pub five_hour: f64,
    pub week: f64,
}

/// The body of `api/oauth/usage`.
pub fn parse_usage(json: &str) -> Option<Usage> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let used = |window: &str| v[window]["utilization"].as_f64();
    Some(Usage { five_hour: used("five_hour")?, week: used("seven_day")? })
}

/// The usage endpoint Claude Code itself reads, with the token Claude Code keeps (and refreshes) in the login
/// keychain, as the dotfiles' statusbar does.
fn read_usage(_: &str) -> Option<Usage> {
    let creds = std::process::Command::new("/usr/bin/security")
        .args(["find-generic-password", "-s", "Claude Code-credentials", "-w"])
        .output()
        .ok()?;
    let creds: serde_json::Value = serde_json::from_slice(&creds.stdout).ok()?;
    let token = creds["claudeAiOauth"]["accessToken"].as_str()?;
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).build().into();
    let mut response = agent
        .get("https://api.anthropic.com/api/oauth/usage")
        .header("Authorization", &format!("Bearer {token}"))
        .header("anthropic-beta", "oauth-2025-04-20")
        .call()
        .ok()?;
    parse_usage(&response.body_mut().read_to_string().ok()?)
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

/// A 12 px progress ring: `used` percent of a turn from 12 o'clock, clockwise, over a full faint track.
fn donut(used: f64, track: gpui::Hsla, color: gpui::Hsla) -> impl IntoElement {
    let fraction = (used / 100.).clamp(0., 1.) as f32;
    gpui::canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let d = f32::from(bounds.size.width.min(bounds.size.height));
            let (width, center) = ((d / 6.).max(2.), bounds.center());
            let r = (d - width) / 2.;
            // ponytail: a polyline arc with butt caps; lyon's round caps aren't re-exported by gpui
            let ring = |turn: f32| {
                let mut path = gpui::PathBuilder::stroke(px(width));
                let steps = (turn * 64.).ceil().max(1.) as usize;
                for i in 0..=steps {
                    let a = turn * std::f32::consts::TAU * i as f32 / steps as f32;
                    let at = gpui::point(center.x + px(r * a.sin()), center.y - px(r * a.cos()));
                    if i == 0 { path.move_to(at) } else { path.line_to(at) }
                }
                if turn >= 1. {
                    path.close();
                }
                path.build().ok()
            };
            if let Some(path) = ring(1.) {
                window.paint_path(path, track);
            }
            if let Some(path) = ring(fraction).filter(|_| fraction > 0.) {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(12.))
    .flex_none()
}

/// A clickable label: a wash on hover.
fn link(el: gpui::Stateful<gpui::Div>, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    let hover = theme.hover;
    el.h(px(20.)).rounded(px(6.)).cursor_pointer().hover(move |s| s.bg(hover))
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
    let git = dir.as_deref().and_then(|d| cached(d, GIT_TTL, read_git, cx));
    let usage = cached("", USAGE_TTL, read_usage, cx);
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
            let short = short_path(&d, home.as_deref());
            link(label(theme, "icons/folder.svg").id("statusbar-cwd"), theme)
                .child(div().min_w_0().truncate().child(short))
                .on_click(move |_, _, cx| cx.open_with_system(Path::new(&d)))
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
        .children(usage.map(|u| {
            // the dotfiles' statusbar: the ring fills with the 5-hour window, colored by the worse of the two
            let worst = u.five_hour.max(u.week);
            let color = if worst >= 90. { theme.danger } else if worst >= 70. { theme.warning } else { theme.accent };
            link(div().id("statusbar-usage").flex_none().flex().items_center().gap(px(6.)).px(px(8.)), theme)
                .child(donut(u.five_hour, theme.text.opacity(0.2), color))
                .child(format!("{}%", u.five_hour.round()))
                .on_click(|_, _, cx| cx.open_url("https://claude.ai/settings/usage"))
        }))
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
    fn parses_claude_usage() {
        let body = r#"{"five_hour":{"utilization":42.0,"resets_at":"2026-10-11T15:00:00.000Z"},"seven_day":{"utilization":18,"resets_at":null}}"#;
        assert_eq!(parse_usage(body), Some(Usage { five_hour: 42.0, week: 18.0 }));
        assert_eq!(parse_usage(r#"{"five_hour":{"utilization":null},"seven_day":{"utilization":1}}"#), None);
        assert_eq!(parse_usage(r#"{"error":{"type":"authentication_error"}}"#), None);
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
