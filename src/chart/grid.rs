use super::*;
use unicode_segmentation::UnicodeSegmentation;
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

pub(super) fn overlay_legend_row(
    item: &(char, String, Color),
    cols: usize,
    plot_styles: &mut [Style],
    axis_style: Style,
) -> Vec<String> {
    let (symbol, name, color) = item;
    let text = format!("{symbol} {name}");
    let mut cells = vec![String::new(); cols.min(UnicodeWidthStr::width(text.as_str()))];
    let end = write_text_cells(&mut cells, 0, &text);
    cells.truncate(end);
    plot_styles[..end].fill(axis_style);
    if end > 0 {
        plot_styles[0] = Style::new().fg(*color);
    }
    cells
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
    let mut t = finite_ratio(value, min, max).unwrap_or(0.0);
    if invert {
        t = 1.0 - t;
    }
    (t * (size.saturating_sub(1)) as f64).round() as usize
}

pub(super) fn center_text(text: &str, width: usize) -> String {
    let text = clip_text_cells(text, width);
    let text_width = UnicodeWidthStr::width(text.as_str());
    let left = (width - text_width) / 2;
    let right = width - text_width - left;
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
}

pub(crate) fn clip_text_cells(text: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return String::new();
    }
    let mut result = String::new();
    let mut width = 0usize;
    for grapheme in text.graphemes(true) {
        let grapheme_width = UnicodeWidthStr::width(grapheme);
        if width.saturating_add(grapheme_width) > max_cols {
            break;
        }
        result.push_str(grapheme);
        width = width.saturating_add(grapheme_width);
    }
    result
}

pub(crate) fn write_text_cells(cells: &mut [String], start: usize, text: &str) -> usize {
    let mut cursor = start;
    let mut last_origin: Option<usize> = None;
    for grapheme in text.graphemes(true) {
        let width = UnicodeWidthStr::width(grapheme);
        if width == 0 {
            if let Some(origin) = last_origin {
                cells[origin].push_str(grapheme);
            }
            continue;
        }
        if cursor.saturating_add(width) > cells.len() {
            break;
        }
        cells[cursor].clear();
        cells[cursor].push_str(grapheme);
        for continuation in 1..width {
            cells[cursor + continuation].clear();
        }
        last_origin = Some(cursor);
        cursor += width;
    }
    cursor
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
    let total = UnicodeWidthStr::width(text);
    if total <= max_cols {
        return text.to_string();
    }
    if max_cols < 3 {
        return String::new();
    }
    let target = max_cols - 1;
    let mut result = String::new();
    let mut width = 0usize;
    for grapheme in text.graphemes(true) {
        let cw = UnicodeWidthStr::width(grapheme);
        if width + cw > target {
            break;
        }
        result.push_str(grapheme);
        width += cw;
    }
    result.push('\u{2026}');
    result
}

#[cfg(test)]
mod tests {
    use super::{clip_text_cells, truncate_label, write_text_cells};
    use unicode_width::UnicodeWidthStr;

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

    #[test]
    fn clipping_preserves_graphemes_and_exact_cell_budget() {
        assert_eq!(clip_text_cells("A한B", 3), "A한");
        assert_eq!(clip_text_cells("e\u{301}x", 1), "e\u{301}");
        assert_eq!(
            UnicodeWidthStr::width(clip_text_cells("🎉파티", 4).as_str()),
            4
        );
    }

    #[test]
    fn cell_writer_reserves_wide_continuation_cells() {
        let mut cells = vec![" ".to_string(); 6];
        let end = write_text_cells(&mut cells, 1, "한e\u{301}");
        assert_eq!(end, 4);
        assert_eq!(cells.concat(), " 한e\u{301}  ");
        assert_eq!(UnicodeWidthStr::width(cells.concat().as_str()), 6);
    }
}
