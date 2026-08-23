use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_bar_dataset(
    dataset: &Dataset,
    _x_min: f64,
    _x_max: f64,
    y_min: f64,
    y_max: f64,
    plot_chars: &mut [char],
    plot_styles: &mut [Style],
    cols: usize,
    rows: usize,
) {
    if dataset.data.is_empty() || cols == 0 || rows == 0 {
        return;
    }

    let n = dataset.data.len();
    let zero_row = map_value_to_cell(0.0, y_min, y_max, rows, true);

    for (index, (_, value)) in dataset.data.iter().enumerate() {
        if !value.is_finite() {
            continue;
        }

        let slot = bar_slot(index, n, cols);
        let x_start = slot.start;
        let x_end = slot.end;
        if x_start >= x_end || x_start >= cols {
            continue;
        }

        let value_row = map_value_to_cell(*value, y_min, y_max, rows, true);
        let (top, bottom) = if value_row <= zero_row {
            (value_row, zero_row)
        } else {
            (zero_row, value_row)
        };

        for row in top..=bottom.min(rows.saturating_sub(1)) {
            for col in x_start..x_end.min(cols) {
                let idx = row * cols + col;
                plot_chars[idx] = '█';
                plot_styles[idx] = Style::new().fg(dataset.color);
            }
        }
    }
}

fn bar_slot(index: usize, count: usize, cols: usize) -> std::ops::Range<usize> {
    if count == 0 {
        return 0..0;
    }
    index.saturating_mul(cols) / count..(index + 1).saturating_mul(cols) / count
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
            title: None,
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
            frame_visible: false,
            x_axis_visible: true,
            y_axis_visible: true,
            width,
            height,
        };
    }

    let n = sorted.len();
    let (min, max) = normalize_bounds(sorted[0], sorted[n.saturating_sub(1)]);
    let bin_count = options.bins.unwrap_or_else(|| sturges_bin_count(n));

    let span = max - min;
    let bin_width = span / bin_count as f64;

    let mut counts = vec![0usize; bin_count];
    for value in sorted {
        let ratio = finite_ratio(value, min, max).unwrap_or(0.0);
        let idx = ((ratio * bin_count as f64).floor() as usize).min(bin_count.saturating_sub(1));
        counts[idx] = counts[idx].saturating_add(1);
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
        title: None,
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
            name: String::new(),
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
        frame_visible: false,
        x_axis_visible: true,
        y_axis_visible: true,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::bar_slot;

    #[test]
    fn adjacent_bar_slots_are_half_open_and_disjoint() {
        for cols in 1..64 {
            for count in 1..64 {
                let slots: Vec<_> = (0..count)
                    .map(|index| bar_slot(index, count, cols))
                    .collect();
                for pair in slots.windows(2) {
                    assert!(pair[0].end <= pair[1].start, "cols={cols}, count={count}");
                }
                assert!(slots.iter().all(|slot| slot.end <= cols));
            }
        }
    }
}
