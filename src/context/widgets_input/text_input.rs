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

    /// Render a text input with custom widget colors.
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
            for (i, key) in self.available_key_presses() {
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
            for (i, text) in self.available_pastes() {
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

            self.consume_indices(consumed_indices);
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
        let (input_text, cursor_offset) = if state.value.is_empty() {
            if state.placeholder.len() > 100 {
                slt_warn(
                    "text_input placeholder is very long (>100 chars) — consider shortening it",
                );
            }
            let mut ph = state.placeholder.clone();
            if focused {
                ph.insert(0, '▎');
                (ph, Some(0))
            } else {
                (ph, None)
            }
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
            let mut cursor_offset = None;
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
                    cursor_offset = Some(rendered.chars().count());
                    rendered.push('▎');
                }
                rendered.push(ch);
                current_width += cw;
            }
            if focused && state.cursor >= display_chars.len() {
                cursor_offset = Some(rendered.chars().count());
                rendered.push('▎');
            }
            (rendered, cursor_offset)
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
            .grow(1)
            .col(|ui| {
                ui.styled_with_cursor(input_text, input_style, cursor_offset);
            });
        response.focused = focused;
        response.changed = state.value != old_value;

        let errors = state.errors();
        if !errors.is_empty() {
            for error in errors {
                let mut warning = String::with_capacity(2 + error.len());
                warning.push_str("⚠ ");
                warning.push_str(error);
                self.styled(
                    warning,
                    Style::new()
                        .dim()
                        .fg(colors.accent.unwrap_or(self.theme.error)),
                );
            }
        } else if let Some(error) = state.validation_error.clone() {
            let mut warning = String::with_capacity(2 + error.len());
            warning.push_str("⚠ ");
            warning.push_str(&error);
            self.styled(
                warning,
                Style::new()
                    .dim()
                    .fg(colors.accent.unwrap_or(self.theme.error)),
            );
        }

        if state.show_suggestions && !matched_suggestions.is_empty() {
            let start = state.suggestion_index.saturating_sub(4);
            let end = (start + 5).min(matched_suggestions.len());
            let suggestion_border = colors.border.unwrap_or(self.theme.border);
            let _ = self
                .bordered(Border::Rounded)
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
}
