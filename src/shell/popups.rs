//! The popups over the window: the sidebar's and agents' menus, workspace, tab and pane menus, dialogs, the
//! command panel and the status bar's cards, each while it is up.
use super::*;

impl Shell {
    pub(super) fn popups(
        &mut self,
        root: Stateful<Div>,
        theme: &Theme,
        snapshot: &Option<Snapshot>,
        sidebar_w: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        root
        .children(self.view_menu.as_ref().map(|menu| {
            // under the `…` button, its left edge on the button's
            let at = gpui::point(px(sidebar_w - 8. - 24.), px(TRAFFIC_LIGHTS_CENTER + 12. + 4.));
            let settings = &SettingsGlobal::of(cx).settings;
            let (show, compact) = (settings.workspace_show, settings.compact_workspaces);
            let show_submenu = self.show_submenu;
            let submenu_bounds = self.show_submenu_bounds.clone();
            let parts: [(&'static str, &'static str, &'static str, fn(&mut crate::settings::WorkspaceShow) -> &mut bool); 5] = [
                ("show-icon", "icons/folder.svg", "Project icon", |s| &mut s.icon),
                ("show-branch", "icons/git-branch.svg", "Branch", |s| &mut s.branch),
                ("show-changes", "icons/split-columns.svg", "Changes", |s| &mut s.changes),
                ("show-pr", "icons/arrow-up.svg", "Pull request", |s| &mut s.pull_request),
                ("show-status", "icons/bot.svg", "Agent status", |s| &mut s.status),
            ];
            gpui::deferred(
                gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(crate::ui::menu_motion(
                    "view-menu-in",
                    menu,
                    menu_card(theme, "view-menu", 200.)
                        .on_mouse_down_out(cx.listener(|this, e: &MouseDownEvent, _, cx| {
                            if this.show_submenu && this.show_submenu_bounds.get().is_some_and(|b| b.contains(&e.position)) {
                                return;
                            }
                            if let Some(m) = this.view_menu.as_mut() { m.close() }
                            cx.notify();
                        }))
                        .child(
                            // Zeron's submenu: the row opens the choices beside it on hover.
                            div()
                                .relative()
                                .child(
                                    row(theme, show_submenu, "workspace-show")
                                        .h(px(30.))
                                        .rounded(px(6.))
                                        .child(div().flex_1().child("Show"))
                                        .child(gpui::svg().path("icons/alt-arrow-right.svg").size(px(12.)).flex_none().text_color(theme.text_faint))
                                        .on_hover(cx.listener(|this, on: &bool, _, cx| {
                                            if *on && !this.show_submenu {
                                                this.show_submenu = true;
                                                cx.notify();
                                            }
                                        }))
                                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                            this.show_submenu = !this.show_submenu;
                                            cx.notify();
                                        })),
                                )
                                .when(show_submenu, |el| {
                                    el.child(
                                        menu_card(theme, "show-submenu", 180.)
                                            .absolute()
                                            .top(px(-4.))
                                            .left(gpui::relative(1.))
                                            .ml(px(8.))
                                            .child(
                                                canvas(move |b, _, _| submenu_bounds.set(Some(b)), |_, _, _, _| {})
                                                    .absolute()
                                                    .top_0()
                                                    .left_0()
                                                    .size_full(),
                                            )
                                            .child(div().px(px(8.)).pt(px(4.)).pb(px(2.)).text_size(px(11.)).text_color(theme.text_faint).child("SHOW"))
                                            .children(parts.map(|(id, icon, label, part)| {
                                                let mut shown = show;
                                                menu_choice(theme, id, icon, label, *part(&mut shown)).on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                                                    save_from_sidebar(cx, |s| {
                                                        let on = part(&mut s.workspace_show);
                                                        *on = !*on;
                                                    })
                                                }))
                                            })),
                                    )
                                }),
                        )
                        .child(div().mx(px(-4.)).my(px(4.)).h(px(1.)).bg(theme.border))
                        .child(
                            row(theme, false, "compact-workspaces")
                                .on_hover(cx.listener(|this, on: &bool, _, cx| {
                                    if *on && this.show_submenu {
                                        this.show_submenu = false;
                                        cx.notify();
                                    }
                                }))
                                .h(px(30.))
                                .rounded(px(6.))
                                .child(div().flex_1().child("Compact"))
                                .child(crate::ui::switch(theme, compact, "compact-workspaces"))
                                .on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                                    save_from_sidebar(cx, |s| s.compact_workspaces = !s.compact_workspaces)
                                })),
                        ),
                )),
            )
            .with_priority(1)
        }))
        .children(self.agents_menu.as_ref().map(|menu| {
            let at = menu.value;
            let sort = self.agent_sort;
            let compact = SettingsGlobal::of(cx).settings.compact_agents;
            let attention = SettingsGlobal::of(cx).settings.attention_agents;
            gpui::deferred(
                gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(crate::ui::menu_motion(
                    "agents-menu-in",
                    menu,
                    menu_card(theme, "agents-menu", 200.)
                        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            if let Some(m) = this.agents_menu.as_mut() { m.close() }
                            cx.notify();
                        }))
                        .child(div().px(px(8.)).pt(px(4.)).pb(px(2.)).text_size(px(11.)).text_color(theme.text_faint).child("SORT"))
                        .children(SORTS.map(|(choice, id, icon, label)| {
                            menu_choice(theme, id, icon, label, sort == choice)
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_agent_sort(choice, cx)))
                        }))
                        .child(div().mx(px(-4.)).my(px(4.)).h(px(1.)).bg(theme.border))
                        .child(
                            row(theme, false, "compact-agents")
                                .h(px(30.))
                                .rounded(px(6.))
                                .child(div().flex_1().child("Compact"))
                                .child(crate::ui::switch(theme, compact, "compact-agents"))
                                .on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                                    save_from_sidebar(cx, |s| s.compact_agents = !s.compact_agents)
                                })),
                        )
                        .child(
                            row(theme, false, "attention-agents")
                                .h(px(30.))
                                .rounded(px(6.))
                                .child(div().flex_1().child("Needs attention only"))
                                .child(crate::ui::switch(theme, attention, "attention-agents"))
                                .on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                                    save_from_sidebar(cx, |s| s.attention_agents = !s.attention_agents)
                                })),
                        ),
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
                    menu_card(theme, "ws-menu", 180.)
                        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            if let Some(m) = this.ws_menu.as_mut() { m.close() }
                            cx.notify();
                        }))
                        .when(has_repo, |el| {
                            el.child(row(theme, false, "ws-menu-worktree").h(px(28.)).rounded(px(6.)).child("New worktree").on_pick(at, 
                                cx.listener({
                                    let id = id.clone();
                                    move |this, _: &ClickEvent, window, cx| {
                                        if let Some(m) = this.ws_menu.as_mut() { m.close() }
                                        this.open_worktree_dialog(id.clone(), window, cx);
                                    }
                                }),
                            ))
                        })
                        .child(row(theme, false, "ws-menu-new-tab").h(px(28.)).rounded(px(6.)).child("New tab").on_pick(at, 
                            cx.listener({
                                let id = id.clone();
                                move |this, _: &ClickEvent, _, cx| {
                                    if let Some(m) = this.ws_menu.as_mut() { m.close() }
                                    this.new_tab(id.clone(), cx);
                                }
                            }),
                        ))
                        .child(row(theme, false, "ws-menu-rename").h(px(28.)).rounded(px(6.)).child("Rename").on_pick(at, 
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
                            el.child(row(theme, false, "ws-menu-reveal").h(px(28.)).rounded(px(6.)).child("Reveal in Finder").on_pick(at, 
                                cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    if let Some(m) = this.ws_menu.as_mut() { m.close() }
                                    cx.reveal_path(std::path::Path::new(&path));
                                    cx.notify();
                                }),
                            ))
                        })
                        .child(div().mx(px(-4.)).my(px(4.)).h(px(1.)).bg(theme.border))
                        .child(
                            row(theme, false, "ws-menu-close")
                                .h(px(28.))
                                .rounded(px(6.))
                                .text_color(theme.danger)
                                .child("Close workspace")
                                .on_pick(at, cx.listener({
                                    let id = id.clone();
                                    move |this, _: &ClickEvent, _, cx| {
                                        if let Some(m) = this.ws_menu.as_mut() { m.close() }
                                        this.send(Command::CloseWorkspace { workspace_id: id.clone() }, cx, |_, _, _| {});
                                    }
                                })),
                        )
                        .when(linked, |el| {
                            el.child(
                                row(theme, false, "ws-menu-remove")
                                    .h(px(28.))
                                    .rounded(px(6.))
                                    .text_color(theme.danger)
                                    .child("Remove worktree")
                                    .on_pick(at, cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        if let Some(m) = this.ws_menu.as_mut() { m.close() }
                                        this.confirm_remove_worktree(id.clone(), false, cx);
                                    })),
                            )
                        }),
                )),
            )
            .with_priority(1)
        }))
        .children(self.worktree_dialog.as_ref().map(|dialog| {
            let card = dialog_card(theme, "worktree-dialog", "New worktree")
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
                            ghost_button(theme, "worktree-cancel", "Cancel")
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
            let (_, label, dirty) = confirm.value.clone();
            let text = if dirty {
                format!("{label} has uncommitted changes. Removing it deletes them; its branch stays.")
            } else {
                format!("{label}'s checkout is deleted and its workspace closed; its branch stays.")
            };
            let card = dialog_card(theme, "remove-dialog", "Remove worktree?")
                .on_mouse_down_out(cx.listener(|this, _, _, cx| this.finish_remove_confirm(false, cx)))
                .child(
                    div()
                        .mt(px(8.))
                        .line_height(px(19.))
                        .text_color(theme.text_muted)
                        .child(text),
                )
                .child(
                    div()
                        .mt(px(16.))
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            ghost_button(theme, "remove-cancel", "Cancel")
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_remove_confirm(false, cx))),
                        )
                        .child(
                            solid_button("remove-force", if dirty { "Remove anyway" } else { "Remove" }, theme.danger, gpui::white())
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
                    menu_card(theme, "tab-menu", 180.)
                        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            if let Some(m) = this.tab_menu.as_mut() { m.close() }
                            cx.notify();
                        }))
                        .child(row(theme, false, "tab-menu-rename").h(px(28.)).rounded(px(6.)).child("Rename").on_pick(at, 
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
                            row(theme, false, "tab-menu-close")
                                .h(px(28.))
                                .rounded(px(6.))
                                .text_color(theme.danger)
                                .child("Close tab")
                                .on_pick(at, cx.listener(move |this, _: &ClickEvent, _, cx| {
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
            let card = crate::ui::statusbar::spent_card(snapshot.as_ref(), shown.as_deref(), theme, self.spent_by_tokens, on_sort, pick, cx).on_mouse_down_out(
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
            let card = crate::ui::statusbar::usage_card(theme, cx).on_mouse_down_out(cx.listener(|this, _, _, cx| {
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
                    menu_card(theme, "pane-menu", 200.)
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
                            let item = row(theme, false, ("pane-menu-item", i))
                                .h(px(28.))
                                .rounded(px(6.))
                                .when(close, |el| el.text_color(theme.danger))
                                .child(div().flex_1().child(label))
                                .when_some(chord, |el, chord| el.child(div().text_size(px(11.)).text_color(theme.text_faint).child(chord)))
                                .on_pick(at, cx.listener(move |this, _: &ClickEvent, _, cx| {
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
