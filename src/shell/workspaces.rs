//! Workspace actions: worktrees made and removed through their dialogs, renames, new workspaces.
use super::*;

impl Shell {
    pub(super) fn open_worktree_dialog(&mut self, workspace_id: String, window: &mut Window, cx: &mut Context<Self>) {
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
    pub(super) fn finish_worktree_dialog(&mut self, create: bool, cx: &mut Context<Self>) {
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

    /// The command panel over the worktrees of the workspace's repo: open at once, filled when herdr answers.
    pub(super) fn open_worktrees_panel(&mut self, workspace_id: String, window: &mut Window, cx: &mut Context<Self>) {
        use crate::ui::command_panel::Kind;
        self.close_command_panel(cx);
        self.toggle_command_panel(Kind::Worktrees, window, cx);
        let Some(panel) = self.command_panel.as_ref().map(|p| p.value.0.downgrade()) else { return };
        let repo = workspace_id.clone();
        self.send_then(Command::ListWorktrees { workspace_id }, cx, move |shell, result, cx| {
            let Some(panel) = panel.upgrade() else { return };
            match result {
                Ok(list) => {
                    let home = crate::settings::home().to_string_lossy().into_owned();
                    let items = crate::ui::command_panel::worktree_items(&list, &repo, &home);
                    panel.update(cx, |panel, cx| panel.set_items(items, cx));
                }
                Err(err) => {
                    // herdr refuses a folder outside git (`not_git_worktree`): say so and drop the empty panel
                    crate::ui::toast::error(cx, format!("{err:#}"));
                    if shell.command_panel.as_ref().is_some_and(|p| p.value.0 == panel) {
                        shell.close_command_panel(cx);
                    }
                }
            }
        });
    }

    pub(super) fn new_worktree(&mut self, workspace_id: String, branch: String, cx: &mut Context<Self>) {
        let pending = (workspace_id.clone(), branch.clone());
        self.pending_worktrees.push(pending.clone());
        self.send_then(Command::CreateWorktree { workspace_id, branch: Some(branch) }, cx, move |shell, result, cx| {
            shell.pending_worktrees.retain(|p| *p != pending);
            match result {
                Ok(value) => shell.select_tab(crate::commands::created_tab_id(&value)),
                Err(err) => crate::ui::toast::error(cx, format!("{err:#}")),
            }
            cx.notify();
        });
        cx.notify();
    }

    /// Asks before a worktree is removed.
    pub(super) fn confirm_remove_worktree(&mut self, workspace_id: String, dirty: bool, cx: &mut Context<Self>) {
        let label = self.snapshot.as_ref().and_then(|s| s.workspaces.iter().find(|w| w.workspace_id == workspace_id)).map(|w| w.label.clone()).unwrap_or_default();
        self.remove_confirm = Some(crate::ui::Popup::new((workspace_id, label, dirty)));
        cx.notify();
    }

    /// herdr deletes the checkout and closes the workspace; one with changes asks again, then goes with `force`.
    pub(super) fn remove_worktree(&mut self, workspace_id: String, force: bool, cx: &mut Context<Self>) {
        self.removing.insert(workspace_id.clone());
        cx.notify();
        let id = workspace_id.clone();
        self.send_then(Command::RemoveWorktree { workspace_id, force }, cx, move |shell, result, cx| {
            // on success the row keeps its spinner until herdr's snapshot drops the workspace
            if let Err(err) = result {
                shell.removing.remove(&id);
                let message = format!("{err:#}");
                if message.starts_with("dirty_worktree_requires_force") {
                    shell.confirm_remove_worktree(id.clone(), true, cx);
                } else {
                    crate::ui::toast::error(cx, message);
                }
                cx.notify();
            }
        });
    }

    pub(super) fn finish_remove_confirm(&mut self, remove: bool, cx: &mut Context<Self>) {
        let Some(confirm) = self.remove_confirm.as_mut().filter(|c| c.is_open()) else { return };
        confirm.close();
        // the confirm held focus: give it back to herdr's focused pane on the next render
        self.focused = None;
        let (workspace_id, _, dirty) = confirm.value.clone();
        if remove {
            self.remove_worktree(workspace_id, dirty, cx);
        }
        cx.notify();
    }

    pub(super) fn new_workspace(&mut self, cx: &mut Context<Self>) {
        self.send(Command::NewWorkspace { cwd: None }, cx, |shell, result, cx| {
            shell.select_tab(crate::commands::created_tab_id(result));
            cx.notify();
        });
    }

    pub(super) fn start_rename(&mut self, target: RenameTarget, label: String, window: &mut Window, cx: &mut Context<Self>) {
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
    pub(super) fn finish_rename(&mut self, commit: bool, cx: &mut Context<Self>) {
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
    pub(super) fn rename_field(&self, target: &RenameTarget, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
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
}

/// A branch name the way herdr generates one.
pub(super) fn new_branch_name() -> String {
    let micros = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_micros() as u64);
    crate::commands::generated_branch(micros)
}
