use super::*;

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

        let pre_lines = state.lines.clone();
        let pre_vlines = textarea_build_visual_lines(&state.lines, wrap_w);

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
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
                    KeyCode::Char(ch) => {
                        if let Some(max) = state.max_length {
                            let total: usize =
                                state.lines.iter().map(|line| grapheme_count(line)).sum();
                            if total >= max {
                                continue;
                            }
                        }
                        // Coalesce a typing burst into one undoable batch:
                        // only the first Char of the burst pushes a snapshot.
                        if !state.last_was_char_insert {
                            state.push_history();
                        }
                        let index = byte_index_for_grapheme(
                            &state.lines[state.cursor_row],
                            state.cursor_col,
                        );
                        state.lines[state.cursor_row].insert(index, ch);
                        state.cursor_col += 1;
                        state.last_was_char_insert = true;
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter => {
                        state.push_history();
                        let split_index = byte_index_for_grapheme(
                            &state.lines[state.cursor_row],
                            state.cursor_col,
                        );
                        let remainder = state.lines[state.cursor_row].split_off(split_index);
                        state.cursor_row += 1;
                        state.lines.insert(state.cursor_row, remainder);
                        state.cursor_col = 0;
                        state.last_was_char_insert = false;
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
                            state.cursor_col -= 1;
                        } else if state.cursor_row > 0 {
                            let current = state.lines.remove(state.cursor_row);
                            state.cursor_row -= 1;
                            state.cursor_col = grapheme_count(&state.lines[state.cursor_row]);
                            state.lines[state.cursor_row].push_str(&current);
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
                            let (vrow, vcol) = textarea_logical_to_visual(
                                &pre_vlines,
                                state.cursor_row,
                                state.cursor_col,
                            );
                            if vrow > 0 {
                                let (lr, lc) =
                                    textarea_visual_to_logical(&pre_vlines, vrow - 1, vcol);
                                state.cursor_row = lr;
                                state.cursor_col = lc;
                            }
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
                            let (vrow, vcol) = textarea_logical_to_visual(
                                &pre_vlines,
                                state.cursor_row,
                                state.cursor_col,
                            );
                            if vrow + 1 < pre_vlines.len() {
                                let (lr, lc) =
                                    textarea_visual_to_logical(&pre_vlines, vrow + 1, vcol);
                                state.cursor_row = lr;
                                state.cursor_col = lc;
                            }
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
                        } else if state.cursor_row + 1 < state.lines.len() {
                            let next = state.lines.remove(state.cursor_row + 1);
                            state.lines[state.cursor_row].push_str(&next);
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
            }
            for (i, text) in self.available_pastes() {
                // A paste is one undoable unit — push a single snapshot
                // before applying the burst.
                if !text.is_empty() {
                    state.push_history();
                }
                // Hoist total char count once per paste event and update
                // incrementally — recomputing via `.iter().map(...).sum()`
                // inside the loop would be O(n²) on large pastes.
                let mut total_chars: usize = state.lines.iter().map(|l| grapheme_count(l)).sum();
                for ch in text.chars() {
                    if let Some(max) = state.max_length
                        && total_chars >= max
                    {
                        break;
                    }
                    if ch == '\n' || ch == '\r' {
                        let split_index = byte_index_for_grapheme(
                            &state.lines[state.cursor_row],
                            state.cursor_col,
                        );
                        let remainder = state.lines[state.cursor_row].split_off(split_index);
                        state.cursor_row += 1;
                        state.lines.insert(state.cursor_row, remainder);
                        state.cursor_col = 0;
                        total_chars += 1;
                    } else {
                        let index = byte_index_for_grapheme(
                            &state.lines[state.cursor_row],
                            state.cursor_col,
                        );
                        state.lines[state.cursor_row].insert(index, ch);
                        state.cursor_col += 1;
                        total_chars += 1;
                    }
                }
                state.last_was_char_insert = false;
                consumed_indices.push(i);
            }

            self.consume_indices(consumed_indices);
        }

        let vlines = if state.lines == pre_lines {
            pre_vlines
        } else {
            textarea_build_visual_lines(&state.lines, wrap_w)
        };
        let (cursor_vrow, cursor_vcol) =
            textarea_logical_to_visual(&vlines, state.cursor_row, state.cursor_col);

        if cursor_vrow < state.scroll_offset {
            state.scroll_offset = cursor_vrow;
        }
        if cursor_vrow >= state.scroll_offset + visible_rows as usize {
            state.scroll_offset = cursor_vrow + 1 - visible_rows as usize;
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
            let (seg_text, is_cursor_line) = if let Some(vl) = vlines.get(actual_vi) {
                let line = &state.lines[vl.logical_row];
                // `char_start` / `char_count` are grapheme-cluster indices, so
                // slice by cluster to keep each cluster whole on its segment.
                let text: String = line
                    .graphemes(true)
                    .skip(vl.char_start)
                    .take(vl.char_count)
                    .collect();
                (text, actual_vi == cursor_vrow)
            } else {
                (String::new(), false)
            };

            let mut rendered = seg_text.clone();
            let mut cursor_offset = None;
            let mut style = if seg_text.is_empty() {
                Style::new().fg(self.theme.text_dim)
            } else {
                Style::new().fg(self.theme.text)
            };

            if is_cursor_line && focused {
                rendered.clear();
                // Iterate by cluster: `cursor_vcol` is a cluster index. The
                // emitted `cursor_offset` is the *scalar* length of `rendered`
                // before the cursor glyph, which is what the renderer consumes
                // (`text.chars().take(cursor_offset)` in render.rs).
                for (idx, g) in seg_text.graphemes(true).enumerate() {
                    if idx == cursor_vcol {
                        cursor_offset = Some(rendered.chars().count());
                        rendered.push('▎');
                    }
                    rendered.push_str(g);
                }
                if cursor_vcol >= grapheme_count(&seg_text) {
                    cursor_offset = Some(rendered.chars().count());
                    rendered.push('▎');
                }
                style = Style::new().fg(self.theme.text);
            }

            self.styled_with_cursor(rendered, style, cursor_offset);
        }
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        response.changed = state.lines != pre_lines;
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
