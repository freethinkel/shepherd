//! The layout herdr-core leaves to its clients: workspaces, tabs and each tab's split tree, as one document kept in
//! the core, so it outlives Shepherd as the terminals do. Pure: [`super::api`] loads it, edits it and saves it.
//! A pane is its terminal: its id is the terminal's address (`t7`).
use crate::model::Worktree;
use crate::model::AREA;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Node {
    Pane(String),
    Split { right: bool, ratio: f32, first: Box<Node>, second: Box<Node> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

pub struct SplitRect {
    /// herdr's id for it (`split_0_01`): [`crate::model::split_path`] reads the path back.
    pub id: String,
    pub right: bool,
    pub ratio: f32,
    pub rect: Rect,
}

impl Node {
    fn split(self, pane: String, down: bool) -> Node {
        Node::Split { right: !down, ratio: 0.5, first: Box::new(self), second: Box::new(Node::Pane(pane)) }
    }

    pub fn terminals(&self) -> Vec<String> {
        match self {
            Node::Pane(t) => vec![t.clone()],
            Node::Split { first, second, .. } => [first.terminals(), second.terminals()].concat(),
        }
    }

    fn contains(&self, terminal: &str) -> bool {
        self.terminals().iter().any(|t| t == terminal)
    }

    /// The tree without `terminal`: its split's other side takes the split's place. `None`: nothing is left.
    fn without(self, terminal: &str) -> Option<Node> {
        match self {
            Node::Pane(t) if t == terminal => None,
            Node::Split { right, ratio, first, second } => match (first.without(terminal), second.without(terminal)) {
                (Some(first), Some(second)) => Some(Node::Split { right, ratio, first: Box::new(first), second: Box::new(second) }),
                (one, other) => one.or(other),
            },
            pane => Some(pane),
        }
    }

    /// `terminal`'s pane split in two, `pane` in the second half.
    fn split_at(self, terminal: &str, pane: &str, down: bool) -> Node {
        match self {
            Node::Pane(t) if t == terminal => Node::Pane(t).split(pane.into(), down),
            Node::Split { right, ratio, first, second } => Node::Split {
                right,
                ratio,
                first: Box::new(first.split_at(terminal, pane, down)),
                second: Box::new(second.split_at(terminal, pane, down)),
            },
            pane => pane,
        }
    }

    fn swap(&mut self, a: &str, b: &str) {
        match self {
            Node::Pane(t) if t == a => *t = b.into(),
            Node::Pane(t) if t == b => *t = a.into(),
            Node::Pane(_) => {}
            Node::Split { first, second, .. } => {
                first.swap(a, b);
                second.swap(a, b);
            }
        }
    }

    fn set_ratio(&mut self, path: &[bool], to: f32) -> bool {
        let Node::Split { ratio, first, second, .. } = self else { return false };
        match path.split_first() {
            None => {
                *ratio = to.clamp(0.05, 0.95);
                true
            }
            Some((false, rest)) => first.set_ratio(rest, to),
            Some((true, rest)) => second.set_ratio(rest, to),
        }
    }

    fn lay_out(&self, rect: Rect, path: &str, panes: &mut Vec<(String, Rect)>, splits: &mut Vec<SplitRect>) {
        match self {
            Node::Pane(t) => panes.push((t.clone(), rect)),
            Node::Split { right, ratio, first, second } => {
                let id = format!("split_0_{}", if path.is_empty() { "root" } else { path });
                splits.push(SplitRect { id, right: *right, ratio: *ratio, rect });
                let length = if *right { rect.width } else { rect.height };
                let cut = (f32::from(length) * ratio).round() as u16;
                let (a, b) = if *right {
                    (Rect { width: cut, ..rect }, Rect { x: rect.x + cut, width: length - cut, ..rect })
                } else {
                    (Rect { height: cut, ..rect }, Rect { y: rect.y + cut, height: length - cut, ..rect })
                };
                first.lay_out(a, &format!("{path}0"), panes, splits);
                second.lay_out(b, &format!("{path}1"), panes, splits);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tab {
    pub id: String,
    pub label: String,
    pub focused: String,
    pub zoomed: bool,
    pub root: Node,
    /// The tab of the client it was taken in from ([`Doc::adopt`]).
    #[serde(default)]
    pub origin: Option<String>,
}

impl Tab {
    pub fn frames(&self) -> (Vec<(String, Rect)>, Vec<SplitRect>) {
        let (mut panes, mut splits) = (Vec::new(), Vec::new());
        self.root.lay_out(Rect { x: 0, y: 0, width: AREA, height: AREA }, "", &mut panes, &mut splits);
        (panes, splits)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub label: String,
    pub active_tab: String,
    pub tabs: Vec<Tab>,
    /// The checkout it is on; only git knows, so it is asked once and kept.
    #[serde(default)]
    pub worktree: Option<Worktree>,
    /// The space of the client it was taken in from ([`Doc::adopt`]).
    #[serde(default)]
    pub origin: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Doc {
    pub workspaces: Vec<Workspace>,
    /// The last id given to a workspace or tab: ids are never reused, so a stale one cannot name a new tab.
    pub next: u32,
    pub focused_tab: Option<String>,
    /// Terminals other clients started: when one is gone, its record is theirs.
    pub adopted: Vec<String>,
}

/// The space of terminals no client groups.
pub const OTHERS: &str = "";

impl Doc {
    fn fresh(&mut self, kind: &str) -> String {
        self.next += 1;
        format!("{kind}{}", self.next)
    }

    fn new_tab(&mut self, terminal: String) -> Tab {
        Tab { id: self.fresh("tab"), label: String::new(), focused: terminal.clone(), zoomed: false, root: Node::Pane(terminal), origin: None }
    }

    pub fn tabs(&self) -> impl Iterator<Item = &Tab> {
        self.workspaces.iter().flat_map(|w| &w.tabs)
    }

    fn tab_mut(&mut self, id: &str) -> Option<&mut Tab> {
        self.workspaces.iter_mut().flat_map(|w| &mut w.tabs).find(|t| t.id == id)
    }

    pub fn tab_of(&self, terminal: &str) -> Option<&Tab> {
        self.tabs().find(|t| t.root.contains(terminal))
    }

    fn tab_of_mut(&mut self, terminal: &str) -> Option<&mut Tab> {
        self.workspaces.iter_mut().flat_map(|w| &mut w.tabs).find(|t| t.root.contains(terminal))
    }

    pub fn workspace_of(&self, tab: &str) -> Option<&Workspace> {
        self.workspaces.iter().find(|w| w.tabs.iter().any(|t| t.id == tab))
    }

    /// The tab everything is measured from: the focused one, else the first.
    pub fn shown_tab(&self) -> Option<&Tab> {
        self.focused_tab.as_deref().and_then(|id| self.tabs().find(|t| t.id == id)).or_else(|| self.tabs().next())
    }

    /// A new workspace around `terminal`: its id and its first tab's.
    pub fn add_workspace(&mut self, terminal: String) -> (String, String) {
        let tab = self.new_tab(terminal);
        let (id, active_tab) = (self.fresh("ws"), tab.id.clone());
        self.workspaces.push(Workspace { id: id.clone(), label: String::new(), active_tab: active_tab.clone(), tabs: vec![tab], worktree: None, origin: None });
        (id, active_tab)
    }

    /// A new last tab of `workspace` around `terminal`, its id. `None`: the workspace is gone.
    pub fn add_tab(&mut self, workspace: &str, terminal: String) -> Option<String> {
        self.workspaces.iter().any(|w| w.id == workspace).then_some(())?;
        let tab = self.new_tab(terminal);
        let id = tab.id.clone();
        self.workspaces.iter_mut().find(|w| w.id == workspace)?.tabs.push(tab);
        Some(id)
    }

    pub fn split(&mut self, target: &str, terminal: &str, down: bool, focus: bool) -> bool {
        let Some(tab) = self.tab_of_mut(target) else { return false };
        tab.root = tab.root.clone().split_at(target, terminal, down);
        tab.zoomed = false;
        if focus {
            tab.focused = terminal.into();
        }
        true
    }

    /// Takes `terminal` out: its tab goes with its last pane, its workspace with its last tab.
    pub fn remove(&mut self, terminal: &str) {
        self.take(terminal, None);
    }

    /// [`Self::remove`], but for the tab `except`, which it has just moved to.
    fn take(&mut self, terminal: &str, except: Option<&str>) {
        for w in &mut self.workspaces {
            w.tabs.retain_mut(|tab| {
                if !tab.root.contains(terminal) || except == Some(tab.id.as_str()) {
                    return true;
                }
                let Some(root) = tab.root.clone().without(terminal) else { return false };
                tab.root = root;
                if tab.focused == terminal {
                    tab.focused = tab.root.terminals().remove(0);
                    tab.zoomed = false;
                }
                true
            });
        }
        self.settle();
    }

    /// Drops what emptied, and points at what is left.
    pub(super) fn settle(&mut self) {
        self.workspaces.retain(|w| !w.tabs.is_empty());
        for w in &mut self.workspaces {
            if !w.tabs.iter().any(|t| t.id == w.active_tab) {
                w.active_tab = w.tabs[0].id.clone();
            }
        }
        if !self.focused_tab.as_ref().is_some_and(|id| self.tabs().any(|t| t.id == *id)) {
            let first = self.tabs().next().map(|t| t.id.clone());
            self.focused_tab = first;
        }
    }

    /// Keeps the panes whose terminal `live` knows; returns Shepherd's own among the others, which are gone from the
    /// layout.
    pub fn retain(&mut self, live: impl Fn(&str) -> bool) -> Vec<String> {
        let gone: Vec<String> = self.tabs().flat_map(|t| t.root.terminals()).filter(|t| !live(t)).collect();
        for terminal in &gone {
            self.remove(terminal);
        }
        let own = gone.into_iter().filter(|t| !self.adopted.contains(t)).collect();
        self.adopted.retain(|t| live(t));
        own
    }

    /// Takes in a terminal another client started: into the tab it shares with others, else a new one in its `space`'s
    /// workspace ([`OTHERS`] without one). Where it is moved here it stays.
    // ponytail: read once, so a tab moved to another space in the Herdr app stays where it was here
    pub fn adopt(&mut self, terminal: &str, space: &str, tab: Option<&str>) {
        self.adopted.push(terminal.into());
        let beside = tab.and_then(|origin| self.tabs().find(|t| t.origin.as_deref() == Some(origin))).and_then(|t| t.root.terminals().pop());
        if let Some(beside) = beside {
            self.split(&beside, terminal, false, false);
            return;
        }
        let workspace = self.workspaces.iter().find(|w| w.origin.as_deref() == Some(space)).map(|w| w.id.clone());
        let made = match workspace {
            Some(workspace) => self.add_tab(&workspace, terminal.into()),
            None => {
                let (workspace, made) = self.add_workspace(terminal.into());
                if let Some(w) = self.workspaces.iter_mut().find(|w| w.id == workspace) {
                    w.origin = Some(space.into());
                }
                Some(made)
            }
        };
        if let Some(made) = made.and_then(|id| self.tab_mut(&id)) {
            made.origin = tab.map(str::to_string);
        }
    }

    /// Closes the tab; returns its terminals, to end.
    pub fn close_tab(&mut self, id: &str) -> Vec<String> {
        let terminals = self.tabs().find(|t| t.id == id).map(|t| t.root.terminals()).unwrap_or_default();
        for w in &mut self.workspaces {
            w.tabs.retain(|t| t.id != id);
        }
        self.settle();
        terminals
    }

    /// Closes the workspace; returns its terminals, to end.
    pub fn close_workspace(&mut self, id: &str) -> Vec<String> {
        let terminals = self.workspaces.iter().filter(|w| w.id == id).flat_map(|w| &w.tabs).flat_map(|t| t.root.terminals()).collect();
        self.workspaces.retain(|w| w.id != id);
        self.settle();
        terminals
    }

    pub fn rename_tab(&mut self, id: &str, label: &str) {
        if let Some(tab) = self.tab_mut(id) {
            tab.label = label.into();
        }
    }

    pub fn rename_workspace(&mut self, id: &str, label: &str) {
        if let Some(w) = self.workspaces.iter_mut().find(|w| w.id == id) {
            w.label = label.into();
        }
    }

    pub fn set_worktree(&mut self, id: &str, worktree: Option<Worktree>) {
        if let Some(w) = self.workspaces.iter_mut().find(|w| w.id == id) {
            w.worktree = worktree;
        }
    }

    /// `slot` as herdr's `tab.move` takes it: a place in the list before the move, 0 first, the tab count last.
    pub fn move_tab(&mut self, id: &str, slot: usize) {
        let Some(w) = self.workspaces.iter_mut().find(|w| w.tabs.iter().any(|t| t.id == id)) else { return };
        let Some(from) = w.tabs.iter().position(|t| t.id == id) else { return };
        let tab = w.tabs.remove(from);
        let to = if slot > from { slot - 1 } else { slot };
        w.tabs.insert(to.min(w.tabs.len()), tab);
    }

    /// Moves `ids` together to before `before`, or to the end.
    pub fn move_workspaces(&mut self, ids: &[String], before: Option<&str>) {
        let (moved, mut rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.workspaces).into_iter().partition(|w| ids.contains(&w.id));
        let at = before.and_then(|b| rest.iter().position(|w| w.id == b)).unwrap_or(rest.len());
        rest.splice(at..at, moved);
        self.workspaces = rest;
    }

    pub fn set_ratio(&mut self, tab: &str, path: &[bool], ratio: f32) -> bool {
        self.tab_mut(tab).is_some_and(|t| t.root.set_ratio(path, ratio))
    }

    pub fn swap(&mut self, a: &str, b: &str) {
        if let Some(tab) = self.tab_of_mut(a).filter(|t| t.root.contains(b)) {
            tab.root.swap(a, b);
        }
    }

    /// Moves `pane` into another tab, beside `target` or the whole tab. False: nothing to do (herdr's `same_tab`).
    pub fn move_pane(&mut self, pane: &str, to: &str, target: Option<&str>, down: bool) -> bool {
        let same = self.tab_of(pane).is_none_or(|t| t.id == to);
        if same || !self.tabs().any(|t| t.id == to) {
            return false;
        }
        self.remove(pane);
        let Some(tab) = self.tab_mut(to) else { return false };
        tab.root = match target {
            Some(target) => tab.root.clone().split_at(target, pane, down),
            None => tab.root.clone().split(pane.into(), down),
        };
        tab.zoomed = false;
        true
    }

    /// Moves `pane` into a new tab of `workspace`, or of its own; the tab's id. `None`: nothing was moved.
    pub fn pane_to_new_tab(&mut self, pane: &str, workspace: Option<&str>) -> Option<String> {
        let old = self.tab_of(pane)?.clone();
        let own = self.workspace_of(&old.id)?.id.clone();
        let to = workspace.unwrap_or(&own).to_string();
        if to == own && old.root.terminals().len() == 1 {
            return Some(old.id); // it is that tab already
        }
        let tab = self.add_tab(&to, pane.into())?;
        self.take(pane, Some(&tab));
        Some(tab)
    }

    pub fn focus_tab(&mut self, id: &str) {
        let Some(w) = self.workspaces.iter_mut().find(|w| w.tabs.iter().any(|t| t.id == id)) else { return };
        w.active_tab = id.into();
        self.focused_tab = Some(id.into());
    }

    pub fn focus_pane(&mut self, pane: &str) {
        let Some(tab) = self.tab_of_mut(pane) else { return };
        tab.focused = pane.into();
        let id = tab.id.clone();
        self.focus_tab(&id);
    }

    pub fn toggle_zoom(&mut self, pane: &str) {
        if let Some(tab) = self.tab_of_mut(pane) {
            tab.zoomed = !tab.zoomed;
            tab.focused = pane.into();
        }
    }

    /// The pane next to `pane` that way (`left`, `right`, `up`, `down`): the nearest that shares an edge with it.
    pub fn neighbour(&self, pane: &str, direction: &str) -> Option<String> {
        let (panes, _) = self.tab_of(pane)?.frames();
        let from = panes.iter().find(|(t, _)| t == pane)?.1;
        let overlap = |a: u16, a_len: u16, b: u16, b_len: u16| i32::from(a + a_len).min(i32::from(b + b_len)) - i32::from(a.max(b));
        let candidates = panes.iter().filter(|(t, _)| t != pane).filter_map(|(t, r)| {
            let (gap, shared) = match direction {
                "left" => (i32::from(from.x) - i32::from(r.x + r.width), overlap(from.y, from.height, r.y, r.height)),
                "right" => (i32::from(r.x) - i32::from(from.x + from.width), overlap(from.y, from.height, r.y, r.height)),
                "up" => (i32::from(from.y) - i32::from(r.y + r.height), overlap(from.x, from.width, r.x, r.width)),
                _ => (i32::from(r.y) - i32::from(from.y + from.height), overlap(from.x, from.width, r.x, r.width)),
            };
            (gap >= 0 && shared > 0).then_some(((gap, -shared), t))
        });
        candidates.min_by_key(|(rank, _)| *rank).map(|(_, t)| t.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn other_clients_terminals_group_by_space_and_tab() {
        let mut doc = Doc::default();
        doc.add_workspace("t9".into());
        doc.adopt("t1", "dots", Some("claude"));
        doc.adopt("t2", "dots", Some("logs"));
        doc.adopt("t3", "dots", Some("claude"));
        doc.adopt("t4", "home", Some("one"));
        doc.adopt("t5", OTHERS, None);
        doc.adopt("t6", OTHERS, None);
        let tabs = |at: usize| doc.workspaces[at].tabs.iter().map(|t| t.root.terminals()).collect::<Vec<_>>();
        assert_eq!(doc.workspaces.len(), 4, "Shepherd's own, two spaces and the rest");
        assert_eq!(tabs(1), [vec!["t1", "t3"], vec!["t2"]], "panes of one tab");
        assert_eq!(tabs(2), [vec!["t4"]]);
        assert_eq!(tabs(3), [vec!["t5"], vec!["t6"]], "no tab of their own: one each");
        assert_eq!(doc.shown_tab().unwrap().root.terminals(), ["t9"], "taking one in moves no focus");
    }

    #[test]
    fn a_gone_terminal_of_another_client_is_left_to_it() {
        let mut doc = Doc::default();
        doc.add_workspace("t1".into());
        doc.adopt("t2", "dots", Some("claude"));
        doc.adopt("t3", "dots", Some("claude"));
        assert_eq!(doc.retain(|t| t == "t2"), ["t1"], "only Shepherd's own is ended");
        assert_eq!(doc.tabs().map(|t| t.root.terminals()).collect::<Vec<_>>(), [vec!["t2"]]);
        assert_eq!(doc.adopted, ["t2"]);
    }

    #[test]
    fn a_document_from_before_origins_still_loads() {
        let old = r#"{"workspaces": [{"id": "ws1", "label": "", "active_tab": "tab1", "tabs": [{"id": "tab1", "label": "", "focused": "t1", "zoomed": false, "root": {"pane": "t1"}}]}], "next": 2}"#;
        let doc: Doc = serde_json::from_str(old).unwrap();
        assert_eq!((doc.workspaces[0].origin.as_deref(), doc.adopted.len()), (None, 0));
    }

    /// One workspace, one tab: `t1`, then `t2` beside it and `t3` under `t2`.
    fn three() -> Doc {
        let mut doc = Doc::default();
        doc.add_workspace("t1".into());
        doc.split("t1", "t2", false, true);
        doc.split("t2", "t3", true, true);
        doc
    }

    fn terminals(doc: &Doc) -> Vec<String> {
        doc.tabs().flat_map(|t| t.root.terminals()).collect()
    }

    #[test]
    fn splits_lay_out_as_herdr_names_them() {
        let doc = three();
        let (panes, splits) = doc.tabs().next().unwrap().frames();
        let half = AREA / 2;
        assert_eq!(panes[0], ("t1".into(), Rect { x: 0, y: 0, width: half, height: AREA }));
        assert_eq!(panes[1], ("t2".into(), Rect { x: half, y: 0, width: half, height: half }));
        assert_eq!(panes[2], ("t3".into(), Rect { x: half, y: half, width: half, height: half }));
        let ids: Vec<_> = splits.iter().map(|s| (s.id.as_str(), s.right)).collect();
        assert_eq!(ids, [("split_0_root", true), ("split_0_1", false)]);
        assert_eq!(crate::model::split_path(&splits[1].id), Some(vec![true]));
    }

    #[test]
    fn a_closed_pane_gives_its_room_to_its_sibling() {
        let mut doc = three();
        doc.remove("t3");
        assert_eq!(terminals(&doc), ["t1", "t2"]);
        let tab = doc.tabs().next().unwrap();
        assert_eq!(tab.focused, "t1", "the focus left the closed pane");
        assert!(matches!(&tab.root, Node::Split { first, second, .. } if **first == Node::Pane("t1".into()) && **second == Node::Pane("t2".into())));
    }

    #[test]
    fn the_last_pane_takes_its_tab_and_workspace() {
        let mut doc = Doc::default();
        let (workspace, first) = doc.add_workspace("t1".into());
        let second = doc.add_tab(&workspace, "t2".into()).unwrap();
        doc.focus_tab(&first);
        doc.remove("t1");
        assert_eq!(doc.workspaces[0].active_tab, second);
        assert_eq!(doc.focused_tab.as_deref(), Some(second.as_str()));
        doc.remove("t2");
        assert!(doc.workspaces.is_empty());
        assert_eq!(doc.focused_tab, None);
    }

    #[test]
    fn dead_terminals_leave_the_layout() {
        let mut doc = three();
        assert_eq!(doc.retain(|t| t != "t2"), ["t2"]);
        assert_eq!(terminals(&doc), ["t1", "t3"]);
    }

    #[test]
    fn ratios_go_by_path() {
        let mut doc = three();
        let tab = doc.tabs().next().unwrap().id.clone();
        assert!(doc.set_ratio(&tab, &[true], 0.25));
        assert!(!doc.set_ratio(&tab, &[false], 0.25), "t1 is a pane, no split");
        let (_, splits) = doc.tabs().next().unwrap().frames();
        assert_eq!((splits[0].ratio, splits[1].ratio), (0.5, 0.25));
    }

    #[test]
    fn tabs_move_by_slots_before_the_move() {
        let mut doc = Doc::default();
        let (workspace, a) = doc.add_workspace("t1".into());
        let b = doc.add_tab(&workspace, "t2".into()).unwrap();
        let c = doc.add_tab(&workspace, "t3".into()).unwrap();
        let order = |doc: &Doc| doc.tabs().map(|t| t.id.clone()).collect::<Vec<_>>();
        doc.move_tab(&a, 3);
        assert_eq!(order(&doc), [b.clone(), c.clone(), a.clone()]);
        doc.move_tab(&a, 0);
        assert_eq!(order(&doc), [a, b, c]);
    }

    #[test]
    fn workspaces_move_together() {
        let mut doc = Doc::default();
        let ids: Vec<_> = ["t1", "t2", "t3"].iter().map(|t| doc.add_workspace((*t).into()).0).collect();
        doc.move_workspaces(&ids[1..], Some(&ids[0]));
        assert_eq!(doc.workspaces.iter().map(|w| w.id.clone()).collect::<Vec<_>>(), [ids[1].clone(), ids[2].clone(), ids[0].clone()]);
        doc.move_workspaces(&ids[1..2], None);
        assert_eq!(doc.workspaces.last().unwrap().id, ids[1]);
    }

    #[test]
    fn panes_move_between_tabs() {
        let mut doc = three();
        let workspace = doc.workspaces[0].id.clone();
        let other = doc.add_tab(&workspace, "t4".into()).unwrap();
        assert!(doc.move_pane("t2", &other, Some("t4"), true));
        assert_eq!(doc.tab_of("t2").unwrap().id, other);
        assert!(!doc.move_pane("t2", &other, None, false), "already there");
        let new = doc.pane_to_new_tab("t3", None).unwrap();
        assert_eq!(doc.tab_of("t3").unwrap().id, new);
        assert_eq!(doc.tabs().filter(|t| t.root.contains("t3")).count(), 1, "out of its old tab");
    }

    #[test]
    fn a_lone_pane_stays_when_it_has_nowhere_new_to_go() {
        let mut doc = Doc::default();
        let (_, tab) = doc.add_workspace("t1".into());
        assert_eq!(doc.pane_to_new_tab("t1", None), Some(tab));
        assert_eq!(doc.workspaces.len(), 1);
    }

    #[test]
    fn a_workspaces_last_pane_moves_to_another_workspace() {
        let mut doc = Doc::default();
        doc.add_workspace("t1".into());
        let (other, _) = doc.add_workspace("t2".into());
        let tab = doc.pane_to_new_tab("t1", Some(&other)).unwrap();
        assert_eq!(doc.workspaces.len(), 1, "the emptied workspace went");
        assert_eq!(doc.tab_of("t1").unwrap().id, tab);
    }

    #[test]
    fn neighbours_share_an_edge() {
        let doc = three();
        assert_eq!(doc.neighbour("t1", "right").as_deref(), Some("t2"));
        assert_eq!(doc.neighbour("t3", "up").as_deref(), Some("t2"));
        assert_eq!(doc.neighbour("t3", "left").as_deref(), Some("t1"));
        assert_eq!(doc.neighbour("t1", "left"), None);
    }

    #[test]
    fn the_document_survives_json() {
        let doc = three();
        let back: Doc = serde_json::from_value(serde_json::to_value(&doc).unwrap()).unwrap();
        assert_eq!(back, doc);
    }
}
