//! The window: workspaces sidebar, the tab bar, and a GPUI terminal per pane of the shown tab, refreshed on
//! every herdr event. Look and titlebar handling follow Collie (Zeron style).
use crate::bindings::*;
use crate::commands::{Command, Direction};
use crate::herdr::{Client, EVENT_TYPES};
use crate::terminal::pane::{GridHold, PaneQueries};
use crate::ui::divider::{Throttle, drag_ratio};
use gpui_base::input::{Input, InputEvent, InputState};
use crate::model::{AgentSort, AgentStatus, PaneFrame, Snapshot, SplitFrame, attachable};
use crate::terminal::PaneTerminal;
use crate::settings::{SIDEBAR_MAX, SIDEBAR_MIN, SIDEBAR_WIDTH, SettingsGlobal};
use crate::theme::Theme;
use gpui::{
    linear_color_stop, linear_gradient, AnimationExt, App, Bounds, ClickEvent, Context, Div, Entity, Focusable, Hsla, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels,
    ParentElement, Render, SharedString, Stateful, Styled, TitlebarOptions, Window, WindowBackgroundAppearance,
    Subscription, WindowControlArea, WindowOptions, canvas, div, point, prelude::*, px, relative, size,
};
use serde_json::json;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

mod pane_drag;
mod tab_bar;
pub(crate) use tab_bar::agent_favicon;

const TITLEBAR_HEIGHT: f32 = 38.0;
/// The titlebar's left cluster: traffic lights, then the sidebar toggle at Zeron's `titlebar_cluster_start` (88).
const TOGGLE_LEFT: f32 = 88.0;
/// Centre line of the traffic lights (placed at y 14), measured on a 2x screen; the toggle centres on it, not on the titlebar.
const TRAFFIC_LIGHTS_CENTER: f32 = 21.25;
/// The Agents list's least height when dragged: one agent row and a bit.
const AGENTS_MIN: f32 = 60.0;
const TITLEBAR_CLUSTER: f32 = TOGGLE_LEFT + 24.0 + 8.0;

pub struct Shell {
    snapshot: Option<Snapshot>,
    error: Option<String>,
    panes: HashMap<String, Entity<PaneTerminal>>,
    own_pane: Option<String>,
    /// herdr's focused pane we last moved keyboard focus to; refocusing on every render would undo clicks.
    focused: Option<String>,
    /// The tab Shepherd shows. Local on purpose: herdr's focus is shared with its TUI clients.
    selected_tab: Option<String>,
    /// Titlebar drag: armed by a press in the strip, consumed by the first move (Collie's `drag_strip`).
    should_move: Rc<Cell<bool>>,
    client: Client,
    rt: tokio::runtime::Handle,
    /// The pane grid's bounds from the last paint, to turn a divider drag into a ratio.
    grid_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    divider: Option<DividerDrag>,
    /// A released divider's ratio, shown until herdr's snapshot catches up (no jump back to the old layout).
    settling: Option<(Vec<bool>, f32)>,
    /// A herdr error from the last command, shown over the grid for a few seconds.
    flash: Option<String>,
    /// The tab or workspace label being edited in place.
    renaming: Option<Rename>,
    // ponytail: in memory only; persist in settings if folds should survive a restart
    folded: std::collections::HashSet<String>,
    /// A workspace's right-click menu and where it opened.
    ws_menu: Option<crate::ui::Popup<(String, gpui::Point<Pixels>)>>,
    /// Groups mid fold or unfold, by workspace id.
    fold_motion: HashMap<String, FoldMotion>,
    /// Resize seams' hover: (hovered, flip count), keyed by the strip id.
    seam_hover: HashMap<SharedString, (bool, u64)>,
    /// The workspace shown last, so closing its tab stays in it.
    shown_workspace: Option<String>,
    /// Saved in the settings when a drag ends.
    sidebar_width: f32,
    // ponytail: open/closed in memory only; persist in settings if it should survive a restart
    sidebar_open: bool,
    /// The sidebar's width tween while it shows or hides.
    sidebar_motion: Option<FoldMotion>,
    /// The sidebar's edge is being dragged.
    sidebar_drag: bool,
    /// The drag hit a width limit (-1 min, +1 max) at this instant: the edge nudges past it and springs back.
    sidebar_edge: Option<(f32, Instant)>,
    agent_sort: AgentSort,
    show_agents: bool,
    show_status_bar: bool,
    /// Saved in the settings when a drag ends.
    agents_height: Option<f32>,
    /// The Agents section's top edge is being dragged: the pointer's y and the list's height at the press.
    agents_drag: Option<(f32, f32)>,
    /// The sidebar's `…` menu, open where it was clicked.
    view_menu: Option<crate::ui::Popup<()>>,
    /// Worktrees herdr is removing: their rows show a spinner until they leave the snapshot.
    removing: std::collections::HashSet<String>,
    /// Remove a worktree with changes anyway? The workspace and its label.
    remove_confirm: Option<crate::ui::Popup<(String, String)>>,
    /// The New worktree dialog, open over the window.
    worktree_dialog: Option<crate::ui::Popup<WorktreeDialog>>,
    /// Worktrees herdr is still creating, as (workspace the dialog was opened on, branch): herdr answers
    /// `worktree.create` only after git and plugin hooks finish, so the sidebar shows a placeholder until then.
    pending_worktrees: Vec<(String, String)>,
    /// A tab's right-click menu: the tab and where it was clicked.
    tab_menu: Option<crate::ui::Popup<(String, gpui::Point<Pixels>)>>,
    /// A pane's right-click menu: the pane and where it was clicked.
    pane_menu: Option<crate::ui::Popup<(String, gpui::Point<Pixels>)>>,
    /// The status bar's spend popup, opened at this point (its bottom-left).
    spent_menu: Option<crate::ui::Popup<gpui::Point<Pixels>>>,
    /// The spend popup lists the most tokens first.
    spent_by_tokens: bool,
    /// The status bar's limits popup, at its anchor.
    usage_menu: Option<crate::ui::Popup<gpui::Point<Pixels>>>,
    agents_scroll: gpui::ScrollHandle,
    tab_bar: tab_bar::TabBar,
    /// Holds keyboard focus when no pane can (renames ended, a tab of only Shepherd's own pane), so the app's
    /// actions still dispatch.
    focus: gpui::FocusHandle,
    /// The Settings page, shown in place of the sidebar and the panes while open.
    settings: Option<(Entity<crate::ui::settings::SettingsView>, Subscription)>,
    /// A pane being dragged by its grip.
    pane_drop: Option<pane_drag::PaneDrop>,
    /// The command panel (cmd-k), open over the window.
    command_panel: Option<crate::ui::Popup<(Entity<crate::ui::command_panel::Panel>, Subscription)>>,
    /// Agents that started waiting or finished out of sight and have not been looked at since: the Dock badge.
    unseen: std::collections::HashSet<String>,
    /// The count on the Dock icon now.
    badge: usize,
}

/// New worktree: the repo's workspace and the branch name being typed.
struct WorktreeDialog {
    workspace_id: String,
    input: Entity<InputState>,
    _events: Subscription,
}

#[derive(Clone, PartialEq)]
enum RenameTarget {
    Tab(String),
    Workspace(String),
}

struct Rename {
    target: RenameTarget,
    input: Entity<InputState>,
    _events: Subscription,
}

struct DividerDrag {
    tab_id: String,
    path: Vec<bool>,
    vertical: bool,
    start_px: f32,
    len_px: f32,
    ratio: f32,
    throttle: Throttle,
}

impl Shell {
    pub fn open(rt: tokio::runtime::Handle, cx: &mut gpui::App) {
        let options = WindowOptions {
            window_bounds: Some(gpui::WindowBounds::Windowed(Bounds::centered(None, size(px(1180.), px(760.)), cx))),
            window_min_size: Some(size(px(820.), px(560.))),
            titlebar: Some(TitlebarOptions {
                title: None,
                appears_transparent: true,
                traffic_light_position: Some(point(px(14.), px(14.))),
            }),
            app_owns_titlebar_drag: true,
            window_background: WindowBackgroundAppearance::Blurred,
            app_id: Some("dev.freethinkel.shepherd".into()),
            ..Default::default()
        };
        cx.open_window(options, |window, cx| {
            cx.set_global(SettingsGlobal::of(cx).settings.theme(window.appearance()));
            window
                .observe_window_appearance(|window, cx| {
                    cx.set_global(SettingsGlobal::of(cx).settings.theme(window.appearance()));
                    cx.refresh_windows();
                })
                .detach();
            cx.new(|cx| {
                cx.observe_window_activation(window, |_: &mut Shell, window: &mut Window, cx: &mut Context<Shell>| {
                    reapply_traffic_lights(window);
                    cx.notify(); // the shown tab's agents count as seen once the window is active
                })
                .detach();
                // A click on a notification brings its agent's tab up.
                let this = cx.entity().downgrade();
                cx.on_system_notification_response(move |response, cx| {
                    let _ = this.update(cx, |s, cx| {
                        let pane = s.snapshot.as_ref().and_then(|snap| snap.panes.iter().find(|p| p.pane_id == response.tag.as_ref()));
                        if let Some(pane) = pane {
                            s.selected_tab = Some(pane.tab_id.clone());
                            cx.notify();
                        }
                    });
                    cx.activate(true);
                });
                let shell = Shell {
                    snapshot: None,
                    error: None,
                    panes: HashMap::new(),
                    own_pane: std::env::var("HERDR_PANE_ID").ok(),
                    focused: None,
                    selected_tab: std::env::var("SHEPHERD_TAB").ok(),
                    should_move: Rc::new(Cell::new(false)),
                    client: SettingsGlobal::of(cx)
                        .settings
                        .herdr_socket
                        .clone()
                        .filter(|p| !p.trim().is_empty())
                        .map_or_else(Client::local, |p| Client::at(p.into())),
                    rt: rt.clone(),
                    grid_bounds: Rc::new(Cell::new(None)),
                    divider: None,
                    settling: None,
                    flash: None,
                    renaming: None,
                    folded: Default::default(),
                    fold_motion: Default::default(),
                    seam_hover: Default::default(),
                    ws_menu: None,
                    shown_workspace: None,
                    sidebar_width: SettingsGlobal::of(cx).settings.sidebar_width,
                    sidebar_open: true,
                    sidebar_motion: None,
                    sidebar_drag: false,
                    sidebar_edge: None,
                    agent_sort: SettingsGlobal::of(cx).settings.agent_sort,
                    show_agents: SettingsGlobal::of(cx).settings.show_agents,
                    show_status_bar: SettingsGlobal::of(cx).settings.show_status_bar,
                    agents_height: SettingsGlobal::of(cx).settings.agents_height,
                    agents_drag: None,
                    view_menu: None,
                    pane_menu: None,
                    spent_menu: None,
                    usage_menu: None,
                    spent_by_tokens: false,
                    tab_menu: None,
                    worktree_dialog: None,
                    removing: Default::default(),
                    remove_confirm: None,
                    pending_worktrees: Vec::new(),
                    agents_scroll: gpui::ScrollHandle::new(),
                    tab_bar: Default::default(),
                    focus: cx.focus_handle(),
                    settings: None,
                    pane_drop: None,
                    command_panel: None,
                    unseen: Default::default(),
                    badge: 0,
                };
                let client = shell.client.clone();
                cx.set_global(PaneQueries(client.clone(), rt.clone()));
                cx.spawn(async move |this, cx| sync_loop(this, rt, client, cx).await).detach();
                shell
            })
        })
        .expect("open window");
    }

    /// Shows Settings in the active Shepherd window (or the first one), or hides it if it is shown there. For
    /// callers outside a window (the menu with no window focused, launch); deferred because a window busy
    /// dispatching cannot be updated.
    pub fn toggle_settings(cx: &mut App) {
        cx.defer(|cx| {
            let active = cx.active_window().and_then(|w| w.downcast::<Shell>());
            let Some(window) = active.or_else(|| cx.windows().into_iter().find_map(|w| w.downcast::<Shell>())) else { return };
            let _ = window.update(cx, |shell, window, cx| shell.toggle_settings_here(window, cx));
        });
    }

    fn toggle_settings_here(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.is_some() {
            return self.close_settings(cx);
        }
        let view = cx.new(|cx| crate::ui::settings::SettingsView::new(window, cx));
        let close = cx.subscribe(&view, |this, _, _: &crate::ui::settings::Close, cx| this.close_settings(cx));
        window.focus(&view.focus_handle(cx), cx);
        self.settings = Some((view, close));
        cx.notify();
    }

    /// cmd-k (commands) or the agents key: opens that panel over the window, or closes it when it is open.
    pub fn toggle_command_panel(&mut self, kind: crate::ui::command_panel::Kind, window: &mut Window, cx: &mut Context<Self>) {
        use crate::ui::command_panel::{Kind, PanelEvent, Run};
        if self.command_panel.as_ref().is_some_and(|p| p.is_open() && p.value.0.read(cx).kind == kind) {
            self.close_command_panel(cx);
            return;
        }
        // the shown pane's repo, for the branches panel
        let dir = crate::ui::statusbar::pane_dir(self.snapshot.as_ref(), self.active_pane(window, cx).as_deref());
        let items = match kind {
            Kind::Commands => {
                let bindings = cx.try_global::<ActiveBindings>().map(|b| b.0.clone()).unwrap_or_default();
                crate::ui::command_panel::items(self.snapshot.as_ref(), &bindings)
            }
            Kind::Agents => crate::ui::command_panel::agent_items(self.snapshot.as_ref(), self.agent_sort),
            Kind::Branches => match dir.as_deref() {
                Some(d) => crate::ui::command_panel::branch_items(&crate::ui::statusbar::read_branches(d), crate::ui::statusbar::branch_of(d, cx).as_deref()),
                None => Vec::new(),
            },
        };
        let panel = cx.new(|cx| crate::ui::command_panel::Panel::new(kind, items, window, cx));
        let events = cx.subscribe_in(&panel, window, move |this, _, event: &PanelEvent, window, cx| {
            if !this.close_command_panel(cx) {
                return;
            }
            match event {
                // from the root, as the key binding would (focus goes back to herdr's pane on the next render)
                PanelEvent::Run(Run::Action(name)) => {
                    if let Ok(action) = cx.build_action(&format!("shepherd::{name}"), None) {
                        window.focus(&this.focus, cx);
                        window.dispatch_action(action, cx);
                    }
                }
                PanelEvent::Run(Run::Tab(tab_id)) => this.selected_tab = Some(tab_id.clone()),
                PanelEvent::Run(Run::Branch(branch)) => {
                    let (Some(dir), branch) = (dir.clone(), branch.clone()) else { return };
                    cx.spawn(async move |this, cx| {
                        let d = dir.clone();
                        let done = cx.background_spawn(async move { crate::ui::statusbar::switch_branch(&d, &branch) }).await;
                        let _ = this.update(cx, |this, cx| match done {
                            Ok(()) => crate::ui::statusbar::refresh_git(&dir, cx),
                            Err(e) => this.show_error(e, cx),
                        });
                    })
                    .detach();
                }
                PanelEvent::Dismiss => {}
            }
        });
        self.command_panel = Some(crate::ui::Popup::new((panel, events)));
        cx.notify();
    }

    /// Fades the command panel out; false if it was not open.
    fn close_command_panel(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(panel) = self.command_panel.as_mut().filter(|p| p.is_open()) else { return false };
        panel.close();
        // The panel's input held focus: give it back to herdr's focused pane on the next render.
        self.focused = None;
        cx.notify();
        true
    }

    fn close_settings(&mut self, cx: &mut Context<Self>) {
        self.settings = None;
        // Settings held focus: give it back to herdr's focused pane on the next render.
        self.focused = None;
        cx.notify();
    }

    /// Send a command to herdr; `then` gets the result. Errors show over the grid for 4 s.
    fn send(
        &mut self,
        command: Command,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Shell, &serde_json::Value, &mut Context<Shell>) + 'static,
    ) {
        self.send_then(command, cx, |shell, result, cx| match result {
            Ok(value) => then(shell, &value, cx),
            Err(err) => shell.show_error(format!("{err:#}"), cx),
        });
    }

    /// [`Self::send`] that sees failures too.
    fn send_then(
        &mut self,
        command: Command,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Shell, anyhow::Result<serde_json::Value>, &mut Context<Shell>) + 'static,
    ) {
        let (method, params) = command.request();
        let client = self.client.clone();
        let call = self.rt.spawn(async move { client.call(method, params).await });
        cx.spawn(async move |this, cx| {
            let result = call.await.map_err(anyhow::Error::from).and_then(|r| r);
            let _ = this.update(cx, |shell, cx| then(shell, result, cx));
        })
        .detach();
    }

    fn shown_tab(&self) -> Option<crate::model::Tab> {
        self.snapshot.as_ref()?.pick_tab_in(self.selected_tab.as_deref(), self.shown_workspace.as_deref()).cloned()
    }

    /// The tab you are looking at: none while the window is inactive or Settings covers the panes.
    fn tab_in_view(&self, window_active: bool) -> Option<String> {
        (window_active && self.settings.is_none()).then(|| self.shown_tab()).flatten().map(|t| t.tab_id)
    }

    /// Agents that just started waiting or finished out of sight: a notification each, and a place in the Dock count.
    fn agents_changed(&mut self, next: &Snapshot, cx: &mut Context<Self>) {
        let in_view = self.tab_in_view(cx.active_window().is_some());
        let settings = &SettingsGlobal::of(cx).settings;
        let (notify, sound) = (settings.notify_agents, settings.notify_sound);
        let Some(old) = self.snapshot.as_ref() else { return };
        for pane in crate::notify::arrivals(old, next) {
            if in_view.as_deref() == Some(pane.tab_id.as_str()) {
                continue;
            }
            self.unseen.insert(pane.pane_id.clone());
            if notify {
                let agent = pane.agent.as_deref().unwrap_or("Agent");
                let workspace = next.workspaces.iter().find(|w| w.workspace_id == pane.workspace_id).map(|w| w.label.as_str());
                let title = workspace.map_or_else(|| agent.to_string(), |w| format!("{agent} · {w}"));
                let status = if pane.agent_status == AgentStatus::Blocked { "Waiting for you" } else { "Done" };
                let body = match pane.terminal_title_stripped.as_deref().filter(|t| !t.is_empty()) {
                    Some(t) => format!("{status}: {t}"),
                    None => status.to_string(),
                };
                crate::notify::show(cx, &pane.pane_id, title, body, sound);
            }
        }
    }

    /// Agents leave the Dock count once their tab is in view or they moved on.
    fn sync_badge(&mut self, window: &Window, cx: &App) {
        let in_view = self.tab_in_view(window.is_window_active());
        if let Some(snapshot) = &self.snapshot {
            self.unseen.retain(|id| {
                snapshot.panes.iter().any(|p| {
                    &p.pane_id == id && crate::notify::wants_attention(p.agent_status) && in_view.as_deref() != Some(p.tab_id.as_str())
                })
            });
        }
        let count = if SettingsGlobal::of(cx).settings.dock_badge { self.unseen.len() } else { 0 };
        if count != self.badge {
            self.badge = count;
            crate::notify::set_badge(count);
        }
    }

    /// The pane keyboard actions apply to: the one with keyboard focus, else herdr's focused pane of the shown tab.
    fn active_pane(&self, window: &Window, cx: &App) -> Option<String> {
        if let Some((id, _)) = self.panes.iter().find(|(_, pane)| pane.read(cx).focus_handle(cx).is_focused(window)) {
            return Some(id.clone());
        }
        let (snapshot, tab) = (self.snapshot.as_ref()?, self.shown_tab()?);
        snapshot.pane_frames(&tab.tab_id).into_iter().find(|f| f.focused).map(|f| f.pane_id)
    }

    fn pane_command(&mut self, window: &Window, cx: &mut Context<Self>, make: impl FnOnce(String) -> Command) {
        if let Some(pane_id) = self.active_pane(window, cx) {
            self.send(make(pane_id), cx, |_, _, _| {});
        }
    }

    fn go_to_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        let (Some(snapshot), Some(tab)) = (self.snapshot.as_ref(), self.shown_tab()) else { return };
        if let Some(target) = snapshot.tabs_of(&tab.workspace_id).get(index) {
            self.selected_tab = Some(target.tab_id.clone());
            cx.notify();
        }
    }

    fn step_tab(&mut self, forward: bool, cx: &mut Context<Self>) {
        let (Some(snapshot), Some(tab)) = (self.snapshot.as_ref(), self.shown_tab()) else { return };
        let tabs = snapshot.tabs_of(&tab.workspace_id);
        let Some(i) = tabs.iter().position(|t| t.tab_id == tab.tab_id) else { return };
        let n = tabs.len();
        self.selected_tab = Some(tabs[if forward { (i + 1) % n } else { (i + n - 1) % n }].tab_id.clone());
        cx.notify();
    }

    fn step_workspace(&mut self, forward: bool, cx: &mut Context<Self>) {
        let (Some(snapshot), Some(tab)) = (self.snapshot.as_ref(), self.shown_tab()) else { return };
        // sidebar order, so Cmd+Up/Down walk the tree as shown
        let workspaces: Vec<_> = snapshot.workspace_tree().into_iter().map(|(w, _)| w).collect();
        let Some(i) = workspaces.iter().position(|w| w.workspace_id == tab.workspace_id) else { return };
        let n = workspaces.len();
        let next = &workspaces[if forward { (i + 1) % n } else { (i + n - 1) % n }];
        self.selected_tab = next.active_tab_id.clone().or_else(|| snapshot.tabs_of(&next.workspace_id).first().map(|t| t.tab_id.clone()));
        cx.notify();
    }

    fn new_tab(&mut self, workspace_id: String, cx: &mut Context<Self>) {
        self.send(Command::NewTab { workspace_id }, cx, |shell, result, cx| {
            shell.selected_tab = crate::commands::created_tab_id(result);
            cx.notify();
        });
    }

    fn open_worktree_dialog(&mut self, workspace_id: String, window: &mut Window, cx: &mut Context<Self>) {
        let theme = Theme::of(cx).clone();
        let input = cx.new(|cx| {
            // herdr's own generated name, shown so it can be kept or typed over.
            let mut input = InputState::new(window, cx).placeholder("Branch").default_value(new_branch_name());
            input.set_editor_style(crate::ui::input_style(&theme));
            input
        });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        let events = cx.subscribe_in(&input, window, |shell, _, event: &InputEvent, _, cx| {
            if let InputEvent::PressEnter { .. } = event {
                shell.finish_worktree_dialog(true, cx);
            }
        });
        self.worktree_dialog = Some(crate::ui::Popup::new(WorktreeDialog { workspace_id, input, _events: events }));
        cx.notify();
    }

    /// Create (an empty name lets herdr pick one) or cancel; either way the dialog fades out.
    fn finish_worktree_dialog(&mut self, create: bool, cx: &mut Context<Self>) {
        let Some(dialog) = self.worktree_dialog.as_mut().filter(|d| d.is_open()) else { return };
        dialog.close();
        // The input held focus: give it back to herdr's focused pane on the next render.
        self.focused = None;
        if create {
            let workspace_id = dialog.value.workspace_id.clone();
            let branch = dialog.value.input.read(cx).value().trim().to_string();
            let branch = if branch.is_empty() { new_branch_name() } else { branch };
            self.new_worktree(workspace_id, branch, cx);
        }
        cx.notify();
    }

    fn new_worktree(&mut self, workspace_id: String, branch: String, cx: &mut Context<Self>) {
        let pending = (workspace_id.clone(), branch.clone());
        self.pending_worktrees.push(pending.clone());
        self.send_then(Command::CreateWorktree { workspace_id, branch: Some(branch) }, cx, move |shell, result, cx| {
            shell.pending_worktrees.retain(|p| *p != pending);
            match result {
                Ok(value) => shell.selected_tab = crate::commands::created_tab_id(&value),
                Err(err) => shell.show_error(format!("{err:#}"), cx),
            }
            cx.notify();
        });
        cx.notify();
    }

    /// herdr deletes the checkout and closes the workspace; one with changes asks first, then goes with `force`.
    fn remove_worktree(&mut self, workspace_id: String, force: bool, cx: &mut Context<Self>) {
        self.removing.insert(workspace_id.clone());
        cx.notify();
        let id = workspace_id.clone();
        self.send_then(Command::RemoveWorktree { workspace_id, force }, cx, move |shell, result, cx| {
            // on success the row keeps its spinner until herdr's snapshot drops the workspace
            if let Err(err) = result {
                shell.removing.remove(&id);
                let message = format!("{err:#}");
                if message.starts_with("dirty_worktree_requires_force") {
                    let label = shell.snapshot.as_ref().and_then(|s| s.workspaces.iter().find(|w| w.workspace_id == id)).map(|w| w.label.clone()).unwrap_or_default();
                    shell.remove_confirm = Some(crate::ui::Popup::new((id.clone(), label)));
                } else {
                    shell.show_error(message, cx);
                }
                cx.notify();
            }
        });
    }

    fn finish_remove_confirm(&mut self, remove: bool, cx: &mut Context<Self>) {
        let Some(confirm) = self.remove_confirm.as_mut().filter(|c| c.is_open()) else { return };
        confirm.close();
        let workspace_id = confirm.value.0.clone();
        if remove {
            self.remove_worktree(workspace_id, true, cx);
        }
        cx.notify();
    }

    fn new_workspace(&mut self, cx: &mut Context<Self>) {
        self.send(Command::NewWorkspace { cwd: None }, cx, |shell, result, cx| {
            shell.selected_tab = crate::commands::created_tab_id(result);
            cx.notify();
        });
    }

    fn start_rename(&mut self, target: RenameTarget, label: String, window: &mut Window, cx: &mut Context<Self>) {
        let theme = Theme::of(cx).clone();
        let input = cx.new(|cx| InputState::new(window, cx).default_value(label));
        input.update(cx, |input, cx| {
            // gpui-base inputs are unstyled: colors from the theme, as Collie does.
            input.set_editor_style(crate::ui::input_style(&theme));
            input.focus(window, cx);
        });
        let events = cx.subscribe_in(&input, window, |shell, _, event: &InputEvent, _, cx| match event {
            InputEvent::PressEnter { .. } => shell.finish_rename(true, cx),
            InputEvent::Blur => shell.finish_rename(false, cx),
            _ => {}
        });
        self.renaming = Some(Rename { target, input, _events: events });
        cx.notify();
    }

    /// Enter renames in herdr; Escape and clicking away keep the old label.
    fn finish_rename(&mut self, commit: bool, cx: &mut Context<Self>) {
        let Some(rename) = self.renaming.take() else { return };
        // The input held focus: give it back to herdr's focused pane on the next render.
        self.focused = None;
        let label = rename.input.read(cx).value().trim().to_string();
        if commit && !label.is_empty() {
            let command = match rename.target {
                RenameTarget::Tab(tab_id) => Command::RenameTab { tab_id, label },
                RenameTarget::Workspace(workspace_id) => Command::RenameWorkspace { workspace_id, label },
            };
            self.send(command, cx, |_, _, _| {});
        }
        cx.notify();
    }

    /// The inline editor when `target` is being renamed.
    fn rename_field(&self, target: &RenameTarget, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let rename = self.renaming.as_ref().filter(|r| &r.target == target)?;
        Some(
            div()
                .flex_1()
                .min_w(px(80.))
                .capture_key_down(cx.listener(|shell, e: &gpui::KeyDownEvent, _, cx| {
                    if e.keystroke.key == "escape" {
                        cx.stop_propagation();
                        shell.finish_rename(false, cx);
                    }
                }))
                // Clicks in the field (a double-click selecting a word) must not reach the row and restart the rename.
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(Input::new(&rename.input)),
        )
    }

    fn show_error(&mut self, message: String, cx: &mut Context<Self>) {
        self.flash = Some(message.clone());
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(4)).await;
            let _ = this.update(cx, |shell, cx| {
                if shell.flash.as_deref() == Some(message.as_str()) {
                    shell.flash = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn start_divider(&mut self, tab_id: String, split: &SplitFrame, cx: &mut Context<Self>) {
        let Some(grid) = self.grid_bounds.get() else { return };
        let (origin, size) = if split.vertical {
            (f32::from(grid.origin.x) + f32::from(grid.size.width) * split.frame.x, f32::from(grid.size.width) * split.frame.w)
        } else {
            (f32::from(grid.origin.y) + f32::from(grid.size.height) * split.frame.y, f32::from(grid.size.height) * split.frame.h)
        };
        cx.set_global(GridHold(true));
        self.divider = Some(DividerDrag {
            tab_id,
            path: split.path.clone(),
            vertical: split.vertical,
            start_px: origin,
            len_px: size,
            ratio: split.ratio,
            throttle: Throttle::default(),
        });
        cx.notify();
    }

    fn drag_divider(&mut self, position: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = &mut self.divider else { return };
        let pointer = f32::from(if drag.vertical { position.x } else { position.y });
        drag.ratio = drag_ratio(drag.start_px, drag.len_px, pointer);
        if drag.throttle.ready(Instant::now()) {
            let command = Command::SetSplitRatio { tab_id: drag.tab_id.clone(), path: drag.path.clone(), ratio: drag.ratio };
            self.send(command, cx, |_, _, _| {});
        }
        cx.notify();
    }

    fn end_divider(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.divider.take() else { return };
        self.settling = Some((drag.path.clone(), drag.ratio));
        // The last position, unthrottled, so herdr ends where the pointer let go.
        self.send(Command::SetSplitRatio { tab_id: drag.tab_id, path: drag.path, ratio: drag.ratio }, cx, |_, _, _| {});
        // Panes held their grids during the drag: re-measure every one now (refresh bypasses view caching).
        cx.set_global(GridHold(false));
        cx.refresh_windows();
        // herdr normally answers within a few frames; never hold a stale preview longer than a second.
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let _ = this.update(cx, |shell, cx| {
                shell.settling = None;
                cx.notify();
            });
        })
        .detach();
    }

    /// The strip under the transparent titlebar drags the window; a double click zooms it.
    fn drag_strip(&self) -> Stateful<Div> {
        let (down, up, out, moved) =
            (self.should_move.clone(), self.should_move.clone(), self.should_move.clone(), self.should_move.clone());
        div()
            .id("titlebar-drag")
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h(px(TITLEBAR_HEIGHT))
            .window_control_area(WindowControlArea::Drag)
            .on_mouse_down_out(move |_, _, _| out.set(false))
            .on_mouse_up(MouseButton::Left, move |_, _, _| up.set(false))
            .on_mouse_down(MouseButton::Left, move |_, _, _| down.set(true))
            .on_mouse_move(move |e: &MouseMoveEvent, window, _| {
                if moved.get() && e.pressed_button == Some(MouseButton::Left) {
                    moved.set(false);
                    window.start_window_move();
                }
            })
            .on_click(|e: &ClickEvent, window, _| {
                if e.click_count() == 2 {
                    window.titlebar_double_click();
                }
            })
    }
}

/// Keep views of shown panes; focus is forgotten with its view so a recreated one gets keyboard focus again.
fn prune<S>(surfaces: &mut HashMap<String, S>, focused: &mut Option<String>, shown: &[PaneFrame], exited: impl Fn(&S) -> bool) {
    surfaces.retain(|id, s| shown.iter().any(|f| &f.pane_id == id) && !exited(s));
    if focused.as_ref().is_some_and(|id| !surfaces.contains_key(id)) {
        *focused = None;
    }
}

/// AppKit lays the traffic lights out again while the window is inactive (a screen recorder takes focus), and gpui
/// moves them back only when it becomes key; setting the title (hidden) re-applies `traffic_light_position`.
fn reapply_traffic_lights(window: &mut Window) {
    window.set_window_title("Shepherd");
}

/// Subscribe, then snapshot; every event (coalesced) triggers another snapshot. Reconnects forever.
async fn sync_loop(this: gpui::WeakEntity<Shell>, rt: tokio::runtime::Handle, client: Client, cx: &mut gpui::AsyncApp) {
    loop {
        let sub = {
            let c = client.clone();
            rt.spawn(async move { c.subscribe(EVENT_TYPES).await })
        };
        let mut rx = match sub.await.expect("tokio") {
            Ok(rx) => rx,
            Err(err) => {
                let alive = this.update(cx, |s, cx| {
                    s.error = Some(format!("{err:#}"));
                    cx.notify()
                });
                if alive.is_err() {
                    return;
                }
                cx.background_executor().timer(Duration::from_secs(1)).await;
                continue;
            }
        };
        loop {
            let c = client.clone();
            let result = rt.spawn(async move { c.call("session.snapshot", json!({})).await }).await.expect("tokio");
            let update = result.and_then(|r| Snapshot::from_result(&r));
            let alive = this.update(cx, |s, cx| {
                match update {
                    Ok(snapshot) => {
                        s.agents_changed(&snapshot, cx);
                        s.snapshot = Some(snapshot);
                        s.error = None;
                    }
                    Err(err) => s.error = Some(format!("{err:#}")),
                }
                cx.notify();
            });
            if alive.is_err() {
                return; // window closed
            }
            // ponytail: refetch the whole snapshot per event burst; incremental apply only if this shows up in profiles.
            if rx.recv().await.is_none() {
                break; // server gone or events_lost: resubscribe
            }
            cx.background_executor().timer(Duration::from_millis(30)).await;
            while rx.try_recv().is_ok() {}
        }
    }
}

pub(crate) fn status_color(status: AgentStatus, theme: &Theme) -> Option<Hsla> {
    match status {
        AgentStatus::Working => Some(theme.accent),
        AgentStatus::Blocked => Some(theme.warning),
        AgentStatus::Done => Some(theme.success.opacity(0.9)),
        AgentStatus::Idle | AgentStatus::Unknown => None,
    }
}

/// Collie's status dot: 8 px, filled for an agent state, a faint ring when idle.
pub(crate) fn status_dot(status: AgentStatus, theme: &Theme) -> Div {
    let dot = div().size(px(8.)).rounded_full().flex_none();
    match status_color(status, theme) {
        Some(c) => dot.bg(c),
        None => dot.border_1().border_color(theme.text_faint.opacity(0.6)),
    }
}

/// A small square text button (`+`).
fn icon_button(theme: &Theme, id: &'static str, glyph: &'static str) -> Stateful<Div> {
    let hover = theme.hover;
    div()
        .id(id)
        .size(px(22.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .text_size(px(15.))
        .text_color(theme.text_muted)
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .child(glyph)
}

/// Zeron's sidebar disclosure (`motion::COLLAPSE`): 180 ms CSS ease-out.
const FOLD_DURATION: Duration = Duration::from_millis(180);

/// CSS `ease-out`, cubic-bezier(0, 0, 0.58, 1), solved for x by bisection.
fn ease_out(x: f32) -> f32 {
    if x <= 0. || x >= 1. {
        return x.clamp(0., 1.);
    }
    let bezier = |p2: f32, t: f32| 3. * p2 * t * t * (1. - t) + t * t * t;
    let (mut lo, mut hi) = (0f32, 1f32);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.;
        if bezier(0.58, mid) < x { lo = mid } else { hi = mid }
    }
    bezier(1., (lo + hi) / 2.).clamp(0., 1.)
}

/// Zeron's resize-edge spring (`motion::resize_bounce_offset`): 5 px out over the first 32 % of 220 ms, then
/// home over the rest, smoothstep both ways.
const EDGE_SPRING: Duration = Duration::from_millis(220);

fn edge_nudge(t: f32) -> f32 {
    let smooth = |t: f32| {
        let t = t.clamp(0., 1.);
        t * t * (3. - 2. * t)
    };
    let out = 0.32;
    5. * if t < out { smooth(t / out) } else { 1. - smooth((t - out) / (1. - out)) }
}

/// Zeron's resize seam (`Shell::resize_handle`): a 1 px `border_strong` line `at` px into the hit strip that
/// fades out toward both ends. Shown while dragged; `hover` (hovered, flip count) fades it in and out like Zeron's
/// `HOVER_FADE`, restarting on every flip.
fn resize_seam(
    id: SharedString,
    vertical: bool,
    at: f32,
    color: Hsla,
    dragging: bool,
    hover: Option<(bool, u64)>,
) -> gpui::AnyElement {
    let clear = color.opacity(0.);
    let angle = if vertical { 180. } else { 90. };
    let half = |from, to| {
        div().flex_1().bg(gpui::linear_gradient(
            angle,
            gpui::linear_color_stop(from, 0.),
            gpui::linear_color_stop(to, 1.),
        ))
    };
    let line = div().absolute().flex().child(half(clear, color)).child(half(color, clear));
    let line = if vertical {
        line.flex_col().top_0().bottom_0().left(px(at)).w(px(1.))
    } else {
        line.flex_row().left_0().right_0().top(px(at)).h(px(1.))
    };
    match (dragging, hover) {
        (true, _) => line.into_any_element(),
        (false, Some((on, flips))) => line
            .with_animation(
                SharedString::from(format!("{id}-fade-{flips}")),
                gpui::Animation::new(SEAM_FADE).with_easing(ease_out),
                move |el, t| el.opacity(if on { t } else { 1. - t }),
            )
            .into_any_element(),
        (false, None) => line.invisible().into_any_element(),
    }
}

const SEAM_FADE: Duration = Duration::from_millis(150);

/// A pane divider's hit strip, centred on the panes' shared edge.
const PANE_SEAM_HIT: f32 = 10.;

/// A pane divider: a 1 px `rest` line always on the panes' shared edge (the strip's centre), lit `lit` while
/// hovered (fading like `resize_seam`) or dragged.
fn pane_seam(id: SharedString, vertical: bool, rest: Hsla, lit: Hsla, dragging: bool, hover: Option<(bool, u64)>) -> Div {
    let line = |color: Hsla| {
        let line = div().absolute().bg(color);
        let at = px((PANE_SEAM_HIT - 1.) / 2.);
        if vertical { line.top_0().bottom_0().left(at).w(px(1.)) } else { line.left_0().right_0().top(at).h(px(1.)) }
    };
    let glow = match (dragging, hover) {
        (true, _) => Some(line(lit).into_any_element()),
        (false, Some((on, flips))) => Some(
            line(lit)
                .with_animation(
                    SharedString::from(format!("{id}-lit-{flips}")),
                    gpui::Animation::new(SEAM_FADE).with_easing(ease_out),
                    move |el, t| el.opacity(if on { t } else { 1. - t }),
                )
                .into_any_element(),
        ),
        (false, None) => None,
    };
    div().absolute().inset_0().child(line(rest)).children(glow)
}

fn fold_animation() -> gpui::Animation {
    gpui::Animation::new(FOLD_DURATION).with_easing(ease_out)
}

/// A group body's height tween; `epoch` restarts the animation on every toggle.
#[derive(Clone, Copy)]
struct FoldMotion {
    epoch: u64,
    from: f32,
    to: f32,
    started: Instant,
}

impl FoldMotion {
    fn current(self) -> f32 {
        let t = (self.started.elapsed().as_secs_f32() / FOLD_DURATION.as_secs_f32()).min(1.);
        self.from + (self.to - self.from) * ease_out(t)
    }

    /// A little past the end, so the last frame lands before the static height takes over.
    fn animating(self) -> bool {
        self.started.elapsed() < FOLD_DURATION + Duration::from_millis(120)
    }
}

impl Shell {
    /// Folds or unfolds a worktree group, tweening from wherever its body is now.
    fn toggle_fold(&mut self, id: &str, full: f32) {
        let open = !self.folded.contains(id);
        let prev = self.fold_motion.get(id).copied();
        let from = prev.filter(|m| m.animating()).map(FoldMotion::current).unwrap_or(if open { full } else { 0. });
        let epoch = prev.map_or(1, |m| m.epoch + 1);
        self.fold_motion.insert(id.to_string(), FoldMotion { epoch, from, to: if open { 0. } else { full }, started: Instant::now() });
        if open {
            self.folded.insert(id.to_string());
        } else {
            self.folded.remove(id);
        }
    }

    /// The sidebar's width this frame: mid tween, or where it rests.
    fn sidebar_now(&self) -> f32 {
        match self.sidebar_motion.filter(|m| m.animating()) {
            Some(m) => m.current(),
            None if self.sidebar_open => self.sidebar_width + self.sidebar_nudge().unwrap_or(0.),
            None => 0.,
        }
    }

    /// The edge spring's offset while it plays.
    fn sidebar_nudge(&self) -> Option<f32> {
        let (sign, started) = self.sidebar_edge?;
        let t = started.elapsed().as_secs_f32() / EDGE_SPRING.as_secs_f32();
        (t < 1.).then(|| sign * edge_nudge(t))
    }

    /// Hides or shows the sidebar with the fold tween; panes keep their grids until it lands.
    fn hover_seam(&mut self, id: impl Into<SharedString>, on: bool, cx: &mut Context<Self>) {
        let entry = self.seam_hover.entry(id.into()).or_insert((false, 0));
        if entry.0 != on {
            *entry = (on, entry.1 + 1);
            cx.notify();
        }
    }

    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        let from = self.sidebar_now();
        self.sidebar_edge = None;
        self.sidebar_open = !self.sidebar_open;
        let to = if self.sidebar_open { self.sidebar_width } else { 0. };
        let motion = FoldMotion { epoch: 0, from, to, started: Instant::now() };
        self.sidebar_motion = Some(motion);
        cx.set_global(GridHold(true));
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(FOLD_DURATION + Duration::from_millis(120)).await;
            let _ = this.update(cx, |shell, cx| {
                // a later toggle or a drag owns the hold now
                if shell.sidebar_motion.is_some_and(|m| m.started == motion.started) && !shell.sidebar_drag && shell.divider.is_none() {
                    shell.sidebar_motion = None;
                    cx.set_global(GridHold(false));
                    cx.refresh_windows();
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn drag_sidebar(&mut self, x: Pixels, cx: &mut Context<Self>) {
        if !self.sidebar_drag {
            return;
        }
        let x = f32::from(x);
        let edge = if x <= SIDEBAR_MIN { -1. } else if x >= SIDEBAR_MAX { 1. } else { 0. };
        // one spring per arrival at a limit, not one per mouse move while held there
        if edge == 0. {
            self.sidebar_edge = None;
        } else if self.sidebar_edge.map(|(s, _)| s) != Some(edge) {
            self.sidebar_edge = Some((edge, Instant::now()));
        }
        self.sidebar_width = x.clamp(SIDEBAR_MIN, SIDEBAR_MAX);
        cx.notify();
    }

    fn end_sidebar_drag(&mut self, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.sidebar_drag) {
            cx.set_global(GridHold(false));
            self.save_sidebar_width(cx);
            cx.refresh_windows();
        }
    }

    fn save_sidebar_width(&self, cx: &mut Context<Self>) {
        let width = self.sidebar_width;
        save_from_sidebar(cx, |s| s.sidebar_width = width);
    }

    fn drag_agents(&mut self, y: Pixels, window: &Window, cx: &mut Context<Self>) {
        let Some((y0, h0)) = self.agents_drag else { return };
        // ponytail: capped at 60 % of the window so the workspaces list keeps some room
        let max = f32::from(window.viewport_size().height) * 0.6;
        self.agents_height = Some((h0 + y0 - f32::from(y)).clamp(AGENTS_MIN, max.max(AGENTS_MIN)));
        cx.notify();
    }

    fn end_agents_drag(&mut self, cx: &mut Context<Self>) {
        if self.agents_drag.take().is_some() {
            let height = self.agents_height;
            save_from_sidebar(cx, |s| s.agents_height = height);
        }
    }

    fn toggle_agents(&mut self, cx: &mut Context<Self>) {
        self.show_agents = !self.show_agents;
        let show = self.show_agents;
        save_from_sidebar(cx, |s| s.show_agents = show);
        cx.notify();
    }

    fn toggle_status_bar(&mut self, cx: &mut Context<Self>) {
        self.show_status_bar = !self.show_status_bar;
        let show = self.show_status_bar;
        save_from_sidebar(cx, |s| s.show_status_bar = show);
        cx.notify();
    }

    fn set_agent_sort(&mut self, sort: AgentSort, cx: &mut Context<Self>) {
        self.agent_sort = sort;
        if let Some(m) = self.view_menu.as_mut() { m.close() }
        save_from_sidebar(cx, |s| s.agent_sort = sort);
        cx.notify();
    }
}

/// Saves a choice made in the sidebar. A settings file that failed to load is the user's to fix: the sidebar must
/// not overwrite it with defaults.
fn save_from_sidebar(cx: &mut App, change: impl FnOnce(&mut crate::settings::Settings)) {
    if SettingsGlobal::of(cx).error.is_none() {
        crate::ui::settings::update(cx, change);
    }
}

/// A popup menu's panel.
pub(crate) fn menu_card(theme: &Theme, id: &'static str, min_w: f32) -> Stateful<Div> {
    div()
        .id(id)
        .occlude()
        .min_w(px(min_w))
        .p(px(4.))
        .rounded(px(10.))
        .bg(theme.card)
        .border_1()
        .border_color(theme.border)
        .shadow_lg()
}

/// Zeron's modal: a 35 % scrim over the window, the 360 px dialog card centred on it; both fade with the popup.
fn modal<T>(name: &'static str, popup: &crate::ui::Popup<T>, window: &Window, card: Stateful<Div>) -> impl IntoElement {
    let viewport = window.viewport_size();
    let scrim = div()
        .id(SharedString::from(format!("{name}-scrim")))
        .occlude()
        .w(viewport.width)
        .h(viewport.height)
        .bg(gpui::black().opacity(0.35))
        .flex()
        .items_center()
        .justify_center()
        .child(crate::ui::dialog_motion(format!("{name}-in"), popup, card, 2.));
    gpui::deferred(
        gpui::anchored()
            .position(point(px(0.), px(0.)))
            .child(crate::ui::dialog_motion(format!("{name}-scrim-in"), popup, scrim, 0.)),
    )
    .with_priority(2)
}

/// Zeron's dialog card (`popover::dialog_card`) with its title.
fn dialog_card(theme: &Theme, id: &'static str, title: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .w(px(360.))
        .p(px(20.))
        .rounded(px(16.))
        .bg(theme.card)
        .border_1()
        .border_color(theme.border)
        .shadow_lg()
        .flex()
        .flex_col()
        .text_color(theme.text)
        .child(div().text_size(px(15.)).font_weight(gpui::FontWeight::SEMIBOLD).child(title))
}

/// Zeron's `btnGhost`: quiet text, a wash on hover.
fn ghost_button(theme: &Theme, id: &'static str, label: &'static str) -> Stateful<Div> {
    let hover = theme.hover;
    div().id(id).px(px(12.)).py(px(6.)).rounded(px(8.)).text_color(theme.text_muted).cursor_pointer().hover(move |s| s.bg(hover)).child(label)
}

/// Zeron's `btnPrimary` (and `btnDestructive` with the danger fill).
fn solid_button(id: &'static str, label: &'static str, fill: gpui::Hsla, text: gpui::Hsla) -> Stateful<Div> {
    div()
        .id(id)
        .px(px(12.))
        .py(px(6.))
        .rounded(px(8.))
        .bg(fill)
        .text_color(text)
        .font_weight(gpui::FontWeight::MEDIUM)
        .cursor_pointer()
        .hover(|s| s.opacity(0.9))
        .child(label)
}

/// A row of the sidebar's `…` menu: icon, label, a check on the current choice.
fn menu_choice(theme: &Theme, id: &'static str, icon: &'static str, label: &'static str, checked: bool) -> Stateful<Div> {
    row(theme, false, id)
        .h(px(28.))
        .rounded(px(6.))
        .child(gpui::svg().path(icon).size(px(14.)).flex_none().text_color(theme.text_muted))
        .child(div().flex_1().child(label))
        .when(checked, |el| el.child(gpui::svg().path("icons/check.svg").size(px(14.)).text_color(theme.text_muted)))
}

/// Zeron's titlebar button: 24 px, a 16 px muted icon, a wash on hover. It occludes the drag strip under it.
fn titlebar_button(theme: &Theme, id: &'static str, icon: &'static str) -> Stateful<Div> {
    let hover = theme.hover;
    div()
        .id(id)
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .cursor_pointer()
        .hover(move |s| s.bg(hover))
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .child(gpui::svg().path(icon).size(px(16.)).text_color(theme.text_muted))
}

/// A branch name the way herdr generates one.
fn new_branch_name() -> String {
    let micros = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_micros() as u64);
    crate::commands::generated_branch(micros)
}

/// A 14 px spinner turning once every 900 ms.
fn spinner(theme: &Theme, id: String) -> impl IntoElement {
    gpui::svg()
        .path("icons/spinner.svg")
        .size(px(14.))
        .flex_none()
        .text_color(theme.text_faint)
        .with_animation(
            SharedString::from(id),
            gpui::Animation::new(std::time::Duration::from_millis(900)).repeat(),
            |svg, t| svg.with_transformation(gpui::Transformation::rotate(gpui::percentage(t))),
        )
}

/// A worktree herdr is still creating: its branch with a spinner, not selectable.
fn pending_worktree_row(theme: &Theme, branch: &str) -> Stateful<Div> {
    div()
        .id(SharedString::from(format!("ws-pending-{branch}")))
        .flex_shrink_0()
        .h(px(32.))
        .pl(px(30.))
        .pr(px(8.))
        .flex()
        .items_center()
        .gap(px(8.))
        .rounded(px(8.))
        .text_size(px(13.))
        .text_color(theme.text_faint)
        .child(spinner(theme, format!("ws-pending-spin-{branch}")))
        .child(div().flex_1().min_w_0().truncate().child(branch.to_string()))
}

/// Zeron's disclosure chevron: points right when folded, turns a quarter down as the group opens.
fn fold_chevron(theme: &Theme, id: &str, open: bool, full: f32, tween: Option<FoldMotion>) -> Stateful<Div> {
    let chevron = gpui::svg().path("icons/alt-arrow-right.svg").size(px(12.)).text_color(theme.text_muted.opacity(0.5));
    let turn = |reveal: f32| gpui::Transformation::rotate(gpui::percentage(reveal * 0.25));
    div()
        .id(SharedString::from(format!("ws-fold-{id}")))
        .size(px(16.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .map(|el| match tween {
            Some(m) => el.child(chevron.with_animation(
                SharedString::from(format!("fold-chevron-{id}-{}", m.epoch)),
                fold_animation(),
                move |el, t| el.with_transformation(turn((m.from + (m.to - m.from) * t) / full)),
            )),
            None => el.child(chevron.with_transformation(turn(if open { 1. } else { 0. }))),
        })
}

/// Collie's sidebar row: 32 px, 8 px radius, a wash under the selected one.
pub(crate) fn row(theme: &Theme, selected: bool, id: impl Into<gpui::ElementId>) -> Stateful<Div> {
    let hover = theme.hover;
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.))
        .h(px(32.))
        .px(px(8.))
        .rounded(px(8.))
        .flex_shrink_0()
        .cursor_pointer()
        .when(selected, |el| el.bg(theme.selected))
        .when(!selected, |el| el.hover(move |s| s.bg(hover)))
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_badge(window, cx);
        crate::ui::reap_popup(&mut self.view_menu, window);
        crate::ui::reap_popup(&mut self.ws_menu, window);
        crate::ui::reap_popup(&mut self.pane_menu, window);
        crate::ui::reap_popup(&mut self.spent_menu, window);
        crate::ui::reap_popup(&mut self.usage_menu, window);
        crate::ui::reap_popup(&mut self.tab_menu, window);
        crate::ui::reap_popup(&mut self.worktree_dialog, window);
        crate::ui::reap_popup(&mut self.remove_confirm, window);
        crate::ui::reap_popup(&mut self.command_panel, window);
        if let Some(s) = &self.snapshot {
            self.removing.retain(|id| s.workspaces.iter().any(|w| &w.workspace_id == id));
        }
        reapply_traffic_lights(window);
        let theme = Theme::of(cx).clone();
        let snapshot = self.snapshot.clone();
        let tab = self.shown_tab();
        self.shown_workspace = tab.as_ref().map(|t| t.workspace_id.clone());
        let (mut frames, mut splits) = match (&snapshot, &tab) {
            (Some(s), Some(t)) => (s.pane_frames(&t.tab_id), s.split_frames(&t.tab_id)),
            _ => (Vec::new(), Vec::new()),
        };
        // herdr caught up with a released divider: its snapshot is the layout again.
        if let Some((path, ratio)) = &self.settling {
            if splits.iter().any(|s| &s.path == path && (s.ratio - ratio).abs() < 0.02) {
                self.settling = None;
            }
        }
        // While a divider moves (or settles), lay the panes out at its ratio, not herdr's last one.
        let live = self.divider.as_ref().map(|d| (d.path.clone(), d.ratio)).or(self.settling.clone());
        if let Some((path, ratio)) = live {
            (frames, splits) = crate::model::preview(&frames, &splits, &path, ratio);
        }
        let own = self.own_pane.clone();
        let shown: Vec<PaneFrame> = frames.iter().filter(|f| attachable(&f.pane_id, own.as_deref())).cloned().collect();
        prune(&mut self.panes, &mut self.focused, &shown, |_| false);
        for f in &shown {
            if !self.panes.contains_key(&f.pane_id) {
                let id = f.pane_id.clone();
                self.panes.insert(id.clone(), cx.new(|cx| PaneTerminal::new(id, cx)));
            }
            if f.focused && self.settings.is_none() && self.focused.as_deref() != Some(f.pane_id.as_str()) {
                let handle = self.panes[&f.pane_id].read(cx).focus_handle(cx);
                window.focus(&handle, cx);
                self.focused = Some(f.pane_id.clone());
            }
        }

        if window.focused(cx).is_none() && self.settings.is_none() {
            window.focus(&self.focus, cx);
        }
        let shown_ws = tab.as_ref().map(|t| t.workspace_id.clone());
        let shown_tab = tab.as_ref().map(|t| t.tab_id.clone());
        let groups: Vec<(crate::model::Workspace, Vec<crate::model::Workspace>)> = snapshot
            .as_ref()
            .map(|s| {
                crate::model::groups(s.workspace_tree())
                    .into_iter()
                    .map(|(w, kids)| (w.clone(), kids.into_iter().cloned().collect()))
                    .collect()
            })
            .unwrap_or_default();
        let agents: Vec<(crate::model::Pane, String)> = snapshot
            .as_ref()
            .map(|s| {
                s.agents_by(self.agent_sort)
                    .into_iter()
                    .map(|p| {
                        let ws = s.workspaces.iter().find(|w| w.workspace_id == p.workspace_id);
                        (p.clone(), ws.map(|w| w.label.clone()).unwrap_or_default())
                    })
                    .collect()
            })
            .unwrap_or_default();
        let agent_workspaces: Vec<String> = agents.iter().map(|(p, _)| p.workspace_id.clone()).collect();
        let hover_bg = theme.hover;
        let sidebar_w = self.sidebar_now();
        let sliding = self.sidebar_motion.is_some_and(|m| m.animating());
        if sliding || self.sidebar_nudge().is_some() {
            window.request_animation_frame();
        }

        // Laid out at full width and clipped by its tweened box, so rows never reflow mid slide.
        let sidebar = div()
            .w(px(self.sidebar_width - 1.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .pt(px(TITLEBAR_HEIGHT))
            .child(
                div()
                    .px(px(16.))
                    .pt(px(8.))
                    .pb(px(6.))
                    .flex()
                    .items_center()
                    .text_size(px(11.))
                    .text_color(theme.text_faint)
                    .child("Workspaces"),
            )
            .child(
                div()
                    .id("workspaces")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(px(8.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .children({
                        let ws_row = |w: &crate::model::Workspace, nested: bool, chevron: Option<gpui::AnyElement>, cx: &mut Context<Self>| {
                            let selected = shown_ws.as_deref() == Some(w.workspace_id.as_str());
                            let first_tab =
                                snapshot.as_ref().and_then(|s| s.tabs_of(&w.workspace_id).first().map(|t| t.tab_id.clone()));
                            let target = w.active_tab_id.clone().or(first_tab);
                            let rename_target = RenameTarget::Workspace(w.workspace_id.clone());
                            let editing = self.rename_field(&rename_target, cx);
                            let (ws_id, label) = (w.workspace_id.clone(), w.label.clone());
                            row(&theme, selected, SharedString::from(format!("ws-{}", w.workspace_id)))
                                .map(|el| pane_drag::accept_pane_in_workspace(el, &w.workspace_id, theme.accent.opacity(0.15), cx))
                                .when(nested, |el| el.pl(px(30.)))
                                .child(
                                    gpui::svg()
                                        .path(if nested { "icons/git-branch.svg" } else { "icons/folder.svg" })
                                        .size(px(14.))
                                        .flex_none()
                                        .text_color(if nested { theme.text_faint } else { theme.text_muted }),
                                )
                                .map(|el| match editing {
                                    Some(field) => el.child(field),
                                    None => el.child(div().flex_1().min_w_0().truncate().child(w.label.clone())),
                                })
                                .map(|el| {
                                    if self.removing.contains(&w.workspace_id) {
                                        el.text_color(theme.text_faint).child(spinner(&theme, format!("ws-removing-{}", w.workspace_id)))
                                    } else {
                                        el.child(status_dot(w.agent_status, &theme))
                                    }
                                })
                                .children(chevron)
                                .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
                                    if e.click_count() == 2 {
                                        this.start_rename(RenameTarget::Workspace(ws_id.clone()), label.clone(), window, cx);
                                        return;
                                    }
                                    this.selected_tab = target.clone();
                                    cx.notify();
                                }))
                                .on_mouse_down(MouseButton::Right, cx.listener({
                                    let id = w.workspace_id.clone();
                                    move |this, e: &MouseDownEvent, _, cx| {
                                        this.ws_menu = Some(crate::ui::Popup::new((id.clone(), e.position)));
                                        cx.notify();
                                    }
                                }))
                        };
                        groups.into_iter().map(|(w, kids)| {
                            // Pending worktrees join the group of the workspace their dialog was opened on.
                            // herdr opens the worktree's workspace before it answers (hooks still run): once
                            // its checkout shows up, the real row replaces the placeholder.
                            let checked_out = |branch: &str| {
                                let slug = crate::commands::branch_path_slug(branch);
                                kids.iter().any(|k| {
                                    k.worktree.as_ref().is_some_and(|t| {
                                        std::path::Path::new(&t.checkout_path).file_name().is_some_and(|n| *n == *slug)
                                    })
                                })
                            };
                            let pending: Vec<String> = self
                                .pending_worktrees
                                .iter()
                                .filter(|(id, _)| *id == w.workspace_id || kids.iter().any(|k| k.workspace_id == *id))
                                .filter(|(_, branch)| !checked_out(branch))
                                .map(|(_, branch)| branch.clone())
                                .collect();
                            if kids.is_empty() && pending.is_empty() {
                                return ws_row(&w, false, None, cx).into_any_element();
                            }
                            // A folded group opens while one of its worktrees is being created.
                            let open = !self.folded.contains(&w.workspace_id) || !pending.is_empty();
                            let full = (kids.len() + pending.len()) as f32 * 34.;
                            let tween = self.fold_motion.get(&w.workspace_id).copied().filter(|m| m.animating());
                            let chevron = fold_chevron(&theme, &w.workspace_id, open, full, tween).on_click(cx.listener({
                                let id = w.workspace_id.clone();
                                move |this, _: &ClickEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.toggle_fold(&id, full);
                                    cx.notify();
                                }
                            }));
                            let header = ws_row(&w, false, Some(chevron.into_any_element()), cx);
                            let body = div()
                                .w_full()
                                .flex_none()
                                .overflow_hidden()
                                .flex()
                                .flex_col()
                                .pt(px(2.))
                                .gap(px(2.))
                                .children(kids.iter().map(|k| ws_row(k, true, None, cx).into_any_element()).collect::<Vec<_>>())
                                .children(pending.iter().map(|b| pending_worktree_row(&theme, b).into_any_element()));
                            let body = match tween {
                                None => body.h(px(if open { full } else { 0. })).into_any_element(),
                                // Zeron's disclosure: the height tweens, the rows fade up from 35 % and slide 3 px.
                                Some(m) => body
                                    .with_animation(
                                        SharedString::from(format!("fold-{}-{}", w.workspace_id, m.epoch)),
                                        fold_animation(),
                                        move |el, t| {
                                            let h = m.from + (m.to - m.from) * t;
                                            let reveal = (h / full).clamp(0., 1.);
                                            el.h(px(h)).opacity(0.35 + 0.65 * reveal).relative().top(px(-3. * (1. - reveal)))
                                        },
                                    )
                                    .into_any_element(),
                            };
                            div().flex().flex_col().flex_none().child(header).child(body).into_any_element()
                        }).collect::<Vec<_>>()
                    }),
            )
            .child(
                div()
                    .id("new-workspace")
                    .flex_none()
                    .m(px(8.))
                    .h(px(32.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .rounded(px(8.))
                    .text_color(theme.text_muted)
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover_bg))
                    .child(div().text_size(px(15.)).child("+"))
                    .child("New workspace")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.new_workspace(cx))),
            )
            .when(self.show_agents && !agents.is_empty(), |el| {
                let seam = theme.border_strong;
                let dragging = self.agents_drag.is_some();
                el.child(
                    div()
                        .relative()
                        .px(px(16.))
                        .pt(px(12.))
                        .pb(px(6.))
                        .border_t_1()
                        .border_color(theme.border)
                        .text_size(px(11.))
                        .text_color(theme.text_faint)
                        .child("Agents")
                        // The section's top edge: drag to resize the list, double-click for the default height.
                        .child(
                            div()
                                .id("agents-resize")
                                .on_hover(cx.listener(|this, on: &bool, _, cx| this.hover_seam("agents-resize", *on, cx)))
                                .absolute()
                                .top(px(-3.))
                                .left_0()
                                .right_0()
                                .h(px(6.))
                                .cursor_row_resize()
                                .child(resize_seam("agents-resize".into(), false, 3., seam, dragging, self.seam_hover.get("agents-resize").copied()))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, e: &MouseDownEvent, _, cx| {
                                        cx.stop_propagation();
                                        let height = f32::from(this.agents_scroll.bounds().size.height);
                                        this.agents_drag = Some((f32::from(e.position.y), height));
                                        cx.notify();
                                    }),
                                )
                                .on_click(cx.listener(|this, e: &ClickEvent, _, cx| {
                                    if e.click_count() == 2 {
                                        this.agents_height = None;
                                        save_from_sidebar(cx, |s| s.agents_height = None);
                                        cx.notify();
                                    }
                                })),
                        ),
                )
                .child(
                    div()
                        .relative()
                        .flex_none()
                        .map(|el| match self.agents_height {
                            Some(h) => el.max_h(px(h)),
                            None => el.max_h(relative(0.4)),
                        })
                        .flex()
                        .flex_col()
                        .child(
                        div()
                            .id("agents")
                            .min_h_0()
                            .overflow_y_scroll()
                            .track_scroll(&self.agents_scroll)
                            .px(px(8.))
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .children(agents.into_iter().enumerate().flat_map(|(i, (pane, workspace))| {
                                // Grouped: a heading where the workspace changes, which the rows then leave out.
                                let grouped = self.agent_sort == AgentSort::Grouped;
                                let heading = (grouped && (i == 0 || agent_workspaces[i - 1] != pane.workspace_id)).then(|| {
                                    div()
                                        .flex_none()
                                        .px(px(8.))
                                        .pt(px(if i == 0 { 2. } else { 8. }))
                                        .pb(px(2.))
                                        .truncate()
                                        .text_size(px(11.))
                                        .text_color(theme.text_faint)
                                        .child(workspace.clone())
                                        .into_any_element()
                                });
                                let selected = shown_tab.as_deref() == Some(pane.tab_id.as_str());
                                let tab_id = pane.tab_id.clone();
                                let title = pane.terminal_title_stripped.clone().unwrap_or_default();
                                let agent = row(&theme, selected, SharedString::from(format!("agent-{}", pane.pane_id)))
                                    .h(px(44.))
                                    .child(status_dot(pane.agent_status, &theme))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .flex()
                                                    .gap(px(6.))
                                                    .child(pane.agent.clone().unwrap_or_default())
                                                    .when(!grouped, |el| {
                                                        el.child(
                                                            div()
                                                                .min_w_0()
                                                                .truncate()
                                                                .text_color(theme.text_faint)
                                                                .child(workspace),
                                                        )
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .truncate()
                                                    .text_size(px(11.))
                                                    .text_color(theme.text_muted)
                                                    .child(title),
                                            ),
                                    )
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        this.selected_tab = Some(tab_id.clone());
                                        cx.notify();
                                    }));
                                heading.into_iter().chain([agent.into_any_element()])
                            })),
                    )
                    .child(crate::ui::scrollbar("agents-scrollbar", &self.agents_scroll, &theme)),
                )
            });

        let tabbar = self.render_tab_bar(&theme, shown_ws.clone(), shown_tab.clone(), sidebar_w, window, cx);

        let shown_tab_id = shown_tab.clone().unwrap_or_default();
        let grid_bounds = self.grid_bounds.clone();
        let ring = theme.accent.opacity(0.5);
        let split = frames.len() > 1;
        let pane_drop = self.pane_drop_overlay(&frames, &shown_tab_id, &theme, window, cx);
        let edges: Vec<f32> = splits.iter().map(|s| crate::model::split_edge(&frames, s)).collect();
        let grid = div()
            .flex_1()
            .min_h_0()
            .relative()
            .overflow_hidden()
            .bg(theme.terminal.background)
            .child(
                canvas(move |bounds, _, _| grid_bounds.set(Some(bounds)), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            )
            .when_some(self.error.clone(), |el, err| el.child(div().p_4().text_color(theme.text_muted).child(err)))
            .when(snapshot.is_none() && self.error.is_none(), |el| {
                el.child(div().p_4().text_color(theme.text_muted).child("Connecting to herdr…"))
            })
            .children(frames.into_iter().map(|f| {
                let cell = div()
                    .absolute()
                    .left(relative(f.frame.x))
                    .top(relative(f.frame.y))
                    .w(relative(f.frame.w))
                    .h(relative(f.frame.h));
                match self.panes.get(&f.pane_id) {
                    Some(pane) => {
                        // a lone pane needs no marker of which pane has focus
                        let focused = split && pane.read(cx).focus_handle(cx).is_focused(window);
                        let pane_id = f.pane_id.clone();
                        let group = SharedString::from(format!("pane-{}", f.pane_id));
                        let grip = self.pane_grip(&f.pane_id, &shown_tab_id, &group, &theme, cx);
                        cell.group(group).on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                                this.pane_menu = Some(crate::ui::Popup::new((pane_id.clone(), e.position)));
                                cx.notify();
                            }),
                        )
                        .child(
                            div().size_full().relative().child(pane.clone()).when(focused, |el| {
                                // ponytail: overlay, not a border on the container, which shrank the grid by 1px
                                el.child(div().absolute().inset_0().border_1().border_color(ring))
                            }),
                        )
                        .child(grip)
                    }
                    None => cell.child(
                        div()
                            .size_full()
                            .p_4()
                            .bg(theme.terminal.background)
                            .text_color(theme.text_muted)
                            .child(format!("{} is this Shepherd's own pane", f.pane_id)),
                    ),
                }
            }));
        let (seam, seam_lit) = (theme.border_strong, theme.accent);
        let grid = grid
            .children(splits.into_iter().zip(edges).map(|(split, edge)| {
                let dragging = self.divider.as_ref().is_some_and(|d| d.path == split.path);
                let id = SharedString::from(format!("split-{}", split.path.iter().map(|b| if *b { '1' } else { '0' }).collect::<String>()));
                // The hit strip is centred on the panes' shared edge (`edge`, already live while dragging).
                let strip = if split.vertical {
                    div()
                        .left(relative(edge))
                        .ml(px(-PANE_SEAM_HIT / 2.))
                        .top(relative(split.frame.y))
                        .h(relative(split.frame.h))
                        .w(px(PANE_SEAM_HIT))
                        .cursor_col_resize()
                } else {
                    div()
                        .top(relative(edge))
                        .mt(px(-PANE_SEAM_HIT / 2.))
                        .left(relative(split.frame.x))
                        .w(relative(split.frame.w))
                        .h(px(PANE_SEAM_HIT))
                        .cursor_row_resize()
                };
                let tab_id = shown_tab_id.clone();
                strip
                    .id(id.clone())
                    .on_hover({
                        let id = id.clone();
                        cx.listener(move |this, on: &bool, _, cx| this.hover_seam(id.clone(), *on, cx))
                    })
                    .absolute()
                    .child(pane_seam(id.clone(), split.vertical, seam, seam_lit, dragging, self.seam_hover.get(&id).copied()))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            this.start_divider(tab_id.clone(), &split, cx);
                        }),
                    )
            }))
            .children(pane_drop)
            .when_some(self.flash.clone(), |el, message| {
                el.child(
                    div()
                        .absolute()
                        .top(px(8.))
                        .left(px(8.))
                        .right(px(8.))
                        .px_3()
                        .py_2()
                        .rounded(px(8.))
                        .bg(theme.card)
                        .border_1()
                        .border_color(theme.danger.opacity(0.5))
                        .text_color(theme.danger)
                        .child(message),
                )
            });

        let statusbar = self.show_status_bar.then(|| {
            let pane = self.active_pane(window, cx);
            let on_branch = cx.listener(|this, _: &ClickEvent, window, cx| {
                this.toggle_command_panel(crate::ui::command_panel::Kind::Branches, window, cx)
            });
            let on_spent = cx.listener(|this, _: &ClickEvent, window, cx| {
                let at = gpui::point(window.mouse_position().x - px(12.), window.viewport_size().height - px(crate::ui::statusbar::HEIGHT + 4.));
                this.spent_menu = Some(crate::ui::Popup::new(at));
                cx.notify();
            });
            let on_usage = cx.listener(|this, _: &ClickEvent, window, cx| {
                // anchored at its right edge: the limits sit at the bar's right end
                let at = gpui::point(window.mouse_position().x + px(12.), window.viewport_size().height - px(crate::ui::statusbar::HEIGHT + 4.));
                this.usage_menu = Some(crate::ui::Popup::new(at));
                cx.notify();
            });
            crate::ui::statusbar::bar(snapshot.as_ref(), pane.as_deref(), &theme, on_branch, on_spent, on_usage, cx)
        });
        let settings_page = self.settings.as_ref().map(|(view, _)| view.clone());
        let on_page = settings_page.is_some();
        div()
            .id("root")
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .flex()
            .flex_row()
            .bg(theme.glass())
            .text_color(theme.text)
            .font_family(theme.font_ui.clone())
            .text_size(px(13.))
            .on_action(cx.listener(|this, _: &NewTab, _, cx| {
                if let Some(tab) = this.shown_tab() {
                    this.new_tab(tab.workspace_id, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &CloseTab, _, cx| {
                if let Some(tab) = this.shown_tab() {
                    this.send(Command::CloseTab { tab_id: tab.tab_id }, cx, |_, _, _| {});
                }
            }))
            .on_action(cx.listener(|this, _: &NewWorkspace, _, cx| this.new_workspace(cx)))
            .on_action(cx.listener(|this, _: &SplitRight, w, cx| this.pane_command(w, cx, |pane_id| Command::Split { pane_id, down: false })))
            .on_action(cx.listener(|this, _: &SplitDown, w, cx| this.pane_command(w, cx, |pane_id| Command::Split { pane_id, down: true })))
            .on_action(cx.listener(|this, _: &ClosePane, w, cx| this.pane_command(w, cx, |pane_id| Command::ClosePane { pane_id })))
            .on_action(cx.listener(|this, _: &ToggleZoom, w, cx| this.pane_command(w, cx, |pane_id| Command::Zoom { pane_id })))
            .on_action(cx.listener(|this, _: &FocusLeft, w, cx| this.pane_command(w, cx, |pane_id| Command::FocusPane { pane_id, direction: Direction::Left })))
            .on_action(cx.listener(|this, _: &FocusRight, w, cx| this.pane_command(w, cx, |pane_id| Command::FocusPane { pane_id, direction: Direction::Right })))
            .on_action(cx.listener(|this, _: &FocusUp, w, cx| this.pane_command(w, cx, |pane_id| Command::FocusPane { pane_id, direction: Direction::Up })))
            .on_action(cx.listener(|this, _: &FocusDown, w, cx| this.pane_command(w, cx, |pane_id| Command::FocusPane { pane_id, direction: Direction::Down })))
            .on_action(cx.listener(|this, _: &Tab1, _, cx| this.go_to_tab(0, cx)))
            .on_action(cx.listener(|this, _: &Tab2, _, cx| this.go_to_tab(1, cx)))
            .on_action(cx.listener(|this, _: &Tab3, _, cx| this.go_to_tab(2, cx)))
            .on_action(cx.listener(|this, _: &Tab4, _, cx| this.go_to_tab(3, cx)))
            .on_action(cx.listener(|this, _: &Tab5, _, cx| this.go_to_tab(4, cx)))
            .on_action(cx.listener(|this, _: &Tab6, _, cx| this.go_to_tab(5, cx)))
            .on_action(cx.listener(|this, _: &Tab7, _, cx| this.go_to_tab(6, cx)))
            .on_action(cx.listener(|this, _: &Tab8, _, cx| this.go_to_tab(7, cx)))
            .on_action(cx.listener(|this, _: &Tab9, _, cx| this.go_to_tab(8, cx)))
            .on_action(cx.listener(|this, _: &PreviousTab, _, cx| this.step_tab(false, cx)))
            .on_action(cx.listener(|this, _: &NextTab, _, cx| this.step_tab(true, cx)))
            .on_action(cx.listener(|this, _: &PreviousWorkspace, _, cx| this.step_workspace(false, cx)))
            .on_action(cx.listener(|this, _: &NextWorkspace, _, cx| this.step_workspace(true, cx)))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| this.toggle_sidebar(cx)))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| this.toggle_settings_here(window, cx)))
            .on_action(cx.listener(|this, _: &CommandPanel, w, cx| this.toggle_command_panel(crate::ui::command_panel::Kind::Commands, w, cx)))
            .on_action(cx.listener(|this, _: &AgentsPanel, w, cx| this.toggle_command_panel(crate::ui::command_panel::Kind::Agents, w, cx)))
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, w, cx| {
                this.drag_divider(e.position, cx);
                this.drag_sidebar(e.position.x, cx);
                this.drag_agents(e.position.y, w, cx);
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.end_divider(cx);
                this.end_sidebar_drag(cx);
                this.end_agents_drag(cx);
            }))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.end_divider(cx);
                this.end_sidebar_drag(cx);
                this.end_agents_drag(cx);
            }))
            .child(self.drag_strip())
            .when(sidebar_w > 0. && !on_page, |el| {
                el.child(
                    div()
                        .w(px(sidebar_w))
                        .h_full()
                        .flex_none()
                        .overflow_hidden()
                        .bg(theme.wash(0.05))
                        .border_r_1()
                        .border_color(theme.border)
                        .child(sidebar),
                )
            })
            .map(|el| match settings_page {
                Some(page) => el.child(page),
                None => el.child(div().flex_1().min_w_0().h_full().flex().flex_col().child(tabbar).child(grid).children(statusbar)),
            })
            // The sidebar's edge: drag to resize, double-click for the default width.
            .when(self.sidebar_open && !sliding && !on_page, |el| {
                el.child(
                    div()
                        .id("sidebar-resize")
                        .on_hover(cx.listener(|this, on: &bool, _, cx| this.hover_seam("sidebar-resize", *on, cx)))
                        .absolute()
                        .top(px(TITLEBAR_HEIGHT))
                        .bottom_0()
                        .left(px(sidebar_w - 3.))
                        .w(px(6.))
                        .cursor_col_resize()
                        // The sidebar's 1 px border sits at x = 2 of the strip.
                        .child(resize_seam(
                            "sidebar-resize".into(),
                            true,
                            2.,
                            theme.border_strong,
                            self.sidebar_drag,
                            self.seam_hover.get("sidebar-resize").copied(),
                        ))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _: &MouseDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.sidebar_drag = true;
                                cx.set_global(GridHold(true));
                                cx.notify();
                            }),
                        )
                        .on_click(cx.listener(|this, e: &ClickEvent, _, cx| {
                            if e.click_count() == 2 {
                                this.sidebar_width = SIDEBAR_WIDTH;
                                this.save_sidebar_width(cx);
                                cx.notify();
                            }
                        })),
                )
            })
            .when(!on_page, |el| {
                el.child(
                    titlebar_button(&theme, "toggle-sidebar", "icons/sidebar-minimalistic-left.svg")
                        .absolute()
                        .top(px(TRAFFIC_LIGHTS_CENTER - 12.))
                        .left(px(TOGGLE_LEFT))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            this.toggle_sidebar(cx);
                        })),
                )
            })
            // Zeron's sidebar `…`: in the titlebar at the sidebar's right edge, hidden once the sidebar is too
            // narrow to hold it beside the toggle.
            .when(sidebar_w >= TITLEBAR_CLUSTER + 32. && !on_page, |el| {
                el.child(
                    titlebar_button(&theme, "sidebar-view", "icons/more-horizontal.svg")
                        .absolute()
                        .top(px(TRAFFIC_LIGHTS_CENTER - 12.))
                        .left(px(sidebar_w - 8. - 24.))
                        .when(self.view_menu.as_ref().is_some_and(|m| m.is_open()), |el| el.bg(theme.hover))
                        // Capture, so a press on the open menu's trigger closes it instead of the menu's
                        // click-outside closing it and the click reopening it.
                        .capture_any_mouse_down(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            match this.view_menu.as_mut() {
                                Some(menu) if menu.is_open() => menu.close(),
                                _ => this.view_menu = Some(crate::ui::Popup::new(())),
                            }
                            cx.notify();
                        })),
                )
            })
            .children(self.view_menu.as_ref().map(|menu| {
                // under the `…` button, its left edge on the button's
                let at = gpui::point(px(sidebar_w - 8. - 24.), px(TRAFFIC_LIGHTS_CENTER + 12. + 4.));
                let sort = self.agent_sort;
                let show_agents = self.show_agents;
                let show_status_bar = self.show_status_bar;
                gpui::deferred(
                    gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(crate::ui::menu_motion(
                        "view-menu-in",
                        menu,
                        menu_card(&theme, "view-menu", 200.)
                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                if let Some(m) = this.view_menu.as_mut() { m.close() }
                                cx.notify();
                            }))
                            .child(div().px(px(8.)).pt(px(4.)).pb(px(2.)).text_size(px(11.)).text_color(theme.text_faint).child("Sort agents"))
                            .children([
                                (AgentSort::Status, "sort-status", "icons/checklist.svg", "Status"),
                                (AgentSort::Grouped, "sort-grouped", "icons/folder.svg", "Grouped"),
                            ]
                            .map(|(choice, id, icon, label)| {
                                menu_choice(&theme, id, icon, label, sort == choice)
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_agent_sort(choice, cx)))
                            }))
                            .child(div().mx(px(-4.)).my(px(4.)).h(px(1.)).bg(theme.border))
                            .child(
                                row(&theme, false, "show-agents")
                                    .h(px(30.))
                                    .rounded(px(6.))
                                    .child(div().flex_1().child("Show agents"))
                                    .child(crate::ui::switch(&theme, show_agents, "show-agents"))
                                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_agents(cx))),
                            )
                            .child(
                                row(&theme, false, "show-status-bar")
                                    .h(px(30.))
                                    .rounded(px(6.))
                                    .child(div().flex_1().child("Show status bar"))
                                    .child(crate::ui::switch(&theme, show_status_bar, "show-status-bar"))
                                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_status_bar(cx))),
                            )
                            .child(div().mx(px(-4.)).my(px(4.)).h(px(1.)).bg(theme.border))
                            .child(menu_choice(&theme, "open-settings", "icons/settings.svg", "Settings", false).on_click(
                                cx.listener(|this, _: &ClickEvent, window, cx| {
                                    if let Some(m) = this.view_menu.as_mut() {
                                        m.close()
                                    }
                                    this.toggle_settings_here(window, cx);
                                }),
                            )),
                    )),
                )
                .with_priority(1)
            }))
            .children(self.ws_menu.as_ref().map(|menu| {
                let (id, at) = menu.value.clone();
                let worktree = snapshot
                    .as_ref()
                    .and_then(|s| s.workspaces.iter().find(|w| w.workspace_id == id))
                    .and_then(|w| w.worktree.clone());
                // a repo's checkout makes worktrees; a linked worktree removes itself instead
                let linked = worktree.as_ref().is_some_and(|w| w.is_linked_worktree);
                let has_repo = worktree.is_some() && !linked;
                let checkout = worktree.map(|w| w.checkout_path).filter(|p| !p.is_empty());
                gpui::deferred(
                    gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(crate::ui::menu_motion(
                        "ws-menu-in",
                        menu,
                        menu_card(&theme, "ws-menu", 180.)
                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                if let Some(m) = this.ws_menu.as_mut() { m.close() }
                                cx.notify();
                            }))
                            .when(has_repo, |el| {
                                el.child(row(&theme, false, "ws-menu-worktree").h(px(28.)).rounded(px(6.)).child("New worktree").on_click(
                                    cx.listener({
                                        let id = id.clone();
                                        move |this, _: &ClickEvent, window, cx| {
                                            if let Some(m) = this.ws_menu.as_mut() { m.close() }
                                            this.open_worktree_dialog(id.clone(), window, cx);
                                        }
                                    }),
                                ))
                            })
                            .child(row(&theme, false, "ws-menu-new-tab").h(px(28.)).rounded(px(6.)).child("New tab").on_click(
                                cx.listener({
                                    let id = id.clone();
                                    move |this, _: &ClickEvent, _, cx| {
                                        if let Some(m) = this.ws_menu.as_mut() { m.close() }
                                        this.new_tab(id.clone(), cx);
                                    }
                                }),
                            ))
                            .child(row(&theme, false, "ws-menu-rename").h(px(28.)).rounded(px(6.)).child("Rename").on_click(
                                cx.listener({
                                    let id = id.clone();
                                    move |this, _: &ClickEvent, window, cx| {
                                        if let Some(m) = this.ws_menu.as_mut() { m.close() }
                                        let label = this
                                            .snapshot
                                            .as_ref()
                                            .and_then(|s| s.workspaces.iter().find(|w| w.workspace_id == id))
                                            .map(|w| w.label.clone())
                                            .unwrap_or_default();
                                        this.start_rename(RenameTarget::Workspace(id.clone()), label, window, cx);
                                    }
                                }),
                            ))
                            .when_some(checkout, |el, path| {
                                el.child(row(&theme, false, "ws-menu-reveal").h(px(28.)).rounded(px(6.)).child("Reveal in Finder").on_click(
                                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        if let Some(m) = this.ws_menu.as_mut() { m.close() }
                                        cx.reveal_path(std::path::Path::new(&path));
                                        cx.notify();
                                    }),
                                ))
                            })
                            .child(div().mx(px(-4.)).my(px(4.)).h(px(1.)).bg(theme.border))
                            .child(
                                row(&theme, false, "ws-menu-close")
                                    .h(px(28.))
                                    .rounded(px(6.))
                                    .text_color(theme.danger)
                                    .child("Close workspace")
                                    .on_click(cx.listener({
                                        let id = id.clone();
                                        move |this, _: &ClickEvent, _, cx| {
                                            if let Some(m) = this.ws_menu.as_mut() { m.close() }
                                            this.send(Command::CloseWorkspace { workspace_id: id.clone() }, cx, |_, _, _| {});
                                        }
                                    })),
                            )
                            .when(linked, |el| {
                                el.child(
                                    row(&theme, false, "ws-menu-remove")
                                        .h(px(28.))
                                        .rounded(px(6.))
                                        .text_color(theme.danger)
                                        .child("Remove worktree")
                                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                            if let Some(m) = this.ws_menu.as_mut() { m.close() }
                                            this.remove_worktree(id.clone(), false, cx);
                                        })),
                                )
                            }),
                    )),
                )
                .with_priority(1)
            }))
            .children(self.worktree_dialog.as_ref().map(|dialog| {
                let card = dialog_card(&theme, "worktree-dialog", "New worktree")
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| this.finish_worktree_dialog(false, cx)))
                    .on_key_down(cx.listener(|this, e: &gpui::KeyDownEvent, _, cx| {
                        if e.keystroke.key == "escape" {
                            cx.stop_propagation();
                            this.finish_worktree_dialog(false, cx);
                        }
                    }))
                    .child(div().mt(px(4.)).text_color(theme.text_muted).child("Branch name"))
                    .child(
                        div()
                            .mt(px(8.))
                            .px(px(12.))
                            .py(px(8.))
                            .rounded(px(8.))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.wash(0.04))
                            .text_size(px(14.))
                            .child(Input::new(&dialog.value.input)),
                    )
                    .child(
                        div()
                            .mt(px(16.))
                            .flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                ghost_button(&theme, "worktree-cancel", "Cancel")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_worktree_dialog(false, cx))),
                            )
                            .child(
                                solid_button("worktree-create", "Create", theme.text, theme.surface)
                                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_worktree_dialog(true, cx))),
                            ),
                    );
                modal("worktree-dialog", dialog, window, card)
            }))
            .children(self.command_panel.as_ref().map(|panel| crate::ui::command_panel::overlay(panel, window)))
            .children(self.remove_confirm.as_ref().map(|confirm| {
                let label = confirm.value.1.clone();
                let card = dialog_card(&theme, "remove-dialog", "Remove worktree?")
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| this.finish_remove_confirm(false, cx)))
                    .child(
                        div()
                            .mt(px(8.))
                            .line_height(px(19.))
                            .text_color(theme.text_muted)
                            .child(format!("{label} has uncommitted changes. Removing it deletes them; its branch stays.")),
                    )
                    .child(
                        div()
                            .mt(px(16.))
                            .flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                ghost_button(&theme, "remove-cancel", "Cancel")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_remove_confirm(false, cx))),
                            )
                            .child(
                                solid_button("remove-force", "Remove anyway", theme.danger, gpui::white())
                                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_remove_confirm(true, cx))),
                            ),
                    );
                modal("remove-dialog", confirm, window, card)
            }))
            .children(self.tab_menu.as_ref().map(|menu| {
                let (id, at) = menu.value.clone();
                gpui::deferred(
                    gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(crate::ui::menu_motion(
                        "tab-menu-in",
                        menu,
                        menu_card(&theme, "tab-menu", 180.)
                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                if let Some(m) = this.tab_menu.as_mut() { m.close() }
                                cx.notify();
                            }))
                            .child(row(&theme, false, "tab-menu-rename").h(px(28.)).rounded(px(6.)).child("Rename").on_click(
                                cx.listener({
                                    let id = id.clone();
                                    move |this, _: &ClickEvent, window, cx| {
                                        if let Some(m) = this.tab_menu.as_mut() { m.close() }
                                        let label = this
                                            .snapshot
                                            .as_ref()
                                            .and_then(|s| s.tabs.iter().find(|t| t.tab_id == id))
                                            .map(|t| t.label.clone())
                                            .unwrap_or_default();
                                        this.start_rename(RenameTarget::Tab(id.clone()), label, window, cx);
                                    }
                                }),
                            ))
                            .child(div().mx(px(-4.)).my(px(4.)).h(px(1.)).bg(theme.border))
                            .child(
                                row(&theme, false, "tab-menu-close")
                                    .h(px(28.))
                                    .rounded(px(6.))
                                    .text_color(theme.danger)
                                    .child("Close tab")
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        if let Some(m) = this.tab_menu.as_mut() { m.close() }
                                        this.send(Command::CloseTab { tab_id: id.clone() }, cx, |_, _, _| {});
                                    })),
                            ),
                    )),
                )
                .with_priority(1)
            }))
            .children(self.spent_menu.as_ref().map(|menu| {
                let this = cx.entity().downgrade();
                let pick = |tab: &str| {
                    let (this, tab) = (this.clone(), tab.to_string());
                    move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                        let _ = this.update(cx, |this, cx| {
                            if let Some(m) = this.spent_menu.as_mut() { m.close() }
                            this.selected_tab = Some(tab.clone());
                            cx.notify();
                        });
                    }
                };
                let shown = self.active_pane(window, cx);
                let on_sort = cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.spent_by_tokens = !this.spent_by_tokens;
                    cx.notify();
                });
                let card = crate::ui::statusbar::spent_card(snapshot.as_ref(), shown.as_deref(), &theme, self.spent_by_tokens, on_sort, pick, cx).on_mouse_down_out(
                    cx.listener(|this, _, _, cx| {
                        if let Some(m) = this.spent_menu.as_mut() { m.close() }
                        cx.notify();
                    }),
                );
                gpui::deferred(
                    gpui::anchored()
                        .anchor(gpui::Anchor::BottomLeft)
                        .position(menu.value)
                        .snap_to_window_with_margin(px(8.))
                        .child(crate::ui::menu_motion("spent-menu-in", menu, card)),
                )
                .with_priority(1)
            }))
            .children(self.usage_menu.as_ref().map(|menu| {
                let card = crate::ui::statusbar::usage_card(&theme, cx).on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    if let Some(m) = this.usage_menu.as_mut() { m.close() }
                    cx.notify();
                }));
                gpui::deferred(
                    gpui::anchored()
                        .anchor(gpui::Anchor::BottomRight)
                        .position(menu.value)
                        .snap_to_window_with_margin(px(8.))
                        .child(crate::ui::menu_motion("usage-menu-in", menu, card)),
                )
                .with_priority(1)
            }))
            .children(self.pane_menu.as_ref().map(|menu| {
                let (pane_id, at) = menu.value.clone();
                let chords = cx.try_global::<ActiveBindings>().map(|b| b.0.clone()).unwrap_or_default();
                let items: [(&str, fn(String) -> Command); 4] = [
                    ("SplitRight", |pane_id| Command::Split { pane_id, down: false }),
                    ("SplitDown", |pane_id| Command::Split { pane_id, down: true }),
                    ("ToggleZoom", |pane_id| Command::Zoom { pane_id }),
                    ("ClosePane", |pane_id| Command::ClosePane { pane_id }),
                ];
                gpui::deferred(
                    gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(crate::ui::menu_motion(
                        "pane-menu-in",
                        menu,
                        menu_card(&theme, "pane-menu", 200.)
                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                if let Some(m) = this.pane_menu.as_mut() { m.close() }
                                cx.notify();
                            }))
                            .children(items.into_iter().enumerate().flat_map(|(i, (action, make))| {
                                let label = ACTIONS.iter().find(|(a, _)| *a == action).map_or(action, |(_, l)| l);
                                let chord = chords.iter().find(|(a, _)| a == action).map(|(_, k)| crate::ui::settings::display_chord(k));
                                let close = action == "ClosePane";
                                let pane_id = pane_id.clone();
                                let separator = close.then(|| div().mx(px(-4.)).my(px(4.)).h(px(1.)).bg(theme.border).into_any_element());
                                let item = row(&theme, false, ("pane-menu-item", i))
                                    .h(px(28.))
                                    .rounded(px(6.))
                                    .when(close, |el| el.text_color(theme.danger))
                                    .child(div().flex_1().child(label))
                                    .when_some(chord, |el, chord| el.child(div().text_size(px(11.)).text_color(theme.text_faint).child(chord)))
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        if let Some(m) = this.pane_menu.as_mut() { m.close() }
                                        this.send(make(pane_id.clone()), cx, |_, _, _| {});
                                    }));
                                separator.into_iter().chain([item.into_any_element()])
                            })),
                    )),
                )
                .with_priority(1)
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::prune;
    use crate::model::{Frac, PaneFrame};
    use std::collections::HashMap;

    fn shown(ids: &[&str]) -> Vec<PaneFrame> {
        let frame = Frac { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
        ids.iter().map(|id| PaneFrame { pane_id: id.to_string(), frame, focused: false }).collect()
    }

    /// Surfaces stand in as "has its process exited?".
    fn surfaces(entries: &[(&str, bool)]) -> HashMap<String, bool> {
        entries.iter().map(|(id, exited)| (id.to_string(), *exited)).collect()
    }

    #[test]
    fn hidden_panes_lose_their_surface_and_focus() {
        let mut s = surfaces(&[("w1:p1", false), ("w1:p2", false)]);
        let mut focused = Some("w1:p2".to_string());
        prune(&mut s, &mut focused, &shown(&["w1:p1"]), |exited| *exited);
        assert_eq!(s.keys().collect::<Vec<_>>(), ["w1:p1"]);
        assert_eq!(focused, None, "a recreated surface for w1:p2 must get focus again");
    }

    #[test]
    fn exited_surfaces_are_dropped_to_be_recreated() {
        let mut s = surfaces(&[("w1:p1", true)]);
        let mut focused = Some("w1:p1".to_string());
        prune(&mut s, &mut focused, &shown(&["w1:p1"]), |exited| *exited);
        assert!(s.is_empty());
        assert_eq!(focused, None);
    }

    #[test]
    fn live_shown_surfaces_keep_focus() {
        let mut s = surfaces(&[("w1:p1", false)]);
        let mut focused = Some("w1:p1".to_string());
        prune(&mut s, &mut focused, &shown(&["w1:p1"]), |exited| *exited);
        assert_eq!(s.len(), 1);
        assert_eq!(focused.as_deref(), Some("w1:p1"));
    }
}

#[test]
fn edge_nudge_springs_out_and_home() {
    assert_eq!(edge_nudge(0.), 0.);
    assert_eq!(edge_nudge(0.32), 5.);
    assert_eq!(edge_nudge(1.), 0.);
}

#[test]
fn ease_out_matches_css() {
    assert_eq!(ease_out(0.), 0.);
    assert_eq!(ease_out(1.), 1.);
    // CSS ease-out at x = 0.5 is ≈ 0.684
    assert!((ease_out(0.5) - 0.684).abs() < 0.01);
}
