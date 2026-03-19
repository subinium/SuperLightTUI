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

        self.commands.push(Command::BeginContainer {
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
        });

        let children_start = self.commands.len();
        f(self);
        let child_commands: Vec<Command> = self.commands.drain(children_start..).collect();

        let mut elements: Vec<Vec<Command>> = Vec::new();
        let mut iter = child_commands.into_iter().peekable();
        while let Some(cmd) = iter.next() {
            match cmd {
                Command::BeginContainer { .. } | Command::BeginScrollable { .. } => {
                    let mut depth = 1_u32;
                    let mut element = vec![cmd];
                    for next in iter.by_ref() {
                        match next {
                            Command::BeginContainer { .. } | Command::BeginScrollable { .. } => {
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
                _ => elements.push(vec![cmd]),
            }
        }

        let cols = cols.max(1) as usize;
        for row in elements.chunks(cols) {
            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
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
            });

            for element in row {
                self.interaction_count += 1;
                self.commands.push(Command::BeginContainer {
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
                });
                self.commands.extend(element.iter().cloned());
                self.commands.push(Command::EndContainer);
            }

            self.commands.push(Command::EndContainer);
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self.response_for(interaction_id)
    }

    /// Render a selectable list. Handles Up/Down (and `k`/`j`) navigation when focused.
    ///
    /// The selected item is highlighted with the theme's primary color. If the
    /// list is empty, nothing is rendered.
    pub fn list(&mut self, state: &mut ListState) -> Response {
        self.list_colored(state, &WidgetColors::new())
    }

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
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
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
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if let Some(rect) = self.prev_hit_map.get(interaction_id).copied() {
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Mouse(mouse) = event {
                    if !matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                        continue;
                    }
                    let in_bounds = mouse.x >= rect.x
                        && mouse.x < rect.right()
                        && mouse.y >= rect.y
                        && mouse.y < rect.bottom();
                    if !in_bounds {
                        continue;
                    }
                    let clicked_idx = (mouse.y - rect.y) as usize;
                    if clicked_idx < visible.len() {
                        state.selected = clicked_idx;
                        self.consumed[i] = true;
                    }
                }
            }
        }

        self.commands.push(Command::BeginContainer {
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
        });

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
        self.last_text_idx = None;

        response.changed = state.selected != old_selected;
        response
    }

    pub fn calendar(&mut self, state: &mut CalendarState) -> Response {
        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;

        let month_days = CalendarState::days_in_month(state.year, state.month);
        state.cursor_day = state.cursor_day.clamp(1, month_days);
        if let Some(day) = state.selected_day {
            state.selected_day = Some(day.min(month_days));
        }
        let old_selected = state.selected_day;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Left => {
                            calendar_move_cursor_by_days(state, -1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Right => {
                            calendar_move_cursor_by_days(state, 1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Up => {
                            calendar_move_cursor_by_days(state, -7);
                            consumed_indices.push(i);
                        }
                        KeyCode::Down => {
                            calendar_move_cursor_by_days(state, 7);
                            consumed_indices.push(i);
                        }
                        KeyCode::Char('h') => {
                            state.prev_month();
                            consumed_indices.push(i);
                        }
                        KeyCode::Char('l') => {
                            state.next_month();
                            consumed_indices.push(i);
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            state.selected_day = Some(state.cursor_day);
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if let Some(rect) = self.prev_hit_map.get(interaction_id).copied() {
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Mouse(mouse) = event {
                    if !matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                        continue;
                    }
                    let in_bounds = mouse.x >= rect.x
                        && mouse.x < rect.right()
                        && mouse.y >= rect.y
                        && mouse.y < rect.bottom();
                    if !in_bounds {
                        continue;
                    }

                    let rel_x = mouse.x.saturating_sub(rect.x);
                    let rel_y = mouse.y.saturating_sub(rect.y);
                    if rel_y == 0 {
                        if rel_x <= 2 {
                            state.prev_month();
                            self.consumed[i] = true;
                            continue;
                        }
                        if rel_x + 3 >= rect.width {
                            state.next_month();
                            self.consumed[i] = true;
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
                    state.selected_day = Some(day);
                    self.consumed[i] = true;
                }
            }
        }

        let title = {
            let month_name = calendar_month_name(state.month);
            let mut s = String::with_capacity(16);
            s.push_str(&state.year.to_string());
            s.push(' ');
            s.push_str(month_name);
            s
        };

        self.commands.push(Command::BeginContainer {
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
        });

        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 1,
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
        });
        self.styled("◀", Style::new().fg(self.theme.text));
        self.styled(title, Style::new().bold().fg(self.theme.text));
        self.styled("▶", Style::new().fg(self.theme.text));
        self.commands.push(Command::EndContainer);

        self.commands.push(Command::BeginContainer {
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
        });
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
            self.commands.push(Command::BeginContainer {
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
            });

            for col in 0..7_u32 {
                let idx = week * 7 + col;
                if idx < first || idx >= first + days {
                    self.styled("   ", Style::new().fg(self.theme.text_dim));
                    continue;
                }
                let day = idx - first + 1;
                let text = format!("{day:>2} ");
                let style = if state.selected_day == Some(day) {
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

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        response.changed = state.selected_day != old_selected;
        response
    }

    pub fn file_picker(&mut self, state: &mut FilePickerState) -> Response {
        if state.dirty {
            state.refresh();
        }
        if !state.entries.is_empty() {
            state.selected = state.selected.min(state.entries.len().saturating_sub(1));
        }

        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;
        let mut file_selected = false;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
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
                            if let Some(parent) =
                                state.current_dir.parent().map(|p| p.to_path_buf())
                            {
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
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if state.dirty {
            state.refresh();
        }

        self.commands.push(Command::BeginContainer {
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
        });

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
        self.last_text_idx = None;

        response.changed = file_selected;
        response
    }

    /// Render a data table with column headers. Handles Up/Down selection when focused.
    ///
    /// Column widths are computed automatically from header and cell content.
    /// The selected row is highlighted with the theme's selection colors.
    pub fn table(&mut self, state: &mut TableState) -> Response {
        self.table_colored(state, &WidgetColors::new())
    }

    pub fn table_colored(&mut self, state: &mut TableState, colors: &WidgetColors) -> Response {
        if state.is_dirty() {
            state.recompute_widths();
        }

        let old_selected = state.selected;
        let old_sort_column = state.sort_column;
        let old_sort_ascending = state.sort_ascending;
        let old_page = state.page;
        let old_filter = state.filter.clone();

        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;

        self.table_handle_events(state, focused, interaction_id);

        if state.is_dirty() {
            state.recompute_widths();
        }

        self.table_render(state, focused, colors);

        response.changed = state.selected != old_selected
            || state.sort_column != old_sort_column
            || state.sort_ascending != old_sort_ascending
            || state.page != old_page
            || state.filter != old_filter;
        response
    }

    fn table_handle_events(
        &mut self,
        state: &mut TableState,
        focused: bool,
        interaction_id: usize,
    ) {
        self.handle_table_keys(state, focused);

        if state.visible_indices().is_empty() && state.headers.is_empty() {
            return;
        }

        if let Some(rect) = self.prev_hit_map.get(interaction_id).copied() {
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Mouse(mouse) = event {
                    if !matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                        continue;
                    }
                    let in_bounds = mouse.x >= rect.x
                        && mouse.x < rect.right()
                        && mouse.y >= rect.y
                        && mouse.y < rect.bottom();
                    if !in_bounds {
                        continue;
                    }

                    if mouse.y == rect.y {
                        let rel_x = mouse.x.saturating_sub(rect.x);
                        let mut x_offset = 0u32;
                        for (col_idx, width) in state.column_widths().iter().enumerate() {
                            if rel_x >= x_offset && rel_x < x_offset + *width {
                                state.toggle_sort(col_idx);
                                state.selected = 0;
                                self.consumed[i] = true;
                                break;
                            }
                            x_offset += *width;
                            if col_idx + 1 < state.column_widths().len() {
                                x_offset += 3;
                            }
                        }
                        continue;
                    }

                    if mouse.y < rect.y + 2 {
                        continue;
                    }

                    let visible_len = if state.page_size > 0 {
                        let start = state
                            .page
                            .saturating_mul(state.page_size)
                            .min(state.visible_indices().len());
                        let end = (start + state.page_size).min(state.visible_indices().len());
                        end.saturating_sub(start)
                    } else {
                        state.visible_indices().len()
                    };
                    let clicked_idx = (mouse.y - rect.y - 2) as usize;
                    if clicked_idx < visible_len {
                        state.selected = clicked_idx;
                        self.consumed[i] = true;
                    }
                }
            }
        }
    }

    fn table_render(&mut self, state: &mut TableState, focused: bool, colors: &WidgetColors) {
        let total_visible = state.visible_indices().len();
        let page_start = if state.page_size > 0 {
            state
                .page
                .saturating_mul(state.page_size)
                .min(total_visible)
        } else {
            0
        };
        let page_end = if state.page_size > 0 {
            (page_start + state.page_size).min(total_visible)
        } else {
            total_visible
        };
        let visible_len = page_end.saturating_sub(page_start);
        state.selected = state.selected.min(visible_len.saturating_sub(1));

        self.commands.push(Command::BeginContainer {
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
        });

        self.render_table_header(state, colors);
        self.render_table_rows(state, focused, page_start, visible_len, colors);

        if state.page_size > 0 && state.total_pages() > 1 {
            let current_page = (state.page + 1).to_string();
            let total_pages = state.total_pages().to_string();
            let mut page_text = String::with_capacity(current_page.len() + total_pages.len() + 6);
            page_text.push_str("Page ");
            page_text.push_str(&current_page);
            page_text.push('/');
            page_text.push_str(&total_pages);
            self.styled(
                page_text,
                Style::new()
                    .dim()
                    .fg(colors.fg.unwrap_or(self.theme.text_dim)),
            );
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
    }

    fn handle_table_keys(&mut self, state: &mut TableState, focused: bool) {
        if !focused || state.visible_indices().is_empty() {
            return;
        }

        let mut consumed_indices = Vec::new();
        for (i, event) in self.events.iter().enumerate() {
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                        let visible_len = table_visible_len(state);
                        state.selected = state.selected.min(visible_len.saturating_sub(1));
                        let _ = handle_vertical_nav(
                            &mut state.selected,
                            visible_len.saturating_sub(1),
                            key.code.clone(),
                        );
                        consumed_indices.push(i);
                    }
                    KeyCode::PageUp => {
                        let old_page = state.page;
                        state.prev_page();
                        if state.page != old_page {
                            state.selected = 0;
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::PageDown => {
                        let old_page = state.page;
                        state.next_page();
                        if state.page != old_page {
                            state.selected = 0;
                        }
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
        }
        for index in consumed_indices {
            self.consumed[index] = true;
        }
    }

    fn render_table_header(&mut self, state: &TableState, colors: &WidgetColors) {
        let header_cells = state
            .headers
            .iter()
            .enumerate()
            .map(|(i, header)| {
                if state.sort_column == Some(i) {
                    if state.sort_ascending {
                        let mut sorted_header = String::with_capacity(header.len() + 2);
                        sorted_header.push_str(header);
                        sorted_header.push_str(" ▲");
                        sorted_header
                    } else {
                        let mut sorted_header = String::with_capacity(header.len() + 2);
                        sorted_header.push_str(header);
                        sorted_header.push_str(" ▼");
                        sorted_header
                    }
                } else {
                    header.clone()
                }
            })
            .collect::<Vec<_>>();
        let header_line = format_table_row(&header_cells, state.column_widths(), " │ ");
        self.styled(
            header_line,
            Style::new().bold().fg(colors.fg.unwrap_or(self.theme.text)),
        );

        let separator = state
            .column_widths()
            .iter()
            .map(|w| "─".repeat(*w as usize))
            .collect::<Vec<_>>()
            .join("─┼─");
        self.text(separator);
    }

    fn render_table_rows(
        &mut self,
        state: &TableState,
        focused: bool,
        page_start: usize,
        visible_len: usize,
        colors: &WidgetColors,
    ) {
        for idx in 0..visible_len {
            let data_idx = state.visible_indices()[page_start + idx];
            let Some(row) = state.rows.get(data_idx) else {
                continue;
            };
            let line = format_table_row(row, state.column_widths(), " │ ");
            if idx == state.selected {
                let mut style = Style::new()
                    .bg(colors.accent.unwrap_or(self.theme.selected_bg))
                    .fg(colors.fg.unwrap_or(self.theme.selected_fg));
                if focused {
                    style = style.bold();
                }
                self.styled(line, style);
            } else {
                let mut style = Style::new().fg(colors.fg.unwrap_or(self.theme.text));
                if state.zebra {
                    let zebra_bg = colors.bg.unwrap_or({
                        if idx % 2 == 0 {
                            self.theme.surface
                        } else {
                            self.theme.surface_hover
                        }
                    });
                    style = style.bg(zebra_bg);
                }
                self.styled(line, style);
            }
        }
    }

    /// Render a tab bar. Handles Left/Right navigation when focused.
    ///
    /// The active tab is rendered in the theme's primary color. If the labels
    /// list is empty, nothing is rendered.
    pub fn tabs(&mut self, state: &mut TabsState) -> Response {
        self.tabs_colored(state, &WidgetColors::new())
    }

    pub fn tabs_colored(&mut self, state: &mut TabsState, colors: &WidgetColors) -> Response {
        if state.labels.is_empty() {
            state.selected = 0;
            return Response::none();
        }

        state.selected = state.selected.min(state.labels.len().saturating_sub(1));
        let old_selected = state.selected;
        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Left => {
                            state.selected = if state.selected == 0 {
                                state.labels.len().saturating_sub(1)
                            } else {
                                state.selected - 1
                            };
                            consumed_indices.push(i);
                        }
                        KeyCode::Right => {
                            if !state.labels.is_empty() {
                                state.selected = (state.selected + 1) % state.labels.len();
                            }
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if let Some(rect) = self.prev_hit_map.get(interaction_id).copied() {
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Mouse(mouse) = event {
                    if !matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                        continue;
                    }
                    let in_bounds = mouse.x >= rect.x
                        && mouse.x < rect.right()
                        && mouse.y >= rect.y
                        && mouse.y < rect.bottom();
                    if !in_bounds {
                        continue;
                    }

                    let mut x_offset = 0u32;
                    let rel_x = mouse.x - rect.x;
                    for (idx, label) in state.labels.iter().enumerate() {
                        let tab_width = UnicodeWidthStr::width(label.as_str()) as u32 + 4;
                        if rel_x >= x_offset && rel_x < x_offset + tab_width {
                            state.selected = idx;
                            self.consumed[i] = true;
                            break;
                        }
                        x_offset += tab_width + 1;
                    }
                }
            }
        }

        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 1,
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
        });
        for (idx, label) in state.labels.iter().enumerate() {
            let style = if idx == state.selected {
                let s = Style::new()
                    .fg(colors.accent.unwrap_or(self.theme.primary))
                    .bold();
                if focused {
                    s.underline()
                } else {
                    s
                }
            } else {
                Style::new().fg(colors.fg.unwrap_or(self.theme.text_dim))
            };
            let mut tab = String::with_capacity(label.len() + 4);
            tab.push_str("[ ");
            tab.push_str(label);
            tab.push_str(" ]");
            self.styled(tab, style);
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        response.changed = state.selected != old_selected;
        response
    }

    /// Render a clickable button. Returns `true` when activated via Enter, Space, or mouse click.
    ///
    /// The button is styled with the theme's primary color when focused and the
    /// accent color when hovered.
    pub fn button(&mut self, label: impl Into<String>) -> Response {
        self.button_colored(label, &WidgetColors::new())
    }

    pub fn button_colored(&mut self, label: impl Into<String>, colors: &WidgetColors) -> Response {
        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;

        let mut activated = response.clicked;
        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        activated = true;
                        consumed_indices.push(i);
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        let hovered = response.hovered;
        let base_fg = colors.fg.unwrap_or(self.theme.text);
        let accent = colors.accent.unwrap_or(self.theme.accent);
        let base_bg = colors.bg.unwrap_or(self.theme.surface_hover);
        let style = if focused {
            Style::new().fg(accent).bold()
        } else if hovered {
            Style::new().fg(accent)
        } else {
            Style::new().fg(base_fg)
        };
        let has_custom_bg = colors.bg.is_some();
        let bg_color = if has_custom_bg || hovered || focused {
            Some(base_bg)
        } else {
            None
        };

        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(colors.border.unwrap_or(self.theme.border)),
            bg_color,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            group_name: None,
        });
        let raw_label = label.into();
        let mut label_text = String::with_capacity(raw_label.len() + 4);
        label_text.push_str("[ ");
        label_text.push_str(&raw_label);
        label_text.push_str(" ]");
        self.styled(label_text, style);
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        response.clicked = activated;
        response
    }

    /// Render a styled button variant. Returns `true` when activated.
    ///
    /// Use [`ButtonVariant::Primary`] for call-to-action, [`ButtonVariant::Danger`]
    /// for destructive actions, or [`ButtonVariant::Outline`] for secondary actions.
    pub fn button_with(&mut self, label: impl Into<String>, variant: ButtonVariant) -> Response {
        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;

        let mut activated = response.clicked;
        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        activated = true;
                        consumed_indices.push(i);
                    }
                }
            }
            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        let label = label.into();
        let hover_bg = if response.hovered || focused {
            Some(self.theme.surface_hover)
        } else {
            None
        };
        let (text, style, bg_color, border) = match variant {
            ButtonVariant::Default => {
                let style = if focused {
                    Style::new().fg(self.theme.primary).bold()
                } else if response.hovered {
                    Style::new().fg(self.theme.accent)
                } else {
                    Style::new().fg(self.theme.text)
                };
                let mut text = String::with_capacity(label.len() + 4);
                text.push_str("[ ");
                text.push_str(&label);
                text.push_str(" ]");
                (text, style, hover_bg, None)
            }
            ButtonVariant::Primary => {
                let style = if focused {
                    Style::new().fg(self.theme.bg).bg(self.theme.primary).bold()
                } else if response.hovered {
                    Style::new().fg(self.theme.bg).bg(self.theme.accent)
                } else {
                    Style::new().fg(self.theme.bg).bg(self.theme.primary)
                };
                let mut text = String::with_capacity(label.len() + 2);
                text.push(' ');
                text.push_str(&label);
                text.push(' ');
                (text, style, hover_bg, None)
            }
            ButtonVariant::Danger => {
                let style = if focused {
                    Style::new().fg(self.theme.bg).bg(self.theme.error).bold()
                } else if response.hovered {
                    Style::new().fg(self.theme.bg).bg(self.theme.warning)
                } else {
                    Style::new().fg(self.theme.bg).bg(self.theme.error)
                };
                let mut text = String::with_capacity(label.len() + 2);
                text.push(' ');
                text.push_str(&label);
                text.push(' ');
                (text, style, hover_bg, None)
            }
            ButtonVariant::Outline => {
                let border_color = if focused {
                    self.theme.primary
                } else if response.hovered {
                    self.theme.accent
                } else {
                    self.theme.border
                };
                let style = if focused {
                    Style::new().fg(self.theme.primary).bold()
                } else if response.hovered {
                    Style::new().fg(self.theme.accent)
                } else {
                    Style::new().fg(self.theme.text)
                };
                (
                    {
                        let mut text = String::with_capacity(label.len() + 2);
                        text.push(' ');
                        text.push_str(&label);
                        text.push(' ');
                        text
                    },
                    style,
                    hover_bg,
                    Some((Border::Rounded, Style::new().fg(border_color))),
                )
            }
        };

        let (btn_border, btn_border_style) = border.unwrap_or((Border::Rounded, Style::new()));
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 0,
            align: Align::Center,
            align_self: None,
            justify: Justify::Center,
            border: if border.is_some() {
                Some(btn_border)
            } else {
                None
            },
            border_sides: BorderSides::all(),
            border_style: btn_border_style,
            bg_color,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            group_name: None,
        });
        self.styled(text, style);
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        response.clicked = activated;
        response
    }

    /// Render a checkbox. Toggles the bool on Enter, Space, or click.
    ///
    /// The checked state is shown with the theme's success color. When focused,
    /// a `▸` prefix is added.
    pub fn checkbox(&mut self, label: impl Into<String>, checked: &mut bool) -> Response {
        self.checkbox_colored(label, checked, &WidgetColors::new())
    }

    pub fn checkbox_colored(
        &mut self,
        label: impl Into<String>,
        checked: &mut bool,
        colors: &WidgetColors,
    ) -> Response {
        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;
        let mut should_toggle = response.clicked;
        let old_checked = *checked;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        should_toggle = true;
                        consumed_indices.push(i);
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if should_toggle {
            *checked = !*checked;
        }

        let hover_bg = if response.hovered || focused {
            Some(self.theme.surface_hover)
        } else {
            None
        };
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 1,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(colors.border.unwrap_or(self.theme.border)),
            bg_color: hover_bg,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            group_name: None,
        });
        let marker_style = if *checked {
            Style::new().fg(colors.accent.unwrap_or(self.theme.success))
        } else {
            Style::new().fg(colors.fg.unwrap_or(self.theme.text_dim))
        };
        let marker = if *checked { "[x]" } else { "[ ]" };
        let label_text = label.into();
        if focused {
            let mut marker_text = String::with_capacity(2 + marker.len());
            marker_text.push_str("▸ ");
            marker_text.push_str(marker);
            self.styled(marker_text, marker_style.bold());
            self.styled(
                label_text,
                Style::new().fg(colors.fg.unwrap_or(self.theme.text)).bold(),
            );
        } else {
            self.styled(marker, marker_style);
            self.styled(
                label_text,
                Style::new().fg(colors.fg.unwrap_or(self.theme.text)),
            );
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        response.changed = *checked != old_checked;
        response
    }

    /// Render an on/off toggle switch.
    ///
    /// Toggles `on` when activated via Enter, Space, or click. The switch
    /// renders as `●━━ ON` or `━━● OFF` colored with the theme's success or
    /// dim color respectively.
    pub fn toggle(&mut self, label: impl Into<String>, on: &mut bool) -> Response {
        self.toggle_colored(label, on, &WidgetColors::new())
    }

    pub fn toggle_colored(
        &mut self,
        label: impl Into<String>,
        on: &mut bool,
        colors: &WidgetColors,
    ) -> Response {
        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;
        let mut should_toggle = response.clicked;
        let old_on = *on;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        should_toggle = true;
                        consumed_indices.push(i);
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if should_toggle {
            *on = !*on;
        }

        let hover_bg = if response.hovered || focused {
            Some(self.theme.surface_hover)
        } else {
            None
        };
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 2,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(colors.border.unwrap_or(self.theme.border)),
            bg_color: hover_bg,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            group_name: None,
        });
        let label_text = label.into();
        let switch = if *on { "●━━ ON" } else { "━━● OFF" };
        let switch_style = if *on {
            Style::new().fg(colors.accent.unwrap_or(self.theme.success))
        } else {
            Style::new().fg(colors.fg.unwrap_or(self.theme.text_dim))
        };
        if focused {
            let mut focused_label = String::with_capacity(2 + label_text.len());
            focused_label.push_str("▸ ");
            focused_label.push_str(&label_text);
            self.styled(
                focused_label,
                Style::new().fg(colors.fg.unwrap_or(self.theme.text)).bold(),
            );
            self.styled(switch, switch_style.bold());
        } else {
            self.styled(
                label_text,
                Style::new().fg(colors.fg.unwrap_or(self.theme.text)),
            );
            self.styled(switch, switch_style);
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        response.changed = *on != old_on;
        response
    }

    // ── select / dropdown ─────────────────────────────────────────────

    /// Render a dropdown select. Shows the selected item; expands on activation.
    ///
    /// Returns `true` when the selection changed this frame.
    pub fn select(&mut self, state: &mut SelectState) -> Response {
        self.select_colored(state, &WidgetColors::new())
    }

    pub fn select_colored(&mut self, state: &mut SelectState, colors: &WidgetColors) -> Response {
        if state.items.is_empty() {
            return Response::none();
        }
        state.selected = state.selected.min(state.items.len().saturating_sub(1));

        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;
        let old_selected = state.selected;

        self.select_handle_events(state, focused, response.clicked);
        self.select_render(state, focused, colors);
        response.changed = state.selected != old_selected;
        response
    }

    fn select_handle_events(&mut self, state: &mut SelectState, focused: bool, clicked: bool) {
        if clicked {
            state.open = !state.open;
            if state.open {
                state.set_cursor(state.selected);
            }
        }

        if !focused {
            return;
        }

        let mut consumed_indices = Vec::new();
        for (i, event) in self.events.iter().enumerate() {
            if self.consumed[i] {
                continue;
            }
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if state.open {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                            let mut cursor = state.cursor();
                            let _ = handle_vertical_nav(
                                &mut cursor,
                                state.items.len().saturating_sub(1),
                                key.code.clone(),
                            );
                            state.set_cursor(cursor);
                            consumed_indices.push(i);
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            state.selected = state.cursor();
                            state.open = false;
                            consumed_indices.push(i);
                        }
                        KeyCode::Esc => {
                            state.open = false;
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                } else if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                    state.open = true;
                    state.set_cursor(state.selected);
                    consumed_indices.push(i);
                }
            }
        }
        for idx in consumed_indices {
            self.consumed[idx] = true;
        }
    }

    fn select_render(&mut self, state: &SelectState, focused: bool, colors: &WidgetColors) {
        let border_color = if focused {
            colors.accent.unwrap_or(self.theme.primary)
        } else {
            colors.border.unwrap_or(self.theme.border)
        };
        let display_text = state
            .items
            .get(state.selected)
            .cloned()
            .unwrap_or_else(|| state.placeholder.clone());
        let arrow = if state.open { "▲" } else { "▼" };

        self.commands.push(Command::BeginContainer {
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
        });

        self.render_select_trigger(&display_text, arrow, border_color, colors);

        if state.open {
            self.render_select_dropdown(state, colors);
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
    }

    fn render_select_trigger(
        &mut self,
        display_text: &str,
        arrow: &str,
        border_color: Color,
        colors: &WidgetColors,
    ) {
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 1,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            border: Some(Border::Rounded),
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(border_color),
            bg_color: None,
            padding: Padding {
                left: 1,
                right: 1,
                top: 0,
                bottom: 0,
            },
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            group_name: None,
        });
        self.interaction_count += 1;
        self.styled(
            display_text,
            Style::new().fg(colors.fg.unwrap_or(self.theme.text)),
        );
        self.styled(
            arrow,
            Style::new().fg(colors.fg.unwrap_or(self.theme.text_dim)),
        );
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
    }

    fn render_select_dropdown(&mut self, state: &SelectState, colors: &WidgetColors) {
        for (idx, item) in state.items.iter().enumerate() {
            let is_cursor = idx == state.cursor();
            let style = if is_cursor {
                Style::new()
                    .bold()
                    .fg(colors.accent.unwrap_or(self.theme.primary))
            } else {
                Style::new().fg(colors.fg.unwrap_or(self.theme.text))
            };
            let prefix = if is_cursor { "▸ " } else { "  " };
            let mut row = String::with_capacity(prefix.len() + item.len());
            row.push_str(prefix);
            row.push_str(item);
            self.styled(row, style);
        }
    }

    // ── radio ────────────────────────────────────────────────────────

    /// Render a radio button group. Returns `true` when selection changed.
    pub fn radio(&mut self, state: &mut RadioState) -> Response {
        self.radio_colored(state, &WidgetColors::new())
    }

    pub fn radio_colored(&mut self, state: &mut RadioState, colors: &WidgetColors) -> Response {
        if state.items.is_empty() {
            return Response::none();
        }
        state.selected = state.selected.min(state.items.len().saturating_sub(1));
        let focused = self.register_focusable();
        let old_selected = state.selected;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                            let _ = handle_vertical_nav(
                                &mut state.selected,
                                state.items.len().saturating_sub(1),
                                key.code.clone(),
                            );
                            consumed_indices.push(i);
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }
            for idx in consumed_indices {
                self.consumed[idx] = true;
            }
        }

        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;

        if let Some(rect) = self.prev_hit_map.get(interaction_id).copied() {
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Mouse(mouse) = event {
                    if !matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                        continue;
                    }
                    let in_bounds = mouse.x >= rect.x
                        && mouse.x < rect.right()
                        && mouse.y >= rect.y
                        && mouse.y < rect.bottom();
                    if !in_bounds {
                        continue;
                    }
                    let clicked_idx = (mouse.y - rect.y) as usize;
                    if clicked_idx < state.items.len() {
                        state.selected = clicked_idx;
                        self.consumed[i] = true;
                    }
                }
            }
        }

        self.commands.push(Command::BeginContainer {
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
        });

        for (idx, item) in state.items.iter().enumerate() {
            let is_selected = idx == state.selected;
            let marker = if is_selected { "●" } else { "○" };
            let style = if is_selected {
                if focused {
                    Style::new()
                        .bold()
                        .fg(colors.accent.unwrap_or(self.theme.primary))
                } else {
                    Style::new().fg(colors.accent.unwrap_or(self.theme.primary))
                }
            } else {
                Style::new().fg(colors.fg.unwrap_or(self.theme.text))
            };
            let prefix = if focused && idx == state.selected {
                "▸ "
            } else {
                "  "
            };
            let mut row = String::with_capacity(prefix.len() + marker.len() + item.len() + 1);
            row.push_str(prefix);
            row.push_str(marker);
            row.push(' ');
            row.push_str(item);
            self.styled(row, style);
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        response.changed = state.selected != old_selected;
        response
    }

    // ── multi-select ─────────────────────────────────────────────────

    /// Render a multi-select list. Space toggles, Up/Down navigates.
    pub fn multi_select(&mut self, state: &mut MultiSelectState) -> Response {
        if state.items.is_empty() {
            return Response::none();
        }
        state.cursor = state.cursor.min(state.items.len().saturating_sub(1));
        let focused = self.register_focusable();
        let old_selected = state.selected.clone();

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                            let _ = handle_vertical_nav(
                                &mut state.cursor,
                                state.items.len().saturating_sub(1),
                                key.code.clone(),
                            );
                            consumed_indices.push(i);
                        }
                        KeyCode::Char(' ') | KeyCode::Enter => {
                            state.toggle(state.cursor);
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }
            for idx in consumed_indices {
                self.consumed[idx] = true;
            }
        }

        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;

        if let Some(rect) = self.prev_hit_map.get(interaction_id).copied() {
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Mouse(mouse) = event {
                    if !matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                        continue;
                    }
                    let in_bounds = mouse.x >= rect.x
                        && mouse.x < rect.right()
                        && mouse.y >= rect.y
                        && mouse.y < rect.bottom();
                    if !in_bounds {
                        continue;
                    }
                    let clicked_idx = (mouse.y - rect.y) as usize;
                    if clicked_idx < state.items.len() {
                        state.toggle(clicked_idx);
                        state.cursor = clicked_idx;
                        self.consumed[i] = true;
                    }
                }
            }
        }

        self.commands.push(Command::BeginContainer {
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
        });

        for (idx, item) in state.items.iter().enumerate() {
            let checked = state.selected.contains(&idx);
            let marker = if checked { "[x]" } else { "[ ]" };
            let is_cursor = idx == state.cursor;
            let style = if is_cursor && focused {
                Style::new().bold().fg(self.theme.primary)
            } else if checked {
                Style::new().fg(self.theme.success)
            } else {
                Style::new().fg(self.theme.text)
            };
            let prefix = if is_cursor && focused { "▸ " } else { "  " };
            let mut row = String::with_capacity(prefix.len() + marker.len() + item.len() + 1);
            row.push_str(prefix);
            row.push_str(marker);
            row.push(' ');
            row.push_str(item);
            self.styled(row, style);
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        response.changed = state.selected != old_selected;
        response
    }

    // ── tree ─────────────────────────────────────────────────────────

    /// Render a tree view. Left/Right to collapse/expand, Up/Down to navigate.
    pub fn tree(&mut self, state: &mut TreeState) -> Response {
        let entries = state.flatten();
        if entries.is_empty() {
            return Response::none();
        }
        state.selected = state.selected.min(entries.len().saturating_sub(1));
        let old_selected = state.selected;
        let focused = self.register_focusable();
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;
        let mut changed = false;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                            let max_index = state.flatten().len().saturating_sub(1);
                            let _ = handle_vertical_nav(
                                &mut state.selected,
                                max_index,
                                key.code.clone(),
                            );
                            changed = changed || state.selected != old_selected;
                            consumed_indices.push(i);
                        }
                        KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => {
                            state.toggle_at(state.selected);
                            changed = true;
                            consumed_indices.push(i);
                        }
                        KeyCode::Left => {
                            let entry = &entries[state.selected.min(entries.len() - 1)];
                            if entry.expanded {
                                state.toggle_at(state.selected);
                                changed = true;
                            }
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }
            for idx in consumed_indices {
                self.consumed[idx] = true;
            }
        }

        self.commands.push(Command::BeginContainer {
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
        });

        let entries = state.flatten();
        for (idx, entry) in entries.iter().enumerate() {
            let indent = "  ".repeat(entry.depth);
            let icon = if entry.is_leaf {
                "  "
            } else if entry.expanded {
                "▾ "
            } else {
                "▸ "
            };
            let is_selected = idx == state.selected;
            let style = if is_selected && focused {
                Style::new().bold().fg(self.theme.primary)
            } else if is_selected {
                Style::new().fg(self.theme.primary)
            } else {
                Style::new().fg(self.theme.text)
            };
            let cursor = if is_selected && focused { "▸" } else { " " };
            let mut row =
                String::with_capacity(cursor.len() + indent.len() + icon.len() + entry.label.len());
            row.push_str(cursor);
            row.push_str(&indent);
            row.push_str(icon);
            row.push_str(&entry.label);
            self.styled(row, style);
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        response.changed = changed || state.selected != old_selected;
        response
    }

    // ── virtual list ─────────────────────────────────────────────────

    /// Render a virtual list that only renders visible items.
    ///
    /// `total` is the number of items. `visible_height` limits how many rows
    /// are rendered. The closure `f` is called only for visible indices.
    pub fn virtual_list(
        &mut self,
        state: &mut ListState,
        visible_height: usize,
        f: impl Fn(&mut Context, usize),
    ) -> Response {
        if state.items.is_empty() {
            return Response::none();
        }
        state.selected = state.selected.min(state.items.len().saturating_sub(1));
        let interaction_id = self.next_interaction_id();
        let focused = self.register_focusable();
        let old_selected = state.selected;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                            let _ = handle_vertical_nav(
                                &mut state.selected,
                                state.items.len().saturating_sub(1),
                                key.code.clone(),
                            );
                            consumed_indices.push(i);
                        }
                        KeyCode::PageUp => {
                            state.selected = state.selected.saturating_sub(visible_height);
                            consumed_indices.push(i);
                        }
                        KeyCode::PageDown => {
                            state.selected = (state.selected + visible_height)
                                .min(state.items.len().saturating_sub(1));
                            consumed_indices.push(i);
                        }
                        KeyCode::Home => {
                            state.selected = 0;
                            consumed_indices.push(i);
                        }
                        KeyCode::End => {
                            state.selected = state.items.len().saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }
            for idx in consumed_indices {
                self.consumed[idx] = true;
            }
        }

        let start = if state.selected >= visible_height {
            state.selected - visible_height + 1
        } else {
            0
        };
        let end = (start + visible_height).min(state.items.len());

        self.commands.push(Command::BeginContainer {
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
        });

        if start > 0 {
            let hidden = start.to_string();
            let mut line = String::with_capacity(hidden.len() + 10);
            line.push_str("  ↑ ");
            line.push_str(&hidden);
            line.push_str(" more");
            self.styled(line, Style::new().fg(self.theme.text_dim).dim());
        }

        for idx in start..end {
            f(self, idx);
        }

        let remaining = state.items.len().saturating_sub(end);
        if remaining > 0 {
            let hidden = remaining.to_string();
            let mut line = String::with_capacity(hidden.len() + 10);
            line.push_str("  ↓ ");
            line.push_str(&hidden);
            line.push_str(" more");
            self.styled(line, Style::new().fg(self.theme.text_dim).dim());
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        let mut response = self.response_for(interaction_id);
        response.focused = focused;
        response.changed = state.selected != old_selected;
        response
    }

    // ── command palette ──────────────────────────────────────────────

    /// Render a command palette overlay.
    pub fn command_palette(&mut self, state: &mut CommandPaletteState) -> Response {
        if !state.open {
            return Response::none();
        }

        state.last_selected = None;
        let interaction_id = self.next_interaction_id();

        let filtered = state.filtered_indices();
        let sel = state.selected().min(filtered.len().saturating_sub(1));
        state.set_selected(sel);

        let mut consumed_indices = Vec::new();

        for (i, event) in self.events.iter().enumerate() {
            if self.consumed[i] {
                continue;
            }
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Esc => {
                        state.open = false;
                        consumed_indices.push(i);
                    }
                    KeyCode::Up => {
                        let s = state.selected();
                        state.set_selected(s.saturating_sub(1));
                        consumed_indices.push(i);
                    }
                    KeyCode::Down => {
                        let s = state.selected();
                        state.set_selected((s + 1).min(filtered.len().saturating_sub(1)));
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter => {
                        if let Some(&cmd_idx) = filtered.get(state.selected()) {
                            state.last_selected = Some(cmd_idx);
                            state.open = false;
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Backspace => {
                        if state.cursor > 0 {
                            let byte_idx = byte_index_for_char(&state.input, state.cursor - 1);
                            let end_idx = byte_index_for_char(&state.input, state.cursor);
                            state.input.replace_range(byte_idx..end_idx, "");
                            state.cursor -= 1;
                            state.set_selected(0);
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Char(ch) => {
                        let byte_idx = byte_index_for_char(&state.input, state.cursor);
                        state.input.insert(byte_idx, ch);
                        state.cursor += 1;
                        state.set_selected(0);
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
        }
        for idx in consumed_indices {
            self.consumed[idx] = true;
        }

        let filtered = state.filtered_indices();

        let _ = self.modal(|ui| {
            let primary = ui.theme.primary;
            let _ = ui
                .container()
                .border(Border::Rounded)
                .border_style(Style::new().fg(primary))
                .pad(1)
                .max_w(60)
                .col(|ui| {
                    let border_color = ui.theme.primary;
                    let _ = ui
                        .bordered(Border::Rounded)
                        .border_style(Style::new().fg(border_color))
                        .px(1)
                        .col(|ui| {
                            let display = if state.input.is_empty() {
                                "Type to search...".to_string()
                            } else {
                                state.input.clone()
                            };
                            let style = if state.input.is_empty() {
                                Style::new().dim().fg(ui.theme.text_dim)
                            } else {
                                Style::new().fg(ui.theme.text)
                            };
                            ui.styled(display, style);
                        });

                    for (list_idx, &cmd_idx) in filtered.iter().enumerate() {
                        let cmd = &state.commands[cmd_idx];
                        let is_selected = list_idx == state.selected();
                        let style = if is_selected {
                            Style::new().bold().fg(ui.theme.primary)
                        } else {
                            Style::new().fg(ui.theme.text)
                        };
                        let prefix = if is_selected { "▸ " } else { "  " };
                        let shortcut_text = cmd
                            .shortcut
                            .as_deref()
                            .map(|s| {
                                let mut text = String::with_capacity(s.len() + 4);
                                text.push_str("  (");
                                text.push_str(s);
                                text.push(')');
                                text
                            })
                            .unwrap_or_default();
                        let mut line = String::with_capacity(
                            prefix.len() + cmd.label.len() + shortcut_text.len(),
                        );
                        line.push_str(prefix);
                        line.push_str(&cmd.label);
                        line.push_str(&shortcut_text);
                        ui.styled(line, style);
                        if is_selected && !cmd.description.is_empty() {
                            let mut desc = String::with_capacity(4 + cmd.description.len());
                            desc.push_str("    ");
                            desc.push_str(&cmd.description);
                            ui.styled(desc, Style::new().dim().fg(ui.theme.text_dim));
                        }
                    }

                    if filtered.is_empty() {
                        ui.styled(
                            "  No matching commands",
                            Style::new().dim().fg(ui.theme.text_dim),
                        );
                    }
                });
        });

        let mut response = self.response_for(interaction_id);
        response.changed = state.last_selected.is_some();
        response
    }

    // ── markdown ─────────────────────────────────────────────────────

    /// Render a markdown string with basic formatting.
    ///
    /// Supports headers (`#`), bold (`**`), italic (`*`), inline code (`` ` ``),
    /// unordered lists (`-`/`*`), ordered lists (`1.`), and horizontal rules (`---`).
    pub fn markdown(&mut self, text: &str) -> Response {
        self.commands.push(Command::BeginContainer {
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
        });
        self.interaction_count += 1;

        let text_style = Style::new().fg(self.theme.text);
        let bold_style = Style::new().fg(self.theme.text).bold();
        let code_style = Style::new().fg(self.theme.accent);

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                self.text(" ");
                continue;
            }
            if trimmed == "---" || trimmed == "***" || trimmed == "___" {
                self.styled("─".repeat(40), Style::new().fg(self.theme.border).dim());
                continue;
            }
            if let Some(heading) = trimmed.strip_prefix("### ") {
                self.styled(heading, Style::new().bold().fg(self.theme.accent));
            } else if let Some(heading) = trimmed.strip_prefix("## ") {
                self.styled(heading, Style::new().bold().fg(self.theme.secondary));
            } else if let Some(heading) = trimmed.strip_prefix("# ") {
                self.styled(heading, Style::new().bold().fg(self.theme.primary));
            } else if let Some(item) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
            {
                let segs = Self::parse_inline_segments(item, text_style, bold_style, code_style);
                if segs.len() <= 1 {
                    let mut line = String::with_capacity(4 + item.len());
                    line.push_str("  • ");
                    line.push_str(item);
                    self.styled(line, text_style);
                } else {
                    self.line(|ui| {
                        ui.styled("  • ", text_style);
                        for (s, st) in segs {
                            ui.styled(s, st);
                        }
                    });
                }
            } else if trimmed.starts_with(|c: char| c.is_ascii_digit()) && trimmed.contains(". ") {
                let parts: Vec<&str> = trimmed.splitn(2, ". ").collect();
                if parts.len() == 2 {
                    let segs =
                        Self::parse_inline_segments(parts[1], text_style, bold_style, code_style);
                    if segs.len() <= 1 {
                        let mut line = String::with_capacity(4 + parts[0].len() + parts[1].len());
                        line.push_str("  ");
                        line.push_str(parts[0]);
                        line.push_str(". ");
                        line.push_str(parts[1]);
                        self.styled(line, text_style);
                    } else {
                        self.line(|ui| {
                            let mut prefix = String::with_capacity(4 + parts[0].len());
                            prefix.push_str("  ");
                            prefix.push_str(parts[0]);
                            prefix.push_str(". ");
                            ui.styled(prefix, text_style);
                            for (s, st) in segs {
                                ui.styled(s, st);
                            }
                        });
                    }
                } else {
                    self.text(trimmed);
                }
            } else if let Some(code) = trimmed.strip_prefix("```") {
                let _ = code;
                self.styled("  ┌─code─", Style::new().fg(self.theme.border).dim());
            } else {
                let segs = Self::parse_inline_segments(trimmed, text_style, bold_style, code_style);
                if segs.len() <= 1 {
                    self.styled(trimmed, text_style);
                } else {
                    self.line(|ui| {
                        for (s, st) in segs {
                            ui.styled(s, st);
                        }
                    });
                }
            }
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        Response::none()
    }

    pub(crate) fn parse_inline_segments(
        text: &str,
        base: Style,
        bold: Style,
        code: Style,
    ) -> Vec<(String, Style)> {
        let mut segments: Vec<(String, Style)> = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
                let rest: String = chars[i + 2..].iter().collect();
                if let Some(end) = rest.find("**") {
                    if !current.is_empty() {
                        segments.push((std::mem::take(&mut current), base));
                    }
                    let inner: String = rest[..end].to_string();
                    let char_count = inner.chars().count();
                    segments.push((inner, bold));
                    i += 2 + char_count + 2;
                    continue;
                }
            }
            if chars[i] == '*'
                && (i + 1 >= chars.len() || chars[i + 1] != '*')
                && (i == 0 || chars[i - 1] != '*')
            {
                let rest: String = chars[i + 1..].iter().collect();
                if let Some(end) = rest.find('*') {
                    if !current.is_empty() {
                        segments.push((std::mem::take(&mut current), base));
                    }
                    let inner: String = rest[..end].to_string();
                    let char_count = inner.chars().count();
                    segments.push((inner, base.italic()));
                    i += 1 + char_count + 1;
                    continue;
                }
            }
            if chars[i] == '`' {
                let rest: String = chars[i + 1..].iter().collect();
                if let Some(end) = rest.find('`') {
                    if !current.is_empty() {
                        segments.push((std::mem::take(&mut current), base));
                    }
                    let inner: String = rest[..end].to_string();
                    let char_count = inner.chars().count();
                    segments.push((inner, code));
                    i += 1 + char_count + 1;
                    continue;
                }
            }
            current.push(chars[i]);
            i += 1;
        }
        if !current.is_empty() {
            segments.push((current, base));
        }
        segments
    }

    // ── key sequence ─────────────────────────────────────────────────

    /// Check if a sequence of character keys was pressed across recent frames.
    ///
    /// Matches when each character in `seq` appears in consecutive unconsumed
    /// key events within this frame. For single-frame sequences only (e.g., "gg").
    pub fn key_seq(&self, seq: &str) -> bool {
        if seq.is_empty() {
            return false;
        }
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        let target: Vec<char> = seq.chars().collect();
        let mut matched = 0;
        for (i, event) in self.events.iter().enumerate() {
            if self.consumed[i] {
                continue;
            }
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if let KeyCode::Char(c) = key.code {
                    if c == target[matched] {
                        matched += 1;
                        if matched == target.len() {
                            return true;
                        }
                    } else {
                        matched = 0;
                        if c == target[0] {
                            matched = 1;
                        }
                    }
                }
            }
        }
        false
    }

    /// Render a horizontal divider line.
    ///
    /// The line is drawn with the theme's border color and expands to fill the
    /// container width.
    pub fn separator(&mut self) -> &mut Self {
        self.commands.push(Command::Text {
            content: "─".repeat(200),
            style: Style::new().fg(self.theme.border).dim(),
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

    pub fn separator_colored(&mut self, color: Color) -> &mut Self {
        self.commands.push(Command::Text {
            content: "─".repeat(200),
            style: Style::new().fg(color),
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

    /// Render a help bar showing keybinding hints.
    ///
    /// `bindings` is a slice of `(key, action)` pairs. Keys are rendered in the
    /// theme's primary color; actions in the dim text color. Pairs are separated
    /// by a `·` character.
    pub fn help(&mut self, bindings: &[(&str, &str)]) -> Response {
        if bindings.is_empty() {
            return Response::none();
        }

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 2,
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
        });
        for (idx, (key, action)) in bindings.iter().enumerate() {
            if idx > 0 {
                self.styled("·", Style::new().fg(self.theme.text_dim));
            }
            self.styled(*key, Style::new().bold().fg(self.theme.primary));
            self.styled(*action, Style::new().fg(self.theme.text_dim));
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        Response::none()
    }

    pub fn help_colored(
        &mut self,
        bindings: &[(&str, &str)],
        key_color: Color,
        text_color: Color,
    ) -> Response {
        if bindings.is_empty() {
            return Response::none();
        }

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 2,
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
        });
        for (idx, (key, action)) in bindings.iter().enumerate() {
            if idx > 0 {
                self.styled("·", Style::new().fg(text_color));
            }
            self.styled(*key, Style::new().bold().fg(key_color));
            self.styled(*action, Style::new().fg(text_color));
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        Response::none()
    }

    // ── events ───────────────────────────────────────────────────────

    /// Check if a character key was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    pub fn key(&self, c: char) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.kind == KeyEventKind::Press && k.code == KeyCode::Char(c))
        })
    }

    /// Check if a specific key code was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    pub fn key_code(&self, code: KeyCode) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.kind == KeyEventKind::Press && k.code == code)
        })
    }

    /// Check if a character key was released this frame.
    ///
    /// Returns `true` if the key release event has not been consumed by another widget.
    pub fn key_release(&self, c: char) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.kind == KeyEventKind::Release && k.code == KeyCode::Char(c))
        })
    }

    /// Check if a specific key code was released this frame.
    ///
    /// Returns `true` if the key release event has not been consumed by another widget.
    pub fn key_code_release(&self, code: KeyCode) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.kind == KeyEventKind::Release && k.code == code)
        })
    }

    /// Check for a character key press and consume the event, preventing other
    /// handlers from seeing it.
    ///
    /// Returns `true` if the key was found unconsumed and is now consumed.
    /// Unlike [`key()`](Self::key) which peeks without consuming, this claims
    /// exclusive ownership of the event.
    ///
    /// Call **after** widgets if you want widgets to have priority over your
    /// handler, or **before** widgets to intercept first.
    pub fn consume_key(&mut self, c: char) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        for (i, event) in self.events.iter().enumerate() {
            if self.consumed[i] {
                continue;
            }
            if matches!(event, Event::Key(k) if k.kind == KeyEventKind::Press && k.code == KeyCode::Char(c))
            {
                self.consumed[i] = true;
                return true;
            }
        }
        false
    }

    /// Check for a special key press and consume the event, preventing other
    /// handlers from seeing it.
    ///
    /// Returns `true` if the key was found unconsumed and is now consumed.
    /// Unlike [`key_code()`](Self::key_code) which peeks without consuming,
    /// this claims exclusive ownership of the event.
    ///
    /// Call **after** widgets if you want widgets to have priority over your
    /// handler, or **before** widgets to intercept first.
    pub fn consume_key_code(&mut self, code: KeyCode) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        for (i, event) in self.events.iter().enumerate() {
            if self.consumed[i] {
                continue;
            }
            if matches!(event, Event::Key(k) if k.kind == KeyEventKind::Press && k.code == code) {
                self.consumed[i] = true;
                return true;
            }
        }
        false
    }

    /// Check if a character key with specific modifiers was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    pub fn key_mod(&self, c: char, modifiers: KeyModifiers) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.kind == KeyEventKind::Press && k.code == KeyCode::Char(c) && k.modifiers.contains(modifiers))
        })
    }

    /// Return the position of a left mouse button down event this frame, if any.
    ///
    /// Returns `None` if no unconsumed mouse-down event occurred.
    pub fn mouse_down(&self) -> Option<(u32, u32)> {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return None;
        }
        self.events.iter().enumerate().find_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            if let Event::Mouse(mouse) = event {
                if matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                    return Some((mouse.x, mouse.y));
                }
            }
            None
        })
    }

    /// Return the current mouse cursor position, if known.
    ///
    /// The position is updated on every mouse move or click event. Returns
    /// `None` until the first mouse event is received.
    pub fn mouse_pos(&self) -> Option<(u32, u32)> {
        self.mouse_pos
    }

    /// Return the first unconsumed paste event text, if any.
    pub fn paste(&self) -> Option<&str> {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return None;
        }
        self.events.iter().enumerate().find_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            if let Event::Paste(ref text) = event {
                return Some(text.as_str());
            }
            None
        })
    }

    /// Check if an unconsumed scroll-up event occurred this frame.
    pub fn scroll_up(&self) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        self.events.iter().enumerate().any(|(i, event)| {
            !self.consumed[i]
                && matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollUp))
        })
    }

    /// Check if an unconsumed scroll-down event occurred this frame.
    pub fn scroll_down(&self) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        self.events.iter().enumerate().any(|(i, event)| {
            !self.consumed[i]
                && matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollDown))
        })
    }

    /// Signal the run loop to exit after this frame.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Copy text to the system clipboard via OSC 52.
    ///
    /// Works transparently over SSH connections. The text is queued and
    /// written to the terminal after the current frame renders.
    ///
    /// Requires a terminal that supports OSC 52 (most modern terminals:
    /// Ghostty, kitty, WezTerm, iTerm2, Windows Terminal).
    pub fn copy_to_clipboard(&mut self, text: impl Into<String>) {
        self.clipboard_text = Some(text.into());
    }

    /// Get the current theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Change the theme for subsequent rendering.
    ///
    /// All widgets rendered after this call will use the new theme's colors.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Check if dark mode is active.
    pub fn is_dark_mode(&self) -> bool {
        self.dark_mode
    }

    /// Set dark mode. When true, dark_* style variants are applied.
    pub fn set_dark_mode(&mut self, dark: bool) {
        self.dark_mode = dark;
    }

    // ── info ─────────────────────────────────────────────────────────

    /// Get the terminal width in cells.
    pub fn width(&self) -> u32 {
        self.area_width
    }

    /// Get the current terminal width breakpoint.
    ///
    /// Returns a [`Breakpoint`] based on the terminal width:
    /// - `Xs`: < 40 columns
    /// - `Sm`: 40-79 columns
    /// - `Md`: 80-119 columns
    /// - `Lg`: 120-159 columns
    /// - `Xl`: >= 160 columns
    ///
    /// Use this for responsive layouts that adapt to terminal size:
    /// ```no_run
    /// # use slt::{Breakpoint, Context};
    /// # slt::run(|ui: &mut Context| {
    /// match ui.breakpoint() {
    ///     Breakpoint::Xs | Breakpoint::Sm => {
    ///         ui.col(|ui| { ui.text("Stacked layout"); });
    ///     }
    ///     _ => {
    ///         ui.row(|ui| { ui.text("Side-by-side layout"); });
    ///     }
    /// }
    /// # });
    /// ```
    pub fn breakpoint(&self) -> Breakpoint {
        let w = self.area_width;
        if w < 40 {
            Breakpoint::Xs
        } else if w < 80 {
            Breakpoint::Sm
        } else if w < 120 {
            Breakpoint::Md
        } else if w < 160 {
            Breakpoint::Lg
        } else {
            Breakpoint::Xl
        }
    }

    /// Get the terminal height in cells.
    pub fn height(&self) -> u32 {
        self.area_height
    }

    /// Get the current tick count (increments each frame).
    ///
    /// Useful for animations and time-based logic. The tick starts at 0 and
    /// increases by 1 on every rendered frame.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Return whether the layout debugger is enabled.
    ///
    /// The debugger is toggled with F12 at runtime.
    pub fn debug_enabled(&self) -> bool {
        self.debug
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
