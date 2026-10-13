//! The review panel's data: the shown branch's GitHub PR (`gh`) or GitLab MR (`glab`), picked by origin's host, with
//! its checks or its pipeline's jobs, and the description as a few markdown blocks. The CLIs run off the main thread,
//! through the status bar's cache.
use crate::ui::statusbar::{Pr, parse_pr, tool, unix_secs};
use serde_json::Value;

/// What the panel shows for a branch.
#[derive(Debug, Clone, PartialEq)]
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
    /// Its workflow, or its stage.
    pub group: String,
    pub state: CheckState,
    /// How long it ran, or has been running, when it was fetched.
    pub secs: Option<i64>,
    pub url: Option<String>,
}

impl Review {
    /// Something still runs or waits to: the panel asks again soon.
    pub fn running(&self) -> bool {
        let busy = |s: CheckState| matches!(s, CheckState::Running | CheckState::Pending);
        // a pipeline's own state counts too: its child pipelines' jobs are not among its own
        matches!(self, Review::Found { checks, pipeline, .. } if checks.iter().any(|c| busy(c.state)) || pipeline.as_ref().is_some_and(|(_, s)| busy(*s)))
    }
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

/// `origin`'s host, from `git@host:a/b.git`, `ssh://git@host:22/a/b`, `https://user:token@host/a/b`.
pub fn host(url: &str) -> Option<String> {
    let rest = url.trim().split_once("://").map_or(url.trim(), |(_, r)| r);
    let authority = rest.split('/').next()?;
    let host = authority.rsplit('@').next()?.split(':').next()?;
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// `gh pr view --json number,title,state,isDraft,url,author,body,statusCheckRollup`, read; durations up to `now`.
pub fn parse_github(json: &str, now: i64) -> Option<(Request, Vec<Check>)> {
    let v: Value = serde_json::from_str(json).ok()?;
    let request = request(&v, json, "login", "body")?;
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
    Some(Check { name: name.into(), group: group.into(), state, secs: span(c["startedAt"].as_str(), c["completedAt"].as_str(), state, now), url: url.map(Into::into) })
}

/// `glab mr view -F json`, read.
pub fn parse_gitlab_mr(json: &str) -> Option<Request> {
    request(&serde_json::from_str(json).ok()?, json, "username", "description")
}

// ponytail: a child pipeline's jobs are left out (its trigger job too); its parent's state and page cover them
/// `glab ci get -F json`, read: the pipeline's page and state, and its jobs in the order they run.
pub fn parse_gitlab_pipeline(json: &str, now: i64) -> Option<((String, CheckState), Vec<Check>)> {
    let v: Value = serde_json::from_str(json).ok()?;
    let pipeline = (v["web_url"].as_str()?.to_string(), check_state(v["status"].as_str().unwrap_or("")));
    let mut jobs: Vec<Check> = v["jobs"]
        .as_array()
        .map(|all| {
            all.iter()
                .filter_map(|j| {
                    let state = check_state(j["status"].as_str()?);
                    let secs = j["duration"].as_f64().filter(|d| *d > 0.).map(|d| d.round() as i64);
                    Some(Check {
                        name: j["name"].as_str()?.into(),
                        group: j["stage"].as_str().unwrap_or("").into(),
                        state,
                        secs: secs.or_else(|| span(j["started_at"].as_str(), j["finished_at"].as_str(), state, now)),
                        url: j["web_url"].as_str().map(Into::into),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    // GitLab lists the newest job first
    jobs.reverse();
    Some((pipeline, jobs))
}

fn request(v: &Value, json: &str, login: &str, body: &str) -> Option<Request> {
    Some(Request {
        pr: parse_pr(json)?,
        title: v["title"].as_str().unwrap_or("").into(),
        author: v["author"][login].as_str().unwrap_or("").into(),
        body: v[body].as_str().unwrap_or("").into(),
    })
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

/// The CLI's stdout and stderr; an error of kind `NotFound` when it is not installed.
fn run(name: &str, args: &[&str], dir: &str) -> Result<(String, String), std::io::Error> {
    let out = std::process::Command::new(tool(name)).args(args).current_dir(dir).stdin(std::process::Stdio::null()).output()?;
    Ok((String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned()))
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// The review of a branch; `key` is its repo's directory and the branch, a line apart.
pub fn read_review(key: &str) -> Option<Review> {
    let (dir, branch) = key.split_once('\n')?;
    let origin = std::process::Command::new("git").args(["-C", dir, "remote", "get-url", "origin"]).output().ok();
    let Some(host) = origin.and_then(|o| host(&String::from_utf8_lossy(&o.stdout))) else { return Some(Review::NoRemote) };
    // ponytail: by name; a GitHub Enterprise host without "github" in it goes to glab
    Some(if host.contains("github") { github(dir, branch, &host) } else { gitlab(dir, branch, &host) })
}

fn missing(e: &std::io::Error, tool: &'static str) -> Review {
    if e.kind() == std::io::ErrorKind::NotFound { Review::NoTool(tool) } else { Review::Failed(e.to_string()) }
}

fn github(dir: &str, branch: &str, host: &str) -> Review {
    let fields = "number,title,state,isDraft,url,author,body,statusCheckRollup";
    let (out, err) = match run("gh", &["pr", "view", branch, "--json", fields], dir) {
        Ok(o) => o,
        Err(e) => return missing(&e, "gh"),
    };
    match parse_github(&out, now()) {
        Some((request, checks)) => Review::Found { request: Some(request), checks, pipeline: None },
        None if err.contains("no pull requests found") => Review::Found { request: None, checks: Vec::new(), pipeline: None },
        None => failure("gh", host, &err),
    }
}

fn gitlab(dir: &str, branch: &str, host: &str) -> Review {
    let (out, err) = match run("glab", &["mr", "view", branch, "-F", "json"], dir) {
        Ok(o) => o,
        Err(e) => return missing(&e, "glab"),
    };
    let request = parse_gitlab_mr(&out);
    if request.is_none() && auth_error(&err) {
        return Review::NoLogin("glab", host.into());
    }
    // no pipeline for the branch is an error to glab; the panel just shows none
    let pipeline = run("glab", &["ci", "get", "-b", branch, "-F", "json"], dir).ok().and_then(|(out, _)| parse_gitlab_pipeline(&out, now()));
    let (pipeline, checks) = pipeline.map_or((None, Vec::new()), |(pipeline, jobs)| (Some(pipeline), jobs));
    Review::Found { request, checks, pipeline }
}

/// A block of a PR description, as the panel draws it.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading(String),
    Paragraph(String),
    /// A list item: its nesting, its marker (`•`, `3.`, `☐`, `☑`) and text.
    Item(usize, String, String),
    Quote(String),
    Code(String),
    Rule,
}

// ponytail: a few block kinds, inline marks stripped; a markdown crate when tables or styled spans are wanted
/// `text` as markdown blocks: headings, lists, quotes, fenced code, rules and paragraphs; HTML comments (GitHub's
/// templates are full of them) and tags dropped, links reduced to their text.
pub fn markdown(text: &str) -> Vec<Block> {
    let text = strip_comments(text);
    let mut blocks = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();
    let mut code: Option<Vec<&str>> = None;
    let flush = |p: &mut Vec<String>, blocks: &mut Vec<Block>| {
        if !p.is_empty() {
            blocks.push(Block::Paragraph(p.join(" ")));
            p.clear();
        }
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(lines) = code.as_mut() {
            if trimmed.starts_with("```") {
                blocks.push(Block::Code(lines.join("\n")));
                code = None;
            } else {
                lines.push(line);
            }
            continue;
        }
        if trimmed.starts_with("```") {
            flush(&mut paragraph, &mut blocks);
            code = Some(Vec::new());
            continue;
        }
        let indent = (line.len() - line.trim_start().len()) / 2;
        let item = |marker: &str, rest: &str| Block::Item(indent, marker.into(), inline(rest));
        let block = if trimmed.is_empty() {
            None
        } else if trimmed.starts_with('#') && trimmed.trim_start_matches('#').starts_with(' ') {
            Some(Block::Heading(inline(trimmed.trim_start_matches('#'))))
        } else if ["---", "***", "___"].contains(&trimmed) {
            Some(Block::Rule)
        } else if let Some(rest) = trimmed.strip_prefix("> ").or(trimmed.strip_prefix('>')) {
            Some(Block::Quote(inline(rest)))
        } else if let Some(rest) = ["- ", "* ", "+ "].iter().find_map(|m| trimmed.strip_prefix(m)) {
            Some(match rest.get(..4) {
                Some("[ ] ") => item("☐", &rest[4..]),
                Some("[x] " | "[X] ") => item("☑", &rest[4..]),
                _ => item("•", rest),
            })
        } else if let Some((n, rest)) = trimmed.split_once(". ").filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())) {
            Some(item(&format!("{n}."), rest))
        } else {
            paragraph.push(inline(trimmed));
            continue;
        };
        flush(&mut paragraph, &mut blocks);
        blocks.extend(block);
    }
    flush(&mut paragraph, &mut blocks);
    if let Some(lines) = code {
        blocks.push(Block::Code(lines.join("\n")));
    }
    blocks.retain(|b| !matches!(b, Block::Paragraph(p) | Block::Heading(p) if p.trim().is_empty()));
    blocks
}

/// The text of `blocks` as one run, headings, rules and code left out: a folded description's few lines.
pub fn preview(blocks: &[Block]) -> String {
    let text = blocks.iter().filter_map(|b| match b {
        Block::Paragraph(t) | Block::Quote(t) => Some(t.clone()),
        Block::Item(_, marker, t) => Some(format!("{marker} {t}")),
        Block::Heading(_) | Block::Code(_) | Block::Rule => None,
    });
    text.collect::<Vec<_>>().join(" ")
}

// ponytail: by size, not by laid-out lines; a long single line in a narrow panel may fold without needing to
/// Whether a description has more than its folded lines show: more than one block, or a paragraph longer than
/// about four lines of a panel.
pub fn folds(blocks: &[Block]) -> bool {
    blocks.len() > 1 || preview(blocks).chars().count() > 160
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

/// A line's text without its marks: `**`, `__`, backticks, HTML tags; `[text](url)` and `![alt](url)` as their text.
fn inline(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s.trim();
    while let Some(c) = rest.chars().next() {
        let after = &rest[c.len_utf8()..];
        // `[text](url)`: the text
        if c == '[' || (c == '!' && after.starts_with('[')) {
            let open = if c == '!' { &after[1..] } else { after };
            if let Some((text, tail)) = open.split_once("](") {
                if let Some(end) = tail.find(')') {
                    out.push_str(text);
                    rest = &tail[end + 1..];
                    continue;
                }
            }
        }
        // an HTML tag: `<br>`, `</details>`
        if c == '<' && after.starts_with(|n: char| n.is_ascii_alphabetic() || n == '/') {
            if let Some(end) = after.find('>') {
                rest = &after[end + 1..];
                continue;
            }
        }
        if !(c == '`' || ((c == '*' || c == '_') && after.starts_with(c))) {
            out.push(c);
            rest = after;
        } else {
            rest = after.strip_prefix(c).filter(|_| c != '`').unwrap_or(after);
        }
    }
    out.trim().to_string()
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
            "author":{"login":"egor"},"body":"Hi",
            "statusCheckRollup":[
              {"__typename":"CheckRun","name":"lint","workflowName":"CI","status":"COMPLETED","conclusion":"FAILURE",
               "startedAt":"2026-10-12T23:09:53Z","completedAt":"2026-10-12T23:11:45Z","detailsUrl":"https://x/1"},
              {"__typename":"CheckRun","name":"test","workflowName":"CI","status":"IN_PROGRESS","conclusion":"",
               "startedAt":"2026-10-12T23:10:00Z","completedAt":"0001-01-01T00:00:00Z","detailsUrl":"https://x/2"},
              {"__typename":"StatusContext","context":"ci/jenkins","state":"PENDING","targetUrl":"https://j"}]}"#;
        let now = unix_secs("2026-10-12T23:12:00Z").unwrap();
        let (request, checks) = parse_github(json, now).unwrap();
        assert_eq!((request.title.as_str(), request.author.as_str(), request.pr.state, request.pr.number), ("Fix it", "egor", "open", 7));
        assert_eq!(checks[0], Check { name: "lint".into(), group: "CI".into(), state: CheckState::Failed, secs: Some(112), url: Some("https://x/1".into()) });
        assert_eq!((checks[1].state, checks[1].secs), (CheckState::Running, Some(120)), "a running check counts up to now");
        assert_eq!((checks[2].name.as_str(), checks[2].state, checks[2].secs), ("ci/jenkins", CheckState::Pending, None));
        let review = Review::Found { request: Some(request), checks, pipeline: None };
        assert!(review.running());
        assert_eq!(parse_github("", now), None);
    }

    #[test]
    fn a_gitlab_mr_and_its_pipeline() {
        let mr = r#"{"iid":93,"title":"SMS gates","state":"opened","draft":true,"web_url":"https://gl/mr/93",
            "author":{"username":"vlad"},"description":"Body"}"#;
        let request = parse_gitlab_mr(mr).unwrap();
        assert_eq!((request.pr.state, request.pr.gitlab, request.author.as_str(), request.body.as_str()), ("draft", true, "vlad", "Body"));
        let pipeline = r#"{"id":1,"status":"running","web_url":"https://gl/p/1","jobs":[
            {"name":"deploy","stage":"deploy","status":"manual","duration":0,"web_url":"https://gl/j/3"},
            {"name":"test","stage":"test","status":"running","duration":null,"started_at":"2026-10-12T23:10:00.5Z","web_url":"https://gl/j/2"},
            {"name":"build","stage":"build","status":"success","duration":63.4,"web_url":"https://gl/j/1"}]}"#;
        let now = unix_secs("2026-10-12T23:11:00Z").unwrap();
        let (pipeline, jobs) = parse_gitlab_pipeline(pipeline, now).unwrap();
        assert_eq!(pipeline, ("https://gl/p/1".to_string(), CheckState::Running));
        let passed = Review::Found { request: None, checks: Vec::new(), pipeline: Some(pipeline) };
        assert!(passed.running(), "a pipeline still running its child pipelines is polled though its own jobs passed");
        let got: Vec<_> = jobs.iter().map(|j| (j.name.as_str(), j.state, j.secs)).collect();
        assert_eq!(got, [("build", CheckState::Success, Some(63)), ("test", CheckState::Running, Some(60)), ("deploy", CheckState::Skipped, None)]);
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
        let text = "<!-- template\nhint -->\n## Summary\n\nFixes **the** `login` [bug](https://x/1).\nSecond line.\n\n- one\n  - nested\n- [x] done\n1. first\n> quoted\n\n```rust\nlet a = 1;\n```\n---\n<details><summary>More</summary>";
        assert_eq!(
            markdown(text),
            [
                Block::Heading("Summary".into()),
                Block::Paragraph("Fixes the login bug. Second line.".into()),
                Block::Item(0, "•".into(), "one".into()),
                Block::Item(1, "•".into(), "nested".into()),
                Block::Item(0, "☑".into(), "done".into()),
                Block::Item(0, "1.".into(), "first".into()),
                Block::Quote("quoted".into()),
                Block::Code("let a = 1;".into()),
                Block::Rule,
                Block::Paragraph("More".into()),
            ]
        );
        assert_eq!(markdown("a_b * c ![img](u)"), [Block::Paragraph("a_b * c img".into())]);
        assert_eq!(preview(&markdown(text)), "Fixes the login bug. Second line. • one • nested ☑ done 1. first quoted More");
        assert!(folds(&markdown(text)));
        assert!(!folds(&markdown("One short line.")));
        assert_eq!(markdown(""), []);
    }
}
