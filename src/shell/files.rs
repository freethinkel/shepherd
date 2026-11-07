//! The right panel's files, Zeron's `crates/ui/src/files/` (MIT): the explorer (`mod.rs`' explorer presentation,
//! `tree.rs`, `search.rs`, `context_menu.rs`) and a file's own tab (its editor presentation, read-only here: the text
//! rows of `preview.rs`, `image_preview.rs`, and `markdown_preview.rs` through Shepherd's markdown). The tree and its
//! search are `crate::files`. Zeron's watch (`watch.rs`) is a poll here, as the diffs': `git status` while the explorer
//! is shown, a file's blob id while its tab is.
use super::review::ShownReview;
use super::*;
use crate::files::*;
use crate::right_panel::Surface;
use crate::ui::surface_chrome::toolbar;
use gpui::{AnyElement, ListAlignment, ListSizingBehavior, ListState, Task, list};
use std::sync::Arc;

const TREE_ROW_HEIGHT: f32 = 27.0;
const TREE_INDENT: f32 = 14.0;
const PREVIEW_LINE_HEIGHT: f32 = 20.0;
const PREVIEW_TEXT_SIZE: f32 = 12.0;
/// Between looks at the checkout, while a tab is shown in a window in use.
const POLL: Duration = Duration::from_secs(2);
/// A pause after a key in the search, so a word typed is one search.
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(80);

pub(super) enum FilesEvent {
    /// A file picked in the tree or the search: its own tab.
    Open { place: Place, path: String },
    /// Open in Editor: the editor the diffs open.
    Edit { place: Place, path: String },
    /// An image clicked in its tab: the panes' image dialog, to zoom.
    Zoom { place: Place, path: String },
}

impl gpui::EventEmitter<FilesEvent> for Files {}
impl gpui::EventEmitter<FilesEvent> for FilePreview {}

/// Draw each ancestor's guide in the row itself so virtualized rows join seamlessly (Zeron's `with_indent_guides`).
fn with_indent_guides(row: AnyElement, depth: usize, theme: &Theme) -> AnyElement {
    div()
        .relative()
        .h(px(TREE_ROW_HEIGHT))
        .w_full()
        .flex_none()
        // the centre of the ancestor's 14 px disclosure slot
        .children((0..depth).map(|level| div().absolute().top_0().bottom_0().left(px(8.0 + 7.0 + level as f32 * TREE_INDENT)).w(px(1.0)).bg(theme.border)))
        .child(row)
        .into_any_element()
}

/// Keep the viewport attached to a path rather than an index when rows move (Zeron's `sync_list_rows`).
fn sync_list_rows(list: &ListState, old: &[VisibleTreeRow], new: &[VisibleTreeRow]) {
    if old == new {
        return;
    }
    let mut anchor = list.logical_scroll_top();
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let suffix = old[prefix..].iter().rev().zip(new[prefix..].iter().rev()).take_while(|(a, b)| a == b).count();
    let positions = new.iter().enumerate().map(|(index, row)| (row.path.as_str(), index)).collect::<HashMap<_, _>>();
    let old_index = anchor.item_ix.min(old.len().saturating_sub(1));
    let mut path = old.get(old_index).map(|row| row.path.clone());
    let mut target = None;
    while let Some(candidate) = path {
        if let Some(index) = positions.get(candidate.as_str()) {
            target = Some(*index);
            break;
        }
        path = parent_path(&candidate);
    }
    let target = target.or_else(|| old.iter().skip(old_index).chain(old[..old_index].iter().rev()).find_map(|row| positions.get(row.path.as_str()).copied()));
    list.splice(prefix..old.len() - suffix, new.len() - prefix - suffix);
    anchor.item_ix = target.unwrap_or(0);
    if target.is_none() {
        anchor.offset_in_item = px(0.0);
    }
    list.scroll_to(anchor);
}

fn entry_icon(name: &str, kind: EntryKind, expanded: bool, theme: &Theme) -> AnyElement {
    match kind {
        EntryKind::Directory => gpui::svg().path(if expanded { "icons/solar/folder-open.svg" } else { "icons/folder.svg" }).size(px(14.0)).flex_none().text_color(theme.text_muted).into_any_element(),
        _ => crate::ui::file_icons::icon(name, theme.appearance).size(px(14.0)).into_any_element(),
    }
}

fn chevron(open: bool, shown: bool, theme: &Theme) -> Div {
    div()
        .size(px(14.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .when(shown, |el| el.child(gpui::svg().path(if open { "icons/alt-arrow-down.svg" } else { "icons/alt-arrow-right.svg" }).size(px(11.0)).text_color(theme.text_faint)))
}

fn centered(message: impl Into<SharedString>, color: Hsla) -> AnyElement {
    div().flex_1().flex().items_center().justify_center().px(px(24.0)).text_center().text_size(px(12.0)).text_color(color).child(message.into()).into_any_element()
}

fn status_row(index: usize, padding: f32, label: &'static str, color: Hsla) -> AnyElement {
    div().id(("files-tree-status", index)).h(px(TREE_ROW_HEIGHT)).w_full().flex_none().pl(px(padding)).pr(px(8.0)).flex().items_center().text_size(px(10.5)).text_color(color).child(label).into_any_element()
}

/// The explorer: the shown pane's checkout as a tree, git's marks on it, and a search by name over it.
pub(super) struct Files {
    /// The shown pane's directory, located.
    dir: Option<String>,
    place: Option<Place>,
    place_task: Option<Task<()>>,
    /// The first `git status` of a checkout.
    status_task: Option<Task<()>>,
    tree: FileTreeModel,
    tree_list: ListState,
    tree_rows: Vec<VisibleTreeRow>,
    tree_generation: u64,
    loads: HashMap<String, Task<()>>,
    error: Option<SharedString>,
    /// A path asked to show in the tree, waiting on its folders' listings.
    reveal: Option<String>,
    marks: Decorations,
    /// The last `git status` read, and whether one was: a change after the first re-lists the folders open.
    status: Option<Option<Vec<u8>>>,
    /// Runs while the tab is shown.
    watch_task: Option<Task<()>>,
    search: Entity<InputState>,
    query: String,
    /// What the search looks through, read once per checkout and visibility, again after a change.
    candidates: Option<Arc<Vec<(String, EntryKind)>>>,
    results: Vec<SearchMatch>,
    search_tree: SearchTreeModel,
    search_active: usize,
    search_list: ListState,
    searching: bool,
    search_error: Option<SharedString>,
    search_task: Option<Task<()>>,
    /// The tree's right-click menu: the paths it acts on, and where.
    menu: Option<crate::ui::Popup<(Vec<String>, gpui::Point<Pixels>)>>,
    /// The keyboard is in the explorer: the cursor shows. The shell says so each render.
    pub(super) keys: bool,
    /// Where the keyboard goes back to from the search: the shell's own, the panel's while it has them.
    keys_back: Option<gpui::FocusHandle>,
    _search_events: Subscription,
}

impl Files {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let theme = Theme::of(cx).clone();
        let search = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("Search files");
            input.set_editor_style(crate::ui::input_style(&theme));
            input
        });
        let events = cx.subscribe(&search, |this: &mut Self, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.on_search_edited(cx);
            }
        });
        Self {
            dir: None,
            place: None,
            place_task: None,
            status_task: None,
            tree: FileTreeModel::default(),
            tree_list: ListState::new(0, ListAlignment::Top, px(560.0)),
            tree_rows: Vec::new(),
            tree_generation: 0,
            loads: HashMap::new(),
            error: None,
            reveal: None,
            marks: Decorations::default(),
            status: None,
            watch_task: None,
            search,
            query: String::new(),
            candidates: None,
            results: Vec::new(),
            search_tree: SearchTreeModel::default(),
            search_active: 0,
            search_list: ListState::new(0, ListAlignment::Top, px(420.0)),
            searching: false,
            search_error: None,
            search_task: None,
            menu: None,
            keys: false,
            keys_back: None,
            _search_events: events,
        }
    }

    /// Follows the shown pane's checkout, and watches it while `watch`.
    fn show(&mut self, dir: Option<&str>, watch: bool, cx: &mut Context<Self>) {
        if let Some(dir) = dir.filter(|d| self.dir.as_deref() != Some(*d)) {
            self.dir = Some(dir.to_string());
            let dir = dir.to_string();
            self.place_task = Some(cx.spawn(async move |this, cx| {
                let place = cx.background_spawn(async move { Place::of(&dir) }).await;
                let _ = this.update(cx, |files, cx| files.set_place(place, cx));
            }));
        }
        if watch != self.watch_task.is_some() {
            self.watch_task = watch.then(|| self.watch(cx));
        }
    }

    /// The checkout found: another one starts the tree over, the same one (another folder in it) keeps it.
    fn set_place(&mut self, place: Place, cx: &mut Context<Self>) {
        let same = self.place.as_ref().is_some_and(|p| p.root == place.root && p.machine() == place.machine());
        self.place = Some(place);
        if same {
            return;
        }
        self.tree.reset();
        self.loads.clear();
        self.reveal = None;
        self.marks = Decorations::default();
        self.status = None;
        self.candidates = None;
        self.error = None;
        self.sync_tree_list();
        self.load_directory(String::new(), cx);
        if !self.query.is_empty() {
            self.run_search(cx);
        }
        // git's marks at once, not at the watch's first look: it runs only while the window is in use
        if let Some(place) = self.place.clone() {
            self.status_task = Some(cx.spawn(async move |this, cx| {
                let asked = place.clone();
                let status = cx.background_spawn(async move { asked.status() }).await;
                let _ = this.update(cx, |files, cx| files.watched(&place, status, cx));
            }));
        }
        cx.notify();
    }

    fn watch(&mut self, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            loop {
                let Ok(place) = this.read_with(cx, |files, _| files.place.clone()) else { return };
                if let Some(place) = place {
                    let asked = place.clone();
                    let status = cx.background_spawn(async move { asked.status() }).await;
                    if this.update(cx, |files, cx| files.watched(&place, status, cx)).is_err() {
                        return;
                    }
                }
                cx.background_executor().timer(POLL).await;
            }
        })
    }

    // ponytail: a change git doesn't see (an ignored file, a file edited again) shows on the next change it does, or
    // the refresh button; a filesystem watch (Zeron's) here if that's felt.
    fn watched(&mut self, place: &Place, status: Option<Vec<u8>>, cx: &mut Context<Self>) {
        if self.place.as_ref() != Some(place) || self.status.as_ref() == Some(&status) {
            return;
        }
        let changed = self.status.is_some();
        self.marks = status.as_deref().map(Decorations::from_porcelain).unwrap_or_default();
        self.status = Some(status);
        if changed {
            self.refresh(cx);
        }
        cx.notify();
    }

    /// Lists again every folder open; the search reads its files again.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.candidates = None;
        self.tree.invalidate_all_directories();
        for directory in std::iter::once(String::new()).chain(self.tree.expanded_directories()) {
            self.load_directory(directory, cx);
        }
        if !self.query.is_empty() {
            self.run_search(cx);
        }
    }

    fn load_directory(&mut self, directory: String, cx: &mut Context<Self>) {
        let Some(place) = self.place.clone() else { return };
        let generation = self.tree.generation();
        if !self.tree.begin_load(&directory, generation) {
            return;
        }
        self.sync_tree_list();
        let (include, asked, key) = (self.tree.include_ignored(), directory.clone(), directory.clone());
        let task = cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { place.list(&asked, include) }).await;
            let _ = this.update(cx, |files, cx| {
                if files.tree.generation() != generation {
                    return;
                }
                let reload = files.tree.node(&directory).is_some_and(|node| node.stale);
                match result {
                    Ok(page) => {
                        if directory.is_empty() {
                            files.error = None;
                        }
                        files.tree.apply_page(page, generation);
                    }
                    Err(error) => {
                        if directory.is_empty() {
                            files.error = Some(error.clone().into());
                        }
                        files.tree.fail_load(&directory, error, generation);
                    }
                }
                files.sync_tree_list();
                if reload {
                    files.load_directory(directory, cx);
                }
                files.continue_reveal(cx);
                cx.notify();
            });
        });
        self.loads.insert(key, task);
    }

    fn sync_tree_list(&mut self) {
        if self.tree_generation != self.tree.generation() {
            self.tree_list.reset_with_uniform_height(self.tree.visible_rows().len(), px(TREE_ROW_HEIGHT));
            self.tree_generation = self.tree.generation();
        } else {
            sync_list_rows(&self.tree_list, &self.tree_rows, self.tree.visible_rows());
        }
        self.tree_rows = self.tree.visible_rows().to_vec();
    }

    fn tree_has_content(&self) -> bool {
        self.tree.node("").is_some_and(|node| node.has_loaded)
    }

    fn toggle_ignored(&mut self, cx: &mut Context<Self>) {
        self.tree.set_include_ignored(!self.tree.include_ignored());
        self.loads.clear();
        self.candidates = None;
        self.sync_tree_list();
        self.load_directory(String::new(), cx);
        if !self.query.is_empty() {
            self.run_search(cx);
        }
        cx.notify();
    }

    fn activate_tree_path(&mut self, path: String, cx: &mut Context<Self>) {
        self.tree.select(path.clone());
        match self.tree.node(&path).map(|node| node.entry.kind) {
            Some(EntryKind::Directory) => self.toggle_tree_directory(path, cx),
            Some(_) => self.open(path, cx),
            None => {}
        }
        self.reveal_tree_selection();
        cx.notify();
    }

    fn open(&self, path: String, cx: &mut Context<Self>) {
        if let Some(place) = self.place.clone() {
            cx.emit(FilesEvent::Open { place, path });
        }
    }

    fn toggle_tree_directory(&mut self, path: String, cx: &mut Context<Self>) {
        if !self.tree.toggle_expanded(&path) {
            return;
        }
        self.sync_tree_list();
        let needs_load = self.tree.node(&path).is_some_and(|node| node.stale || matches!(node.load, DirectoryLoadState::Unloaded | DirectoryLoadState::Error { .. }));
        if self.tree.is_expanded(&path) && needs_load {
            self.load_directory(path, cx);
        }
    }

    fn reveal_tree_selection(&self) {
        if let Some(index) = self.tree.selected().and_then(|selected| self.tree.visible_rows().iter().position(|row| row.path == selected)) {
            self.tree_list.scroll_to_reveal_item(index);
        }
    }

    /// Opens the folders down to the path asked for, each as its parent's listing lands, then selects it.
    fn continue_reveal(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.reveal.clone() else { return };
        let ancestors = self.tree.reveal_search_match(&SearchMatch { path: path.clone(), name: String::new(), kind: EntryKind::File, score: 0 });
        for ancestor in ancestors {
            if self.tree.node(&ancestor).is_none() {
                // its parent is still loading; a listing without it ends the walk
                if self.loads.is_empty() || parent_path(&ancestor).is_some_and(|p| self.tree.is_directory_loaded(&p)) {
                    self.reveal = None;
                }
                return;
            }
            if self.tree.expand(&ancestor) {
                self.sync_tree_list();
            }
            if !self.tree.is_directory_loaded(&ancestor) {
                self.load_directory(ancestor, cx);
                return;
            }
        }
        self.reveal = None;
        if self.tree.select(path) {
            self.sync_tree_list();
            self.reveal_tree_selection();
        }
    }

    fn on_search_edited(&mut self, cx: &mut Context<Self>) {
        let query = self.search.read(cx).value().trim().to_string();
        if self.query == query {
            return;
        }
        self.query = query;
        self.search_active = 0;
        if self.query.is_empty() {
            self.search_task = None;
            self.searching = false;
            self.search_error = None;
            self.results.clear();
            self.search_tree.clear();
            self.search_list.reset(0);
        } else {
            self.run_search(cx);
        }
        cx.notify();
    }

    fn run_search(&mut self, cx: &mut Context<Self>) {
        let Some(place) = self.place.clone() else { return };
        let (query, cached, include) = (self.query.clone(), self.candidates.clone(), self.tree.include_ignored());
        self.searching = true;
        self.search_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SEARCH_DEBOUNCE).await;
            let asked = query.clone();
            let (candidates, found) = cx
                .background_spawn(async move {
                    let candidates = match cached {
                        Some(candidates) => Ok(candidates),
                        None => place.files(include).map(Arc::new),
                    };
                    let found = candidates.as_ref().map(|c| search(c, &asked, SEARCH_RESULT_LIMIT)).map_err(Clone::clone);
                    (candidates.ok(), found)
                })
                .await;
            let _ = this.update(cx, |files, cx| {
                if files.query != query {
                    return;
                }
                files.searching = false;
                files.candidates = candidates.or(files.candidates.take());
                match found {
                    Ok(found) => {
                        files.search_error = None;
                        files.results = found;
                    }
                    Err(error) => files.search_error = Some(error.into()),
                }
                files.search_tree.rebuild(&files.results);
                files.search_active = files.search_active.min(files.search_tree.rows().len().saturating_sub(1));
                files.search_list.reset(files.search_tree.rows().len());
                cx.notify();
            });
        }));
    }

    fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.update(cx, |input, cx| input.set_value("", window, cx));
        self.on_search_edited(cx);
    }

    /// Enter or a click on a result: a file opens, and either shows in the tree with the search cleared.
    fn activate_search_result(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.search_tree.rows().get(self.search_active).cloned() else { return };
        if row.kind != EntryKind::Directory {
            self.open(row.path.clone(), cx);
        }
        self.clear_search(window, cx);
        self.reveal = Some(row.path);
        self.continue_reveal(cx);
        if let Some(back) = self.keys_back.clone() {
            window.focus(&back, cx);
        }
        cx.notify();
    }

    fn search_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let len = self.search_tree.rows().len();
        match event.keystroke.key.as_str() {
            "escape" => {
                self.clear_search(window, cx);
                if let Some(back) = self.keys_back.clone() {
                    window.focus(&back, cx);
                }
            }
            "up" | "down" if len > 0 => {
                self.search_active = if event.keystroke.key == "up" { self.search_active.saturating_sub(1) } else { (self.search_active + 1).min(len - 1) };
                self.search_list.scroll_to_reveal_item(self.search_active);
            }
            "enter" if !self.query.is_empty() => self.activate_search_result(window, cx),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// A key with the keyboard in the explorer (not in its search): Zeron's tree keys, `j` `k` `h` `l` too, as the
    /// diffs take them; `/` goes to the search. False for one it doesn't take.
    pub(super) fn nav_key(&mut self, key: &str, shift: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.menu.as_ref().is_some_and(|m| m.is_open()) {
            if key == "escape" {
                self.close_menu(cx);
                return true;
            }
            return false;
        }
        match key {
            "/" => {
                let focus = self.search.read(cx).focus_handle(cx);
                window.focus(&focus, cx);
                return true;
            }
            "up" | "k" if shift => {
                self.tree.extend_selection(-1);
            }
            "down" | "j" if shift => {
                self.tree.extend_selection(1);
            }
            "up" | "k" => {
                self.tree.select_previous();
            }
            "down" | "j" => {
                self.tree.select_next();
            }
            "left" | "h" => {
                if let Some(path) = self.tree.selected().map(str::to_string) {
                    if self.tree.is_expanded(&path) {
                        self.tree.toggle_expanded(&path);
                        self.sync_tree_list();
                    } else {
                        self.tree.select_parent();
                    }
                }
            }
            "right" | "l" => {
                if let Some(path) = self.tree.selected().map(str::to_string).filter(|p| self.tree.node(p).is_some_and(|n| n.entry.kind == EntryKind::Directory)) {
                    if self.tree.is_expanded(&path) {
                        self.tree.select_first_child();
                    } else {
                        self.toggle_tree_directory(path, cx);
                    }
                }
            }
            "enter" | "space" => {
                if let Some(path) = self.tree.selected().map(str::to_string) {
                    self.activate_tree_path(path, cx);
                }
            }
            "escape" if self.tree.collapse_selection() => {}
            _ => return false,
        }
        self.reveal_tree_selection();
        cx.notify();
        true
    }

    /// What the menu acts on: the selection's roots when the cursor is in it, else the cursor alone.
    fn action_paths(&self) -> Vec<String> {
        match self.tree.selected() {
            Some(cursor) if self.tree.is_selected(cursor) => self.tree.selection_roots(),
            Some(cursor) => vec![cursor.to_string()],
            None => Vec::new(),
        }
    }

    fn open_menu(&mut self, path: String, at: gpui::Point<Pixels>, cx: &mut Context<Self>) {
        if self.tree.node(&path).is_none() {
            return;
        }
        // Like other explorers, a right-click inside the selection keeps it.
        if !self.tree.is_selected(&path) {
            self.tree.select(path);
        }
        self.menu = Some(crate::ui::Popup::new((self.action_paths(), at)));
        cx.notify();
    }

    fn close_menu(&mut self, cx: &mut Context<Self>) {
        if let Some(menu) = self.menu.as_mut() {
            menu.close();
        }
        cx.notify();
    }

    fn render_menu(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        type Action = Box<dyn Fn(&mut Files, &mut Context<Files>)>;
        let menu = self.menu.as_ref()?;
        let (paths, at) = menu.value.clone();
        let place = self.place.clone()?;
        let item = |key: &'static str, icon: &'static str, label: &'static str, action: Action| {
            row(theme, false, key).h(px(28.)).rounded(px(6.)).child(menu_mark(icon, theme.text_muted)).child(label).on_pick(
                at,
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.close_menu(cx);
                    action(this, cx);
                }),
            )
        };
        let open = {
            let (place, paths) = (place.clone(), paths.clone());
            item("files-menu-open", "icons/solar/pen.svg", "Open in Editor", Box::new(move |_, cx| paths.iter().for_each(|path| cx.emit(FilesEvent::Edit { place: place.clone(), path: path.clone() }))))
        };
        let reveal = place.machine().is_none().then(|| {
            let shown = place.full(&paths[0]);
            let label = if cfg!(target_os = "macos") { "Reveal in Finder" } else { "Show in file manager" };
            item("files-menu-reveal", "icons/solar/folder-open.svg", label, Box::new(move |_, cx| cx.reveal_path(&shown)))
        });
        let copied = paths.iter().map(|path| place.full(path).to_string_lossy().into_owned()).collect::<Vec<_>>().join("\n");
        let copy = item("files-menu-copy", "icons/solar/copy.svg", if paths.len() > 1 { "Copy Paths" } else { "Copy Path" }, Box::new(move |_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(copied.clone()))));
        let card = menu_card(theme, "files-menu", 180.)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.close_menu(cx)))
            .flex()
            .flex_col()
            .gap(px(2.))
            .text_size(px(12.))
            .child(open)
            .children(reveal)
            .child(copy);
        let card = crate::ui::menu_motion("files-menu-in", menu, card);
        Some(gpui::deferred(gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(card)).with_priority(1).into_any_element())
    }

    /// Zeron's explorer header: the search field and the eye that shows ignored files, and a refresh.
    fn render_header(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let include_ignored = self.tree.include_ignored();
        let field = div()
            .flex_1()
            .min_w_0()
            .h(px(26.))
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(6.))
            .rounded(px(6.))
            .bg(theme.wash(0.04))
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| this.search_key(event, window, cx)))
            .child(gpui::svg().path("icons/lucide-search.svg").size(px(12.)).flex_none().text_color(theme.text_faint))
            .child(div().flex_1().min_w_0().overflow_hidden().text_size(px(12.)).child(Input::new(&self.search)));
        let eye = super::changes::Changes::header_toggle(
            "files-toggle-ignored",
            if include_ignored { "icons/solar/eye.svg" } else { "icons/solar/eye-closed.svg" },
            if include_ignored { "Hide ignored files" } else { "Show all files" },
            include_ignored,
            theme,
        )
        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_ignored(cx)));
        let refresh = super::changes::Changes::header_toggle("files-refresh", "icons/solar/refresh.svg", "Refresh", false, theme).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.refresh(cx)));
        toolbar(theme).child(field).child(eye).child(refresh).into_any_element()
    }

    fn render_tree(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("files-tree")
            .relative()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    // Like other explorers, a plain click on empty space deselects.
                    let m = event.modifiers;
                    let on_row = this.tree_list.viewport_bounds().contains(&event.position)
                        && (0..this.tree.visible_rows().len()).any(|ix| this.tree_list.bounds_for_item(ix).is_some_and(|b| b.contains(&event.position)));
                    if !m.shift && !m.secondary() && !on_row && this.tree.clear_selection() {
                        cx.notify();
                    }
                }),
            )
            .child(list(self.tree_list.clone(), cx.processor(Self::render_tree_row)).flex_1().min_h_0().with_sizing_behavior(ListSizingBehavior::Auto))
            .child(crate::ui::scrollbar("files-tree-scrollbar", &self.tree_list, theme))
            .into_any_element()
    }

    fn render_tree_row(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.tree.visible_rows().get(index).cloned() else { return gpui::Empty.into_any_element() };
        let theme = Theme::of(cx).clone();
        let padding = 8.0 + row.depth as f32 * TREE_INDENT;
        let content = match row.kind {
            VisibleRowKind::Entry => {
                let Some(node) = self.tree.node(&row.path).cloned() else { return gpui::Empty.into_any_element() };
                let (path, menu_path) = (row.path.clone(), row.path.clone());
                let selected = self.tree.is_selected(&path);
                let cursor = self.tree.selected() == Some(path.as_str());
                // the keyboard cursor, once it no longer equals the whole selection
                let distinct_cursor = cursor && self.keys && (!selected || self.tree.selection_len() > 1);
                let is_directory = node.entry.kind == EntryKind::Directory;
                let decoration = self.marks.get(&row.path, is_directory);
                let expanded = is_directory && self.tree.is_expanded(&path);
                let text_color = match decoration {
                    Some(decoration) => decoration.color(&theme),
                    None if selected => theme.text,
                    None => theme.text_muted,
                };
                let hover = theme.wash(0.055);
                div()
                    .id(SharedString::from(format!("files-tree-entry:{}", row.path)))
                    .h(px(TREE_ROW_HEIGHT))
                    .w_full()
                    .flex_none()
                    .pl(px(padding))
                    .pr(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .cursor_pointer()
                    .when(node.entry.ignored && decoration.is_none(), |el| el.opacity(0.52))
                    .when(selected, |el| el.bg(theme.wash(match (self.keys, distinct_cursor) {
                        (true, true) => 0.16,
                        (true, false) => 0.12,
                        (false, _) => 0.08,
                    })))
                    .when(!selected, |el| el.when(distinct_cursor, |el| el.bg(theme.wash(0.04))).hover(move |s| s.bg(hover)))
                    .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                        let modifiers = event.modifiers();
                        if modifiers.shift {
                            this.tree.select_range_to(&path, modifiers.secondary());
                            cx.notify();
                        } else if modifiers.secondary() {
                            this.tree.toggle_selected(path.clone());
                            cx.notify();
                        } else {
                            this.activate_tree_path(path.clone(), cx);
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            this.open_menu(menu_path.clone(), event.position, cx);
                        }),
                    )
                    .child(chevron(expanded, is_directory, &theme))
                    .child(entry_icon(&node.entry.name, node.entry.kind, expanded, &theme))
                    .child(div().min_w_0().truncate().text_size(px(12.)).text_color(text_color).child(node.entry.name))
                    .into_any_element()
            }
            VisibleRowKind::Loading { .. } => status_row(index, padding + TREE_INDENT, "Loading…", theme.text_faint),
            VisibleRowKind::Empty { .. } => status_row(index, padding + TREE_INDENT, "Empty folder", theme.text_faint.opacity(0.7)),
            VisibleRowKind::Error { directory, message } => {
                let hover = theme.wash(0.055);
                div()
                    .id(("files-tree-error", index))
                    .h(px(TREE_ROW_HEIGHT))
                    .w_full()
                    .flex_none()
                    .pl(px(padding + TREE_INDENT))
                    .pr(px(8.0))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.load_directory(directory.clone(), cx)))
                    .child(div().min_w_0().truncate().text_size(px(10.5)).text_color(theme.danger.opacity(0.82)).child(format!("{message} — Retry")))
                    .into_any_element()
            }
        };
        with_indent_guides(content, row.depth, &theme)
    }

    fn render_search_results(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        if let Some(error) = self.search_error.clone() {
            return centered(error, theme.danger.opacity(0.82));
        }
        if self.results.is_empty() {
            return centered(if self.searching { "Searching…" } else { "No files found." }, theme.text_faint);
        }
        div()
            .id("files-search-results")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .when(self.results.len() >= SEARCH_RESULT_LIMIT, |el| {
                el.child(div().h(px(24.0)).flex_none().px(px(10.0)).flex().items_center().text_size(px(10.0)).text_color(theme.text_faint).child("Showing the first 200 matches"))
            })
            .child(list(self.search_list.clone(), cx.processor(Self::render_search_row)).flex_1().min_h_0().with_sizing_behavior(ListSizingBehavior::Auto))
            .into_any_element()
    }

    fn render_search_row(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.search_tree.rows().get(index).cloned() else { return gpui::Empty.into_any_element() };
        let theme = Theme::of(cx).clone();
        let selected = self.search_active == index;
        let expanded = row.has_children && self.search_tree.is_expanded(&row.path);
        let is_directory = row.kind == EntryKind::Directory;
        let decoration = self.marks.get(&row.path, is_directory);
        let hover = theme.wash(0.055);
        let toggled = row.path.clone();
        let content = div()
            .id(("files-search-result", index))
            .h(px(TREE_ROW_HEIGHT))
            .w_full()
            .flex_none()
            .pl(px(8.0 + row.depth as f32 * TREE_INDENT))
            .pr(px(8.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .cursor_pointer()
            .when(selected, |el| el.bg(theme.wash(0.1)))
            .when(!selected, |el| el.hover(move |s| s.bg(hover)))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.search_active = index;
                this.activate_search_result(window, cx);
            }))
            .child(chevron(expanded, row.has_children, &theme).id(("files-search-fold", index)).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                if this.search_tree.toggle(&toggled) {
                    this.search_list.reset(this.search_tree.rows().len());
                    cx.notify();
                }
            })))
            .child(entry_icon(&row.name, row.kind, expanded, &theme))
            .child(div().min_w_0().truncate().text_size(px(12.)).text_color(match decoration {
                Some(value) => value.color(&theme),
                None if is_directory => theme.text_muted,
                None => theme.text,
            }).child(row.name));
        with_indent_guides(content.into_any_element(), row.depth, &theme)
    }
}

impl Render for Files {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::ui::reap_popup(&mut self.menu, window);
        let theme = Theme::of(cx).clone();
        let header = self.render_header(&theme, cx);
        let loading = !self.tree_has_content() && self.tree.node("").is_some_and(|n| matches!(n.load, DirectoryLoadState::Unloaded | DirectoryLoadState::Loading));
        let content = if self.dir.is_none() {
            centered("The focused pane has no folder.", theme.text_faint)
        } else if !self.query.is_empty() {
            self.render_search_results(&theme, cx)
        } else if let Some(error) = self.error.clone().filter(|_| !self.tree_has_content()) {
            centered(error, theme.warning.opacity(0.85))
        } else if loading {
            div().flex_1().flex().items_center().justify_center().child(crate::ui::loader::spinner(2.5, window)).into_any_element()
        } else {
            self.render_tree(&theme, cx)
        };
        let root = self.place.as_ref().map(|p| p.root.rsplit('/').next().unwrap_or(&p.root).to_string());
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(header)
            .when_some(root, |el, root| el.child(div().flex_none().px(px(10.)).pt(px(6.)).pb(px(2.)).text_size(px(10.5)).text_color(theme.text_faint).truncate().child(root)))
            .child(content)
            .children(self.render_menu(&theme, cx))
    }
}

/// What a file's tab holds.
enum Content {
    Loading,
    /// Text, in the editor; read-only when only part of it was read.
    Text { truncated: bool },
    /// A copy here, under its content's hash: gpui keeps an image by its path.
    Image(std::path::PathBuf),
    Binary,
    Error(SharedString),
}

/// Between a key in the editor and the highlight it asks for.
const HIGHLIGHT_DEBOUNCE: Duration = Duration::from_millis(250);

/// A file's own tab, Zeron's editor presentation: its text in an editor saved with ⌘S, its Markdown drawn (its source
/// a toggle away), or its image.
pub(super) struct FilePreview {
    pub(super) place: Place,
    pub(super) path: String,
    content: Content,
    /// Text just read, for the editor: made on the next render (it needs the window), or replacing a clean one's.
    pending: Option<String>,
    /// The text as on disk, read or saved: the editor's differs from it while it has changes.
    saved: String,
    pub(super) dirty: bool,
    /// The file changed on disk under unsaved changes: which to keep is asked.
    conflict: bool,
    /// The blob id of what was read or saved: a poll asks again, and reads the file again when it moved.
    hash: Option<String>,
    editor: Option<Entity<gpui_base::input::EditorState>>,
    _editor_events: Option<Subscription>,
    highlight_task: Option<Task<()>>,
    read_task: Option<Task<()>>,
    save_task: Option<Task<()>>,
    watch_task: Option<Task<()>>,
    /// The Markdown drawn, from the editor's text when the preview was last asked for.
    markdown: Option<Arc<Vec<crate::ui::review::Block>>>,
    markdown_scroll: gpui::ScrollHandle,
    show_source: bool,
    /// A first close over unsaved changes only warns; the next one discards them.
    pub(super) close_armed: bool,
    /// Its markdown's selection key.
    key: String,
}

impl FilePreview {
    fn new(place: Place, path: String, key: String, cx: &mut Context<Self>) -> Self {
        let mut preview = Self {
            place,
            path,
            content: Content::Loading,
            pending: None,
            saved: String::new(),
            dirty: false,
            conflict: false,
            hash: None,
            editor: None,
            _editor_events: None,
            highlight_task: None,
            read_task: None,
            save_task: None,
            watch_task: None,
            markdown: None,
            markdown_scroll: gpui::ScrollHandle::new(),
            show_source: false,
            close_armed: false,
            key,
        };
        preview.load(cx);
        preview
    }

    /// The tab's title: the file's name, a dot before it while it has unsaved changes.
    pub(super) fn tab_title(&self) -> String {
        let name = self.path.rsplit('/').next().unwrap_or(&self.path);
        if self.dirty { format!("● {name}") } else { name.to_string() }
    }

    fn is_markdown(&self) -> bool {
        matches!(std::path::Path::new(&self.path).extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref(), Some("md" | "markdown" | "mdx"))
    }

    /// Reads the file, and replaces what the tab holds with it: unsaved changes too, as Reload asks.
    fn load(&mut self, cx: &mut Context<Self>) {
        let (place, path) = (self.place.clone(), self.path.clone());
        let image = crate::terminal::pane::is_image(std::path::Path::new(&path));
        self.read_task = Some(cx.spawn(async move |this, cx| {
            let (hash, content, text) = cx
                .background_spawn(async move {
                    let hash = place.hash(&path);
                    match image {
                        true => (hash, image_here(&place, &path).map_or_else(|e| Content::Error(e.into()), Content::Image), None),
                        false => match place.read(&path) {
                            Err(e) => (hash, Content::Error(e.into()), None),
                            Ok(FileText::Binary) => (hash, Content::Binary, None),
                            Ok(FileText::Text { text, truncated }) => (hash, Content::Text { truncated }, Some(text)),
                        },
                    }
                })
                .await;
            let _ = this.update(cx, |preview, cx| {
                preview.hash = hash;
                preview.content = content;
                preview.conflict = false;
                preview.dirty = false;
                if let Some(text) = text {
                    preview.markdown = preview.is_markdown().then(|| Arc::new(crate::ui::review::markdown(&text)));
                    preview.saved = text.clone();
                    preview.pending = Some(text);
                }
                cx.notify();
            });
        }));
    }

    /// The editor, made or refreshed with the text just read.
    fn sync_editor(&mut self, theme: &Theme, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = self.pending.take() else { return };
        let readonly = matches!(self.content, Content::Text { truncated: true });
        match &self.editor {
            Some(editor) => super::file_editor::replace_file_contents(editor, text.clone(), window, cx),
            None => {
                let editor = super::file_editor::new_file_editor(text.clone(), &self.path, readonly, theme, window, cx);
                self._editor_events = Some(cx.subscribe(&editor, |this: &mut Self, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.edited(cx);
                    }
                }));
                self.editor = Some(editor);
            }
        }
        self.dirty = false;
        self.highlight(text, Duration::ZERO, cx);
    }

    fn edited(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = &self.editor else { return };
        let text = editor.read(cx).value().to_string();
        self.dirty = text != self.saved;
        self.close_armed = false;
        self.markdown = None;
        self.highlight(text, HIGHLIGHT_DEBOUNCE, cx);
        cx.notify();
    }

    /// Colors `source` in the editor, once it's highlighted and if the editor still holds it.
    fn highlight(&mut self, source: String, after: Duration, cx: &mut Context<Self>) {
        let path = self.path.clone();
        self.highlight_task = Some(cx.spawn(async move |this, cx| {
            if !after.is_zero() {
                cx.background_executor().timer(after).await;
            }
            let asked = source.clone();
            let document = cx.background_spawn(async move { zeron_syntax::highlight(zeron_syntax::HighlightRequest { source: &asked, path: Some(&path), fence_tag: None }).ok() }).await;
            let _ = this.update(cx, |preview, cx| {
                if let (Some(editor), Some(document)) = (preview.editor.clone(), document) {
                    super::file_editor::install_highlighter(&editor, source, Arc::new(document), cx);
                }
            });
        }));
    }

    /// ⌘S: the editor's text to the file, unless it changed on disk since it was read; `force` writes over that.
    fn save(&mut self, force: bool, cx: &mut Context<Self>) {
        let Some(editor) = &self.editor else { return };
        if (!self.dirty && !force) || self.save_task.is_some() || matches!(self.content, Content::Text { truncated: true }) {
            return;
        }
        let text = editor.read(cx).value().to_string();
        let (place, path, expected) = (self.place.clone(), self.path.clone(), self.hash.clone().filter(|_| !force));
        self.save_task = Some(cx.spawn(async move |this, cx| {
            let written = text.clone();
            let result = cx.background_spawn(async move { place.write(&path, &written, expected.as_deref()) }).await;
            let _ = this.update(cx, |preview, cx| {
                preview.save_task = None;
                match result {
                    Ok(hash) => {
                        preview.hash = hash;
                        preview.saved = text;
                        preview.conflict = false;
                        preview.edited(cx);
                    }
                    Err(SaveError::Changed) => preview.conflict = true,
                    Err(SaveError::Failed(error)) => crate::ui::toast::error(cx, format!("Couldn't save {}: {error}", preview.path)),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Watches the file while its tab is shown in a window in use: a change on disk comes in when there's nothing
    /// unsaved, else it asks.
    fn set_watch(&mut self, watch: bool, cx: &mut Context<Self>) {
        if watch == self.watch_task.is_some() {
            return;
        }
        self.watch_task = watch.then(|| {
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(POLL).await;
                    let Ok((place, path)) = this.read_with(cx, |p, _| (p.place.clone(), p.path.clone())) else { return };
                    let hash = cx.background_spawn(async move { place.hash(&path) }).await;
                    let alive = this.update(cx, |p, cx| {
                        let moved = p.hash.is_some() && hash.is_some() && p.hash != hash && p.save_task.is_none();
                        match moved {
                            true if p.dirty => {
                                if !p.conflict {
                                    p.conflict = true;
                                    cx.notify();
                                }
                            }
                            true => p.load(cx),
                            false => {}
                        }
                    });
                    if alive.is_err() {
                        return;
                    }
                }
            })
        });
    }

    fn render_header(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let mut crumbs = div().flex_1().min_w_0().flex().items_center().gap(px(4.)).overflow_hidden().text_size(px(12.));
        let parts: Vec<&str> = self.path.split('/').collect();
        for (i, part) in parts.iter().enumerate() {
            let last = i + 1 == parts.len();
            if i > 0 {
                crumbs = crumbs.child(div().flex_none().text_color(theme.text_faint).child("/"));
            }
            crumbs = crumbs.child(div().when(!last, |el| el.flex_shrink(1.).min_w_0()).when(last, |el| el.flex_none()).truncate().text_color(if last { theme.text } else { theme.text_muted }).child(part.to_string()));
        }
        if self.dirty {
            crumbs = crumbs.child(div().flex_none().size(px(6.)).rounded_full().bg(theme.text_muted));
        }
        let toggle = super::changes::Changes::header_toggle;
        let save = matches!(self.content, Content::Text { truncated: false }).then(|| {
            toggle("file-save", "icons/check.svg", if cfg!(target_os = "macos") { "Save (⌘S)" } else { "Save (Ctrl+S)" }, false, theme)
                .when(!self.dirty, |el| el.opacity(0.35))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.save(false, cx)))
        });
        let source = (self.is_markdown() && matches!(self.content, Content::Text { .. })).then(|| {
            toggle("file-markdown-source", "icons/solar/code.svg", if self.show_source { "Show preview" } else { "Edit source" }, self.show_source, theme).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.show_source = !this.show_source;
                cx.notify();
            }))
        });
        let edit = toggle("file-open-editor", "icons/solar/pen.svg", "Open in Editor", false, theme).on_click(cx.listener(|this, _: &ClickEvent, _, cx| cx.emit(FilesEvent::Edit { place: this.place.clone(), path: this.path.clone() })));
        let reveal = self.place.machine().is_none().then(|| {
            let shown = self.place.full(&self.path);
            toggle("file-reveal", "icons/solar/folder-open.svg", if cfg!(target_os = "macos") { "Reveal in Finder" } else { "Show in file manager" }, false, theme).on_click(move |_, _, cx| cx.reveal_path(&shown))
        });
        let copied = self.place.full(&self.path).to_string_lossy().into_owned();
        let copy = toggle("file-copy-path", "icons/solar/copy.svg", "Copy Path", false, theme).on_click(move |_, _, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(copied.clone())));
        toolbar(theme)
            .child(crate::ui::file_icons::icon(&self.path, theme.appearance).size(px(14.)))
            .child(crumbs)
            .children(save)
            .children(source)
            .child(edit)
            .children(reveal)
            .child(copy)
            .into_any_element()
    }

    /// The file changed on disk under unsaved changes: theirs back, or mine over it.
    fn render_conflict(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let button = |id: &'static str, label: &'static str| {
            let hover = theme.wash(0.1);
            div().id(id).flex_none().h(px(22.)).px(px(8.)).rounded(px(6.)).flex().items_center().cursor_pointer().text_color(theme.text).hover(move |s| s.bg(hover)).child(label)
        };
        div()
            .flex_none()
            .h(px(32.))
            .px(px(10.))
            .flex()
            .items_center()
            .gap(px(6.))
            .border_b_1()
            .border_color(theme.border)
            .bg(theme.warning.opacity(0.06))
            .text_size(px(11.5))
            .child(div().flex_1().min_w_0().truncate().text_color(theme.warning).child("Changed on disk since it was opened."))
            .child(button("file-conflict-reload", "Reload").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.load(cx))))
            .child(button("file-conflict-overwrite", "Overwrite").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.save(true, cx))))
            .into_any_element()
    }

    fn render_markdown(&self, blocks: &[crate::ui::review::Block], theme: &Theme) -> AnyElement {
        // a local file's images, beside it; a machine's keep their alt text
        let dir = self.place.machine().is_none().then(|| self.place.full(&self.path).parent().map(std::path::Path::to_path_buf)).flatten();
        let look = crate::ui::markdown::Look {
            document: true,
            image: dir.map(|dir| -> Rc<dyn Fn(&str) -> Option<std::path::PathBuf>> {
                Rc::new(move |source: &str| {
                    let path = dir.join(percent_encoding::percent_decode_str(source).decode_utf8_lossy().as_ref());
                    (!source.contains("://") && path.is_file()).then_some(path)
                })
            }),
            link: Some(Rc::new(|url: &str, _: &mut Window, cx: &mut App| {
                if url.contains("://") || url.starts_with("mailto:") {
                    crate::ui::open_outside(url, cx);
                }
            })),
            open_image: None,
        };
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div().id("file-markdown").flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.markdown_scroll).child(
                    div()
                        .max_w(px(800.))
                        .mx_auto()
                        .px(px(20.))
                        .pt(px(12.))
                        .pb(px(24.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .text_size(px(14.))
                        .line_height(px(22.))
                        .child(crate::ui::markdown::frame_reset())
                        .children(crate::ui::markdown::render_with(blocks, &self.key, theme, &look)),
                ),
            )
            .child(crate::ui::scrollbar("file-markdown-scrollbar", &self.markdown_scroll, theme))
            .into_any_element()
    }
}

/// A file as one here for the image view: a copy under its content's hash, so a changed file is a new image.
fn image_here(place: &Place, path: &str) -> Result<std::path::PathBuf, String> {
    use std::hash::{Hash, Hasher};
    if !crate::changes::safe_path(path) {
        return Err("Not a file of this checkout".into());
    }
    let full = place.full(path);
    let bytes = match place.machine() {
        None => std::fs::read(&full),
        Some(target) => crate::herdr::read_file(target, &full.to_string_lossy()),
    }
    .map_err(|e| e.to_string())?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    // ponytail: copies pile up in the cache as the reader's do; prune by age if they ever matter
    let dir = crate::settings::cache_dir().join("files").join(format!("{:016x}", h.finish()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let copy = dir.join(full.file_name().unwrap_or_default());
    std::fs::write(&copy, bytes).map_err(|e| e.to_string())?;
    Ok(copy)
}

impl Render for FilePreview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::of(cx).clone();
        self.sync_editor(&theme, window, cx);
        let header = self.render_header(&theme, cx);
        let conflict = self.conflict.then(|| self.render_conflict(&theme, cx));
        let body = match &self.content {
            Content::Loading => div().flex_1().flex().items_center().justify_center().child(crate::ui::loader::spinner(2.5, window)).into_any_element(),
            Content::Error(error) => centered(error.clone(), theme.warning.opacity(0.85)),
            Content::Binary => centered("This file is binary and can't be previewed.", theme.text_muted),
            Content::Image(path) => div()
                .id("file-image")
                .flex_1()
                .min_h_0()
                .p(px(16.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .tooltip(tip("Click to zoom", None))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| cx.emit(FilesEvent::Zoom { place: this.place.clone(), path: this.path.clone() })))
                .child(gpui::img(path.clone()).max_w_full().max_h_full().object_fit(gpui::ObjectFit::Contain))
                .into_any_element(),
            Content::Text { .. } if self.is_markdown() && !self.show_source => {
                if self.markdown.is_none()
                    && let Some(editor) = &self.editor
                {
                    self.markdown = Some(Arc::new(crate::ui::review::markdown(&editor.read(cx).value())));
                }
                let blocks = self.markdown.clone().unwrap_or_default();
                self.render_markdown(&blocks, &theme)
            }
            Content::Text { truncated } => {
                let truncated = *truncated;
                let editor = self.editor.clone().map(|editor| {
                    div()
                        .id("file-editor")
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                        .relative()
                        .overflow_hidden()
                        .font_family(theme.font_terminal.clone())
                        .text_size(px(PREVIEW_TEXT_SIZE))
                        .line_height(px(PREVIEW_LINE_HEIGHT))
                        .child(gpui_base::input::Editor::new(&editor))
                });
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .when(truncated, |el| {
                        el.child(
                            div()
                                .h(px(28.0))
                                .flex_none()
                                .px(px(10.0))
                                .border_b_1()
                                .border_color(theme.border)
                                .bg(theme.warning.opacity(0.045))
                                .flex()
                                .items_center()
                                .text_size(px(10.5))
                                .text_color(theme.warning)
                                .child("Large file: only its first 1 MB is shown, read-only."),
                        )
                    })
                    .children(editor)
                    .into_any_element()
            }
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            // ⌘S (Ctrl+S off macOS) saves, wherever the keyboard is in the tab
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                let m = &event.keystroke.modifiers;
                if event.keystroke.key == "s" && m.secondary() && !m.shift && !m.alt {
                    this.save(false, cx);
                    cx.stop_propagation();
                }
            }))
            .child(header)
            .children(conflict)
            .child(body)
    }
}

impl Shell {
    /// The picker's Files card, the `+` menu's row: the workspace's explorer, made the first time.
    pub(super) fn add_files_surface(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(workspace) = self.shown_tab().map(|t| t.workspace_id) else { return };
        if !self.right.files.contains_key(&workspace) {
            let files = cx.new(|cx| Files::new(window, cx));
            files.update(cx, |files, _| files.keys_back = Some(self.focus.clone()));
            cx.subscribe_in(&files, window, |this, _, event: &FilesEvent, window, cx| this.files_event(event, window, cx)).detach();
            self.right.files.insert(workspace, files);
        }
        self.update_panel(cx, |p| p.add(Surface::Files));
    }

    fn files_event(&mut self, event: &FilesEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            FilesEvent::Open { place, path } => self.open_file_surface(place.clone(), path.clone(), window, cx),
            FilesEvent::Edit { place, path } => self.edit_diff_file(&place.dir, &place.root, path, cx),
            FilesEvent::Zoom { place, path } => self.preview_diff_file(&place.dir, &place.root, path, window, cx),
        }
    }

    /// A file's tab: the one open on it in this panel, else a new one.
    fn open_file_surface(&mut self, place: Place, path: String, window: &mut Window, cx: &mut Context<Self>) {
        let same = |p: &FilePreview| p.path == path && p.place.root == place.root && p.place.machine() == place.machine();
        let open = self.panel().and_then(|panel| {
            panel.tabs.iter().find(|s| matches!(s, Surface::File(id) if self.right.file_tabs.get(id).is_some_and(|f| same(f.read(cx))))).cloned()
        });
        let surface = open.unwrap_or_else(|| {
            self.right.made_files += 1;
            let surface = Surface::file(self.right.made_files);
            let key = format!("files-md-{}", self.right.made_files);
            let preview = cx.new(|cx| FilePreview::new(place, path, key, cx));
            cx.subscribe_in(&preview, window, |this, _, event: &FilesEvent, window, cx| this.files_event(event, window, cx)).detach();
            self.right.file_tabs.insert(surface.key().to_string(), preview);
            surface
        });
        self.update_panel(cx, |p| p.add(surface));
    }

    /// Tells the explorer what the shown pane is and whether to watch it, and each file's tab whether it's shown.
    pub(super) fn sync_files(&mut self, shown: &ShownReview, window: &Window, cx: &mut Context<Self>) {
        let active = self.panel().and_then(|p| p.active.clone()).filter(|_| self.right_panel_shown(window));
        let in_use = window.is_window_active();
        let keys = self.panel_focused() && self.focus.is_focused(window);
        let workspace = self.shown_tab().map(|t| t.workspace_id);
        for (ws, files) in &self.right.files {
            let here = Some(ws) == workspace.as_ref() && active == Some(Surface::Files);
            files.update(cx, |files, cx| {
                if files.keys != (here && keys) {
                    files.keys = here && keys;
                    cx.notify();
                }
                files.show(shown.dir.as_deref().filter(|_| here), here && in_use, cx);
            });
        }
        for (id, preview) in &self.right.file_tabs {
            let here = matches!(&active, Some(Surface::File(a)) if a == id);
            preview.update(cx, |preview, cx| preview.set_watch(here && in_use, cx));
        }
    }

    pub(super) fn files_of_shown(&self) -> Option<Entity<Files>> {
        self.right.files.get(&self.shown_tab()?.workspace_id).cloned()
    }

    /// The explorer's body; a press in it gives the panel the keyboard.
    pub(super) fn render_files(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(files) = self.files_of_shown() else { return gpui::Empty.into_any_element() };
        Self::panel_body("right-panel-files", files.into_any_element(), cx)
    }

    pub(super) fn render_file(&mut self, id: &str, cx: &mut Context<Self>) -> AnyElement {
        let Some(preview) = self.right.file_tabs.get(id).cloned() else { return gpui::Empty.into_any_element() };
        Self::panel_body("right-panel-file", preview.into_any_element(), cx)
    }

    /// As the diff's: the keys to the panel; a field pressed in takes them on the same press.
    fn panel_body(id: &'static str, child: AnyElement, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id(id)
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .capture_any_mouse_down(cx.listener(|this, _: &MouseDownEvent, window, cx| {
                if !this.panel_focused() || !this.focus.is_focused(window) {
                    this.focus_panel(true, window, cx);
                }
            }))
            .child(child)
            .into_any_element()
    }
}

impl Shell {
    /// `SHEPHERD_PANEL=files`: the explorer; `files:<path>` shows that path in its tree, `file:<path>` opens its tab, once
    /// the checkout is found (screenshots).
    pub fn show_files(&mut self, which: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.add_files_surface(window, cx);
        let (open, path) = match which.split_once(':') {
            Some((kind, path)) => (kind == "file", path.to_string()),
            None => return,
        };
        cx.spawn_in(window, async move |this, cx| {
            for _ in 0..50 {
                cx.background_executor().timer(Duration::from_millis(200)).await;
                let Ok(found) = this.update_in(cx, |shell, window, cx| {
                    let Some(files) = shell.files_of_shown() else { return true };
                    let Some(place) = files.read(cx).place.clone() else { return false };
                    match open {
                        true => shell.open_file_surface(place, path.clone(), window, cx),
                        false => files.update(cx, |files, cx| {
                            files.reveal = Some(path.clone());
                            files.continue_reveal(cx);
                        }),
                    }
                    true
                }) else {
                    return;
                };
                if found {
                    return;
                }
            }
        })
        .detach();
    }
}
