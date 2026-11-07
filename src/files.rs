//! The right panel's files, Zeron's `crates/ui/src/files/` (MIT): what's plain of it. The tree model (`model.rs`), the
//! git decorations (`git_status.rs`) and the search results' tree (`search.rs`) are its own, cut to a first pass that
//! only reads: no pages of a folder, no renames, moves or drags. What its engine answers over RPC
//! (`crates/engine/src/workspace_files.rs`: a folder's listing, the name search, a file's text) is asked here of the
//! disk and `git`, or of the machine shown over SSH, as the diffs are (`shell/files.rs` is the entity and what it draws).
use std::collections::{HashMap, HashSet};

/// The most of a file the preview reads; the rest is cut off, as Zeron's read cuts a large file.
pub const MAX_PREVIEW_BYTES: usize = 1024 * 1024;
/// A folder lists at most this many entries (Zeron's `MAX_DIRECTORY_ENTRIES`).
const MAX_DIRECTORY_ENTRIES: usize = 50_000;
/// Zeron's `SEARCH_RESULT_LIMIT`.
pub const SEARCH_RESULT_LIMIT: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
}

/// A file or folder of the checkout, by its path from the root (`/`-separated, the root itself `""`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub name: String,
    pub kind: EntryKind,
    /// Git ignores it: shown only with all files on, dimmed.
    pub ignored: bool,
}

/// A folder's whole listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryPage {
    pub directory: String,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub path: String,
    pub name: String,
    pub kind: EntryKind,
    pub score: i64,
}

// ---------------------------------------------------------------------------
// The tree (Zeron's model.rs)
// ---------------------------------------------------------------------------

// ponytail: a folder loads whole (Zeron pages it 500 at a time over RPC); its pages and Load more if a huge folder lags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryLoadState {
    Unloaded,
    Loading,
    Loaded,
    Error { message: String },
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub entry: Entry,
    pub children: Vec<String>,
    pub load: DirectoryLoadState,
    pub stale: bool,
    pub has_loaded: bool,
}

impl TreeNode {
    fn new(entry: Entry) -> Self {
        let load = if entry.kind == EntryKind::Directory { DirectoryLoadState::Unloaded } else { DirectoryLoadState::Loaded };
        Self { entry, children: Vec::new(), load, stale: false, has_loaded: false }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisibleRowKind {
    Entry,
    Loading { directory: String },
    Empty { directory: String },
    Error { directory: String, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleTreeRow {
    pub path: String,
    pub depth: usize,
    pub kind: VisibleRowKind,
}

impl VisibleTreeRow {
    pub fn selectable(&self) -> bool {
        self.kind == VisibleRowKind::Entry
    }
}

#[derive(Debug, Clone)]
pub struct FileTreeModel {
    nodes: HashMap<String, TreeNode>,
    expanded: HashSet<String>,
    visible_rows: Vec<VisibleTreeRow>,
    /// The keyboard cursor; it need not be part of `selection`.
    selected: Option<String>,
    selection: HashSet<String>,
    /// Where Shift ranges start; follows plain and Ctrl/Cmd clicks.
    anchor: Option<String>,
    include_ignored: bool,
    generation: u64,
}

impl Default for FileTreeModel {
    fn default() -> Self {
        Self::with_include_ignored(false)
    }
}

impl FileTreeModel {
    pub fn with_include_ignored(include_ignored: bool) -> Self {
        let mut nodes = HashMap::new();
        nodes.insert(String::new(), TreeNode::new(root_entry()));
        Self { nodes, expanded: HashSet::from([String::new()]), visible_rows: Vec::new(), selected: None, selection: HashSet::new(), anchor: None, include_ignored, generation: 0 }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn include_ignored(&self) -> bool {
        self.include_ignored
    }

    pub fn visible_rows(&self) -> &[VisibleTreeRow] {
        &self.visible_rows
    }

    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    pub fn is_selected(&self, path: &str) -> bool {
        self.selection.contains(path)
    }

    pub fn selection_len(&self) -> usize {
        self.selection.len()
    }

    /// Selected entries in visible order.
    pub fn selected_paths(&self) -> Vec<String> {
        self.visible_rows.iter().filter(|row| row.kind == VisibleRowKind::Entry && self.selection.contains(&row.path)).map(|row| row.path.clone()).collect()
    }

    /// Selected entries without those already covered by a selected ancestor.
    pub fn selection_roots(&self) -> Vec<String> {
        self.selected_paths().into_iter().filter(|path| !path.match_indices('/').any(|(end, _)| self.selection.contains(&path[..end]))).collect()
    }

    pub fn node(&self, path: &str) -> Option<&TreeNode> {
        self.nodes.get(path)
    }

    pub fn is_expanded(&self, path: &str) -> bool {
        self.expanded.contains(path)
    }

    pub fn expanded_directories(&self) -> Vec<String> {
        let mut paths = self.expanded.iter().filter(|path| !path.is_empty()).cloned().collect::<Vec<_>>();
        paths.sort_by_key(|path| (path.matches('/').count(), path.clone()));
        paths
    }

    pub fn is_directory_loaded(&self, path: &str) -> bool {
        self.nodes.get(path).is_some_and(|node| node.entry.kind == EntryKind::Directory && node.load == DirectoryLoadState::Loaded)
    }

    pub fn reset(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.nodes.clear();
        self.nodes.insert(String::new(), TreeNode::new(root_entry()));
        self.expanded.clear();
        self.expanded.insert(String::new());
        self.visible_rows.clear();
        self.set_single(None);
        self.generation
    }

    pub fn set_include_ignored(&mut self, include_ignored: bool) -> bool {
        if self.include_ignored == include_ignored {
            return false;
        }
        self.include_ignored = include_ignored;
        self.reset();
        true
    }

    pub fn begin_load(&mut self, directory: &str, generation: u64) -> bool {
        if generation != self.generation {
            return false;
        }
        let Some(node) = self.nodes.get_mut(directory) else { return false };
        if node.load == DirectoryLoadState::Loading {
            return false;
        }
        node.load = DirectoryLoadState::Loading;
        node.stale = false;
        self.rebuild_visible_rows();
        true
    }

    pub fn fail_load(&mut self, directory: &str, message: impl Into<String>, generation: u64) -> bool {
        if generation != self.generation {
            return false;
        }
        let Some(node) = self.nodes.get_mut(directory) else { return false };
        node.load = DirectoryLoadState::Error { message: message.into() };
        self.rebuild_visible_rows();
        true
    }

    pub fn apply_page(&mut self, page: DirectoryPage, generation: u64) -> bool {
        if generation != self.generation {
            return false;
        }
        let directory = page.directory;
        let Some(parent) = self.nodes.get(&directory) else { return false };
        if parent.entry.kind != EntryKind::Directory {
            return false;
        }
        let parent_ignored = parent.entry.ignored;
        let incoming: HashSet<String> = page.entries.iter().filter(|entry| is_direct_child(&entry.path, &directory)).map(|entry| entry.path.clone()).collect();
        for child in parent.children.clone() {
            if !incoming.contains(&child) {
                self.remove_subtree(&child);
            }
        }
        self.nodes.get_mut(&directory).unwrap().children.retain(|path| incoming.contains(path));

        let mut known_children = self.nodes.get(&directory).map(|node| node.children.iter().cloned().collect::<HashSet<_>>()).unwrap_or_default();
        for mut entry in page.entries {
            if !is_direct_child(&entry.path, &directory) {
                continue;
            }
            entry.ignored |= parent_ignored;
            let path = entry.path.clone();
            if self.nodes.get(&path).is_some_and(|node| node.entry.kind != entry.kind) {
                self.remove_subtree(&path);
            }
            self.nodes.entry(path.clone()).and_modify(|node| node.entry = entry.clone()).or_insert_with(|| TreeNode::new(entry));
            if let Some(parent) = self.nodes.get_mut(&directory)
                && known_children.insert(path.clone())
            {
                parent.children.push(path);
            }
        }

        let mut children = self.nodes.get(&directory).map(|node| node.children.clone()).unwrap_or_default();
        children.sort_by(|left, right| compare_paths(&self.nodes, left, right));
        if let Some(parent) = self.nodes.get_mut(&directory) {
            parent.children = children;
            parent.load = DirectoryLoadState::Loaded;
            parent.stale = false;
            parent.has_loaded = true;
        }
        self.rebuild_visible_rows();
        true
    }

    pub fn toggle_expanded(&mut self, path: &str) -> bool {
        let Some(node) = self.nodes.get(path) else { return false };
        if node.entry.kind != EntryKind::Directory {
            return false;
        }
        if !self.expanded.remove(path) {
            self.expanded.insert(path.to_string());
        } else if let Some(cursor) = self.selected.clone().filter(|selected| is_descendant(selected, path)) {
            // The folder replaces a selected cursor; a deselected cursor must not pull the folder into the selection.
            let folder = (!path.is_empty()).then(|| path.to_string());
            if let Some(folder) = &folder
                && self.selection.contains(&cursor)
            {
                self.selection.insert(folder.clone());
            }
            self.selected = folder;
        }
        self.rebuild_visible_rows();
        true
    }

    pub fn expand(&mut self, path: &str) -> bool {
        if self.nodes.get(path).is_none_or(|node| node.entry.kind != EntryKind::Directory) {
            return false;
        }
        let changed = self.expanded.insert(path.to_string());
        if changed {
            self.rebuild_visible_rows();
        }
        changed
    }

    pub fn select(&mut self, path: impl Into<String>) -> bool {
        let path = path.into();
        if !self.nodes.contains_key(&path) {
            return false;
        }
        self.set_single(Some(path));
        true
    }

    /// Ctrl/Cmd+click: flip one entry without disturbing the rest.
    pub fn toggle_selected(&mut self, path: impl Into<String>) -> bool {
        let path = path.into();
        if path.is_empty() || !self.nodes.contains_key(&path) {
            return false;
        }
        if !self.selection.remove(&path) {
            self.selection.insert(path.clone());
        }
        self.anchor = Some(path.clone());
        self.selected = Some(path);
        true
    }

    /// Shift+click: select the visible entries between the anchor and `path`.
    pub fn select_range_to(&mut self, path: &str, additive: bool) -> bool {
        let entries = self.visible_rows.iter().filter(|row| row.kind == VisibleRowKind::Entry).map(|row| row.path.as_str()).collect::<Vec<_>>();
        let Some(end) = entries.iter().position(|candidate| *candidate == path) else { return false };
        let start = self.anchor.as_deref().or(self.selected.as_deref()).and_then(|anchor| entries.iter().position(|candidate| *candidate == anchor)).unwrap_or(end);
        let range = entries[start.min(end)..=start.max(end)].iter().map(|path| path.to_string()).collect::<Vec<_>>();
        if !additive {
            self.selection.clear();
        }
        self.selection.extend(range);
        self.anchor = Some(entries[start].to_string());
        self.selected = Some(path.to_string());
        true
    }

    /// Shift+arrow: move the cursor to the next entry and extend the range.
    pub fn extend_selection(&mut self, delta: isize) -> Option<&str> {
        let entries = self.visible_rows.iter().filter(|row| row.kind == VisibleRowKind::Entry).map(|row| row.path.clone()).collect::<Vec<_>>();
        let current = self.selected.as_deref().and_then(|path| entries.iter().position(|candidate| candidate == path));
        let next = match current {
            Some(index) => index.saturating_add_signed(delta).min(entries.len().checked_sub(1)?),
            None if delta.is_negative() => entries.len().checked_sub(1)?,
            None => 0,
        };
        let target = entries.get(next)?.clone();
        if self.anchor.is_none() {
            self.anchor = Some(entries[current.unwrap_or(next)].clone());
        }
        self.select_range_to(&target, false);
        self.selected.as_deref()
    }

    /// A click on empty space: nothing stays selected.
    pub fn clear_selection(&mut self) -> bool {
        if self.selection.is_empty() && self.selected.is_none() {
            return false;
        }
        self.set_single(None);
        true
    }

    /// Escape: keep only the cursor.
    pub fn collapse_selection(&mut self) -> bool {
        if self.selection.len() <= 1 && self.selected.as_ref().is_none_or(|path| self.selection.contains(path)) {
            return false;
        }
        self.set_single(self.selected.clone());
        true
    }

    pub fn select_next(&mut self) -> Option<&str> {
        self.move_selection(1)
    }

    pub fn select_previous(&mut self) -> Option<&str> {
        self.move_selection(-1)
    }

    pub fn select_parent(&mut self) -> Option<&str> {
        let selected = self.selected.clone()?;
        let parent = parent_path(&selected)?;
        if parent.is_empty() {
            return self.selected.as_deref();
        }
        self.set_single(Some(parent));
        self.selected.as_deref()
    }

    pub fn select_first_child(&mut self) -> Option<&str> {
        let selected = self.selected.clone()?;
        let first = self.nodes.get(&selected)?.children.first()?.clone();
        self.set_single(Some(first));
        self.selected.as_deref()
    }

    pub fn invalidate_all_directories(&mut self) {
        for node in self.nodes.values_mut() {
            if node.entry.kind == EntryKind::Directory {
                node.stale = true;
            }
        }
    }

    /// The folders to open for a search match to show in the tree, outermost first.
    pub fn reveal_search_match(&mut self, result: &SearchMatch) -> Vec<String> {
        let mut ancestors = Vec::new();
        let mut current = parent_path(&result.path);
        while let Some(path) = current {
            if path.is_empty() {
                break;
            }
            ancestors.push(path.clone());
            current = parent_path(&path);
        }
        ancestors.reverse();
        ancestors
    }

    fn move_selection(&mut self, delta: isize) -> Option<&str> {
        let selectable = self.visible_rows.iter().filter(|row| row.selectable()).map(|row| row.path.as_str()).collect::<Vec<_>>();
        if selectable.is_empty() {
            self.set_single(None);
            return None;
        }
        let current = self.selected.as_deref().and_then(|path| selectable.iter().position(|candidate| *candidate == path));
        let next = match (current, delta.is_negative()) {
            (Some(index), true) => index.saturating_sub(delta.unsigned_abs()),
            (Some(index), false) => (index + delta as usize).min(selectable.len() - 1),
            (None, true) => selectable.len() - 1,
            (None, false) => 0,
        };
        self.set_single(Some(selectable[next].to_string()));
        self.selected.as_deref()
    }

    fn set_single(&mut self, path: Option<String>) {
        self.selection.clear();
        self.selection.extend(path.clone());
        self.anchor = path.clone();
        self.selected = path;
    }

    fn remove_subtree(&mut self, path: &str) {
        let children = self.nodes.get(path).map(|node| node.children.clone()).unwrap_or_default();
        for child in children {
            self.remove_subtree(&child);
        }
        self.nodes.remove(path);
        self.expanded.remove(path);
    }

    fn rebuild_visible_rows(&mut self) {
        let mut rows = Vec::new();
        let mut visited = HashSet::from([String::new()]);
        self.append_directory_rows("", 0, &mut rows, &mut visited);
        self.visible_rows = rows;
        if self.selected.as_deref().is_some_and(|selected| !self.visible_rows.iter().any(|row| row.path == selected)) {
            self.selected = None;
        }
        // Like VS Code, entries hidden by a collapse or removal leave the selection.
        let visible = self.visible_rows.iter().filter(|row| row.kind == VisibleRowKind::Entry).map(|row| row.path.as_str()).collect::<HashSet<_>>();
        self.selection.retain(|path| visible.contains(path.as_str()));
        if self.anchor.as_deref().is_some_and(|anchor| !visible.contains(anchor)) {
            self.anchor = None;
        }
    }

    fn append_directory_rows(&self, directory: &str, depth: usize, rows: &mut Vec<VisibleTreeRow>, visited: &mut HashSet<String>) {
        let Some(node) = self.nodes.get(directory) else { return };
        for child_path in &node.children {
            if !visited.insert(child_path.clone()) {
                continue;
            }
            let Some(child) = self.nodes.get(child_path) else { continue };
            rows.push(VisibleTreeRow { path: child_path.clone(), depth, kind: VisibleRowKind::Entry });
            if child.entry.kind == EntryKind::Directory && self.expanded.contains(child_path) {
                self.append_directory_rows(child_path, depth + 1, rows, visited);
            }
        }
        if !self.expanded.contains(directory) {
            return;
        }
        let row = |kind: &str, row: VisibleRowKind| VisibleTreeRow { path: synthetic_path(directory, kind), depth, kind: row };
        match &node.load {
            DirectoryLoadState::Unloaded => {}
            // a refresh keeps what was loaded on screen
            DirectoryLoadState::Loading if node.has_loaded => {
                if node.children.is_empty() {
                    rows.push(row("empty", VisibleRowKind::Empty { directory: directory.to_string() }));
                }
            }
            DirectoryLoadState::Loading => rows.push(row("loading", VisibleRowKind::Loading { directory: directory.to_string() })),
            DirectoryLoadState::Loaded => {
                if node.children.is_empty() {
                    rows.push(row("empty", VisibleRowKind::Empty { directory: directory.to_string() }));
                }
            }
            DirectoryLoadState::Error { message } => rows.push(row("error", VisibleRowKind::Error { directory: directory.to_string(), message: message.clone() })),
        }
    }
}

fn root_entry() -> Entry {
    Entry { path: String::new(), name: String::new(), kind: EntryKind::Directory, ignored: false }
}

fn compare_paths(nodes: &HashMap<String, TreeNode>, left: &str, right: &str) -> std::cmp::Ordering {
    let (left, right) = (&nodes[left].entry, &nodes[right].entry);
    entry_rank(left.kind).cmp(&entry_rank(right.kind)).then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase())).then_with(|| left.path.cmp(&right.path))
}

fn entry_rank(kind: EntryKind) -> u8 {
    match kind {
        EntryKind::Directory => 0,
        EntryKind::File => 1,
        EntryKind::Symlink => 2,
    }
}

pub fn parent_path(path: &str) -> Option<String> {
    path.rsplit_once('/').map(|(parent, _)| parent.to_string()).or_else(|| (!path.is_empty()).then(String::new))
}

fn is_descendant(candidate: &str, ancestor: &str) -> bool {
    if ancestor.is_empty() {
        return !candidate.is_empty();
    }
    candidate.strip_prefix(ancestor).is_some_and(|suffix| suffix.starts_with('/'))
}

fn is_direct_child(candidate: &str, directory: &str) -> bool {
    parent_path(candidate).as_deref() == Some(directory)
}

fn synthetic_path(directory: &str, kind: &str) -> String {
    format!("{directory}\0{kind}")
}

// ---------------------------------------------------------------------------
// Git decorations (Zeron's git_status.rs, over `git status` rather than its watch RPC)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Decoration {
    Untracked,
    Added,
    Modified,
    Renamed,
    Deleted,
    Conflict,
}

impl Decoration {
    pub fn color(self, theme: &crate::theme::Theme) -> gpui::Hsla {
        match self {
            Decoration::Untracked | Decoration::Added => theme.success,
            Decoration::Modified => theme.warning,
            Decoration::Renamed => theme.accent,
            Decoration::Deleted | Decoration::Conflict => theme.danger,
        }
    }
}

/// A porcelain v1 record's `XY`: Zeron's `decoration` over its index and worktree states.
fn decoration(index: u8, worktree: u8) -> Decoration {
    let states = [index, worktree];
    let conflict = states.contains(&b'U') || matches!((index, worktree), (b'A', b'A') | (b'D', b'D'));
    if conflict {
        Decoration::Conflict
    } else if states.contains(&b'D') && states.contains(&b'?') {
        Decoration::Modified
    } else if states.contains(&b'D') {
        Decoration::Deleted
    } else if states.iter().any(|s| matches!(s, b'R' | b'C')) {
        Decoration::Renamed
    } else if states.iter().any(|s| matches!(s, b'M' | b'T')) {
        Decoration::Modified
    } else if states.contains(&b'?') {
        Decoration::Untracked
    } else {
        Decoration::Added
    }
}

fn valid_path(path: &str) -> bool {
    !path.is_empty() && !path.starts_with('/') && path.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
}

/// Each changed file's mark, and each folder's: the strongest of what changed under it.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Decorations {
    files: HashMap<String, Decoration>,
    directories: HashMap<String, Decoration>,
}

impl Decorations {
    /// From `git status --porcelain=v1 -z` at the checkout's root.
    pub fn from_porcelain(status: &[u8]) -> Self {
        let mut result = Self::default();
        let mut records = status.split(|byte| *byte == 0).filter(|record| !record.is_empty());
        while let Some(record) = records.next() {
            if record.len() < 4 || record[2] != b' ' {
                continue;
            }
            let (index, worktree) = (record[0], record[1]);
            let path = String::from_utf8_lossy(&record[3..]).into_owned();
            // a rename's or a copy's old path follows it
            let old = matches!(index, b'R' | b'C').then(|| records.next()).flatten().map(|old| String::from_utf8_lossy(old).into_owned());
            if index == b'!' || !valid_path(&path) {
                continue;
            }
            let value = decoration(index, worktree);
            result.files.insert(path.clone(), value);
            for path in std::iter::once(path.as_str()).chain(old.as_deref()).filter(|p| valid_path(p)) {
                let mut parent = parent_path(path).filter(|p| !p.is_empty());
                while let Some(path) = parent {
                    let entry = result.directories.entry(path.clone()).or_insert(value);
                    *entry = (*entry).max(value);
                    parent = parent_path(&path).filter(|p| !p.is_empty());
                }
            }
        }
        result
    }

    pub fn get(&self, path: &str, directory: bool) -> Option<Decoration> {
        if directory { self.directories.get(path).copied() } else { self.files.get(path).copied() }
    }
}

// ---------------------------------------------------------------------------
// The search results' tree (Zeron's search.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct SearchTreeNode {
    path: String,
    name: String,
    kind: EntryKind,
    score: Option<i64>,
    best_score: i64,
    children: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchTreeRow {
    pub path: String,
    pub name: String,
    pub kind: EntryKind,
    pub depth: usize,
    pub has_children: bool,
    pub score: i64,
}

impl SearchTreeRow {
    pub fn as_match(&self) -> SearchMatch {
        SearchMatch { path: self.path.clone(), name: self.name.clone(), kind: self.kind, score: self.score }
    }
}

/// Matches grouped under their real folders, the best scored first.
#[derive(Debug, Default)]
pub struct SearchTreeModel {
    nodes: HashMap<String, SearchTreeNode>,
    roots: Vec<String>,
    collapsed: HashSet<String>,
    rows: Vec<SearchTreeRow>,
}

impl SearchTreeModel {
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.roots.clear();
        self.collapsed.clear();
        self.rows.clear();
    }

    pub fn rebuild(&mut self, results: &[SearchMatch]) {
        self.clear();
        for result in results {
            let mut parent: Option<String> = None;
            let mut path = String::new();
            for component in result.path.split('/').filter(|component| !component.is_empty()) {
                if !path.is_empty() {
                    path.push('/');
                }
                path.push_str(component);
                let is_match = path == result.path;
                self.nodes
                    .entry(path.clone())
                    .and_modify(|node| {
                        if is_match {
                            node.name = result.name.clone();
                            node.kind = result.kind;
                            node.score = Some(node.score.unwrap_or(i64::MIN).max(result.score));
                        }
                    })
                    .or_insert_with(|| SearchTreeNode {
                        path: path.clone(),
                        name: if is_match { result.name.clone() } else { component.to_string() },
                        kind: if is_match { result.kind } else { EntryKind::Directory },
                        score: is_match.then_some(result.score),
                        best_score: result.score,
                        children: Vec::new(),
                    });
                if let Some(parent) = parent.as_ref() {
                    let children = &mut self.nodes.get_mut(parent).expect("search ancestor must exist").children;
                    if !children.contains(&path) {
                        children.push(path.clone());
                    }
                } else if !self.roots.contains(&path) {
                    self.roots.push(path.clone());
                }
                parent = Some(path.clone());
            }
        }
        let roots = self.roots.clone();
        for root in &roots {
            update_best_score(root, &mut self.nodes);
        }
        sort_search_paths(&mut self.roots, &self.nodes);
        for root in roots {
            sort_search_branch(&root, &mut self.nodes);
        }
        self.rebuild_rows();
    }

    pub fn rows(&self) -> &[SearchTreeRow] {
        &self.rows
    }

    pub fn toggle(&mut self, path: &str) -> bool {
        if !self.nodes.get(path).is_some_and(|node| !node.children.is_empty()) {
            return false;
        }
        if !self.collapsed.remove(path) {
            self.collapsed.insert(path.to_string());
        }
        self.rebuild_rows();
        true
    }

    pub fn is_expanded(&self, path: &str) -> bool {
        !self.collapsed.contains(path)
    }

    fn rebuild_rows(&mut self) {
        let mut rows = Vec::new();
        for root in &self.roots {
            append_search_rows(root, 0, &self.nodes, &self.collapsed, &mut rows);
        }
        self.rows = rows;
    }
}

fn update_best_score(path: &str, nodes: &mut HashMap<String, SearchTreeNode>) -> i64 {
    let Some(node) = nodes.get(path) else { return i64::MIN };
    let children = node.children.clone();
    let mut best_score = node.score.unwrap_or(i64::MIN);
    for child in children {
        best_score = best_score.max(update_best_score(&child, nodes));
    }
    if let Some(node) = nodes.get_mut(path) {
        node.best_score = best_score;
    }
    best_score
}

fn sort_search_branch(path: &str, nodes: &mut HashMap<String, SearchTreeNode>) {
    let mut children = nodes.get(path).map(|node| node.children.clone()).unwrap_or_default();
    for child in &children {
        sort_search_branch(child, nodes);
    }
    sort_search_paths(&mut children, nodes);
    if let Some(node) = nodes.get_mut(path) {
        node.children = children;
    }
}

fn sort_search_paths(paths: &mut [String], nodes: &HashMap<String, SearchTreeNode>) {
    paths.sort_by(|left, right| {
        let (left, right) = (&nodes[left], &nodes[right]);
        right
            .best_score
            .cmp(&left.best_score)
            .then_with(|| (right.kind == EntryKind::Directory).cmp(&(left.kind == EntryKind::Directory)))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.path.cmp(&right.path))
    });
}

fn append_search_rows(path: &str, depth: usize, nodes: &HashMap<String, SearchTreeNode>, collapsed: &HashSet<String>, rows: &mut Vec<SearchTreeRow>) {
    let Some(node) = nodes.get(path) else { return };
    let has_children = !node.children.is_empty();
    rows.push(SearchTreeRow { path: node.path.clone(), name: node.name.clone(), kind: node.kind, depth, has_children, score: node.score.unwrap_or(node.best_score) });
    if has_children && !collapsed.contains(path) {
        for child in &node.children {
            append_search_rows(child, depth + 1, nodes, collapsed, rows);
        }
    }
}

/// The best `limit` of `candidates` for `query`, Zeron's `search_including_ignored` ranking: a name starting with the
/// query first, then a name holding it, then a path holding it; shorter paths first. Folders are the files' own.
pub fn search(candidates: &[(String, EntryKind)], query: &str, limit: usize) -> Vec<SearchMatch> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let mut best: Vec<SearchMatch> = candidates
        .iter()
        .filter_map(|(path, kind)| {
            let name = path.rsplit('/').next().unwrap_or(path);
            let lower = name.to_lowercase();
            let score = if lower.starts_with(&query) {
                2
            } else if lower.contains(&query) {
                1
            } else if path.to_lowercase().contains(&query) {
                0
            } else {
                return None;
            };
            Some(SearchMatch { path: path.clone(), name: name.to_string(), kind: *kind, score })
        })
        .collect();
    best.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.len().cmp(&b.path.len())).then_with(|| a.path.cmp(&b.path)));
    best.truncate(limit);
    best
}

/// The files `git ls-files` printed (`-z`), and every folder above them, once each.
pub fn search_candidates(listing: &[u8]) -> Vec<(String, EntryKind)> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for path in listing.split(|b| *b == 0).filter(|p| !p.is_empty()) {
        let path = String::from_utf8_lossy(path);
        // an untracked repo inside is listed as its folder
        let path = path.trim_end_matches('/');
        if !valid_path(path) || !seen.insert(path.to_string()) {
            continue;
        }
        let mut parent = parent_path(path).filter(|p| !p.is_empty());
        while let Some(dir) = parent {
            if !seen.insert(dir.clone()) {
                break;
            }
            parent = parent_path(&dir).filter(|p| !p.is_empty());
            out.push((dir, EntryKind::Directory));
        }
        out.push((path.to_string(), EntryKind::File));
    }
    out
}

// ---------------------------------------------------------------------------
// The checkout, read: here from the disk, a machine's over SSH
// ---------------------------------------------------------------------------

/// The checkout a tree shows: its root, located as Shepherd keeps directories (a machine's is that machine's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// The shown pane's directory, located: git reaches it here or over SSH.
    pub dir: String,
    /// The checkout's root on its machine.
    pub root: String,
}

fn run(place: &Place, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut all = vec!["-C", place.root.as_str()];
    all.extend_from_slice(args);
    let out = crate::herdr::git(&place.dir, &all).map_err(|e| e.to_string())?;
    match out.status.success() {
        true => Ok(out.stdout),
        false => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
    }
}

impl Place {
    /// The checkout `dir` (located) is in: its worktree's root, or `dir` itself outside git.
    pub fn of(dir: &str) -> Place {
        let top = crate::herdr::git(dir, &["rev-parse", "--show-toplevel"]).ok().filter(|out| out.status.success()).map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string());
        let root = top.filter(|t| !t.is_empty()).unwrap_or_else(|| crate::herdr::split(dir).1.to_string());
        Place { dir: dir.to_string(), root }
    }

    pub fn machine(&self) -> Option<&str> {
        crate::herdr::split(&self.dir).0
    }

    /// `path`'s full path on its machine.
    pub fn full(&self, path: &str) -> std::path::PathBuf {
        let root = std::path::Path::new(&self.root);
        if path.is_empty() { root.to_path_buf() } else { root.join(path) }
    }

    /// Git's marks for the checkout; `None` outside git.
    pub fn status(&self) -> Option<Vec<u8>> {
        run(self, &["--no-optional-locks", "status", "--porcelain=v1", "-z", "--untracked-files=all"]).ok()
    }

    /// The blob id `path` would have now: a cheap look at whether the preview's file changed.
    pub fn hash(&self, path: &str) -> Option<String> {
        run(self, &["hash-object", "--", path]).ok().map(|out| String::from_utf8_lossy(&out).trim().to_string())
    }

    /// Every file git knows of or sees, ignored ones too with `include_ignored`, for the name search.
    pub fn files(&self, include_ignored: bool) -> Result<Vec<(String, EntryKind)>, String> {
        let mut args = vec!["-c", "core.quotePath=false", "ls-files", "-z", "--cached", "--others"];
        if !include_ignored {
            args.push("--exclude-standard");
        }
        run(self, &args).map(|listing| search_candidates(&listing))
    }

    /// `directory`'s entries, the ignored ones marked (and left out without `include_ignored`), `.git` never.
    pub fn list(&self, directory: &str, include_ignored: bool) -> Result<DirectoryPage, String> {
        if !directory.is_empty() && !crate::changes::safe_path(directory) {
            return Err("Not a folder of this checkout".into());
        }
        let prefix = if directory.is_empty() { String::new() } else { format!("{directory}/") };
        let (listed, ignored) = match self.machine() {
            None => {
                let mut listed = Vec::new();
                for item in std::fs::read_dir(self.full(directory)).map_err(|e| e.to_string())?.flatten().take(MAX_DIRECTORY_ENTRIES) {
                    let name = item.file_name().to_string_lossy().into_owned();
                    let Ok(kind) = item.file_type() else { continue };
                    let kind = if kind.is_symlink() {
                        EntryKind::Symlink
                    } else if kind.is_dir() {
                        EntryKind::Directory
                    } else {
                        EntryKind::File
                    };
                    listed.push((name, kind));
                }
                listed.retain(|(name, _)| name != ".git");
                let paths: Vec<String> = listed.iter().map(|(name, _)| format!("{prefix}{name}")).collect();
                (listed, self.check_ignore(&paths))
            }
            Some(target) => {
                let out = crate::herdr::ssh(target, &list_script(&self.root, directory)).map_err(|e| e.to_string())?;
                if !out.status.success() {
                    let error = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    return Err(if error.is_empty() { "Couldn't list this folder".into() } else { error });
                }
                parse_listing(&out.stdout)
            }
        };
        let entries = listed
            .into_iter()
            .map(|(name, kind)| {
                let path = format!("{prefix}{name}");
                Entry { ignored: ignored.contains(&path), path, name, kind }
            })
            .filter(|entry| include_ignored || !entry.ignored)
            .collect();
        Ok(DirectoryPage { directory: directory.to_string(), entries })
    }

    /// Which of `paths` git ignores; none outside git.
    fn check_ignore(&self, paths: &[String]) -> HashSet<String> {
        use std::io::Write;
        let child = crate::process::command("git")
            .arg("-C")
            .arg(&self.root)
            .args(["check-ignore", "-z", "--stdin"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn();
        let Ok(mut child) = child else { return HashSet::new() };
        if let Some(mut stdin) = child.stdin.take() {
            // ponytail: written whole before reading: git answers a path at a time, a pipe's buffer holds a folder's
            let input: Vec<u8> = paths.iter().flat_map(|p| p.bytes().chain([0])).collect();
            let _ = std::thread::spawn(move || stdin.write_all(&input));
        }
        let out = child.wait_with_output().map(|out| out.stdout).unwrap_or_default();
        out.split(|b| *b == 0).filter(|p| !p.is_empty()).map(|p| String::from_utf8_lossy(p).into_owned()).collect()
    }

    /// `path` for the preview: its text, at most [`MAX_PREVIEW_BYTES`] of it.
    pub fn read(&self, path: &str) -> Result<FileText, String> {
        if !crate::changes::safe_path(path) {
            return Err("Not a file of this checkout".into());
        }
        let limit = MAX_PREVIEW_BYTES + 1;
        let bytes = match self.machine() {
            None => {
                use std::io::Read;
                let root = std::fs::canonicalize(&self.root).map_err(|e| e.to_string())?;
                let full = std::fs::canonicalize(self.full(path)).map_err(|e| e.to_string())?;
                // a link may point anywhere: only what's inside the checkout is shown
                if !full.starts_with(&root) {
                    return Err("A link out of the checkout isn't previewed".into());
                }
                let mut bytes = Vec::new();
                std::fs::File::open(&full).map_err(|e| e.to_string())?.take(limit as u64).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
                bytes
            }
            Some(target) => {
                let (root, path) = (crate::herdr::quote(&self.root), crate::herdr::quote(path));
                let script = format!("cd {root} && if [ -L {path} ]; then echo 'A link isn'\\''t previewed on a machine' >&2; exit 1; fi && head -c {limit} -- {path}");
                let out = crate::herdr::ssh(target, &script).map_err(|e| e.to_string())?;
                if !out.status.success() {
                    return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
                }
                out.stdout
            }
        };
        Ok(FileText::of(bytes))
    }
}

/// Why a save didn't happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    /// The file changed on disk since it was read: someone else's edit isn't overwritten unasked.
    Changed,
    Failed(String),
}

impl Place {
    /// `text` as `path`'s whole content, unless the file moved off `expected` (its blob id when read); `None` writes
    /// over whatever is there. Its new blob id back.
    // ponytail: written in place, not to a temporary file renamed over it (Zeron's atomic replace); that if a crash
    // mid-write ever costs a file.
    pub fn write(&self, path: &str, text: &str, expected: Option<&str>) -> Result<Option<String>, SaveError> {
        if !crate::changes::safe_path(path) {
            return Err(SaveError::Failed("Not a file of this checkout".into()));
        }
        if let Some(expected) = expected
            && self.hash(path).is_some_and(|now| now != expected)
        {
            return Err(SaveError::Changed);
        }
        let failed = |e: std::io::Error| SaveError::Failed(e.to_string());
        match self.machine() {
            None => {
                let root = std::fs::canonicalize(&self.root).map_err(failed)?;
                let full = self.full(path);
                let parent = std::fs::canonicalize(full.parent().unwrap_or(&root)).map_err(failed)?;
                let mut target = parent.join(full.file_name().unwrap_or_default());
                if target.is_symlink() {
                    target = std::fs::canonicalize(&target).map_err(failed)?;
                }
                // a link may point anywhere: only what's inside the checkout is written
                if !target.starts_with(&root) {
                    return Err(SaveError::Failed("A link out of the checkout isn't written".into()));
                }
                std::fs::write(&target, text).map_err(failed)?;
            }
            Some(target) => {
                let (root, path) = (crate::herdr::quote(&self.root), crate::herdr::quote(path));
                let script = format!("cd {root} && if [ -L {path} ]; then echo 'A link isn'\\''t written on a machine' >&2; exit 1; fi && cat > {path}");
                let out = crate::herdr::ssh_input(target, &script, text.as_bytes()).map_err(failed)?;
                if !out.status.success() {
                    return Err(SaveError::Failed(String::from_utf8_lossy(&out.stderr).trim().to_string()));
                }
            }
        }
        Ok(self.hash(path))
    }
}

/// What the preview shows of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileText {
    Text { text: String, truncated: bool },
    Binary,
}

impl FileText {
    /// Read bytes, possibly one past [`MAX_PREVIEW_BYTES`]: cut there at a line, or binary (a NUL in its first 8 KiB).
    pub fn of(mut bytes: Vec<u8>) -> FileText {
        if bytes[..bytes.len().min(8192)].contains(&0) {
            return FileText::Binary;
        }
        let truncated = bytes.len() > MAX_PREVIEW_BYTES;
        if truncated {
            let end = bytes[..MAX_PREVIEW_BYTES].iter().rposition(|b| *b == b'\n').unwrap_or(MAX_PREVIEW_BYTES);
            bytes.truncate(end);
        }
        FileText::Text { text: String::from_utf8_lossy(&bytes).into_owned(), truncated }
    }
}

/// A machine's folder as one `sh` lists it: `d`, `l` or `f` and the name per entry, an empty record, then the paths git
/// ignores (from `git check-ignore`, quiet outside a repo), all NUL-separated.
fn list_script(root: &str, directory: &str) -> String {
    let (root, dir) = (crate::herdr::quote(root), crate::herdr::quote(if directory.is_empty() { "." } else { directory }));
    let prefix = crate::herdr::quote(&if directory.is_empty() { String::new() } else { format!("{directory}/") });
    format!(
        "cd {root} && cd {dir} || exit 1; p={prefix}; \
         for f in .* *; do case $f in .|..|.git) continue;; esac; \
         if [ -L \"$f\" ]; then t=l; elif [ -d \"$f\" ]; then t=d; elif [ -e \"$f\" ]; then t=f; else continue; fi; printf '%s%s\\0' \"$t\" \"$f\"; done; \
         printf '\\0'; \
         for f in .* *; do case $f in .|..|.git) continue;; esac; if [ -e \"$f\" ] || [ -L \"$f\" ]; then printf '%s%s\\0' \"$p\" \"$f\"; fi; done \
         | git -C {root} check-ignore -z --stdin 2>/dev/null; exit 0"
    )
}

fn parse_listing(out: &[u8]) -> (Vec<(String, EntryKind)>, HashSet<String>) {
    let mut records = out.split(|b| *b == 0);
    let mut listed = Vec::new();
    for record in records.by_ref() {
        let Some((kind, name)) = record.split_first() else { break };
        let kind = match kind {
            b'd' => EntryKind::Directory,
            b'l' => EntryKind::Symlink,
            _ => EntryKind::File,
        };
        listed.push((String::from_utf8_lossy(name).into_owned(), kind));
    }
    let ignored = records.filter(|p| !p.is_empty()).map(|p| String::from_utf8_lossy(p).into_owned()).collect();
    (listed, ignored)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, kind: EntryKind) -> Entry {
        Entry { path: path.into(), name: path.rsplit('/').next().unwrap_or(path).into(), kind, ignored: false }
    }

    fn page(directory: &str, entries: Vec<Entry>) -> DirectoryPage {
        DirectoryPage { directory: directory.into(), entries }
    }

    fn entries(tree: &FileTreeModel) -> Vec<&str> {
        tree.visible_rows().iter().filter(|row| row.kind == VisibleRowKind::Entry).map(|row| row.path.as_str()).collect()
    }

    #[test]
    fn root_page_is_sorted_and_flattened() {
        let mut tree = FileTreeModel::default();
        tree.begin_load("", 0);
        tree.apply_page(page("", vec![entry("z.rs", EntryKind::File), entry("src", EntryKind::Directory), entry("a.rs", EntryKind::File), entry("link", EntryKind::Symlink)]), 0);
        assert_eq!(entries(&tree), ["src", "a.rs", "z.rs", "link"]);
    }

    #[test]
    fn background_refresh_keeps_loaded_rows_even_when_empty_or_failed() {
        let mut tree = FileTreeModel::default();
        tree.apply_page(page("", vec![]), 0);
        let empty = tree.visible_rows().to_vec();
        tree.begin_load("", 0);
        assert_eq!(tree.visible_rows(), empty);
        tree.apply_page(page("", vec![entry("src", EntryKind::Directory)]), 0);
        let loaded = tree.visible_rows().to_vec();
        tree.begin_load("", 0);
        assert_eq!(tree.visible_rows(), loaded);
        tree.fail_load("", "offline", 0);
        assert_eq!(tree.visible_rows()[0].path, "src");
        assert!(tree.node("").unwrap().has_loaded);
    }

    #[test]
    fn refreshing_a_directory_preserves_descendants_expansion_and_selection() {
        let mut tree = FileTreeModel::default();
        tree.apply_page(page("", vec![entry("src", EntryKind::Directory)]), 0);
        tree.expand("src");
        tree.apply_page(page("src", vec![entry("src/lib.rs", EntryKind::File)]), 0);
        tree.select("src/lib.rs");
        tree.begin_load("", 0);
        tree.apply_page(page("", vec![entry("src", EntryKind::Directory)]), 0);
        assert!(tree.is_expanded("src"));
        assert!(tree.is_directory_loaded("src"));
        assert_eq!(tree.selected(), Some("src/lib.rs"));
    }

    #[test]
    fn refresh_removes_deleted_subtrees_and_handles_directory_becoming_file() {
        let mut tree = FileTreeModel::default();
        tree.apply_page(page("", vec![entry("src", EntryKind::Directory)]), 0);
        tree.expand("src");
        tree.apply_page(page("src", vec![entry("src/lib.rs", EntryKind::File)]), 0);
        tree.select("src/lib.rs");
        tree.apply_page(page("", vec![entry("src", EntryKind::File)]), 0);
        assert!(tree.node("src/lib.rs").is_none());
        assert!(!tree.is_expanded("src"));
        assert_eq!(tree.selected(), None);
        tree.apply_page(page("", vec![]), 0);
        assert!(tree.node("src").is_none());
    }

    #[test]
    fn expanded_directories_add_rows_at_the_correct_depth_and_a_load_shows_its_state() {
        let mut tree = FileTreeModel::default();
        tree.apply_page(page("", vec![entry("src", EntryKind::Directory), entry("README.md", EntryKind::File)]), 0);
        tree.toggle_expanded("src");
        tree.begin_load("src", 0);
        assert!(matches!(tree.visible_rows()[1].kind, VisibleRowKind::Loading { .. }));
        tree.apply_page(page("src", vec![]), 0);
        assert!(matches!(tree.visible_rows()[1].kind, VisibleRowKind::Empty { .. }));
        tree.apply_page(page("src", vec![entry("src/main.rs", EntryKind::File)]), 0);
        let rows: Vec<_> = tree.visible_rows().iter().map(|row| (row.path.as_str(), row.depth)).collect();
        assert_eq!(rows, [("src", 0), ("src/main.rs", 1), ("README.md", 0)]);
        tree.fail_load("src", "gone", 0);
        assert!(matches!(&tree.visible_rows()[2].kind, VisibleRowKind::Error { message, .. } if message == "gone"));
    }

    #[test]
    fn ignored_directories_propagate_ignored_state_to_all_descendants() {
        let mut tree = FileTreeModel::with_include_ignored(true);
        tree.apply_page(page("", vec![Entry { ignored: true, ..entry("target", EntryKind::Directory) }]), 0);
        tree.expand("target");
        tree.apply_page(page("target", vec![entry("target/debug", EntryKind::Directory)]), 0);
        assert!(tree.node("target/debug").unwrap().entry.ignored);
    }

    #[test]
    fn collapsing_an_ancestor_moves_hidden_selection_to_it() {
        let mut tree = FileTreeModel::default();
        tree.apply_page(page("", vec![entry("src", EntryKind::Directory)]), 0);
        tree.expand("src");
        tree.apply_page(page("src", vec![entry("src/lib.rs", EntryKind::File)]), 0);
        tree.select("src/lib.rs");
        tree.toggle_expanded("src");
        assert_eq!(tree.selected(), Some("src"));
        assert_eq!(tree.selected_paths(), ["src"]);
    }

    #[test]
    fn stale_generation_cannot_replace_the_tree() {
        let mut tree = FileTreeModel::default();
        let old = tree.generation();
        tree.reset();
        assert!(!tree.apply_page(page("", vec![entry("a", EntryKind::File)]), old));
        assert!(tree.visible_rows().is_empty());
    }

    #[test]
    fn keyboard_selection_and_ranges_follow_the_visible_rows() {
        let mut tree = FileTreeModel::default();
        tree.apply_page(page("", vec![entry("folder", EntryKind::Directory), entry("a.txt", EntryKind::File), entry("b.txt", EntryKind::File), entry("c.txt", EntryKind::File)]), 0);
        assert_eq!(tree.select_next(), Some("folder"));
        assert_eq!(tree.select_next(), Some("a.txt"));
        assert_eq!(tree.select_previous(), Some("folder"));
        tree.select("a.txt");
        tree.toggle_selected("c.txt");
        assert_eq!(tree.selected_paths(), ["a.txt", "c.txt"]);
        tree.select_range_to("folder", true);
        assert_eq!(tree.selected_paths(), ["folder", "a.txt", "b.txt", "c.txt"]);
        assert!(tree.collapse_selection());
        assert_eq!(tree.selected_paths(), ["folder"]);
        tree.extend_selection(1);
        tree.extend_selection(1);
        assert_eq!(tree.selected_paths(), ["folder", "a.txt", "b.txt"]);
        assert!(tree.clear_selection());
        assert_eq!(tree.selected(), None);
    }

    #[test]
    fn decorations_mark_files_and_the_strongest_change_under_a_folder() {
        let status = b" M src/main.rs\0?? src/new.rs\0UU src/x/conflict.rs\0R  docs/new.md\0docs/old.md\0D  gone.txt\0!! target/\0";
        let marks = Decorations::from_porcelain(status);
        assert_eq!(marks.get("src/main.rs", false), Some(Decoration::Modified));
        assert_eq!(marks.get("src/new.rs", false), Some(Decoration::Untracked));
        assert_eq!(marks.get("docs/new.md", false), Some(Decoration::Renamed));
        assert_eq!(marks.get("gone.txt", false), Some(Decoration::Deleted));
        assert_eq!(marks.get("src", true), Some(Decoration::Conflict));
        assert_eq!(marks.get("src/x", true), Some(Decoration::Conflict));
        assert_eq!(marks.get("docs", true), Some(Decoration::Renamed));
        assert_eq!(marks.get("target", true), None);
        assert_eq!(marks.get("src", false), None);
        assert_eq!(decoration(b'A', b' '), Decoration::Added);
        assert_eq!(decoration(b'D', b'?'), Decoration::Modified);
    }

    #[test]
    fn search_ranks_name_prefixes_then_names_then_paths_and_groups_under_folders() {
        let candidates = search_candidates(b"src/main.rs\0src/domain/model.rs\0README.md\0docs/main-ideas.md\0");
        assert!(candidates.contains(&("src/domain".into(), EntryKind::Directory)));
        let found = search(&candidates, "MAIN", 10);
        let paths: Vec<_> = found.iter().map(|m| (m.path.as_str(), m.score)).collect();
        assert_eq!(paths, [("src/main.rs", 2), ("docs/main-ideas.md", 2), ("src/domain", 1), ("src/domain/model.rs", 0)]);
        let mut tree = SearchTreeModel::default();
        tree.rebuild(&found);
        let rows: Vec<_> = tree.rows().iter().map(|r| (r.path.as_str(), r.depth)).collect();
        assert_eq!(rows, [("docs", 0), ("docs/main-ideas.md", 1), ("src", 0), ("src/main.rs", 1), ("src/domain", 1), ("src/domain/model.rs", 2)]);
        assert!(tree.toggle("src"));
        assert_eq!(tree.rows().len(), 3);
        assert!(search(&candidates, "  ", 10).is_empty());
    }

    #[test]
    fn a_text_is_cut_at_a_line_and_a_nul_makes_it_binary() {
        assert_eq!(FileText::of(b"a\0b".to_vec()), FileText::Binary);
        let mut big = b"line\n".repeat(MAX_PREVIEW_BYTES / 5 + 10);
        big.truncate(MAX_PREVIEW_BYTES + 1);
        let FileText::Text { text, truncated } = FileText::of(big) else { panic!() };
        assert!(truncated && text.ends_with("line") && text.len() <= MAX_PREVIEW_BYTES);
    }

    fn sh(dir: &std::path::Path, script: &str) {
        let out = std::process::Command::new("sh").arg("-c").arg(script).current_dir(dir).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }

    fn checkout() -> (tempfile::TempDir, Place) {
        let repo = tempfile::tempdir().unwrap();
        sh(repo.path(), "git init -q && printf 'target/\\n' > .gitignore && mkdir -p src target/debug && echo fn > src/main.rs && echo x > target/debug/out && echo a > .env && git add -A && git -c user.name=t -c user.email=t@t commit -qm init && echo changed > src/main.rs && echo new > new.txt");
        let root = std::fs::canonicalize(repo.path()).unwrap().to_string_lossy().into_owned();
        let place = Place::of(&format!("{root}/src"));
        assert_eq!(place.root, root);
        (repo, place)
    }

    fn names(page: &DirectoryPage) -> Vec<(&str, EntryKind, bool)> {
        page.entries.iter().map(|e| (e.path.as_str(), e.kind, e.ignored)).collect()
    }

    #[test]
    fn a_local_folder_lists_from_the_disk_with_git_marking_what_it_ignores() {
        let (_repo, place) = checkout();
        let mut shown = place.list("", false).unwrap();
        shown.entries.sort_by(|a, b| a.path.cmp(&b.path));
        assert_eq!(names(&shown), [(".env", EntryKind::File, false), (".gitignore", EntryKind::File, false), ("new.txt", EntryKind::File, false), ("src", EntryKind::Directory, false)]);
        let all = place.list("", true).unwrap();
        assert!(all.entries.iter().any(|e| e.path == "target" && e.ignored));
        assert_eq!(names(&place.list("src", false).unwrap()), [("src/main.rs", EntryKind::File, false)]);
        assert!(place.list("../", false).is_err());
        let marks = Decorations::from_porcelain(&place.status().unwrap());
        assert_eq!(marks.get("src/main.rs", false), Some(Decoration::Modified));
        assert_eq!(marks.get("new.txt", false), Some(Decoration::Untracked));
        assert_eq!(place.read("src/main.rs"), Ok(FileText::Text { text: "changed\n".into(), truncated: false }));
        assert!(place.read("../x").is_err());
        let files = place.files(false).unwrap();
        assert!(files.contains(&("new.txt".into(), EntryKind::File)) && !files.iter().any(|(p, _)| p.starts_with("target")));
        assert!(place.files(true).unwrap().contains(&("target/debug/out".into(), EntryKind::File)));
    }

    #[test]
    fn a_save_writes_over_what_was_read_and_refuses_a_file_changed_since() {
        let (repo, place) = checkout();
        let read = place.hash("src/main.rs").unwrap();
        let saved = place.write("src/main.rs", "fn main() {}\n", Some(&read)).unwrap().unwrap();
        assert_ne!(saved, read);
        assert_eq!(std::fs::read_to_string(repo.path().join("src/main.rs")).unwrap(), "fn main() {}\n");
        // someone else's edit after the read: the stale save is refused, a forced one goes through
        std::fs::write(repo.path().join("src/main.rs"), "theirs\n").unwrap();
        assert_eq!(place.write("src/main.rs", "mine\n", Some(&saved)), Err(SaveError::Changed));
        assert_eq!(std::fs::read_to_string(repo.path().join("src/main.rs")).unwrap(), "theirs\n");
        assert!(place.write("src/main.rs", "mine\n", None).is_ok());
        assert!(matches!(place.write("../escape.txt", "x", None), Err(SaveError::Failed(_))));
    }

    #[test]
    fn the_machine_script_lists_and_marks_as_the_disk_does() {
        let (_repo, place) = checkout();
        let out = std::process::Command::new("sh").arg("-c").arg(list_script(&place.root, "")).output().unwrap();
        let (mut listed, ignored) = parse_listing(&out.stdout);
        listed.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(listed.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), [".env", ".gitignore", "new.txt", "src", "target"]);
        assert!(listed.contains(&("src".into(), EntryKind::Directory)));
        assert_eq!(ignored, HashSet::from(["target".to_string()]));
        let out = std::process::Command::new("sh").arg("-c").arg(list_script(&place.root, "target")).output().unwrap();
        let (listed, ignored) = parse_listing(&out.stdout);
        assert_eq!(listed, [("debug".to_string(), EntryKind::Directory)]);
        assert_eq!(ignored, HashSet::from(["target/debug".to_string()]));
    }
}
