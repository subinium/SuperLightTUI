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
        let default_fg = self.inherited_text_fg();
        self.commands.push(Command::Text {
            content,
            cursor_offset: None,
            cursor_masked: false,
            style: Style::new().fg(default_fg),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = Some(self.commands.len() - 1);
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
        let (_interaction_id, response) = self.begin_widget_interaction(focused);

        let activated = response.clicked || self.consume_activation_keys(focused);

        if activated && let Err(e) = open_url(&url_str) {
            eprintln!("[slt] failed to open URL: {e}");
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
            wrap: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = Some(self.commands.len() - 1);
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
            cursor_masked: false,
            style: Style::new().fg(self.theme.text),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = Some(self.commands.len() - 1);
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
        let inherited_fg = self.inherited_text_fg();
        if let Some(idx) = self.rollback.last_text_idx {
            match &mut self.commands[idx] {
                Command::Text { style, .. } => {
                    style.modifiers |= Modifiers::DIM;
                    if style.fg.is_none() || style.fg == Some(inherited_fg) {
                        style.fg = Some(text_dim);
                    }
                }
                Command::Link { style, .. } => {
                    style.modifiers |= Modifiers::DIM;
                }
                Command::RichText { segments, .. } => {
                    let all_inherited = segments
                        .iter()
                        .all(|(_, style)| style.fg.is_none() || style.fg == Some(inherited_fg));
                    for (_, style) in segments {
                        style.modifiers |= Modifiers::DIM;
                        if all_inherited {
                            style.fg = Some(text_dim);
                        }
                    }
                }
                _ => {}
            }
        }
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
        self.apply_char_gradient(false, |t| to.blend_f64(from, t));
        self
    }

    /// Apply a per-character multi-stop foreground gradient to the last text.
    ///
    /// `stops` is a slice of `(position, color)` pairs where `position` lies in
    /// `0.0..=1.0`. Stops do not need to be pre-sorted. The text is colored by
    /// linearly interpolating between adjacent stops across its displayed
    /// columns, using the same column-mapping and clamping as [`gradient`].
    ///
    /// - An empty slice is a no-op (the text keeps its current style).
    /// - A single stop produces a solid color.
    ///
    /// [`gradient`]: Self::gradient
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// ui.text("rainbow").gradient_stops_f64(&[
    ///     (0.0, Color::Red),
    ///     (0.5, Color::Yellow),
    ///     (1.0, Color::Green),
    /// ]);
    /// # });
    /// ```
    pub fn gradient_stops_f64(&mut self, stops: &[(f64, Color)]) -> &mut Self {
        if stops.is_empty() {
            return self;
        }
        let sorted = Self::sorted_gradient_stops(stops);
        self.apply_char_gradient(false, |t| Self::sample_gradient_stops(&sorted, t));
        self
    }

    /// Deprecated `f32` alias for [`gradient_stops_f64`](Self::gradient_stops_f64).
    #[deprecated(
        since = "0.22.2",
        note = "use Context::gradient_stops_f64() to keep public float APIs on f64"
    )]
    pub fn gradient_stops(&mut self, stops: &[(f32, Color)]) -> &mut Self {
        let stops: Vec<(f64, Color)> = stops
            .iter()
            .map(|(pos, color)| (f64::from(*pos), *color))
            .collect();
        self.gradient_stops_f64(&stops)
    }

    /// Apply a per-character background gradient to the last rendered text.
    ///
    /// The two-stop background analogue of [`gradient`]. Colors the cell
    /// background instead of the foreground, using identical column-mapping and
    /// clamping so width handling stays consistent.
    ///
    /// [`gradient`]: Self::gradient
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// ui.text("banner").bg_gradient(Color::Blue, Color::Magenta);
    /// # });
    /// ```
    pub fn bg_gradient(&mut self, from: Color, to: Color) -> &mut Self {
        self.apply_char_gradient(true, |t| to.blend_f64(from, t));
        self
    }

    /// Apply a per-character multi-stop background gradient to the last text.
    ///
    /// The background analogue of [`gradient_stops`]: identical stop handling
    /// (positions in `0.0..=1.0`, unsorted-safe, empty = no-op, single stop =
    /// solid) but applied to the cell background instead of the foreground.
    ///
    /// [`gradient_stops`]: Self::gradient_stops
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// ui.text("header").bg_gradient_stops_f64(&[
    ///     (0.0, Color::Blue),
    ///     (1.0, Color::Magenta),
    /// ]);
    /// # });
    /// ```
    pub fn bg_gradient_stops_f64(&mut self, stops: &[(f64, Color)]) -> &mut Self {
        if stops.is_empty() {
            return self;
        }
        let sorted = Self::sorted_gradient_stops(stops);
        self.apply_char_gradient(true, |t| Self::sample_gradient_stops(&sorted, t));
        self
    }

    /// Deprecated `f32` alias for [`bg_gradient_stops_f64`](Self::bg_gradient_stops_f64).
    #[deprecated(
        since = "0.22.2",
        note = "use Context::bg_gradient_stops_f64() to keep public float APIs on f64"
    )]
    pub fn bg_gradient_stops(&mut self, stops: &[(f32, Color)]) -> &mut Self {
        let stops: Vec<(f64, Color)> = stops
            .iter()
            .map(|(pos, color)| (f64::from(*pos), *color))
            .collect();
        self.bg_gradient_stops_f64(&stops)
    }

    /// Return `stops` sorted ascending by clamped position. Positions are
    /// clamped into `0.0..=1.0` so out-of-range inputs degrade gracefully.
    fn sorted_gradient_stops(stops: &[(f64, Color)]) -> Vec<(f64, Color)> {
        let mut sorted: Vec<(f64, Color)> = stops
            .iter()
            .map(|(pos, color)| {
                let pos = if pos.is_finite() {
                    pos.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (pos, *color)
            })
            .collect();
        sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
        sorted
    }

    /// Sample the color at position `t` (in `0.0..=1.0`) from pre-sorted,
    /// non-empty `stops`, linearly interpolating between the bracketing stops.
    fn sample_gradient_stops(stops: &[(f64, Color)], t: f64) -> Color {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        // Non-empty is guaranteed by callers; fall back defensively otherwise.
        let first = match stops.first() {
            Some(stop) => *stop,
            None => return Color::Rgb(0, 0, 0),
        };
        let last = *stops.last().unwrap_or(&first);
        if t <= first.0 {
            return first.1;
        }
        if t >= last.0 {
            return last.1;
        }
        for window in stops.windows(2) {
            let (p0, c0) = window[0];
            let (p1, c1) = window[1];
            if t >= p0 && t <= p1 {
                let span = p1 - p0;
                if span <= f64::EPSILON {
                    return c1;
                }
                let local = (t - p0) / span;
                return c1.blend_f64(c0, local);
            }
        }
        last.1
    }

    /// Replace the last `Text` command with a `RichText` gradient, mapping each
    /// grapheme's starting cell to a position in `0.0..=1.0` exactly like
    /// [`gradient`](Self::gradient). `is_bg` selects background vs foreground.
    fn apply_char_gradient(&mut self, is_bg: bool, color_at: impl Fn(f64) -> Color) {
        if let Some(idx) = self.rollback.last_text_idx {
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
                    let graphemes: Vec<&str> = content.graphemes(true).collect();
                    let last_start = graphemes
                        .iter()
                        .take(graphemes.len().saturating_sub(1))
                        .map(|grapheme| UnicodeWidthStr::width(*grapheme))
                        .sum::<usize>();
                    let denom = last_start.max(1) as f64;
                    let mut cell = 0usize;
                    let segments = graphemes
                        .into_iter()
                        .map(|grapheme| {
                            let mut seg_style = *style;
                            let color = color_at(cell as f64 / denom);
                            if is_bg {
                                seg_style.bg = Some(color);
                            } else {
                                seg_style.fg = Some(color);
                            }
                            cell = cell.saturating_add(UnicodeWidthStr::width(grapheme));
                            (grapheme.to_string(), seg_style)
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
    }

    /// Set foreground color when the current group is hovered or focused.
    pub fn group_hover_fg(&mut self, color: Color) -> &mut Self {
        let apply_group_style = self
            .rollback
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
            .rollback
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
        self.styled_with_cursor_privacy(s, style, cursor_offset, false)
    }

    pub(crate) fn styled_with_cursor_privacy(
        &mut self,
        s: impl Into<String>,
        style: Style,
        cursor_offset: Option<usize>,
        cursor_masked: bool,
    ) -> &mut Self {
        self.commands.push(Command::Text {
            content: s.into(),
            cursor_offset,
            cursor_masked,
            style,
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Enable word-boundary wrapping on the last rendered text element.
    pub fn wrap(&mut self) -> &mut Self {
        if let Some(idx) = self.rollback.last_text_idx {
            match &mut self.commands[idx] {
                Command::Text { wrap, .. }
                | Command::Link { wrap, .. }
                | Command::RichText { wrap, .. } => *wrap = true,
                _ => {}
            }
        }
        self
    }

    /// Truncate the last rendered text with `…` when it exceeds its allocated width.
    /// Use with `.w()` to set a fixed width, or let the parent container constrain it.
    pub fn truncate(&mut self) -> &mut Self {
        if let Some(idx) = self.rollback.last_text_idx
            && let Command::Text { truncate, .. } = &mut self.commands[idx]
        {
            *truncate = true;
        }
        self
    }

    fn modify_last_style(&mut self, mut f: impl FnMut(&mut Style)) {
        if let Some(idx) = self.rollback.last_text_idx {
            match &mut self.commands[idx] {
                Command::Text { style, .. } | Command::Link { style, .. } => f(style),
                Command::RichText { segments, .. } => {
                    for (_, style) in segments {
                        f(style);
                    }
                }
                _ => {}
            }
        }
    }

    fn modify_last_constraints(&mut self, f: impl FnOnce(&mut Constraints)) {
        if let Some(idx) = self.rollback.last_text_idx {
            match &mut self.commands[idx] {
                Command::Text { constraints, .. } | Command::Link { constraints, .. } => {
                    f(constraints)
                }
                Command::RichText { constraints, .. } => f(constraints),
                _ => {}
            }
        }
    }

    fn modify_last_margin(&mut self, f: impl FnOnce(&mut Margin)) {
        if let Some(idx) = self.rollback.last_text_idx {
            match &mut self.commands[idx] {
                Command::Text { margin, .. } | Command::Link { margin, .. } => f(margin),
                Command::RichText { margin, .. } => f(margin),
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
        if let Some(idx) = self.rollback.last_text_idx
            && let Command::Text { grow, .. } = &mut self.commands[idx]
        {
            *grow = value;
        }
        self
    }

    /// Set the text alignment of the last rendered text element.
    pub fn align(&mut self, align: Align) -> &mut Self {
        if let Some(idx) = self.rollback.last_text_idx {
            match &mut self.commands[idx] {
                Command::Text {
                    align: text_align, ..
                }
                | Command::RichText {
                    align: text_align, ..
                } => *text_align = align,
                _ => {}
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
    /// Sets the [`WidthSpec`](crate::WidthSpec) to `Fixed(value)`, making the
    /// element occupy exactly that many columns (padded with spaces or
    /// truncated).
    pub fn w(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| {
            *c = c.w(value);
        });
        self
    }

    /// Set a fixed height on the last rendered text or link element.
    ///
    /// Sets the [`HeightSpec`](crate::HeightSpec) to `Fixed(value)`.
    pub fn h(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| {
            *c = c.h(value);
        });
        self
    }

    /// Set the minimum width on the last rendered text or link element.
    pub fn min_w(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| c.set_min_width(Some(value)));
        self
    }

    /// Set the maximum width on the last rendered text or link element.
    pub fn max_w(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| c.set_max_width(Some(value)));
        self
    }

    /// Set the minimum height on the last rendered text or link element.
    pub fn min_h(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| c.set_min_height(Some(value)));
        self
    }

    /// Set the maximum height on the last rendered text or link element.
    pub fn max_h(&mut self, value: u32) -> &mut Self {
        self.modify_last_constraints(|c| c.set_max_height(Some(value)));
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
        self.rollback.last_text_idx = None;
        self
    }

    // ── conditional / grouped style helpers ─────────────────────────

    /// Apply `f` only if `cond` is true. Returns `self` so chaining continues.
    ///
    /// Use this to attach a block of style modifiers to the last rendered text
    /// without breaking the fluent chain. The closure receives the same
    /// `&mut Context`, so any style-chain method (`.bold()`, `.fg()`, etc.)
    /// applies to the most recent text element.
    ///
    /// Zero allocation: the closure is inlined and skipped entirely when
    /// `cond` is `false`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// let is_error = true;
    /// let is_selected = false;
    /// ui.text("Status")
    ///     .with_if(is_error, |t| {
    ///         t.bold().fg(Color::Red);
    ///     })
    ///     .with_if(is_selected, |t| {
    ///         t.bg(Color::DarkGray);
    ///     });
    /// # });
    /// ```
    pub fn with_if(&mut self, cond: bool, f: impl FnOnce(&mut Self)) -> &mut Self {
        if cond {
            f(self);
        }
        self
    }

    /// Apply `f` unconditionally. Useful for factoring out a block of modifier
    /// calls that should always run, while keeping the fluent chain intact.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// ui.text("hi").with(|t| {
    ///     t.bold().fg(Color::Cyan);
    /// });
    /// # });
    /// ```
    pub fn with(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        f(self);
        self
    }

    fn inherited_text_fg(&self) -> Color {
        self.rollback
            .text_color_stack
            .iter()
            .rev()
            .find_map(|color| *color)
            .unwrap_or(self.theme.text)
    }
}

#[cfg(test)]
mod gradient_tests {
    use super::*;
    use crate::TestBackend;

    #[test]
    fn gradient_stops_interpolates_fg_across_columns() {
        let red = Color::Rgb(255, 0, 0);
        let blue = Color::Rgb(0, 0, 255);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("ABC")
                .gradient_stops_f64(&[(0.0, red), (1.0, blue)]);
        });

        let buf = backend.buffer();
        // i=0 → t=0 → stop at 0.0 (red); i=2 → t=1 → stop at 1.0 (blue);
        // i=1 → t=0.5 → halfway blend.
        assert_eq!(
            buf.get(0, 0).style.fg,
            Some(red),
            "first column should be red"
        );
        assert_eq!(
            buf.get(1, 0).style.fg,
            Some(Color::Rgb(128, 0, 128)),
            "middle column should be the halfway blend"
        );
        assert_eq!(
            buf.get(2, 0).style.fg,
            Some(blue),
            "last column should be blue"
        );
    }

    #[test]
    fn two_stop_gradient_uses_documented_endpoint_order() {
        let red = Color::Rgb(255, 0, 0);
        let blue = Color::Rgb(0, 0, 255);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("ABC").gradient(red, blue);
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.fg, Some(red));
        assert_eq!(buf.get(1, 0).style.fg, Some(Color::Rgb(128, 0, 128)));
        assert_eq!(buf.get(2, 0).style.fg, Some(blue));
    }

    #[test]
    fn gradient_stops_unsorted_input_is_sorted() {
        let red = Color::Rgb(255, 0, 0);
        let blue = Color::Rgb(0, 0, 255);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            // Deliberately out of order — must behave identically to sorted.
            ui.text("ABC")
                .gradient_stops_f64(&[(1.0, blue), (0.0, red)]);
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.fg, Some(red));
        assert_eq!(buf.get(2, 0).style.fg, Some(blue));
    }

    #[test]
    fn gradient_stops_multi_stop_hits_middle_stop_exactly() {
        let red = Color::Rgb(255, 0, 0);
        let green = Color::Rgb(0, 255, 0);
        let blue = Color::Rgb(0, 0, 255);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            // len=3, denom=2 → columns map to t = 0.0, 0.5, 1.0.
            ui.text("ABC")
                .gradient_stops_f64(&[(0.0, red), (0.5, green), (1.0, blue)]);
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.fg, Some(red), "t=0 → first stop");
        assert_eq!(
            buf.get(1, 0).style.fg,
            Some(green),
            "t=0.5 → middle stop exactly"
        );
        assert_eq!(buf.get(2, 0).style.fg, Some(blue), "t=1 → last stop");
    }

    #[test]
    fn gradient_stops_single_stop_is_solid() {
        let cyan = Color::Rgb(0, 200, 200);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("ABCD").gradient_stops_f64(&[(0.0, cyan)]);
        });

        let buf = backend.buffer();
        for x in 0..4 {
            assert_eq!(
                buf.get(x, 0).style.fg,
                Some(cyan),
                "every column should be the single solid stop"
            );
        }
    }

    #[test]
    fn gradient_stops_empty_is_noop() {
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            // Empty slice must not panic and must leave content intact.
            ui.text("HELLO").gradient_stops_f64(&[]);
        });

        backend.assert_contains("HELLO");
    }

    #[test]
    fn bg_gradient_applies_to_background() {
        let red = Color::Rgb(255, 0, 0);
        let blue = Color::Rgb(0, 0, 255);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("ABC").bg_gradient(red, blue);
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.bg, Some(red), "first column bg = from");
        assert_eq!(buf.get(2, 0).style.bg, Some(blue), "last column bg = to");
        assert_eq!(
            buf.get(1, 0).style.bg,
            Some(Color::Rgb(128, 0, 128)),
            "middle column bg = halfway blend"
        );
    }

    #[test]
    fn bg_gradient_stops_interpolates_background() {
        let red = Color::Rgb(255, 0, 0);
        let blue = Color::Rgb(0, 0, 255);
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("ABC")
                .bg_gradient_stops_f64(&[(0.0, red), (1.0, blue)]);
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.bg, Some(red), "first column bg = red");
        assert_eq!(
            buf.get(1, 0).style.bg,
            Some(Color::Rgb(128, 0, 128)),
            "middle column bg = halfway blend"
        );
        assert_eq!(buf.get(2, 0).style.bg, Some(blue), "last column bg = blue");
    }

    #[test]
    fn bg_gradient_stops_empty_is_noop() {
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("WORLD").bg_gradient_stops_f64(&[]);
        });

        backend.assert_contains("WORLD");
    }

    #[test]
    fn style_and_constraints_after_gradient_update_rich_text() {
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("ABC")
                .gradient(Color::Red, Color::Blue)
                .bold()
                .fg(Color::Green)
                .bg(Color::Black)
                .w(8)
                .m(1)
                .align(Align::End);
        });

        let buf = backend.buffer();
        let (start_x, y) = backend.find_text("ABC").expect("rendered gradient text");
        for x in start_x..start_x + 3 {
            let cell = buf.get(x, y);
            assert_eq!(cell.style.fg, Some(Color::Green));
            assert_eq!(cell.style.bg, Some(Color::Black));
            assert!(cell.style.modifiers.contains(Modifiers::BOLD));
        }
    }

    #[test]
    fn dim_uses_theme_color_only_for_inherited_foreground() {
        let theme = Theme::dark();
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.set_theme(theme);
            ui.text("inherited").dim();
            ui.text("explicit").fg(Color::Red).dim();
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.fg, Some(theme.text_dim));
        assert_eq!(buf.get(0, 1).style.fg, Some(Color::Red));
        assert!(buf.get(0, 0).style.modifiers.contains(Modifiers::DIM));
        assert!(buf.get(0, 1).style.modifiers.contains(Modifiers::DIM));
    }

    #[test]
    fn gradient_positions_follow_grapheme_cell_width() {
        let mut backend = TestBackend::new(20, 4);
        backend.render(|ui| {
            ui.text("界A").gradient(Color::Red, Color::Blue);
        });

        let buf = backend.buffer();
        assert_eq!(buf.get(0, 0).style.fg, Some(Color::Red));
        assert_eq!(buf.get(2, 0).style.fg, Some(Color::Blue));
    }
}
