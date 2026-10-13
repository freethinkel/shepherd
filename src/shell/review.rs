//! The review panel on the right, after Superconductor's: the shown pane's branch's PR or MR (title, state, author,
//! description) and its checks or pipeline jobs, each opening in the browser.
use super::*;
use gpui::AnyElement;
use crate::settings::{REVIEW_MAX, REVIEW_MIN, REVIEW_WIDTH};
use crate::ui::review::{Check, CheckState, Review, duration, folds, markdown, preview, read_review};
use crate::ui::statusbar::{branch_of, cached, fetching, pane_dir, refetch};

/// How often gh or glab are asked again about a branch, as the status bar asks about its PR.
const REVIEW_TTL: Duration = Duration::from_secs(120);
/// How many lines of a description show folded.
const FOLDED_LINES: usize = 4;
/// How often while one of its checks runs or waits to.
pub(super) const REVIEW_POLL: Duration = Duration::from_secs(10);

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

    /// The panel, and its edge: drag to resize, double-click for the default width.
    pub(super) fn render_review(&mut self, theme: &Theme, snapshot: Option<&Snapshot>, window: &mut Window, cx: &mut Context<Self>) -> [AnyElement; 2] {
        let dir = pane_dir(snapshot, self.active_pane(window, cx).as_deref());
        let branch = dir.as_deref().and_then(|d| branch_of(d, cx));
        let key = dir.zip(branch.clone()).map(|(d, b)| format!("{d}\n{b}"));
        let active = window.is_window_active();
        let review = key.as_deref().and_then(|k| cached(k, if active { REVIEW_TTL } else { Duration::MAX }, read_review, cx));
        let busy = key.as_deref().is_some_and(|k| fetching::<Review>(k, cx));
        self.review_poll = key.clone().filter(|_| active && review.as_ref().is_some_and(Review::running));

        let gitlab = matches!(&review, Some(Review::Found { request: Some(r), .. }) if r.pr.gitlab)
            || matches!(&review, Some(Review::Found { pipeline: Some(_), .. }) | Some(Review::NoTool("glab") | Review::NoLogin("glab", _)));
        let noun = if gitlab { "merge request" } else { "pull request" };
        let header = div()
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            .px(px(12.))
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
            }));

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
                    Some(r) => out.push(request_card(theme, r)),
                    None => out.push(empty(theme, format!("No {noun}"), format!("{} has none yet.", branch.unwrap_or_default()))),
                }
                if !checks.is_empty() || pipeline.is_some() {
                    out.push(checks_section(theme, if gitlab { "Pipeline" } else { "Checks" }, &checks, pipeline, window));
                }
                if let Some(r) = request.filter(|r| !r.body.trim().is_empty()) {
                    let toggle = cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.review_expanded = !this.review_expanded;
                        cx.notify();
                    });
                    out.push(description(theme, &r.body, self.review_expanded, toggle));
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
            .child(div().id("review-body").flex_1().min_h_0().overflow_y_scroll().p(px(12.)).flex().flex_col().gap(px(16.)).children(body));
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

/// The description: folded, its text in a few lines; unfolded, its blocks. The heading, or a click on the folded
/// text, folds and unfolds it.
fn description(theme: &Theme, body: &str, expanded: bool, toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> AnyElement {
    let blocks = markdown(body);
    let long = folds(&blocks);
    let toggle = Rc::new(toggle);
    let on_heading = toggle.clone();
    let text = theme.text;
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
        .map(|el| match long && !expanded {
            true => el.child(
                div()
                    .id("review-description-folded")
                    .line_clamp(FOLDED_LINES)
                    .text_ellipsis()
                    .cursor_pointer()
                    .child(preview(&blocks))
                    .on_click(move |e, window, cx| toggle(e, window, cx)),
            ),
            false => el.children(crate::ui::markdown::render(&blocks, "review-md", theme)),
        })
        .into_any_element()
}

/// The title (a click opens it), its state, number and author.
fn request_card(theme: &Theme, r: &crate::ui::review::Request) -> AnyElement {
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
                .child(
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
                        .child(label),
                )
                .child(number)
                .when(!r.author.is_empty(), |el| el.child(div().min_w_0().truncate().child(format!("by {}", r.author)))),
        )
        .into_any_element()
}

/// The checks or jobs, with how many failed, run and passed; a GitLab pipeline's title carries its own state and
/// opens it.
fn checks_section(theme: &Theme, title: &'static str, checks: &[Check], pipeline: Option<(String, CheckState)>, window: &mut Window) -> AnyElement {
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
    let heading = div()
        .id("review-pipeline")
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(px(11.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(theme.text_faint)
        .children(pipeline.as_ref().map(|(_, state)| check_icon(theme, *state, window)))
        .child(div().flex_1().child(title))
        .child(summary.join(" · "))
        .when_some(pipeline.map(|(url, _)| url), |el, url| el.cursor_pointer().hover(|s| s.underline()).on_click(move |_, _, cx| cx.open_url(&url)));
    div()
        .flex()
        .flex_col()
        .gap(px(2.))
        .text_size(px(12.))
        .child(div().pb(px(4.)).child(heading))
        .children(checks.iter().enumerate().map(|(i, c)| {
            div()
                .id(("review-check", i))
                .h(px(28.))
                .px(px(6.))
                .mx(px(-6.))
                .flex()
                .items_center()
                .gap(px(8.))
                .rounded(px(6.))
                .child(check_icon(theme, c.state, window))
                .child(div().flex_1().min_w_0().truncate().text_color(theme.text).child(c.name.clone()))
                .when(!c.group.is_empty(), |el| el.child(div().min_w_0().max_w(px(96.)).truncate().text_color(theme.text_faint).child(c.group.clone())))
                .children(c.secs.map(|s| div().flex_none().text_color(theme.text_faint).child(duration(s))))
                .when_some(c.url.clone(), |el, url| el.cursor_pointer().hover(move |s| s.bg(hover)).on_click(move |_, _, cx| cx.open_url(&url)))
        }))
        .into_any_element()
}

fn check_icon(theme: &Theme, state: CheckState, window: &mut Window) -> AnyElement {
    let (icon, color) = match state {
        CheckState::Success => ("icons/solar/check-circle.svg", theme.success),
        CheckState::Failed => ("icons/solar/close-circle.svg", theme.danger),
        CheckState::Pending => ("icons/solar/clock-circle.svg", theme.warning),
        CheckState::Skipped => ("icons/solar/forbidden-circle.svg", theme.text_faint),
        CheckState::Running => {
            let loader = crate::ui::loader::mini_spinner(2.5, theme.warning, window);
            return div().size(px(14.)).flex_none().flex().items_center().justify_center().child(loader).into_any_element();
        }
    };
    gpui::svg().path(icon).size(px(14.)).flex_none().text_color(color).into_any_element()
}
