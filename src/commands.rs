//! herdr API requests for managing tabs, workspaces and panes. Pure: the shell sends them and follows herdr's
//! events, never editing its own model.
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    fn name(self) -> &'static str {
        match self {
            Direction::Left => "left",
            Direction::Right => "right",
            Direction::Up => "up",
            Direction::Down => "down",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    NewTab { workspace_id: String },
    CloseTab { tab_id: String },
    RenameTab { tab_id: String, label: String },
    /// `insert_index` is a slot in the workspace's tab list before the move: 0 puts it first, the tab count last.
    MoveTab { tab_id: String, insert_index: usize },
    NewWorkspace { cwd: Option<String> },
    CloseWorkspace { workspace_id: String },
    /// A linked worktree of the workspace's repo, opened as its own workspace; without a branch herdr names one.
    CreateWorktree { workspace_id: String, branch: Option<String> },
    /// Deletes a linked worktree's checkout and closes its workspace; the branch stays. Without `force` herdr
    /// refuses a checkout with changes (`dirty_worktree_requires_force`).
    RemoveWorktree { workspace_id: String, force: bool },
    RenameWorkspace { workspace_id: String, label: String },
    Split { pane_id: String, down: bool },
    ClosePane { pane_id: String },
    FocusPane { pane_id: String, direction: Direction },
    Zoom { pane_id: String },
    SetSplitRatio { tab_id: String, path: Vec<bool>, ratio: f32 },
}

impl Command {
    /// The socket API method and params. New tabs and workspaces never take herdr's focus: that is shared with
    /// its TUI clients, and Shepherd selects them locally.
    pub fn request(&self) -> (&'static str, Value) {
        match self {
            Command::NewTab { workspace_id } => ("tab.create", json!({"workspace_id": workspace_id, "focus": false})),
            Command::CloseTab { tab_id } => ("tab.close", json!({"tab_id": tab_id})),
            Command::RenameTab { tab_id, label } => ("tab.rename", json!({"tab_id": tab_id, "label": label})),
            Command::MoveTab { tab_id, insert_index } => {
                ("tab.move", json!({"tab_id": tab_id, "insert_index": insert_index}))
            }
            Command::NewWorkspace { cwd } => {
                let mut params = json!({"focus": false});
                if let Some(cwd) = cwd {
                    params["cwd"] = json!(cwd);
                }
                ("workspace.create", params)
            }
            Command::CloseWorkspace { workspace_id } => {
                ("workspace.close", json!({"workspace_id": workspace_id, "close_group": false}))
            }
            Command::CreateWorktree { workspace_id, branch } => {
                let mut params = json!({"workspace_id": workspace_id, "focus": false});
                if let Some(branch) = branch {
                    params["branch"] = json!(branch);
                }
                ("worktree.create", params)
            }
            Command::RemoveWorktree { workspace_id, force } => {
                ("worktree.remove", json!({"workspace_id": workspace_id, "force": force}))
            }
            Command::RenameWorkspace { workspace_id, label } => {
                ("workspace.rename", json!({"workspace_id": workspace_id, "label": label}))
            }
            Command::Split { pane_id, down } => (
                "pane.split",
                json!({"target_pane_id": pane_id, "direction": if *down { "down" } else { "right" }, "focus": true}),
            ),
            Command::ClosePane { pane_id } => ("pane.close", json!({"pane_id": pane_id})),
            Command::FocusPane { pane_id, direction } => {
                ("pane.focus_direction", json!({"pane_id": pane_id, "direction": direction.name()}))
            }
            Command::Zoom { pane_id } => ("pane.zoom", json!({"pane_id": pane_id, "mode": "toggle"})),
            Command::SetSplitRatio { tab_id, path, ratio } => {
                ("layout.set_split_ratio", json!({"tab_id": tab_id, "path": path, "ratio": ratio}))
            }
        }
    }
}

/// The tab a `tab.create` or `workspace.create` response made (both carry `tab`).
pub fn created_tab_id(result: &Value) -> Option<String> {
    result["tab"]["tab_id"].as_str().map(str::to_string)
}

/// The branch herdr picks for a worktree created without one, from herdr's `generated_branch_slug`
/// (src/worktree.rs @ v0.9.3, Apache-2.0); herdr seeds it with the time in microseconds.
pub fn generated_branch(seed: u64) -> String {
    const ADJECTIVES: [&str; 8] = ["brave", "calm", "clear", "green", "lucky", "quiet", "rapid", "silver"];
    const NOUNS: [&str; 8] = ["river", "cloud", "field", "forest", "harbor", "meadow", "stone", "valley"];
    let adjective = ADJECTIVES[seed as usize % ADJECTIVES.len()];
    let noun = NOUNS[(seed / ADJECTIVES.len() as u64) as usize % NOUNS.len()];
    format!("worktree/{adjective}-{noun}-{:04x}", seed & 0xffff)
}

/// The directory herdr checks a branch out into, from herdr's `branch_to_path_slug`: lowercase ASCII
/// alphanumerics, every other run collapsed to one `-`.
pub fn branch_path_slug(branch: &str) -> String {
    let mut slug = String::new();
    for ch in branch.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() { "worktree".into() } else { slug.into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_branch_matches_herdr() {
        assert_eq!(generated_branch(0), "worktree/brave-river-0000");
        assert_eq!(generated_branch(9), "worktree/calm-cloud-0009");
        assert_eq!(branch_path_slug("worktree/brave-river"), "worktree-brave-river");
        assert_eq!(branch_path_slug("issue/137 Worktree Spaces"), "issue-137-worktree-spaces");
        assert_eq!(branch_path_slug("///"), "worktree");
    }
    use serde_json::json;

    fn req(c: Command) -> (&'static str, serde_json::Value) {
        c.request()
    }

    #[test]
    fn tabs() {
        assert_eq!(req(Command::NewTab { workspace_id: "w1".into() }), ("tab.create", json!({"workspace_id": "w1", "focus": false})));
        assert_eq!(req(Command::CloseTab { tab_id: "w1:t2".into() }), ("tab.close", json!({"tab_id": "w1:t2"})));
        assert_eq!(
            req(Command::RenameTab { tab_id: "w1:t2".into(), label: "logs".into() }),
            ("tab.rename", json!({"tab_id": "w1:t2", "label": "logs"}))
        );
        assert_eq!(
            req(Command::MoveTab { tab_id: "w1:t2".into(), insert_index: 0 }),
            ("tab.move", json!({"tab_id": "w1:t2", "insert_index": 0}))
        );
    }

    #[test]
    fn workspaces() {
        assert_eq!(req(Command::NewWorkspace { cwd: None }), ("workspace.create", json!({"focus": false})));
        assert_eq!(
            req(Command::NewWorkspace { cwd: Some("/tmp/demo".into()) }),
            ("workspace.create", json!({"cwd": "/tmp/demo", "focus": false}))
        );
        assert_eq!(
            req(Command::CloseWorkspace { workspace_id: "w1".into() }),
            ("workspace.close", json!({"workspace_id": "w1", "close_group": false}))
        );
        assert_eq!(
            req(Command::RemoveWorktree { workspace_id: "w2".into(), force: false }),
            ("worktree.remove", json!({"workspace_id": "w2", "force": false}))
        );
        assert_eq!(
            req(Command::CreateWorktree { workspace_id: "w1".into(), branch: None }),
            ("worktree.create", json!({"workspace_id": "w1", "focus": false}))
        );
        assert_eq!(
            req(Command::CreateWorktree { workspace_id: "w1".into(), branch: Some("fix/login".into()) }),
            ("worktree.create", json!({"workspace_id": "w1", "branch": "fix/login", "focus": false}))
        );
        assert_eq!(
            req(Command::RenameWorkspace { workspace_id: "w1".into(), label: "api".into() }),
            ("workspace.rename", json!({"workspace_id": "w1", "label": "api"}))
        );
    }

    #[test]
    fn panes() {
        assert_eq!(
            req(Command::Split { pane_id: "w1:p1".into(), down: false }),
            ("pane.split", json!({"target_pane_id": "w1:p1", "direction": "right", "focus": true}))
        );
        assert_eq!(
            req(Command::Split { pane_id: "w1:p1".into(), down: true }),
            ("pane.split", json!({"target_pane_id": "w1:p1", "direction": "down", "focus": true}))
        );
        assert_eq!(req(Command::ClosePane { pane_id: "w1:p1".into() }), ("pane.close", json!({"pane_id": "w1:p1"})));
        assert_eq!(
            req(Command::FocusPane { pane_id: "w1:p1".into(), direction: Direction::Left }),
            ("pane.focus_direction", json!({"pane_id": "w1:p1", "direction": "left"}))
        );
        assert_eq!(req(Command::Zoom { pane_id: "w1:p1".into() }), ("pane.zoom", json!({"pane_id": "w1:p1", "mode": "toggle"})));
    }

    #[test]
    fn split_ratio() {
        assert_eq!(
            req(Command::SetSplitRatio { tab_id: "w1:t1".into(), path: vec![false, true], ratio: 0.25 }),
            ("layout.set_split_ratio", json!({"tab_id": "w1:t1", "path": [false, true], "ratio": 0.25}))
        );
    }

    #[test]
    fn created_tab_is_read_from_the_response() {
        let result = json!({"type": "tab_created", "tab": {"tab_id": "w1:t3", "label": "3"}, "root_pane": {"pane_id": "w1:p9"}});
        assert_eq!(created_tab_id(&result).as_deref(), Some("w1:t3"));
        assert_eq!(created_tab_id(&json!({"type": "pong"})), None);
        // workspace.create (herdr 0.9.3) carries the new workspace's first tab the same way.
        let ws = json!({"type": "workspace_created", "workspace": {"workspace_id": "wAA", "active_tab_id": "wAA:t1"},
                        "tab": {"tab_id": "wAA:t1"}, "root_pane": {"pane_id": "wAA:p1"}});
        assert_eq!(created_tab_id(&ws).as_deref(), Some("wAA:t1"));
    }
}
