//! Data visualization: line charts, scatter plots, bar charts, and histograms.
//!
//! Build a chart with [`ChartBuilder`], then pass it to
//! [`Context::chart`](crate::Context::chart). Histograms use
//! [`Context::histogram`](crate::Context::histogram) directly.

use crate::style::{Color, Style};
use unicode_width::UnicodeWidthStr;

const BRAILLE_BASE: u32 = 0x2800;
const BRAILLE_LEFT_BITS: [u32; 4] = [0x01, 0x02, 0x04, 0x40];
const BRAILLE_RIGHT_BITS: [u32; 4] = [0x08, 0x10, 0x20, 0x80];
const PALETTE: [Color; 8] = [
    Color::Cyan,
    Color::Yellow,
    Color::Green,
    Color::Magenta,
    Color::Red,
    Color::Blue,
    Color::White,
    Color::Indexed(208),
];
const BLOCK_FRACTIONS: [char; 9] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// Colored character range `(start, end, color)`.
pub type ColorSpan = (usize, usize, Color);

/// Rendered chart line with color ranges.
pub type RenderedLine = (String, Vec<ColorSpan>);

/// Marker type for data points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    /// Braille marker (2x4 sub-cell dots).
    Braille,
    /// Dot marker.
    Dot,
    /// Full block marker.
    Block,
    /// Half block marker.
    HalfBlock,
    /// Cross marker.
    Cross,
    /// Circle marker.
    Circle,
}

/// Graph rendering style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphType {
    /// Connected points.
    Line,
    /// Connected points with filled area to baseline.
    Area,
    /// Unconnected points.
    Scatter,
    /// Vertical bars from the x-axis baseline.
    Bar,
}

/// Legend placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendPosition {
    /// Top-left corner.
    TopLeft,
    /// Top-right corner.
    TopRight,
    /// Bottom-left corner.
    BottomLeft,
    /// Bottom-right corner.
    BottomRight,
    /// Disable legend.
    None,
}

/// Axis configuration.
#[derive(Debug, Clone)]
pub struct Axis {
    /// Optional axis title.
    pub title: Option<String>,
    /// Manual axis bounds `(min, max)`. Uses auto-scaling when `None`.
    pub bounds: Option<(f64, f64)>,
    /// Optional manual tick labels.
    pub labels: Option<Vec<String>>,
    /// Optional manual tick positions.
    pub ticks: Option<Vec<f64>>,
    /// Optional axis title style override.
    pub title_style: Option<Style>,
    /// Axis text and tick style.
    pub style: Style,
}

impl Default for Axis {
    fn default() -> Self {
        Self {
            title: None,
            bounds: None,
            labels: None,
            ticks: None,
            title_style: None,
            style: Style::new(),
        }
    }
}

/// Dataset for one chart series.
#[derive(Debug, Clone)]
pub struct Dataset {
    /// Dataset label shown in legend.
    pub name: String,
    /// Data points as `(x, y)` pairs.
    pub data: Vec<(f64, f64)>,
    /// Series color.
    pub color: Color,
    /// Marker used for points.
    pub marker: Marker,
    /// Rendering mode for this dataset.
    pub graph_type: GraphType,
    /// Upward segment color override.
    pub up_color: Option<Color>,
    /// Downward (or flat) segment color override.
    pub down_color: Option<Color>,
}

/// OHLC candle datum.
#[derive(Debug, Clone, Copy)]
pub struct Candle {
    /// Open price.
    pub open: f64,
    /// High price.
    pub high: f64,
    /// Low price.
    pub low: f64,
    /// Close price.
    pub close: f64,
}

/// Chart configuration.
#[derive(Debug, Clone)]
pub struct ChartConfig {
    /// Optional chart title.
    pub title: Option<String>,
    /// Optional chart title style override.
    pub title_style: Option<Style>,
    /// X axis configuration.
    pub x_axis: Axis,
    /// Y axis configuration.
    pub y_axis: Axis,
    /// Chart datasets.
    pub datasets: Vec<Dataset>,
    /// Legend position.
    pub legend: LegendPosition,
    /// Whether to render grid lines.
    pub grid: bool,
    /// Optional grid line style override.
    pub grid_style: Option<Style>,
    /// Horizontal reference lines as `(y, style)`.
    pub hlines: Vec<(f64, Style)>,
    /// Vertical reference lines as `(x, style)`.
    pub vlines: Vec<(f64, Style)>,
    /// Whether to render the outer frame.
    pub frame_visible: bool,
    /// Whether to render x-axis line/labels/title rows.
    pub x_axis_visible: bool,
    /// Whether to render y-axis labels/divider column.
    pub y_axis_visible: bool,
    /// Total chart width in terminal cells.
    pub width: u32,
    /// Total chart height in terminal cells.
    pub height: u32,
}

/// One row of styled chart output.
#[derive(Debug, Clone)]
pub(crate) struct ChartRow {
    /// Styled text segments for this row.
    pub segments: Vec<(String, Style)>,
}

/// Histogram configuration builder.
#[derive(Debug, Clone)]
#[must_use = "configure histogram before rendering"]
pub struct HistogramBuilder {
    /// Optional explicit bin count.
    pub bins: Option<usize>,
    /// Histogram bar color.
    pub color: Color,
    /// Optional x-axis title.
    pub x_title: Option<String>,
    /// Optional y-axis title.
    pub y_title: Option<String>,
}

impl Default for HistogramBuilder {
    fn default() -> Self {
        Self {
            bins: None,
            color: Color::Cyan,
            x_title: None,
            y_title: Some("Count".to_string()),
        }
    }
}

impl HistogramBuilder {
    /// Set explicit histogram bin count.
    pub fn bins(&mut self, bins: usize) -> &mut Self {
        self.bins = Some(bins.max(1));
        self
    }

    /// Set histogram bar color.
    pub fn color(&mut self, color: Color) -> &mut Self {
        self.color = color;
        self
    }

    /// Set x-axis title.
    pub fn xlabel(&mut self, title: &str) -> &mut Self {
        self.x_title = Some(title.to_string());
        self
    }

    /// Set y-axis title.
    pub fn ylabel(&mut self, title: &str) -> &mut Self {
        self.y_title = Some(title.to_string());
        self
    }
}

/// Builder entry for one dataset in [`ChartBuilder`].
#[derive(Debug, Clone)]
pub struct DatasetEntry {
    dataset: Dataset,
    color_overridden: bool,
}

impl DatasetEntry {
    /// Set dataset label for legend.
    pub fn label(&mut self, name: &str) -> &mut Self {
        self.dataset.name = name.to_string();
        self
    }

    /// Set dataset color.
    pub fn color(&mut self, color: Color) -> &mut Self {
        self.dataset.color = color;
        self.color_overridden = true;
        self
    }

    /// Set marker style.
    pub fn marker(&mut self, marker: Marker) -> &mut Self {
        self.dataset.marker = marker;
        self
    }

    /// Color line/area segments by direction.
    pub fn color_by_direction(&mut self, up: Color, down: Color) -> &mut Self {
        self.dataset.up_color = Some(up);
        self.dataset.down_color = Some(down);
        self
    }
}

/// Immediate-mode builder for charts.
#[derive(Debug, Clone)]
#[must_use = "configure chart before rendering"]
pub struct ChartBuilder {
    config: ChartConfig,
    entries: Vec<DatasetEntry>,
}

impl ChartBuilder {
    /// Create a chart builder with widget dimensions.
    pub fn new(width: u32, height: u32, x_style: Style, y_style: Style) -> Self {
        Self {
            config: ChartConfig {
                title: None,
                title_style: None,
                x_axis: Axis {
                    style: x_style,
                    ..Axis::default()
                },
                y_axis: Axis {
                    style: y_style,
                    ..Axis::default()
                },
                datasets: Vec::new(),
                legend: LegendPosition::TopRight,
                grid: true,
                grid_style: None,
                hlines: Vec::new(),
                vlines: Vec::new(),
                frame_visible: true,
                x_axis_visible: true,
                y_axis_visible: true,
                width,
                height,
            },
            entries: Vec::new(),
        }
    }

    /// Set chart title.
    pub fn title(&mut self, title: &str) -> &mut Self {
        self.config.title = Some(title.to_string());
        self
    }

    /// Set x-axis title.
    pub fn xlabel(&mut self, label: &str) -> &mut Self {
        self.config.x_axis.title = Some(label.to_string());
        self
    }

    /// Set y-axis title.
    pub fn ylabel(&mut self, label: &str) -> &mut Self {
        self.config.y_axis.title = Some(label.to_string());
        self
    }

    /// Set manual x-axis bounds.
    pub fn xlim(&mut self, min: f64, max: f64) -> &mut Self {
        self.config.x_axis.bounds = Some((min, max));
        self
    }

    /// Set manual y-axis bounds.
    pub fn ylim(&mut self, min: f64, max: f64) -> &mut Self {
        self.config.y_axis.bounds = Some((min, max));
        self
    }

    /// Set manual x-axis tick positions.
    pub fn xticks(&mut self, values: &[f64]) -> &mut Self {
        self.config.x_axis.ticks = Some(values.to_vec());
        self
    }

    /// Set manual y-axis tick positions.
    pub fn yticks(&mut self, values: &[f64]) -> &mut Self {
        self.config.y_axis.ticks = Some(values.to_vec());
        self
    }

    /// Set manual x-axis ticks and labels.
    pub fn xtick_labels(&mut self, values: &[f64], labels: &[&str]) -> &mut Self {
        self.config.x_axis.ticks = Some(values.to_vec());
        self.config.x_axis.labels = Some(labels.iter().map(|label| (*label).to_string()).collect());
        self
    }

    /// Set manual y-axis ticks and labels.
    pub fn ytick_labels(&mut self, values: &[f64], labels: &[&str]) -> &mut Self {
        self.config.y_axis.ticks = Some(values.to_vec());
        self.config.y_axis.labels = Some(labels.iter().map(|label| (*label).to_string()).collect());
        self
    }

    /// Set chart title style.
    pub fn title_style(&mut self, style: Style) -> &mut Self {
        self.config.title_style = Some(style);
        self
    }

    /// Set grid line style.
    pub fn grid_style(&mut self, style: Style) -> &mut Self {
        self.config.grid_style = Some(style);
        self
    }

    /// Set x-axis style.
    pub fn x_axis_style(&mut self, style: Style) -> &mut Self {
        self.config.x_axis.style = style;
        self
    }

    /// Set y-axis style.
    pub fn y_axis_style(&mut self, style: Style) -> &mut Self {
        self.config.y_axis.style = style;
        self
    }

    /// Add a horizontal reference line.
    pub fn axhline(&mut self, y: f64, style: Style) -> &mut Self {
        self.config.hlines.push((y, style));
        self
    }

    /// Add a vertical reference line.
    pub fn axvline(&mut self, x: f64, style: Style) -> &mut Self {
        self.config.vlines.push((x, style));
        self
    }

    /// Enable or disable grid lines.
    pub fn grid(&mut self, on: bool) -> &mut Self {
        self.config.grid = on;
        self
    }

    /// Enable or disable chart frame.
    pub fn frame(&mut self, on: bool) -> &mut Self {
        self.config.frame_visible = on;
        self
    }

    /// Enable or disable x-axis line/labels/title rows.
    pub fn x_axis_visible(&mut self, on: bool) -> &mut Self {
        self.config.x_axis_visible = on;
        self
    }

    /// Enable or disable y-axis labels and divider.
    pub fn y_axis_visible(&mut self, on: bool) -> &mut Self {
        self.config.y_axis_visible = on;
        self
    }

    /// Set legend position.
    pub fn legend(&mut self, position: LegendPosition) -> &mut Self {
        self.config.legend = position;
        self
    }

    /// Add a line dataset.
    pub fn line(&mut self, data: &[(f64, f64)]) -> &mut DatasetEntry {
        self.push_dataset(data, GraphType::Line, Marker::Braille)
    }

    /// Add an area dataset.
    pub fn area(&mut self, data: &[(f64, f64)]) -> &mut DatasetEntry {
        self.push_dataset(data, GraphType::Area, Marker::Braille)
    }

    /// Add a scatter dataset.
    pub fn scatter(&mut self, data: &[(f64, f64)]) -> &mut DatasetEntry {
        self.push_dataset(data, GraphType::Scatter, Marker::Braille)
    }

    /// Add a bar dataset.
    pub fn bar(&mut self, data: &[(f64, f64)]) -> &mut DatasetEntry {
        self.push_dataset(data, GraphType::Bar, Marker::Block)
    }

    /// Build the final chart config.
    pub fn build(mut self) -> ChartConfig {
        for (index, mut entry) in self.entries.drain(..).enumerate() {
            if !entry.color_overridden {
                entry.dataset.color = PALETTE[index % PALETTE.len()];
            }
            self.config.datasets.push(entry.dataset);
        }
        self.config
    }

    fn push_dataset(
        &mut self,
        data: &[(f64, f64)],
        graph_type: GraphType,
        marker: Marker,
    ) -> &mut DatasetEntry {
        let series_name = format!("Series {}", self.entries.len() + 1);
        self.entries.push(DatasetEntry {
            dataset: Dataset {
                name: series_name,
                data: data.to_vec(),
                color: Color::Reset,
                marker,
                graph_type,
                up_color: None,
                down_color: None,
            },
            color_overridden: false,
        });
        let last_index = self.entries.len().saturating_sub(1);
        &mut self.entries[last_index]
    }
}

/// Renderer that emits text rows with per-character color ranges.
#[derive(Debug, Clone)]
pub struct ChartRenderer {
    config: ChartConfig,
}

impl ChartRenderer {
    /// Create a renderer from a chart config.
    pub fn new(config: ChartConfig) -> Self {
        Self { config }
    }

    /// Render chart as lines plus color spans `(start, end, color)`.
    pub fn render(&self) -> Vec<RenderedLine> {
        let rows = render_chart(&self.config);
        rows.into_iter()
            .map(|row| {
                let mut line = String::new();
                let mut spans: Vec<(usize, usize, Color)> = Vec::new();
                let mut cursor = 0usize;

                for (segment, style) in row.segments {
                    let width = UnicodeWidthStr::width(segment.as_str());
                    line.push_str(&segment);
                    if let Some(color) = style.fg {
                        spans.push((cursor, cursor + width, color));
                    }
                    cursor += width;
                }

                (line, spans)
            })
            .collect()
    }
}

/// Build a histogram chart configuration from raw values.
pub(crate) fn build_histogram_config(
    data: &[f64],
    options: &HistogramBuilder,
    width: u32,
    height: u32,
    axis_style: Style,
) -> ChartConfig {
    let mut sorted: Vec<f64> = data.iter().copied().filter(|v| v.is_finite()).collect();
    sorted.sort_by(f64::total_cmp);

    if sorted.is_empty() {
        return ChartConfig {
            title: Some("Histogram".to_string()),
            title_style: None,
            x_axis: Axis {
                title: options.x_title.clone(),
                bounds: Some((0.0, 1.0)),
                labels: None,
                ticks: None,
                title_style: None,
                style: axis_style,
            },
            y_axis: Axis {
                title: options.y_title.clone(),
                bounds: Some((0.0, 1.0)),
                labels: None,
                ticks: None,
                title_style: None,
                style: axis_style,
            },
            datasets: Vec::new(),
            legend: LegendPosition::None,
            grid: true,
            grid_style: None,
            hlines: Vec::new(),
            vlines: Vec::new(),
            frame_visible: true,
            x_axis_visible: true,
            y_axis_visible: true,
            width,
            height,
        };
    }

    let n = sorted.len();
    let min = sorted[0];
    let max = sorted[n.saturating_sub(1)];
    let bin_count = options.bins.unwrap_or_else(|| sturges_bin_count(n));

    let span = if (max - min).abs() < f64::EPSILON {
        1.0
    } else {
        max - min
    };
    let bin_width = span / bin_count as f64;

    let mut counts = vec![0usize; bin_count];
    for value in sorted {
        let raw = ((value - min) / bin_width).floor();
        let mut idx = if raw.is_finite() { raw as isize } else { 0 };
        if idx < 0 {
            idx = 0;
        }
        if idx as usize >= bin_count {
            idx = (bin_count.saturating_sub(1)) as isize;
        }
        counts[idx as usize] = counts[idx as usize].saturating_add(1);
    }

    let mut data_points = Vec::with_capacity(bin_count);
    for (i, count) in counts.iter().enumerate() {
        let center = min + (i as f64 + 0.5) * bin_width;
        data_points.push((center, *count as f64));
    }

    let mut labels: Vec<String> = Vec::new();
    let step = (bin_count / 4).max(1);
    for i in (0..=bin_count).step_by(step) {
        let edge = min + i as f64 * bin_width;
        labels.push(format_number(edge, bin_width));
    }

    ChartConfig {
        title: Some("Histogram".to_string()),
        title_style: None,
        x_axis: Axis {
            title: options.x_title.clone(),
            bounds: Some((min, max.max(min + bin_width))),
            labels: Some(labels),
            ticks: None,
            title_style: None,
            style: axis_style,
        },
        y_axis: Axis {
            title: options.y_title.clone(),
            bounds: Some((0.0, counts.iter().copied().max().unwrap_or(1) as f64)),
            labels: None,
            ticks: None,
            title_style: None,
            style: axis_style,
        },
        datasets: vec![Dataset {
            name: "Histogram".to_string(),
            data: data_points,
            color: options.color,
            marker: Marker::Block,
            graph_type: GraphType::Bar,
            up_color: None,
            down_color: None,
        }],
        legend: LegendPosition::None,
        grid: true,
        grid_style: None,
        hlines: Vec::new(),
        vlines: Vec::new(),
        frame_visible: true,
        x_axis_visible: true,
        y_axis_visible: true,
        width,
        height,
    }
}

/// Render a chart into styled row segments.
pub(crate) fn render_chart(config: &ChartConfig) -> Vec<ChartRow> {
    let width = config.width as usize;
    let height = config.height as usize;
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let frame_style = config.x_axis.style;
    let dim_style = Style::new().dim();
    let axis_style = config.y_axis.style;
    let title_style = Style::new()
        .bold()
        .fg(config.x_axis.style.fg.unwrap_or(Color::White));
    let title_style = config.title_style.unwrap_or(title_style);

    let title_rows = usize::from(config.title.is_some());
    let has_x_title = config.x_axis_visible && config.x_axis.title.is_some();
    let x_title_rows = usize::from(has_x_title);
    let frame_rows = if config.frame_visible { 2 } else { 0 };
    let x_axis_rows = if config.x_axis_visible {
        2 + x_title_rows
    } else {
        0
    };

    // Row budget: title + top_frame + plot + axis_line + x_labels + [x_title] + bottom_frame
    //           = title_rows + 1 + plot_height + 1 + 1 + x_title_rows + 1
    //           = title_rows + plot_height + 3 + x_title_rows
    // Solve for plot_height:
    let overhead = title_rows + frame_rows + x_axis_rows;
    if height <= overhead || width < 3 {
        return minimal_chart(config, width, frame_style, title_style);
    }
    let plot_height = height.saturating_sub(overhead).max(1);

    let (x_min, x_max) = resolve_bounds(
        config
            .datasets
            .iter()
            .flat_map(|d| d.data.iter().map(|p| p.0)),
        config.x_axis.bounds,
    );
    let (y_min, y_max) = resolve_bounds(
        config
            .datasets
            .iter()
            .flat_map(|d| d.data.iter().map(|p| p.1)),
        config.y_axis.bounds,
    );

    let y_label_chars: Vec<char> = if config.y_axis_visible {
        config
            .y_axis
            .title
            .as_deref()
            .map(|t| t.chars().collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let y_label_col_width = if y_label_chars.is_empty() { 0 } else { 2 };

    let legend_items = build_legend_items(&config.datasets);
    let legend_on_right = matches!(
        config.legend,
        LegendPosition::TopRight | LegendPosition::BottomRight
    );
    let legend_width = if legend_on_right && !legend_items.is_empty() {
        legend_items
            .iter()
            .map(|(_, name, _)| 4 + UnicodeWidthStr::width(name.as_str()))
            .max()
            .unwrap_or(0)
    } else {
        0
    };

    let y_ticks = if let Some(ref manual) = config.y_axis.ticks {
        TickSpec {
            values: manual.clone(),
            step: if manual.len() > 1 {
                manual[1] - manual[0]
            } else {
                1.0
            },
        }
    } else {
        build_tui_ticks(y_min, y_max, plot_height)
    };
    let y_min = y_ticks.values.first().copied().unwrap_or(y_min).min(y_min);
    let y_max = y_ticks.values.last().copied().unwrap_or(y_max).max(y_max);

    let use_manual_y_labels = config.y_axis.ticks.is_some() && config.y_axis.labels.is_some();
    let y_tick_labels: Vec<String> = if use_manual_y_labels {
        config
            .y_axis
            .labels
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .take(y_ticks.values.len())
            .cloned()
            .collect()
    } else {
        y_ticks
            .values
            .iter()
            .map(|v| format_number(*v, y_ticks.step))
            .collect()
    };
    let y_tick_width = y_tick_labels
        .iter()
        .map(|s| UnicodeWidthStr::width(s.as_str()))
        .max()
        .unwrap_or(1);
    let y_axis_width = if config.y_axis_visible {
        y_tick_width + 2
    } else {
        0
    };

    let inner_width = if config.frame_visible {
        width.saturating_sub(2)
    } else {
        width
    };
    let plot_width = inner_width
        .saturating_sub(y_label_col_width)
        .saturating_sub(y_axis_width)
        .saturating_sub(legend_width)
        .max(1);
    let content_width = y_label_col_width + y_axis_width + plot_width + legend_width;

    let x_ticks = if let Some(ref manual) = config.x_axis.ticks {
        TickSpec {
            values: manual.clone(),
            step: if manual.len() > 1 {
                manual[1] - manual[0]
            } else {
                1.0
            },
        }
    } else {
        build_tui_ticks(x_min, x_max, plot_width)
    };
    let x_min = x_ticks.values.first().copied().unwrap_or(x_min).min(x_min);
    let x_max = x_ticks.values.last().copied().unwrap_or(x_max).max(x_max);

    let mut plot_chars = vec![vec![' '; plot_width]; plot_height];
    let mut plot_styles = vec![vec![Style::new(); plot_width]; plot_height];

    apply_grid(
        config,
        GridSpec {
            x_ticks: &x_ticks.values,
            y_ticks: &y_ticks.values,
            x_min,
            x_max,
            y_min,
            y_max,
        },
        &mut plot_chars,
        &mut plot_styles,
        config.grid_style.unwrap_or(dim_style),
    );

    for &(y_val, ref style) in &config.hlines {
        let row = map_value_to_cell(y_val, y_min, y_max, plot_height, true);
        if row < plot_height {
            for col in 0..plot_width {
                plot_chars[row][col] = '─';
                plot_styles[row][col] = *style;
            }
        }
    }
    for &(x_val, ref style) in &config.vlines {
        let col = map_value_to_cell(x_val, x_min, x_max, plot_width, false);
        if col < plot_width {
            for row in 0..plot_height {
                if plot_chars[row][col] == ' ' || plot_chars[row][col] == '·' {
                    plot_chars[row][col] = '│';
                    plot_styles[row][col] = *style;
                }
            }
        }
    }

    for dataset in &config.datasets {
        match dataset.graph_type {
            GraphType::Line | GraphType::Area | GraphType::Scatter => {
                draw_braille_dataset(
                    dataset,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &mut plot_chars,
                    &mut plot_styles,
                );
            }
            GraphType::Bar => {
                draw_bar_dataset(
                    dataset,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &mut plot_chars,
                    &mut plot_styles,
                );
            }
        }
    }

    if !legend_items.is_empty()
        && matches!(
            config.legend,
            LegendPosition::TopLeft | LegendPosition::BottomLeft
        )
    {
        overlay_legend_on_plot(
            config.legend,
            &legend_items,
            &mut plot_chars,
            &mut plot_styles,
            axis_style,
        );
    }

    let y_tick_rows = build_y_tick_row_map(
        &y_ticks.values,
        if use_manual_y_labels {
            config.y_axis.labels.as_deref()
        } else {
            None
        },
        y_min,
        y_max,
        plot_height,
    );
    let x_tick_cols = build_x_tick_col_map(
        &x_ticks.values,
        config.x_axis.labels.as_deref(),
        config.x_axis.ticks.is_some() && config.x_axis.labels.is_some(),
        x_min,
        x_max,
        plot_width,
    );

    let mut rows: Vec<ChartRow> = Vec::with_capacity(height);

    // --- Title row ---
    if let Some(title) = &config.title {
        rows.push(ChartRow {
            segments: vec![(center_text(title, width), title_style)],
        });
    }

    if config.frame_visible {
        rows.push(ChartRow {
            segments: vec![(format!("┌{}┐", "─".repeat(content_width)), frame_style)],
        });
    }

    let y_label_start = if y_label_chars.is_empty() {
        0
    } else {
        plot_height.saturating_sub(y_label_chars.len()) / 2
    };
    let y_title_style = config.y_axis.title_style.unwrap_or(axis_style);

    let zero_label = format_number(0.0, y_ticks.step);
    for row in 0..plot_height {
        let mut segments: Vec<(String, Style)> = Vec::new();
        if config.frame_visible {
            segments.push(("│".to_string(), frame_style));
        }

        if config.y_axis_visible {
            if y_label_col_width > 0 {
                let label_idx = row.wrapping_sub(y_label_start);
                if label_idx < y_label_chars.len() {
                    segments.push((format!("{} ", y_label_chars[label_idx]), y_title_style));
                } else {
                    segments.push(("  ".to_string(), Style::new()));
                }
            }

            let (label, divider) =
                if let Some(index) = y_tick_rows.iter().position(|(r, _)| *r == row) {
                    let is_zero = y_tick_rows[index].1 == zero_label;
                    (
                        y_tick_rows[index].1.clone(),
                        if is_zero { '┼' } else { '┤' },
                    )
                } else {
                    (String::new(), '│')
                };
            let padded = format!("{:>w$}", label, w = y_tick_width);
            segments.push((padded, axis_style));
            segments.push((format!("{divider} "), axis_style));
        }

        let mut current_style = Style::new();
        let mut buffer = String::new();
        for col in 0..plot_width {
            let style = plot_styles[row][col];
            if col == 0 {
                current_style = style;
            }
            if style != current_style {
                if !buffer.is_empty() {
                    segments.push((buffer.clone(), current_style));
                    buffer.clear();
                }
                current_style = style;
            }
            buffer.push(plot_chars[row][col]);
        }
        if !buffer.is_empty() {
            segments.push((buffer, current_style));
        }

        if legend_on_right && legend_width > 0 {
            let legend_row = match config.legend {
                LegendPosition::TopRight => row,
                LegendPosition::BottomRight => {
                    row.wrapping_add(legend_items.len().saturating_sub(plot_height))
                }
                _ => usize::MAX,
            };
            if let Some((symbol, name, color)) = legend_items.get(legend_row) {
                let raw = format!("  {symbol} {name}");
                let raw_w = UnicodeWidthStr::width(raw.as_str());
                let pad = legend_width.saturating_sub(raw_w);
                let text = format!("{raw}{}", " ".repeat(pad));
                segments.push((text, Style::new().fg(*color)));
            } else {
                segments.push((" ".repeat(legend_width), Style::new()));
            }
        }

        if config.frame_visible {
            segments.push(("│".to_string(), frame_style));
        }
        rows.push(ChartRow { segments });
    }

    if config.x_axis_visible {
        let mut axis_line = vec!['─'; plot_width];
        for (col, _) in &x_tick_cols {
            if *col < plot_width {
                axis_line[*col] = '┬';
            }
        }
        let footer_legend_pad = " ".repeat(legend_width);
        let footer_ylabel_pad = if config.y_axis_visible {
            " ".repeat(y_label_col_width)
        } else {
            String::new()
        };

        let mut axis_segments: Vec<(String, Style)> = Vec::new();
        if config.frame_visible {
            axis_segments.push(("│".to_string(), frame_style));
        }
        if config.y_axis_visible {
            axis_segments.push((footer_ylabel_pad.clone(), Style::new()));
            axis_segments.push((" ".repeat(y_tick_width), axis_style));
            axis_segments.push(("┴─".to_string(), axis_style));
        }
        axis_segments.push((axis_line.into_iter().collect(), axis_style));
        axis_segments.push((footer_legend_pad.clone(), Style::new()));
        if config.frame_visible {
            axis_segments.push(("│".to_string(), frame_style));
        }
        rows.push(ChartRow {
            segments: axis_segments,
        });

        let mut x_label_line: Vec<char> = vec![' '; plot_width];
        let mut occupied_until: usize = 0;
        for (col, label) in &x_tick_cols {
            if label.is_empty() {
                continue;
            }
            let label_width = UnicodeWidthStr::width(label.as_str());
            let start = col
                .saturating_sub(label_width / 2)
                .min(plot_width.saturating_sub(label_width));
            if start < occupied_until {
                continue;
            }
            for (offset, ch) in label.chars().enumerate() {
                let idx = start + offset;
                if idx < plot_width {
                    x_label_line[idx] = ch;
                }
            }
            occupied_until = start + label_width + 1;
        }

        let mut x_label_segments: Vec<(String, Style)> = Vec::new();
        if config.frame_visible {
            x_label_segments.push(("│".to_string(), frame_style));
        }
        if config.y_axis_visible {
            x_label_segments.push((footer_ylabel_pad.clone(), Style::new()));
            x_label_segments.push((" ".repeat(y_axis_width), Style::new()));
        }
        x_label_segments.push((x_label_line.into_iter().collect(), axis_style));
        x_label_segments.push((footer_legend_pad.clone(), Style::new()));
        if config.frame_visible {
            x_label_segments.push(("│".to_string(), frame_style));
        }
        rows.push(ChartRow {
            segments: x_label_segments,
        });

        if has_x_title {
            let x_title_text = config.x_axis.title.as_deref().unwrap_or_default();
            let x_title = center_text(x_title_text, plot_width);
            let x_title_style = config.x_axis.title_style.unwrap_or(axis_style);
            let mut x_title_segments: Vec<(String, Style)> = Vec::new();
            if config.frame_visible {
                x_title_segments.push(("│".to_string(), frame_style));
            }
            if config.y_axis_visible {
                x_title_segments.push((footer_ylabel_pad, Style::new()));
                x_title_segments.push((" ".repeat(y_axis_width), Style::new()));
            }
            x_title_segments.push((x_title, x_title_style));
            x_title_segments.push((footer_legend_pad, Style::new()));
            if config.frame_visible {
                x_title_segments.push(("│".to_string(), frame_style));
            }
            rows.push(ChartRow {
                segments: x_title_segments,
            });
        }
    }

    if config.frame_visible {
        rows.push(ChartRow {
            segments: vec![(format!("└{}┘", "─".repeat(content_width)), frame_style)],
        });
    }

    rows
}

fn minimal_chart(
    config: &ChartConfig,
    width: usize,
    frame_style: Style,
    title_style: Style,
) -> Vec<ChartRow> {
    let mut rows = Vec::new();
    if let Some(title) = &config.title {
        rows.push(ChartRow {
            segments: vec![(center_text(title, width), title_style)],
        });
    }
    if config.frame_visible {
        let inner = width.saturating_sub(2);
        rows.push(ChartRow {
            segments: vec![(format!("┌{}┐", "─".repeat(inner)), frame_style)],
        });
        rows.push(ChartRow {
            segments: vec![(format!("│{}│", " ".repeat(inner)), frame_style)],
        });
        rows.push(ChartRow {
            segments: vec![(format!("└{}┘", "─".repeat(inner)), frame_style)],
        });
    } else {
        rows.push(ChartRow {
            segments: vec![(" ".repeat(width), Style::new())],
        });
    }
    rows
}

fn resolve_bounds<I>(values: I, manual: Option<(f64, f64)>) -> (f64, f64)
where
    I: Iterator<Item = f64>,
{
    if let Some((min, max)) = manual {
        return normalize_bounds(min, max);
    }

    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for value in values {
        if !value.is_finite() {
            continue;
        }
        min = min.min(value);
        max = max.max(value);
    }

    if !min.is_finite() || !max.is_finite() {
        return (0.0, 1.0);
    }

    normalize_bounds(min, max)
}

fn normalize_bounds(min: f64, max: f64) -> (f64, f64) {
    if (max - min).abs() < f64::EPSILON {
        let pad = if min.abs() < 1.0 {
            1.0
        } else {
            min.abs() * 0.1
        };
        (min - pad, max + pad)
    } else if min < max {
        (min, max)
    } else {
        (max, min)
    }
}

#[derive(Debug, Clone)]
struct TickSpec {
    values: Vec<f64>,
    step: f64,
}

fn build_ticks(min: f64, max: f64, target: usize) -> TickSpec {
    let span = (max - min).abs().max(f64::EPSILON);
    let range = nice_number(span, false);
    let raw_step = range / (target.max(2) as f64 - 1.0);
    let step = nice_number(raw_step, true).max(f64::EPSILON);
    let nice_min = (min / step).floor() * step;
    let nice_max = (max / step).ceil() * step;

    let mut values = Vec::new();
    let mut value = nice_min;
    let limit = nice_max + step * 0.5;
    let mut guard = 0usize;
    while value <= limit && guard < 128 {
        values.push(value);
        value += step;
        guard = guard.saturating_add(1);
    }

    if values.is_empty() {
        values.push(min);
        values.push(max);
    }

    TickSpec { values, step }
}

/// TUI-aware tick generation: picks a nice step whose interval count
/// divides `cell_count - 1` as evenly as possible, with 3-8 intervals
/// and at least 2 rows per interval for readable spacing.
fn build_tui_ticks(data_min: f64, data_max: f64, cell_count: usize) -> TickSpec {
    let last = cell_count.saturating_sub(1).max(1);
    let span = (data_max - data_min).abs().max(f64::EPSILON);
    let log = span.log10().floor();

    let mut candidates: Vec<(f64, f64, usize, usize)> = Vec::new();

    for exp_off in -1..=1i32 {
        let base = 10.0_f64.powf(log + f64::from(exp_off));
        for &mult in &[1.0, 2.0, 2.5, 5.0] {
            let step = base * mult;
            if step <= 0.0 || !step.is_finite() {
                continue;
            }
            let lo = (data_min / step).floor() * step;
            let hi = (data_max / step).ceil() * step;
            let n = ((hi - lo) / step + 0.5) as usize;
            if (3..=8).contains(&n) && last / n >= 2 {
                let rem = last % n;
                candidates.push((step, lo, n, rem));
            }
        }
    }

    candidates.sort_by(|a, b| {
        a.3.cmp(&b.3).then_with(|| {
            let da = (a.2 as i32 - 5).unsigned_abs();
            let db = (b.2 as i32 - 5).unsigned_abs();
            da.cmp(&db)
        })
    });

    if let Some(&(step, lo, n, _)) = candidates.first() {
        let values: Vec<f64> = (0..=n).map(|i| lo + step * i as f64).collect();
        return TickSpec { values, step };
    }

    build_ticks(data_min, data_max, 5)
}

fn nice_number(value: f64, round: bool) -> f64 {
    if value <= 0.0 || !value.is_finite() {
        return 1.0;
    }
    let exponent = value.log10().floor();
    let power = 10.0_f64.powf(exponent);
    let fraction = value / power;

    let nice_fraction = if round {
        if fraction < 1.5 {
            1.0
        } else if fraction < 3.0 {
            2.0
        } else if fraction < 7.0 {
            5.0
        } else {
            10.0
        }
    } else if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };

    nice_fraction * power
}

fn format_number(value: f64, step: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let abs_step = step.abs().max(f64::EPSILON);
    let precision = if abs_step >= 1.0 {
        0
    } else {
        (-abs_step.log10().floor() as i32 + 1).clamp(0, 6) as usize
    };
    format!("{value:.precision$}")
}

fn build_legend_items(datasets: &[Dataset]) -> Vec<(char, String, Color)> {
    datasets
        .iter()
        .filter(|d| !d.name.is_empty())
        .map(|d| {
            let symbol = match d.graph_type {
                GraphType::Line => '─',
                GraphType::Area => '█',
                GraphType::Scatter => marker_char(d.marker),
                GraphType::Bar => '█',
            };
            (symbol, d.name.clone(), d.color)
        })
        .collect()
}

fn marker_char(marker: Marker) -> char {
    match marker {
        Marker::Braille => '⣿',
        Marker::Dot => '•',
        Marker::Block => '█',
        Marker::HalfBlock => '▀',
        Marker::Cross => '×',
        Marker::Circle => '○',
    }
}

struct GridSpec<'a> {
    x_ticks: &'a [f64],
    y_ticks: &'a [f64],
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
}

fn apply_grid(
    config: &ChartConfig,
    grid: GridSpec<'_>,
    plot_chars: &mut [Vec<char>],
    plot_styles: &mut [Vec<Style>],
    grid_style: Style,
) {
    if !config.grid || plot_chars.is_empty() || plot_chars[0].is_empty() {
        return;
    }
    let h = plot_chars.len();
    let w = plot_chars[0].len();

    for tick in grid.y_ticks {
        let row = map_value_to_cell(*tick, grid.y_min, grid.y_max, h, true);
        if row < h {
            for col in 0..w {
                if plot_chars[row][col] == ' ' {
                    plot_chars[row][col] = '·';
                    plot_styles[row][col] = grid_style;
                }
            }
        }
    }

    for tick in grid.x_ticks {
        let col = map_value_to_cell(*tick, grid.x_min, grid.x_max, w, false);
        if col < w {
            for row in 0..h {
                if plot_chars[row][col] == ' ' {
                    plot_chars[row][col] = '·';
                    plot_styles[row][col] = grid_style;
                }
            }
        }
    }
}

fn draw_braille_dataset(
    dataset: &Dataset,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    plot_chars: &mut [Vec<char>],
    plot_styles: &mut [Vec<Style>],
) {
    if dataset.data.is_empty() || plot_chars.is_empty() || plot_chars[0].is_empty() {
        return;
    }

    let cols = plot_chars[0].len();
    let rows = plot_chars.len();
    let px_w = cols * 2;
    let px_h = rows * 4;
    let mut bits = vec![vec![0u32; cols]; rows];
    let mut color_map = vec![vec![None::<Color>; cols]; rows];

    let mut set_dot_colored = |px: usize, py: usize, color: Color| {
        set_braille_dot(px, py, &mut bits, cols, rows);
        let char_col = px / 2;
        let char_row = py / 4;
        if char_col < cols && char_row < rows {
            color_map[char_row][char_col] = Some(color);
        }
    };

    let points = dataset
        .data
        .iter()
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| {
            (
                map_value_to_cell(*x, x_min, x_max, px_w, false),
                map_value_to_cell(*y, y_min, y_max, px_h, true),
                *y,
            )
        })
        .collect::<Vec<_>>();

    if points.is_empty() {
        return;
    }

    if matches!(dataset.graph_type, GraphType::Line | GraphType::Area) {
        let mut line_y_by_x = if matches!(dataset.graph_type, GraphType::Area) {
            vec![None::<usize>; px_w]
        } else {
            Vec::new()
        };
        let mut line_color_by_x = if matches!(dataset.graph_type, GraphType::Area) {
            vec![None::<Color>; px_w]
        } else {
            Vec::new()
        };

        for idx in 0..points.len().saturating_sub(1) {
            let a = points[idx];
            let b = points[idx + 1];
            let seg_color = if let (Some(up), Some(down)) = (dataset.up_color, dataset.down_color) {
                if b.2 > a.2 {
                    up
                } else {
                    down
                }
            } else {
                dataset.color
            };

            plot_bresenham(
                a.0 as isize,
                a.1 as isize,
                b.0 as isize,
                b.1 as isize,
                |x, y| {
                    if x < 0 || y < 0 {
                        return;
                    }
                    let px = x as usize;
                    let py = y as usize;
                    set_dot_colored(px, py, seg_color);
                    if matches!(dataset.graph_type, GraphType::Area) && px < px_w && py < px_h {
                        line_y_by_x[px] = Some(match line_y_by_x[px] {
                            Some(existing) => existing.min(py),
                            None => py,
                        });
                        line_color_by_x[px] = Some(seg_color);
                    }
                },
            );
        }

        if matches!(dataset.graph_type, GraphType::Area) {
            for px in 0..px_w {
                if let Some(line_y) = line_y_by_x[px] {
                    let fill_color = line_color_by_x[px].unwrap_or(dataset.color);
                    for py in line_y..px_h {
                        set_dot_colored(px, py, fill_color);
                    }
                }
            }
        }
    } else {
        for (x, y, _) in &points {
            set_dot_colored(*x, *y, dataset.color);
        }
    }

    for row in 0..rows {
        for col in 0..cols {
            if bits[row][col] != 0 {
                let ch = char::from_u32(BRAILLE_BASE + bits[row][col]).unwrap_or(' ');
                plot_chars[row][col] = ch;
                let color = color_map[row][col].unwrap_or(dataset.color);
                plot_styles[row][col] = Style::new().fg(color);
            }
        }
    }

    if !matches!(dataset.marker, Marker::Braille) {
        let m = marker_char(dataset.marker);
        for (x, y) in dataset
            .data
            .iter()
            .filter(|(x, y)| x.is_finite() && y.is_finite())
        {
            let col = map_value_to_cell(*x, x_min, x_max, cols, false);
            let row = map_value_to_cell(*y, y_min, y_max, rows, true);
            if row < rows && col < cols {
                plot_chars[row][col] = m;
                plot_styles[row][col] = Style::new().fg(dataset.color);
            }
        }
    }
}

fn draw_bar_dataset(
    dataset: &Dataset,
    _x_min: f64,
    _x_max: f64,
    y_min: f64,
    y_max: f64,
    plot_chars: &mut [Vec<char>],
    plot_styles: &mut [Vec<Style>],
) {
    if dataset.data.is_empty() || plot_chars.is_empty() || plot_chars[0].is_empty() {
        return;
    }

    let rows = plot_chars.len();
    let cols = plot_chars[0].len();
    let n = dataset.data.len();
    let slot_width = cols as f64 / n as f64;
    let zero_row = map_value_to_cell(0.0, y_min, y_max, rows, true);

    for (index, (_, value)) in dataset.data.iter().enumerate() {
        if !value.is_finite() {
            continue;
        }

        let start_f = index as f64 * slot_width;
        let bar_width_f = (slot_width * 0.75).max(1.0);
        let full_w = bar_width_f.floor() as usize;
        let frac_w = ((bar_width_f - full_w as f64) * 8.0).round() as usize;

        let x_start = start_f.floor() as usize;
        let x_end = (x_start + full_w).min(cols.saturating_sub(1));
        let frac_col = (x_end + 1).min(cols.saturating_sub(1));

        let value_row = map_value_to_cell(*value, y_min, y_max, rows, true);
        let (top, bottom) = if value_row <= zero_row {
            (value_row, zero_row)
        } else {
            (zero_row, value_row)
        };

        for row in top..=bottom.min(rows.saturating_sub(1)) {
            for col in x_start..=x_end {
                if col < cols {
                    plot_chars[row][col] = '█';
                    plot_styles[row][col] = Style::new().fg(dataset.color);
                }
            }
            if frac_w > 0 && frac_col < cols {
                plot_chars[row][frac_col] = BLOCK_FRACTIONS[frac_w.min(8)];
                plot_styles[row][frac_col] = Style::new().fg(dataset.color);
            }
        }
    }
}

fn overlay_legend_on_plot(
    position: LegendPosition,
    items: &[(char, String, Color)],
    plot_chars: &mut [Vec<char>],
    plot_styles: &mut [Vec<Style>],
    axis_style: Style,
) {
    if plot_chars.is_empty() || plot_chars[0].is_empty() || items.is_empty() {
        return;
    }

    let rows = plot_chars.len();
    let cols = plot_chars[0].len();
    let start_row = match position {
        LegendPosition::TopLeft => 0,
        LegendPosition::BottomLeft => rows.saturating_sub(items.len()),
        _ => 0,
    };

    for (i, (symbol, name, color)) in items.iter().enumerate() {
        let row = start_row + i;
        if row >= rows {
            break;
        }
        let legend_text = format!("{symbol} {name}");
        for (col, ch) in legend_text.chars().enumerate() {
            if col >= cols {
                break;
            }
            plot_chars[row][col] = ch;
            plot_styles[row][col] = if col == 0 {
                Style::new().fg(*color)
            } else {
                axis_style
            };
        }
    }
}

fn build_y_tick_row_map(
    ticks: &[f64],
    labels: Option<&[String]>,
    y_min: f64,
    y_max: f64,
    plot_height: usize,
) -> Vec<(usize, String)> {
    let step = if ticks.len() > 1 {
        (ticks[1] - ticks[0]).abs()
    } else {
        1.0
    };
    ticks
        .iter()
        .enumerate()
        .map(|(idx, v)| {
            let label = labels
                .and_then(|manual| manual.get(idx).cloned())
                .unwrap_or_else(|| format_number(*v, step));
            (
                map_value_to_cell(*v, y_min, y_max, plot_height, true),
                label,
            )
        })
        .collect()
}

fn build_x_tick_col_map(
    ticks: &[f64],
    labels: Option<&[String]>,
    labels_match_manual_ticks: bool,
    x_min: f64,
    x_max: f64,
    plot_width: usize,
) -> Vec<(usize, String)> {
    if let Some(labels) = labels {
        if labels.is_empty() {
            return Vec::new();
        }
        if labels_match_manual_ticks {
            return ticks
                .iter()
                .zip(labels.iter())
                .map(|(tick, label)| {
                    (
                        map_value_to_cell(*tick, x_min, x_max, plot_width, false),
                        label.clone(),
                    )
                })
                .collect();
        }
        let denom = labels.len().saturating_sub(1).max(1);
        return labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let col = (i * plot_width.saturating_sub(1)) / denom;
                (col, label.clone())
            })
            .collect();
    }

    let step = if ticks.len() > 1 {
        (ticks[1] - ticks[0]).abs()
    } else {
        1.0
    };
    ticks
        .iter()
        .map(|v| {
            (
                map_value_to_cell(*v, x_min, x_max, plot_width, false),
                format_number(*v, step),
            )
        })
        .collect()
}

fn map_value_to_cell(value: f64, min: f64, max: f64, size: usize, invert: bool) -> usize {
    if size == 0 {
        return 0;
    }
    let span = (max - min).abs().max(f64::EPSILON);
    let mut t = ((value - min) / span).clamp(0.0, 1.0);
    if invert {
        t = 1.0 - t;
    }
    (t * (size.saturating_sub(1)) as f64).round() as usize
}

fn set_braille_dot(px: usize, py: usize, bits: &mut [Vec<u32>], cols: usize, rows: usize) {
    if cols == 0 || rows == 0 {
        return;
    }
    let char_col = px / 2;
    let char_row = py / 4;
    if char_col >= cols || char_row >= rows {
        return;
    }
    let sub_col = px % 2;
    let sub_row = py % 4;
    bits[char_row][char_col] |= if sub_col == 0 {
        BRAILLE_LEFT_BITS[sub_row]
    } else {
        BRAILLE_RIGHT_BITS[sub_row]
    };
}

fn plot_bresenham(x0: isize, y0: isize, x1: isize, y1: isize, mut plot: impl FnMut(isize, isize)) {
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        plot(x, y);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

fn center_text(text: &str, width: usize) -> String {
    let text_width = UnicodeWidthStr::width(text);
    if text_width >= width {
        return text.chars().take(width).collect();
    }
    let left = (width - text_width) / 2;
    let right = width - text_width - left;
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
}

fn sturges_bin_count(n: usize) -> usize {
    if n <= 1 {
        return 1;
    }
    (1.0 + (n as f64).log2()).ceil() as usize
}
