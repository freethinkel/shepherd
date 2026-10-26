//! Settings → Worktrees: the projects of herdr's worktree hooks (`hooks.conf`), grouped by herdr session, each with
//! its commands, saved when a field is left. A project is added from the repos open in its session (the sidebar's).
//! herdr runs the commands, so they keep working without Shepherd.
use super::*;
use crate::herdr::hooks::{self, Project};
use gpui_base::input::TextareaState;

/// One project's commands; its session, name and main checkout come from the repo it was added from.
pub(super) struct ProjectFields {
    session: String,
    name: String,
    dir: String,
    created: Entity<TextareaState>,
    removed: Entity<TextareaState>,
    _events: [Subscription; 2],
}

/// A herdr session as Worktrees groups projects: its name (the section prefix in `hooks.conf`), as the session bar
/// names it, and the repos open in it as (name, main checkout), none while it is not running.
pub(super) struct SessionRepos {
    name: String,
    label: String,
    running: bool,
    repos: Vec<(String, String)>,
}

/// One command a line: blank lines and the spaces around each command dropped.
fn commands(text: &str) -> Vec<String> {
    text.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect()
}

fn text(commands: &[String]) -> String {
    commands.join("\n")
}

/// The repos open in a session, as its sidebar lists them: each git repo's main checkout once, in herdr's order.
fn repos_of(list: &Value) -> Vec<(String, String)> {
    let mut repos: Vec<(String, String)> = Vec::new();
    for w in list["workspaces"].as_array().into_iter().flatten() {
        let (Some(root), Some(name)) = (w["worktree"]["repo_root"].as_str(), w["worktree"]["repo_name"].as_str()) else { continue };
        if !repos.iter().any(|(_, r)| r == root) {
            repos.push((name.to_string(), root.to_string()));
        }
    }
    repos
}

/// `~/…` for a path in the home folder, as hooks.conf is written by hand.
fn tilde(path: &str) -> String {
    let home = crate::settings::home();
    match std::path::Path::new(path).strip_prefix(&home) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.to_string(),
    }
}

impl SettingsView {
    /// Reads `hooks.conf` the first time the section is shown, makes each project's fields, and asks herdr for its
    /// sessions and the repos open in each.
    pub(super) fn load_worktrees(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for p in std::mem::take(&mut self.worktree_adds) {
            self.add_project(&p, window, cx);
            self.save_worktrees(cx);
        }
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
        let rt = cx.global::<Tokio>().0.clone();
        let list = rt.spawn(async move {
            let mut out = Vec::new();
            for s in crate::herdr::sessions().unwrap_or_default() {
                let repos = match s.running {
                    true => Client::at(s.socket_path.clone().into()).call("workspace.list", json!({})).await.map(|r| repos_of(&r)).unwrap_or_default(),
                    false => Vec::new(),
                };
                out.push((s, repos));
            }
            out
        });
        cx.spawn(async move |v, cx| {
            let list = list.await.unwrap_or_default();
            let _ = v.update(cx, |v, cx| {
                let looks = &SettingsGlobal::of(cx).settings.place_looks;
                let label = |s: &crate::herdr::SessionInfo| looks.get(&s.socket_path).and_then(|l| l.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| s.name.clone());
                v.worktree_sessions = Some(list.into_iter().map(|(s, repos)| SessionRepos { label: label(&s), name: s.name, running: s.running, repos }).collect());
                cx.notify();
            });
        })
        .detach();
    }

    fn add_project(&mut self, p: &Project, window: &mut Window, cx: &mut Context<Self>) {
        let theme = Theme::of(cx).clone();
        let lines = |value: String, placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| {
                // as tall as its commands: the page scrolls, with the app's scrollbar, not gpui-base's inside the field
                // ponytail: past 500 wrapped lines the field scrolls on its own again
                let mut input = TextareaState::new(window, cx).auto_grow(2, 500).placeholder(placeholder).default_value(value);
                input.set_editor_style(input_style(&theme));
                input
            })
        };
        let created = lines(text(&p.created), r#"ln -sf "$MAIN/.env" .env"#, window, cx);
        let removed = lines(text(&p.removed), "docker compose down", window, cx);
        let save_on_blur = |v: &mut Self, event: &InputEvent, cx: &mut Context<Self>| {
            if matches!(event, InputEvent::Blur) {
                v.save_worktrees(cx);
            }
        };
        let events = [
            cx.subscribe_in(&created, window, move |v, _, e, _, cx| save_on_blur(v, e, cx)),
            cx.subscribe_in(&removed, window, move |v, _, e, _, cx| save_on_blur(v, e, cx)),
        ];
        // one without a session, from before sessions or written by hand, becomes the default session's
        let session = p.session.clone().unwrap_or_else(|| "default".into());
        if let Some(Ok(fields)) = &mut self.worktrees {
            fields.push(ProjectFields { session, name: p.name.clone(), dir: p.dir.clone(), created, removed, _events: events });
        }
    }

    /// Writes the projects to `hooks.conf` (installing herdr's plugin the first time) off the main thread.
    fn save_worktrees(&mut self, cx: &mut Context<Self>) {
        let Some(Ok(fields)) = &self.worktrees else { return };
        let projects: Vec<Project> = fields
            .iter()
            .map(|f| Project {
                session: Some(f.session.clone()),
                name: f.name.clone(),
                dir: f.dir.clone(),
                created: commands(&f.created.read(cx).value()),
                removed: commands(&f.removed.read(cx).value()),
            })
            .collect();
        // a save that a later one overtook writes nothing (Blur, then a bin's click, start two at once)
        let saves = self.worktree_saves.clone();
        let this_save = saves.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        let saving = cx.background_spawn(async move { hooks::save(&projects, || saves.load(std::sync::atomic::Ordering::SeqCst) != this_save) });
        cx.spawn(async move |this, cx| {
            let result = saving.await;
            let _ = this.update(cx, |v, cx| {
                v.worktree_note = result.err().map(|e| format!("Not saved: {e:#}"));
                cx.notify();
            });
        })
        .detach();
    }

    /// Settings → Worktrees: a group per herdr session (running ones, and stopped ones with projects), each project
    /// its repo's name and path, a red bin and its two command fields; "Add project" lists the session's open repos.
    pub(super) fn worktrees_page(&self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let muted = theme.text_muted;
        let machine = SettingsGlobal::of(cx).settings.herdr_machine.is_some();
        let body = match &self.worktrees {
            None => div(),
            Some(Err(e)) => note(theme, e.clone()),
            Some(Ok(fields)) => {
                // herdr's sessions in its order, then any a project names that herdr no longer has
                let mut groups: Vec<(String, String, bool, Vec<(String, String)>)> = self
                    .worktree_sessions
                    .iter()
                    .flatten()
                    .map(|s| (s.name.clone(), s.label.clone(), s.running, s.repos.clone()))
                    .collect();
                for f in fields {
                    if !groups.iter().any(|(name, ..)| *name == f.session) {
                        groups.push((f.session.clone(), f.session.clone(), false, Vec::new()));
                    }
                }
                let weak = cx.entity().downgrade();
                let groups: Vec<Div> = groups
                    .into_iter()
                    .filter(|(name, _, running, _)| *running || fields.iter().any(|f| f.session == *name))
                    .map(|(session, label, running, repos)| {
                        let rows: Vec<Div> = fields
                            .iter()
                            .enumerate()
                            .filter(|(_, f)| f.session == session)
                            .enumerate()
                            .map(|(n, (i, f))| {
                                let remove = remove_button(theme, format!("worktree-remove-{i}")).mt(px(0.)).on_click(cx.listener(move |v, _, _, cx| {
                                    if let Some(Ok(fields)) = &mut v.worktrees {
                                        fields.remove(i);
                                    }
                                    v.save_worktrees(cx);
                                    cx.notify();
                                }));
                                div()
                                    .mx(px(16.))
                                    .py(px(14.))
                                    .when(n > 0, |el| el.border_t_1().border_color(theme.border.opacity(0.6)))
                                    .flex()
                                    .flex_col()
                                    .gap(px(12.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(12.))
                                            .child(row_text(theme, f.name.clone(), f.dir.clone()).flex_1().min_w_0())
                                            .child(remove),
                                    )
                                    .child(labelled(theme, "After creating a worktree, in it", text_area(theme, &f.created)))
                                    .child(labelled(theme, "After removing one, in the main checkout", text_area(theme, &f.removed)))
                            })
                            .collect();
                        let empty = rows.is_empty();
                        // repos open in the session and not yet a project of it
                        let addable: Vec<(String, String)> = repos
                            .into_iter()
                            .filter(|(_, root)| !fields.iter().any(|f| f.session == session && (f.dir == *root || f.dir == tilde(root))))
                            .collect();
                        let add = (!addable.is_empty()).then(|| {
                            let labels = addable.iter().map(|(name, _)| SharedString::from(name.clone())).collect();
                            let (weak, for_session) = (weak.clone(), session.clone());
                            let pick = move |i: usize, cx: &mut App| {
                                let (name, root) = addable[i].clone();
                                let p = Project { session: Some(for_session.clone()), name, dir: tilde(&root), ..Project::default() };
                                // the fields need a window: render makes them (load_worktrees)
                                let _ = weak.update(cx, |v, cx| {
                                    v.worktree_adds.push(p);
                                    cx.notify();
                                });
                            };
                            div().w(px(220.)).child(self.select(theme, web_select_id("worktree-add", &session), labels, None, "Add project".into(), |_| None, pick, cx))
                        });
                        div()
                            .child(
                                div()
                                    .mt(px(32.))
                                    .mb(px(8.))
                                    .pl(px(8.))
                                    .min_h(px(32.))
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(div().flex_1().text_color(muted).child(label).when(!running, |el| el.child(div().text_color(theme.text_faint).child("Not running"))))
                                    .children(add),
                            )
                            .child(card(theme).mt(px(0.)).children(rows).when(empty, |el| {
                                el.child(card_row(theme, true).child(row_text(
                                    theme,
                                    "No projects yet",
                                    "Add one of the repos open in this session: its commands run in each new worktree of it.",
                                )))
                            }))
                    })
                    .collect();
                div()
                    .children(groups)
                    .child(note(theme, "One command a line, run in order; the first that fails stops the rest. $MAIN is the main checkout, $WORKTREE the worktree, $PROJECT the project's name."))
            }
        };
        div()
            .child(page_header(
                theme,
                "Worktrees",
                "Commands herdr runs in a new worktree and after one is removed, per session. They run in herdr, so they keep working with Shepherd closed or deleted, from any client.",
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

    #[test]
    fn repos_are_main_checkouts_once() {
        let list = json!({"workspaces": [
            {"worktree": {"repo_root": "/r/app", "repo_name": "app", "checkout_path": "/r/app"}},
            {"worktree": {"repo_root": "/r/app", "repo_name": "app", "checkout_path": "/w/app/feature"}},
            {"label": "a folder, not a repo"},
            {"worktree": {"repo_root": "/r/web", "repo_name": "web"}},
        ]});
        assert_eq!(repos_of(&list), [("app".to_string(), "/r/app".to_string()), ("web".to_string(), "/r/web".to_string())]);
    }
}
