use super::*;
use crate::widgets::{grapheme_cursor_after, prepare_text_insert};

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
        let colors = self.widget_theme.text_input;
        self.text_input_colored(state, &colors)
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
        // v0.21.1: capture the focus-edge flags immediately — this consumes the
        // `register_focusable` marker, so the result is correct regardless of
        // the child containers rendered below. Issue #208 left text_input never
        // populating gained_focus/lost_focus because it assembles its Response
        // by hand instead of via `begin_widget_interaction`.
        let (gained_focus, lost_focus) = self.focus_transitions(focused);
        let mut submitted = false;
        let old_value = state.value.clone();
        state.cursor = state.cursor.min(grapheme_count(&state.value));

        if focused {
            let mut consumed_indices = Vec::new();
            // Hoist matched_suggestions out of the loop and recompute only
            // after a mutation key (Char/Backspace/Delete) sets the dirty flag.
            // A 10-key burst with one mutation: 10 calls -> 2 calls.
            let compute_matched = |state: &TextInputState| -> Vec<String> {
                if state.show_suggestions {
                    state
                        .matched_suggestions()
                        .into_iter()
                        .map(str::to_string)
                        .collect()
                } else {
                    Vec::new()
                }
            };
            let mut matched_suggestions = compute_matched(state);
            let mut suggestions_dirty = false;
            for (i, event) in self
                .events
                .iter()
                .enumerate()
                .filter(|(i, _)| !self.consumed[*i])
            {
                if let Event::Paste(text) = event {
                    let inserted: String = text
                        .graphemes(true)
                        .filter(|cluster| {
                            cluster
                                .chars()
                                .all(|ch| (ch as u32) >= 0x20 && ch != '\u{7f}')
                        })
                        .collect();
                    let index = byte_index_for_grapheme(&state.value, state.cursor);
                    if let Some((value, end)) =
                        prepare_text_insert(&state.value, index, &inserted, state.max_length)
                    {
                        state.cursor = grapheme_cursor_after(&value, end);
                        state.value = value;
                        if !state.suggestions.is_empty() {
                            state.show_suggestions = true;
                            state.suggestion_index = 0;
                        }
                        suggestions_dirty = true;
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
                if suggestions_dirty {
                    matched_suggestions = compute_matched(state);
                    suggestions_dirty = false;
                }
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
                        suggestions_dirty = true;
                        consumed_indices.push(i);
                    }
                    KeyCode::Tab if suggestions_visible => {
                        if let Some(selected) = matched_suggestions
                            .get(state.suggestion_index)
                            .or_else(|| matched_suggestions.first())
                        {
                            state.value = selected.clone();
                            state.cursor = grapheme_count(&state.value);
                            state.show_suggestions = false;
                            state.suggestion_index = 0;
                            suggestions_dirty = true;
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Char(ch) if !has_global_shortcut_modifier(key.modifiers) => {
                        let index = byte_index_for_grapheme(&state.value, state.cursor);
                        let mut encoded = [0; 4];
                        let Some((value, end)) = prepare_text_insert(
                            &state.value,
                            index,
                            ch.encode_utf8(&mut encoded),
                            state.max_length,
                        ) else {
                            consumed_indices.push(i);
                            continue;
                        };
                        state.cursor = grapheme_cursor_after(&value, end);
                        state.value = value;
                        if !state.suggestions.is_empty() {
                            state.show_suggestions = true;
                            state.suggestion_index = 0;
                        }
                        suggestions_dirty = true;
                        consumed_indices.push(i);
                    }
                    KeyCode::Backspace => {
                        if state.cursor > 0 {
                            let start = byte_index_for_grapheme(&state.value, state.cursor - 1);
                            let end = byte_index_for_grapheme(&state.value, state.cursor);
                            state.value.replace_range(start..end, "");
                            state.cursor = grapheme_cursor_after(&state.value, start);
                        }
                        if !state.suggestions.is_empty() {
                            state.show_suggestions = true;
                            state.suggestion_index = 0;
                        }
                        suggestions_dirty = true;
                        consumed_indices.push(i);
                    }
                    KeyCode::Left => {
                        state.cursor = state.cursor.saturating_sub(1);
                        consumed_indices.push(i);
                    }
                    KeyCode::Right => {
                        state.cursor = (state.cursor + 1).min(grapheme_count(&state.value));
                        consumed_indices.push(i);
                    }
                    KeyCode::Home => {
                        state.cursor = 0;
                        consumed_indices.push(i);
                    }
                    KeyCode::Delete => {
                        let len = grapheme_count(&state.value);
                        if state.cursor < len {
                            let start = byte_index_for_grapheme(&state.value, state.cursor);
                            let end = byte_index_for_grapheme(&state.value, state.cursor + 1);
                            state.value.replace_range(start..end, "");
                            state.cursor = grapheme_cursor_after(&state.value, start);
                        }
                        if !state.suggestions.is_empty() {
                            state.show_suggestions = true;
                            state.suggestion_index = 0;
                        }
                        suggestions_dirty = true;
                        consumed_indices.push(i);
                    }
                    KeyCode::End => {
                        state.cursor = grapheme_count(&state.value);
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter => {
                        // v0.21.1: Enter submits the input. If the suggestion
                        // dropdown is open, accept the highlighted suggestion
                        // instead (Tab also accepts) — only a bare Enter with
                        // no open suggestions reports `submitted`.
                        if suggestions_visible {
                            if let Some(selected) = matched_suggestions
                                .get(state.suggestion_index)
                                .or_else(|| matched_suggestions.first())
                            {
                                state.value = selected.clone();
                                state.cursor = grapheme_count(&state.value);
                                state.show_suggestions = false;
                                state.suggestion_index = 0;
                                suggestions_dirty = true;
                            }
                        } else {
                            submitted = true;
                        }
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
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
            // Display units are grapheme clusters: `state.cursor` is a cluster
            // index, so each rendered unit (one source cluster, or one mask
            // glyph standing in for it) advances the cursor index by one.
            let clusters: Vec<&str> = state.value.graphemes(true).collect();
            let display_units: Vec<&str> = if state.masked {
                vec!["•"; clusters.len()]
            } else {
                clusters.clone()
            };

            let cursor_display_pos: usize = display_units[..state.cursor.min(display_units.len())]
                .iter()
                .map(|g| cluster_width(g).max(1) as usize)
                .sum();

            let scroll_offset = if cursor_display_pos >= visible_width {
                cursor_display_pos - visible_width + 1
            } else {
                0
            };

            let mut rendered = String::new();
            let mut cursor_offset = None;
            let mut current_width: usize = 0;
            for (idx, g) in display_units.iter().enumerate() {
                let cw = cluster_width(g).max(1) as usize;
                if current_width + cw <= scroll_offset {
                    current_width += cw;
                    continue;
                }
                if current_width.saturating_sub(scroll_offset) >= visible_width {
                    break;
                }
                // Preserve the cells occupied by a cluster crossing the left
                // edge, without emitting a partial wide grapheme.
                if current_width < scroll_offset {
                    rendered.extend(std::iter::repeat_n(
                        ' ',
                        (current_width + cw - scroll_offset).min(visible_width),
                    ));
                    current_width += cw;
                    continue;
                }
                if focused && idx == state.cursor {
                    cursor_offset = Some(grapheme_count(&rendered));
                    rendered.push('▎');
                }
                let cursor_cells = usize::from(cursor_offset.is_some());
                if current_width - scroll_offset + cw + cursor_cells > visible_width {
                    break;
                }
                rendered.push_str(g);
                current_width += cw;
            }
            if focused && visible_width > 0 && state.cursor >= display_units.len() {
                cursor_offset = Some(grapheme_count(&rendered));
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

        let input_padx = self.theme.spacing.xs();
        let mut response = self
            .bordered(Border::Rounded)
            .border_style(Style::new().fg(border_color))
            .px(input_padx)
            .col(|ui| {
                ui.styled_with_cursor_privacy(input_text, input_style, cursor_offset, state.masked);
            });
        response.focused = focused;
        response.changed = state.value != old_value;
        response.gained_focus = gained_focus;
        response.lost_focus = lost_focus;
        response.submitted = submitted;

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
            let suggestion_padx = self.theme.spacing.xs();
            let _ = self
                .bordered(Border::Rounded)
                .border_style(Style::new().fg(suggestion_border))
                .px(suggestion_padx)
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
