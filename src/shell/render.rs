//! The window's frame of views: titlebar, sidebar, tab bar, panes, panels and toasts, laid out each frame.
use super::*;

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        for image in self.retired_images.drain(..) {
            let _ = window.drop_image(image);
        }
        self.sync_badge(window, cx);
        self.mark_seen(window, cx);
        // any Shepherd window in front, not this one: a window in the back renders too
        secure_input(SettingsGlobal::of(cx).settings.secure_input && cx.active_window().is_some());
        crate::ui::reap_popup(&mut self.view_menu, window);
        crate::ui::reap_popup(&mut self.agents_menu, window);
        crate::ui::reap_popup(&mut self.ws_menu, window);
        crate::ui::reap_popup(&mut self.web_menu, window);
        crate::ui::reap_popup(&mut self.new_tab_menu, window);
        crate::ui::reap_popup(&mut self.pane_menu, window);
        crate::ui::reap_popup(&mut self.spent_menu, window);
        crate::ui::reap_popup(&mut self.usage_menu, window);
        crate::ui::reap_popup(&mut self.statusbar_menu, window);
        crate::ui::reap_popup(&mut self.tab_menu, window);
        crate::ui::reap_popup(&mut self.worktree_dialog, window);
        crate::ui::reap_popup(&mut self.remove_confirm, window);
        crate::ui::reap_popup(&mut self.merge_confirm, window);
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
        self.follow_shown_tab();
        let (mut frames, mut splits) = match (&snapshot, &tab) {
            (Some(s), Some(t)) => (s.pane_frames(&t.tab_id), s.split_frames(&t.tab_id)),
            _ => (Vec::new(), Vec::new()),
        };
        // herdr caught up with a released divider: its snapshot is the layout again.
        if let Some((path, ratio)) = &self.settling
            && splits.iter().any(|s| &s.path == path && (s.ratio - ratio).abs() < 0.02)
        {
            self.settling = None;
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
        for (_, pane) in self.panes.iter().filter(|(id, _)| !self.recent_panes.contains(id)) {
            self.retired_images.extend(pane.update(cx, |pane, cx| pane.set_images(Vec::new(), cx)));
        }
        prune(&mut self.panes, &mut self.focused, &self.recent_panes, |_| false);
        if std::mem::take(&mut self.popup_closed) && !self.panes.values().any(|p| p.read(cx).focus_handle(cx).is_focused(window)) {
            self.focused = None;
        }
        let overlaid = self.settings.is_some() || self.command_panel.as_ref().is_some_and(|p| p.is_open()) || self.floating_open();
        for f in &shown {
            if !self.panes.contains_key(&f.pane_id) {
                let id = f.pane_id.clone();
                let images = self.pane_images.get(&id).cloned().unwrap_or_default();
                self.panes.insert(id.clone(), cx.new(|cx| {
                    let mut pane = PaneTerminal::new(id, cx);
                    pane.set_images(images, cx);
                    pane
                }));
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
        let peek = self.peek_now();
        if sliding || self.sidebar_nudge().is_some() || self.sidebar_peek.is_some_and(|m| m.animating()) {
            window.request_animation_frame();
        }

        let covered = self.overlay_open(cx);
        self.sync_pages(covered, window, cx);
        let web = self.shown_page(cx);
        // a browser tab sits among the workspace's tabs; a sidebar page in place of them: no workspace looks selected
        let browsing = web.as_ref().is_some_and(|(key, _)| browser::is_tab(&key.0));
        let sidebar_ws = if web.is_some() && !browsing { None } else { shown_ws.clone() };
        let sidebar = self.render_sidebar(&theme, &snapshot, &sidebar_ws, &shown_tab, window, cx);
        let (docked, peeking) = if sidebar_w > 0. { (Some(sidebar), None) } else { (None, Some(sidebar)) };

        let tabbar = self.render_tab_bar(&theme, shown_ws.clone(), if browsing { self.pages.shown.clone() } else { shown_tab.clone() }, sidebar_w, window, cx);
        let (tabbar, web_area) = match &web {
            Some((key, _)) if browsing => {
                let bar = self.render_browser_bar(key, &theme, window, cx);
                (tabbar, Some(div().flex_1().min_h_0().flex().flex_col().child(bar).child(self.render_web_area(key, &theme, window, cx))))
            }
            Some((key, entry)) => (self.render_web_bar(key, entry, &theme, sidebar_w, cx), Some(self.render_web_area(key, &theme, window, cx))),
            None => (tabbar, None),
        };

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
                            // cached: a frame the pane did not cause (a spinner, a hover elsewhere) reuses its grid
                            div().size_full().relative().child(pane.clone().cached(gpui::StyleRefinement::default().size_full())).when(focused, |el| {
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

        let show_bar = SettingsGlobal::of(cx).settings.show_status_bar;
        let shown = if show_bar || self.review_open { self.shown_review(snapshot.as_ref(), window, cx) } else { Default::default() };
        let statusbar = show_bar.then(|| {
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
            let on_review = cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_review(cx));
            let show = SettingsGlobal::of(cx).settings.status_bar_show;
            div()
                .flex_none()
                .on_mouse_down(MouseButton::Right, cx.listener(|this, e: &MouseDownEvent, _, cx| {
                    this.statusbar_menu = Some(crate::ui::Popup::new(e.position));
                    cx.notify();
                }))
                .child(crate::ui::statusbar::bar(show, snapshot.as_ref(), pane.as_deref(), shown.review.as_ref(), &theme, open, self.review_open, on_branch, on_spent, on_usage, on_review, window, cx))
        });
        let spring_back = self.pane_spring_back(window);
        let settings_page = self.settings.as_ref().map(|(view, _)| view.clone());
        let on_page = settings_page.is_some();
        // nothing to show outside a repo with a remote, and no PR label to close it from: it waits for one
        let has_remote = shown.key.is_some() && !matches!(shown.review, Some(crate::ui::review::Review::NoRemote));
        let review = (self.review_open && !on_page && has_remote).then(|| self.render_review(&theme, &shown, window, cx));
        if review.is_none() {
            // a hidden panel's selection is not Cmd+C's to copy
            crate::ui::markdown::forget("review-");
        }
        let root = div()
            .id("root")
            .track_focus(&self.focus)
            .capture_key_down(cx.listener(|s, e: &gpui::KeyDownEvent, _, cx| {
                s.touch(cx);
                // Cmd+C (Ctrl+C off macOS) over a selection in the review panel copies it, not the pane's
                let m = &e.keystroke.modifiers;
                let copy = if cfg!(target_os = "macos") { m.platform && !(m.shift || m.alt || m.control) } else { m.control && !(m.alt || m.platform) };
                if copy
                    && e.keystroke.key == "c"
                    && let Some(text) = crate::ui::markdown::selected_text()
                {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                    cx.stop_propagation();
                }
            }))
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
                if let Some(id) = this.shown_browser_tab().map(|t| t.id.clone()) {
                    return this.close_browser_tab(&id, cx);
                }
                if let Some(tab) = this.shown_tab().filter(|_| this.pages.shown.is_none()) {
                    this.send(Command::CloseTab { tab_id: tab.tab_id }, cx, |_, _, _| {});
                }
            }))
            .on_action(cx.listener(|this, _: &NewWorkspace, _, cx| this.new_workspace(cx)))
            .on_action(cx.listener(|this, _: &SplitRight, w, cx| this.pane_command(w, cx, |pane_id| Command::Split { pane_id, down: false })))
            .on_action(cx.listener(|this, _: &SplitDown, w, cx| this.pane_command(w, cx, |pane_id| Command::Split { pane_id, down: true })))
            .on_action(cx.listener(|this, _: &ClosePane, w, cx| {
                // a browser tab is its own one pane, as in Safari
                if let Some(id) = this.shown_browser_tab().map(|t| t.id.clone()) {
                    return this.close_browser_tab(&id, cx);
                }
                this.pane_command(w, cx, |pane_id| Command::ClosePane { pane_id })
            }))
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
            .on_action(cx.listener(|this, _: &PreviousTab, w, cx| this.step_tab(false, w, cx)))
            .on_action(cx.listener(|this, _: &NextTab, w, cx| this.step_tab(true, w, cx)))
            .on_action(cx.listener(|this, _: &PreviousWorkspace, _, cx| this.step_workspace(false, cx)))
            .on_action(cx.listener(|this, _: &NextWorkspace, _, cx| this.step_workspace(true, cx)))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| this.toggle_sidebar(cx)))
            .on_action(cx.listener(|this, _: &ToggleReview, _, cx| this.toggle_review(cx)))
            .on_action(cx.listener(|this, _: &ReloadPage, _, cx| this.reload_shown_page(cx)))
            .on_action(cx.listener(|this, _: &FocusAddress, w, cx| this.focus_address(w, cx)))
            .on_action(cx.listener(|this, _: &CopyPageUrl, _, cx| this.copy_page_url(cx)))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| this.toggle_settings_here(window, cx)))
            .on_action(cx.listener(|this, _: &CommandPanel, w, cx| this.toggle_command_panel(crate::ui::command_panel::Kind::Commands, w, cx)))
            .on_action(cx.listener(|this, _: &ChangeTheme, w, cx| this.toggle_command_panel(crate::ui::command_panel::Kind::Themes, w, cx)))
            .on_action(cx.listener(|this, _: &AgentsPanel, w, cx| this.toggle_command_panel(crate::ui::command_panel::Kind::Agents, w, cx)))
            .on_action(cx.listener(|this, _: &WorkspacesPanel, w, cx| this.toggle_command_panel(crate::ui::command_panel::Kind::Workspaces, w, cx)))
            .on_action(cx.listener(|this, c: &crate::herdr::keys::HerdrCommand, w, cx| this.run_herdr_command(c, w, cx)))
            .on_mouse_move(cx.listener(|this, e: &MouseMoveEvent, w, cx| {
                this.drag_divider(e.position, cx);
                this.drag_sidebar(e.position.x, cx);
                this.drag_review(e.position.x, w, cx);
                this.drag_agents(e.position.y, w, cx);
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.end_divider(cx);
                this.end_sidebar_drag(cx);
                this.end_review_drag(cx);
                this.end_agents_drag(cx);
            }))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.end_divider(cx);
                this.end_sidebar_drag(cx);
                this.end_review_drag(cx);
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
                        .children(docked),
                )
            })
            .map(|el| match settings_page {
                Some(page) => el.child(page),
                None => el.child(div().flex_1().min_w_0().h_full().flex().flex_col().child(tabbar).map(|el| match web_area {
                    Some(area) => el.child(area),
                    None => el.child(grid),
                }).children(statusbar)),
            })
            .children(review.into_iter().flatten())
            // The sidebar closed: the window's left edge brings it out floating over the panes, as Arc's does.
            .when(!self.sidebar_open && sidebar_w == 0. && !on_page, |el| {
                el.child(
                    div()
                        .id("sidebar-peek-edge")
                        .absolute()
                        .top(px(TITLEBAR_HEIGHT))
                        .bottom_0()
                        .left_0()
                        .w(px(6.))
                        .on_hover(cx.listener(|this, on: &bool, _, cx| this.hover_peek(*on, cx))),
                )
                // deferred: drawn above a web page, which sits above the rest
                .when(peek > 0., |el| {
                    el.child(gpui::deferred(
                        div()
                            .id("sidebar-peek")
                            .occlude()
                            .absolute()
                            .top(px(6.))
                            .bottom(px(6.))
                            .left(px(6. - (self.sidebar_width + 12.) * (1. - peek)))
                            .w(px(self.sidebar_width))
                            .opacity(peek)
                            .overflow_hidden()
                            .rounded(px(10.))
                            .bg(theme.card)
                            .border_1()
                            .border_color(theme.border)
                            .shadow_lg()
                            .on_hover(cx.listener(|this, on: &bool, _, cx| this.hover_peek(*on, cx)))
                            .children(peeking),
                    ))
                })
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
            .when(window_buttons(cx), |el| el.children(window_controls(&theme, window)))
            .children(resize_edges(window))
            .child(toasts)
    }
}
