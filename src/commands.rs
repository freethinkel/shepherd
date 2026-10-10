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
    NewWorkspace { cwd: Option<String> },
    CloseWorkspace { workspace_id: String },
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

#[cfg(test)]
mod tests {
    use super::*;
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
