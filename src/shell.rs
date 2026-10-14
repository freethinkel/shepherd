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
use crate::settings::{SIDEBAR_MAX, SIDEBAR_MIN, SIDEBAR_WIDTH, SettingsGlobal, WindowFrame};
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

mod client_shell;
mod divider;
mod floating;
mod motion;
mod pane_drag;
mod popups;
mod sessions;
mod sidebar;
mod sync;
mod tab_bar;
mod widgets;
mod workspaces;
use motion::*;
use sync::sync_loop;
pub(crate) use widgets::{menu_card, modal, row, status_dot, status_icon};
use widgets::*;
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
    /// Since when the place shown has sent no snapshot of its own yet: the one shown, if any, is the last it sent.
    connecting: Option<Instant>,
    panes: HashMap<String, Entity<PaneTerminal>>,
    /// The panes shown last, most recent first: their views (and herdr sessions) outlive a tab switch.
    recent_panes: Vec<String>,
    own_pane: Option<String>,
    /// herdr's focused pane we last moved keyboard focus to; refocusing on every render would undo clicks.
    focused: Option<String>,
    /// herdr's popup just closed: the keyboard goes back to herdr's focused pane unless a press outside it landed
    /// on a pane, which keeps it.
    popup_closed: bool,
    /// The last key or click in Shepherd: herdr pane focus changes soon after one are Shepherd's own.
    touched: Instant,
    /// The tab Shepherd shows. Local on purpose: herdr's focus is shared with its TUI clients.
    selected_tab: Option<String>,
    /// Titlebar drag: armed by a press in the strip, consumed by the first move (Collie's `drag_strip`).
    should_move: Rc<Cell<bool>>,
    client: Client,
    rt: tokio::runtime::Handle,
    /// herdr's plugin actions as (id, title), for the command panel; read again whenever it opens.
    plugin_actions: Vec<(String, String)>,
    /// The pane grid's bounds from the last paint, to turn a divider drag into a ratio.
    grid_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    divider: Option<DividerDrag>,
    /// A released divider's ratio, shown until herdr's snapshot catches up (no jump back to the old layout).
    settling: Option<(Vec<bool>, f32)>,
    /// The tab or workspace label being edited in place.
    renaming: Option<Rename>,
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
    sidebar_open: bool,
    /// The sidebar's width tween while it shows or hides.
    sidebar_motion: Option<FoldMotion>,
    /// The sidebar's edge is being dragged.
    sidebar_drag: bool,
    /// The drag hit a width limit (-1 min, +1 max) at this instant: the edge nudges past it and springs back.
    sidebar_edge: Option<(f32, Instant)>,
    agent_sort: AgentSort,
    /// Saved in the settings when a drag ends.
    agents_height: Option<f32>,
    /// The Agents section's top edge is being dragged: the pointer's y and the list's height at the press.
    agents_drag: Option<(f32, f32)>,
    /// The sidebar's `…` menu, open where it was clicked.
    view_menu: Option<crate::ui::Popup<()>>,
    /// The done agent's pane herdr was last told you looked at, so it is told once, not every frame.
    marked_seen: Option<String>,
    /// An agent's pane to give the keyboard once it is shown, over herdr's focused pane of its tab.
    focus_next: Option<String>,
    /// The `…` menu's Show submenu is out: the pointer went over its row.
    show_submenu: bool,
    /// Where the Show submenu was painted: a press there is inside the `…` menu, though outside its card.
    show_submenu_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// The Agents section's `…` menu, open under its button.
    agents_menu: Option<crate::ui::Popup<gpui::Point<Pixels>>>,
    /// Worktrees herdr is removing: their rows show a spinner until they leave the snapshot.
    removing: std::collections::HashSet<String>,
    /// Remove a worktree? The workspace, its label, and whether herdr found changes (then removal is forced).
    remove_confirm: Option<crate::ui::Popup<(String, String, bool)>>,
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
    /// Holds keyboard focus while a delete or remove confirm is up, for its Enter and Escape.
    confirm_focus: gpui::FocusHandle,
    /// The Settings page, shown in place of the sidebar and the panes while open.
    settings: Option<(Entity<crate::ui::settings::SettingsView>, Subscription)>,
    /// A pane being dragged by its grip.
    pane_drop: Option<pane_drag::PaneDrop>,
    pane_spring_back: Option<pane_drag::SpringBack>,
    /// The command panel (cmd-k), open over the window.
    command_panel: Option<crate::ui::Popup<(Entity<crate::ui::command_panel::Panel>, Subscription)>>,
    /// Agents that started waiting or finished out of sight and have not been looked at since: the Dock badge.
    unseen: std::collections::HashSet<String>,
    /// The count on the Dock icon now.
    badge: usize,
    /// The window's frame not saved yet, and the save it waits for: half a second after the last move, closing or quitting.
    frame: Option<WindowFrame>,
    frame_save: gpui::Task<()>,
    /// Since when each agent has had its status: how long the waiting ones have waited.
    since: crate::model::Since,
    /// The loop that keeps the window in step with herdr; replacing it (a switch of session) stops the last one.
    sync: gpui::Task<()>,
    /// herdr's popup over the panes, and the client shell that hears of it.
    floating: Option<floating::Floating>,
    popup_watch: gpui::Task<()>,
    /// The client shell's surface and the grid it was last sized to.
    popup_surface: Option<(crate::herdr::popup::Surface, ((u16, u16), (u32, u32)))>,
    /// PoC: the window's herdr client shell for theme, popup and commands (`SHEPHERD_CLIENT_SHELL=1`).
    client_shell: Option<client_shell::ShellView>,
    /// herdr's `[[keys.command]]` ids for `command.invoke`, with their keys, from the client shell.
    herdr_commands: Vec<(String, Vec<String>)>,
    /// Panes `[[keys.command]]` of `type = "pane"` opened, by the command's keys: pressed again in their tab, the
    /// command brings its pane back instead of stacking another (herdr opens one per press).
    command_panes: Vec<(Vec<String>, String)>,
    /// herdr's sessions and machines, in the bar under the sidebar.
    places: Vec<sessions::Place>,
    /// A two-finger swipe over the sidebar in progress.
    swipe: Option<sessions::Swipe>,
    /// The workspace list springing home from this offset since then, after a swipe, beside the place it left.
    slide: Option<(f32, Instant, Option<sessions::Place>)>,
    /// The bar's row of places, when it overflows.
    places_scroll: gpui::ScrollHandle,
    /// Where the bar's lit square was last kept in view: it is scrolled to again only when it moves.
    lit_seen: Cell<f32>,
    /// The bar's row gliding from one scroll offset to another since then, to keep the lit square in its middle.
    bar_glide: Cell<Option<(f32, f32, Instant)>>,
    /// Where the bar was painted: a swipe there scrolls the bar, not to the next place.
    bar_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// A place's right-click menu in the bar, and where it was clicked.
    place_menu: Option<crate::ui::Popup<(sessions::Place, gpui::Point<Pixels>)>>,
    /// The Edit dialog of a place's name and icon.
    place_edit: Option<crate::ui::Popup<sessions::PlaceEdit>>,
    /// The places not shown, by key, with their last snapshot.
    snapshots: HashMap<String, Snapshot>,
    /// The New session dialog, open over the window.
    session_dialog: Option<crate::ui::Popup<sessions::SessionDialog>>,
    /// Delete a session, or forget a machine? Asked first.
    session_delete: Option<crate::ui::Popup<sessions::Place>>,
    /// The saved machine shown, by its label; `None` for a session on this Mac.
    machine_label: Option<String>,
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
        let (display_id, window_bounds) = saved_window(cx).map_or_else(
            || (None, gpui::WindowBounds::Windowed(Bounds::centered(None, size(px(1180.), px(760.)), cx))),
            |(display, bounds)| (Some(display), bounds),
        );
        let options = WindowOptions {
            window_bounds: Some(window_bounds),
            display_id,
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
                cx.observe_window_activation(window, |s: &mut Shell, window: &mut Window, cx: &mut Context<Shell>| {
                    reapply_traffic_lights(window);
                    // in the background Shepherd leaves panes' size to herdr's other clients; back, it takes it again
                    let active = window.is_window_active();
                    if active {
                        s.touched = Instant::now();
                    }
                    cx.set_global(crate::terminal::pane::Yielded(!active));
                    if active {
                        s.refresh_places(cx); // sessions made or deleted elsewhere
                        s.push_host_theme(cx);
                    }
                    cx.notify(); // the shown tab's agents count as seen once the window is active
                })
                .detach();
                cx.observe_global::<Theme>(|s: &mut Shell, cx| s.push_host_theme(cx)).detach();
                cx.observe_global::<crate::terminal::pane::Yielded>(|s: &mut Shell, cx| {
                    for pane in s.panes.values() {
                        pane.update(cx, |p, cx| p.follow_yield(cx));
                    }
                })
                .detach();
                // A click on a notification brings its agent's tab up; Approve answers the agent and leaves Shepherd
                // where it is.
                let this = cx.entity().downgrade();
                cx.on_system_notification_response(move |response, cx| {
                    let approve = response.action_id.as_deref() == Some(crate::notify::APPROVE);
                    let _ = this.update(cx, |s, cx| {
                        let pane = s.snapshot.as_ref().and_then(|snap| snap.panes.iter().find(|p| p.pane_id == crate::notify::tag_pane(&response.tag)));
                        match pane.map(|p| (p.tab_id.clone(), p.pane_id.clone(), p.agent_status)) {
                            // only while it still waits: a late press must not type Enter into an agent at work
                            Some((_, pane_id, AgentStatus::Blocked)) if approve => s.send(Command::Approve { pane_id }, cx, |_, _, _| {}),
                            Some((tab, pane, _)) if !approve => s.show_agent(tab, pane, cx),
                            _ => {}
                        }
                    });
                    if !approve {
                        cx.activate(true);
                    }
                });
                cx.observe_window_bounds(window, |s: &mut Shell, window, cx| {
                    let last = s.frame.clone().or_else(|| SettingsGlobal::of(cx).settings.window.clone());
                    s.frame = window_frame(window, last.as_ref(), cx);
                    s.frame_save = cx.spawn(async move |this, cx| {
                        cx.background_executor().timer(Duration::from_millis(500)).await;
                        let _ = this.update(cx, |s, cx| s.save_frame(cx));
                    });
                })
                .detach();
                cx.on_app_quit(|s: &mut Shell, cx| {
                    s.save_frame(cx);
                    async {}
                })
                .detach();
                let this = cx.entity().downgrade();
                window.on_window_should_close(cx, move |_, cx| {
                    let _ = this.update(cx, |s, cx| s.save_frame(cx));
                    true
                });
                let mut shell = Shell {
                    snapshot: None,
                    error: None,
                    connecting: Some(Instant::now()),
                    panes: HashMap::new(),
                    recent_panes: Vec::new(),
                    own_pane: std::env::var("HERDR_PANE_ID").ok(),
                    focused: None,
                    popup_closed: false,
                    touched: Instant::now(),
                    selected_tab: std::env::var("SHEPHERD_TAB").ok(),
                    should_move: Rc::new(Cell::new(false)),
                    client: SettingsGlobal::of(cx)
                        .settings
                        .herdr_socket
                        .clone()
                        .filter(|p| !p.trim().is_empty())
                        .map_or_else(Client::local, |p| Client::at(p.into())),
                    rt: rt.clone(),
                    plugin_actions: Vec::new(),
                    grid_bounds: Rc::new(Cell::new(None)),
                    divider: None,
                    settling: None,
                    renaming: None,
                    folded: SettingsGlobal::of(cx).settings.folded.iter().cloned().collect(),
                    fold_motion: Default::default(),
                    seam_hover: Default::default(),
                    ws_menu: None,
                    shown_workspace: None,
                    sidebar_width: SettingsGlobal::of(cx).settings.sidebar_width,
                    sidebar_open: SettingsGlobal::of(cx).settings.sidebar_open,
                    sidebar_motion: None,
                    sidebar_drag: false,
                    sidebar_edge: None,
                    agent_sort: SettingsGlobal::of(cx).settings.agent_sort,
                    agents_height: SettingsGlobal::of(cx).settings.agents_height,
                    agents_drag: None,
                    view_menu: None,
                    focus_next: None,
                    marked_seen: None,
                    show_submenu: false,
                    show_submenu_bounds: Default::default(),
                    agents_menu: None,
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
                    confirm_focus: cx.focus_handle(),
                    settings: None,
                    pane_drop: None,
                    pane_spring_back: None,
                    command_panel: None,
                    unseen: Default::default(),
                    badge: 0,
                    frame: None,
                    frame_save: gpui::Task::ready(()),
                    since: Default::default(),
                    sync: gpui::Task::ready(()),
                    floating: None,
                    popup_watch: gpui::Task::ready(()),
                    popup_surface: None,
                    client_shell: None,
                    herdr_commands: Vec::new(),
                    command_panes: Vec::new(),
                    places: Vec::new(),
                    swipe: None,
                    slide: None,
                    places_scroll: gpui::ScrollHandle::new(),
                    bar_bounds: Default::default(),
                    lit_seen: Cell::new(-1.),
                    bar_glide: Cell::new(None),
                    place_menu: None,
                    place_edit: None,
                    snapshots: HashMap::new(),
                    session_dialog: None,
                    session_delete: None,
                    machine_label: None,
                };
                shell.resume_machine(cx);
                shell.connect(cx);
                shell.refresh_places(cx);
                shell.refresh_plugin_actions(cx);
                // Waiting times move on between snapshots.
                // ponytail: a minute from launch, not from each wait, so a label can lag by up to a minute.
                cx.spawn(async move |this, cx| loop {
                    cx.background_executor().timer(Duration::from_secs(60)).await;
                    let alive = this.update(cx, |s, cx| {
                        if s.since.values().any(|(status, _)| crate::notify::wants_attention(*status)) {
                            cx.notify();
                        }
                    });
                    if alive.is_err() {
                        return; // window closed
                    }
                })
                .detach();
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

    /// Follows herdr over `self.client` from scratch: its snapshot and the subscription that refreshes it.
    fn connect(&mut self, cx: &mut Context<Self>) {
        let (client, rt) = (self.client.clone(), self.rt.clone());
        cx.set_global(PaneQueries(client.clone(), rt.clone()));
        self.sync = cx.spawn(async move |this, cx| sync_loop(this, rt, client, cx).await);
        self.floating = None;
        self.popup_watch = if crate::herdr::client_shell::enabled() {
            cx.spawn(async move |this, cx| client_shell::shell_loop(this, cx).await)
        } else {
            cx.spawn(async move |this, cx| floating::popup_loop(this, cx).await)
        };
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
                // ponytail: this open lists the actions read last time; one installed since shows next time
                self.refresh_plugin_actions(cx);
                crate::ui::command_panel::items(self.snapshot.as_ref(), &bindings, &self.plugin_actions)
            }
            Kind::Agents => crate::ui::command_panel::agent_items(self.snapshot.as_ref(), self.agent_sort, &self.since),
            Kind::Branches => match dir.as_deref() {
                Some(d) => crate::ui::command_panel::branch_items(&crate::ui::statusbar::read_branches(d), crate::ui::statusbar::branch_of(d, cx).as_deref()),
                None => Vec::new(),
            },
            Kind::Worktrees => Vec::new(), // herdr's list fills it: see `open_worktrees_panel`
            Kind::Themes => {
                let appearance = Theme::of(cx).appearance;
                let dark = appearance == crate::theme::Appearance::Dark;
                let s = &SettingsGlobal::of(cx).settings;
                let current = if dark { &s.theme_dark } else { &s.theme_light };
                crate::ui::command_panel::theme_items(crate::theme::variants_for(appearance), dark, current)
            }
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
                PanelEvent::Run(Run::Herdr(command)) => {
                    window.focus(&this.focus, cx);
                    this.run_herdr_command(command, window, cx);
                }
                PanelEvent::Run(Run::Tab(tab_id)) => this.selected_tab = Some(tab_id.clone()),
                PanelEvent::Run(Run::Branch(branch)) => {
                    let (Some(dir), branch) = (dir.clone(), branch.clone()) else { return };
                    cx.spawn(async move |this, cx| {
                        let d = dir.clone();
                        let done = cx.background_spawn(async move { crate::ui::statusbar::switch_branch(&d, &branch) }).await;
                        let _ = this.update(cx, |_, cx| match done {
                            Ok(()) => crate::ui::statusbar::refresh_git(&dir, cx),
                            Err(e) => crate::ui::toast::error(cx, e),
                        });
                    })
                    .detach();
                }
                PanelEvent::Run(Run::Theme(id)) => {
                    let dark = Theme::of(cx).appearance == crate::theme::Appearance::Dark;
                    let id = id.clone();
                    crate::ui::settings::update(cx, |s| *if dark { &mut s.theme_dark } else { &mut s.theme_light } = id);
                }
                PanelEvent::Run(Run::Worktree { repo, path, workspace_id }) => {
                    let open_tab = workspace_id.as_ref().and_then(|id| {
                        let s = this.snapshot.as_ref()?;
                        let w = s.workspaces.iter().find(|w| &w.workspace_id == id)?;
                        w.active_tab_id.clone().or_else(|| s.tabs_of(id).first().map(|t| t.tab_id.clone()))
                    });
                    match open_tab {
                        Some(tab) => this.selected_tab = Some(tab),
                        None => {
                            let command = Command::OpenWorktree { workspace_id: repo.clone(), path: path.clone() };
                            this.send_then(command, cx, |shell, result, cx| {
                                match result {
                                    Ok(value) => shell.selected_tab = crate::commands::created_tab_id(&value),
                                    Err(err) => crate::ui::toast::error(cx, format!("{err:#}")),
                                }
                                cx.notify();
                            });
                        }
                    }
                }
                PanelEvent::Dismiss => {}
            }
        });
        self.command_panel = Some(crate::ui::Popup::new((panel, events)));
        cx.notify();
    }

    /// A key or click in Shepherd: it is the client in use, so it takes its panes' size back.
    fn touch(&mut self, cx: &mut Context<Self>) {
        self.touched = Instant::now();
        crate::terminal::pane::claim(cx);
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
            Err(err) => crate::ui::toast::error(cx, format!("{err:#}")),
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
                crate::notify::show(cx, &pane.pane_id, title, body, sound, pane.agent_status == AgentStatus::Blocked);
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

    /// herdr keeps an agent done until its pane has herdr's focus; the pane with the keyboard in the active window
    /// counts as looked at, so herdr is told and the agent turns idle.
    fn mark_seen(&mut self, window: &Window, cx: &mut Context<Self>) {
        let focused = window.is_window_active().then(|| {
            self.panes.iter().find(|(_, pane)| pane.read(cx).focus_handle(cx).is_focused(window)).map(|(id, _)| id.clone())
        });
        let done = focused.flatten().filter(|id| {
            self.snapshot.as_ref().is_some_and(|s| s.panes.iter().any(|p| &p.pane_id == id && p.agent_status == AgentStatus::Done))
        });
        if done == self.marked_seen {
            return;
        }
        self.marked_seen = done.clone();
        if let Some(pane_id) = done {
            self.send_then(Command::MarkSeen { pane_id }, cx, |_, _, _| {});
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

    /// herdr paints the terminals' default colors itself: it is told the theme's over the popup watch, which then
    /// is the client in front again (back from herdr's TUI on another machine, say). The client shell (PoC) does
    /// this in `sync_client_shell`.
    fn push_host_theme(&self, cx: &App) {
        if let Some((surface, _)) = &self.popup_surface {
            surface.theme(client_shell::host_theme(Theme::of(cx)));
        }
    }

    fn refresh_plugin_actions(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let call = self.rt.spawn(async move { client.call("plugin.action.list", json!({})).await });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(result)) = call.await else { return };
            let actions = result["actions"].as_array().into_iter().flatten().filter_map(|a| {
                Some((format!("{}.{}", a["plugin_id"].as_str()?, a["action_id"].as_str()?), a["title"].as_str()?.to_string()))
            });
            let actions = actions.collect();
            let _ = this.update(cx, |s, _| s.plugin_actions = actions);
        })
        .detach();
    }

    /// A `[[keys.command]]` of herdr's config, for the pane you are in.
    fn run_herdr_command(&mut self, command: &crate::herdr::keys::HerdrCommand, window: &Window, cx: &mut Context<Self>) {
        let pane_id = self.active_pane(window, cx);
        let Some(focused) = self.focus_shown_tab(cx) else { return self.run_herdr_command_in(command, pane_id, cx) };
        let command = command.clone();
        cx.spawn(async move |this, cx| {
            focused.await;
            let _ = this.update(cx, |s, cx| s.run_herdr_command_in(&command, pane_id, cx));
        })
        .detach();
    }

    fn run_herdr_command_in(&mut self, command: &crate::herdr::keys::HerdrCommand, pane_id: Option<String>, cx: &mut Context<Self>) {
        use crate::herdr::keys::CommandKind;
        let pane = self.snapshot.as_ref().and_then(|s| s.panes.iter().find(|p| Some(&p.pane_id) == pane_id.as_ref())).cloned();
        match command.kind {
            CommandKind::PluginAction => {
                self.send(Command::InvokePluginAction { action_id: command.command.clone(), pane }, cx, |_, _, _| {})
            }
            // herdr's `shell`: detached, in the pane's directory, with herdr's socket for the `herdr` CLI
            CommandKind::Shell => {
                let mut sh = std::process::Command::new("sh");
                sh.args(["-c", &command.command])
                    .env("HERDR_SOCKET_PATH", self.client.path())
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
                if let Some(p) = &pane {
                    sh.env("HERDR_ACTIVE_PANE_ID", &p.pane_id)
                        .env("HERDR_ACTIVE_TAB_ID", &p.tab_id)
                        .env("HERDR_ACTIVE_WORKSPACE_ID", &p.workspace_id);
                }
                if let Some(dir) = crate::ui::statusbar::pane_dir(self.snapshot.as_ref(), pane_id.as_deref()) {
                    sh.env("HERDR_ACTIVE_PANE_CWD", &dir).current_dir(dir);
                }
                match sh.spawn() {
                    Ok(mut child) => drop(std::thread::spawn(move || child.wait())),
                    Err(e) => crate::ui::toast::error(cx, format!("{}: {e}", command.command)),
                }
            }
            // herdr runs these itself: its pane lands in the tab, its popup comes back over the client shell
            CommandKind::Pane | CommandKind::Popup => {
                let panes = self.snapshot.as_ref().map_or(&[][..], |s| &s.panes[..]);
                self.command_panes.retain(|(_, id)| panes.iter().any(|p| &p.pane_id == id));
                let open = self.command_panes.iter().find(|(keys, id)| {
                    *keys == command.keys && pane.as_ref().is_some_and(|shown| panes.iter().any(|p| &p.pane_id == id && p.tab_id == shown.tab_id))
                });
                let open = open.map(|(_, id)| id.clone());
                if let Some(id) = open.filter(|_| command.kind == CommandKind::Pane) {
                    self.focus_next = Some(id.clone());
                    let client = self.client.clone();
                    self.rt.spawn(async move { client.call("pane.focus", json!({"pane_id": id})).await });
                    cx.notify();
                    return;
                }
                let Some((id, _)) = self.herdr_commands.iter().find(|(_, keys)| *keys == command.keys) else {
                    crate::ui::toast::error(cx, format!("{}: herdr has no such command (is its config reloaded?)", command.command));
                    return;
                };
                let mut params = json!({"command_id": id});
                if let Some(p) = &pane {
                    params["workspace_id"] = json!(p.workspace_id);
                    params["tab_id"] = json!(p.tab_id);
                    params["pane_id"] = json!(p.pane_id);
                }
                let (client, label, keys) = (self.client.clone(), command.command.clone(), command.keys.clone());
                let before: Vec<String> = self.snapshot.iter().flat_map(|s| s.panes.iter().map(|p| p.pane_id.clone())).collect();
                let tab = pane.as_ref().map(|p| p.tab_id.clone());
                // herdr opens the pane before it answers: the one in the tab that was not there is it
                let call = self.rt.spawn(async move {
                    client.call("command.invoke", params).await?;
                    client.call("pane.list", json!({})).await
                });
                cx.spawn(async move |this, cx| match call.await {
                    Ok(Ok(list)) => {
                        let new = list["panes"].as_array().into_iter().flatten().find(|p| {
                            p["tab_id"].as_str() == tab.as_deref() && p["pane_id"].as_str().is_some_and(|id| !before.iter().any(|b| b == id))
                        });
                        if let Some(id) = new.and_then(|p| p["pane_id"].as_str()) {
                            let _ = this.update(cx, |s, _| s.command_panes.push((keys, id.to_string())));
                        }
                    }
                    Ok(Err(e)) => {
                        let _ = cx.update(|cx| crate::ui::toast::error(cx, format!("{label}: {e:#}")));
                    }
                    Err(_) => {}
                })
                .detach();
            }
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

    /// While a divider, the sidebar or the agents list is being dragged, the whole window shows the resize cursor:
    /// the pointer outruns the narrow hit strip, whose own cursor would otherwise flicker with the terminal's.
    fn resize_cursor(&self) -> Option<impl IntoElement> {
        let vertical = if let Some(d) = &self.divider {
            d.vertical
        } else if self.sidebar_drag {
            true
        } else if self.agents_drag.is_some() {
            false
        } else {
            return None;
        };
        let style = if vertical { gpui::CursorStyle::ResizeColumn } else { gpui::CursorStyle::ResizeRow };
        Some(canvas(|_, _, _| (), move |_, _, window, _| window.set_window_cursor_style(style)).absolute().size_0())
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

/// Keep views of the `kept` panes; focus is forgotten with its view so a recreated one gets keyboard focus again.
fn prune<S>(surfaces: &mut HashMap<String, S>, focused: &mut Option<String>, kept: &[String], exited: impl Fn(&S) -> bool) {
    surfaces.retain(|id, s| kept.contains(id) && !exited(s));
    if focused.as_ref().is_some_and(|id| !surfaces.contains_key(id)) {
        *focused = None;
    }
}

/// How many panes keep their views once their tab is hidden: each holds a herdr session that keeps streaming.
const KEPT_PANES: usize = 24;

/// Puts the `shown` panes first in `recent`, drops panes herdr no longer has, and keeps the newest `KEPT_PANES`:
/// switching back to a recent tab shows its panes at once instead of attaching to herdr again.
fn remember_shown(recent: &mut Vec<String>, shown: &[PaneFrame], live: impl Fn(&str) -> bool) {
    for f in shown.iter().rev() {
        recent.retain(|id| id != &f.pane_id);
        recent.insert(0, f.pane_id.clone());
    }
    recent.retain(|id| live(id));
    recent.truncate(KEPT_PANES);
}

/// AppKit lays the traffic lights out again while the window is inactive (a screen recorder takes focus), and gpui
/// moves them back only when it becomes key; setting the title (hidden) re-applies `traffic_light_position`.
fn reapply_traffic_lights(window: &mut Window) {
    window.set_window_title("Shepherd");
}

/// Turns macOS Secure Keyboard Entry on or off. The system counts enables, so it is called only on a change.
fn secure_input(on: bool) {
    use std::sync::atomic::{AtomicBool, Ordering};
    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn EnableSecureEventInput() -> i32;
        fn DisableSecureEventInput() -> i32;
    }
    static ON: AtomicBool = AtomicBool::new(false);
    if ON.swap(on, Ordering::Relaxed) != on {
        // SAFETY: no arguments; balanced by `ON`
        unsafe {
            if on { EnableSecureEventInput() } else { DisableSecureEventInput() };
        }
    }
}

/// The window's last frame and its display, while that display is plugged in and the frame on it.
fn saved_window(cx: &App) -> Option<(gpui::DisplayId, gpui::WindowBounds)> {
    let frame = SettingsGlobal::of(cx).settings.window.clone()?;
    let display = cx.displays().into_iter().find(|d| d.uuid().is_ok_and(|u| u.to_string() == frame.display))?;
    let bounds = frame.bounds_on(display.bounds().size)?;
    let bounds = match (frame.fullscreen, frame.maximized) {
        (true, _) => gpui::WindowBounds::Fullscreen(bounds),
        (_, true) => gpui::WindowBounds::Maximized(bounds),
        _ => gpui::WindowBounds::Windowed(bounds),
    };
    Some((display.id(), bounds))
}

/// The window's frame to save: in full screen, the one it leaves full screen to; zoomed, the one before (`last`).
fn window_frame(window: &Window, last: Option<&WindowFrame>, cx: &App) -> Option<WindowFrame> {
    let display = window.display(cx)?.uuid().ok()?.to_string();
    let (bounds, fullscreen) = match window.window_bounds() {
        gpui::WindowBounds::Fullscreen(b) => (b, true),
        b => (b.get_bounds(), false),
    };
    let (x, y, width, height) = (bounds.origin.x.into(), bounds.origin.y.into(), bounds.size.width.into(), bounds.size.height.into());
    let frame = WindowFrame { display, x, y, width, height, fullscreen, maximized: false };
    Some(if !fullscreen && window.is_maximized() { frame.zoomed(last) } else { frame })
}

/// Saves a choice made in the sidebar, or where the window was moved. A settings file that failed to load is the user's to fix: the sidebar must
/// not overwrite it with defaults.
fn save_from_sidebar(cx: &mut App, change: impl FnOnce(&mut crate::settings::Settings)) {
    if SettingsGlobal::of(cx).error.is_none() {
        crate::ui::settings::update(cx, change);
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_badge(window, cx);
        self.mark_seen(window, cx);
        // any Shepherd window in front, not this one: a window in the back renders too
        secure_input(SettingsGlobal::of(cx).settings.secure_input && cx.active_window().is_some());
        crate::ui::reap_popup(&mut self.view_menu, window);
        crate::ui::reap_popup(&mut self.agents_menu, window);
        crate::ui::reap_popup(&mut self.ws_menu, window);
        crate::ui::reap_popup(&mut self.pane_menu, window);
        crate::ui::reap_popup(&mut self.spent_menu, window);
        crate::ui::reap_popup(&mut self.usage_menu, window);
        crate::ui::reap_popup(&mut self.tab_menu, window);
        crate::ui::reap_popup(&mut self.worktree_dialog, window);
        crate::ui::reap_popup(&mut self.remove_confirm, window);
        crate::ui::reap_popup(&mut self.session_dialog, window);
        crate::ui::reap_popup(&mut self.place_menu, window);
        crate::ui::reap_popup(&mut self.place_edit, window);
        crate::ui::reap_popup(&mut self.session_delete, window);
        crate::ui::reap_popup(&mut self.command_panel, window);
        if let Some(s) = &self.snapshot {
            self.removing.retain(|id| s.workspaces.iter().any(|w| &w.workspace_id == id));
        }
        reapply_traffic_lights(window);
        let theme = Theme::of(cx).clone();
        let snapshot = self.snapshot.clone();
        let toasts = crate::ui::toast::stack(&theme, TITLEBAR_HEIGHT + 8., window, cx);
        let tab = self.shown_tab();
        self.shown_workspace = tab.as_ref().map(|t| t.workspace_id.clone());
        self.sync_client_shell(window, cx);
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
        if let Some(snapshot) = &snapshot {
            remember_shown(&mut self.recent_panes, &shown, |id| snapshot.panes.iter().any(|p| p.pane_id == id));
        }
        prune(&mut self.panes, &mut self.focused, &self.recent_panes, |_| false);
        if std::mem::take(&mut self.popup_closed) && !self.panes.values().any(|p| p.read(cx).focus_handle(cx).is_focused(window)) {
            self.focused = None;
        }
        let overlaid = self.settings.is_some() || self.command_panel.as_ref().is_some_and(|p| p.is_open()) || self.floating_open();
        for f in &shown {
            if !self.panes.contains_key(&f.pane_id) {
                let id = f.pane_id.clone();
                self.panes.insert(id.clone(), cx.new(|cx| PaneTerminal::new(id, cx)));
            }
            // an open panel keeps its input's focus; a panel run from the command panel opens before this render
            if f.focused && !overlaid && self.focused.as_deref() != Some(f.pane_id.as_str()) {
                let handle = self.panes[&f.pane_id].read(cx).focus_handle(cx);
                window.focus(&handle, cx);
                self.focused = Some(f.pane_id.clone());
            }
        }
        // after herdr's focus, so the agent's pane wins; its tab is already the shown one
        if let Some(id) = self.focus_next.take().filter(|id| self.settings.is_none() && shown.iter().any(|f| &f.pane_id == id)) {
            let handle = self.panes[&id].read(cx).focus_handle(cx);
            window.focus(&handle, cx);
        }

        // A pane of a tab no longer shown keeps focus, and gpui then dispatches keys to the root, past our actions.
        let hidden = self
            .panes
            .iter()
            .any(|(id, p)| !shown.iter().any(|f| &f.pane_id == id) && p.read(cx).focus_handle(cx).is_focused(window));
        if (window.focused(cx).is_none() || hidden) && self.settings.is_none() {
            window.focus(&self.focus, cx);
            self.focused = None;
        }
        let shown_ws = tab.as_ref().map(|t| t.workspace_id.clone());
        let shown_tab = tab.as_ref().map(|t| t.tab_id.clone());
        let sidebar_w = self.sidebar_now();
        let sliding = self.sidebar_motion.is_some_and(|m| m.animating());
        if sliding || self.sidebar_nudge().is_some() {
            window.request_animation_frame();
        }

        let sidebar = self.render_sidebar(&theme, &snapshot, &shown_ws, &shown_tab, window, cx);

        let tabbar = self.render_tab_bar(&theme, shown_ws.clone(), shown_tab.clone(), sidebar_w, window, cx);

        let shown_tab_id = shown_tab.clone().unwrap_or_default();
        let grid_bounds = self.grid_bounds.clone();
        let ring = theme.accent.opacity(0.5);
        let split = frames.len() > 1;
        let pane_drop = self.pane_drop_overlay(&frames, &shown_tab_id, &theme, window, cx);
        let edges: Vec<f32> = splits.iter().map(|s| crate::model::split_edge(&frames, s)).collect();
        let floating = self.render_floating(&theme, window, cx);
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
            .when(snapshot.is_none() && self.error.is_none() && self.connecting.is_some_and(|at| crate::ui::loader::due(at, window)), |el| {
                el.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap(px(12.))
                        .child(crate::ui::loader::spinner(2.5, window))
                        .child(div().text_size(px(12.)).text_color(theme.text_muted).child(format!("Connecting to {}…", self.place_label(cx)))),
                )
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
            .children(floating);

        let statusbar = SettingsGlobal::of(cx).settings.show_status_bar.then(|| {
            let pane = self.active_pane(window, cx);
            let on_branch = cx.listener(|this, _: &ClickEvent, window, cx| {
                this.toggle_command_panel(crate::ui::command_panel::Kind::Branches, window, cx)
            });
            // Popups sit on the label's edge, just above the bar.
            let on_spent = cx.listener(|this, label: &Bounds<Pixels>, window, cx| {
                let at = gpui::point(label.left(), window.viewport_size().height - px(crate::ui::statusbar::HEIGHT + 4.));
                this.spent_menu = Some(crate::ui::Popup::new(at));
                cx.notify();
            });
            let on_usage = cx.listener(|this, label: &Bounds<Pixels>, window, cx| {
                // anchored at its right edge: the limits sit at the bar's right end
                let at = gpui::point(label.right(), window.viewport_size().height - px(crate::ui::statusbar::HEIGHT + 4.));
                this.usage_menu = Some(crate::ui::Popup::new(at));
                cx.notify();
            });
            use crate::ui::statusbar::Item;
            let branches = self.command_panel.as_ref().is_some_and(|p| p.is_open() && p.value.0.read(cx).kind == crate::ui::command_panel::Kind::Branches);
            let open = if branches {
                Some(Item::Branch)
            } else if self.spent_menu.as_ref().is_some_and(|m| m.is_open()) {
                Some(Item::Spent)
            } else {
                self.usage_menu.as_ref().is_some_and(|m| m.is_open()).then_some(Item::Usage)
            };
            crate::ui::statusbar::bar(snapshot.as_ref(), pane.as_deref(), &theme, open, on_branch, on_spent, on_usage, cx)
        });
        let spring_back = self.pane_spring_back(window);
        let settings_page = self.settings.as_ref().map(|(view, _)| view.clone());
        let on_page = settings_page.is_some();
        let root = div()
            .id("root")
            .track_focus(&self.focus)
            .capture_key_down(cx.listener(|s, _: &gpui::KeyDownEvent, _, cx| s.touch(cx)))
            .capture_any_mouse_down(cx.listener(|s, _: &MouseDownEvent, _, cx| s.touch(cx)))
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
            .on_action(cx.listener(|this, _: &ChangeTheme, w, cx| this.toggle_command_panel(crate::ui::command_panel::Kind::Themes, w, cx)))
            .on_action(cx.listener(|this, _: &AgentsPanel, w, cx| this.toggle_command_panel(crate::ui::command_panel::Kind::Agents, w, cx)))
            .on_action(cx.listener(|this, c: &crate::herdr::keys::HerdrCommand, w, cx| this.run_herdr_command(c, w, cx)))
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
            .children(self.resize_cursor())
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
                            theme.accent,
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
                        .tooltip(tip("Toggle sidebar", Some("ToggleSidebar")))
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
                        .tooltip(tip("Sidebar view", None))
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
                                _ => {
                                    this.view_menu = Some(crate::ui::Popup::new(()));
                                    this.show_submenu = false;
                                }
                            }
                            cx.notify();
                        })),
                )
            });
        self.popups(root, &theme, &snapshot, sidebar_w, window, cx)
            .children(spring_back)
            .child(toasts)
    }
}

#[cfg(test)]
mod tests {
    use super::{KEPT_PANES, prune, remember_shown};
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
    fn recent_tabs_keep_their_panes() {
        let mut recent = Vec::new();
        let all = |_: &str| true;
        remember_shown(&mut recent, &shown(&["w1:p1", "w1:p2"]), all);
        remember_shown(&mut recent, &shown(&["w2:p1"]), all); // another tab
        assert_eq!(recent, ["w2:p1", "w1:p1", "w1:p2"], "the hidden tab's panes stay");
        remember_shown(&mut recent, &shown(&["w1:p1", "w1:p2"]), |id| id != "w2:p1"); // back, w2:p1 closed
        assert_eq!(recent, ["w1:p1", "w1:p2"]);
        for i in 0..KEPT_PANES {
            remember_shown(&mut recent, &shown(&[&format!("w9:p{i}")]), all);
        }
        assert_eq!(recent.len(), KEPT_PANES);
        assert!(!recent.contains(&"w1:p1".to_string()), "the oldest go first");
    }

    #[test]
    fn hidden_panes_lose_their_surface_and_focus() {
        let mut s = surfaces(&[("w1:p1", false), ("w1:p2", false)]);
        let mut focused = Some("w1:p2".to_string());
        prune(&mut s, &mut focused, &["w1:p1".to_string()], |exited| *exited);
        assert_eq!(s.keys().collect::<Vec<_>>(), ["w1:p1"]);
        assert_eq!(focused, None, "a recreated surface for w1:p2 must get focus again");
    }

    #[test]
    fn exited_surfaces_are_dropped_to_be_recreated() {
        let mut s = surfaces(&[("w1:p1", true)]);
        let mut focused = Some("w1:p1".to_string());
        prune(&mut s, &mut focused, &["w1:p1".to_string()], |exited| *exited);
        assert!(s.is_empty());
        assert_eq!(focused, None);
    }

    #[test]
    fn live_shown_surfaces_keep_focus() {
        let mut s = surfaces(&[("w1:p1", false)]);
        let mut focused = Some("w1:p1".to_string());
        prune(&mut s, &mut focused, &["w1:p1".to_string()], |exited| *exited);
        assert_eq!(s.len(), 1);
        assert_eq!(focused.as_deref(), Some("w1:p1"));
    }
}
