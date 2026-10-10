//! herdr's `session.snapshot`, parsed. Events only invalidate it: the app refetches
//! (herdr's docs: events have no shared sequence with snapshots, so never replay them onto one).
use serde::Deserialize;

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
        let mut tabs: Vec<_> = self.tabs.iter().filter(|t| t.workspace_id == workspace_id).collect();
        tabs.sort_by_key(|t| t.number);
        tabs
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

/// A Shepherd started inside a herdr pane must not mirror that pane into itself.
pub fn attachable(pane_id: &str, own_pane: Option<&str>) -> bool {
    own_pane != Some(pane_id)
}
