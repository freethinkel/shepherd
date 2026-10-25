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
        let confirming = self.session_delete.as_ref().is_some_and(|c| c.is_open()) || self.remove_confirm.as_ref().is_some_and(|c| c.is_open());
        if confirming && !self.confirm_focus.is_focused(window) {
            window.focus(&self.confirm_focus, cx);
        }
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
        .children(self.place_menu.as_ref().map(|menu| {
            let (place, at) = menu.value.clone();
            let (edit, doomed) = (place.clone(), place.clone());
            gpui::deferred(
                gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(crate::ui::menu_motion(
                    "place-menu-in",
                    menu,
                    menu_card(theme, "place-menu", 160.)
                        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            if let Some(m) = this.place_menu.as_mut() { m.close() }
                            cx.notify();
                        }))
                        .child(row(theme, false, "place-menu-edit").h(px(28.)).rounded(px(6.)).child("Edit…").on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.open_place_edit(edit.clone(), window, cx)
                        })))
                        .when(place.deletable(), |el| {
                            let label = if matches!(place, sessions::Place::Machine(_)) { "Forget…" } else { "Delete…" };
                            el.child(div().mx(px(-4.)).my(px(4.)).h(px(1.)).bg(theme.border)).child(
                                row(theme, false, "place-menu-delete")
                                    .h(px(28.))
                                    .rounded(px(6.))
                                    .text_color(theme.danger)
                                    .child(label)
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        if let Some(m) = this.place_menu.as_mut() { m.close() }
                                        this.confirm_delete(doomed.clone(), cx)
                                    })),
                            )
                        }),
                )),
            )
            .with_priority(1)
        }))
        .children(self.place_edit.as_ref().map(|edit| {
            let field = |input: &Entity<InputState>| {
                div()
                    .mt(px(8.))
                    .h(px(SESSION_FIELD_H))
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.wash(0.04))
                    .text_size(px(14.))
                    .child(div().flex_1().child(Input::new(input)))
            };
            let card = dialog_card(theme, "place-edit", "Edit session")
                .on_mouse_down_out(cx.listener(|this, _, _, cx| this.finish_place_edit(false, cx)))
                .on_key_down(cx.listener(|this, e: &gpui::KeyDownEvent, _, cx| {
                    if e.keystroke.key == "escape" {
                        cx.stop_propagation();
                        this.finish_place_edit(false, cx);
                    }
                }))
                .child(div().mt(px(4.)).text_color(theme.text_muted).child("Name"))
                .child(field(&edit.value.name))
                // Arc's and Zen's picker: drawn icons or emoji, after the name's letter; the picked one lit
                .child({
                    let (picked, emoji) = (edit.value.icon.clone(), edit.value.emoji);
                    let letter = edit.value.name.read(cx).value().trim().chars().next().unwrap_or('?').to_uppercase().to_string();
                    let tab = |id: &'static str, label: &'static str, on: bool| {
                        div()
                            .id(id)
                            .px(px(10.))
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .rounded(px(6.))
                            .cursor_pointer()
                            .text_color(if on { theme.text } else { theme.text_muted })
                            .when(on, |el| el.bg(theme.selected))
                            .child(label)
                    };
                    let weak = cx.entity().downgrade();
                    let grid: gpui::AnyElement = if emoji {
                        // every emoji, found by name, laid out only where the list shows them
                        let rows = Rc::new(sessions::emoji_rows(&edit.value.search.read(cx).value()));
                        let (rows_, picked_) = (rows.clone(), picked.clone());
                        let (selected, hover, faint) = (theme.selected, theme.hover, theme.text_faint);
                        let list = gpui::uniform_list("place-emoji", rows.len(), move |range, _, _| {
                            range
                                .map(|i| match &rows_[i] {
                                    sessions::EmojiRow::Heading(name) => sessions::emoji_heading(name, faint).into_any_element(),
                                    sessions::EmojiRow::Emoji(emoji) => div()
                                        .h(px(36.))
                                        .w_full()
                                        .pr(px(12.))
                                        .flex()
                                        .gap(px(4.))
                                        .children(emoji.iter().map(|e| {
                                            let (weak, e) = (weak.clone(), e.to_string());
                                            let on = picked_.as_deref() == Some(e.as_str());
                                            div()
                                                .id(SharedString::from(format!("place-emoji-{e}")))
                                                .flex_1()
                                                .h(px(32.))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .rounded(px(8.))
                                                .cursor_pointer()
                                                .text_size(px(18.))
                                                .when(on, |el| el.bg(selected))
                                                .when(!on, |el| el.hover(move |el| el.bg(hover)))
                                                .child(e.clone())
                                                .on_click(move |_, _, cx| {
                                                    let _ = weak.update(cx, |s, cx| s.pick_place_icon(Some(e.clone()), cx));
                                                })
                                        }))
                                        .children((emoji.len()..sessions::EMOJI_PER_ROW).map(|_| div().flex_1()))
                                        .into_any_element(),
                                })
                                .collect()
                        })
                        .with_decoration(sessions::StickyEmojiHeading {
                            headings: rows.iter().enumerate().filter_map(|(i, r)| match r { sessions::EmojiRow::Heading(n) => Some((i, *n)), _ => None }).collect(),
                            bg: theme.card,
                            color: faint,
                        })
                        .track_scroll(&edit.value.emoji_scroll)
                        .w_full()
                        .h(px(252.));
                        let scroll = edit.value.emoji_scroll.0.borrow().base_handle.clone();
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(field(&edit.value.search))
                            .child(match rows.is_empty() {
                                true => div().h(px(252.)).pt(px(8.)).text_color(theme.text_faint).child("No emoji by that name").into_any_element(),
                                false => div()
                                    .relative()
                                    .child(crate::ui::edge_fade::edge_faded(12., &scroll, list).bottom_only())
                                    .child(crate::ui::scrollbar("place-icons-scrollbar", &edit.value.emoji_scroll, theme))
                                    .into_any_element(),
                            })
                            .into_any_element()
                    } else {
                        // every Solar Linear icon, found by name, laid out only where the list shows them
                        let rows = Rc::new(sessions::icon_rows(&edit.value.search.read(cx).value()));
                        let (rows_, picked_, weak) = (rows.clone(), picked.clone(), cx.entity().downgrade());
                        let (accent, muted, selected, hover) = (theme.accent, theme.text_muted, theme.selected, theme.hover);
                        let letter = letter.clone();
                        let list = gpui::uniform_list("place-icons", rows.len(), move |range, _, _| {
                            range
                                .map(|i| {
                                    div().w_full().h(px(36.)).pr(px(12.)).flex().gap(px(4.)).children(rows_[i].iter().map(|tile| {
                                        let (icon, id) = match tile {
                                            sessions::IconTile::Letter => (None, "place-icon-letter".to_string()),
                                            sessions::IconTile::Solar(name) => (Some(format!("icon:{name}")), format!("place-icon-{name}")),
                                        };
                                        let on = picked_ == icon;
                                        let weak = weak.clone();
                                        let mark = match &icon {
                                            Some(icon) => sessions::icon_mark(icon, 16., if on { accent } else { muted }),
                                            None => div().text_size(px(13.)).font_weight(gpui::FontWeight::SEMIBOLD).text_color(muted).child(letter.clone()).into_any_element(),
                                        };
                                        div()
                                            .id(SharedString::from(id))
                                            .flex_1()
                                            .h(px(32.))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(8.))
                                            .cursor_pointer()
                                            .when(on, |el| el.bg(selected))
                                            .when(!on, |el| el.hover(move |el| el.bg(hover)))
                                            .child(mark)
                                            .on_click(move |_, _, cx| {
                                                let _ = weak.update(cx, |s, cx| s.pick_place_icon(icon.clone(), cx));
                                            })
                                    }))
                                    .children((rows_[i].len()..sessions::EMOJI_PER_ROW).map(|_| div().flex_1()))
                                })
                                .collect()
                        })
                        .track_scroll(&edit.value.emoji_scroll)
                        .w_full()
                        .h(px(252.));
                        let scroll = edit.value.emoji_scroll.0.borrow().base_handle.clone();
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(field(&edit.value.search))
                            .child(match rows.is_empty() {
                                true => div().h(px(252.)).pt(px(8.)).text_color(theme.text_faint).child("No icon by that name").into_any_element(),
                                false => div()
                                    .relative()
                                    .child(crate::ui::edge_fade::edge_faded(12., &scroll, list))
                                    .child(crate::ui::scrollbar("place-icons-scrollbar", &edit.value.emoji_scroll, theme))
                                    .into_any_element(),
                            })
                            .into_any_element()
                    };
                    div()
                        .mt(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        // the heading, and on its right which set the grid shows
                        .child(
                            div()
                                .flex()
                                .items_end()
                                .gap(px(2.))
                                .child(div().flex_1().text_color(theme.text_muted).child("Icon"))
                                .child(tab("place-icons-svg", "Icons", !emoji).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.show_place_emoji(false, cx))))
                                .child(tab("place-icons-emoji", "Emoji", emoji).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.show_place_emoji(true, cx)))),
                        )
                        .child(grid)
                })
                .child(
                    div()
                        .mt(px(16.))
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(ghost_button(theme, "place-edit-cancel", "Cancel").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_place_edit(false, cx))))
                        .child(
                            solid_button("place-edit-save", "Save", theme.text, theme.surface)
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_place_edit(true, cx))),
                        ),
                );
            modal("place-edit", edit, window, card)
        }))
        .children(self.web_menu(theme, cx))
        .children(self.new_tab_menu(theme, cx))
        .children(self.ws_menu.as_ref().map(|menu| {
            let (id, at) = menu.value.clone();
            let worktree = snapshot
                .as_ref()
                .and_then(|s| s.workspaces.iter().find(|w| w.workspace_id == id))
                .and_then(|w| w.worktree.clone());
            // a repo's checkout makes and opens worktrees; a linked worktree removes itself instead. herdr reports a
            // repo (`worktree`) only once it has a worktree group, so any other workspace may be one: herdr refuses
            // a folder outside git (`not_git_worktree`), and that shows as a toast.
            let linked = worktree.as_ref().is_some_and(|w| w.is_linked_worktree);
            let has_repo = !linked;
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
                            .child(row(theme, false, "ws-menu-open-worktree").h(px(28.)).rounded(px(6.)).child("Open worktree").on_pick(at,
                                cx.listener({
                                    let id = id.clone();
                                    move |this, _: &ClickEvent, window, cx| {
                                        if let Some(m) = this.ws_menu.as_mut() { m.close() }
                                        this.open_worktrees_panel(id.clone(), window, cx);
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
                            el.child(row(theme, false, "ws-menu-reveal").h(px(28.)).rounded(px(6.)).child(if cfg!(target_os = "macos") { "Reveal in Finder" } else { "Show in file manager" }).on_pick(at, 
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
                                .child(if linked { "Close worktree" } else { "Close workspace" }) // the checkout stays
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
        .children(self.session_dialog.as_ref().map(|dialog| {
            let remote = dialog.value.remote;
            let field = |input: &Entity<InputState>| {
                div()
                    .mt(px(8.))
                    .h(px(SESSION_FIELD_H))
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.wash(0.04))
                    .text_size(px(14.))
                    .child(div().flex_1().child(Input::new(input)))
            };
            let card = dialog_card(theme, "session-dialog", "New session")
                .on_mouse_down_out(cx.listener(|this, _, _, cx| this.finish_session_dialog(false, cx)))
                .on_key_down(cx.listener(|this, e: &gpui::KeyDownEvent, _, cx| {
                    if e.keystroke.key == "escape" {
                        cx.stop_propagation();
                        this.finish_session_dialog(false, cx);
                    }
                }))
                .child(div().mt(px(4.)).text_color(theme.text_muted).child(if remote { "Name, or empty for the host's default session" } else { "Name" }))
                .child(field(&dialog.value.name))
                // the host field slides open under the name, and the card grows with it
                .child({
                    // its margin and box: the height it springs to
                    let full = 8. + SESSION_FIELD_H;
                    let host = div().flex_none().overflow_hidden().child(field(&dialog.value.host));
                    match dialog.value.flips {
                        0 => host.h(px(0.)).into_any_element(),
                        flips => host
                            .with_animation(
                                SharedString::from(format!("session-host-{flips}")),
                                // gpui wants eased time inside 0..=1; the spring overshoots, so it shapes the height here
                                gpui::Animation::new(SPRING_DURATION),
                                move |el, t| {
                                    let open = if remote { spring(t) } else { 1. - spring(t) };
                                    el.h(px(full * open.max(0.))).opacity(open.clamp(0., 1.))
                                },
                            )
                            .into_any_element(),
                    }
                })
                // on: the session runs on an SSH host, saved as a herdr machine
                .child(
                    row(theme, false, "session-remote")
                        .mt(px(12.))
                        .mx(px(-8.))
                        .h(px(30.))
                        .rounded(px(6.))
                        .child(div().flex_1().child("Remote"))
                        .child(crate::ui::switch(theme, remote, "session-remote"))
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_session_remote(window, cx))),
                )
                .child(
                    div()
                        .mt(px(16.))
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            ghost_button(theme, "session-cancel", "Cancel")
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_session_dialog(false, cx))),
                        )
                        .child(
                            solid_button("session-create", "Create", theme.text, theme.surface)
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_session_dialog(true, cx))),
                        ),
                );
            modal("session-dialog", dialog, window, card)
        }))
        .children(self.session_delete.as_ref().map(|confirm| {
            let (title, text, action) = match &confirm.value {
                sessions::Place::Session(s) if s.running => (
                    "Delete session?",
                    format!("{}'s server stops, closing its workspaces and their panes, and the session is deleted.", s.name),
                    "Delete",
                ),
                sessions::Place::Session(s) => ("Delete session?", format!("{} and its saved workspaces are deleted.", s.name), "Delete"),
                sessions::Place::Machine(m) => (
                    "Forget machine?",
                    format!("herdr forgets {}. Its session keeps running on {}.", m.label, m.target),
                    "Forget",
                ),
            };
            let card = dialog_card(theme, "session-delete-dialog", title)
                .on_mouse_down_out(cx.listener(|this, _, _, cx| this.finish_delete(false, cx)))
                .track_focus(&self.confirm_focus)
                .on_key_down(cx.listener(|this, e: &gpui::KeyDownEvent, _, cx| match e.keystroke.key.as_str() {
                    "enter" | "escape" => {
                        cx.stop_propagation();
                        this.finish_delete(e.keystroke.key == "enter", cx);
                    }
                    _ => {}
                }))
                .child(div().mt(px(8.)).line_height(px(19.)).text_color(theme.text_muted).child(text))
                .child(
                    div()
                        .mt(px(16.))
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            ghost_button(theme, "session-delete-cancel", "Cancel")
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_delete(false, cx))),
                        )
                        .child(
                            solid_button("session-delete-confirm", action, theme.danger, gpui::white())
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_delete(true, cx))),
                        ),
                );
            modal("session-delete-dialog", confirm, window, card)
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
                .track_focus(&self.confirm_focus)
                .on_key_down(cx.listener(|this, e: &gpui::KeyDownEvent, _, cx| match e.keystroke.key.as_str() {
                    "enter" | "escape" => {
                        cx.stop_propagation();
                        this.finish_remove_confirm(e.keystroke.key == "enter", cx);
                    }
                    _ => {}
                }))
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
        .children(self.merge_confirm.as_ref().map(|confirm| {
            let ask = &confirm.value;
            let into = if ask.base.is_empty() { "its base branch".to_string() } else { ask.base.clone() };
            let title = match (ask.auto, ask.gitlab) {
                (true, _) => "Merge when the pipeline passes?",
                (false, true) => "Merge merge request?",
                (false, false) => "Merge pull request?",
            };
            let text = if ask.auto {
                format!("{} goes into {into} once its pipeline passes.", ask.label)
            } else {
                format!("{} goes into {into}.", ask.label)
            };
            // a merge's options, each a switch
            let option = |id: &'static str, label: &'static str, on: bool, flip: fn(&mut crate::shell::review::MergeAsk)| {
                row(theme, false, id)
                    .h(px(30.))
                    .mx(px(-8.))
                    .rounded(px(6.))
                    .child(div().flex_1().child(label))
                    .child(crate::ui::switch(theme, on, id))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if let Some(confirm) = this.merge_confirm.as_mut() {
                            flip(&mut confirm.value);
                        }
                        cx.notify();
                    }))
            };
            let card = dialog_card(theme, "merge-dialog", title)
                .on_mouse_down_out(cx.listener(|this, _, _, cx| this.finish_merge(false, cx)))
                .child(div().mt(px(8.)).line_height(px(19.)).text_color(theme.text_muted).child(text))
                .child(
                    div()
                        .mt(px(12.))
                        .flex()
                        .flex_col()
                        .child(option("merge-squash", "Squash commits", ask.squash, |a| a.squash = !a.squash))
                        .when(ask.gitlab, |el| el.child(option("merge-remove-branch", "Delete source branch", ask.remove_branch, |a| a.remove_branch = !a.remove_branch))),
                )
                .child(
                    div()
                        .mt(px(16.))
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(ghost_button(theme, "merge-cancel", "Cancel").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_merge(false, cx))))
                        .child(
                            solid_button("merge-confirm", if ask.auto { "Set auto-merge" } else { "Merge" }, theme.text, theme.surface)
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_merge(true, cx))),
                        ),
                );
            modal("merge-dialog", confirm, window, card)
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
                        this.select_tab(Some(tab.clone()));
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
            let on_settings = cx.listener(|this, _: &ClickEvent, window, cx| {
                if let Some(m) = this.usage_menu.as_mut() { m.close() }
                this.open_usage_settings(window, cx);
            });
            let card = crate::ui::statusbar::usage_card(theme, on_settings, cx).on_mouse_down_out(cx.listener(|this, _, _, cx| {
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
        .children(self.statusbar_menu.as_ref().map(|menu| {
            let show = SettingsGlobal::of(cx).settings.status_bar_show;
            let parts: [(&'static str, &'static str, &'static str, fn(&mut crate::settings::StatusBarShow) -> &mut bool); 5] = [
                ("statusbar-show-branch", "icons/git-branch.svg", "Branch", |s| &mut s.branch),
                ("statusbar-show-pr", "icons/git-pull-request.svg", "Pull request", |s| &mut s.pull_request),
                ("statusbar-show-folder", "icons/folder.svg", "Directory", |s| &mut s.folder),
                ("statusbar-show-tokens", "icons/bot.svg", "Tokens", |s| &mut s.tokens),
                ("statusbar-show-usage", "icons/clock-circle.svg", "Plan limits", |s| &mut s.usage),
            ];
            gpui::deferred(
                gpui::anchored()
                    .anchor(gpui::Anchor::BottomLeft)
                    .position(menu.value)
                    .snap_to_window_with_margin(px(8.))
                    .child(crate::ui::menu_motion(
                        "statusbar-menu-in",
                        menu,
                        menu_card(theme, "statusbar-menu", 200.)
                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                if let Some(m) = this.statusbar_menu.as_mut() { m.close() }
                                cx.notify();
                            }))
                            .child(div().px(px(8.)).pt(px(4.)).pb(px(2.)).text_size(px(11.)).text_color(theme.text_faint).child("SHOW"))
                            .children(parts.map(|(id, icon, label, part)| {
                                let mut shown = show;
                                menu_choice(theme, id, icon, label, *part(&mut shown)).on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                                    save_from_sidebar(cx, |s| {
                                        let on = part(&mut s.status_bar_show);
                                        *on = !*on;
                                    })
                                }))
                            })),
                    )),
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

/// The New session dialog's fields: one 14 px line in a bordered box.
const SESSION_FIELD_H: f32 = 38.;
