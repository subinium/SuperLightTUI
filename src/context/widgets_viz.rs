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
    pub fn bar_chart(&mut self, data: &[(&str, f64)], max_width: u32) -> &mut Self {
        if data.is_empty() {
            return self;
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
            let bar_len = (normalized * max_width as f64).round() as usize;
            let bar = "█".repeat(bar_len);

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

        self
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
    ) -> &mut Self {
        if bars.is_empty() {
            return self;
        }

        let max_value = bars
            .iter()
            .map(|bar| bar.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let denom = if max_value > 0.0 { max_value } else { 1.0 };

        match direction {
            BarDirection::Horizontal => {
                let max_label_width = bars
                    .iter()
                    .map(|bar| UnicodeWidthStr::width(bar.label.as_str()))
                    .max()
                    .unwrap_or(0);

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

                for bar in bars {
                    let label_width = UnicodeWidthStr::width(bar.label.as_str());
                    let label_padding = " ".repeat(max_label_width.saturating_sub(label_width));
                    let normalized = (bar.value / denom).clamp(0.0, 1.0);
                    let bar_len = (normalized * max_width as f64).round() as usize;
                    let bar_text = "█".repeat(bar_len);
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
                        format_compact_number(bar.value),
                        Style::new().fg(self.theme.text_dim),
                    );
                    self.commands.push(Command::EndContainer);
                    self.last_text_idx = None;
                }

                self.commands.push(Command::EndContainer);
                self.last_text_idx = None;
            }
            BarDirection::Vertical => {
                const FRACTION_BLOCKS: [char; 8] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇'];

                let chart_height = max_width.max(1) as usize;
                let value_labels: Vec<String> = bars
                    .iter()
                    .map(|bar| format_compact_number(bar.value))
                    .collect();
                let col_width = bars
                    .iter()
                    .zip(value_labels.iter())
                    .map(|(bar, value)| {
                        UnicodeWidthStr::width(bar.label.as_str())
                            .max(UnicodeWidthStr::width(value.as_str()))
                            .max(1)
                    })
                    .max()
                    .unwrap_or(1);

                let bar_units: Vec<usize> = bars
                    .iter()
                    .map(|bar| {
                        let normalized = (bar.value / denom).clamp(0.0, 1.0);
                        (normalized * chart_height as f64 * 8.0).round() as usize
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
                for value in &value_labels {
                    self.styled(
                        center_text(value, col_width),
                        Style::new().fg(self.theme.text_dim),
                    );
                }
                self.commands.push(Command::EndContainer);
                self.last_text_idx = None;

                for row in (0..chart_height).rev() {
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

                    let row_base = row * 8;
                    for (bar, units) in bars.iter().zip(bar_units.iter()) {
                        let fill = if *units <= row_base {
                            ' '
                        } else {
                            let delta = *units - row_base;
                            if delta >= 8 {
                                '█'
                            } else {
                                FRACTION_BLOCKS[delta]
                            }
                        };

                        self.styled(
                            center_text(&fill.to_string(), col_width),
                            Style::new().fg(bar.color.unwrap_or(self.theme.primary)),
                        );
                    }

                    self.commands.push(Command::EndContainer);
                    self.last_text_idx = None;
                }

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
                for bar in bars {
                    self.styled(
                        center_text(&bar.label, col_width),
                        Style::new().fg(self.theme.text),
                    );
                }
                self.commands.push(Command::EndContainer);
                self.last_text_idx = None;

                self.commands.push(Command::EndContainer);
                self.last_text_idx = None;
            }
        }

        self
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
    pub fn bar_chart_grouped(&mut self, groups: &[BarGroup], max_width: u32) -> &mut Self {
        if groups.is_empty() {
            return self;
        }

        let all_bars: Vec<&Bar> = groups.iter().flat_map(|group| group.bars.iter()).collect();
        if all_bars.is_empty() {
            return self;
        }

        let max_label_width = all_bars
            .iter()
            .map(|bar| UnicodeWidthStr::width(bar.label.as_str()))
            .max()
            .unwrap_or(0);
        let max_value = all_bars
            .iter()
            .map(|bar| bar.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let denom = if max_value > 0.0 { max_value } else { 1.0 };

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
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

        for group in groups {
            self.styled(group.label.clone(), Style::new().bold().fg(self.theme.text));

            for bar in &group.bars {
                let label_width = UnicodeWidthStr::width(bar.label.as_str());
                let label_padding = " ".repeat(max_label_width.saturating_sub(label_width));
                let normalized = (bar.value / denom).clamp(0.0, 1.0);
                let bar_len = (normalized * max_width as f64).round() as usize;
                let bar_text = "█".repeat(bar_len);

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
                    format_compact_number(bar.value),
                    Style::new().fg(self.theme.text_dim),
                );
                self.commands.push(Command::EndContainer);
                self.last_text_idx = None;
            }
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
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
    pub fn sparkline(&mut self, data: &[f64], width: u32) -> &mut Self {
        const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        let w = width as usize;
        let window = if data.len() > w {
            &data[data.len() - w..]
        } else {
            data
        };

        if window.is_empty() {
            return self;
        }

        let min = window.iter().copied().fold(f64::INFINITY, f64::min);
        let max = window.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;

        let line: String = window
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

        self.styled(line, Style::new().fg(self.theme.primary))
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
    pub fn sparkline_styled(&mut self, data: &[(f64, Option<Color>)], width: u32) -> &mut Self {
        const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        let w = width as usize;
        let window = if data.len() > w {
            &data[data.len() - w..]
        } else {
            data
        };

        if window.is_empty() {
            return self;
        }

        let mut finite_values = window
            .iter()
            .map(|(value, _)| *value)
            .filter(|value| !value.is_nan());
        let Some(first) = finite_values.next() else {
            return self.styled(
                " ".repeat(window.len()),
                Style::new().fg(self.theme.text_dim),
            );
        };

        let mut min = first;
        let mut max = first;
        for value in finite_values {
            min = f64::min(min, value);
            max = f64::max(max, value);
        }
        let range = max - min;

        let mut cells: Vec<(char, Color)> = Vec::with_capacity(window.len());
        for (value, color) in window {
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

        self
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
    pub fn line_chart(&mut self, data: &[f64], width: u32, height: u32) -> &mut Self {
        if data.is_empty() || width == 0 || height == 0 {
            return self;
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
        }

        let style = Style::new().fg(self.theme.primary);
        for row in grid {
            let line: String = row
                .iter()
                .map(|&bits| char::from_u32(0x2800 + bits).unwrap_or(' '))
                .collect();
            self.styled(line, style);
        }

        self
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
    ) -> &mut Self {
        if width == 0 || height == 0 {
            return self;
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

        self
    }

    /// Render a multi-series chart with axes, legend, and auto-scaling.
    pub fn chart(
        &mut self,
        configure: impl FnOnce(&mut ChartBuilder),
        width: u32,
        height: u32,
    ) -> &mut Self {
        if width == 0 || height == 0 {
            return self;
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

        self
    }

    /// Renders a scatter plot.
    ///
    /// Each point is a (x, y) tuple. Uses braille markers.
    pub fn scatter(&mut self, data: &[(f64, f64)], width: u32, height: u32) -> &mut Self {
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
    pub fn histogram(&mut self, data: &[f64], width: u32, height: u32) -> &mut Self {
        self.histogram_with(data, |_| {}, width, height)
    }

    /// Render a histogram with configuration options.
    pub fn histogram_with(
        &mut self,
        data: &[f64],
        configure: impl FnOnce(&mut HistogramBuilder),
        width: u32,
        height: u32,
    ) -> &mut Self {
        if width == 0 || height == 0 {
            return self;
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

        self
    }

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
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let border = self.theme.border;

        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
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
    pub fn list(&mut self, state: &mut ListState) -> &mut Self {
        let visible = state.visible_indices().to_vec();
        if visible.is_empty() && state.items.is_empty() {
            state.selected = 0;
            return self;
        }

        if !visible.is_empty() {
            state.selected = state.selected.min(visible.len().saturating_sub(1));
        }

        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.selected = state.selected.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            state.selected =
                                (state.selected + 1).min(visible.len().saturating_sub(1));
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

        for (view_idx, &item_idx) in visible.iter().enumerate() {
            let item = &state.items[item_idx];
            if view_idx == state.selected {
                if focused {
                    self.styled(
                        format!("▸ {item}"),
                        Style::new().bold().fg(self.theme.primary),
                    );
                } else {
                    self.styled(format!("▸ {item}"), Style::new().fg(self.theme.primary));
                }
            } else {
                self.styled(format!("  {item}"), Style::new().fg(self.theme.text));
            }
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a data table with column headers. Handles Up/Down selection when focused.
    ///
    /// Column widths are computed automatically from header and cell content.
    /// The selected row is highlighted with the theme's selection colors.
    pub fn table(&mut self, state: &mut TableState) -> &mut Self {
        if state.is_dirty() {
            state.recompute_widths();
        }

        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;

        if focused && !state.visible_indices().is_empty() {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            let visible_len = if state.page_size > 0 {
                                let start = state
                                    .page
                                    .saturating_mul(state.page_size)
                                    .min(state.visible_indices().len());
                                let end =
                                    (start + state.page_size).min(state.visible_indices().len());
                                end.saturating_sub(start)
                            } else {
                                state.visible_indices().len()
                            };
                            state.selected = state.selected.min(visible_len.saturating_sub(1));
                            state.selected = state.selected.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let visible_len = if state.page_size > 0 {
                                let start = state
                                    .page
                                    .saturating_mul(state.page_size)
                                    .min(state.visible_indices().len());
                                let end =
                                    (start + state.page_size).min(state.visible_indices().len());
                                end.saturating_sub(start)
                            } else {
                                state.visible_indices().len()
                            };
                            state.selected =
                                (state.selected + 1).min(visible_len.saturating_sub(1));
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

        if !state.visible_indices().is_empty() || !state.headers.is_empty() {
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

        if state.is_dirty() {
            state.recompute_widths();
        }

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

        let header_cells = state
            .headers
            .iter()
            .enumerate()
            .map(|(i, header)| {
                if state.sort_column == Some(i) {
                    if state.sort_ascending {
                        format!("{header} ▲")
                    } else {
                        format!("{header} ▼")
                    }
                } else {
                    header.clone()
                }
            })
            .collect::<Vec<_>>();
        let header_line = format_table_row(&header_cells, state.column_widths(), " │ ");
        self.styled(header_line, Style::new().bold().fg(self.theme.text));

        let separator = state
            .column_widths()
            .iter()
            .map(|w| "─".repeat(*w as usize))
            .collect::<Vec<_>>()
            .join("─┼─");
        self.text(separator);

        for idx in 0..visible_len {
            let data_idx = state.visible_indices()[page_start + idx];
            let Some(row) = state.rows.get(data_idx) else {
                continue;
            };
            let line = format_table_row(row, state.column_widths(), " │ ");
            if idx == state.selected {
                let mut style = Style::new()
                    .bg(self.theme.selected_bg)
                    .fg(self.theme.selected_fg);
                if focused {
                    style = style.bold();
                }
                self.styled(line, style);
            } else {
                self.styled(line, Style::new().fg(self.theme.text));
            }
        }

        if state.page_size > 0 && state.total_pages() > 1 {
            self.styled(
                format!("Page {}/{}", state.page + 1, state.total_pages()),
                Style::new().dim().fg(self.theme.text_dim),
            );
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a tab bar. Handles Left/Right navigation when focused.
    ///
    /// The active tab is rendered in the theme's primary color. If the labels
    /// list is empty, nothing is rendered.
    pub fn tabs(&mut self, state: &mut TabsState) -> &mut Self {
        if state.labels.is_empty() {
            state.selected = 0;
            return self;
        }

        state.selected = state.selected.min(state.labels.len().saturating_sub(1));
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;

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
        for (idx, label) in state.labels.iter().enumerate() {
            let style = if idx == state.selected {
                let s = Style::new().fg(self.theme.primary).bold();
                if focused {
                    s.underline()
                } else {
                    s
                }
            } else {
                Style::new().fg(self.theme.text_dim)
            };
            self.styled(format!("[ {label} ]"), style);
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a clickable button. Returns `true` when activated via Enter, Space, or mouse click.
    ///
    /// The button is styled with the theme's primary color when focused and the
    /// accent color when hovered.
    pub fn button(&mut self, label: impl Into<String>) -> bool {
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);

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
        let style = if focused {
            Style::new().fg(self.theme.primary).bold()
        } else if hovered {
            Style::new().fg(self.theme.accent)
        } else {
            Style::new().fg(self.theme.text)
        };
        let hover_bg = if hovered || focused {
            Some(self.theme.surface_hover)
        } else {
            None
        };

        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 0,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(self.theme.border),
            bg_color: hover_bg,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            group_name: None,
        });
        self.styled(format!("[ {} ]", label.into()), style);
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        activated
    }

    /// Render a styled button variant. Returns `true` when activated.
    ///
    /// Use [`ButtonVariant::Primary`] for call-to-action, [`ButtonVariant::Danger`]
    /// for destructive actions, or [`ButtonVariant::Outline`] for secondary actions.
    pub fn button_with(&mut self, label: impl Into<String>, variant: ButtonVariant) -> bool {
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);

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
                (format!("[ {label} ]"), style, hover_bg, None)
            }
            ButtonVariant::Primary => {
                let style = if focused {
                    Style::new().fg(self.theme.bg).bg(self.theme.primary).bold()
                } else if response.hovered {
                    Style::new().fg(self.theme.bg).bg(self.theme.accent)
                } else {
                    Style::new().fg(self.theme.bg).bg(self.theme.primary)
                };
                (format!(" {label} "), style, hover_bg, None)
            }
            ButtonVariant::Danger => {
                let style = if focused {
                    Style::new().fg(self.theme.bg).bg(self.theme.error).bold()
                } else if response.hovered {
                    Style::new().fg(self.theme.bg).bg(self.theme.warning)
                } else {
                    Style::new().fg(self.theme.bg).bg(self.theme.error)
                };
                (format!(" {label} "), style, hover_bg, None)
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
                    format!(" {label} "),
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

        activated
    }

    /// Render a checkbox. Toggles the bool on Enter, Space, or click.
    ///
    /// The checked state is shown with the theme's success color. When focused,
    /// a `▸` prefix is added.
    pub fn checkbox(&mut self, label: impl Into<String>, checked: &mut bool) -> &mut Self {
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);
        let mut should_toggle = response.clicked;

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
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(self.theme.border),
            bg_color: hover_bg,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            group_name: None,
        });
        let marker_style = if *checked {
            Style::new().fg(self.theme.success)
        } else {
            Style::new().fg(self.theme.text_dim)
        };
        let marker = if *checked { "[x]" } else { "[ ]" };
        let label_text = label.into();
        if focused {
            self.styled(format!("▸ {marker}"), marker_style.bold());
            self.styled(label_text, Style::new().fg(self.theme.text).bold());
        } else {
            self.styled(marker, marker_style);
            self.styled(label_text, Style::new().fg(self.theme.text));
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render an on/off toggle switch.
    ///
    /// Toggles `on` when activated via Enter, Space, or click. The switch
    /// renders as `●━━ ON` or `━━● OFF` colored with the theme's success or
    /// dim color respectively.
    pub fn toggle(&mut self, label: impl Into<String>, on: &mut bool) -> &mut Self {
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);
        let mut should_toggle = response.clicked;

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
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(self.theme.border),
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
            Style::new().fg(self.theme.success)
        } else {
            Style::new().fg(self.theme.text_dim)
        };
        if focused {
            self.styled(
                format!("▸ {label_text}"),
                Style::new().fg(self.theme.text).bold(),
            );
            self.styled(switch, switch_style.bold());
        } else {
            self.styled(label_text, Style::new().fg(self.theme.text));
            self.styled(switch, switch_style);
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    // ── select / dropdown ─────────────────────────────────────────────

    /// Render a dropdown select. Shows the selected item; expands on activation.
    ///
    /// Returns `true` when the selection changed this frame.
    pub fn select(&mut self, state: &mut SelectState) -> bool {
        if state.items.is_empty() {
            return false;
        }
        state.selected = state.selected.min(state.items.len().saturating_sub(1));

        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);
        let old_selected = state.selected;

        if response.clicked {
            state.open = !state.open;
            if state.open {
                state.set_cursor(state.selected);
            }
        }

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
                    if state.open {
                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                let c = state.cursor();
                                state.set_cursor(c.saturating_sub(1));
                                consumed_indices.push(i);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                let c = state.cursor();
                                state.set_cursor((c + 1).min(state.items.len().saturating_sub(1)));
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

        let changed = state.selected != old_selected;

        let border_color = if focused {
            self.theme.primary
        } else {
            self.theme.border
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
        self.styled(&display_text, Style::new().fg(self.theme.text));
        self.styled(arrow, Style::new().fg(self.theme.text_dim));
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        if state.open {
            for (idx, item) in state.items.iter().enumerate() {
                let is_cursor = idx == state.cursor();
                let style = if is_cursor {
                    Style::new().bold().fg(self.theme.primary)
                } else {
                    Style::new().fg(self.theme.text)
                };
                let prefix = if is_cursor { "▸ " } else { "  " };
                self.styled(format!("{prefix}{item}"), style);
            }
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        changed
    }

    // ── radio ────────────────────────────────────────────────────────

    /// Render a radio button group. Returns `true` when selection changed.
    pub fn radio(&mut self, state: &mut RadioState) -> bool {
        if state.items.is_empty() {
            return false;
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
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.selected = state.selected.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            state.selected =
                                (state.selected + 1).min(state.items.len().saturating_sub(1));
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

        let interaction_id = self.interaction_count;
        self.interaction_count += 1;

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
            let is_selected = idx == state.selected;
            let marker = if is_selected { "●" } else { "○" };
            let style = if is_selected {
                if focused {
                    Style::new().bold().fg(self.theme.primary)
                } else {
                    Style::new().fg(self.theme.primary)
                }
            } else {
                Style::new().fg(self.theme.text)
            };
            let prefix = if focused && idx == state.selected {
                "▸ "
            } else {
                "  "
            };
            self.styled(format!("{prefix}{marker} {item}"), style);
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        state.selected != old_selected
    }

    // ── multi-select ─────────────────────────────────────────────────

    /// Render a multi-select list. Space toggles, Up/Down navigates.
    pub fn multi_select(&mut self, state: &mut MultiSelectState) -> &mut Self {
        if state.items.is_empty() {
            return self;
        }
        state.cursor = state.cursor.min(state.items.len().saturating_sub(1));
        let focused = self.register_focusable();

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
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.cursor = state.cursor.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            state.cursor =
                                (state.cursor + 1).min(state.items.len().saturating_sub(1));
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

        let interaction_id = self.interaction_count;
        self.interaction_count += 1;

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
            self.styled(format!("{prefix}{marker} {item}"), style);
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        self
    }

    // ── tree ─────────────────────────────────────────────────────────

    /// Render a tree view. Left/Right to collapse/expand, Up/Down to navigate.
    pub fn tree(&mut self, state: &mut TreeState) -> &mut Self {
        let entries = state.flatten();
        if entries.is_empty() {
            return self;
        }
        state.selected = state.selected.min(entries.len().saturating_sub(1));
        let focused = self.register_focusable();

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
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.selected = state.selected.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let max = state.flatten().len().saturating_sub(1);
                            state.selected = (state.selected + 1).min(max);
                            consumed_indices.push(i);
                        }
                        KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => {
                            state.toggle_at(state.selected);
                            consumed_indices.push(i);
                        }
                        KeyCode::Left => {
                            let entry = &entries[state.selected.min(entries.len() - 1)];
                            if entry.expanded {
                                state.toggle_at(state.selected);
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
            self.styled(format!("{cursor}{indent}{icon}{}", entry.label), style);
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        self
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
    ) -> &mut Self {
        if state.items.is_empty() {
            return self;
        }
        state.selected = state.selected.min(state.items.len().saturating_sub(1));
        let focused = self.register_focusable();

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
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.selected = state.selected.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            state.selected =
                                (state.selected + 1).min(state.items.len().saturating_sub(1));
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

        if start > 0 {
            self.styled(
                format!("  ↑ {} more", start),
                Style::new().fg(self.theme.text_dim).dim(),
            );
        }

        for idx in start..end {
            f(self, idx);
        }

        let remaining = state.items.len().saturating_sub(end);
        if remaining > 0 {
            self.styled(
                format!("  ↓ {} more", remaining),
                Style::new().fg(self.theme.text_dim).dim(),
            );
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;
        self
    }

    // ── command palette ──────────────────────────────────────────────

    /// Render a command palette overlay. Returns `Some(index)` when a command is selected.
    pub fn command_palette(&mut self, state: &mut CommandPaletteState) -> Option<usize> {
        if !state.open {
            return None;
        }

        let filtered = state.filtered_indices();
        let sel = state.selected().min(filtered.len().saturating_sub(1));
        state.set_selected(sel);

        let mut consumed_indices = Vec::new();
        let mut result: Option<usize> = None;

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
                            result = Some(cmd_idx);
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

        self.modal(|ui| {
            let primary = ui.theme.primary;
            ui.container()
                .border(Border::Rounded)
                .border_style(Style::new().fg(primary))
                .pad(1)
                .max_w(60)
                .col(|ui| {
                    let border_color = ui.theme.primary;
                    ui.bordered(Border::Rounded)
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
                            .map(|s| format!("  ({s})"))
                            .unwrap_or_default();
                        ui.styled(format!("{prefix}{}{shortcut_text}", cmd.label), style);
                        if is_selected && !cmd.description.is_empty() {
                            ui.styled(
                                format!("    {}", cmd.description),
                                Style::new().dim().fg(ui.theme.text_dim),
                            );
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

        result
    }

    // ── markdown ─────────────────────────────────────────────────────

    /// Render a markdown string with basic formatting.
    ///
    /// Supports headers (`#`), bold (`**`), italic (`*`), inline code (`` ` ``),
    /// unordered lists (`-`/`*`), ordered lists (`1.`), and horizontal rules (`---`).
    pub fn markdown(&mut self, text: &str) -> &mut Self {
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
                    self.styled(format!("  • {item}"), text_style);
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
                        self.styled(format!("  {}. {}", parts[0], parts[1]), text_style);
                    } else {
                        self.line(|ui| {
                            ui.styled(format!("  {}. ", parts[0]), text_style);
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
        self
    }

    fn parse_inline_segments(
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
    pub fn help(&mut self, bindings: &[(&str, &str)]) -> &mut Self {
        if bindings.is_empty() {
            return self;
        }

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 2,
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
        for (idx, (key, action)) in bindings.iter().enumerate() {
            if idx > 0 {
                self.styled("·", Style::new().fg(self.theme.text_dim));
            }
            self.styled(*key, Style::new().bold().fg(self.theme.primary));
            self.styled(*action, Style::new().fg(self.theme.text_dim));
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
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
