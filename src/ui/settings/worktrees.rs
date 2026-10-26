//! Settings → Worktrees: the worktree hooks (`hooks.conf`) of one herdr session at a time, here or on a saved herdr
//! machine (over SSH): every repo open in it (the sidebar's) with its commands, saved when a field is left. herdr runs
//! the commands, so they keep working without Shepherd.
use super::*;
use crate::herdr::hooks::{self, Project, tilde};
use gpui::{HighlightStyle, Hsla};
use gpui_base::input::{Editor, EditorState, FoldRange, HighlightStyleResolver, InputEdit, InputHighlighter, Rope};
use std::{ops::Range, rc::Rc};

/// One project's commands; its machine, session, name and main checkout come from its repo.
pub(super) struct ProjectFields {
    /// The SSH target of the herdr machine whose `hooks.conf` it is in; `None` for this computer's.
    machine: Option<String>,
    session: String,
    name: String,
    dir: String,
    created: Entity<EditorState>,
    removed: Entity<EditorState>,
    _events: [Subscription; 2],
}

/// A herdr session as Worktrees groups projects: its machine (`None` here), name (the section prefix in
/// `hooks.conf`), as the session bar names it, the repos open in it as (name, main checkout), none while it is not
/// running, and why its machine could not be read.
pub(super) struct SessionRepos {
    machine: Option<String>,
    name: String,
    label: String,
    running: bool,
    repos: Vec<(String, String)>,
    error: Option<String>,
}

/// Shell commands colored as Markdown's code blocks are: tree-sitter's bash in the terminal theme's colors.
struct ShellHighlighter {
    theme: Theme,
    spans: Vec<(Range<usize>, Hsla)>,
}

impl InputHighlighter for ShellHighlighter {
    fn language(&self) -> SharedString {
        "sh".into()
    }

    fn update(&mut self, _: Option<InputEdit>, text: &Rope, _: bool, _: &mut Window, _: &mut Context<EditorState>) {
        let spans = crate::ui::markdown::syntax(&text.to_string(), Some("sh"));
        self.spans = spans.iter().map(|(range, kind)| (range.clone(), crate::ui::markdown::syntax_color(*kind, &self.theme))).collect();
    }

    fn styles(&self, range: &Range<usize>, _: &dyn HighlightStyleResolver) -> Vec<(Range<usize>, HighlightStyle)> {
        runs(&self.spans, range)
    }

    fn fold_ranges(&self, _: &Rope) -> Vec<FoldRange> {
        Vec::new()
    }
}

/// `range` cut into runs, gap-free as the editor wants them: each span's color, plain text between.
fn runs(spans: &[(Range<usize>, Hsla)], range: &Range<usize>) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut runs = Vec::new();
    let mut at = range.start;
    for (span, color) in spans {
        let (start, end) = (span.start.max(at), span.end.min(range.end));
        if start >= end {
            continue;
        }
        if at < start {
            runs.push((at..start, HighlightStyle::default()));
        }
        runs.push((start..end, HighlightStyle { color: Some(*color), ..Default::default() }));
        at = end;
    }
    if at < range.end {
        runs.push((at..range.end, HighlightStyle::default()));
    }
    runs
}

/// A command field in the terminal's font, as tall as its lines: the page scrolls, with the app's scrollbar, and a
/// long command scrolls sideways, as in an editor.
fn code_field(theme: &Theme, input: &Entity<EditorState>, cx: &App) -> Div {
    let rows = input.read(cx).value().split('\n').count().max(2);
    let line = (theme.terminal_font_size * theme.terminal_line_height).round();
    div()
        .w_full()
        // under the last line, room for the sideways scrollbar of a long command
        .h(px(rows as f32 * line + 20.))
        .px(px(10.))
        .pt(px(6.))
        .pb(px(14.))
        .rounded(px(8.))
        .bg(theme.wash(0.06))
        .font_family(theme.font_terminal.clone())
        .text_size(px(theme.terminal_font_size))
        .line_height(px(line))
        .child(Editor::new(input))
}

const ABOUT: &str = "Commands herdr runs in a new worktree and after one is removed, per session. They run in herdr, so they keep working with Shepherd closed or deleted, from any client.";

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

/// A project's `dir` (as hooks.conf has it, maybe `~/…`) is the repo at `root`.
fn same_dir(dir: &str, root: &str) -> bool {
    dir == root || dir == tilde(root)
}

impl SettingsView {
    /// Reads `hooks.conf` the first time the section is shown, makes each project's fields, and asks herdr for its
    /// sessions and saved machines (each machine's `hooks.conf` too) and the repos open in each, which get fields too.
    pub(super) fn load_worktrees(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for (machine, p) in std::mem::take(&mut self.worktree_adds) {
            self.add_project(machine, &p, window, cx);
        }
        if self.worktrees.is_some() {
            return;
        }
        match hooks::load() {
            Ok(projects) => {
                self.worktrees = Some(Ok(Vec::new()));
                for p in &projects {
                    self.add_project(None, p, window, cx);
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
                out.push((SessionRepos { machine: None, label: s.socket_path.clone(), name: s.name, running: s.running, repos, error: None }, Vec::new()));
            }
            for m in crate::herdr::machines().unwrap_or_default().into_iter().filter(|m| m.enabled) {
                let (projects, repos, error) = match hooks::remote_load(&m.target, &m.session) {
                    Ok((projects, list)) => (projects, repos_of(&list["result"]), None),
                    Err(e) => (Vec::new(), Vec::new(), Some(format!("{e:#}"))),
                };
                let label = format!("{} on {}", m.session, m.label);
                let session = SessionRepos { machine: Some(m.target), name: m.session, label, running: error.is_none(), repos, error };
                out.push((session, projects));
            }
            out
        });
        cx.spawn(async move |v, cx| {
            let list = list.await.unwrap_or_default();
            let _ = v.update(cx, |v, cx| {
                let looks = &SettingsGlobal::of(cx).settings.place_looks;
                let mut sessions = Vec::new();
                // fields are made at the next render (they need a window): a machine's projects, then each repo open
                // in a session and not yet a project of it, with empty commands
                for (mut s, projects) in list {
                    if s.machine.is_none() {
                        s.label = looks.get(&s.label).and_then(|l| l.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| s.name.clone());
                    }
                    v.worktree_adds.extend(projects.into_iter().map(|p| (s.machine.clone(), p)));
                    for (name, root) in &s.repos {
                        let known = |machine: &Option<String>, session: &str, dir: &str| *machine == s.machine && session == s.name && same_dir(dir, root);
                        let field = matches!(&v.worktrees, Some(Ok(fields)) if fields.iter().any(|f| known(&f.machine, &f.session, &f.dir)));
                        let added = v.worktree_adds.iter().any(|(m, p)| known(m, p.session.as_deref().unwrap_or("default"), &p.dir));
                        if !field && !added {
                            // a machine's paths are its own, not this computer's
                            let dir = if s.machine.is_some() { root.clone() } else { tilde(root) };
                            v.worktree_adds.push((s.machine.clone(), Project { session: Some(s.name.clone()), name: name.clone(), dir, ..Project::default() }));
                        }
                    }
                    sessions.push(s);
                }
                v.worktree_sessions = Some(sessions);
                cx.notify();
            });
        })
        .detach();
    }

    fn add_project(&mut self, machine: Option<String>, p: &Project, window: &mut Window, cx: &mut Context<Self>) {
        let theme = Theme::of(cx).clone();
        let lines = |value: String, placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| {
                let mut input = EditorState::new(window, cx)
                    .language("sh")
                    .line_number(true)
                    .folding(false)
                    // as tall as its lines, nothing to scroll down to: the wheel scrolls the page
                    .scroll_beyond_last_line(Some(0))
                    .cursor_surrounding_lines(Some(0))
                    .soft_wrap(false)
                    .searchable(false)
                    .placeholder(placeholder)
                    .default_value(value);
                input.set_editor_style(input_style(&theme));
                let theme = theme.clone();
                input.set_highlighter_factory(Rc::new(move |_| Some(Box::new(ShellHighlighter { theme: theme.clone(), spans: Vec::new() }) as Box<_>)), cx);
                input
            })
        };
        let created = lines(text(&p.created), r#"ln -sf "$ROOT_PATH/.env" .env"#, window, cx);
        let removed = lines(text(&p.removed), "docker compose down", window, cx);
        // a field grows with its lines as they are typed, and is saved when left
        let on_event = |machine: &Option<String>| {
            let machine = machine.clone();
            move |v: &mut Self, _: &Entity<EditorState>, event: &InputEvent, _: &mut Window, cx: &mut Context<Self>| match event {
                InputEvent::Change => cx.notify(),
                InputEvent::Blur => v.save_worktrees(machine.clone(), cx),
                _ => {}
            }
        };
        let events = [cx.subscribe_in(&created, window, on_event(&machine)), cx.subscribe_in(&removed, window, on_event(&machine))];
        // one without a session, from before sessions or written by hand, becomes the default session's
        let session = p.session.clone().unwrap_or_else(|| "default".into());
        if let Some(Ok(fields)) = &mut self.worktrees {
            fields.push(ProjectFields { machine, session, name: p.name.clone(), dir: p.dir.clone(), created, removed, _events: events });
        }
    }

    /// Writes the projects with commands of `machine` (`None` here) to its `hooks.conf` (installing herdr's plugin
    /// the first time) off the main thread.
    fn save_worktrees(&mut self, machine: Option<String>, cx: &mut Context<Self>) {
        let Some(Ok(fields)) = &self.worktrees else { return };
        let projects: Vec<Project> = fields
            .iter()
            .filter(|f| f.machine == machine)
            .map(|f| Project {
                session: Some(f.session.clone()),
                name: f.name.clone(),
                dir: f.dir.clone(),
                created: commands(&f.created.read(cx).value()),
                removed: commands(&f.removed.read(cx).value()),
            })
            .filter(|p| !p.created.is_empty() || !p.removed.is_empty())
            .collect();
        // a save that a later one overtook writes nothing (Blur, then a bin's click, start two at once)
        // each machine's in turn: a save here overtakes none of a machine's
        let saves = self.worktree_saves.entry(machine.clone()).or_default().clone();
        let this_save = saves.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        let stale = move || saves.load(std::sync::atomic::Ordering::SeqCst) != this_save;
        let saving = cx.background_spawn(async move {
            match &machine {
                Some(target) => hooks::save_remote(target, &projects, stale),
                None => hooks::save(&projects, stale),
            }
        });
        cx.spawn(async move |this, cx| {
            let result = saving.await;
            let _ = this.update(cx, |v, cx| {
                v.worktree_note = result.err().map(|e| format!("Not saved: {e:#}"));
                cx.notify();
            });
        })
        .detach();
    }

    /// Settings → Worktrees: a session picker (herdr's running sessions, and stopped ones with projects, then each
    /// saved machine's; at first the one Shepherd is on), then that session's repos and projects, each its name and path, folded to its count of
    /// commands or open on its two command fields; a project of a repo no longer open in the session has a red bin.
    pub(super) fn worktrees_page(&self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let body = match &self.worktrees {
            None => div(),
            Some(Err(e)) => note(theme, e.clone()),
            Some(Ok(fields)) => {
                // herdr's sessions in its order, then any a project names that herdr no longer has
                let mut groups: Vec<SessionRepos> = self
                    .worktree_sessions
                    .iter()
                    .flatten()
                    .map(|s| SessionRepos { machine: s.machine.clone(), name: s.name.clone(), label: s.label.clone(), running: s.running, repos: s.repos.clone(), error: s.error.clone() })
                    .collect();
                let of = |g: &SessionRepos, f: &ProjectFields| f.machine == g.machine && f.session == g.name;
                for f in fields {
                    if !groups.iter().any(|g| of(g, f)) {
                        let label = match &f.machine {
                            Some(target) => format!("{} on {target}", f.session),
                            None => f.session.clone(),
                        };
                        groups.push(SessionRepos { machine: f.machine.clone(), name: f.session.clone(), label, running: false, repos: Vec::new(), error: None });
                    }
                }
                groups.retain(|g| g.running || g.error.is_some() || fields.iter().any(|f| of(g, f)));
                // at first the place shown: the machine's session, or this computer's
                let shown = self.worktree_session.clone().or_else(|| match crate::herdr::shown_machine() {
                    Some(target) => groups.iter().find(|g| g.machine.as_ref() == Some(&target)).map(|g| (g.machine.clone(), g.name.clone())),
                    None => crate::herdr::current_session().map(|name| (None, name)),
                });
                let at = groups.iter().position(|g| shown.as_ref().is_some_and(|(m, name)| *m == g.machine && *name == g.name)).unwrap_or(0);
                let picker = {
                    let labels = groups
                        .iter()
                        .map(|g| SharedString::from(match (g.running, &g.error) {
                            (_, Some(_)) => format!("{} (unreachable)", g.label),
                            (false, _) => format!("{} (not running)", g.label),
                            _ => g.label.clone(),
                        }))
                        .collect();
                    let names: Vec<(Option<String>, String)> = groups.iter().map(|g| (g.machine.clone(), g.name.clone())).collect();
                    let weak = cx.entity().downgrade();
                    // the select calls this while it updates the view: the change waits until it is done
                    let pick = move |i: usize, cx: &mut App| {
                        let (weak, name) = (weak.clone(), names.get(i).cloned());
                        cx.defer(move |cx| {
                            let _ = weak.update(cx, |v, cx| {
                                v.worktree_session = name;
                                cx.notify();
                            });
                        });
                    };
                    self.select(theme, "worktree-session", labels, Some(at), "Session".into(), |_| None, pick, cx).w(px(220.))
                };
                let Some(group) = groups.into_iter().nth(at) else {
                    return div().child(page_header(theme, "Worktrees", ABOUT)).child(note(theme, "No herdr session answered yet."));
                };
                let rows: Vec<Div> = fields
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| of(&group, f))
                    .enumerate()
                    .map(|(n, (i, f))| {
                        // a repo open in the session stays listed; a project of one no longer open can go
                        let gone = !group.repos.iter().any(|(_, root)| same_dir(&f.dir, root));
                        let machine = f.machine.clone();
                        let remove = gone.then(|| {
                            remove_button(theme, format!("worktree-remove-{i}")).mt(px(0.)).on_click(cx.listener(move |v, _, _, cx| {
                                if let Some(Ok(fields)) = &mut v.worktrees {
                                    fields.remove(i);
                                }
                                v.save_worktrees(machine.clone(), cx);
                                cx.notify();
                            }))
                        });
                        let count = commands(&f.created.read(cx).value()).len() + commands(&f.removed.read(cx).value()).len();
                        let count = match count {
                            0 => "No commands".to_string(),
                            1 => "1 command".to_string(),
                            n => format!("{n} commands"),
                        };
                        let key = format!("{}/{}/{}", f.machine.as_deref().unwrap_or(""), f.session, f.name);
                        let open = self.worktree_open.contains(&key);
                        let toggle = cx.listener(move |v, _, _, cx| {
                            if !v.worktree_open.remove(&key) {
                                v.worktree_open.insert(key.clone());
                            }
                            cx.notify();
                        });
                        let chevron = if open { "icons/alt-arrow-down.svg" } else { "icons/alt-arrow-right.svg" };
                        let path = if gone { format!("{} · not open in this session", f.dir) } else { f.dir.clone() };
                        let head = div()
                            .id(("worktree-project", i))
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .cursor_pointer()
                            .on_click(toggle)
                            .child(gpui::svg().path(chevron).size(px(12.)).flex_none().text_color(theme.text_muted))
                            .child(row_text(theme, f.name.clone(), path).flex_1().min_w_0())
                            .child(div().flex_none().text_color(theme.text_faint).child(count));
                        div()
                            .mx(px(16.))
                            .py(px(14.))
                            .when(n > 0, |el| el.border_t_1().border_color(theme.border.opacity(0.6)))
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .child(div().flex().items_center().gap(px(12.)).child(head).children(remove))
                            .when(open, |el| {
                                el.child(labelled(theme, "After creating a worktree, in it", code_field(theme, &f.created, cx)))
                                    .child(labelled(theme, "After removing one, in the main checkout", code_field(theme, &f.removed, cx)))
                            })
                    })
                    .collect();
                let empty = rows.is_empty().then(|| match &group.error {
                    Some(e) => card_row(theme, true).child(row_text(theme, "Cannot reach the machine", e.clone())),
                    None => card_row(theme, true).child(row_text(theme, "No repos", "No workspace open in this session is a git repo: open one in herdr and it shows here.")),
                });
                div()
                    .child(div().mt(px(24.)).mb(px(8.)).child(picker))
                    .child(card(theme).mt(px(0.)).children(rows).children(empty))
                    .child(note(theme, "One command a line, run in order; the first that fails stops the rest. $ROOT_PATH is the main checkout, $WORKTREE_PATH the worktree, $PROJECT the project's name."))
                    // what to give an agent asked to set them up
                    .children(std::env::current_exe().ok().map(|exe| note(theme, format!("An agent can set them up too: \"{}\" hooks --help", exe.display()))))
            }
        };
        div()
            .child(page_header(
                theme,
                "Worktrees",
                ABOUT,
            ))
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
    fn highlight_runs_cover_the_range_without_gaps() {
        let red = gpui::red();
        let got: Vec<_> = runs(&[(2..4, red), (6..9, red)], &(3..8)).into_iter().map(|(r, s)| (r, s.color.is_some())).collect();
        assert_eq!(got, [(3..4, true), (4..6, false), (6..8, true)]);
        assert_eq!(runs(&[], &(0..3)).len(), 1);
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
