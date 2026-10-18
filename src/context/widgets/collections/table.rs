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
    ///
    /// let mut table = TableState::new(
    ///     vec!["Service", "Status"],
    ///     vec![vec!["api", "OK"], vec!["db", "DOWN"]],
    /// );
    /// slt::run(|ui: &mut slt::Context| {
    ///     ui.table_with(&mut table, |_row, col, raw| {
    ///         if col == 1 {
    ///             let color = if raw == "OK" { Color::Green } else { Color::Red };
    ///             (raw.to_string(), Style::new().fg(color).bold())
    ///         } else {
    ///             (raw.to_string(), Style::default())
    ///         }
    ///     });
    /// })?;
    /// # Ok::<_, std::io::Error>(())
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
}
