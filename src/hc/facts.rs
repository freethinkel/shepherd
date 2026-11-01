//! What Settings → Backend says of herdr-core, from `hc`. It starts a core if none runs, as Shepherd on it would.
use crate::backend::{Fact, Job};
use anyhow::Context;
use serde_json::Value;
use std::path::PathBuf;

const RESTART: Job = Job { label: "Restart", args: &["server", "restart"] };
const UPDATE: Job = Job { label: "Update", args: &["update"] };

/// `config.toml`, which hc reads each time it needs a setting.
pub fn config_path() -> PathBuf {
    crate::herdr::config_home().join("herdr-core").join("config.toml")
}

/// Runs `hc`. Blocks.
pub fn run(args: &[&str]) -> anyhow::Result<String> {
    let binary = super::binary().context("herdr-core isn't installed")?;
    crate::process::answer(crate::process::command(binary).args(args).output().context("cannot run hc")?)
}

fn json(args: &[&str]) -> anyhow::Result<Value> {
    Ok(serde_json::from_str(&run(args)?)?)
}

/// A span of time in its largest unit.
fn span(ms: u64) -> String {
    match ms / 60_000 {
        minutes @ 0..60 => format!("{minutes} min"),
        minutes @ 60..1440 => format!("{} h", minutes / 60),
        minutes => format!("{} d", minutes / 1440),
    }
}

fn core(status: anyhow::Result<Value>) -> Fact {
    let (title, detail) = match status {
        Ok(status) => {
            let live = status["live"].as_u64().unwrap_or(0);
            (format!("herdr-core {}", status["version"].as_str().unwrap_or("")), format!("{live} live terminals · up {}", span(status["uptime_ms"].as_u64().unwrap_or(0))))
        }
        Err(e) => ("herdr-core".to_string(), format!("{e:#}")),
    };
    Fact { title, detail, actions: vec![RESTART, UPDATE], ..Default::default() }
}

fn templates(list: anyhow::Result<Value>) -> Fact {
    let detail = match list {
        Ok(list) => {
            let names: Vec<&str> = list["templates"].as_array().into_iter().flatten().filter_map(|t| t["name"].as_str()).collect();
            if names.is_empty() { "None yet. A file in ~/.config/herdr-core/templates starts an agent by name.".to_string() } else { names.join(", ") }
        }
        Err(e) => format!("{e:#}"),
    };
    Fact { title: "Templates".into(), detail, ..Default::default() }
}

pub fn facts() -> Vec<Fact> {
    vec![core(json(&["server", "status", "--json"])), templates(json(&["templates", "--json"]))]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{Backend, Kind};
    use crate::commands::Command;
    use serde_json::json;

    #[test]
    fn the_core_says_its_version_and_terminals() {
        let fact = core(Ok(json!({"version": "0.1.16", "live": 5, "uptime_ms": 7_200_000})));
        assert_eq!((fact.title.as_str(), fact.detail.as_str()), ("herdr-core 0.1.16", "5 live terminals · up 2 h"));
        assert_eq!(fact.actions, [RESTART, UPDATE]);
        assert_eq!(span(59_999), "0 min");
        assert_eq!(span(2 * 86_400_000), "2 d");
    }

    #[test]
    fn templates_go_by_name() {
        assert_eq!(templates(Ok(json!({"templates": [{"name": "reviewer"}, {"name": "fixer"}]}))).detail, "reviewer, fixer");
        assert!(templates(Ok(json!({"templates": []}))).detail.starts_with("None yet"));
    }

    /// Live, on a core of its own: `HC_SESSION=<name> cargo test --lib hc::facts::tests::live -- --ignored`.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn live() {
        std::env::var("HC_SESSION").expect("run it on a core of its own: it restarts it");
        let backend = Backend::of(Kind::Hc, None);
        let pane = backend.mux.run(Command::NewWorkspace { cwd: None }).await.unwrap().pane_id.unwrap();
        backend.mux.prompt(pane.clone()).await.unwrap();
        let facts = tokio::task::spawn_blocking(|| Kind::Hc.facts()).await.unwrap();
        assert!(facts[0].title.starts_with("herdr-core 0."), "{facts:?}");
        assert!(facts[0].detail.starts_with("1 live terminals"), "{facts:?}");
        tokio::task::spawn_blocking(|| Kind::Hc.run(RESTART.args)).await.unwrap().unwrap();

        // what the window does after it: a connection of its own, on the terminals that ran on
        let again = Backend::of(Kind::Hc, None);
        assert!(again.snapshot().await.unwrap().panes.iter().any(|p| p.pane_id == pane), "the terminal outlived the core");
        again.mux.send(pane.clone(), Some("echo shepherd-$((40+2))".into())).await.unwrap();
        let mut read = String::new();
        for _ in 0..50 {
            read = again.mux.read(pane.clone(), 20, false).await.unwrap();
            if read.lines().any(|l| l.trim() == "shepherd-42") {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        panic!("{read}");
    }
}
