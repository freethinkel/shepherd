//! The review panel on the right, after Superconductor's: the shown pane's branch's PR or MR (title, state, author,
//! description) and its checks or pipeline jobs, each opening in the browser.
use super::*;
use gpui::AnyElement;
use crate::settings::{REVIEW_MAX, REVIEW_MIN, REVIEW_WIDTH};
use crate::ui::review::{Check, CheckState, Request, Review, act, check_icon, duration, markdown, read_review, retry_failed};
use crate::ui::statusbar::{branch_of, cached, fetching, pane_dir, refetch};

/// How often gh or glab are asked again about a branch, as the status bar asks about its PR.
const REVIEW_TTL: Duration = Duration::from_secs(120);
/// How much of a description shows folded: about six lines.
const FOLDED: f32 = 120.;
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
    pub(super) fn toggle_review(&mut self, cx: &mut Context<Self>) {
        self.review_open = !self.review_open;
        let open = self.review_open;
        save_from_sidebar(cx, |s| s.review_open = open);
        cx.notify();
    }

    pub(super) fn drag_review(&mut self, x: Pixels, window: &Window, cx: &mut Context<Self>) {
        if self.review_drag {
            self.review_width = f32::from(window.viewport_size().width - x).clamp(REVIEW_MIN, REVIEW_MAX);
            cx.notify();
        }
    }

    pub(super) fn end_review_drag(&mut self, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.review_drag) {
            cx.set_global(GridHold(false));
            let width = self.review_width;
            save_from_sidebar(cx, |s| s.review_width = width);
            cx.refresh_windows();
        }
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
    pub(super) fn render_review(&mut self, theme: &Theme, shown: &ShownReview, window: &mut Window, cx: &mut Context<Self>) -> [AnyElement; 2] {
        let (key, review, branch) = (shown.key.clone(), shown.review.clone(), shown.branch.clone());
        let busy = key.as_deref().is_some_and(|k| fetching::<Review>(k, cx));

        let gitlab = matches!(&review, Some(Review::Found { request: Some(r), .. }) if r.pr.gitlab)
            || matches!(&review, Some(Review::Found { pipeline: Some(_), .. }) | Some(Review::NoTool("glab") | Review::NoLogin("glab", _)));
        let noun = if gitlab { "merge request" } else { "pull request" };
        let header = div()
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            .pl(px(12.))
            // clear of the window's buttons, which sit over its right end off macOS
            .pr(px(12. + window_controls_w(cx)))
            .flex()
            .items_center()
            .gap(px(6.))
            .border_b_1()
            .border_color(theme.border)
            .child(gpui::svg().path("icons/git-pull-request.svg").size(px(14.)).flex_none().text_color(theme.text_muted))
            .child(div().flex_1().min_w_0().truncate().font_weight(gpui::FontWeight::MEDIUM).child(if gitlab { "Merge request" } else { "Pull request" }))
            .children(key.clone().map(|key| {
                if busy {
                    div().size(px(24.)).flex().items_center().justify_center().child(crate::ui::loader::mini_spinner(2., theme.text_faint, window)).into_any_element()
                } else {
                    titlebar_button(theme, "review-refresh", "icons/solar/refresh.svg")
                        .tooltip(tip("Refresh", None))
                        .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| refetch::<Review>(&key, cx)))
                        .into_any_element()
                }
            }))
            .child(
                titlebar_button(theme, "review-close", "icons/close.svg")
                    .tooltip(tip("Close", Some("ToggleReview")))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_review(cx))),
            );
        let (dir, key_or) = (shown.dir.clone().unwrap_or_default(), key.clone().unwrap_or_default());

        let mut footer = None;
        let body: Vec<AnyElement> = match (&key, review) {
            (None, _) => vec![empty(theme, "No branch", "The focused pane isn't in a git repo.")],
            (Some(_), None) => vec![div().pt(px(24.)).flex().justify_center().child(crate::ui::loader::spinner(2.5, window)).into_any_element()],
            (Some(_), Some(Review::NoRemote)) => vec![empty(theme, "No remote", "This repo has no origin to ask about.")],
            (Some(_), Some(Review::NoTool(tool))) => vec![empty(theme, format!("{tool} isn't installed"), format!("Install it to see the {noun} here: brew install {tool}"))],
            (Some(_), Some(Review::NoLogin(tool, host))) => {
                vec![empty(theme, format!("{tool} isn't logged in"), format!("Run {tool} auth login --hostname {host} in a pane, then refresh."))]
            }
            (Some(_), Some(Review::Failed(why))) => vec![empty(theme, format!("Couldn't load the {noun}"), why)],
            (Some(_), Some(Review::Found { request, checks, pipeline })) => {
                let mut out = Vec::new();
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
                        out.push(request_card(theme, r))
                    }
                    None => out.push(empty(theme, format!("No {noun}"), format!("{} has none yet.", branch.unwrap_or_default()))),
                }
                if !checks.is_empty() || pipeline.is_some() {
                    let this = cx.entity().downgrade();
                    let (dir, key) = (dir.clone(), key_or.clone());
                    let on_act: OnAct = Rc::new(move |busy, calls, cx| {
                        let dir = dir.clone();
                        let job = move || calls.iter().try_for_each(|c| act(&dir, c));
                        let _ = this.update(cx, |s, cx| s.review_act(busy, key.clone(), job, cx));
                    });
                    let title = if gitlab { "Pipeline" } else { "Checks" };
                    let pages = crate::ui::review::pages(&checks);
                    let picked = self.review_page.as_ref().filter(|(k, _)| *k == key_or).and_then(|(_, name)| pages.iter().position(|(n, _)| n == name));
                    let page = picked.unwrap_or_else(|| crate::ui::review::default_page(&pages));
                    let this = cx.entity().downgrade();
                    let names: Vec<String> = pages.iter().map(|(n, _)| n.clone()).collect();
                    // the panel's whole width: pages slide in from its edge
                    let width = self.review_width;
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
                    let section = checks_section(theme, title, &checks, pipeline, gitlab, &self.review_acting, on_act, paging, window);
                    let key = key_or.clone();
                    out.push(
                        div()
                            .on_scroll_wheel(cx.listener(move |this, e: &gpui::ScrollWheelEvent, _, cx| this.review_scroll(e, &key, &names, page, width, cx)))
                            .child(section)
                            .into_any_element(),
                    );
                }
                if let Some(r) = request.filter(|r| !r.body.trim().is_empty()) {
                    let toggle = cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.review_expanded = !this.review_expanded;
                        cx.notify();
                    });
                    out.push(description(theme, &r.body, self.review_expanded, self.review_body_h.clone(), toggle));
                }
                out
            }
        };

        let width = self.review_width;
        let panel = div()
            .w(px(width))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(theme.wash(0.05))
            .border_l_1()
            .border_color(theme.border)
            .child(header)
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
            .children(footer);
        let edge = div()
            .id("review-resize")
            .on_hover(cx.listener(|this, on: &bool, _, cx| this.hover_seam("review-resize", *on, cx)))
            .absolute()
            .top(px(TITLEBAR_HEIGHT))
            .bottom_0()
            .right(px(width - 3.))
            .w(px(6.))
            .cursor_col_resize()
            // the panel's 1 px border sits at x = 3 of the strip
            .child(resize_seam("review-resize".into(), true, 3., theme.accent, self.review_drag, self.seam_hover.get("review-resize").copied()))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.review_drag = true;
                    cx.set_global(GridHold(true));
                    cx.notify();
                }),
            )
            .on_click(cx.listener(|this, e: &ClickEvent, _, cx| {
                if e.click_count() == 2 {
                    this.review_width = REVIEW_WIDTH;
                    save_from_sidebar(cx, |s| s.review_width = REVIEW_WIDTH);
                    cx.notify();
                }
            }));
        [panel.into_any_element(), edge.into_any_element()]
    }
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

// ponytail: one body remembered; the panel shows one description at a time
/// `body`'s blocks, parsed once while it stays the same: a drag over it redraws every move.
fn parsed(body: &str) -> Rc<Vec<crate::ui::review::Block>> {
    thread_local! {
        static LAST: std::cell::RefCell<(String, Rc<Vec<crate::ui::review::Block>>)> = Default::default();
    }
    LAST.with(|last| {
        let mut last = last.borrow_mut();
        if last.0 != body {
            *last = (body.to_string(), Rc::new(markdown(body)));
        }
        last.1.clone()
    })
}

/// The description, drawn whole: folded, cut to its first lines with their bottom fading out. The heading, or a
/// click on the folded text (not a drag selecting it), unfolds and folds it. `height` is the whole text's height at
/// the last paint: a short one has nothing to fold.
fn description(
    theme: &Theme,
    body: &str,
    expanded: bool,
    height: Rc<Cell<f32>>,
    toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let long = height.get() > FOLDED + 1.;
    let folded = long && !expanded;
    let toggle = Rc::new(toggle);
    let on_heading = toggle.clone();
    let text = theme.text;
    let blocks = div()
        .flex_none()
        .relative()
        .flex()
        .flex_col()
        .gap(px(6.))
        .children(crate::ui::markdown::render(&parsed(body), "review-md", theme))
        .child(
            canvas(
                move |bounds, window, _| {
                    let h = f32::from(bounds.size.height);
                    if (height.replace(h) - h).abs() > 0.5 {
                        window.refresh();
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        );
    let body = div()
        .id("review-description-body")
        .flex()
        .flex_col()
        .child(blocks)
        .when(folded, |el| {
            el.max_h(px(FOLDED)).overflow_hidden().cursor_pointer().on_click(move |e, window, cx| {
                if crate::ui::markdown::selected_text().is_none() {
                    toggle(e, window, cx);
                }
            })
        });
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .text_size(px(12.))
        .text_color(theme.text_muted)
        .child(
            div()
                .id("review-description")
                .flex()
                .items_center()
                .gap(px(4.))
                .text_size(px(11.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text_faint)
                .child("Description")
                .when(long, |el| {
                    el.cursor_pointer()
                        .hover(move |s| s.text_color(text))
                        .child(gpui::svg().path(if expanded { "icons/alt-arrow-up.svg" } else { "icons/alt-arrow-down.svg" }).size(px(12.)).text_color(theme.text_faint))
                        .on_click(move |e, window, cx| on_heading(e, window, cx))
                }),
        )
        .map(|el| if folded { el.child(crate::ui::edge_fade::bottom_faded(48., body)) } else { el.child(body) })
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

/// A state badge: `label` on a wash of `color`.
fn badge(label: &'static str, color: gpui::Hsla) -> Div {
    div()
        .px(px(7.))
        .h(px(18.))
        .flex()
        .items_center()
        .rounded(px(9.))
        .bg(color.opacity(0.15))
        .text_color(color)
        .text_size(px(11.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .child(label)
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

/// The title (a click opens it), its state (and conflicts while open), number and author.
fn request_card(theme: &Theme, r: &Request) -> AnyElement {
    let (label, color) = match r.pr.state {
        "draft" => ("Draft", theme.text_muted),
        "merged" => ("Merged", theme.accent),
        "closed" => ("Closed", theme.danger),
        _ => ("Open", theme.success),
    };
    let number = format!("{}{}", if r.pr.gitlab { "!" } else { "#" }, r.pr.number);
    let (url, accent) = (r.pr.url.clone(), theme.accent);
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .id("review-title")
                .text_size(px(14.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(move |s| s.text_color(accent))
                .child(if r.title.is_empty() { number.clone() } else { r.title.clone() })
                .on_click(move |_, _, cx| cx.open_url(&url)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .text_color(theme.text_muted)
                .child(badge(label, color))
                .when(r.pr.state == "open" && r.accepted, |el| el.child(badge("Approved", theme.success)))
                .when(r.pr.state == "open" && !r.mergeable, |el| el.child(badge("Conflicts", theme.danger)))
                .child(number)
                .when(!r.author.is_empty(), |el| el.child(div().min_w_0().truncate().child(format!("by {}", r.author)))),
        )
        .when(!r.reviewers.is_empty() || r.unresolved.is_some_and(|n| n > 0), |el| {
            use crate::ui::review::Verdict;
            el.child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.))
                    .text_color(theme.text_muted)
                    .children(r.reviewers.iter().map(|rv| {
                        let (icon, color, said) = match rv.verdict {
                            Verdict::Approved => ("icons/check.svg", theme.success, "approved"),
                            Verdict::Changes => ("icons/solar/close-circle.svg", theme.danger, "requested changes"),
                            Verdict::Commented => ("icons/solar/chat-round-dots.svg", theme.text_muted, "commented"),
                            Verdict::Pending => ("icons/solar/clock-circle.svg", theme.text_faint, "hasn't reviewed yet"),
                        };
                        div()
                            .id(SharedString::from(format!("review-reviewer-{}", rv.name)))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .tooltip(tip(format!("{} {said}", rv.name), None))
                            .child(gpui::svg().path(icon).size(px(13.)).flex_none().text_color(color))
                            .child(rv.name.clone())
                    }))
                    .children(r.unresolved.filter(|n| *n > 0).map(|n| badge_owned(format!("{n} unresolved"), theme.warning))),
            )
        })
        .into_any_element()
}

/// [`badge`] with a label of its own.
fn badge_owned(label: String, color: gpui::Hsla) -> Div {
    div()
        .px(px(7.))
        .h(px(18.))
        .flex()
        .items_center()
        .rounded(px(9.))
        .bg(color.opacity(0.15))
        .text_color(color)
        .text_size(px(11.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .child(label)
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
