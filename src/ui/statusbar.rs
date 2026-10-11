//! The window's bottom bar (Zeron's footer labels): the shown pane's branch, directory and token spend, and
//! Claude's plan limits. Git runs off the main thread and is cached per directory.
use crate::model::{Pane, Snapshot};
use crate::theme::Theme;
use gpui::{App, ClickEvent, Global, IntoElement, ParentElement, SharedString, Styled, Window, div, prelude::*, px};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

pub const HEIGHT: f32 = 24.0;
/// How old a directory's git state may get before a render asks git again.
const GIT_TTL: Duration = Duration::from_secs(5);
/// Claude's usage endpoint is polled as often as the dotfiles' statusbar does.
const USAGE_TTL: Duration = Duration::from_secs(300);
/// How often the shown agent's transcript is summed again.
const SPENT_TTL: Duration = Duration::from_secs(10);
/// How often a sidebar worktree's diff against its base is asked again.
const CHANGES_TTL: Duration = Duration::from_secs(30);
/// How often gh is asked about a worktree's PR; it goes over the network.
const PR_TTL: Duration = Duration::from_secs(120);

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

/// The directory `pane_id` of `snapshot` shows: its foreground process's, else its shell's.
pub fn pane_dir(snapshot: Option<&Snapshot>, pane_id: Option<&str>) -> Option<String> {
    let pane = snapshot?.panes.iter().find(|p| Some(p.pane_id.as_str()) == pane_id)?;
    pane.foreground_cwd.clone().or_else(|| pane.cwd.clone()).filter(|d| !d.is_empty())
}

/// `dir`'s cached branch, if git has been asked already.
pub fn branch_of(dir: &str, cx: &mut App) -> Option<String> {
    cached(dir, GIT_TTL, read_git, cx).map(|g| g.branch)
}

/// Asks git about `dir` again on the next render, as after a checkout.
pub fn refresh_git(dir: &str, cx: &mut App) {
    if let Some(entry) = cx.default_global::<Cache<Git>>().0.get_mut(dir) {
        entry.fetched = None;
    }
    cx.refresh_windows();
}

/// `dir`'s local branches, newest commit first.
pub fn read_branches(dir: &str) -> String {
    std::process::Command::new("git")
        .args(["-C", dir, "for-each-ref", "--sort=-committerdate", "--format=%(refname:short)", "refs/heads"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// `git switch branch` in `dir`; git's complaint on failure.
pub fn switch_branch(dir: &str, branch: &str) -> Result<(), String> {
    let out = std::process::Command::new("git").args(["-C", dir, "switch", branch]).output().map_err(|e| e.to_string())?;
    match out.status.success() {
        true => Ok(()),
        false => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
    }
}

fn read_git(dir: &str) -> Option<Git> {
    // --no-optional-locks: never take index.lock from under the agents' own git.
    let out = std::process::Command::new("git")
        .args(["--no-optional-locks", "-C", dir, "status", "--porcelain=v2", "--branch"])
        .output()
        .ok()?;
    parse_git(&String::from_utf8_lossy(&out.stdout))
}

/// `git diff --shortstat`, read: lines added and removed; `(0, 0)` for no changes.
pub fn parse_shortstat(out: &str) -> (u32, u32) {
    let count = |word: &str| out.split(", ").find(|p| p.contains(word)).and_then(|p| p.trim().split(' ').next()?.parse().ok()).unwrap_or(0);
    (count("insertion"), count("deletion"))
}

// ponytail: untracked files don't count until they're added; `--intent-to-add` would touch the agents' index.
/// What the worktree at `dir` changed: lines added and removed from its base to its working tree, so commits, staged
/// and unstaged edits all count. The base is the newest commit it shares with any of `origin/HEAD`, its local twin
/// (`develop` for `origin/develop`), `main` and `master`: a local default ahead of origin and an origin that already
/// merged the branch both move it up. `None` without any of them.
fn read_changes(dir: &str) -> Option<(u32, u32)> {
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git").args(["--no-optional-locks", "-C", dir]).args(args).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    };
    let remote = git(&["symbolic-ref", "--short", "refs/remotes/origin/HEAD"]).unwrap_or_default();
    let local = format!("refs/heads/{}", remote.trim().strip_prefix("origin/").unwrap_or("main"));
    let refs = git(&["for-each-ref", "--format=%(objectname)", "refs/remotes/origin/HEAD", &local, "refs/heads/main", "refs/heads/master"])?;
    let base = git(&[&["merge-base", "HEAD"][..], &refs.lines().collect::<Vec<_>>()].concat())?;
    Some(parse_shortstat(&git(&["diff", "--shortstat", base.trim()])?))
}

/// `gh pr view --json state,isDraft`, read: `open`, `draft`, `merged` or `closed`.
pub fn parse_pr(json: &str) -> Option<&'static str> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    match (v["state"].as_str()?, v["isDraft"].as_bool().unwrap_or(false)) {
        ("OPEN", true) => Some("draft"),
        ("OPEN", false) => Some("open"),
        ("MERGED", _) => Some("merged"),
        ("CLOSED", _) => Some("closed"),
        _ => None,
    }
}

/// The state of the PR of `dir`'s branch; `None` without gh, its login, or a PR.
fn read_pr(dir: &str) -> Option<&'static str> {
    // An app opened from Finder gets launchd's PATH, without Homebrew's.
    let gh = ["/opt/homebrew/bin/gh", "/usr/local/bin/gh"].into_iter().find(|p| Path::new(p).is_file()).unwrap_or("gh");
    let out = std::process::Command::new(gh).args(["pr", "view", "--json", "state,isDraft"]).current_dir(dir).output().ok()?;
    parse_pr(&String::from_utf8_lossy(&out.stdout))
}

/// The worktree at `dir`'s cached changes and PR state, asked again after their TTLs while `active` (the window
/// is key); in the background only a first lookup runs.
pub fn changes_of(dir: &str, active: bool, cx: &mut App) -> (Option<(u32, u32)>, Option<&'static str>) {
    let (ttl, pr_ttl) = if active { (CHANGES_TTL, PR_TTL) } else { (Duration::MAX, Duration::MAX) };
    (cached(dir, ttl, read_changes, cx), cached(dir, pr_ttl, read_pr, cx))
}

/// Asks git about the worktree at `dir`'s changes again on the next render, as when its agent stops.
pub fn refresh_changes(dir: &str, cx: &mut App) {
    if let Some(entry) = cx.default_global::<Cache<(u32, u32)>>().0.get_mut(dir) {
        entry.fetched = None;
    }
}

/// Claude's plan usage, in percent of each window, and when each resets (unix seconds).
#[derive(Debug, Clone, PartialEq)]
pub struct Usage {
    pub five_hour: f64,
    pub week: f64,
    pub five_hour_resets: Option<i64>,
    pub week_resets: Option<i64>,
}

/// The body of `api/oauth/usage`.
pub fn parse_usage(json: &str) -> Option<Usage> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let used = |window: &str| v[window]["utilization"].as_f64();
    let resets = |window: &str| v[window]["resets_at"].as_str().and_then(unix_secs);
    Some(Usage { five_hour: used("five_hour")?, week: used("seven_day")?, five_hour_resets: resets("five_hour"), week_resets: resets("seven_day") })
}

/// An RFC 3339 time (`2026-10-11T15:00:00.123Z`, `…+03:00`) as unix seconds.
pub fn unix_secs(t: &str) -> Option<i64> {
    let n = |r: std::ops::Range<usize>| t.get(r)?.parse::<i64>().ok();
    let (y, m, d, hh, mm, ss) = (n(0..4)?, n(5..7)?, n(8..10)?, n(11..13)?, n(14..16)?, n(17..19)?);
    // days from civil (Howard Hinnant's), 1970-01-01 = 0
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + (153 * m + 2) / 5 + d - 1;
    let days = era * 146_097 + doe - 719_468;
    let zone = t[19..].trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let offset = match zone.as_bytes().first() {
        Some(b'+' | b'-') => {
            let sign = if zone.starts_with('-') { -1 } else { 1 };
            sign * (zone.get(1..3)?.parse::<i64>().ok()? * 3600 + zone.get(4..6)?.parse::<i64>().ok()? * 60)
        }
        _ => 0,
    };
    Some(days * 86_400 + hh * 3600 + mm * 60 + ss - offset)
}

/// `secs` from now as `resets in 2h 13m`, `resets in 3d 4h`, `resets in 12m`.
pub fn resets_in(secs: i64) -> String {
    let m = secs.max(0) / 60;
    let (d, h, m) = (m / 1440, m / 60 % 24, m % 60);
    match (d, h) {
        (0, 0) => format!("resets in {m}m"),
        (0, _) => format!("resets in {h}h {m}m"),
        _ => format!("resets in {d}d {h}h"),
    }
}

/// The ring's color: the accent, then warning from 70 %, danger from 90 %.
fn usage_color(used: f64, theme: &Theme) -> gpui::Hsla {
    if used >= 90. { theme.danger } else if used >= 70. { theme.warning } else { theme.accent }
}

/// The limits popup's card: the 5-hour and weekly windows, each with its ring and when it resets.
pub fn usage_card(theme: &Theme, cx: &mut App) -> gpui::Stateful<gpui::Div> {
    let usage = cached("", USAGE_TTL, read_usage, cx);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let faint = theme.text_faint;
    let window = |name: &'static str, used: f64, resets: Option<i64>| {
        div()
            .h(px(40.))
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(10.))
            .child(div().size(px(16.)).flex().items_center().justify_center().child(donut(used, theme.text.opacity(0.2), usage_color(used, theme))))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(name)
                    .child(div().text_size(px(11.)).text_color(faint).child(resets.map(|r| resets_in(r - now)).unwrap_or_default())),
            )
            .child(div().font_weight(gpui::FontWeight::MEDIUM).child(format!("{}%", used.round())))
    };
    let hover = theme.hover;
    crate::shell::menu_card(theme, "usage-menu", 240.)
        .text_size(px(12.))
        .text_color(theme.text)
        .children(usage.map(|u| [window("5-hour limit", u.five_hour, u.five_hour_resets), window("Weekly limit", u.week, u.week_resets)]).into_iter().flatten())
        .child(
            div()
                .id("usage-open")
                .h(px(28.))
                .px(px(8.))
                .flex()
                .items_center()
                .rounded(px(6.))
                .text_color(faint)
                .cursor_pointer()
                .hover(move |s| s.bg(hover))
                .child("Open usage settings")
                .on_click(|_, _, cx| cx.open_url("https://claude.ai/settings/usage")),
        )
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

/// Tokens a Claude Code session spent, by kind.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Spent {
    pub answers: u64,
    pub input: u64,
    pub output: u64,
    pub cache_write: u64,
    pub cache_read: u64,
}

impl Spent {
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_write + self.cache_read
    }
}

/// A Claude Code transcript's usage, summed. Each answer is written once per content block with the same
/// usage, so it counts once by its message id.
pub fn parse_spent(jsonl: &str) -> Spent {
    let mut seen = std::collections::HashSet::new();
    let mut spent = Spent::default();
    let lines = jsonl.lines().filter(|l| l.contains("\"usage\"")).filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok());
    for v in lines.filter(|v| seen.insert(v["message"]["id"].as_str().unwrap_or_default().to_string())) {
        let n = |k: &str| v["message"]["usage"][k].as_u64().unwrap_or(0);
        spent.answers += 1;
        spent.input += n("input_tokens");
        spent.output += n("output_tokens");
        spent.cache_write += n("cache_creation_input_tokens");
        spent.cache_read += n("cache_read_input_tokens");
    }
    spent
}

/// `1234` as `1.2k`, `5600000` as `5.6M`.
pub fn short_count(n: u64) -> String {
    match n {
        0..1_000 => n.to_string(),
        1_000..1_000_000 => format!("{:.1}k", n as f64 / 1e3),
        _ => format!("{:.1}M", n as f64 / 1e6),
    }
}

// ponytail: rereads the whole transcript each SPENT_TTL; read on from the last offset if transcripts get huge.
// Subagents' own transcripts (`<session>/subagents/`) aren't counted.
/// What Claude session `id` spent, from its transcript in any of Claude Code's project folders.
fn read_spent(id: &str) -> Option<Spent> {
    let projects = Path::new(&std::env::var_os("HOME")?).join(".claude/projects");
    let file = std::fs::read_dir(projects).ok()?.flatten().map(|d| d.path().join(format!("{id}.jsonl"))).find(|f| f.is_file())?;
    Some(parse_spent(&std::fs::read_to_string(file).ok()?))
}

/// What `pane`'s agent session spent so far, refreshed in the background.
fn spent_of(pane: &Pane, cx: &mut App) -> Option<Spent> {
    pane.agent_session.as_ref().and_then(|s| cached(&s.value, SPENT_TTL, read_spent, cx))
}

/// The spend popup's card: every agent with a session, what it spent by kind, and the sum; in the sidebar's
/// order, or most tokens first when `by_tokens`. `on_sort` flips that; `pick` makes a row's click handler from
/// its tab.
pub fn spent_card<L: Fn(&ClickEvent, &mut Window, &mut App) + 'static>(
    snapshot: Option<&Snapshot>,
    shown_pane: Option<&str>,
    theme: &Theme,
    by_tokens: bool,
    on_sort: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    pick: impl Fn(&str) -> L,
    cx: &mut App,
) -> gpui::Stateful<gpui::Div> {
    let mut agents: Vec<(&Pane, String, Option<Spent>)> = snapshot
        .map(|s| {
            s.agents()
                .into_iter()
                .filter(|p| p.agent_session.is_some())
                .map(|p| {
                    let ws = s.workspaces.iter().find(|w| w.workspace_id == p.workspace_id).map(|w| w.label.clone());
                    (p, ws.unwrap_or_default(), spent_of(p, cx))
                })
                .collect()
        })
        .unwrap_or_default();
    if by_tokens {
        agents.sort_by_key(|(_, _, s)| std::cmp::Reverse(s.as_ref().map_or(0, Spent::total)));
    }
    let sum = agents.iter().filter_map(|(_, _, s)| s.as_ref()).map(Spent::total).sum::<u64>();
    let faint = theme.text_faint;
    let (hover, selected, ink) = (theme.hover, theme.selected, theme.text);
    crate::shell::menu_card(theme, "spent-menu", 420.)
        .w(px(420.))
        .text_size(px(12.))
        .text_color(theme.text)
        .child(
            div()
                .h(px(28.))
                .px(px(8.))
                .flex()
                .items_center()
                .text_size(px(11.))
                .text_color(faint)
                .gap(px(8.))
                .child(div().flex_1().child("Tokens by agent"))
                .child(
                    div()
                        .id("spent-sort")
                        .h(px(20.))
                        .px(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .rounded(px(6.))
                        .cursor_pointer()
                        .when(by_tokens, |el| el.bg(selected).text_color(ink))
                        .hover(move |s| s.bg(hover))
                        .child(gpui::svg().path("icons/arrow-down.svg").size(px(10.)).text_color(if by_tokens { ink } else { faint }))
                        .child("Most tokens")
                        .on_click(on_sort),
                )
                .child(format!("{} total", short_count(sum))),
        )
        .when(agents.is_empty(), |el| el.child(div().px(px(8.)).pb(px(6.)).text_color(faint).child("No agent sessions")))
        .child(div().id("spent-list").max_h(px(460.)).overflow_y_scroll().flex().flex_col().children(agents.into_iter().map(|(p, ws, spent)| {
            let detail = match spent {
                Some(s) => format!(
                    "out {} · in {} · cache w {} r {} · {} replies",
                    short_count(s.output),
                    short_count(s.input),
                    short_count(s.cache_write),
                    short_count(s.cache_read),
                    s.answers
                ),
                None => "no transcript yet".into(),
            };
            let title = p.terminal_title_stripped.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| p.agent.clone().unwrap_or_default());
            crate::shell::row(theme, shown_pane == Some(p.pane_id.as_str()), SharedString::from(format!("spent-{}", p.pane_id)))
                .h_auto()
                .py(px(6.))
                .items_start()
                .child(crate::shell::agent_favicon(p.agent.as_deref(), theme).mt(px(1.)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(div().flex().gap(px(6.)).child(div().flex_none().child(ws)).child(div().min_w_0().truncate().text_color(faint).child(title)))
                        .child(div().text_size(px(11.)).text_color(faint).truncate().child(detail)),
                )
                .child(
                    div()
                        .flex_none()
                        .h(px(20.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(spent.map(|s| short_count(s.total())).unwrap_or_else(|| "…".into()))
                        .child(crate::shell::status_dot(p.agent_status, theme)),
                )
                .on_click(pick(&p.tab_id))
        })))
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
            // Round caps: gpui doesn't re-export lyon's LineCap, so a dot the line's width sits on each end.
            if fraction > 0. && fraction < 1. {
                for turn in [0., fraction] {
                    let a = turn * std::f32::consts::TAU;
                    let at = gpui::point(center.x + px(r * a.sin() - width / 2.), center.y - px(r * a.cos() + width / 2.));
                    let dot = gpui::Bounds::new(at, gpui::size(px(width), px(width)));
                    window.paint_quad(gpui::fill(dot, color).corner_radii(px(width / 2.)));
                }
            }
        },
    )
    .size(px(12.))
    .flex_none()
}

/// A clickable label: a wash on hover, and kept while its popup is open.
fn link(el: gpui::Stateful<gpui::Div>, theme: &Theme, open: bool) -> gpui::Stateful<gpui::Div> {
    let hover = theme.hover;
    el.h(px(20.)).rounded(px(6.)).cursor_pointer().when(open, |el| el.bg(hover)).hover(move |s| s.bg(hover))
}

/// Runs `on_click` with the label's bounds from the last paint, so a popup can line up with its edge.
fn on_click_at(
    el: gpui::Stateful<gpui::Div>,
    on_click: impl Fn(&gpui::Bounds<gpui::Pixels>, &mut Window, &mut App) + 'static,
) -> gpui::Stateful<gpui::Div> {
    let bounds = std::rc::Rc::new(std::cell::Cell::new(gpui::Bounds::default()));
    let painted = bounds.clone();
    el.relative()
        .child(gpui::canvas(move |b, _, _| painted.set(b), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
        .on_click(move |_, window, cx| on_click(&bounds.get(), window, cx))
}

/// The bar's labels that open a popup.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Branch,
    Spent,
    Usage,
}

/// The bar for the pane `pane_id` of `snapshot`. `on_branch`, `on_spent` and `on_usage` run on a click on
/// the branch, the spend and the limits (the last two with the label's bounds); `open` is the one whose popup is up.
pub fn bar(
    snapshot: Option<&Snapshot>,
    pane_id: Option<&str>,
    theme: &Theme,
    open: Option<Item>,
    on_branch: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_spent: impl Fn(&gpui::Bounds<gpui::Pixels>, &mut Window, &mut App) + 'static,
    on_usage: impl Fn(&gpui::Bounds<gpui::Pixels>, &mut Window, &mut App) + 'static,
    cx: &mut App,
) -> gpui::AnyElement {
    let pane = snapshot.zip(pane_id).and_then(|(s, id)| s.panes.iter().find(|p| p.pane_id == id));
    let dir = pane_dir(snapshot, pane_id);
    let git = dir.as_deref().and_then(|d| cached(d, GIT_TTL, read_git, cx));
    let usage = cached("", USAGE_TTL, read_usage, cx);
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
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
            link(label(theme, "icons/git-branch.svg").id("statusbar-branch"), theme, open == Some(Item::Branch))
                .flex_shrink_0()
                .max_w(px(220.))
                .child(div().min_w_0().truncate().child(g.branch))
                .when(g.dirty, |el| el.child(div().flex_none().text_color(theme.warning).child("●")))
                .when(g.ahead > 0, |el| el.child(div().flex_none().child(format!("↑{}", g.ahead))))
                .when(g.behind > 0, |el| el.child(div().flex_none().child(format!("↓{}", g.behind))))
                .on_click(on_branch)
        }))
        .children(dir.map(|d| {
            let short = short_path(&d, home.as_deref());
            link(label(theme, "icons/folder.svg").id("statusbar-cwd"), theme, false)
                .child(div().min_w_0().truncate().child(short))
                .on_click(move |_, _, cx| cx.open_with_system(Path::new(&d)))
        }))
        .children(pane.and_then(|p| spent_of(p, cx)).map(|s| {
            on_click_at(link(div().id("statusbar-spent").flex_none().flex().items_center().px(px(6.)), theme, open == Some(Item::Spent)), on_spent)
                .child(format!("{} tok", short_count(s.total())))
        }))
        .child(div().flex_1())
        .children(usage.map(|u| {
            // the dotfiles' statusbar: the ring fills with the 5-hour window, colored by the worse of the two
            let color = usage_color(u.five_hour.max(u.week), theme);
            on_click_at(link(div().id("statusbar-usage").flex_none().flex().items_center().gap(px(6.)).px(px(8.)), theme, open == Some(Item::Usage)), on_usage)
                .child(donut(u.five_hour, theme.text.opacity(0.2), color))
                .child(format!("{}%", u.five_hour.round()))
        }))
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
    fn parses_worktree_changes_and_pr() {
        assert_eq!(parse_shortstat(" 3 files changed, 12 insertions(+), 3 deletions(-)\n"), (12, 3));
        assert_eq!(parse_shortstat(" 1 file changed, 1 insertion(+)\n"), (1, 0));
        assert_eq!(parse_shortstat(" 1 file changed, 4 deletions(-)\n"), (0, 4));
        assert_eq!(parse_shortstat(""), (0, 0));
        assert_eq!(parse_pr(r#"{"isDraft":true,"state":"OPEN"}"#), Some("draft"));
        assert_eq!(parse_pr(r#"{"isDraft":false,"state":"MERGED"}"#), Some("merged"));
        assert_eq!(parse_pr(""), None);
    }

    #[test]
    fn parses_claude_usage() {
        let body = r#"{"five_hour":{"utilization":42.0,"resets_at":"2026-10-11T15:00:00.000Z"},"seven_day":{"utilization":18,"resets_at":null}}"#;
        let five_hour_resets = Some(1_791_730_800);
        assert_eq!(parse_usage(body), Some(Usage { five_hour: 42.0, week: 18.0, five_hour_resets, week_resets: None }));
        assert_eq!(unix_secs("2026-10-11T18:00:00.5+03:00"), five_hour_resets);
        assert_eq!(unix_secs("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(unix_secs("soon"), None);
        assert_eq!((resets_in(12 * 60), resets_in(2 * 3600 + 13 * 60), resets_in(3 * 86_400 + 4 * 3600 + 59)), ("resets in 12m".into(), "resets in 2h 13m".into(), "resets in 3d 4h".into()));
        assert_eq!(parse_usage(r#"{"five_hour":{"utilization":null},"seven_day":{"utilization":1}}"#), None);
        assert_eq!(parse_usage(r#"{"error":{"type":"authentication_error"}}"#), None);
    }

    #[test]
    fn sums_transcript_tokens_once_per_answer() {
        let line = |id: &str, out: u64| {
            format!(r#"{{"type":"assistant","message":{{"id":"{id}","usage":{{"input_tokens":2,"output_tokens":{out},"cache_creation_input_tokens":100,"cache_read_input_tokens":1000,"service_tier":"standard"}}}}}}"#)
        };
        let jsonl = [line("a", 10), line("a", 10), r#"{"type":"user","message":{"content":"hi"}}"#.into(), line("b", 5)].join("\n");
        let spent = parse_spent(&jsonl);
        assert_eq!(spent, Spent { answers: 2, input: 4, output: 15, cache_write: 200, cache_read: 2000 });
        assert_eq!(spent.total(), 2219);
        assert_eq!(parse_spent(""), Spent::default());
        assert_eq!(short_count(999), "999");
        assert_eq!(short_count(1234), "1.2k");
        assert_eq!(short_count(5_600_000), "5.6M");
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
}
