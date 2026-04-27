use super::*;
use crate::RichLogState;

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
            state.entries.len().max(1)
        } else {
            viewport_height
        };
        let show_indicator = state.entries.len() > effective_height;
        let visible_rows = if show_indicator {
            effective_height.saturating_sub(1).max(1)
        } else {
            effective_height
        };
        let max_offset = state.entries.len().saturating_sub(visible_rows);
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
            .min(state.entries.len().saturating_sub(visible_rows));
        let end = (start + visible_rows).min(state.entries.len());

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

        for entry in state
            .entries
            .iter()
            .skip(start)
            .take(end.saturating_sub(start))
        {
            self.commands.push(Command::RichText {
                segments: entry.segments.clone(),
                wrap: false,
                align: Align::Start,
                margin: Margin::default(),
                constraints: Constraints::default(),
            });
        }

        if show_indicator {
            let end_pos = end.min(state.entries.len());
            let line = format!(
                "{}-{} / {}",
                start.saturating_add(1),
                end_pos,
                state.entries.len()
            );
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
    pub fn virtual_list(
        &mut self,
        state: &mut ListState,
        visible_height: u32,
        f: impl Fn(&mut Context, usize),
    ) -> Response {
        if state.items.is_empty() {
            return Response::none();
        }
        state.selected = state.selected.min(state.items.len().saturating_sub(1));
        let focused = self.register_focusable();
        let (_interaction_id, mut response) = self.begin_widget_interaction(focused);
        let old_selected = state.selected;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                        let _ = handle_vertical_nav(
                            &mut state.selected,
                            state.items.len().saturating_sub(1),
                            key.code.clone(),
                        );
                        consumed_indices.push(i);
                    }
                    KeyCode::PageUp => {
                        state.selected = state.selected.saturating_sub(visible_height as usize);
                        consumed_indices.push(i);
                    }
                    KeyCode::PageDown => {
                        state.selected = (state.selected + visible_height as usize)
                            .min(state.items.len().saturating_sub(1));
                        consumed_indices.push(i);
                    }
                    KeyCode::Home => {
                        state.selected = 0;
                        consumed_indices.push(i);
                    }
                    KeyCode::End => {
                        state.selected = state.items.len().saturating_sub(1);
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
        }

        let vh = visible_height as usize;
        let start = if state.selected >= vh {
            state.selected - vh + 1
        } else {
            0
        };
        let end = (start + vh).min(state.items.len());

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

        let remaining = state.items.len().saturating_sub(end);
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
                    let s = state.selected();
                    state.set_selected((s + 1).min(filtered.len().saturating_sub(1)));
                    consumed_indices.push(i);
                }
                KeyCode::Enter => {
                    if let Some(&cmd_idx) = filtered.get(state.selected()) {
                        state.last_selected = Some(cmd_idx);
                        state.open = false;
                    }
                    consumed_indices.push(i);
                }
                KeyCode::Backspace => {
                    if state.cursor > 0 {
                        let byte_idx = byte_index_for_char(&state.input, state.cursor - 1);
                        let end_idx = byte_index_for_char(&state.input, state.cursor);
                        state.input.replace_range(byte_idx..end_idx, "");
                        state.cursor -= 1;
                        state.set_selected(0);
                    }
                    consumed_indices.push(i);
                }
                KeyCode::Char(ch) => {
                    let byte_idx = byte_index_for_char(&state.input, state.cursor);
                    state.input.insert(byte_idx, ch);
                    state.cursor += 1;
                    state.set_selected(0);
                    consumed_indices.push(i);
                }
                _ => {}
            }
        }
        self.consume_indices(consumed_indices);

        let filtered: Vec<usize> = state.filtered_indices_cached().to_vec();

        let _ = self.modal(|ui| {
            let primary = ui.theme.primary;
            let _ = ui
                .container()
                .border(Border::Rounded)
                .border_style(Style::new().fg(primary))
                .p(1)
                .max_w(60)
                .col(|ui| {
                    let border_color = ui.theme.primary;
                    let _ = ui
                        .bordered(Border::Rounded)
                        .border_style(Style::new().fg(border_color))
                        .px(1)
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
                        let cmd = &state.commands[cmd_idx];
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
                    let highlighted: Option<Vec<Vec<(String, Style)>>> =
                        crate::syntax::highlight_code(&code_content, &code_block_lang, &theme);
                    let _ = self.container().bg(theme.surface).p(1).col(|ui| {
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
        if items.len() == 1 {
            if let MdInline::Text(ref t) = items[0] {
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
            if chars[i] == '!' && i + 1 < chars.len() && chars[i + 1] == '[' {
                if let Some((alt, _url, consumed)) = Self::parse_md_bracket_paren(&chars, i + 1) {
                    if !current.is_empty() {
                        items.push(MdInline::Text(std::mem::take(&mut current)));
                    }
                    items.push(MdInline::Image { alt });
                    i += 1 + consumed;
                    continue;
                }
            }
            // Link: [text](url)
            if chars[i] == '[' {
                if let Some((link_text, url, consumed)) = Self::parse_md_bracket_paren(&chars, i) {
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
            if chars[ci] == '!' && ci + 1 < chars.len() && chars[ci + 1] == '[' {
                if let Some((alt, _, consumed)) = Self::parse_md_bracket_paren(&chars, ci + 1) {
                    result.push_str(&alt);
                    ci += 1 + consumed;
                    continue;
                }
            }
            // Link: [text](url)
            if chars[ci] == '[' {
                if let Some((link_text, _, consumed)) = Self::parse_md_bracket_paren(&chars, ci) {
                    result.push_str(&link_text);
                    ci += consumed;
                    continue;
                }
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

    // ── key sequence ─────────────────────────────────────────────────

    /// Check if a sequence of character keys was pressed across recent frames.
    ///
    /// Matches when each character in `seq` appears in consecutive unconsumed
    /// key events within this frame. For single-frame sequences only (e.g., "gg").
    pub fn key_seq(&self, seq: &str) -> bool {
        if seq.is_empty() {
            return false;
        }
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        let target: Vec<char> = seq.chars().collect();
        let mut matched = 0;
        for (_, key) in self.available_key_presses() {
            if let KeyCode::Char(c) = key.code {
                if c == target[matched] {
                    matched += 1;
                    if matched == target.len() {
                        return true;
                    }
                } else {
                    matched = 0;
                    if c == target[0] {
                        matched = 1;
                    }
                }
            }
        }
        false
    }
}
