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
}
