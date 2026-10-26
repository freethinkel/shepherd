//! Settings → Worktrees: the projects of herdr's worktree hooks (`hooks.conf`), a block of fields each, saved when a
//! field is left. herdr runs the commands, so they keep working without Shepherd.
use super::*;
use crate::herdr::hooks::{self, Project};
use gpui_base::input::TextareaState;

/// One project's fields.
pub(super) struct ProjectFields {
    name: Entity<InputState>,
    dir: Entity<InputState>,
    created: Entity<TextareaState>,
    removed: Entity<TextareaState>,
    _events: [Subscription; 4],
}

/// One command a line: blank lines and the spaces around each command dropped.
fn commands(text: &str) -> Vec<String> {
    text.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect()
}

fn text(commands: &[String]) -> String {
    commands.join("\n")
}

impl SettingsView {
    /// Reads `hooks.conf` the first time the section is shown and makes each project's fields.
    pub(super) fn load_worktrees(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.worktrees.is_some() {
            return;
        }
        match hooks::load() {
            Ok(projects) => {
                self.worktrees = Some(Ok(Vec::new()));
                for p in &projects {
                    self.add_project(p, window, cx);
                }
            }
            // not saved over: what could not be read stays as it is
            Err(e) => self.worktrees = Some(Err(format!("{e:#}"))),
        }
    }

    fn add_project(&mut self, p: &Project, window: &mut Window, cx: &mut Context<Self>) {
        let theme = Theme::of(cx).clone();
        let line = |value: &str, placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| {
                let mut input = InputState::new(window, cx).placeholder(placeholder).default_value(value.to_string());
                input.set_editor_style(input_style(&theme));
                input
            })
        };
        let lines = |value: String, placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| {
                let mut input = TextareaState::new(window, cx).auto_grow(2, 12).placeholder(placeholder).default_value(value);
                input.set_editor_style(input_style(&theme));
                input
            })
        };
        let name = line(&p.name, "callcenter", window, cx);
        let dir = line(&p.dir, "~/Developer/callcenter", window, cx);
        let created = lines(text(&p.created), r#"ln -sf "$MAIN/.env" .env"#, window, cx);
        let removed = lines(text(&p.removed), "docker compose down", window, cx);
        let save_on_blur = |v: &mut Self, event: &InputEvent, cx: &mut Context<Self>| {
            if matches!(event, InputEvent::Blur) {
                v.save_worktrees(cx);
            }
        };
        let events = [
            cx.subscribe_in(&name, window, move |v, _, e, _, cx| save_on_blur(v, e, cx)),
            cx.subscribe_in(&dir, window, move |v, _, e, _, cx| save_on_blur(v, e, cx)),
            cx.subscribe_in(&created, window, move |v, _, e, _, cx| save_on_blur(v, e, cx)),
            cx.subscribe_in(&removed, window, move |v, _, e, _, cx| save_on_blur(v, e, cx)),
        ];
        if let Some(Ok(fields)) = &mut self.worktrees {
            fields.push(ProjectFields { name, dir, created, removed, _events: events });
        }
    }

    /// Writes the projects to `hooks.conf` (installing herdr's plugin the first time) off the main thread.
    // ponytail: two quick saves may land out of order; the next field left writes again
    fn save_worktrees(&mut self, cx: &mut Context<Self>) {
        let Some(Ok(fields)) = &self.worktrees else { return };
        let value = |input: &Entity<InputState>| input.read(cx).value().trim().to_string();
        let projects: Vec<Project> = fields
            .iter()
            .filter_map(|f| {
                let dir = value(&f.dir);
                // a section name holds no brackets; without one the folder names the project
                let mut name: String = value(&f.name).chars().filter(|c| !matches!(c, '[' | ']')).collect();
                if name.is_empty() {
                    name = dir.trim_end_matches('/').rsplit('/').next().unwrap_or_default().to_string();
                }
                (!name.is_empty() || !dir.is_empty()).then(|| Project {
                    name,
                    dir,
                    created: commands(&f.created.read(cx).value()),
                    removed: commands(&f.removed.read(cx).value()),
                })
            })
            .collect();
        let saving = cx.background_spawn(async move { hooks::save(&projects) });
        cx.spawn(async move |this, cx| {
            let result = saving.await;
            let _ = this.update(cx, |v, cx| {
                v.worktree_note = result.err().map(|e| format!("Not saved: {e:#}"));
                cx.notify();
            });
        })
        .detach();
    }

    /// Settings → Worktrees, laid out as Web pages: each project a block of labelled fields with a red bin, "Add
    /// project" by the list's label.
    pub(super) fn worktrees_page(&self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let muted = theme.text_muted;
        let machine = SettingsGlobal::of(cx).settings.herdr_machine.is_some();
        let body = match &self.worktrees {
            None => div(),
            Some(Err(e)) => note(theme, e.clone()),
            Some(Ok(fields)) => {
                let rows: Vec<Div> = fields
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let remove = remove_button(theme, format!("worktree-remove-{i}")).on_click(cx.listener(move |v, _, _, cx| {
                            if let Some(Ok(fields)) = &mut v.worktrees {
                                fields.remove(i);
                            }
                            v.save_worktrees(cx);
                            cx.notify();
                        }));
                        div()
                            .mx(px(16.))
                            .py(px(14.))
                            .when(i > 0, |el| el.border_t_1().border_color(theme.border.opacity(0.6)))
                            .flex()
                            .items_start()
                            .gap(px(12.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(12.))
                                    .child(
                                        div()
                                            .flex()
                                            .gap(px(12.))
                                            .child(labelled(theme, "Name", field(theme, &f.name).w_full()))
                                            .child(labelled(theme, "Main checkout", field(theme, &f.dir).w_full())),
                                    )
                                    .child(labelled(theme, "After creating a worktree, in it", text_area(theme, &f.created)))
                                    .child(labelled(theme, "After removing one, in the main checkout", text_area(theme, &f.removed))),
                            )
                            .child(remove)
                    })
                    .collect();
                let empty = rows.is_empty();
                div()
                    .child(
                        div()
                            .mt(px(32.))
                            .mb(px(8.))
                            .pl(px(8.))
                            .flex()
                            .items_center()
                            .child(div().flex_1().text_color(muted).child("Projects"))
                            .child(
                                action(theme, "worktree-add", "Add project")
                                    .gap(px(6.))
                                    .child(gpui::svg().path("icons/solar/add-circle.svg").size(px(14.)).text_color(muted))
                                    .on_click(cx.listener(|v, _, window, cx| {
                                        v.add_project(&Project::default(), window, cx);
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(card(theme).mt(px(0.)).children(rows).when(empty, |el| {
                        el.child(card_row(theme, true).child(row_text(
                            theme,
                            "No projects yet",
                            "Add one with its main checkout and the commands a new worktree needs.",
                        )))
                    }))
                    .child(note(theme, "One command a line, run in order; the first that fails stops the rest. $MAIN is the main checkout, $WORKTREE the worktree, $PROJECT the project's name."))
            }
        };
        div()
            .child(page_header(
                theme,
                "Worktrees",
                "Commands herdr runs in a new worktree and after one is removed. They run in herdr, so they keep working with Shepherd closed or deleted, from any client.",
            ))
            .when(machine, |el| el.child(note(theme, "These hooks are installed in the herdr on this computer, not on the machine shown.")))
            .child(body)
            .children(self.worktree_note.clone().map(|text| note(theme, text)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_to_commands() {
        assert_eq!(commands("a\n\n  b  \r\n"), vec!["a".to_string(), "b".to_string()]);
        assert_eq!(text(&["a".into(), "b".into()]), "a\nb");
        assert!(commands("").is_empty());
    }
}
