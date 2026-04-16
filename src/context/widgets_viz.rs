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

        let max_label_width = data
            .iter()
            .map(|(label, _)| UnicodeWidthStr::width(*label))
            .max()
            .unwrap_or(0);
        let max_value = data
            .iter()
            .map(|(_, value)| *value)
            .fold(f64::NEG_INFINITY, f64::max);
        let denom = if max_value > 0.0 { max_value } else { 1.0 };

        self.skip_interaction_slot();
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

        for (label, value) in data {
            let label_width = UnicodeWidthStr::width(*label);
            let label_padding = " ".repeat(max_label_width.saturating_sub(label_width));
            let normalized = (*value / denom).clamp(0.0, 1.0);
            let bar = Self::horizontal_bar_text(normalized, max_width);

            self.skip_interaction_slot();
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
            let mut label_text = String::with_capacity(label.len() + label_padding.len());
            label_text.push_str(label);
            label_text.push_str(&label_padding);
            self.styled(label_text, Style::new().fg(self.theme.text));
            self.styled(bar, Style::new().fg(self.theme.primary));
            self.styled(
                format_compact_number(*value),
                Style::new().fg(self.theme.text_dim),
            );
            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        Response::none()
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
        self.bar_chart_styled_render(bars, max_size, denom, &config);

        Response::none()
    }

    fn bar_chart_styled_layout(
        &self,
        bars: &[Bar],
        configure: impl FnOnce(&mut BarChartConfig),
    ) -> (BarChartConfig, f64) {
        let mut config = BarChartConfig::default();
        configure(&mut config);

        let auto_max = bars
            .iter()
            .map(|bar| bar.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_value = config.max_value.unwrap_or(auto_max);
        let denom = if max_value > 0.0 { max_value } else { 1.0 };

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
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: bar_gap as u32,
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
        let normalized = (bar.value / denom).clamp(0.0, 1.0);
        let bar_text = Self::horizontal_bar_text(normalized, max_width);
        let color = bar.color.unwrap_or(self.theme.primary);

        self.skip_interaction_slot();
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
                ((bar.value / denom).clamp(0.0, 1.0) * chart_height as f64 * 8.0).round() as usize
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
            self.commands.push(Command::BeginContainer {
                direction: Direction::Row,
                gap: bar_gap as u32,
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
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: bar_gap as u32,
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

        let auto_max = all_bars
            .iter()
            .map(|bar| bar.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_value = config.max_value.unwrap_or(auto_max);
        let denom = if max_value > 0.0 { max_value } else { 1.0 };

        match config.direction {
            BarDirection::Horizontal => {
                self.render_grouped_horizontal_bars(groups, max_size, denom, &config)
            }
            BarDirection::Vertical => {
                self.render_grouped_vertical_bars(groups, max_size, denom, &config)
            }
        }

        Response::none()
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
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: config.group_gap as u32,
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

        for group in groups {
            self.skip_interaction_slot();
            self.commands.push(Command::BeginContainer {
                direction: Direction::Column,
                gap: config.bar_gap as u32,
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

            self.styled(group.label.clone(), Style::new().bold().fg(self.theme.text));

            for bar in &group.bars {
                let label_width = UnicodeWidthStr::width(bar.label.as_str());
                let label_padding = " ".repeat(max_label_width.saturating_sub(label_width));
                let normalized = (bar.value / denom).clamp(0.0, 1.0);
                let bar_text = Self::horizontal_bar_text(normalized, max_width);

                self.skip_interaction_slot();
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
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: config.group_gap as u32,
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
        let filled = (normalized.clamp(0.0, 1.0) * max_width as f64).round() as usize;
        "█".repeat(filled)
    }

    fn bar_display_value(bar: &Bar) -> String {
        bar.text_value
            .clone()
            .unwrap_or_else(|| format_compact_number(bar.value))
    }

    fn center_and_truncate_text(text: &str, width: usize) -> String {
        if width == 0 {
            return String::new();
        }

        let mut out = String::new();
        let mut used = 0usize;
        for ch in text.chars() {
            let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
            if used + cw > width {
                break;
            }
            out.push(ch);
            used += cw;
        }
        center_text(&out, width)
    }

    /// Render a single-line sparkline from numeric data.
    ///
    /// Uses the last `width` points (or fewer if the data is shorter) and maps
    /// each point to one of `▁▂▃▄▅▆▇█`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let samples = [12.0, 9.0, 14.0, 18.0, 16.0, 21.0, 20.0, 24.0];
    /// ui.sparkline(&samples, 16);
    /// # });
    /// ```
    ///
    /// For per-point colors and missing values, see [`sparkline_styled`](Self::sparkline_styled).
    pub fn sparkline(&mut self, data: &[f64], width: u32) -> Response {
        const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        let w = width as usize;
        if data.is_empty() || w == 0 {
            return Response::none();
        }

        let points: Vec<f64> = if data.len() >= w {
            data[data.len() - w..].to_vec()
        } else if data.len() == 1 {
            vec![data[0]; w]
        } else {
            (0..w)
                .map(|i| {
                    let t = i as f64 * (data.len() - 1) as f64 / (w - 1) as f64;
                    let idx = t.floor() as usize;
                    let frac = t - idx as f64;
                    if idx + 1 < data.len() {
                        data[idx] * (1.0 - frac) + data[idx + 1] * frac
                    } else {
                        data[idx.min(data.len() - 1)]
                    }
                })
                .collect()
        };

        let min = points.iter().copied().fold(f64::INFINITY, f64::min);
        let max = points.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;

        let line: String = points
            .iter()
            .map(|&value| {
                let normalized = if range == 0.0 {
                    0.5
                } else {
                    (value - min) / range
                };
                let idx = (normalized * 7.0).round() as usize;
                BLOCKS[idx.min(7)]
            })
            .collect();

        self.styled(line, Style::new().fg(self.theme.primary));
        Response::none()
    }

    /// Render a sparkline with per-point colors.
    ///
    /// Each point can have its own color via `(f64, Option<Color>)` tuples.
    /// Use `f64::NAN` for absent values (rendered as spaces).
    ///
    /// # Example
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// let data: Vec<(f64, Option<Color>)> = vec![
    ///     (12.0, Some(Color::Green)),
    ///     (9.0, Some(Color::Red)),
    ///     (14.0, Some(Color::Green)),
    ///     (f64::NAN, None),
    ///     (18.0, Some(Color::Cyan)),
    /// ];
    /// ui.sparkline_styled(&data, 16);
    /// # });
    /// ```
    pub fn sparkline_styled(&mut self, data: &[(f64, Option<Color>)], width: u32) -> Response {
        const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        let w = width as usize;
        if data.is_empty() || w == 0 {
            return Response::none();
        }

        let window: Vec<(f64, Option<Color>)> = if data.len() >= w {
            data[data.len() - w..].to_vec()
        } else if data.len() == 1 {
            vec![data[0]; w]
        } else {
            (0..w)
                .map(|i| {
                    let t = i as f64 * (data.len() - 1) as f64 / (w - 1) as f64;
                    let idx = t.floor() as usize;
                    let frac = t - idx as f64;
                    let nearest = if frac < 0.5 {
                        idx
                    } else {
                        (idx + 1).min(data.len() - 1)
                    };
                    let color = data[nearest].1;
                    let (v1, _) = data[idx];
                    let (v2, _) = data[(idx + 1).min(data.len() - 1)];
                    let value = if v1.is_nan() || v2.is_nan() {
                        if frac < 0.5 {
                            v1
                        } else {
                            v2
                        }
                    } else {
                        v1 * (1.0 - frac) + v2 * frac
                    };
                    (value, color)
                })
                .collect()
        };

        let mut finite_values = window
            .iter()
            .map(|(value, _)| *value)
            .filter(|value| !value.is_nan());
        let Some(first) = finite_values.next() else {
            self.styled(
                " ".repeat(window.len()),
                Style::new().fg(self.theme.text_dim),
            );
            return Response::none();
        };

        let mut min = first;
        let mut max = first;
        for value in finite_values {
            min = f64::min(min, value);
            max = f64::max(max, value);
        }
        let range = max - min;

        let mut cells: Vec<(char, Color)> = Vec::with_capacity(window.len());
        for (value, color) in &window {
            if value.is_nan() {
                cells.push((' ', self.theme.text_dim));
                continue;
            }

            let normalized = if range == 0.0 {
                0.5
            } else {
                ((*value - min) / range).clamp(0.0, 1.0)
            };
            let idx = (normalized * 7.0).round() as usize;
            cells.push((BLOCKS[idx.min(7)], color.unwrap_or(self.theme.primary)));
        }

        self.skip_interaction_slot();
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

        if cells.is_empty() {
            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
            return Response::none();
        }

        let mut seg = String::new();
        let mut seg_color = cells[0].1;
        for (ch, color) in cells {
            if color != seg_color {
                self.styled(seg, Style::new().fg(seg_color));
                seg = String::new();
                seg_color = color;
            }
            seg.push(ch);
        }
        if !seg.is_empty() {
            self.styled(seg, Style::new().fg(seg_color));
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        Response::none()
    }

    /// Render a multi-row line chart using braille characters.
    ///
    /// `width` and `height` are terminal cell dimensions. Internally this uses
    /// braille dot resolution (`width*2` x `height*4`) for smoother plotting.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let data = [1.0, 3.0, 2.0, 5.0, 4.0, 6.0, 3.0, 7.0];
    /// ui.line_chart(&data, 40, 8);
    /// # });
    /// ```
    pub fn line_chart(&mut self, data: &[f64], width: u32, height: u32) -> Response {
        self.line_chart_colored(data, width, height, self.theme.primary)
    }

    /// Render a multi-row line chart using a custom color.
    pub fn line_chart_colored(
        &mut self,
        data: &[f64],
        width: u32,
        height: u32,
        color: Color,
    ) -> Response {
        self.render_line_chart_internal(data, width, height, color, false)
    }

    /// Render a multi-row area chart using the primary theme color.
    pub fn area_chart(&mut self, data: &[f64], width: u32, height: u32) -> Response {
        self.area_chart_colored(data, width, height, self.theme.primary)
    }

    /// Render a multi-row area chart using a custom color.
    pub fn area_chart_colored(
        &mut self,
        data: &[f64],
        width: u32,
        height: u32,
        color: Color,
    ) -> Response {
        self.render_line_chart_internal(data, width, height, color, true)
    }

    fn render_line_chart_internal(
        &mut self,
        data: &[f64],
        width: u32,
        height: u32,
        color: Color,
        fill: bool,
    ) -> Response {
        if data.is_empty() || width == 0 || height == 0 {
            return Response::none();
        }

        let cols = width as usize;
        let rows = height as usize;
        let px_w = cols * 2;
        let px_h = rows * 4;

        let min = data.iter().copied().fold(f64::INFINITY, f64::min);
        let max = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let range = if (max - min).abs() < f64::EPSILON {
            1.0
        } else {
            max - min
        };

        let points: Vec<usize> = (0..px_w)
            .map(|px| {
                let data_idx = if px_w <= 1 {
                    0.0
                } else {
                    px as f64 * (data.len() - 1) as f64 / (px_w - 1) as f64
                };
                let idx = data_idx.floor() as usize;
                let frac = data_idx - idx as f64;
                let value = if idx + 1 < data.len() {
                    data[idx] * (1.0 - frac) + data[idx + 1] * frac
                } else {
                    data[idx.min(data.len() - 1)]
                };

                let normalized = (value - min) / range;
                let py = ((1.0 - normalized) * (px_h - 1) as f64).round() as usize;
                py.min(px_h - 1)
            })
            .collect();

        const LEFT_BITS: [u32; 4] = [0x01, 0x02, 0x04, 0x40];
        const RIGHT_BITS: [u32; 4] = [0x08, 0x10, 0x20, 0x80];

        let mut grid = vec![vec![0u32; cols]; rows];

        for i in 0..points.len() {
            let px = i;
            let py = points[i];
            let char_col = px / 2;
            let char_row = py / 4;
            let sub_col = px % 2;
            let sub_row = py % 4;

            if char_col < cols && char_row < rows {
                grid[char_row][char_col] |= if sub_col == 0 {
                    LEFT_BITS[sub_row]
                } else {
                    RIGHT_BITS[sub_row]
                };
            }

            if i + 1 < points.len() {
                let py_next = points[i + 1];
                let (y_start, y_end) = if py <= py_next {
                    (py, py_next)
                } else {
                    (py_next, py)
                };
                for y in y_start..=y_end {
                    let cell_row = y / 4;
                    let sub_y = y % 4;
                    if char_col < cols && cell_row < rows {
                        grid[cell_row][char_col] |= if sub_col == 0 {
                            LEFT_BITS[sub_y]
                        } else {
                            RIGHT_BITS[sub_y]
                        };
                    }
                }
            }

            if fill {
                for y in py..px_h {
                    let cell_row = y / 4;
                    let sub_y = y % 4;
                    if char_col < cols && cell_row < rows {
                        grid[cell_row][char_col] |= if sub_col == 0 {
                            LEFT_BITS[sub_y]
                        } else {
                            RIGHT_BITS[sub_y]
                        };
                    }
                }
            }
        }

        let style = Style::new().fg(color);
        for row in grid {
            let line: String = row
                .iter()
                .map(|&bits| char::from_u32(0x2800 + bits).unwrap_or(' '))
                .collect();
            self.styled(line, style);
        }

        Response::none()
    }

    /// Render an OHLC candlestick chart.
    pub fn candlestick(
        &mut self,
        candles: &[Candle],
        up_color: Color,
        down_color: Color,
    ) -> Response {
        if candles.is_empty() {
            return Response::none();
        }

        let candles = candles.to_vec();
        self.container().grow(1).draw(move |buf, rect| {
            let w = rect.width as usize;
            let h = rect.height as usize;
            if w < 2 || h < 2 {
                return;
            }

            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for c in &candles {
                if c.low.is_finite() {
                    lo = lo.min(c.low);
                }
                if c.high.is_finite() {
                    hi = hi.max(c.high);
                }
            }

            if !lo.is_finite() || !hi.is_finite() {
                return;
            }

            let range = if (hi - lo).abs() < 0.01 { 1.0 } else { hi - lo };
            let map_y = |v: f64| -> usize {
                let t = ((v - lo) / range).clamp(0.0, 1.0);
                ((1.0 - t) * (h.saturating_sub(1)) as f64).round() as usize
            };

            for (i, c) in candles.iter().enumerate() {
                if !c.open.is_finite()
                    || !c.high.is_finite()
                    || !c.low.is_finite()
                    || !c.close.is_finite()
                {
                    continue;
                }

                let x0 = i * w / candles.len();
                let x1 = ((i + 1) * w / candles.len()).saturating_sub(1).max(x0);
                if x0 >= w {
                    continue;
                }
                let xm = (x0 + x1) / 2;
                let color = if c.close >= c.open {
                    up_color
                } else {
                    down_color
                };

                let wt = map_y(c.high);
                let wb = map_y(c.low);
                for row in wt..=wb.min(h - 1) {
                    buf.set_char(
                        rect.x + xm as u32,
                        rect.y + row as u32,
                        '│',
                        Style::new().fg(color),
                    );
                }

                let bt = map_y(c.open.max(c.close));
                let bb = map_y(c.open.min(c.close));
                for row in bt..=bb.min(h - 1) {
                    for col in x0..=x1.min(w - 1) {
                        buf.set_char(
                            rect.x + col as u32,
                            rect.y + row as u32,
                            '█',
                            Style::new().fg(color),
                        );
                    }
                }
            }
        });

        Response::none()
    }

    /// Render a heatmap from a 2D data grid.
    ///
    /// Each cell maps to a block character with color intensity:
    /// low values -> dim/dark, high values -> bright/saturated.
    ///
    /// # Arguments
    /// * `data` - Row-major 2D grid (outer = rows, inner = columns)
    /// * `width` - Widget width in terminal cells
    /// * `height` - Widget height in terminal cells
    /// * `low_color` - Color for minimum values
    /// * `high_color` - Color for maximum values
    pub fn heatmap(
        &mut self,
        data: &[Vec<f64>],
        width: u32,
        height: u32,
        low_color: Color,
        high_color: Color,
    ) -> Response {
        fn blend_color(a: Color, b: Color, t: f64) -> Color {
            let t = t.clamp(0.0, 1.0);
            match (a, b) {
                (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => Color::Rgb(
                    (r1 as f64 * (1.0 - t) + r2 as f64 * t).round() as u8,
                    (g1 as f64 * (1.0 - t) + g2 as f64 * t).round() as u8,
                    (b1 as f64 * (1.0 - t) + b2 as f64 * t).round() as u8,
                ),
                _ => {
                    if t > 0.5 {
                        b
                    } else {
                        a
                    }
                }
            }
        }

        if data.is_empty() || width == 0 || height == 0 {
            return Response::none();
        }

        let data_rows = data.len();
        let max_data_cols = data.iter().map(Vec::len).max().unwrap_or(0);
        if max_data_cols == 0 {
            return Response::none();
        }

        let mut min_value = f64::INFINITY;
        let mut max_value = f64::NEG_INFINITY;
        for row in data {
            for value in row {
                if value.is_finite() {
                    min_value = min_value.min(*value);
                    max_value = max_value.max(*value);
                }
            }
        }

        if !min_value.is_finite() || !max_value.is_finite() {
            return Response::none();
        }

        let range = max_value - min_value;
        let zero_range = range.abs() < f64::EPSILON;
        let cols = width as usize;
        let rows = height as usize;

        for row_idx in 0..rows {
            let data_row_idx = (row_idx * data_rows / rows).min(data_rows.saturating_sub(1));
            let source_row = &data[data_row_idx];
            let source_cols = source_row.len();

            self.skip_interaction_slot();
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

            let mut segment = String::new();
            let mut segment_color: Option<Color> = None;

            for col_idx in 0..cols {
                let normalized = if source_cols == 0 {
                    0.0
                } else {
                    let data_col_idx = (col_idx * source_cols / cols).min(source_cols - 1);
                    let value = source_row[data_col_idx];

                    if !value.is_finite() {
                        0.0
                    } else if zero_range {
                        0.5
                    } else {
                        ((value - min_value) / range).clamp(0.0, 1.0)
                    }
                };

                let color = blend_color(low_color, high_color, normalized);

                match segment_color {
                    Some(current) if current == color => {
                        segment.push('█');
                    }
                    Some(current) => {
                        self.styled(std::mem::take(&mut segment), Style::new().fg(current));
                        segment.push('█');
                        segment_color = Some(color);
                    }
                    None => {
                        segment.push('█');
                        segment_color = Some(color);
                    }
                }
            }

            if let Some(color) = segment_color {
                self.styled(segment, Style::new().fg(color));
            }

            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }

        Response::none()
    }

    /// Render a braille drawing canvas.
    ///
    /// The closure receives a [`CanvasContext`] for pixel-level drawing. Each
    /// terminal cell maps to a 2x4 braille dot matrix, giving `width*2` x
    /// `height*4` pixel resolution.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.canvas(40, 10, |cv| {
    ///     cv.line(0, 0, cv.width() - 1, cv.height() - 1);
    ///     cv.circle(40, 20, 15);
    /// });
    /// # });
    /// ```
    pub fn canvas(
        &mut self,
        width: u32,
        height: u32,
        draw: impl FnOnce(&mut CanvasContext),
    ) -> Response {
        if width == 0 || height == 0 {
            return Response::none();
        }

        let mut canvas = CanvasContext::new(width as usize, height as usize);
        draw(&mut canvas);

        for segments in canvas.render() {
            self.skip_interaction_slot();
            self.commands.push(Command::BeginContainer {
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
            });
            for (text, color) in segments {
                let c = if color == Color::Reset {
                    self.theme.primary
                } else {
                    color
                };
                self.styled(text, Style::new().fg(c));
            }
            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }

        Response::none()
    }

    /// Render a multi-series chart with axes, legend, and auto-scaling.
    ///
    /// `width` and `height` must be non-zero. For dynamic sizing, read terminal
    /// dimensions first (for example via `ui.width()` / `ui.height()`) and pass
    /// the computed values to this method.
    pub fn chart(
        &mut self,
        configure: impl FnOnce(&mut ChartBuilder),
        width: u32,
        height: u32,
    ) -> Response {
        if width == 0 || height == 0 {
            return Response::none();
        }

        let axis_style = Style::new().fg(self.theme.text_dim);
        let mut builder = ChartBuilder::new(width, height, axis_style, axis_style);
        configure(&mut builder);

        let config = builder.build();
        let rows = render_chart(&config);

        for row in rows {
            self.skip_interaction_slot();
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
            for (text, style) in row.segments {
                self.styled(text, style);
            }
            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }

        Response::none()
    }

    /// Renders a scatter plot.
    ///
    /// Each point is a (x, y) tuple. Uses braille markers.
    pub fn scatter(&mut self, data: &[(f64, f64)], width: u32, height: u32) -> Response {
        self.chart(
            |c| {
                c.scatter(data);
                c.grid(true);
            },
            width,
            height,
        )
    }

    /// Render a histogram from raw data with auto-binning.
    pub fn histogram(&mut self, data: &[f64], width: u32, height: u32) -> Response {
        self.histogram_with(data, |_| {}, width, height)
    }

    /// Render a histogram with configuration options.
    pub fn histogram_with(
        &mut self,
        data: &[f64],
        configure: impl FnOnce(&mut HistogramBuilder),
        width: u32,
        height: u32,
    ) -> Response {
        if width == 0 || height == 0 {
            return Response::none();
        }

        let mut options = HistogramBuilder::default();
        configure(&mut options);
        let axis_style = Style::new().fg(self.theme.text_dim);
        let config = build_histogram_config(data, &options, width, height, axis_style);
        let rows = render_chart(&config);

        for row in rows {
            self.skip_interaction_slot();
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
            for (text, style) in row.segments {
                self.styled(text, style);
            }
            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }

        Response::none()
    }

    #[cfg(feature = "qrcode")]
    /// Render a QR code using half-block characters.
    pub fn qr_code(&mut self, data: impl AsRef<str>) -> Response {
        let code = match qrcode::QrCode::new(data.as_ref()) {
            Ok(code) => code,
            Err(_) => {
                self.text("[QR Error]");
                return Response::none();
            }
        };

        let modules_per_side = code.width();
        let modules = code.to_colors();
        let qr_side = modules_per_side + 2;
        let qr_width = qr_side;
        let qr_height = qr_side.div_ceil(2);
        let theme_text = self.theme.text;
        let theme_bg = self.theme.bg;

        self.container()
            .w(qr_width as u32)
            .h(qr_height as u32)
            .draw(move |buf, rect| {
                let draw_w = (rect.width as usize).min(qr_width);
                let draw_h = (rect.height as usize).min(qr_height);

                for row in 0..draw_h {
                    let upper_y = row * 2;
                    let lower_y = upper_y + 1;

                    for x in 0..draw_w {
                        let resolve_module_color = |mx: usize, my: usize| -> Color {
                            let dark =
                                if mx == 0 || my == 0 || mx == qr_side - 1 || my == qr_side - 1 {
                                    false
                                } else {
                                    let inner_x = mx - 1;
                                    let inner_y = my - 1;
                                    let idx = inner_y * modules_per_side + inner_x;
                                    matches!(modules.get(idx), Some(qrcode::types::Color::Dark))
                                };

                            if dark {
                                theme_text
                            } else {
                                theme_bg
                            }
                        };

                        let upper = resolve_module_color(x, upper_y);
                        let lower = if lower_y < qr_side {
                            resolve_module_color(x, lower_y)
                        } else {
                            theme_bg
                        };

                        buf.set_char(
                            rect.x + x as u32,
                            rect.y + row as u32,
                            '▀',
                            Style::new().fg(upper).bg(lower),
                        );
                    }
                }
            });

        Response::none()
    }

    /// Render a heatmap using half-block characters for 2× vertical resolution.
    ///
    /// Each terminal cell packs two data rows using `▀` with `fg` for the upper
    /// half and `bg` for the lower half. This doubles the effective vertical
    /// resolution compared to [`heatmap`](Self::heatmap).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// let data: Vec<Vec<f64>> = (0..20)
    ///     .map(|r| (0..40).map(|c| ((r * 3 + c * 7) % 20) as f64).collect())
    ///     .collect();
    /// ui.heatmap_halfblock(&data, 40, 10, Color::Rgb(10, 10, 40), Color::Rgb(255, 80, 30));
    /// # });
    /// ```
    pub fn heatmap_halfblock(
        &mut self,
        data: &[Vec<f64>],
        width: u32,
        height: u32,
        low_color: Color,
        high_color: Color,
    ) -> Response {
        if data.is_empty() || width == 0 || height == 0 {
            return Response::none();
        }

        let data_rows = data.len();
        let max_data_cols = data.iter().map(Vec::len).max().unwrap_or(0);
        if max_data_cols == 0 {
            return Response::none();
        }

        let mut min_value = f64::INFINITY;
        let mut max_value = f64::NEG_INFINITY;
        for row in data {
            for value in row {
                if value.is_finite() {
                    min_value = min_value.min(*value);
                    max_value = max_value.max(*value);
                }
            }
        }

        if !min_value.is_finite() || !max_value.is_finite() {
            return Response::none();
        }

        let range = max_value - min_value;
        let zero_range = range.abs() < f64::EPSILON;

        let data = data.to_vec();
        let cols = width as usize;
        let rows = height as usize;
        // Each terminal row maps to 2 data rows
        let virtual_rows = rows * 2;

        self.container().w(width).h(height).draw(move |buf, rect| {
            let w = rect.width as usize;
            let h = rect.height as usize;
            if w == 0 || h == 0 {
                return;
            }

            let sample = |data_row_idx: usize, col_idx: usize| -> f64 {
                let src_row = &data[data_row_idx.min(data_rows.saturating_sub(1))];
                let src_cols = src_row.len();
                if src_cols == 0 {
                    return 0.0;
                }
                let data_col = (col_idx * src_cols / cols.max(1)).min(src_cols - 1);
                let v = src_row[data_col];
                if !v.is_finite() {
                    0.0
                } else if zero_range {
                    0.5
                } else {
                    ((v - min_value) / range).clamp(0.0, 1.0)
                }
            };

            let blend = |t: f64| -> Color {
                let t = t.clamp(0.0, 1.0);
                match (low_color, high_color) {
                    (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => Color::Rgb(
                        (r1 as f64 * (1.0 - t) + r2 as f64 * t).round() as u8,
                        (g1 as f64 * (1.0 - t) + g2 as f64 * t).round() as u8,
                        (b1 as f64 * (1.0 - t) + b2 as f64 * t).round() as u8,
                    ),
                    _ => {
                        if t > 0.5 {
                            high_color
                        } else {
                            low_color
                        }
                    }
                }
            };

            for row in 0..h {
                let upper_data_row =
                    (row * 2 * data_rows / virtual_rows).min(data_rows.saturating_sub(1));
                let lower_data_row =
                    ((row * 2 + 1) * data_rows / virtual_rows).min(data_rows.saturating_sub(1));

                for col in 0..w.min(cols) {
                    let upper_t = sample(upper_data_row, col);
                    let lower_t = sample(lower_data_row, col);
                    let upper_color = blend(upper_t);
                    let lower_color = blend(lower_t);

                    buf.set_char(
                        rect.x + col as u32,
                        rect.y + row as u32,
                        '▀',
                        Style::new().fg(upper_color).bg(lower_color),
                    );
                }
            }
        });

        Response::none()
    }

    /// Render a candlestick chart with heavy box-drawing and half-block precision.
    ///
    /// Uses `┃` for wicks (heavier than `│`) and `▀`/`▄` at body edges for
    /// sub-cell vertical precision, effectively doubling the price resolution.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::{Candle, Color};
    /// let candles = vec![
    ///     Candle { open: 100.0, high: 108.0, low: 98.0, close: 105.0 },
    ///     Candle { open: 105.0, high: 112.0, low: 103.0, close: 110.0 },
    /// ];
    /// ui.candlestick_hd(&candles, Color::Rgb(38, 166, 91), Color::Rgb(234, 57, 67));
    /// # });
    /// ```
    pub fn candlestick_hd(
        &mut self,
        candles: &[Candle],
        up_color: Color,
        down_color: Color,
    ) -> Response {
        if candles.is_empty() {
            return Response::none();
        }

        let candles = candles.to_vec();
        self.container().grow(1).draw(move |buf, rect| {
            let w = rect.width as usize;
            let h = rect.height as usize;
            if w < 2 || h < 2 {
                return;
            }

            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for c in &candles {
                if c.low.is_finite() {
                    lo = lo.min(c.low);
                }
                if c.high.is_finite() {
                    hi = hi.max(c.high);
                }
            }
            if !lo.is_finite() || !hi.is_finite() {
                return;
            }

            let price_range = if (hi - lo).abs() < 0.01 { 1.0 } else { hi - lo };
            let map_y = |v: f64| -> usize {
                let t = ((v - lo) / price_range).clamp(0.0, 1.0);
                ((1.0 - t) * h.saturating_sub(1) as f64).round() as usize
            };

            let n = candles.len();

            for (i, c) in candles.iter().enumerate() {
                if !c.open.is_finite()
                    || !c.high.is_finite()
                    || !c.low.is_finite()
                    || !c.close.is_finite()
                {
                    continue;
                }

                // Distribute candles evenly across full width
                let x0 = i * w / n;
                let x1 = ((i + 1) * w / n).saturating_sub(1).max(x0);
                if x0 >= w {
                    continue;
                }
                // Wick at exact center of body range (inclusive)
                let xm = x0 + (x1 - x0) / 2;
                let color = if c.close >= c.open {
                    up_color
                } else {
                    down_color
                };

                // Wick
                let wick_top = map_y(c.high);
                let wick_bot = map_y(c.low);
                for row in wick_top..=wick_bot.min(h - 1) {
                    buf.set_char(
                        rect.x + xm as u32,
                        rect.y + row as u32,
                        '┃',
                        Style::new().fg(color),
                    );
                }

                // Body
                let body_top = map_y(c.open.max(c.close));
                let body_bot = map_y(c.open.min(c.close));
                for row in body_top..=body_bot.min(h - 1) {
                    for col in x0..=x1.min(w - 1) {
                        buf.set_char(
                            rect.x + col as u32,
                            rect.y + row as u32,
                            '█',
                            Style::new().fg(color),
                        );
                    }
                }
            }
        });

        Response::none()
    }

    /// Render a treemap using the squarified layout algorithm.
    ///
    /// Each item occupies a rectangle proportional to its value, filled with the
    /// item's color and labeled when space permits.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::{TreemapItem, Color};
    /// let items = vec![
    ///     TreemapItem::new("Rust", 40.0, Color::Cyan),
    ///     TreemapItem::new("Go", 25.0, Color::Blue),
    ///     TreemapItem::new("Python", 20.0, Color::Yellow),
    ///     TreemapItem::new("Java", 15.0, Color::Red),
    /// ];
    /// ui.treemap(&items);
    /// # });
    /// ```
    pub fn treemap(&mut self, items: &[TreemapItem]) -> Response {
        if items.is_empty() {
            return Response::none();
        }

        let items = items.to_vec();
        self.container().grow(1).draw(move |buf, rect| {
            let w = rect.width as usize;
            let h = rect.height as usize;
            if w < 2 || h < 2 {
                return;
            }

            // Filter out items that would be too small to render (< 1 cell)
            let total_area = w as f64 * h as f64;
            let total_value: f64 = items.iter().map(|i| i.value.max(0.0)).sum();
            let min_area_threshold = 1.0; // at least 1 cell
            let visible_items: Vec<&TreemapItem> = if total_value > 0.0 {
                items
                    .iter()
                    .filter(|item| {
                        item.value.max(0.0) / total_value * total_area >= min_area_threshold
                    })
                    .collect()
            } else {
                return;
            };

            if visible_items.is_empty() {
                return;
            }

            // Build filtered items for layout
            let filtered: Vec<TreemapItem> = visible_items.into_iter().cloned().collect();
            let rects = squarify_layout(&filtered, 0.0, 0.0, w as f64, h as f64);

            for (item, r) in filtered.iter().zip(rects.iter()) {
                // Integer cell bounds — use round for consistent placement
                let x0 = r.x.round() as usize;
                let y0 = r.y.round() as usize;
                let x1 = (r.x + r.w).round() as usize;
                let y1 = (r.y + r.h).round() as usize;

                let cell_w = x1.min(w).saturating_sub(x0);
                let cell_h = y1.min(h).saturating_sub(y0);
                if cell_w == 0 || cell_h == 0 {
                    continue;
                }

                // Fill the rectangle with the item's color
                for row in y0..y1.min(h) {
                    for col in x0..x1.min(w) {
                        buf.set_char(
                            rect.x + col as u32,
                            rect.y + row as u32,
                            ' ',
                            Style::new().bg(item.color),
                        );
                    }
                }

                let text_color = treemap_label_color(item.color);

                // Label: truncate to fit, center in cell
                if cell_w >= 2 {
                    let max_label_w = cell_w.saturating_sub(1);
                    let label = if item.label.len() > max_label_w {
                        &item.label[..max_label_w]
                    } else {
                        &item.label
                    };
                    let label_y = y0 + cell_h / 2;
                    let label_x = x0 + (cell_w.saturating_sub(label.len())) / 2;
                    if label_y < y1.min(h) {
                        for (offset, ch) in label.chars().enumerate() {
                            let cx = label_x + offset;
                            if cx < x1.min(w) {
                                buf.set_char(
                                    rect.x + cx as u32,
                                    rect.y + label_y as u32,
                                    ch,
                                    Style::new().fg(text_color).bg(item.color).bold(),
                                );
                            }
                        }
                    }

                    // Value label below if space permits
                    if cell_h >= 3 {
                        let value_str = format_compact_number(item.value);
                        let value_y = label_y + 1;
                        if value_y < y1.min(h) && value_str.len() < cell_w {
                            let vx = x0 + (cell_w.saturating_sub(value_str.len())) / 2;
                            for (offset, ch) in value_str.chars().enumerate() {
                                let cx = vx + offset;
                                if cx < x1.min(w) {
                                    buf.set_char(
                                        rect.x + cx as u32,
                                        rect.y + value_y as u32,
                                        ch,
                                        Style::new().fg(text_color).bg(item.color).dim(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        });

        Response::none()
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
            .map(|g| g.bars.iter().map(|b| b.value.max(0.0)).sum::<f64>())
            .fold(f64::NEG_INFINITY, f64::max);
        let denom = config.max_value.unwrap_or(max_total);
        let denom = if denom > 0.0 { denom } else { 1.0 };

        let chart_height = max_height.max(1) as usize;
        let bar_width = config.bar_width.max(1) as usize;
        let gap = config.bar_gap as u32;

        const FRACTION_BLOCKS: [char; 8] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇'];

        self.skip_interaction_slot();
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
                        let normalized = (b.value.max(0.0) / denom).clamp(0.0, 1.0);
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
            self.commands.push(Command::BeginContainer {
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
            });

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
        self.commands.push(Command::BeginContainer {
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
        });
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

        Response::none()
    }
}

/// A single item in a treemap.
#[derive(Debug, Clone)]
pub struct TreemapItem {
    /// Display label.
    pub label: String,
    /// Numeric value determining area.
    pub value: f64,
    /// Fill color for this item's rectangle.
    pub color: Color,
}

impl TreemapItem {
    /// Create a new treemap item.
    pub fn new(label: impl Into<String>, value: f64, color: Color) -> Self {
        Self {
            label: label.into(),
            value,
            color,
        }
    }
}

/// Rectangle produced by the squarified layout.
#[derive(Clone)]
struct LayoutRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Squarified treemap layout algorithm (Bruls, Huizing, van Wijk 2000).
fn squarify_layout(items: &[TreemapItem], x: f64, y: f64, w: f64, h: f64) -> Vec<LayoutRect> {
    if items.is_empty() || w <= 0.0 || h <= 0.0 {
        return Vec::new();
    }

    let total: f64 = items.iter().map(|i| i.value.max(0.0)).sum();
    if total <= 0.0 {
        return items
            .iter()
            .map(|_| LayoutRect {
                x,
                y,
                w: 0.0,
                h: 0.0,
            })
            .collect();
    }

    // Normalize values to fill the available area
    let area = w * h;
    let mut sorted_indices: Vec<usize> = (0..items.len()).collect();
    sorted_indices.sort_by(|a, b| {
        items[*b]
            .value
            .partial_cmp(&items[*a].value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let areas: Vec<f64> = sorted_indices
        .iter()
        .map(|&i| items[i].value.max(0.0) / total * area)
        .collect();

    let mut result = vec![
        LayoutRect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        };
        items.len()
    ];
    squarify_recursive(&areas, &sorted_indices, x, y, w, h, &mut result);
    result
}

fn worst_ratio(row: &[f64], side: f64) -> f64 {
    if row.is_empty() || side <= 0.0 {
        return f64::INFINITY;
    }
    let sum: f64 = row.iter().sum();
    let mut worst = 0.0f64;
    for &a in row {
        if a <= 0.0 {
            continue;
        }
        let ratio1 = (side * side * a) / (sum * sum);
        let ratio2 = (sum * sum) / (side * side * a);
        worst = worst.max(ratio1.max(ratio2));
    }
    worst
}

fn squarify_recursive(
    areas: &[f64],
    indices: &[usize],
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    result: &mut [LayoutRect],
) {
    if areas.is_empty() || w <= 0.0 || h <= 0.0 {
        return;
    }

    if areas.len() == 1 {
        result[indices[0]] = LayoutRect { x, y, w, h };
        return;
    }

    let short_side = w.min(h);
    let mut row: Vec<f64> = Vec::new();
    let mut row_indices: Vec<usize> = Vec::new();

    for (i, &area) in areas.iter().enumerate() {
        let mut candidate = row.clone();
        candidate.push(area);
        if row.is_empty() || worst_ratio(&candidate, short_side) <= worst_ratio(&row, short_side) {
            row.push(area);
            row_indices.push(indices[i]);
        } else {
            // Layout the current row
            let row_sum: f64 = row.iter().sum();
            let row_fraction = row_sum / (w * h).max(f64::EPSILON);

            if w >= h {
                // Lay out vertically on the left
                let row_w = w * row_fraction;
                let mut cy = y;
                for (j, &a) in row.iter().enumerate() {
                    let cell_h = if row_sum > 0.0 {
                        h * (a / row_sum)
                    } else {
                        0.0
                    };
                    result[row_indices[j]] = LayoutRect {
                        x,
                        y: cy,
                        w: row_w,
                        h: cell_h,
                    };
                    cy += cell_h;
                }
                squarify_recursive(
                    &areas[i..],
                    &indices[i..],
                    x + row_w,
                    y,
                    w - row_w,
                    h,
                    result,
                );
            } else {
                // Lay out horizontally on top
                let row_h = h * row_fraction;
                let mut cx = x;
                for (j, &a) in row.iter().enumerate() {
                    let cell_w = if row_sum > 0.0 {
                        w * (a / row_sum)
                    } else {
                        0.0
                    };
                    result[row_indices[j]] = LayoutRect {
                        x: cx,
                        y,
                        w: cell_w,
                        h: row_h,
                    };
                    cx += cell_w;
                }
                squarify_recursive(
                    &areas[i..],
                    &indices[i..],
                    x,
                    y + row_h,
                    w,
                    h - row_h,
                    result,
                );
            }
            return;
        }
    }

    // Layout remaining row
    if !row.is_empty() {
        let row_sum: f64 = row.iter().sum();
        if w >= h {
            let mut cy = y;
            for (j, &a) in row.iter().enumerate() {
                let cell_h = if row_sum > 0.0 {
                    h * (a / row_sum)
                } else {
                    0.0
                };
                result[row_indices[j]] = LayoutRect {
                    x,
                    y: cy,
                    w,
                    h: cell_h,
                };
                cy += cell_h;
            }
        } else {
            let mut cx = x;
            for (j, &a) in row.iter().enumerate() {
                let cell_w = if row_sum > 0.0 {
                    w * (a / row_sum)
                } else {
                    0.0
                };
                result[row_indices[j]] = LayoutRect {
                    x: cx,
                    y,
                    w: cell_w,
                    h,
                };
                cx += cell_w;
            }
        }
    }
}

/// Choose a contrasting label color for treemap cells.
fn treemap_label_color(bg: Color) -> Color {
    match bg {
        Color::Rgb(r, g, b) => {
            // Relative luminance (simplified)
            let lum = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
            if lum > 128.0 {
                Color::Rgb(0, 0, 0)
            } else {
                Color::Rgb(255, 255, 255)
            }
        }
        _ => Color::White,
    }
}

#[cfg(all(test, feature = "qrcode"))]
#[test]
fn test_qr_code() {
    let mut backend = crate::TestBackend::new(60, 30);
    backend.render(|ui| {
        let _ = ui.qr_code("hello");
    });

    let output = backend.to_string();
    assert!(output.contains('▀') || output.contains('█'));
}
