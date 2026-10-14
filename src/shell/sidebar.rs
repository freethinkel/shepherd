//! The sidebar: its slide, resize and folds, and the agents list's height and sort.
use super::*;

/// How long the floating sidebar stays after the pointer leaves it.
const PEEK_LINGER: Duration = Duration::from_millis(200);

impl Shell {
    /// Folds or unfolds a worktree group, tweening from wherever its body is now.
    pub(super) fn toggle_fold(&mut self, id: &str, full: f32, cx: &mut Context<Self>) {
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
        // ponytail: ids of workspaces since closed stay in the list; prune against the snapshot if it grows
        let mut folded: Vec<String> = self.folded.iter().cloned().collect();
        folded.sort();
        save_from_sidebar(cx, |s| s.folded = folded);
    }

    /// The sidebar's width this frame: mid tween, or where it rests.
    pub(super) fn sidebar_now(&self) -> f32 {
        match self.sidebar_motion.filter(|m| m.animating()) {
            Some(m) => m.current(),
            None if self.sidebar_open => self.sidebar_width + self.sidebar_nudge().unwrap_or(0.),
            None => 0.,
        }
    }

    /// The edge spring's offset while it plays.
    pub(super) fn sidebar_nudge(&self) -> Option<f32> {
        let (sign, started) = self.sidebar_edge?;
        let t = started.elapsed().as_secs_f32() / EDGE_SPRING.as_secs_f32();
        (t < 1.).then(|| sign * edge_nudge(t))
    }

    /// Hides or shows the sidebar with the fold tween; panes keep their grids until it lands.
    pub(super) fn hover_seam(&mut self, id: impl Into<SharedString>, on: bool, cx: &mut Context<Self>) {
        let entry = self.seam_hover.entry(id.into()).or_insert((false, 0));
        if entry.0 != on {
            *entry = (on, entry.1 + 1);
            cx.notify();
        }
    }

    /// The floating sidebar's slide this frame: 0 hidden, 1 shown.
    pub(super) fn peek_now(&self) -> f32 {
        self.sidebar_peek.map_or(0., FoldMotion::current)
    }

    /// The pointer came onto the window's left edge or the floating sidebar (`on`), or left them. It shows at once
    /// and hides a moment after the pointer leaves, so crossing from the edge to the card doesn't flicker.
    pub(super) fn hover_peek(&mut self, on: bool, cx: &mut Context<Self>) {
        self.peek_hovered = on;
        if on {
            self.set_peek(true, cx);
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(PEEK_LINGER).await;
            let _ = this.update(cx, |shell, cx| {
                // ponytail: a sidebar menu holds it out; one closed with the pointer elsewhere leaves it out until
                // the pointer passes over it again
                let open = |m: Option<bool>| m.unwrap_or(false);
                let menu = open(shell.ws_menu.as_ref().map(|m| m.is_open()))
                    || open(shell.view_menu.as_ref().map(|m| m.is_open()))
                    || open(shell.agents_menu.as_ref().map(|m| m.is_open()))
                    || open(shell.place_menu.as_ref().map(|m| m.is_open()));
                if !shell.peek_hovered && !menu {
                    shell.set_peek(false, cx);
                }
            });
        })
        .detach();
    }

    fn set_peek(&mut self, open: bool, cx: &mut Context<Self>) {
        let to = if open { 1. } else { 0. };
        if self.sidebar_peek.map_or(0., |m| m.to) == to {
            return;
        }
        self.sidebar_peek = Some(FoldMotion { epoch: 0, from: self.peek_now(), to, started: Instant::now() });
        cx.notify();
    }

    pub(super) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        let from = self.sidebar_now();
        self.sidebar_peek = None;
        self.sidebar_edge = None;
        self.sidebar_open = !self.sidebar_open;
        let open = self.sidebar_open;
        save_from_sidebar(cx, |s| s.sidebar_open = open);
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

    pub(super) fn drag_sidebar(&mut self, x: Pixels, cx: &mut Context<Self>) {
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

    pub(super) fn end_sidebar_drag(&mut self, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.sidebar_drag) {
            cx.set_global(GridHold(false));
            self.save_sidebar_width(cx);
            cx.refresh_windows();
        }
    }

    pub(super) fn save_frame(&mut self, cx: &mut Context<Self>) {
        if let Some(frame) = self.frame.take() {
            save_from_sidebar(cx, |s| s.window = Some(frame));
        }
    }

    pub(super) fn save_sidebar_width(&self, cx: &mut Context<Self>) {
        let width = self.sidebar_width;
        save_from_sidebar(cx, |s| s.sidebar_width = width);
    }

    pub(super) fn drag_agents(&mut self, y: Pixels, window: &Window, cx: &mut Context<Self>) {
        let Some((y0, h0)) = self.agents_drag else { return };
        // ponytail: capped at 60 % of the window so the workspaces list keeps some room
        let max = f32::from(window.viewport_size().height) * 0.6;
        self.agents_height = Some((h0 + y0 - f32::from(y)).clamp(AGENTS_MIN, max.max(AGENTS_MIN)));
        cx.notify();
    }

    pub(super) fn end_agents_drag(&mut self, cx: &mut Context<Self>) {
        if self.agents_drag.take().is_some() {
            let height = self.agents_height;
            save_from_sidebar(cx, |s| s.agents_height = height);
        }
    }

    /// Brings the agent's tab up and its pane to the keyboard.
    pub(super) fn show_agent(&mut self, tab_id: String, pane_id: String, cx: &mut Context<Self>) {
        self.selected_tab = Some(tab_id);
        self.focus_next = Some(pane_id);
        cx.notify();
    }

    pub(super) fn set_agent_sort(&mut self, sort: AgentSort, cx: &mut Context<Self>) {
        self.agent_sort = sort;
        if let Some(m) = self.agents_menu.as_mut() { m.close() }
        save_from_sidebar(cx, |s| s.agent_sort = sort);
        cx.notify();
    }

    /// The workspace list's rows for `snapshot`: each workspace with its worktrees folded under it. `live` rows answer
    /// clicks, renames and drags and show worktrees being made; a swipe's preview of another place draws the same rows
    /// inert, so the list that slides in is the one that stays when it lands.
    pub(super) fn workspace_rows(
        &self,
        theme: &Theme,
        snapshot: Option<&Snapshot>,
        shown_ws: &Option<String>,
        live: bool,
        active: bool,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        // a workspace row's height, one line; a worktree's, which always has a branch line unless compact
        let (ws_h, kid_h) = if SettingsGlobal::of(cx).settings.compact_workspaces { (26., 26.) } else { (32., WS_TWO_LINES) };
        let groups: Vec<(crate::model::Workspace, Vec<crate::model::Workspace>)> = snapshot
            .as_ref()
            .map(|s| {
                crate::model::groups(s.workspace_tree())
                    .into_iter()
                    .map(|(w, kids)| (w.clone(), kids.into_iter().cloned().collect()))
                    .collect()
            })
            .unwrap_or_default();
        let ws_row = |w: &crate::model::Workspace, nested: bool, chevron: Option<gpui::AnyElement>, folded: Option<usize>, cx: &mut Context<Self>| {
            let show = SettingsGlobal::of(cx).settings.workspace_show;
            let compact = SettingsGlobal::of(cx).settings.compact_workspaces;
            let selected = shown_ws.as_deref() == Some(w.workspace_id.as_str());
            let first_tab =
                snapshot.as_ref().and_then(|s| s.tabs_of(&w.workspace_id).first().map(|t| t.tab_id.clone()));
            let target = w.active_tab_id.clone().or(first_tab);
            let rename_target = RenameTarget::Workspace(w.workspace_id.clone());
            let editing = live.then(|| self.rename_field(&rename_target, cx)).flatten();
            let (ws_id, label) = (w.workspace_id.clone(), w.label.clone());
            // a workspace herdr knows no worktree of lives where its panes were opened
            let dir = w.worktree.as_ref().map(|t| t.checkout_path.as_str()).filter(|d| !d.is_empty()).or_else(|| {
                snapshot.as_ref()?.panes.iter().find(|p| p.workspace_id == w.workspace_id)?.cwd.as_deref()
            }).map(crate::herdr::locate);
            let git = dir.as_deref().filter(|_| show.branch).and_then(|d| crate::ui::statusbar::git_of(d, cx));
            // herdr's commits ahead of and behind the upstream, also on a compact row
            let sync = git.as_ref().filter(|g| g.ahead > 0 || g.behind > 0).map(|g| {
                div()
                    .flex_none()
                    .flex()
                    .gap(px(3.))
                    .text_size(px(11.))
                    .when(g.ahead > 0, |el| el.child(div().text_color(theme.success.opacity(0.8)).child(format!("↑{}", g.ahead))))
                    .when(g.behind > 0, |el| el.child(div().text_color(theme.danger.opacity(0.8)).child(format!("↓{}", g.behind))))
            });
            let branch = git.map(|g| g.branch).filter(|_| !compact);
            let (sync, sync_inline) = if compact { (None, sync) } else { (sync, None) };
            // compact rows are the name and its ↑↓ alone: gh and git's diff aren't even asked
            let (changes, pr) = match dir.as_deref().filter(|_| nested && !compact) {
                Some(d) => crate::ui::statusbar::changes_of(d, active, cx),
                None => (None, None),
            };
            let changes = changes.filter(|c| show.changes && *c != (0, 0)).map(|(added, removed)| {
                div()
                    .flex()
                    .gap(px(3.))
                    .when(added > 0, |el| el.child(div().text_color(theme.success.opacity(0.7)).child(format!("+{added}"))))
                    .when(removed > 0, |el| el.child(div().text_color(theme.danger.opacity(0.7)).child(format!("−{removed}"))))
            });
            let pr = pr.filter(|_| show.pull_request);
            let extras = (changes.is_some() || pr.is_some()).then(|| {
                div().flex_none().flex().gap(px(4.)).text_size(px(11.)).text_color(theme.text_faint).children(changes).children(pr)
            });
            // Zeron's card: the name, and under it the branch with what it changed; one line when compact
            // a worktree always has its branch line, so a folding group knows its height
            let two = !compact && (nested || branch.is_some() || extras.is_some());
            let (inline, below) = if two { (None, extras) } else { (extras, None) };
            // a repo's own artwork or its monogram; its worktrees, indented under its name, go without
            // compact, a worktree has no branch line: a branch mark in front of its name tells it apart
            let icon = if nested {
                compact.then(|| {
                    div()
                        .size(px(14.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(gpui::svg().path("icons/git-branch.svg").size(px(12.)).text_color(theme.text_muted))
                        .into_any_element()
                })
            } else {
                show.icon.then(|| {
                    let seed = w.worktree.as_ref().map_or(w.label.as_str(), |t| t.repo_key.as_str());
                    crate::ui::project_icon::project_icon(&w.label, seed, dir.as_deref(), 14., selected, theme, cx)
                })
            };
            let name = match editing {
                Some(field) => field.into_any_element(),
                None => div().min_w_0().truncate().child(w.label.clone()).into_any_element(),
            };
            row(theme, selected, SharedString::from(format!("ws-{}", w.workspace_id)))
                .h(px(if two { WS_TWO_LINES } else { ws_h }))
                .when(live, |el| pane_drag::accept_pane_in_workspace(el, &w.workspace_id, theme.accent.opacity(0.15), cx))
                .when(nested, |el| el.pl(px(if show.icon { 30. } else { 20. })))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .children(icon)
                                .child(name)
                                .children(folded.map(|n| count_badge(theme, n)))
                                .children(sync_inline)
                                .child(div().flex_1())
                                .children(inline),
                        )
                        .when(two, |el| {
                            el.child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(11.))
                                    .text_color(theme.text_muted)
                                    .when(show.icon && !nested, |el| el.pl(px(22.)))
                                    .children(branch.map(|b| {
                                        div()
                                            .min_w_0()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .child(gpui::svg().path("icons/git-branch.svg").size(px(11.)).flex_none().text_color(theme.text_muted))
                                            .child(div().min_w_0().truncate().child(b))
                                    }))
                                    .children(sync)
                                    .child(div().flex_1())
                                    .children(below),
                            )
                        }),
                )
                .map(|el| {
                    if live && self.removing.contains(&w.workspace_id) {
                        el.text_color(theme.text_faint).child(spinner(theme, format!("ws-removing-{}", w.workspace_id)))
                    } else {
                        el.when(show.status, |el| el.child(status_dot(w.agent_status, theme)))
                    }
                })
                .children(chevron)
                .when(live, |el| {
                    el.on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
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
                })
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
                .filter(|(_, branch)| live && !checked_out(branch))
                .map(|(_, branch)| branch.clone())
                .collect();
            if kids.is_empty() && pending.is_empty() {
                return ws_row(&w, false, None, None, cx).into_any_element();
            }
            // A folded group opens while one of its worktrees is being created.
            let open = !self.folded.contains(&w.workspace_id) || !pending.is_empty();
            let full = (kids.len() + pending.len()) as f32 * (kid_h + 2.);
            let tween = self.fold_motion.get(&w.workspace_id).copied().filter(|m| live && m.animating());
            let chevron = fold_chevron(theme, &w.workspace_id, open, full, tween).when(live, |el| {
                el.on_click(cx.listener({
                    let id = w.workspace_id.clone();
                    move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        this.toggle_fold(&id, full, cx);
                        cx.notify();
                    }
                }))
            });
            let header = ws_row(&w, false, Some(chevron.into_any_element()), (!open).then_some(kids.len()), cx);
            let body = div()
                .w_full()
                .flex_none()
                .overflow_hidden()
                .flex()
                .flex_col()
                .pt(px(2.))
                .gap(px(2.))
                .children(kids.iter().map(|k| ws_row(k, true, None, None, cx).into_any_element()).collect::<Vec<_>>())
                .children(pending.iter().map(|b| pending_worktree_row(theme, b).h(px(kid_h)).into_any_element()));
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
    }

    /// The sidebar's rows: workspaces with their worktrees folded under them, the agents list and the update strip.
    pub(super) fn render_sidebar(
        &mut self,
        theme: &Theme,
        snapshot: &Option<Snapshot>,
        shown_ws: &Option<String>,
        shown_tab: &Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut agents = snapshot.as_ref().map(|s| agent_entries(s, self.agent_sort, &self.since)).unwrap_or_default();
        // the section stays while any agent runs, so its menu can turn the filter off again
        let any_agents = !agents.is_empty();
        if SettingsGlobal::of(cx).settings.attention_agents {
            agents.retain(|(p, _, _)| crate::notify::wants_attention(p.agent_status));
        }
        let hover_bg = theme.hover;
        let active = window.is_window_active();

        let strip = update_strip(theme, cx);
        let waiting = self.error.is_none() && self.connecting.is_some_and(|at| crate::ui::loader::due(at, window));
        let (list_offset, beside) = self.list_offset(window);
        let list_fade = 1. - (list_offset.abs() / 400.).min(0.5);
        let beside = beside.map(|(place, at)| {
            div().absolute().top_0().bottom_0().w(px(self.sidebar_width - 1.)).left(px(at)).child(self.place_preview(&place, theme, window, cx))
        });
        // Laid out at full width and clipped by its tweened box, so rows never reflow mid slide.
        div()
            .on_scroll_wheel(cx.listener(|this, e: &gpui::ScrollWheelEvent, _, cx| this.sidebar_scroll(e, cx)))
            .w(px(self.sidebar_width - 1.))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .pt(px(TITLEBAR_HEIGHT))
            // everything between the titlebar and the place bar goes under a swipe's fingers, then springs home, and
            // beside it the place it goes to (or comes from)
            .child(
                div().relative().flex_1().min_h_0().overflow_hidden().children(beside).child(
            div()
                .relative()
                .left(px(list_offset))
                .opacity(list_fade)
                .size_full()
                .flex()
                .flex_col()
            .child(
                div()
                    .px(px(16.))
                    .pt(px(8.))
                    .pb(px(6.))
                    .flex()
                    .items_center()
                    .text_size(px(11.))
                    .text_color(theme.text_faint)
                    // the herdr session shown, as Arc names the space
                    .child(div().flex_1().truncate().child(self.place_label(cx)))
                    // the place's own snapshot hasn't come yet: the list is its last, or none
                    .when(waiting, |el| el.child(crate::ui::loader::mini_spinner(2., theme.text_faint, window))),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                div()
                    .id("workspaces")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    // a sideways swipe moves between places, not down the list
                    .map(|mut el| {
                        el.style().restrict_scroll_to_axis = Some(true);
                        el
                    })
                    .px(px(8.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .children(self.workspace_rows(theme, snapshot.as_ref(), shown_ws, true, active, cx))
            ),
            )
            .child(
                div().flex_none().m(px(8.)).flex().gap(px(4.)).child(
                div()
                    .id("new-workspace")
                    .flex_1()
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
                ),
            )
            .when(SettingsGlobal::of(cx).settings.show_agents && any_agents, |el| {
                let seam = theme.accent;
                let dragging = self.agents_drag.is_some();
                el.child(
                    div()
                        .relative()
                        .px(px(16.))
                        .pt(px(12.))
                        // the list's top fade starts just under the label
                        .pb(px(4.))
                        .border_t_1()
                        .border_color(theme.border)
                        .text_size(px(11.))
                        .text_color(theme.text_faint)
                        .flex()
                        .items_center()
                        .justify_between()
                        .child("Agents")
                        .child(
                            icon_button(theme, "agents-view", "")
                                .tooltip(tip("Sort agents", None))
                                .size(px(20.))
                                .my(px(-4.))
                                .child(gpui::svg().path("icons/more-horizontal.svg").size(px(14.)).text_color(theme.text_faint))
                                .when(self.agents_menu.as_ref().is_some_and(|m| m.is_open()), |el| el.bg(theme.hover))
                                // capture, like the sidebar's `…`: a press on the open menu's trigger closes it
                                .capture_any_mouse_down(cx.listener(|this, e: &MouseDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    match this.agents_menu.as_mut() {
                                        Some(menu) if menu.is_open() => menu.close(),
                                        _ => this.agents_menu = Some(crate::ui::Popup::new(e.position)),
                                    }
                                    cx.notify();
                                })),
                        )
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
                        // Zeron's fade where rows scroll under the edges (the bar below)
                        crate::ui::edge_fade::edge_faded(12., &self.agents_scroll, div()
                            .id("agents")
                            .min_h_0()
                            .overflow_y_scroll()
                            .track_scroll(&self.agents_scroll)
                            .px(px(8.))
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .when(agents.is_empty(), |el| {
                                el.child(div().px(px(8.)).py(px(6.)).text_size(px(11.)).text_color(theme.text_faint).child("Nothing needs you"))
                            })
                            .children(agent_rows(agents, self.agent_sort, SettingsGlobal::of(cx).settings.compact_agents, shown_tab.as_deref(), theme, |row, pane| {
                                let (tab_id, pane_id) = (pane.tab_id.clone(), pane.pane_id.clone());
                                row.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.show_agent(tab_id.clone(), pane_id.clone(), cx)))
                            }))
                            // the last row scrolls clear of the list's faded edge
                            .child(div().flex_none().h(px(4.)))),
                    )
                    .child(crate::ui::scrollbar("agents-scrollbar", &self.agents_scroll, theme)),
                )
            }),
            ))
            .children(strip)
            .child(self.place_bar(theme, list_offset, window, cx))
    }

    /// Arc's space bar: a button for each herdr session and machine, the shown one lit, and + for a new one.
    /// `offset` is the workspace list's: the lit square under the buttons moves as far toward the next place as the list
    /// has moved toward it.
    fn place_bar(&self, theme: &Theme, offset: f32, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let hover_bg = theme.hover;
        let shown_at = self.places.iter().position(|p| self.places_shown(p));
        let buttons = self.places.iter().enumerate().map(|(i, place)| {
            let shown = self.places_shown(place);
            let (name, icon) = place.look(cx);
            let letter = || div().text_size(px(12.)).font_weight(gpui::FontWeight::SEMIBOLD).child(name.chars().next().unwrap_or('?').to_uppercase().to_string());
            let mark = match (icon, place) {
                (Some(icon), _) => sessions::icon_mark(&icon, 15., if shown { theme.accent } else { theme.text_faint }),
                (None, sessions::Place::Machine(_)) => {
                    gpui::svg().path("icons/remote-server.svg").size(px(15.)).text_color(if shown { theme.accent } else { theme.text_faint }).into_any_element()
                }
                (None, sessions::Place::Session(_)) => letter().into_any_element(),
            };
            let (to, menu) = (place.clone(), place.clone());
            div()
                .id(("place", i))
                .tooltip(tip(name.clone(), None))
                .size(px(30.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.))
                .cursor_pointer()
                .relative()
                .text_color(if shown { theme.accent } else { theme.text_faint })
                .when(!shown, |el| el.hover(move |s| s.bg(hover_bg)))
                .child(mark)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    if !this.places_shown(&to) {
                        this.switch_to(to.clone(), cx)
                    }
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, e: &MouseDownEvent, _, cx| this.open_place_menu(menu.clone(), e.position, cx)),
                )
        });
        // one button and the gap after it
        let step = 34.;
        if let Some((from, to, at)) = self.bar_glide.get() {
            let t = at.elapsed().as_secs_f32() / FOLD_DURATION.as_secs_f32();
            let x = if t >= 1. { to } else { from + (to - from) * ease_out(t) };
            self.places_scroll.set_offset(gpui::point(px(-x), px(0.)));
            if t >= 1. {
                self.bar_glide.set(None);
            } else {
                window.request_animation_frame();
            }
        }
        let lit = shown_at.map(|at| {
            let x = (at as f32 - offset / self.sidebar_width).clamp(0., (self.places.len() - 1) as f32) * step;
            // a moved square glides to the row's middle; one that stays put leaves the row where it was scrolled
            if x != self.lit_seen.get() {
                self.lit_seen.set(x);
                let width = f32::from(self.places_scroll.bounds().size.width);
                let to = (x + 15. - width / 2.).clamp(0., f32::from(self.places_scroll.max_offset().x));
                let from = -f32::from(self.places_scroll.offset().x);
                if (to - from).abs() > 0.5 {
                    self.bar_glide.set(Some((from, to, Instant::now())));
                }
            }
            div().absolute().top_0().left(px(x)).size(px(30.)).rounded(px(8.)).bg(theme.selected)
        });
        if self.bar_glide.get().is_some() {
            window.request_animation_frame(); // a glide set just now starts on the next frame
        }
        let bar_bounds = self.bar_bounds.clone();
        div()
            .relative()
            .flex_none()
            // close under the agents list, the window's edge kept below
            .pt(px(2.))
            .pb(px(8.))
            .px(px(8.))
            .flex()
            .items_center()
            .child(
                gpui::canvas(move |bounds, _, _| bar_bounds.set(Some(bounds)), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            )
            .child(
                div()
                    .id("open-settings")
                    .tooltip(tip("Settings", Some("OpenSettings")))
                    .size(px(30.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(8.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover_bg))
                    .child(gpui::svg().path("icons/settings.svg").size(px(16.)).text_color(theme.text_muted))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_settings_here(window, cx))),
            )
            // centred while they fit, scrolled sideways (faded at the edges) once they don't
            .child(
                div().flex_1().min_w_0().mx(px(12.)).child(crate::ui::edge_fade::edge_faded_x(16., 
                    &self.places_scroll,
                    div()
                        .id("places")
                        .w_full()
                        .flex()
                        .overflow_x_scroll()
                        .track_scroll(&self.places_scroll)
                        // auto margins centre the row while it fits and fall to 0 once it scrolls
                        .child(div().flex_none().mx_auto().relative().flex().gap(px(step - 30.)).children(lit).children(buttons)),
                )),
            )
            .child(
                div()
                    .id("place-new")
                    .tooltip(tip("New session", None))
                    .size(px(30.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(8.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover_bg))
                    .child(gpui::svg().path("icons/plus.svg").size(px(16.)).text_color(theme.text_muted))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.open_session_dialog(window, cx))),
            )
    }
}

/// The agents list's entries for `s`, in `sort`'s order: each agent with its workspace's label and how long it has
/// waited.
pub(super) fn agent_entries(s: &Snapshot, sort: AgentSort, since: &crate::model::Since) -> Vec<(crate::model::Pane, String, Option<String>)> {
    s.agents_by(sort, since)
        .into_iter()
        .map(|p| {
            let ws = s.workspaces.iter().find(|w| w.workspace_id == p.workspace_id);
            let waited = crate::model::waiting_since(since, p).map(|at| crate::model::ago(at.elapsed()));
            (p.clone(), ws.map(|w| w.label.clone()).unwrap_or_default(), waited)
        })
        .collect()
}

/// The agents list's rows, `act` making each one do something (the swipe's preview of another place leaves them inert).
pub(super) fn agent_rows(
    agents: Vec<(crate::model::Pane, String, Option<String>)>,
    sort: AgentSort,
    compact: bool,
    shown_tab: Option<&str>,
    theme: &Theme,
    act: impl Fn(Stateful<Div>, &crate::model::Pane) -> Stateful<Div>,
) -> Vec<gpui::AnyElement> {
    let workspaces: Vec<String> = agents.iter().map(|(p, _, _)| p.workspace_id.clone()).collect();
    let grouped = sort == AgentSort::Grouped;
    agents
        .into_iter()
        .enumerate()
        .flat_map(|(i, (pane, workspace, waited))| {
            // Grouped: a heading where the workspace changes, which the rows then leave out.
            let heading = (grouped && (i == 0 || workspaces[i - 1] != pane.workspace_id)).then(|| {
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
            let selected = shown_tab == Some(pane.tab_id.as_str());
            let title = pane.terminal_title_stripped.clone().unwrap_or_default();
            let mark = crate::ui::command_panel::agent_icon(pane.agent.as_deref().unwrap_or_default());
            let mark = || gpui::svg().path(mark).size(px(14.)).flex_none().text_color(crate::ui::command_panel::icon_ink(mark, theme.text_muted));
            let agent = row(theme, selected, SharedString::from(format!("agent-{}", pane.pane_id)))
                .h(px(if compact { 30. } else { 44. }))
                .child(status_dot(pane.agent_status, theme))
                .when(compact, |el| {
                    el.child(mark())
                        .child(div().flex_1().min_w_0().truncate().text_size(px(11.)).text_color(theme.text_muted).child(title.clone()))
                        .children(waited.clone().map(|w| div().flex_none().text_size(px(11.)).text_color(theme.text_faint).child(w)))
                })
                .when(!compact, |el| {
                    el.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .gap(px(6.))
                                    .items_center()
                                    .child(mark())
                                    .when(!grouped, |el| el.child(div().min_w_0().truncate().text_color(theme.text_faint).child(workspace)))
                                    .children(waited.map(|w| div().ml_auto().flex_none().text_size(px(11.)).text_color(theme.text_faint).child(w))),
                            )
                            .child(div().truncate().text_size(px(11.)).text_color(theme.text_muted).child(title)),
                    )
                });
            heading.into_iter().chain([act(agent, &pane).into_any_element()])
        })
        .collect()
}
