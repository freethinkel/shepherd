//! Annotations in a browser tab, Zeron's way (`crates/ui/src/browser`, MIT): the toolbar's button injects the picker
//! (`assets/annotate.js`), which is polled for picks only while it runs. Zeron's picks go to its composer; here they
//! go to a panel beside the page, with a note each, that sends them to an agent. Beside it, not over it: off macOS
//! GPUI can't draw above a page. The draft lives in memory, on its page.
use super::web::{Page, PageEvent, PageKey};
use super::*;
use crate::annotations::{BrowserAnnotation, Poll, annotation_prompt, default_recipient};
use crate::model::AgentScope;
use gpui_base::input::{Textarea, TextareaState};
use std::hash::BuildHasher;

const SCRIPT: &str = include_str!("../../assets/annotate.js");
/// How often an active pick is polled. Nothing polls otherwise.
const POLL: Duration = Duration::from_millis(100);
/// One session's picks. The page's script answers the polls, so a hostile page could otherwise stream picks into the
/// draft without a click.
const MAX_PICKS: usize = 24;
/// Unreadable polls (about two seconds' worth) before the session gives up.
const MAX_MISSES: u32 = 20;

/// The `…` menu's choices of which agents the panel lists: the choice, its id, icon and label.
const SCOPES: [(AgentScope, &str, &str, &str); 3] = [
    (AgentScope::Worktree, "annotation-agents-worktree", "icons/git-branch.svg", "This worktree"),
    (AgentScope::Project, "annotation-agents-project", "icons/folder.svg", "This project"),
    (AgentScope::All, "annotation-agents-all", "icons/bot.svg", "All agents"),
];

/// While the user picks on the page. Dropping it stops the poll.
struct Session {
    nonce: u32,
    /// Picks received so far; the session ends at [`MAX_PICKS`].
    picks: usize,
    /// Polls in a row that returned nothing readable.
    misses: u32,
    _poll: gpui::Task<()>,
}

struct Item {
    annotation: BrowserAnnotation,
    /// The page's marker for it, numbered like it.
    marker: u32,
    note: Entity<TextareaState>,
}

/// A page's annotations. Only the polls of a session running add to it, and only a click in Shepherd sends it.
#[derive(Default)]
pub(super) struct Draft {
    session: Option<Session>,
    items: Vec<Item>,
    sending: bool,
}

/// The overlay's palette and type, so the page chrome reads as Shepherd's.
fn annotation_theme(cx: &App) -> serde_json::Value {
    let theme = Theme::of(cx);
    let css = |color: Hsla| {
        let rgba: gpui::Rgba = color.into();
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("#{:02x}{:02x}{:02x}{:02x}", byte(rgba.r), byte(rgba.g), byte(rgba.b), byte(rgba.a))
    };
    json!({
        "accent": css(theme.accent),
        "surface": css(theme.surface),
        "border": css(theme.border),
        "text": css(theme.text),
        "muted": css(theme.text_muted),
        "faint": css(theme.text_faint),
        "chip": css(theme.wash(0.09)),
        "dark": theme.appearance == crate::theme::Appearance::Dark,
        "font": theme.font_ui.to_string(),
    })
}

impl Page {
    /// Annotation needs a loaded page.
    pub(super) fn can_annotate(&self) -> bool {
        self.view.url().is_ok_and(|url| !url.is_empty() && url != "about:blank")
    }
}

impl Draft {
    /// Asks the picker to keep the markers of the annotations left, under their numbers in the list.
    fn sync_markers(&self, numbers_before: &[u32], view: &wry::WebView) {
        let labels: String = self.items.iter().enumerate().map(|(i, item)| format!("a.label({}, {});", item.marker, i + 1)).collect();
        let _ = view.evaluate_script(&format!("(() => {{ const a = window.__shepherdAnnotator; if (a) {{ a.keep({numbers_before:?}); {labels} }} }})()"));
    }
}

impl Shell {
    pub(super) fn annotating(&self, key: &PageKey) -> bool {
        self.pages.live.get(key).is_some_and(|p| p.annotations.session.is_some())
    }

    pub(super) fn toggle_annotation(&mut self, key: &PageKey, window: &mut Window, cx: &mut Context<Self>) {
        match self.annotating(key) {
            true => self.stop_annotation(key),
            false => self.start_annotation(key, window, cx),
        }
        cx.notify();
    }

    fn start_annotation(&mut self, key: &PageKey, window: &mut Window, cx: &mut Context<Self>) {
        // the keyboard to the page, for its Esc
        if self.note_focused(window, cx) {
            window.focus(&self.focus, cx);
        }
        let theme = annotation_theme(cx);
        let Some(page) = self.pages.live.get_mut(key).filter(|p| p.can_annotate()) else { return };
        // Below 2^32, so the page's JavaScript numbers hold it exactly.
        let nonce = (std::collections::hash_map::RandomState::new().hash_one(0) as u32).max(1);
        let _ = page.view.evaluate_script(&format!("{SCRIPT}\n;window.__shepherdAnnotator.start({nonce}, {theme}); 'started'"));
        let polled = key.clone();
        let poll = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(POLL).await;
                let polling = this.update(cx, |s, _| s.poll_annotation(&polled, nonce));
                if !matches!(polling, Ok(true)) {
                    return;
                }
            }
        });
        page.annotations.session = Some(Session { nonce, picks: 0, misses: 0, _poll: poll });
    }

    pub(super) fn stop_annotation(&mut self, key: &PageKey) {
        let Some(page) = self.pages.live.get_mut(key) else { return };
        if page.annotations.session.take().is_some() {
            let _ = page.view.evaluate_script("(() => { window.__shepherdAnnotator?.stop(); return 'stopped'; })()");
        }
    }

    /// Asks the picker for its picks; the answer comes as a `PageEvent`. False once the session is over.
    fn poll_annotation(&self, key: &PageKey, nonce: u32) -> bool {
        let Some(page) = self.pages.live.get(key).filter(|p| p.annotations.session.as_ref().is_some_and(|s| s.nonce == nonce)) else { return false };
        let Some(events) = self.pages.events.clone() else { return false };
        let (key, serial) = (key.clone(), page.serial);
        let take = format!("(() => {{ const a = window.__shepherdAnnotator; return a ? a.take({nonce}) : '{{\"state\":\"gone\"}}'; }})()");
        let _ = page.view.evaluate_script_with_callback(&take, move |result| drop(events.unbounded_send((key.clone(), serial, PageEvent::Picked(result)))));
        true
    }

    /// Picks arrive as the poll's result, the page's data. A `gone` state means the picker left the page (it navigated),
    /// which ends the session; the picks made stay in the panel.
    pub(super) fn annotation_polled(&mut self, key: &PageKey, result: &str, window: &mut Window, cx: &mut Context<Self>) {
        let theme = Theme::of(cx).clone();
        let Some(page) = self.pages.live.get_mut(key) else { return };
        let draft = &mut page.annotations;
        let Some(session) = draft.session.as_mut() else { return };
        let Some(poll) = Poll::parse(result) else {
            // A page that is briefly unreachable. Only a run of these ends the session.
            session.misses += 1;
            if session.misses >= MAX_MISSES {
                self.stop_annotation(key);
            }
            return;
        };
        session.misses = 0;
        let room = MAX_PICKS.saturating_sub(session.picks);
        let picks: Vec<_> = poll.picks.into_iter().take(room).collect();
        session.picks += picks.len();
        let finished = poll.state != "active" || session.picks >= MAX_PICKS;
        let picked = !picks.is_empty();
        if !picked && !finished {
            return;
        }
        for pick in picks {
            let number = draft.items.len() + 1;
            let annotation = BrowserAnnotation { index: number as u32, ..pick.annotation.bounded() };
            let note = cx.new(|cx| TextareaState::new(window, cx).placeholder("Note for the agent").auto_grow(1, 6));
            note.update(cx, |note, _| note.set_editor_style(crate::ui::input_style(&theme)));
            let _ = page.view.evaluate_script(&format!("(() => {{ window.__shepherdAnnotator?.label({}, {number}); return 'ok'; }})()", pick.marker));
            draft.items.push(Item { annotation, marker: pick.marker, note });
        }
        let last = draft.items.last().map(|item| item.note.clone()).filter(|_| picked && finished);
        if finished {
            // `stop` tidies the page's overlay when the cap, not the page, ended the session
            self.stop_annotation(key);
        }
        // the keyboard to the note, as Zeron's to its composer
        if let Some(note) = last {
            note.update(cx, |note, cx| note.focus(window, cx));
        }
        cx.notify();
    }

    /// Whether a note has the keyboard, which then isn't the page's.
    pub(super) fn note_focused(&self, window: &Window, cx: &App) -> bool {
        let shown = self.shown_page(cx).and_then(|(key, _)| self.pages.live.get(&key));
        shown.is_some_and(|p| p.annotations.items.iter().any(|item| item.note.read(cx).focus_handle(cx).is_focused(window)))
    }

    /// Removes annotation `index` and its marker; the ones after it move up a number.
    fn remove_annotation(&mut self, key: &PageKey, index: u32, window: &mut Window, cx: &mut Context<Self>) {
        // the row goes: a note it held the keyboard with would hold it out of sight
        if self.note_focused(window, cx) {
            window.focus(&self.focus, cx);
        }
        let Some(page) = self.pages.live.get_mut(key) else { return };
        let draft = &mut page.annotations;
        draft.items.retain(|item| item.annotation.index != index);
        let kept: Vec<u32> = draft.items.iter().map(|item| item.annotation.index).collect();
        draft.sync_markers(&kept, &page.view);
        for (i, item) in draft.items.iter_mut().enumerate() {
            item.annotation.index = i as u32 + 1;
        }
        cx.notify();
    }

    /// The agents page `key`'s tab can send to, as the panel's `…` menu chose, in `Snapshot::agents`'s order, each with
    /// its workspace's label.
    fn recipients(&self, key: &PageKey, cx: &App) -> Vec<(crate::model::Pane, String)> {
        let (Some(snapshot), Some(tab)) = (&self.snapshot, self.browser.tabs.iter().find(|t| t.id == key.0)) else { return Vec::new() };
        let scope = SettingsGlobal::of(cx).settings.annotation_agents;
        snapshot.agents_in(&tab.workspace, scope).into_iter().map(|p| (p.clone(), self.workspace_label(&p.workspace_id))).collect()
    }

    fn workspace_label(&self, id: &str) -> String {
        let workspaces = self.snapshot.iter().flat_map(|s| &s.workspaces);
        workspaces.into_iter().find(|w| w.workspace_id == id).map(|w| w.label.clone()).unwrap_or_default()
    }

    /// The agent page `key`'s prompt goes to.
    fn recipient(&self, key: &PageKey, cx: &App) -> Option<crate::model::Pane> {
        let tab = self.browser.tabs.iter().find(|t| t.id == key.0)?;
        let agents = self.snapshot.as_ref()?.agents_in(&tab.workspace, SettingsGlobal::of(cx).settings.annotation_agents);
        default_recipient(&agents, self.pages.recipients.get(key).map(String::as_str), Some(tab.workspace.as_str())).cloned()
    }

    /// Only a click in the panel comes here. Sent, the draft and its markers go; failed, it all stays.
    fn send_annotations(&mut self, key: &PageKey, cx: &mut Context<Self>) {
        let Some(agent) = self.recipient(key, cx) else { return };
        let Some(page) = self.pages.live.get_mut(key).filter(|p| !p.annotations.sending && !p.annotations.items.is_empty()) else { return };
        let draft = &mut page.annotations;
        draft.sending = true;
        let picks: Vec<_> = draft.items.iter().map(|item| (item.annotation.clone(), item.note.read(cx).value().to_string())).collect();
        let (key, name, workspace) = (key.clone(), agent.agent.clone().unwrap_or_default(), self.workspace_label(&agent.workspace_id));
        self.send_then(Command::Reply { pane_id: agent.pane_id, text: annotation_prompt(&picks) }, cx, move |shell, result, cx| {
            match result {
                Ok(_) => {
                    shell.stop_annotation(&key);
                    if let Some(page) = shell.pages.live.get_mut(&key) {
                        page.annotations.items.clear();
                        page.annotations.sync_markers(&[], &page.view);
                    }
                    crate::ui::toast::info(cx, format!("Sent to {name} in {workspace}"), None);
                }
                Err(e) => crate::ui::toast::error(cx, format!("Not sent to {name}: {e:#}")),
            }
            if let Some(page) = shell.pages.live.get_mut(&key) {
                page.annotations.sending = false;
            }
            cx.notify();
        });
        cx.notify();
    }

    /// The toolbar's button, left of Open in Browser, in the accent while picking.
    pub(super) fn annotate_button(&self, key: &PageKey, theme: &Theme, cx: &mut Context<Self>) -> Stateful<Div> {
        let annotating = self.annotating(key);
        let can_annotate = self.pages.live.get(key).is_some_and(Page::can_annotate);
        let key = key.clone();
        div()
            .id("browser-annotate")
            .size(px(28.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(8.))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .tooltip(tip(if annotating { "Stop annotating" } else { "Annotate an element" }, None))
            .when(!can_annotate, |el| el.opacity(0.35))
            .when(annotating, |el| el.bg(theme.wash(0.10)))
            .child(gpui::svg().path("icons/solar/cursor-square.svg").size(px(16.)).text_color(if annotating { theme.accent } else { theme.text_muted }))
            .when(can_annotate, |el| {
                el.cursor_pointer().hover(|s| s.bg(theme.hover)).on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.toggle_annotation(&key, window, cx)))
            })
    }

    /// The notes under the page, which needs the panel's width, while picking and while picks wait to be sent.
    pub(super) fn render_annotations(&self, key: &PageKey, theme: &Theme, cx: &mut Context<Self>) -> Option<Div> {
        let page = self.pages.live.get(key).filter(|p| p.annotations.session.is_some() || !p.annotations.items.is_empty())?;
        let draft = &page.annotations;
        let rows: Vec<Stateful<Div>> = draft.items.iter().map(|item| self.annotation_row(key, item, theme, cx)).collect();
        let hint = draft.items.is_empty().then(|| div().p(px(8.)).text_color(theme.text_faint).child("Click an element or drag over an area."));
        let chosen = self.recipient(key, cx).map(|p| p.pane_id);
        let recipients = self.recipients(key, cx);
        let (heading, empty) = match SettingsGlobal::of(cx).settings.annotation_agents {
            AgentScope::Worktree => ("Agents in this worktree", "No agents in this worktree."),
            AgentScope::Project => ("Agents in this project", "No agents in this project."),
            AgentScope::All => ("All agents", "No agents running."),
        };
        let none = recipients.is_empty();
        let agents = recipients.into_iter().map(|(pane, workspace)| {
            let (id, picked, key) = (pane.pane_id.clone(), chosen.as_deref() == Some(pane.pane_id.as_str()), key.clone());
            row(theme, picked, SharedString::from(format!("annotate-agent-{id}")))
                .h(px(28.))
                .child(status_dot(pane.agent_status, theme))
                .child(div().flex_none().text_color(theme.text).child(pane.agent.clone().unwrap_or_default()))
                .child(div().flex_1().min_w_0().truncate().text_color(theme.text_muted).child(workspace))
                .child(div().flex_none().text_size(px(11.)).text_color(theme.text_faint).child(status_word(pane.agent_status)))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.pages.recipients.insert(key.clone(), id.clone());
                    cx.notify();
                }))
        });
        let ready = !none && !draft.sending && !draft.items.is_empty();
        let (send_key, stop_key) = (key.clone(), key.clone());
        let hovered = theme.accent.opacity(0.85);
        Some(
            div()
                .w_full()
                .h(relative(0.4))
                .flex_none()
                .flex()
                .flex_col()
                .bg(theme.surface)
                .border_t_1()
                .border_color(theme.border)
                .text_size(px(12.))
                .capture_key_down(cx.listener(move |this, e: &gpui::KeyDownEvent, _, cx| {
                    if e.keystroke.key == "escape" && this.annotating(&stop_key) {
                        cx.stop_propagation();
                        this.stop_annotation(&stop_key);
                        cx.notify();
                    }
                }))
                .child(div().id("annotations-list").flex_1().min_h_0().overflow_y_scroll().p(px(8.)).flex().flex_col().gap(px(8.)).children(hint).children(rows))
                .child(
                    div()
                        .id("annotations-agents")
                        .flex_none()
                        .max_h(px(160.))
                        .overflow_y_scroll()
                        .p(px(8.))
                        .border_t_1()
                        .border_color(theme.border)
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .px(px(8.))
                                .pb(px(2.))
                                .text_size(px(11.))
                                .text_color(theme.text_faint)
                                .child(heading)
                                .child(
                                    icon_button(theme, "annotation-agents-view", "")
                                        .tooltip(tip("Which agents to show", None))
                                        .size(px(20.))
                                        .child(gpui::svg().path("icons/more-horizontal.svg").size(px(14.)).text_color(theme.text_faint))
                                        .when(self.annotation_agents_menu.as_ref().is_some_and(|m| m.is_open()), |el| el.bg(theme.hover))
                                        // capture, like the sidebar's `…`: a press on the open menu's trigger closes it
                                        .capture_any_mouse_down(cx.listener(|this, e: &MouseDownEvent, _, cx| {
                                            cx.stop_propagation();
                                            match this.annotation_agents_menu.as_mut() {
                                                Some(menu) if menu.is_open() => menu.close(),
                                                _ => this.annotation_agents_menu = Some(crate::ui::Popup::new(e.position)),
                                            }
                                            cx.notify();
                                        })),
                                ),
                        )
                        .children(agents)
                        .when(none, |el| el.child(div().px(px(8.)).text_color(theme.text_faint).child(empty))),
                )
                .child(
                    div().flex_none().p(px(8.)).pt(px(0.)).child(
                        div()
                            .id("annotations-send")
                            .h(px(32.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(8.))
                            .text_color(if ready { gpui::white() } else { theme.text_faint })
                            .bg(if ready { theme.accent } else { theme.wash(0.06) })
                            .child("Send to agent")
                            .when(ready, |el| {
                                el.cursor_pointer().hover(move |s| s.bg(hovered)).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.send_annotations(&send_key, cx)))
                            }),
                    ),
                ),
        )
    }

    /// The panel's `…` menu: which agents it lists.
    pub(super) fn annotation_agents_menu(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let menu = self.annotation_agents_menu.as_ref()?;
        let scope = SettingsGlobal::of(cx).settings.annotation_agents;
        let card = menu_card(theme, "annotation-agents-menu", 160.)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if let Some(m) = this.annotation_agents_menu.as_mut() {
                    m.close()
                }
                cx.notify();
            }))
            .children(SCOPES.map(|(choice, id, icon, label)| {
                menu_choice(theme, id, icon, label, scope == choice).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    save_from_sidebar(cx, |s| s.annotation_agents = choice);
                    if let Some(m) = this.annotation_agents_menu.as_mut() {
                        m.close()
                    }
                    cx.notify();
                }))
            }));
        let menu_el = crate::ui::menu_motion("annotation-agents-menu-in", menu, card);
        Some(gpui::deferred(gpui::anchored().position(menu.value).anchor(gpui::Anchor::BottomRight).snap_to_window_with_margin(px(8.)).child(menu_el)).with_priority(1).into_any_element())
    }

    /// An annotation's row: hovered, its element's outline deepens on the page.
    fn annotation_row(&self, key: &PageKey, item: &Item, theme: &Theme, cx: &mut Context<Self>) -> Stateful<Div> {
        let index = item.annotation.index;
        let (removed, hovered) = (key.clone(), key.clone());
        let hover = theme.hover;
        div()
            .id(SharedString::from(format!("annotation-{index}")))
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(4.))
            .on_hover(cx.listener(move |this, on: &bool, _, _| {
                if let Some(page) = this.pages.live.get(&hovered) {
                    let number = if *on { index } else { 0 };
                    let _ = page.view.evaluate_script(&format!("(() => {{ window.__shepherdAnnotator?.focus({number}); return 'ok'; }})()"));
                }
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().flex_none().px(px(5.)).rounded(px(3.)).bg(theme.accent).text_color(gpui::white()).text_size(px(11.)).child(index.to_string()))
                    .child(div().flex_1().min_w_0().truncate().font_family("Geist Mono").text_size(px(11.)).text_color(theme.text).child(item.annotation.summary()))
                    .child(
                        div()
                            .id(SharedString::from(format!("annotation-remove-{index}")))
                            .size(px(20.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.))
                            .cursor_pointer()
                            .hover(move |s| s.bg(hover))
                            .child(gpui::svg().path("icons/close.svg").size(px(12.)).text_color(theme.text_muted))
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.remove_annotation(&removed, index, window, cx))),
                    ),
            )
            .when(!item.annotation.text.is_empty(), |el| el.child(div().truncate().text_color(theme.text_muted).child(item.annotation.text.clone())))
            .child(div().px(px(8.)).py(px(5.)).rounded(px(6.)).border_1().border_color(theme.border).bg(theme.wash(0.04)).child(Textarea::new(&item.note)))
    }
}

/// The word for an agent's status beside its dot.
fn status_word(status: AgentStatus) -> &'static str {
    match status {
        AgentStatus::Working => "working",
        AgentStatus::Blocked => "blocked",
        AgentStatus::Done => "done",
        AgentStatus::Idle => "idle",
        AgentStatus::Unknown => "unknown",
    }
}
