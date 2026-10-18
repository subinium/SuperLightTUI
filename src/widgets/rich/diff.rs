//! Diff viewer state — unified-diff model, parser and per-hunk review state.
//!
//! Rendered by `context/widgets/rich/diff.rs` ([`crate::Context::diff`]).
//!
//! The model is `files -> hunks -> lines`. [`DiffState::from_unified`] parses
//! a unified diff (plain `diff -u` output or `git diff` output with extended
//! headers) without any new dependency, and [`DiffState::to_unified`] writes
//! the model back out. Per-hunk review state (collapsed flag and
//! [`HunkDecision`]) lives next to the model so it survives across frames.

use super::*;
use crate::event::KeyCode;
use crate::style::{Color, SyntaxPalette};
use std::sync::Arc;

mod parse;

#[cfg(test)]
mod tests;

/// Kind of a single line inside a diff hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiffLineKind {
    /// Unchanged line present on both sides (` ` prefix).
    Context,
    /// Line present only on the new side (`+` prefix).
    Added,
    /// Line present only on the old side (`-` prefix).
    Removed,
}

/// One line of a diff hunk, with its old/new line numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DiffLine {
    /// Whether the line is context, added or removed.
    pub kind: DiffLineKind,
    /// Line text without the leading `+` / `-` / ` ` marker and without the
    /// trailing newline (a trailing `\r` from CRLF patches is stripped too).
    pub text: String,
    /// 1-based line number in the old file (`None` for added lines).
    pub old_line: Option<u32>,
    /// 1-based line number in the new file (`None` for removed lines).
    pub new_line: Option<u32>,
    /// `true` when the line was followed by `\ No newline at end of file`.
    pub no_newline: bool,
}

/// One `@@ -a,b +c,d @@` hunk of a file diff.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DiffHunk {
    /// First old-side line covered by the hunk (`a` in `-a,b`).
    pub old_start: u32,
    /// Number of old-side lines (context + removed) in the hunk.
    pub old_count: u32,
    /// First new-side line covered by the hunk (`c` in `+c,d`).
    pub new_start: u32,
    /// Number of new-side lines (context + added) in the hunk.
    pub new_count: u32,
    /// Optional section heading after the closing `@@` (often a function
    /// signature). Empty when absent.
    pub section: String,
    /// Hunk body lines in patch order.
    pub lines: Vec<DiffLine>,
}

impl DiffHunk {
    /// Number of added lines in this hunk.
    pub fn added(&self) -> usize {
        self.lines
            .iter()
            .filter(|line| line.kind == DiffLineKind::Added)
            .count()
    }

    /// Number of removed lines in this hunk.
    pub fn removed(&self) -> usize {
        self.lines
            .iter()
            .filter(|line| line.kind == DiffLineKind::Removed)
            .count()
    }
}

/// What happened to a file in a diff.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiffFileStatus {
    /// Content (or mode) changed in place.
    Modified,
    /// File was created (`new file mode` or `--- /dev/null`).
    Added,
    /// File was deleted (`deleted file mode` or `+++ /dev/null`).
    Deleted,
    /// File was renamed (`rename from` / `rename to`).
    Renamed,
    /// File was copied (`copy from` / `copy to`).
    Copied,
}

/// One file section of a unified diff.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DiffFile {
    /// Path on the old side, `None` for created files. `a/` prefixes are
    /// stripped.
    pub old_path: Option<String>,
    /// Path on the new side, `None` for deleted files. `b/` prefixes are
    /// stripped.
    pub new_path: Option<String>,
    /// How the file changed.
    pub status: DiffFileStatus,
    /// `true` for binary files (`Binary files ... differ` or a
    /// `GIT binary patch`). Binary content is reported but never rendered.
    pub binary: bool,
    /// Text hunks in patch order.
    pub hunks: Vec<DiffHunk>,
}

impl DiffFile {
    fn empty() -> Self {
        Self {
            old_path: None,
            new_path: None,
            status: DiffFileStatus::Modified,
            binary: false,
            hunks: Vec::new(),
        }
    }

    /// Display path: the new path, or the old path for deleted files.
    pub fn path(&self) -> &str {
        self.new_path
            .as_deref()
            .or(self.old_path.as_deref())
            .unwrap_or("")
    }

    /// Language hint derived from the file extension (e.g. `"rs"`), suitable
    /// for the syntax highlighter. Empty when the path has no extension.
    pub fn lang_hint(&self) -> &str {
        let path = self.path();
        let name = path.rsplit('/').next().unwrap_or(path);
        let Some((stem, ext)) = name.rsplit_once('.') else {
            return "";
        };
        if stem.is_empty() {
            return "";
        }
        match ext {
            "mjs" | "cjs" => "js",
            "pyi" => "py",
            "hh" | "hxx" => "cpp",
            "mts" | "cts" => "ts",
            other => other,
        }
    }
}

/// What kind of malformed input [`DiffState::from_unified`] rejected.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiffParseErrorKind {
    /// Non-empty input contained no file header and no hunk.
    NotADiff,
    /// A line starting with `@@` is not a valid `@@ -a,b +c,d @@` header.
    InvalidHunkHeader,
    /// A hunk body line has an unknown prefix, or more lines than the hunk
    /// header announced for one side.
    UnexpectedLine,
    /// The input ended before a hunk had all the lines its header announced.
    TruncatedHunk,
    /// `\ No newline at end of file` appeared with no preceding hunk line.
    OrphanNoNewlineMarker,
    /// A hunk range does not fit in `u32` line numbers.
    LineNumberOverflow,
}

/// Error returned by [`DiffState::from_unified`] for malformed input.
///
/// The parser never panics; every rejected input maps to one of the
/// [`DiffParseErrorKind`] variants together with the 1-based input line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffParseError {
    line: usize,
    kind: DiffParseErrorKind,
}

impl DiffParseError {
    pub(crate) fn new(line: usize, kind: DiffParseErrorKind) -> Self {
        Self { line, kind }
    }

    /// 1-based input line where the problem was detected.
    pub fn line(&self) -> usize {
        self.line
    }

    /// What was wrong with the input.
    pub fn kind(&self) -> DiffParseErrorKind {
        self.kind
    }
}

impl std::fmt::Display for DiffParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let what = match self.kind {
            DiffParseErrorKind::NotADiff => "input is not a unified diff",
            DiffParseErrorKind::InvalidHunkHeader => "invalid hunk header",
            DiffParseErrorKind::UnexpectedLine => "unexpected line in hunk",
            DiffParseErrorKind::TruncatedHunk => "hunk ended early",
            DiffParseErrorKind::OrphanNoNewlineMarker => {
                "`\\ No newline at end of file` without a preceding line"
            }
            DiffParseErrorKind::LineNumberOverflow => "hunk range overflows u32",
        };
        write!(f, "{what} at line {}", self.line)
    }
}

impl std::error::Error for DiffParseError {}

/// Identifies one hunk: file index and hunk index within that file, both
/// matching [`DiffState::files`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HunkId {
    /// Index into [`DiffState::files`].
    pub file: usize,
    /// Index into that file's [`DiffFile::hunks`].
    pub hunk: usize,
}

/// Stored review decision for one hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HunkDecision {
    /// Not reviewed yet.
    #[default]
    Pending,
    /// The user accepted the hunk.
    Accepted,
    /// The user rejected the hunk.
    Rejected,
}

/// A review action reported once in [`DiffResponse::hunk_actions`] on the
/// frame the decision changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HunkAction {
    /// The hunk was marked [`HunkDecision::Accepted`].
    Accept,
    /// The hunk was marked [`HunkDecision::Rejected`].
    Reject,
}

/// Diff layout passed to [`DiffView::layout`](crate::DiffView::layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DiffLayout {
    /// One column with `+` / `-` markers and an old/new line-number gutter.
    #[default]
    Unified,
    /// Old and new side by side, blank rows aligned. Falls back to
    /// [`DiffLayout::Unified`] below the
    /// [`split_min_width`](crate::DiffView::split_min_width) threshold.
    Split,
}

/// Key bindings used by [`Context::diff`](crate::Context::diff) while the
/// viewer is focused.
///
/// Keys match presses without Ctrl / Alt / Super. `PageUp`, `PageDown`,
/// `Home` and `End` always scroll by a page or to the ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffKeys {
    /// Move the cursor up one row. Default: `Up`, `k`.
    pub up: Vec<KeyCode>,
    /// Move the cursor down one row. Default: `Down`, `j`.
    pub down: Vec<KeyCode>,
    /// Jump to the next hunk header. Default: `]`, `n`.
    pub next_hunk: Vec<KeyCode>,
    /// Jump to the previous hunk header. Default: `[`, `p`.
    pub prev_hunk: Vec<KeyCode>,
    /// Collapse or expand the focused hunk. Default: `Enter`, `Space`.
    pub toggle: Vec<KeyCode>,
    /// Accept the focused hunk. Default: `a`.
    pub accept: Vec<KeyCode>,
    /// Reject the focused hunk. Default: `r`.
    pub reject: Vec<KeyCode>,
}

impl Default for DiffKeys {
    fn default() -> Self {
        Self {
            up: vec![KeyCode::Up, KeyCode::Char('k')],
            down: vec![KeyCode::Down, KeyCode::Char('j')],
            next_hunk: vec![KeyCode::Char(']'), KeyCode::Char('n')],
            prev_hunk: vec![KeyCode::Char('['), KeyCode::Char('p')],
            toggle: vec![KeyCode::Enter, KeyCode::Char(' ')],
            accept: vec![KeyCode::Char('a')],
            reject: vec![KeyCode::Char('r')],
        }
    }
}

/// Optional color overrides for the diff viewer.
///
/// Every `None` falls back to a color derived from the active theme:
/// `added_bg` blends [`Theme::success`](crate::Theme::success) into
/// `surface`, `removed_bg` blends [`Theme::error`](crate::Theme::error) into
/// `surface`, and `hunk_fg` uses [`Theme::text_dim`](crate::Theme::text_dim),
/// so existing themes need no edits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiffColors {
    /// Background of added lines.
    pub added_bg: Option<Color>,
    /// Background of removed lines.
    pub removed_bg: Option<Color>,
    /// Foreground of hunk headers.
    pub hunk_fg: Option<Color>,
}

/// Response from [`Context::diff`](crate::Context::diff).
///
/// Derefs to the viewer's [`Response`] for `hovered`, `focused`, `rect`.
/// `changed` is `true` when a decision or a collapsed flag changed this
/// frame.
#[derive(Debug, Clone, Default)]
#[must_use = "DiffResponse reports per-hunk decisions — check .hunk_actions"]
pub struct DiffResponse {
    /// Interaction response for the whole viewer.
    pub response: Response,
    /// Decisions made this frame, in input order. A decision is reported
    /// exactly once — on the frame it changed — and stays readable through
    /// [`DiffState::decision`] afterwards.
    pub hunk_actions: Vec<(HunkId, HunkAction)>,
    /// Hunks whose collapsed flag was toggled this frame (keyboard or click).
    pub toggled: Vec<HunkId>,
    /// Hunk under the cursor after this frame's input, if any.
    pub focused_hunk: Option<HunkId>,
}

impl std::ops::Deref for DiffResponse {
    type Target = Response;
    fn deref(&self) -> &Response {
        &self.response
    }
}

/// One visual row of the diff viewer. Indices are `u32` to keep the cached
/// row list compact for large patches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiffRow {
    /// File header for `files[i]`.
    File(u32),
    /// "binary file" notice for `files[i]`.
    Binary(u32),
    /// Header of global hunk `g`.
    Hunk(u32),
    /// Unified layout: line `line` of global hunk `hunk`.
    Line { hunk: u32, line: u32 },
    /// Split layout: aligned old / new line indices of global hunk `hunk`.
    Pair {
        hunk: u32,
        old: Option<u32>,
        new: Option<u32>,
    },
}

/// Cached row list for one layout + collapse configuration.
#[derive(Debug, Clone, Default)]
pub(crate) struct DiffRows {
    /// `Some(split)` when `rows` is valid for that layout.
    pub(crate) built_for: Option<bool>,
    pub(crate) rows: Vec<DiffRow>,
    /// Row index of each global hunk header.
    pub(crate) hunk_rows: Vec<usize>,
    /// Row index of each file header.
    pub(crate) file_rows: Vec<usize>,
}

type HighlightedLines = Vec<Vec<(String, Style)>>;

/// Per-hunk syntax highlight of the reconstructed old / new text.
#[derive(Debug, Clone, Default)]
pub(crate) struct HunkHighlight {
    pub(crate) old: Option<Arc<HighlightedLines>>,
    pub(crate) new: Option<Arc<HighlightedLines>>,
}

/// Lazily filled highlight cache; reset when the language override or the
/// theme's syntax colors change.
#[derive(Debug, Clone, Default)]
pub(crate) struct DiffHighlightCache {
    pub(crate) key: Option<(Option<String>, Color, SyntaxPalette)>,
    pub(crate) hunks: Vec<Option<HunkHighlight>>,
}

/// State for the diff viewer widget ([`Context::diff`](crate::Context::diff)).
///
/// Holds the parsed patch, the cursor / scroll position, and per-hunk
/// collapsed flags and [`HunkDecision`]s.
///
/// # Example
///
/// ```
/// use slt::{DiffState, HunkDecision, HunkId};
///
/// let patch = "--- a/greet.txt\n+++ b/greet.txt\n@@ -1 +1 @@\n-hello\n+hello, world\n";
/// let mut diff = DiffState::from_unified(patch).expect("valid patch");
/// assert_eq!(diff.files()[0].path(), "greet.txt");
/// let id = HunkId { file: 0, hunk: 0 };
/// diff.set_decision(id, HunkDecision::Accepted);
/// assert_eq!(diff.decision(id), Some(HunkDecision::Accepted));
/// ```
#[derive(Debug, Clone, Default)]
pub struct DiffState {
    files: Vec<DiffFile>,
    /// Global hunk index -> id.
    hunk_ids: Vec<HunkId>,
    /// File index -> global index of its first hunk.
    hunk_offsets: Vec<usize>,
    /// Per-file `(added, removed)` line counts.
    file_stats: Vec<(usize, usize)>,
    collapsed: Vec<bool>,
    decisions: Vec<HunkDecision>,
    /// Digits needed for the widest line number in the patch.
    line_no_width: usize,
    /// Key bindings used while the viewer is focused.
    pub keys: DiffKeys,
    /// Cursor row index into the cached row list.
    pub(crate) cursor: usize,
    /// First visible row.
    pub(crate) scroll: usize,
    /// Whether the cursor was placed on the first hunk yet.
    pub(crate) cursor_ready: bool,
    pub(crate) rows: DiffRows,
    pub(crate) highlight: DiffHighlightCache,
}

impl DiffState {
    /// Create an empty diff viewer state (no files).
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a unified diff.
    ///
    /// Accepts plain `diff -u` output and `git diff` / `git format-patch`
    /// output: multiple files, `new file` / `deleted file` / `rename` /
    /// `copy` headers, `Binary files ... differ` and `GIT binary patch`
    /// markers (reported via [`DiffFile::binary`], never rendered), and
    /// `\ No newline at end of file`. Text before the first file header
    /// (commit messages, `Index:` lines) and between hunks is ignored.
    /// Empty input yields an empty state.
    ///
    /// # Errors
    ///
    /// Returns [`DiffParseError`] for malformed hunk headers, hunk bodies
    /// that disagree with their header counts, orphan no-newline markers,
    /// line-number overflow, and non-empty input with no diff content. The
    /// parser never panics.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::DiffState;
    ///
    /// let diff = DiffState::from_unified("@@ -1 +1 @@\n-a\n+b\n").unwrap();
    /// assert_eq!(diff.hunk_count(), 1);
    /// assert!(DiffState::from_unified("@@ nonsense @@\n").is_err());
    /// ```
    pub fn from_unified(patch: &str) -> Result<Self, DiffParseError> {
        Ok(Self::from_files(parse::parse_unified(patch)?))
    }

    fn from_files(files: Vec<DiffFile>) -> Self {
        let mut hunk_ids = Vec::new();
        let mut hunk_offsets = Vec::with_capacity(files.len());
        let mut file_stats = Vec::with_capacity(files.len());
        let mut max_line: u64 = 1;
        for (file_idx, file) in files.iter().enumerate() {
            hunk_offsets.push(hunk_ids.len());
            let mut added = 0;
            let mut removed = 0;
            for (hunk_idx, hunk) in file.hunks.iter().enumerate() {
                hunk_ids.push(HunkId {
                    file: file_idx,
                    hunk: hunk_idx,
                });
                added += hunk.added();
                removed += hunk.removed();
                max_line = max_line
                    .max(u64::from(hunk.old_start) + u64::from(hunk.old_count))
                    .max(u64::from(hunk.new_start) + u64::from(hunk.new_count));
            }
            file_stats.push((added, removed));
        }
        let hunks = hunk_ids.len();
        Self {
            files,
            hunk_ids,
            hunk_offsets,
            file_stats,
            collapsed: vec![false; hunks],
            decisions: vec![HunkDecision::Pending; hunks],
            line_no_width: max_line.to_string().len(),
            ..Self::default()
        }
    }

    /// Serialize the model back to a git-style unified diff.
    ///
    /// The output re-parses to the same [`DiffState::files`] (review state is
    /// not part of the text). File modes are not tracked, so created and
    /// deleted files are written with mode `100644`.
    pub fn to_unified(&self) -> String {
        parse::write_unified(&self.files)
    }

    /// Parsed files in patch order.
    pub fn files(&self) -> &[DiffFile] {
        &self.files
    }

    /// `true` when the patch has no files.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Total number of hunks across all files.
    pub fn hunk_count(&self) -> usize {
        self.hunk_ids.len()
    }

    /// Every hunk id in patch order.
    pub fn hunk_ids(&self) -> impl Iterator<Item = HunkId> + '_ {
        self.hunk_ids.iter().copied()
    }

    /// The hunk for `id`, or `None` when out of range.
    pub fn hunk(&self, id: HunkId) -> Option<&DiffHunk> {
        self.files.get(id.file)?.hunks.get(id.hunk)
    }

    /// Stored decision for `id`, or `None` when out of range.
    pub fn decision(&self, id: HunkId) -> Option<HunkDecision> {
        self.global(id).map(|g| self.decisions[g])
    }

    /// Set the decision for `id`. Returns `false` when `id` is out of range.
    /// Programmatic changes are not reported in [`DiffResponse::hunk_actions`].
    pub fn set_decision(&mut self, id: HunkId, decision: HunkDecision) -> bool {
        match self.global(id) {
            Some(g) => {
                self.decisions[g] = decision;
                true
            }
            None => false,
        }
    }

    /// Whether the body of `id` is hidden. `false` for out-of-range ids.
    pub fn is_collapsed(&self, id: HunkId) -> bool {
        self.global(id).is_some_and(|g| self.collapsed[g])
    }

    /// Collapse or expand `id`. Returns `false` when `id` is out of range.
    pub fn set_collapsed(&mut self, id: HunkId, collapsed: bool) -> bool {
        match self.global(id) {
            Some(g) => {
                self.set_collapsed_global(g, collapsed);
                true
            }
            None => false,
        }
    }

    /// Collapse every hunk.
    pub fn collapse_all(&mut self) {
        self.collapsed.iter_mut().for_each(|c| *c = true);
        self.rows.built_for = None;
    }

    /// Expand every hunk.
    pub fn expand_all(&mut self) {
        self.collapsed.iter_mut().for_each(|c| *c = false);
        self.rows.built_for = None;
    }

    /// Hunk under the cursor, or `None` when the cursor is on a file header
    /// or the viewer has not rendered yet.
    pub fn focused_hunk(&self) -> Option<HunkId> {
        self.rows.built_for?;
        self.row_hunk(self.cursor)
            .map(|g| self.hunk_ids[g as usize])
    }

    /// Move the cursor to the header of `id` (scrolled into view on the next
    /// render). Returns `false` when `id` is out of range.
    pub fn focus_hunk(&mut self, id: HunkId) -> bool {
        let Some(g) = self.global(id) else {
            return false;
        };
        let split = self.rows.built_for.unwrap_or(false);
        self.ensure_rows(split);
        self.cursor = self.rows.hunk_rows[g];
        true
    }

    // ── crate-internal helpers used by the renderer ───────────────────

    pub(crate) fn global(&self, id: HunkId) -> Option<usize> {
        let offset = *self.hunk_offsets.get(id.file)?;
        if id.hunk < self.files[id.file].hunks.len() {
            Some(offset + id.hunk)
        } else {
            None
        }
    }

    pub(crate) fn id_of(&self, global: usize) -> HunkId {
        self.hunk_ids[global]
    }

    pub(crate) fn hunk_global(&self, global: usize) -> &DiffHunk {
        let id = self.hunk_ids[global];
        &self.files[id.file].hunks[id.hunk]
    }

    pub(crate) fn file_stats(&self, file: usize) -> (usize, usize) {
        self.file_stats.get(file).copied().unwrap_or((0, 0))
    }

    pub(crate) fn line_no_width(&self) -> usize {
        self.line_no_width.max(1)
    }

    pub(crate) fn collapsed_global(&self, global: usize) -> bool {
        self.collapsed[global]
    }

    pub(crate) fn decision_global(&self, global: usize) -> HunkDecision {
        self.decisions[global]
    }

    /// Set a decision from user input; returns `true` when it changed.
    pub(crate) fn decide_global(&mut self, global: usize, decision: HunkDecision) -> bool {
        let changed = self.decisions[global] != decision;
        self.decisions[global] = decision;
        changed
    }

    pub(crate) fn set_collapsed_global(&mut self, global: usize, collapsed: bool) {
        if self.collapsed[global] != collapsed {
            self.collapsed[global] = collapsed;
            self.rows.built_for = None;
        }
    }

    /// Global hunk index owning row `row`, if any.
    pub(crate) fn row_hunk(&self, row: usize) -> Option<u32> {
        match self.rows.rows.get(row)? {
            DiffRow::Hunk(g) | DiffRow::Line { hunk: g, .. } | DiffRow::Pair { hunk: g, .. } => {
                Some(*g)
            }
            DiffRow::File(_) | DiffRow::Binary(_) => None,
        }
    }

    /// Rebuild the row list when the layout or collapsed flags changed,
    /// keeping the cursor anchored on the same hunk / file header.
    pub(crate) fn ensure_rows(&mut self, split: bool) {
        if self.rows.built_for == Some(split) {
            return;
        }
        // Anchor: the hunk (or file) under the cursor in the previous rows.
        let anchor = match self.rows.rows.get(self.cursor) {
            Some(DiffRow::File(f) | DiffRow::Binary(f)) => Some((true, *f as usize)),
            Some(
                DiffRow::Hunk(g) | DiffRow::Line { hunk: g, .. } | DiffRow::Pair { hunk: g, .. },
            ) => Some((false, *g as usize)),
            None => None,
        };
        self.build_rows(split);
        match anchor {
            Some((true, f)) => self.cursor = self.rows.file_rows.get(f).copied().unwrap_or(0),
            Some((false, g)) => self.cursor = self.rows.hunk_rows.get(g).copied().unwrap_or(0),
            None => {}
        }
        if !self.cursor_ready {
            self.cursor_ready = true;
            if let Some(&first) = self.rows.hunk_rows.first() {
                self.cursor = first;
            }
        }
        self.cursor = self.cursor.min(self.rows.rows.len().saturating_sub(1));
    }

    fn build_rows(&mut self, split: bool) {
        let rows = &mut self.rows;
        rows.rows.clear();
        rows.hunk_rows.clear();
        rows.file_rows.clear();
        let mut global = 0usize;
        for (file_idx, file) in self.files.iter().enumerate() {
            rows.file_rows.push(rows.rows.len());
            rows.rows.push(DiffRow::File(file_idx as u32));
            if file.binary {
                rows.rows.push(DiffRow::Binary(file_idx as u32));
            }
            for hunk in &file.hunks {
                let g = global as u32;
                rows.hunk_rows.push(rows.rows.len());
                rows.rows.push(DiffRow::Hunk(g));
                if !self.collapsed[global] {
                    if split {
                        push_split_rows(&mut rows.rows, g, &hunk.lines);
                    } else {
                        rows.rows
                            .extend((0..hunk.lines.len()).map(|line| DiffRow::Line {
                                hunk: g,
                                line: line as u32,
                            }));
                    }
                }
                global += 1;
            }
        }
        rows.built_for = Some(split);
    }
}

/// Pair removed / added runs side by side; context lines occupy both sides.
fn push_split_rows(out: &mut Vec<DiffRow>, hunk: u32, lines: &[DiffLine]) {
    let mut i = 0;
    while i < lines.len() {
        if lines[i].kind == DiffLineKind::Context {
            out.push(DiffRow::Pair {
                hunk,
                old: Some(i as u32),
                new: Some(i as u32),
            });
            i += 1;
            continue;
        }
        let removed_start = i;
        while i < lines.len() && lines[i].kind == DiffLineKind::Removed {
            i += 1;
        }
        let added_start = i;
        while i < lines.len() && lines[i].kind == DiffLineKind::Added {
            i += 1;
        }
        let removed = added_start - removed_start;
        let added = i - added_start;
        for k in 0..removed.max(added) {
            out.push(DiffRow::Pair {
                hunk,
                old: (k < removed).then(|| (removed_start + k) as u32),
                new: (k < added).then(|| (added_start + k) as u32),
            });
        }
    }
}
