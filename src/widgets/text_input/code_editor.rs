//! Code editor state (issue #466): a [`TextareaState`] plus a language,
//! indentation settings and a chunked syntax-highlight cache.
//!
//! Rendered by `Context::code_editor` in
//! `context/widgets/text_input/code_editor.rs`.

use super::*;
use crate::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::style::{Color, SyntaxPalette, Theme};

/// Default indentation width (spaces per level and tab-stop spacing).
const DEFAULT_TAB_WIDTH: usize = 4;

/// Longest chunk the highlighter re-runs on one edit. Chunks normally end at
/// top-level items (see [`chunk_ranges`]); a run of indented code longer
/// than this is split so a single keystroke never re-highlights more.
pub(crate) const MAX_CHUNK_LINES: usize = 128;

/// Cached chunks kept beyond the ones the current frame uses.
const MAX_CACHED_CHUNKS: usize = 64;

/// State for [`Context::code_editor`](crate::Context::code_editor): an
/// editable, syntax-highlighted code field.
///
/// Wraps a [`TextareaState`] for the text, cursor, selection and undo /
/// redo history (reach it with [`textarea`](Self::textarea) /
/// [`textarea_mut`](Self::textarea_mut)) and adds:
///
/// - a highlight language ([`lang`](Self::lang)); highlighting needs the
///   matching `syntax-*` feature and falls back to plain text otherwise,
///   with identical editing behaviour;
/// - indentation: `Tab` inserts spaces up to the next tab stop
///   ([`tab_width`](Self::tab_width), default 4) or a `\t` with
///   [`hard_tabs`](Self::hard_tabs); with a multi-line selection `Tab`
///   indents and `Shift+Tab` dedents every selected line; `Enter` copies the
///   current line's leading whitespace and adds one level after an opening
///   bracket ([`auto_indent`](Self::auto_indent)).
///
/// # Highlighting cost
///
/// The document is highlighted in chunks that end before an unindented line
/// following a blank line (top-level items in most languages), capped at
/// 128 lines. Only chunks that intersect the viewport are highlighted, and
/// each chunk's result is cached by its exact text, so an edit re-highlights
/// the edited chunk only. A construct that spans a chunk boundary (a block
/// comment or string containing a blank line followed by an unindented
/// line) is highlighted as if the chunk started fresh.
///
/// # Example
///
/// ```no_run
/// use slt::CodeEditorState;
///
/// let mut editor = CodeEditorState::new("fn main() {\n    println!(\"hi\");\n}")
///     .lang("rust")
///     .tab_width(4);
/// slt::run(|ui: &mut slt::Context| {
///     let r = ui.code_editor(&mut editor).line_numbers(true).h(20).show();
///     if r.changed {
///         let _source = editor.value();
///     }
/// })?;
/// # Ok::<_, std::io::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct CodeEditorState {
    pub(crate) textarea: TextareaState,
    lang: Option<String>,
    tab_width: usize,
    hard_tabs: bool,
    tab_indents: bool,
    auto_indent: bool,
    /// Horizontal scroll in cells (no-wrap mode).
    pub(crate) hscroll: usize,
    /// Cursor at the end of the previous frame; the viewport follows the
    /// cursor only when it moved, so the mouse wheel can scroll freely.
    pub(crate) last_cursor: Option<(usize, usize)>,
    /// Previous frame's text geometry, used to map mouse cells.
    pub(crate) layout: EditorLayout,
    pub(crate) highlight: HighlightCache,
}

/// Issue #466: text geometry of one rendered frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct EditorLayout {
    /// Gutter width in cells (0 without line numbers).
    pub(crate) gutter: u32,
    /// Horizontal scroll in cells.
    pub(crate) hscroll: usize,
    /// Whether the frame soft-wrapped.
    pub(crate) wrap: bool,
}

impl CodeEditorState {
    /// Create an editor holding `source` with the cursor at the start.
    pub fn new(source: impl Into<String>) -> Self {
        let mut textarea = TextareaState::new();
        textarea.set_value(source);
        Self {
            textarea,
            lang: None,
            tab_width: DEFAULT_TAB_WIDTH,
            hard_tabs: false,
            tab_indents: true,
            auto_indent: true,
            hscroll: 0,
            last_cursor: None,
            layout: EditorLayout::default(),
            highlight: HighlightCache::default(),
        }
    }

    /// Builder: highlight language, such as `"rust"`, `"py"` or `"json"`
    /// (the names [`code_block`](crate::Context::code_block) accepts).
    pub fn lang(mut self, lang: impl Into<String>) -> Self {
        self.set_lang(Some(lang.into()));
        self
    }

    /// Change the highlight language (`None` for plain text).
    pub fn set_lang(&mut self, lang: Option<String>) {
        self.lang = lang;
        self.highlight.clear();
    }

    /// The highlight language, if one is set.
    pub fn language(&self) -> Option<&str> {
        self.lang.as_deref()
    }

    /// Builder: spaces per indentation level and tab-stop spacing used to
    /// display `\t` (default 4; `0` is treated as 1).
    pub fn tab_width(mut self, width: usize) -> Self {
        self.tab_width = width.max(1);
        self
    }

    /// The indentation width.
    pub fn indent_width(&self) -> usize {
        self.tab_width
    }

    /// Builder: `Tab` inserts a `\t` character instead of spaces (default
    /// `false`).
    pub fn hard_tabs(mut self, on: bool) -> Self {
        self.hard_tabs = on;
        self
    }

    /// Builder: whether `Tab` / `Shift+Tab` indent and dedent while the
    /// editor is focused (default `true`). With `false` they move focus like
    /// in any other widget.
    pub fn tab_indents(mut self, on: bool) -> Self {
        self.tab_indents = on;
        self
    }

    /// Whether `Tab` / `Shift+Tab` indent (see [`tab_indents`](Self::tab_indents)).
    pub fn tabs_indent(&self) -> bool {
        self.tab_indents
    }

    /// Builder: whether `Enter` copies the current line's indentation
    /// (default `true`).
    pub fn auto_indent(mut self, on: bool) -> Self {
        self.auto_indent = on;
        self
    }

    /// The document with lines joined by `\n`.
    pub fn value(&self) -> String {
        self.textarea.value()
    }

    /// Replace the document, moving the cursor to the start and clearing the
    /// undo history (like [`TextareaState::set_value`]).
    pub fn set_value(&mut self, source: impl Into<String>) {
        self.textarea.set_value(source);
        self.hscroll = 0;
        self.last_cursor = None;
    }

    /// The document lines.
    pub fn lines(&self) -> &[String] {
        &self.textarea.lines
    }

    /// Number of lines (at least 1).
    pub fn line_count(&self) -> usize {
        self.textarea.lines.len().max(1)
    }

    /// Cursor position as `(row, column)` in grapheme clusters.
    pub fn cursor(&self) -> (usize, usize) {
        (self.textarea.cursor_row, self.textarea.cursor_col)
    }

    /// Move the cursor to `(row, column)` (clamped) and drop the selection.
    pub fn set_cursor(&mut self, row: usize, col: usize) {
        let row = row.min(self.textarea.lines.len().saturating_sub(1));
        let len = self
            .textarea
            .lines
            .get(row)
            .map_or(0, |line| line.graphemes(true).count());
        self.textarea.cursor_row = row;
        self.textarea.cursor_col = col.min(len);
        self.textarea.anchor = None;
        self.textarea.navigation = None;
        self.textarea.last_was_char_insert = false;
    }

    /// The editing core (text, cursor, selection, history).
    pub fn textarea(&self) -> &TextareaState {
        &self.textarea
    }

    /// Mutable access to the editing core. Edits made through it are picked
    /// up by the highlighter on the next frame.
    pub fn textarea_mut(&mut self) -> &mut TextareaState {
        &mut self.textarea
    }

    pub(crate) fn indent_config(&self) -> IndentConfig {
        IndentConfig {
            tab_width: self.tab_width,
            hard_tabs: self.hard_tabs,
            tab_indents: self.tab_indents,
            auto_indent: self.auto_indent,
        }
    }

    /// Highlight the chunks covering logical rows `rows` (see
    /// [`HighlightCache::prepare`]).
    pub(crate) fn prepare_highlight(&mut self, rows: std::ops::Range<usize>, theme: &Theme) {
        let lang = self.lang.as_deref();
        self.highlight
            .prepare(&self.textarea.lines, rows, lang, theme);
    }
}

impl Default for CodeEditorState {
    fn default() -> Self {
        Self::new("")
    }
}

// ── indentation ─────────────────────────────────────────────────────────

/// Issue #466: editing settings the editor's key hook needs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct IndentConfig {
    pub(crate) tab_width: usize,
    pub(crate) hard_tabs: bool,
    pub(crate) tab_indents: bool,
    pub(crate) auto_indent: bool,
}

/// Display width of a tab at cell column `col` (relative to the line or
/// visual-segment start).
pub(crate) fn tab_cells(col: usize, tab_width: usize) -> usize {
    let tab_width = tab_width.max(1);
    tab_width - col % tab_width
}

/// Display cells of the first `cols` grapheme clusters of `line`, with tabs
/// expanded to tab stops.
pub(crate) fn display_cols(line: &str, cols: usize, tab_width: usize) -> usize {
    let mut cells = 0;
    for cluster in line.graphemes(true).take(cols) {
        cells += cluster_cells(cluster, cells, tab_width);
    }
    cells
}

/// Cells one grapheme cluster occupies at cell column `col`.
pub(crate) fn cluster_cells(cluster: &str, col: usize, tab_width: usize) -> usize {
    if is_printable_ascii(cluster) {
        1
    } else if cluster == "\t" {
        tab_cells(col, tab_width)
    } else {
        crate::width::str_width(&crate::cell::normalize_cell_symbol(cluster))
    }
}

/// A single printable ASCII byte: one cell, rendered as is (the common
/// case, which skips cell-symbol normalization and width lookups).
pub(crate) fn is_printable_ascii(cluster: &str) -> bool {
    matches!(cluster.as_bytes(), [byte] if (0x20..0x7f).contains(byte))
}

/// Byte length of `line`'s leading spaces and tabs.
fn leading_ws(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

fn closer_for(open: char) -> Option<char> {
    match open {
        '{' => Some('}'),
        '(' => Some(')'),
        '[' => Some(']'),
        _ => None,
    }
}

impl IndentConfig {
    /// One indentation level.
    fn unit(&self) -> String {
        if self.hard_tabs {
            "\t".to_string()
        } else {
            " ".repeat(self.tab_width)
        }
    }

    /// Handle the editor's own keys. Returns `true` when `key` was handled.
    pub(crate) fn handle_key(&self, state: &mut TextareaState, key: &KeyEvent) -> bool {
        let mods = key.modifiers;
        if mods.contains(KeyModifiers::CONTROL)
            || mods.contains(KeyModifiers::ALT)
            || mods.contains(KeyModifiers::SUPER)
        {
            return false;
        }
        let shift = mods.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Tab if self.tab_indents && !shift => {
                self.indent(state);
                true
            }
            KeyCode::BackTab | KeyCode::Tab if self.tab_indents => {
                self.dedent(state);
                true
            }
            KeyCode::Enter if self.auto_indent => {
                self.newline(state);
                true
            }
            _ => false,
        }
    }

    /// Rows a line-wise operation touches: the selected rows (a selection
    /// ending at column 0 leaves that last row out) or the cursor row.
    fn target_rows(state: &TextareaState) -> std::ops::RangeInclusive<usize> {
        match state.selection() {
            Some(range) => {
                let (r1, r2) = (range.start.0, range.end.0);
                let last = if r2 > r1 && range.end.1 == 0 {
                    r2 - 1
                } else {
                    r2
                };
                r1..=last
            }
            None => state.cursor_row..=state.cursor_row,
        }
    }

    /// `Tab`: indent the selected lines (multi-line selection) or insert one
    /// level at the cursor.
    pub(crate) fn indent(&self, state: &mut TextareaState) {
        let multi_line = state
            .selection()
            .is_some_and(|range| range.start.0 != range.end.0);
        if !multi_line {
            let text = if self.hard_tabs {
                "\t".to_string()
            } else {
                let line = &state.lines[state.cursor_row];
                let col = match state.selection() {
                    Some(range) => range.start.1,
                    None => state.cursor_col,
                };
                " ".repeat(tab_cells(
                    display_cols(line, col, self.tab_width),
                    self.tab_width,
                ))
            };
            state.replace_selection_or_insert(&text, false);
            state.last_was_char_insert = false;
            return;
        }
        let rows = Self::target_rows(state);
        let unit = self.unit();
        let added = unit.len() * rows.clone().count();
        if state
            .max_length
            .is_some_and(|max| state.grapheme_len().saturating_add(added) > max)
        {
            return;
        }
        state.push_history();
        for row in rows.clone() {
            state.lines[row].insert_str(0, &unit);
        }
        let shift = |(row, col): (usize, usize)| {
            if rows.contains(&row) && col > 0 {
                (row, col + unit.len())
            } else {
                (row, col)
            }
        };
        (state.cursor_row, state.cursor_col) = shift((state.cursor_row, state.cursor_col));
        state.anchor = state.anchor.map(shift);
        state.last_was_char_insert = false;
    }

    /// `Shift+Tab`: remove up to one level of leading whitespace from the
    /// selected lines or the cursor line.
    pub(crate) fn dedent(&self, state: &mut TextareaState) {
        let rows = Self::target_rows(state);
        let removals: Vec<(usize, usize)> = rows
            .map(|row| {
                let line = &state.lines[row];
                let remove = if line.starts_with('\t') {
                    1
                } else {
                    line.bytes()
                        .take(self.tab_width)
                        .take_while(|&byte| byte == b' ')
                        .count()
                };
                (row, remove)
            })
            .filter(|&(_, remove)| remove > 0)
            .collect();
        state.last_was_char_insert = false;
        if removals.is_empty() {
            return;
        }
        state.push_history();
        for &(row, remove) in &removals {
            state.lines[row].replace_range(..remove, "");
        }
        let shift = |(row, col): (usize, usize)| match removals.iter().find(|(r, _)| *r == row) {
            Some(&(_, remove)) => (row, col.saturating_sub(remove)),
            None => (row, col),
        };
        (state.cursor_row, state.cursor_col) = shift((state.cursor_row, state.cursor_col));
        state.anchor = state.anchor.map(shift);
    }

    /// `Enter`: break the line, copying the leading whitespace of the line
    /// being split. After an opening bracket one level is added, and when
    /// the matching closer follows the cursor it moves to its own line.
    pub(crate) fn newline(&self, state: &mut TextareaState) {
        let (start, end) = match state.selection() {
            Some(range) => (range.start, range.end),
            None => {
                let cursor = (state.cursor_row, state.cursor_col);
                (cursor, cursor)
            }
        };
        let line = &state.lines[start.0];
        let split = grapheme_byte(line, start.1);
        let indent = line[..leading_ws(line).min(split)].to_string();
        let opener = line[..split].trim_end().chars().last().and_then(closer_for);
        let end_line = &state.lines[end.0];
        let after = end_line[grapheme_byte(end_line, end.1)..].trim_start();
        let split_closer = opener.is_some_and(|closer| after.starts_with(closer));
        let mut text = String::with_capacity(indent.len() * 2 + self.tab_width + 2);
        text.push('\n');
        text.push_str(&indent);
        if opener.is_none() {
            state.replace_selection_or_insert(&text, false);
            state.last_was_char_insert = false;
            return;
        }
        let unit = self.unit();
        text.push_str(&unit);
        if split_closer {
            text.push('\n');
            text.push_str(&indent);
        }
        if state.replace_selection_or_insert(&text, false) && split_closer {
            state.cursor_row -= 1;
            state.cursor_col = indent.len() + unit.len();
        }
        state.last_was_char_insert = false;
    }
}

// ── chunked highlighting ────────────────────────────────────────────────

fn is_blank(line: &str) -> bool {
    line.bytes()
        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r'))
}

/// Whether a chunk starts at `row`: the first line, or an unindented line
/// that follows a blank line and does not start with a closing bracket.
fn natural_start(lines: &[String], row: usize) -> bool {
    if row == 0 {
        return true;
    }
    let Some(&first) = lines[row].as_bytes().first() else {
        return false;
    };
    !first.is_ascii_whitespace()
        && !matches!(first, b'}' | b')' | b']')
        && is_blank(&lines[row - 1])
}

/// End (exclusive) of the chunk starting at `start`.
fn chunk_end(lines: &[String], start: usize) -> usize {
    let len = lines.len();
    let cap = start + MAX_CHUNK_LINES;
    if let Some(row) = (start + 1..cap.min(len)).find(|&row| natural_start(lines, row)) {
        return row;
    }
    if cap >= len {
        return len;
    }
    // No top-level boundary within the cap: split at the last blank-line
    // break in the second half of the chunk, or at the cap itself.
    (start + MAX_CHUNK_LINES / 2 + 1..=cap)
        .rev()
        .find(|&row| is_blank(&lines[row - 1]) && !is_blank(&lines[row]))
        .unwrap_or(cap)
}

/// Issue #466: the highlight chunks covering rows `rows` of `lines`.
///
/// Boundaries depend only on the text, so the chunking of a row does not
/// change with the viewport, and an edit only changes the chunks near it.
pub(crate) fn chunk_ranges(
    lines: &[String],
    rows: std::ops::Range<usize>,
) -> Vec<std::ops::Range<usize>> {
    let len = lines.len();
    let end = rows.end.min(len);
    if rows.start >= end {
        return Vec::new();
    }
    let mut start = rows.start;
    while !natural_start(lines, start) {
        start -= 1;
    }
    let mut out = Vec::new();
    while start < end {
        let chunk_end = chunk_end(lines, start);
        if chunk_end > rows.start {
            out.push(start..chunk_end);
        }
        start = chunk_end;
    }
    out
}

fn hash_lines(lines: &[String]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    lines.hash(&mut hasher);
    hasher.finish()
}

/// Spans of one highlighted line.
pub(crate) type LineSpans = Vec<(String, Style)>;

#[derive(Debug, Clone)]
struct CachedChunk {
    hash: u64,
    lines: Vec<String>,
    spans: Vec<LineSpans>,
    /// Frame stamp of the last use (for eviction).
    used: u64,
}

/// Issue #466: highlight work counters (cache misses re-run tree-sitter).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct HighlightStats {
    /// Chunks highlighted (cache misses).
    pub(crate) runs: u64,
    /// Chunks served from the cache.
    pub(crate) hits: u64,
    /// Lines passed to the highlighter.
    pub(crate) lines: u64,
}

type HighlightKey = (String, Color, Color, SyntaxPalette);

/// Issue #466: per-chunk highlight cache keyed by the chunk's exact text.
#[derive(Debug, Clone, Default)]
pub(crate) struct HighlightCache {
    key: Option<HighlightKey>,
    chunks: Vec<CachedChunk>,
    /// Chunks of the last [`prepare`](Self::prepare): `(rows, chunk index)`.
    window: Vec<(std::ops::Range<usize>, usize)>,
    clock: u64,
    pub(crate) stats: HighlightStats,
}

impl HighlightCache {
    pub(crate) fn clear(&mut self) {
        self.key = None;
        self.chunks.clear();
        self.window.clear();
    }

    /// Highlight (or fetch from the cache) every chunk covering `rows`.
    /// Without a supported language the window is left empty and
    /// [`line_spans`](Self::line_spans) returns `None` (plain text).
    pub(crate) fn prepare(
        &mut self,
        lines: &[String],
        rows: std::ops::Range<usize>,
        lang: Option<&str>,
        theme: &Theme,
    ) {
        self.window.clear();
        let Some(lang) = lang.filter(|lang| crate::syntax::is_language_supported(lang)) else {
            return;
        };
        let key_matches = self.key.as_ref().is_some_and(|(l, text, dim, syntax)| {
            l == lang && *text == theme.text && *dim == theme.text_dim && *syntax == theme.syntax
        });
        if !key_matches {
            self.chunks.clear();
            self.key = Some((lang.to_string(), theme.text, theme.text_dim, theme.syntax));
        }
        self.clock += 1;
        for range in chunk_ranges(lines, rows) {
            let source = &lines[range.clone()];
            let hash = hash_lines(source);
            let index = match self
                .chunks
                .iter()
                .position(|chunk| chunk.hash == hash && chunk.lines == source)
            {
                Some(index) => {
                    self.stats.hits += 1;
                    index
                }
                None => {
                    let code = source.join("\n");
                    let mut spans =
                        crate::syntax::highlight_lines(&code, lang, theme).unwrap_or_default();
                    spans.resize_with(source.len(), Vec::new);
                    self.stats.runs += 1;
                    self.stats.lines += source.len() as u64;
                    self.chunks.push(CachedChunk {
                        hash,
                        lines: source.to_vec(),
                        spans,
                        used: 0,
                    });
                    self.chunks.len() - 1
                }
            };
            self.chunks[index].used = self.clock;
            self.window.push((range, index));
        }
        self.evict();
    }

    /// Drop the least recently used chunks beyond the cap, never one the
    /// current window uses.
    fn evict(&mut self) {
        let in_use = self.window.len();
        if self.chunks.len() <= MAX_CACHED_CHUNKS + in_use {
            return;
        }
        let clock = self.clock;
        let mut stale: Vec<(u64, usize)> = self
            .chunks
            .iter()
            .enumerate()
            .filter(|(_, chunk)| chunk.used != clock)
            .map(|(index, chunk)| (chunk.used, index))
            .collect();
        stale.sort_unstable();
        let excess = self.chunks.len() - (MAX_CACHED_CHUNKS + in_use);
        let mut keep = vec![true; self.chunks.len()];
        for (_, index) in stale.into_iter().take(excess) {
            keep[index] = false;
        }
        let mut remap = vec![usize::MAX; self.chunks.len()];
        let mut kept = Vec::with_capacity(self.chunks.len() - excess);
        for (index, chunk) in self.chunks.drain(..).enumerate() {
            if keep[index] {
                remap[index] = kept.len();
                kept.push(chunk);
            }
        }
        self.chunks = kept;
        for (_, chunk) in &mut self.window {
            *chunk = remap[*chunk];
        }
    }

    /// Highlighted spans of logical `row`, if the last
    /// [`prepare`](Self::prepare) covered it with a supported language.
    pub(crate) fn line_spans(&self, row: usize) -> Option<&[(String, Style)]> {
        let (range, index) = self.window.iter().find(|(range, _)| range.contains(&row))?;
        self.chunks[*index]
            .spans
            .get(row - range.start)
            .map(Vec::as_slice)
    }

    /// Number of cached chunks (tests).
    #[cfg(all(test, feature = "syntax-rust"))]
    pub(crate) fn cached_chunks(&self) -> usize {
        self.chunks.len()
    }
}

#[cfg(test)]
mod tests;
