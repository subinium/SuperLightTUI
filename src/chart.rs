//! Data visualization: line charts, scatter plots, bar charts, and histograms.
//!
//! Build a chart with [`ChartBuilder`], then pass it to
//! [`Context::chart`](crate::Context::chart). Histograms use
//! [`Context::histogram`](crate::Context::histogram) directly.

use crate::style::{Color, Style};

mod axis;
mod bar;
mod braille;
mod grid;
mod render;

pub(crate) use bar::build_histogram_config;
pub(crate) use render::render_chart;

use axis::{build_tui_ticks, format_number, resolve_bounds, TickSpec};
use bar::draw_bar_dataset;
use braille::draw_braille_dataset;
use grid::{
    apply_grid, build_legend_items, build_x_tick_col_map, build_y_tick_row_map, center_text,
    map_value_to_cell, marker_char, overlay_legend_on_plot, sturges_bin_count, GridSpec,
};

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
            y_title: None,
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
                frame_visible: false,
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
                    let width = unicode_width::UnicodeWidthStr::width(segment.as_str());
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
