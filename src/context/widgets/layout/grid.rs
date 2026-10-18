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
        self.invalidate_geometry_cache_from(children_start);

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
    #[deprecated(since = "0.26.0", note = "renamed to `grid_columns(columns, f)`")]
    pub fn grid_with(&mut self, columns: &[GridColumn], f: impl FnOnce(&mut Context)) -> Response {
        self.grid_columns(columns, f)
    }

    /// Render children in a grid with per-column width specifications (CSS
    /// `grid-template-columns`).
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
    /// ui.grid_columns(&[
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
    pub fn grid_columns(
        &mut self,
        columns: &[GridColumn],
        f: impl FnOnce(&mut Context),
    ) -> Response {
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
        self.invalidate_geometry_cache_from(children_start);

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
