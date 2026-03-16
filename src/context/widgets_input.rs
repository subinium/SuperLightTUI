use super::*;

impl Context {
    /// Render a single-line text input. Auto-handles cursor, typing, and backspace.
    ///
    /// The widget claims focus via [`Context::register_focusable`]. When focused,
    /// it consumes character, backspace, arrow, Home, and End key events.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::TextInputState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut input = TextInputState::with_placeholder("Search...");
    /// ui.text_input(&mut input);
    /// // input.value holds the current text
    /// # });
    /// ```
    pub fn text_input(&mut self, state: &mut TextInputState) -> Response {
        self.text_input_colored(state, &WidgetColors::new())
    }

    pub fn text_input_colored(
        &mut self,
        state: &mut TextInputState,
        colors: &WidgetColors,
    ) -> Response {
        slt_assert(
            !state.value.contains('\n'),
            "text_input got a newline — use textarea instead",
        );
        let focused = self.register_focusable();
        let old_value = state.value.clone();
        state.cursor = state.cursor.min(state.value.chars().count());

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    let matched_suggestions = if state.show_suggestions {
                        state
                            .matched_suggestions()
                            .into_iter()
                            .map(str::to_string)
                            .collect::<Vec<String>>()
                    } else {
                        Vec::new()
                    };
                    let suggestions_visible = !matched_suggestions.is_empty();
                    if suggestions_visible {
                        state.suggestion_index = state
                            .suggestion_index
                            .min(matched_suggestions.len().saturating_sub(1));
                    }
                    match key.code {
                        KeyCode::Up if suggestions_visible => {
                            state.suggestion_index = state.suggestion_index.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Down if suggestions_visible => {
                            state.suggestion_index = (state.suggestion_index + 1)
                                .min(matched_suggestions.len().saturating_sub(1));
                            consumed_indices.push(i);
                        }
                        KeyCode::Esc if state.show_suggestions => {
                            state.show_suggestions = false;
                            state.suggestion_index = 0;
                            consumed_indices.push(i);
                        }
                        KeyCode::Tab if suggestions_visible => {
                            if let Some(selected) = matched_suggestions
                                .get(state.suggestion_index)
                                .or_else(|| matched_suggestions.first())
                            {
                                state.value = selected.clone();
                                state.cursor = state.value.chars().count();
                                state.show_suggestions = false;
                                state.suggestion_index = 0;
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Char(ch) => {
                            if let Some(max) = state.max_length {
                                if state.value.chars().count() >= max {
                                    continue;
                                }
                            }
                            let index = byte_index_for_char(&state.value, state.cursor);
                            state.value.insert(index, ch);
                            state.cursor += 1;
                            if !state.suggestions.is_empty() {
                                state.show_suggestions = true;
                                state.suggestion_index = 0;
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Backspace => {
                            if state.cursor > 0 {
                                let start = byte_index_for_char(&state.value, state.cursor - 1);
                                let end = byte_index_for_char(&state.value, state.cursor);
                                state.value.replace_range(start..end, "");
                                state.cursor -= 1;
                            }
                            if !state.suggestions.is_empty() {
                                state.show_suggestions = true;
                                state.suggestion_index = 0;
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Left => {
                            state.cursor = state.cursor.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Right => {
                            state.cursor = (state.cursor + 1).min(state.value.chars().count());
                            consumed_indices.push(i);
                        }
                        KeyCode::Home => {
                            state.cursor = 0;
                            consumed_indices.push(i);
                        }
                        KeyCode::Delete => {
                            let len = state.value.chars().count();
                            if state.cursor < len {
                                let start = byte_index_for_char(&state.value, state.cursor);
                                let end = byte_index_for_char(&state.value, state.cursor + 1);
                                state.value.replace_range(start..end, "");
                            }
                            if !state.suggestions.is_empty() {
                                state.show_suggestions = true;
                                state.suggestion_index = 0;
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::End => {
                            state.cursor = state.value.chars().count();
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
                if let Event::Paste(ref text) = event {
                    for ch in text.chars() {
                        if let Some(max) = state.max_length {
                            if state.value.chars().count() >= max {
                                break;
                            }
                        }
                        let index = byte_index_for_char(&state.value, state.cursor);
                        state.value.insert(index, ch);
                        state.cursor += 1;
                    }
                    if !state.suggestions.is_empty() {
                        state.show_suggestions = true;
                        state.suggestion_index = 0;
                    }
                    consumed_indices.push(i);
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if state.value.is_empty() {
            state.show_suggestions = false;
            state.suggestion_index = 0;
        }

        let matched_suggestions = if state.show_suggestions {
            state
                .matched_suggestions()
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<String>>()
        } else {
            Vec::new()
        };
        if !matched_suggestions.is_empty() {
            state.suggestion_index = state
                .suggestion_index
                .min(matched_suggestions.len().saturating_sub(1));
        }

        let visible_width = self.area_width.saturating_sub(4) as usize;
        let input_text = if state.value.is_empty() {
            if state.placeholder.len() > 100 {
                slt_warn(
                    "text_input placeholder is very long (>100 chars) — consider shortening it",
                );
            }
            let mut ph = state.placeholder.clone();
            if focused {
                ph.insert(0, '▎');
            }
            ph
        } else {
            let chars: Vec<char> = state.value.chars().collect();
            let display_chars: Vec<char> = if state.masked {
                vec!['•'; chars.len()]
            } else {
                chars.clone()
            };

            let cursor_display_pos: usize = display_chars[..state.cursor.min(display_chars.len())]
                .iter()
                .map(|c| UnicodeWidthChar::width(*c).unwrap_or(1))
                .sum();

            let scroll_offset = if cursor_display_pos >= visible_width {
                cursor_display_pos - visible_width + 1
            } else {
                0
            };

            let mut rendered = String::new();
            let mut current_width: usize = 0;
            for (idx, &ch) in display_chars.iter().enumerate() {
                let cw = UnicodeWidthChar::width(ch).unwrap_or(1);
                if current_width + cw <= scroll_offset {
                    current_width += cw;
                    continue;
                }
                if current_width - scroll_offset >= visible_width {
                    break;
                }
                if focused && idx == state.cursor {
                    rendered.push('▎');
                }
                rendered.push(ch);
                current_width += cw;
            }
            if focused && state.cursor >= display_chars.len() {
                rendered.push('▎');
            }
            rendered
        };
        let input_style = if state.value.is_empty() && !focused {
            Style::new()
                .dim()
                .fg(colors.fg.unwrap_or(self.theme.text_dim))
        } else {
            Style::new().fg(colors.fg.unwrap_or(self.theme.text))
        };

        let border_color = if focused {
            colors.accent.unwrap_or(self.theme.primary)
        } else if state.validation_error.is_some() {
            colors.accent.unwrap_or(self.theme.error)
        } else {
            colors.border.unwrap_or(self.theme.border)
        };

        let mut response = self
            .bordered(Border::Rounded)
            .border_style(Style::new().fg(border_color))
            .px(1)
            .col(|ui| {
                ui.styled(input_text, input_style);
            });
        response.focused = focused;
        response.changed = state.value != old_value;

        let errors = state.errors();
        if !errors.is_empty() {
            for error in errors {
                self.styled(
                    format!("⚠ {error}"),
                    Style::new()
                        .dim()
                        .fg(colors.accent.unwrap_or(self.theme.error)),
                );
            }
        } else if let Some(error) = state.validation_error.clone() {
            self.styled(
                format!("⚠ {error}"),
                Style::new()
                    .dim()
                    .fg(colors.accent.unwrap_or(self.theme.error)),
            );
        }

        if state.show_suggestions && !matched_suggestions.is_empty() {
            let start = state.suggestion_index.saturating_sub(4);
            let end = (start + 5).min(matched_suggestions.len());
            let suggestion_border = colors.border.unwrap_or(self.theme.border);
            self.bordered(Border::Rounded)
                .border_style(Style::new().fg(suggestion_border))
                .px(1)
                .col(|ui| {
                    for (idx, suggestion) in matched_suggestions[start..end].iter().enumerate() {
                        let actual_idx = start + idx;
                        if actual_idx == state.suggestion_index {
                            ui.styled(
                                suggestion.clone(),
                                Style::new()
                                    .bg(colors.accent.unwrap_or(ui.theme().selected_bg))
                                    .fg(colors.fg.unwrap_or(ui.theme().selected_fg)),
                            );
                        } else {
                            ui.styled(
                                suggestion.clone(),
                                Style::new().fg(colors.fg.unwrap_or(ui.theme().text)),
                            );
                        }
                    }
                });
        }
        response
    }

    /// Render an animated spinner.
    ///
    /// The spinner advances one frame per tick. Use [`SpinnerState::dots`] or
    /// [`SpinnerState::line`] to create the state, then chain style methods to
    /// color it.
    pub fn spinner(&mut self, state: &SpinnerState) -> &mut Self {
        self.styled(
            state.frame(self.tick).to_string(),
            Style::new().fg(self.theme.primary),
        )
    }

    /// Render toast notifications. Calls `state.cleanup(tick)` automatically.
    ///
    /// Expired messages are removed before rendering. If there are no active
    /// messages, nothing is rendered and `self` is returned unchanged.
    pub fn toast(&mut self, state: &mut ToastState) -> &mut Self {
        state.cleanup(self.tick);
        if state.messages.is_empty() {
            return self;
        }

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
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
        for message in state.messages.iter().rev() {
            let color = match message.level {
                ToastLevel::Info => self.theme.primary,
                ToastLevel::Success => self.theme.success,
                ToastLevel::Warning => self.theme.warning,
                ToastLevel::Error => self.theme.error,
            };
            self.styled(format!("  ● {}", message.text), Style::new().fg(color));
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Horizontal slider for numeric values.
    ///
    /// # Examples
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(80, 24).render(|ui| {
    /// let mut volume = 75.0_f64;
    /// let r = ui.slider("Volume", &mut volume, 0.0..=100.0);
    /// if r.changed { /* volume was adjusted */ }
    /// # });
    /// ```
    pub fn slider(
        &mut self,
        label: &str,
        value: &mut f64,
        range: std::ops::RangeInclusive<f64>,
    ) -> Response {
        let focused = self.register_focusable();
        let mut changed = false;

        let start = *range.start();
        let end = *range.end();
        let span = (end - start).max(0.0);
        let step = if span > 0.0 { span / 20.0 } else { 0.0 };

        *value = (*value).clamp(start, end);

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    match key.code {
                        KeyCode::Left | KeyCode::Char('h') => {
                            if step > 0.0 {
                                let next = (*value - step).max(start);
                                if (next - *value).abs() > f64::EPSILON {
                                    *value = next;
                                    changed = true;
                                }
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            if step > 0.0 {
                                let next = (*value + step).min(end);
                                if (next - *value).abs() > f64::EPSILON {
                                    *value = next;
                                    changed = true;
                                }
                            }
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }

            for idx in consumed_indices {
                self.consumed[idx] = true;
            }
        }

        let ratio = if span <= f64::EPSILON {
            0.0
        } else {
            ((*value - start) / span).clamp(0.0, 1.0)
        };

        let value_text = format_compact_number(*value);
        let label_width = UnicodeWidthStr::width(label) as u32;
        let value_width = UnicodeWidthStr::width(value_text.as_str()) as u32;
        let track_width = self
            .area_width
            .saturating_sub(label_width + value_width + 8)
            .max(10) as usize;
        let thumb_idx = if track_width <= 1 {
            0
        } else {
            (ratio * (track_width as f64 - 1.0)).round() as usize
        };

        let mut track = String::with_capacity(track_width);
        for i in 0..track_width {
            if i == thumb_idx {
                track.push('○');
            } else if i < thumb_idx {
                track.push('█');
            } else {
                track.push('━');
            }
        }

        let text_color = self.theme.text;
        let border_color = self.theme.border;
        let primary_color = self.theme.primary;
        let dim_color = self.theme.text_dim;
        let mut response = self.container().row(|ui| {
            ui.text(label).fg(text_color);
            ui.text("[").fg(border_color);
            ui.text(track).grow(1).fg(primary_color);
            ui.text("]").fg(border_color);
            if focused {
                ui.text(value_text.as_str()).bold().fg(primary_color);
            } else {
                ui.text(value_text.as_str()).fg(dim_color);
            }
        });
        response.focused = focused;
        response.changed = changed;
        response
    }

    /// Render a multi-line text area with the given number of visible rows.
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
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Char(ch) => {
                            if let Some(max) = state.max_length {
                                let total: usize =
                                    state.lines.iter().map(|line| line.chars().count()).sum();
                                if total >= max {
                                    continue;
                                }
                            }
                            let index = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            state.lines[state.cursor_row].insert(index, ch);
                            state.cursor_col += 1;
                            consumed_indices.push(i);
                        }
                        KeyCode::Enter => {
                            let split_index = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
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
                if let Event::Paste(ref text) = event {
                    for ch in text.chars() {
                        if ch == '\n' || ch == '\r' {
                            let split_index = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            let remainder = state.lines[state.cursor_row].split_off(split_index);
                            state.cursor_row += 1;
                            state.lines.insert(state.cursor_row, remainder);
                            state.cursor_col = 0;
                        } else {
                            if let Some(max) = state.max_length {
                                let total: usize =
                                    state.lines.iter().map(|l| l.chars().count()).sum();
                                if total >= max {
                                    break;
                                }
                            }
                            let index = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            state.lines[state.cursor_row].insert(index, ch);
                            state.cursor_col += 1;
                        }
                    }
                    consumed_indices.push(i);
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
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

        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let mut response = self.response_for(interaction_id);
        response.focused = focused;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
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
            let mut style = if seg_text.is_empty() {
                Style::new().fg(self.theme.text_dim)
            } else {
                Style::new().fg(self.theme.text)
            };

            if is_cursor_line && focused {
                rendered.clear();
                for (idx, ch) in seg_text.chars().enumerate() {
                    if idx == cursor_vcol {
                        rendered.push('▎');
                    }
                    rendered.push(ch);
                }
                if cursor_vcol >= seg_text.chars().count() {
                    rendered.push('▎');
                }
                style = Style::new().fg(self.theme.text);
            }

            self.styled(rendered, style);
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

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
    pub fn progress_bar(&mut self, ratio: f64, width: u32) -> &mut Self {
        self.progress_bar_colored(ratio, width, self.theme.primary)
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventBuilder, KeyCode, TestBackend};

    #[test]
    fn text_input_shows_matched_suggestions_for_prefix() {
        let mut backend = TestBackend::new(40, 10);
        let mut input = TextInputState::new();
        input.set_suggestions(vec!["hello".into(), "help".into(), "world".into()]);

        let events = EventBuilder::new().key('h').key('e').key('l').build();
        backend.run_with_events(events, |ui| {
            ui.text_input(&mut input);
        });

        backend.assert_contains("hello");
        backend.assert_contains("help");
        assert!(!backend.to_string_trimmed().contains("world"));
        assert_eq!(input.matched_suggestions().len(), 2);
    }

    #[test]
    fn text_input_tab_accepts_top_suggestion() {
        let mut backend = TestBackend::new(40, 10);
        let mut input = TextInputState::new();
        input.set_suggestions(vec!["hello".into(), "help".into(), "world".into()]);

        let events = EventBuilder::new()
            .key('h')
            .key('e')
            .key('l')
            .key_code(KeyCode::Tab)
            .build();
        backend.run_with_events(events, |ui| {
            ui.text_input(&mut input);
        });

        assert_eq!(input.value, "hello");
        assert!(!input.show_suggestions);
    }

    #[test]
    fn text_input_empty_value_shows_no_suggestions() {
        let mut backend = TestBackend::new(40, 10);
        let mut input = TextInputState::new();
        input.set_suggestions(vec!["hello".into(), "help".into(), "world".into()]);

        backend.render(|ui| {
            ui.text_input(&mut input);
        });

        let rendered = backend.to_string_trimmed();
        assert!(!rendered.contains("hello"));
        assert!(!rendered.contains("help"));
        assert!(!rendered.contains("world"));
        assert!(input.matched_suggestions().is_empty());
        assert!(!input.show_suggestions);
    }
}
