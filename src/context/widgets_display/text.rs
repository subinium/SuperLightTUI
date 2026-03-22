use super::*;
use crate::KeyMap;

impl Context {
    /// Render a text element. Returns `&mut Self` for style chaining.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// ui.text("hello").bold().fg(Color::Cyan);
    /// # });
    /// ```
    pub fn text(&mut self, s: impl Into<String>) -> &mut Self {
        let content = s.into();
        let default_fg = self
            .text_color_stack
            .iter()
            .rev()
            .find_map(|c| *c)
            .unwrap_or(self.theme.text);
        self.commands.push(Command::Text {
            content,
            cursor_offset: None,
            style: Style::new().fg(default_fg),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render a clickable hyperlink.
    ///
    /// The link is interactive: clicking it (or pressing Enter/Space when
    /// focused) opens the URL in the system browser. OSC 8 is also emitted
    /// for terminals that support native hyperlinks.
    #[allow(clippy::print_stderr)]
    pub fn link(&mut self, text: impl Into<String>, url: impl Into<String>) -> &mut Self {
        let url_str = url.into();
        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
        let response = self.response_for(interaction_id);

        let mut activated = response.clicked;
        if focused {
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        activated = true;
                        self.consumed[i] = true;
                    }
                }
            }
        }

        if activated {
            if let Err(e) = open_url(&url_str) {
                eprintln!("[slt] failed to open URL: {e}");
            }
        }

        let style = if focused {
            Style::new()
                .fg(self.theme.primary)
                .bg(self.theme.surface_hover)
                .underline()
                .bold()
        } else if response.hovered {
            Style::new()
                .fg(self.theme.accent)
                .bg(self.theme.surface_hover)
                .underline()
        } else {
            Style::new().fg(self.theme.primary).underline()
        };

        self.commands.push(Command::Link {
            text: text.into(),
            url: url_str,
            style,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render a text element with word-boundary wrapping.
    ///
    /// Long lines are broken at word boundaries to fit the container width.
    /// Style chaining works the same as [`Context::text`].
    ///
    /// **Prefer** `ui.text("...").wrap()` — this method exists for convenience
    /// but the chaining form is more consistent with the rest of the API.
    #[deprecated(since = "0.15.4", note = "use ui.text(s).wrap() instead")]
    pub fn text_wrap(&mut self, s: impl Into<String>) -> &mut Self {
        let content = s.into();
        let default_fg = self
            .text_color_stack
            .iter()
            .rev()
            .find_map(|c| *c)
            .unwrap_or(self.theme.text);
        self.commands.push(Command::Text {
            content,
            cursor_offset: None,
            style: Style::new().fg(default_fg),
            grow: 0,
            align: Align::Start,
            wrap: true,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render an elapsed time display.
    ///
    /// Formats as `HH:MM:SS.CC` when hours are non-zero, otherwise `MM:SS.CC`.
    pub fn timer_display(&mut self, elapsed: std::time::Duration) -> &mut Self {
        let total_centis = elapsed.as_millis() / 10;
        let centis = total_centis % 100;
        let total_seconds = total_centis / 100;
        let seconds = total_seconds % 60;
        let minutes = (total_seconds / 60) % 60;
        let hours = total_seconds / 3600;

        let content = if hours > 0 {
            format!("{hours:02}:{minutes:02}:{seconds:02}.{centis:02}")
        } else {
            format!("{minutes:02}:{seconds:02}.{centis:02}")
        };

        self.commands.push(Command::Text {
            content,
            cursor_offset: None,
            style: Style::new().fg(self.theme.text),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render help bar from a KeyMap. Shows visible bindings as key-description pairs.
    pub fn help_from_keymap(&mut self, keymap: &KeyMap) -> Response {
        let pairs: Vec<(&str, &str)> = keymap
            .visible_bindings()
            .map(|binding| (binding.display.as_str(), binding.description.as_str()))
            .collect();
        self.help(&pairs)
    }

    // ── style chain (applies to last text) ───────────────────────────

    /// Apply bold to the last rendered text element.
    pub fn bold(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::BOLD);
        self
    }

    /// Apply dim styling to the last rendered text element.
    ///
    /// Also sets the foreground color to the theme's `text_dim` color if no
    /// explicit foreground has been set.
    pub fn dim(&mut self) -> &mut Self {
        let text_dim = self.theme.text_dim;
        self.modify_last_style(|s| {
            s.modifiers |= Modifiers::DIM;
            if s.fg.is_none() {
                s.fg = Some(text_dim);
            }
        });
        self
    }

    /// Apply italic to the last rendered text element.
    pub fn italic(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::ITALIC);
        self
    }

    /// Apply underline to the last rendered text element.
    pub fn underline(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::UNDERLINE);
        self
    }

    /// Apply reverse-video to the last rendered text element.
    pub fn reversed(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::REVERSED);
        self
    }

    /// Apply strikethrough to the last rendered text element.
    pub fn strikethrough(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::STRIKETHROUGH);
        self
    }

    /// Set the foreground color of the last rendered text element.
    pub fn fg(&mut self, color: Color) -> &mut Self {
        self.modify_last_style(|s| s.fg = Some(color));
        self
    }

    /// Set the background color of the last rendered text element.
    pub fn bg(&mut self, color: Color) -> &mut Self {
        self.modify_last_style(|s| s.bg = Some(color));
        self
    }

    /// Apply a per-character foreground gradient to the last rendered text.
    pub fn gradient(&mut self, from: Color, to: Color) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            let replacement = match &self.commands[idx] {
                Command::Text {
                    content,
                    style,
                    wrap,
                    align,
                    margin,
                    constraints,
                    ..
                } => {
                    let chars: Vec<char> = content.chars().collect();
                    let len = chars.len();
                    let denom = len.saturating_sub(1).max(1) as f32;
                    let segments = chars
                        .into_iter()
                        .enumerate()
                        .map(|(i, ch)| {
                            let mut seg_style = *style;
                            seg_style.fg = Some(from.blend(to, i as f32 / denom));
                            (ch.to_string(), seg_style)
                        })
                        .collect();

                    Some(Command::RichText {
                        segments,
                        wrap: *wrap,
                        align: *align,
                        margin: *margin,
                        constraints: *constraints,
                    })
                }
                _ => None,
            };

            if let Some(command) = replacement {
                self.commands[idx] = command;
            }
        }

        self
    }

    /// Set foreground color when the current group is hovered or focused.
    pub fn group_hover_fg(&mut self, color: Color) -> &mut Self {
        let apply_group_style = self
            .group_stack
            .last()
            .map(|name| self.is_group_hovered(name) || self.is_group_focused(name))
            .unwrap_or(false);
        if apply_group_style {
            self.modify_last_style(|s| s.fg = Some(color));
        }
        self
    }

    /// Set background color when the current group is hovered or focused.
    pub fn group_hover_bg(&mut self, color: Color) -> &mut Self {
        let apply_group_style = self
            .group_stack
            .last()
            .map(|name| self.is_group_hovered(name) || self.is_group_focused(name))
            .unwrap_or(false);
        if apply_group_style {
            self.modify_last_style(|s| s.bg = Some(color));
        }
        self
    }

    /// Render a text element with an explicit [`Style`] applied immediately.
    ///
    /// Equivalent to calling `text(s)` followed by style-chain methods, but
    /// more concise when you already have a `Style` value.
    pub fn styled(&mut self, s: impl Into<String>, style: Style) -> &mut Self {
        self.styled_with_cursor(s, style, None)
    }

    pub(crate) fn styled_with_cursor(
        &mut self,
        s: impl Into<String>,
        style: Style,
        cursor_offset: Option<usize>,
    ) -> &mut Self {
        self.commands.push(Command::Text {
            content: s.into(),
            cursor_offset,
            style,
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Enable word-boundary wrapping on the last rendered text element.
    pub fn wrap(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text { wrap, .. } = &mut self.commands[idx] {
                *wrap = true;
            }
        }
        self
    }

    /// Truncate the last rendered text with `…` when it exceeds its allocated width.
    /// Use with `.w()` to set a fixed width, or let the parent container constrain it.
    pub fn truncate(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text { truncate, .. } = &mut self.commands[idx] {
                *truncate = true;
            }
        }
        self
    }

    fn modify_last_style(&mut self, f: impl FnOnce(&mut Style)) {
        if let Some(idx) = self.last_text_idx {
            match &mut self.commands[idx] {
                Command::Text { style, .. } | Command::Link { style, .. } => f(style),
                _ => {}
            }
        }
    }

    fn modify_last_constraints(&mut self, f: impl FnOnce(&mut Constraints)) {
        if let Some(idx) = self.last_text_idx {
            match &mut self.commands[idx] {
                Command::Text { constraints, .. } | Command::Link { constraints, .. } => {
                    f(constraints)
                }
                _ => {}
            }
        }
    }

    fn modify_last_margin(&mut self, f: impl FnOnce(&mut Margin)) {
        if let Some(idx) = self.last_text_idx {
            match &mut self.commands[idx] {
                Command::Text { margin, .. } | Command::Link { margin, .. } => f(margin),
                _ => {}
            }
        }
    }

    // ── containers ───────────────────────────────────────────────────

    /// Set the flex-grow factor of the last rendered text element.
    ///
    /// A value of `1` causes the element to expand and fill remaining space
    /// along the main axis.
    pub fn grow(&mut self, value: u16) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text { grow, .. } = &mut self.commands[idx] {
                *grow = value;
            }
        }
        self
    }

    /// Set the text alignment of the last rendered text element.
    pub fn align(&mut self, align: Align) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text {
                align: text_align, ..
            } = &mut self.commands[idx]
            {
                *text_align = align;
            }
        }
        self
    }

    /// Center-align the last rendered text element horizontally.
    /// Shorthand for `.align(Align::Center)`. Requires the text to have
    /// a width constraint (via `.w()` or parent container) to be visible.
    pub fn text_center(&mut self) -> &mut Self {
        self.align(Align::Center)
    }

    /// Right-align the last rendered text element horizontally.
    /// Shorthand for `.align(Align::End)`.
    pub fn text_right(&mut self) -> &mut Self {
        self.align(Align::End)
    }

    // ── size constraints on last text/link ──────────────────────────

    /// Set a fixed width on the last rendered text or link element.
    ///
    /// Sets both `min_width` and `max_width` to `value`, making the element
    /// occupy exactly that many columns (padded with spaces or truncated).
    pub fn w(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| {
            c.min_width = Some(value);
            c.max_width = Some(value);
        });
        self
    }

    /// Set a fixed height on the last rendered text or link element.
    ///
    /// Sets both `min_height` and `max_height` to `value`.
    pub fn h(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| {
            c.min_height = Some(value);
            c.max_height = Some(value);
        });
        self
    }

    /// Set the minimum width on the last rendered text or link element.
    pub fn min_w(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| c.min_width = Some(value));
        self
    }

    /// Set the maximum width on the last rendered text or link element.
    pub fn max_w(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| c.max_width = Some(value));
        self
    }

    /// Set the minimum height on the last rendered text or link element.
    pub fn min_h(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| c.min_height = Some(value));
        self
    }

    /// Set the maximum height on the last rendered text or link element.
    pub fn max_h(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| c.max_height = Some(value));
        self
    }

    // ── margin on last text/link ────────────────────────────────────

    /// Set uniform margin on all sides of the last rendered text or link element.
    pub fn m(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| *m = Margin::all(value));
        self
    }

    /// Set horizontal margin (left + right) on the last rendered text or link.
    pub fn mx(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| {
            m.left = value;
            m.right = value;
        });
        self
    }

    /// Set vertical margin (top + bottom) on the last rendered text or link.
    pub fn my(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| {
            m.top = value;
            m.bottom = value;
        });
        self
    }

    /// Set top margin on the last rendered text or link element.
    pub fn mt(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| m.top = value);
        self
    }

    /// Set right margin on the last rendered text or link element.
    pub fn mr(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| m.right = value);
        self
    }

    /// Set bottom margin on the last rendered text or link element.
    pub fn mb(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| m.bottom = value);
        self
    }

    /// Set left margin on the last rendered text or link element.
    pub fn ml(&mut self, value: u32) -> &mut Self {
        self.modify_last_margin(|m| m.left = value);
        self
    }

    /// Render an invisible spacer that expands to fill available space.
    ///
    /// Useful for pushing siblings to opposite ends of a row or column.
    pub fn spacer(&mut self) -> &mut Self {
        self.commands.push(Command::Spacer { grow: 1 });
        self.last_text_idx = None;
        self
    }
}
