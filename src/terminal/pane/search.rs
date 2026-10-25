//! Cmd+F in a pane: herdr's history searched through `pane.read`, its matches marked on the grid.
use super::*;

/// What herdr's `pane.read` gives at most (it caps `lines`, with no paging), so what search sees of a history.
pub(super) const READ_LINES: u32 = 1000;

pub(super) struct Search {
    input: Entity<InputState>,
    /// Matches in herdr's history: line from the oldest one, char range.
    hits: Vec<(usize, Range<usize>)>,
    /// The history is longer than herdr reads (`READ_LINES`): only its newest lines were searched.
    partial: bool,
    current: Option<usize>,
    _events: Subscription,
    _read: Option<Task<()>>,
}

/// A row's chars and, per char, its cell (wide chars' spacers skipped).
pub(super) fn row_chars(cells: &[CellSnapshot]) -> (Vec<char>, Vec<usize>) {
    cells.iter().enumerate().filter(|(_, c)| !c.wide_spacer).map(|(i, c)| (c.ch, i)).unzip()
}

/// A char range of `row_chars` as cells.
pub(super) fn cell_range(cells: &[CellSnapshot], cols: &[usize], chars: Range<usize>) -> Range<usize> {
    let last = cols[chars.end - 1];
    cols[chars.start]..last + 1 + usize::from(cells[last].wide)
}

impl PaneTerminal {
    pub(super) fn search_marks(&self, lines: &[Vec<CellSnapshot>], cx: &App) -> Vec<(usize, Range<usize>, bool)> {
        let Some(search) = &self.search else { return Vec::new() };
        let query: Vec<char> = search.input.read(cx).value().chars().collect();
        // The current hit's row on screen: herdr's lines count from the oldest, the screen's top is `max - offset`.
        let current = search.current.and_then(|i| search.hits.get(i)).and_then(|(line, chars)| {
            let row = *line as i64 - (self.scroll.max_offset_from_bottom as i64 - self.scroll.offset_from_bottom as i64);
            usize::try_from(row).ok().map(|row| (row, chars.clone()))
        });
        let mut marks = Vec::new();
        for (row, cells) in lines.iter().enumerate() {
            let (chars, cols) = row_chars(cells);
            for hit in find_all(&chars, &query) {
                let now = current.as_ref().is_some_and(|(r, c)| *r == row && *c == hit);
                marks.push((row, cell_range(cells, &cols, hit), now));
            }
        }
        marks
    }

    pub(super) fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.search.is_none() {
            let theme = Theme::of(cx).clone();
            let input = cx.new(|cx| {
                let mut input = InputState::new(window, cx).placeholder("Find");
                input.set_editor_style(crate::ui::input_style(&theme));
                input
            });
            let events = cx.subscribe_in(&input, window, |pane, _, event: &InputEvent, _, cx| match event {
                InputEvent::Change => pane.search_history(cx),
                InputEvent::PressEnter { shift, .. } => pane.step(!shift, cx),
                _ => {}
            });
            self.search = Some(Search { input, hits: Vec::new(), partial: false, current: None, _events: events, _read: None });
        }
        if let Some(search) = &self.search {
            search.input.update(cx, |input, cx| {
                input.focus(window, cx);
                input.select_all(window, cx);
            });
        }
        cx.notify();
    }

    pub(super) fn close_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Reads herdr's history and finds the query in it; the current match becomes the newest one not below the
    /// screen.
    pub(super) fn search_history(&mut self, cx: &mut Context<Self>) {
        let Some(search) = &mut self.search else { return };
        let query: Vec<char> = search.input.read(cx).value().chars().collect();
        search.hits.clear();
        search.current = None;
        search._read = None;
        cx.notify();
        let Some(PaneQueries(client, rt)) = cx.try_global::<PaneQueries>().filter(|_| !query.is_empty()) else { return };
        let (client, pane_id) = (client.clone(), self.pane_id.clone());
        let read = rt.spawn(async move {
            let read = client.call("pane.read", serde_json::json!({"pane_id": pane_id, "source": "recent", "lines": READ_LINES})).await;
            let text = read.ok().and_then(|r| r["read"]["text"].as_str().map(str::to_owned)).unwrap_or_default();
            let lines = text.lines().enumerate();
            let hits = lines.flat_map(|(i, l)| find_all(&l.chars().collect::<Vec<_>>(), &query).into_iter().map(move |r| (i, r)));
            (text.lines().count(), hits.collect::<Vec<_>>())
        });
        search._read = Some(cx.spawn(async move |this, cx| {
            let Ok((read, hits)) = read.await else { return };
            let _ = this.update(cx, |pane, cx| {
                let s = pane.scroll;
                let bottom = (s.max_offset_from_bottom - s.offset_from_bottom.min(s.max_offset_from_bottom)) as usize + pane.emulator.rows();
                // herdr reads the newest lines: the first one read is this far from the oldest
                let skipped = (s.max_offset_from_bottom as usize + pane.emulator.rows()).saturating_sub(read);
                let hits: Vec<_> = hits.into_iter().map(|(line, chars)| (line + skipped, chars)).collect();
                let Some(search) = &mut pane.search else { return };
                search.partial = skipped > 0;
                search.current = hits.iter().rposition(|(line, _)| *line < bottom).or(hits.len().checked_sub(1));
                search.hits = hits;
                pane.reveal(cx);
            });
        }));
    }

    /// The next match up into history (`older`) or back down, wrapping around.
    pub(super) fn step(&mut self, older: bool, cx: &mut Context<Self>) {
        let Some(search) = &mut self.search else { return };
        let n = search.hits.len();
        if n == 0 {
            return;
        }
        search.current = Some(match search.current {
            None => n - 1,
            Some(i) if older => (i + n - 1) % n,
            Some(i) => (i + 1) % n,
        });
        self.reveal(cx);
    }

    /// Scrolls herdr to the current match unless it is on screen.
    pub(super) fn reveal(&mut self, cx: &mut Context<Self>) {
        let line = self.search.as_ref().and_then(|s| s.current.and_then(|i| s.hits.get(i))).map(|(line, _)| *line);
        match line.and_then(|line| reveal_offset(self.scroll, line, self.emulator.rows())) {
            Some(offset) => self.scroll_by(offset as i64 - self.scroll.offset_from_bottom as i64, None, cx),
            None => cx.notify(),
        }
    }

    pub(super) fn search_bar(&self, search: &Search, theme: &Theme, cx: &mut Context<Self>) -> gpui::AnyElement {
        let count = match (search.current, search.hits.len()) {
            (Some(i), n) => format!("{}/{n}", i + 1),
            _ if search.input.read(cx).value().is_empty() => String::new(),
            _ => "No results".into(),
        };
        let count = if search.partial && !count.is_empty() { format!("{count} · last {READ_LINES}") } else { count };
        div()
            .id("search")
            .absolute()
            .top(px(8.))
            .right(px(20.))
            .w(px(300.))
            .flex()
            .items_center()
            .gap_2()
            .pl_2()
            .pr_3()
            .py_1()
            .rounded_md()
            .bg(theme.card)
            .border_1()
            .border_color(theme.border)
            .text_sm()
            .text_color(theme.text)
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .capture_key_down(cx.listener(|pane, e: &KeyDownEvent, window, cx| {
                let k = &e.keystroke;
                match k.key.as_str() {
                    "escape" => pane.close_search(window, cx),
                    "g" if app_shortcut(&k.modifiers) => pane.step(!search_back(&k.modifiers), cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .child(div().flex_1().child(Input::new(&search.input)))
            .child(div().flex_none().text_color(theme.text_muted).child(count))
            .into_any_element()
    }
}
