use super::*;
use crate::event::{KeyEvent, MouseEvent};
use crate::widgets::{TextareaNavigation, grapheme_cursor_after, word_range_at};

/// Issue #465: text position under a mouse cell, using the previous frame's
/// layout (scroll offset, soft wrap and the caret glyph that shifted the
/// cells after it).
fn textarea_pos_at(
    state: &TextareaState,
    vlines: Option<&[TextareaVLine]>,
    rect: Rect,
    x: u32,
    y: u32,
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
        Some(map) => {
            let line = &map[vrow];
            (line.logical_row, line.char_start, line.char_count)
        }
        None => (vrow, 0, grapheme_count(&state.lines[vrow])),
    };
    let mut cell = x.saturating_sub(rect.x) as usize;
    if let Some((glyph_row, glyph_col)) = state.caret_glyph
        && glyph_row == vrow
    {
        let glyph_cell: usize = state.lines[row]
            .graphemes(true)
            .skip(start)
            .take(glyph_col)
            .map(|cluster| cluster_width(cluster).max(1) as usize)
            .sum();
        if cell == glyph_cell {
            return (row, start + glyph_col);
        }
        if cell > glyph_cell {
            cell -= 1;
        }
    }
    let mut used = 0;
    let mut col = 0;
    for cluster in state.lines[row].graphemes(true).skip(start).take(count) {
        let width = cluster_width(cluster).max(1) as usize;
        if cell < used + width {
            break;
        }
        used += width;
        col += 1;
    }
    (row, start + col)
}

/// Issue #466: maps a mouse cell to a text position (see [`textarea_pos_at`]).
pub(super) type TextareaPosAt<'a> =
    &'a dyn Fn(&TextareaState, Option<&[TextareaVLine]>, Rect, u32, u32) -> (usize, usize);

/// Issue #466: customization points of [`Context::textarea_input`] used by
/// the code editor. `textarea` passes no-op / default hooks.
pub(super) struct TextareaInputHooks<'a> {
    /// Runs first for each unconsumed key press; `true` means the key was
    /// handled (and is consumed).
    pub(super) key: &'a mut dyn FnMut(&mut TextareaState, &KeyEvent) -> bool,
    /// Soft-wrap layout used for Up/Down and mouse mapping.
    pub(super) visual_lines: &'a dyn Fn(&[String]) -> Vec<TextareaVLine>,
    /// Mouse cell → text position.
    pub(super) pos_at: TextareaPosAt<'a>,
    /// Whether Tab / Shift+Tab edit (for the `changed` baseline).
    pub(super) edits_tab: bool,
}

/// Issue #466: what [`Context::textarea_input`] hands back to the renderer.
pub(super) struct TextareaInput {
    /// Lines before the batch, when the batch contained edits.
    pub(super) pre_lines: Option<Vec<String>>,
    /// Soft-wrap layout built while handling input, if any.
    pub(super) vlines: Option<Vec<TextareaVLine>>,
}

/// Issue #465: mouse selection for a focused `textarea`. Returns `true` when
/// the event belongs to the text area.
fn textarea_pointer(
    state: &mut TextareaState,
    mouse: &MouseEvent,
    rect: Rect,
    vlines: Option<&[TextareaVLine]>,
    pos_at: TextareaPosAt<'_>,
    now: crate::clock::Instant,
) -> bool {
    match mouse.kind {
        MouseKind::Down(MouseButton::Left) => {
            if !rect.contains(mouse.x, mouse.y) {
                state.pointer.dragging = false;
                return false;
            }
            let pos = pos_at(state, vlines, rect, mouse.x, mouse.y);
            match state.pointer.register_click(now, (mouse.x, mouse.y)) {
                1 => {
                    let cursor = (state.cursor_row, state.cursor_col);
                    let anchor = if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                        state.anchor.unwrap_or(cursor)
                    } else {
                        pos
                    };
                    state.set_selection(anchor, pos);
                    state.pointer.dragging = true;
                }
                2 => {
                    let word = word_range_at(&state.lines[pos.0], pos.1);
                    state.set_selection((pos.0, word.start), (pos.0, word.end));
                    state.pointer.dragging = false;
                }
                _ => {
                    let len = grapheme_count(&state.lines[pos.0]);
                    state.set_selection((pos.0, 0), (pos.0, len));
                    state.pointer.dragging = false;
                }
            }
            state.last_was_char_insert = false;
            true
        }
        MouseKind::Drag(MouseButton::Left) if state.pointer.dragging => {
            let pos = pos_at(state, vlines, rect, mouse.x, mouse.y);
            let anchor = state.anchor.unwrap_or((state.cursor_row, state.cursor_col));
            state.set_selection(anchor, pos);
            true
        }
        MouseKind::Up(MouseButton::Left) if state.pointer.dragging => {
            state.pointer.dragging = false;
            true
        }
        _ => false,
    }
}

/// Issue #465: the selected grapheme range of one visual segment, relative
/// to the segment start.
fn segment_selection(
    selection: Option<&std::ops::Range<(usize, usize)>>,
    row: usize,
    start: usize,
    count: usize,
) -> Option<std::ops::Range<usize>> {
    let range = selection?;
    let ((r1, c1), (r2, c2)) = (range.start, range.end);
    if row < r1 || row > r2 {
        return None;
    }
    let from = if row == r1 { c1 } else { 0 };
    let to = if row == r2 { c2 } else { usize::MAX };
    let lo = from.max(start);
    let hi = to.min(start + count);
    (lo < hi).then(|| lo - start..hi - start)
}

fn current_navigation(state: &TextareaState) -> Option<TextareaNavigation> {
    state.navigation.filter(|nav| {
        nav.row == state.cursor_row
            && nav.col == state.cursor_col
            && nav.wrap_width == state.wrap_width
    })
}

pub(super) fn visual_cursor(state: &TextareaState, vlines: &[TextareaVLine]) -> (usize, usize) {
    if current_navigation(state).is_some_and(|nav| nav.upstream)
        && let Some((index, line)) = vlines.iter().enumerate().find(|(index, line)| {
            line.logical_row == state.cursor_row
                && line.char_start + line.char_count == state.cursor_col
                && vlines
                    .get(index + 1)
                    .is_some_and(|next| next.logical_row == line.logical_row)
        })
    {
        return (index, line.char_count);
    }
    textarea_logical_to_visual(vlines, state.cursor_row, state.cursor_col)
}

fn navigate_visual(state: &mut TextareaState, vlines: &[TextareaVLine], down: bool) {
    let (row, col) = visual_cursor(state, vlines);
    let source = &vlines[row];
    let desired_cells = current_navigation(state).map_or_else(
        || {
            state.lines[source.logical_row]
                .graphemes(true)
                .skip(source.char_start)
                .take(col)
                .map(|cluster| cluster_width(cluster) as usize)
                .sum()
        },
        |nav| nav.desired_cells,
    );
    let target_row = if down {
        (row + 1).min(vlines.len() - 1)
    } else {
        row.saturating_sub(1)
    };
    if target_row == row {
        return;
    }
    let target = &vlines[target_row];
    let mut cells = 0;
    let mut target_col = 0;
    for cluster in state.lines[target.logical_row]
        .graphemes(true)
        .skip(target.char_start)
        .take(target.char_count)
    {
        let width = cluster_width(cluster) as usize;
        if cells + width > desired_cells {
            break;
        }
        cells += width;
        target_col += 1;
    }
    (state.cursor_row, state.cursor_col) =
        textarea_visual_to_logical(vlines, target_row, target_col);
    state.navigation = Some(TextareaNavigation {
        row: state.cursor_row,
        col: state.cursor_col,
        wrap_width: state.wrap_width,
        upstream: target_col == target.char_count,
        desired_cells,
    });
}

/// Test whether a grapheme cluster is "alphanumeric" for word-boundary
/// navigation: its first scalar is alphanumeric (a cluster's base scalar
/// determines its class; trailing combining marks do not change it).
fn cluster_is_alphanumeric(cluster: &str) -> bool {
    cluster.chars().next().is_some_and(|c| c.is_alphanumeric())
}

/// Move a logical column index backward to the start of the previous word.
///
/// Columns are **grapheme-cluster** indices. Word boundary: a run of
/// one-or-more alphanumeric clusters. Leading non-alphanumeric clusters before
/// the cursor are skipped first, then the run of alphanumerics is consumed.
pub(super) fn prev_word_col(line: &str, col: usize) -> usize {
    let clusters: Vec<&str> = line.graphemes(true).collect();
    let mut pos = col.min(clusters.len());
    while pos > 0 && !cluster_is_alphanumeric(clusters[pos - 1]) {
        pos -= 1;
    }
    while pos > 0 && cluster_is_alphanumeric(clusters[pos - 1]) {
        pos -= 1;
    }
    pos
}

/// Move a logical column index forward past the end of the next word.
///
/// Columns are **grapheme-cluster** indices (see [`prev_word_col`]).
pub(super) fn next_word_col(line: &str, col: usize) -> usize {
    let clusters: Vec<&str> = line.graphemes(true).collect();
    let mut pos = col.min(clusters.len());
    while pos < clusters.len() && !cluster_is_alphanumeric(clusters[pos]) {
        pos += 1;
    }
    while pos < clusters.len() && cluster_is_alphanumeric(clusters[pos]) {
        pos += 1;
    }
    pos
}

impl Context {
    /// Shared input loop of `textarea` and `code_editor` (issue #466):
    /// keyboard editing, paste, mouse selection and clipboard for a
    /// [`TextareaState`]. `hooks` lets the code editor run its own key
    /// handling first and supply its own visual layout and mouse mapping.
    pub(super) fn textarea_input(
        &mut self,
        state: &mut TextareaState,
        focused: bool,
        focus_slot: Option<usize>,
        box_rect: Option<Rect>,
        hooks: TextareaInputHooks<'_>,
    ) -> TextareaInput {
        let TextareaInputHooks {
            key: key_hook,
            visual_lines: build_vlines,
            pos_at,
            edits_tab,
        } = hooks;
        let wrapping = state.wrap_width.is_some();
        if state.navigation.is_some_and(|nav| {
            nav.row != state.cursor_row
                || nav.col != state.cursor_col
                || nav.wrap_width != state.wrap_width
        }) {
            state.navigation = None;
        }
        // Only editing batches need a baseline for exact net-result changed
        // semantics. Idle and navigation frames never clone the document.
        let pre_lines = (focused
            && self.events.iter().enumerate().any(|(i, event)| {
                !self.consumed[i]
                    && match event {
                        Event::Paste(text) => !text.is_empty(),
                        Event::Key(key) if key.kind == KeyEventKind::Press => {
                            matches!(
                                key.code,
                                KeyCode::Char(_)
                                    | KeyCode::Enter
                                    | KeyCode::Backspace
                                    | KeyCode::Delete
                            ) || (edits_tab && matches!(key.code, KeyCode::Tab | KeyCode::BackTab))
                        }
                        _ => false,
                    }
            }))
        .then(|| state.lines.clone());
        let mut vlines = None;

        if focused {
            let mut consumed_indices = Vec::new();
            let mut clipboard: Option<String> = None;
            let mut pointer_captured = false;
            for (i, event) in self
                .events
                .iter()
                .enumerate()
                .filter(|(i, _)| !self.consumed[*i])
            {
                if let Event::Paste(text) = event {
                    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
                    if !normalized.is_empty()
                        && state.replace_selection_or_insert(&normalized, false)
                    {
                        vlines = None;
                    }
                    consumed_indices.push(i);
                    continue;
                }
                if let Event::Mouse(mouse) = event {
                    if let Some(rect) = box_rect {
                        let map = wrapping.then(|| {
                            vlines
                                .get_or_insert_with(|| build_vlines(&state.lines))
                                .as_slice()
                        });
                        if textarea_pointer(state, mouse, rect, map, pos_at, self.frame_instant) {
                            consumed_indices.push(i);
                            pointer_captured = true;
                        }
                    }
                    continue;
                }
                let Event::Key(key) = event else {
                    continue;
                };
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if matches!(
                    key.code,
                    KeyCode::Char(_) | KeyCode::Enter | KeyCode::Backspace | KeyCode::Delete
                ) {
                    vlines = None;
                }
                // Issue #466: the code editor's own keys (Tab, Enter
                // auto-indent, ...) run before the built-in ones.
                if key_hook(state, key) {
                    vlines = None;
                    state.navigation = None;
                    consumed_indices.push(i);
                    continue;
                }
                let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                let command = key.modifiers.contains(KeyModifiers::CONTROL)
                    || key.modifiers.contains(KeyModifiers::SUPER);
                let word = key.modifiers.contains(KeyModifiers::CONTROL)
                    || key.modifiers.contains(KeyModifiers::ALT);
                let before = (state.cursor_row, state.cursor_col);
                let selection = state.selection();
                // Issue #465: movement keys extend the selection with Shift
                // and drop it otherwise (after the match below).
                let movement = matches!(
                    key.code,
                    KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Up
                        | KeyCode::Down
                        | KeyCode::Home
                        | KeyCode::End
                );
                // Plain Left/Right with a selection collapse it to its edge.
                let collapse = match (&key.code, &selection) {
                    (KeyCode::Left, Some(range)) if !shift && !word => Some(range.start),
                    (KeyCode::Right, Some(range)) if !shift && !word => Some(range.end),
                    _ => None,
                };
                match key.code {
                    _ if collapse.is_some() => {
                        if let Some(edge) = collapse {
                            (state.cursor_row, state.cursor_col) = edge;
                        }
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Char('a' | 'A') if command && !state.ctrl_a_disabled => {
                        state.select_all();
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Char('c' | 'C') if command && !shift => {
                        // Without a selection Ctrl+C stays an app shortcut.
                        if let Some(text) = state.selected_text() {
                            clipboard = Some(text);
                            consumed_indices.push(i);
                        }
                    }
                    KeyCode::Char('x' | 'X') if command && !shift => {
                        if let Some(text) = state.selected_text() {
                            clipboard = Some(text);
                            state.delete_selection();
                            consumed_indices.push(i);
                        }
                    }
                    KeyCode::Char('z' | 'Z') if command && shift => {
                        state.redo();
                        state.anchor = None;
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.undo();
                        state.anchor = None;
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.redo();
                        state.anchor = None;
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Char('k')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && state.delete_selection() =>
                    {
                        consumed_indices.push(i);
                    }
                    KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let line_len = grapheme_count(&state.lines[state.cursor_row]);
                        if state.cursor_col < line_len {
                            state.push_history();
                            let cut = byte_index_for_grapheme(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            state.lines[state.cursor_row].truncate(cut);
                        }
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Left
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            || key.modifiers.contains(KeyModifiers::ALT) =>
                    {
                        if state.cursor_col > 0 {
                            state.cursor_col =
                                prev_word_col(&state.lines[state.cursor_row], state.cursor_col);
                        } else if state.cursor_row > 0 {
                            state.cursor_row -= 1;
                            state.cursor_col = grapheme_count(&state.lines[state.cursor_row]);
                        }
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Right
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            || key.modifiers.contains(KeyModifiers::ALT) =>
                    {
                        let line_len = grapheme_count(&state.lines[state.cursor_row]);
                        if state.cursor_col < line_len {
                            state.cursor_col =
                                next_word_col(&state.lines[state.cursor_row], state.cursor_col);
                        } else if state.cursor_row + 1 < state.lines.len() {
                            state.cursor_row += 1;
                            state.cursor_col = 0;
                        }
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Char(ch) if !has_global_shortcut_modifier(key.modifiers) => {
                        let mut encoded = [0; 4];
                        state.replace_selection_or_insert(ch.encode_utf8(&mut encoded), true);
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter => {
                        state.replace_selection_or_insert("\n", false);
                        consumed_indices.push(i);
                    }
                    KeyCode::Backspace | KeyCode::Delete if state.delete_selection() => {
                        consumed_indices.push(i);
                    }
                    KeyCode::Backspace => {
                        if state.cursor_col > 0 || state.cursor_row > 0 {
                            state.push_history();
                        }
                        if state.cursor_col > 0 {
                            let start = byte_index_for_grapheme(
                                &state.lines[state.cursor_row],
                                state.cursor_col - 1,
                            );
                            let end = byte_index_for_grapheme(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            state.lines[state.cursor_row].replace_range(start..end, "");
                            state.cursor_col =
                                grapheme_cursor_after(&state.lines[state.cursor_row], start);
                        } else if state.cursor_row > 0 {
                            let current = state.lines.remove(state.cursor_row);
                            state.cursor_row -= 1;
                            let edge = state.lines[state.cursor_row].len();
                            state.lines[state.cursor_row].push_str(&current);
                            state.cursor_col =
                                grapheme_cursor_after(&state.lines[state.cursor_row], edge);
                        }
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Left => {
                        if state.cursor_col > 0 {
                            state.cursor_col -= 1;
                        } else if state.cursor_row > 0 {
                            state.cursor_row -= 1;
                            state.cursor_col = grapheme_count(&state.lines[state.cursor_row]);
                        }
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Right => {
                        let line_len = grapheme_count(&state.lines[state.cursor_row]);
                        if state.cursor_col < line_len {
                            state.cursor_col += 1;
                        } else if state.cursor_row + 1 < state.lines.len() {
                            state.cursor_row += 1;
                            state.cursor_col = 0;
                        }
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Up => {
                        if wrapping {
                            let map = vlines.get_or_insert_with(|| build_vlines(&state.lines));
                            navigate_visual(state, map, false);
                        } else if state.cursor_row > 0 {
                            state.cursor_row -= 1;
                            state.cursor_col = state
                                .cursor_col
                                .min(grapheme_count(&state.lines[state.cursor_row]));
                        }
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Down => {
                        if wrapping {
                            let map = vlines.get_or_insert_with(|| build_vlines(&state.lines));
                            navigate_visual(state, map, true);
                        } else if state.cursor_row + 1 < state.lines.len() {
                            state.cursor_row += 1;
                            state.cursor_col = state
                                .cursor_col
                                .min(grapheme_count(&state.lines[state.cursor_row]));
                        }
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Home => {
                        state.cursor_col = 0;
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Delete => {
                        let line_len = grapheme_count(&state.lines[state.cursor_row]);
                        let will_mutate =
                            state.cursor_col < line_len || state.cursor_row + 1 < state.lines.len();
                        if will_mutate {
                            state.push_history();
                        }
                        if state.cursor_col < line_len {
                            let start = byte_index_for_grapheme(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            let end = byte_index_for_grapheme(
                                &state.lines[state.cursor_row],
                                state.cursor_col + 1,
                            );
                            state.lines[state.cursor_row].replace_range(start..end, "");
                            state.cursor_col =
                                grapheme_cursor_after(&state.lines[state.cursor_row], start);
                        } else if state.cursor_row + 1 < state.lines.len() {
                            let next = state.lines.remove(state.cursor_row + 1);
                            let edge = state.lines[state.cursor_row].len();
                            state.lines[state.cursor_row].push_str(&next);
                            state.cursor_col =
                                grapheme_cursor_after(&state.lines[state.cursor_row], edge);
                        }
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::End => {
                        state.cursor_col = grapheme_count(&state.lines[state.cursor_row]);
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
                if movement && collapse.is_none() {
                    let after = (state.cursor_row, state.cursor_col);
                    if shift {
                        if after != before || state.anchor.is_some() {
                            state.anchor.get_or_insert(before);
                        }
                    } else {
                        state.anchor = None;
                    }
                }
                let history_navigation = matches!(key.code, KeyCode::Char('z' | 'y'))
                    && key.modifiers.contains(KeyModifiers::CONTROL);
                if !matches!(key.code, KeyCode::Up | KeyCode::Down) && !history_navigation {
                    state.navigation = None;
                }
            }
            self.consume_indices(consumed_indices);
            if let Some(text) = clipboard {
                self.copy_to_clipboard(text);
            }
            if pointer_captured {
                self.text_pointer_capture = true;
            }
        } else {
            state.pointer.dragging = false;
        }
        let mut chords = crate::runtime::CHORD_HISTORY | crate::runtime::CHORD_KILL_LINE;
        if !state.ctrl_a_disabled {
            chords |= crate::runtime::CHORD_SELECT_ALL;
        }
        if state.selection().is_some() {
            chords |= crate::runtime::CHORD_COPY_CUT;
        }
        self.set_focus_slot_chords(focus_slot, chords);
        TextareaInput { pre_lines, vlines }
    }

    ///
    /// When focused, handles character input, Enter (new line), Backspace,
    /// arrow keys, Home, and End. The cursor is rendered as a block character.
    ///
    /// Set [`TextareaState::word_wrap`] to enable soft-wrapping at a given
    /// display-column width. Up/Down then navigate visual lines.
    ///
    /// Editing shortcuts: `Ctrl+K` deletes from the cursor to the end of the
    /// current line. `Ctrl+Left` / `Alt+Left` jumps to the previous word
    /// boundary; `Ctrl+Right` / `Alt+Right` jumps past the next word end.
    /// `Ctrl+Z` undoes the last edit and `Ctrl+Y` redoes it — see the
    /// [`TextareaState`] docs for the undo history policy.
    pub fn textarea(&mut self, state: &mut TextareaState, visible_rows: u32) -> Response {
        if state.lines.is_empty() {
            state.lines.push(String::new());
        }
        state.cursor_row = state.cursor_row.min(state.lines.len().saturating_sub(1));
        state.cursor_col = state
            .cursor_col
            .min(grapheme_count(&state.lines[state.cursor_row]));

        let focused = self.register_text_entry_focusable(
            crate::runtime::TextEntryKind::MultiLine,
            state.focus_key(),
        );
        let focus_slot = self.rollback.last_focusable_id;
        // Issue #469: an accessible-mode answer replaces the whole text.
        let mut a11y = self.a11y_begin(focused);
        let a11y_changed = match crate::runtime::A11yBegin::take_text(a11y.as_mut()) {
            Some(Some(text)) if text != state.lines.join("\n") => {
                state.set_value(text);
                true
            }
            _ => false,
        };
        let wrap_w = state.wrap_width.unwrap_or(u32::MAX);
        let wrapping = state.wrap_width.is_some();
        // Issue #465: `begin_widget_interaction` below takes the next
        // interaction slot; its previous-frame rect locates the rows.
        let box_rect = self
            .prev_hit_map
            .get(self.rollback.interaction_count)
            .copied()
            .filter(|rect| !rect.is_empty());

        let mut no_key_hook = |_: &mut TextareaState, _: &KeyEvent| false;
        let build_vlines = |lines: &[String]| textarea_build_visual_lines(lines, wrap_w);
        let TextareaInput { pre_lines, vlines } = self.textarea_input(
            state,
            focused,
            focus_slot,
            box_rect,
            TextareaInputHooks {
                key: &mut no_key_hook,
                visual_lines: &build_vlines,
                pos_at: &textarea_pos_at,
                edits_tab: false,
            },
        );

        // Issue #415: reuse last frame's soft-wrap layout while the document
        // and width are unchanged instead of re-wrapping it every frame.
        let mut wrap_cache = std::mem::take(&mut state.wrap_cache);
        let vlines = match (&vlines, wrapping) {
            (_, false) => None,
            (Some(map), true) => Some(map.as_slice()),
            (None, true) => Some(wrap_cache.visual_lines(&state.lines, wrap_w)),
        };
        let (cursor_vrow, cursor_vcol) = vlines
            .map_or((state.cursor_row, state.cursor_col), |map| {
                visual_cursor(state, map)
            });

        if cursor_vrow < state.scroll_offset {
            state.scroll_offset = cursor_vrow;
        }
        if cursor_vrow
            >= state
                .scroll_offset
                .saturating_add(visible_rows.max(1) as usize)
        {
            state.scroll_offset = cursor_vrow + 1 - visible_rows.max(1) as usize;
        }

        let (_interaction_id, mut response) = self.begin_widget_interaction(focused);
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Column,
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(self.theme.border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));

        let selection = state.selection();
        let selected_style = self.theme.text_selection_style();
        state.caret_glyph = focused.then_some((cursor_vrow, cursor_vcol));
        for vi in 0..visible_rows as usize {
            let actual_vi = state.scroll_offset + vi;
            let (seg_text, seg_selection) = if let Some(map) = vlines {
                if let Some(vl) = map.get(actual_vi) {
                    let line = &state.lines[vl.logical_row];
                    let start = byte_index_for_grapheme(line, vl.char_start);
                    let end = start + byte_index_for_grapheme(&line[start..], vl.char_count);
                    (
                        &line[start..end],
                        segment_selection(
                            selection.as_ref(),
                            vl.logical_row,
                            vl.char_start,
                            vl.char_count,
                        ),
                    )
                } else {
                    ("", None)
                }
            } else {
                match state.lines.get(actual_vi) {
                    Some(line) => (
                        line.as_str(),
                        segment_selection(selection.as_ref(), actual_vi, 0, grapheme_count(line)),
                    ),
                    None => ("", None),
                }
            };

            if let Some(sel) = seg_selection {
                // Issue #465: split the row into unselected / selected runs,
                // keeping the caret glyph in an unselected run.
                let caret = (actual_vi == cursor_vrow && focused).then_some(cursor_vcol);
                let mut runs: Vec<(String, bool)> = Vec::with_capacity(4);
                let mut cursor = None;
                let push = |runs: &mut Vec<(String, bool)>, text: &str, selected: bool| match runs
                    .last_mut()
                {
                    Some((run, flag)) if *flag == selected => run.push_str(text),
                    _ => runs.push((text.to_string(), selected)),
                };
                for (col, cluster) in seg_text.graphemes(true).enumerate() {
                    if caret == Some(col) {
                        push(&mut runs, "", false);
                        cursor = Some((runs.len() - 1, grapheme_count(&runs[runs.len() - 1].0)));
                        push(&mut runs, "▎", false);
                    }
                    push(&mut runs, cluster, sel.contains(&col));
                }
                if caret.is_some() && cursor.is_none() {
                    push(&mut runs, "", false);
                    cursor = Some((runs.len() - 1, grapheme_count(&runs[runs.len() - 1].0)));
                    push(&mut runs, "▎", false);
                }
                self.text_runs_with_cursor(
                    &runs,
                    Style::new().fg(self.theme.text),
                    selected_style,
                    cursor,
                    false,
                );
                continue;
            }

            let mut rendered = String::with_capacity(seg_text.len() + 3);
            let mut cursor_offset = None;
            let mut style = if seg_text.is_empty() {
                Style::new().fg(self.theme.text_dim)
            } else {
                Style::new().fg(self.theme.text)
            };

            if actual_vi == cursor_vrow && focused {
                let edge = byte_index_for_grapheme(seg_text, cursor_vcol);
                cursor_offset = Some(grapheme_count(&seg_text[..edge]));
                rendered.push_str(&seg_text[..edge]);
                rendered.push('▎');
                rendered.push_str(&seg_text[edge..]);
                style = Style::new().fg(self.theme.text);
            } else {
                rendered.push_str(seg_text);
            }

            self.styled_with_cursor(rendered, style, cursor_offset);
        }
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
        state.wrap_cache = wrap_cache;

        response.changed = pre_lines.is_some_and(|before| state.lines != before) || a11y_changed;
        if let Some(begin) = a11y {
            self.a11y_end_textarea(begin, state);
        }
        response
    }
}
