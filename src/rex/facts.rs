//! What Settings → Backend says of Rex, from `rex`, which starts no server for it.
use crate::backend::{Fact, Job};
use anyhow::Context;
use serde_json::Value;

const RELOAD: Job = Job { label: "Reload", args: &["config", "reload"] };
const SKILLS: Job = Job { label: "Set up", args: &["skill", "setup"] };
const TERMINFO: Job = Job { label: "Install", args: &["terminfo", "setup"] };

/// Runs `rex`. Blocks.
pub fn run(args: &[&str]) -> anyhow::Result<String> {
    let binary = super::binary().context("Rex isn't installed")?;
    crate::process::answer(crate::process::command(binary).args(args).arg("--autostart=false").output().context("cannot run rex")?)
}

fn json(args: &[&str]) -> anyhow::Result<Value> {
    Ok(serde_json::from_str(&run(args)?)?)
}

fn server(who: anyhow::Result<Value>) -> Fact {
    let (title, detail) = match who {
        Ok(who) => {
            let tailscale = match (who["tailscale"]["state"].as_str(), who["tailscale"]["dns_name"].as_str()) {
                (Some("running"), Some(name)) => format!("Tailscale: {name}"),
                _ => "Tailscale off".to_string(),
            };
            (format!("Rex {}", who["rex_version"].as_str().unwrap_or("")), format!("{} sessions · {tailscale}", who["session_count"].as_u64().unwrap_or(0)))
        }
        Err(e) => ("Rex".to_string(), format!("{e:#}")),
    };
    Fact { title, detail, ..Default::default() }
}

/// `rex config check`'s `key: value` lines, or why it failed.
fn config(check: anyhow::Result<String>) -> Fact {
    let actions = vec![RELOAD];
    let check = match check {
        Ok(check) => check,
        Err(e) => return Fact { title: "Config".into(), detail: format!("{e:#}"), actions, ..Default::default() },
    };
    let field = |key: &str| check.lines().find_map(|l| l.strip_prefix(key)?.strip_prefix(": "));
    let path = field("path").unwrap_or("init.lua");
    let detail = match field("exists") {
        Some("no") => format!("{path} · no file yet"),
        _ => format!("{path} · {} bindings, {} actions", field("bindings").unwrap_or("0"), field("actions").unwrap_or("0")),
    };
    Fact { title: "Config".into(), detail, actions, file: Some(path.into()) }
}

/// The agents `rex skill status` found, and those it has left to set up.
fn agents(status: &str) -> (Vec<&str>, Vec<&str>) {
    let (mut found, mut behind) = (Vec::new(), Vec::new());
    let mut agent = None;
    for line in status.lines() {
        if let Some(name) = line.strip_suffix(" (found)") {
            found.push(name);
            agent = Some(name);
        } else if !line.starts_with(' ') {
            agent = None; // one not on this machine
        } else if let Some(name) = agent.filter(|_| !matches!(line.split_whitespace().nth(1), Some("current" | "unsupported")))
            && behind.last() != Some(&name)
        {
            behind.push(name);
        }
    }
    (found, behind)
}

fn skills(status: anyhow::Result<String>) -> Fact {
    let title = "Agent skill".to_string();
    let status = match status {
        Ok(status) => status,
        Err(e) => return Fact { title, detail: format!("{e:#}"), ..Default::default() },
    };
    let (found, behind) = agents(&status);
    match (found.is_empty(), behind.is_empty()) {
        (true, _) => Fact { title, detail: "No agents found on this machine.".into(), ..Default::default() },
        (_, true) => Fact { title, detail: found.join(", "), ..Default::default() },
        _ => Fact { title, detail: format!("Not set up: {}", behind.join(", ")), actions: vec![SKILLS], ..Default::default() },
    }
}

/// `rex terminfo status`'s first line, `term: <what a new terminal gets> (why)`.
fn terminfo(status: anyhow::Result<String>) -> Fact {
    let title = "Terminfo".to_string();
    let status = match status {
        Ok(status) => status,
        Err(e) => return Fact { title, detail: format!("{e:#}"), ..Default::default() },
    };
    let line = status.lines().next().unwrap_or_default().trim_start_matches("term: ");
    match line.contains("not installed") {
        true => Fact { title, detail: format!("{line}. Install adds an Include line to ~/.ssh/config."), actions: vec![TERMINFO], ..Default::default() },
        false => Fact { title, detail: line.to_string(), ..Default::default() },
    }
}

fn hosts(list: anyhow::Result<Value>) -> Fact {
    let detail = match list {
        Ok(list) => {
            let labels: Vec<&str> = list["hosts"].as_array().into_iter().flatten().filter_map(|h| h["label"].as_str()).collect();
            if labels.is_empty() { "None yet. `rex hosts add` adds one.".to_string() } else { labels.join(", ") }
        }
        Err(e) => format!("{e:#}"),
    };
    Fact { title: "Hosts".into(), detail, ..Default::default() }
}

pub fn facts() -> Vec<Fact> {
    vec![
        server(json(&["whoami", "--json"])),
        config(run(&["config", "check"])),
        skills(run(&["skill", "status"])),
        terminfo(run(&["terminfo", "status"])),
        hosts(json(&["hosts", "list", "--json"])),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::Kind;
    use anyhow::anyhow;
    use serde_json::json;

    #[test]
    fn the_server_says_its_version_sessions_and_tailnet() {
        let who = json!({"rex_version": "0.1", "session_count": 5, "tailscale": {"state": "running", "dns_name": "mac.ts.net"}});
        let fact = server(Ok(who));
        assert_eq!((fact.title.as_str(), fact.detail.as_str()), ("Rex 0.1", "5 sessions · Tailscale: mac.ts.net"));
        assert_eq!(server(Ok(json!({"session_count": 1}))).detail, "1 sessions · Tailscale off");
    }

    #[test]
    fn the_config_says_what_it_holds_or_what_is_wrong() {
        let fact = config(Ok("path: /c/init.lua\nexists: yes\nbindings: 4\nremovals: 0\nmodes: 0\nactions: 2".into()));
        assert_eq!(fact.detail, "/c/init.lua · 4 bindings, 2 actions");
        assert_eq!(fact.file.as_deref(), Some(std::path::Path::new("/c/init.lua")));
        assert_eq!(config(Ok("path: /c/init.lua\nexists: no\nbindings: 0".into())).detail, "/c/init.lua · no file yet");
        let broken = config(Err(anyhow!("init.lua:3: nope")));
        assert_eq!((broken.detail.as_str(), broken.file.is_none()), ("init.lua:3: nope", true));
    }

    #[test]
    fn an_agent_left_behind_asks_for_setup() {
        let set = "Claude Code (found)\n  skill  current      ~/.claude/skills/rex\n  hook   current      ~/.claude/settings.json\n\nPi (found)\n  skill  current      ~/.agents\n  hook   unsupported\n";
        assert_eq!(agents(set), (vec!["Claude Code", "Pi"], vec![]));
        assert!(skills(Ok(set.into())).actions.is_empty());
        let late = "Codex (found)\n  skill  missing\n  hook   missing\n\nGemini (not found)\n  skill  missing\n";
        assert_eq!(agents(late), (vec!["Codex"], vec!["Codex"]), "once, and not one that isn't here");
        assert_eq!(skills(Ok(late.into())).actions, [SKILLS]);
    }

    #[test]
    fn terminfo_offers_what_is_missing() {
        let missing = "term: xterm-256color (the xterm-rex entry is not installed on this host)\nentry: xterm-rex is not installed here";
        assert_eq!(terminfo(Ok(missing.into())).actions, [TERMINFO]);
        assert!(terminfo(Ok("term: xterm-rex\nentry: found".into())).actions.is_empty());
    }

    /// Live, on a server of its own: `REX_SERVER=unix://… cargo test --lib rex::facts::tests::live -- --ignored`.
    /// `rex skill setup` and `rex terminfo setup` change this account's files, so nothing here runs them.
    #[test]
    #[ignore]
    fn live() {
        std::env::var("REX_SERVER").expect("run it on a server of its own");
        let facts = Kind::Rex.facts();
        assert_eq!(facts.iter().map(|f| f.title.split(' ').next().unwrap()).collect::<Vec<_>>(), ["Rex", "Config", "Agent", "Terminfo", "Hosts"], "{facts:?}");
        assert!(facts[0].title.len() > 4 && facts[0].detail.contains("sessions"), "{facts:?}");
        assert!(facts[1].file.is_some(), "{facts:?}");
        Kind::Rex.run(RELOAD.args).unwrap();
    }
}
