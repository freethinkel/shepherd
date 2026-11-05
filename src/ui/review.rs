//! The review panel's data: the shown branch's GitHub PR (`gh`) or GitLab MR (`glab`), picked by origin's host, with
//! its checks or its pipeline's jobs, and the description as a few markdown blocks. The CLIs run off the main thread,
//! through the status bar's cache.
use crate::ui::statusbar::{Pr, parse_pr, tool, unix_secs};
use serde_json::Value;

/// What the panel shows for a branch.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)] // one per branch shown; boxing saves nothing
pub enum Review {
    /// The PR or MR, if the branch has one, and its checks: a GitLab branch's pipeline (its page and own state)
    /// runs without an MR too.
    Found { request: Option<Request>, checks: Vec<Check>, pipeline: Option<(String, CheckState)> },
    /// The repo has no `origin`.
    NoRemote,
    /// The CLI (`gh`, `glab`) is not installed.
    NoTool(&'static str),
    /// The CLI is not logged in to the host.
    NoLogin(&'static str, String),
    /// The CLI's complaint, its first line.
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub pr: Pr,
    pub title: String,
    pub author: String,
    pub body: String,
    /// The branch it goes into, and the one it comes from.
    pub base: String,
    pub head: String,
    /// When it last changed, unix seconds.
    pub updated: Option<i64>,
    pub labels: Vec<String>,
    /// Files changed, lines added and removed (not in glab's REST answer).
    pub diff: Option<Diff>,
    /// No conflicts with the base, as far as the host knows.
    pub mergeable: bool,
    /// The CLI's user may approve it: not their own GitHub PR, a GitLab MR its rules let them.
    pub can_approve: bool,
    /// The CLI's user approved it already.
    pub approved: bool,
    /// Approved, by anyone: GitHub's review decision, GitLab's approval rules met.
    pub accepted: bool,
    /// Who was asked to review and what they said.
    pub reviewers: Vec<Reviewer>,
    /// Threads still open (GitLab; GitHub's are not asked).
    pub unresolved: Option<u32>,
    /// It merges by itself once its pipeline passes.
    pub auto_merge: bool,
    /// How it merges unless told otherwise: squashed, its branch deleted after (GitLab's settings for it).
    pub squash: bool,
    pub remove_branch: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Diff {
    pub files: u64,
    pub additions: u64,
    pub deletions: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Reviewer {
    pub name: String,
    pub verdict: Verdict,
}

/// A reviewer's last word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Approved,
    Changes,
    Commented,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    Success,
    Failed,
    Running,
    Pending,
    Skipped,
}

/// A GitHub check or a GitLab job.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub name: String,
    /// The pipeline it runs in (a GitLab child pipeline's trigger job, `parent › child` deeper down), or its workflow:
    /// the panel's page for it.
    pub page: String,
    /// Its GitLab stage; empty on GitHub.
    pub group: String,
    pub state: CheckState,
    /// How long it ran, or has been running, when it was fetched.
    pub secs: Option<i64>,
    pub url: Option<String>,
    /// The id the CLI acts on: a GitLab job's, a GitHub Actions job's (its page's last number).
    pub job: Option<String>,
    /// A GitHub Actions job's workflow run.
    pub run: Option<String>,
    /// A GitLab job waiting for someone to start it.
    pub manual: bool,
    /// A GitLab trigger job: it starts a child pipeline, and the CLI cannot retry, play or cancel it.
    pub bridge: bool,
    /// The page of the child pipeline it started.
    pub child: Option<String>,
    /// The GitLab project it runs in when not the repo's own (a multi-project pipeline's).
    pub repo: Option<String>,
}

/// What the panel's buttons do to a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Retry,
    /// Retries a failed check; on GitHub with the rest of its run's failed jobs.
    RetryFailed,
    /// Starts a manual GitLab job.
    Play,
    /// Cancels a GitLab job, or a GitHub job's whole run (GitHub cancels no single job).
    Cancel,
}

impl Check {
    /// The row's button: cancel while it runs, play a manual job, retry one that finished.
    pub fn action(&self) -> Action {
        match self.state {
            CheckState::Running | CheckState::Pending => Action::Cancel,
            _ if self.manual => Action::Play,
            _ => Action::Retry,
        }
    }

    /// The CLI call that does `action` to it; `None` when its host has none, or it lacks the id (a commit status).
    pub fn command(&self, gitlab: bool, action: Action) -> Option<Vec<String>> {
        if self.bridge {
            return None;
        }
        let job = self.job.clone();
        let mut args: Vec<String> = match (gitlab, action) {
            (true, Action::Retry | Action::RetryFailed) => vec!["glab".into(), "ci".into(), "retry".into(), job?],
            (true, Action::Play) => vec!["glab".into(), "ci".into(), "trigger".into(), job?],
            (true, Action::Cancel) => vec!["glab".into(), "ci".into(), "cancel".into(), "job".into(), job?],
            (false, Action::Retry) => vec!["gh".into(), "run".into(), "rerun".into(), "--job".into(), job?],
            (false, Action::RetryFailed) => vec!["gh".into(), "run".into(), "rerun".into(), self.run.clone()?, "--failed".into()],
            (false, Action::Cancel) => vec!["gh".into(), "run".into(), "cancel".into(), self.run.clone()?],
            (false, Action::Play) => return None,
        };
        if let Some(repo) = &self.repo {
            args.extend(["-R".into(), repo.clone()]);
        }
        Some(args)
    }
}

/// The calls the checks section's Retry failed makes: one per failed GitLab job, one per GitHub run with a failure.
pub fn retry_failed(checks: &[Check], gitlab: bool) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    for cmd in checks.iter().filter(|c| c.state == CheckState::Failed).filter_map(|c| c.command(gitlab, Action::RetryFailed)) {
        if !out.contains(&cmd) {
            out.push(cmd);
        }
    }
    out
}

/// Runs a [`Check::command`] in `dir`; the CLI's complaint, its first line, when it fails.
pub fn act(dir: &str, args: &[String]) -> Result<(), String> {
    let (bin, rest) = args.split_first().ok_or("nothing to run")?;
    let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
    let out = cli(bin, &rest, dir).map_err(|e| format!("{bin}: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    Err(err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("failed").to_string())
}

/// `…/actions/runs/<run>/job/<job>`, a GitHub Actions check's page: its run and job.
fn actions_ids(url: &str) -> Option<(String, String)> {
    let (run, job) = url.split_once("/actions/runs/")?.1.split_once("/job/")?;
    let job = job.split(['/', '?', '#']).next()?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (digits(run) && digits(job)).then(|| (run.to_string(), job.to_string()))
}

impl Review {
    /// Something still runs or waits to: the panel asks again soon.
    pub fn running(&self) -> bool {
        let busy = |s: CheckState| matches!(s, CheckState::Running | CheckState::Pending);
        // a pipeline's own state counts too: its child pipelines' jobs are not among its own
        matches!(self, Review::Found { checks, pipeline, .. } if checks.iter().any(|c| busy(c.state)) || pipeline.as_ref().is_some_and(|(_, s)| busy(*s)))
    }

    /// CI as a whole: a GitLab pipeline's own state, else the worst of the checks; `None` without any that ran.
    pub fn ci(&self) -> Option<CheckState> {
        let Review::Found { checks, pipeline, .. } = self else { return None };
        if let Some((_, state)) = pipeline {
            return Some(*state);
        }
        Some(worst(checks.iter().map(|c| c.state))).filter(|s| *s != CheckState::Skipped)
    }
}

/// The state that matters most of `states`: a failure, then one running, waiting, passed; skipped when all were.
pub fn worst(states: impl Iterator<Item = CheckState>) -> CheckState {
    let rank = |s: CheckState| [CheckState::Skipped, CheckState::Success, CheckState::Pending, CheckState::Running, CheckState::Failed].iter().position(|r| *r == s);
    states.max_by_key(|s| rank(*s)).unwrap_or(CheckState::Skipped)
}

/// The checks' pages (GitLab pipelines, GitHub workflows) in the order they first come, each with its worst state
/// leaving trigger jobs out: a child pipeline's failure is its own page's, not its parent's too.
pub fn pages(checks: &[Check]) -> Vec<(String, CheckState)> {
    let mut out: Vec<(String, Option<CheckState>)> = Vec::new();
    for c in checks {
        let i = out.iter().position(|(n, _)| *n == c.page).unwrap_or_else(|| {
            out.push((c.page.clone(), None));
            out.len() - 1
        });
        if !c.bridge {
            let state = &mut out[i].1;
            *state = Some(state.map_or(c.state, |s| worst([s, c.state].into_iter())));
        }
    }
    // a page of trigger jobs only takes theirs
    out.into_iter()
        .map(|(name, state)| {
            let state = state.unwrap_or_else(|| worst(checks.iter().filter(|c| c.page == name).map(|c| c.state)));
            (name, state)
        })
        .collect()
}

/// The page the panel opens on: the first that failed, else the first still running or waiting, else the first.
pub fn default_page(pages: &[(String, CheckState)]) -> usize {
    let first = |want: &[CheckState]| pages.iter().position(|(_, s)| want.contains(s));
    first(&[CheckState::Failed]).or_else(|| first(&[CheckState::Running, CheckState::Pending])).unwrap_or(0)
}

impl Check {
}

/// A status as GitHub (`IN_PROGRESS`, `FAILURE`) or GitLab (`running`, `failed`) writes it.
pub fn check_state(s: &str) -> CheckState {
    match s.to_ascii_lowercase().as_str() {
        "success" | "passed" => CheckState::Success,
        "failure" | "failed" | "error" | "timed_out" | "startup_failure" | "action_required" => CheckState::Failed,
        "in_progress" | "running" => CheckState::Running,
        // a manual job waits for a person, not for CI: polling for it would never end
        "skipped" | "neutral" | "stale" | "cancelled" | "canceled" | "manual" => CheckState::Skipped,
        _ => CheckState::Pending,
    }
}

/// A check's state as the review panel and the status bar draw it, `size` px: a spinner while it runs.
pub fn check_icon(theme: &crate::theme::Theme, state: CheckState, size: f32, window: &mut gpui::Window) -> gpui::AnyElement {
    use gpui::{IntoElement, ParentElement, Styled, px};
    let (icon, color) = match state {
        CheckState::Success => ("icons/solar/check-circle.svg", theme.success),
        CheckState::Failed => ("icons/solar/close-circle.svg", theme.danger),
        CheckState::Pending => ("icons/solar/clock-circle.svg", theme.warning),
        CheckState::Skipped => ("icons/solar/forbidden-circle.svg", theme.text_faint),
        CheckState::Running => {
            let loader = crate::ui::loader::mini_spinner(2.5, theme.warning, window);
            return gpui::div().size(px(size)).flex_none().flex().items_center().justify_center().child(loader).into_any_element();
        }
    };
    gpui::svg().path(icon).size(px(size)).flex_none().text_color(color).into_any_element()
}

/// `origin`'s host, from `git@host:a/b.git`, `ssh://git@host:22/a/b`, `https://user:token@host/a/b`.
pub fn host(url: &str) -> Option<String> {
    let rest = url.trim().split_once("://").map_or(url.trim(), |(_, r)| r);
    let authority = rest.split('/').next()?;
    let host = authority.rsplit('@').next()?.split(':').next()?;
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// `gh pr view --json` with [`GITHUB_FIELDS`], read, for the CLI's user `viewer`; durations up to `now`.
pub fn parse_github(json: &str, viewer: &str, now: i64) -> Option<(Request, Vec<Check>)> {
    let v: Value = serde_json::from_str(json).ok()?;
    let request = request(&v, json, "login", "body", viewer)?;
    let checks = v["statusCheckRollup"].as_array().map(|all| all.iter().filter_map(|c| github_check(c, now)).collect()).unwrap_or_default();
    Some((request, checks))
}

fn github_check(c: &Value, now: i64) -> Option<Check> {
    let status = c["status"].as_str().unwrap_or("COMPLETED");
    let (name, group, url, state) = match c["__typename"].as_str() {
        // a commit status (an external CI's): only a state
        Some("StatusContext") => (c["context"].as_str()?, "", c["targetUrl"].as_str(), c["state"].as_str()?),
        _ if status != "COMPLETED" => (c["name"].as_str()?, c["workflowName"].as_str().unwrap_or(""), c["detailsUrl"].as_str(), status),
        _ => (c["name"].as_str()?, c["workflowName"].as_str().unwrap_or(""), c["detailsUrl"].as_str(), c["conclusion"].as_str()?),
    };
    let state = check_state(state);
    let (run, job) = url.and_then(actions_ids).unzip();
    let secs = span(c["startedAt"].as_str(), c["completedAt"].as_str(), state, now);
    let page = if group.is_empty() { "Other" } else { group };
    Some(Check {
        name: name.into(),
        page: page.into(),
        group: String::new(),
        state,
        secs,
        url: url.map(Into::into),
        job,
        run,
        manual: false,
        bridge: false,
        child: None,
        repo: None,
    })
}

/// `glab mr view -F json`, read, for the CLI's user `viewer`.
pub fn parse_gitlab_mr(json: &str, viewer: &str) -> Option<Request> {
    request(&serde_json::from_str(json).ok()?, json, "username", "description", viewer)
}

/// The page of a GitLab branch's own pipeline, before its child pipelines'.
pub const ROOT: &str = "Pipeline";

/// A branch's MR (its approvals, reviewers, threads and merge settings) and the MR's pipeline with its child pipelines'
/// jobs, else the branch's own pipeline, in one GraphQL query; the CLI's user too.
const GITLAB_QUERY: &str = "query($path: ID!, $branch: String!) { currentUser { username } project(fullPath: $path) { \
    mergeRequests(sourceBranches: [$branch], first: 1, sort: CREATED_DESC) { nodes { \
    iid title state draft webUrl description targetBranch sourceBranch updatedAt conflicts autoMergeEnabled squash forceRemoveSourceBranch \
    labels { nodes { title } } diffStatsSummary { additions deletions fileCount } \
    author { username } approved approvedBy { nodes { username } } userPermissions { canApprove } \
    discussions(first: 100) { nodes { resolvable resolved } } \
    reviewers { nodes { username mergeRequestInteraction { reviewState } } } \
    headPipeline { status path ...P downstream { nodes { ...P sourceJob { id name } project { fullPath } \
    downstream { nodes { ...P sourceJob { id name } project { fullPath } } } } } } } } \
    pipelines(ref: $branch, first: 1) { nodes { iid status path } } } } \
    fragment P on Pipeline { stages { nodes { name } } \
    jobs { nodes { id name status duration startedAt finishedAt webPath stage { name } } } }";

/// [`GITLAB_QUERY`]'s answer, read: the MR, and the pipeline (its page and state) with its jobs. Without an MR it is
/// the branch's pipeline, its jobs left to ask for by its iid, the last item. `base` is the host's address.
#[allow(clippy::type_complexity)]
pub fn parse_gitlab_review(json: &str, base: &str, path: &str, now: i64) -> Option<(Option<Request>, Option<(String, CheckState)>, Vec<Check>, Option<String>)> {
    let v: Value = serde_json::from_str(json).ok()?;
    let me = v["data"]["currentUser"]["username"].as_str().unwrap_or("");
    let project = &v["data"]["project"];
    project.as_object()?;
    let names = |list: &Value| list["nodes"].as_array().into_iter().flatten().filter_map(|n| n["username"].as_str()).map(String::from).collect::<Vec<_>>();
    let Some(mr) = project["mergeRequests"]["nodes"].get(0) else {
        let p = project["pipelines"]["nodes"].get(0);
        let pipeline = p.and_then(|p| Some((format!("{base}{}", p["path"].as_str()?), check_state(p["status"].as_str()?))));
        return Some((None, pipeline, Vec::new(), p.and_then(|p| p["iid"].as_str().map(String::from))));
    };
    let draft = mr["draft"].as_bool().unwrap_or(false);
    let state = match (mr["state"].as_str().unwrap_or(""), draft) {
        ("opened", true) => "draft",
        ("opened", false) => "open",
        ("merged", _) => "merged",
        _ => "closed",
    };
    let approvers = names(&mr["approvedBy"]);
    let mut reviewers: Vec<Reviewer> = mr["reviewers"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            let name = r["username"].as_str()?.to_string();
            let verdict = match r["mergeRequestInteraction"]["reviewState"].as_str() {
                _ if approvers.contains(&name) => Verdict::Approved,
                Some("APPROVED") => Verdict::Approved,
                Some("REQUESTED_CHANGES") => Verdict::Changes,
                Some("REVIEWED") => Verdict::Commented,
                _ => Verdict::Pending,
            };
            Some(Reviewer { name, verdict })
        })
        .collect();
    for name in &approvers {
        if !reviewers.iter().any(|r| r.name == *name) {
            reviewers.push(Reviewer { name: name.clone(), verdict: Verdict::Approved });
        }
    }
    let threads = mr["discussions"]["nodes"].as_array().into_iter().flatten();
    let request = Request {
        pr: Pr { number: mr["iid"].as_str()?.parse().ok()?, url: mr["webUrl"].as_str()?.into(), state, gitlab: true },
        title: mr["title"].as_str().unwrap_or("").into(),
        author: mr["author"]["username"].as_str().unwrap_or("").into(),
        body: mr["description"].as_str().unwrap_or("").into(),
        base: mr["targetBranch"].as_str().unwrap_or("").into(),
        head: mr["sourceBranch"].as_str().unwrap_or("").into(),
        updated: mr["updatedAt"].as_str().and_then(unix_secs),
        labels: mr["labels"]["nodes"].as_array().into_iter().flatten().filter_map(|l| l["title"].as_str()).map(String::from).collect(),
        diff: mr["diffStatsSummary"].as_object().and_then(|d| {
            Some(Diff { files: d.get("fileCount")?.as_u64()?, additions: d.get("additions")?.as_u64()?, deletions: d.get("deletions")?.as_u64()? })
        }),
        mergeable: mr["conflicts"].as_bool() != Some(true),
        can_approve: mr["userPermissions"]["canApprove"].as_bool().unwrap_or(true),
        approved: !me.is_empty() && approvers.iter().any(|a| a == me),
        accepted: mr["approved"].as_bool().unwrap_or(false),
        reviewers,
        unresolved: Some(threads.filter(|t| t["resolvable"].as_bool() == Some(true) && t["resolved"].as_bool() != Some(true)).count() as u32),
        auto_merge: mr["autoMergeEnabled"].as_bool().unwrap_or(false),
        squash: mr["squash"].as_bool().unwrap_or(false),
        remove_branch: mr["forceRemoveSourceBranch"].as_bool().unwrap_or(false),
    };
    let head = &mr["headPipeline"];
    let pipeline = head["path"].as_str().map(|p| (format!("{base}{p}"), check_state(head["status"].as_str().unwrap_or(""))));
    let mut checks = Vec::new();
    if head.is_object() {
        gitlab_pipeline_jobs(head, ROOT, None, base, path, now, &mut checks);
    }
    Some((Some(request), pipeline, checks, None))
}

/// A pipeline's jobs, its child pipelines' and theirs, in one GraphQL query: `glab ci get` has only its own.
const GITLAB_JOBS: &str = "query($path: ID!, $iid: ID!) { project(fullPath: $path) { pipeline(iid: $iid) { ...P \
    downstream { nodes { ...P sourceJob { id name } project { fullPath } \
    downstream { nodes { ...P sourceJob { id name } project { fullPath } } } } } } } } \
    fragment P on Pipeline { stages { nodes { name } } \
    jobs { nodes { id name status duration startedAt finishedAt webPath stage { name } } } }";

/// [`GITLAB_JOBS`]' answer, read: every job, a pipeline's stage by stage, and its child pipelines' after it. `base` is
/// the host's address (`https://gitlab.com`), `path` the project's; durations run up to `now`.
pub fn parse_gitlab_jobs(json: &str, base: &str, path: &str, now: i64) -> Option<Vec<Check>> {
    let v: Value = serde_json::from_str(json).ok()?;
    let root = &v["data"]["project"]["pipeline"];
    let mut out = Vec::new();
    gitlab_pipeline_jobs(root.as_object().map(|_| root)?, ROOT, None, base, path, now, &mut out);
    Some(out)
}

fn gitlab_pipeline_jobs(p: &Value, page: &str, repo: Option<&str>, base: &str, path: &str, now: i64, out: &mut Vec<Check>) {
    let nodes = |v: &Value| v["nodes"].as_array().cloned().unwrap_or_default();
    let mut children: Vec<(Value, String)> = nodes(&p["downstream"])
        .into_iter()
        .map(|d| {
            let name = d["sourceJob"]["name"].as_str().unwrap_or("child").to_string();
            let name = if page == ROOT { name } else { format!("{page} › {name}") };
            (d, name)
        })
        .collect();
    // GitLab lists the newest job first
    let mut jobs = nodes(&p["jobs"]);
    jobs.reverse();
    let start = out.len();
    for stage in nodes(&p["stages"]) {
        let stage = stage["name"].as_str().unwrap_or("");
        for j in jobs.iter().filter(|j| j["stage"]["name"].as_str() == Some(stage)) {
            let (Some(name), Some(status), Some(id)) = (j["name"].as_str(), j["status"].as_str(), j["id"].as_str()) else { continue };
            let state = check_state(status);
            let secs = j["duration"].as_i64().filter(|d| *d > 0).or_else(|| span(j["startedAt"].as_str(), j["finishedAt"].as_str(), state, now));
            out.push(Check {
                name: name.into(),
                page: page.into(),
                group: stage.into(),
                state,
                secs,
                url: j["webPath"].as_str().map(|w| format!("{base}{w}")),
                job: id.rsplit('/').next().map(Into::into),
                run: None,
                manual: status.eq_ignore_ascii_case("manual"),
                bridge: id.contains("Ci::Bridge"),
                child: children.iter().find(|(d, _)| d["sourceJob"]["id"].as_str() == Some(id)).map(|(_, n)| n.clone()),
                repo: repo.map(Into::into),
            });
        }
    }
    // child pages in their trigger jobs' order
    let order: Vec<&str> = out[start..].iter().filter_map(|c| c.child.as_deref()).collect();
    let rank = |name: &str| order.iter().position(|n| *n == name).unwrap_or(usize::MAX);
    let mut ranked: Vec<(usize, usize)> = children.iter().enumerate().map(|(i, (_, n))| (rank(n), i)).collect();
    ranked.sort();
    let children: Vec<(Value, String)> = ranked.into_iter().map(|(_, i)| std::mem::take(&mut children[i])).collect();
    for (d, name) in &children {
        // with its host: glab's own default is gitlab.com
        let project = d["project"]["fullPath"].as_str().filter(|p| *p != path).map(|p| format!("{base}/{p}"));
        gitlab_pipeline_jobs(d, name, project.as_deref(), base, path, now, out);
    }
}

/// `glab ci get -F json`, read: the pipeline's page and state, and its own jobs in the order they run.
pub fn parse_gitlab_pipeline(json: &str, now: i64) -> Option<((String, CheckState), Vec<Check>)> {
    let v: Value = serde_json::from_str(json).ok()?;
    let pipeline = (v["web_url"].as_str()?.to_string(), check_state(v["status"].as_str().unwrap_or("")));
    let mut jobs: Vec<Check> = v["jobs"]
        .as_array()
        .map(|all| {
            all.iter()
                .filter_map(|j| {
                    let status = j["status"].as_str()?;
                    let state = check_state(status);
                    let secs = j["duration"].as_f64().filter(|d| *d > 0.).map(|d| d.round() as i64);
                    Some(Check {
                        name: j["name"].as_str()?.into(),
                        page: ROOT.into(),
                        group: j["stage"].as_str().unwrap_or("").into(),
                        state,
                        secs: secs.or_else(|| span(j["started_at"].as_str(), j["finished_at"].as_str(), state, now)),
                        url: j["web_url"].as_str().map(Into::into),
                        job: j["id"].as_u64().map(|id| id.to_string()),
                        run: None,
                        manual: status == "manual",
                        bridge: false,
                        child: None,
                        repo: None,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    // GitLab lists the newest job first
    jobs.reverse();
    Some((pipeline, jobs))
}

fn request(v: &Value, json: &str, login: &str, body: &str, viewer: &str) -> Option<Request> {
    let author = v["author"][login].as_str().unwrap_or("");
    let me = |who: &Value| !viewer.is_empty() && who[login].as_str() == Some(viewer);
    let reviews = v["latestReviews"].as_array().map(Vec::as_slice).unwrap_or_default();
    Some(Request {
        pr: parse_pr(json)?,
        title: v["title"].as_str().unwrap_or("").into(),
        author: author.into(),
        body: v[body].as_str().unwrap_or("").into(),
        base: v["baseRefName"].as_str().or(v["target_branch"].as_str()).unwrap_or("").into(),
        head: v["headRefName"].as_str().or(v["source_branch"].as_str()).unwrap_or("").into(),
        updated: v["updatedAt"].as_str().or(v["updated_at"].as_str()).and_then(unix_secs),
        // GitHub's are objects with a name, glab's REST answer plain strings
        labels: v["labels"].as_array().into_iter().flatten().filter_map(|l| l["name"].as_str().or(l.as_str())).map(String::from).collect(),
        diff: v["changedFiles"].as_u64().zip(v["additions"].as_u64()).zip(v["deletions"].as_u64()).map(|((files, additions), deletions)| Diff { files, additions, deletions }),
        mergeable: v["mergeable"].as_str() != Some("CONFLICTING") && v["has_conflicts"].as_bool() != Some(true),
        can_approve: !me(&v["author"]),
        approved: reviews.iter().any(|r| me(&r["author"]) && r["state"].as_str() == Some("APPROVED")),
        accepted: v["reviewDecision"].as_str() == Some("APPROVED"),
        reviewers: github_reviewers(v),
        unresolved: None,
        auto_merge: !v["autoMergeRequest"].is_null(),
        squash: v["squash"].as_bool().unwrap_or(false),
        remove_branch: v["force_remove_source_branch"].as_bool().unwrap_or(false),
    })
}

/// A PR's reviews, the last of each reviewer, and the reviews still asked for.
fn github_reviewers(v: &Value) -> Vec<Reviewer> {
    let mut out: Vec<Reviewer> = Vec::new();
    for r in v["latestReviews"].as_array().into_iter().flatten() {
        let Some(name) = r["author"]["login"].as_str() else { continue };
        let verdict = match r["state"].as_str() {
            Some("APPROVED") => Verdict::Approved,
            Some("CHANGES_REQUESTED") => Verdict::Changes,
            _ => Verdict::Commented,
        };
        out.push(Reviewer { name: name.into(), verdict });
    }
    for r in v["reviewRequests"].as_array().into_iter().flatten() {
        let Some(name) = r["login"].as_str().or(r["slug"].as_str()).or(r["name"].as_str()) else { continue };
        if !out.iter().any(|o| o.name == name) {
            out.push(Reviewer { name: name.into(), verdict: Verdict::Pending });
        }
    }
    out
}

/// `glab api projects/:id/merge_requests/<iid>/approvals`, read into `request`.
pub fn gitlab_approvals(json: &str, request: &mut Request) {
    let Ok(v) = serde_json::from_str::<Value>(json) else { return };
    request.approved = v["user_has_approved"].as_bool().unwrap_or(false);
    request.can_approve = v["user_can_approve"].as_bool().unwrap_or(true);
    request.accepted = v["approved"].as_bool().unwrap_or(false);
}

/// gh's user on `host`, asked once per run; empty when it cannot say.
fn viewer(host: &str, dir: &str) -> String {
    static SEEN: std::sync::Mutex<Option<std::collections::HashMap<String, String>>> = std::sync::Mutex::new(None);
    let key = host.to_string();
    if let Some(login) = SEEN.lock().unwrap().get_or_insert_default().get(&key) {
        return login.clone();
    }
    let login = run("gh", &["api", "user", "--hostname", host, "--jq", ".login"], dir).ok().map(|(out, _)| out.trim().to_string());
    // a failure (offline, logged out) is asked again next time
    let Some(login) = login.filter(|l| !l.is_empty()) else { return String::new() };
    SEEN.lock().unwrap().get_or_insert_default().insert(key, login.clone());
    login
}

/// Approves the PR or MR `number` in `dir`.
pub fn approve(dir: &str, gitlab: bool, number: u64) -> Result<(), String> {
    let n = number.to_string();
    let args: &[&str] = if gitlab { &["glab", "mr", "approve", &n] } else { &["gh", "pr", "review", &n, "--approve"] };
    act(dir, &args.iter().map(|a| a.to_string()).collect::<Vec<_>>())
}

/// Marks the draft PR or MR `number` in `dir` ready for review.
pub fn mark_ready(dir: &str, gitlab: bool, number: u64) -> Result<(), String> {
    let n = number.to_string();
    let args: &[&str] = if gitlab { &["glab", "mr", "update", &n, "--ready"] } else { &["gh", "pr", "ready", &n] };
    act(dir, &args.iter().map(|a| a.to_string()).collect::<Vec<_>>())
}

/// Takes the CLI user's approval of the GitLab MR `number` in `dir` back.
pub fn revoke(dir: &str, number: u64) -> Result<(), String> {
    act(dir, &["glab".into(), "mr".into(), "revoke".into(), number.to_string()])
}

/// Merges the PR or MR `number` in `dir`, now or, with `auto`, once its pipeline passes. A GitLab MR is squashed and
/// its branch deleted as told; a GitHub PR is squashed when told, else merged the first way its repo allows of a merge
/// commit and a rebase.
pub fn merge(dir: &str, gitlab: bool, number: u64, auto: bool, squash: bool, remove_branch: bool) -> Result<(), String> {
    let n = number.to_string();
    let args: Vec<String> = if gitlab {
        // the API, not `glab mr merge`: that leaves out what is turned off, and the MR's own setting wins
        let flag = |name: &str, on: bool| ["-F".to_string(), format!("{name}={on}")];
        let mut args: Vec<String> = ["glab", "api", "-X", "PUT"].map(String::from).to_vec();
        args.push(format!("projects/:id/merge_requests/{n}/merge"));
        args.extend(flag("squash", squash));
        args.extend(flag("should_remove_source_branch", remove_branch));
        args.extend(flag("merge_when_pipeline_succeeds", auto));
        args
    } else {
        let way = if squash {
            "--squash"
        } else {
            let (out, err) = run("gh", &["repo", "view", "--json", "mergeCommitAllowed,squashMergeAllowed,rebaseMergeAllowed"], dir).map_err(|e| e.to_string())?;
            let allowed: Value = serde_json::from_str(&out).map_err(|_| err.lines().next().unwrap_or("gh repo view failed").to_string())?;
            [("mergeCommitAllowed", "--merge"), ("rebaseMergeAllowed", "--rebase"), ("squashMergeAllowed", "--squash")]
                .into_iter()
                .find(|(field, _)| allowed[*field].as_bool() == Some(true))
                .map_or("--merge", |(_, flag)| flag)
        };
        let mut args: Vec<String> = ["gh", "pr", "merge", &n, way].map(String::from).to_vec();
        if auto {
            args.push("--auto".into());
        }
        args
    };
    act(dir, &args)
}

/// Turns the PR or MR `number`'s merge once its pipeline passes off.
pub fn cancel_auto_merge(dir: &str, gitlab: bool, number: u64) -> Result<(), String> {
    let args: Vec<String> = if gitlab {
        vec!["glab".into(), "api".into(), "-X".into(), "POST".into(), format!("projects/:id/merge_requests/{number}/cancel_merge_when_pipeline_succeeds")]
    } else {
        vec!["gh".into(), "pr".into(), "merge".into(), number.to_string(), "--disable-auto".into()]
    };
    act(dir, &args)
}

/// How many of a job's log's last lines the panel shows.
pub const LOG_LINES: usize = 25;

/// A job's log, its last [`LOG_LINES`] lines as text; `key` is the located repo directory, `gitlab` or not, the job
/// and its project when not the repo's own, a line apart.
pub fn read_log(key: &str) -> Option<String> {
    let mut parts = key.split('\n');
    let (dir, gitlab, job, repo) = (parts.next()?, parts.next()? == "true", parts.next()?, parts.next().filter(|r| !r.is_empty()));
    let raw = if gitlab {
        let project = repo.map_or(":id".to_string(), |r| r.replace('/', "%2F"));
        run("glab", &["api", &format!("projects/{project}/jobs/{job}/trace")], dir).ok()?.0
    } else {
        run("gh", &["run", "view", "--job", job, "--log"], dir).ok()?.0
    };
    Some(tail_log(&raw, !gitlab, LOG_LINES))
}

/// A log's last `n` lines as they show in a terminal: what a carriage return wrote over and color codes gone, and
/// GitHub's job, step and time before each line.
pub fn tail_log(raw: &str, github: bool, n: usize) -> String {
    let lines: Vec<String> = raw
        .lines()
        .map(|line| {
            let line = if github { line.splitn(3, '\t').nth(2).unwrap_or(line) } else { line };
            let line = line.rsplit('\r').next().unwrap_or(line);
            let line = strip_ansi(line);
            // GitHub's `2026-10-12T23:09:53.1234567Z ` before the text
            match line.split_once(' ') {
                Some((stamp, rest)) if github && stamp.ends_with('Z') && stamp.contains('T') => rest.to_string(),
                _ => line,
            }
        })
        .collect();
    let end = lines.iter().rposition(|l| !l.trim().is_empty()).map_or(0, |i| i + 1);
    lines[end.saturating_sub(n)..end].join("\n")
}

/// `s` without its ANSI escape sequences.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        if chars.next() == Some('[') {
            // up to and with its final byte, `@` to `~`
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        }
    }
    out
}

/// Seconds from `started` to `finished`, or to `now` while running.
fn span(started: Option<&str>, finished: Option<&str>, state: CheckState, now: i64) -> Option<i64> {
    let start = unix_secs(started?)?;
    // GitHub writes a running check's end as year 1
    let end = finished.and_then(unix_secs).filter(|end| *end >= start).or((state == CheckState::Running).then_some(now))?;
    Some(end - start)
}

/// `42s`, `3m 12s`, `1h 4m`.
pub fn duration(secs: i64) -> String {
    match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m {}s", s / 60, s % 60),
        s => format!("{}h {}m", s / 3600, s / 60 % 60),
    }
}

fn auth_error(stderr: &str) -> bool {
    let e = stderr.to_ascii_lowercase();
    ["auth login", "401", "unauthorized", "not logged", "authenticat"].iter().any(|s| e.contains(s))
}

fn failure(tool: &'static str, host: &str, stderr: &str) -> Review {
    if auth_error(stderr) {
        return Review::NoLogin(tool, host.into());
    }
    Review::Failed(stderr.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("failed").into())
}

/// gh or glab with `args` for the repo at the located `dir`: run there, or for a machine's checkout run here against
/// its origin (`-R`, an API call's `--hostname`), with this Mac's login, so the host needs neither.
pub(crate) fn cli(name: &str, args: &[&str], dir: &str) -> std::io::Result<std::process::Output> {
    let mut cmd = crate::process::command(tool(name));
    cmd.stdin(std::process::Stdio::null());
    let (machine, path) = crate::herdr::split(dir);
    if machine.is_none() {
        return cmd.args(args).current_dir(path).output();
    }
    let (host, repo) = origin_of(dir).ok_or_else(|| std::io::Error::other("the checkout has no origin"))?;
    let mut args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    if args.first().map(String::as_str) == Some("api") {
        // glab fills `:id` in from the directory's repo; here it's the project's path
        args = args.into_iter().map(|a| a.replace(":id", &repo.replace('/', "%2F"))).collect();
        if !args.iter().any(|a| a == "--hostname") {
            args.extend(["--hostname".into(), host]);
        }
    } else if name == "gh" && args.starts_with(&["repo".into(), "view".into()]) {
        args.insert(2, format!("{host}/{repo}"));
    } else if !args.iter().any(|a| a == "-R") {
        args.extend(["-R".into(), if name == "gh" { format!("{host}/{repo}") } else { format!("https://{host}/{repo}") }]);
    }
    cmd.args(args).current_dir(std::env::home_dir().unwrap_or_else(|| "/".into())).output()
}

/// The host and project path of the located `dir`'s origin, asked once per directory.
fn origin_of(dir: &str) -> Option<(String, String)> {
    static SEEN: std::sync::Mutex<Option<std::collections::HashMap<String, (String, String)>>> = std::sync::Mutex::new(None);
    if let Some(got) = SEEN.lock().unwrap().get_or_insert_default().get(dir) {
        return Some(got.clone());
    }
    let out = crate::herdr::git(dir, &["remote", "get-url", "origin"]).ok()?;
    let url = String::from_utf8_lossy(&out.stdout).into_owned();
    let got = (host(&url)?, repo_path(&url)?);
    SEEN.lock().unwrap().get_or_insert_default().insert(dir.to_string(), got.clone());
    Some(got)
}

/// The project's path in `origin`'s URL: `group/app` from `git@host:group/app.git` or `ssh://git@host:22/group/app`.
pub fn repo_path(url: &str) -> Option<String> {
    let url = url.trim();
    let rest = match url.split_once("://") {
        Some((_, rest)) => rest.split_once('/')?.1,
        None => url.split_once(':')?.1,
    };
    let path = rest.trim_matches('/').trim_end_matches(".git");
    (!path.is_empty()).then(|| path.to_string())
}

/// The CLI's stdout and stderr; an error of kind `NotFound` when it is not installed.
fn run(name: &str, args: &[&str], dir: &str) -> Result<(String, String), std::io::Error> {
    let out = cli(name, args, dir)?;
    Ok((String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned()))
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// The review of a branch; `key` is its repo's directory and the branch, a line apart.
pub fn read_review(key: &str) -> Option<Review> {
    let (dir, branch) = key.split_once('\n')?;
    let origin = crate::herdr::git(dir, &["remote", "get-url", "origin"]).ok();
    let Some(host) = origin.and_then(|o| host(&String::from_utf8_lossy(&o.stdout))) else { return Some(Review::NoRemote) };
    // ponytail: by name; a GitHub Enterprise host without "github" in it goes to glab
    Some(if host.contains("github") { github(dir, branch, &host) } else { gitlab(dir, branch, &host) })
}

fn missing(e: &std::io::Error, tool: &'static str) -> Review {
    if e.kind() == std::io::ErrorKind::NotFound { Review::NoTool(tool) } else { Review::Failed(e.to_string()) }
}

/// What `gh pr view` is asked for.
const GITHUB_FIELDS: &str = "number,title,state,isDraft,url,author,body,statusCheckRollup,baseRefName,mergeable,latestReviews,reviewDecision,reviewRequests,autoMergeRequest,headRefName,updatedAt,labels,additions,deletions,changedFiles";

fn github(dir: &str, branch: &str, host: &str) -> Review {
    let (out, err) = match run("gh", &["pr", "view", branch, "--json", GITHUB_FIELDS], dir) {
        Ok(o) => o,
        Err(e) => return missing(&e, "gh"),
    };
    match parse_github(&out, &viewer(host, dir), now()) {
        Some((request, checks)) => Review::Found { request: Some(request), checks, pipeline: None },
        None if err.contains("no pull requests found") => Review::Found { request: None, checks: Vec::new(), pipeline: None },
        None => failure("gh", host, &err),
    }
}

fn gitlab(dir: &str, branch: &str, host: &str) -> Review {
    gitlab_graphql(dir, branch).unwrap_or_else(|| gitlab_rest(dir, branch, host))
}

/// The branch's review from [`GITLAB_QUERY`]; `None` when it can't be had (no origin, no login, an older GitLab), for
/// the REST calls to try and say why.
fn gitlab_graphql(dir: &str, branch: &str) -> Option<Review> {
    let (host, path) = origin_of(dir)?;
    let base = format!("https://{host}");
    let ask = |query: &str, var: String| {
        let out = run("glab", &["api", "graphql", "--hostname", &host, "-f", &format!("query={query}"), "-f", &format!("path={path}"), "-f", &var], dir).ok()?.0;
        Some(out)
    };
    let out = ask(GITLAB_QUERY, format!("branch={branch}"))?;
    let (request, pipeline, checks, iid) = parse_gitlab_review(&out, &base, &path, now())?;
    let checks = match iid {
        Some(iid) => ask(GITLAB_JOBS, format!("iid={iid}")).and_then(|out| parse_gitlab_jobs(&out, &base, &path, now())).unwrap_or_default(),
        None => checks,
    };
    Some(Review::Found { request, checks, pipeline })
}

/// The branch's review from glab's REST calls, one by one.
fn gitlab_rest(dir: &str, branch: &str, host: &str) -> Review {
    let (out, err) = match run("glab", &["mr", "view", branch, "-F", "json"], dir) {
        Ok(o) => o,
        Err(e) => return missing(&e, "glab"),
    };
    let mut request = parse_gitlab_mr(&out, "");
    if request.is_none() && auth_error(&err) {
        return Review::NoLogin("glab", host.into());
    }
    if let Some(r) = request.as_mut().filter(|r| r.pr.state == "open" || r.pr.state == "draft") {
        let path = format!("projects/:id/merge_requests/{}/approvals", r.pr.number);
        if let Ok((json, _)) = run("glab", &["api", &path], dir) {
            gitlab_approvals(&json, r);
        }
    }
    // no pipeline for the branch is an error to glab; the panel just shows none
    let Some(got) = run("glab", &["ci", "get", "-b", branch, "-F", "json"], dir).ok().map(|(out, _)| out) else {
        return Review::Found { request, checks: Vec::new(), pipeline: None };
    };
    let Some((pipeline, own)) = parse_gitlab_pipeline(&got, now()) else {
        return Review::Found { request, checks: Vec::new(), pipeline: None };
    };
    // its page, https://host/group/project/-/pipelines/1, names the host and the project
    let iid = serde_json::from_str::<Value>(&got).ok().and_then(|v| v["iid"].as_u64());
    let (base, path) = pipeline.0.split_once("/-/").and_then(|(project, _)| {
        let host_end = project.find("://").map(|i| i + 3)?;
        let slash = project[host_end..].find('/')? + host_end;
        Some((project[..slash].to_string(), project[slash + 1..].to_string()))
    }).unzip();
    let all = iid.zip(base.zip(path)).and_then(|(iid, (base, path))| {
        let query = format!("query={GITLAB_JOBS}");
        let (path_arg, iid_arg) = (format!("path={path}"), format!("iid={iid}"));
        let (out, _) = run("glab", &["api", "graphql", "-f", &query, "-f", &path_arg, "-f", &iid_arg], dir).ok()?;
        parse_gitlab_jobs(&out, &base, &path, now()).filter(|jobs| !jobs.is_empty())
    });
    Review::Found { request, checks: all.unwrap_or(own), pipeline: Some(pipeline) }
}

/// One thing that happened on a PR or MR, for the Timeline tab.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// Unix seconds.
    pub at: i64,
    /// Who did it; empty when the host doesn't say.
    pub who: String,
    pub what: Happened,
    /// What they wrote, markdown; empty for most.
    pub body: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Happened {
    Opened,
    Commented,
    Approved,
    Changes,
    /// A review without a verdict (GitHub's COMMENTED, DISMISSED).
    Reviewed,
    /// Commits pushed in a row by the same person: their short hash and first line.
    Commits(Vec<(String, String)>),
    Merged,
    Closed,
    /// Anything else GitLab notes by itself; its words are the body.
    Note,
}

/// What `gh pr view --json` is asked for the timeline.
const GITHUB_TIMELINE: &str = "author,createdAt,comments,reviews,commits,state,mergedAt,mergedBy,closedAt";

/// [`GITHUB_TIMELINE`]'s answer, read: the PR's events, oldest first.
pub fn parse_github_timeline(json: &str) -> Option<Vec<Event>> {
    let v: Value = serde_json::from_str(json).ok()?;
    let login = |who: &Value| who["login"].as_str().unwrap_or("").to_string();
    let at = |t: &Value| t.as_str().and_then(unix_secs);
    let mut out = Vec::new();
    if let Some(t) = at(&v["createdAt"]) {
        out.push(Event { at: t, who: login(&v["author"]), what: Happened::Opened, body: String::new() });
    }
    for c in v["comments"].as_array().into_iter().flatten() {
        let Some(t) = at(&c["createdAt"]) else { continue };
        out.push(Event { at: t, who: login(&c["author"]), what: Happened::Commented, body: c["body"].as_str().unwrap_or("").into() });
    }
    for r in v["reviews"].as_array().into_iter().flatten() {
        let Some(t) = at(&r["submittedAt"]) else { continue };
        let what = match r["state"].as_str() {
            Some("APPROVED") => Happened::Approved,
            Some("CHANGES_REQUESTED") => Happened::Changes,
            _ => Happened::Reviewed,
        };
        out.push(Event { at: t, who: login(&r["author"]), what, body: r["body"].as_str().unwrap_or("").into() });
    }
    for c in v["commits"].as_array().into_iter().flatten() {
        let Some(t) = at(&c["committedDate"]).or_else(|| at(&c["authoredDate"])) else { continue };
        let author = &c["authors"][0];
        let who = author["login"].as_str().filter(|l| !l.is_empty()).or(author["name"].as_str()).unwrap_or("");
        let sha: String = c["oid"].as_str().unwrap_or("").chars().take(7).collect();
        let line = c["messageHeadline"].as_str().unwrap_or("").to_string();
        out.push(Event { at: t, who: who.into(), what: Happened::Commits(vec![(sha, line)]), body: String::new() });
    }
    if let Some(t) = at(&v["mergedAt"]) {
        out.push(Event { at: t, who: login(&v["mergedBy"]), what: Happened::Merged, body: String::new() });
    } else if v["state"].as_str() == Some("CLOSED")
        && let Some(t) = at(&v["closedAt"])
    {
        out.push(Event { at: t, who: String::new(), what: Happened::Closed, body: String::new() });
    }
    Some(in_order(out))
}

/// `glab api projects/:id/merge_requests/<iid>` and its `/notes`, read: the MR's events, oldest first. GitLab notes
/// commits, approvals, merges and the rest itself, as system notes.
pub fn parse_gitlab_timeline(mr: &str, notes: &str) -> Option<Vec<Event>> {
    let mr: Value = serde_json::from_str(mr).ok()?;
    let notes: Value = serde_json::from_str(notes).ok()?;
    let mut out = Vec::new();
    if let Some(t) = mr["created_at"].as_str().and_then(unix_secs) {
        out.push(Event { at: t, who: mr["author"]["username"].as_str().unwrap_or("").into(), what: Happened::Opened, body: String::new() });
    }
    for n in notes.as_array()? {
        let Some(t) = n["created_at"].as_str().and_then(unix_secs) else { continue };
        let who = n["author"]["username"].as_str().unwrap_or("").to_string();
        let body = n["body"].as_str().unwrap_or("").trim().to_string();
        if n["system"].as_bool() != Some(true) {
            out.push(Event { at: t, who, what: Happened::Commented, body });
            continue;
        }
        let first = body.lines().next().unwrap_or("");
        let what = if first.starts_with("added ") && first.contains("commit") {
            Happened::Commits(gitlab_commits(&body))
        } else if first == "approved this merge request" {
            Happened::Approved
        } else if first == "requested changes" {
            Happened::Changes
        } else if first == "merged" {
            Happened::Merged
        } else if first == "closed" {
            Happened::Closed
        } else {
            out.push(Event { at: t, who, what: Happened::Note, body: first.to_string() });
            continue;
        };
        out.push(Event { at: t, who, what, body: String::new() });
    }
    Some(in_order(out))
}

/// A GitLab "added N commits" note's commits: `<li>abc12345 - Fix it</li>`.
fn gitlab_commits(body: &str) -> Vec<(String, String)> {
    body.split("<li>")
        .skip(1)
        .filter_map(|item| {
            let item = item.split("</li>").next()?;
            let (sha, line) = item.split_once(" - ")?;
            let sha = sha.trim();
            // "1 commit from branch …" lines aren't commits
            (!sha.contains(' ') && !sha.is_empty()).then(|| (sha.chars().take(7).collect(), line.trim().to_string()))
        })
        .collect()
}

/// Oldest first, commits pushed in a row by one person as one event.
fn in_order(mut events: Vec<Event>) -> Vec<Event> {
    events.sort_by_key(|e| e.at);
    let mut out: Vec<Event> = Vec::new();
    for e in events {
        if let (Some(last), Happened::Commits(more)) = (out.last_mut(), &e.what)
            && last.who == e.who
            && let Happened::Commits(run) = &mut last.what
        {
            run.extend(more.iter().cloned());
            last.at = e.at;
            continue;
        }
        out.push(e);
    }
    out
}

/// A PR's or MR's timeline; `key` is the located repo directory, `gitlab` or not and its number, a line apart. A
/// failure's line is the error.
pub fn read_timeline(key: &str) -> Option<Result<Vec<Event>, String>> {
    let mut parts = key.split('\n');
    let (dir, gitlab, number) = (parts.next()?, parts.next()? == "true", parts.next()?);
    let said = |(out, err): (String, String), parse: &dyn Fn(&str) -> Option<Vec<Event>>| {
        parse(&out).ok_or_else(|| err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("Couldn't read the timeline").to_string())
    };
    Some(if gitlab {
        let mr = run("glab", &["api", &format!("projects/:id/merge_requests/{number}")], dir).map_err(|e| e.to_string());
        let notes = run("glab", &["api", &format!("projects/:id/merge_requests/{number}/notes?sort=asc&order_by=created_at&per_page=100")], dir).map_err(|e| e.to_string());
        mr.and_then(|mr| notes.and_then(|notes| said(notes, &|n| parse_gitlab_timeline(&mr.0, n))))
    } else {
        run("gh", &["pr", "view", number, "--json", GITHUB_TIMELINE], dir).map_err(|e| e.to_string()).and_then(|o| said(o, &parse_github_timeline))
    })
}

/// The most of a PR's diff the Code tab reads.
const MAX_DIFF_BYTES: usize = 3 * 1024 * 1024;

/// An MR's patch, made from GitLab's API (`/diffs`, else the older `/changes`) rather than `glab mr diff`, whose
/// output and flags vary with its version; that is the last resort.
fn gitlab_patch(dir: &str, number: &str) -> Result<(String, String), std::io::Error> {
    let mut said = String::new();
    for path in [format!("projects/:id/merge_requests/{number}/diffs?per_page=100"), format!("projects/:id/merge_requests/{number}/changes")] {
        let (out, err) = run("glab", &["api", "--paginate", &path], dir)?;
        if let Some(patch) = gitlab_diffs_patch(&out) {
            return Ok((patch, String::new()));
        }
        said = if err.trim().is_empty() { out } else { err };
    }
    let (out, err) = run("glab", &["mr", "diff", number, "--raw"], dir)?;
    Ok(if out.trim().is_empty() { (out, if err.trim().is_empty() { said } else { err }) } else { (out, err) })
}

/// GitLab's MR diffs as one patch: `/diffs` pages (arrays, one after another with `--paginate`) or `/changes` (an
/// object with them under `changes`), each file with git's headers put back. `None` without any.
pub fn gitlab_diffs_patch(json: &str) -> Option<String> {
    let mut files = Vec::new();
    for page in serde_json::Deserializer::from_str(json).into_iter::<Value>() {
        let page = page.ok()?;
        match &page {
            Value::Array(list) => files.extend(list.iter().cloned()),
            Value::Object(o) if o.contains_key("changes") => files.extend(page["changes"].as_array().into_iter().flatten().cloned()),
            _ => return None,
        }
    }
    if files.is_empty() && !json.trim_start().starts_with('[') {
        return None;
    }
    let mut out = String::new();
    for f in &files {
        let (old, new) = (f["old_path"].as_str()?, f["new_path"].as_str()?);
        let (added, deleted, renamed) = (f["new_file"].as_bool() == Some(true), f["deleted_file"].as_bool() == Some(true), f["renamed_file"].as_bool() == Some(true));
        push_file(&mut out, old, new, added, deleted, renamed, f["diff"].as_str().unwrap_or(""));
    }
    Some(out)
}

/// A PR's patch from GitHub's list of its files (`pulls/N/files` pages, one after another with `--paginate`), for a
/// diff too big for `gh pr diff` (GitHub's stops at 20 000 lines). `None` without any.
// ponytail: GitHub lists 3000 files at most, and leaves out the patch of a file too big itself: it shows as binary
pub fn github_files_patch(json: &str) -> Option<String> {
    let mut out = String::new();
    for page in serde_json::Deserializer::from_str(json).into_iter::<Value>() {
        for f in page.ok()?.as_array()? {
            let new = f["filename"].as_str()?;
            let old = f["previous_filename"].as_str().unwrap_or(new);
            let status = f["status"].as_str().unwrap_or("");
            push_file(&mut out, old, new, status == "added", status == "removed", status == "renamed", f["patch"].as_str().unwrap_or(""));
        }
    }
    (!out.is_empty()).then_some(out)
}

/// One file's diff in a patch, with git's headers put back around the host's hunks.
fn push_file(out: &mut String, old: &str, new: &str, added: bool, deleted: bool, renamed: bool, diff: &str) {
    out.push_str(&format!("diff --git a/{old} b/{new}\n"));
    if added {
        out.push_str("new file mode 100644\n");
    }
    if deleted {
        out.push_str("deleted file mode 100644\n");
    }
    if renamed {
        out.push_str(&format!("rename from {old}\nrename to {new}\n"));
    }
    if diff.is_empty() {
        if !renamed {
            out.push_str(&format!("Binary files a/{old} and b/{new} differ\n"));
        }
        return;
    }
    out.push_str(&format!("--- {}\n+++ {}\n", if added { "/dev/null".into() } else { format!("a/{old}") }, if deleted { "/dev/null".into() } else { format!("b/{new}") }));
    out.push_str(diff);
    if !diff.ends_with('\n') {
        out.push('\n');
    }
}

/// A PR's patch: `gh pr diff`, else, when GitHub finds it too big, the patch made from its list of files.
fn github_patch(dir: &str, number: &str) -> Result<(String, String), std::io::Error> {
    let (out, err) = run("gh", &["pr", "diff", number, "--color", "never"], dir)?;
    if !(out.trim().is_empty() && err.contains("too_large")) {
        return Ok((out, err));
    }
    let path = format!("repos/{{owner}}/{{repo}}/pulls/{number}/files?per_page=100");
    let (files, files_err) = run("gh", &["api", "--paginate", &path], dir)?;
    Ok(match github_files_patch(&files) {
        Some(patch) => (patch, String::new()),
        None => (String::new(), if files_err.trim().is_empty() { err } else { files_err }),
    })
}

/// One commit's patch: git's here, where the branch has it (a merge against its first parent), else GitHub's.
fn commit_patch(dir: &str, gitlab: bool, sha: &str) -> Result<(String, String), std::io::Error> {
    let local = crate::herdr::git(dir, &["show", "--format=", "--no-color", "--diff-merges=first-parent", sha]).ok().filter(|o| o.status.success());
    if let Some(out) = local {
        return Ok((String::from_utf8_lossy(&out.stdout).into_owned(), String::new()));
    }
    if gitlab {
        return Ok((String::new(), format!("{} isn't here: fetch the branch to see it", &sha[..sha.len().min(7)])));
    }
    run("gh", &["api", &format!("repos/{{owner}}/{{repo}}/commits/{sha}"), "-H", "Accept: application/vnd.github.diff"], dir)
}

/// A PR's diff for the Code tab: the patch (the Changes view parses it again, as it does a checkout's), its files, and
/// whether it was cut at [`MAX_DIFF_BYTES`].
#[derive(Debug, Default, PartialEq)]
pub struct PrDiff {
    /// From the checkout's directory, which may be deep in the repo, to its root: the patch's paths are the root's.
    pub root: String,
    pub patch: String,
    pub files: Vec<crate::changes::FileDiff>,
    pub cut: bool,
}

/// A PR's or MR's commits, newest first: the full hash and the first line; `key` as [`read_timeline`]'s.
pub fn read_commits(key: &str) -> Option<Result<Vec<(String, String)>, String>> {
    let mut parts = key.split('\n');
    let (dir, gitlab, number) = (parts.next()?, parts.next()? == "true", parts.next()?);
    let got = if gitlab {
        run("glab", &["api", &format!("projects/:id/merge_requests/{number}/commits?per_page=100")], dir)
    } else {
        run("gh", &["pr", "view", number, "--json", "commits"], dir)
    };
    Some(got.map_err(|e| e.to_string()).and_then(|(out, err)| {
        parse_commits(&out, gitlab).ok_or_else(|| err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("Couldn't read the commits").to_string())
    }))
}

/// `gh pr view --json commits` (oldest first) or GitLab's MR commits (newest first), read newest first.
pub fn parse_commits(json: &str, gitlab: bool) -> Option<Vec<(String, String)>> {
    let v: Value = serde_json::from_str(json).ok()?;
    let list = if gitlab { v.as_array()? } else { v["commits"].as_array()? };
    let mut out: Vec<(String, String)> = list
        .iter()
        .filter_map(|c| {
            let sha = c["oid"].as_str().or(c["id"].as_str())?;
            let line = c["messageHeadline"].as_str().or(c["title"].as_str()).unwrap_or("");
            Some((sha.to_string(), line.to_string()))
        })
        .collect();
    if !gitlab {
        out.reverse();
    }
    Some(out)
}

/// One of an MR's pipelines, for the Pipeline tab's picker.
#[derive(Debug, Clone, PartialEq)]
pub struct PastPipeline {
    pub iid: String,
    pub state: CheckState,
    pub sha: String,
    /// Unix seconds it was created.
    pub at: Option<i64>,
    pub url: String,
}

/// An MR's pipelines, newest first; `key` is the PR's, `dir\ngitlab\nnumber`.
pub fn read_pipelines(key: &str) -> Option<Result<Vec<PastPipeline>, String>> {
    let mut parts = key.split('\n');
    let (dir, _, number) = (parts.next()?, parts.next()?, parts.next()?);
    let got = run("glab", &["api", &format!("projects/:id/merge_requests/{number}/pipelines?per_page=50")], dir);
    Some(got.map_err(|e| e.to_string()).and_then(|(out, err)| {
        parse_pipelines(&out).ok_or_else(|| err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("Couldn't read the pipelines").to_string())
    }))
}

/// GitLab's MR pipelines (newest first), read.
pub fn parse_pipelines(json: &str) -> Option<Vec<PastPipeline>> {
    let v: Value = serde_json::from_str(json).ok()?;
    let list = v.as_array()?.iter().filter_map(|p| {
        Some(PastPipeline {
            iid: p["iid"].as_u64()?.to_string(),
            state: check_state(p["status"].as_str()?),
            sha: p["sha"].as_str().unwrap_or("").into(),
            at: p["created_at"].as_str().and_then(unix_secs),
            url: p["web_url"].as_str()?.into(),
        })
    });
    Some(list.collect())
}

/// A pipeline's jobs and its child pipelines', by its iid; `key` is `dir\niid`.
pub fn read_pipeline_jobs(key: &str) -> Option<Result<Vec<Check>, String>> {
    let (dir, iid) = key.split_once('\n')?;
    let Some((host, path)) = origin_of(dir) else { return Some(Err("This repo has no origin to ask about.".into())) };
    let (query, path_arg, iid_arg) = (format!("query={GITLAB_JOBS}"), format!("path={path}"), format!("iid={iid}"));
    let got = run("glab", &["api", "graphql", "--hostname", &host, "-f", &query, "-f", &path_arg, "-f", &iid_arg], dir);
    Some(got.map_err(|e| e.to_string()).and_then(|(out, err)| {
        parse_gitlab_jobs(&out, &format!("https://{host}"), &path, now())
            .ok_or_else(|| err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("Couldn't read the pipeline").to_string())
    }))
}

/// What a file's diff says, as a key: a file viewed stays so until its diff changes.
pub fn fingerprint(f: &crate::changes::FileDiff) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for line in f.hunks.iter().flat_map(|h| &h.lines) {
        (line.kind as u8, &line.text).hash(&mut h);
    }
    format!("{}\t{:016x}", f.path, h.finish())
}

/// A PR's or MR's diff as the host has it; `key` as [`read_timeline`]'s, with a commit after it for that commit's own
/// diff (from git here, else from GitHub).
pub fn read_diff(key: &str) -> Option<Result<std::sync::Arc<PrDiff>, String>> {
    let mut parts = key.split('\n');
    let (dir, gitlab, number) = (parts.next()?, parts.next()? == "true", parts.next()?);
    let commit = parts.next().filter(|c| !c.is_empty());
    let got = match commit {
        Some(sha) => commit_patch(dir, gitlab, sha),
        None if gitlab => gitlab_patch(dir, number),
        None => github_patch(dir, number),
    };
    Some(got.map_err(|e| e.to_string()).and_then(|(mut patch, err)| {
        if patch.trim().is_empty() && !err.trim().is_empty() {
            // glab's first line can be a bare "ERROR": what it says comes after
            let said: Vec<&str> = err.lines().map(str::trim).filter(|l| !l.is_empty()).take(3).collect();
            return Err(if said.is_empty() { "Couldn't read the diff".into() } else { said.join(" ") });
        }
        let cut = patch.len() > MAX_DIFF_BYTES;
        if cut {
            let end = patch[..patch.floor_char_boundary(MAX_DIFF_BYTES)].rfind('\n').unwrap_or(0);
            patch.truncate(end);
        }
        let files = crate::changes::parse_patch(&patch);
        let root = crate::changes::repo_root(dir).unwrap_or_default();
        Ok(std::sync::Arc::new(PrDiff { root, patch, files, cut }))
    }))
}

/// The comments on a PR's or MR's lines as the host has them, oldest first, each a [`crate::changes::ReviewComment`]
/// with its author in `by`; `key` as [`read_timeline`]'s.
// ponytail: the first 100 of them, and a GitLab note outdated by a later push shows on its old line numbers
pub fn read_line_comments(key: &str) -> Option<Result<Vec<crate::changes::ReviewComment>, String>> {
    let mut parts = key.split('\n');
    let (dir, gitlab, number) = (parts.next()?, parts.next()? == "true", parts.next()?);
    let got = if gitlab {
        run("glab", &["api", &format!("projects/:id/merge_requests/{number}/discussions?per_page=100")], dir)
    } else {
        run("gh", &["api", &format!("repos/{{owner}}/{{repo}}/pulls/{number}/comments?per_page=100")], dir)
    };
    Some(got.map_err(|e| e.to_string()).and_then(|(out, err)| {
        let parsed = if gitlab { parse_gitlab_line_comments(&out) } else { parse_github_line_comments(&out) };
        parsed.ok_or_else(|| err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("Couldn't read the comments").to_string())
    }))
}

/// A host comment's id as a card's: apart from the ones staged here, which count up from 1.
fn posted_id(id: u64) -> u64 {
    id | 1 << 63
}

/// `gh api repos/…/pulls/<n>/comments`, read: the comments on lines the diff still has (an outdated one's `line` is
/// null), on their side.
pub fn parse_github_line_comments(json: &str) -> Option<Vec<crate::changes::ReviewComment>> {
    use crate::changes::{CommentSide, ReviewComment};
    let list: Vec<Value> = serde_json::from_str(json).ok()?;
    Some(
        list.iter()
            .filter_map(|c| {
                Some(ReviewComment {
                    id: posted_id(c["id"].as_u64()?),
                    path: c["path"].as_str()?.to_string(),
                    old_path: None,
                    side: if c["side"].as_str() == Some("LEFT") { CommentSide::Old } else { CommentSide::New },
                    line: c["line"].as_u64()? as u32,
                    body: c["body"].as_str().unwrap_or_default().to_string(),
                    review: true,
                    by: Some(c["user"]["login"].as_str().unwrap_or_default().to_string()),
                })
            })
            .collect(),
    )
}

/// `glab api projects/:id/merge_requests/<n>/discussions`, read: the notes placed on a diff line, on the new side when
/// the line has a new number.
pub fn parse_gitlab_line_comments(json: &str) -> Option<Vec<crate::changes::ReviewComment>> {
    use crate::changes::{CommentSide, ReviewComment};
    let list: Vec<Value> = serde_json::from_str(json).ok()?;
    let notes = list.iter().flat_map(|d| d["notes"].as_array().into_iter().flatten());
    Some(
        notes
            .filter(|n| n["system"].as_bool() != Some(true) && n["position"]["position_type"].as_str() == Some("text"))
            .filter_map(|n| {
                let p = &n["position"];
                let (side, line) = match (p["new_line"].as_u64(), p["old_line"].as_u64()) {
                    (Some(line), _) => (CommentSide::New, line),
                    (None, Some(line)) => (CommentSide::Old, line),
                    (None, None) => return None,
                };
                let path = p["new_path"].as_str().or(p["old_path"].as_str())?.to_string();
                let old_path = p["old_path"].as_str().filter(|o| *o != path).map(str::to_string);
                Some(ReviewComment {
                    id: posted_id(n["id"].as_u64()?),
                    path,
                    old_path,
                    side,
                    line: line as u32,
                    body: n["body"].as_str().unwrap_or_default().to_string(),
                    review: true,
                    by: Some(n["author"]["username"].as_str().unwrap_or_default().to_string()),
                })
            })
            .collect(),
    )
}

/// How a review is sent: with a verdict, or as comments only. GitLab has no "request changes".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewEvent {
    Comment,
    Approve,
    RequestChanges,
}

/// A comment of a review, with both of its line's numbers (GitLab places a note on an unchanged line by both).
pub struct ReviewNote {
    pub comment: crate::changes::ReviewComment,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
}

/// GitHub's `POST /pulls/{n}/reviews` body: the verdict, the summary and the comments, each on its line's side.
pub fn github_review(verdict: ReviewEvent, body: &str, notes: &[ReviewNote]) -> Value {
    let event = match verdict {
        ReviewEvent::Comment => "COMMENT",
        ReviewEvent::Approve => "APPROVE",
        ReviewEvent::RequestChanges => "REQUEST_CHANGES",
    };
    let comments: Vec<Value> = notes
        .iter()
        .map(|n| {
            let side = if n.comment.side == crate::changes::CommentSide::Old { "LEFT" } else { "RIGHT" };
            serde_json::json!({ "path": n.comment.path, "line": n.comment.line, "side": side, "body": n.comment.body })
        })
        .collect();
    let mut review = serde_json::json!({ "event": event, "comments": comments });
    if !body.trim().is_empty() {
        review["body"] = body.trim().into();
    }
    review
}

/// A GitLab draft note's body: the note on its line, placed against the MR's `diff_refs` (base, start, head).
pub fn gitlab_draft(note: &ReviewNote, refs: &Value) -> Value {
    let c = &note.comment;
    let mut position = serde_json::json!({
        "position_type": "text",
        "base_sha": refs["base_sha"],
        "start_sha": refs["start_sha"],
        "head_sha": refs["head_sha"],
        "old_path": c.old_path.as_deref().unwrap_or(&c.path),
        "new_path": c.path,
    });
    if let Some(n) = note.old_no {
        position["old_line"] = n.into();
    }
    if let Some(n) = note.new_no {
        position["new_line"] = n.into();
    }
    serde_json::json!({ "note": c.body, "position": position })
}

/// Sends a review of the PR or MR at `url` (checked out in `dir`): GitHub takes it in one request; on GitLab each
/// comment is a draft note, then they're published together, the summary is a note and an approval is its own call.
/// An error leaves nothing on the host; `Ok(Some(why))` is a GitLab summary or approval that failed after the comments
/// went out, so they aren't sent again.
pub fn submit_review(dir: &str, url: &str, gitlab: bool, number: u64, verdict: ReviewEvent, body: &str, notes: &[ReviewNote]) -> Result<Option<String>, String> {
    let input = |payload: &Value| -> Result<std::path::PathBuf, String> {
        let path = std::env::temp_dir().join(format!("shepherd-review-{}-{number}.json", std::process::id()));
        std::fs::write(&path, payload.to_string()).map_err(|e| e.to_string())?;
        Ok(path)
    };
    let args = |list: &[&str]| list.iter().map(|a| a.to_string()).collect::<Vec<_>>();
    if !gitlab {
        let host = host(url).ok_or("no host in the PR's link")?;
        let repo = url.split_once("/pull/").and_then(|(project, _)| repo_path(project)).ok_or("no repo in the PR's link")?;
        let file = input(&github_review(verdict, body, notes))?;
        let done = act(dir, &args(&["gh", "api", "-X", "POST", "--hostname", &host, &format!("repos/{repo}/pulls/{number}/reviews"), "--input", &file.to_string_lossy()]));
        let _ = std::fs::remove_file(file);
        return done.map(|_| None);
    }
    let mr = format!("projects/:id/merge_requests/{number}");
    if !notes.is_empty() {
        let (out, err) = run("glab", &["api", &mr], dir).map_err(|e| e.to_string())?;
        let refs = serde_json::from_str::<Value>(&out).ok().map(|v| v["diff_refs"].clone()).filter(|r| r.is_object());
        let refs = refs.ok_or_else(|| err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("Couldn't read the MR").to_string())?;
        // the drafts made so far go again if a later one fails: a retry would post them twice
        let mut made: Vec<u64> = Vec::new();
        let undo = |made: &[u64], why: String| {
            for id in made {
                let _ = act(dir, &args(&["glab", "api", "-X", "DELETE", &format!("{mr}/draft_notes/{id}")]));
            }
            why
        };
        for note in notes {
            let file = input(&gitlab_draft(note, &refs)).map_err(|why| undo(&made, why))?;
            let posted = run("glab", &["api", "-X", "POST", &format!("{mr}/draft_notes"), "-H", "Content-Type: application/json", "--input", &file.to_string_lossy()], dir);
            let _ = std::fs::remove_file(file);
            let id = posted.as_ref().ok().and_then(|(out, _)| serde_json::from_str::<Value>(out).ok()?["id"].as_u64());
            match id {
                Some(id) => made.push(id),
                None => {
                    let why = posted.map_or_else(|e| e.to_string(), |(_, err)| err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("Couldn't add a comment").to_string());
                    return Err(undo(&made, why));
                }
            }
        }
        act(dir, &args(&["glab", "api", "-X", "POST", &format!("{mr}/draft_notes/bulk_publish")])).map_err(|why| undo(&made, why))?;
    }
    // something already on the host makes a later failure a partial send, not an error to retry
    let mut wrote = !notes.is_empty();
    let partial = |wrote: bool, why: String| if wrote { Ok(Some(why)) } else { Err(why) };
    if !body.trim().is_empty() {
        if let Err(why) = act(dir, &args(&["glab", "api", "-X", "POST", &format!("{mr}/notes"), "-f", &format!("body={}", body.trim())])) {
            return partial(wrote, why);
        }
        wrote = true;
    }
    if verdict == ReviewEvent::Approve
        && let Err(why) = approve(dir, true, number)
    {
        return partial(wrote, why);
    }
    Ok(None)
}

pub use zeron_markdown::{Block, InlineRun};

/// `text` as Zeron's markdown blocks (pulldown-cmark), for a panel without HTML: comments (GitHub's templates are
/// full of them) and tags (`<details>`, `<br>`) dropped; a task's `[x]` as a check box.
pub fn markdown(text: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = zeron_markdown::parse_full(&img_tags(&strip_comments(text))).blocks.iter().map(|b| b.block.clone()).collect();
    blocks.iter_mut().for_each(tidy);
    blocks.retain(|b| !matches!(b, Block::Paragraph { runs } if runs_text(runs).trim().is_empty() && !has_image(runs)));
    blocks
}

fn has_image(runs: &[InlineRun]) -> bool {
    runs.iter().any(|r| r.style.image.is_some())
}

fn tidy(block: &mut Block) {
    match block {
        Block::Paragraph { runs } | Block::Heading { runs, .. } => tidy_runs(runs),
        Block::BlockQuote { children } => children.iter_mut().for_each(tidy),
        Block::List { items, .. } => items.iter_mut().flatten().for_each(tidy),
        Block::Table { header, rows, .. } => header.iter_mut().chain(rows.iter_mut().flatten()).for_each(tidy_runs),
        Block::CodeBlock { .. } | Block::Rule => {}
    }
}

fn tidy_runs(runs: &mut Vec<InlineRun>) {
    for run in runs.iter_mut() {
        if let Some(task) = &run.style.task {
            run.text = if task.checked { "☑ " } else { "☐ " }.into();
        } else if !run.style.code {
            let text = strip_tags(&run.text);
            // a tag's own line (`<details>`) leaves only its line breaks
            if text != run.text {
                run.text = text.trim().to_string();
            }
        }
    }
    runs.retain(|r| !r.text.is_empty() || r.style.image.is_some());
}

/// The runs' text, as it reads.
pub fn runs_text(runs: &[InlineRun]) -> String {
    runs.iter().map(|r| r.text.as_str()).collect()
}

fn strip_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        rest = rest[start..].find("-->").map_or("", |end| &rest[start + end + 3..]);
    }
    out + rest
}

/// `<img src="…" alt="…">` (what GitHub writes for a dropped screenshot) as markdown's `![alt](src)`: the tags go
/// otherwise, the image with them.
fn img_tags(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<img ").or_else(|| rest.find("<IMG ")) {
        let Some(end) = rest[start..].find('>') else { break };
        let tag = &rest[start..start + end + 1];
        out.push_str(&rest[..start]);
        let attr = |name: &str| {
            let at = tag.find(&format!("{name}="))? + name.len() + 1;
            let quote = tag[at..].chars().next().filter(|c| *c == '"' || *c == '\'')?;
            let value = &tag[at + 1..];
            Some(value[..value.find(quote)?].to_string())
        };
        match attr("src") {
            Some(src) => out.push_str(&format!("![{}]({})", attr("alt").unwrap_or_default().replace(['[', ']'], ""), src.replace(' ', "%20"))),
            None => out.push_str(tag),
        }
        rest = &rest[start + end + 1..];
    }
    out + rest
}

/// The images' sources in `blocks`, each once, in document order.
pub fn image_sources(blocks: &[Block], out: &mut Vec<String>) {
    let runs = |runs: &[InlineRun], out: &mut Vec<String>| {
        for image in runs.iter().filter_map(|r| r.style.image.as_ref()) {
            if !out.contains(&image.source) {
                out.push(image.source.clone());
            }
        }
    };
    for block in blocks {
        match block {
            Block::Paragraph { runs: r } | Block::Heading { runs: r, .. } => runs(r, out),
            Block::BlockQuote { children } => image_sources(children, out),
            Block::List { items, .. } => items.iter().for_each(|item| image_sources(item, out)),
            Block::Table { header, rows, .. } => header.iter().chain(rows.iter().flatten()).for_each(|cell| runs(cell, out)),
            _ => {}
        }
    }
}

/// An image in a PR's description or comments, fetched into a file here: `None` when it couldn't be.
#[derive(Debug, Clone, PartialEq)]
pub struct PrImage(pub Option<std::path::PathBuf>);

/// The most of an image the panel fetches.
const MAX_IMAGE_BYTES: u64 = 16 << 20;

/// Fetches an image of a PR's text; `key` is the located repo directory, `gitlab` or not, the PR's page and the
/// image's source, a line apart. A GitLab upload (`/uploads/…`) is under the project's page; one on the PR's own host
/// goes with the CLI's token, as a private repo's attachments need.
pub fn read_image(key: &str) -> Option<PrImage> {
    let mut parts = key.splitn(4, '\n');
    let (dir, gitlab, page, source) = (parts.next()?, parts.next()? == "true", parts.next()?, parts.next()?);
    let url = if source.starts_with("https://") || source.starts_with("http://") {
        source.to_string()
    } else if gitlab && source.starts_with("/uploads/") {
        let project = page.split_once("/-/").map_or(page, |(p, _)| p);
        format!("{project}{source}")
    } else {
        return Some(PrImage(None));
    };
    let pr_host = host(page);
    let token = (host(&url) == pr_host).then(|| pr_host.as_deref().and_then(|h| cli_token(dir, gitlab, h))).flatten();
    Some(PrImage(fetch_image(&url, gitlab, token.as_deref())))
}

/// The CLI's token for `host`, asked once per run.
fn cli_token(dir: &str, gitlab: bool, host: &str) -> Option<String> {
    static SEEN: std::sync::Mutex<Option<std::collections::HashMap<String, Option<String>>>> = std::sync::Mutex::new(None);
    if let Some(token) = SEEN.lock().unwrap().get_or_insert_default().get(host) {
        return token.clone();
    }
    let out = if gitlab { run("glab", &["config", "get", "token", "--host", host], dir) } else { run("gh", &["auth", "token", "--hostname", host], dir) };
    let token = out.ok().map(|(out, _)| out.trim().to_string()).filter(|t| !t.is_empty() && !t.contains(' '));
    SEEN.lock().unwrap().get_or_insert_default().insert(host.to_string(), token.clone());
    token
}

fn fetch_image(url: &str, gitlab: bool, token: Option<&str>) -> Option<std::path::PathBuf> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(15))).build().into();
    let mut request = agent.get(url);
    if let Some(token) = token {
        request = if gitlab { request.header("PRIVATE-TOKEN", token) } else { request.header("Authorization", format!("token {token}")) };
    }
    let mut response = request.call().ok()?;
    let kind = response.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
    if !kind.starts_with("image/") {
        return None;
    }
    let bytes = response.body_mut().with_config().limit(MAX_IMAGE_BYTES).read_to_vec().ok()?;
    let ext = ["svg", "png", "gif", "webp", "jpeg"].into_iter().find(|e| kind.contains(e)).unwrap_or("png");
    let hash = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut h);
        h.finish()
    };
    let folder = crate::settings::cache_dir().join("files").join(format!("{hash:016x}"));
    std::fs::create_dir_all(&folder).ok()?;
    let path = folder.join(format!("image.{ext}"));
    std::fs::write(&path, bytes).ok()?;
    Some(path)
}

/// `s` without its HTML tags (`<br>`, `</details>`); a `<` not opening one stays.
fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find('<') {
        let after = &rest[start + 1..];
        out.push_str(&rest[..start]);
        match after.find('>').filter(|_| after.starts_with(|c: char| c.is_ascii_alphabetic() || c == '/')) {
            Some(end) => rest = &after[end + 1..],
            None => {
                out.push('<');
                rest = after;
            }
        }
    }
    out + rest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_hosts() {
        assert_eq!(host("git@github.com:freethinkel/shepherd.git").as_deref(), Some("github.com"));
        assert_eq!(host("ssh://git@ci.example.net:10022/team/app.git\n").as_deref(), Some("ci.example.net"));
        assert_eq!(host("https://oauth2:token@GitLab.com/a/b.git").as_deref(), Some("gitlab.com"));
        assert_eq!(host("https://github.com/a/b").as_deref(), Some("github.com"));
        assert_eq!(host(""), None);
        assert_eq!(repo_path("git@github.com:freethinkel/shepherd.git").as_deref(), Some("freethinkel/shepherd"));
        assert_eq!(repo_path("ssh://git@ci.example.net:10022/team/app.git\n").as_deref(), Some("team/app"));
        assert_eq!(repo_path("https://github.com/a/b").as_deref(), Some("a/b"));
    }

    #[test]
    fn states_from_both_hosts() {
        assert_eq!(check_state("SUCCESS"), CheckState::Success);
        assert_eq!(check_state("failed"), CheckState::Failed);
        assert_eq!(check_state("IN_PROGRESS"), CheckState::Running);
        assert_eq!(check_state("QUEUED"), CheckState::Pending);
        assert_eq!(check_state("created"), CheckState::Pending);
        assert_eq!(check_state("manual"), CheckState::Skipped);
        assert_eq!(check_state("CANCELLED"), CheckState::Skipped);
    }

    #[test]
    fn a_github_pr_and_its_checks() {
        let json = r#"{"number":7,"title":"Fix it","state":"OPEN","isDraft":false,"url":"https://github.com/a/b/pull/7",
            "author":{"login":"egor"},"body":"Hi","baseRefName":"main","mergeable":"CONFLICTING","reviewDecision":"APPROVED",
            "headRefName":"fix","updatedAt":"2026-10-12T23:00:00Z","labels":[{"name":"bug"}],"additions":1200,"deletions":3,"changedFiles":4,
            "latestReviews":[{"author":{"login":"anna"},"state":"APPROVED"}],
            "statusCheckRollup":[
              {"__typename":"CheckRun","name":"lint","workflowName":"CI","status":"COMPLETED","conclusion":"FAILURE",
               "startedAt":"2026-10-12T23:09:53Z","completedAt":"2026-10-12T23:11:45Z","detailsUrl":"https://github.com/a/b/actions/runs/38/job/115"},
              {"__typename":"CheckRun","name":"test","workflowName":"CI","status":"IN_PROGRESS","conclusion":"",
               "startedAt":"2026-10-12T23:10:00Z","completedAt":"0001-01-01T00:00:00Z","detailsUrl":"https://x/2"},
              {"__typename":"StatusContext","context":"ci/jenkins","state":"PENDING","targetUrl":"https://j"}]}"#;
        let now = unix_secs("2026-10-12T23:12:00Z").unwrap();
        let (request, checks) = parse_github(json, "anna", now).unwrap();
        assert_eq!((request.base.as_str(), request.mergeable, request.can_approve, request.approved, request.accepted), ("main", false, true, true, true));
        assert!(!parse_github(json, "egor", now).unwrap().0.can_approve, "not one's own");
        assert_eq!((request.head.as_str(), request.updated, request.labels.as_slice()), ("fix", unix_secs("2026-10-12T23:00:00Z"), ["bug".to_string()].as_slice()));
        assert_eq!(request.diff, Some(Diff { files: 4, additions: 1200, deletions: 3 }));
        assert_eq!((request.title.as_str(), request.author.as_str(), request.pr.state, request.pr.number), ("Fix it", "egor", "open", 7));
        let lint = Check {
            name: "lint".into(),
            page: "CI".into(),
            group: String::new(),
            state: CheckState::Failed,
            secs: Some(112),
            url: Some("https://github.com/a/b/actions/runs/38/job/115".into()),
            job: Some("115".into()),
            run: Some("38".into()),
            manual: false,
            bridge: false,
            child: None,
            repo: None,
        };
        assert_eq!(checks[0], lint);
        assert_eq!(lint.command(false, lint.action()).unwrap().join(" "), "gh run rerun --job 115");
        assert_eq!(retry_failed(&[lint.clone(), lint.clone()], false), [["gh", "run", "rerun", "38", "--failed"]], "one call per run");
        assert_eq!(checks[2].command(false, checks[2].action()), None, "a commit status has nothing to act on");
        assert_eq!((checks[1].state, checks[1].secs), (CheckState::Running, Some(120)), "a running check counts up to now");
        assert_eq!((checks[2].name.as_str(), checks[2].state, checks[2].secs), ("ci/jenkins", CheckState::Pending, None));
        let review = Review::Found { request: Some(request), checks, pipeline: None };
        assert!(review.running());
        assert_eq!(review.ci(), Some(CheckState::Failed));
        assert_eq!(parse_github("", "", now), None);
    }

    #[test]
    fn a_gitlab_mr_and_its_pipeline() {
        let mr = r#"{"iid":93,"title":"SMS gates","state":"opened","draft":true,"web_url":"https://gl/mr/93",
            "author":{"username":"vlad"},"description":"Body","target_branch":"dev","has_conflicts":false,
            "source_branch":"sms","labels":["backend"],"updated_at":"2026-10-12T23:00:00.000Z"}"#;
        let mut request = parse_gitlab_mr(mr, "").unwrap();
        assert_eq!((request.base.as_str(), request.mergeable), ("dev", true));
        assert_eq!((request.head.as_str(), request.labels.as_slice(), request.diff), ("sms", ["backend".to_string()].as_slice(), None), "glab's REST labels are strings");
        gitlab_approvals(r#"{"approved":true,"user_has_approved":true,"user_can_approve":false}"#, &mut request);
        assert_eq!((request.can_approve, request.approved, request.accepted), (false, true, true));
        assert_eq!((request.pr.state, request.pr.gitlab, request.author.as_str(), request.body.as_str()), ("draft", true, "vlad", "Body"));
        let pipeline = r#"{"id":1,"status":"running","web_url":"https://gl/p/1","jobs":[
            {"id":3,"name":"deploy","stage":"deploy","status":"manual","duration":0,"web_url":"https://gl/j/3"},
            {"id":2,"name":"test","stage":"test","status":"running","duration":null,"started_at":"2026-10-12T23:10:00.5Z","web_url":"https://gl/j/2"},
            {"id":1,"name":"build","stage":"build","status":"success","duration":63.4,"web_url":"https://gl/j/1"}]}"#;
        let now = unix_secs("2026-10-12T23:11:00Z").unwrap();
        let (pipeline, jobs) = parse_gitlab_pipeline(pipeline, now).unwrap();
        assert_eq!(pipeline, ("https://gl/p/1".to_string(), CheckState::Running));
        let passed = Review::Found { request: None, checks: Vec::new(), pipeline: Some(pipeline) };
        assert!(passed.running(), "a pipeline still running its child pipelines is polled though its own jobs passed");
        let got: Vec<_> = jobs.iter().map(|j| (j.name.as_str(), j.state, j.secs)).collect();
        assert_eq!(got, [("build", CheckState::Success, Some(63)), ("test", CheckState::Running, Some(60)), ("deploy", CheckState::Skipped, None)]);
        let calls: Vec<_> = jobs.iter().map(|j| j.command(true, j.action()).unwrap().join(" ")).collect();
        assert_eq!(calls, ["glab ci retry 1", "glab ci cancel job 2", "glab ci trigger 3"]);
        assert_eq!(passed.ci(), Some(CheckState::Running));
        assert_eq!(pages(&jobs), [(ROOT.into(), CheckState::Running)]);
        assert_eq!(default_page(&[("a".into(), CheckState::Success), ("b".into(), CheckState::Failed)]), 1);
        assert_eq!(default_page(&[("a".into(), CheckState::Success), ("b".into(), CheckState::Running)]), 1);
        assert_eq!(default_page(&[("a".into(), CheckState::Success), ("b".into(), CheckState::Success)]), 0, "the first when all passed");
        assert_eq!(default_page(&[]), 0);
    }

    #[test]
    fn a_gitlab_review_in_one_query() {
        let json = r#"{"data":{"currentUser":{"username":"egor"},"project":{"mergeRequests":{"nodes":[{
            "iid":"402","title":"Hide maps","state":"opened","draft":false,"webUrl":"https://gl/g/p/-/merge_requests/402",
            "description":"Body","targetBranch":"develop","sourceBranch":"maps","conflicts":false,
            "labels":{"nodes":[{"title":"web"}]},"diffStatsSummary":{"additions":10,"deletions":2,"fileCount":3},"autoMergeEnabled":true,"squash":true,
            "forceRemoveSourceBranch":true,"author":{"username":"egor"},"approved":true,
            "approvedBy":{"nodes":[{"username":"dima"}]},"userPermissions":{"canApprove":true},
            "discussions":{"nodes":[{"resolvable":true,"resolved":false},{"resolvable":true,"resolved":true},{"resolvable":false,"resolved":false}]},
            "reviewers":{"nodes":[{"username":"vlad","mergeRequestInteraction":{"reviewState":"REQUESTED_CHANGES"}},
                                  {"username":"anna","mergeRequestInteraction":{"reviewState":"UNREVIEWED"}}]},
            "headPipeline":{"status":"RUNNING","path":"/g/p/-/pipelines/7","stages":{"nodes":[{"name":"test"}]},
              "jobs":{"nodes":[{"id":"gid://gitlab/Ci::Build/1","name":"unit","status":"RUNNING","duration":null,"webPath":"/g/p/-/jobs/1","stage":{"name":"test"}}]},
              "downstream":{"nodes":[]}}}]},
            "pipelines":{"nodes":[]}}}}"#;
        let (request, pipeline, checks, iid) = parse_gitlab_review(json, "https://gl", "g/p", 0).unwrap();
        let r = request.unwrap();
        assert_eq!((r.pr.number, r.pr.state, r.base.as_str(), r.mergeable, r.approved, r.accepted), (402, "open", "develop", true, false, true));
        assert_eq!((r.unresolved, r.auto_merge, r.squash, r.remove_branch), (Some(1), true, true, true));
        assert_eq!((r.head.as_str(), r.labels.as_slice(), r.diff), ("maps", ["web".to_string()].as_slice(), Some(Diff { files: 3, additions: 10, deletions: 2 })));
        let said: Vec<_> = r.reviewers.iter().map(|rv| (rv.name.as_str(), rv.verdict)).collect();
        assert_eq!(said, [("vlad", Verdict::Changes), ("anna", Verdict::Pending), ("dima", Verdict::Approved)], "an approver who wasn't asked counts too");
        assert_eq!(pipeline, Some(("https://gl/g/p/-/pipelines/7".into(), CheckState::Running)));
        assert_eq!((checks.len(), iid), (1, None));
        let none = r#"{"data":{"currentUser":{"username":"egor"},"project":{"mergeRequests":{"nodes":[]},
            "pipelines":{"nodes":[{"iid":"55","status":"SUCCESS","path":"/g/p/-/pipelines/9"}]}}}}"#;
        let (request, pipeline, _, iid) = parse_gitlab_review(none, "https://gl", "g/p", 0).unwrap();
        assert_eq!((request, pipeline.map(|p| p.1), iid.as_deref()), (None, Some(CheckState::Success), Some("55")), "the branch's own pipeline, its jobs asked for apart");
    }

    #[test]
    fn a_gitlab_mr_diff_from_its_api() {
        let pages = r#"[{"old_path":"a.rs","new_path":"a.rs","new_file":false,"renamed_file":false,"deleted_file":false,"diff":"@@ -1,2 +1,2 @@\n-x\n+y\n z\n"}]
            [{"old_path":"b.rs","new_path":"b.rs","new_file":true,"renamed_file":false,"deleted_file":false,"diff":"@@ -0,0 +1 @@\n+new\n"},
             {"old_path":"old.txt","new_path":"new.txt","new_file":false,"renamed_file":true,"deleted_file":false,"diff":""}]"#;
        let files = crate::changes::parse_patch(&gitlab_diffs_patch(pages).unwrap());
        let got: Vec<_> = files.iter().map(|f| (f.path.as_str(), f.additions, f.deletions)).collect();
        assert!(got.contains(&("a.rs", 1, 1)) && got.contains(&("b.rs", 1, 0)), "{got:?}");
        assert!(files.iter().any(|f| f.path == "new.txt" && f.old_path.as_deref() == Some("old.txt")), "a rename keeps both paths");
        assert_eq!(gitlab_diffs_patch(r#"{"message":"404 Not found"}"#), None, "an error isn't a diff");
        let changes = r#"{"changes":[{"old_path":"c.rs","new_path":"c.rs","diff":"@@ -1 +1 @@\n-a\n+b\n"}]}"#;
        assert_eq!(crate::changes::parse_patch(&gitlab_diffs_patch(changes).unwrap()).len(), 1);
    }

    #[test]
    fn a_github_pr_diff_from_its_files() {
        let pages = r#"[{"filename":"a.rs","status":"modified","patch":"@@ -1,2 +1,2 @@\n-x\n+y\n z"}]
            [{"filename":"b.rs","status":"added","patch":"@@ -0,0 +1 @@\n+new"},
             {"filename":"new.txt","previous_filename":"old.txt","status":"renamed"},
             {"filename":"big.bin","status":"modified"}]"#;
        let files = crate::changes::parse_patch(&github_files_patch(pages).unwrap());
        let got: Vec<_> = files.iter().map(|f| (f.path.as_str(), f.additions, f.deletions)).collect();
        assert!(got.contains(&("a.rs", 1, 1)) && got.contains(&("b.rs", 1, 0)), "{got:?}");
        assert!(files.iter().any(|f| f.path == "new.txt" && f.old_path.as_deref() == Some("old.txt")), "a rename keeps both paths");
        assert!(files.iter().any(|f| f.path == "big.bin"), "a file without its patch is still listed");
        assert_eq!(github_files_patch(r#"{"message":"Not Found"}"#), None, "an error isn't a diff");
    }

    #[test]
    fn commits_newest_first() {
        let gh = r#"{"commits":[{"oid":"aaa","messageHeadline":"One"},{"oid":"bbb","messageHeadline":"Two"}]}"#;
        assert_eq!(parse_commits(gh, false).unwrap(), [("bbb".to_string(), "Two".to_string()), ("aaa".to_string(), "One".to_string())]);
        let gl = r#"[{"id":"ccc","title":"Three"},{"id":"ddd","title":"Four"}]"#;
        assert_eq!(parse_commits(gl, true).unwrap()[0].0, "ccc", "GitLab's come newest first already");
    }

    #[test]
    fn img_tags_become_images() {
        let body = "Before\n\n<img width=\"600\" alt=\"The panel\" src=\"https://github.com/user-attachments/assets/abc\" />\n\nafter";
        let mut sources = Vec::new();
        image_sources(&markdown(body), &mut sources);
        assert_eq!(sources, ["https://github.com/user-attachments/assets/abc"]);
        assert_eq!(img_tags("<img src='a.png'> and <b>bold</b>"), "![](a.png) and <b>bold</b>");
    }

    #[test]
    fn a_github_timeline() {
        let json = r#"{"author":{"login":"egor"},"createdAt":"2026-10-12T10:00:00Z","state":"MERGED",
            "mergedAt":"2026-10-12T15:00:00Z","mergedBy":{"login":"anna"},"closedAt":"2026-10-12T15:00:00Z",
            "comments":[{"author":{"login":"anna"},"body":"Looks good","createdAt":"2026-10-12T12:00:00Z"}],
            "reviews":[{"author":{"login":"anna"},"state":"APPROVED","body":"","submittedAt":"2026-10-12T14:00:00Z"}],
            "commits":[
              {"oid":"aaaaaaaaaa","messageHeadline":"One","committedDate":"2026-10-12T10:30:00Z","authors":[{"login":"egor","name":"Egor"}]},
              {"oid":"bbbbbbbbbb","messageHeadline":"Two","committedDate":"2026-10-12T11:00:00Z","authors":[{"login":"egor","name":"Egor"}]},
              {"oid":"cccccccccc","messageHeadline":"Three","committedDate":"2026-10-12T13:00:00Z","authors":[{"login":"","name":"Egor K"}]}]}"#;
        let events = parse_github_timeline(json).unwrap();
        let got: Vec<_> = events.iter().map(|e| (e.who.as_str(), e.what.clone())).collect();
        assert_eq!(
            got,
            [
                ("egor", Happened::Opened),
                ("egor", Happened::Commits(vec![("aaaaaaa".into(), "One".into()), ("bbbbbbb".into(), "Two".into())])),
                ("anna", Happened::Commented),
                ("Egor K", Happened::Commits(vec![("ccccccc".into(), "Three".into())])),
                ("anna", Happened::Approved),
                ("anna", Happened::Merged),
            ],
            "oldest first, commits in a row by one person together"
        );
        assert_eq!(events[2].body, "Looks good");
    }

    #[test]
    fn a_gitlab_timeline() {
        let mr = r#"{"created_at":"2026-10-12T10:00:00.000Z","author":{"username":"vlad"}}"#;
        let notes = r#"[
            {"body":"added 2 commits\n\n<ul><li>1234567890 - First</li><li>abcdef1234 - Second</li></ul>\n\n[Compare with previous version](x)","author":{"username":"vlad"},"created_at":"2026-10-12T11:00:00Z","system":true},
            {"body":"Why this?","author":{"username":"egor"},"created_at":"2026-10-12T12:00:00Z","system":false},
            {"body":"approved this merge request","author":{"username":"egor"},"created_at":"2026-10-12T13:00:00Z","system":true},
            {"body":"changed the description","author":{"username":"vlad"},"created_at":"2026-10-12T14:00:00Z","system":true}]"#;
        let events = parse_gitlab_timeline(mr, notes).unwrap();
        let got: Vec<_> = events.iter().map(|e| (e.who.as_str(), e.what.clone(), e.body.as_str())).collect();
        assert_eq!(
            got,
            [
                ("vlad", Happened::Opened, ""),
                ("vlad", Happened::Commits(vec![("1234567".into(), "First".into()), ("abcdef1".into(), "Second".into())]), ""),
                ("egor", Happened::Commented, "Why this?"),
                ("egor", Happened::Approved, ""),
                ("vlad", Happened::Note, "changed the description"),
            ]
        );
    }

    #[test]
    fn logs_as_a_terminal_shows_them() {
        let gitlab = "section_start:1:step\r\u{1b}[0K\u{1b}[32;1mRunning\u{1b}[0;m\nold\rnew\n\n";
        assert_eq!(tail_log(gitlab, false, 5), "Running\nnew");
        let github = "build\tRun tests\t2026-10-12T23:09:53.1234567Z ok 1\nbuild\tRun tests\t2026-10-12T23:09:54.0Z ok 2";
        assert_eq!(tail_log(github, true, 1), "ok 2");
    }

    #[test]
    fn an_mrs_pipelines_newest_first() {
        let json = r#"[{"id":2,"iid":12,"sha":"a2cc560e","status":"running","created_at":"2026-10-19T14:16:20.412Z","web_url":"https://gl/g/p/-/pipelines/2"},
            {"id":1,"iid":11,"sha":"28a615af","status":"failed","created_at":"2026-10-19T13:57:18.949Z","web_url":"https://gl/g/p/-/pipelines/1"},
            {"id":0,"status":"failed"}]"#;
        let got = parse_pipelines(json).unwrap();
        assert_eq!(got.iter().map(|p| (p.iid.as_str(), p.state, p.sha.as_str())).collect::<Vec<_>>(), [("12", CheckState::Running, "a2cc560e"), ("11", CheckState::Failed, "28a615af")]);
        assert!(got[0].at > got[1].at && got[1].url.ends_with("/1"));
    }

    #[test]
    fn a_gitlab_pipeline_with_child_pipelines() {
        let json = r#"{"data":{"project":{"pipeline":{
            "stages":{"nodes":[{"name":"lint"},{"name":"repos"}]},
            "jobs":{"nodes":[
              {"id":"gid://gitlab/Ci::Bridge/9","name":"repos:web","status":"FAILED","duration":null,"webPath":"/g/p/-/jobs/9","stage":{"name":"repos"}},
              {"id":"gid://gitlab/Ci::Build/8","name":"lint:docs","status":"SUCCESS","duration":12,"webPath":"/g/p/-/jobs/8","stage":{"name":"lint"}}]},
            "downstream":{"nodes":[{
              "sourceJob":{"id":"gid://gitlab/Ci::Bridge/9","name":"repos:web"},"project":{"fullPath":"g/p"},
              "stages":{"nodes":[{"name":"test"},{"name":"deploy"}]},
              "jobs":{"nodes":[
                {"id":"gid://gitlab/Ci::Build/12","name":"deploy","status":"MANUAL","duration":null,"webPath":"/g/p/-/jobs/12","stage":{"name":"deploy"}},
                {"id":"gid://gitlab/Ci::Build/11","name":"unit","status":"FAILED","duration":40,"webPath":"/g/p/-/jobs/11","stage":{"name":"test"}}]},
              "downstream":{"nodes":[]}}]}}}}}"#;
        let jobs = parse_gitlab_jobs(json, "https://gl", "g/p", 0).unwrap();
        let got: Vec<_> = jobs.iter().map(|j| (j.page.as_str(), j.group.as_str(), j.name.as_str(), j.state)).collect();
        assert_eq!(got, [
            (ROOT, "lint", "lint:docs", CheckState::Success),
            (ROOT, "repos", "repos:web", CheckState::Failed),
            ("repos:web", "test", "unit", CheckState::Failed),
            ("repos:web", "deploy", "deploy", CheckState::Skipped),
        ], "stage by stage, the child pipeline after its parent");
        assert_eq!((jobs[1].bridge, jobs[1].child.as_deref()), (true, Some("repos:web")));
        assert_eq!(jobs[1].command(true, Action::Retry), None, "a trigger job is not the CLI's to retry");
        assert_eq!((jobs[2].url.as_deref(), jobs[2].job.as_deref(), jobs[2].secs), (Some("https://gl/g/p/-/jobs/11"), Some("11"), Some(40)));
        assert!(jobs[3].manual);
        let staged = pages(&jobs);
        assert_eq!(staged, [(ROOT.into(), CheckState::Success), ("repos:web".into(), CheckState::Failed)], "a child's failure is its own page's");
        assert_eq!(default_page(&staged), 1);
        assert_eq!(jobs[2].command(true, Action::Retry).unwrap().join(" "), "glab ci retry 11", "its own project's needs no -R");
        let other = json.replace(r#""fullPath":"g/p""#, r#""fullPath":"infra/deploy""#);
        let jobs = parse_gitlab_jobs(&other, "https://gl", "g/p", 0).unwrap();
        assert_eq!(jobs[2].command(true, Action::Retry).unwrap().join(" "), "glab ci retry 11 -R https://gl/infra/deploy", "another project's on its host");
    }

    #[test]
    fn durations() {
        assert_eq!(duration(42), "42s");
        assert_eq!(duration(192), "3m 12s");
        assert_eq!(duration(3840), "1h 4m");
    }

    #[test]
    fn errors_read_as_states() {
        assert_eq!(failure("gh", "github.com", "To get started with GitHub CLI, please run:  gh auth login"), Review::NoLogin("gh", "github.com".into()));
        assert_eq!(failure("glab", "gl", "\nGET https://gl/api/v4/user: 401 {message: 401 Unauthorized}"), Review::NoLogin("glab", "gl".into()));
        assert_eq!(failure("gh", "github.com", "\n  could not resolve host\nmore"), Review::Failed("could not resolve host".into()));
    }

    #[test]
    fn markdown_blocks() {
        let text = "<!-- template\nhint -->\n## Summary\n\nFixes **the** `login` [bug](https://x/1).\n\n- one\n- [x] done\n\n<details><summary>More</summary>\n\na < b";
        let blocks = markdown(text);
        assert!(matches!(&blocks[0], Block::Heading { level: 2, runs } if runs_text(runs) == "Summary"));
        let Block::Paragraph { runs } = &blocks[1] else { panic!("{blocks:?}") };
        assert_eq!(runs_text(runs), "Fixes the login bug.");
        assert!(runs.iter().any(|r| r.text == "the" && r.style.bold));
        assert!(runs.iter().any(|r| r.text == "login" && r.style.code));
        assert!(runs.iter().any(|r| r.text == "bug" && r.style.link.as_deref() == Some("https://x/1")));
        assert!(matches!(&blocks[2], Block::List { items, .. } if items.len() == 2));
        assert!(matches!(&blocks[3], Block::Paragraph { runs } if runs_text(runs) == "More"));
        assert!(matches!(&blocks[4], Block::Paragraph { runs } if runs_text(runs) == "a < b"));
        assert_eq!(markdown(""), []);
        assert_eq!(markdown("<!-- only a hint -->"), []);
    }

    #[test]
    fn a_review_puts_each_comment_on_its_side_and_line() {
        use crate::changes::{CommentSide, ReviewComment};
        let note = |side, line, old_no, new_no| ReviewNote {
            comment: ReviewComment { id: 1, path: "src/new.rs".into(), old_path: Some("src/old.rs".into()), side, line, body: "why?".into(), review: true, by: None },
            old_no,
            new_no,
        };
        let notes = [note(CommentSide::Old, 7, Some(7), None), note(CommentSide::New, 9, Some(8), Some(9))];
        let gh = github_review(ReviewEvent::RequestChanges, "  ", &notes);
        assert_eq!(gh["event"], "REQUEST_CHANGES");
        assert!(gh.get("body").is_none(), "an empty summary is left out");
        assert_eq!((gh["comments"][0]["side"].as_str(), gh["comments"][0]["line"].as_u64()), (Some("LEFT"), Some(7)));
        assert_eq!((gh["comments"][1]["side"].as_str(), gh["comments"][1]["path"].as_str()), (Some("RIGHT"), Some("src/new.rs")));
        let refs = serde_json::json!({ "base_sha": "b", "start_sha": "s", "head_sha": "h" });
        let deleted = gitlab_draft(&notes[0], &refs)["position"].clone();
        assert_eq!((deleted["old_path"].as_str(), deleted["old_line"].as_u64(), deleted.get("new_line")), (Some("src/old.rs"), Some(7), None));
        let unchanged = gitlab_draft(&notes[1], &refs)["position"].clone();
        assert_eq!((unchanged["old_line"].as_u64(), unchanged["new_line"].as_u64(), unchanged["head_sha"].as_str()), (Some(8), Some(9), Some("h")));
    }

    #[test]
    fn the_hosts_line_comments_land_on_their_lines() {
        use crate::changes::CommentSide;
        let gh = r#"[{"id":7,"path":"a.rs","line":10,"side":"RIGHT","body":"why?","user":{"login":"anna"}},
                     {"id":8,"path":"a.rs","line":null,"side":"RIGHT","body":"outdated","user":{"login":"anna"}},
                     {"id":9,"path":"a.rs","line":3,"side":"LEFT","body":"gone","user":{"login":"egor"}}]"#;
        let got = parse_github_line_comments(gh).unwrap();
        assert_eq!(got.iter().map(|c| (c.side, c.line, c.by.as_deref())).collect::<Vec<_>>(), [(CommentSide::New, 10, Some("anna")), (CommentSide::Old, 3, Some("egor"))]);
        assert!(got[0].id > u32::MAX as u64, "apart from the ones staged here");
        let gl = r#"[{"notes":[{"id":1,"system":true,"body":"added 1 commit"}]},
                     {"notes":[{"id":2,"body":"here","author":{"username":"anna"},"position":{"position_type":"text","old_path":"b.rs","new_path":"c.rs","old_line":4,"new_line":null}},
                               {"id":3,"body":"and here","author":{"username":"egor"},"position":{"position_type":"text","old_path":"c.rs","new_path":"c.rs","old_line":5,"new_line":6}}]}]"#;
        let got = parse_gitlab_line_comments(gl).unwrap();
        assert_eq!(got.iter().map(|c| (c.side, c.line, c.path.as_str(), c.old_path.as_deref())).collect::<Vec<_>>(), [(CommentSide::Old, 4, "c.rs", Some("b.rs")), (CommentSide::New, 6, "c.rs", None)]);
    }
}
