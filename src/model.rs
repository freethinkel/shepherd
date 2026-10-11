//! herdr's `session.snapshot`, parsed. Events only invalidate it: the app refetches
//! (herdr's docs: events have no shared sequence with snapshots, so never replay them onto one).
use serde::{Deserialize, Serialize};

/// How the sidebar orders its agents (its `…` menu); saved in the settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentSort {
    /// Under a heading per workspace, in the sidebar's workspace order.
    Grouped,
    /// Blocked, working, done, idle: what needs you first. Also what an unknown (old or future) value reads as.
    #[default]
    #[serde(other)]
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Idle,
    Working,
    Blocked,
    Done,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Snapshot {
    pub focused_workspace_id: Option<String>,
    pub focused_tab_id: Option<String>,
    pub focused_pane_id: Option<String>,
    pub workspaces: Vec<Workspace>,
    pub tabs: Vec<Tab>,
    pub panes: Vec<Pane>,
    pub layouts: Vec<Layout>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    pub workspace_id: String,
    pub label: String,
    pub number: u32,
    pub active_tab_id: Option<String>,
    pub agent_status: AgentStatus,
    #[serde(default)]
    pub worktree: Option<Worktree>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Worktree {
    pub repo_key: String,
    pub is_linked_worktree: bool,
    #[serde(default)]
    pub checkout_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tab {
    pub tab_id: String,
    pub workspace_id: String,
    pub label: String,
    pub number: u32,
    pub agent_status: AgentStatus,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pane {
    pub pane_id: String,
    pub tab_id: String,
    pub workspace_id: String,
    pub terminal_id: String,
    pub agent_status: AgentStatus,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub terminal_title_stripped: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    /// The directory of the pane's foreground process (a shell's `cd` moves it).
    #[serde(default)]
    pub foreground_cwd: Option<String>,
    /// The agent's own session, as its herdr integration reports it (Claude's is its transcript's file name).
    #[serde(default)]
    pub agent_session: Option<AgentSession>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentSession {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Layout {
    pub tab_id: String,
    pub zoomed: bool,
    pub focused_pane_id: String,
    pub area: Rect,
    pub panes: Vec<LayoutPane>,
    #[serde(default)]
    pub splits: Vec<LayoutSplit>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutSplit {
    /// `split_<n>_root` or `split_<n>_<0|1…>`: the path from the tab's root (0 = first, 1 = second).
    pub id: String,
    /// `right` (side by side) or `down` (stacked).
    pub direction: String,
    pub ratio: f32,
    pub rect: Rect,
}

/// A divider of the shown tab, as a fraction of the tab area.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitFrame {
    /// The tree path `layout.set_split_ratio` takes (false = first child, true = second).
    pub path: Vec<bool>,
    /// The divider line is vertical: herdr's direction `right`.
    pub vertical: bool,
    pub ratio: f32,
    pub frame: Frac,
}

/// herdr's split id → its tree path (`src/app/api/panes.rs::split_path_id`).
pub fn split_path(id: &str) -> Option<Vec<bool>> {
    let rest = id.strip_prefix("split_")?;
    let (_, path) = rest.split_once('_')?;
    if path == "root" {
        return Some(Vec::new());
    }
    path.chars()
        .map(|c| match c {
            '0' => Some(false),
            '1' => Some(true),
            _ => None,
        })
        .collect()
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayoutPane {
    pub pane_id: String,
    pub focused: bool,
    pub rect: Rect,
}

/// Terminal cells, in herdr's own client geometry.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frac {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaneFrame {
    pub pane_id: String,
    pub frame: Frac,
    pub focused: bool,
}

impl Snapshot {
    pub fn from_result(result: &serde_json::Value) -> anyhow::Result<Self> {
        Ok(Self::deserialize(&result["snapshot"])?)
    }

    pub fn tabs_of(&self, workspace_id: &str) -> Vec<&Tab> {
        // herdr lists tabs in their order; `number` is the tab's id and stays put when `tab.move` reorders them
        self.tabs.iter().filter(|t| t.workspace_id == workspace_id).collect()
    }

    /// Workspaces in sidebar order: linked worktrees right under their repo's checkout workspace, with
    /// `Some(is_last_child)`; roots (and worktrees whose checkout has no workspace) get `None`.
    pub fn workspace_tree(&self) -> Vec<(&Workspace, Option<bool>)> {
        let mut all: Vec<_> = self.workspaces.iter().collect();
        all.sort_by_key(|w| w.number);
        let repo = |w: &Workspace| w.worktree.as_ref().map(|t| (t.repo_key.clone(), t.is_linked_worktree));
        let is_checkout = |key: &str| all.iter().any(|w| repo(w) == Some((key.to_string(), false)));
        let mut tree = Vec::new();
        for w in &all {
            match repo(w) {
                Some((key, true)) if is_checkout(&key) => continue,
                Some((key, false)) => {
                    tree.push((*w, None));
                    let kids: Vec<_> = all.iter().filter(|c| repo(c) == Some((key.clone(), true))).collect();
                    for (i, c) in kids.iter().enumerate() {
                        tree.push((**c, Some(i + 1 == kids.len())));
                    }
                }
                _ => tree.push((*w, None)),
            }
        }
        tree
    }

    /// Panes running a detected agent, the ones that need you first (herdr's "priority" sort).
    pub fn agents(&self) -> Vec<&Pane> {
        let rank = |s: AgentStatus| match s {
            AgentStatus::Blocked => 0,
            AgentStatus::Working => 1,
            AgentStatus::Done => 2,
            AgentStatus::Idle => 3,
            AgentStatus::Unknown => 4,
        };
        let mut agents: Vec<_> = self.panes.iter().filter(|p| p.agent.is_some()).collect();
        agents.sort_by_key(|p| rank(p.agent_status));
        agents
    }

    /// The agents in `sort` order, ties in status order.
    pub fn agents_by(&self, sort: AgentSort) -> Vec<&Pane> {
        let mut agents = self.agents();
        match sort {
            AgentSort::Status => {}
            AgentSort::Grouped => {
                let order: Vec<_> = self.workspace_tree().into_iter().map(|(w, _)| w.workspace_id.as_str()).collect();
                agents.sort_by_key(|p| order.iter().position(|id| *id == p.workspace_id).unwrap_or(usize::MAX));
            }
        }
        agents
    }

    /// Like `pick_tab`, but a gone choice falls back within `workspace` (its active tab, then its first) before
    /// herdr's focus, which may be in a workspace the TUI shows.
    pub fn pick_tab_in(&self, wanted: Option<&str>, workspace: Option<&str>) -> Option<&Tab> {
        let find = |id: &str| self.tabs.iter().find(|t| t.tab_id == id);
        wanted
            .and_then(find)
            .or_else(|| {
                let ws = self.workspaces.iter().find(|w| Some(w.workspace_id.as_str()) == workspace)?;
                ws.active_tab_id.as_deref().and_then(find).or_else(|| self.tabs_of(&ws.workspace_id).into_iter().next())
            })
            .or_else(|| self.pick_tab(None))
    }

    /// The tab Shepherd shows: its own choice while that tab lives, else herdr's focused one, else the first.
    pub fn pick_tab(&self, wanted: Option<&str>) -> Option<&Tab> {
        let find = |id: &str| self.tabs.iter().find(|t| t.tab_id == id);
        wanted.and_then(find).or_else(|| self.focused_tab_id.as_deref().and_then(find)).or(self.tabs.first())
    }

    pub fn split_frames(&self, tab_id: &str) -> Vec<SplitFrame> {
        let Some(layout) = self.layouts.iter().find(|l| l.tab_id == tab_id) else { return Vec::new() };
        if layout.zoomed {
            return Vec::new();
        }
        layout
            .splits
            .iter()
            .filter_map(|split| {
                Some(SplitFrame {
                    path: split_path(&split.id)?,
                    vertical: split.direction == "right",
                    ratio: split.ratio,
                    frame: frac(&layout.area, &split.rect)?,
                })
            })
            .collect()
    }

    // ponytail: fractions of herdr's cell rects; plan 2 builds a real split tree (dividers, set_split_ratio paths).
    pub fn pane_frames(&self, tab_id: &str) -> Vec<PaneFrame> {
        let Some(layout) = self.layouts.iter().find(|l| l.tab_id == tab_id) else { return Vec::new() };
        if layout.zoomed {
            let full = Frac { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
            return vec![PaneFrame { pane_id: layout.focused_pane_id.clone(), frame: full, focused: true }];
        }
        layout
            .panes
            .iter()
            .filter_map(|p| Some(PaneFrame { pane_id: p.pane_id.clone(), frame: frac(&layout.area, &p.rect)?, focused: p.focused }))
            .collect()
    }
}

/// The tab's panes and dividers while the divider at `path` is dragged to `ratio`, before herdr re-lays out:
/// inside that split's area, positions along its axis are remapped from the old ratio to the new one, so both
/// sides (and dividers nested in them) resize with the pointer. Frames outside the split are unchanged.
pub fn preview(panes: &[PaneFrame], splits: &[SplitFrame], path: &[bool], ratio: f32) -> (Vec<PaneFrame>, Vec<SplitFrame>) {
    let Some(dragged) = splits.iter().find(|s| s.path == path) else { return (panes.to_vec(), splits.to_vec()) };
    let node = dragged.frame;
    let (start, len) = if dragged.vertical { (node.x, node.w) } else { (node.y, node.h) };
    let (old, new) = (start + len * dragged.ratio, start + len * ratio);
    let end = start + len;
    // A position on the split's axis: the first side scales into [start, new], the second into [new, end].
    let remap = |v: f32| {
        if v <= old {
            if old - start <= f32::EPSILON { start } else { start + (v - start) * (new - start) / (old - start) }
        } else if end - old <= f32::EPSILON {
            end
        } else {
            new + (v - old) * (end - new) / (end - old)
        }
    };
    let moved = |f: Frac| {
        if !inside(f, node) {
            return f;
        }
        if dragged.vertical {
            let (x, right) = (remap(f.x), remap(f.x + f.w));
            Frac { x, w: right - x, ..f }
        } else {
            let (y, bottom) = (remap(f.y), remap(f.y + f.h));
            Frac { y, h: bottom - y, ..f }
        }
    };
    let panes = panes.iter().map(|p| PaneFrame { frame: moved(p.frame), ..p.clone() }).collect();
    let splits = splits
        .iter()
        .map(|s| if s.path == path { SplitFrame { ratio, ..s.clone() } } else { SplitFrame { frame: moved(s.frame), ..s.clone() } })
        .collect();
    (panes, splits)
}

/// `f` lies within `node` (allowing float noise).
fn inside(f: Frac, node: Frac) -> bool {
    const EPS: f32 = 1e-4;
    f.x >= node.x - EPS && f.y >= node.y - EPS && f.x + f.w <= node.x + node.w + EPS && f.y + f.h <= node.y + node.h + EPS
}

/// `rect` as a fraction of `area`; `None` for an empty rect or area.
fn frac(area: &Rect, rect: &Rect) -> Option<Frac> {
    let (aw, ah) = (f32::from(area.width), f32::from(area.height));
    if aw == 0.0 || ah == 0.0 || rect.width == 0 || rect.height == 0 {
        return None;
    }
    Some(Frac {
        x: f32::from(rect.x.saturating_sub(area.x)) / aw,
        y: f32::from(rect.y.saturating_sub(area.y)) / ah,
        w: f32::from(rect.width) / aw,
        h: f32::from(rect.height) / ah,
    })
}

/// The tree as sidebar groups: each top-level workspace with the worktrees nested under it.
pub fn groups<'a>(tree: Vec<(&'a Workspace, Option<bool>)>) -> Vec<(&'a Workspace, Vec<&'a Workspace>)> {
    let mut out: Vec<(&Workspace, Vec<&Workspace>)> = Vec::new();
    for (w, nest) in tree {
        match (nest, out.last_mut()) {
            (Some(_), Some((_, kids))) => kids.push(w),
            _ => out.push((w, Vec::new())),
        }
    }
    out
}

/// A Shepherd started inside a herdr pane must not mirror that pane into itself.
pub fn attachable(pane_id: &str, own_pane: Option<&str>) -> bool {
    own_pane != Some(pane_id)
}
