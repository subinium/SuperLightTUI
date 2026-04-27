use super::*;

impl Context {
    /// Render an alert banner with icon and level-based coloring.
    pub fn alert(&mut self, message: &str, level: crate::widgets::AlertLevel) -> Response {
        use crate::widgets::AlertLevel;

        let theme = self.theme;
        let (icon, color) = match level {
            AlertLevel::Info => ("ℹ", theme.accent),
            AlertLevel::Success => ("✓", theme.success),
            AlertLevel::Warning => ("⚠", theme.warning),
            AlertLevel::Error => ("✕", theme.error),
        };

        let focused = self.register_focusable();
        let key_dismiss = if focused {
            let consumed: Vec<usize> = self
                .available_key_presses()
                .filter_map(|(i, key)| {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char('x')) {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect();
            let dismissed = !consumed.is_empty();
            self.consume_indices(consumed);
            dismissed
        } else {
            false
        };

        let mut response = self.container().col(|ui| {
            ui.line(|ui| {
                let mut icon_text = String::with_capacity(icon.len() + 2);
                icon_text.push(' ');
                icon_text.push_str(icon);
                icon_text.push(' ');
                ui.text(icon_text).fg(color).bold();
                ui.text(message).grow(1);
                ui.text(" [×] ").dim();
            });
        });
        response.focused = focused;
        if key_dismiss {
            response.clicked = true;
        }

        response
    }

    /// Yes/No confirmation dialog. Returns Response with .clicked=true when answered.
    ///
    /// `result` is set to true for Yes, false for No.
    ///
    /// # Examples
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(80, 24).render(|ui| {
    /// let mut answer = false;
    /// let r = ui.confirm("Delete this file?", &mut answer);
    /// if r.clicked && answer { /* user confirmed */ }
    /// # });
    /// ```
    pub fn confirm(&mut self, question: &str, result: &mut bool) -> Response {
        let focused = self.register_focusable();
        let mut is_yes = *result;
        let mut clicked = false;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Char('y') => {
                        is_yes = true;
                        *result = true;
                        clicked = true;
                        consumed_indices.push(i);
                    }
                    KeyCode::Char('n') => {
                        is_yes = false;
                        *result = false;
                        clicked = true;
                        consumed_indices.push(i);
                    }
                    KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right => {
                        is_yes = !is_yes;
                        *result = is_yes;
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter => {
                        *result = is_yes;
                        clicked = true;
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
        }

        let yes_style = if is_yes {
            if focused {
                Style::new().fg(self.theme.bg).bg(self.theme.success).bold()
            } else {
                Style::new().fg(self.theme.success).bold()
            }
        } else {
            Style::new().fg(self.theme.text_dim)
        };
        let no_style = if !is_yes {
            if focused {
                Style::new().fg(self.theme.bg).bg(self.theme.error).bold()
            } else {
                Style::new().fg(self.theme.error).bold()
            }
        } else {
            Style::new().fg(self.theme.text_dim)
        };

        let q_width = UnicodeWidthStr::width(question) as u32;
        let mut response = self.row(|ui| {
            ui.text(question);
            ui.text(" ");
            ui.styled("[Yes]", yes_style);
            ui.text(" ");
            ui.styled("[No]", no_style);
        });

        if !clicked {
            if let Some((mx, my)) = self.click_pos {
                // Hit-test against the row's recorded rect when available.
                // On the first frame the row has no entry in `prev_hit_map`
                // yet, so `response.rect` is the zero rect. In that case we
                // fall back to assuming the row starts at (0, 0): callers
                // testing confirm() on the first frame still get correct
                // x-axis hit-testing because the column layout begins at
                // x=0 by default.
                let row_x = response.rect.x;
                let in_row_y = response.rect.height == 0
                    || (my >= response.rect.y && my < response.rect.bottom());
                if in_row_y {
                    let yes_start = row_x + q_width + 1;
                    let yes_end = yes_start + 5;
                    let no_start = yes_end + 1;
                    let no_end = no_start + 4; // "[No]" = 4 display columns
                    if mx >= yes_start && mx < yes_end {
                        is_yes = true;
                        *result = true;
                        clicked = true;
                    } else if mx >= no_start && mx < no_end {
                        is_yes = false;
                        *result = false;
                        clicked = true;
                    }
                }
            }
        }

        response.focused = focused;
        response.clicked = clicked;
        response.changed = clicked;
        let _ = is_yes;
        response
    }

    /// Render a breadcrumb navigation bar. Returns the clicked segment index.
    pub fn breadcrumb(&mut self, segments: &[&str]) -> Option<usize> {
        self.breadcrumb_with(segments, " › ")
    }

    /// Render a breadcrumb with a custom separator string.
    pub fn breadcrumb_with(&mut self, segments: &[&str], separator: &str) -> Option<usize> {
        self.breadcrumb_response_with(segments, separator).1
    }

    /// Render a breadcrumb and return both the row [`Response`] and the
    /// clicked segment index.
    ///
    /// The `Response` exposes hover/focus state and the widget rectangle, while
    /// the `Option<usize>` carries the clicked segment index (same value as
    /// [`Self::breadcrumb`]).
    pub fn breadcrumb_response(&mut self, segments: &[&str]) -> (Response, Option<usize>) {
        self.breadcrumb_response_with(segments, " › ")
    }

    /// Render a breadcrumb with a custom separator and return both the row
    /// [`Response`] and the clicked segment index.
    pub fn breadcrumb_response_with(
        &mut self,
        segments: &[&str],
        separator: &str,
    ) -> (Response, Option<usize>) {
        let theme = self.theme;
        let last_idx = segments.len().saturating_sub(1);
        let mut clicked_idx: Option<usize> = None;

        let response = self.row(|ui| {
            for (i, segment) in segments.iter().enumerate() {
                let is_last = i == last_idx;
                if is_last {
                    ui.text(*segment).bold();
                } else {
                    let focused = ui.register_focusable();
                    let resp = ui.interaction();
                    let activated = resp.clicked || ui.consume_activation_keys(focused);
                    let color = if resp.hovered || focused {
                        theme.accent
                    } else {
                        theme.primary
                    };
                    ui.text(*segment).fg(color).underline();
                    if activated {
                        clicked_idx = Some(i);
                    }
                    ui.text(separator).dim();
                }
            }
        });

        (response, clicked_idx)
    }

    /// Collapsible section that toggles on click, Enter, or Space.
    pub fn accordion(
        &mut self,
        title: &str,
        open: &mut bool,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        let theme = self.theme;
        let focused = self.register_focusable();
        let old_open = *open;
        let toggled_from_key = self.consume_activation_keys(focused);
        if toggled_from_key {
            *open = !*open;
        }

        let icon = if *open { "▾" } else { "▸" };
        let title_color = if focused { theme.primary } else { theme.text };

        let mut response = self.container().col(|ui| {
            ui.line(|ui| {
                ui.text(icon).fg(title_color);
                let mut title_text = String::with_capacity(1 + title.len());
                title_text.push(' ');
                title_text.push_str(title);
                ui.text(title_text).bold().fg(title_color);
            });
        });

        if response.clicked {
            *open = !*open;
        }

        if *open {
            let _ = self.container().pl(2).col(f);
        }

        response.focused = focused;
        response.changed = *open != old_open;
        response
    }

    /// Render a key-value definition list with aligned columns.
    pub fn definition_list(&mut self, items: &[(&str, &str)]) -> Response {
        let max_key_width = items
            .iter()
            .map(|(k, _)| UnicodeWidthStr::width(*k))
            .max()
            .unwrap_or(0);

        let _ = self.col(|ui| {
            for (key, value) in items {
                ui.line(|ui| {
                    let key_display_w = UnicodeWidthStr::width(*key);
                    let pad = max_key_width.saturating_sub(key_display_w);
                    let mut padded = String::with_capacity(key.len() + pad);
                    padded.extend(std::iter::repeat(' ').take(pad));
                    padded.push_str(key);
                    ui.text(padded).dim();
                    ui.text("  ");
                    ui.text(*value);
                });
            }
        });

        Response::none()
    }

    /// Render a horizontal divider with a centered text label.
    pub fn divider_text(&mut self, label: &str) -> Response {
        let w = self.width();
        let label_len = UnicodeWidthStr::width(label) as u32;
        // Reserve `label_len + 2` for the label and its single-space padding on
        // each side, then split the remaining width evenly. On odd widths the
        // right separator is one cell longer (no asymmetry that's visible).
        let total_separator = w.saturating_sub(label_len + 2);
        let left_len = total_separator / 2;
        let right_len = total_separator - left_len;
        let left: String = "─".repeat(left_len as usize);
        let right: String = "─".repeat(right_len as usize);
        let theme = self.theme;
        self.line(|ui| {
            ui.text(&left).fg(theme.border);
            let mut label_text = String::with_capacity(label.len() + 2);
            label_text.push(' ');
            label_text.push_str(label);
            label_text.push(' ');
            ui.text(label_text).fg(theme.text);
            ui.text(&right).fg(theme.border);
        });

        Response::none()
    }

    /// Render a badge with the theme's primary color.
    pub fn badge(&mut self, label: &str) -> Response {
        let theme = self.theme;
        self.badge_colored(label, theme.primary)
    }

    /// Render a badge with a custom background color.
    pub fn badge_colored(&mut self, label: &str, color: Color) -> Response {
        let fg = Color::contrast_fg(color);
        let mut label_text = String::with_capacity(label.len() + 2);
        label_text.push(' ');
        label_text.push_str(label);
        label_text.push(' ');
        self.text(label_text).fg(fg).bg(color);

        Response::none()
    }

    /// Render a keyboard shortcut hint with reversed styling.
    pub fn key_hint(&mut self, key: &str) -> Response {
        let theme = self.theme;
        let mut key_text = String::with_capacity(key.len() + 2);
        key_text.push(' ');
        key_text.push_str(key);
        key_text.push(' ');
        self.text(key_text).reversed().fg(theme.text_dim);

        Response::none()
    }

    /// Render a label-value stat pair.
    pub fn stat(&mut self, label: &str, value: &str) -> Response {
        let _ = self.col(|ui| {
            ui.text(label).dim();
            ui.text(value).bold();
        });

        Response::none()
    }

    /// Render a stat pair with a custom value color.
    pub fn stat_colored(&mut self, label: &str, value: &str, color: Color) -> Response {
        let _ = self.col(|ui| {
            ui.text(label).dim();
            ui.text(value).bold().fg(color);
        });

        Response::none()
    }

    /// Render a stat pair with an up/down trend arrow.
    pub fn stat_trend(
        &mut self,
        label: &str,
        value: &str,
        trend: crate::widgets::Trend,
    ) -> Response {
        let theme = self.theme;
        let (arrow, color) = match trend {
            crate::widgets::Trend::Up => ("↑", theme.success),
            crate::widgets::Trend::Down => ("↓", theme.error),
        };
        let _ = self.col(|ui| {
            ui.text(label).dim();
            ui.line(|ui| {
                ui.text(value).bold();
                let mut arrow_text = String::with_capacity(1 + arrow.len());
                arrow_text.push(' ');
                arrow_text.push_str(arrow);
                ui.text(arrow_text).fg(color);
            });
        });

        Response::none()
    }

    /// Render a centered empty-state placeholder.
    pub fn empty_state(&mut self, title: &str, description: &str) -> Response {
        let _ = self.container().center().col(|ui| {
            ui.text(title).align(Align::Center);
            ui.text(description).dim().align(Align::Center);
        });

        Response::none()
    }

    /// Render a centered empty-state placeholder with an action button.
    pub fn empty_state_action(
        &mut self,
        title: &str,
        description: &str,
        action_label: &str,
    ) -> Response {
        let mut clicked = false;
        let _ = self.container().center().col(|ui| {
            ui.text(title).align(Align::Center);
            ui.text(description).dim().align(Align::Center);
            if ui.button(action_label).clicked {
                clicked = true;
            }
        });

        Response {
            clicked,
            changed: clicked,
            ..Response::none()
        }
    }

    /// Render a code block with keyword-based syntax highlighting.
    pub fn code_block(&mut self, code: &str) -> Response {
        self.code_block_lang(code, "")
    }

    /// Render a code block with language-aware syntax highlighting.
    pub fn code_block_lang(&mut self, code: &str, lang: &str) -> Response {
        let theme = self.theme;
        let highlighted: Option<Vec<Vec<(String, Style)>>> =
            crate::syntax::highlight_code(code, lang, &theme);
        let _ = self
            .bordered(Border::Rounded)
            .bg(theme.surface)
            .p(1)
            .col(|ui| {
                if let Some(ref lines) = highlighted {
                    render_tree_sitter_lines(ui, lines);
                } else {
                    for line in code.lines() {
                        render_highlighted_line(ui, line);
                    }
                }
            });

        Response::none()
    }

    /// Render a code block with line numbers and keyword highlighting.
    pub fn code_block_numbered(&mut self, code: &str) -> Response {
        self.code_block_numbered_lang(code, "")
    }

    /// Render a code block with line numbers and language-aware highlighting.
    pub fn code_block_numbered_lang(&mut self, code: &str, lang: &str) -> Response {
        let lines: Vec<&str> = code.lines().collect();
        let gutter_w = (lines.len().max(1).ilog10() + 1) as usize;
        let theme = self.theme;
        let highlighted: Option<Vec<Vec<(String, Style)>>> =
            crate::syntax::highlight_code(code, lang, &theme);
        let _ = self
            .bordered(Border::Rounded)
            .bg(theme.surface)
            .p(1)
            .col(|ui| {
                if let Some(ref hl_lines) = highlighted {
                    for (i, segs) in hl_lines.iter().enumerate() {
                        ui.line(|ui| {
                            ui.text(format!("{:>gutter_w$} │ ", i + 1))
                                .fg(theme.text_dim);
                            for (text, style) in segs {
                                ui.styled(text, *style);
                            }
                        });
                    }
                } else {
                    for (i, line) in lines.iter().enumerate() {
                        ui.line(|ui| {
                            ui.text(format!("{:>gutter_w$} │ ", i + 1))
                                .fg(theme.text_dim);
                            render_highlighted_line(ui, line);
                        });
                    }
                }
            });

        Response::none()
    }
}
