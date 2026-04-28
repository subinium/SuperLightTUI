use super::*;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
    plot_chars: &mut [char],
    plot_styles: &mut [Style],
    cols: usize,
    rows: usize,
    grid_style: Style,
) {
    if !config.grid || cols == 0 || rows == 0 {
        return;
    }
    let h = rows;
    let w = cols;

    for tick in grid.y_ticks {
        let row = map_value_to_cell(*tick, grid.y_min, grid.y_max, h, true);
        if row < h {
            for col in 0..w {
                let idx = row * w + col;
                if plot_chars[idx] == ' ' {
                    plot_chars[idx] = '·';
                    plot_styles[idx] = grid_style;
                }
            }
        }
    }

    for tick in grid.x_ticks {
        let col = map_value_to_cell(*tick, grid.x_min, grid.x_max, w, false);
        if col < w {
            for row in 0..h {
                let idx = row * w + col;
                if plot_chars[idx] == ' ' {
                    plot_chars[idx] = '·';
                    plot_styles[idx] = grid_style;
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
    plot_chars: &mut [char],
    plot_styles: &mut [Style],
    cols: usize,
    rows: usize,
    axis_style: Style,
) {
    if cols == 0 || rows == 0 || items.is_empty() {
        return;
    }

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
            let idx = row * cols + col;
            plot_chars[idx] = ch;
            plot_styles[idx] = if col == 0 {
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

/// Fit `text` into at most `max_cols` terminal cells, replacing the tail with
/// a single-cell ellipsis (`…`) when it would otherwise be clipped.
///
/// Width is measured in unicode display cells (CJK = 2). Returns the original
/// string when it already fits, an ellipsis-truncated prefix when it does not,
/// and an empty string when `max_cols < 3` (a 1- or 2-cell budget cannot fit
/// any meaningful prefix plus an ellipsis, so we drop the label entirely
/// rather than emit a single garbled character).
pub(crate) fn truncate_label(text: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    let total: usize = text
        .chars()
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(0))
        .sum();
    if total <= max_cols {
        return text.to_string();
    }
    if max_cols < 3 {
        return String::new();
    }
    let target = max_cols - 1;
    let mut result = String::new();
    let mut width = 0usize;
    for ch in text.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + cw > target {
            break;
        }
        result.push(ch);
        width += cw;
    }
    result.push('\u{2026}');
    result
}

#[cfg(test)]
mod tests {
    use super::truncate_label;

    #[test]
    fn keeps_short_label_unchanged() {
        assert_eq!(truncate_label("CPU", 10), "CPU");
        assert_eq!(truncate_label("CPU", 3), "CPU");
    }

    #[test]
    fn adds_ellipsis_when_truncated() {
        // "Python" is 6 cells; budget 5 → "Pyth" + "…" = 5 cells.
        assert_eq!(truncate_label("Python", 5), "Pyth\u{2026}");
        // budget 4 → "Pyt" + "…" = 4 cells.
        assert_eq!(truncate_label("Python", 4), "Pyt\u{2026}");
        // budget 3 → "Py" + "…" = 3 cells.
        assert_eq!(truncate_label("Python", 3), "Py\u{2026}");
    }

    #[test]
    fn drops_label_when_too_narrow() {
        assert_eq!(truncate_label("Python", 0), "");
        assert_eq!(truncate_label("Python", 1), "");
        assert_eq!(truncate_label("Python", 2), "");
    }

    #[test]
    fn handles_cjk_double_width() {
        // "한글" is 4 cells (each char = 2). Budget 4 → fits.
        assert_eq!(truncate_label("한글", 4), "한글");
        // Budget 3 → can't fit one CJK + ellipsis (would need 3 cells exactly:
        // 2 + 1) so it works: "한…" is 3 cells.
        assert_eq!(truncate_label("한글", 3), "한\u{2026}");
        // Budget 2 → falls through to drop (max_cols < 3).
        // (We could fit "…" but the policy is: prefer dropping over a lone
        // ellipsis, since that conveys no information.)
        // Actually max_cols=2 IS >= 3? No, 2 < 3, so dropped.
        assert_eq!(truncate_label("한글파일", 2), "");
    }
}
