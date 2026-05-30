use super::*;

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
        // v0.21.1: capture focus-edge flags (issue #208 gap — slider assembled
        // its Response by hand and never set gained_focus/lost_focus).
        let (gained_focus, lost_focus) = self.focus_transitions(focused);
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
        response.gained_focus = gained_focus;
        response.lost_focus = lost_focus;
        response
    }

    /// Numeric stepper field: Up/Down (or `k`/`j`) and scroll-wheel adjust by
    /// `step`, or type a value directly and press `Enter`. The committed value
    /// is always clamped to `[min, max]` (and rounded in integer mode).
    ///
    /// Unlike [`slider`](Context::slider) — a bar-and-thumb control keyed by
    /// Left/Right — this renders the raw value as a `▾ 42 ▴` field that accepts
    /// direct typing. Config lives on [`NumberInputState`]. Up/`k` increments,
    /// Down/`j` decrements; `Enter` commits a typed buffer, `Esc` discards it,
    /// `Backspace` edits it. Left/Right are intentionally unused (reserved).
    ///
    /// `Response.focused` reflects focus and `Response.changed` is `true` iff
    /// the committed value changed this frame. All handled key and scroll
    /// events are consumed so they do not leak to other widgets or the global
    /// quit handler.
    ///
    /// Available since `0.21.0`.
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::*;
    /// # use slt::widgets::NumberInputState;
    /// # TestBackend::new(80, 24).render(|ui| {
    /// let mut qty = NumberInputState::integer(3, 0, 10).step(1.0);
    /// let r = ui.number_input(&mut qty);
    /// if r.changed { /* qty.value updated */ }
    /// # });
    /// ```
    pub fn number_input(&mut self, state: &mut NumberInputState) -> Response {
        let focused = self.register_focusable();
        // v0.21.1: capture focus-edge flags (issue #208 gap — number_input
        // assembled its Response by hand and never set gained/lost_focus).
        let (gained_focus, lost_focus) = self.focus_transitions(focused);

        // Normalize the committed value before processing input so the
        // pre-frame baseline used for `changed` is itself in-range.
        state.value = state.clamped();
        let old = state.value;
        let step = state.step.max(0.0);

        let adjust = |state: &mut NumberInputState, delta: f64| {
            if delta == 0.0 {
                return;
            }
            // Adjusting commits any in-progress buffer (discarding it) and
            // clears a prior parse error.
            state.editing = None;
            state.parse_error = None;
            state.value = (state.value + delta).clamp(state.min, state.max);
            if state.integer {
                state.value = state.value.round();
            }
        };

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        adjust(state, step);
                        consumed_indices.push(i);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        adjust(state, -step);
                        consumed_indices.push(i);
                    }
                    KeyCode::Char(ch) if is_number_char(ch, state) => {
                        let buf = state.editing.get_or_insert_with(String::new);
                        buf.push(ch);
                        state.parse_error = None;
                        consumed_indices.push(i);
                    }
                    KeyCode::Backspace => {
                        if let Some(buf) = state.editing.as_mut() {
                            buf.pop();
                            state.parse_error = None;
                            consumed_indices.push(i);
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(buf) = state.editing.take() {
                            let trimmed = buf.trim();
                            match trimmed.parse::<f64>() {
                                Ok(parsed) if parsed.is_finite() => {
                                    state.value = parsed.clamp(state.min, state.max);
                                    if state.integer {
                                        state.value = state.value.round();
                                    }
                                    state.parse_error = None;
                                }
                                _ => {
                                    state.parse_error = Some(format!("invalid number: {trimmed}"));
                                }
                            }
                            consumed_indices.push(i);
                        }
                    }
                    KeyCode::Esc if state.editing.is_some() => {
                        state.editing = None;
                        state.parse_error = None;
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
        }

        // Clamp again after key handling so the rendered value is in-range.
        state.value = state.clamped();

        let display = if let Some(buf) = state.editing.as_ref() {
            buf.clone()
        } else if state.integer {
            format!("{:.0}", state.value)
        } else {
            format_compact_number(state.value)
        };

        let primary_color = self.theme.primary;
        let dim_color = self.theme.text_dim;
        let error_color = self.theme.error;
        let value_color = if focused { primary_color } else { dim_color };
        let arrow_color = if focused { primary_color } else { dim_color };
        let parse_error = state.parse_error.clone();
        let editing = state.editing.is_some();

        let mut response = self.container().row(|ui| {
            ui.text("▾").fg(arrow_color);
            ui.text(" ");
            if focused {
                ui.text(display.as_str()).bold().fg(value_color);
            } else {
                ui.text(display.as_str()).fg(value_color);
            }
            ui.text(" ");
            ui.text("▴").fg(arrow_color);
            if editing {
                ui.text(" ✎").fg(dim_color);
            }
            if let Some(err) = parse_error.as_ref() {
                let mut indicator = String::with_capacity(2 + err.len());
                indicator.push_str("  ⚠ ");
                indicator.push_str(err);
                ui.text(indicator).dim().fg(error_color);
            }
        });

        // Scroll-wheel adjustment over the rendered field's rect. The row's
        // `Response.rect` comes from the previous frame's hit map (the standard
        // `prev_hit_map` pattern, mirroring `rich_log`), so a scroll tick takes
        // effect on the next frame. `ScrollUp` increments, `ScrollDown`
        // decrements, both clamped to `[min, max]`.
        if response.rect.width > 0 && response.rect.height > 0 {
            let rect = response.rect;
            let mut consumed = Vec::new();
            for (i, mouse) in self.mouse_events_in_rect(rect) {
                match mouse.kind {
                    MouseKind::ScrollUp => {
                        adjust(state, step);
                        consumed.push(i);
                    }
                    MouseKind::ScrollDown => {
                        adjust(state, -step);
                        consumed.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed);
        }

        // Final clamp guards against any direct mutation or scroll adjustment.
        state.value = state.clamped();

        response.focused = focused;
        // `changed` is true iff the committed value actually moved this frame.
        response.changed = (state.value - old).abs() > f64::EPSILON;
        response.gained_focus = gained_focus;
        response.lost_focus = lost_focus;
        response
    }
}

/// Whether `ch` may be appended to the in-progress edit buffer.
///
/// Always allows ASCII digits. Allows a single `.` in float mode (not when the
/// buffer already contains one). Allows a leading `-` only when negatives are
/// representable (`min < 0`) and the buffer is empty.
fn is_number_char(ch: char, state: &NumberInputState) -> bool {
    if ch.is_ascii_digit() {
        return true;
    }
    let buf = state.editing.as_deref().unwrap_or("");
    match ch {
        '.' => !state.integer && !buf.contains('.'),
        '-' => state.min < 0.0 && buf.is_empty(),
        _ => false,
    }
}
