use shepherd::backend::Change;
use shepherd::model::{AgentScope, AgentSort, AgentStatus, Frac, Since, Snapshot, ago, attachable, split_path, thread_shown};

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
fn only_a_pane_update_that_changes_nothing_shown_is_held() {
    let s = fixture();
    let update = |title: &str, status: &str| {
        Change::Pane(Box::new(serde_json::from_value(serde_json::json!({
            "pane_id": "w1:p2", "tab_id": "w1:t1", "workspace_id": "w1", "terminal_id": "term_b", "agent_status": status,
            "focused": true, "cwd": "/tmp/demo", "revision": 8, "agent": "claude",
            "terminal_title": format!("◐ {title}"), "terminal_title_stripped": title})).unwrap()))
    };
    assert!(s.holds(&update("fixing tests", "working")));
    assert!(!s.holds(&update("fixing more tests", "working")));
    assert!(!s.holds(&update("fixing tests", "done")));
    assert!(!s.holds(&Change::Focused));
    assert!(!s.holds(&Change::Any)); // a resync
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
fn a_pane_has_neighbors_only_away_from_the_tab_edge() {
    use shepherd::commands::Direction::*;
    let mut s = fixture();
    let (left, right) = (s.layouts[0].clone(), "w1:p2");
    assert!(left.has_neighbor("w1:p1", Right));
    assert!(left.has_neighbor(right, Left));
    for side in [Left, Up, Down] {
        assert!(!left.has_neighbor("w1:p1", side));
    }
    assert!(!left.has_neighbor(right, Right));
    assert!(!left.has_neighbor("w7:p7", Right), "unknown pane");
    s.layouts[0].zoomed = true;
    assert!(!s.layouts[0].has_neighbor("w1:p1", Right), "a zoomed pane hides the others");
}

#[test]
fn the_neighbor_is_the_pane_sharing_most_of_that_side() {
    use shepherd::commands::Direction::*;
    let mut s = fixture();
    let layout = &mut s.layouts[0];
    let mut bottom = layout.panes[1].clone();
    bottom.pane_id = "w1:p3".into();
    (bottom.rect.y, bottom.rect.height, layout.panes[1].rect.height) = (10, 30, 10);
    layout.panes.push(bottom);
    assert_eq!(layout.neighbor("w1:p1", Right), Some("w1:p3"));
    assert_eq!(layout.neighbor("w1:p2", Left), Some("w1:p1"));
    assert_eq!(layout.neighbor("w1:p2", Down), Some("w1:p3"));
    assert_eq!(layout.neighbor("w1:p3", Up), Some("w1:p2"));
    assert_eq!(layout.neighbor("w1:p3", Right), None);
    assert_eq!(s.layouts[1].neighbor("w1:p3", Right), None, "a zero-size pane is not shown");
}

#[test]
fn focus_moved_here_shows_before_herdrs_next_snapshot() {
    let mut s = fixture();
    s.focus_pane("w1:t1", "w1:p1");
    let frames = s.pane_frames("w1:t1");
    assert!(frames[0].focused && !frames[1].focused);
    assert_eq!(s.focused_pane_id.as_deref(), Some("w1:p1"));
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
fn tabs_keep_herdrs_order_not_their_numbers() {
    let mut s = fixture();
    s.tabs.reverse(); // as after a tab.move: the list reorders, each tab keeps its number
    let labels: Vec<_> = s.tabs_of("w1").iter().map(|t| t.label.as_str()).collect();
    assert_eq!(labels, ["logs", "main"]);
}

#[test]
fn a_panes_tab_position_follows_the_order_not_the_number() {
    let mut s = fixture();
    let pane = s.panes.iter().find(|p| s.tabs_of("w1").first().is_some_and(|t| t.tab_id == p.tab_id)).unwrap().clone();
    assert_eq!(s.tab_position(&pane), Some(1));
    s.tabs.reverse();
    assert_eq!(s.tab_position(&pane), Some(2));
    assert_eq!(s.tab_position(&shepherd::model::Pane::default()), None);
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
fn a_divider_sits_on_the_panes_rounded_edge_not_its_ratio() {
    // herdr's real layout: 69 rows split at 0.5594 → panes of 39 and 30 rows, the edge at row 39, not 38.6.
    let (top, bottom) = (39.0 / 69.0, 30.0 / 69.0);
    let panes = [pane("t", 0.0, 0.0, 1.0, top), pane("b", 0.0, top, 1.0, bottom)];
    let full = Frac { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
    assert_eq!(shepherd::model::split_edge(&panes, &split(vec![], false, 0.5594, full)), top);
    // l | (t / b): the right column's divider snaps to its own panes' edge, never to the column's start.
    let panes = [pane("l", 0.0, 0.0, 0.5, 1.0), pane("t", 0.5, 0.0, 0.5, 0.4), pane("b", 0.5, 0.4, 0.5, 0.6)];
    let right = Frac { x: 0.5, y: 0.0, w: 0.5, h: 1.0 };
    assert_eq!(shepherd::model::split_edge(&panes, &split(vec![true], false, 0.38, right)), 0.4);
    assert_eq!(shepherd::model::split_edge(&panes, &split(vec![], true, 0.49, full)), 0.5);
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

#[test]
fn priority_puts_unread_before_working_and_the_latest_change_first() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!("fixtures/snapshot.json")).unwrap();
    value["snapshot"]["agents"] = serde_json::json!([
        {"pane_id": "w1:p1", "state_change_seq": 5},
        {"pane_id": "w1:p2", "state_change_seq": 9},
        {"pane_id": "w1:p3", "state_change_seq": 7},
    ]);
    let mut s = Snapshot::from_result(&value).unwrap();
    for p in &mut s.panes {
        p.agent = Some("claude".into());
    }
    s.panes[0].agent_status = AgentStatus::Working;
    s.panes[1].agent_status = AgentStatus::Working;
    s.panes[2].agent_status = AgentStatus::Done;
    let order: Vec<_> = s.agents().iter().map(|p| p.pane_id.as_str()).collect();
    assert_eq!(order, ["w1:p3", "w1:p2", "w1:p1"]);
}

#[test]
fn grouped_agents_follow_workspace_order_ties_in_status_order() {
    let mut s = fixture();
    for p in &mut s.panes {
        p.agent = Some("claude".into());
    }
    s.panes[1].agent_status = AgentStatus::Blocked; // status order: p2, p1, p3
    let ids = |agents: Vec<&shepherd::model::Pane>| agents.iter().map(|p| p.pane_id.clone()).collect::<Vec<_>>();
    s.workspaces = vec![ws("w2", 1, None), ws("w1", 2, None)];
    s.panes[2].workspace_id = "w2".into();
    assert_eq!(ids(s.agents_by(AgentSort::Grouped, &Since::new())), ["w1:p3", "w1:p2", "w1:p1"]);
    assert_eq!(ids(s.agents_by(AgentSort::Status, &Since::new())), ["w1:p2", "w1:p1", "w1:p3"]);
}

#[test]
fn grouped_agents_keep_a_repos_worktrees_together_and_idle_ones_show_a_time() {
    let mut s = fixture();
    for p in &mut s.panes {
        p.agent = Some("claude".into());
    }
    s.workspaces = vec![ws("mochi", 1, Some(("m", false))), ws("plain", 2, None), ws("mochi-a", 3, Some(("m", true)))];
    for (p, (ws, status)) in s.panes.iter_mut().zip([("mochi-a", AgentStatus::Blocked), ("plain", AgentStatus::Working), ("mochi", AgentStatus::Idle)]) {
        (p.workspace_id, p.agent_status) = (ws.into(), status);
    }
    let ids = |agents: Vec<&shepherd::model::Pane>| agents.iter().map(|p| p.pane_id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(s.agents_by(AgentSort::Grouped, &Since::new())), ["w1:p1", "w1:p3", "w1:p2"], "mochi and its worktree, then plain");
    assert_eq!(s.project_of("mochi-a").map(|w| w.workspace_id.as_str()), Some("mochi"));
    let mut since = Since::new();
    s.track(&mut since, std::time::Instant::now());
    let timed: Vec<bool> = s.panes.iter().map(|p| shepherd::model::timed_since(&since, p).is_some()).collect();
    assert_eq!(timed, [true, false, true], "blocked and idle show a time, working doesn't");
}

#[test]
fn a_scope_takes_in_the_workspaces_agents_its_projects_or_all() {
    let mut s = fixture();
    for p in &mut s.panes {
        p.agent = Some("claude".into());
    }
    s.workspaces = vec![ws("mochi", 1, Some(("m", false))), ws("plain", 2, None), ws("mochi-a", 3, Some(("m", true)))];
    for (p, (ws, status)) in s.panes.iter_mut().zip([("mochi-a", AgentStatus::Blocked), ("plain", AgentStatus::Working), ("mochi", AgentStatus::Idle)]) {
        (p.workspace_id, p.agent_status) = (ws.into(), status);
    }
    let ids = |workspace: &str, scope| s.agents_in(workspace, scope).iter().map(|p| p.pane_id.clone()).collect::<Vec<_>>();
    assert_eq!(ids("mochi", AgentScope::Worktree), ["w1:p3"], "the workspace alone");
    assert_eq!(ids("mochi-a", AgentScope::Worktree), ["w1:p1"]);
    assert_eq!(ids("mochi", AgentScope::Project), ["w1:p1", "w1:p3"], "the checkout's project, in agents' order");
    assert_eq!(ids("mochi-a", AgentScope::Project), ids("mochi", AgentScope::Project), "a worktree's is the same project");
    assert_eq!(ids("plain", AgentScope::Project), ["w1:p2"]);
    assert_eq!(ids("plain", AgentScope::All), ["w1:p1", "w1:p2", "w1:p3"]);
    assert!(ids("gone", AgentScope::Worktree).is_empty());
    assert!(ids("gone", AgentScope::Project).is_empty());
}

#[test]
fn waiting_agents_count_from_their_status_change_longest_first() {
    use std::time::{Duration, Instant};
    let mut s = fixture();
    for p in &mut s.panes {
        p.agent = Some("claude".into());
    }
    let (t0, mut since) = (Instant::now(), Since::new());
    s.panes[0].agent_status = AgentStatus::Done;
    s.track(&mut since, t0);
    s.panes[1].agent_status = AgentStatus::Blocked;
    s.track(&mut since, t0 + Duration::from_secs(90));
    let ids = |agents: Vec<&shepherd::model::Pane>| agents.iter().map(|p| p.pane_id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(s.agents_by(AgentSort::Waiting, &since)), ["w1:p1", "w1:p2", "w1:p3"], "done since t0 waited longest");
    s.track(&mut since, t0 + Duration::from_secs(600));
    assert_eq!(since["w1:p1"].1, t0, "a status that stays keeps its clock");
    s.panes[0].agent_status = AgentStatus::Blocked;
    s.track(&mut since, t0 + Duration::from_secs(600));
    assert_eq!(ids(s.agents_by(AgentSort::Waiting, &since)), ["w1:p2", "w1:p1", "w1:p3"], "done then blocked starts over");
    s.panes.remove(2);
    s.track(&mut since, t0);
    assert!(!since.contains_key("w1:p3"), "a gone pane leaves");
    let labels: Vec<_> = [0, 59, 60, 3599, 3600, 86_399, 86_400].map(|secs| ago(Duration::from_secs(secs))).into();
    assert_eq!(labels, ["now", "now", "1m", "59m", "1h", "23h", "1d"]);
}

#[test]
fn the_next_waiting_agent_goes_round_and_saved_times_survive_a_restart() {
    use std::time::{Duration, Instant, SystemTime};
    let mut s = fixture();
    for p in &mut s.panes {
        p.agent = Some("claude".into());
    }
    let (t0, mut since) = (Instant::now(), Since::new());
    s.panes[1].agent_status = AgentStatus::Blocked;
    s.track(&mut since, t0);
    s.panes[0].agent_status = AgentStatus::Done;
    s.track(&mut since, t0 + Duration::from_secs(5));
    let next = |shown| s.next_waiting(&since, shown).map(|p| p.pane_id.clone());
    assert_eq!(next(None).as_deref(), Some("w1:p2"), "the longest wait first");
    assert_eq!(next(Some("w1:p2")).as_deref(), Some("w1:p1"));
    assert_eq!(next(Some("w1:p1")).as_deref(), Some("w1:p2"), "round again");
    assert_eq!(next(Some("w1:p3")).as_deref(), Some("w1:p2"), "from an agent not waiting, the first");

    // Shepherd restarts: herdr's panes are the same, ten minutes on
    let (wall, now) = (SystemTime::now(), t0 + Duration::from_secs(5));
    let saved = s.remember(&mut since, &Default::default(), now, wall);
    s.panes[2].state_change_seq = 7; // p3's id came back for another agent
    let mut saved_p3 = saved.clone();
    saved_p3.get_mut("w1:p3").unwrap().1 = 6;
    let later = now + Duration::from_secs(3600);
    let mut fresh = Since::new();
    s.track(&mut fresh, later);
    let again = s.remember(&mut fresh, &saved_p3, later, wall + Duration::from_secs(600));
    assert_eq!(later.duration_since(fresh["w1:p2"].1), Duration::from_secs(605), "blocked since before the restart");
    assert_eq!(fresh["w1:p3"].1, later, "another state change starts afresh");
    assert_eq!(again["w1:p2"], saved["w1:p2"], "the saved time stays as it was");
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

#[test]
fn a_dropped_workspace_group_takes_the_place_of_the_one_it_is_dropped_on() {
    let groups: Vec<Vec<String>> = [vec!["a"], vec!["b", "b1"], vec!["c"]]
        .map(|g| g.into_iter().map(String::from).collect())
        .into();
    let ids = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    // down onto the last: to the end; down onto the middle: before the one after it
    assert_eq!(shepherd::model::group_move(&groups, 0, 2), Some((ids(&["a"]), None)));
    assert_eq!(shepherd::model::group_move(&groups, 0, 1), Some((ids(&["a"]), Some("c".into()))));
    // up: before the one dropped on, worktrees along
    assert_eq!(shepherd::model::group_move(&groups, 1, 0), Some((ids(&["b", "b1"]), Some("a".into()))));
    assert_eq!(shepherd::model::group_move(&groups, 1, 1), None);
}

/// A project (`w1`) with an idle worktree (`w2`) and a worktree (`w3`) running two agents in one tab and one in another.
fn project() -> Snapshot {
    let pane = |id: &str, workspace: &str, tab: &str, status: &str| {
        serde_json::json!({"pane_id": id, "tab_id": tab, "workspace_id": workspace, "terminal_id": id, "agent_status": status, "agent": "claude"})
    };
    let workspace = |id: &str| serde_json::json!({"workspace_id": id, "label": id, "number": 1, "active_tab_id": null, "agent_status": "idle"});
    serde_json::from_value(serde_json::json!({
        "workspaces": [workspace("w1"), workspace("w2"), workspace("w3")],
        "tabs": [],
        "panes": [pane("a", "w3", "t1", "working"), pane("b", "w3", "t2", "blocked"), serde_json::json!({"pane_id": "shell", "tab_id": "t3", "workspace_id": "w2", "terminal_id": "shell", "agent_status": "idle"})],
        "layouts": [],
    }))
    .unwrap()
}

fn ids(threads: &[shepherd::model::Thread]) -> Vec<String> {
    threads.iter().map(|(w, p)| p.map_or(w.workspace_id.clone(), |p| p.pane_id.clone())).collect()
}

#[test]
fn agents_go_on_their_shelves_and_a_workspace_without_one_settles() {
    let s = project();
    let shelves = s.shelves(None);
    assert_eq!(ids(&shelves.active), ["b"]);
    assert_eq!(ids(&shelves.working), ["a"]);
    // a plain shell is no agent
    assert_eq!(ids(&shelves.settled), ["w1", "w2"]);
}

#[test]
fn the_project_select_narrows_the_shelves_and_one_gone_shows_them_all() {
    let s = project();
    let shelves = s.shelves(Some("w2"));
    assert_eq!((ids(&shelves.active), ids(&shelves.settled)), (Vec::<String>::new(), vec!["w2".to_string()]));
    assert_eq!(ids(&s.shelves(Some("another-places")).working), ["a"]);
}

#[test]
fn the_search_finds_a_thread_by_its_title_workspace_or_project() {
    let mut s = project();
    s.panes[0].terminal_title_stripped = Some("Fix the Parser".into());
    let shelves = s.shelves(None);
    let (titled, settled) = (shelves.working[0], shelves.settled[0]);
    assert!(s.thread_matches(titled, "parser"));
    assert!(s.thread_matches(titled, "w3"));
    assert!(!s.thread_matches(settled, "parser"));
    assert!(s.thread_matches(settled, ""), "an empty search keeps them all");
}

#[test]
fn the_list_marks_the_shown_tabs_agent_or_else_the_workspaces_first_card() {
    let s = project();
    let shelves = s.shelves(None);
    let threads: Vec<_> = shelves.active.iter().chain(&shelves.working).chain(&shelves.settled).copied().collect();
    let lit = |workspace, tab| (0..threads.len()).filter(|i| thread_shown(&threads, *i, workspace, tab)).collect::<Vec<_>>();
    assert_eq!(lit(Some("w3"), Some("t1")), [1], "the shown agent, not its neighbour");
    assert_eq!(lit(Some("w3"), Some("t9")), [0], "a tab without an agent still marks its workspace");
    assert_eq!(lit(Some("w1"), None), [2]);
    assert_eq!(lit(None, Some("t1")), Vec::<usize>::new(), "a web page is shown: nothing is");
}
