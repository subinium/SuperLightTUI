//! Heatmap (full-cell and half-block) and squarified treemap widgets.

use super::*;

impl Context {
    /// Render a heatmap from a 2D data grid.
    ///
    /// Each cell maps to a block character with color intensity:
    /// low values -> dim/dark, high values -> bright/saturated.
    ///
    /// # Arguments
    /// * `data` - Row-major 2D grid (outer = rows, inner = columns)
    /// * `width` - Widget width in terminal cells
    /// * `height` - Widget height in terminal cells
    /// * `low_color` - Color for minimum values
    /// * `high_color` - Color for maximum values
    pub fn heatmap(
        &mut self,
        data: &[Vec<f64>],
        width: u32,
        height: u32,
        low_color: Color,
        high_color: Color,
    ) -> Response {
        if data.is_empty() || width == 0 || height == 0 {
            return Response::none();
        }

        let data_rows = data.len();
        let max_data_cols = data.iter().map(Vec::len).max().unwrap_or(0);
        if max_data_cols == 0 {
            return Response::none();
        }

        let mut min_value = f64::INFINITY;
        let mut max_value = f64::NEG_INFINITY;
        for row in data {
            for value in row {
                if value.is_finite() {
                    min_value = min_value.min(*value);
                    max_value = max_value.max(*value);
                }
            }
        }

        if !min_value.is_finite() || !max_value.is_finite() {
            return Response::none();
        }

        let zero_range = min_value == max_value;
        let cols = width as usize;
        let rows = height as usize;

        let response = self.begin_sized_viz_root(width, height);
        for row_idx in 0..rows {
            let data_row_idx = (row_idx * data_rows / rows).min(data_rows.saturating_sub(1));
            let source_row = &data[data_row_idx];
            let source_cols = source_row.len();

            self.skip_interaction_slot();
            self.commands
                .push(Command::BeginContainer(BeginContainerArgs {
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
                }));

            let mut segment = String::new();
            let mut segment_color: Option<Color> = None;

            for col_idx in 0..cols {
                let normalized = if source_cols == 0 {
                    0.0
                } else {
                    let data_col_idx = (col_idx * source_cols / cols).min(source_cols - 1);
                    let value = source_row[data_col_idx];

                    if !value.is_finite() {
                        0.0
                    } else if zero_range {
                        0.5
                    } else {
                        unit_ratio(value, min_value, max_value)
                    }
                };

                let color = blend_color(low_color, high_color, normalized);

                match segment_color {
                    Some(current) if current == color => {
                        segment.push('█');
                    }
                    Some(current) => {
                        self.styled(std::mem::take(&mut segment), Style::new().fg(current));
                        segment.push('█');
                        segment_color = Some(color);
                    }
                    None => {
                        segment.push('█');
                        segment_color = Some(color);
                    }
                }
            }

            if let Some(color) = segment_color {
                self.styled(segment, Style::new().fg(color));
            }

            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }
        self.end_viz_root();

        response
    }

    /// Render a heatmap using half-block characters for 2× vertical resolution.
    ///
    /// Each terminal cell packs two data rows using `▀` with `fg` for the upper
    /// half and `bg` for the lower half. This doubles the effective vertical
    /// resolution compared to [`heatmap`](Self::heatmap).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// let data: Vec<Vec<f64>> = (0..20)
    ///     .map(|r| (0..40).map(|c| ((r * 3 + c * 7) % 20) as f64).collect())
    ///     .collect();
    /// ui.heatmap_halfblock(&data, 40, 10, Color::Rgb(10, 10, 40), Color::Rgb(255, 80, 30));
    /// # });
    /// ```
    pub fn heatmap_halfblock(
        &mut self,
        data: &[Vec<f64>],
        width: u32,
        height: u32,
        low_color: Color,
        high_color: Color,
    ) -> Response {
        if data.is_empty() || width == 0 || height == 0 {
            return Response::none();
        }

        let data_rows = data.len();
        let max_data_cols = data.iter().map(Vec::len).max().unwrap_or(0);
        if max_data_cols == 0 {
            return Response::none();
        }

        let mut min_value = f64::INFINITY;
        let mut max_value = f64::NEG_INFINITY;
        for row in data {
            for value in row {
                if value.is_finite() {
                    min_value = min_value.min(*value);
                    max_value = max_value.max(*value);
                }
            }
        }

        if !min_value.is_finite() || !max_value.is_finite() {
            return Response::none();
        }

        let zero_range = min_value == max_value;

        let data = data.to_vec();
        let cols = width as usize;
        let rows = height as usize;
        // Each terminal row maps to 2 data rows
        let virtual_rows = rows * 2;

        let response = self.interaction();
        self.container().w(width).h(height).draw(move |buf, rect| {
            let w = rect.width as usize;
            let h = rect.height as usize;
            if w == 0 || h == 0 {
                return;
            }

            let sample = |data_row_idx: usize, col_idx: usize| -> f64 {
                let src_row = &data[data_row_idx.min(data_rows.saturating_sub(1))];
                let src_cols = src_row.len();
                if src_cols == 0 {
                    return 0.0;
                }
                let data_col = (col_idx * src_cols / cols.max(1)).min(src_cols - 1);
                let v = src_row[data_col];
                if !v.is_finite() {
                    0.0
                } else if zero_range {
                    0.5
                } else {
                    unit_ratio(v, min_value, max_value)
                }
            };

            for row in 0..h {
                let upper_data_row =
                    (row * 2 * data_rows / virtual_rows).min(data_rows.saturating_sub(1));
                let lower_data_row =
                    ((row * 2 + 1) * data_rows / virtual_rows).min(data_rows.saturating_sub(1));

                for col in 0..w.min(cols) {
                    let upper_t = sample(upper_data_row, col);
                    let lower_t = sample(lower_data_row, col);
                    let upper_color = blend_color(low_color, high_color, upper_t);
                    let lower_color = blend_color(low_color, high_color, lower_t);

                    buf.set_char(
                        rect.x + col as u32,
                        rect.y + row as u32,
                        '▀',
                        Style::new().fg(upper_color).bg(lower_color),
                    );
                }
            }
        });

        response
    }

    /// Render a treemap using the squarified layout algorithm.
    ///
    /// Each item occupies a rectangle proportional to its value, filled with the
    /// item's color and labeled when space permits.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::{TreemapItem, Color};
    /// let items = vec![
    ///     TreemapItem::new("Rust", 40.0, Color::Cyan),
    ///     TreemapItem::new("Go", 25.0, Color::Blue),
    ///     TreemapItem::new("Python", 20.0, Color::Yellow),
    ///     TreemapItem::new("Java", 15.0, Color::Red),
    /// ];
    /// ui.treemap(&items);
    /// # });
    /// ```
    pub fn treemap(&mut self, items: &[TreemapItem]) -> Response {
        if items.is_empty() {
            return Response::none();
        }

        let items: Vec<TreemapItem> = items
            .iter()
            .filter(|item| item.value.is_finite() && item.value > 0.0)
            .cloned()
            .collect();
        if items.is_empty() {
            return Response::none();
        }
        let response = self.interaction();
        self.container().grow(1).draw(move |buf, rect| {
            let w = rect.width as usize;
            let h = rect.height as usize;
            if w < 2 || h < 2 {
                return;
            }

            // Filter out items that would be too small to render (< 1 cell)
            let total_area = w as f64 * h as f64;
            let max_value = items.iter().map(|item| item.value).fold(0.0, f64::max);
            let total_value: f64 = items.iter().map(|item| item.value / max_value).sum();
            let min_area_threshold = 1.0; // at least 1 cell
            let visible_items: Vec<&TreemapItem> = if total_value > 0.0 {
                items
                    .iter()
                    .filter(|item| {
                        (item.value / max_value) / total_value * total_area >= min_area_threshold
                    })
                    .collect()
            } else {
                return;
            };

            if visible_items.is_empty() {
                return;
            }

            // Build filtered items for layout
            let filtered: Vec<TreemapItem> = visible_items.into_iter().cloned().collect();
            let rects = squarify_layout(&filtered, 0.0, 0.0, w as f64, h as f64);

            for (item, r) in filtered.iter().zip(rects.iter()) {
                // Integer cell bounds — use round for consistent placement
                let x0 = r.x.round() as usize;
                let y0 = r.y.round() as usize;
                let x1 = (r.x + r.w).round() as usize;
                let y1 = (r.y + r.h).round() as usize;

                let cell_w = x1.min(w).saturating_sub(x0);
                let cell_h = y1.min(h).saturating_sub(y0);
                if cell_w == 0 || cell_h == 0 {
                    continue;
                }

                // Fill the rectangle with the item's color
                for row in y0..y1.min(h) {
                    for col in x0..x1.min(w) {
                        buf.set_char(
                            rect.x + col as u32,
                            rect.y + row as u32,
                            ' ',
                            Style::new().bg(item.color),
                        );
                    }
                }

                let text_color = treemap_label_color(item.color);

                // Label: truncate to fit with ellipsis, center in cell
                // (unicode-safe, fix #112; ellipsis fix for fix #v020-truncate)
                if cell_w >= 2 {
                    let max_label_w = cell_w.saturating_sub(1);
                    let label_owned = crate::chart::truncate_label(&item.label, max_label_w);
                    let label = label_owned.as_str();
                    let label_unicode_w = crate::width::unicode_width(label);
                    let label_y = y0 + cell_h / 2;
                    let label_x = x0 + (cell_w.saturating_sub(label_unicode_w)) / 2;
                    if label_y < y1.min(h) {
                        buf.set_string(
                            rect.x + label_x as u32,
                            rect.y + label_y as u32,
                            label,
                            Style::new().fg(text_color).bg(item.color).bold(),
                        );
                    }

                    // Value label below if space permits
                    if cell_h >= 3 {
                        let value_str = format_compact_number(item.value);
                        let value_y = label_y + 1;
                        let value_width = crate::width::unicode_width(value_str.as_str());
                        if value_y < y1.min(h) && value_width < cell_w {
                            let vx = x0 + (cell_w.saturating_sub(value_width)) / 2;
                            buf.set_string(
                                rect.x + vx as u32,
                                rect.y + value_y as u32,
                                &value_str,
                                Style::new().fg(text_color).bg(item.color).dim(),
                            );
                        }
                    }
                }
            }
        });

        response
    }
}

/// A single item in a treemap.
#[derive(Debug, Clone)]
pub struct TreemapItem {
    /// Display label.
    pub label: String,
    /// Numeric value determining area.
    pub value: f64,
    /// Fill color for this item's rectangle.
    pub color: Color,
}

impl TreemapItem {
    /// Create a new treemap item.
    pub fn new(label: impl Into<String>, value: f64, color: Color) -> Self {
        Self {
            label: label.into(),
            value,
            color,
        }
    }
}

/// Rectangle produced by the squarified layout.
#[derive(Clone)]
struct LayoutRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Squarified treemap layout algorithm (Bruls, Huizing, van Wijk 2000).
fn squarify_layout(items: &[TreemapItem], x: f64, y: f64, w: f64, h: f64) -> Vec<LayoutRect> {
    if items.is_empty()
        || !x.is_finite()
        || !y.is_finite()
        || !w.is_finite()
        || !h.is_finite()
        || w <= 0.0
        || h <= 0.0
    {
        return Vec::new();
    }

    let max_value = items
        .iter()
        .map(|item| item.value)
        .filter(|value| value.is_finite() && *value > 0.0)
        .fold(0.0, f64::max);
    if max_value <= 0.0 {
        return items
            .iter()
            .map(|_| LayoutRect {
                x,
                y,
                w: 0.0,
                h: 0.0,
            })
            .collect();
    }
    let total: f64 = items
        .iter()
        .map(|item| {
            if item.value.is_finite() && item.value > 0.0 {
                item.value / max_value
            } else {
                0.0
            }
        })
        .sum();

    // Normalize values to fill the available area
    let area = w * h;
    let mut sorted_indices: Vec<usize> = (0..items.len()).collect();
    sorted_indices.sort_by(|a, b| items[*b].value.total_cmp(&items[*a].value));

    let areas: Vec<f64> = sorted_indices
        .iter()
        .map(|&i| {
            let value = if items[i].value.is_finite() && items[i].value > 0.0 {
                items[i].value / max_value
            } else {
                0.0
            };
            value / total * area
        })
        .collect();

    let mut result = vec![
        LayoutRect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        };
        items.len()
    ];
    squarify_recursive(&areas, &sorted_indices, x, y, w, h, &mut result);
    result
}

/// Incremental form of [`worst_ratio`] that uses pre-aggregated row statistics.
///
/// `sum` is the total of all areas in the row (including non-positive ones, to
/// match `row.iter().sum()`). `pos_max` and `pos_min` are the max/min over
/// strictly positive areas only — set them to `f64::NEG_INFINITY` / `f64::INFINITY`
/// when there are no positives. `pos_count` is the number of strictly positive areas.
///
/// Returns the same value as `worst_ratio` for any row whose sum/min/max/count
/// match, since `worst_ratio`'s loop reduces to `max(side²·max / sum², sum² / (side²·min))`
/// when all summed areas are positive, and to `0.0` when none are.
#[inline]
fn worst_ratio_incremental(
    sum: f64,
    pos_max: f64,
    pos_min: f64,
    pos_count: usize,
    side: f64,
) -> f64 {
    if side <= 0.0 {
        return f64::INFINITY;
    }
    if pos_count == 0 {
        return 0.0;
    }
    let s2 = side * side;
    let sum2 = sum * sum;
    // sum2 > 0 here because pos_count > 0 implies sum > 0.
    (s2 * pos_max / sum2).max(sum2 / (s2 * pos_min))
}

fn squarify_recursive(
    areas: &[f64],
    indices: &[usize],
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    result: &mut [LayoutRect],
) {
    if areas.is_empty() || w <= 0.0 || h <= 0.0 {
        return;
    }

    if areas.len() == 1 {
        result[indices[0]] = LayoutRect { x, y, w, h };
        return;
    }

    let short_side = w.min(h);
    let mut row: Vec<f64> = Vec::new();
    let mut row_indices: Vec<usize> = Vec::new();
    // Incremental row statistics avoid cloning `row` each iteration just to
    // probe the ratio of `row + [area]`. `pos_*` track positive-only stats to
    // match `worst_ratio`'s zero-skip behavior exactly.
    let mut row_sum_acc = 0f64;
    let mut row_pos_max = f64::NEG_INFINITY;
    let mut row_pos_min = f64::INFINITY;
    let mut row_pos_count: usize = 0;

    for (i, &area) in areas.iter().enumerate() {
        let cand_sum = row_sum_acc + area;
        let (cand_pos_max, cand_pos_min, cand_pos_count) = if area > 0.0 {
            (
                row_pos_max.max(area),
                row_pos_min.min(area),
                row_pos_count + 1,
            )
        } else {
            (row_pos_max, row_pos_min, row_pos_count)
        };

        let candidate_ratio = worst_ratio_incremental(
            cand_sum,
            cand_pos_max,
            cand_pos_min,
            cand_pos_count,
            short_side,
        );
        let current_ratio = worst_ratio_incremental(
            row_sum_acc,
            row_pos_max,
            row_pos_min,
            row_pos_count,
            short_side,
        );
        if row.is_empty() || candidate_ratio <= current_ratio {
            row.push(area);
            row_indices.push(indices[i]);
            row_sum_acc = cand_sum;
            row_pos_max = cand_pos_max;
            row_pos_min = cand_pos_min;
            row_pos_count = cand_pos_count;
        } else {
            // Layout the current row
            let row_sum: f64 = row.iter().sum();
            let row_fraction = row_sum / (w * h).max(f64::EPSILON);

            if w >= h {
                // Lay out vertically on the left
                let row_w = w * row_fraction;
                let mut cy = y;
                for (j, &a) in row.iter().enumerate() {
                    let cell_h = if row_sum > 0.0 {
                        h * (a / row_sum)
                    } else {
                        0.0
                    };
                    result[row_indices[j]] = LayoutRect {
                        x,
                        y: cy,
                        w: row_w,
                        h: cell_h,
                    };
                    cy += cell_h;
                }
                squarify_recursive(
                    &areas[i..],
                    &indices[i..],
                    x + row_w,
                    y,
                    w - row_w,
                    h,
                    result,
                );
            } else {
                // Lay out horizontally on top
                let row_h = h * row_fraction;
                let mut cx = x;
                for (j, &a) in row.iter().enumerate() {
                    let cell_w = if row_sum > 0.0 {
                        w * (a / row_sum)
                    } else {
                        0.0
                    };
                    result[row_indices[j]] = LayoutRect {
                        x: cx,
                        y,
                        w: cell_w,
                        h: row_h,
                    };
                    cx += cell_w;
                }
                squarify_recursive(
                    &areas[i..],
                    &indices[i..],
                    x,
                    y + row_h,
                    w,
                    h - row_h,
                    result,
                );
            }
            return;
        }
    }

    // Layout remaining row
    if !row.is_empty() {
        let row_sum: f64 = row.iter().sum();
        if w >= h {
            let mut cy = y;
            for (j, &a) in row.iter().enumerate() {
                let cell_h = if row_sum > 0.0 {
                    h * (a / row_sum)
                } else {
                    0.0
                };
                result[row_indices[j]] = LayoutRect {
                    x,
                    y: cy,
                    w,
                    h: cell_h,
                };
                cy += cell_h;
            }
        } else {
            let mut cx = x;
            for (j, &a) in row.iter().enumerate() {
                let cell_w = if row_sum > 0.0 {
                    w * (a / row_sum)
                } else {
                    0.0
                };
                result[row_indices[j]] = LayoutRect {
                    x: cx,
                    y,
                    w: cell_w,
                    h,
                };
                cx += cell_w;
            }
        }
    }
}

/// Linearly interpolate between two RGB colors.
///
/// Both colors must be `Color::Rgb` for true interpolation; mixed/named inputs
/// fall back to a binary threshold at `t = 0.5`. `t` is clamped to `[0.0, 1.0]`.
#[inline]
fn blend_color(a: Color, b: Color, t: f64) -> Color {
    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => Color::Rgb(
            (r1 as f64 * (1.0 - t) + r2 as f64 * t).round() as u8,
            (g1 as f64 * (1.0 - t) + g2 as f64 * t).round() as u8,
            (b1 as f64 * (1.0 - t) + b2 as f64 * t).round() as u8,
        ),
        _ => {
            if t > 0.5 {
                b
            } else {
                a
            }
        }
    }
}

/// Choose a contrasting label color for treemap cells.
fn treemap_label_color(bg: Color) -> Color {
    Color::contrast_fg(bg)
}

#[test]
fn treemap_named_indexed_and_rgb_colors_choose_contrast() {
    assert_eq!(treemap_label_color(Color::Black), Color::Rgb(255, 255, 255));
    assert_eq!(treemap_label_color(Color::White), Color::Rgb(0, 0, 0));
    assert_eq!(
        treemap_label_color(Color::Indexed(231)),
        Color::Rgb(0, 0, 0)
    );
    assert_eq!(
        treemap_label_color(Color::Rgb(10, 20, 30)),
        Color::Rgb(255, 255, 255)
    );
}
