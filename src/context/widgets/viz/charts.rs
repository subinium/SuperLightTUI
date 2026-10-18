use super::*;

impl Context {
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
                        finite_lerp(data[idx], data[idx + 1], frac)
                    } else {
                        data[idx.min(data.len() - 1)]
                    }
                })
                .collect()
        };

        let mut finite = points.iter().copied().filter(|value| value.is_finite());
        let Some(first) = finite.next() else {
            let response = self.interaction();
            self.styled(" ".repeat(w), Style::new().fg(self.theme.text_dim))
                .w(width);
            return response;
        };
        let (mut min, mut max) = (first, first);
        for value in finite {
            min = min.min(value);
            max = max.max(value);
        }

        let line: String = points
            .iter()
            .map(|&value| {
                if !value.is_finite() {
                    return ' ';
                }
                let normalized = if min == max {
                    0.5
                } else {
                    unit_ratio(value, min, max)
                };
                let idx = (normalized * 7.0).round() as usize;
                BLOCKS[idx.min(7)]
            })
            .collect();

        let response = self.interaction();
        self.styled(line, Style::new().fg(self.theme.primary))
            .w(width);
        response
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
                    let value = if !v1.is_finite() || !v2.is_finite() {
                        if frac < 0.5 { v1 } else { v2 }
                    } else {
                        finite_lerp(v1, v2, frac)
                    };
                    (value, color)
                })
                .collect()
        };

        let mut finite_values = window
            .iter()
            .map(|(value, _)| *value)
            .filter(|value| value.is_finite());
        let Some(first) = finite_values.next() else {
            let response = self.interaction();
            self.styled(
                " ".repeat(window.len()),
                Style::new().fg(self.theme.text_dim),
            );
            return response;
        };

        let mut min = first;
        let mut max = first;
        for value in finite_values {
            min = f64::min(min, value);
            max = f64::max(max, value);
        }
        let mut cells: Vec<(char, Color)> = Vec::with_capacity(window.len());
        for (value, color) in &window {
            if !value.is_finite() {
                cells.push((' ', self.theme.text_dim));
                continue;
            }

            let normalized = if min == max {
                0.5
            } else {
                unit_ratio(*value, min, max)
            };
            let idx = (normalized * 7.0).round() as usize;
            cells.push((BLOCKS[idx.min(7)], color.unwrap_or(self.theme.primary)));
        }

        let response = self.begin_sized_viz_root(width, 1);
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
                border_style: Style::new().fg(self.theme.border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));

        if cells.is_empty() {
            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
            self.end_viz_root();
            return response;
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
        self.end_viz_root();

        response
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
        let data: Vec<f64> = data
            .iter()
            .copied()
            .filter(|value| value.is_finite())
            .collect();
        if data.is_empty() {
            return Response::none();
        }

        let cols = width as usize;
        let rows = height as usize;
        let px_w = cols * 2;
        let px_h = rows * 4;

        let min = data.iter().copied().fold(f64::INFINITY, f64::min);
        let max = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);

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
                    finite_lerp(data[idx], data[idx + 1], frac)
                } else {
                    data[idx.min(data.len() - 1)]
                };

                let normalized = if min == max {
                    0.5
                } else {
                    unit_ratio(value, min, max)
                };
                let py = ((1.0 - normalized) * (px_h - 1) as f64).round() as usize;
                py.min(px_h - 1)
            })
            .collect();

        // Braille dot bit masks shared with `chart::braille` (fix #114).
        use crate::chart::{BRAILLE_LEFT_BITS as LEFT_BITS, BRAILLE_RIGHT_BITS as RIGHT_BITS};

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
        let response = self.begin_sized_viz_root(width, height);
        for row in grid {
            let line: String = row
                .iter()
                .map(|&bits| char::from_u32(0x2800 + bits).unwrap_or(' '))
                .collect();
            self.styled(line, style);
        }
        self.end_viz_root();

        response
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

        let candles: Vec<Candle> = candles
            .iter()
            .copied()
            .filter(|candle| {
                candle.open.is_finite()
                    && candle.high.is_finite()
                    && candle.low.is_finite()
                    && candle.close.is_finite()
            })
            .collect();
        if candles.is_empty() {
            return Response::none();
        }
        let response = self.interaction();
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

            let map_y = |v: f64| -> usize {
                let t = if lo == hi { 0.5 } else { unit_ratio(v, lo, hi) };
                ((1.0 - t) * (h.saturating_sub(1)) as f64).round() as usize
            };

            for (i, c) in candles.iter().enumerate() {
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

        response
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

        let response = self.begin_sized_viz_root(width, height);
        for row in rows {
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
                    border_style: Style::new().fg(self.theme.border),
                    bg_color: None,
                    padding: Padding::default(),
                    margin: Margin::default(),
                    constraints: Constraints::default(),
                    title: None,
                    grow: 0,
                    group_name: None,
                })));
            for (text, style) in row.segments {
                self.styled(text, style);
            }
            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }
        self.end_viz_root();

        response
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

        let response = self.begin_sized_viz_root(width, height);
        for row in rows {
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
                    border_style: Style::new().fg(self.theme.border),
                    bg_color: None,
                    padding: Padding::default(),
                    margin: Margin::default(),
                    constraints: Constraints::default(),
                    title: None,
                    grow: 0,
                    group_name: None,
                })));
            for (text, style) in row.segments {
                self.styled(text, style);
            }
            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }
        self.end_viz_root();

        response
    }

    /// Render a candlestick chart with heavy box-drawing and half-block precision.
    ///
    /// Uses `┃` for wicks (heavier than `│`) and `▀`/`▄` at body edges for
    /// sub-cell vertical precision, effectively doubling the price resolution.
    ///
    /// Each terminal cell represents two half-cells vertically: the body's open
    /// and close prices snap to the nearest half-cell, so a body that covers an
    /// odd number of half-cells terminates with `▀` (top half only) or `▄`
    /// (bottom half only) rather than rounding to a full cell.
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

        let candles: Vec<Candle> = candles
            .iter()
            .copied()
            .filter(|candle| {
                candle.open.is_finite()
                    && candle.high.is_finite()
                    && candle.low.is_finite()
                    && candle.close.is_finite()
            })
            .collect();
        if candles.is_empty() {
            return Response::none();
        }
        let response = self.interaction();
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

            // Cell-resolution price-to-row map (used for wicks).
            let map_y = |v: f64| -> usize {
                let t = if lo == hi { 0.5 } else { unit_ratio(v, lo, hi) };
                ((1.0 - t) * h.saturating_sub(1) as f64).round() as usize
            };
            // Half-cell resolution price-to-row map (used for body edges).
            // Returns half-cell index in [0, 2h-1]; 0 = top of cell 0, 2h-1 = bottom of cell h-1.
            let half_rows = h.saturating_mul(2);
            let map_y_half = |v: f64| -> usize {
                let t = if lo == hi { 0.5 } else { unit_ratio(v, lo, hi) };
                ((1.0 - t) * half_rows.saturating_sub(1) as f64).round() as usize
            };

            let n = candles.len();

            for (i, c) in candles.iter().enumerate() {
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

                // Wick (cell-resolution; body draws over wick within its range).
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

                // Body uses half-cell precision: each cell covers two half-cells
                // (top = 2·row, bottom = 2·row + 1). Body extends from
                // `body_top_half` (highest price) down to `body_bot_half`
                // (lowest price) inclusive, in half-cell units.
                let body_top_half = map_y_half(c.open.max(c.close));
                let body_bot_half = map_y_half(c.open.min(c.close));
                let row_first = body_top_half / 2;
                let row_last = (body_bot_half / 2).min(h - 1);
                for row in row_first..=row_last {
                    let top_hc = row * 2;
                    let bot_hc = row * 2 + 1;
                    let top_in = top_hc >= body_top_half && top_hc <= body_bot_half;
                    let bot_in = bot_hc >= body_top_half && bot_hc <= body_bot_half;
                    let body_char = match (top_in, bot_in) {
                        (true, true) => '█',
                        (true, false) => '▀',
                        (false, true) => '▄',
                        (false, false) => continue,
                    };
                    for col in x0..=x1.min(w - 1) {
                        buf.set_char(
                            rect.x + col as u32,
                            rect.y + row as u32,
                            body_char,
                            Style::new().fg(color),
                        );
                    }
                }
            }
        });

        response
    }
}
