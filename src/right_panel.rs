//! The right panel's logic: a workspace's tabs, the widths the panel may take and what it offers. What here is plain;
//! the panel itself is the Shell's (`shell/right_panel.rs`).
use crate::settings::{RIGHT_PANEL_CENTER_MIN, RIGHT_PANEL_MIN};

/// A tab of the panel; a new tool is one more variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Surface {
    /// A browser tab, by its id.
    Browser(String),
    /// The shown pane's branch's pull request.
    PullRequest,
}

/// A workspace's panel. In memory only: a launch starts with every panel closed.
#[derive(Debug, Default)]
pub struct PanelState {
    /// In the strip's order.
    pub tabs: Vec<Surface>,
    /// `None` shows the picker.
    pub active: Option<Surface>,
    pub open: bool,
    /// The tabs, visited last at the end: where a closed tab's panel falls back to.
    history: Vec<Surface>,
}

impl PanelState {
    /// Tab `surface`, made if it isn't there yet, shown in the opened panel.
    pub fn add(&mut self, surface: Surface) {
        if !self.tabs.contains(&surface) {
            self.tabs.push(surface.clone());
        }
        self.activate(&surface);
        self.open = true;
    }

    pub fn activate(&mut self, surface: &Surface) {
        self.history.retain(|s| s != surface);
        self.history.push(surface.clone());
        self.active = Some(surface.clone());
    }

    /// Closes a tab: the last one visited shows instead, and the last one closes the panel.
    pub fn close(&mut self, surface: &Surface) {
        if !self.tabs.contains(surface) {
            return;
        }
        self.tabs.retain(|s| s != surface);
        self.history.retain(|s| s != surface);
        if self.active.as_ref() == Some(surface) {
            self.active = self.history.last().cloned();
        }
        if self.tabs.is_empty() {
            self.open = false;
        }
    }

    pub fn reorder(&mut self, from: usize, to: usize) {
        if from < self.tabs.len() && to < self.tabs.len() {
            let surface = self.tabs.remove(from);
            self.tabs.insert(to, surface);
        }
    }

    /// Shows the next or previous tab, round the strip.
    pub fn step(&mut self, forward: bool) {
        let n = self.tabs.len();
        let at = self.active.as_ref().and_then(|a| self.tabs.iter().position(|s| s == a));
        if let Some(at) = at.filter(|_| n > 1) {
            let next = self.tabs[if forward { (at + 1) % n } else { (at + n - 1) % n }].clone();
            self.activate(&next);
        }
    }

    /// The toggle goes by what's seen: a panel open but hidden (no room, a page in its place) is asked for, not to go.
    pub fn toggle_seen(&mut self, on_screen: bool) {
        self.open = !on_screen;
    }

    /// Closes the panel while it's seen on the pull request, else shows it there. The tab stays.
    pub fn toggle_pull_request(&mut self, on_screen: bool) {
        match on_screen && self.active == Some(Surface::PullRequest) {
            true => self.open = false,
            false => self.add(Surface::PullRequest),
        }
    }

    /// Closes the browser tabs not in `live`.
    pub fn retain_browsers(&mut self, live: &[&str]) {
        let gone = |s: &&Surface| matches!(s, Surface::Browser(id) if !live.contains(&id.as_str()));
        let gone: Vec<Surface> = self.tabs.iter().filter(gone).cloned().collect();
        gone.iter().for_each(|s| self.close(s));
    }
}

/// Whether a visible page holds the keyboard: a sidebar page has the window to itself, a panel page beside the panes
/// only once it's asked to, and `popup` (a menu, the address field, a note) keeps it from either.
pub fn page_takes_keyboard(sidebar_page: bool, focus_in_panel: bool, popup: bool) -> bool {
    (sidebar_page || focus_in_panel) && !popup
}

/// What the picker and the `+` list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Offer {
    Browser,
    PullRequest,
}

/// A page needs a window that can embed it; a pull request, a remote to ask.
pub fn offered(has_remote: bool, embeds_pages: bool) -> Vec<Offer> {
    let browser = embeds_pages.then_some(Offer::Browser);
    let request = has_remote.then_some(Offer::PullRequest);
    browser.into_iter().chain(request).collect()
}

/// Whether the panel's smallest width leaves the panes theirs, beside a left sidebar `sidebar` wide.
pub fn fits(window: f32, sidebar: f32) -> bool {
    window - sidebar - RIGHT_PANEL_CENTER_MIN >= RIGHT_PANEL_MIN
}

/// The width to draw a panel saved `width` wide at, leaving the panes theirs.
pub fn clamp_width(width: f32, window: f32, sidebar: f32) -> f32 {
    width.min(window - sidebar - RIGHT_PANEL_CENTER_MIN).max(RIGHT_PANEL_MIN)
}

/// Zeron's `drop_index`: the slot a tab dragged `rel_x` into the strip lands in, among `count` tabs `tab_w` apart.
pub fn drop_index(rel_x: f32, tab_w: f32, count: usize) -> usize {
    if count == 0 || tab_w <= 0. {
        return 0;
    }
    ((rel_x / tab_w).floor().max(0.) as usize).min(count - 1)
}

/// Zeron's `slide_offset`: in slots, how far tab `ix` shifts while tab `from` is dragged over slot `over`.
pub fn slide_offset(ix: usize, from: usize, over: usize) -> f32 {
    if from < over && ix > from && ix <= over {
        -1.
    } else if over < from && ix >= over && ix < from {
        1.
    } else {
        0.
    }
}
