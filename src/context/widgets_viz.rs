use super::*;

impl Context {
    /// Render a horizontal bar chart from `(label, value)` pairs.
    ///
    /// Bars are normalized against the largest value and rendered with `█` up to
    /// `max_width` characters.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # slt::run(|ui: &mut slt::Context| {
    /// let data = [
    ///     ("Sales", 160.0),
    ///     ("Revenue", 120.0),
    ///     ("Users", 220.0),
    ///     ("Costs", 60.0),
    /// ];
    /// ui.bar_chart(&data, 24);
    ///
    /// For styled bars with per-bar colors, see [`bar_chart_styled`].
    /// # });
    /// ```
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

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
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

            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
                direction: Direction::Row,
                gap: 1,
                align: Align::Start,
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
            self.styled(
                format!("{label}{label_padding}"),
                Style::new().fg(self.theme.text),
            );
            self.styled(bar, Style::new().fg(self.theme.primary));
            self.styled(
                format_compact_number(*value),
                Style::new().fg(self.theme.text_dim),
            );
            self.commands.push(Command::EndContainer);
            self.last_text_idx = None;
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        Response::none()
    }

    /// Render a styled bar chart with per-bar colors, grouping, and direction control.
    ///
    /// # Example
    /// ```ignore
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::{Bar, Color};
    /// let bars = vec![
    ///     Bar::new("Q1", 32.0).color(Color::Cyan),
    ///     Bar::new("Q2", 46.0).color(Color::Green),
    ///     Bar::new("Q3", 28.0).color(Color::Yellow),
    ///     Bar::new("Q4", 54.0).color(Color::Red),
    /// ];
    /// ui.bar_chart_styled(&bars, 30, slt::BarDirection::Horizontal);
    /// # });
    /// ```
    pub fn bar_chart_styled(
        &mut self,
        bars: &[Bar],
        max_width: u32,
        direction: BarDirection,
    ) -> Response {
        self.bar_chart_with(
            bars,
            |config| {
                config.direction(direction);
            },
            max_width,
        )
    }

    pub fn bar_chart_with(
        &mut self,
        bars: &[Bar],
        configure: impl FnOnce(&mut BarChartConfig),
        max_size: u32,
    ) -> Response {
        if bars.is_empty() {
            return Response::none();
        }

        let mut config = BarChartConfig::default();
        configure(&mut config);

        let auto_max = bars
            .iter()
            .map(|bar| bar.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_value = config.max_value.unwrap_or(auto_max);
        let denom = if max_value > 0.0 { max_value } else { 1.0 };

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

        Response::none()
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

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: bar_gap as u32,
            align: Align::Start,
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
        self.last_text_idx = None;
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

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 1,
            align: Align::Start,
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
        self.styled(
            format!("{}{label_padding}", bar.label),
            Style::new().fg(self.theme.text),
        );
        self.styled(bar_text, Style::new().fg(color));
        self.styled(
            Self::bar_display_value(bar),
            bar.value_style
                .unwrap_or(Style::new().fg(self.theme.text_dim)),
        );
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
    }

    fn render_vertical_styled_bars(
        &mut self,
        bars: &[Bar],
        max_height: u32,
        denom: f64,
        bar_width: u16,
        bar_gap: u16,
    ) {
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

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
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
            &bar_units,
            chart_height,
            col_width,
            bar_width,
            bar_gap,
            &value_labels,
        );
        self.render_vertical_bar_labels(bars, col_width, bar_gap);

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
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
            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
                direction: Direction::Row,
                gap: bar_gap as u32,
                align: Align::Start,
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
            self.last_text_idx = None;
        }
    }

    fn render_vertical_bar_labels(&mut self, bars: &[Bar], col_width: usize, bar_gap: u16) {
        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: bar_gap as u32,
            align: Align::Start,
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
        self.last_text_idx = None;
    }

    /// Render a grouped bar chart.
    ///
    /// Each group contains multiple bars rendered side by side. Useful for
    /// comparing categories across groups (e.g., quarterly revenue by product).
    ///
    /// # Example
    /// ```ignore
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

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: config.group_gap as u32,
            align: Align::Start,
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
            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
                direction: Direction::Column,
                gap: config.bar_gap as u32,
                align: Align::Start,
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

                self.interaction_count += 1;
                self.commands.push(Command::BeginContainer {
                    direction: Direction::Row,
                    gap: 1,
                    align: Align::Start,
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
                self.styled(
                    format!("  {}{label_padding}", bar.label),
                    Style::new().fg(self.theme.text),
                );
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
                self.last_text_idx = None;
            }

            self.commands.push(Command::EndContainer);
            self.last_text_idx = None;
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
    }

    fn render_grouped_vertical_bars(
        &mut self,
        groups: &[BarGroup],
        max_height: u32,
        denom: f64,
        config: &BarChartConfig,
    ) {
        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: config.group_gap as u32,
            align: Align::Start,
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
        self.last_text_idx = None;
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
    /// ```ignore
    /// # slt::run(|ui: &mut slt::Context| {
    /// let samples = [12.0, 9.0, 14.0, 18.0, 16.0, 21.0, 20.0, 24.0];
    /// ui.sparkline(&samples, 16);
    ///
    /// For per-point colors and missing values, see [`sparkline_styled`].
    /// # });
    /// ```
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
    /// ```ignore
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

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 0,
            align: Align::Start,
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
        self.last_text_idx = None;

        Response::none()
    }

    /// Render a multi-row line chart using braille characters.
    ///
    /// `width` and `height` are terminal cell dimensions. Internally this uses
    /// braille dot resolution (`width*2` x `height*4`) for smoother plotting.
    ///
    /// # Example
    ///
    /// ```ignore
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
        width: u32,
        height: u32,
        up_color: Color,
        down_color: Color,
    ) -> Response {
        if candles.is_empty() || width == 0 || height == 0 {
            return Response::none();
        }

        let cols = width as usize;
        let rows = height as usize;

        let mut min_price = f64::INFINITY;
        let mut max_price = f64::NEG_INFINITY;
        for candle in candles {
            if candle.low.is_finite() {
                min_price = min_price.min(candle.low);
            }
            if candle.high.is_finite() {
                max_price = max_price.max(candle.high);
            }
        }

        if !min_price.is_finite() || !max_price.is_finite() {
            return Response::none();
        }

        let range = if (max_price - min_price).abs() < f64::EPSILON {
            1.0
        } else {
            max_price - min_price
        };
        let map_row = |value: f64| -> usize {
            let t = ((value - min_price) / range).clamp(0.0, 1.0);
            ((1.0 - t) * (rows.saturating_sub(1)) as f64).round() as usize
        };

        let mut chars = vec![vec![' '; cols]; rows];
        let mut colors = vec![vec![None::<Color>; cols]; rows];

        for (index, candle) in candles.iter().enumerate() {
            if !candle.open.is_finite()
                || !candle.high.is_finite()
                || !candle.low.is_finite()
                || !candle.close.is_finite()
            {
                continue;
            }

            let x_start = index * cols / candles.len();
            let mut x_end = ((index + 1) * cols / candles.len()).saturating_sub(1);
            if x_end < x_start {
                x_end = x_start;
            }
            if x_start >= cols {
                continue;
            }
            x_end = x_end.min(cols.saturating_sub(1));
            let wick_x = (x_start + x_end) / 2;

            let high_row = map_row(candle.high);
            let low_row = map_row(candle.low);
            let open_row = map_row(candle.open);
            let close_row = map_row(candle.close);

            let (wick_top, wick_bottom) = if high_row <= low_row {
                (high_row, low_row)
            } else {
                (low_row, high_row)
            };
            let color = if candle.close >= candle.open {
                up_color
            } else {
                down_color
            };

            for row in wick_top..=wick_bottom.min(rows.saturating_sub(1)) {
                chars[row][wick_x] = '│';
                colors[row][wick_x] = Some(color);
            }

            let (body_top, body_bottom) = if open_row <= close_row {
                (open_row, close_row)
            } else {
                (close_row, open_row)
            };
            for row in body_top..=body_bottom.min(rows.saturating_sub(1)) {
                for col in x_start..=x_end {
                    chars[row][col] = '█';
                    colors[row][col] = Some(color);
                }
            }
        }

        for row in 0..rows {
            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
                direction: Direction::Row,
                gap: 0,
                align: Align::Start,
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

            let mut seg = String::new();
            let mut seg_color = colors[row][0];
            for col in 0..cols {
                if colors[row][col] != seg_color {
                    let style = if let Some(c) = seg_color {
                        Style::new().fg(c)
                    } else {
                        Style::new()
                    };
                    self.styled(seg, style);
                    seg = String::new();
                    seg_color = colors[row][col];
                }
                seg.push(chars[row][col]);
            }
            if !seg.is_empty() {
                let style = if let Some(c) = seg_color {
                    Style::new().fg(c)
                } else {
                    Style::new()
                };
                self.styled(seg, style);
            }

            self.commands.push(Command::EndContainer);
            self.last_text_idx = None;
        }

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

            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
                direction: Direction::Row,
                gap: 0,
                align: Align::Start,
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
            self.last_text_idx = None;
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
    /// ```ignore
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
            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
                direction: Direction::Row,
                gap: 0,
                align: Align::Start,
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
            self.last_text_idx = None;
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
            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
                direction: Direction::Row,
                gap: 0,
                align: Align::Start,
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
            self.last_text_idx = None;
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
            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
                direction: Direction::Row,
                gap: 0,
                align: Align::Start,
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
            self.last_text_idx = None;
        }

        Response::none()
    }
}
