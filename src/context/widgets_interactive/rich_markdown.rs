use super::*;
use crate::{DEFAULT_CHORD_TIMEOUT_TICKS, RichLogState};

impl Context {
    /// Render a scrollable rich log view with styled entries.
    pub fn rich_log(&mut self, state: &mut RichLogState) -> Response {
        let focused = self.register_focusable();
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);

        let widget_height = if response.rect.height > 0 {
            response.rect.height as usize
        } else {
            self.area_height as usize
        };
        let viewport_height = widget_height.saturating_sub(2);
        let effective_height = if viewport_height == 0 {
            state.len().max(1)
        } else {
            viewport_height
        };
        let show_indicator = state.len() > effective_height;
        let visible_rows = if show_indicator {
            effective_height.saturating_sub(1).max(1)
        } else {
            effective_height
        };
        let max_offset = state.len().saturating_sub(visible_rows);
        if state.auto_scroll && state.scroll_offset == usize::MAX {
            state.scroll_offset = max_offset;
        } else {
            state.scroll_offset = state.scroll_offset.min(max_offset);
        }
        let old_offset = state.scroll_offset;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(1);
                        consumed_indices.push(i);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        state.scroll_offset = (state.scroll_offset + 1).min(max_offset);
                        consumed_indices.push(i);
                    }
                    KeyCode::PageUp => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(10);
                        consumed_indices.push(i);
                    }
                    KeyCode::PageDown => {
                        state.scroll_offset = (state.scroll_offset + 10).min(max_offset);
                        consumed_indices.push(i);
                    }
                    KeyCode::Home => {
                        state.scroll_offset = 0;
                        consumed_indices.push(i);
                    }
                    KeyCode::End => {
                        state.scroll_offset = max_offset;
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
        }

        if let Some(rect) = self.prev_hit_map.get(interaction_id).copied() {
            let mut consumed = Vec::new();
            for (i, mouse) in self.mouse_events_in_rect(rect) {
                let delta = self.scroll_lines_per_event as usize;
                match mouse.kind {
                    MouseKind::ScrollUp => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(delta);
                        consumed.push(i);
                    }
                    MouseKind::ScrollDown => {
                        state.scroll_offset = (state.scroll_offset + delta).min(max_offset);
                        consumed.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed);
        }

        state.scroll_offset = state.scroll_offset.min(max_offset);
        let start = state
            .scroll_offset
            .min(state.len().saturating_sub(visible_rows));
        let end = (start + visible_rows).min(state.len());

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Column,
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: Some(Border::Single),
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

        for entry in state.entries().skip(start).take(end.saturating_sub(start)) {
            self.commands.push(Command::RichText {
                segments: entry.segments.clone(),
                wrap: false,
                align: Align::Start,
                margin: Margin::default(),
                constraints: Constraints::default(),
            });
        }

        if show_indicator {
            let end_pos = end.min(state.len());
            let line = format!("{}-{} / {}", start.saturating_add(1), end_pos, state.len());
            self.styled(line, Style::new().dim().fg(self.theme.text_dim));
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
        response.changed = state.scroll_offset != old_offset;
        response
    }

    // ── virtual list ─────────────────────────────────────────────────

    /// Render a virtual list that only renders visible items.
    ///
    /// `total` is the number of items. `visible_height` limits how many rows
    /// are rendered. The closure `f` is called only for visible indices.
    ///
    /// This is the uniform fixed-height fast path: every item is treated as
    /// exactly one row. For chat/feed bubbles of differing heights see
    /// [`virtual_list_variable`](Context::virtual_list_variable).
    pub fn virtual_list(
        &mut self,
        state: &mut ListState,
        visible_height: u32,
        f: impl Fn(&mut Context, usize),
    ) -> Response {
        self.virtual_list_impl(state, visible_height, false, f)
    }

    /// Variable-height variant of [`virtual_list`](Context::virtual_list).
    ///
    /// Each item's height (in rows) comes from
    /// [`ListState::set_item_heights`](crate::widgets::ListState::set_item_heights);
    /// the visible range is computed so the rendered items fill at most
    /// `visible_height` rows starting from the current viewport. This is the
    /// chat/feed use case where bubbles vary in height (a one-line reply next
    /// to a 30-line code block). When no per-item heights are set it falls back
    /// to the uniform fast path and produces output identical to
    /// [`virtual_list`](Context::virtual_list). Rendering remains `O(visible)`:
    /// only items in the computed range invoke `f`, prefix-sum lookups are
    /// `O(log n)` and the prefix-sum rebuild is `O(n)` gated behind a dirty
    /// flag.
    ///
    /// An item taller than `visible_height` renders from its top and is never
    /// skipped. `PageUp`/`PageDown` move the selection by the number of *items*
    /// that fill `visible_height` *rows* from the current position. The
    /// "↑ N more / ↓ N more" affordances continue to count *items*.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::widgets::ListState;
    ///
    /// let mut state = ListState::new(vec!["short reply", "long\ncode\nblock", "ok"])
    ///     .with_item_heights(vec![1, 3, 1]);
    ///
    /// slt::run(|ui| {
    ///     ui.virtual_list_variable(&mut state, 10, |ui, idx| {
    ///         ui.text(format!("bubble {idx}"));
    ///     });
    /// })?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    ///
    /// Available since `0.21.0`.
    pub fn virtual_list_variable(
        &mut self,
        state: &mut ListState,
        visible_height: u32,
        f: impl Fn(&mut Context, usize),
    ) -> Response {
        self.virtual_list_impl(state, visible_height, true, f)
    }

    fn virtual_list_impl(
        &mut self,
        state: &mut ListState,
        visible_height: u32,
        variable: bool,
        f: impl Fn(&mut Context, usize),
    ) -> Response {
        if state.is_empty() {
            return Response::none();
        }
        state.selected = state.selected.min(state.len().saturating_sub(1));
        let use_heights = variable && state.has_item_heights();
        let focused = self.register_focusable();
        let (_interaction_id, mut response) = self.begin_widget_interaction(focused);
        let old_selected = state.selected;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                        let max_index = state.len().saturating_sub(1);
                        let _ =
                            handle_vertical_nav(&mut state.selected, max_index, key.code.clone());
                        consumed_indices.push(i);
                    }
                    KeyCode::PageUp => {
                        state.selected = if use_heights {
                            page_up_target(state, state.selected, visible_height)
                        } else {
                            state.selected.saturating_sub(visible_height as usize)
                        };
                        consumed_indices.push(i);
                    }
                    KeyCode::PageDown => {
                        state.selected = if use_heights {
                            page_down_target(state, state.selected, visible_height)
                        } else {
                            (state.selected + visible_height as usize)
                                .min(state.len().saturating_sub(1))
                        };
                        consumed_indices.push(i);
                    }
                    KeyCode::Home => {
                        state.selected = 0;
                        consumed_indices.push(i);
                    }
                    KeyCode::End => {
                        state.selected = state.len().saturating_sub(1);
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
        }

        let vh = visible_height as usize;
        let (start, end) = if use_heights {
            row_visible_range(state, vh)
        } else {
            // Uniform fixed-height path — byte-identical to the original
            // `virtual_list`: one item == one row.
            //
            // Clamp viewport_offset so `selected` stays inside [offset, offset + vh)
            // without forcing the cursor onto the bottom row when scrolling down.
            if state.selected < state.viewport_offset {
                state.viewport_offset = state.selected;
            }
            if vh > 0 && state.selected >= state.viewport_offset + vh {
                state.viewport_offset = state.selected - vh + 1;
            }
            let start = state.viewport_offset;
            (start, (start + vh).min(state.len()))
        };

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

        if start > 0 {
            let hidden = start.to_string();
            let mut line = String::with_capacity(hidden.len() + 10);
            line.push_str("  ↑ ");
            line.push_str(&hidden);
            line.push_str(" more");
            self.styled(line, Style::new().fg(self.theme.text_dim).dim());
        }

        for idx in start..end {
            f(self, idx);
        }

        let remaining = state.len().saturating_sub(end);
        if remaining > 0 {
            let hidden = remaining.to_string();
            let mut line = String::with_capacity(hidden.len() + 10);
            line.push_str("  ↓ ");
            line.push_str(&hidden);
            line.push_str(" more");
            self.styled(line, Style::new().fg(self.theme.text_dim).dim());
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
        response.changed = state.selected != old_selected;
        response
    }

    // ── command palette ──────────────────────────────────────────────

    /// Render a command palette overlay.
    pub fn command_palette(&mut self, state: &mut CommandPaletteState) -> Response {
        if !state.open {
            return Response::none();
        }

        state.last_selected = None;
        let interaction_id = self.next_interaction_id();

        let filtered: Vec<usize> = state.filtered_indices_cached().to_vec();
        let sel = state.selected().min(filtered.len().saturating_sub(1));
        state.set_selected(sel);

        let mut consumed_indices = Vec::new();

        for (i, key) in self.available_key_presses() {
            match key.code {
                KeyCode::Esc => {
                    state.open = false;
                    consumed_indices.push(i);
                }
                KeyCode::Up => {
                    let s = state.selected();
                    state.set_selected(s.saturating_sub(1));
                    consumed_indices.push(i);
                }
                KeyCode::Down => {
                    let filtered_len = state.filtered_indices_cached().len();
                    let s = state.selected();
                    state.set_selected((s + 1).min(filtered_len.saturating_sub(1)));
                    consumed_indices.push(i);
                }
                KeyCode::Enter => {
                    let filtered = state.filtered_indices_cached().to_vec();
                    if let Some(&cmd_idx) = filtered.get(state.selected()) {
                        state.last_selected = Some(cmd_idx);
                        state.open = false;
                    }
                    consumed_indices.push(i);
                }
                KeyCode::Backspace => {
                    if state.cursor > 0 {
                        let byte_idx = byte_index_for_grapheme(&state.input, state.cursor - 1);
                        let end_idx = byte_index_for_grapheme(&state.input, state.cursor);
                        state.input.replace_range(byte_idx..end_idx, "");
                        state.cursor -= 1;
                        state.set_selected(0);
                    }
                    consumed_indices.push(i);
                }
                KeyCode::Char(ch) if !has_global_shortcut_modifier(key.modifiers) => {
                    let byte_idx = byte_index_for_grapheme(&state.input, state.cursor);
                    state.input.insert(byte_idx, ch);
                    state.cursor = grapheme_count(&state.input[..byte_idx + ch.len_utf8()]);
                    state.set_selected(0);
                    consumed_indices.push(i);
                }
                _ => {}
            }
        }
        for (i, text) in self.available_pastes() {
            let inserted = text
                .graphemes(true)
                .filter(|cluster| {
                    cluster
                        .chars()
                        .all(|ch| (ch as u32) >= 0x20 && ch != '\u{7f}')
                })
                .collect::<String>();
            if !inserted.is_empty() {
                let byte_idx = byte_index_for_grapheme(&state.input, state.cursor);
                let inserted_end = byte_idx + inserted.len();
                state.input.insert_str(byte_idx, &inserted);
                state.cursor = grapheme_count(&state.input[..inserted_end]);
                state.set_selected(0);
            }
            consumed_indices.push(i);
        }
        self.consume_indices(consumed_indices);

        let filtered: Vec<usize> = state.filtered_indices_cached().to_vec();

        let _ = self.modal(|ui| {
            let primary = ui.theme.primary;
            let palette_pad = ui.theme.spacing.xs();
            let palette_input_padx = ui.theme.spacing.xs();
            let _ = ui
                .container()
                .border(Border::Rounded)
                .border_style(Style::new().fg(primary))
                .p(palette_pad)
                .max_w(60)
                .col(|ui| {
                    let border_color = ui.theme.primary;
                    let _ = ui
                        .bordered(Border::Rounded)
                        .border_style(Style::new().fg(border_color))
                        .px(palette_input_padx)
                        .col(|ui| {
                            let display = if state.input.is_empty() {
                                "Type to search...".to_string()
                            } else {
                                state.input.clone()
                            };
                            let style = if state.input.is_empty() {
                                Style::new().dim().fg(ui.theme.text_dim)
                            } else {
                                Style::new().fg(ui.theme.text)
                            };
                            ui.styled(display, style);
                        });

                    for (list_idx, &cmd_idx) in filtered.iter().enumerate() {
                        let cmd = &state.commands()[cmd_idx];
                        let is_selected = list_idx == state.selected();
                        let style = if is_selected {
                            Style::new().bold().fg(ui.theme.primary)
                        } else {
                            Style::new().fg(ui.theme.text)
                        };
                        let prefix = if is_selected { "▸ " } else { "  " };
                        let shortcut_text = cmd
                            .shortcut
                            .as_deref()
                            .map(|s| {
                                let mut text = String::with_capacity(s.len() + 4);
                                text.push_str("  (");
                                text.push_str(s);
                                text.push(')');
                                text
                            })
                            .unwrap_or_default();
                        let mut line = String::with_capacity(
                            prefix.len() + cmd.label.len() + shortcut_text.len(),
                        );
                        line.push_str(prefix);
                        line.push_str(&cmd.label);
                        line.push_str(&shortcut_text);
                        ui.styled(line, style);
                        if is_selected && !cmd.description.is_empty() {
                            let mut desc = String::with_capacity(4 + cmd.description.len());
                            desc.push_str("    ");
                            desc.push_str(&cmd.description);
                            ui.styled(desc, Style::new().dim().fg(ui.theme.text_dim));
                        }
                    }

                    if filtered.is_empty() {
                        ui.styled(
                            "  No matching commands",
                            Style::new().dim().fg(ui.theme.text_dim),
                        );
                    }
                });
        });

        let mut response = self.response_for(interaction_id);
        response.changed = state.last_selected.is_some();
        response
    }

    // ── markdown ─────────────────────────────────────────────────────

    /// Render a markdown string with basic formatting.
    ///
    /// Supports headers (`#`), bold (`**`), italic (`*`), inline code (`` ` ``),
    /// unordered lists (`-`/`*`), ordered lists (`1.`), blockquotes (`>`),
    /// horizontal rules (`---`), links (`[text](url)`), image placeholders
    /// (`![alt](url)`), code blocks with syntax highlighting, and GFM-style
    /// pipe tables. Paragraph text auto-wraps to container width.
    pub fn markdown(&mut self, text: &str) -> Response {
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
        self.skip_interaction_slot();

        let text_style = Style::new().fg(self.theme.text);
        let bold_style = Style::new().fg(self.theme.text).bold();
        let code_style = Style::new().fg(self.theme.accent);
        let border_style = Style::new().fg(self.theme.border).dim();

        let mut in_code_block = false;
        let mut code_block_lang = String::new();
        let mut code_block_lines: Vec<String> = Vec::new();
        let mut table_lines: Vec<String> = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();

            if in_code_block {
                if trimmed.starts_with("```") {
                    in_code_block = false;
                    let code_content = code_block_lines.join("\n");
                    let theme = self.theme;
                    let code_pad = theme.spacing.xs();
                    let highlighted: Option<Vec<Vec<(String, Style)>>> =
                        crate::syntax::highlight_code(&code_content, &code_block_lang, &theme);
                    let _ = self.container().bg(theme.surface).p(code_pad).col(|ui| {
                        if let Some(ref hl_lines) = highlighted {
                            for segs in hl_lines {
                                if segs.is_empty() {
                                    ui.text(" ");
                                } else {
                                    ui.line(|ui| {
                                        for (t, s) in segs {
                                            ui.styled(t, *s);
                                        }
                                    });
                                }
                            }
                        } else {
                            for cl in &code_block_lines {
                                ui.styled(cl, code_style);
                            }
                        }
                    });
                    code_block_lang.clear();
                    code_block_lines.clear();
                } else {
                    code_block_lines.push(line.to_string());
                }
                continue;
            }

            // Table row detection — collect lines starting with `|`
            if trimmed.starts_with('|') && trimmed.matches('|').count() >= 2 {
                table_lines.push(trimmed.to_string());
                continue;
            }
            // Flush accumulated table rows when a non-table line is encountered
            if !table_lines.is_empty() {
                self.render_markdown_table(
                    &table_lines,
                    text_style,
                    bold_style,
                    code_style,
                    border_style,
                );
                table_lines.clear();
            }

            if trimmed.is_empty() {
                self.text(" ");
                continue;
            }
            if trimmed == "---" || trimmed == "***" || trimmed == "___" {
                self.styled("─".repeat(40), border_style);
                continue;
            }
            if let Some(quote) = trimmed.strip_prefix("> ") {
                let quote_style = Style::new().fg(self.theme.text_dim).italic();
                let bar_style = Style::new().fg(self.theme.border);
                self.line(|ui| {
                    ui.styled("│ ", bar_style);
                    ui.styled(quote, quote_style);
                });
            } else if let Some(heading) = trimmed.strip_prefix("### ") {
                self.styled(heading, Style::new().bold().fg(self.theme.accent));
            } else if let Some(heading) = trimmed.strip_prefix("## ") {
                self.styled(heading, Style::new().bold().fg(self.theme.secondary));
            } else if let Some(heading) = trimmed.strip_prefix("# ") {
                self.styled(heading, Style::new().bold().fg(self.theme.primary));
            } else if let Some(item) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
            {
                self.line_wrap(|ui| {
                    ui.styled("  • ", text_style);
                    Self::render_md_inline_into(ui, item, text_style, bold_style, code_style);
                });
            } else if trimmed.starts_with(|c: char| c.is_ascii_digit()) && trimmed.contains(". ") {
                let parts: Vec<&str> = trimmed.splitn(2, ". ").collect();
                if parts.len() == 2 {
                    self.line_wrap(|ui| {
                        let mut prefix = String::with_capacity(4 + parts[0].len());
                        prefix.push_str("  ");
                        prefix.push_str(parts[0]);
                        prefix.push_str(". ");
                        ui.styled(prefix, text_style);
                        Self::render_md_inline_into(
                            ui, parts[1], text_style, bold_style, code_style,
                        );
                    });
                } else {
                    self.text(trimmed);
                }
            } else if let Some(lang) = trimmed.strip_prefix("```") {
                in_code_block = true;
                code_block_lang = lang.trim().to_string();
            } else {
                self.render_md_inline(trimmed, text_style, bold_style, code_style);
            }
        }

        if in_code_block && !code_block_lines.is_empty() {
            for cl in &code_block_lines {
                self.styled(cl, code_style);
            }
        }

        // Flush any remaining table rows at end of input
        if !table_lines.is_empty() {
            self.render_markdown_table(
                &table_lines,
                text_style,
                bold_style,
                code_style,
                border_style,
            );
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
        Response::none()
    }

    /// Render a GFM-style pipe table collected from markdown lines.
    fn render_markdown_table(
        &mut self,
        lines: &[String],
        text_style: Style,
        bold_style: Style,
        code_style: Style,
        border_style: Style,
    ) {
        if lines.is_empty() {
            return;
        }

        // Separate header, separator, and data rows
        let is_separator = |line: &str| -> bool {
            let inner = line.trim_matches('|').trim();
            !inner.is_empty()
                && inner
                    .chars()
                    .all(|c| c == '-' || c == ':' || c == '|' || c == ' ')
        };

        let parse_row = |line: &str| -> Vec<String> {
            let trimmed = line.trim().trim_start_matches('|').trim_end_matches('|');
            trimmed.split('|').map(|c| c.trim().to_string()).collect()
        };

        let mut header: Option<Vec<String>> = None;
        let mut data_rows: Vec<Vec<String>> = Vec::new();
        let mut found_separator = false;

        for (i, line) in lines.iter().enumerate() {
            if is_separator(line) {
                found_separator = true;
                continue;
            }
            if i == 0 && !found_separator {
                header = Some(parse_row(line));
            } else {
                data_rows.push(parse_row(line));
            }
        }

        // If no separator found, treat first row as header anyway
        if !found_separator && header.is_none() && !data_rows.is_empty() {
            header = Some(data_rows.remove(0));
        }

        // Calculate column count and widths
        let all_rows: Vec<&Vec<String>> = header.iter().chain(data_rows.iter()).collect();
        let col_count = all_rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if col_count == 0 {
            return;
        }
        let mut col_widths = vec![0usize; col_count];
        // Strip markdown formatting for accurate display-width calculation
        let stripped_rows: Vec<Vec<String>> = all_rows
            .iter()
            .map(|row| row.iter().map(|c| Self::md_strip(c)).collect())
            .collect();
        for row in &stripped_rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_count {
                    col_widths[i] = col_widths[i].max(UnicodeWidthStr::width(cell.as_str()));
                }
            }
        }

        // Top border ┌───┬───┐
        let mut top = String::from("┌");
        for (i, &w) in col_widths.iter().enumerate() {
            for _ in 0..w + 2 {
                top.push('─');
            }
            top.push(if i < col_count - 1 { '┬' } else { '┐' });
        }
        self.styled(&top, border_style);

        // Header row │ H1 │ H2 │
        if let Some(ref hdr) = header {
            self.line(|ui| {
                ui.styled("│", border_style);
                for (i, w) in col_widths.iter().enumerate() {
                    let raw = hdr.get(i).map(String::as_str).unwrap_or("");
                    let display_text = Self::md_strip(raw);
                    let cell_w = UnicodeWidthStr::width(display_text.as_str());
                    let padding: String = " ".repeat(w.saturating_sub(cell_w));
                    ui.styled(" ", bold_style);
                    ui.styled(&display_text, bold_style);
                    ui.styled(padding, bold_style);
                    ui.styled(" │", border_style);
                }
            });

            // Separator ├───┼───┤
            let mut sep = String::from("├");
            for (i, &w) in col_widths.iter().enumerate() {
                for _ in 0..w + 2 {
                    sep.push('─');
                }
                sep.push(if i < col_count - 1 { '┼' } else { '┤' });
            }
            self.styled(&sep, border_style);
        }

        // Data rows — render with inline formatting (bold, italic, code, links)
        for row in &data_rows {
            self.line(|ui| {
                ui.styled("│", border_style);
                for (i, w) in col_widths.iter().enumerate() {
                    let raw = row.get(i).map(String::as_str).unwrap_or("");
                    let display_text = Self::md_strip(raw);
                    let cell_w = UnicodeWidthStr::width(display_text.as_str());
                    let padding: String = " ".repeat(w.saturating_sub(cell_w));
                    ui.styled(" ", text_style);
                    Self::render_md_inline_into(ui, raw, text_style, bold_style, code_style);
                    ui.styled(padding, text_style);
                    ui.styled(" │", border_style);
                }
            });
        }

        // Bottom border └───┴───┘
        let mut bot = String::from("└");
        for (i, &w) in col_widths.iter().enumerate() {
            for _ in 0..w + 2 {
                bot.push('─');
            }
            bot.push(if i < col_count - 1 { '┴' } else { '┘' });
        }
        self.styled(&bot, border_style);
    }

    pub(crate) fn parse_inline_segments(
        text: &str,
        base: Style,
        bold: Style,
        code: Style,
    ) -> Vec<(String, Style)> {
        // All inline markers (`**`, `*`, `` ` ``) are single-byte ASCII, so
        // byte-index slicing of `text` is safe — multi-byte chars in `inner`
        // are never split. Avoids the `chars().collect::<Vec<_>>()` allocation
        // and per-match `String` reconstructions of the prior implementation.
        let mut segments: Vec<(String, Style)> = Vec::new();
        let bytes = text.as_bytes();
        let mut current = String::new();
        let mut i: usize = 0;

        while i < bytes.len() {
            // Bold: **text**
            if bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                let after_open = i + 2;
                if let Some(rel_end) = text[after_open..].find("**") {
                    let close = after_open + rel_end;
                    if !current.is_empty() {
                        segments.push((std::mem::take(&mut current), base));
                    }
                    let inner = text[after_open..close].to_string();
                    segments.push((inner, bold));
                    i = close + 2;
                    continue;
                }
            }

            // Italic: *text* — skipped if part of a `**` run.
            if bytes[i] == b'*'
                && (i + 1 >= bytes.len() || bytes[i + 1] != b'*')
                && (i == 0 || bytes[i - 1] != b'*')
            {
                let after_open = i + 1;
                if let Some(rel_end) = text[after_open..].find('*') {
                    let close = after_open + rel_end;
                    if !current.is_empty() {
                        segments.push((std::mem::take(&mut current), base));
                    }
                    let inner = text[after_open..close].to_string();
                    segments.push((inner, base.italic()));
                    i = close + 1;
                    continue;
                }
            }

            // Inline code: `text`
            if bytes[i] == b'`' {
                let after_open = i + 1;
                if let Some(rel_end) = text[after_open..].find('`') {
                    let close = after_open + rel_end;
                    if !current.is_empty() {
                        segments.push((std::mem::take(&mut current), base));
                    }
                    let inner = text[after_open..close].to_string();
                    segments.push((inner, code));
                    i = close + 1;
                    continue;
                }
            }

            // No marker — append one whole character (possibly multi-byte)
            // and advance past it.
            let ch = text[i..]
                .chars()
                .next()
                .expect("non-empty tail past bounds check");
            current.push(ch);
            i += ch.len_utf8();
        }

        if !current.is_empty() {
            segments.push((current, base));
        }
        segments
    }

    /// Render a markdown line with link/image support.
    ///
    /// Parses `[text](url)` as clickable OSC 8 links and `![alt](url)` as
    /// image placeholders, delegating the rest to `parse_inline_segments`.
    fn render_md_inline(
        &mut self,
        text: &str,
        text_style: Style,
        bold_style: Style,
        code_style: Style,
    ) {
        let items = Self::split_md_links(text);

        // Fast path: no links/images found
        if items.len() == 1
            && let MdInline::Text(ref t) = items[0]
        {
            let segs = Self::parse_inline_segments(t, text_style, bold_style, code_style);
            if segs.len() <= 1 {
                self.text(text)
                    .wrap()
                    .fg(text_style.fg.unwrap_or(Color::Reset));
            } else {
                self.line_wrap(|ui| {
                    for (s, st) in segs {
                        ui.styled(s, st);
                    }
                });
            }
            return;
        }

        // Mixed content — line_wrap collects both Text and Link commands
        self.line_wrap(|ui| {
            for item in &items {
                match item {
                    MdInline::Text(t) => {
                        let segs =
                            Self::parse_inline_segments(t, text_style, bold_style, code_style);
                        for (s, st) in segs {
                            ui.styled(s, st);
                        }
                    }
                    MdInline::Link { text, url } => {
                        ui.link(text.clone(), url.clone());
                    }
                    MdInline::Image { alt, .. } => {
                        // Render alt text only — matches md_strip() output for width consistency
                        ui.styled(alt.as_str(), code_style);
                    }
                }
            }
        });
    }

    /// Emit inline markdown segments into an existing context.
    ///
    /// Unlike `render_md_inline` which wraps in its own `line_wrap`,
    /// this emits raw commands into `ui` so callers can prepend a bullet
    /// or prefix before calling this inside their own `line_wrap`.
    fn render_md_inline_into(
        ui: &mut Context,
        text: &str,
        text_style: Style,
        bold_style: Style,
        code_style: Style,
    ) {
        let items = Self::split_md_links(text);
        for item in &items {
            match item {
                MdInline::Text(t) => {
                    let segs = Self::parse_inline_segments(t, text_style, bold_style, code_style);
                    for (s, st) in segs {
                        ui.styled(s, st);
                    }
                }
                MdInline::Link { text, url } => {
                    ui.link(text.clone(), url.clone());
                }
                MdInline::Image { alt, .. } => {
                    ui.styled(alt.as_str(), code_style);
                }
            }
        }
    }

    /// Split a markdown line into text, link, and image segments.
    fn split_md_links(text: &str) -> Vec<MdInline> {
        let chars: Vec<char> = text.chars().collect();
        let mut items: Vec<MdInline> = Vec::new();
        let mut current = String::new();
        let mut i = 0;

        while i < chars.len() {
            // Image: ![alt](url)
            if chars[i] == '!'
                && i + 1 < chars.len()
                && chars[i + 1] == '['
                && let Some((alt, _url, consumed)) = Self::parse_md_bracket_paren(&chars, i + 1)
            {
                if !current.is_empty() {
                    items.push(MdInline::Text(std::mem::take(&mut current)));
                }
                items.push(MdInline::Image { alt });
                i += 1 + consumed;
                continue;
            }
            // Link: [text](url)
            if chars[i] == '['
                && let Some((link_text, url, consumed)) = Self::parse_md_bracket_paren(&chars, i)
            {
                if !current.is_empty() {
                    items.push(MdInline::Text(std::mem::take(&mut current)));
                }
                items.push(MdInline::Link {
                    text: link_text,
                    url,
                });
                i += consumed;
                continue;
            }
            current.push(chars[i]);
            i += 1;
        }
        if !current.is_empty() {
            items.push(MdInline::Text(current));
        }
        if items.is_empty() {
            items.push(MdInline::Text(String::new()));
        }
        items
    }

    /// Parse `[text](url)` starting at `chars[start]` which must be `[`.
    /// Returns `(text, url, chars_consumed)` or `None` if no match.
    fn parse_md_bracket_paren(chars: &[char], start: usize) -> Option<(String, String, usize)> {
        if start >= chars.len() || chars[start] != '[' {
            return None;
        }
        // Find closing ]
        let mut depth = 0i32;
        let mut bracket_end = None;
        for (j, &ch) in chars.iter().enumerate().skip(start) {
            if ch == '[' {
                depth += 1;
            } else if ch == ']' {
                depth -= 1;
                if depth == 0 {
                    bracket_end = Some(j);
                    break;
                }
            }
        }
        let bracket_end = bracket_end?;
        // Must be followed by (
        if bracket_end + 1 >= chars.len() || chars[bracket_end + 1] != '(' {
            return None;
        }
        // Find closing )
        let paren_start = bracket_end + 2;
        let mut paren_end = None;
        let mut paren_depth = 1i32;
        for (j, &ch) in chars.iter().enumerate().skip(paren_start) {
            if ch == '(' {
                paren_depth += 1;
            } else if ch == ')' {
                paren_depth -= 1;
                if paren_depth == 0 {
                    paren_end = Some(j);
                    break;
                }
            }
        }
        let paren_end = paren_end?;
        let text: String = chars[start + 1..bracket_end].iter().collect();
        let url: String = chars[paren_start..paren_end].iter().collect();
        let consumed = paren_end - start + 1;
        Some((text, url, consumed))
    }

    /// Strip markdown inline formatting, returning plain display text.
    ///
    /// `**bold**` → `bold`, `*italic*` → `italic`, `` `code` `` → `code`,
    /// `[text](url)` → `text`, `![alt](url)` → `alt`.
    fn md_strip(text: &str) -> String {
        // Bracket/paren parsing for links/images still uses a `Vec<char>`
        // because the helper takes a char-slice; pre-build it once and reuse
        // the precomputed char→byte mapping for both code paths.
        let chars: Vec<char> = text.chars().collect();
        let char_to_byte = {
            let mut v = Vec::with_capacity(chars.len() + 1);
            let mut acc = 0usize;
            v.push(0);
            for ch in &chars {
                acc += ch.len_utf8();
                v.push(acc);
            }
            v
        };
        let bytes = text.as_bytes();
        let mut result = String::with_capacity(text.len());
        let mut ci: usize = 0;

        while ci < chars.len() {
            // Image: ![alt](url) — char-based bracket scanner is reused as-is.
            if chars[ci] == '!'
                && ci + 1 < chars.len()
                && chars[ci + 1] == '['
                && let Some((alt, _, consumed)) = Self::parse_md_bracket_paren(&chars, ci + 1)
            {
                result.push_str(&alt);
                ci += 1 + consumed;
                continue;
            }
            // Link: [text](url)
            if chars[ci] == '['
                && let Some((link_text, _, consumed)) = Self::parse_md_bracket_paren(&chars, ci)
            {
                result.push_str(&link_text);
                ci += consumed;
                continue;
            }

            let bi = char_to_byte[ci];

            // Bold: **text**
            if bytes[bi] == b'*' && bi + 1 < bytes.len() && bytes[bi + 1] == b'*' {
                let after_open = bi + 2;
                if let Some(rel_end) = text[after_open..].find("**") {
                    let close = after_open + rel_end;
                    let inner = &text[after_open..close];
                    result.push_str(inner);
                    ci += 2 + inner.chars().count() + 2;
                    continue;
                }
            }

            // Italic: *text* — skipped inside a `**` run.
            if bytes[bi] == b'*'
                && (bi + 1 >= bytes.len() || bytes[bi + 1] != b'*')
                && (bi == 0 || bytes[bi - 1] != b'*')
            {
                let after_open = bi + 1;
                if let Some(rel_end) = text[after_open..].find('*') {
                    let close = after_open + rel_end;
                    let inner = &text[after_open..close];
                    result.push_str(inner);
                    ci += 1 + inner.chars().count() + 1;
                    continue;
                }
            }

            // Inline code: `text`
            if bytes[bi] == b'`' {
                let after_open = bi + 1;
                if let Some(rel_end) = text[after_open..].find('`') {
                    let close = after_open + rel_end;
                    let inner = &text[after_open..close];
                    result.push_str(inner);
                    ci += 1 + inner.chars().count() + 1;
                    continue;
                }
            }

            result.push(chars[ci]);
            ci += 1;
        }
        result
    }

    // ── key chord (cross-frame multi-key sequence) ───────────────────

    /// Match a multi-key sequence whose keystrokes may span multiple frames
    /// (vi `gg`, leader keys).
    ///
    /// Unlike a single-frame matcher, `key_chord` buffers partial input in
    /// crate-internal `FrameState` across frames: typing `g` on one frame
    /// and `g` on the next returns `true` on the second frame. The partial
    /// prefix is cleared on a non-matching key press (vi semantics: `g` then
    /// `x` cancels a pending `gg`) or after
    /// [`DEFAULT_CHORD_TIMEOUT_TICKS`](crate::DEFAULT_CHORD_TIMEOUT_TICKS) of
    /// inactivity (measured on the same tick clock as notifications/animation).
    ///
    /// Returns `true` exactly once, on the frame that completes the sequence;
    /// the completing key event is consumed so downstream widgets in the same
    /// frame do not also handle it. It does not re-fire on later frames without
    /// new input.
    ///
    /// # Leader notation
    ///
    /// A leading `<space>` or `<leader>` token (or a literal space) matches the
    /// space key, e.g. `key_chord("<space>ff")`, `key_chord("<leader>ff")`, and
    /// `key_chord(" ff")` are equivalent. Only `<space>` / `<leader>` are
    /// recognized as special tokens; every other character is matched
    /// literally. Modifier-aware chords (`C-x C-s`) are out of scope.
    ///
    /// An empty sequence always returns `false`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// slt::run(|ui: &mut slt::Context| {
    ///     if ui.key_chord("gg") {
    ///         // vi-style: jump to the top
    ///     }
    ///     if ui.key_chord("<space>ff") {
    ///         // leader key: open a file finder
    ///     }
    /// });
    /// ```
    pub fn key_chord(&mut self, seq: &str) -> bool {
        self.key_chord_timeout(seq, DEFAULT_CHORD_TIMEOUT_TICKS)
    }

    /// [`key_chord`](Self::key_chord) with an explicit per-call timeout in ticks.
    ///
    /// A partial sequence is abandoned if `timeout_ticks` elapse on the tick
    /// clock without a matching next key. Use this when a chord should be more
    /// forgiving (large value) or stricter (small value) than the
    /// [`DEFAULT_CHORD_TIMEOUT_TICKS`](crate::DEFAULT_CHORD_TIMEOUT_TICKS)
    /// default. All other behavior matches [`key_chord`](Self::key_chord).
    ///
    /// # Example
    ///
    /// ```no_run
    /// slt::run(|ui: &mut slt::Context| {
    ///     // Require the second `g` within ~0.25s at 60Hz.
    ///     if ui.key_chord_timeout("gg", 15) {
    ///         // jump to top
    ///     }
    /// });
    /// ```
    pub fn key_chord_timeout(&mut self, seq: &str, timeout_ticks: u64) -> bool {
        let target = parse_chord(seq);
        if target.is_empty() {
            return false;
        }
        // Modal guard parity with the (deprecated) `key_seq`: suppress chords
        // while a modal owns input and no overlay is layered on top.
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }

        // Expire a stale prefix before processing this frame's keys.
        if self.tick.saturating_sub(self.chord.last_tick) > timeout_ticks {
            self.chord.pending.clear();
        }

        // Snapshot this frame's unconsumed char presses up front so the
        // immutable borrow from `available_key_presses` is released before we
        // mutate `self.chord` / call `consume_indices`.
        let char_presses: Vec<(usize, char)> = self
            .available_key_presses()
            .filter_map(|(i, key)| match key.code {
                KeyCode::Char(c) => Some((i, c)),
                _ => None,
            })
            .collect();

        let tick = self.tick;
        let mut completed_index: Option<usize> = None;
        let mut buf: Vec<char> = self.chord.pending.chars().collect();

        for (i, c) in char_presses {
            buf.push(c);
            // Keep only the longest suffix of `buf` that is a prefix of
            // `target`, giving vi-style overlap semantics (typing `gxg` still
            // arms `gg` from the trailing `g`).
            retain_longest_prefix(&mut buf, &target);
            self.chord.last_tick = tick;
            if buf.len() == target.len() {
                completed_index = Some(i);
                buf.clear();
                break;
            }
        }

        self.chord.pending = buf.into_iter().collect();
        if let Some(i) = completed_index {
            self.consume_indices([i]);
            true
        } else {
            false
        }
    }

    /// Check if a sequence of character keys was pressed.
    ///
    /// Deprecated alias for [`key_chord`](Self::key_chord). The original
    /// `key_seq` only matched when every key arrived in a single poll batch
    /// (i.e. physically simultaneous keypresses), so vi `gg` / leader keys
    /// were unreachable at any human typing speed. It now delegates to
    /// [`key_chord`](Self::key_chord) and matches across frames.
    #[deprecated(
        since = "0.21.0",
        note = "renamed to `key_chord`; now matches across frames"
    )]
    pub fn key_seq(&mut self, seq: &str) -> bool {
        self.key_chord(seq)
    }
}

/// Expand `<space>` / `<leader>` tokens in a chord spec into the characters
/// the matcher compares against. Everything else is taken literally. The only
/// special tokens are `<space>` and `<leader>` (both map to a literal space);
/// a literal space in the input is preserved as-is.
fn parse_chord(seq: &str) -> Vec<char> {
    let mut out = Vec::new();
    let mut rest = seq;
    while !rest.is_empty() {
        if let Some(tail) = rest.strip_prefix("<space>") {
            out.push(' ');
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("<leader>") {
            out.push(' ');
            rest = tail;
        } else {
            let c = rest.chars().next().expect("rest is non-empty");
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

/// Shrink `buf` to the longest suffix that is still a prefix of `target`.
///
/// This gives vi-style overlap semantics: after a mismatch the matcher does
/// not reset to empty but keeps any trailing characters that could begin a
/// fresh match. For example, with `target = ['g', 'g']`, the input `g x g`
/// leaves `buf = ['g']` (the trailing `g` re-arms the chord) rather than
/// discarding it.
fn retain_longest_prefix(buf: &mut Vec<char>, target: &[char]) {
    // Try progressively shorter suffixes of `buf`; the first that is a prefix
    // of `target` wins. An empty suffix is always a prefix, so this terminates.
    let mut start = 0;
    while start < buf.len() {
        if buf[start..].iter().zip(target).all(|(b, t)| b == t) {
            break;
        }
        start += 1;
    }
    if start > 0 {
        buf.drain(0..start);
    }
}

// ── variable-height virtual_list helpers ─────────────────────────────────
//
// These operate on the per-item `row_prefix` cached in `ListState` so the
// visible range and page jumps are computed in *rows*, not items, while the
// public `viewport_offset` keeps its "top item index" meaning. All lookups are
// O(log n) (binary search) or O(visible) (bounded linear accumulation).

/// Largest item index `i` such that `row_prefix[i] <= target_row` — i.e. the
/// item containing (or starting at) `target_row`. Result is in `0..n`.
fn item_at_row(row_prefix: &[u32], target_row: u32, n: usize) -> usize {
    // `row_prefix` has `n + 1` entries; entry `i` is the first row of item `i`.
    // partition_point returns the count of entries `<= target_row`; subtract one
    // to get the index of the item that owns that row, clamped to the last item.
    if n == 0 {
        return 0;
    }
    let count = row_prefix.partition_point(|&r| r <= target_row);
    count.saturating_sub(1).min(n - 1)
}

/// Compute the `[start, end)` item range for the variable-height path.
///
/// Clamps `state.viewport_offset` (top item index) so `selected` is fully
/// visible by *rows*, then accumulates item heights from the top until the
/// viewport is filled. `viewport_row_offset` is kept in sync with the top
/// item's starting row. `end` always covers at least one item, so an item
/// taller than the viewport renders from its top instead of being skipped
/// (no zero-progress loop).
fn row_visible_range(state: &mut ListState, vh: usize) -> (usize, usize) {
    state.ensure_row_prefix();
    let n = state.len();
    if n == 0 || vh == 0 {
        state.viewport_offset = state.viewport_offset.min(n.saturating_sub(1));
        state.viewport_row_offset = 0;
        return (state.viewport_offset, state.viewport_offset);
    }

    let vh_rows = vh as u32;
    let row_prefix = state.row_prefix();
    // `row_prefix[i]` is the top row of item `i`.
    let sel = state.selected.min(n - 1);
    let sel_top = row_prefix[sel];
    let sel_bottom = row_prefix[sel + 1]; // exclusive bottom row of `selected`

    let mut top = state.viewport_offset.min(n - 1);

    // Scroll up: if the selection's top row is above the viewport top row,
    // pull the viewport up to the selected item.
    if sel_top < row_prefix[top] {
        top = sel;
    }

    // Scroll down: while the selection's bottom row falls past the viewport
    // window, advance the top item. Each step makes progress (top increases),
    // so this terminates. Stop once `selected` fits or the selected item is
    // itself the top (a single item taller than the viewport).
    while top < sel && sel_bottom.saturating_sub(row_prefix[top]) > vh_rows {
        top += 1;
    }

    // Accumulate items from `top` until adding the next item would overflow
    // `vh` rows; the rendered items then sum to at most `vh` rows (a partially
    // clipped item is excluded). Always include at least the top item so a
    // tall item is never skipped (it renders from its top).
    let top_row = row_prefix[top];
    let target_bottom = top_row.saturating_add(vh_rows);
    // Largest exclusive `end` such that `row_prefix[end] <= target_bottom`,
    // i.e. items `top..end` fully fit within `vh` rows (their cumulative bottom
    // row does not exceed the viewport bottom). `partition_point` returns the
    // count of entries `<= target_bottom` (one past the largest matching
    // index), so subtract one to get the inclusive prefix index = exclusive
    // item end. Clamp to `[top + 1, n]` so at least one item always renders
    // (a tall item that overflows `vh` shows alone, from its top).
    let end = row_prefix
        .partition_point(|&r| r <= target_bottom)
        .saturating_sub(1)
        .clamp(top + 1, n);

    state.viewport_offset = top;
    state.viewport_row_offset = top_row as usize;
    (top, end)
}

/// Item index reached by paging *down* one viewport (`vh` rows) from `from`.
/// Advances by the count of items whose cumulative height fills `vh` rows,
/// guaranteeing forward progress of at least one item.
fn page_down_target(state: &mut ListState, from: usize, visible_height: u32) -> usize {
    state.ensure_row_prefix();
    let n = state.len();
    if n == 0 {
        return 0;
    }
    let from = from.min(n - 1);
    let row_prefix = state.row_prefix();
    let from_top = row_prefix[from];
    let target = from_top.saturating_add(visible_height.max(1));
    let next = item_at_row(row_prefix, target, n);
    next.max(from + 1).min(n - 1)
}

/// Item index reached by paging *up* one viewport (`vh` rows) from `from`.
/// Retreats by the count of items whose cumulative height fills `vh` rows,
/// guaranteeing backward progress of at least one item (until index 0).
fn page_up_target(state: &mut ListState, from: usize, visible_height: u32) -> usize {
    state.ensure_row_prefix();
    let n = state.len();
    if n == 0 {
        return 0;
    }
    let from = from.min(n - 1);
    let row_prefix = state.row_prefix();
    let from_bottom = row_prefix[from + 1];
    let target = from_bottom.saturating_sub(visible_height.max(1));
    let prev = item_at_row(row_prefix, target, n);
    prev.min(from.saturating_sub(1))
}
