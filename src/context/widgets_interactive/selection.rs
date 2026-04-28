use super::*;

impl Context {
    /// Render a data table with sortable columns and row selection.
    ///
    /// Handles Up/Down selection when focused. Column widths are computed
    /// automatically from header and cell content. The selected row is
    /// highlighted with the theme's selection colors.
    pub fn table(&mut self, state: &mut TableState) -> Response {
        let colors = self.widget_theme.table;
        self.table_colored(state, &colors)
    }

    /// Render a data table with custom widget colors.
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
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);

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

        if let Some((rect, clicks)) = self.left_clicks_for_interaction(interaction_id) {
            let mut consumed = Vec::new();
            for (i, mouse) in clicks {
                if mouse.y == rect.y {
                    let rel_x = mouse.x.saturating_sub(rect.x);
                    let mut x_offset = 0u32;
                    for (col_idx, width) in state.column_widths().iter().enumerate() {
                        if rel_x >= x_offset && rel_x < x_offset + *width {
                            state.toggle_sort(col_idx);
                            state.selected = 0;
                            consumed.push(i);
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
                    consumed.push(i);
                }
            }
            self.consume_indices(consumed);
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
        self.rollback.last_text_idx = None;
    }

    fn handle_table_keys(&mut self, state: &mut TableState, focused: bool) {
        if !focused || state.visible_indices().is_empty() {
            return;
        }

        let mut consumed_indices = Vec::new();
        for (i, key) in self.available_key_presses() {
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
        self.consume_indices(consumed_indices);
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

    /// Render a horizontal tab bar. Handles Left/Right navigation when focused.
    ///
    /// The active tab is rendered in the theme's primary color. If the labels
    /// list is empty, nothing is rendered.
    pub fn tabs(&mut self, state: &mut TabsState) -> Response {
        let colors = self.widget_theme.tabs;
        self.tabs_colored(state, &colors)
    }

    /// Render a horizontal tab bar with custom widget colors.
    pub fn tabs_colored(&mut self, state: &mut TabsState, colors: &WidgetColors) -> Response {
        if state.labels.is_empty() {
            state.selected = 0;
            return Response::none();
        }

        state.selected = state.selected.min(state.labels.len().saturating_sub(1));
        let old_selected = state.selected;
        let focused = self.register_focusable();
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
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
            self.consume_indices(consumed_indices);
        }

        if let Some((rect, clicks)) = self.left_clicks_for_interaction(interaction_id) {
            let mut consumed = Vec::new();
            for (i, mouse) in clicks {
                let mut x_offset = 0u32;
                let rel_x = mouse.x.saturating_sub(rect.x);
                for (idx, label) in state.labels.iter().enumerate() {
                    let tab_width = UnicodeWidthStr::width(label.as_str()) as u32 + 4;
                    if rel_x >= x_offset && rel_x < x_offset + tab_width {
                        state.selected = idx;
                        consumed.push(i);
                        break;
                    }
                    x_offset += tab_width + 1;
                }
            }
            self.consume_indices(consumed);
        }

        let tabs_gap = self.theme.spacing.xs();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Row,
                gap: tabs_gap,
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
        self.rollback.last_text_idx = None;

        response.changed = state.selected != old_selected;
        response
    }

    /// Render a clickable button. Activation fires via Enter, Space, or mouse click.
    ///
    /// The returned [`Response::clicked`] flag is set on activation. The button
    /// is styled with the theme's primary color when focused and the accent
    /// color when hovered.
    pub fn button(&mut self, label: impl Into<String>) -> Response {
        let colors = self.widget_theme.button;
        self.button_colored(label, &colors)
    }

    /// Render a clickable button with custom widget colors.
    pub fn button_colored(&mut self, label: impl Into<String>, colors: &WidgetColors) -> Response {
        let focused = self.register_focusable();
        let (_interaction_id, mut response) = self.begin_widget_interaction(focused);

        let activated = response.clicked || self.consume_activation_keys(focused);

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

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
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
            })));
        let raw_label = label.into();
        let mut label_text = String::with_capacity(raw_label.len() + 4);
        label_text.push_str("[ ");
        label_text.push_str(&raw_label);
        label_text.push_str(" ]");
        self.styled(label_text, style);
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        response.clicked = activated;
        response
    }

    /// Render a styled button variant. Returns `true` when activated.
    ///
    /// Use [`ButtonVariant::Primary`] for call-to-action, [`ButtonVariant::Danger`]
    /// for destructive actions, or [`ButtonVariant::Outline`] for secondary actions.
    pub fn button_with(&mut self, label: impl Into<String>, variant: ButtonVariant) -> Response {
        let focused = self.register_focusable();
        let (_interaction_id, mut response) = self.begin_widget_interaction(focused);

        let activated = response.clicked || self.consume_activation_keys(focused);

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
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
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
            })));
        self.styled(text, style);
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        response.clicked = activated;
        response
    }

    /// Render a checkbox. Toggles the bool on Enter, Space, or click.
    ///
    /// The checked state is shown with the theme's success color. When focused,
    /// a `▸` prefix is added.
    /// Render a checkbox toggle.
    pub fn checkbox(&mut self, label: impl Into<String>, checked: &mut bool) -> Response {
        let colors = self.widget_theme.checkbox;
        self.checkbox_colored(label, checked, &colors)
    }

    /// Render a checkbox toggle with custom widget colors.
    pub fn checkbox_colored(
        &mut self,
        label: impl Into<String>,
        checked: &mut bool,
        colors: &WidgetColors,
    ) -> Response {
        let focused = self.register_focusable();
        let (_interaction_id, mut response) = self.begin_widget_interaction(focused);
        let mut should_toggle = response.clicked;
        let old_checked = *checked;

        should_toggle |= self.consume_activation_keys(focused);

        if should_toggle {
            *checked = !*checked;
        }

        let hover_bg = if response.hovered || focused {
            Some(self.theme.surface_hover)
        } else {
            None
        };
        let cb_gap = self.theme.spacing.xs();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Row,
                gap: cb_gap,
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
            })));
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
        self.rollback.last_text_idx = None;

        response.changed = *checked != old_checked;
        response
    }

    /// Render an on/off toggle switch.
    ///
    /// Toggles `on` when activated via Enter, Space, or click. The switch
    /// renders as `●━━ ON` or `━━● OFF` colored with the theme's success or
    /// dim color respectively.
    /// Render an on/off toggle switch.
    pub fn toggle(&mut self, label: impl Into<String>, on: &mut bool) -> Response {
        let colors = self.widget_theme.toggle;
        self.toggle_colored(label, on, &colors)
    }

    /// Render an on/off toggle switch with custom widget colors.
    pub fn toggle_colored(
        &mut self,
        label: impl Into<String>,
        on: &mut bool,
        colors: &WidgetColors,
    ) -> Response {
        let focused = self.register_focusable();
        let (_interaction_id, mut response) = self.begin_widget_interaction(focused);
        let mut should_toggle = response.clicked;
        let old_on = *on;

        should_toggle |= self.consume_activation_keys(focused);

        if should_toggle {
            *on = !*on;
        }

        let hover_bg = if response.hovered || focused {
            Some(self.theme.surface_hover)
        } else {
            None
        };
        let toggle_gap = self.theme.spacing.sm();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Row,
                gap: toggle_gap,
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
            })));
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
        self.rollback.last_text_idx = None;

        response.changed = *on != old_on;
        response
    }

    // ── select / dropdown ─────────────────────────────────────────────

    /// Render a dropdown select. Shows the selected item; expands on activation.
    ///
    /// Returns `true` when the selection changed this frame.
    /// Render a dropdown select widget.
    pub fn select(&mut self, state: &mut SelectState) -> Response {
        let colors = self.widget_theme.select;
        self.select_colored(state, &colors)
    }

    /// Render a dropdown select widget with custom widget colors.
    pub fn select_colored(&mut self, state: &mut SelectState, colors: &WidgetColors) -> Response {
        if state.items.is_empty() {
            return Response::none();
        }
        state.selected = state.selected.min(state.items.len().saturating_sub(1));

        let focused = self.register_focusable();
        let (_interaction_id, mut response) = self.begin_widget_interaction(focused);
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
        for (i, key) in self.available_key_presses() {
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
        self.consume_indices(consumed_indices);
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

        self.render_select_trigger(&display_text, arrow, border_color, colors);

        if state.open {
            self.render_select_dropdown(state, colors);
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
    }

    fn render_select_trigger(
        &mut self,
        display_text: &str,
        arrow: &str,
        border_color: Color,
        colors: &WidgetColors,
    ) {
        let trig_gap = self.theme.spacing.xs();
        let trig_h = self.theme.spacing.xs();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Row,
                gap: trig_gap,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: Some(Border::Rounded),
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(border_color),
                bg_color: None,
                padding: Padding {
                    left: trig_h,
                    right: trig_h,
                    top: 0,
                    bottom: 0,
                },
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));
        self.skip_interaction_slot();
        self.styled(
            display_text,
            Style::new().fg(colors.fg.unwrap_or(self.theme.text)),
        );
        self.styled(
            arrow,
            Style::new().fg(colors.fg.unwrap_or(self.theme.text_dim)),
        );
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
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
    /// Render a radio button group.
    pub fn radio(&mut self, state: &mut RadioState) -> Response {
        let colors = self.widget_theme.radio;
        self.radio_colored(state, &colors)
    }

    /// Render a radio button group with custom widget colors.
    pub fn radio_colored(&mut self, state: &mut RadioState, colors: &WidgetColors) -> Response {
        if state.items.is_empty() {
            return Response::none();
        }
        state.selected = state.selected.min(state.items.len().saturating_sub(1));
        let focused = self.register_focusable();
        let old_selected = state.selected;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
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
            self.consume_indices(consumed_indices);
        }

        let (interaction_id, mut response) = self.begin_widget_interaction(focused);

        if let Some((rect, clicks)) = self.left_clicks_for_interaction(interaction_id) {
            let mut consumed = Vec::new();
            for (i, mouse) in clicks {
                let clicked_idx = (mouse.y - rect.y) as usize;
                if clicked_idx < state.items.len() {
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
        self.rollback.last_text_idx = None;
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
            for (i, key) in self.available_key_presses() {
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
            self.consume_indices(consumed_indices);
        }

        let (interaction_id, mut response) = self.begin_widget_interaction(focused);

        if let Some((rect, clicks)) = self.left_clicks_for_interaction(interaction_id) {
            let mut consumed = Vec::new();
            for (i, mouse) in clicks {
                let clicked_idx = (mouse.y - rect.y) as usize;
                if clicked_idx < state.items.len() {
                    state.toggle(clicked_idx);
                    state.cursor = clicked_idx;
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
                border_style: Style::new().fg(self.theme.border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));

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
        self.rollback.last_text_idx = None;
        response.changed = state.selected != old_selected;
        response
    }

    // ── tree ─────────────────────────────────────────────────────────
}
