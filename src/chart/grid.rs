use super::*;
use unicode_width::UnicodeWidthStr;

pub(super) struct GridSpec<'a> {
    pub(super) x_ticks: &'a [f64],
    pub(super) y_ticks: &'a [f64],
    pub(super) x_min: f64,
    pub(super) x_max: f64,
    pub(super) y_min: f64,
    pub(super) y_max: f64,
}

pub(super) fn apply_grid(
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

pub(super) fn build_legend_items(datasets: &[Dataset]) -> Vec<(char, String, Color)> {
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

pub(super) fn marker_char(marker: Marker) -> char {
    match marker {
        Marker::Braille => '⣿',
        Marker::Dot => '•',
        Marker::Block => '█',
        Marker::HalfBlock => '▀',
        Marker::Cross => '×',
        Marker::Circle => '○',
    }
}

pub(super) fn overlay_legend_on_plot(
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

pub(super) fn build_y_tick_row_map(
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

pub(super) fn build_x_tick_col_map(
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

pub(super) fn map_value_to_cell(
    value: f64,
    min: f64,
    max: f64,
    size: usize,
    invert: bool,
) -> usize {
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

pub(super) fn center_text(text: &str, width: usize) -> String {
    let text_width = UnicodeWidthStr::width(text);
    if text_width >= width {
        return text.chars().take(width).collect();
    }
    let left = (width - text_width) / 2;
    let right = width - text_width - left;
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
}

pub(super) fn sturges_bin_count(n: usize) -> usize {
    if n <= 1 {
        return 1;
    }
    (1.0 + (n as f64).log2()).ceil() as usize
}
