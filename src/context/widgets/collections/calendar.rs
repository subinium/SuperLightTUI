//! The `calendar` date-picker widget: month grid, keyboard day navigation
//! and month paging.

use super::*;

impl Context {
    /// Render a calendar date picker with month navigation.
    ///
    /// Single-date mode is the default. Opt into range selection with
    /// [`CalendarState::with_range`] and an optional `HH:MM` time row with
    /// [`CalendarState::with_time`].
    ///
    /// # Keybindings (when focused)
    ///
    /// | Key | Action |
    /// |-----|--------|
    /// | `Left` / `h` | Previous day |
    /// | `Right` / `l` | Next day |
    /// | `Up` | Previous week (−7 days) |
    /// | `Down` | Next week (+7 days) |
    /// | `[` | Previous month |
    /// | `]` | Next month |
    /// | `Enter` / `Space` | Select cursor day (range: set anchor) |
    /// | `Shift+Left` / `Shift+H` | Extend range −1 day |
    /// | `Shift+Right` / `Shift+L` | Extend range +1 day |
    /// | `Shift+Up` | Extend range −7 days |
    /// | `Shift+Down` | Extend range +7 days |
    /// | `Shift+Enter` / `Shift+Space` | Set range extent at cursor |
    ///
    /// `h`/`l` follow vim convention (cursor by one day). Use `[`/`]` for
    /// month navigation. Mouse clicks on the title row navigate months and
    /// clicks inside the day grid select that day; in range mode a
    /// `Shift`+left-click sets the range extent endpoint.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// # let mut cal = slt::CalendarState::from_ym(2024, 3);
    /// cal.with_range();
    /// let resp = ui.calendar(&mut cal);
    /// if resp.changed {
    ///     if let Some((start, end)) = cal.selected_range() {
    ///         ui.text(format!("{}-{:02}-{:02} → {}-{:02}-{:02}",
    ///             start.year, start.month, start.day,
    ///             end.year, end.month, end.day));
    ///     }
    /// }
    /// # });
    /// ```
    pub fn calendar(&mut self, state: &mut CalendarState) -> Response {
        let focused = self.register_focusable();
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);

        let month_days = CalendarState::days_in_month(state.year, state.month);
        state.cursor_day = state.cursor_day.clamp(1, month_days);
        if let Some(day) = state.selected_day {
            state.selected_day = Some(day.min(month_days));
        }
        let old_selected = state.selected_day;
        let old_anchor = state.anchor;
        let old_extent = state.extent;
        let old_time = (state.hour, state.minute);

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                let range = state.mode == CalendarSelect::Range;
                // Day delta for cursor-movement keys; `None` for non-movement keys.
                let movement_delta = match key.code {
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => Some(-1),
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => Some(1),
                    KeyCode::Up => Some(-7),
                    KeyCode::Down => Some(7),
                    _ => None,
                };

                if let Some(delta) = movement_delta {
                    calendar_move_cursor_by_days(state, delta);
                    if range && shift {
                        // Shift-extend: move the extent endpoint with the cursor.
                        state.extend_to_cursor();
                    }
                    consumed_indices.push(i);
                    continue;
                }

                match key.code {
                    KeyCode::Char('[') => {
                        state.prev_month();
                        consumed_indices.push(i);
                    }
                    KeyCode::Char(']') => {
                        state.next_month();
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if range {
                            if shift {
                                // Set the range extent endpoint at the cursor.
                                state.extend_to_cursor();
                            } else {
                                // Set / reset the anchor at the cursor.
                                state.set_anchor_to_cursor();
                            }
                        } else {
                            state.selected_day = Some(state.cursor_day);
                        }
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
        }

        if let Some((rect, clicks)) = self.left_clicks_for_interaction(interaction_id) {
            let mut consumed = Vec::new();
            for (i, mouse) in clicks {
                let rel_x = mouse.x.saturating_sub(rect.x);
                let rel_y = mouse.y.saturating_sub(rect.y);
                if rel_y == 0 {
                    if rel_x <= 2 {
                        state.prev_month();
                        consumed.push(i);
                        continue;
                    }
                    if rel_x + 3 >= rect.width {
                        state.next_month();
                        consumed.push(i);
                        continue;
                    }
                }

                if !(2..8).contains(&rel_y) {
                    continue;
                }
                if rel_x >= 21 {
                    continue;
                }

                let week = rel_y - 2;
                let col = rel_x / 3;
                let day_index = week * 7 + col;
                let first = CalendarState::first_weekday(state.year, state.month);
                let days = CalendarState::days_in_month(state.year, state.month);
                if day_index < first {
                    continue;
                }
                let day = day_index - first + 1;
                if day == 0 || day > days {
                    continue;
                }
                state.cursor_day = day;
                if state.mode == CalendarSelect::Range {
                    if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                        // Shift+click sets the range extent endpoint.
                        state.extend_to_cursor();
                    } else {
                        // Plain click (re)sets the anchor.
                        state.set_anchor_to_cursor();
                    }
                } else {
                    state.selected_day = Some(day);
                }
                consumed.push(i);
            }
            self.consume_indices(consumed);
        }

        let title = {
            let month_name = calendar_month_name(state.month);
            let mut s = String::with_capacity(16);
            s.push_str(&state.year.to_string());
            s.push(' ');
            s.push_str(month_name);
            s
        };

        self.commands
            .push(Command::BeginContainer(BeginContainerArgs {
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
            }));

        let cal_gap = self.theme.spacing.xs();
        self.commands
            .push(Command::BeginContainer(BeginContainerArgs {
                direction: Direction::Row,
                gap: cal_gap as i32,
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
            }));
        self.styled("◀", Style::new().fg(self.theme.text));
        self.styled(title, Style::new().bold().fg(self.theme.text));
        self.styled("▶", Style::new().fg(self.theme.text));
        self.commands.push(Command::EndContainer);

        self.commands
            .push(Command::BeginContainer(BeginContainerArgs {
                direction: Direction::Row,
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
            }));
        for wd in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
            self.styled(
                format!("{wd:>2} "),
                Style::new().fg(self.theme.text_dim).bold(),
            );
        }
        self.commands.push(Command::EndContainer);

        let first = CalendarState::first_weekday(state.year, state.month);
        let days = CalendarState::days_in_month(state.year, state.month);
        for week in 0..6_u32 {
            self.commands
                .push(Command::BeginContainer(BeginContainerArgs {
                    direction: Direction::Row,
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
                }));

            for col in 0..7_u32 {
                let idx = week * 7 + col;
                if idx < first || idx >= first + days {
                    self.styled("   ", Style::new().fg(self.theme.text_dim));
                    continue;
                }
                let day = idx - first + 1;
                let text = format!("{day:>2} ");
                let cell = CalDate {
                    year: state.year,
                    month: state.month,
                    day,
                };
                let style = if state.mode == CalendarSelect::Range {
                    if state.is_range_endpoint(cell) {
                        // Endpoints get the strong selected highlight.
                        Style::new()
                            .bg(self.theme.selected_bg)
                            .fg(self.theme.selected_fg)
                    } else if state.in_range(cell) {
                        // Interior band: subtler surface fill, distinct from endpoints.
                        Style::new().bg(self.theme.surface).fg(self.theme.text)
                    } else if state.cursor_day == day {
                        Style::new().fg(self.theme.primary).bold()
                    } else {
                        Style::new().fg(self.theme.text)
                    }
                } else if state.selected_day == Some(day) {
                    Style::new()
                        .bg(self.theme.selected_bg)
                        .fg(self.theme.selected_fg)
                } else if state.cursor_day == day {
                    Style::new().fg(self.theme.primary).bold()
                } else {
                    Style::new().fg(self.theme.text)
                };
                self.styled(text, style);
            }

            self.commands.push(Command::EndContainer);
        }

        if state.time_enabled {
            let time_text = format!("{:02}:{:02}", state.hour, state.minute);
            self.styled(time_text, Style::new().fg(self.theme.text).bold());
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
        response.changed = state.selected_day != old_selected
            || state.anchor != old_anchor
            || state.extent != old_extent
            || (state.hour, state.minute) != old_time;
        response
    }
}

fn calendar_month_name(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

fn calendar_move_cursor_by_days(state: &mut CalendarState, delta: i32) {
    let mut remaining = delta;
    while remaining != 0 {
        let days = CalendarState::days_in_month(state.year, state.month);
        if remaining > 0 {
            let forward = days.saturating_sub(state.cursor_day) as i32;
            if remaining <= forward {
                state.cursor_day += remaining as u32;
                return;
            }

            remaining -= forward + 1;
            if state.month == 12 {
                state.month = 1;
                state.year += 1;
            } else {
                state.month += 1;
            }
            state.cursor_day = 1;
        } else {
            let backward = state.cursor_day.saturating_sub(1) as i32;
            if -remaining <= backward {
                state.cursor_day -= (-remaining) as u32;
                return;
            }

            remaining += backward + 1;
            if state.month == 1 {
                state.month = 12;
                state.year -= 1;
            } else {
                state.month -= 1;
            }
            state.cursor_day = CalendarState::days_in_month(state.year, state.month);
        }
    }
}
