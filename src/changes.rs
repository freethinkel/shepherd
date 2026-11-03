//! The right panel's diffs, Zeron's `crates/ui/src/changes.rs` (MIT): what's plain of it. The patch parser, the split
//! pairing, the row model and the labels are its own; `git_status` and the capture are from its engine's
//! `diff_sync.rs`, asked of `git` here (or on the machine shown) rather than over its RPC. There's no commit tab and no
//! composer here: History and *Commit* are left out, and a comment goes to an agent from the panel
//! (`shell/changes.rs`, the entity and all that draws). The last turn is herdr-reviewr's `turn.rs` (MIT) cut down.
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use unicode_width::UnicodeWidthChar as _;
use zeron_syntax::LanguageId as Lang;

pub const FILE_HEADER_HEIGHT: f32 = crate::ui::surface_chrome::HEADER_HEIGHT;
pub const HUNK_HEADER_HEIGHT: f32 = 28.0;
pub const DIFF_LINE_HEIGHT: f32 = 21.0;
pub const NOTICE_HEIGHT: f32 = 24.0;
pub const BODY_BOTTOM_PAD: f32 = 8.0;
/// Gutter width per line-number column.
pub const GUTTER_WIDTH: f32 = 36.0;
const DIFF_TAB_SIZE: usize = 4;

/// How the diff is laid out. Persisted in `ui-settings.json` (`diffSplit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffMode {
    /// One column: deletions above additions (the classic patch reading).
    #[default]
    Unified,
    /// Two columns: old on the left, new on the right, paired per hunk.
    Split,
}

impl DiffMode {
    pub fn from_split(split: bool) -> Self {
        if split { Self::Split } else { Self::Unified }
    }

    pub fn is_split(self) -> bool {
        self == Self::Split
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Unified => Self::Split,
            Self::Split => Self::Unified,
        }
    }
}
// ---------------------------------------------------------------------------
// Patch model + parser (pure)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Add,
    Del,
    /// `\ No newline at end of file` and friends.
    Meta,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceSide {
    Old,
    New,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLineRef {
    pub side: SourceSide,
    /// One-based source line number.
    pub line_number: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiffHighlights {
    pub old: Option<Arc<zeron_syntax::HighlightedDocument>>,
    pub new: Option<Arc<zeron_syntax::HighlightedDocument>>,
}

impl DiffHighlights {
    pub fn source_ref(&self, line: &DiffLine) -> Option<SourceLineRef> {
        match line.kind {
            LineKind::Del => line.old_no.map(|line_number| SourceLineRef {
                side: SourceSide::Old,
                line_number,
            }),
            LineKind::Add => line.new_no.map(|line_number| SourceLineRef {
                side: SourceSide::New,
                line_number,
            }),
            LineKind::Context => line
                .new_no
                .filter(|_| self.new.is_some())
                .map(|line_number| SourceLineRef {
                    side: SourceSide::New,
                    line_number,
                })
                .or_else(|| {
                    line.old_no.map(|line_number| SourceLineRef {
                        side: SourceSide::Old,
                        line_number,
                    })
                }),
            LineKind::Meta => None,
        }
    }

    pub fn spans(&self, line: &DiffLine) -> &[zeron_syntax::HighlightSpan] {
        let Some(source_ref) = self.source_ref(line) else {
            return &[];
        };
        let document = match source_ref.side {
            SourceSide::Old => self.old.as_deref(),
            SourceSide::New => self.new.as_deref(),
        };
        document
            .and_then(|document| document.lines.get(source_ref.line_number as usize - 1))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileDiff {
    /// Display path (the post-change side).
    pub path: String,
    /// Pre-rename path, when different.
    pub old_path: Option<String>,
    pub status: FileStatus,
    pub binary: bool,
    /// Parser-collected notices (mode changes etc.).
    pub notices: Vec<String>,
    pub hunks: Vec<Hunk>,
    pub additions: u32,
    pub deletions: u32,
    /// Largest line number on either side — sizes the gutters analytically
    /// (a fixed column overflowed past 4 digits; user report).
    pub max_line: u32,
}

impl FileDiff {
    fn new(path: String, old_path: Option<String>) -> Self {
        Self {
            path,
            old_path,
            status: FileStatus::Modified,
            binary: false,
            notices: Vec::new(),
            hunks: Vec::new(),
            additions: 0,
            deletions: 0,
            max_line: 0,
        }
    }
}
/// Width of one line-number gutter column, fitted to the file's largest
/// line number: 12px mono ≈ 7.2px per digit, the 8px right pad, and a 6px
/// left gap so the number never abuts the accent bar (at 4 digits the old
/// formula left 1.6px — visually touching; user report). Never narrower
/// than the classic 36px column.
pub fn gutter_width(file: &FileDiff) -> f32 {
    let digits = file.max_line.max(1).ilog10() + 1;
    (digits as f32 * 7.2 + 8.0 + 6.0).max(GUTTER_WIDTH)
}

/// Count terminal-style display columns, including tab stops and wide
/// Unicode glyphs. This is only a floor; actual shaped runs determine the extent.
pub fn visual_columns(text: &str) -> usize {
    text.chars().fold(0usize, |columns, ch| {
        if ch == '\t' {
            columns + (DIFF_TAB_SIZE - columns % DIFF_TAB_SIZE)
        } else {
            columns + ch.width().unwrap_or(0)
        }
    })
}

fn strip_git_prefix(path: &str) -> &str {
    path.strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path)
}

/// Split the tail of a `diff --git a/… b/…` line into (old, new) paths.
/// Quoted paths (spaces/unicode) are handled; for unquoted paths with spaces
/// the split favors the last ` b/` separator, which is git's own convention.
fn parse_git_paths(rest: &str) -> (String, String) {
    fn unquote(s: &str) -> String {
        let trimmed = s.trim();
        if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
            trimmed[1..trimmed.len() - 1]
                .replace("\\\"", "\"")
                .replace("\\\\", "\\")
        } else {
            trimmed.to_string()
        }
    }
    if let Some(pos) = rest.rfind(" b/").or_else(|| rest.rfind(" \"b/")) {
        let old = unquote(&rest[..pos]);
        let new = unquote(&rest[pos + 1..]);
        (
            strip_git_prefix(&old).to_string(),
            strip_git_prefix(&new).to_string(),
        )
    } else {
        let p = strip_git_prefix(&unquote(rest)).to_string();
        (p.clone(), p)
    }
}

/// Parse one `@@ -a[,b] +c[,d] @@ …` header into starting line numbers.
fn parse_hunk_header(line: &str) -> Option<(u32, u32)> {
    let rest = line.strip_prefix("@@")?;
    let minus = rest.find('-')?;
    let after_minus = &rest[minus + 1..];
    let old: u32 = after_minus
        .split(|c: char| c == ',' || c.is_whitespace())
        .next()?
        .parse()
        .ok()?;
    let plus = rest.find('+')?;
    let after_plus = &rest[plus + 1..];
    let new: u32 = after_plus
        .split(|c: char| c == ',' || c.is_whitespace())
        .next()?
        .parse()
        .ok()?;
    Some((old, new))
}

/// Parse a unified git patch into file sections. Tolerant: unknown header
/// lines are skipped, truncated hunks keep what parsed so far.
pub fn parse_patch(patch: &str) -> Vec<FileDiff> {
    let mut files = parse_sections(patch);
    files.sort_by(|a, b| tree_order(&a.path, &b.path));
    files
}

/// lazygit's file tree, as a list: at each level the folders, then the files, each by name.
pub fn tree_order(a: &str, b: &str) -> std::cmp::Ordering {
    let (a, b): (Vec<_>, Vec<_>) = (a.split('/').collect(), b.split('/').collect());
    for (i, (x, y)) in a.iter().zip(&b).enumerate() {
        if x != y {
            return (i + 1 < b.len()).cmp(&(i + 1 < a.len())).then(x.cmp(y));
        }
    }
    a.len().cmp(&b.len())
}

fn parse_sections(patch: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut in_hunk = false;
    let mut old_no: u32 = 0;
    let mut new_no: u32 = 0;

    for raw in patch.lines() {
        if let Some(rest) = raw.strip_prefix("diff --git ") {
            let (old, new) = parse_git_paths(rest);
            let old_path = (old != new).then_some(old);
            files.push(FileDiff::new(new, old_path));
            in_hunk = false;
            continue;
        }
        let Some(file) = files.last_mut() else {
            continue;
        };

        if raw.starts_with("@@") {
            if let Some((o, n)) = parse_hunk_header(raw) {
                old_no = o;
                new_no = n;
                file.hunks.push(Hunk {
                    header: raw.to_string(),
                    lines: Vec::new(),
                });
                in_hunk = true;
            }
            continue;
        }

        if in_hunk {
            let mut chars = raw.chars();
            let marker = chars.next();
            let body: String = chars.collect();
            let line = match marker {
                Some('+') => {
                    file.additions += 1;
                    let l = DiffLine {
                        kind: LineKind::Add,
                        old_no: None,
                        new_no: Some(new_no),
                        text: body,
                    };
                    new_no += 1;
                    Some(l)
                }
                Some('-') => {
                    file.deletions += 1;
                    let l = DiffLine {
                        kind: LineKind::Del,
                        old_no: Some(old_no),
                        new_no: None,
                        text: body,
                    };
                    old_no += 1;
                    Some(l)
                }
                Some(' ') | None => {
                    let l = DiffLine {
                        kind: LineKind::Context,
                        old_no: Some(old_no),
                        new_no: Some(new_no),
                        text: body,
                    };
                    old_no += 1;
                    new_no += 1;
                    Some(l)
                }
                Some('\\') => Some(DiffLine {
                    kind: LineKind::Meta,
                    old_no: None,
                    new_no: None,
                    text: raw.trim_start_matches('\\').trim().to_string(),
                }),
                _ => {
                    // A non-hunk line ends the hunk; reprocess as a header.
                    in_hunk = false;
                    None
                }
            };
            if let Some(line) = line
                && let Some(hunk) = file.hunks.last_mut()
            {
                file.max_line = file
                    .max_line
                    .max(line.old_no.unwrap_or(0))
                    .max(line.new_no.unwrap_or(0));
                hunk.lines.push(line);
                continue;
            }
            if in_hunk {
                continue;
            }
        }

        // File header territory.
        if raw.starts_with("new file mode") {
            file.status = FileStatus::Added;
        } else if raw.starts_with("deleted file mode") {
            file.status = FileStatus::Deleted;
        } else if let Some(from) = raw.strip_prefix("rename from ") {
            file.status = FileStatus::Renamed;
            file.old_path = Some(from.trim().to_string());
        } else if let Some(to) = raw.strip_prefix("rename to ") {
            file.status = FileStatus::Renamed;
            file.path = to.trim().to_string();
        } else if raw.starts_with("Binary files") || raw.starts_with("GIT binary patch") {
            file.binary = true;
        } else if let Some(mode) = raw.strip_prefix("new mode ") {
            file.notices
                .push(format!("Mode changed to {}", mode.trim()));
        } else if let Some(new) = raw.strip_prefix("+++ ") {
            let new = new.trim();
            if new == "/dev/null" {
                file.status = FileStatus::Deleted;
            } else if file.old_path.is_none() {
                file.path = strip_git_prefix(new).to_string();
            }
        } else if let Some(old) = raw.strip_prefix("--- ")
            && old.trim() == "/dev/null"
        {
            file.status = FileStatus::Added;
        }
        // "index …", "similarity index …", "old mode …" etc.: skipped.
    }
    files
}

/// Derived per-file notice rows (new/deleted/renamed/binary + parser notices).
pub fn file_notices(file: &FileDiff) -> Vec<String> {
    let mut notices = Vec::new();
    match file.status {
        FileStatus::Added => notices.push("New file".to_string()),
        FileStatus::Deleted => notices.push("Deleted file".to_string()),
        FileStatus::Renamed => {
            let from = file.old_path.as_deref().unwrap_or("?");
            notices.push(format!("Renamed from {from}"));
        }
        FileStatus::Modified => {}
    }
    if file.binary {
        notices.push("Binary file — contents not shown".to_string());
    }
    notices.extend(file.notices.iter().cloned());
    notices
}

/// One split row: indices into the hunk's lines for the left (old) and right
/// (new) column. `None` on a side means that column is empty for this row.
pub type LinePair = (Option<u32>, Option<u32>);

/// Pair a hunk's lines into split rows.
///
/// A hunk reads as runs: context lines sit on both sides, and each run of
/// deletions immediately followed by additions is a *change block* whose two
/// sides line up index-for-index (the shape git already emits — an edited
/// line's `-`/`+` are adjacent). The longer side's leftovers get one-sided
/// rows, so a 3-for-1 rewrite is 1 paired row and 2 add-only rows rather than
/// a ragged interleave. A deletion arriving after additions opens a new block
/// (`-a +b -c +d` is two edits, not one four-line one).
///
/// Pure and index-only: the caller keeps owning the lines, and the result is
/// small enough to live in the row model.
pub fn split_pairs(lines: &[DiffLine]) -> Vec<LinePair> {
    split_pairs_upto(lines, usize::MAX)
}

/// [`split_pairs`], stopping at `max_rows`.
///
/// The fold tween's stand-in builds only the slice its clip can reveal and
/// re-renders every frame of the tween, so it must not pay to pair a 50k-line
/// hunk to draw twenty rows of it. Bounding the *output* is not enough — the
/// pending runs are bounded too, since a change block yields
/// `max(dels, adds)` rows and so anything past the budget can only land past
/// it as well.
pub fn split_pairs_upto(lines: &[DiffLine], max_rows: usize) -> Vec<LinePair> {
    /// The block being accumulated: the two sides' code lines, plus the
    /// `\ No newline…` marker each side may end on.
    #[derive(Default)]
    struct Block {
        dels: Vec<u32>,
        adds: Vec<u32>,
        del_meta: Vec<u32>,
        add_meta: Vec<u32>,
    }

    fn flush(pairs: &mut Vec<LinePair>, block: &mut Block, max_rows: usize) {
        let mut drain = |left: &mut Vec<u32>, right: &mut Vec<u32>| {
            for ix in 0..left.len().max(right.len()) {
                if pairs.len() >= max_rows {
                    break;
                }
                pairs.push((left.get(ix).copied(), right.get(ix).copied()));
            }
            left.clear();
            right.clear();
        };
        drain(&mut block.dels, &mut block.adds);
        // Markers trail the code they annotate, and pair with each other — a
        // modification where both files lost their final newline is one
        // aligned row plus one marker row, not two one-sided rows plus two
        // markers. They never share a row with code, so both render arms can
        // treat a marker on either side as spanning the row.
        drain(&mut block.del_meta, &mut block.add_meta);
    }

    let mut pairs = Vec::with_capacity(lines.len().min(max_rows));
    let mut block = Block::default();
    let mut pending_side: Option<LineKind> = None;
    for (ix, line) in lines.iter().enumerate() {
        match line.kind {
            LineKind::Del => {
                // A marker already closes its side, so code arriving after one
                // starts a fresh block — the marker row keeps its place in the
                // file's order.
                if !block.adds.is_empty()
                    || !block.del_meta.is_empty()
                    || !block.add_meta.is_empty()
                {
                    flush(&mut pairs, &mut block, max_rows);
                }
                let remaining = max_rows - pairs.len().min(max_rows);
                if remaining == 0 {
                    break;
                }
                if block.dels.len() < remaining {
                    block.dels.push(ix as u32);
                }
                pending_side = Some(LineKind::Del);
            }
            LineKind::Add => {
                // The old side's marker is the one case where a marker does
                // not close the block: `-old`, marker, `+new` is one edit.
                if !block.add_meta.is_empty() {
                    flush(&mut pairs, &mut block, max_rows);
                }
                let remaining = max_rows - pairs.len().min(max_rows);
                if remaining == 0 {
                    break;
                }
                if block.adds.len() < remaining {
                    block.adds.push(ix as u32);
                }
                pending_side = Some(LineKind::Add);
            }
            // `\ No newline at end of file` belongs to the side whose line it
            // follows, so it must NOT close the block: git writes `-old`,
            // marker, `+new`, marker for an edited last line, and treating
            // either marker as a boundary would tear that edit apart. A marker
            // after context describes the same line on both sides.
            LineKind::Meta => match pending_side {
                Some(LineKind::Del) => block.del_meta.push(ix as u32),
                Some(LineKind::Add) => block.add_meta.push(ix as u32),
                _ => {
                    block.del_meta.push(ix as u32);
                    block.add_meta.push(ix as u32);
                }
            },
            // Context sits on both sides.
            LineKind::Context => {
                flush(&mut pairs, &mut block, max_rows);
                if pairs.len() >= max_rows {
                    break;
                }
                pairs.push((Some(ix as u32), Some(ix as u32)));
                pending_side = Some(LineKind::Context);
            }
        }
    }
    flush(&mut pairs, &mut block, max_rows);
    pairs.truncate(max_rows);
    pairs
}

/// Every anchor a split row can *display* a card for, left column first.
///
/// Wider than what the row lets you write: only the right column takes a `+`
/// (see the `SplitLine` render arm), but an old-side note staged from the
/// unified layout must still show its card here, or toggling layouts would
/// look like it dropped one. A context row names the same anchor on both
/// sides, so the duplicate is dropped — the caller flattens, and two
/// identical anchors would stage the card twice. A fixed array, not a `Vec`:
/// this runs per row of every re-flatten.
fn pair_anchors(lines: &[DiffLine], pair: LinePair) -> [Option<(CommentSide, u32)>; 2] {
    let anchor = |ix: Option<u32>| {
        ix.and_then(|ix| lines.get(ix as usize))
            .and_then(line_anchor)
    };
    let (left, right) = (anchor(pair.0), anchor(pair.1));
    if left == right {
        [left, None]
    } else {
        [left, right]
    }
}

/// A deletion only exists in the pre-change file; everything else is cited
/// against the post-change file, which is what the agent edits.
pub fn line_anchor(line: &DiffLine) -> Option<(CommentSide, u32)> {
    match line.kind {
        LineKind::Meta => None,
        LineKind::Del => line.old_no.map(|no| (CommentSide::Old, no)),
        _ => line.new_no.map(|no| (CommentSide::New, no)),
    }
}

/// Header label: "N Uncommitted change(s)".
pub fn uncommitted_label(count: usize) -> String {
    if count == 1 {
        "1 Uncommitted change".to_string()
    } else {
        format!("{count} Uncommitted changes")
    }
}

/// What the pane diffs against (t3code's scope dropdown).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffScope {
    /// Uncommitted changes vs HEAD.
    #[default]
    WorkingTree,
    /// Everything this branch adds over `merge-base(base_ref, HEAD)`, working tree included.
    Branch,
    /// What changed since the agents' latest turn that changed anything began.
    LastTurn,
}

impl DiffScope {
    pub const ALL: [DiffScope; 3] = [Self::WorkingTree, Self::Branch, Self::LastTurn];

    pub fn label(self) -> &'static str {
        match self {
            Self::WorkingTree => "Uncommitted",
            Self::Branch => "Branch changes",
            Self::LastTurn => "Last turn",
        }
    }
}

/// Header-strip label per scope.
pub fn scope_label(scope: DiffScope, count: usize, base: Option<&str>) -> String {
    let files = if count == 1 { "file" } else { "files" };
    match scope {
        DiffScope::WorkingTree => uncommitted_label(count),
        DiffScope::Branch => match base {
            Some(base) => format!("{count} Changed {files} vs {base}"),
            None => format!("{count} Changed {files}"),
        },
        DiffScope::LastTurn => format!("{count} Changed {files} in the last turn"),
    }
}

/// The comparison ref the branch scope preselects. `branches` comes from [`list_branches`] with the repo's default
/// branch first, but a repo with no `origin/HEAD` falls back to the *checked-out* branch there, and comparing a branch
/// with itself is useless; prefer `main`/`master` in that case, on `origin` if not here.
pub fn default_base_ref(branches: &[String], current: Option<&str>) -> Option<String> {
    let first = branches.first()?;
    if current != Some(first.as_str()) {
        return Some(first.clone());
    }
    for candidate in ["main", "master", "origin/main", "origin/master"] {
        if branches.iter().any(|b| b == candidate) {
            return Some(candidate.to_string());
        }
    }
    branches.iter().find(|b| current != Some(b.as_str())).or(Some(first)).cloned()
}

/// Empty-state copy per scope.
pub fn clean_message(scope: DiffScope, base: Option<&str>) -> String {
    match scope {
        DiffScope::WorkingTree => "No uncommitted changes".to_string(),
        DiffScope::Branch => match base {
            Some(base) => format!("No changes vs {base}"),
            None => "No branch changes".to_string(),
        },
        DiffScope::LastTurn => "No changes from the last turn".to_string(),
    }
}

/// Match rank of a label against a query: `0` prefix match, `1` substring, `None` no match. Case-insensitive; an empty
/// query matches everything at rank 1 (input order preserved).
fn match_rank(query: &str, label: &str) -> Option<usize> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Some(1);
    }
    let label = label.to_lowercase();
    if label.starts_with(&query) {
        Some(0)
    } else if label.contains(&query) {
        Some(1)
    } else {
        None
    }
}

/// Filter + rank labels for a search query: prefix matches first, then substring matches, stable within each rank.
/// Returns indices into `labels`.
pub fn filter_indices<S: AsRef<str>>(query: &str, labels: &[S]) -> Vec<usize> {
    let mut ranked: Vec<(usize, usize)> = labels.iter().enumerate().filter_map(|(ix, label)| match_rank(query, label.as_ref()).map(|rank| (rank, ix))).collect();
    ranked.sort_by_key(|&(rank, ix)| (rank, ix));
    ranked.into_iter().map(|(_, ix)| ix).collect()
}

// ---------------------------------------------------------------------------
// Review comments (Zeron's `comments.rs`): pinned to lines of a diff, sent to an agent as plain text
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommentSide {
    Old,
    New,
}

impl CommentSide {
    pub fn tag(self) -> &'static str {
        match self {
            Self::Old => "L",
            Self::New => "R",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewComment {
    pub id: u64,
    /// The current workspace path. For an old-side comment on a renamed file, [`Self::cite_path`] returns the
    /// pre-rename path instead.
    pub path: String,
    pub old_path: Option<String>,
    pub side: CommentSide,
    pub line: u32,
    pub body: String,
}

impl ReviewComment {
    pub fn anchor(&self) -> (CommentSide, u32) {
        (self.side, self.line)
    }

    /// The path the line number is valid in. An `Old`-side line only exists in the pre-rename file, so citing `path`
    /// there points the agent at a line of a file that never held it.
    pub fn cite_path(&self) -> &str {
        match self.side {
            CommentSide::Old => self.old_path.as_deref().unwrap_or(&self.path),
            CommentSide::New => &self.path,
        }
    }

    pub fn location(&self) -> String {
        format!("{}:{}", self.cite_path(), self.line)
    }
}

const COMMENT_ONLY_TEXT: &str = "Address the review comments below.";
const COMMENT_BLOCK_HEADER: &str = "Comments on the diff (each cites the file and line it belongs to; L = line number in the original file, R = in the changed file):";

/// What an agent receives. It arrives as one paste, so nothing a terminal acts on stays in it (see `annotation_prompt`).
pub fn comments_prompt(comments: &[ReviewComment]) -> String {
    let bullets: Vec<String> = comments
        .iter()
        .map(|comment| {
            let body = comment.body.trim().replace('\n', "\n  ");
            format!("- {} ({}): {body}", comment.location(), comment.side.tag())
        })
        .collect();
    let out = format!("{COMMENT_ONLY_TEXT}\n\n{COMMENT_BLOCK_HEADER}\n{}", bullets.join("\n"));
    out.chars().filter(|c| !c.is_control() || matches!(c, '\n' | '\t')).collect()
}

pub const CARD_PAD_V: f32 = 20.0;
pub const CARD_HEADER_HEIGHT: f32 = 22.0;
pub const CARD_LINE_HEIGHT: f32 = 18.0;
pub const DRAFT_CARD_HEIGHT: f32 = 116.0;
const CARD_GAP: f32 = 6.0;
const CARD_WRAP_COLUMNS: usize = 64;
const CARD_MAX_LINES: usize = 8;

/// Wraps are guessed, not measured: the changes pane sizes bodies by arithmetic to drive the fold tween, and a measured
/// card would desync it.
pub fn card_body_lines(body: &str) -> usize {
    body.lines().map(|line| line.chars().count().div_ceil(CARD_WRAP_COLUMNS).max(1)).sum::<usize>().clamp(1, CARD_MAX_LINES)
}

pub fn card_height(body: &str) -> f32 {
    CARD_PAD_V + CARD_HEADER_HEIGHT + card_body_lines(body) as f32 * CARD_LINE_HEIGHT + CARD_GAP
}

pub fn comment_state_key(comments: &[ReviewComment], draft: Option<&(String, CommentSide, u32)>) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    for comment in comments {
        (comment.id, &comment.body).hash(&mut hasher);
    }
    if let Some((path, side, line)) = draft {
        (path, side.tag(), line).hash(&mut hasher);
    }
    hasher.finish()
}

// ---------------------------------------------------------------------------
// Syntax highlight sources
// ---------------------------------------------------------------------------

const MAX_EXCERPT_SOURCE_LINES: usize = 200_000;

fn excerpt_side(file: &FileDiff, side: SourceSide, language: Lang, path: &str) -> Option<Arc<zeron_syntax::HighlightedDocument>> {
    let number = |line: &DiffLine| match side {
        SourceSide::Old => line.old_no,
        SourceSide::New => line.new_no,
    };
    let max_line = file.hunks.iter().flat_map(|hunk| &hunk.lines).filter_map(number).max().unwrap_or(0) as usize;
    if max_line > MAX_EXCERPT_SOURCE_LINES {
        return None;
    }
    let mut lines = vec![Vec::new(); max_line];
    for hunk in &file.hunks {
        let visible = hunk.lines.iter().filter_map(|line| (line.kind != LineKind::Meta).then_some((number(line)?, line.text.as_str()))).collect::<Vec<_>>();
        if visible.is_empty() {
            continue;
        }
        let source = visible.iter().map(|(_, text)| *text).collect::<Vec<_>>().join("\n");
        let document = zeron_syntax::highlight(zeron_syntax::HighlightRequest { source: &source, path: Some(path), fence_tag: None }).ok()?;
        for ((number, _), spans) in visible.into_iter().zip(document.lines) {
            lines[number as usize - 1] = spans;
        }
    }
    Some(Arc::new(zeron_syntax::HighlightedDocument { language, lines }))
}

pub fn excerpt_highlights(file: &FileDiff, language: Lang) -> Option<DiffHighlights> {
    if !zeron_syntax::supports_language(language) {
        return None;
    }
    let old = if file.status == FileStatus::Added {
        None
    } else {
        Some(excerpt_side(file, SourceSide::Old, language, file.old_path.as_deref().unwrap_or(&file.path))?)
    };
    let new = if file.status == FileStatus::Deleted {
        None
    } else {
        Some(excerpt_side(file, SourceSide::New, language, &file.path)?)
    };
    Some(DiffHighlights { old, new })
}

fn sources_match_patch(file: &FileDiff, old: Option<&str>, new: Option<&str>) -> bool {
    let old = old.map(|source| source.lines().collect::<Vec<_>>());
    let new = new.map(|source| source.lines().collect::<Vec<_>>());
    file.hunks.iter().flat_map(|hunk| &hunk.lines).all(|line| {
        fn of<'a>(lines: &Option<Vec<&'a str>>, number: Option<u32>) -> Option<&'a str> {
            lines.as_ref()?.get(number? as usize - 1).copied()
        }
        let actual = match line.kind {
            LineKind::Del => of(&old, line.old_no),
            LineKind::Add => of(&new, line.new_no),
            LineKind::Context => of(&new, line.new_no).or_else(|| of(&old, line.old_no)),
            LineKind::Meta => return true,
        };
        actual == Some(line.text.as_str())
    })
}

/// The whole old and new documents' highlights, `None` when they no longer match the patch (the tree moved since).
pub fn full_highlights(file: &FileDiff, language: Lang, old: Option<&str>, new: Option<&str>) -> Option<DiffHighlights> {
    if !sources_match_patch(file, old, new) {
        return None;
    }
    let parse = |source: &str, path: &str| zeron_syntax::highlight(zeron_syntax::HighlightRequest { source, path: Some(path), fence_tag: None }).ok().map(Arc::new);
    let old_doc = match old {
        Some(source) => Some(parse(source, file.old_path.as_deref().unwrap_or(&file.path))?),
        None => None,
    };
    let new_doc = match new {
        Some(source) => Some(parse(source, &file.path)?),
        None => None,
    };
    if old_doc.is_none() && new_doc.is_none() && zeron_syntax::supports_language(language) {
        return None;
    }
    Some(DiffHighlights { old: old_doc, new: new_doc })
}

// ---------------------------------------------------------------------------
// Row model: the diff flattened to line granularity
// ---------------------------------------------------------------------------

/// One virtualized list row. The diff is flattened so each visible LINE is its own row (Zed's editor draws exactly the
/// visible line range the same way): scrolling a 10k-line file materializes ~50 line rows per frame, not one 10k-line
/// element, and a collapsed file contributes no body rows at all. Nowrap heights use the analytic constants above;
/// wrapped line rows are measured by the list at the current pane width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffRow {
    FileHeader { file: u32 },
    Notice { file: u32, notice: u32 },
    HunkHeader { file: u32, hunk: u32 },
    Line { file: u32, hunk: u32, line: u32 },
    /// One split row: the two line indices [`split_pairs`] paired. Carrying them inline keeps the pairing off the
    /// render path: it is computed once, when the body is flattened.
    SplitLine { file: u32, hunk: u32, left: Option<u32>, right: Option<u32> },
    /// `card` indexes the file's own staged-comment slice, in staged order.
    CommentCard { file: u32, card: u32 },
    CommentDraft { file: u32 },
    /// Trailing pad closing an expanded body ([`BODY_BOTTOM_PAD`]).
    BodyPad { file: u32 },
    /// A body mid-fold-tween: one height-animated, clipped row standing in for the whole body. Only the slice that can
    /// be revealed is built: the tween never pays for off-screen lines.
    FoldingBody { file: u32 },
}

impl DiffRow {
    pub fn file(self) -> usize {
        match self {
            Self::FileHeader { file }
            | Self::Notice { file, .. }
            | Self::HunkHeader { file, .. }
            | Self::Line { file, .. }
            | Self::SplitLine { file, .. }
            | Self::CommentCard { file, .. }
            | Self::CommentDraft { file }
            | Self::BodyPad { file }
            | Self::FoldingBody { file } => file as usize,
        }
    }

    /// `FoldingBody` is height-animated, so it reports 0 and never lands in a height sum.
    fn height(self, comments: &[ReviewComment]) -> f32 {
        match self {
            DiffRow::FileHeader { .. } => FILE_HEADER_HEIGHT,
            DiffRow::Notice { .. } => NOTICE_HEIGHT,
            DiffRow::HunkHeader { .. } => HUNK_HEADER_HEIGHT,
            DiffRow::Line { .. } | DiffRow::SplitLine { .. } => DIFF_LINE_HEIGHT,
            DiffRow::CommentCard { card, .. } => comments.get(card as usize).map(|comment| card_height(&comment.body)).unwrap_or(0.0),
            DiffRow::CommentDraft { .. } => DRAFT_CARD_HEIGHT,
            DiffRow::BodyPad { .. } => BODY_BOTTOM_PAD,
            DiffRow::FoldingBody { .. } => 0.0,
        }
    }
}

/// Analytic expanded-body height: drives the fold tween without measurement.
pub fn body_height(file: &FileDiff, comments: &[ReviewComment], draft: Option<(CommentSide, u32)>, mode: DiffMode) -> f32 {
    body_rows(0, file, comments, draft, mode).iter().map(|row| row.height(comments)).sum()
}

/// Capacity hint only: comment cards are not counted. Split pairs can only shrink the line count, so the unified count
/// is a safe hint for both.
fn body_row_count(file: &FileDiff) -> usize {
    let lines: usize = file.hunks.iter().map(|h| h.lines.len()).sum();
    file_notices(file).len() + file.hunks.len() + lines + 1
}

pub fn body_rows(file_ix: u32, file: &FileDiff, comments: &[ReviewComment], draft: Option<(CommentSide, u32)>, mode: DiffMode) -> Vec<DiffRow> {
    fn push_cards(rows: &mut Vec<DiffRow>, file_ix: u32, comments: &[ReviewComment], draft: Option<(CommentSide, u32)>, anchors: &[Option<(CommentSide, u32)>]) {
        for anchor in anchors.iter().flatten() {
            for (ix, comment) in comments.iter().enumerate() {
                if comment.anchor() == *anchor {
                    rows.push(DiffRow::CommentCard { file: file_ix, card: ix as u32 });
                }
            }
            if draft == Some(*anchor) {
                rows.push(DiffRow::CommentDraft { file: file_ix });
            }
        }
    }

    let mut rows = Vec::with_capacity(body_row_count(file));
    for notice in 0..file_notices(file).len() {
        rows.push(DiffRow::Notice { file: file_ix, notice: notice as u32 });
    }
    for (hunk_ix, hunk) in file.hunks.iter().enumerate() {
        rows.push(DiffRow::HunkHeader { file: file_ix, hunk: hunk_ix as u32 });
        match mode {
            DiffMode::Unified => {
                for (line_ix, line) in hunk.lines.iter().enumerate() {
                    rows.push(DiffRow::Line { file: file_ix, hunk: hunk_ix as u32, line: line_ix as u32 });
                    push_cards(&mut rows, file_ix, comments, draft, &[line_anchor(line)]);
                }
            }
            DiffMode::Split => {
                for (left, right) in split_pairs(&hunk.lines) {
                    rows.push(DiffRow::SplitLine { file: file_ix, hunk: hunk_ix as u32, left, right });
                    let anchors = pair_anchors(&hunk.lines, (left, right));
                    push_cards(&mut rows, file_ix, comments, draft, &anchors);
                }
            }
        }
    }
    rows.push(DiffRow::BodyPad { file: file_ix });
    rows
}

/// Flatten all files into rows + each file's row span (header at `range.start`, body rows after it). `collapsed(ix)`
/// folds a file to just its header. `comments` is the whole staged set; each file takes its own path's slice.
pub fn flatten_rows(
    files: &[FileDiff],
    comments: &[ReviewComment],
    draft: Option<(&str, CommentSide, u32)>,
    mode: DiffMode,
    mut collapsed: impl FnMut(usize) -> bool,
) -> (Vec<DiffRow>, Vec<std::ops::Range<usize>>) {
    let mut rows = Vec::new();
    let mut ranges = Vec::with_capacity(files.len());
    for (ix, file) in files.iter().enumerate() {
        let start = rows.len();
        rows.push(DiffRow::FileHeader { file: ix as u32 });
        if !collapsed(ix) {
            let file_comments: Vec<ReviewComment> = comments.iter().filter(|comment| comment.path == file.path).cloned().collect();
            let file_draft = draft.filter(|(path, _, _)| *path == file.path).map(|(_, side, line)| (side, line));
            rows.extend(body_rows(ix as u32, file, &file_comments, file_draft, mode));
        }
        ranges.push(start..rows.len());
    }
    (rows, ranges)
}

/// The file header that should remain visible for a logical list position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickyFileHeader {
    pub file_ix: usize,
    pub header_row: usize,
    pub next_header_row: Option<usize>,
}

/// Resolve a sticky file header from the current flattened row ranges. This remains independent of the rendered list
/// so folds and diff resets cannot leave a second, stale active-file state behind.
pub fn sticky_file_header(row_ranges: &[std::ops::Range<usize>], item_ix: usize, offset_in_item: f32) -> Option<StickyFileHeader> {
    let file_ix = row_ranges.partition_point(|range| range.start <= item_ix).checked_sub(1)?;
    let range = row_ranges.get(file_ix)?;
    // A reset can briefly leave ListState pointing past the replacement model. Treat that frame as having no sticky
    // header.
    if !range.contains(&item_ix) || (item_ix == range.start && offset_in_item <= 0.0) {
        return None;
    }
    Some(StickyFileHeader { file_ix, header_row: range.start, next_header_row: row_ranges.get(file_ix + 1).map(|range| range.start) })
}

/// Offset a sticky header upward as the next file header enters its slot.
pub fn sticky_header_push_offset(next_header_y: Option<f32>) -> f32 {
    next_header_y.map(|y| (y - FILE_HEADER_HEIGHT).min(0.0)).unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// Capture, Zeron's `capture_diff_against`, by `git`: here or on the machine shown
// ---------------------------------------------------------------------------

/// `git hash-object -t tree /dev/null`: the diff base of a repo with no commits yet.
const EMPTY_TREE_SHA: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
const MAX_PATCH_BYTES: usize = 3 * 1024 * 1024;
const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024;
/// Each untracked file is a `git diff --no-index` of its own, an SSH round trip on a machine.
const MAX_UNTRACKED: usize = 256;

/// One capture of a checkout's diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutDiff {
    /// `--show-cdup`: from the pane's directory, which may be deep in the repo, to its root.
    pub root: String,
    /// The commit the patch is against.
    pub base: String,
    pub head: Option<String>,
    pub patch: String,
    /// Not all of it: a patch past the limit, too many untracked files, a path that isn't UTF-8.
    pub truncated: bool,
    /// Identifies what was captured, and what a discard was confirmed over.
    pub checksum: String,
}

fn run(dir: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let out = crate::herdr::git(dir, args).map_err(|e| e.to_string())?;
    match out.status.success() {
        true => Ok(out.stdout),
        false => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
    }
}

fn line(bytes: Vec<u8>) -> String {
    String::from_utf8_lossy(&bytes).trim().to_string()
}

/// `git diff`'s words whatever the user's config says. `bigFileThreshold` makes a bigger file a binary one, unread.
const DIFF: [&str; 12] = ["-c", "core.quotePath=false", "-c", "core.bigFileThreshold=2m", "--no-optional-locks", "diff", "--no-ext-diff", "--no-color", "--src-prefix=a/", "--dst-prefix=b/", "--find-renames", "--unified=3"];

fn repo_root(dir: &str) -> Result<String, String> {
    let cdup = line(run(dir, &["rev-parse", "--show-cdup"])?);
    Ok(if cdup.is_empty() { ".".to_string() } else { cdup })
}

fn at_root<'a>(root: &'a str, args: &[&'a str]) -> Vec<&'a str> {
    [&["-C", root][..], args].concat()
}

/// Porcelain v1 `-z` records into (tracked, untracked) paths; `None` when one isn't UTF-8. An untracked directory (a
/// nested repo, which git keeps atomic) ends in `/`.
pub fn status_paths(status: &[u8]) -> Option<(Vec<String>, Vec<String>)> {
    let records: Vec<&[u8]> = status.split(|byte| *byte == 0).filter(|record| !record.is_empty()).collect();
    let text = |bytes: &[u8]| std::str::from_utf8(bytes).ok().map(str::to_string);
    let (mut tracked, mut untracked) = (Vec::new(), Vec::new());
    let mut i = 0usize;
    while i < records.len() {
        let record = records[i];
        i += 1;
        if record.len() < 4 || record[2] != b' ' {
            continue;
        }
        let code = &record[..2];
        let path = text(&record[3..])?;
        if code == b"??" {
            untracked.push(path);
            continue;
        }
        if code == b"!!" {
            continue;
        }
        tracked.push(path);
        if (code.contains(&b'R') || code.contains(&b'C')) && i < records.len() {
            tracked.push(text(records[i])?);
            i += 1;
        }
    }
    tracked.sort();
    tracked.dedup();
    untracked.sort();
    untracked.dedup();
    Some((tracked, untracked))
}

fn checkout_status(dir: &str, root: &str) -> Result<Vec<u8>, String> {
    run(dir, &at_root(root, &["--no-optional-locks", "status", "--porcelain=v1", "-z", "--untracked-files=all"]))
}

/// A hunk git makes for an untracked file: `git diff` never shows them. An exit of 1 is what `--no-index` answers for
/// files that differ, so this reads its output whatever the status. A directory is a nested repo: named, never read.
fn untracked_patch(dir: &str, root: &str, path: &str) -> Option<String> {
    if let Some(path) = path.strip_suffix('/') {
        return Some(format!("diff --git a/{path} b/{path}\nnew file mode 160000\nBinary files /dev/null and b/{path} differ\n"));
    }
    let mut args = at_root(root, &DIFF);
    args.extend(["--no-index", "--", "/dev/null", path]);
    let out = crate::herdr::git(dir, &args).ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

// ponytail: `git diff` is read whole before the limit cuts it, and it runs on a poll; a kill at the limit, and a
// filesystem watch here (Zeron's), if a huge tree or diff makes it felt.
/// The working tree against HEAD, or with `base_ref` against its merge base with HEAD; untracked files count as new
/// either way.
pub fn capture(dir: &str, base_ref: Option<&str>) -> Result<CheckoutDiff, String> {
    let root = repo_root(dir)?;
    let head = run(dir, &at_root(&root, &["rev-parse", "--verify", "HEAD"])).ok().map(line);
    let base = match base_ref {
        Some(base) => line(run(dir, &at_root(&root, &["merge-base", base, "HEAD"]))?),
        None => head.clone().unwrap_or_else(|| EMPTY_TREE_SHA.to_string()),
    };
    let mut diff = at_root(&root, &DIFF);
    diff.extend([base.as_str(), "--"]);
    let mut patch = String::from_utf8_lossy(&run(dir, &diff)?).into_owned();
    let status = checkout_status(dir, &root)?;
    let paths = status_paths(&status);
    let mut truncated = paths.is_none();
    let untracked = paths.map(|(_, untracked)| untracked).unwrap_or_default();
    truncated |= untracked.len() > MAX_UNTRACKED;
    for path in untracked.iter().take(MAX_UNTRACKED) {
        if patch.len() > MAX_PATCH_BYTES {
            break;
        }
        if let Some(addition) = untracked_patch(dir, &root, path) {
            patch.push_str(&addition);
        }
    }
    if patch.len() > MAX_PATCH_BYTES {
        let end = patch[..patch.floor_char_boundary(MAX_PATCH_BYTES)].rfind('\n').unwrap_or(0);
        patch.truncate(end);
        truncated = true;
    }
    Ok(checkout_diff(root, base, head, patch, truncated))
}

fn checkout_diff(root: String, base: String, head: Option<String>, patch: String, truncated: bool) -> CheckoutDiff {
    let mut hasher = std::hash::DefaultHasher::new();
    (&head, &base, &patch, truncated).hash(&mut hasher);
    CheckoutDiff { root, base, head, patch, truncated, checksum: format!("{:016x}", hasher.finish()) }
}

/// Where a worktree keeps the tree its last turn began from: a per-worktree ref, so it outlives the app and gc.
const TURN_REF: &str = "refs/worktree/shepherd-last-turn";

/// The checkout as a tree, untracked files in, ignored ones out, on a copy of the index: the real one and the stash stay
/// as they are.
pub fn snapshot_tree(dir: &str) -> Result<String, String> {
    let root = repo_root(dir)?;
    let git_dir = line(run(dir, &at_root(&root, &["rev-parse", "--absolute-git-dir"]))?);
    // one per run: a turn's watch and the diff's poll snapshot the same checkout at once
    static RUNS: AtomicU64 = AtomicU64::new(0);
    let copy = format!("{git_dir}/shepherd-turn-index-{}-{}", std::process::id(), RUNS.fetch_add(1, Ordering::Relaxed));
    let tree = match crate::herdr::split(dir) {
        (None, here) => {
            // none before the first `git add`: the snapshot then reads every file
            let _ = std::fs::copy(format!("{git_dir}/index"), &copy);
            let git = |args: &[&str]| {
                let mut git = crate::process::command("git");
                git.arg("-C").arg(here).args(at_root(&root, args)).env("GIT_INDEX_FILE", &copy).stdin(std::process::Stdio::null()).output()
            };
            let made = git(&["add", "-A"]).and_then(|added| if added.status.success() { git(&["write-tree"]) } else { Ok(added) });
            let _ = std::fs::remove_file(&copy);
            made
        }
        (Some(target), there) => {
            let (index, copied, there, root) = (crate::herdr::quote(&format!("{git_dir}/index")), crate::herdr::quote(&copy), crate::herdr::quote(there), crate::herdr::quote(&root));
            let script = format!("cd {there} && cd {root} && {{ cp {index} {copied} 2>/dev/null; GIT_INDEX_FILE={copied} git add -A && GIT_INDEX_FILE={copied} git write-tree; code=$?; rm -f {copied}; exit $code; }}");
            crate::herdr::ssh(target, &script)
        }
    }
    .map_err(|e| e.to_string())?;
    match tree.status.success() {
        true => Ok(line(tree.stdout)),
        false => Err(String::from_utf8_lossy(&tree.stderr).trim().to_string()),
    }
}

/// Whether the checkout moved off `start`: if so `start` becomes the last turn's beginning.
pub fn turn_moved(dir: &str, start: &str) -> Result<bool, String> {
    if snapshot_tree(dir)? == start {
        return Ok(false);
    }
    run(dir, &["update-ref", TURN_REF, start])?;
    Ok(true)
}

/// The tree the last turn began from against the checkout now; no patch before a turn has changed anything. Two trees, not
/// `git diff <tree>`: that reads a file the snapshot holds and the index doesn't, untracked before the turn, as deleted.
pub fn capture_turn(dir: &str) -> Result<CheckoutDiff, String> {
    let root = repo_root(dir)?;
    let head = run(dir, &at_root(&root, &["rev-parse", "--verify", "HEAD"])).ok().map(line);
    let spec = format!("{TURN_REF}^{{tree}}");
    let Some(base) = run(dir, &at_root(&root, &["rev-parse", "--verify", "--quiet", &spec])).ok().map(line) else {
        return Ok(checkout_diff(root, EMPTY_TREE_SHA.to_string(), head, String::new(), false));
    };
    let now = snapshot_tree(dir)?;
    let mut diff = at_root(&root, &DIFF);
    diff.extend([base.as_str(), now.as_str(), "--"]);
    let mut patch = String::from_utf8_lossy(&run(dir, &diff)?).into_owned();
    let truncated = patch.len() > MAX_PATCH_BYTES;
    if truncated {
        let end = patch[..patch.floor_char_boundary(MAX_PATCH_BYTES)].rfind('\n').unwrap_or(0);
        patch.truncate(end);
    }
    Ok(checkout_diff(root, base, head, patch, truncated))
}

/// What one look at a worktree's agents says of its turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnEdge {
    pub started: bool,
    pub ended: bool,
}

/// A worktree's turns, read off its agents' statuses: one turn for all of them, from the first's start to the last's
/// rest. `blocked` and `unknown` belong to the turn they're in: an agent asking leaves it open, and starts none. Between
/// turns they change nothing: the next `working` still starts one.
#[derive(Debug, Default)]
pub struct Turns {
    /// False at first: the first look can't tell a turn started, only that one is going on.
    rested: bool,
    worked: bool,
}

impl Turns {
    pub fn observe(&mut self, statuses: impl IntoIterator<Item = crate::model::AgentStatus>) -> TurnEdge {
        use crate::model::AgentStatus::{Done, Idle, Working};
        let statuses: Vec<_> = statuses.into_iter().collect();
        let working = statuses.contains(&Working);
        let resting = statuses.iter().all(|s| matches!(s, Idle | Done));
        let edge = TurnEdge { started: working && self.rested, ended: self.worked && resting };
        self.rested = resting || (self.rested && !working);
        self.worked = working || (self.worked && !resting);
        edge
    }
}

/// `dir`'s branches: locals, then remote-only ones under their remote's name, which is the only one git knows them by, the
/// default branch first (`origin/HEAD`'s target, else the checked-out one).
pub fn list_branches(dir: &str) -> Result<Vec<String>, String> {
    let out = line(run(dir, &["branch", "-a", "--format=%(refname)"])?);
    let mut names: Vec<String> = out.lines().filter_map(|l| l.trim().strip_prefix("refs/heads/")).map(str::to_string).collect();
    for remote in out.lines().filter_map(|l| l.trim().strip_prefix("refs/remotes/")) {
        let name = remote.split_once('/').map_or(remote, |(_, name)| name);
        if name != "HEAD" && !names.iter().any(|n| n == name) {
            names.push(remote.to_string());
        }
    }
    let default = run(dir, &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"]).ok().map(line).and_then(|short| short.split_once('/').map(|(_, b)| b.to_string()));
    let default = default.or_else(|| run(dir, &["rev-parse", "--abbrev-ref", "HEAD"]).ok().map(line).filter(|b| b != "HEAD"));
    if let Some(pos) = default.and_then(|default| names.iter().position(|n| *n == default || n.strip_prefix("origin/") == Some(default.as_str()))) {
        names[..=pos].rotate_right(1);
    }
    Ok(names)
}

/// Whether a path from a patch stays inside the checkout.
pub fn safe_path(path: &str) -> bool {
    let path = std::path::Path::new(path);
    !path.as_os_str().is_empty() && !path.is_absolute() && path.components().all(|c| matches!(c, std::path::Component::Normal(_)))
}

fn text_source(bytes: Vec<u8>) -> Option<String> {
    if bytes.contains(&0) || bytes.len() as u64 > MAX_SOURCE_BYTES {
        return None;
    }
    String::from_utf8(bytes).ok()
}

/// A checkout's file as it is now, a regular one inside the checkout; never through a link out of it.
fn worktree_source(dir: &str, root: &str, path: &str) -> Option<String> {
    let bytes = match crate::herdr::split(dir) {
        (None, here) => {
            let checkout = std::fs::canonicalize(std::path::Path::new(here).join(root)).ok()?;
            let full = checkout.join(path);
            let meta = std::fs::symlink_metadata(&full).ok()?;
            let inside = std::fs::canonicalize(&full).ok()?.starts_with(&checkout);
            if !meta.is_file() || !inside || meta.len() > MAX_SOURCE_BYTES {
                return None;
            }
            std::fs::read(&full).ok()?
        }
        (Some(target), there) => {
            let script = format!("cd {} && cd {} && [ -f {path} ] && [ ! -L {path} ] && cat -- {path}", crate::herdr::quote(there), crate::herdr::quote(root), path = crate::herdr::quote(path));
            let out = crate::herdr::ssh(target, &script).ok()?;
            out.status.success().then_some(out.stdout)?
        }
    };
    text_source(bytes)
}

fn blob_source(dir: &str, root: &str, base: &str, path: &str) -> Option<String> {
    let spec = format!("{base}:{path}");
    text_source(run(dir, &at_root(root, &["cat-file", "blob", &spec])).ok()?)
}

/// The exact old and new documents of `file`, for its highlights; `None` for a side that doesn't exist, and for the
/// whole when one that should can't be read.
pub fn file_sources(dir: &str, diff: &CheckoutDiff, file: &FileDiff) -> Option<(Option<String>, Option<String>)> {
    let old_path = file.old_path.as_deref().unwrap_or(&file.path);
    if file.binary || !safe_path(&file.path) || !safe_path(old_path) {
        return None;
    }
    let old = match file.status {
        FileStatus::Added => None,
        _ => Some(blob_source(dir, &diff.root, &diff.base, old_path)?),
    };
    let new = match file.status {
        FileStatus::Deleted => None,
        _ => Some(worktree_source(dir, &diff.root, &file.path)?),
    };
    Some((old, new))
}

fn is_directory(dir: &str, root: &str, path: &str) -> bool {
    match crate::herdr::split(dir) {
        (None, here) => std::fs::symlink_metadata(std::path::Path::new(here).join(root).join(path)).is_ok_and(|meta| meta.is_dir()),
        (Some(target), there) => {
            let script = format!("cd {} && cd {} && [ -d {path} ] && [ ! -L {path} ]", crate::herdr::quote(there), crate::herdr::quote(root), path = crate::herdr::quote(path));
            crate::herdr::ssh(target, &script).is_ok_and(|out| out.status.success())
        }
    }
}

/// How many paths go to one `git restore` or `git clean`: Windows caps a command line at 32,767 characters.
const PATH_BATCH: usize = 64;

/// Restores staged and unstaged tracked paths to the snapshot's HEAD, then asks git to remove only the exact untracked
/// paths it reported. Ignored files are excluded by porcelain status and `git clean` omits `-x`; nested repositories
/// and submodule contents are never recursively cleaned.
pub fn discard_working_tree(dir: &str, expected_checksum: &str) -> Result<(), String> {
    let (snapshot, head) = discard_snapshot(dir)?;
    if snapshot.checksum != expected_checksum {
        return Err("working tree changed since the confirmation was opened".into());
    }
    let root = snapshot.root.as_str();
    let status = checkout_status(dir, root)?;
    let Some((tracked, untracked)) = status_paths(&status) else { return Err("cannot safely discard a path that isn't UTF-8".into()) };
    discard_paths(dir, root, &head, &tracked, &untracked)?;
    let left = capture(dir, None)?;
    match left.patch.trim().is_empty() {
        true => Ok(()),
        false => Err("some changes could not be discarded safely".into()),
    }
}

/// [`discard_working_tree`] for one file, and only as `shown`: the rest of the tree may move (an agent at work), so what
/// must hold is the file's own diff, the one confirmed over. A rename takes its old path back with it.
pub fn discard_file(dir: &str, shown: &FileDiff) -> Result<(), String> {
    let (snapshot, head) = discard_snapshot(dir)?;
    if !parse_patch(&snapshot.patch).contains(shown) {
        return Err(format!("{} changed since the confirmation was opened", shown.path));
    }
    let root = snapshot.root.as_str();
    let status = checkout_status(dir, root)?;
    let Some((mut tracked, mut untracked)) = status_paths(&status) else { return Err("cannot safely discard a path that isn't UTF-8".into()) };
    let its = |path: &String| {
        let path = path.strip_suffix('/').unwrap_or(path);
        path == shown.path || shown.old_path.as_deref() == Some(path)
    };
    tracked.retain(its);
    untracked.retain(its);
    if tracked.is_empty() && untracked.is_empty() {
        return Err(format!("{} has nothing to discard", shown.path));
    }
    discard_paths(dir, root, &head, &tracked, &untracked)?;
    match parse_patch(&capture(dir, None)?.patch).iter().any(|file| file.path == shown.path) {
        true => Err(format!("{} could not be discarded safely", shown.path)),
        false => Ok(()),
    }
}

/// The capture a discard stands on, and its HEAD.
fn discard_snapshot(dir: &str) -> Result<(CheckoutDiff, String), String> {
    let snapshot = capture(dir, None)?;
    let Some(head) = snapshot.head.clone() else { return Err("cannot discard changes before the first commit".into()) };
    if snapshot.truncated {
        return Err("cannot safely discard a partial diff snapshot".into());
    }
    Ok((snapshot, head))
}

fn discard_paths(dir: &str, root: &str, head: &str, tracked: &[String], untracked: &[String]) -> Result<(), String> {
    if untracked.iter().any(|path| path.ends_with('/')) {
        return Err("cannot discard an untracked nested repository safely".into());
    }
    // Git tracks files and symlinks, so a tracked path that is a directory on disk is a submodule or a file replaced by
    // a directory. `git restore` leaves submodule contents in place (the final check would then fail after everything
    // else was already discarded) and removes a replacing directory wholesale, ignored files included. Refuse before
    // mutating.
    if tracked.iter().any(|path| is_directory(dir, root, path)) {
        return Err("cannot discard submodule or directory changes safely".into());
    }
    let source = format!("--source={head}");
    for paths in tracked.chunks(PATH_BATCH) {
        let mut args = at_root(root, &["restore", &source, "--staged", "--worktree", "--"]);
        args.extend(paths.iter().map(String::as_str));
        run(dir, &args)?;
    }
    for paths in untracked.chunks(PATH_BATCH) {
        let mut args = at_root(root, &["clean", "-fd", "--"]);
        args.extend(paths.iter().map(String::as_str));
        run(dir, &args)?;
    }
    // ponytail: `git clean` leaves the now-empty parents of files on a machine; git doesn't see them. Here Zeron's sweep.
    if let (None, here) = crate::herdr::split(dir) {
        let checkout = std::path::Path::new(here).join(root);
        for path in untracked {
            let full = checkout.join(path);
            let mut parent = full.parent();
            while let Some(directory) = parent {
                if directory == checkout || std::fs::remove_dir(directory).is_err() {
                    break;
                }
                parent = directory.parent();
            }
        }
    }
    Ok(())
}

/// Where a capture's `path` is: on which machine (none, here), at what path.
pub fn file_location(dir: &str, root: &str, path: &str) -> (Option<String>, std::path::PathBuf) {
    let (machine, here) = crate::herdr::split(dir);
    let full = std::path::Path::new(here).join(root).join(path);
    let full = if machine.is_none() { std::fs::canonicalize(&full).unwrap_or(full) } else { full };
    (machine.map(str::to_string), full)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATCH: &str = "\
diff --git a/src/main.rs b/src/main.rs
index 111..222 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,4 +1,5 @@ fn main
 fn main() {
-    println!(\"old\");
+    println!(\"new\");
+    let x = 1;
 }
@@ -10,2 +11,2 @@
 // tail
-old_line
+new_line
diff --git a/added.txt b/added.txt
new file mode 100644
--- /dev/null
+++ b/added.txt
@@ -0,0 +1,2 @@
+first
+second
\\ No newline at end of file
diff --git a/gone.txt b/gone.txt
deleted file mode 100644
--- a/gone.txt
+++ /dev/null
@@ -1,1 +0,0 @@
-bye
diff --git a/img.png b/img.png
new file mode 100644
Binary files /dev/null and b/img.png differ
diff --git a/old_name.rs b/new_name.rs
similarity index 90%
rename from old_name.rs
rename to new_name.rs
";

    #[test]
    fn parses_files_hunks_and_lines() {
        let files = parse_patch(PATCH);
        assert_eq!(files.len(), 5);

        let main = &files[0];
        assert_eq!(main.path, "src/main.rs");
        assert_eq!(main.status, FileStatus::Modified);
        assert_eq!(main.hunks.len(), 2);
        assert_eq!(main.additions, 3);
        assert_eq!(main.deletions, 2);
        let h0 = &main.hunks[0];
        assert_eq!(h0.header, "@@ -1,4 +1,5 @@ fn main");
        assert_eq!(h0.lines.len(), 5);
        assert_eq!(h0.lines[0].kind, LineKind::Context);
        assert_eq!(h0.lines[0].old_no, Some(1));
        assert_eq!(h0.lines[0].new_no, Some(1));
        assert_eq!(h0.lines[1].kind, LineKind::Del);
        assert_eq!(h0.lines[1].old_no, Some(2));
        assert_eq!(h0.lines[1].new_no, None);
        assert_eq!(h0.lines[2].kind, LineKind::Add);
        assert_eq!(h0.lines[2].new_no, Some(2));
        assert_eq!(h0.lines[3].kind, LineKind::Add);
        assert_eq!(h0.lines[3].new_no, Some(3));
        // Closing context line: numbering advanced past the add/del block.
        assert_eq!(h0.lines[4].old_no, Some(3));
        assert_eq!(h0.lines[4].new_no, Some(4));
        // Second hunk restarts numbering from its header.
        assert_eq!(main.hunks[1].lines[0].old_no, Some(10));
        assert_eq!(main.hunks[1].lines[0].new_no, Some(11));
    }

    #[test]
    fn detects_new_deleted_binary_and_renamed() {
        let files = parse_patch(PATCH);
        let added = &files[1];
        assert_eq!(added.status, FileStatus::Added);
        assert_eq!(added.additions, 2);
        // The no-newline marker rides as a Meta line.
        let last = added.hunks[0].lines.last().unwrap();
        assert_eq!(last.kind, LineKind::Meta);
        assert!(last.text.contains("No newline"));
        assert!(file_notices(added).iter().any(|n| n == "New file"));

        let deleted = &files[2];
        assert_eq!(deleted.status, FileStatus::Deleted);
        assert_eq!(deleted.deletions, 1);
        assert!(file_notices(deleted).iter().any(|n| n == "Deleted file"));

        let binary = &files[3];
        assert!(binary.binary);
        assert_eq!(binary.status, FileStatus::Added);
        assert!(binary.hunks.is_empty());
        assert!(file_notices(binary).iter().any(|n| n.contains("Binary")));

        let renamed = &files[4];
        assert_eq!(renamed.status, FileStatus::Renamed);
        assert_eq!(renamed.path, "new_name.rs");
        assert_eq!(renamed.old_path.as_deref(), Some("old_name.rs"));
        assert!(
            file_notices(renamed)
                .iter()
                .any(|n| n.contains("old_name.rs"))
        );
    }

    #[test]
    fn empty_and_garbage_patches_parse_to_nothing() {
        assert!(parse_patch("").is_empty());
        assert!(parse_patch("not a diff\nat all\n").is_empty());
        // Truncated mid-hunk: keeps what parsed.
        let files = parse_patch("diff --git a/x b/x\n@@ -1,9 +1,9 @@\n ctx\n+add");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].hunks[0].lines.len(), 2);
        assert_eq!(files[0].additions, 1);
    }

    #[test]
    fn quoted_and_spaced_paths() {
        let (old, new) = parse_git_paths("a/simple.rs b/simple.rs");
        assert_eq!((old.as_str(), new.as_str()), ("simple.rs", "simple.rs"));
        let (old, new) = parse_git_paths("\"a/with space.rs\" \"b/with space.rs\"");
        assert_eq!(old, "with space.rs");
        assert_eq!(new, "with space.rs");
    }

    #[test]
    fn hunk_headers_parse_with_and_without_counts() {
        assert_eq!(parse_hunk_header("@@ -1,4 +2,5 @@"), Some((1, 2)));
        assert_eq!(parse_hunk_header("@@ -7 +9 @@ fn ctx"), Some((7, 9)));
        assert_eq!(parse_hunk_header("@@ garbage"), None);
    }

    #[test]
    fn rows_flatten_to_line_granularity() {
        let files = parse_patch(PATCH);
        let (rows, ranges) = flatten_rows(&files, &[], None, DiffMode::Unified, |_| false);
        assert_eq!(ranges.len(), files.len());
        // Every file's span starts with its header…
        for (ix, range) in ranges.iter().enumerate() {
            assert_eq!(rows[range.start], DiffRow::FileHeader { file: ix as u32 });
            // …and spans exactly header + analytic body rows.
            assert_eq!(range.len(), 1 + body_row_count(&files[ix]));
        }
        // Spans tile the whole row vec.
        assert_eq!(ranges.last().unwrap().end, rows.len());

        // src/main.rs: header, 2 hunk headers, 8 lines, pad.
        let main_rows = &rows[ranges[0].clone()];
        assert_eq!(main_rows.len(), 1 + 2 + 8 + 1);
        assert_eq!(main_rows[1], DiffRow::HunkHeader { file: 0, hunk: 0 });
        assert_eq!(*main_rows.last().unwrap(), DiffRow::BodyPad { file: 0 });

        // A collapsed file contributes its header row only.
        let (rows, ranges) = flatten_rows(&files, &[], None, DiffMode::Unified, |ix| ix == 0);
        assert_eq!(ranges[0].len(), 1);
        assert_eq!(rows[ranges[1].start], DiffRow::FileHeader { file: 1 });

        // Notices lead the body: the added file carries "New file".
        let added_rows = &rows[ranges[1].clone()];
        assert_eq!(added_rows[1], DiffRow::Notice { file: 1, notice: 0 });
    }

    #[test]
    fn sticky_header_tracks_the_logical_top_row() {
        let ranges = vec![0..4, 4..5, 5..10];

        assert_eq!(sticky_file_header(&[], 0, 0.0), None);
        assert_eq!(sticky_file_header(&ranges, 0, 0.0), None);
        assert_eq!(
            sticky_file_header(&ranges, 0, 0.5),
            Some(StickyFileHeader {
                file_ix: 0,
                header_row: 0,
                next_header_row: Some(4),
            })
        );
        assert_eq!(
            sticky_file_header(&ranges, 2, 0.0),
            Some(StickyFileHeader {
                file_ix: 0,
                header_row: 0,
                next_header_row: Some(4),
            })
        );

        // Landing exactly on a new header hands ownership to that file; its
        // real row remains visible until it starts crossing the viewport.
        assert_eq!(sticky_file_header(&ranges, 4, 0.0), None);
        assert_eq!(
            sticky_file_header(&ranges, 4, 1.0),
            Some(StickyFileHeader {
                file_ix: 1,
                header_row: 4,
                next_header_row: Some(5),
            })
        );
        assert_eq!(sticky_file_header(&ranges, 5, 0.0), None);
        assert_eq!(
            sticky_file_header(&ranges, 8, 0.0),
            Some(StickyFileHeader {
                file_ix: 2,
                header_row: 5,
                next_header_row: None,
            })
        );
        assert_eq!(sticky_file_header(&ranges, 10, 0.0), None);
    }

    #[test]
    fn sticky_header_is_pushed_by_the_next_file() {
        assert_eq!(sticky_header_push_offset(None), 0.0);
        assert_eq!(sticky_header_push_offset(Some(80.0)), 0.0);
        assert_eq!(sticky_header_push_offset(Some(FILE_HEADER_HEIGHT)), 0.0);
        assert_eq!(
            sticky_header_push_offset(Some(FILE_HEADER_HEIGHT - 12.0)),
            -12.0
        );
        assert_eq!(sticky_header_push_offset(Some(0.0)), -FILE_HEADER_HEIGHT);
    }

    #[test]
    fn split_pairs_align_edits_and_strand_the_rest() {
        let files = parse_patch(PATCH);
        // src/main.rs hunk 0: context, −1, +1, +1, context. The edited line
        // pairs across; the extra addition is stranded on the right.
        assert_eq!(
            split_pairs(&files[0].hunks[0].lines),
            vec![
                (Some(0), Some(0)),
                (Some(1), Some(2)),
                (None, Some(3)),
                (Some(4), Some(4)),
            ]
        );
        // A pure add: every row is right-only, including the trailing
        // no-newline Meta line — it belongs to the side it follows, and its
        // row spans both columns at render.
        assert_eq!(
            split_pairs(&files[1].hunks[0].lines),
            vec![(None, Some(0)), (None, Some(1)), (None, Some(2))]
        );
        // A pure delete strands the left.
        assert_eq!(split_pairs(&files[2].hunks[0].lines), vec![(Some(0), None)]);
        assert!(split_pairs(&[]).is_empty());

        // `-a +b -c +d` is two one-line edits, not one four-line one: a
        // deletion arriving after additions opens a new block.
        let line = |kind| DiffLine {
            kind,
            old_no: Some(1),
            new_no: Some(1),
            text: String::new(),
        };
        let lines = [
            line(LineKind::Del),
            line(LineKind::Add),
            line(LineKind::Del),
            line(LineKind::Add),
        ];
        assert_eq!(
            split_pairs(&lines),
            vec![(Some(0), Some(1)), (Some(2), Some(3))]
        );
    }

    #[test]
    fn no_newline_markers_keep_their_edit_paired() {
        // Both files lost their final newline: git writes the marker twice,
        // once per side. Neither may split the edit into one-sided rows.
        let both = "diff --git a/a.txt b/a.txt\n\
             --- a/a.txt\n\
             +++ b/a.txt\n\
             @@ -1 +1 @@\n\
             -old\n\
             \\ No newline at end of file\n\
             +new\n\
             \\ No newline at end of file\n";
        let files = parse_patch(both);
        let lines = &files[0].hunks[0].lines;
        assert_eq!(
            lines.iter().map(|line| line.kind).collect::<Vec<_>>(),
            vec![LineKind::Del, LineKind::Meta, LineKind::Add, LineKind::Meta]
        );
        // One aligned old/new row, then the two markers on one row of their
        // own — four lines read as two rows, not four.
        assert_eq!(
            split_pairs(lines),
            vec![(Some(0), Some(2)), (Some(1), Some(3))]
        );
        let full = split_pairs(lines);
        for cap in 0..=full.len() + 2 {
            assert_eq!(split_pairs_upto(lines, cap), full[..cap.min(full.len())]);
        }

        // Only the old file lacked one: the edit still pairs, and the lone
        // marker takes a row on its own side.
        let old_only = "diff --git a/a.txt b/a.txt\n\
             --- a/a.txt\n\
             +++ b/a.txt\n\
             @@ -1 +1 @@\n\
             -old\n\
             \\ No newline at end of file\n\
             +new\n";
        let files = parse_patch(old_only);
        assert_eq!(
            split_pairs(&files[0].hunks[0].lines),
            vec![(Some(0), Some(2)), (Some(1), None)]
        );
    }

    #[test]
    fn split_flattening_pairs_rows_and_keeps_heights_analytic() {
        let files = parse_patch(PATCH);
        let (rows, ranges) = flatten_rows(&files, &[], None, DiffMode::Split, |_| false);
        assert_eq!(ranges.len(), files.len());
        assert_eq!(ranges.last().unwrap().end, rows.len());

        // src/main.rs: header, 2 hunk headers, 4 + 2 paired rows, pad — the
        // same 8 lines, two columns.
        let main_rows = &rows[ranges[0].clone()];
        assert_eq!(main_rows.len(), 1 + 2 + (4 + 2) + 1);
        assert_eq!(
            main_rows[2],
            DiffRow::SplitLine {
                file: 0,
                hunk: 0,
                left: Some(0),
                right: Some(0),
            }
        );
        assert_eq!(
            main_rows[4],
            DiffRow::SplitLine {
                file: 0,
                hunk: 0,
                left: None,
                right: Some(3),
            }
        );
        assert_eq!(*main_rows.last().unwrap(), DiffRow::BodyPad { file: 0 });
        // Pairing only ever merges rows, so split is never the taller layout.
        assert!(main_rows.len() < 1 + body_row_count(&files[0]));

        // Heights stay analytic — the fold tween needs no measurement.
        assert_eq!(
            body_height(&files[0], &[], None, DiffMode::Split),
            2.0 * HUNK_HEADER_HEIGHT + 6.0 * DIFF_LINE_HEIGHT + BODY_BOTTOM_PAD
        );
    }

    #[test]
    fn capped_pairing_agrees_with_the_full_pairing_and_stays_bounded() {
        // The fold tween re-renders its stand-in every frame, so the capped
        // walk must be a true prefix of the full one — not an approximation.
        let lines = &parse_patch(PATCH)[0].hunks[0].lines;
        let full = split_pairs(lines);
        for cap in 0..=full.len() + 2 {
            assert_eq!(split_pairs_upto(lines, cap), full[..cap.min(full.len())]);
        }

        // A huge single-sided run must not be materialized to yield a few
        // rows: 20k deletions, 5 rows asked for, 5 rows built.
        let many: Vec<DiffLine> = (0..20_000u32)
            .map(|n| DiffLine {
                kind: LineKind::Del,
                old_no: Some(n + 1),
                new_no: None,
                text: String::new(),
            })
            .collect();
        let capped = split_pairs_upto(&many, 5);
        assert_eq!(capped.len(), 5);
        assert!(
            capped.capacity() < 100,
            "capacity tracks the cap, not the hunk"
        );
        assert_eq!(capped[4], (Some(4), None));
    }

    #[test]
    fn a_split_row_offers_each_column_its_own_anchor() {
        let files = parse_patch(PATCH);
        let lines = &files[0].hunks[0].lines;
        // The paired edit cites the old line on the left, the new on the right.
        assert_eq!(
            pair_anchors(lines, (Some(1), Some(2))),
            [Some((CommentSide::Old, 2)), Some((CommentSide::New, 2))]
        );
        // A context row names one anchor, not the same one twice — otherwise
        // its card would be pushed into the body in duplicate. The caller
        // flattens, so the dropped duplicate reads as an empty slot.
        assert_eq!(
            pair_anchors(lines, (Some(0), Some(0))),
            [Some((CommentSide::New, 1)), None]
        );
        // A stranded side contributes nothing.
        assert_eq!(
            pair_anchors(lines, (None, Some(3))),
            [None, Some((CommentSide::New, 3))]
        );
    }

    #[test]
    fn split_rows_carry_the_comments_of_both_columns() {
        let files = parse_patch(PATCH);
        // A context row must not stack the same card twice.
        let rows = body_rows(0, &files[0], &[comment("src/main.rs", CommentSide::New, 1, "why")], None, DiffMode::Split);
        assert_eq!(
            rows.iter()
                .filter(|row| matches!(row, DiffRow::CommentCard { .. }))
                .count(),
            1
        );

        // Both sides of one paired row hang off that row, in column order.
        let staged = vec![
            comment("src/main.rs", CommentSide::Old, 2, "left"),
            comment("src/main.rs", CommentSide::New, 2, "right"),
        ];
        let rows = body_rows(0, &files[0], &staged, None, DiffMode::Split);
        let edit = rows
            .iter()
            .position(|row| matches!(row, DiffRow::SplitLine { left: Some(1), .. }))
            .unwrap();
        assert_eq!(rows[edit + 1], DiffRow::CommentCard { file: 0, card: 0 });
        assert_eq!(rows[edit + 2], DiffRow::CommentCard { file: 0, card: 1 });
    }

    #[test]
    fn a_split_rows_right_column_is_never_a_deletion() {
        // The invariant the `+` placement rests on: only the right column is
        // hoverable, so every note a split row can start must cite the
        // post-change file. Were a deletion ever to land on the right, that
        // rule would quietly start filing notes against lines the agent
        // cannot edit.
        for file in parse_patch(PATCH) {
            for hunk in &file.hunks {
                for (_, right) in split_pairs(&hunk.lines) {
                    let Some(line) = right.and_then(|ix| hunk.lines.get(ix as usize)) else {
                        continue;
                    };
                    assert_ne!(line.kind, LineKind::Del, "{:?}", line);
                    assert!(matches!(
                        line_anchor(line),
                        None | Some((CommentSide::New, _))
                    ));
                }
            }
        }
    }

    #[test]
    fn gutters_fit_the_largest_line_number() {
        let files = parse_patch(PATCH);
        // src/main.rs second hunk ends at old 11 / new 12.
        assert_eq!(files[0].max_line, 12);
        assert_eq!(gutter_width(&files[0]), GUTTER_WIDTH);

        // Every digit count keeps ≥6px clear of the accent bar on the left
        // of the number (digits×7.2 + 8px right pad + 6px gap), and the
        // column never shrinks below the classic 36px.
        let mut file = files[0].clone();
        for digits in 1..=7u32 {
            file.max_line = 10u32.pow(digits) - 1;
            let w = gutter_width(&file);
            assert!(w >= GUTTER_WIDTH);
            let left_gap = w - (digits as f32 * 7.2 + 8.0);
            assert!(
                left_gap >= 5.99, // f32: 7 × 7.2 lands a hair under
                "{digits} digits: left gap {left_gap} < 6px"
            );
        }
        // 4 digits outgrow the classic column now (the old formula left
        // them 1.6px off the bar — visually touching).
        file.max_line = 9999;
        assert!(gutter_width(&file) > GUTTER_WIDTH);
        file.max_line = 27404;
        assert!(
            gutter_width(&file)
                > gutter_width(&{
                    let mut f = file.clone();
                    f.max_line = 9999;
                    f
                })
        );

    }

    #[test]
    fn scope_labels_and_clean_messages() {
        assert_eq!(
            scope_label(DiffScope::WorkingTree, 2, None),
            "2 Uncommitted changes"
        );
        assert_eq!(
            scope_label(DiffScope::Branch, 1, Some("main")),
            "1 Changed file vs main"
        );
        assert_eq!(scope_label(DiffScope::Branch, 3, None), "3 Changed files");
        assert_eq!(
            clean_message(DiffScope::WorkingTree, None),
            "No uncommitted changes"
        );
        assert_eq!(
            clean_message(DiffScope::Branch, Some("develop")),
            "No changes vs develop"
        );
    }

    #[test]
    fn base_ref_defaults_to_repo_default_then_main() {
        let branches =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };
        // Engine order puts the repo default first — take it when it isn't
        // the checked-out branch itself.
        let b = branches(&["main", "feature"]);
        assert_eq!(
            default_base_ref(&b, Some("feature")).as_deref(),
            Some("main")
        );
        // No origin/HEAD: engine "default" is the current branch — fall
        // through to main/master.
        let b = branches(&["feature", "main"]);
        assert_eq!(
            default_base_ref(&b, Some("feature")).as_deref(),
            Some("main")
        );
        let b = branches(&["feature", "master"]);
        assert_eq!(
            default_base_ref(&b, Some("feature")).as_deref(),
            Some("master")
        );
        // No main/master: any branch that isn't the current one.
        let b = branches(&["feature", "develop"]);
        assert_eq!(
            default_base_ref(&b, Some("feature")).as_deref(),
            Some("develop")
        );
        // Checked out ON main: comparing main with itself is the honest
        // default (empty branch diff).
        let b = branches(&["main", "feature"]);
        assert_eq!(default_base_ref(&b, Some("main")).as_deref(), Some("main"));
        // Single-branch repo, and empty list.
        let b = branches(&["main"]);
        assert_eq!(default_base_ref(&b, Some("main")).as_deref(), Some("main"));
        assert_eq!(default_base_ref(&[], Some("main")), None);
        // No origin/HEAD and the default only on the remote.
        let b = branches(&["feature", "origin/develop", "origin/main"]);
        assert_eq!(
            default_base_ref(&b, Some("feature")).as_deref(),
            Some("origin/main")
        );
    }

    #[test]
    fn full_diff_highlights_map_old_new_and_context_by_source_line() {
        let old_source = "export function old(value: string) {\n    return value.trim();\n}\n";
        let new_source = "export function new(value: string) {\n    return value.trim();\n}\n";
        let parse = |source| {
            Arc::new(
                zeron_syntax::highlight(zeron_syntax::HighlightRequest {
                    source,
                    path: Some("src/derive.ts"),
                    fence_tag: None,
                })
                .unwrap(),
            )
        };
        let highlights = DiffHighlights {
            old: Some(parse(old_source)),
            new: Some(parse(new_source)),
        };
        let deleted = DiffLine {
            kind: LineKind::Del,
            old_no: Some(1),
            new_no: None,
            text: "export function old(value: string) {".into(),
        };
        let added = DiffLine {
            kind: LineKind::Add,
            old_no: None,
            new_no: Some(1),
            text: "export function new(value: string) {".into(),
        };
        let context = DiffLine {
            kind: LineKind::Context,
            old_no: Some(2),
            new_no: Some(2),
            text: "    return value.trim();".into(),
        };
        assert_eq!(
            highlights.source_ref(&deleted),
            Some(SourceLineRef {
                side: SourceSide::Old,
                line_number: 1
            })
        );
        assert_eq!(
            highlights.source_ref(&added),
            Some(SourceLineRef {
                side: SourceSide::New,
                line_number: 1
            })
        );
        assert_eq!(
            highlights.source_ref(&context),
            Some(SourceLineRef {
                side: SourceSide::New,
                line_number: 2
            })
        );
        assert!(
            highlights
                .spans(&deleted)
                .iter()
                .any(|span| span.kind == zeron_syntax::HighlightKind::Function)
        );
        assert!(
            highlights
                .spans(&added)
                .iter()
                .any(|span| span.kind == zeron_syntax::HighlightKind::Function)
        );
    }
    fn comment(path: &str, side: CommentSide, line: u32, body: &str) -> ReviewComment {
        ReviewComment { id: u64::from(line), path: path.into(), old_path: None, side, line, body: body.into() }
    }

    #[test]
    fn the_prompt_cites_each_line_and_leaves_nothing_a_terminal_acts_on() {
        let mut moved = comment("new.rs", CommentSide::Old, 3, "first\nsecond");
        moved.old_path = Some("old.rs".into());
        let prompt = comments_prompt(&[moved, comment("new.rs", CommentSide::New, 4, "\x1b[201~then\rthis")]);
        assert!(prompt.contains("- old.rs:3 (L): first\n  second"));
        assert!(prompt.contains("- new.rs:4 (R): [201~thenthis"));
        assert!(!prompt.chars().any(|c| c.is_control() && c != '\n'));
    }

    #[test]
    fn branch_search_ranks_prefixes_first() {
        assert_eq!(filter_indices("ma", &["feature/main", "main", "xyz"]), [1, 0]);
        assert_eq!(filter_indices("", &["b", "a"]), [0, 1]);
    }

    /// `path` in a throwaway repo, a commit on `main` holding `a.txt`, `b.txt`, a `.gitignore` for `*.log`.
    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        // autocrlf off: Git for Windows has it on, and a discard's checkout would write the files back with CRLF
        sh(dir.path(), "git init -q -b main && git config user.email t@t && git config user.name t && git config core.autocrlf false && printf 'one\ntwo\n' > a.txt && printf 'keep\n' > b.txt && printf '*.log\n' > .gitignore && mkdir sub && printf 'x\n' > sub/c.txt && git add -A && git commit -qm base");
        dir
    }

    fn sh(dir: &std::path::Path, script: &str) {
        let out = std::process::Command::new("sh").arg("-c").arg(script).current_dir(dir).output().unwrap();
        assert!(out.status.success(), "{script}: {}", String::from_utf8_lossy(&out.stderr));
    }

    #[cfg(unix)] // its tests are unix-only: on Windows it would be dead code, an error under clippy's -D warnings
    fn take(dir: &tempfile::TempDir, base: Option<&str>) -> (CheckoutDiff, Vec<FileDiff>) {
        let diff = capture(dir.path().to_str().unwrap(), base).unwrap();
        let files = parse_patch(&diff.patch);
        (diff, files)
    }

    #[cfg(unix)]
    #[test]
    fn a_capture_holds_edits_and_untracked_files_from_anywhere_in_the_repo() {
        let dir = repo();
        sh(dir.path(), "printf 'one\nTWO\n' > a.txt && printf 'new\nfile\n' > 'with space.txt' && printf 'ignored\n' > x.log && git config diff.noprefix true");
        let inner = dir.path().join("sub");
        let diff = capture(inner.to_str().unwrap(), None).unwrap();
        let files = parse_patch(&diff.patch);
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["a.txt", "with space.txt"]);
        assert_eq!((files[0].additions, files[0].deletions), (1, 1));
        assert_eq!((files[1].status, files[1].additions), (FileStatus::Added, 2));
        let (_, new) = file_sources(inner.to_str().unwrap(), &diff, &files[0]).unwrap();
        assert_eq!(new.as_deref(), Some("one\nTWO\n"));
    }

    #[cfg(unix)]
    #[test]
    fn a_clean_tree_captures_empty_and_outside_a_repo_is_an_error() {
        let dir = repo();
        assert!(take(&dir, None).0.patch.is_empty());
        let plain = tempfile::tempdir().unwrap();
        assert!(capture(plain.path().to_str().unwrap(), None).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_repo_with_no_commit_diffs_against_the_empty_tree() {
        let dir = tempfile::tempdir().unwrap();
        sh(dir.path(), "git init -q -b main && printf 'hi\n' > new.txt && git add new.txt");
        let (diff, files) = take(&dir, None);
        assert_eq!(diff.head, None);
        assert_eq!(files[0].status, FileStatus::Added);
    }

    #[cfg(unix)]
    #[test]
    fn the_branch_scope_diffs_against_the_merge_base_and_keeps_the_working_tree() {
        let dir = repo();
        sh(dir.path(), "git switch -qc feature && printf 'feat\n' > f.txt && git add -A && git commit -qm feat && git switch -q main && printf 'moved\n' > m.txt && git add -A && git commit -qm moved && git switch -q feature && printf 'edit\n' >> b.txt");
        let (_, files) = take(&dir, Some("main"));
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["b.txt", "f.txt"]);
        assert_eq!(list_branches(dir.path().to_str().unwrap()).unwrap()[0], "feature");
    }

    #[cfg(unix)]
    #[test]
    fn a_branch_only_on_the_remote_keeps_its_remote_and_diffs() {
        let upstream = repo();
        let clone = tempfile::tempdir().unwrap();
        sh(clone.path(), &format!("git clone -q {} . && git switch -qc feature && git branch -qD main && printf 'edit\n' >> a.txt", upstream.path().display()));
        let branches = list_branches(clone.path().to_str().unwrap()).unwrap();
        assert_eq!(branches, ["origin/main", "feature"]);
        assert_eq!(default_base_ref(&branches, Some("feature")).as_deref(), Some("origin/main"));
        let (_, files) = take(&clone, Some("origin/main"));
        assert_eq!(files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["a.txt"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_local_branch_hides_its_remote_copy() {
        let upstream = repo();
        let clone = tempfile::tempdir().unwrap();
        sh(clone.path(), &format!("git clone -q {} . && git switch -qc feature", upstream.path().display()));
        assert_eq!(list_branches(clone.path().to_str().unwrap()).unwrap(), ["main", "feature"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_rename_keeps_both_paths() {
        let dir = repo();
        sh(dir.path(), "git mv a.txt moved.txt");
        let (_, files) = take(&dir, None);
        assert_eq!((files[0].status, files[0].path.as_str(), files[0].old_path.as_deref()), (FileStatus::Renamed, "moved.txt", Some("a.txt")));
    }

    #[cfg(unix)]
    #[test]
    fn a_big_or_binary_file_is_named_not_read() {
        let dir = repo();
        sh(dir.path(), "yes line | head -c 3000000 > big.txt && printf 'a\\000b' > bin.dat");
        let (diff, files) = take(&dir, None);
        assert!(files.iter().all(|f| f.binary), "{:?}", files.iter().map(|f| (&f.path, f.binary)).collect::<Vec<_>>());
        assert_eq!(files.len(), 2);
        assert!(diff.patch.len() < 1024);
    }

    #[cfg(unix)]
    #[test]
    fn a_patch_past_the_limit_is_cut_at_a_line_and_marked() {
        let dir = repo();
        sh(dir.path(), "yes 'a line of text' | head -c 1500000 > one.txt && yes 'b line of text' | head -c 1500000 > two.txt && yes 'c line of text' | head -c 1500000 > three.txt");
        let (diff, _) = take(&dir, None);
        assert!(diff.truncated);
        assert!(diff.patch.len() <= MAX_PATCH_BYTES);
    }

    #[cfg(unix)]
    #[test]
    fn a_discard_restores_tracked_files_and_removes_exactly_the_untracked_ones() {
        let dir = repo();
        sh(dir.path(), "printf 'edited\n' > a.txt && git add a.txt && rm b.txt && mkdir -p fresh/deep && printf 'new\n' > fresh/deep/n.txt && printf 'ignored\n' > x.log");
        let path = dir.path().to_str().unwrap();
        let (diff, _) = take(&dir, None);
        discard_working_tree(path, &diff.checksum).unwrap();
        assert_eq!(std::fs::read_to_string(dir.path().join("a.txt")).unwrap(), "one\ntwo\n");
        assert!(dir.path().join("b.txt").exists());
        assert!(!dir.path().join("fresh").exists());
        assert!(dir.path().join("x.log").exists());
    }

    #[test]
    fn a_turn_runs_from_the_first_agent_at_work_to_the_last_at_rest() {
        use crate::model::AgentStatus::{Blocked, Done, Idle, Working};
        let mut turns = Turns::default();
        let edge = |started, ended| TurnEdge { started, ended };
        assert_eq!(turns.observe([Working]), edge(false, false), "the first look finds one going on, it saw no start");
        assert_eq!(turns.observe([Idle]), edge(false, true));
        assert_eq!(turns.observe([Idle, Done]), edge(false, false));
        assert_eq!(turns.observe([Working, Idle]), edge(true, false));
        assert_eq!(turns.observe([Blocked, Idle]), edge(false, false), "asking leaves it open");
        assert_eq!(turns.observe([Working, Idle]), edge(false, false), "and going on starts no other");
        assert_eq!(turns.observe([Idle, Working]), edge(false, false));
        assert_eq!(turns.observe([Done, Idle]), edge(false, true));
        assert_eq!(turns.observe([]), edge(false, false));
    }

    #[test]
    fn a_question_between_turns_doesnt_hide_the_next_ones_start() {
        use crate::model::AgentStatus::{Blocked, Idle, Unknown, Working};
        let mut turns = Turns::default();
        turns.observe([Idle]);
        assert_eq!(turns.observe([Blocked]), TurnEdge { started: false, ended: false }, "asking starts none");
        assert_eq!(turns.observe([Unknown]), TurnEdge { started: false, ended: false });
        assert_eq!(turns.observe([Working]), TurnEdge { started: true, ended: false });
    }

    #[cfg(unix)]
    #[test]
    fn the_last_turn_holds_what_changed_since_it_began_and_only_that() {
        let dir = repo();
        let path = dir.path().to_str().unwrap();
        sh(dir.path(), "printf 'there before\n' > before.txt && printf 'edited before\n' > b.txt");
        assert!(capture_turn(path).unwrap().patch.is_empty(), "no turn yet");

        let start = snapshot_tree(path).unwrap();
        assert!(!turn_moved(path, &start).unwrap());
        assert!(capture_turn(path).unwrap().patch.is_empty(), "a turn that changed nothing isn't one");

        sh(dir.path(), "printf 'by the agent\n' > a.txt && printf 'new\n' > fresh.txt");
        assert!(turn_moved(path, &start).unwrap());
        let files = parse_patch(&capture_turn(path).unwrap().patch);
        assert_eq!(files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["a.txt", "fresh.txt"]);

        let status = run(path, &["status", "--porcelain"]).unwrap();
        assert_eq!(String::from_utf8_lossy(&status), " M a.txt\n M b.txt\n?? before.txt\n?? fresh.txt\n", "the index stays as it was");
    }

    #[cfg(unix)]
    #[test]
    fn snapshots_running_at_once_dont_share_an_index() {
        let dir = repo();
        let path = dir.path().to_str().unwrap();
        let tree = snapshot_tree(path).unwrap();
        std::thread::scope(|scope| {
            for _ in 0..6 {
                scope.spawn(|| (0..15).for_each(|_| assert_eq!(snapshot_tree(path).as_ref(), Ok(&tree))));
            }
        });
    }

    #[cfg(unix)]
    #[test]
    fn a_snapshot_that_cant_add_everything_is_an_error() {
        let dir = repo();
        sh(dir.path(), "git init -q nested"); // a repo with no commit: `git add` refuses it
        assert!(snapshot_tree(dir.path().to_str().unwrap()).is_err());
    }

    #[test]
    fn files_sort_as_lazygits_tree_folders_first_at_every_level() {
        let mut paths = vec!["z.rs", "src/z.rs", "a.rs", "src/ui/mod.rs", "src/a.rs", "assets/x.png", "src/ui/deep/y.rs", "src/ui-kit/b.rs"];
        paths.sort_by(|a, b| tree_order(a, b));
        assert_eq!(paths, ["assets/x.png", "src/ui/deep/y.rs", "src/ui/mod.rs", "src/ui-kit/b.rs", "src/a.rs", "src/z.rs", "a.rs", "z.rs"]);
    }

    #[test]
    fn a_file_discard_takes_that_file_and_leaves_the_rest_moving() {
        let dir = repo();
        sh(dir.path(), "printf 'edited\n' > a.txt && printf 'edited\n' > b.txt && mkdir -p fresh/deep && printf 'new\n' > fresh/deep/n.txt");
        let path = dir.path().to_str().unwrap();
        let file = |name: &str| parse_patch(&capture(path, None).unwrap().patch).into_iter().find(|f| f.path == name).unwrap();
        let (a, new) = (file("a.txt"), file("fresh/deep/n.txt"));
        sh(dir.path(), "printf 'moved on\n' > b.txt"); // the agent keeps working elsewhere
        discard_file(path, &a).unwrap();
        discard_file(path, &new).unwrap();
        assert_eq!(std::fs::read_to_string(dir.path().join("a.txt")).unwrap(), "one\ntwo\n");
        assert!(!dir.path().join("fresh").exists());
        assert_eq!(std::fs::read_to_string(dir.path().join("b.txt")).unwrap(), "moved on\n");
    }

    #[test]
    fn a_file_discard_refuses_a_diff_that_moved_and_undoes_a_rename_whole() {
        let dir = repo();
        sh(dir.path(), "printf 'edited\n' > a.txt");
        let path = dir.path().to_str().unwrap();
        let shown = parse_patch(&capture(path, None).unwrap().patch).remove(0);
        sh(dir.path(), "printf 'edited again\n' > a.txt");
        assert!(discard_file(path, &shown).unwrap_err().contains("changed"));
        assert_eq!(std::fs::read_to_string(dir.path().join("a.txt")).unwrap(), "edited again\n");

        sh(dir.path(), "git checkout -q a.txt && git mv a.txt moved.txt");
        let renamed = parse_patch(&capture(path, None).unwrap().patch).remove(0);
        assert_eq!(renamed.old_path.as_deref(), Some("a.txt"));
        discard_file(path, &renamed).unwrap();
        assert!(dir.path().join("a.txt").exists());
        assert!(!dir.path().join("moved.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_discard_refuses_a_tree_that_moved_and_a_nested_repo() {
        let dir = repo();
        sh(dir.path(), "printf 'edited\n' > a.txt");
        let path = dir.path().to_str().unwrap();
        let (diff, _) = take(&dir, None);
        sh(dir.path(), "printf 'again\n' > a.txt");
        assert!(discard_working_tree(path, &diff.checksum).unwrap_err().contains("changed"));
        assert_eq!(std::fs::read_to_string(dir.path().join("a.txt")).unwrap(), "again\n");

        sh(dir.path(), "mkdir nested && cd nested && git init -q && git -c user.email=t@t -c user.name=t commit -q --allow-empty -m x && printf 'mine\n' > mine.txt");
        let (diff, _) = take(&dir, None);
        assert!(discard_working_tree(path, &diff.checksum).unwrap_err().contains("nested"));
        assert!(dir.path().join("nested/mine.txt").exists());
    }

    #[test]
    fn a_path_from_a_patch_stays_in_the_checkout() {
        assert!(safe_path("src/main.rs"));
        assert!(!safe_path("../secret"));
        assert!(!safe_path("/etc/passwd"));
        assert!(!safe_path(""));
    }

    #[cfg(unix)]
    #[test]
    fn a_source_that_left_the_patch_is_not_highlighted() {
        let dir = repo();
        sh(dir.path(), "printf 'fn main() {}\n' > m.rs && git add -A && git commit -qm m && printf 'fn other() {}\n' > m.rs");
        let (diff, files) = take(&dir, None);
        let path = dir.path().to_str().unwrap();
        let (old, new) = file_sources(path, &diff, &files[0]).unwrap();
        let lang = zeron_syntax::language_for_path("m.rs").unwrap();
        assert!(full_highlights(&files[0], lang, old.as_deref(), new.as_deref()).is_some());
        assert!(full_highlights(&files[0], lang, old.as_deref(), Some("fn third() {}\n")).is_none());
    }
}
