use super::*;
use crate::widgets::{TextareaNavigation, grapheme_cursor_after};

fn current_navigation(state: &TextareaState) -> Option<TextareaNavigation> {
    state.navigation.filter(|nav| {
        nav.row == state.cursor_row
            && nav.col == state.cursor_col
            && nav.wrap_width == state.wrap_width
    })
}

fn visual_cursor(state: &TextareaState, vlines: &[TextareaVLine]) -> (usize, usize) {
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
fn prev_word_col(line: &str, col: usize) -> usize {
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
fn next_word_col(line: &str, col: usize) -> usize {
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
    /// [`TextareaState`] docs for the snapshot policy.
    pub fn textarea(&mut self, state: &mut TextareaState, visible_rows: u32) -> Response {
        if state.lines.is_empty() {
            state.lines.push(String::new());
        }
        state.cursor_row = state.cursor_row.min(state.lines.len().saturating_sub(1));
        state.cursor_col = state
            .cursor_col
            .min(grapheme_count(&state.lines[state.cursor_row]));

        let focused = self.register_focusable();
        let wrap_w = state.wrap_width.unwrap_or(u32::MAX);
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
                        Event::Key(key) if key.kind == KeyEventKind::Press => matches!(
                            key.code,
                            KeyCode::Char(_)
                                | KeyCode::Enter
                                | KeyCode::Backspace
                                | KeyCode::Delete
                        ),
                        _ => false,
                    }
            }))
        .then(|| state.lines.clone());
        let mut vlines = None;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self
                .events
                .iter()
                .enumerate()
                .filter(|(i, _)| !self.consumed[*i])
            {
                if let Event::Paste(text) = event {
                    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
                    if state.insert_text(&normalized, false) {
                        vlines = None;
                    }
                    consumed_indices.push(i);
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
                match key.code {
                    KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.undo();
                        state.last_was_char_insert = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.redo();
                        state.last_was_char_insert = false;
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
                        state.insert_text(ch.encode_utf8(&mut encoded), true);
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter => {
                        state.insert_text("\n", false);
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
                            let map = vlines.get_or_insert_with(|| {
                                textarea_build_visual_lines(&state.lines, wrap_w)
                            });
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
                            let map = vlines.get_or_insert_with(|| {
                                textarea_build_visual_lines(&state.lines, wrap_w)
                            });
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
                let history_navigation = matches!(key.code, KeyCode::Char('z' | 'y'))
                    && key.modifiers.contains(KeyModifiers::CONTROL);
                if !matches!(key.code, KeyCode::Up | KeyCode::Down) && !history_navigation {
                    state.navigation = None;
                }
            }
            self.consume_indices(consumed_indices);
        }

        let vlines = wrapping
            .then(|| vlines.unwrap_or_else(|| textarea_build_visual_lines(&state.lines, wrap_w)));
        let (cursor_vrow, cursor_vcol) = vlines
            .as_ref()
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

        for vi in 0..visible_rows as usize {
            let actual_vi = state.scroll_offset + vi;
            let seg_text = if let Some(map) = &vlines {
                if let Some(vl) = map.get(actual_vi) {
                    let line = &state.lines[vl.logical_row];
                    let start = byte_index_for_grapheme(line, vl.char_start);
                    let end = start + byte_index_for_grapheme(&line[start..], vl.char_count);
                    &line[start..end]
                } else {
                    ""
                }
            } else {
                state.lines.get(actual_vi).map_or("", String::as_str)
            };

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

        response.changed = pre_lines.is_some_and(|before| state.lines != before);
        response
    }

    /// Render a progress bar (20 chars wide). `ratio` is clamped to `0.0..=1.0`.
    ///
    /// Uses block characters (`█` filled, `░` empty). For a custom width use
    /// [`Context::progress_bar`]. For an inline label use [`Context::gauge`].
    ///
    /// Returns a [`Response`] so callers can detect hover, attach a tooltip,
    /// or implement click-to-set scrubbers. Prior to v0.20.0 this returned
    /// `&mut Self`; ignoring the return value still compiles but the
    /// `#[must_use]` attribute on `Response` warns at the call site.
    pub fn progress(&mut self, ratio: f64) -> Response {
        self.progress_bar(ratio, 20)
    }

    /// Render a progress bar with a custom character width.
    ///
    /// `ratio` is clamped to `0.0..=1.0`. `width` is the total number of
    /// characters rendered.
    pub fn progress_bar(&mut self, ratio: f64, width: u32) -> Response {
        self.progress_bar_colored(ratio, width, self.theme.primary)
    }

    /// Render a progress bar with a custom fill color.
    pub fn progress_bar_colored(&mut self, ratio: f64, width: u32, color: Color) -> Response {
        let response = self.interaction();
        let clamped = ratio.clamp(0.0, 1.0);
        let filled = (clamped * width as f64).round() as u32;
        let empty = width.saturating_sub(filled);
        let mut bar = String::with_capacity(width as usize * 3);
        for _ in 0..filled {
            bar.push('█');
        }
        for _ in 0..empty {
            bar.push('░');
        }
        self.styled(bar, Style::new().fg(color));
        response
    }
}
