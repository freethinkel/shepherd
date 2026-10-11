//! The command panel (cmd-k, Zeron's command palette): every app action with its chord, then every workspace
//! and tab of the snapshot, filtered as you type.
use crate::bindings::ACTIONS;
use crate::model::Snapshot;
use crate::theme::Theme;
use crate::ui::Popup;
use gpui::{
    AppContext, Context, Entity, EventEmitter, InteractiveElement, IntoElement, ParentElement, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, div, point, prelude::FluentBuilder, px,
};
use gpui_base::input::{Escape, Input, InputEvent, InputState, MoveDown, MoveUp};

/// What an item does when run.
#[derive(Debug, Clone, PartialEq)]
pub enum Run {
    /// An app action by its name in [`ACTIONS`], dispatched as its key binding would be.
    Action(&'static str),
    /// Show this tab.
    Tab(String),
}

#[derive(Debug, Clone)]
pub struct Item {
    pub label: String,
    /// The action's chord, or a tab's workspace.
    pub hint: Option<String>,
    pub run: Run,
}

/// Every action but the panel's own (with its chord in `bindings`), then a "Go to workspace" per workspace
/// with a tab to show and a "Go to tab" per tab, in the sidebar's order.
pub fn items(snapshot: Option<&Snapshot>, bindings: &[(String, String)]) -> Vec<Item> {
    let chord = |name: &str| {
        let (_, keys) = bindings.iter().find(|(action, _)| action == name)?;
        Some(keys.split_whitespace().map(crate::ui::settings::display_chord).collect::<Vec<_>>().join(" "))
    };
    let mut out: Vec<Item> = ACTIONS
        .iter()
        .filter(|(name, _)| *name != "CommandPanel")
        .map(|(name, label)| Item { label: label.to_string(), hint: chord(name), run: Run::Action(name) })
        .collect();
    let Some(s) = snapshot else { return out };
    let workspaces: Vec<_> = s.workspace_tree().into_iter().map(|(w, _)| w).collect();
    for w in &workspaces {
        // the tab a click on its sidebar row shows
        let tab = w.active_tab_id.clone().or_else(|| s.tabs_of(&w.workspace_id).first().map(|t| t.tab_id.clone()));
        if let Some(tab) = tab {
            out.push(Item { label: format!("Go to workspace {}", w.label), hint: None, run: Run::Tab(tab) });
        }
    }
    for w in &workspaces {
        for t in s.tabs_of(&w.workspace_id) {
            out.push(Item { label: format!("Go to tab {}", t.label), hint: Some(w.label.clone()), run: Run::Tab(t.tab_id.clone()) });
        }
    }
    out
}

/// How well `query` matches `text`, ignoring case, or None: `text` starting with it ranks first, then it
/// starting a word, then anywhere, then its letters in order (more of them on word starts, fewer gaps, higher).
pub fn score(query: &str, text: &str) -> Option<i32> {
    let q: Vec<char> = query.trim().to_lowercase().chars().collect();
    if q.is_empty() {
        return Some(0);
    }
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let word_start = |i: usize| i == 0 || !t[i - 1].is_alphanumeric();
    let hits: Vec<usize> = (0..t.len()).filter(|&i| t[i..].starts_with(&q)).collect();
    if hits.first() == Some(&0) {
        return Some(3000);
    }
    if hits.iter().any(|&i| word_start(i)) {
        return Some(2000);
    }
    if !hits.is_empty() {
        return Some(1000);
    }
    // ponytail: greedy leftmost letters; "nt" scores Next tab's t in "Next", not "tab". Best alignment (fzf's DP) if it matters.
    let (mut score, mut next) = (0, 0);
    for c in q.into_iter().filter(|c| !c.is_whitespace()) {
        let i = (next..t.len()).find(|&i| t[i] == c)?;
        score += if word_start(i) { 10 } else { 0 } - (i - next) as i32;
        next = i + 1;
    }
    Some(score.min(999))
}

/// Indices of the `labels` matching `query`, best first; ties keep their order.
pub fn rank<'a>(query: &str, labels: impl IntoIterator<Item = &'a str>) -> Vec<usize> {
    let mut hits: Vec<(usize, i32)> = labels.into_iter().enumerate().filter_map(|(i, l)| Some((i, score(query, l)?))).collect();
    hits.sort_by_key(|&(_, s)| std::cmp::Reverse(s));
    hits.into_iter().map(|(i, _)| i).collect()
}

pub enum PanelEvent {
    Run(Run),
    Dismiss,
}

pub struct Panel {
    input: Entity<InputState>,
    items: Vec<Item>,
    /// Indices into `items` of the rows shown, best match first.
    matches: Vec<usize>,
    /// Index into `matches`.
    selected: usize,
    scroll: ScrollHandle,
    _events: Subscription,
}

impl EventEmitter<PanelEvent> for Panel {}

impl Panel {
    pub fn new(items: Vec<Item>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let theme = Theme::of(cx).clone();
        let input = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("Type a command, workspace or tab");
            input.set_editor_style(crate::ui::input_style(&theme));
            input
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        let events = cx.subscribe_in(&input, window, |panel, _, event: &InputEvent, _, cx| match event {
            InputEvent::Change => panel.refilter(cx),
            InputEvent::PressEnter { .. } => panel.run(panel.selected, cx),
            _ => {}
        });
        let matches = (0..items.len()).collect();
        Panel { input, items, matches, selected: 0, scroll: ScrollHandle::new(), _events: events }
    }

    fn refilter(&mut self, cx: &mut Context<Self>) {
        let query = self.input.read(cx).value();
        self.matches = rank(&query, self.items.iter().map(|i| i.label.as_str()));
        self.selected = 0;
        self.scroll.scroll_to_item(0);
        cx.notify();
    }

    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let n = self.matches.len() as isize;
        if n > 0 {
            self.selected = (self.selected as isize + by).rem_euclid(n) as usize;
            self.scroll.scroll_to_item(self.selected);
            cx.notify();
        }
    }

    fn run(&mut self, row: usize, cx: &mut Context<Self>) {
        if let Some(&i) = self.matches.get(row) {
            cx.emit(PanelEvent::Run(self.items[i].run.clone()));
        }
    }
}

impl Render for Panel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let rows = self.matches.iter().enumerate().map(|(row, &i)| {
            let item = &self.items[i];
            let selected = row == self.selected;
            let chord = matches!(item.run, Run::Action(_));
            div()
                .id(("command-item", row))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(10.))
                .min_h(px(30.))
                .px(px(8.))
                .py(px(4.))
                .rounded(px(10.))
                .cursor_pointer()
                .when(selected, |el| el.bg(theme.selected))
                // the pointer moves the highlight, so hover and keyboard never light two rows
                .on_mouse_move(cx.listener(move |panel, _, _, cx| {
                    if panel.selected != row {
                        panel.selected = row;
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |panel, _, _, cx| panel.run(row, cx)))
                .child(div().flex_1().min_w_0().truncate().child(item.label.clone()))
                .children(item.hint.clone().map(|hint| {
                    if chord {
                        kbd(&theme, hint)
                    } else {
                        div().flex_none().max_w(px(180.)).truncate().text_size(px(12.)).text_color(theme.text_faint).child(hint)
                    }
                }))
        });
        div()
            .id("command-panel")
            .occlude()
            .w(px(560.))
            .flex()
            .flex_col()
            .rounded(px(16.))
            .bg(theme.card)
            .border_1()
            .border_color(theme.border)
            .shadow_lg()
            .text_color(theme.text)
            .text_size(px(13.))
            // before the input's own Up, Down and Escape
            .capture_action(cx.listener(|panel, _: &MoveUp, _, cx| {
                cx.stop_propagation();
                panel.step(-1, cx);
            }))
            .capture_action(cx.listener(|panel, _: &MoveDown, _, cx| {
                cx.stop_propagation();
                panel.step(1, cx);
            }))
            .capture_action(cx.listener(|_, _: &Escape, _, cx| {
                cx.stop_propagation();
                cx.emit(PanelEvent::Dismiss);
            }))
            .on_mouse_down_out(cx.listener(|_, _, _, cx| cx.emit(PanelEvent::Dismiss)))
            .child(
                div()
                    .min_h(px(44.))
                    .px(px(16.))
                    .py(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(gpui::svg().path("icons/palette-search.svg").size(px(16.)).flex_none().text_color(theme.text_muted))
                    .child(div().flex_1().min_w_0().text_size(px(14.)).child(Input::new(&self.input))),
            )
            .child(
                div()
                    .id("command-items")
                    .max_h(px(360.))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .p(px(8.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .children(rows)
                    .when(self.matches.is_empty(), |el| {
                        el.child(div().py(px(20.)).flex().justify_center().text_color(theme.text_muted).child("No results"))
                    }),
            )
    }
}

/// Zeron's key-cap hint.
fn kbd(theme: &Theme, keys: String) -> gpui::Div {
    div()
        .flex_none()
        .px(px(5.))
        .py(px(1.))
        .rounded(px(5.))
        .bg(theme.wash(0.05))
        .text_size(px(10.))
        .font_family("Geist Mono")
        .text_color(theme.text_muted)
        .child(SharedString::from(keys))
}

/// The panel over a 35 % scrim, a fifth of the way down the window; both fade in and out with the popup.
pub fn overlay<T>(popup: &Popup<(Entity<Panel>, T)>, window: &Window) -> impl IntoElement + use<T> {
    let viewport = window.viewport_size();
    let card = div().id("command-panel-motion").child(popup.value.0.clone());
    let scrim = div()
        .id("command-panel-scrim")
        .occlude()
        .w(viewport.width)
        .h(viewport.height)
        .pt(viewport.height * 0.18)
        .bg(gpui::black().opacity(0.35))
        .flex()
        .justify_center()
        .items_start()
        .child(crate::ui::dialog_motion("command-panel-in", popup, card, 2.));
    gpui::deferred(gpui::anchored().position(point(px(0.), px(0.))).child(crate::ui::dialog_motion("command-panel-scrim-in", popup, scrim, 0.)))
        .with_priority(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranked<'a>(query: &str, labels: &[&'a str]) -> Vec<&'a str> {
        rank(query, labels.iter().copied()).into_iter().map(|i| labels[i]).collect()
    }

    #[test]
    fn prefix_then_word_start_then_substring_then_letters() {
        let labels = ["Close tab", "Split right", "Focus pane right", "Righteous", "Bright", "Go right"];
        assert_eq!(ranked("right", &labels), ["Righteous", "Split right", "Focus pane right", "Go right", "Bright"]);
    }

    #[test]
    fn letters_in_order_ignoring_case_and_spaces() {
        assert!(score("SPR", "Split right").is_some());
        assert!(score("sp r", "Split right").is_some());
        assert_eq!(score("rs", "Split right"), None, "letters out of order");
        assert_eq!(score("zz", "Split right"), None);
    }

    #[test]
    fn word_start_letters_beat_scattered_ones() {
        assert!(score("sr", "Split right") > score("sr", "Toggle sidebar"));
        assert!(score("nt", "New tab") > score("nt", "Focus pane left"));
    }

    #[test]
    fn an_empty_query_keeps_every_item_in_order() {
        assert_eq!(ranked("  ", &["b", "a"]), ["b", "a"]);
    }

    #[test]
    fn items_list_actions_with_chords_then_workspaces_and_tabs() {
        let snapshot: Snapshot = serde_json::from_value(serde_json::json!({
            "focused_workspace_id": null, "focused_tab_id": null, "focused_pane_id": null,
            "workspaces": [
                {"workspace_id": "w1", "label": "shepherd", "number": 1, "active_tab_id": "t2", "agent_status": "idle"},
                {"workspace_id": "w2", "label": "empty", "number": 2, "active_tab_id": null, "agent_status": "idle"}
            ],
            "tabs": [
                {"tab_id": "t1", "workspace_id": "w1", "label": "zsh", "number": 1, "agent_status": "idle"},
                {"tab_id": "t2", "workspace_id": "w1", "label": "claude", "number": 2, "agent_status": "idle"}
            ],
            "panes": [], "layouts": []
        }))
        .unwrap();
        let bindings = vec![("NewTab".to_string(), "cmd-t".to_string()), ("CommandPanel".to_string(), "cmd-k".to_string())];
        let items = items(Some(&snapshot), &bindings);
        let new_tab = items.iter().find(|i| i.label == "New tab").unwrap();
        assert_eq!((new_tab.hint.as_deref(), &new_tab.run), (Some("Cmd+T"), &Run::Action("NewTab")));
        assert!(items.iter().all(|i| i.run != Run::Action("CommandPanel")), "the panel does not list itself");
        let rest: Vec<_> = items.iter().filter(|i| matches!(i.run, Run::Tab(_))).map(|i| (i.label.as_str(), i.run.clone())).collect();
        assert_eq!(
            rest,
            [
                ("Go to workspace shepherd", Run::Tab("t2".into())),
                ("Go to tab zsh", Run::Tab("t1".into())),
                ("Go to tab claude", Run::Tab("t2".into())),
            ],
            "a workspace without tabs has nothing to show"
        );
    }
}
