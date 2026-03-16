use super::*;

pub(super) fn draw_bar_dataset(
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
