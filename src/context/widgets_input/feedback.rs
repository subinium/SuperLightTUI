use super::*;

impl Context {
    ///
    /// The spinner advances one frame per tick. Use [`SpinnerState::dots`] or
    /// [`SpinnerState::line`] to create the state, then chain style methods to
    /// color it.
    pub fn spinner(&mut self, state: &SpinnerState) -> &mut Self {
        self.styled(
            state.frame(self.tick).to_string(),
            Style::new().fg(self.theme.primary),
        )
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
    /// Step defaults to `span / 20.0`. Use [`Context::slider_with_step`] for an
    /// explicit step (e.g. integer volume controls).
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
        let span = (*range.end() - *range.start()).max(0.0);
        let step = if span > 0.0 { span / 20.0 } else { 0.0 };
        self.slider_inner(label, value, range, step)
    }

    /// Horizontal slider with an explicit step size.
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
    /// ui.slider_with_step("Volume", &mut volume, 0.0..=100.0, 1.0);
    /// # });
    /// ```
    pub fn slider_with_step(
        &mut self,
        label: &str,
        value: &mut f64,
        range: std::ops::RangeInclusive<f64>,
        step: f64,
    ) -> Response {
        self.slider_inner(label, value, range, step.max(0.0))
    }

    fn slider_inner(
        &mut self,
        label: &str,
        value: &mut f64,
        range: std::ops::RangeInclusive<f64>,
        step: f64,
    ) -> Response {
        let focused = self.register_focusable();
        let mut changed = false;

        let start = *range.start();
        let end = *range.end();
        let span = (end - start).max(0.0);

        *value = (*value).clamp(start, end);

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        if step > 0.0 {
                            let next = (*value - step).max(start);
                            if (next - *value).abs() > f64::EPSILON {
                                *value = next;
                                changed = true;
                            }
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if step > 0.0 {
                            let next = (*value + step).min(end);
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
        response
    }
}
