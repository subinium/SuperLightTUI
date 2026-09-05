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
        // Exact zero has no height. Nonzero values keep at least one cell,
        // including small values quantized onto the baseline and one-row plots.
        if !value.is_finite() || *value == 0.0 {
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
    let mut n = 0usize;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &value in data.iter().filter(|value| value.is_finite()) {
        n += 1;
        min = min.min(value);
        max = max.max(value);
    }

    if n == 0 {
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

    let (min, max) = normalize_bounds(min, max);
    let bin_count = options.bins.unwrap_or_else(|| sturges_bin_count(n)).max(1);

    let span = max - min;
    let bin_width = span / bin_count as f64;

    let mut counts = vec![0usize; bin_count];
    for &value in data.iter().filter(|value| value.is_finite()) {
        let ratio = finite_ratio(value, min, max).unwrap_or(0.0);
        let idx = ((ratio * bin_count as f64).floor() as usize).min(bin_count.saturating_sub(1));
        counts[idx] = counts[idx].saturating_add(1);
    }

    let mut data_points = Vec::with_capacity(bin_count);
    for (i, count) in counts.iter().enumerate() {
        let center = min + (i as f64 + 0.5) * bin_width;
        data_points.push((center, *count as f64));
    }

    let mut ticks = Vec::new();
    let step = (bin_count / 4).max(1);
    for i in (0..=bin_count).step_by(step) {
        ticks.push(if i == bin_count {
            max
        } else {
            min + i as f64 * bin_width
        });
    }
    if !bin_count.is_multiple_of(step) {
        ticks.push(max);
    }
    let labels = ticks
        .iter()
        .map(|edge| format_number(*edge, bin_width))
        .collect();

    ChartConfig {
        title: None,
        title_style: None,
        x_axis: Axis {
            title: options.x_title.clone(),
            bounds: Some((min, max.max(min + bin_width))),
            labels: Some(labels),
            ticks: Some(ticks),
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
    use super::*;

    #[test]
    fn histogram_ticks_preserve_numeric_edges_and_endpoints() {
        for bins in [1, 3, 8, 9, 11] {
            for (min, max) in [(0.0, 9.0), (-0.75, 2.25)] {
                let options = HistogramBuilder {
                    bins: Some(bins),
                    ..Default::default()
                };
                let config = build_histogram_config(&[min, max], &options, 60, 10, Style::new());
                let ticks = config.x_axis.ticks.as_ref().unwrap();
                assert_eq!(ticks.first(), Some(&min));
                assert_eq!(ticks.last(), Some(&max));
                assert_eq!(ticks.len(), config.x_axis.labels.as_ref().unwrap().len());
                let positions = build_x_tick_col_map(
                    ticks,
                    config.x_axis.labels.as_deref(),
                    true,
                    min,
                    max,
                    91,
                );
                for ((col, _), tick) in positions.iter().zip(ticks) {
                    assert_eq!(*col, ((tick - min) / (max - min) * 90.0).round() as usize);
                }
            }
        }
    }

    #[test]
    fn histogram_linear_preparation_matches_sorted_counting() {
        let inputs = [
            vec![],
            vec![f64::NAN, f64::INFINITY],
            vec![2.0; 32],
            vec![-f64::MAX, f64::MAX, 0.0, -0.0, f64::NAN],
            (0..1000).map(|i| ((i * 7919) % 997) as f64 / 7.0).collect(),
        ];
        for data in inputs {
            for bins in [None, Some(0), Some(1), Some(9), Some(11)] {
                let options = HistogramBuilder {
                    bins,
                    ..Default::default()
                };
                let config = build_histogram_config(&data, &options, 60, 10, Style::new());
                let mut sorted: Vec<_> = data.iter().copied().filter(|v| v.is_finite()).collect();
                sorted.sort_by(f64::total_cmp);
                if sorted.is_empty() {
                    assert!(config.datasets.is_empty());
                    continue;
                }
                let (min, max) = normalize_bounds(sorted[0], *sorted.last().unwrap());
                assert_eq!(config.x_axis.bounds, Some((min, max)));
                let count = bins
                    .unwrap_or_else(|| sturges_bin_count(sorted.len()))
                    .max(1);
                let mut expected = vec![0.0; count];
                for value in sorted {
                    let index = ((finite_ratio(value, min, max).unwrap() * count as f64).floor()
                        as usize)
                        .min(count - 1);
                    expected[index] += 1.0;
                }
                assert_eq!(
                    config.datasets[0]
                        .data
                        .iter()
                        .map(|point| point.1)
                        .collect::<Vec<_>>(),
                    expected
                );
                assert!(config.x_axis.ticks.unwrap().iter().all(|v| v.is_finite()));
            }
        }
    }

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
