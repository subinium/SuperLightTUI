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

        // 1) Keyboard hit-test runs first so it can mutate `is_yes`.
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

        // 2) Mouse hit-test runs *before* style computation and rendering so
        // the visual feedback for `[Yes]` / `[No]` reflects the click in the
        // same frame the click happened. Predict the row's interaction id
        // (the next slot the row will allocate) and look up the previous
        // frame's rect from `prev_hit_map`. On the first frame the row has
        // no entry yet, so we fall back to assuming the row starts at (0,0)
        // — same behaviour as the prior implementation.
        let q_width = UnicodeWidthStr::width(question) as u32;
        if !clicked {
            if let Some((mx, my)) = self.click_pos {
                let next_id = self.rollback.interaction_count;
                let prev_rect = self.prev_hit_map.get(next_id).copied();
                let row_x = prev_rect.map(|r| r.x).unwrap_or(0);
                let in_row_y = match prev_rect {
                    Some(r) if r.height > 0 => my >= r.y && my < r.bottom(),
                    _ => true,
                };
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

        // 3) Style computation reads the now-mutated `is_yes`.
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

        // 4) Render with the post-hit-test styles.
        let mut response = self.row(|ui| {
            ui.text(question);
            ui.text(" ");
            ui.styled("[Yes]", yes_style);
            ui.text(" ");
            ui.styled("[No]", no_style);
        });

        response.focused = focused;
        response.clicked = clicked;
        response.changed = clicked;
        response
    }

    /// Begin building a breadcrumb navigation bar with the default separator
    /// (` › `).
    ///
    /// Returns a [`Breadcrumb`] builder that auto-renders on `Drop`. Chain
    /// `.separator(s)` for a custom separator and `.color(c)` for a custom
    /// link color. Call `.show()` to render and obtain a
    /// [`BreadcrumbResponse`] carrying `clicked_segment` and `Deref<Response>`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// // simple
    /// ui.breadcrumb(&["Home", "Settings", "Profile"]);
    ///
    /// // with custom separator + color, capturing the response
    /// let r = ui
    ///     .breadcrumb(&["Home", "src", "lib.rs"])
    ///     .separator(" > ")
    ///     .show();
    /// if let Some(i) = r.clicked_segment {
    ///     // navigate to segment `i`
    /// }
    /// # });
    /// ```
    pub fn breadcrumb<'a>(&'a mut self, segments: &'a [&'a str]) -> Breadcrumb<'a> {
        Breadcrumb::new(self, segments)
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
            let indent = self.theme.spacing.sm();
            let _ = self.container().pl(indent).col(f);
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.badge("NEW");
    /// # });
    /// ```
    pub fn badge(&mut self, label: &str) -> Response {
        let theme = self.theme;
        self.badge_colored(label, theme.primary)
    }

    /// Render a badge with a custom background color.
    ///
    /// Foreground is auto-selected for contrast via [`Color::contrast_fg`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::Color;
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.badge_colored("ALPHA", Color::Magenta);
    /// # });
    /// ```
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.line(|ui| {
    ///     ui.text("Quit: ");
    ///     ui.key_hint("Ctrl+Q");
    /// });
    /// # });
    /// ```
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
    ///
    /// Renders as a column: a dim label above a bold value. Pair multiple
    /// stats in a [`row`](Self::row) for a compact dashboard strip.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.row(|ui| {
    ///     ui.stat("Users", "1.2k");
    ///     ui.stat("Revenue", "$8,420");
    /// });
    /// # });
    /// ```
    pub fn stat(&mut self, label: &str, value: &str) -> Response {
        let _ = self.col(|ui| {
            ui.text(label).dim();
            ui.text(value).bold();
        });

        Response::none()
    }

    /// Render a stat pair with a custom value color.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::Color;
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.stat_colored("Errors", "0", Color::Green);
    /// # });
    /// ```
    pub fn stat_colored(&mut self, label: &str, value: &str, color: Color) -> Response {
        let _ = self.col(|ui| {
            ui.text(label).dim();
            ui.text(value).bold().fg(color);
        });

        Response::none()
    }

    /// Render a stat pair with an up/down trend arrow.
    ///
    /// The arrow color follows the theme: `success` for [`Trend::Up`],
    /// `error` for [`Trend::Down`].
    ///
    /// [`Trend::Up`]: crate::widgets::Trend::Up
    /// [`Trend::Down`]: crate::widgets::Trend::Down
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::Trend;
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.stat_trend("MRR", "$24.5k", Trend::Up);
    /// ui.stat_trend("Churn", "1.8%", Trend::Down);
    /// # });
    /// ```
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
    ///
    /// Title is rendered prominently; description is dimmed below. Both are
    /// centered horizontally and vertically inside the available space.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # let items: Vec<&str> = vec![];
    /// # slt::run(|ui: &mut slt::Context| {
    /// if items.is_empty() {
    ///     ui.empty_state("No items yet", "Press 'a' to add one");
    /// }
    /// # });
    /// ```
    pub fn empty_state(&mut self, title: &str, description: &str) -> Response {
        let _ = self.container().center().col(|ui| {
            ui.text(title).align(Align::Center);
            ui.text(description).dim().align(Align::Center);
        });

        Response::none()
    }

    /// Render a centered empty-state placeholder with an action button.
    ///
    /// Returns a [`Response`] whose `clicked` field is `true` on the frame
    /// the action button is activated.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # let items: Vec<&str> = vec![];
    /// # slt::run(|ui: &mut slt::Context| {
    /// if items.is_empty() {
    ///     let r = ui.empty_state_action("No items yet", "Get started", "Add first item");
    ///     if r.clicked {
    ///         // open create flow
    ///     }
    /// }
    /// # });
    /// ```
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
        let pad = theme.spacing.xs();
        let highlighted: Option<Vec<Vec<(String, Style)>>> =
            crate::syntax::highlight_code(code, lang, &theme);
        let _ = self
            .bordered(Border::Rounded)
            .bg(theme.surface)
            .p(pad)
            .col(|ui| {
                if let Some(ref lines) = highlighted {
                    render_tree_sitter_lines(ui, lines);
                } else {
                    for line in code.lines() {
                        ui.line(|ui| render_highlighted_line(ui, line));
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
        let pad = theme.spacing.xs();
        let highlighted: Option<Vec<Vec<(String, Style)>>> =
            crate::syntax::highlight_code(code, lang, &theme);
        let _ = self
            .bordered(Border::Rounded)
            .bg(theme.surface)
            .p(pad)
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

/// Breadcrumb navigation bar builder. Auto-renders on `Drop`.
///
/// Constructed via [`Context::breadcrumb`]. Chain `.separator(s)` to override
/// the default ` › ` separator and `.color(c)` to override the link color.
/// Drop the value to render without capturing a response, or call
/// [`Self::show`] to render and obtain a [`BreadcrumbResponse`].
///
/// `Drop` is intentional: `ui.breadcrumb(&["Home", "src"]).separator(" > ");`
/// is the idiomatic form when the response isn't needed.
pub struct Breadcrumb<'a> {
    ctx: Option<&'a mut Context>,
    segments: &'a [&'a str],
    separator: &'a str,
    color: Option<Color>,
}

impl<'a> Breadcrumb<'a> {
    pub(super) fn new(ctx: &'a mut Context, segments: &'a [&'a str]) -> Self {
        Self {
            ctx: Some(ctx),
            segments,
            separator: " › ",
            color: None,
        }
    }

    /// Set the separator string between segments (default: ` › `).
    pub fn separator(mut self, sep: &'a str) -> Self {
        self.separator = sep;
        self
    }

    /// Override the link (clickable segment) color. Defaults to `theme.primary`.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Render now and return the [`BreadcrumbResponse`].
    pub fn show(mut self) -> BreadcrumbResponse {
        let ctx = self.ctx.take().expect("Breadcrumb::show called twice");
        render_breadcrumb(ctx, self.segments, self.separator, self.color)
    }
}

impl Drop for Breadcrumb<'_> {
    fn drop(&mut self) {
        if let Some(ctx) = self.ctx.take() {
            let _ = render_breadcrumb(ctx, self.segments, self.separator, self.color);
        }
    }
}

fn render_breadcrumb(
    ctx: &mut Context,
    segments: &[&str],
    separator: &str,
    color_override: Option<Color>,
) -> BreadcrumbResponse {
    let theme = ctx.theme;
    let last_idx = segments.len().saturating_sub(1);
    let mut clicked_segment: Option<usize> = None;
    let link_color = color_override.unwrap_or(theme.primary);

    let response = ctx.row(|ui| {
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
                    link_color
                };
                ui.text(*segment).fg(color).underline();
                if activated {
                    clicked_segment = Some(i);
                }
                ui.text(separator).dim();
            }
        }
    });

    BreadcrumbResponse {
        response,
        clicked_segment,
    }
}
