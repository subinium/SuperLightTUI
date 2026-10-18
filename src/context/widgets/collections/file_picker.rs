use super::*;

impl Context {
    /// Render a file system browser with directory navigation.
    pub fn file_picker(&mut self, state: &mut FilePickerState) -> Response {
        if state.dirty && state.last_error().is_none() {
            let _ = state.try_refresh();
        }
        if !state.entries.is_empty() {
            state.selected = state.selected.min(state.entries.len().saturating_sub(1));
        }

        let focused = self.register_focusable();
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);
        let mut file_selected = false;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                        if !state.entries.is_empty() {
                            let _ = handle_vertical_nav(
                                &mut state.selected,
                                state.entries.len().saturating_sub(1),
                                key.code.clone(),
                            );
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter => {
                        if let Some(entry) = state.entries.get(state.selected).cloned() {
                            if entry.is_dir {
                                state.current_dir = entry.path;
                                state.selected = 0;
                                state.selected_file = None;
                                state.invalidate_scan();
                            } else {
                                state.selected_file = Some(entry.path);
                                file_selected = true;
                            }
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Backspace => {
                        if let Some(parent) = state.current_dir.parent().map(|p| p.to_path_buf()) {
                            state.current_dir = parent;
                            state.selected = 0;
                            state.selected_file = None;
                            state.invalidate_scan();
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Char('h') => {
                        state.show_hidden = !state.show_hidden;
                        state.selected = 0;
                        state.invalidate_scan();
                        consumed_indices.push(i);
                    }
                    KeyCode::Esc => {
                        state.selected_file = None;
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
        }

        if state.dirty && state.last_error().is_none() {
            let _ = state.try_refresh();
        }

        let widget_height = self.available_content_size().1 as usize;
        let status_rows =
            usize::from(state.scan_status() != crate::widgets::FilePickerScanStatus::Ready);
        let available_rows = widget_height.saturating_sub(1 + status_rows);
        let show_indicator = state.entries.len() > available_rows;
        let visible_rows = if show_indicator {
            available_rows.saturating_sub(1)
        } else {
            available_rows
        };
        state.keep_selected_visible(visible_rows);
        let start = state.viewport_offset();
        let end = start.saturating_add(visible_rows).min(state.entries.len());

        if let Some((rect, clicks)) = self.left_clicks_for_interaction(interaction_id) {
            let mut consumed = Vec::new();
            let row_start = rect.y.saturating_add(1 + status_rows as u32);
            for (i, mouse) in clicks {
                if mouse.y >= row_start {
                    let clicked = start.saturating_add((mouse.y - row_start) as usize);
                    if clicked < end {
                        state.selected = clicked;
                        consumed.push(i);
                    }
                }
            }
            self.consume_indices(consumed);
        }

        self.commands
            .push(Command::BeginContainer(BeginContainerArgs {
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
            }));

        let dir_text = {
            let dir = state.current_dir.display().to_string();
            let mut text = String::with_capacity(5 + dir.len());
            text.push_str("Dir: ");
            text.push_str(&dir);
            text
        };
        self.styled(dir_text, Style::new().fg(self.theme.text_dim).dim());

        match state.scan_status() {
            crate::widgets::FilePickerScanStatus::Error => {
                if let Some(error) = state.last_error() {
                    self.styled(
                        format!("Error: {error}"),
                        Style::new().fg(self.theme.error).bold(),
                    );
                }
            }
            crate::widgets::FilePickerScanStatus::Pending => {
                self.styled("(loading)", Style::new().fg(self.theme.text_dim).dim());
            }
            crate::widgets::FilePickerScanStatus::Stale => {
                self.styled(
                    "(stale; refresh pending)",
                    Style::new().fg(self.theme.warning).dim(),
                );
            }
            crate::widgets::FilePickerScanStatus::Ready => {}
        }

        if state.entries.is_empty() {
            if state.scan_status() == crate::widgets::FilePickerScanStatus::Ready {
                self.styled("(empty)", Style::new().fg(self.theme.text_dim).dim());
            }
        } else {
            for (idx, entry) in state
                .entries
                .iter()
                .enumerate()
                .skip(start)
                .take(end.saturating_sub(start))
            {
                let icon = if entry.is_dir { "▸ " } else { "  " };
                let row = if entry.is_dir {
                    let mut row = String::with_capacity(icon.len() + entry.name.len());
                    row.push_str(icon);
                    row.push_str(&entry.name);
                    row
                } else {
                    let size_text = entry
                        .size
                        .map(|size| size.to_string())
                        .unwrap_or_else(|| "?".to_string());
                    let mut row =
                        String::with_capacity(icon.len() + entry.name.len() + size_text.len() + 4);
                    row.push_str(icon);
                    row.push_str(&entry.name);
                    row.push_str("  ");
                    row.push_str(&size_text);
                    row.push_str(" B");
                    row
                };

                let style = if idx == state.selected {
                    if focused {
                        Style::new().bold().fg(self.theme.primary)
                    } else {
                        Style::new().fg(self.theme.primary)
                    }
                } else {
                    Style::new().fg(self.theme.text)
                };
                self.styled(row, style);
            }

            if show_indicator {
                self.styled(
                    format!(
                        "{}-{} / {}",
                        if start < end { start + 1 } else { 0 },
                        end,
                        state.entries.len()
                    ),
                    Style::new().fg(self.theme.text_dim).dim(),
                );
            }
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        response.changed = file_selected;
        response
    }
}
