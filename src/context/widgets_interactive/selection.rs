use super::*;

/// Maximum page count rendered as dots before [`Context::paginator`] falls back
/// to the compact `{page}/{total}` counter to avoid overflowing the line.
const PAGINATOR_MAX_DOTS: usize = 12;

fn table_page_bounds(state: &TableState) -> (usize, usize) {
    let total = state.visible_indices().len();
    if state.page_size == 0 {
        return (0, total);
    }
    let start = state.page.saturating_mul(state.page_size).min(total);
    (
        start,
        start.saturating_add(table_visible_len(state)).min(total),
    )
}

fn move_table_selection(selected: &mut usize, start: usize, end: usize, key: &KeyCode) {
    if start >= end {
        *selected = 0;
        return;
    }
    match key {
        KeyCode::Up | KeyCode::Char('k') => *selected = selected.saturating_sub(1).max(start),
        KeyCode::Down | KeyCode::Char('j') => *selected = selected.saturating_add(1).min(end - 1),
        _ => {}
    }
}

/// Per-column cell renderer for [`Context::table_with`]: maps
/// `(row_view_index, col_index, raw_cell)` to styled content.
type TableCellRenderer = Box<dyn Fn(usize, usize, &str) -> (String, Style)>;

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
        self.table_inner(state, colors, None)
    }

    /// Render a data table with a per-column cell renderer.
    ///
    /// `cell` maps `(row_view_index, col_index, raw_cell)` to a
    /// `(content, Style)` pair, letting any column carry its own foreground /
    /// background / modifiers (a colored badge, a status label, an icon, …).
    /// Columns whose closure returns the unchanged raw string with a default
    /// [`Style`] fall back to the plain string-grid behavior. The closure is
    /// `'static` (it is invoked during deferred row rendering) and is called
    /// once per visible cell per frame.
    ///
    /// Sorting, filtering, pagination, width constraints, and multi-row
    /// selection all behave exactly as in [`table`](Context::table); only the
    /// per-cell content/style differs.
    ///
    /// Available since v0.21.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::{Color, Style, widgets::TableState};
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut table = TableState::new(
    ///     vec!["Service", "Status"],
    ///     vec![vec!["api", "OK"], vec!["db", "DOWN"]],
    /// );
    /// ui.table_with(&mut table, |_row, col, raw| {
    ///     if col == 1 {
    ///         let color = if raw == "OK" { Color::Green } else { Color::Red };
    ///         (raw.to_string(), Style::new().fg(color).bold())
    ///     } else {
    ///         (raw.to_string(), Style::default())
    ///     }
    /// });
    /// # });
    /// ```
    pub fn table_with(
        &mut self,
        state: &mut TableState,
        cell: impl Fn(usize, usize, &str) -> (String, Style) + 'static,
    ) -> Response {
        let colors = self.widget_theme.table;
        self.table_inner(state, &colors, Some(Box::new(cell)))
    }

    fn table_inner(
        &mut self,
        state: &mut TableState,
        colors: &WidgetColors,
        cell: Option<TableCellRenderer>,
    ) -> Response {
        if state.is_dirty() {
            state.recompute_widths();
        }

        let old_selected = state.selected;
        let old_sort_column = state.sort_column;
        let old_sort_ascending = state.sort_ascending;
        let old_page = state.page;
        let old_filter = state.filter().to_string();
        let old_multi = state.multi_selected.clone();

        let focused = self.register_focusable();
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);

        self.table_handle_events(state, focused, interaction_id);

        if state.is_dirty() {
            state.recompute_widths();
        }
        let (available, _) = self.available_content_size();
        let separators = (state.headers().len().saturating_sub(1) as u32).saturating_mul(3);
        state.resolve_column_widths(available.saturating_sub(separators));

        self.table_render(state, focused, colors, cell);

        response.changed = state.selected != old_selected
            || state.sort_column != old_sort_column
            || state.sort_ascending != old_sort_ascending
            || state.page != old_page
            || state.filter() != old_filter
            || state.multi_selected != old_multi;
        response
    }

    fn table_handle_events(
        &mut self,
        state: &mut TableState,
        focused: bool,
        interaction_id: usize,
    ) {
        self.handle_table_keys(state, focused);

        if state.visible_indices().is_empty() && state.headers().is_empty() {
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

                let (page_start, page_end) = table_page_bounds(state);
                let visible_len = page_end.saturating_sub(page_start);
                let clicked_idx = (mouse.y - rect.y - 2) as usize;
                if clicked_idx < visible_len {
                    let clicked_idx = page_start + clicked_idx;
                    state.selected = clicked_idx;
                    if mouse.modifiers.contains(KeyModifiers::SHIFT) {
                        let anchor = state.selection_anchor.unwrap_or(clicked_idx);
                        state.select_range(anchor, clicked_idx);
                    } else if mouse.modifiers.contains(KeyModifiers::CONTROL) {
                        state.toggle_row(clicked_idx);
                    } else {
                        state.select_single(clicked_idx);
                    }
                    consumed.push(i);
                }
            }
            self.consume_indices(consumed);
        }
    }

    fn table_render(
        &mut self,
        state: &mut TableState,
        focused: bool,
        colors: &WidgetColors,
        cell: Option<TableCellRenderer>,
    ) {
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
        if visible_len == 0 {
            state.selected = 0;
        } else {
            state.selected = state.selected.clamp(page_start, page_end - 1);
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

        self.render_table_header(state, colors);
        self.render_table_rows(state, focused, page_start, visible_len, colors, cell);

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
            let shift = key.modifiers.contains(KeyModifiers::SHIFT);
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            let (page_start, page_end) = table_page_bounds(state);
            if page_start < page_end {
                state.selected = state.selected.clamp(page_start, page_end - 1);
            }
            match key.code {
                // Shift+Up/Down: extend a contiguous range from the anchor.
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') if shift => {
                    let anchor = *state.selection_anchor.get_or_insert(state.selected);
                    move_table_selection(&mut state.selected, page_start, page_end, &key.code);
                    state.select_range(anchor, state.selected);
                    consumed_indices.push(i);
                }
                // Plain Up/Down (or k/j): move the cursor only (back-compat).
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                    move_table_selection(&mut state.selected, page_start, page_end, &key.code);
                    consumed_indices.push(i);
                }
                // Ctrl+Space: toggle the focused row without clearing the set.
                // Space: toggle the focused row (additive toggle).
                KeyCode::Char(' ') if ctrl => {
                    state.toggle_row(state.selected);
                    consumed_indices.push(i);
                }
                KeyCode::Char(' ') => {
                    state.toggle_row(state.selected);
                    consumed_indices.push(i);
                }
                KeyCode::PageUp => {
                    let old_page = state.page;
                    state.prev_page();
                    if state.page != old_page {
                        state.selected = table_page_bounds(state).0;
                    }
                    consumed_indices.push(i);
                }
                KeyCode::PageDown => {
                    let old_page = state.page;
                    state.next_page();
                    if state.page != old_page {
                        state.selected = table_page_bounds(state).0;
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
            .headers()
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
        cell: Option<TableCellRenderer>,
    ) {
        for idx in 0..visible_len {
            let view_idx = page_start + idx;
            let data_idx = state.visible_indices()[view_idx];
            let Some(row) = state.row(data_idx) else {
                continue;
            };

            // Base style for the whole row, applied to every cell unless the
            // per-column renderer overrides it. Priority: focused cursor row >
            // multi-selected row > zebra > plain. When `multi_selected` is empty
            // (the default), this collapses to the pre-v0.21 behavior verbatim.
            let base = if view_idx == state.selected {
                let mut style = Style::new()
                    .bg(colors.accent.unwrap_or(self.theme.selected_bg))
                    .fg(colors.fg.unwrap_or(self.theme.selected_fg));
                if focused {
                    style = style.bold();
                }
                style
            } else if state.is_row_selected(view_idx) {
                // Dimmer selection background to distinguish set members from
                // the brighter focused-cursor row.
                Style::new()
                    .bg(colors.accent.unwrap_or(self.theme.selected_bg))
                    .fg(colors.fg.unwrap_or(self.theme.selected_fg))
                    .dim()
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
                style
            };

            match &cell {
                None => {
                    let line = format_table_row(row, state.column_widths(), " │ ");
                    self.styled(line, base);
                }
                Some(render) => {
                    let widths = state.column_widths();
                    let mut segments: Vec<(String, Style)> =
                        Vec::with_capacity(widths.len().saturating_mul(2));
                    for (col, width) in widths.iter().enumerate() {
                        if col > 0 {
                            segments.push((" │ ".to_string(), base));
                        }
                        let raw = row.get(col).map(String::as_str).unwrap_or("");
                        let (content, cell_style) = render(view_idx, col, raw);
                        // Overlay the per-cell style onto the row base: the cell
                        // fg / bg win when set, modifiers are unioned. This keeps
                        // the row selection background unless the cell overrides
                        // it, while letting a column carry its own colored text.
                        let mut merged = base;
                        if cell_style.fg.is_some() {
                            merged.fg = cell_style.fg;
                        }
                        if cell_style.bg.is_some() {
                            merged.bg = cell_style.bg;
                        }
                        merged.modifiers |= cell_style.modifiers;
                        let padded = clamp_table_cell(&content, *width);
                        segments.push((padded, merged));
                    }
                    self.line(move |ui| {
                        for (text, style) in segments {
                            ui.styled(text, style);
                        }
                    });
                }
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
                gap: tabs_gap as i32,
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
                if focused { s.underline() } else { s }
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

    /// Render a standalone paginator, decoupled from any list or table.
    ///
    /// Consumes Left/`h`/PageUp (previous page) and Right/`l`/PageDown (next
    /// page) when focused, and consumes those key events when handled. Clicking
    /// a dot (in [`PaginatorStyle::Dots`]) jumps to that page; clicking the
    /// left/right half of the counter (in [`PaginatorStyle::Arabic`]) goes to
    /// the previous/next page. [`Response::changed`] is `true` iff the page
    /// changed this frame.
    ///
    /// Pass a `&mut PaginatorState` each frame and use
    /// [`PaginatorState::page_bounds`] to slice your own data.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::PaginatorState;
    ///
    /// let mut state = PaginatorState::new(42, 10);
    /// # slt::run(move |ui: &mut slt::Context| {
    /// ui.paginator(&mut state);
    /// # });
    /// ```
    pub fn paginator(&mut self, state: &mut PaginatorState) -> Response {
        // Reuse the tabs WidgetColors slot until a dedicated paginator slot lands.
        let colors = self.widget_theme.tabs;
        self.paginator_colored(state, &colors)
    }

    /// Render a standalone paginator with custom widget colors.
    ///
    /// Behaves exactly like [`Context::paginator`] but draws with the provided
    /// [`WidgetColors`] instead of the theme defaults.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::{Color, PaginatorState, WidgetColors};
    ///
    /// let mut state = PaginatorState::new(20, 5);
    /// let colors = WidgetColors {
    ///     accent: Some(Color::Cyan),
    ///     ..WidgetColors::default()
    /// };
    /// # slt::run(move |ui: &mut slt::Context| {
    /// ui.paginator_colored(&mut state, &colors);
    /// # });
    /// ```
    pub fn paginator_colored(
        &mut self,
        state: &mut PaginatorState,
        colors: &WidgetColors,
    ) -> Response {
        state.page = state.page.min(state.total_pages().saturating_sub(1));
        let old_page = state.page;

        let focused = self.register_focusable();
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::PageUp => {
                        state.prev_page();
                        consumed_indices.push(i);
                    }
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::PageDown => {
                        state.next_page();
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
        }

        let total_pages = state.total_pages();
        // Dots style overflows past 12 pages, so fall back to the compact counter.
        let use_dots =
            matches!(state.style, PaginatorStyle::Dots) && total_pages <= PAGINATOR_MAX_DOTS;

        if let Some((rect, clicks)) = self.left_clicks_for_interaction(interaction_id) {
            let mut consumed = Vec::new();
            for (i, mouse) in clicks {
                if mouse.y != rect.y {
                    continue;
                }
                let rel_x = mouse.x.saturating_sub(rect.x);
                if use_dots {
                    // Dots render with no inter-glyph gap, so dot `n` is at column `n`.
                    let target = rel_x as usize;
                    if target < total_pages {
                        state.set_page(target);
                        consumed.push(i);
                    }
                } else {
                    // Counter: left half -> prev, right half -> next.
                    let label = format!("{}/{}", state.page + 1, total_pages);
                    let width = UnicodeWidthStr::width(label.as_str()) as u32;
                    if rel_x < width {
                        if rel_x < width / 2 {
                            state.prev_page();
                        } else {
                            state.next_page();
                        }
                        consumed.push(i);
                    }
                }
            }
            self.consume_indices(consumed);
        }

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
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));

        if use_dots {
            let active_color = colors.accent.unwrap_or(self.theme.primary);
            let inactive_color = colors.fg.unwrap_or(self.theme.text_dim);
            for page in 0..total_pages {
                let (glyph, color) = if page == state.page {
                    ("●", active_color)
                } else {
                    ("○", inactive_color)
                };
                let style = if page == state.page && focused {
                    Style::new().fg(color).bold()
                } else {
                    Style::new().fg(color)
                };
                self.styled(glyph, style);
            }
        } else {
            let label = format!("{}/{}", state.page + 1, total_pages);
            let style = Style::new().fg(colors.fg.unwrap_or(self.theme.text_dim));
            self.styled(label, style);
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        response.changed = state.page != old_page;
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
                gap: cb_gap as i32,
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
                gap: toggle_gap as i32,
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
        if !state.is_empty() {
            state.selected = state.selected.min(state.len().saturating_sub(1));
        }

        let focused = self.register_focusable();
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);
        let old_selected = state.selected;

        self.select_handle_events(state, focused, interaction_id);
        // Keep the cursor within the filtered subset before rendering.
        if state.open {
            let flen = state.filtered_indices().len();
            let cur = state.cursor();
            if flen == 0 {
                state.set_cursor(0);
            } else if cur >= flen {
                state.set_cursor(flen - 1);
            }
        }
        self.select_render(state, focused, colors);
        response.changed = state.selected != old_selected;
        response
    }

    fn select_handle_events(
        &mut self,
        state: &mut SelectState,
        focused: bool,
        interaction_id: usize,
    ) {
        if let Some((rect, clicks)) = self.left_clicks_for_interaction(interaction_id) {
            let mut consumed = Vec::new();
            for (event_index, mouse) in clicks {
                let relative_y = mouse.y.saturating_sub(rect.y) as usize;
                if relative_y < 3 {
                    if !state.is_empty() {
                        state.open = !state.open;
                        if state.open {
                            state.filter.clear();
                            state.set_cursor(state.selected);
                        }
                    }
                    consumed.push(event_index);
                    continue;
                }

                if state.open {
                    let query_rows = usize::from(!state.filter.is_empty());
                    let row_start = 3 + query_rows;
                    if relative_y >= row_start {
                        let filtered = state.filtered_indices();
                        let row = relative_y - row_start;
                        if let Some(&data_index) = filtered.get(row) {
                            state.selected = data_index;
                            state.set_cursor(row);
                            state.open = false;
                            state.filter.clear();
                            consumed.push(event_index);
                        }
                    }
                }
            }
            self.consume_indices(consumed);
        }

        if !focused {
            return;
        }

        let mut consumed_indices = Vec::new();
        for (i, key) in self.available_key_presses() {
            if state.open {
                // Cursor indexes into the filtered subset (not `items`); arrow
                // keys navigate, printable keys type into the filter.
                let filtered_len = state.filtered_indices().len();
                match key.code {
                    KeyCode::Up => {
                        state.set_cursor(state.cursor().saturating_sub(1));
                        consumed_indices.push(i);
                    }
                    KeyCode::Down => {
                        if filtered_len > 0 {
                            let next = (state.cursor() + 1).min(filtered_len - 1);
                            state.set_cursor(next);
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter => {
                        if let Some(&real) = state.filtered_indices().get(state.cursor()) {
                            state.selected = real;
                        }
                        state.open = false;
                        state.filter.clear();
                        consumed_indices.push(i);
                    }
                    KeyCode::Esc => {
                        // First Esc clears a non-empty query; a second closes.
                        if state.filter.is_empty() {
                            state.open = false;
                        } else {
                            state.filter.clear();
                            state.set_cursor(0);
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Backspace => {
                        if let Some((byte_index, _)) =
                            state.filter.grapheme_indices(true).next_back()
                        {
                            state.filter.truncate(byte_index);
                        }
                        state.set_cursor(0);
                        consumed_indices.push(i);
                    }
                    KeyCode::Char(c) if !has_global_shortcut_modifier(key.modifiers) => {
                        // Printable keys (including space, 'j', 'k') type into the
                        // filter — arrows remain the only navigation while open.
                        state.filter.push(c);
                        state.set_cursor(0);
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            } else if !state.is_empty() && matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                state.open = true;
                state.filter.clear();
                state.set_cursor(state.selected);
                consumed_indices.push(i);
            }
        }
        if state.open {
            for (event_index, text) in self.available_pastes() {
                let inserted = text
                    .graphemes(true)
                    .filter(|cluster| {
                        cluster
                            .chars()
                            .all(|ch| (ch as u32) >= 0x20 && ch != '\u{7f}')
                    })
                    .collect::<String>();
                if !inserted.is_empty() {
                    state.filter.push_str(&inserted);
                    state.set_cursor(0);
                }
                consumed_indices.push(event_index);
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
            .items()
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
                gap: trig_gap as i32,
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
        let filtered = state.filtered_indices();

        // Show the active query so typing has visible feedback.
        if !state.filter.is_empty() {
            let dim = self.theme.text_dim;
            let mut q = String::with_capacity(state.filter.len() + 1);
            q.push('/');
            q.push_str(&state.filter);
            self.styled(q, Style::new().fg(dim).italic());
        }

        if filtered.is_empty() {
            let dim = self.theme.text_dim;
            self.styled("  (no matches)".to_string(), Style::new().fg(dim).dim());
            return;
        }

        let cursor = state.cursor();
        for (pos, &idx) in filtered.iter().enumerate() {
            let item = &state.items()[idx];
            let is_cursor = pos == cursor;
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
        if state.is_empty() {
            return Response::none();
        }
        state.cursor = state.cursor.min(state.len().saturating_sub(1));
        let focused = self.register_focusable();
        let old_selected = state.selected.clone();

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                        let max_index = state.len().saturating_sub(1);
                        let _ = handle_vertical_nav(&mut state.cursor, max_index, key.code.clone());
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
                if clicked_idx < state.len() {
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

        for (idx, item) in state.items().iter().enumerate() {
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

    // ── color picker ───────────────────────────────────────────────────

    /// Render an interactive color picker over the [`Color`] model.
    ///
    /// Shows a grid of color swatches plus an optional hex-entry field. When
    /// focused, the arrow keys / `hjkl` move the 2D swatch cursor (clamped at
    /// the grid edges), `Tab` toggles between palette and hex entry, and
    /// `Enter` / `Space` confirms the current color. Returns `changed` on the
    /// exact frames where the selected [`Color`] differs from the previous
    /// frame. Read the chosen color back via
    /// [`ColorPickerState::selected`](crate::widgets::ColorPickerState::selected).
    ///
    /// Each swatch is emitted with a full-RGB background; the terminal backend
    /// downsamples it to the active [`ColorDepth`](crate::ColorDepth) on flush,
    /// so the picker degrades correctly on 256-color, 16-color, and no-color
    /// terminals. Uses the theme's `color_picker` slot for border and cursor
    /// colors; override per-call with
    /// [`color_picker_colored`](Self::color_picker_colored).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::ColorPickerState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut picker = ColorPickerState::tailwind();
    /// if ui.color_picker(&mut picker).changed {
    ///     let chosen = picker.selected();
    ///     let _ = chosen;
    /// }
    /// # });
    /// ```
    pub fn color_picker(&mut self, state: &mut ColorPickerState) -> Response {
        let colors = self.widget_theme.color_picker;
        self.color_picker_colored(state, &colors)
    }

    /// Render a color picker with custom [`WidgetColors`].
    ///
    /// Behaves exactly like [`color_picker`](Self::color_picker) but draws the
    /// border, cursor highlight, and hex field with the supplied colors instead
    /// of the theme's `color_picker` slot.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::ColorPickerState;
    /// # use slt::{Color, WidgetColors};
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut picker = ColorPickerState::tailwind();
    /// let theme = WidgetColors::new().accent(Color::Cyan);
    /// ui.color_picker_colored(&mut picker, &theme);
    /// # });
    /// ```
    pub fn color_picker_colored(
        &mut self,
        state: &mut ColorPickerState,
        colors: &WidgetColors,
    ) -> Response {
        if state.colors.is_empty() {
            return Response::none();
        }
        let columns = state.columns.max(1);
        state.selected = state.selected.min(state.colors.len() - 1);

        let focused = self.register_focusable();
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);
        let old_color = state.selected();

        self.color_picker_handle_keys(state, focused, columns);
        self.color_picker_handle_clicks(state, interaction_id, columns);
        self.color_picker_render(state, focused, columns, colors);

        response.changed = state.selected() != old_color;
        response
    }

    fn color_picker_handle_keys(
        &mut self,
        state: &mut ColorPickerState,
        focused: bool,
        columns: usize,
    ) {
        if !focused {
            return;
        }
        let len = state.colors.len();
        let mut consumed_indices = Vec::new();
        for (i, key) in self.available_key_presses() {
            match state.mode {
                PickerMode::Palette => match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        if !state.selected.is_multiple_of(columns) {
                            state.selected -= 1;
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if state.selected % columns < columns - 1 && state.selected + 1 < len {
                            state.selected += 1;
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if state.selected >= columns {
                            state.selected -= columns;
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if state.selected + columns < len {
                            state.selected += columns;
                        }
                        consumed_indices.push(i);
                    }
                    KeyCode::Tab => {
                        state.mode = PickerMode::Hex;
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        consumed_indices.push(i);
                    }
                    _ => {}
                },
                PickerMode::Hex => match key.code {
                    KeyCode::Tab => {
                        state.mode = PickerMode::Palette;
                        consumed_indices.push(i);
                    }
                    KeyCode::Enter => {
                        consumed_indices.push(i);
                    }
                    KeyCode::Char(ch) => {
                        let index =
                            byte_index_for_char(&state.hex_input.value, state.hex_input.cursor);
                        state.hex_input.value.insert(index, ch);
                        state.hex_input.cursor += 1;
                        color_picker_validate_hex(&mut state.hex_input);
                        consumed_indices.push(i);
                    }
                    KeyCode::Backspace => {
                        if state.hex_input.cursor > 0 {
                            let start = byte_index_for_char(
                                &state.hex_input.value,
                                state.hex_input.cursor - 1,
                            );
                            let end =
                                byte_index_for_char(&state.hex_input.value, state.hex_input.cursor);
                            state.hex_input.value.replace_range(start..end, "");
                            state.hex_input.cursor -= 1;
                        }
                        color_picker_validate_hex(&mut state.hex_input);
                        consumed_indices.push(i);
                    }
                    _ => {}
                },
            }
        }
        self.consume_indices(consumed_indices);
    }

    fn color_picker_handle_clicks(
        &mut self,
        state: &mut ColorPickerState,
        interaction_id: usize,
        columns: usize,
    ) {
        if let Some((rect, clicks)) = self.left_clicks_for_interaction(interaction_id) {
            // The interaction rect spans the whole bordered container; the
            // swatch grid starts inside the top border and the left
            // border + x-padding. Offset clicks back into grid space.
            let grid_x0 = rect.x + GRID_X_OFFSET;
            let grid_y0 = rect.y + GRID_Y_OFFSET;
            let rows = state.colors.len().div_ceil(columns);
            let mut consumed = Vec::new();
            for (i, mouse) in clicks {
                if mouse.x < grid_x0 || mouse.y < grid_y0 {
                    continue;
                }
                let row = (mouse.y - grid_y0) as usize;
                let col = (mouse.x - grid_x0) as usize / SWATCH_WIDTH;
                if row < rows && col < columns {
                    let idx = row * columns + col;
                    if idx < state.colors.len() {
                        state.mode = PickerMode::Palette;
                        state.selected = idx;
                        consumed.push(i);
                    }
                }
            }
            self.consume_indices(consumed);
        }
    }

    fn color_picker_render(
        &mut self,
        state: &ColorPickerState,
        focused: bool,
        columns: usize,
        colors: &WidgetColors,
    ) {
        let border_color = if focused {
            colors.accent.unwrap_or(self.theme.primary)
        } else {
            colors.border.unwrap_or(self.theme.border)
        };
        let text_color = colors.fg.unwrap_or(self.theme.text);

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Column,
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: Some(Border::Rounded),
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(border_color),
                bg_color: None,
                padding: Padding::xy(1, 0),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));

        // Swatch grid: one Row container per grid row, one cell per swatch.
        let rows = state.colors.len().div_ceil(columns);
        for row in 0..rows {
            self.commands
                .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                    direction: Direction::Row,
                    gap: 0,
                    align: Align::Start,
                    align_self: None,
                    justify: Justify::Start,
                    border: None,
                    border_sides: BorderSides::all(),
                    border_style: Style::new(),
                    bg_color: None,
                    padding: Padding::default(),
                    margin: Margin::default(),
                    constraints: Constraints::default(),
                    title: None,
                    grow: 0,
                    group_name: None,
                })));
            for col in 0..columns {
                let idx = row * columns + col;
                let Some(&swatch) = state.colors.get(idx) else {
                    break;
                };
                let is_cursor = idx == state.selected && state.mode == PickerMode::Palette;
                let marker = if is_cursor { '▣' } else { ' ' };
                let mut cell = String::with_capacity(SWATCH_WIDTH);
                cell.push(' ');
                cell.push(marker);
                cell.push(' ');
                // Full-RGB bg; the terminal flush downsamples per ColorDepth.
                // contrast_fg keeps the cursor marker legible on any swatch.
                let mut style = Style::new().bg(swatch).fg(Color::contrast_fg(swatch));
                if is_cursor {
                    style = style.bold();
                }
                self.styled(cell, style);
            }
            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }

        // Selected color readout: a `#RRGGBB` label keeps the picker legible
        // under `ColorDepth::NoColor`, where no background color is emitted.
        let selected = state.selected();
        let label = color_hex_label(selected).unwrap_or_else(|| "selected".to_string());
        let mut readout = String::with_capacity(label.len() + 3);
        readout.push_str("▸ ");
        readout.push_str(&label);
        self.styled(readout, Style::new().fg(text_color).bold());

        // Hex entry line. The embedded field shows the typed value (or its
        // placeholder); a `✗` flag surfaces the text-input validation error
        // path on malformed input without panicking.
        let hex_active = state.mode == PickerMode::Hex;
        let hex_display = if state.hex_input.value.is_empty() {
            state.hex_input.placeholder.clone()
        } else {
            state.hex_input.value.clone()
        };
        let mut hex_line = String::with_capacity(hex_display.len() + 6);
        hex_line.push_str(if hex_active { "▸ hex " } else { "  hex " });
        hex_line.push_str(&hex_display);
        if state.hex_input.validation_error.is_some() {
            hex_line.push_str(" ✗");
        }
        let hex_style = if hex_active {
            Style::new()
                .fg(colors.accent.unwrap_or(self.theme.primary))
                .bold()
        } else {
            Style::new().fg(colors.fg.unwrap_or(self.theme.text_dim))
        };
        self.styled(hex_line, hex_style);

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
    }

    // ── tree ─────────────────────────────────────────────────────────
}

/// Display width in cells of one color-picker swatch (` ▣ ` / `   `).
const SWATCH_WIDTH: usize = 3;

/// Horizontal offset from the picker's interaction rect to the swatch grid:
/// the rounded left border (1) plus the container's left x-padding (1).
const GRID_X_OFFSET: u32 = 2;

/// Vertical offset from the picker's interaction rect to the swatch grid:
/// the rounded top border (1); the container has no top padding.
const GRID_Y_OFFSET: u32 = 1;

/// Validate the hex-entry field, setting/clearing its `validation_error`.
///
/// An empty field is treated as "not yet entered" (no error). Any non-empty
/// value that does not parse as `#RRGGBB` / `#RGB` records an error so the
/// widget can surface the text-input validation path.
fn color_picker_validate_hex(input: &mut TextInputState) {
    if input.value.is_empty() {
        input.validation_error = None;
    } else if parse_hex_color(&input.value).is_none() {
        input.validation_error = Some("invalid hex".to_string());
    } else {
        input.validation_error = None;
    }
}
