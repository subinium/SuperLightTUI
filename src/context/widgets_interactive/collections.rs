use super::*;

impl Context {
    /// Render children in a fixed grid with the given number of columns.
    ///
    /// Children are placed left-to-right, top-to-bottom. Each cell has equal
    /// width (`area_width / cols`). Rows wrap automatically.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.grid(3, |ui| {
    ///     for i in 0..9 {
    ///         ui.text(format!("Cell {i}"));
    ///     }
    /// });
    /// # });
    /// ```
    pub fn grid(&mut self, cols: u32, f: impl FnOnce(&mut Context)) -> Response {
        slt_assert(cols > 0, "grid() requires at least 1 column");
        let interaction_id = self.next_interaction_id();
        let border = self.theme.border;

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Column,
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));

        let children_start = self.commands.len();
        f(self);
        let child_commands: Vec<Command> = self.commands.drain(children_start..).collect();

        let elements = collect_grid_elements(child_commands);

        let cols = cols.max(1) as usize;
        for row in elements.chunks(cols) {
            self.skip_interaction_slot();
            self.commands
                .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                    direction: Direction::Row,
                    gap: 0,
                    align: Align::Start,
                    align_self: None,
                    justify: Justify::Start,
                    border: None,
                    border_sides: BorderSides::all(),
                    border_style: Style::new().fg(border),
                    bg_color: None,
                    padding: Padding::default(),
                    margin: Margin::default(),
                    constraints: Constraints::default(),
                    title: None,
                    grow: 0,
                    group_name: None,
                })));

            for element in row {
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
                        border_style: Style::new().fg(border),
                        bg_color: None,
                        padding: Padding::default(),
                        margin: Margin::default(),
                        constraints: Constraints::default(),
                        title: None,
                        grow: 1,
                        group_name: None,
                    })));
                self.commands.extend(element.iter().cloned());
                self.commands.push(Command::EndContainer);
            }

            self.commands.push(Command::EndContainer);
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        self.response_for(interaction_id)
    }

    /// Render children in a grid with per-column width specifications.
    ///
    /// The number of columns is determined by the length of `columns`. Children
    /// are placed left-to-right, top-to-bottom, wrapping into rows
    /// automatically.
    ///
    /// # Column specifications
    ///
    /// - [`GridColumn::Auto`] — equal-width flex column (same as `grid()`)
    /// - [`GridColumn::Fixed(n)`](GridColumn::Fixed) — exactly `n` character cells wide
    /// - [`GridColumn::Grow(w)`](GridColumn::Grow) — flexible with grow weight `w`
    /// - [`GridColumn::Percent(p)`](GridColumn::Percent) — `p`% of the grid width
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::GridColumn;
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.grid_with(&[
    ///     GridColumn::Fixed(8),
    ///     GridColumn::Grow(1),
    ///     GridColumn::Grow(1),
    ///     GridColumn::Fixed(4),
    /// ], |ui| {
    ///     for i in 0..8 {
    ///         ui.text(format!("Cell {i}"));
    ///     }
    /// });
    /// # });
    /// ```
    pub fn grid_with(&mut self, columns: &[GridColumn], f: impl FnOnce(&mut Context)) -> Response {
        let cols = columns.len().max(1);
        let interaction_id = self.next_interaction_id();
        let border = self.theme.border;

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Column,
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));

        let children_start = self.commands.len();
        f(self);
        let child_commands: Vec<Command> = self.commands.drain(children_start..).collect();

        let elements = collect_grid_elements(child_commands);

        for row in elements.chunks(cols) {
            self.skip_interaction_slot();
            self.commands
                .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                    direction: Direction::Row,
                    gap: 0,
                    align: Align::Start,
                    align_self: None,
                    justify: Justify::Start,
                    border: None,
                    border_sides: BorderSides::all(),
                    border_style: Style::new().fg(border),
                    bg_color: None,
                    padding: Padding::default(),
                    margin: Margin::default(),
                    constraints: Constraints::default(),
                    title: None,
                    grow: 0,
                    group_name: None,
                })));

            for (col_idx, element) in row.iter().enumerate() {
                let spec = columns.get(col_idx).copied().unwrap_or(GridColumn::Auto);
                let (grow, constraints) = match spec {
                    GridColumn::Auto => (1, Constraints::default()),
                    GridColumn::Fixed(w) => (0, Constraints::default().w(w)),
                    GridColumn::Grow(g) => (g, Constraints::default()),
                    GridColumn::Percent(p) => (0, Constraints::default().w_pct(p)),
                };

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
                        border_style: Style::new().fg(border),
                        bg_color: None,
                        padding: Padding::default(),
                        margin: Margin::default(),
                        constraints,
                        title: None,
                        grow,
                        group_name: None,
                    })));
                self.commands.extend(element.iter().cloned());
                self.commands.push(Command::EndContainer);
            }

            self.commands.push(Command::EndContainer);
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        self.response_for(interaction_id)
    }

    /// Render a selectable list. Handles Up/Down (and `k`/`j`) navigation when focused.
    ///
    /// The selected item is highlighted with the theme's primary color. If the
    /// list is empty, nothing is rendered.
    pub fn list(&mut self, state: &mut ListState) -> Response {
        let colors = self.widget_theme.list;
        self.list_colored(state, &colors)
    }

    /// Render a navigable list with custom widget colors.
    pub fn list_colored(&mut self, state: &mut ListState, colors: &WidgetColors) -> Response {
        let visible = state.visible_indices().to_vec();
        if visible.is_empty() && state.items.is_empty() {
            state.selected = 0;
            return Response::none();
        }

        if !visible.is_empty() {
            state.selected = state.selected.min(visible.len().saturating_sub(1));
        }

        let old_selected = state.selected;
        let focused = self.register_focusable();
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                        let _ = handle_vertical_nav(
                            &mut state.selected,
                            visible.len().saturating_sub(1),
                            key.code.clone(),
                        );
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
                let clicked_idx = (mouse.y - rect.y) as usize;
                if clicked_idx < visible.len() {
                    state.selected = clicked_idx;
                    consumed.push(i);
                }
            }
            self.consume_indices(consumed);
        }

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Column,
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(colors.border.unwrap_or(self.theme.border)),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));

        for (view_idx, &item_idx) in visible.iter().enumerate() {
            let item = &state.items[item_idx];
            if view_idx == state.selected {
                let mut selected_style = Style::new()
                    .bg(colors.accent.unwrap_or(self.theme.selected_bg))
                    .fg(colors.fg.unwrap_or(self.theme.selected_fg));
                if focused {
                    selected_style = selected_style.bold();
                }
                let mut row = String::with_capacity(2 + item.len());
                row.push_str("▸ ");
                row.push_str(item);
                self.styled(row, selected_style);
            } else {
                let mut row = String::with_capacity(2 + item.len());
                row.push_str("  ");
                row.push_str(item);
                self.styled(row, Style::new().fg(colors.fg.unwrap_or(self.theme.text)));
            }
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        response.changed = state.selected != old_selected;
        response
    }

    /// Render a selectable list that supports keyboard reordering of items.
    ///
    /// Behaves exactly like [`list`](Context::list) for navigation (Up/Down and
    /// `k`/`j` move the selection) and click selection, but additionally lets the
    /// focused user reorder the selected item with `Shift+Up`/`Shift+Down` or
    /// `Alt+Up`/`Alt+Down`. Reordering operates on the underlying item order via
    /// [`ListState::move_item`], keeping the selection on the moved item.
    ///
    /// Returns a [`ListResponse`] which derefs to the standard [`Response`] and
    /// exposes [`reordered`](ListResponse::reordered) — `Some((from, to))` with
    /// the data indices when an item moved this frame, otherwise `None`.
    ///
    /// The plain [`list`](Context::list) entry point is unchanged; opt into
    /// reordering by calling this method instead.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::ListState;
    /// # let mut list = ListState::new(vec!["First", "Second", "Third"]);
    /// # slt::run(move |ui: &mut slt::Context| {
    /// let r = ui.list_reorderable(&mut list);
    /// if let Some((from, to)) = r.reordered {
    ///     let _ = (from, to); // persist new order
    /// }
    /// # });
    /// ```
    ///
    /// Available since `0.21.1`.
    pub fn list_reorderable(&mut self, state: &mut ListState) -> crate::widgets::ListResponse {
        let colors = self.widget_theme.list;
        self.list_reorderable_colored(state, &colors)
    }

    /// Render a reorderable list with custom widget colors.
    ///
    /// See [`list_reorderable`](Context::list_reorderable) for the reorder
    /// keybindings and return semantics.
    ///
    /// Available since `0.21.1`.
    pub fn list_reorderable_colored(
        &mut self,
        state: &mut ListState,
        colors: &WidgetColors,
    ) -> crate::widgets::ListResponse {
        let visible = state.visible_indices().to_vec();
        if visible.is_empty() && state.items.is_empty() {
            state.selected = 0;
            return crate::widgets::ListResponse::default();
        }

        if !visible.is_empty() {
            state.selected = state.selected.min(visible.len().saturating_sub(1));
        }

        let old_selected = state.selected;
        let focused = self.register_focusable();
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);

        let mut reordered: Option<(usize, usize)> = None;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                // Reorder takes precedence over navigation when a Shift/Alt
                // modifier is held with a vertical-movement key.
                let modded = key.modifiers.contains(KeyModifiers::SHIFT)
                    || key.modifiers.contains(KeyModifiers::ALT);
                // Direction of the move: -1 (up) or +1 (down) for the selected
                // view row, `None` for non-movement keys.
                let dir: Option<isize> = match key.code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Some(-1),
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => Some(1),
                    _ => None,
                };

                if modded {
                    if let Some(delta) = dir {
                        let cur_view = state.selected;
                        let target_view = if delta < 0 {
                            cur_view.checked_sub(1)
                        } else {
                            let next = cur_view + 1;
                            (next < visible.len()).then_some(next)
                        };
                        // Map both endpoints from view positions to data indices
                        // so reordering survives an active filter.
                        if let Some(target_view) = target_view {
                            if let (Some(&from), Some(&to)) =
                                (visible.get(cur_view), visible.get(target_view))
                            {
                                if state.move_item(from, to) {
                                    reordered = Some((from, to));
                                }
                            }
                        }
                        // Consume regardless so a held modifier never also
                        // triggers a plain navigation step on the same key.
                        consumed_indices.push(i);
                    }
                    continue;
                }

                if dir.is_some() {
                    let _ = handle_vertical_nav(
                        &mut state.selected,
                        visible.len().saturating_sub(1),
                        key.code.clone(),
                    );
                    consumed_indices.push(i);
                }
            }
            self.consume_indices(consumed_indices);
        }

        if let Some((rect, clicks)) = self.left_clicks_for_interaction(interaction_id) {
            let mut consumed = Vec::new();
            // `visible` may be stale after a reorder rebuilt the view; re-read
            // the current visible count for bounds.
            let visible_len = state.visible_indices().len();
            for (i, mouse) in clicks {
                let clicked_idx = (mouse.y - rect.y) as usize;
                if clicked_idx < visible_len {
                    state.selected = clicked_idx;
                    consumed.push(i);
                }
            }
            self.consume_indices(consumed);
        }

        // Re-read the (possibly reordered) view for rendering.
        let visible = state.visible_indices().to_vec();

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Column,
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(colors.border.unwrap_or(self.theme.border)),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));

        for (view_idx, &item_idx) in visible.iter().enumerate() {
            let item = &state.items[item_idx];
            if view_idx == state.selected {
                let mut selected_style = Style::new()
                    .bg(colors.accent.unwrap_or(self.theme.selected_bg))
                    .fg(colors.fg.unwrap_or(self.theme.selected_fg));
                if focused {
                    selected_style = selected_style.bold();
                }
                let mut row = String::with_capacity(2 + item.len());
                row.push_str("▸ ");
                row.push_str(item);
                self.styled(row, selected_style);
            } else {
                let mut row = String::with_capacity(2 + item.len());
                row.push_str("  ");
                row.push_str(item);
                self.styled(row, Style::new().fg(colors.fg.unwrap_or(self.theme.text)));
            }
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        response.changed = state.selected != old_selected || reordered.is_some();
        crate::widgets::ListResponse {
            response,
            reordered,
        }
    }

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

        let cal_gap = self.theme.spacing.xs();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
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
            })));
        self.styled("◀", Style::new().fg(self.theme.text));
        self.styled(title, Style::new().bold().fg(self.theme.text));
        self.styled("▶", Style::new().fg(self.theme.text));
        self.commands.push(Command::EndContainer);

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
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
            })));
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
                .push(Command::BeginContainer(Box::new(BeginContainerArgs {
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
                })));

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

    /// Render a file system browser with directory navigation.
    pub fn file_picker(&mut self, state: &mut FilePickerState) -> Response {
        if state.dirty {
            state.refresh();
        }
        if !state.entries.is_empty() {
            state.selected = state.selected.min(state.entries.len().saturating_sub(1));
        }

        let focused = self.register_focusable();
        let (_interaction_id, mut response) = self.begin_widget_interaction(focused);
        let mut file_selected = false;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                        if !state.entries.is_empty() {
                            let _ = handle_vertical_nav(
                                &mut state.selected,
                                state.entries.len().saturating_sub(1),
                                key.code.clone(),
                            );
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter => {
                        if let Some(entry) = state.entries.get(state.selected).cloned() {
                            if entry.is_dir {
                                state.current_dir = entry.path;
                                state.selected = 0;
                                state.selected_file = None;
                                state.dirty = true;
                            } else {
                                state.selected_file = Some(entry.path);
                                file_selected = true;
                            }
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Backspace => {
                        if let Some(parent) = state.current_dir.parent().map(|p| p.to_path_buf()) {
                            state.current_dir = parent;
                            state.selected = 0;
                            state.selected_file = None;
                            state.dirty = true;
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Char('h') => {
                        state.show_hidden = !state.show_hidden;
                        state.selected = 0;
                        state.dirty = true;
                        consumed_indices.push(i);
                    }
                    KeyCode::Esc => {
                        state.selected_file = None;
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
        }

        if state.dirty {
            state.refresh();
        }

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

        let dir_text = {
            let dir = state.current_dir.display().to_string();
            let mut text = String::with_capacity(5 + dir.len());
            text.push_str("Dir: ");
            text.push_str(&dir);
            text
        };
        self.styled(dir_text, Style::new().fg(self.theme.text_dim).dim());

        if state.entries.is_empty() {
            self.styled("(empty)", Style::new().fg(self.theme.text_dim).dim());
        } else {
            for (idx, entry) in state.entries.iter().enumerate() {
                let icon = if entry.is_dir { "▸ " } else { "  " };
                let row = if entry.is_dir {
                    let mut row = String::with_capacity(icon.len() + entry.name.len());
                    row.push_str(icon);
                    row.push_str(&entry.name);
                    row
                } else {
                    let size_text = entry.size.to_string();
                    let mut row =
                        String::with_capacity(icon.len() + entry.name.len() + size_text.len() + 4);
                    row.push_str(icon);
                    row.push_str(&entry.name);
                    row.push_str("  ");
                    row.push_str(&size_text);
                    row.push_str(" B");
                    row
                };

                let style = if idx == state.selected {
                    if focused {
                        Style::new().bold().fg(self.theme.primary)
                    } else {
                        Style::new().fg(self.theme.primary)
                    }
                } else {
                    Style::new().fg(self.theme.text)
                };
                self.styled(row, style);
            }
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        response.changed = file_selected;
        response
    }
}

/// Group `child_commands` into per-cell command vectors for `grid()` / `grid_with()`.
///
/// Each container subtree (`BeginContainer`/`BeginScrollable` … matching `EndContainer`)
/// becomes one element. Bare `InteractionMarker`s are flushed onto the next element so
/// hit-testing slots stay attached to the cell that owns them. Trailing markers with
/// no following command form their own (empty-content) element.
fn collect_grid_elements(child_commands: Vec<Command>) -> Vec<Vec<Command>> {
    let mut elements: Vec<Vec<Command>> = Vec::new();
    let mut iter = child_commands.into_iter().peekable();
    let mut pending_markers: Vec<Command> = Vec::new();
    while let Some(cmd) = iter.next() {
        match cmd {
            Command::InteractionMarker(_) => {
                pending_markers.push(cmd);
            }
            Command::BeginContainer(_) | Command::BeginScrollable(_) => {
                let mut depth = 1_u32;
                let mut element: Vec<Command> = std::mem::take(&mut pending_markers);
                element.push(cmd);
                for next in iter.by_ref() {
                    match next {
                        Command::BeginContainer(_) | Command::BeginScrollable(_) => {
                            depth += 1;
                        }
                        Command::EndContainer => {
                            depth = depth.saturating_sub(1);
                        }
                        _ => {}
                    }
                    let at_end = matches!(next, Command::EndContainer) && depth == 0;
                    element.push(next);
                    if at_end {
                        break;
                    }
                }
                elements.push(element);
            }
            Command::EndContainer => {}
            _ => {
                let mut element = std::mem::take(&mut pending_markers);
                element.push(cmd);
                elements.push(element);
            }
        }
    }
    // Flush any trailing markers (edge case: marker with no following command)
    if !pending_markers.is_empty() {
        elements.push(pending_markers);
    }
    elements
}

#[cfg(test)]
mod list_reorder_render_tests {
    use crate::widgets::ListState;
    use crate::{EventBuilder, KeyCode, KeyModifiers, TestBackend};

    #[test]
    fn shift_down_reorders_selected_item() {
        let mut backend = TestBackend::new(20, 6);
        let mut state = ListState::new(vec!["alpha", "beta", "gamma"]);
        state.selected = 0; // "alpha"

        let events = EventBuilder::new()
            .key_with(KeyCode::Down, KeyModifiers::SHIFT)
            .build();

        let mut reordered = None;
        backend.run_with_events(events, |ui| {
            let r = ui.list_reorderable(&mut state);
            reordered = r.reordered;
        });

        // "alpha" (data 0) swapped down with "beta" (data 1).
        assert_eq!(reordered, Some((0, 1)));
        assert_eq!(state.items, vec!["beta", "alpha", "gamma"]);
        // Selection follows the moved item to its new position.
        assert_eq!(state.selected, 1);
        assert_eq!(state.selected_item(), Some("alpha"));
    }

    #[test]
    fn alt_up_reorders_selected_item() {
        let mut backend = TestBackend::new(20, 6);
        let mut state = ListState::new(vec!["one", "two", "three"]);
        state.selected = 2; // "three"

        let events = EventBuilder::new()
            .key_with(KeyCode::Up, KeyModifiers::ALT)
            .build();

        let mut reordered = None;
        backend.run_with_events(events, |ui| {
            reordered = ui.list_reorderable(&mut state).reordered;
        });

        assert_eq!(reordered, Some((2, 1)));
        assert_eq!(state.items, vec!["one", "three", "two"]);
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn shift_up_at_top_is_a_noop() {
        let mut backend = TestBackend::new(20, 6);
        let mut state = ListState::new(vec!["a", "b", "c"]);
        state.selected = 0;

        let events = EventBuilder::new()
            .key_with(KeyCode::Up, KeyModifiers::SHIFT)
            .build();

        let mut reordered = Some((9, 9));
        backend.run_with_events(events, |ui| {
            reordered = ui.list_reorderable(&mut state).reordered;
        });

        // No room to move up from the top: nothing reordered.
        assert_eq!(reordered, None);
        assert_eq!(state.items, vec!["a", "b", "c"]);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn plain_down_navigates_without_reordering() {
        let mut backend = TestBackend::new(20, 6);
        let mut state = ListState::new(vec!["a", "b", "c"]);
        state.selected = 0;

        let events = EventBuilder::new().key_code(KeyCode::Down).build();

        let mut reordered = Some((9, 9));
        backend.run_with_events(events, |ui| {
            reordered = ui.list_reorderable(&mut state).reordered;
        });

        // Without a modifier, Down moves the selection but never reorders.
        assert_eq!(reordered, None);
        assert_eq!(state.items, vec!["a", "b", "c"]);
        assert_eq!(state.selected, 1);
    }
}
