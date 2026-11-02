//! The T3 Code sidebar, as Zeron ported it: a search and project select over a flat list of agents, each a card of three lines
//! (the project with the status, the title, the branch with what it changed), and the Working and Settled shelves at
//! the bottom. Shepherd has no threads: an agent is one, and a workspace without an agent settles as itself.
// ponytail: no Snoozed shelf or pins
use super::*;
use crate::model::{Since, Thread, Workspace, thread_shown};
use crate::settings::SidebarStyle;

/// Zeron's card: 6 px above and below, 2 between its lines.
const CARD_H: f32 = 66.;
const CARD_TWO_LINES_H: f32 = 48.;
const LINE_H: f32 = 16.;
/// T3 Code's shelf heading, `h-8`.
const SHELF_H: f32 = 32.;
const SHELF_WORKING: &str = "shelf:working";
const SHELF_SETTLED: &str = "shelf:settled";

/// A status as a card says it, with T3 Code's icon for it (its `TOP_STATUS_ICONS`); what needs no word shows only its
/// time.
fn status_mark(status: AgentStatus) -> Option<(&'static str, &'static str)> {
    match status {
        AgentStatus::Working => Some(("icons/lucide-circle-dashed.svg", "Working")),
        AgentStatus::Blocked => Some(("icons/lucide-message-circle-question.svg", "Input")),
        AgentStatus::Done => Some(("icons/lucide-circle-check.svg", "Done")),
        AgentStatus::Idle | AgentStatus::Unknown => None,
    }
}

impl Shell {
    /// The rows of the sidebar chosen in General settings; `shown_tab` and `live` as [`Self::workspace_rows`] has them.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn sidebar_rows(
        &self,
        theme: &Theme,
        snapshot: Option<&Snapshot>,
        shown_ws: &Option<String>,
        shown_tab: Option<&str>,
        live: bool,
        active: bool,
        cx: &mut Context<Self>,
    ) -> Vec<(String, gpui::AnyElement)> {
        match SettingsGlobal::of(cx).settings.sidebar_style {
            SidebarStyle::Herdr => self.workspace_rows(theme, snapshot, shown_ws, live, active, cx),
            SidebarStyle::T3 => self.thread_list(theme, snapshot, shown_ws.as_deref(), shown_tab, live, active, cx),
        }
    }

    /// T3 Code's bar over the list: the search, then the project select, a new workspace and a new tab. A selected
    /// project's icon stands in for the filter's. Only the T3 Code sidebar has one; a swipe's preview draws it inert.
    pub(super) fn thread_bar(&self, theme: &Theme, snapshot: Option<&Snapshot>, live: bool, cx: &mut Context<Self>) -> Option<Div> {
        let settings = &SettingsGlobal::of(cx).settings;
        if settings.sidebar_style != SidebarStyle::T3 {
            return None;
        }
        let project = settings.thread_project.clone().and_then(|id| crate::model::groups(snapshot?.workspace_tree()).into_iter().find(|(w, _)| w.workspace_id == id).map(|(w, _)| w));
        let hover = theme.hover;
        let glyph = |icon: &'static str| gpui::svg().path(icon).size(px(16.)).flex_none().text_color(theme.text_muted);
        let filter = project.map_or_else(|| glyph("icons/lucide-list-filter.svg").into_any_element(), |w| project_art(w, snapshot, 16., false, theme, cx));
        let tip_text = project.map_or_else(|| "Filter by project".to_string(), |w| format!("Showing {}", w.label));
        let search = self.thread_search.0.clone();
        // from the theme of the moment: the field outlives a change of it
        search.update(cx, |input, _| input.set_editor_style(crate::ui::input_style(theme)));
        let field = div()
            .flex_1()
            .min_w_0()
            .h(px(32.))
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(8.))
            .rounded(px(6.))
            .hover(move |s| s.bg(hover))
            .child(glyph("icons/lucide-search.svg"))
            .map(|el| match live {
                true => el
                    // Escape clears it and hands the keys back to the pane
                    .capture_key_down(cx.listener(|this, e: &gpui::KeyDownEvent, window, cx| {
                        if e.keystroke.key == "escape" {
                            cx.stop_propagation();
                            this.thread_search.0.update(cx, |input, cx| input.set_value("", window, cx));
                            this.focused = None;
                            cx.notify();
                        }
                    }))
                    .child(div().flex_1().min_w_0().child(Input::new(&search))),
                false => el.text_color(theme.text_faint).child("Search"),
            });
        let button = |id: &'static str, glyph: gpui::AnyElement| titlebar_button_with(theme, id, glyph).size(px(28.)).flex_none();
        let bar = div().flex_none().flex().items_center().gap(px(2.)).px(px(8.)).pb(px(4.)).child(field);
        if !live {
            return Some(bar.child(button("thread-filter", filter)).child(button("thread-new-project", glyph("icons/lucide-folder-plus.svg").into_any_element())).child(button("thread-new", glyph("icons/lucide-square-pen.svg").into_any_element())));
        }
        Some(
            bar.child(
                button("thread-filter", filter)
                    .tooltip(tip(tip_text, None))
                    .when(self.project_menu.as_ref().is_some_and(|m| m.is_open()), |el| el.bg(theme.hover))
                    // capture, like the sidebar's `…`: a press on the open menu's trigger closes it
                    .capture_any_mouse_down(cx.listener(|this, e: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        match this.project_menu.as_mut() {
                            Some(menu) if menu.is_open() => menu.close(),
                            _ => this.project_menu = Some(crate::ui::Popup::new(e.position)),
                        }
                        cx.notify();
                    })),
            )
            .child(
                button("thread-new-project", glyph("icons/lucide-folder-plus.svg").into_any_element())
                    .tooltip(tip("New workspace", None))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.new_workspace(cx))),
            )
            .child(
                button("thread-new", glyph("icons/lucide-square-pen.svg").into_any_element())
                    .tooltip(tip("New tab", Some("NewTab")))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        if let Some(id) = this.shown_workspace.clone() {
                            this.new_tab(id, cx);
                        }
                    })),
            ),
        )
    }

    /// The project select's list: all of them, then each project with a check on the current one.
    pub(super) fn project_menu(&self, theme: &Theme, snapshot: Option<&Snapshot>, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let menu = self.project_menu.as_ref()?;
        let at = menu.value;
        let current = SettingsGlobal::of(cx).settings.thread_project.clone();
        let projects: Vec<&Workspace> = snapshot.map(|s| crate::model::groups(s.workspace_tree()).into_iter().map(|(w, _)| w).collect()).unwrap_or_default();
        let shown = projects.iter().any(|w| current.as_deref() == Some(w.workspace_id.as_str()));
        let choice = |id: SharedString, art: gpui::AnyElement, label: String, checked: bool, pick: Option<String>, cx: &mut Context<Self>| {
            row(theme, false, id)
                .h(px(28.))
                .rounded(px(6.))
                .child(art)
                .child(div().flex_1().min_w_0().truncate().child(label))
                .when(checked, |el| el.child(gpui::svg().path("icons/check.svg").size(px(14.)).flex_none().text_color(theme.text_muted)))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    if let Some(m) = this.project_menu.as_mut() { m.close() }
                    let pick = pick.clone();
                    save_from_sidebar(cx, |s| s.thread_project = pick);
                    cx.notify();
                }))
        };
        let folder = gpui::svg().path("icons/folder.svg").size(px(14.)).flex_none().text_color(theme.text_muted).into_any_element();
        let all = choice("project-menu-all".into(), folder, "All projects".into(), !shown, None, cx);
        let rows: Vec<_> = projects
            .iter()
            .map(|w| {
                let art = project_art(w, snapshot, 14., false, theme, cx);
                choice(SharedString::from(format!("project-menu-{}", w.workspace_id)), art, w.label.clone(), current.as_deref() == Some(w.workspace_id.as_str()), Some(w.workspace_id.clone()), cx)
            })
            .collect();
        Some(
            gpui::deferred(gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(crate::ui::menu_motion(
                "project-menu-in",
                menu,
                menu_card(theme, "project-menu", 220.)
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                        if let Some(m) = this.project_menu.as_mut() { m.close() }
                        cx.notify();
                    }))
                    .child(all)
                    .children(rows),
            )))
            .with_priority(1),
        )
    }

    /// Worktrees being made, the agents that want you, then the shelves, pushed down to the bottom of the column.
    #[allow(clippy::too_many_arguments)]
    fn thread_list(&self, theme: &Theme, snapshot: Option<&Snapshot>, shown_ws: Option<&str>, shown_tab: Option<&str>, live: bool, active: bool, cx: &mut Context<Self>) -> Vec<(String, gpui::AnyElement)> {
        let Some(snapshot) = snapshot else { return Vec::new() };
        let project = SettingsGlobal::of(cx).settings.thread_project.clone();
        let mut shelves = snapshot.shelves(project.as_deref());
        let query = if live { self.thread_search.0.read(cx).value().trim().to_lowercase() } else { String::new() };
        for shelf in [&mut shelves.active, &mut shelves.working, &mut shelves.settled] {
            shelf.retain(|t| snapshot.thread_matches(*t, &query));
        }
        let (main, working) = (shelves.active.len(), shelves.working.len());
        let threads: Vec<Thread> = shelves.active.into_iter().chain(shelves.working).chain(shelves.settled).collect();
        // a swipe's preview of another place: its waits are not tracked here
        let none = Since::default();
        let since = if live { &self.since } else { &none };
        let mut cards: Vec<(gpui::AnyElement, f32)> = (0..threads.len())
            .map(|i| {
                let (card, height) = self.thread_card(snapshot, &threads, i, thread_shown(&threads, i, shown_ws, shown_tab), since, live, active, theme, cx);
                (card.into_any_element(), height)
            })
            .collect();
        let settled_cards = cards.split_off(main + working);
        let working_cards = cards.split_off(main);
        let pending = crate::model::groups(snapshot.workspace_tree())
            .into_iter()
            .filter(|(w, _)| project.as_deref().is_none_or(|id| id == w.workspace_id) || !snapshot.workspaces.iter().any(|o| project.as_deref() == Some(o.workspace_id.as_str())))
            .flat_map(|(w, kids)| self.pending_in(w, &kids, live))
            .map(|b| (format!("pending-{b}"), pending_worktree_row(theme, &b).into_any_element()));
        let mut rows: Vec<(String, gpui::AnyElement)> = pending.collect();
        if threads.is_empty() && !query.is_empty() {
            rows.push(("no-threads".into(), div().px(px(8.)).py(px(24.)).flex().justify_center().text_size(px(12.)).text_color(theme.text_faint).child("No threads found").into_any_element()));
        }
        rows.extend(cards.into_iter().enumerate().map(|(i, (card, _))| (format!("card-{i}"), card)));
        rows.push(("shelves".into(), div().flex_grow(1.).min_h(px(8.)).into_any_element()));
        rows.extend(self.shelf("Working", SHELF_WORKING, working_cards, live, theme, cx));
        rows.extend(self.shelf("Settled", SHELF_SETTLED, settled_cards, live, theme, cx));
        rows
    }

    /// T3 Code's shelf: a heading with a rule, which folds its cards away, and none at all while it is empty.
    fn shelf(&self, label: &'static str, key: &'static str, cards: Vec<(gpui::AnyElement, f32)>, live: bool, theme: &Theme, cx: &mut Context<Self>) -> Option<(String, gpui::AnyElement)> {
        (!cards.is_empty()).then(|| {
            let open = !self.folded.contains(key);
            let full: f32 = cards.iter().map(|(_, h)| h + 2.).sum();
            let tween = self.fold_motion.get(key).copied().filter(|m| live && m.animating());
            let title = if open { label.to_string() } else { format!("{label} ({})", cards.len()) };
            let head = div()
                .id(key)
                .flex_none()
                .h(px(SHELF_H))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(12.))
                .text_color(theme.text_faint)
                .cursor_pointer()
                .child(title)
                .child(div().flex_1().h(px(1.)).bg(theme.border))
                .child(fold_chevron(theme, key, open, full, tween))
                .when(live, |el| {
                    el.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.toggle_fold(key, full, cx);
                        cx.notify();
                    }))
                });
            let body = div().w_full().flex_none().overflow_hidden().flex().flex_col().pt(px(2.)).gap(px(2.)).children(cards.into_iter().map(|(card, _)| card));
            (key.to_string(), div().flex().flex_col().flex_none().child(head).child(fold_body(body, key, open, full, tween)).into_any_element())
        })
    }

    /// Goes to a card: its agent, or without one its workspace's `tab`.
    fn show_thread(&mut self, tab: Option<String>, agent: Option<(String, String)>, cx: &mut Context<Self>) {
        match agent {
            Some((tab, pane)) => self.show_agent(tab, pane, cx),
            None => {
                self.select_tab(tab);
                cx.notify();
            }
        }
    }

    /// Card `i` of the list and its height: Zeron's `render_chat_row` on an agent or a workspace. Its right-click menu
    /// is its workspace's, which is where a worktree is merged or removed.
    #[allow(clippy::too_many_arguments)]
    fn thread_card(&self, snapshot: &Snapshot, threads: &[Thread], i: usize, selected: bool, since: &Since, live: bool, active: bool, theme: &Theme, cx: &mut Context<Self>) -> (Stateful<Div>, f32) {
        let (w, pane) = threads[i];
        let id = w.workspace_id.clone();
        let (show, compact) = {
            let settings = &SettingsGlobal::of(cx).settings;
            (settings.workspace_show, settings.compact_workspaces)
        };
        let project = snapshot.project_of(&id).unwrap_or(w);
        let worktree = w.worktree.as_ref().filter(|t| t.is_linked_worktree);
        let dir = super::sidebar::workspace_dir(w, Some(snapshot));
        let branch = dir.as_deref().and_then(|d| crate::ui::statusbar::git_of(d, cx)).map(|g| g.branch);
        // a project's own checkout is named by the line above: its branch says more
        let title = match pane {
            Some(p) => p.terminal_title_stripped.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| w.label.clone()),
            None if project.workspace_id == id => branch.clone().unwrap_or_else(|| w.label.clone()),
            None => w.label.clone(),
        };
        let branch = branch.filter(|b| show.branch && (pane.is_some() || *b != title));

        // the first line: the project, and its agent's status with how long it has had it
        let art = show.icon.then(|| project_art(project, Some(snapshot), 14., selected, theme, cx));
        let status = self.busy_mark(w, live, theme, cx).or_else(|| {
            let pane = pane?;
            let mark = status_mark(pane.agent_status);
            let time = since.get(&pane.pane_id).filter(|(status, _)| *status == pane.agent_status).map(|(_, at)| crate::model::ago(at.elapsed()));
            (mark.is_some() || time.is_some()).then(|| {
                let color = status_color(pane.agent_status, theme).unwrap_or(theme.text_faint);
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_color(color)
                    .children(mark.filter(|_| show.status).map(|(icon, _)| gpui::svg().path(icon).size(px(14.)).flex_none().text_color(color)))
                    .children(mark.map(|(_, word)| div().font_weight(gpui::FontWeight::MEDIUM).child(word)))
                    .children(time)
                    .into_any_element()
            })
        });
        let top = div()
            .w_full()
            .h(px(LINE_H))
            .flex()
            .items_center()
            .gap(px(6.))
            .text_size(px(12.))
            .text_color(theme.text_faint)
            .children(art)
            .child(div().flex_1().min_w_0().truncate().child(project.label.clone()))
            .children(status);

        // the title, or what stands in for it
        let first = threads.iter().position(|(o, _)| o.workspace_id == id) == Some(i);
        let editing = (live && first).then(|| self.rename_field(&RenameTarget::Workspace(id.clone()), cx)).flatten();
        let merging = self.merging.get(&id).filter(|_| live);
        let name = match (editing, merging) {
            (Some(field), _) => field.into_any_element(),
            (None, Some(m)) => div().flex_1().min_w_0().truncate().when(m.failed.is_some(), |el| el.text_color(theme.danger)).child(m.step().to_string()).into_any_element(),
            (None, None) => div().flex_1().min_w_0().truncate().font_weight(gpui::FontWeight::MEDIUM).child(title.clone()).into_any_element(),
        };

        // the last line: the branch with what it changed, and the agent at the end; without one, T3 Code's `titleLineIcons`
        let (changes, pr) = match dir.as_deref().filter(|_| worktree.is_some() && !compact) {
            Some(d) => (crate::ui::statusbar::changes_of(d, active, cx).0, crate::ui::statusbar::pr_of(d, active, cx)),
            None => (None, None),
        };
        let changes = changes.filter(|c| show.changes && *c != (0, 0)).map(|(added, removed)| {
            div()
                .flex_none()
                .flex()
                .gap(px(3.))
                .when(added > 0, |el| el.child(div().text_color(theme.success.opacity(0.7)).child(format!("+{added}"))))
                .when(removed > 0, |el| el.child(div().text_color(theme.danger.opacity(0.7)).child(format!("−{removed}"))))
        });
        let tab = w.active_tab_id.clone().or_else(|| snapshot.tabs_of(&id).first().map(|t| t.tab_id.clone()));
        let agent = pane.map(|p| (p.tab_id.clone(), p.pane_id.clone()));
        // Zeron's `pull_request_badge`: a click goes to the card and opens its PR in the panel
        let pr = pr.filter(|_| show.pull_request).map(|pr| {
            let (_, color) = super::review::pr_tone(pr.state, theme);
            div()
                .id("thread-pr")
                .h(px(LINE_H))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(3.))
                .px(px(4.))
                .rounded(px(4.))
                .bg(color.opacity(0.08))
                .text_size(px(10.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(color.opacity(0.85))
                .child(gpui::svg().path("icons/git-pull-request.svg").size(px(10.)).flex_none().text_color(color.opacity(0.85)))
                .child(pr.number.to_string())
                .when(live, |el| {
                    let (id, tab, agent) = (id.clone(), tab.clone(), agent.clone());
                    el.cursor_pointer().hover(move |s| s.bg(color.opacity(0.16)).text_color(color)).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        this.show_thread(tab.clone(), agent.clone(), cx);
                        this.right.panels.entry(id.clone()).or_default().add(crate::right_panel::Surface::PullRequest);
                    }))
                })
        });
        let mark = pane.map(|p| {
            let icon = crate::ui::command_panel::agent_icon(p.agent.as_deref().unwrap_or_default());
            gpui::svg().path(icon).size(px(14.)).flex_none().text_color(crate::ui::command_panel::icon_ink(icon, theme.text_muted))
        });
        let has_last = !compact && (branch.is_some() || changes.is_some() || pr.is_some());
        let (last_mark, title_mark) = if has_last { (mark, None) } else { (None, mark) };
        // T3 Code's `ThreadWorktreeIndicator`, in place of the branch mark
        let branch_mark = branch.as_ref().map(|b| match worktree {
            Some(t) => {
                let text = format!("Worktree: {} ({b})", crate::ui::statusbar::short_path(&t.checkout_path, std::env::home_dir().as_deref()));
                div()
                    .id("thread-worktree")
                    .flex_none()
                    .child(gpui::svg().path("icons/lucide-folder-git-2.svg").size(px(12.)).text_color(theme.text_faint))
                    .when(live, |el| el.tooltip(tip(text, None)))
                    .into_any_element()
            }
            None => gpui::svg().path("icons/git-branch.svg").size(px(11.)).flex_none().text_color(theme.text_faint).into_any_element(),
        });
        let last = has_last.then(|| {
            div()
                .w_full()
                .h(px(LINE_H))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(11.))
                .text_color(theme.text_faint)
                .children(branch_mark)
                .child(div().flex_1().min_w_0().truncate().children(branch))
                .children(changes)
                .children(pr)
                .children(last_mark)
        });
        let middle = div().w_full().flex().items_center().gap(px(6.)).child(name).children(title_mark);

        let height = if last.is_some() { CARD_H } else { CARD_TWO_LINES_H };
        let menu_on = live && self.ws_menu.as_ref().is_some_and(|m| m.is_open() && m.value.0 == id);
        let label = w.label.clone();
        let card = row(theme, selected, SharedString::from(format!("thread-{}", pane.map_or(id.as_str(), |p| p.pane_id.as_str()))))
            .h(px(height))
            .flex_col()
            .items_start()
            .justify_center()
            .gap(px(2.))
            .text_color(if selected { theme.text } else { theme.text.opacity(0.8) })
            .when(menu_on && !selected, |el| el.bg(theme.hover))
            .when(live, |el| pane_drag::accept_pane_in_workspace(el, &id, theme.accent.opacity(0.15), cx))
            .child(top)
            .child(middle)
            .children(last)
            .when(live, |el| {
                el.on_click(cx.listener({
                    let id = id.clone();
                    move |this, e: &ClickEvent, window, cx| match agent.is_none() && e.click_count() == 2 {
                        true => this.start_rename(RenameTarget::Workspace(id.clone()), label.clone(), window, cx),
                        false => this.show_thread(tab.clone(), agent.clone(), cx),
                    }
                }))
                .on_mouse_down(MouseButton::Right, cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                    this.ws_menu = Some(crate::ui::Popup::new((id.clone(), e.position)));
                    cx.notify();
                }))
            });
        (card, height)
    }
}

/// The search field, for the window's life so a query outlives a look at the settings.
pub(super) fn search_field(window: &mut Window, cx: &mut Context<Shell>) -> (Entity<InputState>, gpui::Subscription) {
    let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search"));
    let events = cx.subscribe_in(&input, window, |_, _, event: &InputEvent, _, cx| {
        if let InputEvent::Change = event {
            cx.notify();
        }
    });
    (input, events)
}

/// A project's artwork or monogram, as the workspace list draws it.
fn project_art(project: &Workspace, snapshot: Option<&Snapshot>, size: f32, selected: bool, theme: &Theme, cx: &mut Context<Shell>) -> gpui::AnyElement {
    let seed = project.worktree.as_ref().map_or(project.label.as_str(), |t| t.repo_key.as_str());
    let dir = super::sidebar::workspace_dir(project, snapshot);
    crate::ui::project_icon::project_icon(&project.label, seed, dir.as_deref(), size, selected, theme, cx)
}
