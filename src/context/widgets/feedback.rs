//! Feedback family — spinners, toasts, sliders, alerts, confirm prompts,
//! progress bars and gauges.
//!
//! State types ([`crate::widgets::ToastState`],
//! [`crate::widgets::SpinnerState`], [`crate::widgets::SliderOpts`],
//! [`crate::widgets::AlertLevel`], ...) live in `src/widgets/feedback.rs`.

use super::*;

mod gauge;

pub use gauge::{Gauge, LineGauge};

#[cfg(test)]
mod tests;

impl Context {
    /// Render an animated spinner.
    ///
    /// The spinner advances one frame per tick. Use [`SpinnerState::dots`] or
    /// [`SpinnerState::line`] to create the state.
    ///
    /// Returns a [`Response`] with `hovered` populated correctly so callers
    /// can attach tooltips or react to mouse interaction. Prior to v0.20.0
    /// this returned `&mut Self`; existing code that ignores the return value
    /// keeps compiling, though the `#[must_use]` attribute on `Response`
    /// surfaces a warning that nudges callers to handle interaction state.
    pub fn spinner(&mut self, state: &SpinnerState) -> Response {
        let response = self.interaction();
        self.styled(
            state.frame(self.tick).to_string(),
            Style::new().fg(self.theme.primary),
        );
        response
    }

    /// Render toast notifications. Calls `state.cleanup(tick)` automatically.
    ///
    /// Expired messages are removed before rendering. If there are no active
    /// messages, nothing is rendered and `self` is returned unchanged.
    pub fn toast(&mut self, state: &mut ToastState) -> &mut Self {
        state.cleanup(self.tick);
        if state.messages.is_empty() {
            return self;
        }

        self.skip_interaction_slot();
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
        for message in state.messages.iter().rev() {
            let color = match message.level {
                ToastLevel::Info => self.theme.primary,
                ToastLevel::Success => self.theme.success,
                ToastLevel::Warning => self.theme.warning,
                ToastLevel::Error => self.theme.error,
            };
            let mut line = String::with_capacity(4 + message.text.len());
            line.push_str("  ● ");
            line.push_str(&message.text);
            self.styled(line, Style::new().fg(color));
        }
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        self
    }

    /// Horizontal slider for numeric values.
    ///
    /// Step defaults to `span / 20.0`. Use [`Context::slider_with`] with
    /// [`SliderOpts`](crate::widgets::SliderOpts) for an explicit step.
    ///
    /// # Examples
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(80, 24).render(|ui| {
    /// let mut volume = 75.0_f64;
    /// let r = ui.slider("Volume", &mut volume, 0.0..=100.0);
    /// if r.changed { /* volume was adjusted */ }
    /// # });
    /// ```
    pub fn slider(
        &mut self,
        label: &str,
        value: &mut f64,
        range: std::ops::RangeInclusive<f64>,
    ) -> Response {
        self.slider_with(value, crate::widgets::SliderOpts::new(label, range))
    }

    /// Horizontal slider with options for label, range, and explicit step.
    ///
    /// Each Left/Right (or `h`/`l`) advances `value` by `step`. Use this when
    /// the default step (`span / 20`) is too coarse or too fine — for example
    /// integer counters need `step = 1.0`, fine controls need `step = 0.1`.
    ///
    /// # Examples
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(80, 24).render(|ui| {
    /// let mut volume = 50.0_f64;
    /// use slt::widgets::SliderOpts;
    /// ui.slider_with(&mut volume, SliderOpts::new("Volume", 0.0..=100.0).step(1.0));
    /// # });
    /// ```
    pub fn slider_with(&mut self, value: &mut f64, opts: crate::widgets::SliderOpts) -> Response {
        let (start, end) =
            crate::widgets::normalize_numeric_range(*opts.range.start(), *opts.range.end());
        let span = end - start;
        let step = opts
            .step
            .unwrap_or_else(|| if span > 0.0 { span / 20.0 } else { 0.0 });
        self.slider_inner(&opts.label, value, start..=end, step)
    }

    /// Horizontal slider with an explicit step size.
    ///
    /// Deprecated compatibility alias for [`Context::slider_with`].
    #[deprecated(
        since = "0.23.0",
        note = "use slider_with(value, SliderOpts::new(label, range).step(step))"
    )]
    pub fn slider_with_step(
        &mut self,
        label: &str,
        value: &mut f64,
        range: std::ops::RangeInclusive<f64>,
        step: f64,
    ) -> Response {
        self.slider_with(
            value,
            crate::widgets::SliderOpts::new(label, range).step(step),
        )
    }

    fn slider_inner(
        &mut self,
        label: &str,
        value: &mut f64,
        range: std::ops::RangeInclusive<f64>,
        step: f64,
    ) -> Response {
        let focused = self.register_focusable();
        // v0.21.1: capture focus-edge flags (issue #208 gap — slider assembled
        // its Response by hand and never set gained_focus/lost_focus).
        let (gained_focus, lost_focus) = self.focus_transitions(focused);
        let mut changed = false;

        let (start, end) = crate::widgets::normalize_numeric_range(*range.start(), *range.end());
        let span = end - start;
        let step = crate::widgets::normalize_numeric_step(step);

        *value = crate::widgets::normalize_numeric_value(*value, start, end);

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        if step > 0.0 {
                            let next =
                                crate::widgets::normalize_numeric_value(*value - step, start, end);
                            if (next - *value).abs() > f64::EPSILON {
                                *value = next;
                                changed = true;
                            }
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if step > 0.0 {
                            let next =
                                crate::widgets::normalize_numeric_value(*value + step, start, end);
                            if (next - *value).abs() > f64::EPSILON {
                                *value = next;
                                changed = true;
                            }
                        }
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
        }

        let ratio = if span <= f64::EPSILON {
            0.0
        } else {
            ((*value - start) / span).clamp(0.0, 1.0)
        };

        let value_text = format_compact_number(*value);
        let label_width = UnicodeWidthStr::width(label) as u32;
        let value_width = UnicodeWidthStr::width(value_text.as_str()) as u32;
        let track_width = self
            .area_width
            .saturating_sub(label_width + value_width + 8)
            .max(10) as usize;
        let thumb_idx = if track_width <= 1 {
            0
        } else {
            (ratio * (track_width as f64 - 1.0)).round() as usize
        };

        let mut track = String::with_capacity(track_width);
        for i in 0..track_width {
            if i == thumb_idx {
                track.push('○');
            } else if i < thumb_idx {
                track.push('█');
            } else {
                track.push('━');
            }
        }

        let text_color = self.theme.text;
        let border_color = self.theme.border;
        let primary_color = self.theme.primary;
        let dim_color = self.theme.text_dim;
        let mut response = self.container().row(|ui| {
            ui.text(label).fg(text_color);
            ui.text("[").fg(border_color);
            ui.text(track).grow(1).fg(primary_color);
            ui.text("]").fg(border_color);
            if focused {
                ui.text(value_text.as_str()).bold().fg(primary_color);
            } else {
                ui.text(value_text.as_str()).fg(dim_color);
            }
        });
        response.focused = focused;
        response.changed = changed;
        response.gained_focus = gained_focus;
        response.lost_focus = lost_focus;
        response
    }

    /// Render an alert banner with icon and level-based coloring.
    ///
    /// Argument order is `(message, level)` — message first, then the
    /// [`AlertLevel`](crate::widgets::AlertLevel). This is the executable
    /// proof that [API_DESIGN.md](https://github.com/subinium/superlighttui/blob/main/docs/API_DESIGN.md)
    /// Rule 3 matches the shipped signature.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::AlertLevel;
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.alert("Disk full", AlertLevel::Error);
    /// ui.alert("Saved", AlertLevel::Success);
    /// # });
    /// ```
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
        if !clicked && let Some((mx, my)) = self.click_pos {
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

    /// Render a progress bar (20 chars wide). `ratio` is clamped to `0.0..=1.0`.
    ///
    /// Uses block characters (`█` filled, `░` empty). For a custom width use
    /// [`Context::progress_bar`]. For an inline label use [`Context::gauge`].
    ///
    /// Returns a [`Response`] so callers can detect hover, attach a tooltip,
    /// or implement click-to-set scrubbers. Prior to v0.20.0 this returned
    /// `&mut Self`; ignoring the return value still compiles but the
    /// `#[must_use]` attribute on `Response` warns at the call site.
    pub fn progress(&mut self, ratio: f64) -> Response {
        self.progress_bar(ratio, 20)
    }

    /// Render a progress bar with a custom character width.
    ///
    /// `ratio` is clamped to `0.0..=1.0`. `width` is the total number of
    /// characters rendered.
    pub fn progress_bar(&mut self, ratio: f64, width: u32) -> Response {
        self.progress_bar_colored(ratio, width, self.theme.primary)
    }

    /// Render a progress bar with a custom fill color.
    pub fn progress_bar_colored(&mut self, ratio: f64, width: u32, color: Color) -> Response {
        let response = self.interaction();
        let clamped = ratio.clamp(0.0, 1.0);
        let filled = (clamped * width as f64).round() as u32;
        let empty = width.saturating_sub(filled);
        let mut bar = String::with_capacity(width as usize * 3);
        for _ in 0..filled {
            bar.push('█');
        }
        for _ in 0..empty {
            bar.push('░');
        }
        self.styled(bar, Style::new().fg(color));
        response
    }
}
