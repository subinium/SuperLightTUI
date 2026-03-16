use super::*;

pub(super) fn draw_braille_dataset(
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
