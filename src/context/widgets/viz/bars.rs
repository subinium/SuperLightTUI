use super::*;

struct VerticalBarLayout {
    chart_height: usize,
    bar_width: usize,
    value_labels: Vec<String>,
    col_width: usize,
    bar_units: Vec<usize>,
}

impl Context {
    /// Render a horizontal bar chart from `(label, value)` pairs.
    ///
    /// Bars are normalized against the largest value and rendered with `█` up to
    /// `max_width` characters.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let data = [
    ///     ("Sales", 160.0),
    ///     ("Revenue", 120.0),
    ///     ("Users", 220.0),
    ///     ("Costs", 60.0),
    /// ];
    /// ui.bar_chart(&data, 24);
    /// # });
    /// ```
    ///
    /// For styled bars with per-bar colors, see [`bar_chart_with`](Self::bar_chart_with).
    pub fn bar_chart(&mut self, data: &[(&str, f64)], max_width: u32) -> Response {
        if data.is_empty() {
            return Response::none();
        }
        let response = self.begin_viz_root();

        let max_label_width = data
            .iter()
            .map(|(label, _)| UnicodeWidthStr::width(*label))
            .max()
            .unwrap_or(0);
        let denom = positive_finite_max(data.iter().map(|(_, value)| *value));

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

        for (label, value) in data {
            let label_width = UnicodeWidthStr::width(*label);
            let label_padding = " ".repeat(max_label_width.saturating_sub(label_width));
            let value = finite_or_zero(*value);
            let normalized = unit_ratio(value, 0.0, denom);
            let bar = Self::horizontal_bar_text(normalized, max_width);

            self.skip_interaction_slot();
            self.commands
                .push(Command::BeginContainer(Box::new(BeginContainerArgs {
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
                })));
            let mut label_text = String::with_capacity(label.len() + label_padding.len());
            label_text.push_str(label);
            label_text.push_str(&label_padding);
            self.styled(label_text, Style::new().fg(self.theme.text));
            self.styled(bar, Style::new().fg(self.theme.primary));
            self.styled(
                format_compact_number(value),
                Style::new().fg(self.theme.text_dim),
            );
            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
        self.end_viz_root();

        response
    }

    /// Render a bar chart with custom configuration.
    pub fn bar_chart_with(
        &mut self,
        bars: &[Bar],
        configure: impl FnOnce(&mut BarChartConfig),
        max_size: u32,
    ) -> Response {
        if bars.is_empty() {
            return Response::none();
        }

        let (config, denom) = self.bar_chart_styled_layout(bars, configure);
        let response = self.begin_viz_root();
        self.bar_chart_styled_render(bars, max_size, denom, &config);
        self.end_viz_root();

        response
    }

    fn bar_chart_styled_layout(
        &self,
        bars: &[Bar],
        configure: impl FnOnce(&mut BarChartConfig),
    ) -> (BarChartConfig, f64) {
        let mut config = BarChartConfig::default();
        configure(&mut config);

        let auto_max = positive_finite_max(bars.iter().map(|bar| bar.value));
        let configured_max = config
            .max_value
            .filter(|value| value.is_finite() && *value > 0.0);
        let denom = configured_max.unwrap_or(auto_max);

        (config, denom)
    }

    fn bar_chart_styled_render(
        &mut self,
        bars: &[Bar],
        max_size: u32,
        denom: f64,
        config: &BarChartConfig,
    ) {
        match config.direction {
            BarDirection::Horizontal => {
                self.render_horizontal_styled_bars(bars, max_size, denom, config.bar_gap)
            }
            BarDirection::Vertical => self.render_vertical_styled_bars(
                bars,
                max_size,
                denom,
                config.bar_width,
                config.bar_gap,
            ),
        }
    }

    fn render_horizontal_styled_bars(
        &mut self,
        bars: &[Bar],
        max_width: u32,
        denom: f64,
        bar_gap: u16,
    ) {
        let max_label_width = bars
            .iter()
            .map(|bar| UnicodeWidthStr::width(bar.label.as_str()))
            .max()
            .unwrap_or(0);

        self.skip_interaction_slot();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Column,
                gap: bar_gap as i32,
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

        for bar in bars {
            self.render_horizontal_styled_bar_row(bar, max_label_width, max_width, denom);
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
    }

    fn render_horizontal_styled_bar_row(
        &mut self,
        bar: &Bar,
        max_label_width: usize,
        max_width: u32,
        denom: f64,
    ) {
        let label_width = UnicodeWidthStr::width(bar.label.as_str());
        let label_padding = " ".repeat(max_label_width.saturating_sub(label_width));
        let normalized = unit_ratio(finite_or_zero(bar.value), 0.0, denom);
        let bar_text = Self::horizontal_bar_text(normalized, max_width);
        let color = bar.color.unwrap_or(self.theme.primary);

        self.skip_interaction_slot();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
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
            })));
        let mut label_text = String::with_capacity(bar.label.len() + label_padding.len());
        label_text.push_str(&bar.label);
        label_text.push_str(&label_padding);
        self.styled(label_text, Style::new().fg(self.theme.text));
        self.styled(bar_text, Style::new().fg(color));
        self.styled(
            Self::bar_display_value(bar),
            bar.value_style
                .unwrap_or(Style::new().fg(self.theme.text_dim)),
        );
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
    }

    fn render_vertical_styled_bars(
        &mut self,
        bars: &[Bar],
        max_height: u32,
        denom: f64,
        bar_width: u16,
        bar_gap: u16,
    ) {
        let layout = self.compute_vertical_bar_layout(bars, max_height, denom, bar_width);

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

        self.render_vertical_bar_body(
            bars,
            &layout.bar_units,
            layout.chart_height,
            layout.col_width,
            layout.bar_width,
            bar_gap,
            &layout.value_labels,
        );
        self.render_vertical_bar_labels(bars, layout.col_width, bar_gap);

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
    }

    fn compute_vertical_bar_layout(
        &self,
        bars: &[Bar],
        max_height: u32,
        denom: f64,
        bar_width: u16,
    ) -> VerticalBarLayout {
        let chart_height = max_height.max(1) as usize;
        let bar_width = bar_width.max(1) as usize;
        let value_labels: Vec<String> = bars.iter().map(Self::bar_display_value).collect();
        let label_width = bars
            .iter()
            .map(|bar| UnicodeWidthStr::width(bar.label.as_str()))
            .max()
            .unwrap_or(1);
        let value_width = value_labels
            .iter()
            .map(|value| UnicodeWidthStr::width(value.as_str()))
            .max()
            .unwrap_or(1);
        let col_width = bar_width.max(label_width.max(value_width).max(1));
        let bar_units: Vec<usize> = bars
            .iter()
            .map(|bar| {
                (unit_ratio(finite_or_zero(bar.value), 0.0, denom) * chart_height as f64 * 8.0)
                    .round() as usize
            })
            .collect();

        VerticalBarLayout {
            chart_height,
            bar_width,
            value_labels,
            col_width,
            bar_units,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_vertical_bar_body(
        &mut self,
        bars: &[Bar],
        bar_units: &[usize],
        chart_height: usize,
        col_width: usize,
        bar_width: usize,
        bar_gap: u16,
        value_labels: &[String],
    ) {
        const FRACTION_BLOCKS: [char; 8] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇'];

        // Pre-compute the topmost filled row for each bar (for value label placement).
        let top_rows: Vec<usize> = bar_units
            .iter()
            .map(|units| {
                if *units == 0 {
                    usize::MAX
                } else {
                    (*units - 1) / 8
                }
            })
            .collect();

        for row in (0..chart_height).rev() {
            self.skip_interaction_slot();
            self.commands
                .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                    direction: Direction::Row,
                    gap: bar_gap as i32,
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

            let row_base = row * 8;
            for (i, (bar, units)) in bars.iter().zip(bar_units.iter()).enumerate() {
                let color = bar.color.unwrap_or(self.theme.primary);

                if *units <= row_base {
                    // Value label one row above the bar top (plain text, no bg).
                    if top_rows[i] != usize::MAX && row == top_rows[i] + 1 {
                        let label = &value_labels[i];
                        let centered = Self::center_and_truncate_text(label, col_width);
                        self.styled(
                            centered,
                            bar.value_style.unwrap_or(Style::new().fg(color).bold()),
                        );
                    } else {
                        let empty = " ".repeat(col_width);
                        self.styled(empty, Style::new());
                    }
                    continue;
                }

                if row == top_rows[i] && top_rows[i] + 1 >= chart_height {
                    let label = &value_labels[i];
                    let centered = Self::center_and_truncate_text(label, col_width);
                    self.styled(
                        centered,
                        bar.value_style.unwrap_or(Style::new().fg(color).bold()),
                    );
                    continue;
                }

                let delta = *units - row_base;
                let fill = if delta >= 8 {
                    '█'
                } else {
                    FRACTION_BLOCKS[delta]
                };
                let fill_text = fill.to_string().repeat(bar_width);
                let centered_fill = center_text(&fill_text, col_width);
                self.styled(centered_fill, Style::new().fg(color));
            }

            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }
    }

    fn render_vertical_bar_labels(&mut self, bars: &[Bar], col_width: usize, bar_gap: u16) {
        self.skip_interaction_slot();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Row,
                gap: bar_gap as i32,
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
        for bar in bars {
            self.styled(
                Self::center_and_truncate_text(&bar.label, col_width),
                Style::new().fg(self.theme.text),
            );
        }
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
    }

    /// Render a grouped bar chart.
    ///
    /// Each group contains multiple bars rendered side by side. Useful for
    /// comparing categories across groups (e.g., quarterly revenue by product).
    ///
    /// # Example
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::{Bar, BarGroup, Color};
    /// let groups = vec![
    ///     BarGroup::new("2023", vec![Bar::new("Rev", 100.0).color(Color::Cyan), Bar::new("Cost", 60.0).color(Color::Red)]),
    ///     BarGroup::new("2024", vec![Bar::new("Rev", 140.0).color(Color::Cyan), Bar::new("Cost", 80.0).color(Color::Red)]),
    /// ];
    /// ui.bar_chart_grouped(&groups, 40);
    /// # });
    /// ```
    pub fn bar_chart_grouped(&mut self, groups: &[BarGroup], max_width: u32) -> Response {
        self.bar_chart_grouped_with(groups, |_| {}, max_width)
    }

    /// Render a grouped bar chart with custom configuration.
    pub fn bar_chart_grouped_with(
        &mut self,
        groups: &[BarGroup],
        configure: impl FnOnce(&mut BarChartConfig),
        max_size: u32,
    ) -> Response {
        if groups.is_empty() {
            return Response::none();
        }

        let all_bars: Vec<&Bar> = groups.iter().flat_map(|group| group.bars.iter()).collect();
        if all_bars.is_empty() {
            return Response::none();
        }

        let mut config = BarChartConfig::default();
        configure(&mut config);

        let auto_max = positive_finite_max(all_bars.iter().map(|bar| bar.value));
        let denom = config
            .max_value
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(auto_max);

        let response = self.begin_viz_root();
        match config.direction {
            BarDirection::Horizontal => {
                self.render_grouped_horizontal_bars(groups, max_size, denom, &config)
            }
            BarDirection::Vertical => {
                self.render_grouped_vertical_bars(groups, max_size, denom, &config)
            }
        }
        self.end_viz_root();

        response
    }

    fn render_grouped_horizontal_bars(
        &mut self,
        groups: &[BarGroup],
        max_width: u32,
        denom: f64,
        config: &BarChartConfig,
    ) {
        let all_bars: Vec<&Bar> = groups.iter().flat_map(|group| group.bars.iter()).collect();
        let max_label_width = all_bars
            .iter()
            .map(|bar| UnicodeWidthStr::width(bar.label.as_str()))
            .max()
            .unwrap_or(0);

        self.skip_interaction_slot();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Column,
                gap: config.group_gap as i32,
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

        for group in groups {
            self.skip_interaction_slot();
            self.commands
                .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                    direction: Direction::Column,
                    gap: config.bar_gap as i32,
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

            self.styled(group.label.clone(), Style::new().bold().fg(self.theme.text));

            for bar in &group.bars {
                let label_width = UnicodeWidthStr::width(bar.label.as_str());
                let label_padding = " ".repeat(max_label_width.saturating_sub(label_width));
                let normalized = unit_ratio(finite_or_zero(bar.value), 0.0, denom);
                let bar_text = Self::horizontal_bar_text(normalized, max_width);

                self.skip_interaction_slot();
                self.commands
                    .push(Command::BeginContainer(Box::new(BeginContainerArgs {
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
                    })));
                let mut label_text =
                    String::with_capacity(2 + bar.label.len() + label_padding.len());
                label_text.push_str("  ");
                label_text.push_str(&bar.label);
                label_text.push_str(&label_padding);
                self.styled(label_text, Style::new().fg(self.theme.text));
                self.styled(
                    bar_text,
                    Style::new().fg(bar.color.unwrap_or(self.theme.primary)),
                );
                self.styled(
                    Self::bar_display_value(bar),
                    bar.value_style
                        .unwrap_or(Style::new().fg(self.theme.text_dim)),
                );
                self.commands.push(Command::EndContainer);
                self.rollback.last_text_idx = None;
            }

            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
    }

    fn render_grouped_vertical_bars(
        &mut self,
        groups: &[BarGroup],
        max_height: u32,
        denom: f64,
        config: &BarChartConfig,
    ) {
        self.skip_interaction_slot();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Column,
                gap: config.group_gap as i32,
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

        for group in groups {
            self.styled(group.label.clone(), Style::new().bold().fg(self.theme.text));
            if !group.bars.is_empty() {
                self.render_vertical_styled_bars(
                    &group.bars,
                    max_height,
                    denom,
                    config.bar_width,
                    config.bar_gap,
                );
            }
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
    }

    fn horizontal_bar_text(normalized: f64, max_width: u32) -> String {
        let normalized = if normalized.is_finite() {
            normalized.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let filled = (normalized * max_width as f64).round() as usize;
        "█".repeat(filled)
    }

    fn bar_display_value(bar: &Bar) -> String {
        bar.text_value
            .clone()
            .unwrap_or_else(|| format_compact_number(finite_or_zero(bar.value)))
    }

    fn center_and_truncate_text(text: &str, width: usize) -> String {
        if width == 0 {
            return String::new();
        }

        let out = crate::chart::clip_text_cells(text, width);
        center_text(&out, width)
    }

    /// Render a stacked bar chart with custom configuration.
    ///
    /// Each group's bars are stacked on top of each other rather than placed
    /// side-by-side.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::{Bar, BarGroup, Color};
    /// let groups = vec![
    ///     BarGroup::new("2023", vec![
    ///         Bar::new("Rev", 100.0).color(Color::Cyan),
    ///         Bar::new("Cost", 60.0).color(Color::Red),
    ///     ]),
    ///     BarGroup::new("2024", vec![
    ///         Bar::new("Rev", 140.0).color(Color::Cyan),
    ///         Bar::new("Cost", 80.0).color(Color::Red),
    ///     ]),
    /// ];
    /// ui.bar_chart_stacked(&groups, 20);
    /// # });
    /// ```
    pub fn bar_chart_stacked(&mut self, groups: &[BarGroup], max_height: u32) -> Response {
        self.bar_chart_stacked_with(groups, |_| {}, max_height)
    }

    /// Render a stacked bar chart with custom configuration.
    ///
    /// Uses [`BarChartConfig`] for bar width, gap, and max value settings.
    pub fn bar_chart_stacked_with(
        &mut self,
        groups: &[BarGroup],
        configure: impl FnOnce(&mut BarChartConfig),
        max_height: u32,
    ) -> Response {
        if groups.is_empty() {
            return Response::none();
        }

        let all_bars: Vec<&Bar> = groups.iter().flat_map(|g| g.bars.iter()).collect();
        if all_bars.is_empty() {
            return Response::none();
        }

        let mut config = BarChartConfig::default();
        config.bar_width(3).bar_gap(1);
        configure(&mut config);

        // Find max stacked total
        let max_total: f64 = groups
            .iter()
            .map(|g| bounded_positive_sum(g.bars.iter().map(|bar| bar.value)))
            .fold(1.0, f64::max);
        let denom = config
            .max_value
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(max_total);

        let chart_height = max_height.max(1) as usize;
        let bar_width = config.bar_width.max(1) as usize;
        let gap = config.bar_gap as i32;

        const FRACTION_BLOCKS: [char; 8] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇'];

        let response = self.begin_viz_root();
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

        // Compute stacked units per group
        struct StackedSegment {
            units: usize,
            color: Color,
        }
        let stacked_groups: Vec<(String, Vec<StackedSegment>)> = groups
            .iter()
            .map(|g| {
                let segs: Vec<StackedSegment> = g
                    .bars
                    .iter()
                    .map(|b| {
                        let normalized = unit_ratio(finite_or_zero(b.value).max(0.0), 0.0, denom);
                        StackedSegment {
                            units: (normalized * chart_height as f64 * 8.0).round() as usize,
                            color: b.color.unwrap_or(self.theme.primary),
                        }
                    })
                    .collect();
                (g.label.clone(), segs)
            })
            .collect();

        // Render rows top to bottom
        for row in (0..chart_height).rev() {
            self.skip_interaction_slot();
            self.commands
                .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                    direction: Direction::Row,
                    gap,
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

            let row_base = row * 8;

            for (_label, segs) in &stacked_groups {
                // Find which segment covers this row
                let mut accumulated = 0usize;
                let mut cell_char = ' ';
                let mut cell_color = self.theme.bg;

                for seg in segs {
                    let seg_bottom = accumulated;
                    let seg_top = accumulated + seg.units;

                    if seg_top <= row_base {
                        // Segment is entirely below this row
                        accumulated = seg_top;
                        continue;
                    }

                    if seg_bottom >= row_base + 8 {
                        // Segment is entirely above this row
                        break;
                    }

                    // This segment covers (part of) this row
                    let local_bottom = seg_bottom.saturating_sub(row_base);
                    let local_top = (seg_top - row_base).min(8);
                    let fill = local_top - local_bottom;

                    if local_bottom == 0 {
                        // This segment starts from the bottom of the cell
                        cell_char = if fill >= 8 {
                            '█'
                        } else {
                            FRACTION_BLOCKS[fill]
                        };
                        cell_color = seg.color;
                    } else {
                        // This segment starts partway up — just use full block
                        cell_char = '█';
                        cell_color = seg.color;
                    }

                    accumulated = seg_top;
                }

                let fill_text = cell_char.to_string().repeat(bar_width);
                self.styled(fill_text, Style::new().fg(cell_color));
            }

            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }

        // Labels row
        self.skip_interaction_slot();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Row,
                gap,
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
        for (label, _) in &stacked_groups {
            self.styled(
                Self::center_and_truncate_text(label, bar_width),
                Style::new().fg(self.theme.text),
            );
        }
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
        self.end_viz_root();

        response
    }
}
