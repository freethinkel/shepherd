//! The right panel's diffs, Zeron's `crates/ui/src/changes.rs` (MIT): a unified or split diff viewer over `git`.
//! The patch, its rows and the capture are `crate::changes`; this is the entity and what it draws, with Zeron's fold
//! tween, sticky headers, syntax runs and comment cards. Its watch stream is a poll here, while the tab is shown. A
//! comment goes from the panel's footer to the shown pane's agent, there being no composer to stage it on.
use super::review::ShownReview;
use super::*;
use crate::changes::*;
use crate::right_panel::Surface;
use crate::ui::surface_chrome::{CONTROL_GAP, CONTROL_RADIUS, CONTROL_SIZE, HEADER_HEIGHT, ICON_SIZE};
use gpui::{AnyElement, ListAlignment, ListState, Task, font, list};
use gpui_base::input::{Textarea, TextareaState};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

const DIFF_TEXT_SIZE: f32 = 13.0;
/// The marker column between the gutters and the code.
const MARKER_WIDTH: f32 = 28.0;
/// Width of the coloured accent bar on the left edge of +/− rows.
const ACCENT_BAR_WIDTH: f32 = 3.0;
/// The marker column in split mode: each half pays for its own, so it is narrower than [`MARKER_WIDTH`].
const SPLIT_MARKER_WIDTH: f32 = 18.0;
/// Hairline between the two split columns.
const SPLIT_DIVIDER_WIDTH: f32 = 1.0;
const UNIFIED_CODE_PADDING_LEFT: f32 = 12.0;
const SPLIT_CODE_PADDING_LEFT: f32 = 6.0;
/// Breathing room after the widest source line when scrolled fully right.
const CODE_PADDING_RIGHT: f32 = 24.0;
const COMMENT_ADDER_SIZE: f32 = 16.0;
/// Between a capture's end and the next, while the tab is shown in a window in use.
const POLL: Duration = Duration::from_secs(2);
/// Between looks at whether a running turn has changed anything yet.
const TURN_POLL: Duration = Duration::from_secs(3);
/// Tween arming window after a fold toggle (the fold's 180 ms plus margin).
const FOLD_TWEEN_WINDOW: Duration = Duration::from_millis(400);
/// Ceiling on how much body a fold tween's stand-in row materializes. A tween always starts from a clicked (on-screen)
/// header, so the revealable slice is at most one viewport tall.
const FOLD_TWEEN_MAX_PX: f32 = 2400.0;

fn hash64(parts: &[&str]) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    parts.hash(&mut hasher);
    hasher.finish()
}

/// Measure the same runs the row paints. Color boundaries can break kerning and ligatures on native platforms even when
/// every run uses the same font.
fn max_shaped_text_width(file: &FileDiff, highlights: Option<&DiffHighlights>, theme: &Theme, text_system: &gpui::WindowTextSystem) -> f32 {
    let mono = font(theme.font_terminal.clone());
    let size = px(DIFF_TEXT_SIZE);
    let column_width = f32::from(text_system.ch_advance(text_system.resolve_font(&mono), size).unwrap_or(size * 0.6));
    file.hunks.iter().flat_map(|hunk| &hunk.lines).fold(0.0f32, |widest, line| {
        let runs = line_runs(line, highlights, theme);
        let shaped = f32::from(text_system.shape_line(line.text.clone().into(), size, &runs, None).width());
        // Preserve the old column estimate as a floor, including tab stops.
        widest.max(shaped).max(visual_columns(&line.text) as f32 * column_width)
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DiffHorizontalMetrics {
    max_text_width: f32,
    max_gutter_width: f32,
}

impl DiffHorizontalMetrics {
    /// Compensating for the file-local gutter keeps every unified code viewport's effective scroll range identical.
    fn unified_content_width(self, gutter_width: f32) -> f32 {
        self.max_text_width + UNIFIED_CODE_PADDING_LEFT + CODE_PADDING_RIGHT + 2.0 * (self.max_gutter_width - gutter_width)
    }

    /// Split has one gutter per half. Both halves use this same extent so old and new remain synchronized even when one
    /// side is a filler.
    fn split_content_width(self, gutter_width: f32) -> f32 {
        self.max_text_width + SPLIT_CODE_PADDING_LEFT + CODE_PADDING_RIGHT + (self.max_gutter_width - gutter_width)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DiffCodeWidth {
    /// Rows expose a stable intrinsic code width.
    Scrollable(DiffHorizontalMetrics),
    /// Rows consume their viewport width and grow vertically.
    Wrapped,
}

#[derive(Clone)]
struct DiffCodeScroll {
    handle: gpui::ScrollHandle,
    id: SharedString,
}

#[derive(Clone)]
struct DiffCodeScrollContext {
    handle: gpui::ScrollHandle,
    prefix: SharedString,
}

impl DiffCodeScrollContext {
    fn slot(&self, suffix: impl std::fmt::Display) -> DiffCodeScroll {
        DiffCodeScroll { handle: self.handle.clone(), id: SharedString::from(format!("{}-{suffix}", self.prefix)) }
    }
}

struct MeasuredDiffWidth {
    font: SharedString,
    // Retaining the Arc makes pointer identity safe against allocator reuse.
    highlights: Option<Arc<DiffHighlights>>,
    width: f32,
}

struct FileHorizontalState {
    max_gutter_width: f32,
    scroll: gpui::ScrollHandle,
    /// Shape once per file, font, and highlight revision.
    measured: std::cell::RefCell<Option<MeasuredDiffWidth>>,
}

impl FileHorizontalState {
    fn new(file: &FileDiff) -> Self {
        Self { max_gutter_width: gutter_width(file), scroll: gpui::ScrollHandle::new(), measured: std::cell::RefCell::new(None) }
    }

    fn metrics(&self, file: &FileDiff, highlights: Option<&Arc<DiffHighlights>>, theme: &Theme, text_system: &gpui::WindowTextSystem) -> DiffHorizontalMetrics {
        let mut cached = self.measured.borrow_mut();
        let current = cached.as_ref().is_some_and(|cached| {
            cached.font == theme.font_terminal
                && match (cached.highlights.as_ref(), highlights) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                    _ => false,
                }
        });
        if !current {
            *cached = Some(MeasuredDiffWidth {
                font: theme.font_terminal.clone(),
                highlights: highlights.cloned(),
                width: max_shaped_text_width(file, highlights.map(AsRef::as_ref), theme, text_system),
            });
        }
        DiffHorizontalMetrics { max_text_width: cached.as_ref().unwrap().width, max_gutter_width: self.max_gutter_width }
    }
}

struct ParsedDiff {
    /// The capture's checksum and the scope: identity of the parsed content.
    key: String,
    truncated: bool,
    additions: u32,
    deletions: u32,
    /// Indexed like `files`; survives row virtualization and folding.
    horizontal: Vec<FileHorizontalState>,
    files: Arc<Vec<FileDiff>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileHeaderPresentation {
    Row,
    Sticky,
}

impl FileHeaderPresentation {
    fn key_prefix(self) -> &'static str {
        match self {
            Self::Row => "file-hdr",
            Self::Sticky => "sticky-file-hdr",
        }
    }
}

#[derive(Default, Clone, Copy)]
struct FileFold {
    collapsed: bool,
    /// Bumped per toggle: keys the height tween + chevron transition.
    epoch: usize,
    from: f32,
    to: f32,
    /// When the toggle happened: the tweens are armed only briefly after the click, as gpui replays an element's
    /// animation on remount, and in the virtualized list a row scrolling back into view is a remount.
    toggled_at: Option<Instant>,
}

impl FileFold {
    fn animating(&self) -> bool {
        self.epoch > 0 && self.toggled_at.is_some_and(|at| at.elapsed() < FOLD_TWEEN_WINDOW)
    }
}

struct HighlightSlot {
    fingerprint: u64,
    state: DiffHighlightState,
    _excerpt_task: Option<Task<()>>,
    _fetch_task: Option<Task<()>>,
}

enum DiffHighlightState {
    Pending,
    Ready(Arc<DiffHighlights>),
    Excerpt(Arc<DiffHighlights>),
    Plain,
}

/// The open base-ref dropdown: a filter input on top, ranked substring rows below.
struct RefMenu {
    search: Entity<InputState>,
    /// Keyboard highlight within the filtered rows.
    active: usize,
    at: gpui::Point<Pixels>,
    _search_events: Subscription,
}

/// The line the pointer is on. Only one element per anchor ever takes the hover (the unified row, or a split row's
/// right column), so the anchor alone identifies it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct HoverRow {
    path: String,
    side: CommentSide,
    line: u32,
}

struct CommentDraft {
    editing_id: Option<u64>,
    path: String,
    /// The file's pre-rename path, when it moved: carried onto the comment so an `Old`-side citation names the file that
    /// line lives in.
    old_path: Option<String>,
    side: CommentSide,
    line: u32,
    input: Entity<TextareaState>,
    _events: Subscription,
}

/// A workspace's agents' turns, and the running one's watch.
#[derive(Default)]
pub(super) struct Turn {
    turns: Turns,
    ended: Arc<std::sync::atomic::AtomicBool>,
    _watch: Option<Task<()>>,
}

/// A discard waiting on its dialog: of which tab, in which checkout, over which capture.
#[derive(Clone)]
pub(super) struct DiscardAsk {
    tab: String,
    dir: String,
    target: DiscardTarget,
}

/// What a discard takes: all of the capture confirmed over, or one file as it was shown.
#[derive(Clone)]
pub(super) enum DiscardTarget {
    Tree { checksum: String },
    File(FileDiff),
}

pub(super) enum ChangesEvent {
    /// The working-tree trash button, or a file's menu. The shell owns the confirmation dialog.
    Discard { dir: String, target: DiscardTarget },
    /// A file's menu, for the user's editor.
    Open { dir: String, root: String, path: String },
    /// A file's menu, for the image dialog.
    Preview { dir: String, root: String, path: String },
    /// Cmd+Enter in a comment's field: the comments go to the agent.
    Send,
}

impl gpui::EventEmitter<ChangesEvent> for Changes {}

/// A diff tab: follows the shown pane's checkout while it's the tab shown.
pub(super) struct Changes {
    /// The checkout, located.
    dir: Option<String>,
    branch: Option<String>,
    /// The last capture, for `dir` and the scope.
    diff: Option<Arc<CheckoutDiff>>,
    error: Option<SharedString>,
    /// Runs while the tab is shown; none, nothing is asked of git.
    watch_task: Option<Task<()>>,
    parsed: Option<ParsedDiff>,
    parse_task: Option<Task<()>>,
    folds: HashMap<String, FileFold>,
    highlights: HashMap<String, HighlightSlot>,
    /// The flattened row model the list virtualizes over (line granularity; collapsed bodies excluded) + each file's row
    /// span within it.
    rows: Vec<DiffRow>,
    row_ranges: Vec<std::ops::Range<usize>>,
    /// Sweeps [`DiffRow::FoldingBody`] stand-ins back to steady-state rows once their tween window elapses.
    fold_settle: Option<Task<()>>,
    list: ListState,
    scope: DiffScope,
    mode: DiffMode,
    /// Wrap long source lines instead of exposing the horizontal code plane.
    wrap_lines: bool,
    /// Comparison ref for [`DiffScope::Branch`]: preset to the repo's default branch once the branch list lands.
    base_ref: Option<String>,
    branches: Vec<String>,
    /// The `dir` the branch list was fetched for.
    branches_for: Option<String>,
    branches_task: Option<Task<()>>,
    scope_menu: Option<crate::ui::Popup<gpui::Point<Pixels>>>,
    /// A file header's right-click menu: which file, and where.
    file_menu: Option<crate::ui::Popup<(String, gpui::Point<Pixels>)>>,
    /// The scope menu's, for its Esc.
    scope_focus: gpui::FocusHandle,
    ref_menu: Option<crate::ui::Popup<RefMenu>>,
    ref_scroll: gpui::ScrollHandle,
    /// Comments of each checkout shown, in staged order; in memory only, as a composer's draft.
    comments: HashMap<String, Vec<ReviewComment>>,
    made_comments: u64,
    /// Only ever one: a second `+` moves the card rather than stacking two half-written notes.
    draft: Option<CommentDraft>,
    hover: Option<HoverRow>,
    comment_key: u64,
    pub(super) sending: bool,
    /// The file the keyboard's cursor is on, by index.
    cursor: Option<usize>,
    /// The keyboard is in this diff: the cursor shows. The shell says so each render.
    pub(super) keys: bool,
    /// Where the keyboard goes back to when a comment's card closes: the shell's own, the panel's while it has them.
    pub(super) back: Option<gpui::FocusHandle>,
    /// A card just closed: the next render gives the keyboard `back`.
    refocus: bool,
}

impl Changes {
    fn new(cx: &mut Context<Self>) -> Self {
        let settings = &SettingsGlobal::of(cx).settings;
        Self {
            dir: None,
            branch: None,
            diff: None,
            error: None,
            watch_task: None,
            parsed: None,
            parse_task: None,
            folds: HashMap::new(),
            highlights: HashMap::new(),
            rows: Vec::new(),
            row_ranges: Vec::new(),
            fold_settle: None,
            // Rows are single lines: a deep overdraw is cheap and keeps fast wheel flicks from outrunning measurement.
            list: ListState::new(0, ListAlignment::Top, px(1024.0)),
            scope: DiffScope::default(),
            mode: DiffMode::from_split(settings.diff_split),
            wrap_lines: settings.diff_wrap,
            base_ref: None,
            branches: Vec::new(),
            branches_for: None,
            branches_task: None,
            scope_menu: None,
            file_menu: None,
            scope_focus: cx.focus_handle(),
            ref_menu: None,
            ref_scroll: gpui::ScrollHandle::new(),
            comments: HashMap::new(),
            made_comments: 0,
            draft: None,
            hover: None,
            comment_key: 0,
            sending: false,
            cursor: None,
            keys: false,
            back: None,
            refocus: false,
        }
    }

    /// The surface-tab title: the scope's label.
    pub(super) fn tab_title(&self) -> &'static str {
        self.scope.label()
    }

    /// Called every render of the shell. A new `dir` follows the shown pane; `watch` runs the poll.
    fn show(&mut self, dir: Option<&str>, branch: Option<&str>, watch: bool, cx: &mut Context<Self>) {
        if branch.is_some() && self.branch.as_deref() != branch {
            self.branch = branch.map(str::to_string);
            cx.notify();
        }
        let moved = dir.is_some_and(|dir| self.dir.as_deref() != Some(dir));
        if moved {
            self.dir = dir.map(str::to_string);
            // A draft belongs to the checkout it was opened over: it doesn't follow the user across.
            self.draft = None;
            self.base_ref = None;
            self.branches.clear();
            self.forget_diff(cx);
        }
        if moved || watch != self.watch_task.is_some() {
            self.watch_task = None;
            if watch {
                self.ensure_watch(cx);
            }
        }
    }

    /// Starts over on the capture: `dir`, the scope or the base changed under it, or a discard ran.
    fn restart_watch(&mut self, cx: &mut Context<Self>) {
        if self.watch_task.take().is_some() {
            self.ensure_watch(cx);
        }
    }

    fn ensure_watch(&mut self, cx: &mut Context<Self>) {
        self.ensure_branches(cx);
        let Some(dir) = self.dir.clone() else {
            // nothing to watch yet: holds the place, so `show` doesn't start one every render
            self.watch_task = Some(Task::ready(()));
            return;
        };
        let turn = self.scope == DiffScope::LastTurn;
        let base = match self.scope {
            DiffScope::WorkingTree | DiffScope::LastTurn => None,
            DiffScope::Branch => match self.base_ref.clone() {
                Some(base) => Some(base),
                None => {
                    // the branch list is still loading, and restarts this
                    self.watch_task = Some(Task::ready(()));
                    return;
                }
            },
        };
        self.watch_task = Some(cx.spawn(async move |this, cx| {
            loop {
                let (dir, base) = (dir.clone(), base.clone());
                let capture = cx.background_spawn(async move { if turn { capture_turn(&dir) } else { capture(&dir, base.as_deref()) } }).await;
                if this.update(cx, |changes, cx| changes.captured(capture, cx)).is_err() {
                    return;
                }
                cx.background_executor().timer(POLL).await;
            }
        }));
    }

    fn captured(&mut self, capture: Result<CheckoutDiff, String>, cx: &mut Context<Self>) {
        match capture {
            Ok(diff) => {
                self.error = None;
                if self.diff.as_ref().is_none_or(|d| d.checksum != diff.checksum) {
                    self.diff = Some(Arc::new(diff));
                    self.sync(cx);
                }
            }
            Err(e) => self.error = Some(e.into()),
        }
        cx.notify();
    }

    /// What's shown belongs to another checkout, scope or base now: the spinner, until the next capture.
    fn forget_diff(&mut self, cx: &mut Context<Self>) {
        self.diff = None;
        self.error = None;
        self.parse_task = None;
        if self.parsed.take().is_some() {
            self.rows.clear();
            self.row_ranges.clear();
            self.list.reset(0);
            self.folds.clear();
            self.highlights.clear();
        }
        cx.notify();
    }

    fn discard_request(&self) -> Option<(String, String)> {
        let diff = self.diff.as_ref().filter(|d| self.scope == DiffScope::WorkingTree && !d.truncated && !d.patch.trim().is_empty())?;
        Some((self.dir.clone()?, diff.checksum.clone()))
    }

    /// Scope discriminant folded into the parse key, so a scope or base switch re-parses even when checksums collide.
    fn parse_key(&self, diff: &CheckoutDiff) -> String {
        match self.scope {
            DiffScope::WorkingTree => format!("{}:wt", diff.checksum),
            DiffScope::LastTurn => format!("{}:lt", diff.checksum),
            DiffScope::Branch => format!("{}:br:{}", diff.checksum, self.base_ref.as_deref().unwrap_or("")),
        }
    }

    /// Fetch the branch list for the checkout (idempotent per `dir`); the repo's default branch (first entry) becomes the
    /// comparison base unless the user already picked one that still exists.
    fn ensure_branches(&mut self, cx: &mut Context<Self>) {
        let Some(dir) = self.dir.clone().filter(|dir| self.branches_for.as_ref() != Some(dir)) else { return };
        self.branches_for = Some(dir.clone());
        self.branches_task = Some(cx.spawn(async move |this, cx| {
            let asked = dir.clone();
            let result = cx.background_spawn(async move { crate::changes::list_branches(&asked) }).await;
            this.update(cx, |changes, cx| {
                if changes.dir.as_ref() != Some(&dir) {
                    return; // superseded by a pane switch
                }
                match result {
                    Ok(branches) => {
                        changes.branches = branches;
                        if !changes.base_ref.as_ref().is_some_and(|base| changes.branches.contains(base)) {
                            changes.base_ref = default_base_ref(&changes.branches, changes.branch.as_deref());
                        }
                        if changes.scope == DiffScope::Branch {
                            changes.restart_watch(cx);
                        }
                    }
                    // Allow a retry on the next start.
                    Err(_) => changes.branches_for = None,
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn reset_horizontal_scroll(&self) {
        if let Some(parsed) = &self.parsed {
            for file in &parsed.horizontal {
                file.scroll.set_offset(gpui::Point::default());
            }
        }
    }

    fn set_scope(&mut self, scope: DiffScope, cx: &mut Context<Self>) {
        if self.scope != scope {
            self.scope = scope;
            self.reset_horizontal_scroll();
            self.forget_diff(cx);
            self.restart_watch(cx);
        }
        cx.notify();
    }

    fn set_base_ref(&mut self, base: String, cx: &mut Context<Self>) {
        if self.base_ref.as_deref() != Some(base.as_str()) {
            self.base_ref = Some(base);
            self.forget_diff(cx);
            self.restart_watch(cx);
        }
        cx.notify();
    }

    /// Reconcile parsed content with the capture.
    fn sync(&mut self, cx: &mut Context<Self>) {
        let Some(diff) = self.diff.clone() else { return };
        let key = self.parse_key(&diff);
        if self.parsed.as_ref().is_some_and(|p| p.key == key) {
            self.sync_comment_rows(cx);
            return;
        }
        // Parse off the render path: patches run to megabytes.
        self.parse_task = Some(cx.spawn(async move |this, cx| {
            let (files, truncated) = cx
                .background_spawn(async move {
                    let files = parse_patch(&diff.patch);
                    (files, diff.truncated)
                })
                .await;
            this.update(cx, |changes, cx| changes.parsed_files(key, files, truncated, cx)).ok();
        }));
    }

    fn parsed_files(&mut self, key: String, files: Vec<FileDiff>, truncated: bool, cx: &mut Context<Self>) {
        // Late results for a superseded diff are re-checked by key.
        if self.diff.as_ref().map(|d| self.parse_key(d)).as_deref() != Some(key.as_str()) {
            return;
        }
        // the tree moved under a reader: their folds stay, and the file at the top of the list
        let top = self.top_file_path();
        let additions = files.iter().map(|f| f.additions).sum();
        let deletions = files.iter().map(|f| f.deletions).sum();
        let horizontal = files.iter().map(FileHorizontalState::new).collect();
        self.highlights.clear();
        let staged = self.staged_comments();
        let draft = self.draft_anchor();
        // a file first seen comes shut, unless a comment waits in it: the footer counts those, the list has to show them
        for file in &files {
            let commented = staged.iter().any(|c| c.path == file.path) || draft.as_ref().is_some_and(|(path, ..)| *path == file.path);
            self.folds.entry(file.path.clone()).or_insert(FileFold { collapsed: !commented, ..FileFold::default() });
        }
        let (rows, ranges) = flatten_rows(
            &files,
            &staged,
            draft.as_ref().map(|(path, side, line)| (path.as_str(), *side, *line)),
            self.mode,
            |ix| self.folds.get(&files[ix].path).is_some_and(|fold| fold.collapsed),
        );
        self.comment_key = comment_state_key(&staged, draft.as_ref());
        // The uniform hint keeps offsets for never-rendered rows sane (most rows ARE lines); real heights land as rows
        // render.
        self.list.reset_with_uniform_height(rows.len(), px(DIFF_LINE_HEIGHT));
        self.rows = rows;
        self.row_ranges = ranges;
        if let Some(start) = top.and_then(|path| files.iter().position(|f| f.path == path)).and_then(|ix| self.row_ranges.get(ix)).map(|r| r.start) {
            self.list.scroll_to_reveal_item(start);
        }
        self.parsed = Some(ParsedDiff { key, truncated, additions, deletions, horizontal, files: Arc::new(files) });
        cx.notify();
    }

    /// The file under the viewport's top edge.
    fn top_file_path(&self) -> Option<String> {
        let top = self.list.logical_scroll_top().item_ix;
        let ix = self.row_ranges.iter().position(|range| range.contains(&top))?;
        Some(self.parsed.as_ref()?.files.get(ix)?.path.clone())
    }

    /// A key with the keyboard in the panel, as in the sidebar: j/k and ↓/↑ move the cursor over file headers and the
    /// lines of open files, l/→ opens the file (or steps into it), h/← goes up to the file's header and shuts it,
    /// Enter or o flips a file, c (or Enter) on a line comments on it, Shift+j/k and Shift+↓/↑ scroll a few lines,
    /// Space and Shift+Space a page, g/G the first and last row. False when the key isn't the diff's.
    // ponytail: the cursor is a row index, so a file opening mid-tween (one stand-in row) is stepped over, and a new
    // capture can move it to another line
    pub(super) fn nav_key(&mut self, key: &str, shift: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.rows.is_empty() {
            return false;
        }
        let stop = |row: &DiffRow| matches!(row, DiffRow::FileHeader { .. } | DiffRow::Line { .. } | DiffRow::SplitLine { .. });
        let top = self.list.logical_scroll_top().item_ix;
        let file_top = self.row_ranges.iter().find(|r| r.contains(&top)).map_or(0, |r| r.start);
        let at = self.cursor.filter(|c| *c < self.rows.len()).unwrap_or(file_top);
        let file = self.rows[at].file();
        let header = self.row_ranges.get(file).map_or(at, |r| r.start);
        let next = self.rows.iter().enumerate().skip(at + 1).find(|(_, r)| stop(r)).map(|(i, _)| i);
        let prev = self.rows[..at].iter().rposition(stop);
        let collapsed = self.parsed.as_ref().and_then(|p| p.files.get(file)).is_some_and(|f| self.folds.get(&f.path).is_some_and(|fold| fold.collapsed));
        let page = (f32::from(self.list.viewport_bounds().size.height) - 40.).max(40.);
        let to = match key {
            "down" | "j" if shift => return self.scrolled(px(60.), cx),
            "up" | "k" if shift => return self.scrolled(px(-60.), cx),
            "space" => return self.scrolled(px(if shift { -page } else { page }), cx),
            "down" | "j" => next.unwrap_or(at),
            "up" | "k" => prev.unwrap_or(at),
            "g" if shift => self.rows.iter().rposition(stop).unwrap_or(at),
            "g" => 0,
            "right" | "l" if at == header && collapsed => {
                self.toggle_fold(file, cx);
                at
            }
            "right" | "l" => next.unwrap_or(at),
            "left" | "h" if at == header && !collapsed => {
                self.toggle_fold(file, cx);
                at
            }
            "left" | "h" => header,
            "enter" | "o" if at == header => {
                self.toggle_fold(file, cx);
                at
            }
            "enter" | "c" => {
                if let Some((path, side, line)) = self.anchor_at(at) {
                    self.open_draft(path, side, line, None, window, cx);
                }
                return true;
            }
            _ => return false,
        };
        self.cursor = Some(to);
        self.list.scroll_to_reveal_item(to);
        cx.notify();
        true
    }

    /// The line a comment on row `ix` is about: its new side, else its old.
    fn anchor_at(&self, ix: usize) -> Option<(String, CommentSide, u32)> {
        let row = *self.rows.get(ix)?;
        let file = self.parsed.as_ref()?.files.get(row.file())?;
        let line = |hunk: u32, slot: u32| file.hunks.get(hunk as usize)?.lines.get(slot as usize).and_then(line_anchor);
        let anchor = match row {
            DiffRow::Line { hunk, line: slot, .. } => line(hunk, slot),
            DiffRow::SplitLine { hunk, left, right, .. } => right.and_then(|r| line(hunk, r)).or_else(|| left.and_then(|l| line(hunk, l))),
            _ => None,
        }?;
        Some((file.path.clone(), anchor.0, anchor.1))
    }

    fn scrolled(&mut self, by: Pixels, cx: &mut Context<Self>) -> bool {
        self.list.scroll_by(by);
        cx.notify();
        true
    }

    /// Swap one file's body rows (everything after its header) for `new_body`, splicing both the row model and the list
    /// state. gpui's `splice` shifts the logical scroll anchor by the count delta, so content below the fold stays put.
    fn replace_file_body(&mut self, file_ix: usize, new_body: Vec<DiffRow>) {
        let Some(range) = self.row_ranges.get(file_ix).cloned() else { return };
        let body = range.start + 1..range.end;
        let delta = new_body.len() as isize - body.len() as isize;
        // Only splice the rows that moved: `ListState::splice` clamps the scroll anchor to the range start when the
        // anchored row is inside it, so replacing a whole body jumped the pane to the top of the file.
        let (prefix, suffix) = {
            let old = &self.rows[body.clone()];
            let prefix = old.iter().zip(&new_body).take_while(|(a, b)| a == b).count();
            let suffix = old[prefix..].iter().rev().zip(new_body[prefix..].iter().rev()).take_while(|(a, b)| a == b).count();
            (prefix, suffix)
        };
        if delta == 0 && prefix + suffix >= body.len() {
            return;
        }
        let changed = body.start + prefix..body.end - suffix;
        let mid: Vec<DiffRow> = new_body[prefix..new_body.len() - suffix].to_vec();
        self.list.splice(changed.clone(), mid.len());
        self.rows.splice(changed, mid);
        self.row_ranges[file_ix] = range.start..(range.end as isize + delta) as usize;
        for r in &mut self.row_ranges[file_ix + 1..] {
            *r = (r.start as isize + delta) as usize..(r.end as isize + delta) as usize;
        }
    }

    fn toggle_fold(&mut self, file_ix: usize, cx: &mut Context<Self>) {
        let Some(file) = self.parsed.as_ref().and_then(|p| p.files.get(file_ix)).cloned() else { return };
        if self.wrap_lines {
            let collapsed = !self.folds.get(&file.path).is_some_and(|fold| fold.collapsed);
            let body = if collapsed { Vec::new() } else { body_rows(file_ix as u32, &file, &self.comments_for(&file.path), self.draft_anchor_in(&file.path), self.mode) };
            let fold = self.folds.entry(file.path.clone()).or_default();
            fold.collapsed = collapsed;
            fold.toggled_at = None;
            self.replace_file_body(file_ix, body);
            cx.notify();
            return;
        }
        let expanded_height = body_height(&file, &self.comments_for(&file.path), self.draft_anchor_in(&file.path), self.mode);
        let fold = self.folds.entry(file.path.clone()).or_default();
        let currently_collapsed = fold.collapsed;
        fold.from = if currently_collapsed { 0.0 } else { expanded_height };
        fold.to = if currently_collapsed { expanded_height } else { 0.0 };
        fold.collapsed = !currently_collapsed;
        fold.epoch += 1;
        fold.toggled_at = Some(Instant::now());
        // The body tweens as ONE clipped stand-in row; the settle sweep swaps it for steady rows (all lines, or none) once
        // the window elapses.
        self.replace_file_body(file_ix, vec![DiffRow::FoldingBody { file: file_ix as u32 }]);
        self.ensure_fold_settle(cx);
        cx.notify();
    }

    /// Keep a sweep alive while any [`DiffRow::FoldingBody`] stand-ins remain; each tick settles the ones whose tween
    /// window has elapsed.
    fn ensure_fold_settle(&mut self, cx: &mut Context<Self>) {
        if self.fold_settle.is_some() {
            return;
        }
        self.fold_settle = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(FOLD_TWEEN_WINDOW).await;
                let more = this.update(cx, |changes, cx| changes.settle_folds(cx)).unwrap_or(false);
                if !more {
                    break;
                }
            }
            this.update(cx, |changes, _| changes.fold_settle = None).ok();
        }));
    }

    /// Replace every settled folding stand-in with its steady-state rows. Returns whether any stand-ins are still
    /// mid-tween.
    fn settle_folds(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(parsed) = &self.parsed else { return false };
        let files = parsed.files.clone();
        let mut pending = false;
        for file_ix in (0..self.row_ranges.len()).rev() {
            let range = &self.row_ranges[file_ix];
            if self.rows.get(range.start + 1) != Some(&DiffRow::FoldingBody { file: file_ix as u32 }) {
                continue;
            }
            let Some(file) = files.get(file_ix) else { continue };
            let fold = self.folds.get(&file.path).copied().unwrap_or_default();
            if fold.animating() {
                pending = true;
                continue;
            }
            let body = if fold.collapsed { Vec::new() } else { body_rows(file_ix as u32, file, &self.comments_for(&file.path), self.draft_anchor_in(&file.path), self.mode) };
            self.replace_file_body(file_ix, body);
        }
        cx.notify();
        pending
    }

    /// Every parsed file currently folded shut?
    fn all_collapsed(&self) -> bool {
        let Some(parsed) = &self.parsed else { return false };
        !parsed.files.is_empty() && parsed.files.iter().all(|file| self.folds.get(&file.path).is_some_and(|fold| fold.collapsed))
    }

    fn fold_all_label(&self) -> &'static str {
        if self.all_collapsed() { "Expand all files" } else { "Collapse all files" }
    }

    /// Collapse every file section, or expand them all when everything is already shut. Steady-state writes: no per-row
    /// tween arming, the whole list just snaps.
    fn toggle_collapse_all(&mut self, cx: &mut Context<Self>) {
        let Some(parsed) = &self.parsed else { return };
        let collapse = !self.all_collapsed();
        for file in parsed.files.clone().iter() {
            let fold = self.folds.entry(file.path.clone()).or_default();
            fold.collapsed = collapse;
            fold.toggled_at = None;
        }
        self.reflatten(cx);
    }

    /// Swap unified ⇄ split. The parse is untouched, only the flattening changes.
    fn toggle_mode(&mut self, cx: &mut Context<Self>) {
        self.mode = self.mode.toggled();
        self.reset_horizontal_scroll();
        let split = self.mode.is_split();
        save_from_sidebar(cx, |s| s.diff_split = split);
        // A draft's `+` sits in a column that may not exist after the swap.
        self.hover = None;
        self.reflatten(cx);
    }

    fn toggle_wrap(&mut self, cx: &mut Context<Self>) {
        self.wrap_lines = !self.wrap_lines;
        self.reset_horizontal_scroll();
        let wrap = self.wrap_lines;
        save_from_sidebar(cx, |s| s.diff_wrap = wrap);
        // A folding stand-in has an analytic fixed-line height. Settle it before switching to variable-height rows; steady
        // rows can then be measured by the virtual list at the current pane width.
        if self.rows.iter().any(|row| matches!(row, DiffRow::FoldingBody { .. })) {
            self.fold_settle = None;
            for fold in self.folds.values_mut() {
                fold.toggled_at = None;
            }
            self.reflatten(cx);
        } else {
            self.list.remeasure();
            cx.notify();
        }
    }

    /// Rebuilds the row model and re-anchors the scroll onto whichever file was under the viewport's top edge (row indices
    /// do not survive the re-pairing).
    fn reflatten(&mut self, cx: &mut Context<Self>) {
        let Some(parsed) = &self.parsed else {
            cx.notify();
            return;
        };
        let files = parsed.files.clone();
        let top = self.list.logical_scroll_top().item_ix;
        let anchor_file = self.row_ranges.iter().position(|range| range.contains(&top));
        let staged = self.staged_comments();
        let draft = self.draft_anchor();
        let (rows, ranges) = flatten_rows(
            &files,
            &staged,
            draft.as_ref().map(|(path, side, line)| (path.as_str(), *side, *line)),
            self.mode,
            |ix| self.folds.get(&files[ix].path).is_some_and(|fold| fold.collapsed),
        );
        self.list.reset_with_uniform_height(rows.len(), px(DIFF_LINE_HEIGHT));
        self.rows = rows;
        self.row_ranges = ranges;
        if let Some(start) = anchor_file.and_then(|ix| self.row_ranges.get(ix)).map(|r| r.start) {
            self.list.scroll_to_reveal_item(start);
        }
        cx.notify();
    }

    /// The comments of the checkout shown, the one being edited left to its draft.
    fn staged_comments(&self) -> Vec<ReviewComment> {
        let editing = self.draft.as_ref().and_then(|draft| draft.editing_id);
        let shown = self.dir.as_ref().and_then(|dir| self.comments.get(dir));
        shown.into_iter().flatten().filter(|comment| Some(comment.id) != editing).cloned().collect()
    }

    fn comments_for(&self, path: &str) -> Vec<ReviewComment> {
        self.staged_comments().into_iter().filter(|comment| comment.path == path).collect()
    }

    /// The comments waiting to be sent from the checkout shown.
    pub(super) fn comment_count(&self) -> usize {
        self.dir.as_ref().and_then(|dir| self.comments.get(dir)).map_or(0, Vec::len)
    }

    /// The parsed diff's pre-rename path for `path`, when the file moved.
    fn old_path_of(&self, path: &str) -> Option<String> {
        self.parsed.as_ref()?.files.iter().find(|file| file.path == path)?.old_path.clone()
    }

    fn draft_anchor(&self) -> Option<(String, CommentSide, u32)> {
        self.draft.as_ref().map(|draft| (draft.path.clone(), draft.side, draft.line))
    }

    fn draft_anchor_in(&self, path: &str) -> Option<(CommentSide, u32)> {
        self.draft.as_ref().filter(|draft| draft.path == path).map(|draft| (draft.side, draft.line))
    }

    fn sync_comment_rows(&mut self, cx: &mut Context<Self>) {
        let Some(parsed) = &self.parsed else { return };
        let files = parsed.files.clone();
        let staged = self.staged_comments();
        let draft = self.draft_anchor();
        let key = comment_state_key(&staged, draft.as_ref());
        if key == self.comment_key {
            return;
        }
        self.comment_key = key;
        for file_ix in (0..self.row_ranges.len().min(files.len())).rev() {
            let file = &files[file_ix];
            // A mid-tween stand-in is the settle sweep's to replace.
            if self.folds.get(&file.path).is_some_and(|fold| fold.collapsed) {
                continue;
            }
            let range = &self.row_ranges[file_ix];
            if self.rows.get(range.start + 1) == Some(&DiffRow::FoldingBody { file: file_ix as u32 }) {
                continue;
            }
            let comments: Vec<ReviewComment> = staged.iter().filter(|comment| comment.path == file.path).cloned().collect();
            let body = body_rows(file_ix as u32, file, &comments, self.draft_anchor_in(&file.path), self.mode);
            self.replace_file_body(file_ix, body);
        }
        cx.notify();
    }

    fn set_hover(&mut self, path: &str, anchor: Option<(CommentSide, u32)>, cx: &mut Context<Self>) {
        let next = anchor.map(|(side, line)| HoverRow { path: path.to_string(), side, line });
        if next != self.hover {
            self.hover = next;
            cx.notify();
        }
    }

    fn hovering(&self, path: &str, anchor: (CommentSide, u32)) -> bool {
        self.hover.as_ref().is_some_and(|hover| hover.path == path && (hover.side, hover.line) == anchor)
    }

    fn clear_hover_at(&mut self, path: &str, anchor: (CommentSide, u32), cx: &mut Context<Self>) {
        if self.hovering(path, anchor) {
            self.hover = None;
            cx.notify();
        }
    }

    fn open_draft(&mut self, path: String, side: CommentSide, line: u32, body: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let theme = Theme::of(cx).clone();
        // Enter commits and Shift+Enter breaks the line, as a composer's
        let input = cx.new(|cx| TextareaState::new(window, cx).placeholder("Request a change…").submit_on_enter(true));
        let events = cx.subscribe(&input, |this: &mut Self, _, event: &InputEvent, cx| {
            if let InputEvent::PressEnter { secondary, .. } = event {
                this.commit_draft(cx);
                // Cmd+Enter (Ctrl+Enter off macOS): the field takes it, so the send goes by way of the shell
                if *secondary {
                    cx.emit(ChangesEvent::Send);
                }
            }
        });
        input.update(cx, |input, cx| {
            input.set_editor_style(crate::ui::input_style(&theme));
            if let Some(body) = body {
                input.set_value(body, window, cx);
            }
            input.focus(window, cx);
        });
        let old_path = self.old_path_of(&path);
        self.draft = Some(CommentDraft { editing_id: None, path, old_path, side, line, input, _events: events });
        self.sync_comment_rows(cx);
        cx.notify();
    }

    fn edit_comment(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let shown = self.dir.as_ref().and_then(|dir| self.comments.get(dir));
        let Some(comment) = shown.and_then(|comments| comments.iter().find(|comment| comment.id == id)).cloned() else { return };
        self.open_draft(comment.path.clone(), comment.side, comment.line, Some(comment.body), window, cx);
        if let Some(draft) = self.draft.as_mut() {
            draft.editing_id = Some(comment.id);
            draft.old_path = comment.old_path;
        }
        self.sync_comment_rows(cx);
    }

    fn cancel_draft(&mut self, cx: &mut Context<Self>) {
        self.draft = None;
        self.refocus = true;
        self.sync_comment_rows(cx);
        cx.notify();
    }

    pub(super) fn commit_draft(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.draft.take() else { return };
        self.refocus = true;
        let body = draft.input.read(cx).value().trim().to_string();
        if let (false, Some(dir)) = (body.is_empty(), self.dir.clone()) {
            let comments = self.comments.entry(dir).or_default();
            match draft.editing_id.and_then(|id| comments.iter_mut().find(|comment| comment.id == id)) {
                Some(comment) => comment.body = body,
                None => {
                    self.made_comments += 1;
                    comments.push(ReviewComment { id: self.made_comments, path: draft.path, old_path: draft.old_path, side: draft.side, line: draft.line, body });
                }
            }
        }
        self.sync_comment_rows(cx);
        cx.notify();
    }

    fn remove_comment(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(comments) = self.dir.as_ref().and_then(|dir| self.comments.get_mut(dir)) {
            comments.retain(|comment| comment.id != id);
        }
        self.sync_comment_rows(cx);
        cx.notify();
    }

    /// What to send, once: the prompt and which comments it holds.
    fn take_prompt(&mut self) -> Option<(String, String, Vec<u64>)> {
        let dir = self.dir.clone().filter(|_| !self.sending)?;
        let comments = self.comments.get(&dir).filter(|comments| !comments.is_empty())?;
        let ids = comments.iter().map(|comment| comment.id).collect();
        let prompt = comments_prompt(comments);
        self.sending = true;
        Some((dir, prompt, ids))
    }

    /// Sent, they go; failed, they all stay.
    fn sent(&mut self, dir: &str, ids: Option<Vec<u64>>, cx: &mut Context<Self>) {
        self.sending = false;
        if let (Some(ids), Some(comments)) = (ids, self.comments.get_mut(dir)) {
            comments.retain(|comment| !ids.contains(&comment.id));
        }
        self.sync_comment_rows(cx);
        cx.notify();
    }

    fn close_scope_menu(&mut self, cx: &mut Context<Self>) {
        if let Some(menu) = self.scope_menu.as_mut() {
            menu.close();
        }
        cx.notify();
    }

    fn close_ref_menu(&mut self, cx: &mut Context<Self>) {
        if let Some(menu) = self.ref_menu.as_mut() {
            menu.close();
        }
        cx.notify();
    }

    fn open_ref_menu(&mut self, at: gpui::Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let theme = Theme::of(cx).clone();
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search branches…"));
        search.update(cx, |input, cx| {
            input.set_editor_style(crate::ui::input_style(&theme));
            input.focus(window, cx);
        });
        let events = cx.subscribe(&search, |this: &mut Self, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change)
                && let Some(menu) = this.ref_menu.as_mut()
            {
                menu.value.active = 0;
                cx.notify();
            }
        });
        // The highlight starts ON the current base (query is empty, so the filtered rows are just the branch list).
        let active = self.base_ref.as_ref().and_then(|base| self.branches.iter().position(|b| b == base)).unwrap_or(0);
        self.ref_menu = Some(crate::ui::Popup::new(RefMenu { search, active, at, _search_events: events }));
        cx.notify();
    }

    /// Filtered branch indices for the open ref menu.
    fn ref_menu_rows(&self, cx: &App) -> Vec<usize> {
        let query = self.ref_menu.as_ref().map(|menu| menu.value.search.read(cx).value().to_string()).unwrap_or_default();
        filter_indices(&query, &self.branches)
    }

    /// ↑↓ navigate, ⏎ picks the highlighted branch, Esc closes.
    fn ref_menu_key(&mut self, event: &gpui::KeyDownEvent, cx: &mut Context<Self>) {
        // The card stays mounted through the exit animation: keys must not drive a dying menu.
        if !self.ref_menu.as_ref().is_some_and(|menu| menu.is_open()) {
            return;
        }
        match event.keystroke.key.as_str() {
            "escape" => self.close_ref_menu(cx),
            "up" | "down" => {
                let count = self.ref_menu_rows(cx).len().max(1) as isize;
                let delta = if event.keystroke.key == "up" { -1 } else { 1 };
                if let Some(menu) = self.ref_menu.as_mut() {
                    menu.value.active = (menu.value.active as isize + delta).rem_euclid(count) as usize;
                    crate::ui::reveal(&self.ref_scroll, menu.value.active..=menu.value.active, 8.);
                }
                cx.notify();
            }
            "enter" => {
                let active = self.ref_menu.as_ref().map_or(0, |menu| menu.value.active);
                let pick = self.ref_menu_rows(cx).get(active).and_then(|ix| self.branches.get(*ix).cloned());
                if let Some(branch) = pick {
                    self.set_base_ref(branch, cx);
                    self.close_ref_menu(cx);
                }
            }
            _ => return,
        }
        cx.stop_propagation();
    }

    /// Start excerpt parsing and a lazy full-source fetch for an expanded file.
    fn request_highlight(&mut self, file: &FileDiff, parsed_key: &str, cx: &mut Context<Self>) -> Option<Arc<DiffHighlights>> {
        let lang = zeron_syntax::language_for_path(&file.path)?;
        let fingerprint = hash64(&[parsed_key, &file.path]);
        if let Some(slot) = self.highlights.get(&file.path)
            && slot.fingerprint == fingerprint
        {
            return match &slot.state {
                DiffHighlightState::Ready(highlights) | DiffHighlightState::Excerpt(highlights) => Some(highlights.clone()),
                DiffHighlightState::Pending | DiffHighlightState::Plain => None,
            };
        }
        if !zeron_syntax::supports_language(lang) {
            self.highlights.insert(file.path.clone(), HighlightSlot { fingerprint, state: DiffHighlightState::Plain, _excerpt_task: None, _fetch_task: None });
            return None;
        }
        let path = file.path.clone();
        let excerpt_file = file.clone();
        let excerpt_path = path.clone();
        let excerpt_task = cx.spawn(async move |this, cx| {
            let highlights = cx.background_spawn(async move { excerpt_highlights(&excerpt_file, lang).map(Arc::new) }).await;
            this.update(cx, |changes, cx| {
                if let Some(slot) = changes.highlights.get_mut(&excerpt_path)
                    && slot.fingerprint == fingerprint
                    && matches!(slot.state, DiffHighlightState::Pending)
                {
                    slot.state = match highlights {
                        Some(highlights) => DiffHighlightState::Excerpt(highlights),
                        None => DiffHighlightState::Plain,
                    };
                    cx.notify();
                }
            })
            .ok();
        });
        let fetch_file = file.clone();
        let fetch_task = self.diff.clone().zip(self.dir.clone()).map(|(diff, dir)| {
            cx.spawn(async move |this, cx| {
                let highlights = cx
                    .background_spawn(async move {
                        let (old, new) = file_sources(&dir, &diff, &fetch_file)?;
                        full_highlights(&fetch_file, lang, old.as_deref(), new.as_deref()).map(Arc::new)
                    })
                    .await;
                this.update(cx, |changes, cx| {
                    if let Some(slot) = changes.highlights.get_mut(&path)
                        && slot.fingerprint == fingerprint
                        && let Some(highlights) = highlights
                    {
                        slot.state = DiffHighlightState::Ready(highlights);
                        cx.notify();
                    }
                })
                .ok();
            })
        });
        self.highlights.insert(file.path.clone(), HighlightSlot { fingerprint, state: DiffHighlightState::Pending, _excerpt_task: Some(excerpt_task), _fetch_task: fetch_task });
        None
    }

    /// Row `ix`, lit under the keyboard's cursor (a file header lights itself).
    fn render_row(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let row = self.render_row_inner(ix, window, cx);
        let lit = self.keys && self.cursor == Some(ix) && !matches!(self.rows.get(ix), Some(DiffRow::FileHeader { .. }));
        if !lit {
            return row;
        }
        let accent = Theme::of(cx).accent;
        div().relative().w_full().child(row).child(div().absolute().inset_0().bg(accent.opacity(0.1)).border_l_2().border_color(accent)).into_any_element()
    }

    fn render_row_inner(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let empty = || gpui::Empty.into_any_element();
        let Some(parsed) = &self.parsed else { return empty() };
        let files = parsed.files.clone();
        let parsed_key = parsed.key.clone();
        let Some(row) = self.rows.get(ix).copied() else { return empty() };
        let Some(file_diff) = files.get(row.file()) else { return empty() };
        let theme = Theme::of(cx).clone();
        let highlight = self.request_highlight(file_diff, &parsed_key, cx);
        let Some(horizontal) = self.parsed.as_ref().and_then(|p| p.horizontal.get(row.file())) else { return empty() };
        let code_width = match self.wrap_lines {
            false => DiffCodeWidth::Scrollable(horizontal.metrics(file_diff, highlight.as_ref(), &theme, window.text_system())),
            true => DiffCodeWidth::Wrapped,
        };
        let code_scroll = DiffCodeScrollContext { handle: horizontal.scroll.clone(), prefix: SharedString::from(format!("changes-code-row-{ix}")) };
        match row {
            DiffRow::FileHeader { file } => {
                let fold = self.folds.get(&file_diff.path).copied().unwrap_or_default();
                self.render_file_header(file as usize, file_diff, &fold, FileHeaderPresentation::Row, &theme, cx)
            }
            DiffRow::Notice { notice, .. } => file_notices(file_diff).into_iter().nth(notice as usize).map(|text| notice_row(text, &theme)).unwrap_or_else(empty),
            DiffRow::HunkHeader { hunk, .. } => file_diff.hunks.get(hunk as usize).map(|h| hunk_header_row(&h.header, &theme)).unwrap_or_else(empty),
            DiffRow::Line { hunk, line, .. } => {
                let Some(line) = file_diff.hunks.get(hunk as usize).and_then(|h| h.lines.get(line as usize)) else { return empty() };
                let gutter_px = gutter_width(file_diff);
                let row = diff_line_row(line, highlight.as_deref(), &theme, gutter_px, code_width, code_scroll.slot("unified"));
                let Some((side, line_no)) = line_anchor(line) else { return row };
                let path = file_diff.path.clone();
                let hovered = self.hovering(&path, (side, line_no));
                let (move_path, leave_path) = (path.clone(), path.clone());
                div()
                    .id(("diff-line", ix))
                    .w_full()
                    .relative()
                    .child(row)
                    .when(hovered, |el| el.child(positioned_adder(comment_adder_left(side, gutter_px), render_comment_adder(&path, side, line_no, &theme, cx))))
                    .on_mouse_move(cx.listener(move |this, _, _, cx| this.set_hover(&move_path, Some((side, line_no)), cx)))
                    .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        if !*hovered {
                            this.clear_hover_at(&leave_path, (side, line_no), cx);
                        }
                    }))
                    .into_any_element()
            }
            DiffRow::SplitLine { hunk, left, right, .. } => {
                let Some(lines) = file_diff.hunks.get(hunk as usize).map(|h| &h.lines) else { return empty() };
                let gutter_px = gutter_width(file_diff);
                // Same slot on both sides = a context row: one line, drawn in both columns.
                let mirrored = left.is_some() && left == right;
                let left = left.and_then(|slot| lines.get(slot as usize));
                let right = right.and_then(|slot| lines.get(slot as usize));
                // `\ No newline at end of file` is not code on one side: it is a note about the row, so it spans both
                // columns. Pairing never puts a marker opposite code, so either side having one means the whole row is
                // the marker.
                if let Some(line) = [left, right].into_iter().flatten().find(|line| line.kind == LineKind::Meta) {
                    return meta_line_row(&line.text, &theme, 2.0 * (ACCENT_BAR_WIDTH + gutter_px));
                }
                // Refcounted, not cloned per listener: this runs for every row in the viewport plus the list's overdraw.
                let path: SharedString = file_diff.path.clone().into();
                // A mirrored row's columns carry the same text and the same spans, so the runs are built once and shared.
                let shared_runs = mirrored.then(|| left.map(|line| line_runs(line, highlight.as_deref(), &theme))).flatten();
                let cell = |line: Option<&DiffLine>, old: bool| {
                    line.map(|line| {
                        let runs = shared_runs.clone().unwrap_or_else(|| line_runs(line, highlight.as_deref(), &theme));
                        split_line_cell(line, if old { line.old_no } else { line.new_no }, runs, &theme, gutter_px, code_width, code_scroll.slot(if old { "old" } else { "new" }))
                    })
                };
                // The left column is inert. It shows the pre-change file, and a deleted line is not there to be changed: a
                // note on it would cite a line the agent cannot edit. Cards for old-side notes still render (they are
                // pushed by the row, not the column), so switching layouts never hides one that is already staged.
                let left = cell(left, true).map(IntoElement::into_any_element).unwrap_or_else(|| split_filler(&theme).into_any_element());
                let right = match (cell(right, false), right.and_then(line_anchor)) {
                    (Some(cell), Some(anchor)) => {
                        let (side, line_no) = anchor;
                        let (move_path, leave_path) = (path.clone(), path.clone());
                        cell.id(("split-new", ix))
                            .when(self.hovering(&path, anchor), |el| el.relative().child(positioned_adder(split_adder_left(gutter_px), render_comment_adder(&path, side, line_no, &theme, cx))))
                            .on_mouse_move(cx.listener(move |this, _, _, cx| this.set_hover(&move_path, Some(anchor), cx)))
                            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                                if !*hovered {
                                    this.clear_hover_at(&leave_path, anchor, cx);
                                }
                            }))
                            .into_any_element()
                    }
                    (Some(cell), None) => cell.into_any_element(),
                    (None, _) => split_filler(&theme).into_any_element(),
                };
                split_row(left, right, self.wrap_lines, &theme).into_any_element()
            }
            DiffRow::CommentCard { card, .. } => self.comments_for(&file_diff.path).get(card as usize).map(|comment| render_comment_card(comment, &theme, cx)).unwrap_or_else(empty),
            DiffRow::CommentDraft { .. } => self.draft.as_ref().filter(|draft| draft.path == file_diff.path).map(|draft| render_comment_draft(draft, &theme, cx)).unwrap_or_else(empty),
            DiffRow::BodyPad { .. } => div().w_full().h(px(BODY_BOTTOM_PAD)).into_any_element(),
            DiffRow::FoldingBody { file } => {
                let fold = self.folds.get(&file_diff.path).copied().unwrap_or_default();
                let (from, to) = (fold.from, fold.to);
                // Only the revealable slice is built: the tween never pays for lines it cannot show.
                let cap = from.max(to).min(FOLD_TWEEN_MAX_PX);
                let scroll = DiffCodeScrollContext { handle: code_scroll.handle.clone(), prefix: SharedString::from(format!("changes-fold-code-{file}-{}", fold.epoch)) };
                let body = render_file_body_upto(file_diff, highlight, &theme, cap, self.mode, code_width, scroll);
                let clipped = div().w_full().overflow_hidden().child(body);
                if fold.animating() {
                    clipped
                        .with_animation(SharedString::from(format!("fold-{}-{}", file_diff.path, fold.epoch)), fold_animation(), move |el, t| el.h(px(from + (to - from) * t)))
                        .into_any_element()
                } else {
                    // Post-tween, pre-settle: hold the full target height so the settle splice swaps rows without any
                    // reflow.
                    clipped.h(px(to)).into_any_element()
                }
            }
        }
    }

    fn render_file_header(&mut self, ix: usize, file: &FileDiff, fold: &FileFold, presentation: FileHeaderPresentation, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let (adds, dels) = (file.additions, file.deletions);
        let sticky = presentation == FileHeaderPresentation::Sticky;
        let hover_bg = theme.wash(0.05);
        // Chevron: right closed, down open; the glyph swap crossfades.
        let chevron = gpui::svg()
            .path(if fold.collapsed { "icons/alt-arrow-right.svg" } else { "icons/alt-arrow-down.svg" })
            .size(px(13.0))
            .text_color(theme.text_muted.opacity(0.7));
        let chevron = div().flex_none().size(px(14.0)).flex().items_center().justify_center().child(chevron);
        let chevron: AnyElement = if fold.animating() {
            let id = SharedString::from(format!("chev-{}-{}-{}", presentation.key_prefix(), file.path, fold.epoch));
            chevron.with_animation(id, fold_animation(), |el, t| el.opacity(0.25 + 0.75 * t)).into_any_element()
        } else {
            chevron.into_any_element()
        };
        let mono = theme.font_terminal.clone();
        // Header row: chevron + mono path + right-aligned +N / −N counts on a slightly raised wash. The header carries the
        // section separator.
        div()
            .id(SharedString::from(format!("{}-{ix}", presentation.key_prefix())))
            .w_full()
            .h(px(FILE_HEADER_HEIGHT))
            .when(!sticky && ix > 0, |el| el.border_t_1().border_color(theme.wash(0.04)))
            .when(sticky, |el| el.border_b_1().border_color(theme.border).block_mouse_except_scroll())
            .flex_none()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .bg(theme.wash(0.025))
            .when(self.keys && self.cursor.is_some() && self.cursor == self.row_ranges.get(ix).map(|r| r.start), |el| el.bg(theme.selected))
            .cursor_pointer()
            .hover(move |s| s.bg(hover_bg))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.cursor = this.row_ranges.get(ix).map(|r| r.start);
                this.toggle_fold(ix, cx)
            }))
            .on_mouse_down(MouseButton::Right, {
                let path = file.path.clone();
                cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                    this.file_menu = Some(crate::ui::Popup::new((path.clone(), e.position)));
                    cx.notify();
                })
            })
            .child(chevron)
            .child(crate::ui::file_icons::icon(&file.path, theme.appearance).size(px(14.0)).flex_none())
            .child(div().flex_1().min_w_0().truncate().font_family(mono.clone()).text_size(px(13.0)).text_color(theme.text_muted).child(SharedString::from(file.path.clone())))
            .when(file.binary, |el| el.child(div().flex_none().text_size(px(10.0)).text_color(theme.text_faint).child("BIN")))
            .when(adds > 0 || !file.binary, |el| el.child(count_label(format!("+{adds}"), theme.diff_add, theme)))
            .when(dels > 0 || !file.binary, |el| el.child(count_label(format!("−{dels}"), theme.diff_del, theme)))
            .into_any_element()
    }

    fn render_sticky_file_header(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let scroll_top = self.list.logical_scroll_top();
        let sticky = sticky_file_header(&self.row_ranges, scroll_top.item_ix, f32::from(scroll_top.offset_in_item))?;
        let files = self.parsed.as_ref()?.files.clone();
        let file = files.get(sticky.file_ix)?;
        let fold = self.folds.get(&file.path).copied().unwrap_or_default();
        let next_header_y = sticky.next_header_row.and_then(|row| {
            let bounds = self.list.bounds_for_item(row)?;
            Some(f32::from(bounds.origin.y - self.list.viewport_bounds().origin.y))
        });
        let top_offset = sticky_header_push_offset(next_header_y);
        let header = self.render_file_header(sticky.file_ix, file, &fold, FileHeaderPresentation::Sticky, theme, cx);
        // opaque: rows scroll under it
        Some(div().absolute().top(px(top_offset)).left_0().w_full().bg(theme.surface).child(header).into_any_element())
    }

    /// A latched toggle holds the wash and the full text tone, so the pane says which layout it is in without a label.
    fn header_toggle(id: &'static str, icon: &'static str, label: &'static str, active: bool, theme: &Theme) -> Stateful<Div> {
        let hover = theme.wash(0.14);
        div()
            .id(id)
            .size(px(CONTROL_SIZE))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(CONTROL_RADIUS))
            .cursor_pointer()
            .when(active, |el| el.bg(hover))
            .hover(move |s| s.bg(hover))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .tooltip(tip(label, None))
            .child(gpui::svg().path(icon).size(px(ICON_SIZE)).text_color(if active { theme.text } else { theme.text_muted.opacity(0.7) }))
    }

    /// The pane-header controls: scope dropdown, `{branch} → {base ⌄}` ref selector (branch scope), and on the right
    /// discard (working tree), layout, wrap and fold-all.
    fn render_header_controls(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let scope = self.scope;
        let hover = theme.wash(0.14);
        let scope_trigger = div()
            .id("changes-scope-trigger")
            .h(px(CONTROL_SIZE))
            .px(px(8.0))
            .flex_none()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .rounded(px(CONTROL_RADIUS))
            .cursor_pointer()
            .bg(theme.wash(0.05))
            .hover(move |s| s.bg(hover))
            .on_click(|_, _, cx| cx.stop_propagation())
            // capture, like the sidebar's `…`: a press on the open menu's trigger closes it
            .capture_any_mouse_down(cx.listener(|this, e: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                match this.scope_menu.as_mut() {
                    Some(menu) if menu.is_open() => menu.close(),
                    _ => {
                        this.scope_menu = Some(crate::ui::Popup::new(menu_at(e.position)));
                        window.focus(&this.scope_focus, cx);
                    }
                }
                cx.notify();
            }))
            .child(div().text_size(px(12.0)).line_height(px(14.0)).text_color(theme.text).child(scope.label()))
            .child(gpui::svg().path("icons/alt-arrow-down.svg").size(px(12.0)).text_color(theme.text_muted.opacity(0.7)));
        let discard = self.discard_request();
        let enabled = discard.is_some();
        let discard = Self::header_toggle("changes-discard-working-tree", "icons/solar/trash-bin-minimalistic.svg", "Discard changes", false, theme)
            .when(!enabled, |button| button.opacity(0.35))
            .when_some(discard, |button, (dir, checksum)| {
                button.on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    cx.emit(ChangesEvent::Discard { dir: dir.clone(), target: DiscardTarget::Tree { checksum: checksum.clone() } });
                }))
            });
        let split = Self::header_toggle("changes-split", "icons/split-columns.svg", "Split view", self.mode.is_split(), theme).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_mode(cx)));
        let wrap = Self::header_toggle("changes-wrap", "icons/wrap-text.svg", "Wrap long lines", self.wrap_lines, theme).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_wrap(cx)));
        let fold_all = Self::header_toggle("changes-fold-all", "icons/fold-vertical.svg", self.fold_all_label(), false, theme).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_collapse_all(cx)));
        let trailing = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(CONTROL_GAP))
            .when(scope == DiffScope::WorkingTree, |el| el.child(discard))
            .child(split)
            .child(wrap)
            .child(fold_all);
        crate::ui::surface_chrome::toolbar(theme)
            .child(scope_trigger)
            .children(self.render_ref_selector(theme, cx))
            .child(div().flex_1())
            .child(trailing)
    }

    fn render_scope_menu(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.scope_menu.as_ref()?;
        let at = menu.value;
        let current = self.scope;
        let rows = DiffScope::ALL.into_iter().enumerate().map(|(ix, scope)| {
            row(theme, scope == current, ("changes-scope-row", ix))
                .h(px(28.))
                .rounded(px(6.))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.set_scope(scope, cx);
                    this.close_scope_menu(cx);
                }))
                .child(div().flex_1().child(scope.label()))
        });
        let card = menu_card(theme, "changes-scope-menu", 180.)
            .track_focus(&self.scope_focus)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.close_scope_menu(cx);
                }
            }))
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.close_scope_menu(cx)))
            .flex()
            .flex_col()
            .gap(px(2.))
            .children(rows);
        let card = crate::ui::menu_motion("changes-scope-menu-in", menu, card);
        Some(gpui::deferred(gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(card)).with_priority(1).into_any_element())
    }

    /// A file header's right-click menu. Only the working tree can be discarded; a deleted file has nothing to open.
    fn render_file_menu(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        type Action = Box<dyn Fn(&mut Changes, &mut Context<Changes>)>;
        let menu = self.file_menu.as_ref()?;
        let (path, at) = menu.value.clone();
        let file = self.parsed.as_ref()?.files.iter().find(|file| file.path == path)?.clone();
        let (dir, root) = (self.dir.clone()?, self.diff.as_ref()?.root.clone());
        let on_disk = file.status != FileStatus::Deleted;
        let local = crate::herdr::split(&dir).0.is_none();
        let discard = self.discard_request().is_some();
        let item = |key: &'static str, icon: &'static str, color: Hsla, label: &'static str, action: Action| {
            row(theme, false, key).h(px(28.)).rounded(px(6.)).child(menu_mark(icon, color)).child(label).on_pick(
                at,
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    if let Some(menu) = this.file_menu.as_mut() {
                        menu.close();
                    }
                    action(this, cx);
                    cx.notify();
                }),
            )
        };
        let preview = {
            let (dir, root, path) = (dir.clone(), root.clone(), path.clone());
            let image = on_disk && crate::terminal::pane::is_image(std::path::Path::new(&path));
            image.then(|| item("changes-file-preview", "icons/solar/gallery.svg", theme.text_muted, "Preview", Box::new(move |_, cx| cx.emit(ChangesEvent::Preview { dir: dir.clone(), root: root.clone(), path: path.clone() }))))
        };
        let open = {
            let (dir, root, path) = (dir.clone(), root.clone(), path.clone());
            on_disk.then(|| item("changes-file-open", "icons/solar/pen.svg", theme.text_muted, "Open in Editor", Box::new(move |_, cx| cx.emit(ChangesEvent::Open { dir: dir.clone(), root: root.clone(), path: path.clone() }))))
        };
        let reveal = {
            let shown = crate::changes::file_location(&dir, &root, &path).1;
            let label = if cfg!(target_os = "macos") { "Reveal in Finder" } else { "Show in file manager" };
            (on_disk && local).then(|| item("changes-file-reveal", "icons/solar/folder-open.svg", theme.text_muted, label, Box::new(move |_, cx| cx.reveal_path(&shown))))
        };
        let copied = path.clone();
        let copy = item("changes-file-copy", "icons/solar/copy.svg", theme.text_muted, "Copy Path", Box::new(move |_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(copied.clone()))));
        let discard = discard.then(|| {
            let file = file.clone();
            item("changes-file-discard", "icons/trash.svg", theme.danger, "Discard Changes…", Box::new(move |_, cx| cx.emit(ChangesEvent::Discard { dir: dir.clone(), target: DiscardTarget::File(file.clone()) }))).text_color(theme.danger)
        });
        let card = menu_card(theme, "changes-file-menu", 180.)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if let Some(menu) = this.file_menu.as_mut() {
                    menu.close();
                }
                cx.notify();
            }))
            .flex()
            .flex_col()
            .gap(px(2.))
            .text_size(px(12.))
            .children(preview)
            .children(open)
            .children(reveal)
            .child(copy)
            .when(discard.is_some(), |el| el.child(div().mx(px(-4.)).my(px(4.)).h(px(1.)).bg(theme.border)))
            .children(discard);
        let card = crate::ui::menu_motion("changes-file-menu-in", menu, card);
        Some(gpui::deferred(gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(card)).with_priority(1).into_any_element())
    }

    /// `{branch} → {base ⌄}`: which ref the branch scope compares against. Branch scope only.
    fn render_ref_selector(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.scope != DiffScope::Branch {
            return None;
        }
        let branch = self.branch.clone().unwrap_or_else(|| "HEAD".to_string());
        let base = self.base_ref.clone().unwrap_or_else(|| "…".to_string());
        // Even truncation: taffy shrinks flex items ∝ factor × basis, so a long branch stayed near-whole while a short base
        // ("main") read as a bare ellipsis. Weighting each side by its own length SQUARED lands the deficit on the longer.
        let branch_weight = (branch.chars().count().max(1) as f32).powi(2);
        let base_weight = (base.chars().count().max(1) as f32).powi(2);
        let hover = theme.wash(0.12);
        let mut trigger = div()
            .id("changes-ref-trigger")
            .h(px(CONTROL_SIZE))
            .px(px(6.0))
            // Shrinkable, like the branch label beside it: both sides truncate instead of plowing over the buttons.
            .min_w_0()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(4.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .on_click(|_, _, cx| cx.stop_propagation())
            .capture_any_mouse_down(cx.listener(|this, e: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                match this.ref_menu.as_mut() {
                    Some(menu) if menu.is_open() => this.close_ref_menu(cx),
                    _ => this.open_ref_menu(menu_at(e.position), window, cx),
                }
            }))
            .child(div().min_w_0().truncate().font_family(theme.font_terminal.clone()).text_size(px(11.5)).text_color(theme.text).child(base))
            .child(gpui::svg().path("icons/alt-arrow-down.svg").size(px(11.0)).flex_none().text_color(theme.text_muted.opacity(0.7)));
        trigger.style().flex_shrink = Some(base_weight);
        let mut branch_label = div().min_w_0().truncate().font_family(theme.font_terminal.clone()).text_size(px(11.5)).text_color(theme.text_muted).child(branch);
        branch_label.style().flex_shrink = Some(branch_weight);
        Some(
            div()
                .min_w_0()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .ml(px(CONTROL_GAP))
                .child(branch_label)
                .child(gpui::svg().path("icons/arrow-right.svg").size(px(12.0)).flex_none().text_color(theme.text_faint))
                .child(trigger)
                .into_any_element(),
        )
    }

    fn render_ref_menu(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.ref_menu.as_ref()?;
        let (search, active, at) = (menu.value.search.clone(), menu.value.active, menu.value.at);
        let rows = self.ref_menu_rows(cx);
        let list: AnyElement = if rows.is_empty() {
            div().px(px(8.0)).py(px(6.0)).text_size(px(12.0)).text_color(theme.text_faint).child(if self.branches.is_empty() { "No branches" } else { "No matching branches" }).into_any_element()
        } else {
            let items = rows.into_iter().enumerate().map(|(row_ix, branch_ix)| {
                let name = self.branches[branch_ix].clone();
                let selected = self.base_ref.as_deref() == Some(name.as_str());
                let label = name.clone();
                row(theme, selected || row_ix == active, ("changes-ref-row", row_ix))
                    .h(px(28.))
                    .rounded(px(6.))
                    .flex_none()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.set_base_ref(name.clone(), cx);
                        this.close_ref_menu(cx);
                    }))
                    .child(div().flex_1().min_w_0().truncate().font_family(theme.font_terminal.clone()).text_size(px(12.0)).child(label))
            });
            div().id("changes-ref-list").flex().flex_col().gap(px(2.0)).max_h(px(240.0)).overflow_y_scroll().track_scroll(&self.ref_scroll).children(items).into_any_element()
        };
        let field = div().px(px(8.)).py(px(6.)).mb(px(4.)).border_b_1().border_color(theme.border).child(Input::new(&search));
        let card = menu_card(theme, "changes-ref-menu", 240.)
            .w(px(240.))
            .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| this.ref_menu_key(event, cx)))
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.close_ref_menu(cx)))
            .flex()
            .flex_col()
            .child(field)
            .child(list);
        let card = crate::ui::menu_motion("changes-ref-menu-in", menu, card);
        Some(gpui::deferred(gpui::anchored().position(at).snap_to_window_with_margin(px(8.)).child(card)).with_priority(1).into_any_element())
    }

    fn render_header_strip(&self, theme: &Theme) -> Option<AnyElement> {
        let parsed = self.parsed.as_ref()?;
        Some(
            div()
                .flex_none()
                .h(px(HEADER_HEIGHT))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .px(px(16.0))
                .border_b_1()
                .border_color(theme.wash(0.06))
                .child(div().min_w_0().truncate().text_size(px(12.0)).text_color(theme.text_muted).child(scope_label(self.scope, parsed.files.len(), self.base_ref.as_deref())))
                .child(count_label(format!("+{}", parsed.additions), theme.diff_add, theme))
                .child(count_label(format!("−{}", parsed.deletions), theme.diff_del, theme))
                .child(div().flex_1())
                .when(parsed.truncated, |el| {
                    el.child(div().flex_none().text_size(px(10.0)).px(px(6.0)).py(px(2.0)).rounded(px(4.0)).bg(theme.warning.opacity(0.08)).text_color(theme.warning.opacity(0.75)).child("Partial snapshot"))
                })
                .into_any_element(),
        )
    }
}

/// Where a toolbar's menu opens: under the toolbar, at the press.
fn menu_at(press: gpui::Point<Pixels>) -> gpui::Point<Pixels> {
    gpui::point(press.x - px(8.), px(TITLEBAR_HEIGHT + HEADER_HEIGHT - 4.))
}

fn count_label(text: String, color: Hsla, theme: &Theme) -> Div {
    div().flex_none().font_family(theme.font_terminal.clone()).text_size(px(11.0)).text_color(color).child(text)
}

/// One notice row ("New file", "Binary file — contents not shown", …).
fn notice_row(notice: String, theme: &Theme) -> AnyElement {
    div().h(px(NOTICE_HEIGHT)).w_full().flex_none().flex().items_center().px(px(16.0)).text_size(px(11.0)).text_color(theme.text_faint).child(notice).into_any_element()
}

/// One `@@ … @@` hunk-header row on the bluish-grey wash.
fn hunk_header_row(header: &str, theme: &Theme) -> AnyElement {
    div()
        .h(px(HUNK_HEADER_HEIGHT))
        .w_full()
        .flex_none()
        .flex()
        .items_center()
        .px(px(16.0))
        .bg(theme.diff_hunk)
        .font_family(theme.font_terminal.clone())
        .text_size(px(12.0))
        .text_color(theme.text_faint)
        .child(header.to_string())
        .into_any_element()
}

/// The only part of a diff row allowed to exceed its viewport. The outer element keeps row chrome fixed; the inner
/// element owns the intrinsic code width and is the only plane moved by the file's horizontal scroll handle.
fn code_text_viewport(text: String, runs: Vec<gpui::TextRun>, theme: &Theme, padding_left: f32, content_width: Option<f32>, wrapped: bool, scroll: DiffCodeScroll) -> AnyElement {
    let content = div()
        .when(wrapped, |el| el.w_full().min_w_0())
        // Keep every tracked row's scroll extent identical. The width already includes shaping slack on the right, so
        // clipping here only prevents a child from redefining the shared maximum.
        .when_some(content_width, |el, width| el.w(px(width)).flex_none().overflow_hidden())
        .pl(px(padding_left))
        .font_family(theme.font_terminal.clone())
        .text_size(px(DIFF_TEXT_SIZE))
        .line_height(px(DIFF_LINE_HEIGHT))
        .map(|el| if wrapped { el.whitespace_normal() } else { el.whitespace_nowrap() })
        .child(gpui::StyledText::new(text).with_runs(runs));
    let viewport = div().flex_1().min_w_0().min_h(px(DIFF_LINE_HEIGHT)).overflow_hidden().child(content);
    if wrapped {
        return viewport.into_any_element();
    }
    let mut viewport = viewport.id(scroll.id).overflow_x_scroll().track_scroll(&scroll.handle);
    // Without this GPUI maps a vertical-only wheel delta onto x for an x-only scroller, starving the virtualized list
    // underneath.
    viewport.style().restrict_scroll_to_axis = Some(true);
    viewport.into_any_element()
}

/// What sets a line's row apart: its marker, the marker's and the numbers' color, its tint and accent bar.
struct LineTone {
    marker: &'static str,
    marker_color: Hsla,
    row_bg: Option<Hsla>,
    accent: Option<Hsla>,
    number_color: Hsla,
}

fn line_tone(kind: LineKind, theme: &Theme) -> LineTone {
    // Row tints sampled from the reference: ~5–6% washes over the pane tone.
    let tinted = |marker, color: Hsla| LineTone { marker, marker_color: color, row_bg: Some(color.opacity(0.055)), accent: Some(color.opacity(0.55)), number_color: color.opacity(0.9) };
    match kind {
        LineKind::Add => tinted("+", theme.diff_add),
        LineKind::Del => tinted("−", theme.diff_del),
        _ => LineTone { marker: "·", marker_color: theme.text_faint.opacity(0.5), row_bg: None, accent: None, number_color: theme.text_faint.opacity(0.8) },
    }
}

fn gutter(no: Option<u32>, color: Hsla, gutter_px: f32, theme: &Theme) -> Div {
    div()
        .w(px(gutter_px))
        .flex_none()
        .font_family(theme.font_terminal.clone())
        .text_size(px(12.0))
        .line_height(px(DIFF_LINE_HEIGHT))
        .text_color(color)
        .flex()
        .justify_end()
        .pr(px(8.0))
        .child(no.map(|n| n.to_string()).unwrap_or_default())
}

fn marker_cell(tone: &LineTone, width: f32, theme: &Theme) -> Div {
    div()
        .w(px(width))
        .flex_none()
        .flex()
        .justify_center()
        .text_size(px(DIFF_TEXT_SIZE))
        .line_height(px(DIFF_LINE_HEIGHT))
        .text_color(tone.marker_color)
        .font_family(theme.font_terminal.clone())
        .child(tone.marker)
}

fn accent_bar(tone: &LineTone) -> Div {
    // solid on +/− rows, an invisible spacer on context rows so columns always align
    div().w(px(ACCENT_BAR_WIDTH)).self_stretch().flex_none().when_some(tone.accent, |el, color| el.bg(color))
}

/// One +/−/context/meta diff line: coloured accent bar, dual line-number gutters, marker column, and paint-only syntax
/// runs.
fn diff_line_row(line: &DiffLine, highlights: Option<&DiffHighlights>, theme: &Theme, gutter_px: f32, code_width: DiffCodeWidth, scroll: DiffCodeScroll) -> AnyElement {
    if line.kind == LineKind::Meta {
        return meta_line_row(&line.text, theme, ACCENT_BAR_WIDTH + 2.0 * gutter_px + MARKER_WIDTH + 12.0);
    }
    let tone = line_tone(line.kind, theme);
    let idle = theme.text_faint.opacity(0.8);
    let content_width = match code_width {
        DiffCodeWidth::Scrollable(metrics) => Some(metrics.unified_content_width(gutter_px)),
        DiffCodeWidth::Wrapped => None,
    };
    let wrapped = matches!(code_width, DiffCodeWidth::Wrapped);
    div()
        .map(|el| if wrapped { el.min_h(px(DIFF_LINE_HEIGHT)) } else { el.h(px(DIFF_LINE_HEIGHT)) })
        .w_full()
        .flex_none()
        .flex()
        .flex_row()
        .items_start()
        .when_some(tone.row_bg, |el, bg| el.bg(bg))
        .child(accent_bar(&tone))
        .child(gutter(line.old_no, if line.kind == LineKind::Del { tone.number_color } else { idle }, gutter_px, theme))
        .child(gutter(line.new_no, if line.kind == LineKind::Add { tone.number_color } else { idle }, gutter_px, theme))
        .child(marker_cell(&tone, MARKER_WIDTH, theme))
        .child(code_text_viewport(line.text.clone(), line_runs(line, highlights, theme), theme, UNIFIED_CODE_PADDING_LEFT, content_width, wrapped, scroll))
        .into_any_element()
}

/// `\ No newline at end of file` and friends: a note about the row rather than code, so it is indented past the columns
/// and never tinted. In split mode it spans both halves.
fn meta_line_row(text: &str, theme: &Theme, pad_left: f32) -> AnyElement {
    div().h(px(DIFF_LINE_HEIGHT)).w_full().flex_none().flex().items_center().pl(px(pad_left)).text_size(px(10.5)).text_color(theme.text_faint).italic().child(text.to_string()).into_any_element()
}

/// The paint-only syntax runs for one diff line.
pub(super) fn line_runs(line: &DiffLine, highlights: Option<&DiffHighlights>, theme: &Theme) -> Vec<gpui::TextRun> {
    let spans = highlights.map(|h| h.spans(line)).unwrap_or(&[]);
    let mono = font(theme.font_terminal.clone());
    let plain_color = theme.text.opacity(0.92);
    let plain = |len: usize| gpui::TextRun { len, font: mono.clone(), color: plain_color, background_color: None, underline: None, strikethrough: None };
    let mut runs = Vec::new();
    let mut at = 0usize;
    for span in spans {
        if span.range.start > at {
            runs.push(plain(span.range.start - at));
        }
        let mut run = plain(span.range.len());
        run.color = crate::ui::markdown::syntax_color(span.kind, theme);
        runs.push(run);
        at = span.range.end;
    }
    if at < line.text.len() {
        runs.push(plain(line.text.len() - at));
    }
    runs.retain(|run| run.len > 0);
    runs
}

/// One half of a split row: the same accent bar / gutter / marker / code columns a unified row uses, minus the second
/// gutter. Takes prebuilt `runs` so a mirrored row can share one set across both columns.
fn split_line_cell(line: &DiffLine, number: Option<u32>, runs: Vec<gpui::TextRun>, theme: &Theme, gutter_px: f32, code_width: DiffCodeWidth, scroll: DiffCodeScroll) -> Div {
    let tone = line_tone(line.kind, theme);
    let content_width = match code_width {
        DiffCodeWidth::Scrollable(metrics) => Some(metrics.split_content_width(gutter_px)),
        DiffCodeWidth::Wrapped => None,
    };
    let wrapped = matches!(code_width, DiffCodeWidth::Wrapped);
    div()
        .flex_1()
        .min_w_0()
        .self_stretch()
        .overflow_hidden()
        .flex()
        .flex_row()
        .items_start()
        .when_some(tone.row_bg, |el, bg| el.bg(bg))
        .child(accent_bar(&tone))
        .child(gutter(number, tone.number_color, gutter_px, theme))
        .child(marker_cell(&tone, SPLIT_MARKER_WIDTH, theme))
        .child(code_text_viewport(line.text.clone(), runs, theme, SPLIT_CODE_PADDING_LEFT, content_width, wrapped, scroll))
}

/// The empty half of a one-sided split row. A flat wash, quieter than either tint, reads as "nothing here".
fn split_filler(theme: &Theme) -> Div {
    div().flex_1().min_w_0().self_stretch().bg(theme.wash(0.03))
}

/// Compose the two halves with the centre hairline.
fn split_row(left: AnyElement, right: AnyElement, wrapped: bool, theme: &Theme) -> Div {
    div()
        .map(|el| if wrapped { el.min_h(px(DIFF_LINE_HEIGHT)) } else { el.h(px(DIFF_LINE_HEIGHT)) })
        .w_full()
        .flex_none()
        .flex()
        .flex_row()
        .items_stretch()
        .child(left)
        .child(div().w(px(SPLIT_DIVIDER_WIDTH)).self_stretch().flex_none().bg(theme.wash(0.06)))
        .child(right)
}

/// A split row's `+` only ever appears in the right column, which carries one gutter, so the offset is the same for
/// every line. It is measured from the column: the halves are fluid.
fn split_adder_left(gutter_px: f32) -> f32 {
    ACCENT_BAR_WIDTH + (gutter_px - COMMENT_ADDER_SIZE) / 2.0
}

/// A unified row carries both gutters side by side, and a deletion numbers in the first.
fn comment_adder_left(side: CommentSide, gutter_px: f32) -> f32 {
    let column = match side {
        CommentSide::Old => 0.0,
        CommentSide::New => gutter_px,
    };
    ACCENT_BAR_WIDTH + column + (gutter_px - COMMENT_ADDER_SIZE) / 2.0
}

fn positioned_adder(left: f32, adder: AnyElement) -> Div {
    div().absolute().left(px(left)).top(px(0.0)).h_full().flex().items_center().child(adder)
}

fn render_comment_adder(path: &str, side: CommentSide, line: u32, theme: &Theme, cx: &mut Context<Changes>) -> AnyElement {
    let target = path.to_string();
    div()
        .id(SharedString::from(format!("cmt-add-{path}-{}-{line}", side.tag())))
        .size(px(COMMENT_ADDER_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .bg(theme.text)
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
            cx.stop_propagation();
            this.open_draft(target.clone(), side, line, None, window, cx);
        }))
        .tooltip(tip("Add comment", None))
        .child(gpui::svg().path("icons/plus.svg").size(px(11.0)).text_color(theme.surface))
        .into_any_element()
}

fn comment_bar(color: Hsla) -> Div {
    // A bar, not a border: it must match ACCENT_BAR_WIDTH exactly or the card's edge steps in and out of the column.
    div().w(px(ACCENT_BAR_WIDTH)).h_full().flex_none().bg(color)
}

fn comment_head(text: String, theme: &Theme) -> Div {
    div()
        .h(px(CARD_HEADER_HEIGHT))
        .flex_none()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .child(gpui::svg().path("icons/solar/chat-round-line.svg").size(px(12.0)).flex_none().text_color(theme.text_faint))
        .child(div().flex_1().min_w_0().truncate().font_family(theme.font_terminal.clone()).text_size(px(11.0)).text_color(theme.text_faint).child(text))
}

fn comment_card_action(id: String, icon: &'static str, label: &'static str, group: SharedString, theme: &Theme) -> Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .flex_none()
        .size(px(16.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .cursor_pointer()
        .opacity(0.0)
        .group_hover(group, |style| style.opacity(1.0))
        .tooltip(tip(label, None))
        .child(gpui::svg().path(icon).size(px(12.0)).text_color(theme.text_muted))
}

fn render_comment_card(comment: &ReviewComment, theme: &Theme, cx: &mut Context<Changes>) -> AnyElement {
    let group: SharedString = format!("cmt-card-{}", comment.id).into();
    let id = comment.id;
    let edit = comment_card_action(format!("cmt-edit-{id}"), "icons/solar/pen.svg", "Edit comment", group.clone(), theme)
        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.edit_comment(id, window, cx)));
    let remove = comment_card_action(format!("cmt-remove-{id}"), "icons/close-circle.svg", "Remove comment", group.clone(), theme)
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.remove_comment(id, cx)));
    div()
        .group(group)
        .h(px(card_height(&comment.body)))
        .w_full()
        .flex_none()
        .flex()
        .flex_row()
        .bg(theme.wash(0.05))
        .child(comment_bar(theme.text.opacity(0.35)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .px(px(16.0))
                .py(px(CARD_PAD_V / 2.0))
                .child(comment_head(comment.location(), theme).child(edit).child(remove))
                // Height is analytic, so an over-long body clips inside the card rather than past the fold height.
                .child(div().flex_1().min_h_0().overflow_hidden().text_size(px(12.0)).line_height(px(CARD_LINE_HEIGHT)).text_color(theme.text_muted).child(comment.body.clone())),
        )
        .into_any_element()
}

/// Fixed height, so an open draft never fights the fold tween.
fn render_comment_draft(draft: &CommentDraft, theme: &Theme, cx: &mut Context<Changes>) -> AnyElement {
    // Mirrors `ReviewComment::cite_path` for the not-yet-staged note.
    let cite = match draft.side {
        CommentSide::Old => draft.old_path.as_deref().unwrap_or(&draft.path),
        CommentSide::New => &draft.path,
    };
    let action = |id: &'static str, label: &'static str, primary: bool| {
        let hover = theme.hover;
        div()
            .id(id)
            .h(px(22.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .rounded(px(6.0))
            .text_size(px(11.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .cursor_pointer()
            .when(primary, |el| el.bg(theme.text).text_color(theme.surface))
            .when(!primary, |el| el.text_color(theme.text_muted).hover(move |s| s.bg(hover)))
            .child(label)
    };
    div()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
            if event.keystroke.key == "escape" {
                cx.stop_propagation();
                this.cancel_draft(cx);
            }
        }))
        .h(px(DRAFT_CARD_HEIGHT))
        .w_full()
        .flex_none()
        .flex()
        .flex_row()
        .bg(theme.wash(0.08))
        .child(comment_bar(theme.text.opacity(0.7)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .px(px(16.0))
                .py(px(10.0))
                .child(comment_head(format!("{cite}:{}", draft.line), theme))
                .child(div().h(px(46.0)).flex_none().overflow_hidden().text_size(px(12.0)).child(Textarea::new(&draft.input)))
                .child(
                    div()
                        .h(px(28.0))
                        .flex_none()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_end()
                        .gap(px(6.0))
                        .child(action("cmt-cancel", "Cancel", false).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.cancel_draft(cx))))
                        .child(action("cmt-commit", if draft.editing_id.is_some() { "Save" } else { "Comment" }, true).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.commit_draft(cx)))),
                ),
        )
        .into_any_element()
}

/// Build only rows that start above `max_px` so the fold tween's stand-in never materializes lines its clip cannot
/// reveal.
fn render_file_body_upto(file: &FileDiff, highlight: Option<Arc<DiffHighlights>>, theme: &Theme, max_px: f32, mode: DiffMode, code_width: DiffCodeWidth, scroll: DiffCodeScrollContext) -> AnyElement {
    let mut children: Vec<AnyElement> = Vec::new();
    let mut y = 0.0f32;
    let gutter_px = gutter_width(file);
    let wrapped = matches!(code_width, DiffCodeWidth::Wrapped);
    'build: {
        for notice in file_notices(file) {
            if y >= max_px {
                break 'build;
            }
            children.push(notice_row(notice, theme));
            y += NOTICE_HEIGHT;
        }
        for (hunk_ix, hunk) in file.hunks.iter().enumerate() {
            if y >= max_px {
                break 'build;
            }
            children.push(hunk_header_row(&hunk.header, theme));
            y += HUNK_HEADER_HEIGHT;
            match mode {
                DiffMode::Unified => {
                    for (line_ix, line) in hunk.lines.iter().enumerate() {
                        if y >= max_px {
                            break 'build;
                        }
                        children.push(diff_line_row(line, highlight.as_deref(), theme, gutter_px, code_width, scroll.slot(format_args!("{hunk_ix}-{line_ix}"))));
                        y += DIFF_LINE_HEIGHT;
                    }
                }
                DiffMode::Split => {
                    // Pair only what the clip can still reveal: the unified arm breaks out of a lazy walk, so the split arm
                    // must not materialize the whole hunk first.
                    let budget = ((max_px - y) / DIFF_LINE_HEIGHT).ceil().max(0.0) as usize;
                    for (pair_ix, (left, right)) in split_pairs_upto(&hunk.lines, budget).into_iter().enumerate() {
                        if y >= max_px {
                            break 'build;
                        }
                        let line_at = |slot: Option<u32>| slot.and_then(|slot| hunk.lines.get(slot as usize));
                        let cell = |line: Option<&DiffLine>, old: bool| match line {
                            Some(line) => split_line_cell(
                                line,
                                if old { line.old_no } else { line.new_no },
                                line_runs(line, highlight.as_deref(), theme),
                                theme,
                                gutter_px,
                                code_width,
                                scroll.slot(format_args!("{hunk_ix}-{pair_ix}-{}", if old { "old" } else { "new" })),
                            )
                            .into_any_element(),
                            None => split_filler(theme).into_any_element(),
                        };
                        let (left, right) = (line_at(left), line_at(right));
                        let marker = [left, right].into_iter().flatten().find(|line| line.kind == LineKind::Meta);
                        children.push(match marker {
                            Some(line) => meta_line_row(&line.text, theme, 2.0 * (ACCENT_BAR_WIDTH + gutter_px)),
                            None => split_row(cell(left, true), cell(right, false), wrapped, theme).into_any_element(),
                        });
                        y += DIFF_LINE_HEIGHT;
                    }
                }
            }
        }
    }
    div().flex().flex_col().pb(px(BODY_BOTTOM_PAD)).children(children).into_any_element()
}

impl Render for Changes {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::ui::reap_popup(&mut self.scope_menu, window);
        crate::ui::reap_popup(&mut self.ref_menu, window);
        crate::ui::reap_popup(&mut self.file_menu, window);
        if std::mem::take(&mut self.refocus)
            && let Some(back) = &self.back
        {
            window.focus(back, cx);
        }
        let theme = Theme::of(cx).clone();
        let notice = |text: SharedString, warn: bool| {
            div().flex_1().flex().items_center().justify_center().px(px(16.0)).text_size(px(12.0)).text_color(if warn { theme.warning.opacity(0.85) } else { theme.text_faint }).child(text).into_any_element()
        };
        let toolbar = self.render_header_controls(&theme, cx);
        let content = match (&self.dir, &self.diff, &self.error) {
            (None, _, _) => notice("The focused pane isn't in a git repo.".into(), false),
            (_, None, Some(error)) => notice(error.clone(), true),
            (_, None, None) => div().flex_1().flex().items_center().justify_center().child(crate::ui::loader::spinner(2.5, window)).into_any_element(),
            (_, Some(diff), _) if diff.patch.trim().is_empty() => notice(clean_message(self.scope, self.base_ref.as_deref()).into(), false),
            _ if self.parsed.is_none() => div().flex_1().flex().items_center().justify_center().child(crate::ui::loader::spinner(2.5, window)).into_any_element(),
            _ => {
                let sticky_header = self.render_sticky_file_header(&theme, cx);
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .children(self.render_header_strip(&theme))
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h_0()
                            .overflow_hidden()
                            .child(list(self.list.clone(), cx.processor(Self::render_row)).size_full().with_sizing_behavior(gpui::ListSizingBehavior::Auto))
                            .when_some(sticky_header, |el, header| el.child(header))
                            .child(crate::ui::scrollbar("changes-scrollbar", &self.list, &theme)),
                    )
                    // what the keys do, while they're here
                    .when(self.keys, |el| {
                        el.child(
                            div()
                                .flex_none()
                                .px(px(12.0))
                                .py(px(4.0))
                                .border_t_1()
                                .border_color(theme.border)
                                .text_size(px(11.0))
                                .text_color(theme.text_faint)
                                .truncate()
                                .child(if cfg!(target_os = "macos") {
                                    "j k move · l open · h close · c comment · ⌘↵ send · ⇧j ⇧k scroll · esc back"
                                } else {
                                    "j k move · l open · h close · c comment · ctrl+↵ send · ⇧j ⇧k scroll · esc back"
                                }),
                        )
                    })
                    .into_any_element()
            }
        };
        // a capture failing over one that holds: its content stays under the word
        let banner = self.error.clone().filter(|_| self.diff.is_some());
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(toolbar)
            .when_some(banner, |el, message| {
                el.child(div().flex_none().px(px(12.0)).py(px(4.0)).border_b_1().border_color(theme.border).text_size(px(11.0)).text_color(theme.warning).child(message))
            })
            .child(content)
            .children(self.render_scope_menu(&theme, cx))
            .children(self.render_ref_menu(&theme, cx))
            .children(self.render_file_menu(&theme, cx))
    }
}

impl Shell {
    /// The picker's Diffs card / the `+` menu's row: every click opens a fresh diff tab with its own scope and base.
    pub(super) fn add_diff_surface(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.right.made_diffs += 1;
        let surface = Surface::diff(self.right.made_diffs);
        let key = surface.key().to_string();
        let tab = key.clone();
        let changes = cx.new(Changes::new);
        changes.update(cx, |c, _| c.back = Some(self.focus.clone()));
        cx.subscribe_in(&changes, window, move |this, _, event: &ChangesEvent, window, cx| match event {
            ChangesEvent::Discard { dir, target } => {
                this.right.discard = Some(crate::ui::Popup::new(DiscardAsk { tab: tab.clone(), dir: dir.clone(), target: target.clone() }));
                cx.notify();
            }
            ChangesEvent::Open { dir, root, path } => this.edit_diff_file(dir, root, path, cx),
            ChangesEvent::Preview { dir, root, path } => this.preview_diff_file(dir, root, path, window, cx),
            ChangesEvent::Send => this.send_diff_comments(&tab, window, cx),
        })
        .detach();
        self.right.diffs.insert(key, changes);
        self.update_panel(cx, |p| p.add(surface));
    }

    /// A diff's file in the user's editor, Neovim opening in a pane beside the tab's focused one.
    fn edit_diff_file(&mut self, dir: &str, root: &str, path: &str, cx: &mut Context<Self>) {
        let (Some(snapshot), Some(tab)) = (self.snapshot.as_ref(), self.shown_tab()) else { return };
        let Some(pane) = snapshot.pane_frames(&tab.tab_id).into_iter().find(|f| f.focused).map(|f| f.pane_id) else { return };
        let (machine, full) = crate::changes::file_location(dir, root, path);
        let cwd = crate::herdr::split(dir).1.to_string();
        let editor = SettingsGlobal::of(cx).settings.editor.clone();
        let backend = self.backend.clone();
        self.rt.spawn(async move { crate::terminal::pane::edit_file(&backend, &pane, &cwd, editor.as_deref(), full, machine).await });
    }

    /// Sees each workspace's turns off its agents' statuses. A turn starting has its checkout snapshotted, and that
    /// becomes the last turn's beginning at the first change: one that changes nothing leaves the last turn alone.
    // ponytail: the first write can beat herdr's `working` into the snapshot, and goes unshown; and a running turn costs
    // a `git add -A` every TURN_POLL until it writes. A filesystem watch (reviewr's) answers both, if either is felt.
    pub(super) fn track_turns(&mut self, cx: &mut Context<Self>) {
        let Some(snapshot) = self.snapshot.clone() else { return };
        for workspace in &snapshot.workspaces {
            let agents: Vec<_> = snapshot.panes.iter().filter(|p| p.workspace_id == workspace.workspace_id && p.agent.is_some()).collect();
            let turn = self.right.turns.entry(workspace.workspace_id.clone()).or_default();
            let edge = turn.turns.observe(agents.iter().map(|p| p.agent_status));
            if edge.ended {
                turn.ended.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            let working = agents.iter().find(|p| p.agent_status == crate::model::AgentStatus::Working);
            let Some(dir) = working.filter(|_| edge.started).and_then(|p| crate::ui::statusbar::pane_dir(Some(&snapshot), Some(&p.pane_id))) else { continue };
            let ended = Arc::new(std::sync::atomic::AtomicBool::new(false));
            turn.ended = ended.clone();
            turn._watch = Some(cx.spawn(async move |_, cx| {
                let snap = dir.clone();
                let Ok(start) = cx.background_spawn(async move { snapshot_tree(&snap) }).await else { return };
                loop {
                    cx.background_executor().timer(TURN_POLL).await;
                    let last = ended.load(std::sync::atomic::Ordering::Relaxed);
                    let (dir, start) = (dir.clone(), start.clone());
                    let moved = cx.background_spawn(async move { turn_moved(&dir, &start) }).await;
                    if last || moved == Ok(true) {
                        return;
                    }
                }
            }));
        }
    }

    /// A diff's image in the panes' image dialog. It's a pane's and takes its keys there, so the keyboard goes along.
    fn preview_diff_file(&mut self, dir: &str, root: &str, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pane) = self.active_pane(window, cx).and_then(|id| self.panes.get(&id)).cloned() else { return };
        let (machine, full) = crate::changes::file_location(dir, root, path);
        self.release_panel();
        window.focus(&pane.focus_handle(cx), cx);
        self.ask(async move { Ok(crate::terminal::pane::image_copy(full, machine).await) }, cx, move |_, copy, cx| match copy {
            Ok(Some(path)) => pane.update(cx, |pane, cx| pane.show_preview(path, cx)),
            _ => crate::ui::toast::error(cx, "Couldn't read this image"),
        });
    }

    /// Tells each diff tab what the shown pane is and whether to watch it: the one shown, while the window is in use.
    pub(super) fn sync_diffs(&mut self, shown: &ShownReview, window: &Window, cx: &mut Context<Self>) {
        let active = self.panel().and_then(|p| p.active.clone()).filter(|_| self.right_panel_shown(window));
        let in_use = window.is_window_active();
        let keys = self.panel_focused() && self.focus.is_focused(window);
        for (id, changes) in &self.right.diffs {
            let here = matches!(&active, Some(Surface::Diff(a)) if a == id);
            changes.update(cx, |changes, cx| {
                if changes.keys != (here && keys) {
                    changes.keys = here && keys;
                    cx.notify();
                }
            });
            changes.update(cx, |changes, cx| changes.show(shown.dir.as_deref().filter(|_| here), shown.branch.as_deref().filter(|_| here), here && in_use, cx));
        }
    }

    /// The agent tab `id`'s comments go to: the one picked for it while it's there; else the shown pane's, it being what
    /// the diff shows; else the workspace's first.
    fn diff_recipient(&self, id: &str, window: &mut Window, cx: &mut Context<Self>) -> Option<crate::model::Pane> {
        let workspace = self.shown_tab()?.workspace_id;
        let pane = self.active_pane(window, cx);
        let agents = self.snapshot.as_ref()?.agents_in(&workspace, SettingsGlobal::of(cx).settings.annotation_agents);
        let picked = self.right.diff_agents.get(id).filter(|picked| agents.iter().any(|p| &p.pane_id == *picked));
        crate::annotations::default_recipient(&agents, picked.map(String::as_str).or(pane.as_deref()), Some(&workspace)).cloned()
    }

    /// Only a click in the footer comes here.
    pub(super) fn send_diff_comments(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(changes) = self.right.diffs.get(id).cloned() else { return };
        // a card still open is a comment too
        changes.update(cx, |changes, cx| changes.commit_draft(cx));
        if changes.read(cx).comment_count() == 0 {
            return;
        }
        let Some(agent) = self.diff_recipient(id, window, cx) else { return crate::ui::toast::error(cx, "No agent here to send the comments to") };
        let Some((dir, prompt, ids)) = changes.update(cx, |changes, _| changes.take_prompt()) else { return };
        let name = agent.agent.clone().unwrap_or_default();
        let (backend, pane) = (self.backend.clone(), agent.pane_id);
        self.ask(async move { backend.mux.send(pane, Some(prompt)).await }, cx, move |_, result, cx| {
            let sent = result.is_ok();
            match result {
                Ok(_) => crate::ui::toast::info(cx, format!("Sent to {name}"), None),
                Err(e) => crate::ui::toast::error(cx, format!("Not sent to {name}: {e:#}")),
            }
            changes.update(cx, |changes, cx| changes.sent(&dir, sent.then_some(ids), cx));
        });
    }

    /// The tab's body: the diff, and its comments' way to an agent while there are any.
    pub(super) fn render_diff(&mut self, id: &str, theme: &Theme, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(changes) = self.right.diffs.get(id).cloned() else { return gpui::Empty.into_any_element() };
        let count = changes.read(cx).comment_count();
        let agent = (count > 0).then(|| self.diff_recipient(id, window, cx)).flatten();
        let agent_tab = agent.as_ref().and_then(|a| self.snapshot.as_ref()?.tab_position(a));
        let footer = (count > 0).then(|| {
            let send_id = id.to_string();
            let picker_id = id.to_string();
            let ready = agent.is_some() && !changes.read(cx).sending;
            let hovered = theme.accent.opacity(0.85);
            let hover = theme.hover;
            let open = self.right.agents.as_ref().is_some_and(|m| m.is_open());
            let chosen = agent.as_ref().map_or_else(
                || div().text_color(theme.text_faint).child("No agent here").into_any_element(),
                |a| {
                    let name = a.agent.clone().unwrap_or_default();
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(status_dot(a.agent_status, theme))
                        .child(div().truncate().text_color(theme.text).child(name))
                        .when_some(agent_tab, |el, n| el.child(div().flex_none().text_color(theme.text_muted).child(format!("tab {n}"))))
                        .into_any_element()
                },
            );
            let picker = div()
                .id("changes-agent")
                .h(px(28.))
                .min_w_0()
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(6.))
                .rounded(px(8.))
                .cursor_pointer()
                .hover(move |s| s.bg(hover))
                .when(open, |el| el.bg(hover))
                .tooltip(tip("Who gets the comments", None))
                .on_click(|_, _, cx| cx.stop_propagation())
                // capture, like the sidebar's `…`: a press on the open menu's trigger closes it
                .capture_any_mouse_down(cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    match this.right.agents.as_mut() {
                        Some(menu) if menu.is_open() => menu.close(),
                        _ => this.right.agents = Some(crate::ui::Popup::new((picker_id.clone(), e.position))),
                    }
                    cx.notify();
                }))
                .child(chosen)
                .child(gpui::svg().path("icons/alt-arrow-down.svg").size(px(12.)).flex_none().text_color(theme.text_muted.opacity(0.7)));
            let noun = if count == 1 { "1 comment".to_string() } else { format!("{count} comments") };
            div()
                .flex_none()
                .h(px(44.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .border_t_1()
                .border_color(theme.border)
                .bg(theme.surface)
                .text_size(px(12.))
                .child(div().flex_1().min_w_0().truncate().text_color(theme.text_muted).child(noun))
                .child(picker)
                .child(
                    div()
                        .id("changes-send")
                        .flex_none()
                        .h(px(28.))
                        .px(px(12.))
                        .flex()
                        .items_center()
                        .rounded(px(8.))
                        .text_color(if ready { gpui::white() } else { theme.text_faint })
                        .bg(if ready { theme.accent } else { theme.wash(0.06) })
                        .child("Send")
                        .when(ready, |el| el.cursor_pointer().hover(move |s| s.bg(hovered)).on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.send_diff_comments(&send_id, window, cx)))),
                )
        });
        div()
            .id("right-panel-diff")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            // keys to the panel, a comment's field, and tab keys; a pane's click takes them back
            // also when the panel had them but a field took them since (a page's address, a tab gone): its own field
            // takes them back on this same press
            .capture_any_mouse_down(cx.listener(|this, _: &MouseDownEvent, window, cx| {
                if !this.panel_focused() || !this.focus.is_focused(window) {
                    this.focus_panel(true, window, cx);
                }
            }))
            .child(div().flex_1().min_h_0().child(changes))
            .children(footer)
            .into_any_element()
    }

    /// The footer's menu: the agents to send to, and which of them to list, as the browser's notes list theirs.
    pub(super) fn diff_agents_menu(&mut self, theme: &Theme, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (id, at) = self.right.agents.as_ref()?.value.clone();
        let chosen = self.diff_recipient(&id, window, cx).map(|p| p.pane_id);
        let workspace = self.shown_tab()?.workspace_id;
        let recipients = self.agents_of(&workspace, cx);
        let scope = SettingsGlobal::of(cx).settings.annotation_agents;
        let (heading, empty) = super::annotations::agents_words(scope);
        let none = recipients.is_empty();
        let agents = recipients.into_iter().map(|(pane, label)| {
            let picked = chosen.as_deref() == Some(pane.pane_id.as_str());
            let (tab, pane_id) = (id.clone(), pane.pane_id.clone());
            super::annotations::agent_row(&pane, label, picked, theme).rounded(px(6.)).on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.right.diff_agents.insert(tab.clone(), pane_id.clone());
                if let Some(m) = this.right.agents.as_mut() {
                    m.close()
                }
                cx.notify();
            }))
        });
        let scopes = super::annotations::SCOPES.map(|(choice, row_id, icon, label)| {
            menu_choice(theme, row_id, icon, label, scope == choice).on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                save_from_sidebar(cx, |s| s.annotation_agents = choice);
                cx.notify();
            }))
        });
        let card = menu_card(theme, "changes-agents-menu", 280.)
            .w(px(280.))
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if let Some(m) = this.right.agents.as_mut() {
                    m.close()
                }
                cx.notify();
            }))
            .flex()
            .flex_col()
            .gap(px(2.))
            .text_size(px(12.))
            .child(div().px(px(8.)).py(px(4.)).text_size(px(11.)).text_color(theme.text_faint).child(heading))
            .child(div().id("changes-agents-list").flex().flex_col().gap(px(2.)).max_h(px(220.)).overflow_y_scroll().children(agents))
            .when(none, |el| el.child(div().px(px(8.)).pb(px(4.)).text_color(theme.text_faint).child(empty)))
            .child(div().my(px(2.)).h(px(1.)).bg(theme.border))
            .children(scopes);
        let menu = self.right.agents.as_ref()?;
        let card = crate::ui::menu_motion("changes-agents-menu-in", menu, card);
        Some(gpui::deferred(gpui::anchored().position(at).anchor(gpui::Anchor::BottomRight).snap_to_window_with_margin(px(8.)).child(card)).with_priority(1).into_any_element())
    }

    fn finish_discard(&mut self, confirmed: bool, cx: &mut Context<Self>) {
        let Some(confirm) = self.right.discard.as_mut().filter(|c| c.is_open()) else { return };
        confirm.close();
        let ask = confirm.value.clone();
        cx.notify();
        if !confirmed {
            return;
        }
        cx.spawn(async move |this, cx| {
            let DiscardAsk { tab, dir, target } = ask;
            let result = cx
                .background_spawn(async move {
                    match target {
                        DiscardTarget::Tree { checksum } => crate::changes::discard_working_tree(&dir, &checksum),
                        DiscardTarget::File(file) => crate::changes::discard_file(&dir, &file),
                    }
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = result {
                    crate::ui::toast::error(cx, format!("Couldn't discard changes: {e}"));
                }
                if let Some(changes) = this.right.diffs.get(&tab) {
                    changes.update(cx, |changes, cx| changes.restart_watch(cx));
                }
            });
        })
        .detach();
    }

    pub(super) fn discard_dialog(&self, theme: &Theme, window: &Window, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let confirm = self.right.discard.as_ref()?;
        let (title, words) = match &confirm.value.target {
            DiscardTarget::Tree { .. } => ("Discard uncommitted changes?", "Discard all uncommitted changes in this working tree? This can’t be undone.".to_string()),
            DiscardTarget::File(file) if file.status == FileStatus::Added => ("Delete this new file?", format!("{} isn’t in any commit: deleting it can’t be undone.", file.path)),
            DiscardTarget::File(file) => ("Discard file changes?", format!("Discard all uncommitted changes in {}? This can’t be undone.", file.path)),
        };
        let card = dialog_card(theme, "discard-dialog", title)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.finish_discard(false, cx)))
            .child(div().mt(px(8.)).line_height(px(19.)).text_color(theme.text_muted).child(words))
            .child(
                div()
                    .mt(px(16.))
                    .flex()
                    .justify_end()
                    .gap(px(8.))
                    .child(ghost_button(theme, "discard-cancel", "Cancel").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_discard(false, cx))))
                    .child(solid_button("discard-confirm", "Discard changes", theme.danger, gpui::white()).on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.finish_discard(true, cx)))),
            );
        Some(modal("discard-dialog", confirm, window, card))
    }
}
