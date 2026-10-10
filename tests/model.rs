use shepherd::model::{AgentStatus, Frac, Snapshot, attachable, split_path};

fn fixture() -> Snapshot {
    let value: serde_json::Value = serde_json::from_str(include_str!("fixtures/snapshot.json")).unwrap();
    Snapshot::from_result(&value).unwrap()
}

#[test]
fn parses_snapshot() {
    let s = fixture();
    assert_eq!(s.focused_tab_id.as_deref(), Some("w1:t1"));
    assert_eq!(s.workspaces[0].label, "demo");
    assert_eq!(s.tabs.len(), 2);
    let p2 = s.panes.iter().find(|p| p.pane_id == "w1:p2").unwrap();
    assert_eq!(p2.agent_status, AgentStatus::Working);
    assert_eq!(p2.terminal_title_stripped.as_deref(), Some("fixing tests"));
}

#[test]
fn pane_frames_are_fractions_of_the_tab_area() {
    let frames = fixture().pane_frames("w1:t1");
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].pane_id, "w1:p1");
    assert_eq!(frames[0].frame, Frac { x: 0.0, y: 0.0, w: 0.5, h: 1.0 });
    assert_eq!(frames[1].frame, Frac { x: 0.5, y: 0.0, w: 0.5, h: 1.0 });
    assert!(frames[1].focused);
}

#[test]
fn zoomed_tab_shows_only_the_focused_pane_full_size() {
    let mut s = fixture();
    s.layouts[0].zoomed = true;
    let frames = s.pane_frames("w1:t1");
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].pane_id, "w1:p2");
    assert_eq!(frames[0].frame, Frac { x: 0.0, y: 0.0, w: 1.0, h: 1.0 });
}

#[test]
fn zero_area_panes_are_skipped() {
    let frames = fixture().pane_frames("w1:t2");
    assert_eq!(frames.iter().map(|f| f.pane_id.as_str()).collect::<Vec<_>>(), ["w1:p3"]);
}

#[test]
fn unknown_tab_has_no_frames() {
    assert!(fixture().pane_frames("w7:t7").is_empty());
}

#[test]
fn own_pane_is_not_attachable() {
    assert!(!attachable("w1:p2", Some("w1:p2")));
    assert!(attachable("w1:p1", Some("w1:p2")));
    assert!(attachable("w1:p1", None));
}

#[test]
fn unknown_agent_status_parses_as_unknown() {
    let s: AgentStatus = serde_json::from_str("\"sleeping\"").unwrap();
    assert_eq!(s, AgentStatus::Unknown);
}

/// `herdr api snapshot | jq .result > /tmp/live.json && SHEPHERD_LIVE_SNAPSHOT=/tmp/live.json cargo test --test model -- --ignored`
#[test]
#[ignore]
fn parses_live_snapshot() {
    let path = std::env::var("SHEPHERD_LIVE_SNAPSHOT").expect("SHEPHERD_LIVE_SNAPSHOT");
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let s = Snapshot::from_result(&value).unwrap();
    for tab in &s.tabs {
        let _ = s.pane_frames(&tab.tab_id);
    }
}

#[test]
fn tabs_of_a_workspace_in_order() {
    let s = fixture();
    let labels: Vec<_> = s.tabs_of("w1").iter().map(|t| t.label.as_str()).collect();
    assert_eq!(labels, ["main", "logs"]);
    assert!(s.tabs_of("w7").is_empty());
}

#[test]
fn pick_tab_keeps_a_live_choice() {
    assert_eq!(fixture().pick_tab(Some("w1:t2")).map(|t| t.tab_id.as_str()), Some("w1:t2"));
}

#[test]
fn pick_tab_falls_back_to_herdr_focus_when_the_choice_is_gone() {
    // The chosen tab was closed in herdr: show herdr's focused tab instead of nothing.
    assert_eq!(fixture().pick_tab(Some("w1:t9")).map(|t| t.tab_id.as_str()), Some("w1:t1"));
    assert_eq!(fixture().pick_tab(None).map(|t| t.tab_id.as_str()), Some("w1:t1"));
}

#[test]
fn pick_tab_falls_back_to_the_first_tab_without_focus() {
    let mut s = fixture();
    s.focused_tab_id = None;
    assert_eq!(s.pick_tab(None).map(|t| t.tab_id.as_str()), Some("w1:t1"));
}

#[test]
fn split_ids_decode_to_tree_paths() {
    assert_eq!(split_path("split_0_root"), Some(vec![]));
    assert_eq!(split_path("split_0_01"), Some(vec![false, true]));
    assert_eq!(split_path("s0"), None);
}

#[test]
fn split_frames_are_fractions_of_the_tab_area() {
    let splits = fixture().split_frames("w1:t1");
    assert_eq!(splits.len(), 1);
    assert_eq!(splits[0].path, Vec::<bool>::new());
    assert!(splits[0].vertical);
    assert_eq!(splits[0].ratio, 0.5);
    assert_eq!(splits[0].frame, Frac { x: 0.0, y: 0.0, w: 1.0, h: 1.0 });
}

#[test]
fn zoomed_tabs_have_no_dividers() {
    let mut s = fixture();
    s.layouts[0].zoomed = true;
    assert!(s.split_frames("w1:t1").is_empty());
}

#[test]
fn pick_tab_stays_in_the_shown_workspace_when_its_tab_closes() {
    // Shepherd's selection is local; herdr's focus may be in another workspace (the TUI's).
    let mut s = fixture();
    s.focused_tab_id = Some("w2:t1".into());
    s.tabs.push(shepherd::model::Tab {
        tab_id: "w2:t1".into(),
        workspace_id: "w2".into(),
        label: "other".into(),
        number: 1,
        agent_status: AgentStatus::Unknown,
    });
    let picked = s.pick_tab_in(Some("w1:t9"), Some("w1")).map(|t| t.tab_id.clone());
    assert_eq!(picked.as_deref(), Some("w1:t1"), "the workspace's active tab, not herdr's focus elsewhere");
}

fn pane(id: &str, x: f32, y: f32, w: f32, h: f32) -> shepherd::model::PaneFrame {
    shepherd::model::PaneFrame { pane_id: id.into(), frame: Frac { x, y, w, h }, focused: false }
}

fn split(path: Vec<bool>, vertical: bool, ratio: f32, frame: Frac) -> shepherd::model::SplitFrame {
    shepherd::model::SplitFrame { path, vertical, ratio, frame }
}

fn close(a: Frac, b: Frac) -> bool {
    [(a.x, b.x), (a.y, b.y), (a.w, b.w), (a.h, b.h)].iter().all(|(p, q)| (p - q).abs() < 1e-5)
}

#[test]
fn dragging_the_root_divider_resizes_both_sides() {
    let s = fixture();
    let (panes, _) = shepherd::model::preview(&s.pane_frames("w1:t1"), &s.split_frames("w1:t1"), &[], 0.3);
    assert!(close(panes[0].frame, Frac { x: 0.0, y: 0.0, w: 0.3, h: 1.0 }), "{:?}", panes[0].frame);
    assert!(close(panes[1].frame, Frac { x: 0.3, y: 0.0, w: 0.7, h: 1.0 }), "{:?}", panes[1].frame);
}

#[test]
fn dragging_a_nested_divider_leaves_the_other_side_alone() {
    // left | (top / bottom): drag the right column's horizontal divider from 0.5 to 0.8.
    let panes = [pane("l", 0.0, 0.0, 0.5, 1.0), pane("t", 0.5, 0.0, 0.5, 0.5), pane("b", 0.5, 0.5, 0.5, 0.5)];
    let full = Frac { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
    let right = Frac { x: 0.5, y: 0.0, w: 0.5, h: 1.0 };
    let splits = [split(vec![], true, 0.5, full), split(vec![true], false, 0.5, right)];
    let (out, _) = shepherd::model::preview(&panes, &splits, &[true], 0.8);
    assert!(close(out[0].frame, panes[0].frame));
    assert!(close(out[1].frame, Frac { x: 0.5, y: 0.0, w: 0.5, h: 0.8 }), "{:?}", out[1].frame);
    assert!(close(out[2].frame, Frac { x: 0.5, y: 0.8, w: 0.5, h: 0.2 }), "{:?}", out[2].frame);
}

#[test]
fn nested_dividers_move_with_the_root() {
    let panes = [pane("l", 0.0, 0.0, 0.5, 1.0), pane("t", 0.5, 0.0, 0.5, 0.5), pane("b", 0.5, 0.5, 0.5, 0.5)];
    let full = Frac { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
    let right = Frac { x: 0.5, y: 0.0, w: 0.5, h: 1.0 };
    let splits = [split(vec![], true, 0.5, full), split(vec![true], false, 0.5, right)];
    let (out_panes, out_splits) = shepherd::model::preview(&panes, &splits, &[], 0.25);
    assert!(close(out_panes[1].frame, Frac { x: 0.25, y: 0.0, w: 0.75, h: 0.5 }), "{:?}", out_panes[1].frame);
    assert_eq!(out_splits[0].ratio, 0.25);
    assert!(close(out_splits[1].frame, Frac { x: 0.25, y: 0.0, w: 0.75, h: 1.0 }), "{:?}", out_splits[1].frame);
    assert_eq!(out_splits[1].ratio, 0.5);
}

#[test]
fn an_unknown_path_changes_nothing() {
    let s = fixture();
    let (panes, _) = shepherd::model::preview(&s.pane_frames("w1:t1"), &s.split_frames("w1:t1"), &[true, true], 0.3);
    assert_eq!(panes, s.pane_frames("w1:t1"));
}

#[test]
fn agents_are_panes_with_an_agent_most_urgent_first() {
    let mut s = fixture();
    s.panes[0].agent = Some("codex".into());
    s.panes[0].agent_status = AgentStatus::Blocked;
    s.panes[2].agent = Some("pi".into());
    s.panes[2].agent_status = AgentStatus::Idle;
    let order: Vec<_> = s.agents().iter().map(|p| p.pane_id.as_str()).collect();
    assert_eq!(order, ["w1:p1", "w1:p2", "w1:p3"]);
    s.panes[0].agent = None;
    let order: Vec<_> = s.agents().iter().map(|p| p.pane_id.as_str()).collect();
    assert_eq!(order, ["w1:p2", "w1:p3"]);
}

fn ws(id: &str, number: u32, repo: Option<(&str, bool)>) -> shepherd::model::Workspace {
    serde_json::from_value(serde_json::json!({
        "workspace_id": id, "label": id, "number": number, "agent_status": "idle",
        "worktree": repo.map(|(key, linked)| serde_json::json!({"repo_key": key, "is_linked_worktree": linked})),
    }))
    .unwrap()
}

#[test]
fn linked_worktrees_nest_under_their_repo_checkout() {
    let mut s = fixture();
    s.workspaces = vec![
        ws("mochi", 1, Some(("m", false))),
        ws("plain", 2, None),
        ws("mono-wt", 3, Some(("x", true))),
        ws("mochi-a", 4, Some(("m", true))),
        ws("mochi-b", 5, Some(("m", true))),
    ];
    let tree: Vec<_> = s.workspace_tree().into_iter().map(|(w, nest)| (w.workspace_id.as_str(), nest)).collect();
    assert_eq!(
        tree,
        [
            ("mochi", None),
            ("mochi-a", Some(false)),
            ("mochi-b", Some(true)),
            ("plain", None),
            // its checkout has no workspace: shown at the top level
            ("mono-wt", None),
        ]
    );
}

#[test]
fn worktrees_group_under_their_repo() {
    let mut s = fixture();
    s.workspaces = vec![
        ws("mochi", 1, Some(("m", false))),
        ws("mochi-a", 2, Some(("m", true))),
        ws("mochi-b", 3, Some(("m", true))),
        ws("plain", 4, None),
    ];
    let groups: Vec<(&str, Vec<&str>)> = shepherd::model::groups(s.workspace_tree())
        .into_iter()
        .map(|(w, kids)| (w.workspace_id.as_str(), kids.iter().map(|k| k.workspace_id.as_str()).collect()))
        .collect();
    assert_eq!(groups, [("mochi", vec!["mochi-a", "mochi-b"]), ("plain", vec![])]);
}
