use super::*;
use unicode_width::UnicodeWidthStr;

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
