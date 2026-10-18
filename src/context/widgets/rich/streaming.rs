use super::*;

impl Context {
    /// Render streaming text with a typing cursor indicator.
    ///
    /// Displays the accumulated text content. While `streaming` is true,
    /// shows a blinking cursor (`▌`) at the end.
    ///
    /// ```no_run
    /// use slt::widgets::StreamingTextState;
    ///
    /// let mut stream = StreamingTextState::new();
    /// stream.start();
    /// stream.push("Hello from ");
    /// stream.push("the AI!");
    /// slt::run(|ui: &mut slt::Context| {
    ///     // Push new tokens here as they arrive (e.g. drained from a channel).
    ///     ui.streaming_text(&mut stream);
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn streaming_text(&mut self, state: &mut StreamingTextState) -> Response {
        if state.streaming {
            state.cursor_tick = state.cursor_tick.wrapping_add(1);
            state.cursor_visible = (state.cursor_tick / 8).is_multiple_of(2);
        }

        if state.content.is_empty() && state.streaming {
            let cursor = if state.cursor_visible { "▌" } else { " " };
            let primary = self.theme.primary;
            self.text(cursor).fg(primary);
            return Response::none();
        }

        if !state.content.is_empty() {
            self.text(&state.content).wrap();
            if state.streaming && state.cursor_visible {
                let primary = self.theme.primary;
                self.styled("▌", Style::new().fg(primary));
            }
        }

        Response::none()
    }

    /// Render streaming markdown with a typing cursor indicator.
    ///
    /// Parses accumulated markdown content line-by-line while streaming.
    /// Supports headings, lists, inline formatting, horizontal rules, and
    /// fenced code blocks with open/close tracking across stream chunks.
    ///
    /// ```no_run
    /// use slt::widgets::StreamingMarkdownState;
    ///
    /// let mut stream = StreamingMarkdownState::new();
    /// stream.start();
    /// stream.push("# Hello\n");
    /// stream.push("- **streaming** markdown\n");
    /// stream.push("```rust\nlet x = 1;\n");
    /// slt::run(|ui: &mut slt::Context| {
    ///     // Push new chunks here as they arrive (e.g. drained from a channel).
    ///     ui.streaming_markdown(&mut stream);
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn streaming_markdown(
        &mut self,
        state: &mut crate::widgets::StreamingMarkdownState,
    ) -> Response {
        if state.streaming {
            state.cursor_tick = state.cursor_tick.wrapping_add(1);
            state.cursor_visible = (state.cursor_tick / 8).is_multiple_of(2);
        }

        if state.content.is_empty() && state.streaming {
            let cursor = if state.cursor_visible { "▌" } else { " " };
            let primary = self.theme.primary;
            self.text(cursor).fg(primary);
            return Response::none();
        }

        let show_cursor = state.streaming && state.cursor_visible;
        let trailing_newline = state.content.ends_with('\n');
        let lines: Vec<&str> = state.content.lines().collect();
        let last_line_index = lines.len().saturating_sub(1);

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

        for (idx, line) in lines.iter().enumerate() {
            let line = *line;
            let trimmed = line.trim();
            let append_cursor = show_cursor && !trailing_newline && idx == last_line_index;
            let cursor = if append_cursor { "▌" } else { "" };

            if in_code_block {
                if trimmed.starts_with("```") {
                    in_code_block = false;
                    code_block_lang.clear();
                    let mut line = String::from("  └────");
                    line.push_str(cursor);
                    self.styled(line, border_style);
                } else {
                    self.line(|ui| {
                        ui.text("  ");
                        render_highlighted_line(ui, line);
                        if !cursor.is_empty() {
                            ui.styled(cursor, Style::new().fg(ui.theme.primary));
                        }
                    });
                }
                continue;
            }

            if trimmed.is_empty() {
                if append_cursor {
                    self.styled("▌", Style::new().fg(self.theme.primary));
                } else {
                    self.text(" ");
                }
                continue;
            }

            if trimmed == "---" || trimmed == "***" || trimmed == "___" {
                let mut line = "─".repeat(40);
                line.push_str(cursor);
                self.styled(line, border_style);
                continue;
            }

            if let Some(heading) = trimmed.strip_prefix("### ") {
                let mut line = String::with_capacity(heading.len() + cursor.len());
                line.push_str(heading);
                line.push_str(cursor);
                self.styled(line, Style::new().bold().fg(self.theme.accent));
                continue;
            }

            if let Some(heading) = trimmed.strip_prefix("## ") {
                let mut line = String::with_capacity(heading.len() + cursor.len());
                line.push_str(heading);
                line.push_str(cursor);
                self.styled(line, Style::new().bold().fg(self.theme.secondary));
                continue;
            }

            if let Some(heading) = trimmed.strip_prefix("# ") {
                let mut line = String::with_capacity(heading.len() + cursor.len());
                line.push_str(heading);
                line.push_str(cursor);
                self.styled(line, Style::new().bold().fg(self.theme.primary));
                continue;
            }

            if let Some(code) = trimmed.strip_prefix("```") {
                in_code_block = true;
                code_block_lang = code.trim().to_string();
                let label = if code_block_lang.is_empty() {
                    "code".to_string()
                } else {
                    let mut label = String::from("code:");
                    label.push_str(&code_block_lang);
                    label
                };
                let mut line = String::with_capacity(5 + label.len() + cursor.len());
                line.push_str("  ┌─");
                line.push_str(&label);
                line.push('─');
                line.push_str(cursor);
                self.styled(line, border_style);
                continue;
            }

            if let Some(item) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
            {
                let segs = Self::parse_inline_segments(item, text_style, bold_style, code_style);
                if segs.len() <= 1 {
                    let mut line = String::with_capacity(4 + item.len() + cursor.len());
                    line.push_str("  • ");
                    line.push_str(item);
                    line.push_str(cursor);
                    self.styled(line, text_style);
                } else {
                    self.line(|ui| {
                        ui.styled("  • ", text_style);
                        for (s, st) in segs {
                            ui.styled(s, st);
                        }
                        if append_cursor {
                            ui.styled("▌", Style::new().fg(ui.theme.primary));
                        }
                    });
                }
                continue;
            }

            if trimmed.starts_with(|c: char| c.is_ascii_digit()) && trimmed.contains(". ") {
                let parts: Vec<&str> = trimmed.splitn(2, ". ").collect();
                if parts.len() == 2 {
                    let segs =
                        Self::parse_inline_segments(parts[1], text_style, bold_style, code_style);
                    if segs.len() <= 1 {
                        let mut line = String::with_capacity(
                            4 + parts[0].len() + parts[1].len() + cursor.len(),
                        );
                        line.push_str("  ");
                        line.push_str(parts[0]);
                        line.push_str(". ");
                        line.push_str(parts[1]);
                        line.push_str(cursor);
                        self.styled(line, text_style);
                    } else {
                        self.line(|ui| {
                            let mut prefix = String::with_capacity(4 + parts[0].len());
                            prefix.push_str("  ");
                            prefix.push_str(parts[0]);
                            prefix.push_str(". ");
                            ui.styled(prefix, text_style);
                            for (s, st) in segs {
                                ui.styled(s, st);
                            }
                            if append_cursor {
                                ui.styled("▌", Style::new().fg(ui.theme.primary));
                            }
                        });
                    }
                } else {
                    let mut line = String::with_capacity(trimmed.len() + cursor.len());
                    line.push_str(trimmed);
                    line.push_str(cursor);
                    self.styled(line, text_style);
                }
                continue;
            }

            let segs = Self::parse_inline_segments(trimmed, text_style, bold_style, code_style);
            if segs.len() <= 1 {
                let mut line = String::with_capacity(trimmed.len() + cursor.len());
                line.push_str(trimmed);
                line.push_str(cursor);
                self.styled(line, text_style);
            } else {
                self.line(|ui| {
                    for (s, st) in segs {
                        ui.styled(s, st);
                    }
                    if append_cursor {
                        ui.styled("▌", Style::new().fg(ui.theme.primary));
                    }
                });
            }
        }

        if show_cursor && trailing_newline {
            if in_code_block {
                self.styled("  ▌", code_style);
            } else {
                self.styled("▌", Style::new().fg(self.theme.primary));
            }
        }

        if state.in_code_block != in_code_block {
            state.in_code_block = in_code_block;
        }
        if state.code_block_lang != code_block_lang {
            state.code_block_lang = code_block_lang;
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
        Response::none()
    }

    /// Render a tool approval widget with approve/reject buttons.
    ///
    /// Shows the tool name, description, and two action buttons.
    /// Returns the updated [`ApprovalAction`] each frame.
    ///
    /// ```no_run
    /// use slt::widgets::{ApprovalAction, ToolApprovalState};
    ///
    /// let mut tool = ToolApprovalState::new("read_file", "Read contents of config.toml");
    /// slt::run(|ui: &mut slt::Context| {
    ///     ui.tool_approval(&mut tool);
    ///     if tool.action == ApprovalAction::Approved {
    ///         // run the tool
    ///     }
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn tool_approval(&mut self, state: &mut ToolApprovalState) -> Response {
        let old_action = state.action;
        let theme = self.theme;
        let _ = self.bordered(Border::Rounded).col(|ui| {
            let _ = ui.row(|ui| {
                ui.text("⚡").fg(theme.warning);
                ui.text(&state.tool_name).bold().fg(theme.primary);
            });
            ui.text(&state.description).dim();

            if state.action == ApprovalAction::Pending {
                let _ = ui.row(|ui| {
                    if ui.button("✓ Approve").clicked {
                        state.action = ApprovalAction::Approved;
                    }
                    if ui.button("✗ Reject").clicked {
                        state.action = ApprovalAction::Rejected;
                    }
                });
            } else {
                let (label, color) = match state.action {
                    ApprovalAction::Approved => ("✓ Approved", theme.success),
                    ApprovalAction::Rejected => ("✗ Rejected", theme.error),
                    ApprovalAction::Pending => unreachable!(),
                };
                ui.text(label).fg(color).bold();
            }
        });

        Response {
            changed: state.action != old_action,
            ..Response::none()
        }
    }

    /// Render a context bar showing active context items with token counts.
    ///
    /// Displays a horizontal bar of context sources (files, URLs, etc.)
    /// with their token counts, useful for AI chat interfaces.
    ///
    /// ```no_run
    /// # use slt::widgets::ContextItem;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let items = vec![ContextItem::new("main.rs", 1200), ContextItem::new("lib.rs", 800)];
    /// ui.context_bar(&items);
    /// # });
    /// ```
    pub fn context_bar(&mut self, items: &[ContextItem]) -> Response {
        if items.is_empty() {
            return Response::none();
        }

        let theme = self.theme;
        let total: usize = items.iter().map(|item| item.tokens).sum();

        let _ = self.container().row(|ui| {
            ui.text("📎").dim();
            for item in items {
                let token_count = format_token_count(item.tokens);
                let mut line = String::with_capacity(item.label.len() + token_count.len() + 3);
                line.push_str(&item.label);
                line.push_str(" (");
                line.push_str(&token_count);
                line.push(')');
                ui.text(line).fg(theme.secondary);
            }
            ui.spacer();
            let total_text = format_token_count(total);
            let mut line = String::with_capacity(2 + total_text.len());
            line.push_str("Σ ");
            line.push_str(&total_text);
            ui.text(line).dim();
        });

        Response::none()
    }
}
