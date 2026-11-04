//! The review panel on the right, after Superconductor's: the shown pane's branch's PR or MR (title, state, author,
//! description) and its checks or pipeline jobs, each opening in the browser.
use super::*;
use gpui::AnyElement;
use crate::ui::review::{Check, CheckState, Request, Review, act, check_icon, duration, markdown, read_review, retry_failed};
use crate::ui::statusbar::{branch_of, cached, fetching, pane_dir, refetch};

/// How often gh or glab are asked again about a branch, as the status bar asks about its PR.
const REVIEW_TTL: Duration = Duration::from_secs(120);
/// How often while one of its checks runs or waits to.
pub(super) const REVIEW_POLL: Duration = Duration::from_secs(10);
/// Past this a swipe over the checks pages when the fingers lift; short of it, the page springs back.
const PAGE_COMMIT: f32 = 60.;
/// How long after a retry, play or cancel before asking again: the host takes a moment to queue it.
const SETTLE: Duration = Duration::from_secs(3);

/// The shown pane's branch and its review, for the panel and the status bar.
#[derive(Default)]
pub(super) struct ShownReview {
    pub dir: Option<String>,
    pub branch: Option<String>,
    /// The review cache's key: the directory and the branch, a line apart.
    pub key: Option<String>,
    pub review: Option<Review>,
}

/// The review panel's tabs under a PR's head.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Tab {
    #[default]
    Summary,
    Timeline,
    Code,
    Checks,
}

impl Tab {
    /// `summary`, `timeline`, `code`, `checks`.
    fn of_name(name: &str) -> Option<Tab> {
        [Tab::Summary, Tab::Timeline, Tab::Code, Tab::Checks].into_iter().find(|t| format!("{t:?}").eq_ignore_ascii_case(name))
    }
}

/// A merge the panel asks about before it runs.
#[derive(Clone)]
pub(super) struct MergeAsk {
    pub dir: String,
    pub key: String,
    pub gitlab: bool,
    pub number: u64,
    /// `#12 Fix it`.
    pub label: String,
    pub base: String,
    /// Merges once the pipeline passes, not now.
    pub auto: bool,
    pub squash: bool,
    pub remove_branch: bool,
}

impl Shell {
    pub fn toggle_review(&mut self, window: &Window, cx: &mut Context<Self>) {
        let seen = self.right_panel_shown(window);
        self.update_panel(cx, |p| p.toggle_pull_request(seen));
    }

    /// Shows the review panel's tab `name` (`summary`, `timeline`, `code`, `checks`), or with `menu` its "…" menu
    /// under the panel's top right corner, for screenshots.
    pub fn show_review_tab(&mut self, name: &str, window: &Window, cx: &mut Context<Self>) {
        if name == "menu" {
            let size = window.viewport_size();
            self.review_menu = Some(crate::ui::Popup::new(gpui::point(size.width - px(290.), px(112.))));
        } else if let Some(tab) = Tab::of_name(name) {
            self.review_tab = tab;
        }
        cx.notify();
    }

    /// Asks gh or glab again about the branch the panel shows while its checks run; called every `REVIEW_POLL`.
    pub(super) fn poll_review(&mut self, cx: &mut Context<Self>) {
        if let Some(key) = self.review_poll.clone() {
            refetch::<Review>(&key, cx);
        }
    }

    /// The shown pane's branch's review, asked again while its checks run (also with the window in the background,
    /// so their end is noticed); when they stop, says how CI went.
    pub(super) fn shown_review(&mut self, snapshot: Option<&Snapshot>, window: &mut Window, cx: &mut Context<Self>) -> ShownReview {
        let pane = self.active_pane(window, cx);
        let dir = pane_dir(snapshot, pane.as_deref());
        let branch = dir.as_deref().and_then(|d| branch_of(d, cx));
        let key = dir.clone().zip(branch.clone()).map(|(d, b)| format!("{d}\n{b}"));
        let ttl = if window.is_window_active() { REVIEW_TTL } else { Duration::MAX };
        let review = key.as_deref().and_then(|k| cached(k, ttl, read_review, cx));
        let running = review.as_ref().is_some_and(Review::running);
        self.review_poll = key.clone().filter(|_| running);
        if let (Some(k), Some(r)) = (&key, &review) {
            let last = self.review_last.replace((k.clone(), running));
            if !running && last.as_ref().is_some_and(|(l, ran)| l == k && *ran) {
                self.ci_finished(r, branch.as_deref().unwrap_or_default(), pane.as_deref(), window, cx);
            }
        }
        ShownReview { dir, branch, key, review }
    }

    /// A toast when the window is key, else a notification, saying how the shown branch's CI ended.
    fn ci_finished(&mut self, review: &Review, branch: &str, pane: Option<&str>, window: &Window, cx: &mut Context<Self>) {
        let (title, failed) = match review.ci() {
            Some(CheckState::Failed) => ("CI failed", true),
            Some(CheckState::Success) => ("CI passed", false),
            _ => ("CI finished", false),
        };
        if window.is_window_active() {
            let message = format!("{title} on {branch}");
            if failed {
                crate::ui::toast::error(cx, message);
            } else {
                crate::ui::toast::info(cx, message, None);
            }
        } else {
            let sound = SettingsGlobal::of(cx).settings.notify_sound;
            crate::notify::show(cx, pane.unwrap_or_default(), title.into(), branch.into(), sound, None);
        }
    }

    /// Runs `job` (CLI calls) off the main thread under `busy` (a spinner on its button); a failure's line goes to a
    /// toast. The review at `key` is asked again once the host has had a moment.
    pub(super) fn review_act(&mut self, busy: String, key: String, job: impl FnOnce() -> Result<(), String> + Send + 'static, cx: &mut Context<Self>) {
        if !self.review_acting.insert(busy.clone()) {
            return;
        }
        cx.notify();
        cx.spawn(async move |this, cx| {
            let done = cx.background_spawn(async move { job() }).await;
            if let Err(e) = &done {
                let e = e.clone();
                let _ = this.update(cx, |_, cx| crate::ui::toast::error(cx, e));
            } else {
                cx.background_executor().timer(SETTLE).await;
            }
            let _ = this.update(cx, |s, cx| {
                s.review_acting.remove(&busy);
                refetch::<Review>(&key, cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// The panel, and its edge: drag to resize, double-click for the default width.
    pub(super) fn render_review(&mut self, theme: &Theme, shown: &ShownReview, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let panel_width = self.right_panel_drawn(window);
        let (key, review, branch) = (shown.key.clone(), shown.review.clone(), shown.branch.clone());
        // the PR's timeline and diff are cached by its number, apart from the branch's review
        let pr_key = match (&shown.dir, &review) {
            (Some(dir), Some(Review::Found { request: Some(r), .. })) => Some(format!("{dir}\n{}\n{}", r.pr.gitlab, r.pr.number)),
            _ => None,
        };
        let busy = key.as_deref().is_some_and(|k| fetching::<Review>(k, cx))
            || pr_key.as_deref().is_some_and(|k| match self.review_tab {
                Tab::Timeline => fetching::<Timeline>(k, cx),
                Tab::Code => fetching::<PrDiff>(k, cx),
                _ => false,
            });

        let gitlab = matches!(&review, Some(Review::Found { request: Some(r), .. }) if r.pr.gitlab)
            || matches!(&review, Some(Review::Found { pipeline: Some(_), .. }) | Some(Review::NoTool("glab") | Review::NoLogin("glab", _)));
        let noun = if gitlab { "merge request" } else { "pull request" };
        let noun_title = match &review {
            Some(Review::Found { request: None, pipeline: Some(_), .. }) => "Pipeline",
            _ if gitlab => "Merge request",
            _ => "Pull request",
        };
        let header = crate::ui::surface_chrome::toolbar(theme)
            .child(div().flex_1().min_w_0().truncate().font_weight(gpui::FontWeight::MEDIUM).child(noun_title))
            .children(key.clone().map(|key| {
                if busy {
                    div().size(px(24.)).flex().items_center().justify_center().child(crate::ui::loader::mini_spinner(2., theme.text_faint, window)).into_any_element()
                } else {
                    titlebar_button(theme, "review-refresh", "icons/solar/refresh.svg")
                        .tooltip(tip("Refresh", None))
                        .on_click(cx.listener({
                            let pr_key = pr_key.clone();
                            move |_, _: &ClickEvent, _, cx| {
                                refetch::<Review>(&key, cx);
                                if let Some(k) = &pr_key {
                                    refetch::<Timeline>(k, cx);
                                    refetch::<PrDiff>(k, cx);
                                }
                            }
                        }))
                        .into_any_element()
                }
            }));
        let (dir, key_or) = (shown.dir.clone().unwrap_or_default(), key.clone().unwrap_or_default());

        let mut footer = None;
        // where the "…" menu was clicked and the PR it is about
        let mut menu: Option<(gpui::Point<Pixels>, Request)> = None;
        // the PR's head and its tabs, above the body that scrolls
        let mut top = None;
        let body: Vec<AnyElement> = match (&key, review) {
            (None, _) => vec![empty(theme, "No branch", "The focused pane isn't in a git repo.")],
            (Some(_), None) => vec![loading(window)],
            (Some(_), Some(Review::NoRemote)) => vec![empty(theme, "No remote", "This repo has no origin to ask about.")],
            (Some(_), Some(Review::NoTool(tool))) => vec![empty(theme, format!("{tool} isn't installed"), format!("Install it to see the {noun} here: brew install {tool}"))],
            (Some(_), Some(Review::NoLogin(tool, host))) => {
                vec![empty(theme, format!("{tool} isn't logged in"), format!("Run {tool} auth login --hostname {host} in a pane, then refresh."))]
            }
            (Some(_), Some(Review::Failed(why))) => vec![empty(theme, format!("Couldn't load the {noun}"), why)],
            (Some(_), Some(Review::Found { request, checks, pipeline })) => {
                let mut out = Vec::new();
                let checks_title = if gitlab { "Pipeline" } else { "Checks" };
                let ci = ci_line(&checks, pipeline.as_ref().map(|(_, s)| *s), gitlab);
                let checks_el = (!checks.is_empty() || pipeline.is_some()).then(|| {
                    let this = cx.entity().downgrade();
                    let (dir, key) = (dir.clone(), key_or.clone());
                    let on_act: OnAct = Rc::new(move |busy, calls, cx| {
                        let dir = dir.clone();
                        let job = move || calls.iter().try_for_each(|c| act(&dir, c));
                        let _ = this.update(cx, |s, cx| s.review_act(busy, key.clone(), job, cx));
                    });
                    let pages = crate::ui::review::pages(&checks);
                    let picked = self.review_page.as_ref().filter(|(k, _)| *k == key_or).and_then(|(_, name)| pages.iter().position(|(n, _)| n == name));
                    let page = picked.unwrap_or_else(|| crate::ui::review::default_page(&pages));
                    let this = cx.entity().downgrade();
                    let names: Vec<String> = pages.iter().map(|(n, _)| n.clone()).collect();
                    // the panel's whole width: pages slide in from its edge
                    let width = panel_width;
                    let on_page: OnPage = Rc::new({
                        let (names, key) = (names.clone(), key_or.clone());
                        move |to, cx| {
                            let _ = this.update(cx, |s, cx| s.page_to(&key, &names, page, to, width, cx));
                        }
                    });
                    let (offset, beside) = self.review_offset(&names, page, width, window);
                    // the log asked for, read again while its job runs
                    let shown_log = self.review_log.as_ref().filter(|(k, _)| *k == key_or).map(|(_, job)| job.clone());
                    let log = shown_log.and_then(|job| {
                        let c = checks.iter().find(|c| c.job.as_deref() == Some(job.as_str()))?;
                        let log_key = format!("{}\n{gitlab}\n{job}\n{}", shown.dir.as_deref().unwrap_or_default(), c.repo.as_deref().unwrap_or_default());
                        let busy = matches!(c.state, CheckState::Running | CheckState::Pending);
                        let ttl = if busy { Duration::from_secs(5) } else { Duration::MAX };
                        Some((job, cached(&log_key, ttl, crate::ui::review::read_log, cx)))
                    });
                    let this = cx.entity().downgrade();
                    let on_log: Rc<dyn Fn(String, &mut App)> = Rc::new({
                        let key = key_or.clone();
                        move |job, cx| {
                            let _ = this.update(cx, |s, cx| {
                                let open = s.review_log.as_ref().is_some_and(|(k, j)| *k == key && *j == job);
                                s.review_log = (!open).then(|| (key.clone(), job));
                                cx.notify();
                            });
                        }
                    });
                    let paging = Paging { pages, page, on_page, offset, beside, width, log, on_log };
                    let section = checks_section(theme, checks_title, &checks, pipeline.clone(), gitlab, &self.review_acting, on_act, paging, window);
                    let key = key_or.clone();
                    div()
                        .on_scroll_wheel(cx.listener(move |this, e: &gpui::ScrollWheelEvent, _, cx| this.review_scroll(e, &key, &names, page, width, cx)))
                        .child(section)
                        .into_any_element()
                });
                match &request {
                    Some(r) => {
                        let on_approve = cx.listener({
                            let (dir, key, number, approved) = (dir.clone(), key_or.clone(), r.pr.number, r.approved);
                            move |this, _: &ClickEvent, _, cx| {
                                let dir = dir.clone();
                                // an approved GitLab MR's button takes it back
                                let job = move || if approved { crate::ui::review::revoke(&dir, number) } else { crate::ui::review::approve(&dir, gitlab, number) };
                                this.review_act("approve".into(), key.clone(), job, cx)
                            }
                        });
                        let running = checks.iter().any(|c| matches!(c.state, CheckState::Running | CheckState::Pending))
                            || pipeline.as_ref().is_some_and(|(_, st)| matches!(st, CheckState::Running | CheckState::Pending));
                        let on_cancel = cx.listener({
                            let (dir, key, number) = (dir.clone(), key_or.clone(), r.pr.number);
                            move |this, _: &ClickEvent, _, cx| {
                                let dir = dir.clone();
                                this.review_act("merge".into(), key.clone(), move || crate::ui::review::cancel_auto_merge(&dir, gitlab, number), cx)
                            }
                        });
                        let on_merge = cx.listener({
                            let ask = MergeAsk {
                                dir: dir.clone(),
                                key: key_or.clone(),
                                gitlab,
                                number: r.pr.number,
                                label: format!("{}{} {}", if gitlab { "!" } else { "#" }, r.pr.number, r.title),
                                base: r.base.clone(),
                                auto: running,
                                squash: r.squash,
                                remove_branch: r.remove_branch,
                            };
                            move |this, _: &ClickEvent, _, cx| {
                                this.merge_confirm = Some(crate::ui::Popup::new(ask.clone()));
                                cx.notify();
                            }
                        });
                        let on_ready = cx.listener({
                            let (dir, key, number) = (dir.clone(), key_or.clone(), r.pr.number);
                            move |this, _: &ClickEvent, _, cx| {
                                let dir = dir.clone();
                                this.review_act("ready".into(), key.clone(), move || crate::ui::review::mark_ready(&dir, gitlab, number), cx)
                            }
                        });
                        footer = request_footer(theme, r, running, &self.review_acting, on_approve, on_merge, on_cancel, on_ready);
                        let tab = self.review_tab;
                        let pick = |to: Tab| -> Listener {
                            Box::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.review_tab = to;
                                cx.notify();
                            }))
                        };
                        let tabs = tab_bar(theme, tab, checks_title, ci, &pick, window);
                        let menu_open = self.review_menu.as_ref().is_some_and(|m| m.is_open());
                        let menu_button = titlebar_button(theme, "review-menu", "icons/more-horizontal.svg")
                            .when(menu_open, |el| el.bg(theme.hover))
                            .tooltip(tip("More", None))
                            .on_click(cx.listener(|this, e: &ClickEvent, _, cx| {
                                this.review_menu = Some(crate::ui::Popup::new(e.position()));
                                cx.notify();
                            }))
                            .into_any_element();
                        top = Some(div().flex_none().flex().flex_col().child(request_head(theme, r, menu_button)).child(tabs).into_any_element());
                        menu = self.review_menu.as_ref().map(|m| (m.value, r.clone()));
                        let pr_key = pr_key.clone().unwrap_or_default();
                        match tab {
                            Tab::Summary => {
                                out.push(summary(theme, r));
                                if !r.body.trim().is_empty() {
                                    let toggle = cx.listener(|this, _: &ClickEvent, _, cx| {
                                        this.review_collapsed = !this.review_collapsed;
                                        cx.notify();
                                    });
                                    out.push(description(theme, &r.body, self.review_collapsed, toggle));
                                }
                            }
                            Tab::Timeline => out.push(match cached(&pr_key, REVIEW_TTL, crate::ui::review::read_timeline, cx) {
                                None => loading(window),
                                Some(Err(why)) => empty(theme, "Couldn't load the timeline", why),
                                Some(Ok(events)) if events.is_empty() => empty(theme, "Nothing yet", "No comments, reviews or commits."),
                                Some(Ok(events)) => timeline(theme, &events),
                            }),
                            Tab::Code => out.push(match cached(&pr_key, REVIEW_TTL, crate::ui::review::read_diff, cx) {
                                None => loading(window),
                                Some(Err(why)) => empty(theme, "Couldn't load the diff", why),
                                Some(Ok((files, _))) if files.is_empty() => empty(theme, "No changes", format!("This {noun} changes no files.")),
                                Some(Ok((files, cut))) => {
                                    let url = r.pr.url.clone();
                                    // a small PR's files show unfolded; a click flips a file from what it was
                                    let unfolded = files.len() <= SMALL_PR;
                                    let open: Vec<bool> = files.iter().map(|f| unfolded != self.review_files.contains(&format!("{url}\n{}", f.path))).collect();
                                    let on_file: Rc<dyn Fn(String, &mut App)> = Rc::new({
                                        let this = cx.entity().downgrade();
                                        move |path, cx| {
                                            let _ = this.update(cx, |s, cx| {
                                                let key = format!("{url}\n{path}");
                                                if !s.review_files.remove(&key) {
                                                    s.review_files.insert(key);
                                                }
                                                cx.notify();
                                            });
                                        }
                                    });
                                    code(theme, &files, &open, cut, &r.pr, on_file)
                                }
                            }),
                            Tab::Checks => out.push(checks_el.unwrap_or_else(|| empty(theme, format!("No {}", checks_title.to_lowercase()), "Nothing has run on this branch yet."))),
                        }
                    }
                    None => {
                        out.push(empty(theme, format!("No {noun}"), format!("{} has none yet.", branch.unwrap_or_default())));
                        out.extend(checks_el);
                    }
                }
                out
            }
        };

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(header)
            .children(top)
            .child(
                div()
                    .id("review-body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    // one reset for the description's and the logs' selectable text, before either paints
                    .child(crate::ui::markdown::frame_reset())
                    .children(body),
            )
            .children(footer)
            .children(menu.and_then(|(at, r)| self.render_review_menu(theme, at, &r, pr_key.clone(), key.clone(), window, cx)))
            .into_any_element()
    }

    /// The agent the review panel's "…" menu talks to: the shown pane's, else the workspace's first.
    fn review_agent(&self, window: &mut Window, cx: &mut Context<Self>) -> Option<crate::model::Pane> {
        let workspace = self.shown_tab()?.workspace_id;
        let pane = self.active_pane(window, cx);
        let agents = self.snapshot.as_ref()?.agents_in(&workspace, SettingsGlobal::of(cx).settings.annotation_agents);
        crate::annotations::default_recipient(&agents, pane.as_deref(), Some(&workspace)).cloned()
    }

    /// Puts `text` in the agent's prompt, without sending it, and gives it the keyboard; an agent whose pane isn't
    /// shown gets it on the clipboard instead.
    fn ask_agent(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(agent) = self.review_agent(window, cx) else { return crate::ui::toast::error(cx, "No agent here to ask") };
        let name = agent.agent.clone().unwrap_or_default();
        match self.panes.get(&agent.pane_id).cloned() {
            Some(pane) => {
                pane.update(cx, |pane, cx| pane.paste(&text, cx));
                self.release_panel();
                window.focus(&pane.focus_handle(cx), cx);
            }
            None => {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                crate::ui::toast::info(cx, format!("Copied: paste it to {name}"), None);
            }
        }
        cx.notify();
    }

    /// Sends `prompt` to the agent, as the diff's comments go.
    fn tell_agent(&mut self, prompt: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(agent) = self.review_agent(window, cx) else { return crate::ui::toast::error(cx, "No agent here to send it to") };
        let name = agent.agent.clone().unwrap_or_default();
        let (backend, pane) = (self.backend.clone(), agent.pane_id);
        self.ask(async move { backend.mux.send(pane, Some(prompt)).await }, cx, move |_, result, cx| match result {
            Ok(_) => crate::ui::toast::info(cx, format!("Sent to {name}"), None),
            Err(e) => crate::ui::toast::error(cx, format!("Not sent to {name}: {e:#}")),
        });
    }

    /// The "…" menu: the agent's questions about the PR, then refresh, open and copy.
    #[allow(clippy::too_many_arguments)]
    fn render_review_menu(
        &mut self,
        theme: &Theme,
        at: gpui::Point<Pixels>,
        r: &Request,
        pr_key: Option<String>,
        key: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        type Act = Box<dyn Fn(&mut Shell, &mut Window, &mut Context<Shell>)>;
        let menu = self.review_menu.as_ref()?;
        let agent = self.review_agent(window, cx).and_then(|a| a.agent);
        let prompts = review_prompts(r);
        let host = if r.pr.gitlab { "GitLab" } else { "GitHub" };
        let noun = if r.pr.gitlab { "MR" } else { "PR" };
        let number = format!("{}{}", if r.pr.gitlab { "!" } else { "#" }, r.pr.number);
        let item = |id: &'static str, icon: &'static str, label: String, hint: Option<String>, off: bool, act: Act| {
            let hover = theme.hover;
            div()
                .id(id)
                .px(px(8.))
                .py(px(5.))
                .rounded(px(6.))
                .flex()
                .items_start()
                .gap(px(8.))
                .child(div().h(px(18.)).flex().items_center().child(menu_mark(icon, theme.text_muted)))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .child(div().line_height(px(18.)).text_color(theme.text).child(label))
                        .children(hint.map(|h| div().text_size(px(11.)).line_height(px(15.)).text_color(theme.text_faint).child(h))),
                )
                .map(|el| {
                    if off {
                        el.opacity(0.45)
                    } else {
                        el.cursor_pointer().hover(move |s| s.bg(hover)).on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            if let Some(m) = this.review_menu.as_mut() {
                                m.close();
                            }
                            act(this, window, cx);
                            cx.notify();
                        }))
                    }
                })
        };
        let to = agent.clone().unwrap_or_else(|| "the agent".into());
        let no_agent = agent.is_none();
        let (ask, explain, fix) = prompts;
        let (url, url2) = (r.pr.url.clone(), r.pr.url.clone());
        let checkout = format!("{} checkout {}", if r.pr.gitlab { "glab mr" } else { "gh pr" }, r.pr.number);
        let copy = |text: String, said: &'static str| -> Act {
            Box::new(move |_, _, cx| {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.clone()));
                crate::ui::toast::info(cx, said, None);
            })
        };
        let card = menu_card(theme, "review-menu-card", 260.)
            .max_w(px(320.))
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if let Some(m) = this.review_menu.as_mut() {
                    m.close();
                }
                cx.notify();
            }))
            .flex()
            .flex_col()
            .gap(px(1.))
            .text_size(px(12.))
            .child(item(
                "review-menu-refresh",
                "icons/solar/refresh.svg",
                "Refresh".into(),
                None,
                false,
                Box::new(move |_, _, cx| {
                    if let Some(k) = &key {
                        refetch::<Review>(k, cx);
                    }
                    if let Some(k) = &pr_key {
                        refetch::<Timeline>(k, cx);
                        refetch::<PrDiff>(k, cx);
                    }
                }),
            ))
            .child(item(
                "review-menu-ask",
                "icons/lucide-message-circle-question.svg",
                "Ask a question".into(),
                Some(format!("Puts the {noun} in {to}'s prompt.")),
                no_agent,
                Box::new(move |this, window, cx| this.ask_agent(ask.clone(), window, cx)),
            ))
            .child(item(
                "review-menu-explain",
                "icons/solar/document-text.svg",
                format!("Explain this {noun}"),
                Some("A walk through the diff and what to read closely.".into()),
                no_agent,
                Box::new(move |this, window, cx| this.tell_agent(explain.clone(), window, cx)),
            ))
            .child(item(
                "review-menu-fix",
                "icons/solar/pen.svg",
                "Fix review findings".into(),
                Some("Asks for its comments and failing checks to be dealt with.".into()),
                no_agent,
                Box::new(move |this, window, cx| this.tell_agent(fix.clone(), window, cx)),
            ))
            .child(div().mx(px(-4.)).my(px(4.)).h(px(1.)).bg(theme.border))
            .child(item("review-menu-open", "icons/solar/square-arrow-right-up.svg", format!("Open on {host}"), None, false, Box::new(move |_, _, cx| cx.open_url(&url))))
            .child(item("review-menu-link", "icons/solar/link.svg", "Copy link".into(), None, false, copy(url2, "Copied the link")))
            .child(item("review-menu-number", "icons/solar/copy.svg", format!("Copy {noun} number"), None, false, copy(number, "Copied the number")))
            .child(item("review-menu-checkout", "icons/terminal.svg", "Copy checkout command".into(), None, false, copy(checkout, "Copied the checkout command")));
        let card = crate::ui::menu_motion("review-menu-in", menu, card);
        Some(gpui::deferred(gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(card)).with_priority(1).into_any_element())
    }
}

/// What the "…" menu gives the agent about `r`: the question's start, left to finish, and the requests to explain it
/// and to deal with its findings.
fn review_prompts(r: &Request) -> (String, String, String) {
    let (noun, cli) = if r.pr.gitlab { ("merge request", "glab mr") } else { ("pull request", "gh pr") };
    let number = format!("{}{}", if r.pr.gitlab { "!" } else { "#" }, r.pr.number);
    let which = format!("{noun} {number} \"{}\" ({})", r.title, r.pr.url);
    let ask = format!("About {which}: ");
    let explain = format!(
        "Explain {which}. Read it with `{cli} view {n}` and its diff with `{cli} diff {n}`, then walk me through the change: what it does, \
         in what order to read it, and which parts deserve a close look.",
        n = r.pr.number
    );
    let comments = if r.pr.gitlab { format!("`glab mr view {} --comments`", r.pr.number) } else { format!("`gh pr view {} --comments` and `gh api repos/{{owner}}/{{repo}}/pulls/{}/comments`", r.pr.number, r.pr.number) };
    let checks = if r.pr.gitlab { "`glab ci status`".to_string() } else { format!("`gh pr checks {}`", r.pr.number) };
    let fix = format!(
        "Deal with the review findings on {which}: read its comments ({comments}) and its failing checks ({checks}), \
         fix in this checkout what should be fixed, and tell me what you left and why."
    );
    (ask, explain, fix)
}

/// A click handler handed to a tab.
type Listener = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
/// The Timeline tab's cache entry: a PR's events, or why they couldn't be read.
type Timeline = Result<Vec<crate::ui::review::Event>, String>;
/// The Code tab's: a PR's files and whether the diff was cut short, or why it couldn't be read.
type PrDiff = Result<(Vec<crate::changes::FileDiff>, bool), String>;

fn loading(window: &mut Window) -> AnyElement {
    div().pt(px(24.)).flex().justify_center().child(crate::ui::loader::spinner(2.5, window)).into_any_element()
}

fn empty(theme: &Theme, title: impl Into<SharedString>, hint: impl Into<SharedString>) -> AnyElement {
    div()
        .pt(px(24.))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.))
        .text_center()
        .child(div().text_color(theme.text).child(title.into()))
        .child(div().text_size(px(12.)).text_color(theme.text_muted).child(hint.into()))
        .into_any_element()
}

/// `body`'s blocks, parsed once while it stays around: a drag over the text redraws every move, and the timeline
/// shows a body per comment.
fn parsed(body: &str) -> Rc<Vec<crate::ui::review::Block>> {
    thread_local! {
        static SEEN: std::cell::RefCell<std::collections::HashMap<String, Rc<Vec<crate::ui::review::Block>>>> = Default::default();
    }
    SEEN.with(|seen| {
        let mut seen = seen.borrow_mut();
        if let Some(blocks) = seen.get(body) {
            return blocks.clone();
        }
        // a PR's worth, more or less; past that start over rather than keep every body ever shown
        if seen.len() > 256 {
            seen.clear();
        }
        let blocks = Rc::new(markdown(body));
        seen.insert(body.to_string(), blocks.clone());
        blocks
    })
}

/// The description under a heading that folds it away.
fn description(theme: &Theme, body: &str, collapsed: bool, toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> AnyElement {
    let text = theme.text;
    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(
            div()
                .id("review-description")
                .flex()
                .items_center()
                .gap(px(4.))
                .text_size(px(12.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text_muted)
                .cursor_pointer()
                .hover(move |s| s.text_color(text))
                .child("Description")
                .child(gpui::svg().path(if collapsed { "icons/alt-arrow-right.svg" } else { "icons/alt-arrow-down.svg" }).size(px(12.)).text_color(theme.text_faint))
                .on_click(toggle),
        )
        .when(!collapsed, |el| {
            el.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .text_color(theme.text)
                    .children(crate::ui::markdown::render(&parsed(body), "review-md", theme)),
            )
        })
        .into_any_element()
}

impl Shell {
    /// Pages the checks from `page` to `to` of `names` (the branch at `key`'s), the new page sliding in from the
    /// side it lies on, the old one going out the other.
    pub(super) fn page_to(&mut self, key: &str, names: &[String], page: usize, to: usize, width: f32, cx: &mut Context<Self>) {
        let Some(name) = names.get(to).filter(|_| to != page) else { return };
        self.review_page = Some((key.to_string(), name.clone()));
        self.review_spring = Some((if to > page { width } else { -width }, Instant::now(), names.get(page).cloned()));
        cx.notify();
    }

    /// One event of a scroll over the checks, as the sidebar takes a swipe between places: across, the page follows
    /// the fingers with its neighbour beside it, and when they lift far enough that one comes in; else it springs
    /// back. Momentum after the lift is ignored.
    fn review_scroll(&mut self, e: &gpui::ScrollWheelEvent, key: &str, names: &[String], page: usize, width: f32, cx: &mut Context<Self>) {
        use gpui::TouchPhase;
        if names.len() < 2 {
            return;
        }
        let gpui::ScrollDelta::Pixels(d) = e.delta else { return };
        let (dx, dy) = (f32::from(d.x), f32::from(d.y));
        match e.touch_phase {
            TouchPhase::Started => self.review_swipe = Some((None, 0.)),
            TouchPhase::Moved => {
                let Some((across, travel)) = self.review_swipe.as_mut() else { return };
                if across.is_none() && dx.hypot(dy) > 2. {
                    *across = Some(dx.abs() > dy.abs() * 1.5);
                }
                if *across == Some(true) {
                    *travel += dx;
                    self.review_spring = None;
                    cx.stop_propagation();
                    cx.notify();
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let Some((Some(true), travel)) = self.review_swipe.take() else { return };
                let offset = page_offset(travel, neighbour(names.len(), page, travel).is_some(), width);
                match neighbour(names.len(), page, travel).filter(|_| travel.abs() > PAGE_COMMIT) {
                    Some(to) => {
                        self.review_page = Some((key.to_string(), names[to].clone()));
                        // the new page carries on from where it was beside the old one
                        let from = if travel < 0. { width + offset } else { offset - width };
                        self.review_spring = Some((from, Instant::now(), names.get(page).cloned()));
                    }
                    None => self.review_spring = Some((offset, Instant::now(), None)),
                }
                cx.notify();
            }
        }
    }

    /// The shown page's offset now (under the fingers, springing home, else 0) and the page beside it with its
    /// offset: the one a swipe goes to, or the one just left.
    fn review_offset(&mut self, names: &[String], page: usize, width: f32, window: &mut Window) -> (f32, Option<(usize, f32)>) {
        if let Some((Some(true), travel)) = self.review_swipe {
            let near = neighbour(names.len(), page, travel);
            let offset = page_offset(travel, near.is_some(), width);
            return (offset, near.map(|n| (n, if travel < 0. { offset + width } else { offset - width })));
        }
        let Some((from, at, left)) = self.review_spring.clone() else { return (0., None) };
        let t = at.elapsed().as_secs_f32() / SPRING_DURATION.as_secs_f32();
        if t >= 1. {
            self.review_spring = None;
            return (0., None);
        }
        window.request_animation_frame();
        let offset = from * (1. - damped_spring(0.85, t));
        let left = left.and_then(|l| names.iter().position(|n| *n == l));
        (offset, left.map(|l| (l, if from > 0. { offset - width } else { offset + width })))
    }

    pub(super) fn finish_merge(&mut self, go: bool, cx: &mut Context<Self>) {
        let Some(confirm) = self.merge_confirm.as_mut().filter(|c| c.is_open()) else { return };
        confirm.close();
        let MergeAsk { dir, key, gitlab, number, auto, squash, remove_branch, .. } = confirm.value.clone();
        if go {
            self.review_act("merge".into(), key, move || crate::ui::review::merge(&dir, gitlab, number, auto, squash, remove_branch), cx);
        }
        cx.notify();
    }
}

/// The panel's bottom bar while the PR or MR is open (a draft's has Mark as ready): Approve, which once one has reads Approved and takes it back
/// (GitLab), and Merge (off with conflicts or while CI runs); a button that is off says why.
fn request_footer(
    theme: &Theme,
    r: &Request,
    running: bool,
    acting: &std::collections::HashSet<String>,
    on_approve: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_merge: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_ready: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Option<AnyElement> {
    let bar = || div().flex_none().px(px(12.)).py(px(10.)).flex().items_center().gap(px(8.)).border_t_1().border_color(theme.border).text_size(px(12.));
    if r.pr.state == "draft" {
        // a draft merges once it's ready
        let readying = acting.contains("ready");
        let hover = theme.hover;
        let ready = footer_button("review-ready", "icons/check.svg", if readying { "Marking…" } else { "Mark as ready" }, theme.text, None, readying)
            .border_1()
            .border_color(theme.border)
            .when(!readying, |el| el.hover(move |s| s.bg(hover)).on_click(on_ready));
        return Some(bar().child(div().flex_1().text_color(theme.text_faint).child("Draft")).child(ready).into_any_element());
    }
    if r.pr.state != "open" {
        return None;
    }
    let (approving, merging) = (acting.contains("approve"), acting.contains("merge"));
    let no_approve = match (r.approved, r.pr.gitlab) {
        (true, false) => Some("Dismiss the review on GitHub to take it back"),
        (true, true) => None,
        (false, _) if r.can_approve => None,
        (false, true) => Some("You can't approve this merge request"),
        (false, false) => Some("You can't approve your own pull request"),
    };
    let no_merge = (!r.mergeable).then_some("Resolve the conflicts first");
    let label = match (r.approved, approving) {
        (true, true) => "Revoking…",
        (true, false) => "Approved",
        (false, true) => "Approving…",
        (false, false) => "Approve",
    };
    let (icon, ink, edge) = if r.approved {
        ("icons/solar/check-circle.svg", theme.success, theme.success.opacity(0.5))
    } else {
        ("icons/check.svg", theme.text, theme.border)
    };
    let approve = footer_button("review-approve", icon, label, ink, no_approve, approving)
        .border_1()
        .border_color(edge)
        .when(r.approved, |el| el.bg(theme.success.opacity(0.12)))
        .when(no_approve.is_none() && !approving, |el| {
            let hover = theme.hover;
            el.hover(move |s| s.bg(hover)).when(r.approved, |el| el.tooltip(tip("Revoke approval", None))).on_click(on_approve)
        });
    // while the pipeline runs it merges once it passes; set so, a click turns that off
    let merge = if r.auto_merge {
        footer_button("review-merge", "icons/solar/clock-circle.svg", if merging { "Cancelling…" } else { "Auto-merge on" }, theme.success, None, merging)
            .border_1()
            .border_color(theme.success.opacity(0.5))
            .bg(theme.success.opacity(0.12))
            .when(!merging, |el| {
                let hover = theme.hover;
                el.hover(move |s| s.bg(hover)).tooltip(tip("Cancel auto-merge", None)).on_click(on_cancel)
            })
    } else {
        let (icon, label) = match (running, merging) {
            (_, true) => ("icons/git-pull-request.svg", "Merging…"),
            (true, false) => ("icons/solar/clock-circle.svg", "Auto-merge"),
            (false, false) => ("icons/git-pull-request.svg", "Merge"),
        };
        footer_button("review-merge", icon, label, gpui::white(), no_merge, merging)
            .bg(theme.danger)
            .when(running && no_merge.is_none() && !merging, |el| el.tooltip(tip("Merge once the pipeline passes", None)))
            .when(no_merge.is_none() && !merging, |el| el.on_click(on_merge))
    };
    Some(bar().child(div().flex_1()).child(approve).child(merge).into_any_element())
}

/// A footer button: icon and label in `ink`, dimmed with `off` (its reason, as the tooltip) or while `busy`.
fn footer_button(id: &'static str, icon: &'static str, label: &'static str, ink: gpui::Hsla, off: Option<&'static str>, busy: bool) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(28.))
        .px(px(10.))
        .flex()
        .items_center()
        .gap(px(6.))
        .rounded(px(8.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(ink)
        .child(gpui::svg().path(icon).size(px(13.)).flex_none().text_color(ink))
        .child(label)
        .map(|el| match off {
            Some(why) => el.opacity(0.45).tooltip(tip(why, None)),
            None if busy => el.opacity(0.7),
            None => el.cursor_pointer(),
        })
}

/// A PR state as it is written and coloured.
pub(super) fn pr_tone(state: &str, theme: &Theme) -> (&'static str, Hsla) {
    match state {
        "draft" => ("Draft", theme.text_muted),
        "merged" => ("Merged", theme.accent),
        "closed" => ("Closed", theme.danger),
        _ => ("Open", theme.success),
    }
}

/// `1234567` as `1,234,567`.
fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The project a PR's or MR's page belongs to: `owner/repo` of `https://github.com/owner/repo/pull/42`.
fn repo_of(url: &str) -> Option<String> {
    let project = url.split_once("/-/merge_requests/").or_else(|| url.split_once("/pull/"))?.0;
    crate::ui::review::repo_path(project)
}

/// A state pill: `icon` and `label` in `color`, on a wash of it inside its edge.
fn pill(icon: &'static str, label: impl Into<SharedString>, color: Hsla) -> Div {
    div()
        .flex_none()
        .h(px(22.))
        .px(px(8.))
        .flex()
        .items_center()
        .gap(px(5.))
        .rounded(px(6.))
        .border_1()
        .border_color(color.opacity(0.35))
        .bg(color.opacity(0.1))
        .text_color(color)
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .child(gpui::svg().path(icon).size(px(13.)).text_color(color))
        .child(label.into())
}

/// The PR's head: its project and number (a click opens it) with its state; the title; the author, when it last
/// changed and the command that checks it out (a click copies it); the branches and how much it changes.
fn request_head(theme: &Theme, r: &Request, menu: AnyElement) -> AnyElement {
    let (label, color) = pr_tone(r.pr.state, theme);
    let number = format!("{}{}", if r.pr.gitlab { "!" } else { "#" }, r.pr.number);
    let (accent, text) = (theme.accent, theme.text);
    let open = {
        let url = r.pr.url.clone();
        move |_: &ClickEvent, _: &mut Window, cx: &mut App| cx.open_url(&url)
    };
    let project = div()
        .id("review-project")
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(4.))
        .cursor_pointer()
        .text_size(px(12.))
        .text_color(theme.text_muted)
        .hover(move |s| s.text_color(text))
        .children(repo_of(&r.pr.url).map(|repo| div().min_w_0().truncate().child(repo)))
        .child(div().flex_none().text_color(accent).child(number.clone()))
        .child(gpui::svg().path("icons/solar/square-arrow-right-up.svg").size(px(12.)).flex_none().text_color(accent))
        .tooltip(tip(format!("Open on {}", if r.pr.gitlab { "GitLab" } else { "GitHub" }), None))
        .on_click(open.clone());
    let open_now = r.pr.state == "open";
    let states = div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(6.))
        .when(open_now && !r.mergeable, |el| el.child(pill("icons/solar/close-circle.svg", "Conflicts", theme.danger)))
        .when(open_now && r.accepted, |el| el.child(pill("icons/solar/check-circle.svg", "Approved", theme.success)))
        .child(pill("icons/git-pull-request.svg", label, color))
        .child(menu);
    let title = div()
        .id("review-title")
        .text_size(px(15.))
        .line_height(px(20.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme.text)
        .cursor_pointer()
        .hover(move |s| s.text_color(accent))
        .child(if r.title.is_empty() { number.clone() } else { r.title.clone() })
        .on_click(open);
    let updated = r.updated.map(|at| format!("updated {}", ago_text(at)));
    let checkout = format!("{} checkout {}", if r.pr.gitlab { "glab mr" } else { "gh pr" }, r.pr.number);
    let mono = theme.font_terminal.clone();
    let byline = div()
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(px(12.))
        .text_color(theme.text_faint)
        .when(!r.author.is_empty(), |el| {
            let initial = r.author.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
            el.child(
                div()
                    .size(px(18.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(theme.hover)
                    .text_size(px(10.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.text_muted)
                    .child(initial),
            )
            .child(div().min_w_0().truncate().text_color(theme.text).child(r.author.clone()))
        })
        .children(updated.map(|u| div().flex_none().child(format!("· {u}"))))
        .child(div().flex_1())
        .child(
            div()
                .id("review-checkout")
                .flex_none()
                .font_family(mono.clone())
                .text_color(theme.text_muted)
                .cursor_pointer()
                .hover(move |s| s.text_color(text))
                .tooltip(tip("Copy", None))
                .child(checkout.clone())
                .on_click(move |_, _, cx| {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(checkout.clone()));
                    crate::ui::toast::info(cx, "Copied the checkout command", None);
                }),
        );
    let branches = div()
        .flex()
        .items_center()
        .gap(px(10.))
        .text_size(px(12.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_family(mono.clone())
                .text_color(theme.text_faint)
                .child(if r.head.is_empty() { r.base.clone() } else { format!("{} ← {}", r.base, r.head) }),
        )
        .children(r.diff.map(|d| {
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(gpui::svg().path("icons/solar/file-text.svg").size(px(13.)).text_color(theme.text_faint))
                .child(div().text_color(theme.text_muted).child(format!("{} file{}", thousands(d.files), if d.files == 1 { "" } else { "s" })))
                .child(div().font_family(mono.clone()).text_color(theme.success).child(format!("+{}", thousands(d.additions))))
                .child(div().font_family(mono.clone()).text_color(theme.danger).child(format!("-{}", thousands(d.deletions))))
        }));
    div()
        .px(px(12.))
        .pt(px(12.))
        .pb(px(12.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .border_b_1()
        .border_color(theme.border)
        .child(div().flex().items_center().gap(px(8.)).child(project).child(states))
        .child(title)
        .child(byline)
        .child(branches)
        .into_any_element()
}

/// How CI went, as the tabs' line says it: its state and a few words.
fn ci_line(checks: &[Check], pipeline: Option<CheckState>, gitlab: bool) -> Option<(CheckState, String)> {
    if checks.is_empty() && pipeline.is_none() {
        return None;
    }
    let state = pipeline.unwrap_or_else(|| crate::ui::review::worst(checks.iter().map(|c| c.state)));
    let count = |states: &[CheckState]| checks.iter().filter(|c| states.contains(&c.state)).count();
    let some = |n: usize, what: &str| format!("{n} {}{} {what}", if gitlab { "job" } else { "check" }, if n == 1 { "" } else { "s" });
    let words = match state {
        CheckState::Success if gitlab => "Pipeline passed".into(),
        CheckState::Success => "All checks passed".into(),
        CheckState::Failed => match count(&[CheckState::Failed]) {
            0 => "Pipeline failed".into(),
            n => some(n, "failed"),
        },
        CheckState::Running | CheckState::Pending => match count(&[CheckState::Running, CheckState::Pending]) {
            0 => "Pipeline running".into(),
            n => some(n, "running"),
        },
        CheckState::Skipped => "Checks skipped".into(),
    };
    Some((state, words))
}

/// The tabs as one segmented control, and how CI went beside them (a click shows the checks).
fn tab_bar(theme: &Theme, shown: Tab, checks_title: &'static str, ci: Option<(CheckState, String)>, pick: &dyn Fn(Tab) -> Listener, window: &mut Window) -> AnyElement {
    let (text, muted, on) = (theme.text, theme.text_muted, theme.card);
    let tab = |t: Tab, label: &'static str| {
        let selected = t == shown;
        div()
            .id(SharedString::from(format!("review-tab-{t:?}")))
            .h(px(24.))
            .px(px(10.))
            .flex()
            .items_center()
            .rounded(px(6.))
            .text_size(px(12.))
            .font_weight(gpui::FontWeight::MEDIUM)
            .map(|el| if selected { el.bg(on).text_color(text) } else { el.text_color(muted).cursor_pointer().hover(move |s| s.text_color(text)) })
            .child(label)
            .on_click(pick(t))
    };
    let tabs = div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(2.))
        .p(px(2.))
        .rounded(px(8.))
        .bg(theme.hover)
        .child(tab(Tab::Summary, "Summary"))
        .child(tab(Tab::Timeline, "Timeline"))
        .child(tab(Tab::Code, "Code"))
        .child(tab(Tab::Checks, checks_title));
    let on_ci = pick(Tab::Checks);
    div()
        .flex_none()
        .px(px(12.))
        .py(px(8.))
        .flex()
        .items_center()
        .gap(px(8.))
        .border_b_1()
        .border_color(theme.border)
        .child(tabs)
        .child(div().flex_1())
        .children(ci.map(|(state, words)| {
            div()
                .id("review-ci")
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .text_color(muted)
                .cursor_pointer()
                .hover(move |s| s.text_color(text))
                .tooltip(tip(words.clone(), None))
                .child(check_icon(theme, state, 14., window))
                .child(div().min_w_0().truncate().child(words))
                .on_click(on_ci)
        }))
        .into_any_element()
}

/// How long ago unix second `at` was: "just now", "5m ago", "2d ago".
fn ago_text(at: i64) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(at);
    match crate::model::ago(Duration::from_secs((now - at).max(0) as u64)).as_str() {
        "now" => "just now".to_string(),
        ago => format!("{ago} ago"),
    }
}

/// A person's initial in a small circle, standing in for their avatar.
fn avatar(theme: &Theme, who: &str) -> Div {
    let initial = who.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
    div()
        .size(px(20.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(theme.hover)
        .text_size(px(10.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme.text_muted)
        .child(initial)
}

/// The Timeline tab: what happened on the PR, oldest first. A comment, or a review that says something, is a card with
/// its text; the rest are one line each, a push with its commits under it.
fn timeline(theme: &Theme, events: &[crate::ui::review::Event]) -> AnyElement {
    use crate::ui::review::Happened;
    let mono = theme.font_terminal.clone();
    let item = |i: usize, e: &crate::ui::review::Event| -> AnyElement {
        let (icon, color, said): (&'static str, Hsla, String) = match &e.what {
            Happened::Opened => ("icons/git-pull-request.svg", theme.success, "opened this".into()),
            Happened::Commented => ("icons/solar/chat-round-line.svg", theme.text_muted, "commented".into()),
            Happened::Approved => ("icons/solar/check-circle.svg", theme.success, "approved".into()),
            Happened::Changes => ("icons/solar/close-circle.svg", theme.danger, "requested changes".into()),
            Happened::Reviewed => ("icons/solar/eye.svg", theme.text_muted, "reviewed".into()),
            Happened::Commits(c) => ("icons/solar/git-commit.svg", theme.text_muted, format!("pushed {} commit{}", c.len(), if c.len() == 1 { "" } else { "s" })),
            Happened::Merged => ("icons/git-pull-request.svg", theme.accent, "merged this".into()),
            Happened::Closed => ("icons/solar/close-circle.svg", theme.danger, "closed this".into()),
            Happened::Note => ("icons/solar/pen.svg", theme.text_faint, e.body.clone()),
        };
        let card = !e.body.trim().is_empty() && !matches!(e.what, Happened::Note);
        let line = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .text_size(px(12.))
            .text_color(theme.text_muted)
            .child(if card { avatar(theme, &e.who) } else { div().size(px(20.)).flex_none().flex().items_center().justify_center().child(gpui::svg().path(icon).size(px(14.)).text_color(color)) })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    // one run of text, the name set apart: two children would stack
                    .child({
                        let who = if e.who.is_empty() { String::new() } else { format!("{} ", e.who) };
                        let name = gpui::HighlightStyle { color: Some(theme.text), font_weight: Some(gpui::FontWeight::MEDIUM), ..Default::default() };
                        gpui::StyledText::new(format!("{who}{said}")).with_highlights([(0..who.len(), name)])
                    }),
            )
            .when(card && !matches!(e.what, Happened::Commented), |el| el.child(gpui::svg().path(icon).size(px(14.)).flex_none().text_color(color)))
            .child(div().flex_none().text_color(theme.text_faint).child(ago_text(e.at)));
        let commits = match &e.what {
            Happened::Commits(c) => Some(div().pl(px(28.)).flex().flex_col().gap(px(2.)).text_size(px(12.)).children(c.iter().map(|(sha, headline)| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().flex_none().font_family(mono.clone()).text_color(theme.text_faint).child(sha.clone()))
                    .child(div().min_w_0().truncate().text_color(theme.text).child(headline.clone()))
            }))),
            _ => None,
        };
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(line)
            .children(commits)
            .when(card, |el| {
                el.child(
                    div()
                        .ml(px(28.))
                        .p(px(10.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.card)
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .text_color(theme.text)
                        .children(crate::ui::markdown::render(&parsed(&e.body), &format!("review-tl-{i}"), theme)),
                )
            })
            .into_any_element()
    };
    div().flex().flex_col().gap(px(14.)).children(events.iter().enumerate().map(|(i, e)| item(i, e))).into_any_element()
}

/// Up to this many files, the Code tab opens with every file's diff unfolded.
const SMALL_PR: usize = 3;
/// How many of a file's diff lines the Code tab draws; the rest are a link to the host.
const MAX_FILE_LINES: usize = 400;

/// The Code tab: the PR's files with what each adds and removes; a click on one unfolds its diff (`open` is which
/// are), and `on_file` folds or unfolds one by its path.
fn code(theme: &Theme, files: &[crate::changes::FileDiff], open: &[bool], cut: bool, pr: &crate::ui::statusbar::Pr, on_file: Rc<dyn Fn(String, &mut App)>) -> AnyElement {
    use crate::changes::{FileStatus, LineKind};
    let mono = theme.font_terminal.clone();
    let (hover, accent) = (theme.hover, theme.accent);
    let page = if pr.gitlab { format!("{}/diffs", pr.url) } else { format!("{}/files", pr.url) };
    let (adds, dels) = files.iter().fold((0, 0), |(a, d), f| (a + f.additions as u64, d + f.deletions as u64));
    let open_page = |id: gpui::ElementId, label: String| {
        let page = page.clone();
        div().id(id).cursor_pointer().text_color(accent).hover(|s| s.underline()).child(label).on_click(move |_, _, cx| cx.open_url(&page))
    };
    let heading = div()
        .flex()
        .items_center()
        .gap(px(8.))
        .text_size(px(12.))
        .text_color(theme.text_muted)
        .child(div().flex_1().child(format!("{} file{} changed", thousands(files.len() as u64), if files.len() == 1 { "" } else { "s" })))
        .child(div().font_family(mono.clone()).text_color(theme.success).child(format!("+{}", thousands(adds))))
        .child(div().font_family(mono.clone()).text_color(theme.danger).child(format!("-{}", thousands(dels))));
    let diff = |i: usize, f: &crate::changes::FileDiff| -> AnyElement {
        let gutter = (f.max_line.max(1).to_string().len() as f32) * 7. + 10.;
        let number = |n: Option<u32>, color: Hsla| {
            div().w(px(gutter)).flex_none().pr(px(6.)).flex().justify_end().text_color(color).child(n.map(|n| n.to_string()).unwrap_or_default())
        };
        let mut rows: Vec<AnyElement> = crate::changes::file_notices(f)
            .into_iter()
            .map(|n| div().px(px(10.)).py(px(4.)).text_color(theme.text_faint).child(n).into_any_element())
            .collect();
        let mut shown = 0;
        let total: usize = f.hunks.iter().map(|h| h.lines.len()).sum();
        'hunks: for h in &f.hunks {
            rows.push(div().px(px(10.)).py(px(3.)).bg(theme.diff_hunk).text_color(theme.text_faint).truncate().child(h.header.clone()).into_any_element());
            for l in &h.lines {
                if shown == MAX_FILE_LINES {
                    break 'hunks;
                }
                shown += 1;
                let (marker, ink, bg) = match l.kind {
                    LineKind::Add => ("+", theme.diff_add, Some(theme.diff_add.opacity(0.08))),
                    LineKind::Del => ("-", theme.diff_del, Some(theme.diff_del.opacity(0.08))),
                    _ => (" ", theme.text_faint, None),
                };
                let text = l.text.replace('\t', "    ");
                rows.push(
                    div()
                        .flex()
                        .items_start()
                        .when_some(bg, |el, bg| el.bg(bg))
                        .child(number(l.old_no, theme.text_faint.opacity(0.8)))
                        .child(number(l.new_no, theme.text_faint.opacity(0.8)))
                        .child(div().w(px(14.)).flex_none().text_color(ink).child(marker))
                        .child(div().flex_1().min_w_0().pr(px(8.)).text_color(theme.text).child(if text.is_empty() { " ".to_string() } else { text }))
                        .into_any_element(),
                );
            }
        }
        if shown < total {
            rows.push(
                div()
                    .px(px(10.))
                    .py(px(6.))
                    .flex()
                    .gap(px(6.))
                    .text_color(theme.text_faint)
                    .child(format!("{} more lines", thousands((total - shown) as u64)))
                    .child(open_page(("review-code-more", i).into(), format!("Open on {} ↗", if pr.gitlab { "GitLab" } else { "GitHub" })))
                    .into_any_element(),
            );
        }
        div()
            .ml(px(6.))
            .mt(px(2.))
            .mb(px(6.))
            .rounded(px(6.))
            .border_1()
            .border_color(theme.border)
            .overflow_hidden()
            .font_family(mono.clone())
            .text_size(px(11.5))
            .line_height(px(17.))
            .children(rows)
            .into_any_element()
    };
    let rows = files.iter().enumerate().flat_map(|(i, f)| {
        let unfolded = open.get(i).copied().unwrap_or(false);
        let (folder, name) = match f.path.rsplit_once('/') {
            Some((folder, name)) => (format!("{folder}/"), name.to_string()),
            None => (String::new(), f.path.clone()),
        };
        let tint = match f.status {
            FileStatus::Added => theme.success,
            FileStatus::Deleted => theme.danger,
            _ => theme.text,
        };
        let on_file = on_file.clone();
        let path = f.path.clone();
        let row = div()
            .id(("review-file", i))
            .h(px(26.))
            .px(px(6.))
            .mx(px(-6.))
            .flex()
            .items_center()
            .gap(px(6.))
            .rounded(px(6.))
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .text_size(px(12.))
            .child(gpui::svg().path(if unfolded { "icons/alt-arrow-down.svg" } else { "icons/alt-arrow-right.svg" }).size(px(12.)).flex_none().text_color(theme.text_faint))
            .child(crate::ui::file_icons::icon(&f.path, theme.appearance).size(px(14.)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .when_some(f.old_path.clone(), |el, old| el.child(gpui::StyledText::new(format!("{old} → ")).with_highlights([(0..old.len() + 5, gpui::HighlightStyle { color: Some(theme.text_faint), ..Default::default() })])))
                    .child(gpui::StyledText::new(format!("{folder}{name}")).with_highlights([
                        (0..folder.len(), gpui::HighlightStyle { color: Some(theme.text_faint), ..Default::default() }),
                        (folder.len()..folder.len() + name.len(), gpui::HighlightStyle { color: Some(tint), ..Default::default() }),
                    ])),
            )
            .when(f.binary, |el| el.child(div().flex_none().text_color(theme.text_faint).child("binary")))
            .when(!f.binary, |el| {
                el.child(div().flex_none().font_family(mono.clone()).text_color(theme.success).child(format!("+{}", f.additions)))
                    .child(div().flex_none().font_family(mono.clone()).text_color(theme.danger).child(format!("-{}", f.deletions)))
            })
            .on_click(move |_, _, cx| on_file(path.clone(), cx))
            .into_any_element();
        std::iter::once(row).chain(unfolded.then(|| diff(i, f)))
    });
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(div().pb(px(6.)).child(heading))
        .children(rows.collect::<Vec<_>>())
        .when(cut, |el| {
            el.child(
                div()
                    .pt(px(6.))
                    .flex()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .text_color(theme.text_faint)
                    .child("The diff is too big to show whole.")
                    .child(open_page("review-code-cut".into(), "See it all ↗".into())),
            )
        })
        .into_any_element()
}

/// The Summary tab's properties: who reviews it and what they said, its labels, and (GitLab) its open threads.
fn summary(theme: &Theme, r: &Request) -> AnyElement {
    use crate::ui::review::Verdict;
    let none = || div().text_color(theme.text_faint).child("None").into_any_element();
    let row = |icon: &'static str, name: &'static str, value: AnyElement| {
        div()
            .flex()
            .items_start()
            .gap(px(8.))
            .child(
                div()
                    .w(px(96.))
                    .h(px(22.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_color(theme.text_muted)
                    .child(gpui::svg().path(icon).size(px(14.)).flex_none().text_color(theme.text_faint))
                    .child(name),
            )
            .child(div().flex_1().min_w_0().min_h(px(22.)).flex().flex_wrap().items_center().gap(px(6.)).child(value))
    };
    let reviewers = if r.reviewers.is_empty() {
        none()
    } else {
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(10.))
            .children(r.reviewers.iter().map(|rv| {
                let (icon, color, said) = match rv.verdict {
                    Verdict::Approved => ("icons/solar/check-circle.svg", theme.success, "approved"),
                    Verdict::Changes => ("icons/solar/close-circle.svg", theme.danger, "requested changes"),
                    Verdict::Commented => ("icons/solar/chat-round-dots.svg", theme.text_muted, "commented"),
                    Verdict::Pending => ("icons/solar/clock-circle.svg", theme.text_faint, "hasn't reviewed yet"),
                };
                div()
                    .id(SharedString::from(format!("review-reviewer-{}", rv.name)))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_color(theme.text)
                    .tooltip(tip(format!("{} {said}", rv.name), None))
                    .child(gpui::svg().path(icon).size(px(14.)).flex_none().text_color(color))
                    .child(rv.name.clone())
            }))
            .into_any_element()
    };
    let labels = if r.labels.is_empty() {
        none()
    } else {
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(4.))
            .children(r.labels.iter().map(|l| {
                div().h(px(20.)).px(px(6.)).flex().items_center().rounded(px(5.)).bg(theme.hover).text_color(theme.text).child(l.clone())
            }))
            .into_any_element()
    };
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .text_size(px(12.))
        .child(row("icons/solar/users-group-rounded.svg", "Reviewers", reviewers))
        .child(row("icons/solar/tag.svg", "Labels", labels))
        .children(r.unresolved.map(|n| {
            let value = match n {
                0 => div().text_color(theme.text_faint).child("All resolved"),
                n => div().text_color(theme.warning).child(format!("{n} unresolved")),
            };
            row("icons/solar/chat-round-dots.svg", "Threads", value.into_any_element())
        }))
        .into_any_element()
}

/// Runs CLI calls for a button, under its busy key.
type OnAct = Rc<dyn Fn(String, Vec<Vec<String>>, &mut App)>;
/// Pages the checks to a pipeline, by its index.
type OnPage = Rc<dyn Fn(usize, &mut App)>;

/// The checks' pages, GitLab pipelines or GitHub workflows, one shown at a time.
struct Paging {
    pages: Vec<(String, CheckState)>,
    /// The one shown.
    page: usize,
    on_page: OnPage,
    /// The shown page's offset, and the page beside it with its own, as `Shell::review_offset` has them.
    offset: f32,
    beside: Option<(usize, f32)>,
    /// How far a page slides: the panel's width.
    width: f32,
    /// The job whose log shows under it, and the log's last lines once read.
    log: Option<(String, Option<String>)>,
    /// Shows a job's log, or hides it shown.
    on_log: Rc<dyn Fn(String, &mut App)>,
}

/// The page a swipe of `travel` px goes to: fingers to the left bring the next one.
fn neighbour(count: usize, page: usize, travel: f32) -> Option<usize> {
    if travel < 0. { (page + 1 < count).then_some(page + 1) } else { page.checked_sub(1) }
}

/// The page's offset under the fingers, as the sidebar's list follows them: most of the way to the next page, past
/// that (or with none next) against a pull that grows the further they go.
fn page_offset(travel: f32, near: bool, width: f32) -> f32 {
    let reach = if near { width * 0.85 } else { 0. };
    let over = (travel.abs() - reach).max(0.);
    travel.signum() * (travel.abs().min(reach) + (1. - 1. / (over * 0.4 / width + 1.)) * width)
}

/// The checks or jobs, with how many failed, run and passed; a GitLab pipeline's title carries its own state and
/// opens it, and a button retrying the failed ones. With child pipelines (or workflows) they page, one at a time: the
/// title has every page's state (its name on hover, a click opens it) and a swipe drags to the next; a trigger job
/// opens its child's page. A page's jobs come stage by stage under their stage's name. A row's button, shown on hover, retries,
/// plays or cancels it.
fn checks_section(
    theme: &Theme,
    title: &'static str,
    checks: &[Check],
    pipeline: Option<(String, CheckState)>,
    gitlab: bool,
    acting: &std::collections::HashSet<String>,
    on_act: OnAct,
    paging: Paging,
    window: &mut Window,
) -> AnyElement {
    let count = |states: &[CheckState]| checks.iter().filter(|c| states.contains(&c.state)).count();
    let summary: Vec<String> = [
        (count(&[CheckState::Failed]), "failed"),
        (count(&[CheckState::Running, CheckState::Pending]), "running"),
        (count(&[CheckState::Success]), "passed"),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, what)| format!("{n} {what}"))
    .collect();
    let hover = theme.hover;
    let Paging { pages, page, on_page, offset, beside, width, log, on_log } = paging;
    let paged = pages.len() > 1;
    let dots = paged.then(|| {
        div().flex().items_center().gap(px(1.)).children(pages.iter().enumerate().map(|(i, (name, state))| {
            let on_page = on_page.clone();
            div()
                .id(("review-page", i))
                .p(px(3.))
                .rounded(px(5.))
                .cursor_pointer()
                .when(i == page, |el| el.bg(theme.selected))
                .hover(move |s| s.bg(hover))
                .tooltip(tip(name.clone(), None))
                .child(check_icon(theme, *state, 12., window))
                .on_click(move |_, _, cx| {
                    cx.stop_propagation();
                    on_page(i, cx);
                })
        }))
    });
    let heading = div()
        .id("review-pipeline")
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(px(11.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(theme.text_faint)
        .children(pipeline.as_ref().map(|(_, state)| check_icon(theme, *state, 14., window)))
        .child(div().flex_1().child(title))
        .map(|el| match dots {
            Some(dots) => el.child(dots),
            None => el.child(summary.join(" · ")),
        })
        .children({
            let calls = retry_failed(checks, gitlab);
            (!calls.is_empty()).then(|| {
                let on_act = on_act.clone();
                act_button(theme, "review-retry-failed".into(), "icons/solar/restart.svg", "Retry failed jobs", theme.text_muted, acting.contains("failed"), true, window)
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        on_act("failed".into(), calls.clone(), cx);
                    })
            })
        })
        .when_some(pipeline.map(|(url, _)| url), |el, url| el.cursor_pointer().hover(|s| s.underline()).on_click(move |_, _, cx| cx.open_url(&url)));
    let row = |i: usize, c: &Check, window: &mut Window| {
        let group: SharedString = format!("review-check-{i}").into();
        let action = c.action();
        let button = c.command(gitlab, action).map(|cmd| {
            let busy = cmd.join(" ");
            let (icon, label) = match action {
                crate::ui::review::Action::Cancel if !gitlab => ("icons/solar/stop-circle.svg", "Cancel workflow run"),
                crate::ui::review::Action::Cancel => ("icons/solar/stop-circle.svg", "Cancel job"),
                crate::ui::review::Action::Play => ("icons/solar/play.svg", "Run job"),
                _ => ("icons/solar/restart.svg", "Retry job"),
            };
            let ink = if action == crate::ui::review::Action::Cancel { theme.danger } else { theme.text_muted };
            let on_act = on_act.clone();
            act_button(theme, ("review-check-act", i).into(), icon, label, ink, acting.contains(&busy), false, window)
                .group_hover(group.clone(), |s| s.opacity(1.))
                .on_click(move |_, _, cx| {
                    cx.stop_propagation();
                    on_act(busy.clone(), vec![cmd.clone()], cx);
                })
        });
        // a trigger job opens its child pipeline's page, not GitLab
        let child = c.child.as_ref().and_then(|name| pages.iter().position(|(n, _)| n == name));
        div()
            .id(("review-check", i))
            .group(group.clone())
            .h(px(28.))
            .px(px(6.))
            .mx(px(-6.))
            .flex()
            .items_center()
            .gap(px(8.))
            .rounded(px(6.))
            .child(check_icon(theme, c.state, 14., window))
            .child(div().flex_1().min_w_0().truncate().text_color(theme.text).child(c.name.clone()))
            .children(c.secs.map(|s| div().flex_none().text_color(theme.text_faint).child(duration(s))))
            .children(button)
            .when(child.is_some(), |el| el.child(gpui::svg().path("icons/alt-arrow-right.svg").size(px(12.)).flex_none().text_color(theme.text_faint)))
            // a job (not a trigger job, not a commit status) shows its log under it
            .map(|el| match (child, c.job.clone().filter(|_| !c.bridge && c.state != CheckState::Pending && !c.manual), c.url.clone()) {
                (Some(to), _, _) => {
                    let on_page = on_page.clone();
                    el.cursor_pointer().hover(move |s| s.bg(hover)).on_click(move |_, _, cx| on_page(to, cx))
                }
                (None, Some(job), _) => {
                    let on_log = on_log.clone();
                    el.cursor_pointer().hover(move |s| s.bg(hover)).on_click(move |_, _, cx| on_log(job.clone(), cx))
                }
                (None, None, Some(url)) => el.cursor_pointer().hover(move |s| s.bg(hover)).on_click(move |_, _, cx| cx.open_url(&url)),
                (None, None, None) => el,
            })
    };
    // a page's jobs, a run of them per stage
    let rows_for = |name: &str, window: &mut Window| -> Vec<AnyElement> {
        let mut stages: Vec<(&str, Vec<(usize, &Check)>)> = Vec::new();
        for (i, c) in checks.iter().enumerate().filter(|(_, c)| !paged || c.page == name) {
            match stages.last_mut() {
                Some((name, run)) if *name == c.group => run.push((i, c)),
                _ => stages.push((&c.group, vec![(i, c)])),
            }
        }
        let titled = stages.iter().any(|(name, _)| !name.is_empty());
        let mut rows: Vec<AnyElement> = Vec::new();
        for (stage, run) in stages {
            if titled {
                let state = crate::ui::review::worst(run.iter().map(|(_, c)| c.state));
                rows.push(
                    div()
                        .pt(px(if rows.is_empty() { 0. } else { 8. }))
                        .pb(px(2.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(11.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.text_faint)
                        .child(check_icon(theme, state, 12., window))
                        .child(stage.to_string())
                        .into_any_element(),
                );
            }
            for (i, c) in run {
                rows.push(row(i, c, window).into_any_element());
                if let Some((_, text)) = log.as_ref().filter(|(job, _)| c.job.as_deref() == Some(job.as_str())) {
                    rows.push(log_box(theme, i, text.as_deref(), c.url.clone(), window));
                }
            }
        }
        rows
    };
    let shown = pages.get(page).map(|(n, _)| n.clone()).unwrap_or_default();
    let list = div().flex().flex_col().gap(px(2.)).children(rows_for(&shown, window));
    // pages slide within the panel, past the body's padding to its edges
    let pager = div().mx(px(-12.)).px(px(12.)).relative().overflow_hidden().child(list.relative().left(px(offset))).children(beside.map(|(n, at)| {
        let name = pages[n].0.clone();
        div().absolute().top_0().left(px(12. + at)).w(px(width - 24.)).flex().flex_col().gap(px(2.)).children(rows_for(&name, window))
    }));
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .text_size(px(12.))
        .child(div().pb(px(4.)).child(heading))
        .child(pager)
        .into_any_element()
}

/// A job's log under its row: its last lines as a code block, selectable and copied with Cmd+C, a spinner until read,
/// a link to its page.
fn log_box(theme: &Theme, i: usize, text: Option<&str>, url: Option<String>, window: &mut Window) -> AnyElement {
    let accent = theme.accent;
    div()
        .mb(px(6.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .flex()
                .items_center()
                .text_size(px(11.))
                .text_color(theme.text_faint)
                .child(div().flex_1().child(format!("Last {} lines", crate::ui::review::LOG_LINES)))
                .children(url.map(|url| {
                    div()
                        .id(("review-log-open", i))
                        .cursor_pointer()
                        .hover(move |s| s.text_color(accent))
                        .child("Open job ↗")
                        .on_click(move |_, _, cx| cx.open_url(&url))
                })),
        )
        .child(match text {
            None => div().py(px(4.)).flex().justify_center().child(crate::ui::loader::mini_spinner(2., theme.text_faint, window)).into_any_element(),
            Some("") => div().text_size(px(11.)).text_color(theme.text_faint).child("No log yet").into_any_element(),
            Some(text) => {
                div().line_height(px(15.)).child(crate::ui::markdown::code(text, &format!("review-log-{i}"), px(10.5), theme)).into_any_element()
            }
        })
        .into_any_element()
}

/// A check's or the section's small button, its icon in `ink`; `busy` puts a spinner in it and keeps it shown, as
/// `shown` does.
fn act_button(
    theme: &Theme,
    id: gpui::ElementId,
    icon: &'static str,
    label: &'static str,
    ink: gpui::Hsla,
    busy: bool,
    shown: bool,
    window: &mut Window,
) -> Stateful<Div> {
    let hover = theme.hover;
    div()
        .id(id)
        .size(px(20.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(5.))
        .when(!busy && !shown, |el| el.opacity(0.))
        .map(|el| {
            if busy {
                el.child(crate::ui::loader::mini_spinner(2., theme.text_faint, window))
            } else {
                el.cursor_pointer()
                    .hover(move |s| s.bg(hover))
                    .tooltip(tip(label, None))
                    .child(gpui::svg().path(icon).size(px(13.)).text_color(ink))
            }
        })
}
