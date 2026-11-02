//! Agents for a backend that doesn't follow them, as all but herdr. A [`crate::backend::Mux`] says what a pane's program
//! reports of itself ([`Report`]); this names the agent, reads its status, and keeps which finished one you have seen.
use crate::backend::{Agents, Mux, Reply};
use crate::commands::Command;
use crate::model::{AgentStatus, Snapshot};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Agents known by their program's name, for one that does not report itself over OSC 7501.
const AGENTS: [&str; 9] = ["claude", "codex", "pi", "opencode", "gemini", "amp", "droid", "copilot", "kimi"];

/// What a pane's program says of itself.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    /// Its state over OSC 7501: `working`, `blocked`, `error`, `done` or another word for idle.
    pub state: Option<String>,
    /// The name it gives over OSC 7501.
    pub app: Option<String>,
    /// Its foreground program, a name or a path.
    pub program: Option<String>,
    /// Counts its reports: an agent seen as done, then done again, is news.
    pub turn: u64,
}

/// A title without the spinner or glyph an agent keeps in front of it, which changes a few times a second.
pub fn plain_title(title: &str) -> &str {
    title.trim_start_matches(|c: char| !c.is_alphanumeric()).trim_end()
}

/// The agent a pane runs, by the name Shepherd knows it by: what it reports itself as (`claude-code`), else a known
/// name in its foreground program, else `agent` when it reports without a name.
fn agent(report: &Report) -> Option<String> {
    let reported = report.app.as_deref().filter(|a| !a.is_empty()).map(|a| {
        let a = a.to_lowercase();
        a.strip_suffix("-code").map_or_else(|| a.clone(), str::to_string)
    });
    let program = report.program.as_deref().and_then(|p| p.rsplit('/').next()).filter(|p| AGENTS.contains(p)).map(str::to_string);
    reported.or(program).or_else(|| report.state.is_some().then(|| "agent".into()))
}

/// herdr's word for a pane's status. An error needs you as a block does; `seen` is the turn you saw it done in.
fn status(report: &Report, seen: Option<u64>) -> AgentStatus {
    match report.state.as_deref() {
        Some("working") => AgentStatus::Working,
        Some("blocked" | "error") => AgentStatus::Blocked,
        Some("done") if seen != Some(report.turn) => AgentStatus::Done,
        Some(_) => AgentStatus::Idle,
        None => AgentStatus::Unknown,
    }
}

/// The one that needs you first: what a tab or workspace of these shows.
fn loudest(statuses: impl Iterator<Item = AgentStatus>) -> AgentStatus {
    const ORDER: [AgentStatus; 4] = [AgentStatus::Blocked, AgentStatus::Done, AgentStatus::Working, AgentStatus::Idle];
    statuses.min_by_key(|s| ORDER.iter().position(|o| o == s).unwrap_or(ORDER.len())).unwrap_or_default()
}

#[derive(Default)]
struct Turns {
    /// The turn each pane is on, as last read.
    now: HashMap<String, u64>,
    /// The turn each finished agent was looked at in, which reads as idle after that (herdr's done → idle).
    // ponytail: in memory, so a restart shows finished agents as done again
    seen: HashMap<String, u64>,
}

/// [`Agents`] from what panes report.
pub struct Own {
    mux: Arc<dyn Mux>,
    turns: Mutex<Turns>,
}

impl Own {
    pub fn new(mux: Arc<dyn Mux>) -> Self {
        Self { mux, turns: Mutex::default() }
    }
}

impl Agents for Own {
    fn read(&self, snapshot: &mut Snapshot) {
        let mut turns = self.turns.lock().unwrap();
        turns.now.clear();
        for pane in &mut snapshot.panes {
            let report = pane.report.clone().unwrap_or_default();
            pane.agent = agent(&report);
            pane.agent_status = status(&report, turns.seen.get(&pane.pane_id).copied());
            turns.now.insert(pane.pane_id.clone(), report.turn);
        }
        let Turns { now, seen } = &mut *turns;
        seen.retain(|pane, _| now.contains_key(pane));
        for tab in &mut snapshot.tabs {
            let agents = snapshot.panes.iter().filter(|p| p.tab_id == tab.tab_id && p.agent.is_some());
            tab.agent_status = loudest(agents.map(|p| p.agent_status));
        }
        for workspace in &mut snapshot.workspaces {
            let tabs = snapshot.tabs.iter().filter(|t| t.workspace_id == workspace.workspace_id);
            workspace.agent_status = loudest(tabs.map(|t| t.agent_status));
        }
    }

    fn seen(&self, pane: String) -> Reply<'_, ()> {
        Box::pin(async move {
            {
                let mut turns = self.turns.lock().unwrap();
                if let Some(turn) = turns.now.get(&pane).copied() {
                    turns.seen.insert(pane.clone(), turn);
                }
            }
            // the focus, as on herdr; it also says a snapshot is due
            self.mux.run(Command::Focus { pane_id: pane }).await.map(drop)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Pane, Tab, Workspace};

    fn report(state: &str, turn: u64) -> Report {
        Report { state: Some(state.into()), turn, ..Report::default() }
    }

    #[test]
    fn statuses_read_as_herdrs() {
        assert_eq!(status(&report("working", 2), None), AgentStatus::Working);
        assert_eq!(status(&report("error", 2), None), AgentStatus::Blocked);
        assert_eq!(status(&report("done", 2), None), AgentStatus::Done);
        assert_eq!(status(&report("done", 2), Some(2)), AgentStatus::Idle, "seen this turn");
        assert_eq!(status(&report("done", 2), Some(1)), AgentStatus::Done, "seen before it finished again");
        assert_eq!(status(&Report::default(), None), AgentStatus::Unknown);
    }

    #[test]
    fn agents_are_named_by_themselves_or_their_program() {
        let app = |a: &str| Report { app: Some(a.into()), ..report("idle", 0) };
        assert_eq!(agent(&app("Pi")).as_deref(), Some("pi"));
        assert_eq!(agent(&app("claude-code")).as_deref(), Some("claude"));
        assert_eq!(agent(&Report { program: Some("/usr/local/bin/codex".into()), ..Report::default() }).as_deref(), Some("codex"));
        assert_eq!(agent(&report("idle", 0)).as_deref(), Some("agent"));
        assert_eq!(agent(&Report { program: Some("vim".into()), ..Report::default() }), None);
    }

    #[test]
    fn a_tab_shows_what_needs_you_first() {
        assert_eq!(loudest([AgentStatus::Idle, AgentStatus::Working, AgentStatus::Done].into_iter()), AgentStatus::Done);
        assert_eq!(loudest(std::iter::empty()), AgentStatus::Unknown);
    }

    /// A pane of an agent you finished looking at, and a shell beside it, in one tab of one workspace.
    fn two_panes(state: &str, turn: u64) -> Snapshot {
        let pane = |id: &str, report: Option<Report>| Pane { pane_id: id.into(), tab_id: "t".into(), workspace_id: "w".into(), report, ..Pane::default() };
        Snapshot {
            workspaces: vec![Workspace { workspace_id: "w".into(), label: "w".into(), number: 1, active_tab_id: None, agent_status: AgentStatus::Unknown, worktree: None }],
            tabs: vec![Tab { tab_id: "t".into(), workspace_id: "w".into(), label: "1".into(), number: 1, agent_status: AgentStatus::Unknown }],
            panes: vec![pane("agent", Some(Report { app: Some("pi".into()), ..report(state, turn) })), pane("shell", None)],
            ..Snapshot::default()
        }
    }

    #[tokio::test]
    async fn what_you_saw_stays_seen_until_it_finishes_again() {
        let own = Own::new(Arc::new(crate::herdr::Client::at("/nowhere/herdr.sock".into())));
        let mut shot = two_panes("done", 1);
        own.read(&mut shot);
        assert_eq!((shot.panes[0].agent.as_deref(), shot.panes[0].agent_status), (Some("pi"), AgentStatus::Done));
        assert_eq!((shot.panes[1].agent.as_deref(), shot.panes[1].agent_status), (None, AgentStatus::Unknown));
        assert_eq!((shot.tabs[0].agent_status, shot.workspaces[0].agent_status), (AgentStatus::Done, AgentStatus::Done), "a shell adds nothing");

        assert!(own.seen("agent".into()).await.is_err(), "the focus has nowhere to go here, and the mark stays");
        let mut shot = two_panes("done", 1);
        own.read(&mut shot);
        assert_eq!(shot.panes[0].agent_status, AgentStatus::Idle);

        let mut shot = two_panes("done", 2);
        own.read(&mut shot);
        assert_eq!(shot.panes[0].agent_status, AgentStatus::Done, "a new turn");

        let mut gone = Snapshot::default();
        own.read(&mut gone);
        assert!(own.turns.lock().unwrap().seen.is_empty(), "a closed pane is forgotten");
    }
}
