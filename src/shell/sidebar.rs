//! The sidebar: its slide, resize and folds, and the agents list's height and sort.
use super::*;

/// How long the floating sidebar stays after the pointer leaves it.
const PEEK_LINGER: Duration = Duration::from_millis(200);

impl Shell {
    /// Folds or unfolds a worktree group, tweening from wherever its body is now.
    pub(super) fn toggle_fold(&mut self, id: &str, full: f32, cx: &mut Context<Self>) {
        let open = !self.folded.contains(id);
        self.start_fold_motion(id, open, full);
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
        spring_offset(self.sidebar_edge)
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
                    || open(shell.web_menu.as_ref().map(|m| m.is_open()))
                    || open(shell.view_menu.as_ref().map(|m| m.is_open()))
                    || open(shell.agents_menu.as_ref().map(|m| m.is_open()))
                    || open(shell.agent_menu.as_ref().map(|m| m.is_open()))
                    || open(shell.project_menu.as_ref().map(|m| m.is_open()))
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
                // a later toggle owns the hold now
                if shell.sidebar_motion.is_some_and(|m| m.started == motion.started) {
                    shell.sidebar_motion = None;
                    shell.release_grids(cx);
                }
            });
        })
        .detach();
        cx.notify();
    }

    /// Lets the panes' grids go, unless a drag or a slide still holds them.
    pub(super) fn release_grids(&self, cx: &mut Context<Self>) {
        if self.divider.is_none() && !self.sidebar_drag && !self.right.drag && self.sidebar_motion.is_none() && self.right.motion.is_none() {
            cx.set_global(GridHold(false));
            cx.refresh_windows();
        }
    }

    pub(super) fn drag_sidebar(&mut self, x: Pixels, cx: &mut Context<Self>) {
        if !self.sidebar_drag {
            return;
        }
        let x = f32::from(x);
        spring_at_limit(&mut self.sidebar_edge, x, SIDEBAR_MIN, SIDEBAR_MAX);
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
        self.select_tab(Some(tab_id));
        self.update_panel(cx, |p| p.expanded = false);
        self.focus_next = Some(pane_id);
        cx.notify();
    }

    /// Starts the tween of group `key`'s body as it folds (`folding`) or opens, from wherever it is now.
    fn start_fold_motion(&mut self, key: &str, folding: bool, full: f32) {
        let prev = self.fold_motion.get(key).copied();
        let from = prev.filter(|m| m.animating()).map(FoldMotion::current).unwrap_or(if folding { full } else { 0. });
        let epoch = prev.map_or(1, |m| m.epoch + 1);
        self.fold_motion.insert(key.to_string(), FoldMotion { epoch, from, to: if folding { 0. } else { full }, started: Instant::now() });
    }

    /// Folds or unfolds a project in the agents list, tweening like a worktree group.
    pub(super) fn toggle_agents_fold(&mut self, id: &str, full: f32, cx: &mut Context<Self>) {
        self.fold_agents(id, !self.agents_folded.contains(id), full);
        self.save_agents_folded(cx);
    }

    /// Folds every project in the agents list, or opens them all.
    pub(super) fn fold_all_agents(&mut self, fold: bool, cx: &mut Context<Self>) {
        let row_h = agent_row_h(SettingsGlobal::of(cx).settings.compact_agents);
        let agents = self.shown_agents(cx);
        for group in agents.chunk_by(|a, b| a.project.0 == b.project.0) {
            if self.agents_folded.contains(&group[0].project.0) != fold {
                self.fold_agents(&group[0].project.0, fold, group.len() as f32 * (row_h + 2.));
            }
        }
        self.save_agents_folded(cx);
    }

    fn fold_agents(&mut self, id: &str, fold: bool, full: f32) {
        self.start_fold_motion(&format!("agents-{id}"), fold, full);
        if fold {
            self.agents_folded.insert(id.to_string());
        } else {
            self.agents_folded.remove(id);
        }
    }

    fn save_agents_folded(&mut self, cx: &mut Context<Self>) {
        let mut folded: Vec<String> = self.agents_folded.iter().cloned().collect();
        folded.sort();
        save_from_sidebar(cx, |s| s.agents_folded = folded);
        cx.notify();
    }

    /// Shows the agent waiting longest, or the next one after the agent shown: one key goes round them all.
    pub(super) fn next_waiting_agent(&mut self, cx: &mut Context<Self>) {
        let next = self.snapshot.as_ref().and_then(|s| s.next_waiting(&self.since, self.focused.as_deref()));
        if let Some((tab_id, pane_id)) = next.map(|p| (p.tab_id.clone(), p.pane_id.clone())) {
            self.show_agent(tab_id, pane_id, cx);
        }
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
    ) -> Vec<(String, gpui::AnyElement)> {
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
            let dir = workspace_dir(w, snapshot);
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
            // a worktree being merged and deleted shows its step in place of its branch, or what failed
            let merging = self.merging.get(&w.workspace_id).filter(|_| live);
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
            // its right-click menu open: lit until it closes
            let menu_on = live && self.ws_menu.as_ref().is_some_and(|m| m.is_open() && m.value.0 == w.workspace_id);
            let ring = live.then(|| self.cursor_ring(&SidebarItem::Workspace(w.workspace_id.clone()), &self.workspaces_scroll, theme)).flatten();
            row(theme, selected, SharedString::from(format!("ws-{}", w.workspace_id)))
                .relative()
                .children(ring)
                .h(px(if two { WS_TWO_LINES } else { ws_h }))
                .when(menu_on && !selected, |el| el.bg(theme.hover))
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
                                    .map(|el| match merging {
                                        Some(m) => el.child(
                                            div().min_w_0().truncate().when(m.failed.is_some(), |el| el.text_color(theme.danger)).child(m.step().to_string()),
                                        ),
                                        None => el
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
                                    }),
                            )
                        }),
                )
                .map(|el| match self.busy_mark(w, live, theme, cx) {
                    Some(mark) => el.when(merging.is_none(), |el| el.text_color(theme.text_faint)).child(mark),
                    None => el.when(show.status, |el| el.child(status_dot(w.agent_status, theme))),
                })
                .children(chevron)
                .when(live, |el| {
                    el.on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
                        if e.click_count() == 2 {
                            this.start_rename(RenameTarget::Workspace(ws_id.clone()), label.clone(), window, cx);
                            return;
                        }
                        this.select_tab(target.clone());
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
        let reorder = self.ws_reorder.clone();
        // a top-level row, a workspace or a repo with its worktrees, is dragged to another group's place
        let draggable = |el: Stateful<Div>, w: &crate::model::Workspace| match live && self.renaming.is_none() {
            true => reorder::draggable(el, &reorder, &w.workspace_id),
            false => el,
        };
        groups.into_iter().map(|(w, kids)| {
            let pending = self.pending_in(&w, &kids, live);
            if kids.is_empty() && pending.is_empty() {
                return (w.workspace_id.clone(), draggable(ws_row(&w, false, None, None, cx), &w).into_any_element());
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
            let header = draggable(header, &w);
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
            let body = fold_body(body, &w.workspace_id, open, full, tween);
            (w.workspace_id.clone(), div().flex().flex_col().flex_none().child(header).child(body).into_any_element())
        }).collect::<Vec<_>>()
    }

    /// Worktrees being made in group `w`: they join the group of the workspace their dialog was opened on. herdr opens
    /// the worktree's workspace before it answers (hooks still run): once its checkout shows up, the real row replaces
    /// the placeholder. A swipe's preview shows none.
    pub(super) fn pending_in<W: std::borrow::Borrow<crate::model::Workspace>>(&self, w: &crate::model::Workspace, kids: &[W], live: bool) -> Vec<String> {
        let checked_out = |branch: &str| {
            let slug = crate::commands::branch_path_slug(branch);
            kids.iter().any(|k| {
                k.borrow().worktree.as_ref().is_some_and(|t| {
                    std::path::Path::new(&t.checkout_path).file_name().is_some_and(|n| *n == *slug)
                })
            })
        };
        self.pending_worktrees
            .iter()
            .filter(|(id, _)| *id == w.workspace_id || kids.iter().any(|k| k.borrow().workspace_id == *id))
            .filter(|(_, branch)| live && !checked_out(branch))
            .map(|(_, branch)| branch.clone())
            .collect()
    }

    /// What stands for `w`'s status while it is being merged or removed: a spinner, or the `!` that opens what failed.
    pub(super) fn busy_mark(&self, w: &crate::model::Workspace, live: bool, theme: &Theme, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let id = &w.workspace_id;
        match self.merging.get(id).filter(|_| live) {
            None if live && self.removing.contains(id) => Some(spinner(theme, format!("ws-removing-{id}")).into_any_element()),
            None => None,
            Some(m) if m.failed.is_none() => Some(spinner(theme, format!("ws-merging-{id}")).into_any_element()),
            // the `!` opens what failed, with Force delete
            Some(_) => Some(
                div()
                    .id(SharedString::from(format!("ws-merge-failed-{id}")))
                    .size(px(16.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(theme.danger.opacity(0.18))
                    .text_color(theme.danger)
                    .text_size(px(11.))
                    .font_weight(gpui::FontWeight::BOLD)
                    .cursor_pointer()
                    .child("!")
                    .map(|el| {
                        let id = id.clone();
                        crate::ui::trigger(el, cx.listener(move |this, e: &MouseDownEvent, _, cx| this.open_merge_failure(id.clone(), e.position, cx)))
                    })
                    .into_any_element(),
            ),
        }
    }

    /// The sidebar's workspace rows, dragged to reorder them: a group dropped on another takes its place in herdr.
    #[allow(clippy::too_many_arguments)]
    fn workspace_column(
        &self,
        theme: &Theme,
        snapshot: Option<&Snapshot>,
        shown_ws: &Option<String>,
        shown_tab: Option<&str>,
        active: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let rows = self.sidebar_rows(theme, snapshot, shown_ws, shown_tab, true, active, cx);
        // T3 Code's list has its order from the agents, and fills the column so its shelves sit at the bottom
        if SettingsGlobal::of(cx).settings.sidebar_style == crate::settings::SidebarStyle::T3 {
            return div().flex_grow(1.).flex().flex_col().gap(px(2.)).children(rows.into_iter().map(|(_, row)| row));
        }
        let (column, dropped) = reorder::column(&self.ws_reorder, rows, 2., theme, window, cx.has_active_drag());
        let blocks: Vec<Vec<String>> = snapshot
            .map(|s| crate::model::groups(s.workspace_tree()).into_iter().map(|(w, kids)| std::iter::once(w).chain(kids).map(|w| w.workspace_id.clone()).collect()).collect())
            .unwrap_or_default();
        if let Some((workspace_ids, before)) = dropped.and_then(|(from, to)| crate::model::group_move(&blocks, from, to)) {
            let this = cx.entity();
            cx.defer(move |cx| this.update(cx, |this, cx| this.send(Command::MoveWorkspaces { workspace_ids, before }, cx, |_, _, _| {})));
        }
        column
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
            agents.retain(|e| crate::notify::wants_attention(e.pane.agent_status));
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
            .children(self.web_rows(self.shown_place_key().as_deref(), true, theme, window, cx))
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
            .children(self.thread_bar(theme, snapshot.as_ref(), true, cx))
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
                    .track_scroll(&self.workspaces_scroll)
                    // a sideways swipe moves between places, not down the list
                    .map(|mut el| {
                        el.style().restrict_scroll_to_axis = Some(true);
                        el
                    })
                    .px(px(8.))
                    .flex()
                    .flex_col()
                    .child(self.workspace_column(theme, snapshot.as_ref(), shown_ws, shown_tab.as_deref(), active, window, cx))
            ),
            )
            // T3 Code's bar has this button
            .when(SettingsGlobal::of(cx).settings.sidebar_style == crate::settings::SidebarStyle::Herdr, |el| el.child(
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
            ))
            .when(SettingsGlobal::of(cx).settings.shows_agents_section() && any_agents, |el| {
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
                            .children(self.agent_rows(agents, SettingsGlobal::of(cx).settings.compact_agents, shown_tab.as_deref(), true, theme, |row, pane| {
                                let (tab_id, pane_id) = (pane.tab_id.clone(), pane.pane_id.clone());
                                let menu = AgentMenu::Agent(pane_id.clone());
                                row.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.show_agent(tab_id.clone(), pane_id.clone(), cx)))
                                    .on_mouse_down(MouseButton::Right, cx.listener(move |this, e: &MouseDownEvent, _, cx| this.open_agent_menu(menu.clone(), e.position, cx)))
                            }, |heading, id, full| {
                                let (id, menu) = (id.to_string(), AgentMenu::Project(id.to_string()));
                                heading
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle_agents_fold(&id, full, cx)))
                                    .on_mouse_down(MouseButton::Right, cx.listener(move |this, e: &MouseDownEvent, _, cx| this.open_agent_menu(menu.clone(), e.position, cx)))
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

    /// Arc's space bar: a button for each herdr session and machine, the shown one lit, and + for a new one. Off herdr
    /// there are none, and only the settings stay.
    /// `offset` is the workspace list's: the lit square under the buttons moves as far toward the next place as the list
    /// has moved toward it.
    fn place_bar(&self, theme: &Theme, offset: f32, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let hover_bg = theme.hover;
        let shown_at = self.places.iter().position(|p| self.places_shown(p));
        // each button at its slot, springing there when the order or a drag's placeholder moves it
        let keys: Vec<String> = self.places.iter().map(sessions::Place::key).collect();
        let dragging = cx.has_active_drag();
        let (slots, moving) = self.place_motion.borrow_mut().layout(&keys, dragging, Instant::now());
        if moving {
            window.request_animation_frame();
        }
        let held = self.place_motion.borrow().drag.as_ref().filter(|_| dragging).map(|d| d.key.clone());
        // the slot it would land in, its mark faint there
        let placeholder = held.as_ref().and_then(|key| keys.iter().position(|k| k == key)).map(|i| {
            let place = &self.places[i];
            let (name, icon) = place.look(cx);
            let mark = sessions::place_mark(&name, icon.as_deref(), matches!(place, sessions::Place::Machine(_)), theme.text_muted);
            div()
                .absolute()
                .top_0()
                .left(px(slots[i].0))
                .size(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.wash(0.04))
                .child(div().opacity(0.4).child(mark))
        });
        let buttons = self.places.iter().enumerate().filter(|(i, _)| held.as_ref() != Some(&keys[*i])).map(|(i, place)| {
            let shown = self.places_shown(place);
            let (name, icon) = place.look(cx);
            let machine = matches!(place, sessions::Place::Machine(_));
            let mark = sessions::place_mark(&name, icon.as_deref(), machine, if shown { theme.accent } else { theme.text_faint });
            let dragged = sessions::DraggedPlace {
                index: i,
                key: keys[i].clone(),
                name: name.clone(),
                icon,
                machine,
                theme: theme.clone(),
                grab: Default::default(),
                motion: self.place_motion.clone(),
            };
            let (to, menu) = (place.clone(), place.clone());
            let (x, y) = slots[i];
            div()
                .id(("place", i))
                .tooltip(tip(name.clone(), None))
                .absolute()
                .left(px(x))
                .top(px(y))
                .size(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.))
                .cursor_pointer()
                .text_color(if shown { theme.accent } else { theme.text_faint })
                .when(!shown, |el| el.hover(move |s| s.bg(hover_bg)))
                .child(mark)
                .on_drag(dragged, |d, grab, _, cx| {
                    d.motion.borrow_mut().drag = Some(sessions::PlaceDrag { key: d.key.clone(), slot: d.index, ghost: None });
                    cx.new(|_| sessions::DraggedPlace { grab, ..d.clone() })
                })
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
        let step = sessions::PLACE_STEP;
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
            // the square stays on its button as that springs to another slot
            let x = (x + slots[at].0 - at as f32 * step).clamp(0., (self.places.len() - 1) as f32 * step);
            div().absolute().top_0().left(px(x)).size(px(30.)).rounded(px(8.)).bg(theme.selected)
        });
        if self.bar_glide.get().is_some() {
            window.request_animation_frame(); // a glide set just now starts on the next frame
        }
        let bar_bounds = self.bar_bounds.clone();
        let motion = self.place_motion.clone();
        div()
            .relative()
            .flex_none()
            // a place dragged over the bar holds the slot under the pointer; let go there, it moves to it
            .on_drag_move(cx.listener(|this, e: &gpui::DragMoveEvent<sessions::DraggedPlace>, _, cx| {
                let slot = this.place_motion.borrow().slot_at(e.event.position.x, this.places.len());
                if let Some(drag) = this.place_motion.borrow_mut().drag.as_mut().filter(|d| d.slot != slot) {
                    drag.slot = slot;
                    haptic();
                    cx.notify();
                }
            }))
            .on_drop(cx.listener(|this, d: &sessions::DraggedPlace, _, cx| {
                let slot = this.place_motion.borrow().drag.as_ref().map_or(d.index, |drag| drag.slot);
                this.move_place(d.index, slot, cx);
            }))
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
                        .child(
                            div()
                                .flex_none()
                                .mx_auto()
                                .relative()
                                .w(px((self.places.len() as f32 * step - (step - 30.)).max(0.)))
                                .h(px(30.))
                                .child(gpui::canvas(move |bounds, _, _| motion.borrow_mut().row = Some(bounds), |_, _, _, _| {}).absolute().size_full())
                                .children(lit)
                                .children(placeholder)
                                .children(buttons),
                        ),
                )),
            )
            .when(self.backend.herdr().is_some(), |bar| {
                bar.child(
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
            })
    }
}

/// Where `w` lives: a workspace herdr knows no worktree of, where its panes were opened.
pub(super) fn workspace_dir(w: &crate::model::Workspace, snapshot: Option<&Snapshot>) -> Option<String> {
    w.worktree
        .as_ref()
        .map(|t| t.checkout_path.as_str())
        .filter(|d| !d.is_empty())
        .or_else(|| snapshot?.panes.iter().find(|p| p.workspace_id == w.workspace_id)?.cwd.as_deref())
        .map(crate::herdr::locate)
}

/// An agents list entry: the agent, its workspace's label, its project (id and label: see
/// [`Snapshot::project_of`]) and how long it has had its status.
pub(super) struct AgentEntry {
    pub pane: crate::model::Pane,
    pub workspace: String,
    pub project: (String, String),
    pub time: Option<String>,
}

/// The agents list's entries for `s`, in `sort`'s order.
pub(super) fn agent_entries(s: &Snapshot, sort: AgentSort, since: &crate::model::Since) -> Vec<AgentEntry> {
    let label = |id: &str| s.workspaces.iter().find(|w| w.workspace_id == id).map(|w| w.label.clone()).unwrap_or_default();
    s.agents_by(sort, since)
        .into_iter()
        .map(|p| AgentEntry {
            pane: p.clone(),
            workspace: label(&p.workspace_id),
            project: s.project_of(&p.workspace_id).map_or_else(|| (p.workspace_id.clone(), label(&p.workspace_id)), |w| (w.workspace_id.clone(), w.label.clone())),
            time: crate::model::timed_since(since, p).map(|at| crate::model::ago(at.elapsed())),
        })
        .collect()
}

/// What the agents list's right-click menu is for: an agent by its pane, or a project by its top-level workspace.
#[derive(Clone, PartialEq)]
pub(super) enum AgentMenu {
    Agent(String),
    Project(String),
}

/// A row the sidebar's keyboard cursor stands on: a workspace, or an agent by its pane.
#[derive(Clone, PartialEq)]
pub(super) enum SidebarItem {
    Workspace(String),
    Agent(String),
}

impl Shell {
    /// The rows the cursor walks, top to bottom as drawn: workspaces with the worktrees of open groups, then the agents
    /// of open projects.
    // ponytail: the Herdr sidebar's order; T3 Code's list walks the same workspaces and draws no cursor
    fn sidebar_items(&self, cx: &App) -> Vec<SidebarItem> {
        let Some(s) = self.snapshot.as_ref() else { return Vec::new() };
        let mut items = Vec::new();
        for (w, kids) in crate::model::groups(s.workspace_tree()) {
            items.push(SidebarItem::Workspace(w.workspace_id.clone()));
            if !self.folded.contains(&w.workspace_id) {
                items.extend(kids.iter().map(|k| SidebarItem::Workspace(k.workspace_id.clone())));
            }
        }
        if SettingsGlobal::of(cx).settings.shows_agents_section() {
            let grouped = self.agent_sort == AgentSort::Grouped;
            items.extend(
                self.shown_agents(cx)
                    .into_iter()
                    .filter(|a| !grouped || !self.agents_folded.contains(&a.project.0))
                    .map(|a| SidebarItem::Agent(a.pane.pane_id)),
            );
        }
        items
    }

    /// cmd-e: the keys to the sidebar, the cursor on the workspace shown; again, back to the panes.
    pub(super) fn focus_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sidebar_cursor.is_some() {
            return self.leave_sidebar(cx);
        }
        // Settings stands in for the sidebar and has its own keys
        if self.settings.is_some() {
            return;
        }
        if !self.sidebar_open {
            self.toggle_sidebar(cx);
        }
        self.release_panel();
        let shown = self.shown_tab().map(|t| SidebarItem::Workspace(t.workspace_id));
        self.sidebar_cursor = shown.or_else(|| self.sidebar_items(cx).into_iter().next());
        if self.sidebar_cursor.is_some() {
            self.reveal_cursor.set(true);
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }

    /// The keys back to the panes: forgetting the pane focused has the next render focus herdr's again.
    pub(super) fn leave_sidebar(&mut self, cx: &mut Context<Self>) {
        if self.sidebar_cursor.take().is_some() {
            self.focused = None;
        }
        cx.notify();
    }

    /// A key while the sidebar has the keys: j/k and ↓/↑ move, Enter opens and goes back to the panes, Space shows
    /// what's under the cursor and stays, h/← folds a group (or goes up to it), l/→ opens one, Esc leaves. False when
    /// the key isn't the sidebar's.
    pub(super) fn sidebar_key(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(at) = self.sidebar_cursor.clone() else { return false };
        let items = self.sidebar_items(cx);
        let i = items.iter().position(|it| *it == at);
        let mut move_to = |to: Option<usize>| {
            if let Some(item) = to.and_then(|to| items.get(to)) {
                self.sidebar_cursor = Some(item.clone());
                self.reveal_cursor.set(true);
            }
        };
        match key {
            "down" | "j" => move_to(Some(i.map_or(0, |i| (i + 1).min(items.len().saturating_sub(1))))),
            "up" | "k" => move_to(Some(i.map_or(0, |i| i.saturating_sub(1)))),
            "enter" | "space" => {
                self.open_sidebar_item(&at, key == "enter", cx);
                if key == "enter" {
                    self.leave_sidebar(cx);
                }
            }
            "left" | "h" | "right" | "l" => {
                let SidebarItem::Workspace(id) = &at else { return true };
                let Some(s) = self.snapshot.as_ref() else { return true };
                let groups = crate::model::groups(s.workspace_tree());
                let fold = matches!(key, "left" | "h");
                if let Some((_, kids)) = groups.iter().find(|(w, kids)| w.workspace_id == *id && !kids.is_empty()) {
                    // a header: fold or open it, as its chevron does
                    if self.folded.contains(id) != fold {
                        let kid_h = if SettingsGlobal::of(cx).settings.compact_workspaces { 26. } else { WS_TWO_LINES };
                        let full = kids.len() as f32 * (kid_h + 2.);
                        let id = id.clone();
                        self.toggle_fold(&id, full, cx);
                    }
                } else if fold && let Some((w, _)) = groups.iter().find(|(_, kids)| kids.iter().any(|k| k.workspace_id == *id)) {
                    // a worktree: up to its group's header
                    self.sidebar_cursor = Some(SidebarItem::Workspace(w.workspace_id.clone()));
                    self.reveal_cursor.set(true);
                }
            }
            "escape" => self.leave_sidebar(cx),
            _ => return false,
        }
        cx.notify();
        true
    }

    /// Shows the cursor's workspace or agent; `focus` gives an agent's pane the keys.
    fn open_sidebar_item(&mut self, item: &SidebarItem, focus: bool, cx: &mut Context<Self>) {
        let Some(s) = self.snapshot.as_ref() else { return };
        match item {
            SidebarItem::Workspace(id) => {
                let target = s.workspaces.iter().find(|w| &w.workspace_id == id).and_then(|w| w.active_tab_id.clone()).or_else(|| s.tabs_of(id).first().map(|t| t.tab_id.clone()));
                self.select_tab(target);
            }
            SidebarItem::Agent(pane_id) => {
                let Some(tab_id) = s.panes.iter().find(|p| &p.pane_id == pane_id).map(|p| p.tab_id.clone()) else { return };
                match focus {
                    true => self.show_agent(tab_id, pane_id.clone(), cx),
                    false => self.select_tab(Some(tab_id)),
                }
            }
        }
    }

    /// FocusLeft from the leftmost pane: the keys to the sidebar. False when a pane lies that way; already there, it
    /// stays.
    pub(super) fn enter_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.sidebar_cursor.is_some() {
            return true;
        }
        let tab = self.shown_tab().map(|t| t.tab_id);
        let layout = self.snapshot.as_ref().and_then(|s| s.layouts.iter().find(|l| Some(&l.tab_id) == tab.as_ref()));
        let beside = self.active_pane(window, cx).is_some_and(|p| layout.is_some_and(|l| l.has_neighbor(&p, Direction::Left)));
        if !beside {
            self.focus_sidebar(window, cx);
        }
        !beside
    }

    /// The cursor's ring over row `item`, while the sidebar has the keys. Just moved, it scrolls the row (in the list
    /// `scroll` tracks) into view once laid out.
    pub(super) fn cursor_ring(&self, item: &SidebarItem, scroll: &gpui::ScrollHandle, theme: &Theme) -> Option<Div> {
        (self.sidebar_cursor.as_ref() == Some(item)).then(|| {
            let (reveal, scroll) = (self.reveal_cursor.clone(), scroll.clone());
            let follow = gpui::canvas(
                move |row, window, _| {
                    if !reveal.replace(false) {
                        return;
                    }
                    let (view, mut at, pad) = (scroll.bounds(), scroll.offset(), px(8.));
                    if row.top() < view.top() {
                        at.y += view.top() - row.top() + pad;
                    } else if row.bottom() > view.bottom() {
                        at.y -= row.bottom() - view.bottom() + pad;
                    } else {
                        return;
                    }
                    at.y = at.y.clamp(-scroll.max_offset().y, px(0.));
                    scroll.set_offset(at);
                    window.refresh();
                },
                |_, _, _, _| {},
            )
            .size_full();
            div().absolute().inset_0().rounded(px(8.)).border_1().border_color(theme.accent.opacity(0.7)).child(follow)
        })
    }
}

/// An agent row's height in the list.
fn agent_row_h(compact: bool) -> f32 {
    if compact { 30. } else { 44. }
}

impl Shell {
    fn open_agent_menu(&mut self, menu: AgentMenu, at: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        self.agent_menu = Some(crate::ui::Popup::new((menu, at)));
        cx.notify();
    }

    /// The agents list as the sidebar shows it: only those waiting on you while that filter is on.
    pub(super) fn shown_agents(&self, cx: &App) -> Vec<AgentEntry> {
        let Some(s) = self.snapshot.as_ref() else { return Vec::new() };
        let mut agents = agent_entries(s, self.agent_sort, &self.since);
        if SettingsGlobal::of(cx).settings.attention_agents {
            agents.retain(|e| crate::notify::wants_attention(e.pane.agent_status));
        }
        agents
    }

    /// Project `id`'s linked worktrees whose branch's PR or MR is merged, as far as gh or glab has said yet.
    pub(super) fn merged_worktrees(&self, id: &str, cx: &mut App) -> Vec<String> {
        let Some(s) = self.snapshot.as_ref() else { return Vec::new() };
        let Some((w, kids)) = crate::model::groups(s.workspace_tree()).into_iter().find(|(w, _)| w.workspace_id == id) else { return Vec::new() };
        let linked: Vec<(String, String)> = std::iter::once(w)
            .chain(kids)
            .filter(|w| !self.removing.contains(&w.workspace_id))
            .filter_map(|w| {
                let t = w.worktree.as_ref().filter(|t| t.is_linked_worktree && !t.checkout_path.is_empty())?;
                Some((w.workspace_id.clone(), crate::herdr::locate(&t.checkout_path)))
            })
            .collect();
        linked.into_iter().filter(|(_, dir)| crate::ui::statusbar::pr_of(dir, true, cx).is_some_and(|pr| pr.state == "merged")).map(|(id, _)| id).collect()
    }

    /// The agents list's rows, `act` making each one do something and `fold` each project heading (given its id and
    /// its body's full height); `live` false is the swipe's preview of another place, inert and without tweens.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn agent_rows(
        &self,
        agents: Vec<AgentEntry>,
        compact: bool,
        shown_tab: Option<&str>,
        live: bool,
        theme: &Theme,
        act: impl Fn(Stateful<Div>, &crate::model::Pane) -> Stateful<Div>,
        fold: impl Fn(Stateful<Div>, &str, f32) -> Stateful<Div>,
    ) -> Vec<gpui::AnyElement> {
        let grouped = self.agent_sort == AgentSort::Grouped;
        let row_h = agent_row_h(compact);
        // the row or heading whose menu is open stays lit
        let menu_for = self.agent_menu.as_ref().filter(|m| live && m.is_open()).map(|m| &m.value.0);
        let mut out = Vec::new();
        // Grouped: a heading per project, which folds its rows away like a worktree group; otherwise one run of rows.
        for (i, group) in agents.chunk_by(|a, b| !grouped || a.project.0 == b.project.0).enumerate() {
            let (id, project) = &group[0].project;
            let key = format!("agents-{id}");
            let open = !grouped || !self.agents_folded.contains(id);
            let full = group.len() as f32 * (row_h + 2.);
            let tween = self.fold_motion.get(&key).copied().filter(|m| live && m.animating());
            let rows = (open || tween.is_some()).then(|| group.iter().map(|AgentEntry { pane, workspace, project, time }| {
                let selected = shown_tab == Some(pane.tab_id.as_str());
                let title = pane.terminal_title_stripped.clone().unwrap_or_default();
                let mark = crate::ui::command_panel::agent_icon(pane.agent.as_deref().unwrap_or_default());
                let mark = || gpui::svg().path(mark).size(px(14.)).flex_none().text_color(crate::ui::command_panel::icon_ink(mark, theme.text_muted));
                // under a project heading, only a worktree's own name says where the agent is
                let place = (!grouped || workspace != &project.1).then(|| workspace.clone());
                let ring = live.then(|| self.cursor_ring(&SidebarItem::Agent(pane.pane_id.clone()), &self.agents_scroll, theme)).flatten();
                let agent = row(theme, selected, SharedString::from(format!("agent-{}", pane.pane_id)))
                    .relative()
                    .children(ring)
                    .h(px(row_h))
                    .when(!selected && matches!(menu_for, Some(AgentMenu::Agent(p)) if *p == pane.pane_id), |el| el.bg(theme.hover))
                    .child(status_dot(pane.agent_status, theme))
                    .when(compact, |el| {
                        el.child(mark())
                            .child(div().flex_1().min_w_0().truncate().text_size(px(11.)).text_color(theme.text_muted).child(title.clone()))
                            .children(time.clone().map(|w| div().flex_none().text_size(px(11.)).text_color(theme.text_faint).child(w)))
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
                                        .children(place.map(|w| div().min_w_0().truncate().text_color(theme.text_faint).child(w)))
                                        .children(time.clone().map(|w| div().ml_auto().flex_none().text_size(px(11.)).text_color(theme.text_faint).child(w))),
                                )
                                .child(div().truncate().text_size(px(11.)).text_color(theme.text_muted).child(title)),
                        )
                    });
                act(agent, pane).into_any_element()
            }).collect::<Vec<_>>());
            if !grouped {
                out.extend(rows.into_iter().flatten());
                continue;
            }
            let heading = div()
                .id(SharedString::from(format!("agents-project-{id}")))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(6.))
                .px(px(8.))
                .mt(px(if i == 0 { 0. } else { 6. }))
                .py(px(2.))
                .rounded(px(6.))
                .cursor_pointer()
                .text_size(px(11.))
                .text_color(theme.text_faint)
                .when(matches!(menu_for, Some(AgentMenu::Project(p)) if p == id), |el| el.bg(theme.hover).text_color(theme.text_muted))
                .child(div().flex_1().min_w_0().truncate().child(project.clone()))
                // folded: how many, and the most urgent status (the group's first)
                .when(!open, |el| el.child(status_dot(group[0].pane.agent_status, theme)).child(group.len().to_string()))
                .child(fold_chevron(theme, &key, open, full, tween));
            // the body's top padding stands in for the gap a heading had above its first row
            let body = div().w_full().flex_none().overflow_hidden().flex().flex_col().pt(px(2.)).gap(px(2.)).children(rows.into_iter().flatten());
            out.push(div().flex().flex_col().flex_none().child(fold(heading, id, full)).child(fold_body(body, &key, open, full, tween)).into_any_element());
        }
        out
    }
}

