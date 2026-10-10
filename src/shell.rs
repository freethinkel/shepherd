//! The window: workspaces sidebar, the tab bar, and a GPUI terminal per pane of the shown tab, refreshed on
//! every herdr event. Look and titlebar handling follow Collie (Zeron style).
use crate::commands::Command;
use crate::herdr::{Client, EVENT_TYPES};
use crate::ui::divider::{Throttle, drag_ratio};
use gpui_base::input::{Input, InputEditorStyle, InputEvent, InputState};
use crate::model::{AgentStatus, PaneFrame, Snapshot, SplitFrame, attachable};
use crate::terminal::PaneTerminal;
use crate::theme::Theme;
use gpui::{
    Bounds, ClickEvent, Context, Div, Entity, Focusable, Hsla, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels,
    ParentElement, Render, SharedString, Stateful, Styled, TitlebarOptions, Window, WindowBackgroundAppearance,
    Subscription, WindowControlArea, WindowOptions, canvas, div, point, prelude::*, px, relative, size,
};
use serde_json::json;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

const TITLEBAR_HEIGHT: f32 = 38.0;
const SIDEBAR_WIDTH: f32 = 256.0;

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
    /// A herdr error from the last command, shown over the grid for a few seconds.
    flash: Option<String>,
    /// The tab or workspace label being edited in place.
    renaming: Option<Rename>,
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
            cx.set_global(Theme::for_appearance(window.appearance()));
            window
                .observe_window_appearance(|window, cx| {
                    cx.set_global(Theme::for_appearance(window.appearance()));
                    cx.refresh_windows();
                })
                .detach();
            cx.new(|cx| {
                let shell = Shell {
                    snapshot: None,
                    error: None,
                    panes: HashMap::new(),
                    own_pane: std::env::var("HERDR_PANE_ID").ok(),
                    focused: None,
                    selected_tab: std::env::var("SHEPHERD_TAB").ok(),
                    should_move: Rc::new(Cell::new(false)),
                    client: Client::local(),
                    rt: rt.clone(),
                    grid_bounds: Rc::new(Cell::new(None)),
                    divider: None,
                    flash: None,
                    renaming: None,
                };
                let client = shell.client.clone();
                cx.spawn(async move |this, cx| sync_loop(this, rt, client, cx).await).detach();
                shell
            })
        })
        .expect("open window");
    }

    /// Send a command to herdr; `then` gets the result. Errors show over the grid for 4 s.
    fn send(
        &mut self,
        command: Command,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Shell, &serde_json::Value, &mut Context<Shell>) + 'static,
    ) {
        let (method, params) = command.request();
        let client = self.client.clone();
        let call = self.rt.spawn(async move { client.call(method, params).await });
        cx.spawn(async move |this, cx| {
            let result = call.await.map_err(anyhow::Error::from).and_then(|r| r);
            let _ = this.update(cx, |shell, cx| match result {
                Ok(value) => then(shell, &value, cx),
                Err(err) => shell.show_error(format!("{err:#}"), cx),
            });
        })
        .detach();
    }

    fn new_tab(&mut self, workspace_id: String, cx: &mut Context<Self>) {
        self.send(Command::NewTab { workspace_id }, cx, |shell, result, cx| {
            shell.selected_tab = crate::commands::created_tab_id(result);
            cx.notify();
        });
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
            input.set_editor_style(InputEditorStyle {
                foreground: theme.text,
                muted_foreground: theme.text_faint,
                border: theme.border,
                selection: theme.accent.opacity(0.3),
                caret: theme.text,
                ..Default::default()
            });
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
        // The last position, unthrottled, so herdr ends where the pointer let go.
        self.send(Command::SetSplitRatio { tab_id: drag.tab_id, path: drag.path, ratio: drag.ratio }, cx, |_, _, _| {});
        cx.notify();
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

fn status_color(status: AgentStatus, theme: &Theme) -> Option<Hsla> {
    match status {
        AgentStatus::Working => Some(theme.accent),
        AgentStatus::Blocked => Some(theme.warning),
        AgentStatus::Done => Some(theme.success.opacity(0.9)),
        AgentStatus::Idle | AgentStatus::Unknown => None,
    }
}

/// Collie's status dot: 6 px, filled for an agent state, a faint ring when idle.
fn status_dot(status: AgentStatus, theme: &Theme) -> Div {
    let dot = div().size(px(6.)).rounded_full().flex_none();
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

/// `×` that shows while the pointer is over its row (`group`).
fn close_button(theme: &Theme, group: &SharedString, id: SharedString) -> Stateful<Div> {
    let hover = theme.hover;
    div()
        .id(id)
        .size(px(16.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .text_size(px(13.))
        .text_color(theme.text_faint)
        .invisible()
        .group_hover(group.clone(), |s| s.visible())
        .hover(move |s| s.bg(hover))
        .child("×")
}

/// Collie's sidebar row: 32 px, 8 px radius, a wash under the selected one.
fn row(theme: &Theme, selected: bool, id: impl Into<gpui::ElementId>) -> Stateful<Div> {
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
        let theme = Theme::of(cx).clone();
        let snapshot = self.snapshot.clone();
        let tab = snapshot.as_ref().and_then(|s| s.pick_tab(self.selected_tab.as_deref())).cloned();
        let frames = match (&snapshot, &tab) {
            (Some(s), Some(t)) => s.pane_frames(&t.tab_id),
            _ => Vec::new(),
        };
        let own = self.own_pane.clone();
        let shown: Vec<PaneFrame> = frames.iter().filter(|f| attachable(&f.pane_id, own.as_deref())).cloned().collect();
        prune(&mut self.panes, &mut self.focused, &shown, |_| false);
        for f in &shown {
            if !self.panes.contains_key(&f.pane_id) {
                let id = f.pane_id.clone();
                self.panes.insert(id.clone(), cx.new(|cx| PaneTerminal::new(id, cx)));
            }
            if f.focused && self.focused.as_deref() != Some(f.pane_id.as_str()) {
                let handle = self.panes[&f.pane_id].read(cx).focus_handle(cx);
                window.focus(&handle, cx);
                self.focused = Some(f.pane_id.clone());
            }
        }

        let shown_ws = tab.as_ref().map(|t| t.workspace_id.clone());
        let shown_tab = tab.as_ref().map(|t| t.tab_id.clone());
        let mut workspaces = snapshot.as_ref().map(|s| s.workspaces.clone()).unwrap_or_default();
        workspaces.sort_by_key(|w| w.number);

        let sidebar = div()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .pt(px(TITLEBAR_HEIGHT))
            .bg(theme.wash(0.05))
            .border_r_1()
            .border_color(theme.border)
            .child(
                div()
                    .px(px(16.))
                    .pt(px(8.))
                    .pb(px(6.))
                    .flex()
                    .items_center()
                    .text_size(px(11.))
                    .text_color(theme.text_faint)
                    .child(div().flex_1().child("Workspaces"))
                    .child(
                        icon_button(&theme, "new-workspace", "+")
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.new_workspace(cx))),
                    ),
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
                    .children(workspaces.into_iter().map(|w| {
                        let selected = shown_ws.as_deref() == Some(w.workspace_id.as_str());
                        let first_tab =
                            snapshot.as_ref().and_then(|s| s.tabs_of(&w.workspace_id).first().map(|t| t.tab_id.clone()));
                        let target = w.active_tab_id.clone().or(first_tab);
                        let rename_target = RenameTarget::Workspace(w.workspace_id.clone());
                        let editing = self.rename_field(&rename_target, cx);
                        let (ws_id, label) = (w.workspace_id.clone(), w.label.clone());
                        let group = SharedString::from(format!("ws-row-{}", w.workspace_id));
                        row(&theme, selected, SharedString::from(format!("ws-{}", w.workspace_id)))
                            .group(group.clone())
                            .child(status_dot(w.agent_status, &theme))
                            .map(|el| match editing {
                                Some(field) => el.child(field),
                                None => el.child(div().flex_1().min_w_0().truncate().child(w.label.clone())),
                            })
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(theme.text_faint)
                                    .group_hover(group.clone(), |s| s.invisible())
                                    .child(w.number.to_string()),
                            )
                            .child(close_button(&theme, &group, SharedString::from(format!("ws-x-{}", w.workspace_id))).on_click(
                                cx.listener({
                                    let ws_id = ws_id.clone();
                                    move |this, _: &ClickEvent, _, cx| {
                                        cx.stop_propagation();
                                        this.send(Command::CloseWorkspace { workspace_id: ws_id.clone() }, cx, |_, _, _| {});
                                    }
                                }),
                            ))
                            .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
                                if e.click_count() == 2 {
                                    this.start_rename(RenameTarget::Workspace(ws_id.clone()), label.clone(), window, cx);
                                    return;
                                }
                                this.selected_tab = target.clone();
                                cx.notify();
                            }))
                    })),
            );

        let tabs = match (&snapshot, &shown_ws) {
            (Some(s), Some(ws)) => s.tabs_of(ws).into_iter().cloned().collect(),
            _ => Vec::new(),
        };
        let tabbar = div()
            .id("tabbar")
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(2.))
            .px(px(8.))
            .overflow_x_scroll()
            .children(tabs.into_iter().map(|t| {
                let selected = shown_tab.as_deref() == Some(t.tab_id.as_str());
                let id = t.tab_id.clone();
                let editing = self.rename_field(&RenameTarget::Tab(t.tab_id.clone()), cx);
                let label = t.label.clone();
                let group = SharedString::from(format!("tab-row-{}", t.tab_id));
                let close_id = t.tab_id.clone();
                row(&theme, selected, SharedString::from(format!("tab-{}", t.tab_id)))
                    .group(group.clone())
                    .h(px(28.))
                    .max_w(px(240.))
                    .when(!selected, |el| el.text_color(theme.text_muted))
                    .child(status_dot(t.agent_status, &theme))
                    .map(|el| match editing {
                        Some(field) => el.child(field),
                        None => el.child(div().min_w_0().truncate().child(t.label.clone())),
                    })
                    .child(close_button(&theme, &group, SharedString::from(format!("tab-x-{}", t.tab_id))).on_click(
                        cx.listener(move |this, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            this.send(Command::CloseTab { tab_id: close_id.clone() }, cx, |_, _, _| {});
                        }),
                    ))
                    .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
                        if e.click_count() == 2 {
                            this.start_rename(RenameTarget::Tab(id.clone()), label.clone(), window, cx);
                            return;
                        }
                        this.selected_tab = Some(id.clone());
                        cx.notify();
                    }))
            }))
            .when_some(shown_ws.clone(), |el, ws| {
                el.child(
                    icon_button(&theme, "new-tab", "+")
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.new_tab(ws.clone(), cx))),
                )
            });

        let splits = match (&snapshot, &tab) {
            (Some(s), Some(t)) => s.split_frames(&t.tab_id),
            _ => Vec::new(),
        };
        let shown_tab_id = shown_tab.clone().unwrap_or_default();
        let grid_bounds = self.grid_bounds.clone();
        let border = theme.border;
        let ring = theme.accent.opacity(0.5);
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
                    .h(relative(f.frame.h))
                    .p(px(0.5))
                    .bg(border);
                match self.panes.get(&f.pane_id) {
                    Some(pane) => {
                        let focused = pane.read(cx).focus_handle(cx).is_focused(window);
                        cell.child(
                            div().size_full().when(focused, |el| el.border_1().border_color(ring)).child(pane.clone()),
                        )
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
        let hover = theme.border_strong;
        let active = theme.accent;
        let grid = grid
            .children(splits.into_iter().map(|split| {
                let dragging = self.divider.as_ref().is_some_and(|d| d.path == split.path);
                let ratio = self.divider.as_ref().filter(|d| d.path == split.path).map_or(split.ratio, |d| d.ratio);
                let id = SharedString::from(format!("split-{}", split.path.iter().map(|b| if *b { '1' } else { '0' }).collect::<String>()));
                let strip = if split.vertical {
                    div()
                        .left(relative(split.frame.x + split.frame.w * ratio))
                        .ml(px(-3.))
                        .top(relative(split.frame.y))
                        .h(relative(split.frame.h))
                        .w(px(6.))
                        .cursor_col_resize()
                } else {
                    div()
                        .top(relative(split.frame.y + split.frame.h * ratio))
                        .mt(px(-3.))
                        .left(relative(split.frame.x))
                        .w(relative(split.frame.w))
                        .h(px(6.))
                        .cursor_row_resize()
                };
                let tab_id = shown_tab_id.clone();
                strip
                    .id(id)
                    .absolute()
                    .when(dragging, |el| el.bg(active.opacity(0.6)))
                    .when(!dragging, |el| el.hover(move |s| s.bg(hover)))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            this.start_divider(tab_id.clone(), &split, cx);
                        }),
                    )
            }))
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

        div()
            .id("root")
            .relative()
            .size_full()
            .flex()
            .flex_row()
            .bg(theme.glass())
            .text_color(theme.text)
            .font_family(theme.font_ui.clone())
            .text_size(px(13.))
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, _, cx| this.drag_divider(e.position, cx)))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| this.end_divider(cx)))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _, _, cx| this.end_divider(cx)))
            .child(self.drag_strip())
            .child(sidebar)
            .child(div().flex_1().min_w_0().h_full().flex().flex_col().child(tabbar).child(grid))
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
