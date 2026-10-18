//! Code editor widget ([`Context::code_editor`], issue #466): a
//! [`TextareaState`] editor with a line-number gutter, syntax highlighting,
//! current-line and bracket-match highlights and horizontal scrolling.
//!
//! Editing reuses the `textarea` input loop (`Context::textarea_input`);
//! the editor adds its own keys (Tab / Shift+Tab indentation, Enter
//! auto-indent, PageUp / PageDown) through the loop's key hook. Rendering
//! is virtualized: only visible rows are emitted and only the highlight
//! chunks that intersect them are (re-)highlighted.

use super::textarea::{TextareaInput, TextareaInputHooks, visual_cursor};
use super::*;
use crate::event::KeyEvent;
use crate::widgets::{
    CodeEditorState, EditorLayout, cluster_cells, display_cols, grapheme_byte, is_printable_ascii,
};

/// Rows scrolled per mouse-wheel notch.
const WHEEL_ROWS: usize = 3;
/// Cells the gutter adds around the line number: `" │ "`.
const GUTTER_PAD: u32 = 3;
/// Lines scanned in each direction when looking for a matching bracket.
const BRACKET_SCAN_LINES: usize = 1_000;

impl Context {
    /// Begin rendering an editable, syntax-highlighted code field.
    ///
    /// Edits `state`'s [`TextareaState`] with the same keys, selection,
    /// clipboard and undo / redo as [`textarea`](Self::textarea), plus:
    /// `Tab` inserts indentation (or indents every line of a multi-line
    /// selection), `Shift+Tab` dedents, `Enter` keeps the current
    /// indentation (one more level after `{`, `(` or `[`), and `PageUp` /
    /// `PageDown` move by a page. Chain [`CodeEditor::line_numbers`],
    /// [`CodeEditor::h`], [`CodeEditor::wrap`] and friends, then
    /// [`CodeEditor::show`] for the [`Response`] (`changed` is set when the
    /// text changed this frame). The builder renders on drop too.
    ///
    /// Highlighting uses [`CodeEditorState::lang`] and needs the matching
    /// `syntax-*` feature; otherwise the text renders plain and editing is
    /// unchanged. Long lines scroll horizontally unless
    /// [`CodeEditor::wrap`] is on.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::CodeEditorState;
    ///
    /// let mut editor = CodeEditorState::new("fn main() {}\n").lang("rust");
    /// slt::run(|ui| {
    ///     let r = ui.code_editor(&mut editor).h(20).show();
    ///     if r.changed {
    ///         let _source = editor.value();
    ///     }
    /// })?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn code_editor<'a>(&'a mut self, state: &'a mut CodeEditorState) -> CodeEditor<'a> {
        CodeEditor {
            ctx: Some(self),
            state,
            opts: CodeEditorOpts::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CodeEditorOpts {
    width: Option<u32>,
    height: Option<u32>,
    line_numbers: bool,
    wrap: bool,
    current_line: bool,
    bracket_match: bool,
}

impl Default for CodeEditorOpts {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            line_numbers: true,
            wrap: false,
            current_line: true,
            bracket_match: true,
        }
    }
}

/// Code editor builder returned by [`Context::code_editor`]. Renders on
/// `Drop`; call [`Self::show`] to render and get the [`Response`].
pub struct CodeEditor<'a> {
    ctx: Option<&'a mut Context>,
    state: &'a mut CodeEditorState,
    opts: CodeEditorOpts,
}

impl CodeEditor<'_> {
    /// Fixed width in cells, gutter included. Defaults to the parent's
    /// available width.
    pub fn w(mut self, width: u32) -> Self {
        self.opts.width = Some(width);
        self
    }

    /// Viewport height in rows. Defaults to the parent's available height.
    pub fn h(mut self, height: u32) -> Self {
        self.opts.height = Some(height);
        self
    }

    /// Show or hide the line-number gutter (shown by default). The gutter
    /// is as wide as the line count's digit count plus 3 cells.
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.opts.line_numbers = show;
        self
    }

    /// Soft-wrap long lines at the text width instead of scrolling
    /// horizontally (off by default). Wrapping lays out the whole document
    /// each frame, so very long files are cheaper without it.
    pub fn wrap(mut self, on: bool) -> Self {
        self.opts.wrap = on;
        self
    }

    /// Tint the cursor line while focused (on by default).
    pub fn current_line(mut self, on: bool) -> Self {
        self.opts.current_line = on;
        self
    }

    /// Highlight the bracket matching the one at (or just before) the
    /// cursor while focused (on by default). Brackets inside strings and
    /// comments are counted too.
    pub fn bracket_match(mut self, on: bool) -> Self {
        self.opts.bracket_match = on;
        self
    }

    /// Render now and return the [`Response`].
    pub fn show(mut self) -> Response {
        match self.ctx.take() {
            Some(ctx) => render_code_editor(ctx, self.state, self.opts),
            None => Response::none(),
        }
    }
}

impl Drop for CodeEditor<'_> {
    fn drop(&mut self) {
        if let Some(ctx) = self.ctx.take() {
            let _ = render_code_editor(ctx, self.state, self.opts);
        }
    }
}

/// Gutter width in cells for a document of `lines` lines.
fn gutter_width(lines: usize, line_numbers: bool) -> u32 {
    if !line_numbers {
        return 0;
    }
    let digits = lines.max(1).ilog10() + 1;
    digits + GUTTER_PAD
}

/// Soft-wrap layout with tab stops measured from each segment's start.
fn editor_visual_lines(lines: &[String], width: u32, tab_width: usize) -> Vec<TextareaVLine> {
    let width = width.max(1) as usize;
    let mut out = Vec::with_capacity(lines.len());
    for (row, line) in lines.iter().enumerate() {
        let mut seg_start = 0;
        let mut seg_chars = 0;
        let mut seg_cells = 0;
        for (index, cluster) in line.graphemes(true).enumerate() {
            let mut cells = cluster_cells(cluster, seg_cells, tab_width);
            if seg_cells + cells > width && seg_chars > 0 {
                out.push(TextareaVLine {
                    logical_row: row,
                    char_start: seg_start,
                    char_count: seg_chars,
                });
                seg_start = index;
                seg_chars = 0;
                seg_cells = 0;
                cells = cluster_cells(cluster, 0, tab_width);
            }
            seg_chars += 1;
            seg_cells += cells;
        }
        out.push(TextareaVLine {
            logical_row: row,
            char_start: seg_start,
            char_count: seg_chars,
        });
    }
    out
}

/// Text position under a mouse cell, using the previous frame's layout.
fn editor_pos_at(
    state: &TextareaState,
    vlines: Option<&[TextareaVLine]>,
    rect: Rect,
    (x, y): (u32, u32),
    layout: EditorLayout,
    tab_width: usize,
) -> (usize, usize) {
    let total = vlines
        .map_or(state.lines.len(), <[TextareaVLine]>::len)
        .max(1);
    let vrow = if y < rect.y {
        state.scroll_offset.saturating_sub(1)
    } else {
        state.scroll_offset + (y - rect.y) as usize
    }
    .min(total - 1);
    let (row, start, count) = match vlines {
        Some(map) => (
            map[vrow].logical_row,
            map[vrow].char_start,
            map[vrow].char_count,
        ),
        None => (vrow, 0, grapheme_count(&state.lines[vrow])),
    };
    let text_x = rect.x.saturating_add(layout.gutter);
    let mut cell = x.saturating_sub(text_x) as usize;
    if vlines.is_none() {
        cell += layout.hscroll;
    }
    let mut used = 0;
    let mut col = 0;
    for cluster in state.lines[row].graphemes(true).skip(start).take(count) {
        let cells = cluster_cells(cluster, used, tab_width).max(1);
        if cell < used + cells {
            break;
        }
        used += cells;
        col += 1;
    }
    (row, start + col)
}

/// `PageUp` / `PageDown` (with `Shift` to extend the selection).
fn page_key(state: &mut TextareaState, key: &KeyEvent, page: usize) -> bool {
    let up = match key.code {
        KeyCode::PageUp => true,
        KeyCode::PageDown => false,
        _ => return false,
    };
    if has_global_shortcut_modifier(key.modifiers) {
        return false;
    }
    let before = (state.cursor_row, state.cursor_col);
    let last = state.lines.len().saturating_sub(1);
    state.cursor_row = if up {
        state.cursor_row.saturating_sub(page.max(1))
    } else {
        state.cursor_row.saturating_add(page.max(1)).min(last)
    };
    state.cursor_col = state
        .cursor_col
        .min(grapheme_count(&state.lines[state.cursor_row]));
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        state.anchor.get_or_insert(before);
    } else {
        state.anchor = None;
    }
    state.last_was_char_insert = false;
    true
}

fn bracket_pair(byte: u8) -> Option<(u8, u8, bool)> {
    match byte {
        b'(' => Some((b'(', b')', true)),
        b'[' => Some((b'[', b']', true)),
        b'{' => Some((b'{', b'}', true)),
        b')' => Some((b'(', b')', false)),
        b']' => Some((b'[', b']', false)),
        b'}' => Some((b'{', b'}', false)),
        _ => None,
    }
}

/// The bracket at the cursor (or just before it) and its match, as
/// `(row, grapheme column)` pairs.
fn matching_bracket(lines: &[String], (row, col): (usize, usize)) -> Option<[(usize, usize); 2]> {
    let line = lines.get(row)?;
    let at = grapheme_byte(line, col);
    let (byte, (open, close, forward)) = [Some(at), at.checked_sub(1)]
        .into_iter()
        .flatten()
        .find_map(|byte| Some((byte, bracket_pair(*line.as_bytes().get(byte)?)?)))?;
    let mut depth = 0usize;
    let found = if forward {
        let mut found = None;
        'scan: for (r, text) in lines.iter().enumerate().skip(row).take(BRACKET_SCAN_LINES) {
            let from = if r == row { byte + 1 } else { 0 };
            for (b, &ch) in text.as_bytes().iter().enumerate().skip(from) {
                if ch == open {
                    depth += 1;
                } else if ch == close {
                    if depth == 0 {
                        found = Some((r, b));
                        break 'scan;
                    }
                    depth -= 1;
                }
            }
        }
        found
    } else {
        let mut found = None;
        let first = row.saturating_sub(BRACKET_SCAN_LINES - 1);
        'scan: for r in (first..=row).rev() {
            let text = lines[r].as_bytes();
            let to = if r == row { byte } else { text.len() };
            for b in (0..to).rev() {
                if text[b] == close {
                    depth += 1;
                } else if text[b] == open {
                    if depth == 0 {
                        found = Some((r, b));
                        break 'scan;
                    }
                    depth -= 1;
                }
            }
        }
        found
    }?;
    let to_col = |(r, b): (usize, usize)| (r, grapheme_count(&lines[r][..b]));
    Some([to_col((row, byte)), to_col(found)])
}

/// Styles resolved once per frame.
struct EditorPalette {
    text: Style,
    gutter: Style,
    gutter_current: Style,
    separator: Style,
    selection: Style,
    bracket_bg: Color,
    current_bg: Option<Color>,
}

/// One visual row's segments, split around the cursor cell so the cursor
/// can carry the caret position.
#[derive(Default)]
struct RowSegments {
    before: Vec<(String, Style)>,
    cursor: Option<(String, Style)>,
    after: Vec<(String, Style)>,
    /// Text cells emitted so far (gutter excluded).
    cells: usize,
}

impl RowSegments {
    fn push(&mut self, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }
        let target = if self.cursor.is_some() {
            &mut self.after
        } else {
            &mut self.before
        };
        match target.last_mut() {
            Some((run, run_style)) if *run_style == style => run.push_str(text),
            _ => target.push((text.to_string(), style)),
        }
    }
}

/// What one visual row shows.
struct RowSpec<'a> {
    line: &'a str,
    spans: Option<&'a [(String, Style)]>,
    start: usize,
    count: usize,
    selection: Option<std::ops::Range<usize>>,
    cursor: Option<usize>,
    brackets: [Option<usize>; 2],
    current: bool,
}

#[derive(Clone, Copy)]
struct RowGeometry {
    hscroll: usize,
    text_w: usize,
    tab_width: usize,
}

fn build_row(spec: &RowSpec<'_>, geo: RowGeometry, palette: &EditorPalette, out: &mut RowSegments) {
    let lo = geo.hscroll;
    let hi = geo.hscroll + geo.text_w;
    let current_bg = palette.current_bg.filter(|_| spec.current);
    let style_at = |col: usize, mut style: Style| {
        if let Some(bg) = current_bg
            && style.bg.is_none()
        {
            style = style.bg(bg);
        }
        if spec.brackets.contains(&Some(col)) {
            style = style.bg(palette.bracket_bg).bold();
        }
        if spec
            .selection
            .as_ref()
            .is_some_and(|range| range.contains(&col))
        {
            style = palette.selection;
        }
        style
    };
    let spans = spec.spans.unwrap_or(&[]);
    let mut span_index = 0;
    let mut span_end = spans.first().map_or(0, |(text, _)| text.len());
    let seg_byte = grapheme_byte(spec.line, spec.start);
    let mut cells = 0usize;
    let mut buf = String::new();
    // Each byte of an ASCII line is its own cluster (a line never holds
    // `\r\n`), which skips grapheme segmentation on the common path.
    let segment = &spec.line[seg_byte..];
    let clusters: Box<dyn Iterator<Item = (usize, &str)>> = if segment.is_ascii() {
        Box::new((0..segment.len()).map(|i| (i, &segment[i..=i])))
    } else {
        Box::new(segment.grapheme_indices(true))
    };
    let mut seen = 0;
    let mut truncated = false;
    for (index, (byte, cluster)) in clusters.take(spec.count).enumerate() {
        seen = index + 1;
        let col = spec.start + index;
        let byte = seg_byte + byte;
        while span_index < spans.len() && byte >= span_end {
            span_index += 1;
            span_end += spans.get(span_index).map_or(0, |(text, _)| text.len());
        }
        let base = spans
            .get(span_index)
            .map_or(palette.text, |(_, style)| *style);
        let width = cluster_cells(cluster, cells, geo.tab_width);
        let (start, end) = (cells, cells + width);
        cells = end;
        if end <= lo || width == 0 {
            continue;
        }
        if start >= hi {
            truncated = true;
            break;
        }
        let style = style_at(col, base);
        let visible = end.min(hi) - start.max(lo);
        buf.clear();
        if is_printable_ascii(cluster) {
            buf.push_str(cluster);
        } else if cluster == "\t" || start < lo || end > hi {
            buf.extend(std::iter::repeat_n(' ', visible));
        } else {
            buf.push_str(&crate::cell::normalize_cell_symbol(cluster));
        }
        if spec.cursor == Some(col) && out.cursor.is_none() {
            if cluster == "\t" && visible > 1 {
                out.cursor = Some((" ".to_string(), style.reversed()));
                out.push(&buf[1..], style);
            } else {
                out.cursor = Some((buf.clone(), style.reversed()));
            }
        } else {
            out.push(&buf, style);
        }
        out.cells += visible;
    }
    if !truncated && spec.cursor == Some(spec.start + seen) && out.cursor.is_none() && cells < hi {
        let style = current_bg.map_or(palette.text, |bg| palette.text.bg(bg));
        out.cursor = Some((" ".to_string(), style.reversed()));
        out.cells += 1;
    }
    if let Some(bg) = current_bg {
        let pad = geo.text_w.saturating_sub(out.cells);
        if pad > 0 {
            out.push(&" ".repeat(pad), palette.text.bg(bg));
            out.cells += pad;
        }
    }
}

fn push_rich(ctx: &mut Context, segments: Vec<(String, Style)>) {
    ctx.commands.push(Command::RichText {
        segments,
        wrap: false,
        align: Align::Start,
        margin: Margin::default(),
        constraints: Constraints::default(),
    });
}

fn begin_box(ctx: &mut Context, direction: Direction, constraints: Constraints) {
    ctx.commands
        .push(Command::BeginContainer(Box::new(BeginContainerArgs {
            direction,
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints,
            title: None,
            grow: 0,
            group_name: None,
        })));
}

/// Emit one row. The cursor row puts the cursor cell in its own text
/// element carrying the caret offset (IME / caret geometry).
fn push_row(ctx: &mut Context, mut row: RowSegments) {
    let Some((cursor_text, cursor_style)) = row.cursor.take() else {
        if row.before.is_empty() {
            row.before.push((" ".to_string(), Style::new()));
        }
        push_rich(ctx, row.before);
        return;
    };
    begin_box(ctx, Direction::Row, Constraints::default());
    if !row.before.is_empty() {
        push_rich(ctx, row.before);
    }
    ctx.styled_with_cursor(cursor_text, cursor_style, Some(0));
    if !row.after.is_empty() {
        push_rich(ctx, row.after);
    }
    ctx.commands.push(Command::EndContainer);
    ctx.rollback.last_text_idx = None;
}

fn render_code_editor(
    ctx: &mut Context,
    state: &mut CodeEditorState,
    opts: CodeEditorOpts,
) -> Response {
    let cfg = state.indent_config();
    let tab_width = cfg.tab_width;
    {
        let ta = &mut state.textarea;
        if ta.lines.is_empty() {
            ta.lines.push(String::new());
        }
        ta.cursor_row = ta.cursor_row.min(ta.lines.len() - 1);
        ta.cursor_col = ta.cursor_col.min(grapheme_count(&ta.lines[ta.cursor_row]));
    }
    let kind = if cfg.tab_indents {
        crate::runtime::TextEntryKind::Code
    } else {
        crate::runtime::TextEntryKind::MultiLine
    };
    let focused = ctx.register_text_entry_focusable(kind, state.textarea.focus_key());
    let focus_slot = ctx.rollback.last_focusable_id;
    let (avail_w, avail_h) = ctx.available_content_size();
    let width = opts.width.unwrap_or(avail_w).max(1);
    let height = opts.height.unwrap_or(avail_h).max(1);
    let rows = height as usize;
    let text_width = |lines: usize| width.saturating_sub(gutter_width(lines, opts.line_numbers));
    let pre_text_w = text_width(state.textarea.lines.len()).max(1);
    state.textarea.wrap_width = opts.wrap.then_some(pre_text_w);
    // `begin_widget_interaction` below takes the next interaction slot; its
    // previous-frame rect locates the rows for mouse input.
    let box_rect = ctx
        .prev_hit_map
        .get(ctx.rollback.interaction_count)
        .copied()
        .filter(|rect| !rect.is_empty());

    let layout = state.layout;
    let pos_at = move |ta: &TextareaState, vlines: Option<&[TextareaVLine]>, rect, x, y| {
        editor_pos_at(ta, vlines, rect, (x, y), layout, tab_width)
    };
    let mut key_hook =
        |ta: &mut TextareaState, key: &KeyEvent| cfg.handle_key(ta, key) || page_key(ta, key, rows);
    let build = |lines: &[String]| editor_visual_lines(lines, pre_text_w, tab_width);
    let TextareaInput { pre_lines, vlines } = ctx.textarea_input(
        &mut state.textarea,
        focused,
        focus_slot,
        box_rect,
        TextareaInputHooks {
            key: &mut key_hook,
            visual_lines: &build,
            pos_at: &pos_at,
            edits_tab: cfg.tab_indents,
        },
    );

    let ta = &mut state.textarea;
    let line_count = ta.lines.len();
    let gutter = gutter_width(line_count, opts.line_numbers);
    let text_w = text_width(line_count).max(1);
    let vlines = opts.wrap.then(|| {
        ta.wrap_width = Some(text_w);
        match vlines {
            Some(map) if text_w == pre_text_w => map,
            _ => editor_visual_lines(&ta.lines, text_w, tab_width),
        }
    });
    let cursor = (ta.cursor_row, ta.cursor_col);
    let (cursor_vrow, _) = match &vlines {
        Some(map) => visual_cursor(ta, map),
        None => cursor,
    };
    let total = vlines.as_ref().map_or(line_count, Vec::len);

    // Follow the cursor when it moved or the text changed; otherwise keep
    // the viewport so the wheel can scroll away from the cursor.
    let follow =
        state.last_cursor != Some(cursor) || pre_lines.is_some() || layout.wrap != opts.wrap;
    let (_interaction_id, mut response) = ctx.begin_widget_interaction(focused);
    if follow {
        if cursor_vrow < ta.scroll_offset {
            ta.scroll_offset = cursor_vrow;
        } else if cursor_vrow >= ta.scroll_offset + rows {
            ta.scroll_offset = cursor_vrow + 1 - rows;
        }
    }
    let max_scroll = total.saturating_sub(rows);
    if response.scroll_delta != 0 {
        let delta = (response.scroll_delta.unsigned_abs() as usize).saturating_mul(WHEEL_ROWS);
        ta.scroll_offset = if response.scroll_delta > 0 {
            ta.scroll_offset.saturating_sub(delta)
        } else {
            ta.scroll_offset.saturating_add(delta)
        };
    }
    ta.scroll_offset = ta.scroll_offset.min(max_scroll);
    let mut hscroll = if opts.wrap { 0 } else { state.hscroll };
    if follow && !opts.wrap {
        let col = display_cols(&ta.lines[cursor.0], cursor.1, tab_width);
        let text_w = text_w as usize;
        if col < hscroll {
            hscroll = col;
        } else if col >= hscroll + text_w {
            hscroll = col + 1 - text_w;
        }
    }
    state.hscroll = hscroll;
    state.last_cursor = Some(cursor);
    state.layout = EditorLayout {
        gutter,
        hscroll,
        wrap: opts.wrap,
    };
    ta.caret_glyph = None;

    let scroll = ta.scroll_offset;
    let end = (scroll + rows).min(total);
    let logical = match &vlines {
        Some(map) if end > scroll => map[scroll].logical_row..map[end - 1].logical_row + 1,
        Some(_) => 0..0,
        None => scroll..end,
    };
    let theme = ctx.theme;
    state.prepare_highlight(logical, &theme);

    let ta = &state.textarea;
    let selection = ta.selection();
    let brackets = if focused && opts.bracket_match {
        matching_bracket(&ta.lines, cursor)
    } else {
        None
    };
    let palette = EditorPalette {
        text: Style::new().fg(theme.text),
        gutter: Style::new().fg(theme.text_dim),
        gutter_current: Style::new().fg(theme.text),
        separator: Style::new().fg(theme.border),
        selection: theme.text_selection_style(),
        bracket_bg: theme.surface_hover,
        current_bg: (focused && opts.current_line).then_some(theme.surface),
    };
    let geo = RowGeometry {
        hscroll,
        text_w: text_w as usize,
        tab_width,
    };
    let digits = gutter.saturating_sub(GUTTER_PAD) as usize;

    begin_box(
        ctx,
        Direction::Column,
        Constraints::default().w(width).h(height),
    );
    for vrow in scroll..end {
        let (row, start, count, first_segment) = match &vlines {
            Some(map) => {
                let line = &map[vrow];
                let first = vrow == 0 || map[vrow - 1].logical_row != line.logical_row;
                (line.logical_row, line.char_start, line.char_count, first)
            }
            None => (vrow, 0, usize::MAX, true),
        };
        let line = ta.lines[row].as_str();
        let mut out = RowSegments::default();
        if opts.line_numbers {
            let current = focused && row == cursor.0;
            let number = if first_segment {
                format!("{:>digits$}", row + 1)
            } else {
                " ".repeat(digits)
            };
            out.push(
                &number,
                if current {
                    palette.gutter_current
                } else {
                    palette.gutter
                },
            );
            out.push(" │ ", palette.separator);
        }
        let on_cursor = focused && vrow == cursor_vrow;
        let row_selection = selection.as_ref().and_then(|range| {
            let ((r1, c1), (r2, c2)) = (range.start, range.end);
            (r1..=r2).contains(&row).then(|| {
                let from = if row == r1 { c1 } else { 0 };
                let to = if row == r2 { c2 } else { usize::MAX };
                from..to
            })
        });
        let bracket_cols = brackets.map_or([None, None], |pair| {
            pair.map(|(r, c)| (r == row).then_some(c))
        });
        let spec = RowSpec {
            line,
            spans: state.highlight.line_spans(row),
            start,
            count,
            selection: row_selection,
            // `visual_cursor` already picked the row (upstream affinity puts
            // a cursor at a wrap point at the end of the earlier segment).
            cursor: on_cursor.then_some(cursor.1),
            brackets: bracket_cols,
            current: focused && row == cursor.0,
        };
        build_row(&spec, geo, &palette, &mut out);
        push_row(ctx, out);
    }
    ctx.commands.push(Command::EndContainer);
    ctx.rollback.last_text_idx = None;

    response.changed = pre_lines.is_some_and(|before| state.textarea.lines != before);
    response
}

#[cfg(test)]
mod tests;
