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

    // ponytail: fractions of herdr's cell rects; plan 2 builds a real split tree (dividers, set_split_ratio paths).
    pub fn pane_frames(&self, tab_id: &str) -> Vec<PaneFrame> {
        let Some(layout) = self.layouts.iter().find(|l| l.tab_id == tab_id) else { return Vec::new() };
        if layout.zoomed {
            let full = Frac { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
            return vec![PaneFrame { pane_id: layout.focused_pane_id.clone(), frame: full, focused: true }];
        }
        let (aw, ah) = (f32::from(layout.area.width), f32::from(layout.area.height));
        if aw == 0.0 || ah == 0.0 {
            return Vec::new();
        }
        layout
            .panes
            .iter()
            .filter(|p| p.rect.width > 0 && p.rect.height > 0)
            .map(|p| PaneFrame {
                pane_id: p.pane_id.clone(),
                frame: Frac {
                    x: f32::from(p.rect.x.saturating_sub(layout.area.x)) / aw,
                    y: f32::from(p.rect.y.saturating_sub(layout.area.y)) / ah,
                    w: f32::from(p.rect.width) / aw,
                    h: f32::from(p.rect.height) / ah,
                },
                focused: p.focused,
            })
            .collect()
    }
}

/// A Shepherd started inside a herdr pane must not mirror that pane into itself.
pub fn attachable(pane_id: &str, own_pane: Option<&str>) -> bool {
    own_pane != Some(pane_id)
}
