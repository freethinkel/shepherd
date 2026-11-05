//! Merge and delete: a linked worktree's branch merged into its repo's default branch in the main checkout, then the
//! worktree and the branch removed. The sidebar row shows each step; a failure leaves a `!` there whose popover offers
//! Force delete.
use super::*;

/// The Merge and delete dialog, opened once git has said what the worktree holds.
pub(super) struct MergeDelete {
    workspace_id: String,
    /// The worktree's checkout and its repo's main one, located.
    dir: String,
    root: String,
    branch: String,
    base: String,
    /// `git status --porcelain`: each file's status and path.
    files: Vec<(String, String)>,
    /// Commit the files before the merge; else they go with the worktree.
    commit: bool,
    message: Entity<InputState>,
    _events: Subscription,
}

/// A worktree being merged and deleted, or the step that failed.
pub(super) struct Merging {
    root: String,
    branch: String,
    base: String,
    step: String,
    /// What failed, for the row, and git's or herdr's words, for the popover.
    pub failed: Option<(&'static str, String)>,
}

impl Merging {
    pub fn step(&self) -> &str {
        self.failed.as_ref().map_or(self.step.as_str(), |f| f.0)
    }
}

impl Shell {
    /// Reads the worktree's branch, its repo's default branch and its uncommitted files, then asks.
    pub(super) fn ask_merge_delete(&mut self, workspace_id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.merging.contains_key(&workspace_id) {
            return;
        }
        let worktree = self.snapshot.as_ref().and_then(|s| s.workspaces.iter().find(|w| w.workspace_id == workspace_id)).and_then(|w| w.worktree.clone());
        let Some(t) = worktree.filter(|t| t.is_linked_worktree && !t.checkout_path.is_empty()) else {
            return crate::ui::toast::error(cx, "This isn't a worktree of a repo");
        };
        let (dir, root) = (crate::herdr::locate(&t.checkout_path), crate::herdr::locate(crate::worktree::root(&t)));
        cx.spawn_in(window, async move |this, cx| {
            let read = { let dir = dir.clone(); cx.background_spawn(async move { read(&dir) }).await };
            let _ = this.update_in(cx, |shell, window, cx| match read {
                Ok((branch, base, files)) => shell.open_merge_delete(workspace_id, dir, root, branch, base, files, window, cx),
                Err(e) => crate::ui::toast::error(cx, e),
            });
        })
        .detach();
    }

    #[allow(clippy::too_many_arguments)]
    fn open_merge_delete(&mut self, workspace_id: String, dir: String, root: String, branch: String, base: String, files: Vec<(String, String)>, window: &mut Window, cx: &mut Context<Self>) {
        let theme = Theme::of(cx).clone();
        let message = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("Commit message").default_value(format!("wip: {branch}"));
            input.set_editor_style(crate::ui::input_style(&theme));
            input
        });
        if files.is_empty() {
            window.focus(&self.confirm_focus, cx);
        } else {
            message.update(cx, |input, cx| {
                input.focus(window, cx);
                input.select_all(window, cx);
            });
        }
        let events = cx.subscribe_in(&message, window, |shell, _, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                shell.finish_merge_delete(true, window, cx);
            }
        });
        let ask = MergeDelete { workspace_id, dir, root, branch, base, files, commit: true, message, _events: events };
        self.merge_delete = Some(crate::ui::Popup::new(ask));
        cx.notify();
    }

    fn toggle_merge_commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ask) = self.merge_delete.as_mut().map(|d| &mut d.value) else { return };
        ask.commit = !ask.commit;
        // the message field goes with the switch: the dialog keeps the keyboard for Enter and Escape
        match ask.commit {
            true => ask.message.update(cx, |input, cx| input.focus(window, cx)),
            false => window.focus(&self.confirm_focus, cx),
        }
        cx.notify();
    }

    pub(super) fn finish_merge_delete(&mut self, go: bool, _: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = self.merge_delete.as_mut().filter(|d| d.is_open()) else { return };
        dialog.close();
        // the dialog held focus: give it back to herdr's focused pane on the next render
        self.focused = None;
        cx.notify();
        if !go {
            return;
        }
        let ask = &dialog.value;
        let dirty = !ask.files.is_empty();
        let message = (dirty && ask.commit).then(|| {
            let typed = ask.message.read(cx).value().trim().to_string();
            if typed.is_empty() { format!("wip: {}", ask.branch) } else { typed }
        });
        // left out, the files go with the worktree: herdr removes a dirty one only when forced
        let (id, dir, force) = (ask.workspace_id.clone(), ask.dir.clone(), dirty && !ask.commit);
        let merging = Merging { root: ask.root.clone(), branch: ask.branch.clone(), base: ask.base.clone(), step: String::new(), failed: None };
        self.merging.insert(id.clone(), merging);
        self.run_merge_delete(id, dir, message, force, cx);
    }

    /// Commit, merge, remove the worktree, delete the branch; each step shows on the row, and the first that fails
    /// stops there.
    fn run_merge_delete(&mut self, id: String, dir: String, message: Option<String>, force: bool, cx: &mut Context<Self>) {
        let Some(m) = self.merging.get(&id) else { return };
        let (root, branch, base) = (m.root.clone(), m.branch.clone(), m.base.clone());
        let (backend, rt) = (self.backend.clone(), self.rt.clone());
        cx.spawn(async move |this, cx| {
            let step = |cx: &mut gpui::AsyncApp, text: String| {
                let id = id.clone();
                let _ = this.update(cx, |shell, cx| {
                    if let Some(m) = shell.merging.get_mut(&id) {
                        m.step = text;
                    }
                    cx.notify();
                });
            };
            let done: Result<(), (&'static str, String)> = async {
                if let Some(message) = message {
                    step(cx, "Committing…".into());
                    cx.background_spawn(async move { commit(&dir, &message) }).await.map_err(|e| ("Commit failed", e))?;
                }
                step(cx, format!("Merging into {base}…"));
                let (r, b, into) = (root.clone(), branch.clone(), base.clone());
                cx.background_spawn(async move { merge(&r, &b, &into) }).await.map_err(|e| ("Merge failed", e))?;
                step(cx, "Removing…".into());
                let workspace = id.clone();
                let removed = rt.spawn(async move { backend.worktrees.remove(workspace, force).await }).await;
                removed.map_err(anyhow::Error::from).and_then(|r| r).map_err(|e| ("Remove failed", format!("Merged into {base}, but the worktree stays:\n{e:#}")))?;
                Ok(())
            }
            .await;
            match done {
                // the worktree is gone and its row with it, so a branch git won't delete is a toast
                Ok(()) => {
                    let deleted = cx.background_spawn(async move { git(&root, &["branch", "-d", &branch]) }).await;
                    let _ = this.update(cx, |_, cx| deleted.map_err(|e| crate::ui::toast::error(cx, e)));
                }
                Err(failed) => {
                    let _ = this.update(cx, |shell, cx| {
                        if let Some(m) = shell.merging.get_mut(&id) {
                            m.failed = Some(failed);
                        }
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    /// The failed row's Force delete: the worktree goes with whatever it holds, and its branch, merged or not.
    fn force_merge_delete(&mut self, id: String, cx: &mut Context<Self>) {
        let Some(m) = self.merging.get_mut(&id) else { return };
        m.failed = None;
        m.step = "Removing…".into();
        let (root, branch) = (m.root.clone(), m.branch.clone());
        cx.notify();
        let (backend, workspace) = (self.backend.clone(), id.clone());
        self.ask(async move { backend.worktrees.remove(workspace, true).await }, cx, move |shell, result, cx| {
            match result {
                Ok(_) => {
                    cx.spawn(async move |_, cx| {
                        let deleted = cx.background_spawn(async move { git(&root, &["branch", "-D", &branch]) }).await;
                        let _ = cx.update(|cx| deleted.map_err(|e| crate::ui::toast::error(cx, e)));
                    })
                    .detach();
                }
                Err(e) => {
                    if let Some(m) = shell.merging.get_mut(&id) {
                        m.failed = Some(("Remove failed", format!("{e:#}")));
                    }
                }
            }
            cx.notify();
        });
    }

    /// A row's `!`: shuts its own popover, else opens it, over another row's.
    pub(super) fn open_merge_failure(&mut self, id: String, at: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        match self.merge_failure.as_mut().filter(|p| p.is_open() && p.value.0 == id) {
            Some(p) => p.close(),
            None => self.merge_failure = Some(crate::ui::Popup::new((id, at))),
        }
        cx.notify();
    }

    /// Shuts the popover of the row `id`: another row's `!` may have just opened its own in the same press.
    fn close_merge_failure(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(p) = self.merge_failure.as_mut().filter(|p| p.value.0 == id) {
            p.close();
        }
        cx.notify();
    }

    /// Merge and delete?: the branch and where it goes, and the uncommitted files with a switch to commit them first.
    pub(super) fn merge_delete_dialog(&self, theme: &Theme, window: &Window, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let dialog = self.merge_delete.as_ref()?;
        let ask = &dialog.value;
        let chip = div()
            .px(px(8.))
            .py(px(3.))
            .rounded(px(6.))
            .bg(theme.wash(0.06))
            .text_size(px(12.))
            .font_family("Geist Mono")
            .child(format!("{} → {}", ask.branch, ask.base));
        let files = (!ask.files.is_empty()).then(|| {
            let n = ask.files.len();
            div()
                .mt(px(12.))
                .px(px(12.))
                .py(px(10.))
                .rounded(px(10.))
                .bg(theme.warning.opacity(0.12))
                .border_1()
                .border_color(theme.warning.opacity(0.3))
                .child(
                    div()
                        .text_color(theme.warning)
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(if n == 1 { "1 uncommitted file".to_string() } else { format!("{n} uncommitted files") }),
                )
                .child(
                    div()
                        .id("merge-files")
                        .mt(px(6.))
                        .max_h(px(116.))
                        .overflow_y_scroll()
                        .text_size(px(12.))
                        .line_height(px(20.))
                        .font_family("Geist Mono")
                        .children(ask.files.iter().map(|(status, path)| {
                            let color = match status.chars().find(|c| *c != ' ') {
                                Some('D') => theme.danger,
                                Some('M' | 'R' | 'U') => theme.warning,
                                _ => theme.success,
                            };
                            div()
                                .flex()
                                .child(div().w(px(22.)).flex_none().text_color(color).child(status.clone()))
                                .child(div().min_w_0().truncate().child(path.clone()))
                        })),
                )
        });
        let choice = (!ask.files.is_empty()).then(|| {
            div()
                .mt(px(10.))
                .flex()
                .flex_col()
                .child(
                    row(theme, false, "merge-commit")
                        .mx(px(-8.))
                        .h(px(30.))
                        .rounded(px(6.))
                        .child(div().flex_1().child("Commit them first"))
                        .child(crate::ui::switch(theme, ask.commit, "merge-commit"))
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_merge_commit(window, cx))),
                )
                .child(match ask.commit {
                    true => div()
                        .mt(px(4.))
                        .px(px(12.))
                        .py(px(8.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.wash(0.04))
                        .child(Input::new(&ask.message)),
                    false => div().mt(px(2.)).text_color(theme.text_muted).child("They're deleted with the worktree."),
                })
        });
        let card = dialog_card(theme, "merge-delete-dialog", "Merge and delete?")
            .on_mouse_down_out(cx.listener(|this, _, window, cx| this.finish_merge_delete(false, window, cx)))
            .track_focus(&self.confirm_focus)
            .on_key_down(cx.listener(|this, e: &gpui::KeyDownEvent, window, cx| match e.keystroke.key.as_str() {
                "enter" | "escape" => {
                    cx.stop_propagation();
                    this.finish_merge_delete(e.keystroke.key == "enter", window, cx);
                }
                _ => {}
            }))
            .child(div().mt(px(10.)).flex().child(chip))
            .child(
                div()
                    .mt(px(8.))
                    .line_height(px(19.))
                    .text_color(theme.text_muted)
                    .child(format!("{} merges into {} in the main checkout, then its worktree and branch are deleted.", ask.branch, ask.base)),
            )
            .children(files)
            .children(choice)
            .child(
                div()
                    .mt(px(16.))
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(ghost_button(theme, "merge-delete-cancel", "Cancel").on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.finish_merge_delete(false, window, cx))))
                    .child(
                        solid_button("merge-delete-confirm", "Merge and delete", theme.danger, gpui::white())
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.finish_merge_delete(true, window, cx))),
                    ),
            );
        Some(modal("merge-delete-dialog", dialog, window, card))
    }

    /// The failed row's `!`, clicked: what failed and git's words, Dismiss, and Force delete.
    pub(super) fn merge_failure_popover(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let popover = self.merge_failure.as_ref()?;
        let (id, at) = popover.value.clone();
        let m = self.merging.get(&id)?;
        let (title, detail) = m.failed.clone()?;
        let card = menu_card(theme, "merge-failure", 300.)
            .w(px(300.))
            .p(px(14.))
            .text_color(theme.text)
            .on_mouse_down_out(cx.listener({
                let id = id.clone();
                move |this, _, _, cx| this.close_merge_failure(&id, cx)
            }))
            .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(title))
            .child(div().mt(px(4.)).text_color(theme.text_muted).child(format!("{} → {}", m.branch, m.base)))
            .child(
                div()
                    .id("merge-failure-log")
                    .mt(px(8.))
                    .px(px(10.))
                    .py(px(8.))
                    .max_h(px(160.))
                    .overflow_y_scroll()
                    .rounded(px(8.))
                    .bg(theme.wash(0.05))
                    .text_size(px(12.))
                    .line_height(px(18.))
                    .font_family("Geist Mono")
                    .child(detail),
            )
            .child(
                div()
                    .mt(px(12.))
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(ghost_button(theme, "merge-failure-dismiss", "Dismiss").on_click(cx.listener({
                        let id = id.clone();
                        move |this, _: &ClickEvent, _, cx| {
                            this.merging.remove(&id);
                            this.close_merge_failure(&id, cx);
                        }
                    })))
                    .child(solid_button("merge-failure-force", "Force delete", theme.danger, gpui::white()).on_click(cx.listener(
                        move |this, _: &ClickEvent, _, cx| {
                            this.close_merge_failure(&id, cx);
                            this.force_merge_delete(id.clone(), cx);
                        },
                    ))),
            );
        Some(
            gpui::deferred(
                gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(crate::ui::menu_motion("merge-failure-in", popover, card)),
            )
            .with_priority(1),
        )
    }
}

/// git in a located directory: its output, or what it said when it failed (a conflict is on stdout).
fn git(dir: &str, args: &[&str]) -> Result<String, String> {
    let out = crate::herdr::git(dir, args).map_err(|e| e.to_string())?;
    let text = |b: &[u8]| String::from_utf8_lossy(b).trim_end().to_string();
    match out.status.success() {
        true => Ok(text(&out.stdout)),
        false => Err([text(&out.stdout), text(&out.stderr)].into_iter().filter(|t| !t.is_empty()).collect::<Vec<_>>().join("\n")),
    }
}

/// The worktree's branch, the branch it merges into and its uncommitted files.
fn read(dir: &str) -> Result<(String, String, Vec<(String, String)>), String> {
    let branch = git(dir, &["symbolic-ref", "--short", "HEAD"]).map_err(|_| "This worktree isn't on a branch".to_string())?;
    let base = base_branch(dir).ok_or("No origin/HEAD, main or master branch to merge into")?;
    if base == branch {
        return Err(format!("This worktree is on {base} itself"));
    }
    Ok((branch, base, parse_status(&git(dir, &["status", "--porcelain", "--untracked-files=all"])?)))
}

/// The local twin of `origin/HEAD` (`develop` for `origin/develop`), else `main`, else `master`.
fn base_branch(dir: &str) -> Option<String> {
    let remote = git(dir, &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"]).ok();
    let twin = remote.as_deref().and_then(|r| r.strip_prefix("origin/")).map(str::to_string);
    twin.into_iter()
        .chain(["main".to_string(), "master".to_string()])
        .find(|b| git(dir, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{b}")]).is_ok())
}

/// `git status --porcelain`, as each file's status and path.
fn parse_status(out: &str) -> Vec<(String, String)> {
    out.lines()
        .filter_map(|l| l.trim_start().split_once(' '))
        .map(|(status, path)| (status.to_string(), path.trim_start().to_string()))
        .collect()
}

fn commit(dir: &str, message: &str) -> Result<(), String> {
    git(dir, &["add", "-A"])?;
    git(dir, &["commit", "-m", message]).map(drop)
}

/// `branch` merged into `base` in the main checkout at `root`, which must have `base` out; a merge that stops (a
/// conflict) is aborted, leaving `base` where it was.
fn merge(root: &str, branch: &str, base: &str) -> Result<(), String> {
    let on = git(root, &["symbolic-ref", "--short", "HEAD"]).unwrap_or_else(|_| "a detached HEAD".into());
    if on != base {
        return Err(format!("The main checkout is on {on}, not {base}."));
    }
    // --no-edit: no editor for the merge commit's message, there's no terminal to show it
    git(root, &["merge", "--no-edit", branch]).map(drop).inspect_err(|_| {
        let _ = git(root, &["merge", "--abort"]);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn porcelain_reads_as_status_and_path() {
        let files = parse_status(" M src/a.rs\n?? new/\nR  old.rs -> new.rs\nD  gone.rs\n");
        assert_eq!(
            files,
            [("M", "src/a.rs"), ("??", "new/"), ("R", "old.rs -> new.rs"), ("D", "gone.rs")].map(|(s, p)| (s.to_string(), p.to_string()))
        );
    }

    #[test]
    fn merges_into_main_and_aborts_a_conflict() {
        let tmp = tempfile::tempdir().unwrap();
        let (root, wt) = (tmp.path().join("repo"), tmp.path().join("wt"));
        let (root, wt) = (root.to_str().unwrap(), wt.to_str().unwrap());
        let sh = |dir: &str, args: &[&str]| git(dir, args).unwrap();
        std::fs::create_dir(root).unwrap();
        sh(root, &["init", "-q", "-b", "main"]);
        // Windows' git checks a.txt out with \r\n otherwise
        sh(root, &["config", "core.autocrlf", "false"]);
        sh(root, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "init"]);
        sh(root, &["worktree", "add", "-q", "-b", "feature", wt]);
        std::fs::write(format!("{wt}/a.txt"), "feature\n").unwrap();

        let (branch, base, files) = read(wt).unwrap();
        assert_eq!((branch.as_str(), base.as_str(), files.len()), ("feature", "main", 1));
        sh(wt, &["config", "user.name", "t"]);
        sh(wt, &["config", "user.email", "t@t"]);
        commit(wt, "wip: feature").unwrap();

        // main got its own a.txt meanwhile: the merge stops, and main is left as it was
        std::fs::write(format!("{root}/a.txt"), "main\n").unwrap();
        commit(root, "main's a").unwrap();
        let before = sh(root, &["rev-parse", "HEAD"]);
        assert!(merge(root, "feature", "main").unwrap_err().contains("CONFLICT"));
        assert_eq!(sh(root, &["rev-parse", "HEAD"]), before);
        assert_eq!(sh(root, &["status", "--porcelain"]), "");

        sh(root, &["reset", "-q", "--hard", "HEAD~1"]);
        merge(root, "feature", "main").unwrap();
        assert_eq!(std::fs::read_to_string(format!("{root}/a.txt")).unwrap(), "feature\n");
        assert!(merge(root, "feature", "dev").unwrap_err().contains("is on main, not dev"));
    }
}
