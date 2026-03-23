use super::*;

impl Context {
    ///
    /// When focused, handles character input, Enter (new line), Backspace,
    /// arrow keys, Home, and End. The cursor is rendered as a block character.
    ///
    /// Set [`TextareaState::word_wrap`] to enable soft-wrapping at a given
    /// display-column width. Up/Down then navigate visual lines.
    pub fn textarea(&mut self, state: &mut TextareaState, visible_rows: u32) -> Response {
        if state.lines.is_empty() {
            state.lines.push(String::new());
        }
        let old_lines = state.lines.clone();
        state.cursor_row = state.cursor_row.min(state.lines.len().saturating_sub(1));
        state.cursor_col = state
            .cursor_col
            .min(state.lines[state.cursor_row].chars().count());

        let focused = self.register_focusable();
        let wrap_w = state.wrap_width.unwrap_or(u32::MAX);
        let wrapping = state.wrap_width.is_some();

        let pre_vlines = textarea_build_visual_lines(&state.lines, wrap_w);

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Char(ch) => {
                        if let Some(max) = state.max_length {
                            let total: usize =
                                state.lines.iter().map(|line| line.chars().count()).sum();
                            if total >= max {
                                continue;
                            }
                        }
                        let index =
                            byte_index_for_char(&state.lines[state.cursor_row], state.cursor_col);
                        state.lines[state.cursor_row].insert(index, ch);
                        state.cursor_col += 1;
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter => {
                        let split_index =
                            byte_index_for_char(&state.lines[state.cursor_row], state.cursor_col);
                        let remainder = state.lines[state.cursor_row].split_off(split_index);
                        state.cursor_row += 1;
                        state.lines.insert(state.cursor_row, remainder);
                        state.cursor_col = 0;
                        consumed_indices.push(i);
                    }
                    KeyCode::Backspace => {
                        if state.cursor_col > 0 {
                            let start = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col - 1,
                            );
                            let end = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            state.lines[state.cursor_row].replace_range(start..end, "");
                            state.cursor_col -= 1;
                        } else if state.cursor_row > 0 {
                            let current = state.lines.remove(state.cursor_row);
                            state.cursor_row -= 1;
                            state.cursor_col = state.lines[state.cursor_row].chars().count();
                            state.lines[state.cursor_row].push_str(&current);
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Left => {
                        if state.cursor_col > 0 {
                            state.cursor_col -= 1;
                        } else if state.cursor_row > 0 {
                            state.cursor_row -= 1;
                            state.cursor_col = state.lines[state.cursor_row].chars().count();
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Right => {
                        let line_len = state.lines[state.cursor_row].chars().count();
                        if state.cursor_col < line_len {
                            state.cursor_col += 1;
                        } else if state.cursor_row + 1 < state.lines.len() {
                            state.cursor_row += 1;
                            state.cursor_col = 0;
                        }
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
                                .min(state.lines[state.cursor_row].chars().count());
                        }
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
                                .min(state.lines[state.cursor_row].chars().count());
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Home => {
                        state.cursor_col = 0;
                        consumed_indices.push(i);
                    }
                    KeyCode::Delete => {
                        let line_len = state.lines[state.cursor_row].chars().count();
                        if state.cursor_col < line_len {
                            let start = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            let end = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col + 1,
                            );
                            state.lines[state.cursor_row].replace_range(start..end, "");
                        } else if state.cursor_row + 1 < state.lines.len() {
                            let next = state.lines.remove(state.cursor_row + 1);
                            state.lines[state.cursor_row].push_str(&next);
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::End => {
                        state.cursor_col = state.lines[state.cursor_row].chars().count();
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            for (i, text) in self.available_pastes() {
                for ch in text.chars() {
                    if ch == '\n' || ch == '\r' {
                        let split_index =
                            byte_index_for_char(&state.lines[state.cursor_row], state.cursor_col);
                        let remainder = state.lines[state.cursor_row].split_off(split_index);
                        state.cursor_row += 1;
                        state.lines.insert(state.cursor_row, remainder);
                        state.cursor_col = 0;
                    } else {
                        if let Some(max) = state.max_length {
                            let total: usize = state.lines.iter().map(|l| l.chars().count()).sum();
                            if total >= max {
                                break;
                            }
                        }
                        let index =
                            byte_index_for_char(&state.lines[state.cursor_row], state.cursor_col);
                        state.lines[state.cursor_row].insert(index, ch);
                        state.cursor_col += 1;
                    }
                }
                consumed_indices.push(i);
            }

            self.consume_indices(consumed_indices);
        }

        let vlines = textarea_build_visual_lines(&state.lines, wrap_w);
        let (cursor_vrow, cursor_vcol) =
            textarea_logical_to_visual(&vlines, state.cursor_row, state.cursor_col);

        if cursor_vrow < state.scroll_offset {
            state.scroll_offset = cursor_vrow;
        }
        if cursor_vrow >= state.scroll_offset + visible_rows as usize {
            state.scroll_offset = cursor_vrow + 1 - visible_rows as usize;
        }

        let (_interaction_id, mut response) = self.begin_widget_interaction(focused);
        self.commands.push(Command::BeginContainer {
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
        });

        for vi in 0..visible_rows as usize {
            let actual_vi = state.scroll_offset + vi;
            let (seg_text, is_cursor_line) = if let Some(vl) = vlines.get(actual_vi) {
                let line = &state.lines[vl.logical_row];
                let text: String = line
                    .chars()
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
                for (idx, ch) in seg_text.chars().enumerate() {
                    if idx == cursor_vcol {
                        cursor_offset = Some(rendered.chars().count());
                        rendered.push('▎');
                    }
                    rendered.push(ch);
                }
                if cursor_vcol >= seg_text.chars().count() {
                    cursor_offset = Some(rendered.chars().count());
                    rendered.push('▎');
                }
                style = Style::new().fg(self.theme.text);
            }

            self.styled_with_cursor(rendered, style, cursor_offset);
        }
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        response.changed = state.lines != old_lines;
        response
    }

    /// Render a progress bar (20 chars wide). `ratio` is clamped to `0.0..=1.0`.
    ///
    /// Uses block characters (`█` filled, `░` empty). For a custom width use
    /// [`Context::progress_bar`].
    pub fn progress(&mut self, ratio: f64) -> &mut Self {
        self.progress_bar(ratio, 20)
    }

    /// Render a progress bar with a custom character width.
    ///
    /// `ratio` is clamped to `0.0..=1.0`. `width` is the total number of
    /// characters rendered.
    /// Render a progress bar filled to the given ratio (0.0–1.0).
    pub fn progress_bar(&mut self, ratio: f64, width: u32) -> &mut Self {
        self.progress_bar_colored(ratio, width, self.theme.primary)
    }

    /// Render a progress bar with a custom fill color.
    pub fn progress_bar_colored(&mut self, ratio: f64, width: u32, color: Color) -> &mut Self {
        let clamped = ratio.clamp(0.0, 1.0);
        let filled = (clamped * width as f64).round() as u32;
        let empty = width.saturating_sub(filled);
        let mut bar = String::new();
        for _ in 0..filled {
            bar.push('█');
        }
        for _ in 0..empty {
            bar.push('░');
        }
        self.styled(bar, Style::new().fg(color))
    }
}
