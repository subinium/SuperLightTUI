use super::*;

impl Context {
    /// Render a help bar showing keybinding hints.
    ///
    /// `bindings` is a slice of `(key, action)` pairs. Keys are rendered in the
    /// theme's primary color; actions in the dim text color. Pairs are separated
    /// by a `·` character.
    pub fn help(&mut self, bindings: &[(&str, &str)]) -> Response {
        if bindings.is_empty() {
            return Response::none();
        }

        self.skip_interaction_slot();
        let help_gap = self.theme.spacing.sm();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Row,
                gap: help_gap as i32,
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
        for (idx, (key, action)) in bindings.iter().enumerate() {
            if idx > 0 {
                self.styled("·", Style::new().fg(self.theme.text_dim));
            }
            self.styled(*key, Style::new().bold().fg(self.theme.primary));
            self.styled(*action, Style::new().fg(self.theme.text_dim));
        }
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        Response::none()
    }

    /// Render a help bar with custom key/description colors.
    pub fn help_colored(
        &mut self,
        bindings: &[(&str, &str)],
        key_color: Color,
        text_color: Color,
    ) -> Response {
        if bindings.is_empty() {
            return Response::none();
        }

        self.skip_interaction_slot();
        let help_gap = self.theme.spacing.sm();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Row,
                gap: help_gap as i32,
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
        for (idx, (key, action)) in bindings.iter().enumerate() {
            if idx > 0 {
                self.styled("·", Style::new().fg(text_color));
            }
            self.styled(*key, Style::new().bold().fg(key_color));
            self.styled(*action, Style::new().fg(text_color));
        }
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        Response::none()
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// # let mut open = true;
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.accordion("Advanced", &mut open, |ui| {
    ///     ui.text("Expert settings");
    /// });
    /// # });
    /// ```
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
    ///
    /// Keys are right-padded to the widest key so the value column lines up.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.definition_list(&[
    ///     ("Name", "SuperLightTUI"),
    ///     ("Version", "0.21.1"),
    ///     ("License", "MIT"),
    /// ]);
    /// # });
    /// ```
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
                    padded.extend(std::iter::repeat_n(' ', pad));
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
    ///
    /// The label is padded with one space on each side and centered between
    /// two `─` separator runs spanning the available width.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.divider_text("Settings");
    /// # });
    /// ```
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
    /// Returns a [`Response`] carrying real `hovered` / `right_clicked` state
    /// for the badge's rect, so callers can attach `.on_hover(...)` tooltips.
    /// Prior to v0.21.0 this always returned [`Response::none()`]; statement-form
    /// callers (`ui.badge("NEW");`) compile unchanged.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let r = ui.badge("NEW");
    /// if r.hovered { /* attach a tooltip */ }
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
    /// Returns a [`Response`] carrying real `hovered` / `right_clicked` state
    /// for the badge's rect, so callers can attach `.on_hover(...)` tooltips.
    /// Prior to v0.21.0 this always returned [`Response::none()`]; statement-form
    /// callers compile unchanged.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::Color;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let r = ui.badge_colored("ALPHA", Color::Magenta);
    /// if r.hovered { /* attach a tooltip */ }
    /// # });
    /// ```
    pub fn badge_colored(&mut self, label: &str, color: Color) -> Response {
        let fg = Color::contrast_fg(color);
        let mut label_text = String::with_capacity(label.len() + 2);
        label_text.push(' ');
        label_text.push_str(label);
        label_text.push(' ');
        // Reserve the interaction slot *before* the text so the marker
        // attaches to the badge's rect (same pattern as `spinner` / `gauge`).
        let response = self.interaction();
        self.text(label_text).fg(fg).bg(color);

        response
    }

    /// Render a keyboard shortcut hint with reversed styling.
    ///
    /// Returns a [`Response`] carrying real `hovered` / `right_clicked` state
    /// for the hint's rect, so callers can attach `.on_hover(...)` tooltips.
    /// Prior to v0.21.0 this always returned [`Response::none()`]; statement-form
    /// callers compile unchanged.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.line(|ui| {
    ///     ui.text("Quit: ");
    ///     let r = ui.key_hint("Ctrl+Q");
    ///     if r.hovered { /* attach a tooltip */ }
    /// });
    /// # });
    /// ```
    pub fn key_hint(&mut self, key: &str) -> Response {
        let theme = self.theme;
        let mut key_text = String::with_capacity(key.len() + 2);
        key_text.push(' ');
        key_text.push_str(key);
        key_text.push(' ');
        // Reserve the interaction slot *before* the text so the marker
        // attaches to the hint's rect.
        let response = self.interaction();
        self.text(key_text).reversed().fg(theme.text_dim);

        response
    }

    /// Render a label-value stat pair.
    ///
    /// Renders as a column: a dim label above a bold value. Pair multiple
    /// stats in a [`row`](Self::row) for a compact dashboard strip.
    ///
    /// Returns a [`Response`] carrying real `hovered` / `clicked` /
    /// `right_clicked` state for the stat's column rect, so callers can attach
    /// `.on_hover(...)` tooltips. Prior to v0.21.0 this always returned
    /// [`Response::none()`]; statement-form callers compile unchanged.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.row(|ui| {
    ///     let r = ui.stat("Users", "1.2k");
    ///     if r.hovered { /* attach a tooltip */ }
    ///     ui.stat("Revenue", "$8,420");
    /// });
    /// # });
    /// ```
    pub fn stat(&mut self, label: &str, value: &str) -> Response {
        self.col(|ui| {
            ui.text(label).dim();
            ui.text(value).bold();
        })
    }

    /// Render a stat pair with a custom value color.
    ///
    /// Returns a [`Response`] carrying real `hovered` / `clicked` /
    /// `right_clicked` state for the stat's column rect, so callers can attach
    /// `.on_hover(...)` tooltips. Prior to v0.21.0 this always returned
    /// [`Response::none()`]; statement-form callers compile unchanged.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::Color;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let r = ui.stat_colored("Errors", "0", Color::Green);
    /// if r.hovered { /* attach a tooltip */ }
    /// # });
    /// ```
    pub fn stat_colored(&mut self, label: &str, value: &str, color: Color) -> Response {
        self.col(|ui| {
            ui.text(label).dim();
            ui.text(value).bold().fg(color);
        })
    }

    /// Render a stat pair with an up/down trend arrow.
    ///
    /// The arrow color follows the theme: `success` for [`Trend::Up`],
    /// `error` for [`Trend::Down`].
    ///
    /// Returns a [`Response`] carrying real `hovered` / `clicked` /
    /// `right_clicked` state for the stat's column rect, so callers can attach
    /// `.on_hover(...)` tooltips. Prior to v0.21.0 this always returned
    /// [`Response::none()`]; statement-form callers compile unchanged.
    ///
    /// [`Trend::Up`]: crate::widgets::Trend::Up
    /// [`Trend::Down`]: crate::widgets::Trend::Down
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::Trend;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let r = ui.stat_trend("MRR", "$24.5k", Trend::Up);
    /// if r.hovered { /* attach a tooltip */ }
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
        self.col(|ui| {
            ui.text(label).dim();
            ui.line(|ui| {
                ui.text(value).bold();
                let mut arrow_text = String::with_capacity(1 + arrow.len());
                arrow_text.push(' ');
                arrow_text.push_str(arrow);
                ui.text(arrow_text).fg(color);
            });
        })
    }

    /// Render a centered empty-state placeholder.
    ///
    /// Title is rendered prominently; description is dimmed below. Both are
    /// centered horizontally and vertically inside the available space.
    ///
    /// Returns a [`Response`] carrying real `hovered` / `clicked` /
    /// `right_clicked` state for the placeholder rect, so callers can attach
    /// `.on_hover(...)` tooltips. Prior to v0.21.0 this always returned
    /// [`Response::none()`]; statement-form callers compile unchanged.
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
        self.container().center().col(|ui| {
            ui.text(title).align(Align::Center);
            ui.text(description).dim().align(Align::Center);
        })
    }

    /// Render a centered empty-state placeholder with an action button.
    ///
    /// Returns a [`Response`] whose `clicked` field is `true` on the frame
    /// the action button is activated. As of v0.21.0 the response also carries
    /// real `hovered` / `right_clicked` state (and the laid-out `rect`) for the
    /// placeholder area, so callers can attach `.on_hover(...)` tooltips. The
    /// `clicked` / `changed` fields still track the action button specifically,
    /// not the whole placeholder.
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
        // The container response carries hover / right-click / rect for the
        // whole placeholder area; `clicked` still tracks the action button.
        let mut response = self.container().center().col(|ui| {
            ui.text(title).align(Align::Center);
            ui.text(description).dim().align(Align::Center);
            if ui.button(action_label).clicked {
                clicked = true;
            }
        });

        response.clicked = clicked;
        response.changed = clicked;
        response
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
        let Some(ctx) = self.ctx.take() else {
            // `show` consumes the builder, so safe code cannot reach this
            // branch. Stay defensive if the internal invariant changes.
            return BreadcrumbResponse::default();
        };
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
