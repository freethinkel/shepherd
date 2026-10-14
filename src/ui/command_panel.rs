//! The command panel (cmd-k, Zeron's command palette): every app action with its chord, then every workspace
//! and tab of the snapshot, filtered as you type. The agents panel is the same list over the agents.
use crate::bindings::{ACTIONS, ActiveBindings};
use crate::model::{AgentSort, AgentStatus, Since, Snapshot, ago, waiting_since};
use crate::theme::Theme;
use crate::ui::Popup;
use gpui::{
    App, AppContext, Context, Entity, EventEmitter, HighlightStyle, InteractiveElement, IntoElement, KeyBinding, Keystroke,
    ParentElement, Render, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, StyledText, Subscription, Window,
    div, linear_color_stop, linear_gradient, point, prelude::FluentBuilder, px,
};
use gpui_base::input::{Escape, Input, InputEvent, InputState, MoveDown, MoveUp};

/// What an item does when run.
#[derive(Debug, Clone, PartialEq)]
pub enum Run {
    /// An app action by its name in [`ACTIONS`], dispatched as its key binding would be.
    Action(&'static str),
    /// Show this tab.
    Tab(String),
    /// Check out this branch in the shown pane's repo.
    Branch(String),
    /// A herdr plugin's action.
    Herdr(crate::herdr::keys::HerdrCommand),
    /// Use this theme variant for the shown appearance.
    Theme(String),
    /// Show this worktree of `repo`'s repo (a workspace of its checkout): its workspace when one has it open, else
    /// herdr opens it.
    Worktree { repo: String, path: String, workspace_id: Option<String> },
}

/// The heading an item sits under; also searched, so "tab" finds the tabs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Section {
    Recent,
    Commands,
    Workspaces,
    Tabs,
    Agents,
    Branches,
    Themes,
    Worktrees,
}

impl Section {
    fn title(self) -> &'static str {
        match self {
            Section::Recent => "Recent",
            Section::Commands => "Commands",
            Section::Workspaces => "Workspaces",
            Section::Tabs => "Tabs",
            Section::Agents => "Agents",
            Section::Branches => "Branches",
            Section::Themes => "Themes",
            Section::Worktrees => "Worktrees",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    /// Names the item in the recent list: an action's name, `workspace:<id>` or `tab:<id>`.
    pub key: String,
    pub section: Section,
    pub icon: &'static str,
    pub label: String,
    /// Faint text on the right: the workspace of a tab or agent.
    pub hint: Option<String>,
    /// An action's chord in GPUI's spelling, drawn as key caps.
    pub chord: Option<String>,
    /// The sidebar's status dot, for agents and for tabs and workspaces with an agent at work.
    pub status: Option<AgentStatus>,
    pub run: Run,
}

impl Item {
    /// What the query matches: an action's label, else the label, its workspace and its section. The label
    /// comes first so match positions below its length are in the label.
    fn haystack(&self) -> String {
        match self.run {
            Run::Action(_) | Run::Branch(_) | Run::Herdr(_) | Run::Theme(_) => self.label.clone(),
            // the branch, then its folder
            Run::Worktree { .. } => [Some(self.label.as_str()), self.hint.as_deref()].into_iter().flatten().collect::<Vec<_>>().join(" "),
            Run::Tab(_) => [Some(self.label.as_str()), self.hint.as_deref(), Some(self.section.title())].into_iter().flatten().collect::<Vec<_>>().join(" "),
        }
    }
}

/// Which list a panel shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Commands,
    Agents,
    Workspaces,
    Branches,
    Themes,
    Worktrees,
}

impl Kind {
    fn action(self) -> &'static str {
        match self {
            Kind::Commands => "CommandPanel",
            Kind::Agents => "AgentsPanel",
            Kind::Workspaces => "WorkspacesPanel",
            Kind::Branches => "", // opened from the status bar, no chord
            Kind::Worktrees => "", // opened from a workspace's menu, no chord
            Kind::Themes => "ChangeTheme",
        }
    }
}

const CONTEXT: &str = "CommandPanel";

/// Emacs's ctrl-p and ctrl-n move the highlight like Up and Down.
pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-p", MoveUp, Some(CONTEXT)), KeyBinding::new("ctrl-n", MoveDown, Some(CONTEXT))]);
}

/// Zeron's icon for an action.
fn action_icon(name: &str) -> &'static str {
    match name {
        "NewTab" => "icons/plus.svg",
        "CloseTab" => "icons/close-circle.svg",
        "NewWorkspace" => "icons/add-circle.svg",
        "SplitRight" => "icons/split-columns.svg",
        "SplitDown" => "icons/split-rows.svg",
        "ClosePane" => "icons/close.svg",
        "FocusLeft" => "icons/arrow-left.svg",
        "FocusRight" => "icons/arrow-right.svg",
        "FocusUp" => "icons/arrow-up.svg",
        "FocusDown" => "icons/arrow-down.svg",
        "ToggleZoom" => "icons/expand-arrows.svg",
        "PreviousTab" => "icons/alt-arrow-left.svg",
        "NextTab" => "icons/alt-arrow-right.svg",
        "PreviousWorkspace" => "icons/alt-arrow-up.svg",
        "NextWorkspace" => "icons/alt-arrow-down.svg",
        "ToggleSidebar" => "icons/sidebar-minimalistic-left.svg",
        "ToggleReview" => "icons/sidebar-minimalistic-right.svg",
        "OpenSettings" => "icons/settings.svg",
        "ChangeTheme" => "icons/sun.svg",
        "ToggleSecureInput" => "icons/keyboard.svg",
        "AgentsPanel" => "icons/bot.svg",
        "WorkspacesPanel" => "icons/folder.svg",
        "Quit" => "icons/logout-2.svg",
        _ => "icons/terminal.svg",
    }
}

/// Zeron's one brand-colored mark keeps its color; every other icon takes `ink`.
pub(crate) fn icon_ink(icon: &str, ink: gpui::Hsla) -> gpui::Hsla {
    if icon == "icons/claude-mark.svg" { gpui::rgb(0xD97757).into() } else { ink }
}

/// The harness's mark for an agent herdr detected (its `agent start --kind` names), else a robot.
pub(crate) fn agent_icon(agent: &str) -> &'static str {
    match agent {
        "claude" => "icons/claude-mark.svg",
        "codex" => "icons/openai-mark.svg",
        "cursor" => "icons/cursor-mark.svg",
        "agy" | "antigravity_cli" => "icons/antigravity-mark.svg", // herdr's agent kind, and its integration target
        "opencode" => "icons/opencode-mark.svg",
        "grok" => "icons/grok-mark.svg",
        "devin" => "icons/devin-mark.svg",
        "pi" | "omp" => "icons/pi-mark.svg", // oh-my-pi is pi's fork
        "copilot" => "icons/copilot-mark.svg",
        "droid" => "icons/factory-mark.svg",
        "kimi" => "icons/kimi-mark.svg",
        "kilo" => "icons/kilo-mark.svg",
        "qoder" | "qodercli" => "icons/qoder-mark.svg",
        "qwen" => "icons/qwen-mark.svg",
        "mastra" | "mastracode" => "icons/mastra-mark.svg",
        "hermes" => "icons/hermes-mark.svg",
        _ => "icons/bot.svg",
    }
}

/// Every action but the panel's own (with its chord in `bindings`) and herdr's plugin actions as (id, title),
/// then every workspace with a tab to show and every tab, in the sidebar's order.
/// Every workspace in the sidebar's order, running the tab a click on its row shows: the workspaces panel, and a
/// section of the command panel.
pub fn workspace_items(snapshot: Option<&Snapshot>) -> Vec<Item> {
    let Some(s) = snapshot else { return Vec::new() };
    let busy = |status: AgentStatus| (!matches!(status, AgentStatus::Idle | AgentStatus::Unknown)).then_some(status);
    s.workspace_tree()
        .into_iter()
        .filter_map(|(w, _)| {
            let tab = w.active_tab_id.clone().or_else(|| s.tabs_of(&w.workspace_id).first().map(|t| t.tab_id.clone()))?;
            let linked = w.worktree.as_ref().is_some_and(|t| t.is_linked_worktree);
            Some(Item {
                key: format!("workspace:{}", w.workspace_id),
                section: Section::Workspaces,
                icon: if linked { "icons/git-branch.svg" } else { "icons/folder.svg" },
                label: w.label.clone(),
                hint: None,
                chord: None,
                status: busy(w.agent_status),
                run: Run::Tab(tab),
            })
        })
        .collect()
}

pub fn items(snapshot: Option<&Snapshot>, bindings: &[(String, String)], plugin_actions: &[(String, String)]) -> Vec<Item> {
    let busy = |status: AgentStatus| (!matches!(status, AgentStatus::Idle | AgentStatus::Unknown)).then_some(status);
    let mut out: Vec<Item> = ACTIONS
        .iter()
        .filter(|(name, _)| *name != "CommandPanel")
        .map(|(name, label)| Item {
            key: name.to_string(),
            section: Section::Commands,
            icon: action_icon(name),
            label: label.to_string(),
            hint: None,
            chord: bindings.iter().find(|(action, _)| action == name).map(|(_, keys)| keys.clone()),
            status: None,
            run: Run::Action(name),
        })
        .collect();
    out.extend(plugin_actions.iter().map(|(id, title)| Item {
        key: format!("plugin:{id}"),
        section: Section::Commands,
        icon: "icons/widget.svg",
        label: title.clone(),
        hint: None,
        chord: None,
        status: None,
        run: Run::Herdr(crate::herdr::keys::HerdrCommand { kind: crate::herdr::keys::CommandKind::PluginAction, command: id.clone(), keys: Vec::new() }),
    }));
    let Some(s) = snapshot else { return out };
    out.extend(workspace_items(Some(s)));
    for (w, _) in s.workspace_tree() {
        for t in s.tabs_of(&w.workspace_id) {
            out.push(Item {
                key: format!("tab:{}", t.tab_id),
                section: Section::Tabs,
                icon: "icons/terminal.svg",
                label: short_tab_label(&t.label).to_string(),
                hint: Some(w.label.clone()),
                chord: None,
                status: busy(t.agent_status),
                run: Run::Tab(t.tab_id.clone()),
            });
        }
    }
    out
}

/// How many recent items the empty command panel shows on top.
const RECENT: usize = 5;

/// `items` with the ones in `recent` (newest first) copied on top under Recent; gone ones are skipped.
pub fn with_recent(mut items: Vec<Item>, recent: &[String]) -> Vec<Item> {
    let top: Vec<Item> = recent
        .iter()
        .filter_map(|key| items.iter().find(|i| &i.key == key))
        .take(RECENT)
        .map(|i| Item { section: Section::Recent, ..i.clone() })
        .collect();
    items.splice(0..0, top);
    items
}

/// `recent` with `key` moved to the front, at most [`RECENT`] long.
pub fn remember(recent: &mut Vec<String>, key: &str) {
    recent.retain(|k| k != key);
    recent.insert(0, key.to_string());
    recent.truncate(RECENT);
}

/// herdr's tab label without its number and path: "2 · main › claude › Fix tabs" reads "Fix tabs" (the
/// workspace shows beside it).
fn short_tab_label(label: &str) -> &str {
    let rest = label.split_once(" · ").filter(|(n, _)| n.chars().all(|c| c.is_ascii_digit())).map_or(label, |(_, rest)| rest);
    rest.rsplit(" › ").next().unwrap_or(rest)
}

/// The sidebar's agents, in its order, by what they work on: each runs to its tab, as a click on its row does.
/// A waiting one's hint leads with how long it has waited.
pub fn agent_items(snapshot: Option<&Snapshot>, sort: AgentSort, since: &Since) -> Vec<Item> {
    let Some(s) = snapshot else { return Vec::new() };
    s.agents_by(sort, since)
        .into_iter()
        .map(|p| {
            let agent = p.agent.clone().unwrap_or_default();
            Item {
                key: format!("agent:{}", p.pane_id),
                section: Section::Agents,
                icon: agent_icon(&agent),
                label: p.terminal_title_stripped.clone().filter(|t| !t.is_empty()).unwrap_or(agent),
                hint: s.workspaces.iter().find(|w| w.workspace_id == p.workspace_id).map(|w| match waiting_since(since, p) {
                    Some(at) => format!("{} · {}", ago(at.elapsed()), w.label),
                    None => w.label.clone(),
                }),
                chord: None,
                status: Some(p.agent_status),
                run: Run::Tab(p.tab_id.clone()),
            }
        })
        .collect()
}

/// `git for-each-ref` over local branches, newest commit first; `current` is marked and listed first.
pub fn branch_items(refs: &str, current: Option<&str>) -> Vec<Item> {
    let mut names: Vec<&str> = refs.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    names.sort_by_key(|n| Some(*n) != current); // stable: the rest keep git's order
    names
        .into_iter()
        .map(|name| Item {
            key: format!("branch:{name}"),
            section: Section::Branches,
            icon: "icons/git-branch.svg",
            label: name.to_string(),
            hint: (Some(name) == current).then(|| "current".into()),
            chord: None,
            status: None,
            run: Run::Branch(name.to_string()),
        })
        .collect()
}

/// The theme variants of one appearance as (id, name); `current` is marked and listed first.
pub fn theme_items(variants: Vec<(String, String)>, dark: bool, current: &str) -> Vec<Item> {
    let mut variants = variants;
    variants.sort_by_key(|(id, _)| id != current); // stable: the rest keep their order
    variants
        .into_iter()
        .map(|(id, name)| Item {
            key: format!("theme:{id}"),
            section: Section::Themes,
            icon: if dark { "icons/moon.svg" } else { "icons/sun.svg" },
            label: name,
            hint: (id == current).then(|| "current".into()),
            chord: None,
            status: None,
            run: Run::Theme(id),
        })
        .collect()
}

/// herdr's `worktree.list` answer as rows, in its order (the repo's checkout first), bare ones left out: the branch,
/// then the folder (`home` as `~`) after "open" when a workspace has it, or "prunable" when its folder is gone.
pub fn worktree_items(list: &serde_json::Value, repo: &str, home: &str) -> Vec<Item> {
    let entries = list["worktrees"].as_array().map(Vec::as_slice).unwrap_or_default();
    entries
        .iter()
        .filter(|w| !w["is_bare"].as_bool().unwrap_or(false))
        .filter_map(|w| {
            let path = w["path"].as_str()?.to_string();
            let workspace_id = w["open_workspace_id"].as_str().map(str::to_string);
            let shown = match path.strip_prefix(home).filter(|rest| !home.is_empty() && (rest.is_empty() || rest.starts_with('/'))) {
                Some(rest) => format!("~{rest}"),
                None => path.clone(),
            };
            let state = if workspace_id.is_some() { Some("open") } else { w["is_prunable"].as_bool().unwrap_or(false).then_some("prunable") };
            Some(Item {
                key: format!("worktree:{path}"),
                section: Section::Worktrees,
                icon: if w["is_linked_worktree"].as_bool().unwrap_or(false) { "icons/git-branch.svg" } else { "icons/folder.svg" },
                label: w["branch"].as_str().unwrap_or("detached").to_string(),
                hint: Some(match state {
                    Some(state) => format!("{state} · {shown}"),
                    None => shown,
                }),
                chord: None,
                status: None,
                run: Run::Worktree { repo: repo.to_string(), path, workspace_id },
            })
        })
        .collect()
}

/// How well `query` matches `text`, ignoring case, and which chars of `text` it matched; None if it does not:
/// `text` starting with it ranks first, then it starting a word, then anywhere, then its letters in order (more
/// of them on word starts, fewer gaps, higher).
pub fn matched(query: &str, text: &str) -> Option<(i32, Vec<usize>)> {
    let q: Vec<char> = query.trim().to_lowercase().chars().collect();
    if q.is_empty() {
        return Some((0, Vec::new()));
    }
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let word_start = |i: usize| i == 0 || !t[i - 1].is_alphanumeric();
    let hits: Vec<usize> = (0..t.len()).filter(|&i| t[i..].starts_with(&q)).collect();
    let run = |at: usize| (at..at + q.len()).collect();
    if hits.first() == Some(&0) {
        return Some((3000, run(0)));
    }
    if let Some(&at) = hits.iter().find(|&&i| word_start(i)) {
        return Some((2000, run(at)));
    }
    if let Some(&at) = hits.first() {
        return Some((1000, run(at)));
    }
    // The best placement of the letters: +10 for each on a word start, -1 for each char skipped before it.
    // best[k][i] scores q[..=k] with q[k] at t[i]; a running max over the previous row keeps it O(q·t).
    let q: Vec<char> = q.into_iter().filter(|c| !c.is_whitespace()).collect();
    if q.is_empty() {
        return None;
    }
    let bonus = |i: usize| if word_start(i) { 10 } else { 0 };
    let mut best: Vec<Vec<Option<(i32, usize)>>> = Vec::with_capacity(q.len());
    for (k, &c) in q.iter().enumerate() {
        let mut row = vec![None; t.len()];
        // the best (score + j, j) among the previous letter's places before i
        let mut prev: Option<(i32, usize)> = None;
        for i in 0..t.len() {
            if t[i] == c {
                row[i] = match k {
                    0 => Some((bonus(i) - i as i32, 0)),
                    _ => prev.map(|(s, j)| (s + bonus(i) - i as i32 + 1, j)),
                };
            }
            if k > 0
                && let Some((s, _)) = best[k - 1][i]
                && prev.is_none_or(|(p, _)| s + i as i32 > p)
            {
                prev = Some((s + i as i32, i));
            }
        }
        best.push(row);
    }
    let (mut i, score) = best[q.len() - 1].iter().enumerate().filter_map(|(i, c)| c.map(|(s, _)| (i, s))).max_by_key(|&(i, s)| (s, std::cmp::Reverse(i)))?;
    let mut at = vec![i];
    for k in (1..q.len()).rev() {
        i = best[k][i]?.1;
        at.push(i);
    }
    at.reverse();
    Some((score.min(999), at))
}

pub fn score(query: &str, text: &str) -> Option<i32> {
    matched(query, text).map(|(score, _)| score)
}

/// Indices of the `items` matching `query`, by section, best match first in each; a section with a better best
/// match comes first, and ties keep their order.
pub fn rank(query: &str, items: &[Item]) -> Vec<usize> {
    // Recent repeats items, so it shows only while nothing is typed
    let typing = !query.trim().is_empty();
    let mut hits: Vec<(usize, i32)> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| !(typing && item.section == Section::Recent))
        .filter_map(|(i, item)| Some((i, score(query, &item.haystack())?)))
        .collect();
    hits.sort_by_key(|&(_, s)| std::cmp::Reverse(s));
    let mut sections: Vec<Section> = Vec::new();
    for &(i, _) in &hits {
        if !sections.contains(&items[i].section) {
            sections.push(items[i].section);
        }
    }
    sections.into_iter().flat_map(|section| hits.iter().filter(move |&&(i, _)| items[i].section == section).map(|&(i, _)| i)).collect()
}

/// `text` with the chars the query matched in bold accent.
fn highlighted(text: &str, query: &str, theme: &Theme) -> StyledText {
    let mut chars: Vec<usize> = matched(query, text).map(|(_, at)| at).unwrap_or_default();
    // lowercasing that changes the char count (İ) would misplace the indices: leave such labels plain
    if text.to_lowercase().chars().count() != text.chars().count() {
        chars.clear();
    }
    let bytes: Vec<(usize, char)> = text.char_indices().collect();
    let style = HighlightStyle { color: Some(theme.accent), font_weight: Some(gpui::FontWeight::SEMIBOLD), ..Default::default() };
    let ranges: Vec<_> = chars.into_iter().filter_map(|i| bytes.get(i).map(|&(b, c)| (b..b + c.len_utf8(), style))).collect();
    StyledText::new(text.to_string()).with_highlights(ranges)
}

/// A keystroke the way the macOS menus write it: ⌃⌥⇧⌘ then the key. Elsewhere in words, as Settings shows it.
pub(crate) fn glyphs(keystroke: &str) -> String {
    if !cfg!(target_os = "macos") {
        return crate::ui::settings::display_chord(keystroke);
    }
    let Ok(k) = Keystroke::parse(keystroke) else { return keystroke.to_string() };
    let m = k.modifiers;
    let key = match k.key.as_str() {
        "left" => "←".into(),
        "right" => "→".into(),
        "up" => "↑".into(),
        "down" => "↓".into(),
        "enter" => "↵".into(),
        "escape" => "esc".into(),
        "backspace" => "⌫".into(),
        "delete" => "⌦".into(),
        "tab" => "⇥".into(),
        "space" => "Space".into(),
        key if key.chars().count() == 1 => key.to_uppercase(),
        key => key[..1].to_uppercase() + &key[1..],
    };
    [(m.control, "⌃"), (m.alt, "⌥"), (m.shift, "⇧"), (m.platform, "⌘"), (m.function, "fn")]
        .into_iter()
        .filter_map(|(on, glyph)| on.then_some(glyph))
        .chain([key.as_str()])
        .collect()
}

pub enum PanelEvent {
    Run(Run),
    Dismiss,
}

pub struct Panel {
    pub kind: Kind,
    input: Entity<InputState>,
    items: Vec<Item>,
    query: String,
    /// Indices into `items` of the rows shown, in order.
    matches: Vec<usize>,
    /// Index into `matches`.
    selected: usize,
    /// The panel's own chord, shown by the search field.
    chord: Option<String>,
    scroll: ScrollHandle,
    _events: Subscription,
}

impl EventEmitter<PanelEvent> for Panel {}

impl Panel {
    pub fn new(kind: Kind, items: Vec<Item>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items = match kind {
            Kind::Commands => with_recent(items, &crate::settings::SettingsGlobal::of(cx).settings.recent_commands),
            Kind::Agents | Kind::Workspaces | Kind::Branches | Kind::Themes | Kind::Worktrees => items,
        };
        let theme = Theme::of(cx).clone();
        let placeholder = match kind {
            Kind::Commands => "Search commands, workspaces and tabs",
            Kind::Agents => "Search agents",
            Kind::Workspaces => "Go to workspace",
            Kind::Branches => "Switch branch",
            Kind::Themes => "Change theme",
            Kind::Worktrees => "Open worktree",
        };
        let input = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder(placeholder);
            input.set_editor_style(crate::ui::input_style(&theme));
            input
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        let events = cx.subscribe_in(&input, window, |panel, _, event: &InputEvent, _, cx| match event {
            InputEvent::Change => panel.refilter(cx),
            InputEvent::PressEnter { .. } => panel.run(panel.selected, cx),
            _ => {}
        });
        let chord = cx.try_global::<ActiveBindings>().and_then(|b| b.0.iter().find(|(a, _)| a == kind.action()).map(|(_, k)| k.clone()));
        let matches = rank("", &items);
        Panel { kind, input, items, query: String::new(), matches, selected: 0, chord, scroll: ScrollHandle::new(), _events: events }
    }

    /// Fills a panel opened before its list arrived (the worktrees herdr answers with), keeping what is typed.
    pub fn set_items(&mut self, items: Vec<Item>, cx: &mut Context<Self>) {
        self.items = items;
        self.refilter(cx);
    }

    fn refilter(&mut self, cx: &mut Context<Self>) {
        self.query = self.input.read(cx).value().to_string();
        self.matches = rank(&self.query, &self.items);
        self.selected = 0;
        self.scroll.scroll_to_item(0);
        cx.notify();
    }

    /// Headings show in the command panel; the agents panel is one section.
    fn headings(&self) -> bool {
        self.kind == Kind::Commands
    }

    /// Whether `row` starts its section, so a heading goes before it.
    fn starts_section(&self, row: usize) -> bool {
        row == 0 || self.items[self.matches[row - 1]].section != self.items[self.matches[row]].section
    }

    fn step(&mut self, by: isize, cx: &mut Context<Self>) {
        let n = self.matches.len() as isize;
        if n == 0 {
            return;
        }
        self.selected = (self.selected as isize + by).rem_euclid(n) as usize;
        // the list's children are the rows with a heading before each section; a section's first row brings its
        // heading into view too
        let (child, heading) = match self.headings() {
            true => {
                let child = self.selected + (0..=self.selected).filter(|&r| self.starts_section(r)).count();
                (child, child - self.starts_section(self.selected) as usize)
            }
            false => (self.selected, self.selected),
        };
        // a little clear of the list's edges
        crate::ui::reveal(&self.scroll, heading..=child, 12.);
        cx.notify();
    }

    fn run(&mut self, row: usize, cx: &mut Context<Self>) {
        let Some(item) = self.matches.get(row).map(|&i| self.items[i].clone()) else { return };
        // a settings file that failed to load is not overwritten
        if self.kind == Kind::Commands && crate::settings::SettingsGlobal::of(cx).error.is_none() {
            crate::ui::settings::update(cx, |s| remember(&mut s.recent_commands, &item.key));
        }
        cx.emit(PanelEvent::Run(item.run));
    }
}

impl Render for Panel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        let card = theme.card;
        let mut children: Vec<gpui::AnyElement> = Vec::new();
        for (row, &i) in self.matches.iter().enumerate() {
            let item = &self.items[i];
            if self.headings() && self.starts_section(row) {
                children.push(
                    div()
                        .flex_none()
                        .px(px(8.))
                        .pt(px(if row == 0 { 2. } else { 10. }))
                        .pb(px(4.))
                        .text_size(px(11.))
                        .text_color(theme.text_faint)
                        .child(item.section.title())
                        .into_any_element(),
                );
            }
            let selected = row == self.selected;
            children.push(
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
                    .child(gpui::svg().path(item.icon).size(px(16.)).flex_none().text_color(icon_ink(
                        item.icon,
                        if selected { theme.text } else { theme.text_muted },
                    )))
                    .child(div().flex_1().min_w_0().truncate().child(highlighted(&item.label, &self.query, &theme)))
                    .children(item.hint.clone().map(|hint| div().flex_none().max_w(px(160.)).truncate().text_size(px(12.)).text_color(theme.text_faint).child(hint)))
                    .children(item.status.map(|status| crate::shell::status_dot(status, &theme)))
                    .children(item.chord.as_ref().map(|chord| div().flex().flex_none().gap(px(4.)).children(chord.split_whitespace().map(|k| kbd(&theme, glyphs(k))))))
                    .into_any_element(),
            );
        }
        let (offset, max) = (-f32::from(self.scroll.offset().y), f32::from(self.scroll.max_offset().y));
        // ponytail: gpui has no alpha mask, so the fades are gradients of the card's own color over the rows
        let fade = |angle: f32| {
            div().absolute().left_0().right_0().h(px(24.)).bg(linear_gradient(
                angle,
                linear_color_stop(card, 0.),
                linear_color_stop(card.opacity(0.), 1.),
            ))
        };
        let empty = self.matches.is_empty().then(|| {
            let (title, hint) = match self.kind {
                Kind::Commands => ("No results", "Try a command, a workspace or a tab."),
                Kind::Agents if self.items.is_empty() => ("No agents", "Agents herdr finds in your panes show up here."),
                Kind::Agents => ("No results", "Try an agent's task or its workspace."),
                Kind::Workspaces if self.items.is_empty() => ("No workspaces", "herdr's workspaces show up here."),
                Kind::Workspaces => ("No results", "Try a workspace's name."),
                Kind::Branches if self.items.is_empty() => ("No branches", "This pane isn't in a git repo."),
                Kind::Branches => ("No results", "Try a branch name."),
                Kind::Themes => ("No results", "Try a theme name."),
                // herdr always lists the repo's own checkout, so an empty list is one still on its way
                Kind::Worktrees if self.items.is_empty() => ("Asking herdr…", "Its worktrees of this repo show up here."),
                Kind::Worktrees => ("No results", "Try a branch or a folder."),
            };
            div().py(px(24.)).flex().flex_col().items_center().gap(px(6.)).child(title).child(div().text_color(theme.text_muted).child(hint))
        });
        div()
            .id("command-panel")
            .key_context(CONTEXT)
            .occlude()
            .w(px(560.))
            .flex()
            .flex_col()
            .rounded(px(CARD_RADIUS))
            .bg(card)
            .border_1()
            .border_color(theme.border)
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
                    .child(div().flex_1().min_w_0().text_size(px(14.)).child(Input::new(&self.input)))
                    .children(self.chord.as_deref().and_then(|c| c.split_whitespace().next()).map(|k| kbd(&theme, glyphs(k)))),
            )
            .child(
                div()
                    .relative()
                    .child(
                        div()
                            .id("command-items")
                            .max_h(px(360.))
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll)
                            // re-render so the edge fades follow the scroll position
                            .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.notify()))
                            .p(px(8.))
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .children(children)
                            .children(empty),
                    )
                    .when(offset > 0.5, |el| el.child(fade(180.).top_0()))
                    .when(offset < max - 0.5, |el| el.child(fade(0.).bottom_0())),
            )
            .child(
                div()
                    .px(px(16.))
                    .py(px(7.))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .border_t_1()
                    .border_color(theme.border)
                    .child(key_hint(&theme, &["↑", "↓"], "Navigate"))
                    .child(key_hint(&theme, &["↵"], "Open"))
                    .child(key_hint(&theme, &["esc"], "Close")),
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
        .bg(theme.wash(0.06))
        .text_size(px(11.))
        .text_color(theme.text_muted)
        .child(SharedString::from(keys))
}

/// A footer legend: key caps and the verb after them.
fn key_hint(theme: &Theme, keys: &[&str], label: &'static str) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(px(4.))
        .children(keys.iter().map(|k| kbd(theme, k.to_string())))
        .child(div().ml(px(2.)).text_size(px(11.)).text_color(theme.text_muted).child(label))
}

const CARD_RADIUS: f32 = 16.;

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

    fn item(section: Section, label: &str) -> Item {
        let run = if section == Section::Commands { Run::Action("NewTab") } else { Run::Tab("t".into()) };
        Item { key: label.into(), section, icon: "", label: label.into(), hint: None, chord: None, status: None, run }
    }

    #[test]
    fn recent_items_lead_the_empty_list_and_hide_while_typing() {
        let items = with_recent(commands(&["New tab", "Split right", "Settings"]), &["Settings".into(), "gone".into(), "New tab".into()]);
        assert_eq!(ranked("", &items), ["Settings", "New tab", "New tab", "Split right", "Settings"]);
        assert_eq!(items[0].section, Section::Recent);
        assert_eq!(ranked("set", &items), ["Settings"], "no duplicate from Recent");
    }

    #[test]
    fn remember_moves_to_front_and_caps() {
        let mut recent: Vec<String> = ["a", "b", "c", "d", "e"].map(String::from).to_vec();
        remember(&mut recent, "c");
        assert_eq!(recent, ["c", "a", "b", "d", "e"]);
        remember(&mut recent, "f");
        assert_eq!(recent, ["f", "c", "a", "b", "d"]);
    }

    fn ranked(query: &str, items: &[Item]) -> Vec<String> {
        rank(query, items).into_iter().map(|i| items[i].label.clone()).collect()
    }

    fn commands(labels: &[&str]) -> Vec<Item> {
        labels.iter().map(|l| item(Section::Commands, l)).collect()
    }

    #[test]
    fn prefix_then_word_start_then_substring_then_letters() {
        let items = commands(&["Close tab", "Split right", "Focus pane right", "Righteous", "Bright", "Go right"]);
        assert_eq!(ranked("right", &items), ["Righteous", "Split right", "Focus pane right", "Go right", "Bright"]);
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
    fn letters_land_on_word_starts_when_they_can() {
        assert_eq!(matched("nt", "Next tab").map(|m| m.1), Some(vec![0, 5]), "the t of tab, not of Next");
    }

    #[test]
    fn tab_labels_lose_their_number_and_path() {
        assert_eq!(short_tab_label("1 · main › claude › IP и root-доступ"), "IP и root-доступ");
        assert_eq!(short_tab_label("2 · mono"), "mono");
        assert_eq!(short_tab_label("v1 · build"), "v1 · build");
        assert_eq!(short_tab_label("zsh"), "zsh");
    }

    #[test]
    fn matched_chars_are_the_ones_highlighted() {
        assert_eq!(matched("spl", "Split right").map(|m| m.1), Some(vec![0, 1, 2]));
        assert_eq!(matched("right", "Split right").map(|m| m.1), Some(vec![6, 7, 8, 9, 10]));
        assert_eq!(matched("sr", "Split right").map(|m| m.1), Some(vec![0, 6]));
    }

    #[test]
    fn an_empty_query_keeps_every_item_in_order() {
        assert_eq!(ranked("  ", &commands(&["b", "a"])), ["b", "a"]);
    }

    #[test]
    fn sections_stay_together_and_the_best_section_leads() {
        let items = vec![
            item(Section::Commands, "New tab"),
            item(Section::Commands, "Flush"),
            item(Section::Workspaces, "shepherd"),
            item(Section::Tabs, "shell"),
            item(Section::Tabs, "zsh"),
        ];
        assert_eq!(ranked("", &items), ["New tab", "Flush", "shepherd", "shell", "zsh"]);
        // "sh" starts a workspace and a tab, and only sits inside "zsh" and "Flush"
        assert_eq!(ranked("sh", &items), ["shepherd", "shell", "zsh", "Flush"]);
    }

    #[test]
    fn tabs_match_on_their_workspace_and_section_but_actions_only_on_their_label() {
        let tab = Item { hint: Some("shepherd".into()), ..item(Section::Tabs, "claude") };
        assert!(score("shep", &tab.haystack()).is_some());
        assert!(score("tabs", &tab.haystack()).is_some());
        let action = Item { chord: Some("cmd-t".into()), ..item(Section::Commands, "New tab") };
        assert_eq!(score("cmd", &action.haystack()), None, "a chord is not searched");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn chords_read_as_macos_writes_them() {
        assert_eq!(glyphs("cmd-shift-d"), "⇧⌘D");
        assert_eq!(glyphs("alt-cmd-left"), "⌥⌘←");
        assert_eq!(glyphs("cmd-,"), "⌘,");
        assert_eq!(glyphs("shift-cmd-enter"), "⇧⌘↵");
    }

    #[test]
    fn items_list_actions_with_chords_then_workspaces_and_tabs() {
        let snapshot: Snapshot = serde_json::from_value(serde_json::json!({
            "focused_workspace_id": null, "focused_tab_id": null, "focused_pane_id": null,
            "workspaces": [
                {"workspace_id": "w1", "label": "shepherd", "number": 1, "active_tab_id": "t2", "agent_status": "working"},
                {"workspace_id": "w2", "label": "empty", "number": 2, "active_tab_id": null, "agent_status": "idle"}
            ],
            "tabs": [
                {"tab_id": "t1", "workspace_id": "w1", "label": "zsh", "number": 1, "agent_status": "idle"},
                {"tab_id": "t2", "workspace_id": "w1", "label": "claude", "number": 2, "agent_status": "working"}
            ],
            "panes": [], "layouts": []
        }))
        .unwrap();
        let bindings = vec![("NewTab".to_string(), "cmd-t".to_string()), ("CommandPanel".to_string(), "cmd-k".to_string())];
        let items = items(Some(&snapshot), &bindings, &[]);
        let new_tab = items.iter().find(|i| i.label == "New tab").unwrap();
        assert_eq!((new_tab.chord.as_deref(), &new_tab.run), (Some("cmd-t"), &Run::Action("NewTab")));
        assert!(items.iter().all(|i| i.run != Run::Action("CommandPanel")), "the panel does not list itself");
        let rest: Vec<_> = items.iter().filter(|i| i.section != Section::Commands).map(|i| (i.label.as_str(), i.run.clone(), i.status)).collect();
        assert_eq!(
            rest,
            [
                ("shepherd", Run::Tab("t2".into()), Some(AgentStatus::Working)),
                ("zsh", Run::Tab("t1".into()), None),
                ("claude", Run::Tab("t2".into()), Some(AgentStatus::Working)),
            ],
            "a workspace without tabs has nothing to show; idle ones show no dot"
        );
    }

    #[test]
    fn branches_list_the_current_one_first() {
        let items = branch_items("feature\nmain\n\nfix\n", Some("main"));
        let rows: Vec<_> = items.iter().map(|i| (i.label.as_str(), i.hint.as_deref())).collect();
        assert_eq!(rows, [("main", Some("current")), ("feature", None), ("fix", None)]);
        assert_eq!(items[1].run, Run::Branch("feature".into()));
    }

    #[test]
    fn themes_list_the_current_one_first() {
        let variants = [("a", "A"), ("b", "B"), ("c", "C")].map(|(i, n)| (i.to_string(), n.to_string())).to_vec();
        let items = theme_items(variants, true, "b");
        let rows: Vec<_> = items.iter().map(|i| (i.label.as_str(), i.hint.as_deref())).collect();
        assert_eq!(rows, [("B", Some("current")), ("A", None), ("C", None)]);
        assert_eq!(items[1].run, Run::Theme("a".into()));
    }

    #[test]
    fn worktrees_list_herdr_s_order_without_bare_ones() {
        let list = serde_json::json!({"type": "worktree_list", "worktrees": [
            {"path": "/u/repo", "branch": "main", "is_bare": false, "is_detached": false, "is_prunable": false,
             "is_linked_worktree": false, "open_workspace_id": "w1", "label": "repo"},
            {"path": "/u/.herdr/worktrees/repo/fix", "branch": "fix", "is_bare": false, "is_detached": false,
             "is_prunable": true, "is_linked_worktree": true, "label": "repo"},
            {"path": "/u/bare.git", "is_bare": true, "is_detached": false, "is_prunable": false, "is_linked_worktree": true, "label": "x"},
            {"path": "/tmp/probe", "is_bare": false, "is_detached": true, "is_prunable": false, "is_linked_worktree": true, "label": "repo"},
        ]});
        let items = worktree_items(&list, "w1", "/u");
        let rows: Vec<_> = items.iter().map(|i| (i.label.as_str(), i.hint.as_deref(), i.icon)).collect();
        assert_eq!(
            rows,
            [
                ("main", Some("open · ~/repo"), "icons/folder.svg"),
                ("fix", Some("prunable · ~/.herdr/worktrees/repo/fix"), "icons/git-branch.svg"),
                ("detached", Some("/tmp/probe"), "icons/git-branch.svg"),
            ]
        );
        assert_eq!(items[0].run, Run::Worktree { repo: "w1".into(), path: "/u/repo".into(), workspace_id: Some("w1".into()) });
        assert_eq!(items[1].run, Run::Worktree { repo: "w1".into(), path: "/u/.herdr/worktrees/repo/fix".into(), workspace_id: None });
    }
}
