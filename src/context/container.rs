use super::*;

/// Fluent builder for configuring containers before calling `.col()` or `.row()`.
///
/// Obtain one via [`Context::container`] or [`Context::bordered`]. Chain the
/// configuration methods you need, then finalize with `.col(|ui| { ... })` or
/// `.row(|ui| { ... })`.
///
/// # Example
///
/// ```no_run
/// # slt::run(|ui: &mut slt::Context| {
/// use slt::{Border, Color};
/// ui.container()
///     .border(Border::Rounded)
///     .pad(1)
///     .grow(1)
///     .col(|ui| {
///         ui.text("inside a bordered, padded, growing column");
///     });
/// # });
/// ```
#[must_use = "ContainerBuilder does nothing until .col(), .row(), .line(), or .draw() is called"]
pub struct ContainerBuilder<'a> {
    pub(crate) ctx: &'a mut Context,
    pub(crate) gap: u32,
    pub(crate) row_gap: Option<u32>,
    pub(crate) col_gap: Option<u32>,
    pub(crate) align: Align,
    pub(crate) align_self_value: Option<Align>,
    pub(crate) justify: Justify,
    pub(crate) border: Option<Border>,
    pub(crate) border_sides: BorderSides,
    pub(crate) border_style: Style,
    pub(crate) bg: Option<Color>,
    pub(crate) text_color: Option<Color>,
    pub(crate) dark_bg: Option<Color>,
    pub(crate) dark_border_style: Option<Style>,
    pub(crate) group_hover_bg: Option<Color>,
    pub(crate) group_hover_border_style: Option<Style>,
    pub(crate) group_name: Option<String>,
    pub(crate) padding: Padding,
    pub(crate) margin: Margin,
    pub(crate) constraints: Constraints,
    pub(crate) title: Option<(String, Style)>,
    pub(crate) grow: u16,
    pub(crate) scroll_offset: Option<u32>,
}

/// Drawing context for the [`Context::canvas`] widget.
///
/// Provides pixel-level drawing on a braille character grid. Each terminal
/// cell maps to a 2x4 dot matrix, so a canvas of `width` columns x `height`
/// rows gives `width*2` x `height*4` pixel resolution.
/// A colored pixel in the canvas grid.
#[derive(Debug, Clone, Copy)]
struct CanvasPixel {
    bits: u32,
    color: Color,
}

/// Text label placed on the canvas.
#[derive(Debug, Clone)]
struct CanvasLabel {
    x: usize,
    y: usize,
    text: String,
    color: Color,
}

/// A layer in the canvas, supporting z-ordering.
#[derive(Debug, Clone)]
struct CanvasLayer {
    grid: Vec<Vec<CanvasPixel>>,
    labels: Vec<CanvasLabel>,
}

/// Drawing context for the canvas widget.
pub struct CanvasContext {
    layers: Vec<CanvasLayer>,
    cols: usize,
    rows: usize,
    px_w: usize,
    px_h: usize,
    current_color: Color,
}

impl CanvasContext {
    pub(crate) fn new(cols: usize, rows: usize) -> Self {
        Self {
            layers: vec![Self::new_layer(cols, rows)],
            cols,
            rows,
            px_w: cols * 2,
            px_h: rows * 4,
            current_color: Color::Reset,
        }
    }

    fn new_layer(cols: usize, rows: usize) -> CanvasLayer {
        CanvasLayer {
            grid: vec![
                vec![
                    CanvasPixel {
                        bits: 0,
                        color: Color::Reset,
                    };
                    cols
                ];
                rows
            ],
            labels: Vec::new(),
        }
    }

    fn current_layer_mut(&mut self) -> Option<&mut CanvasLayer> {
        self.layers.last_mut()
    }

    fn dot_with_color(&mut self, x: usize, y: usize, color: Color) {
        if x >= self.px_w || y >= self.px_h {
            return;
        }

        let char_col = x / 2;
        let char_row = y / 4;
        let sub_col = x % 2;
        let sub_row = y % 4;
        const LEFT_BITS: [u32; 4] = [0x01, 0x02, 0x04, 0x40];
        const RIGHT_BITS: [u32; 4] = [0x08, 0x10, 0x20, 0x80];

        let bit = if sub_col == 0 {
            LEFT_BITS[sub_row]
        } else {
            RIGHT_BITS[sub_row]
        };

        if let Some(layer) = self.current_layer_mut() {
            let cell = &mut layer.grid[char_row][char_col];
            let new_bits = cell.bits | bit;
            if new_bits != cell.bits {
                cell.bits = new_bits;
                cell.color = color;
            }
        }
    }

    fn dot_isize(&mut self, x: isize, y: isize) {
        if x >= 0 && y >= 0 {
            self.dot(x as usize, y as usize);
        }
    }

    /// Get the pixel width of the canvas.
    pub fn width(&self) -> usize {
        self.px_w
    }

    /// Get the pixel height of the canvas.
    pub fn height(&self) -> usize {
        self.px_h
    }

    /// Set a single pixel at `(x, y)`.
    pub fn dot(&mut self, x: usize, y: usize) {
        self.dot_with_color(x, y, self.current_color);
    }

    /// Draw a line from `(x0, y0)` to `(x1, y1)` using Bresenham's algorithm.
    pub fn line(&mut self, x0: usize, y0: usize, x1: usize, y1: usize) {
        let (mut x, mut y) = (x0 as isize, y0 as isize);
        let (x1, y1) = (x1 as isize, y1 as isize);
        let dx = (x1 - x).abs();
        let dy = -(y1 - y).abs();
        let sx = if x < x1 { 1 } else { -1 };
        let sy = if y < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.dot_isize(x, y);
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

    /// Draw a rectangle outline from `(x, y)` with `w` width and `h` height.
    pub fn rect(&mut self, x: usize, y: usize, w: usize, h: usize) {
        if w == 0 || h == 0 {
            return;
        }

        self.line(x, y, x + w.saturating_sub(1), y);
        self.line(
            x + w.saturating_sub(1),
            y,
            x + w.saturating_sub(1),
            y + h.saturating_sub(1),
        );
        self.line(
            x + w.saturating_sub(1),
            y + h.saturating_sub(1),
            x,
            y + h.saturating_sub(1),
        );
        self.line(x, y + h.saturating_sub(1), x, y);
    }

    /// Draw a circle outline centered at `(cx, cy)` with radius `r`.
    pub fn circle(&mut self, cx: usize, cy: usize, r: usize) {
        let mut x = r as isize;
        let mut y: isize = 0;
        let mut err: isize = 1 - x;
        let (cx, cy) = (cx as isize, cy as isize);

        while x >= y {
            for &(dx, dy) in &[
                (x, y),
                (y, x),
                (-x, y),
                (-y, x),
                (x, -y),
                (y, -x),
                (-x, -y),
                (-y, -x),
            ] {
                let px = cx + dx;
                let py = cy + dy;
                self.dot_isize(px, py);
            }

            y += 1;
            if err < 0 {
                err += 2 * y + 1;
            } else {
                x -= 1;
                err += 2 * (y - x) + 1;
            }
        }
    }

    /// Set the drawing color for subsequent shapes.
    pub fn set_color(&mut self, color: Color) {
        self.current_color = color;
    }

    /// Get the current drawing color.
    pub fn color(&self) -> Color {
        self.current_color
    }

    /// Draw a filled rectangle.
    pub fn filled_rect(&mut self, x: usize, y: usize, w: usize, h: usize) {
        if w == 0 || h == 0 {
            return;
        }

        let x_end = x.saturating_add(w).min(self.px_w);
        let y_end = y.saturating_add(h).min(self.px_h);
        if x >= x_end || y >= y_end {
            return;
        }

        for yy in y..y_end {
            self.line(x, yy, x_end.saturating_sub(1), yy);
        }
    }

    /// Draw a filled circle.
    pub fn filled_circle(&mut self, cx: usize, cy: usize, r: usize) {
        let (cx, cy, r) = (cx as isize, cy as isize, r as isize);
        for y in (cy - r)..=(cy + r) {
            let dy = y - cy;
            let span_sq = (r * r - dy * dy).max(0);
            let dx = (span_sq as f64).sqrt() as isize;
            for x in (cx - dx)..=(cx + dx) {
                self.dot_isize(x, y);
            }
        }
    }

    /// Draw a triangle outline.
    pub fn triangle(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, x2: usize, y2: usize) {
        self.line(x0, y0, x1, y1);
        self.line(x1, y1, x2, y2);
        self.line(x2, y2, x0, y0);
    }

    /// Draw a filled triangle.
    pub fn filled_triangle(
        &mut self,
        x0: usize,
        y0: usize,
        x1: usize,
        y1: usize,
        x2: usize,
        y2: usize,
    ) {
        let vertices = [
            (x0 as isize, y0 as isize),
            (x1 as isize, y1 as isize),
            (x2 as isize, y2 as isize),
        ];
        let min_y = vertices.iter().map(|(_, y)| *y).min().unwrap_or(0);
        let max_y = vertices.iter().map(|(_, y)| *y).max().unwrap_or(-1);

        for y in min_y..=max_y {
            let mut intersections: Vec<f64> = Vec::new();

            for edge in [(0usize, 1usize), (1usize, 2usize), (2usize, 0usize)] {
                let (x_a, y_a) = vertices[edge.0];
                let (x_b, y_b) = vertices[edge.1];
                if y_a == y_b {
                    continue;
                }

                let (x_start, y_start, x_end, y_end) = if y_a < y_b {
                    (x_a, y_a, x_b, y_b)
                } else {
                    (x_b, y_b, x_a, y_a)
                };

                if y < y_start || y >= y_end {
                    continue;
                }

                let t = (y - y_start) as f64 / (y_end - y_start) as f64;
                intersections.push(x_start as f64 + t * (x_end - x_start) as f64);
            }

            intersections.sort_by(|a, b| a.total_cmp(b));
            let mut i = 0usize;
            while i + 1 < intersections.len() {
                let x_start = intersections[i].ceil() as isize;
                let x_end = intersections[i + 1].floor() as isize;
                for x in x_start..=x_end {
                    self.dot_isize(x, y);
                }
                i += 2;
            }
        }

        self.triangle(x0, y0, x1, y1, x2, y2);
    }

    /// Draw multiple points at once.
    pub fn points(&mut self, pts: &[(usize, usize)]) {
        for &(x, y) in pts {
            self.dot(x, y);
        }
    }

    /// Draw a polyline connecting the given points in order.
    pub fn polyline(&mut self, pts: &[(usize, usize)]) {
        for window in pts.windows(2) {
            if let [(x0, y0), (x1, y1)] = window {
                self.line(*x0, *y0, *x1, *y1);
            }
        }
    }

    /// Place a text label at pixel position `(x, y)`.
    /// Text is rendered in regular characters overlaying the braille grid.
    pub fn print(&mut self, x: usize, y: usize, text: &str) {
        if text.is_empty() {
            return;
        }

        let color = self.current_color;
        if let Some(layer) = self.current_layer_mut() {
            layer.labels.push(CanvasLabel {
                x,
                y,
                text: text.to_string(),
                color,
            });
        }
    }

    /// Start a new drawing layer. Shapes on later layers overlay earlier ones.
    pub fn layer(&mut self) {
        self.layers.push(Self::new_layer(self.cols, self.rows));
    }

    pub(crate) fn render(&self) -> Vec<Vec<(String, Color)>> {
        let mut final_grid = vec![
            vec![
                CanvasPixel {
                    bits: 0,
                    color: Color::Reset,
                };
                self.cols
            ];
            self.rows
        ];
        let mut labels_overlay: Vec<Vec<Option<(char, Color)>>> =
            vec![vec![None; self.cols]; self.rows];

        for layer in &self.layers {
            for (row, final_row) in final_grid.iter_mut().enumerate().take(self.rows) {
                for (col, dst) in final_row.iter_mut().enumerate().take(self.cols) {
                    let src = layer.grid[row][col];
                    if src.bits == 0 {
                        continue;
                    }

                    let merged = dst.bits | src.bits;
                    if merged != dst.bits {
                        dst.bits = merged;
                        dst.color = src.color;
                    }
                }
            }

            for label in &layer.labels {
                let row = label.y / 4;
                if row >= self.rows {
                    continue;
                }
                let start_col = label.x / 2;
                for (offset, ch) in label.text.chars().enumerate() {
                    let col = start_col + offset;
                    if col >= self.cols {
                        break;
                    }
                    labels_overlay[row][col] = Some((ch, label.color));
                }
            }
        }

        let mut lines: Vec<Vec<(String, Color)>> = Vec::with_capacity(self.rows);
        for row in 0..self.rows {
            let mut segments: Vec<(String, Color)> = Vec::new();
            let mut current_color: Option<Color> = None;
            let mut current_text = String::new();

            for col in 0..self.cols {
                let (ch, color) = if let Some((label_ch, label_color)) = labels_overlay[row][col] {
                    (label_ch, label_color)
                } else {
                    let bits = final_grid[row][col].bits;
                    let ch = char::from_u32(0x2800 + bits).unwrap_or(' ');
                    (ch, final_grid[row][col].color)
                };

                match current_color {
                    Some(c) if c == color => {
                        current_text.push(ch);
                    }
                    Some(c) => {
                        segments.push((std::mem::take(&mut current_text), c));
                        current_text.push(ch);
                        current_color = Some(color);
                    }
                    None => {
                        current_text.push(ch);
                        current_color = Some(color);
                    }
                }
            }

            if let Some(color) = current_color {
                segments.push((current_text, color));
            }
            lines.push(segments);
        }

        lines
    }
}

macro_rules! define_breakpoint_methods {
    (
        base = $base:ident,
        arg = $arg:ident : $arg_ty:ty,
        xs = $xs_fn:ident => [$( $xs_doc:literal ),* $(,)?],
        sm = $sm_fn:ident => [$( $sm_doc:literal ),* $(,)?],
        md = $md_fn:ident => [$( $md_doc:literal ),* $(,)?],
        lg = $lg_fn:ident => [$( $lg_doc:literal ),* $(,)?],
        xl = $xl_fn:ident => [$( $xl_doc:literal ),* $(,)?],
        at = $at_fn:ident => [$( $at_doc:literal ),* $(,)?]
    ) => {
        $(#[doc = $xs_doc])*
        pub fn $xs_fn(self, $arg: $arg_ty) -> Self {
            if self.ctx.breakpoint() == Breakpoint::Xs {
                self.$base($arg)
            } else {
                self
            }
        }

        $(#[doc = $sm_doc])*
        pub fn $sm_fn(self, $arg: $arg_ty) -> Self {
            if self.ctx.breakpoint() == Breakpoint::Sm {
                self.$base($arg)
            } else {
                self
            }
        }

        $(#[doc = $md_doc])*
        pub fn $md_fn(self, $arg: $arg_ty) -> Self {
            if self.ctx.breakpoint() == Breakpoint::Md {
                self.$base($arg)
            } else {
                self
            }
        }

        $(#[doc = $lg_doc])*
        pub fn $lg_fn(self, $arg: $arg_ty) -> Self {
            if self.ctx.breakpoint() == Breakpoint::Lg {
                self.$base($arg)
            } else {
                self
            }
        }

        $(#[doc = $xl_doc])*
        pub fn $xl_fn(self, $arg: $arg_ty) -> Self {
            if self.ctx.breakpoint() == Breakpoint::Xl {
                self.$base($arg)
            } else {
                self
            }
        }

        $(#[doc = $at_doc])*
        pub fn $at_fn(self, bp: Breakpoint, $arg: $arg_ty) -> Self {
            if self.ctx.breakpoint() == bp {
                self.$base($arg)
            } else {
                self
            }
        }
    };
}

impl<'a> ContainerBuilder<'a> {
    // ── border ───────────────────────────────────────────────────────

    /// Apply a reusable [`ContainerStyle`] recipe. Only set fields override
    /// the builder's current values. Chain multiple `.apply()` calls to compose.
    ///
    /// If the style has an [`ContainerStyle::extends`] base, the base is applied
    /// first, then the style's own fields override.
    ///
    /// [`ThemeColor`] fields (`theme_bg`, `theme_text_color`, `theme_border_fg`)
    /// are resolved against the active theme at apply time.
    pub fn apply(mut self, style: &ContainerStyle) -> Self {
        // Apply base style first if this style extends another
        if let Some(base) = style.extends {
            self = self.apply(base);
        }
        if let Some(v) = style.border {
            self.border = Some(v);
        }
        if let Some(v) = style.border_sides {
            self.border_sides = v;
        }
        if let Some(v) = style.border_style {
            self.border_style = v;
        }
        if let Some(v) = style.bg {
            self.bg = Some(v);
        }
        if let Some(v) = style.dark_bg {
            self.dark_bg = Some(v);
        }
        if let Some(v) = style.dark_border_style {
            self.dark_border_style = Some(v);
        }
        if let Some(v) = style.padding {
            self.padding = v;
        }
        if let Some(v) = style.margin {
            self.margin = v;
        }
        if let Some(v) = style.gap {
            self.gap = v;
        }
        if let Some(v) = style.row_gap {
            self.row_gap = Some(v);
        }
        if let Some(v) = style.col_gap {
            self.col_gap = Some(v);
        }
        if let Some(v) = style.grow {
            self.grow = v;
        }
        if let Some(v) = style.align {
            self.align = v;
        }
        if let Some(v) = style.align_self {
            self.align_self_value = Some(v);
        }
        if let Some(v) = style.justify {
            self.justify = v;
        }
        if let Some(v) = style.text_color {
            self.text_color = Some(v);
        }
        if let Some(w) = style.w {
            self.constraints.min_width = Some(w);
            self.constraints.max_width = Some(w);
        }
        if let Some(h) = style.h {
            self.constraints.min_height = Some(h);
            self.constraints.max_height = Some(h);
        }
        if let Some(v) = style.min_w {
            self.constraints.min_width = Some(v);
        }
        if let Some(v) = style.max_w {
            self.constraints.max_width = Some(v);
        }
        if let Some(v) = style.min_h {
            self.constraints.min_height = Some(v);
        }
        if let Some(v) = style.max_h {
            self.constraints.max_height = Some(v);
        }
        if let Some(v) = style.w_pct {
            self.constraints.width_pct = Some(v);
        }
        if let Some(v) = style.h_pct {
            self.constraints.height_pct = Some(v);
        }
        // Resolve ThemeColor fields against the active theme (overrides literal colors)
        if let Some(tc) = style.theme_bg {
            self.bg = Some(self.ctx.theme.resolve(tc));
        }
        if let Some(tc) = style.theme_text_color {
            self.text_color = Some(self.ctx.theme.resolve(tc));
        }
        if let Some(tc) = style.theme_border_fg {
            let color = self.ctx.theme.resolve(tc);
            self.border_style = Style::new().fg(color);
        }
        self
    }

    /// Set the border style.
    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }

    /// Show or hide the top border.
    pub fn border_top(mut self, show: bool) -> Self {
        self.border_sides.top = show;
        self
    }

    /// Show or hide the right border.
    pub fn border_right(mut self, show: bool) -> Self {
        self.border_sides.right = show;
        self
    }

    /// Show or hide the bottom border.
    pub fn border_bottom(mut self, show: bool) -> Self {
        self.border_sides.bottom = show;
        self
    }

    /// Show or hide the left border.
    pub fn border_left(mut self, show: bool) -> Self {
        self.border_sides.left = show;
        self
    }

    /// Set which border sides are visible.
    pub fn border_sides(mut self, sides: BorderSides) -> Self {
        self.border_sides = sides;
        self
    }

    /// Show only left and right borders. Shorthand for horizontal border sides.
    pub fn border_x(self) -> Self {
        self.border_sides(BorderSides {
            top: false,
            right: true,
            bottom: false,
            left: true,
        })
    }

    /// Show only top and bottom borders. Shorthand for vertical border sides.
    pub fn border_y(self) -> Self {
        self.border_sides(BorderSides {
            top: true,
            right: false,
            bottom: true,
            left: false,
        })
    }

    /// Set rounded border style. Shorthand for `.border(Border::Rounded)`.
    pub fn rounded(self) -> Self {
        self.border(Border::Rounded)
    }

    /// Set the style applied to the border characters.
    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    /// Set the border foreground color.
    pub fn border_fg(mut self, color: Color) -> Self {
        self.border_style = self.border_style.fg(color);
        self
    }

    /// Border style used when dark mode is active.
    pub fn dark_border_style(mut self, style: Style) -> Self {
        self.dark_border_style = Some(style);
        self
    }

    /// Set the background color.
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Set the default text color for all child text elements in this container.
    /// Individual `.fg()` calls on text elements will still override this.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Background color used when dark mode is active.
    pub fn dark_bg(mut self, color: Color) -> Self {
        self.dark_bg = Some(color);
        self
    }

    /// Background color applied when the parent group is hovered.
    pub fn group_hover_bg(mut self, color: Color) -> Self {
        self.group_hover_bg = Some(color);
        self
    }

    /// Border style applied when the parent group is hovered.
    pub fn group_hover_border_style(mut self, style: Style) -> Self {
        self.group_hover_border_style = Some(style);
        self
    }

    // ── padding (Tailwind: p, px, py, pt, pr, pb, pl) ───────────────

    /// Set uniform padding on all sides. Alias for [`pad`](Self::pad).
    pub fn p(self, value: u32) -> Self {
        self.pad(value)
    }

    /// Set uniform padding on all sides.
    pub fn pad(mut self, value: u32) -> Self {
        self.padding = Padding::all(value);
        self
    }

    /// Set horizontal padding (left and right).
    pub fn px(mut self, value: u32) -> Self {
        self.padding.left = value;
        self.padding.right = value;
        self
    }

    /// Set vertical padding (top and bottom).
    pub fn py(mut self, value: u32) -> Self {
        self.padding.top = value;
        self.padding.bottom = value;
        self
    }

    /// Set top padding.
    pub fn pt(mut self, value: u32) -> Self {
        self.padding.top = value;
        self
    }

    /// Set right padding.
    pub fn pr(mut self, value: u32) -> Self {
        self.padding.right = value;
        self
    }

    /// Set bottom padding.
    pub fn pb(mut self, value: u32) -> Self {
        self.padding.bottom = value;
        self
    }

    /// Set left padding.
    pub fn pl(mut self, value: u32) -> Self {
        self.padding.left = value;
        self
    }

    /// Set per-side padding using a [`Padding`] value.
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    // ── margin (Tailwind: m, mx, my, mt, mr, mb, ml) ────────────────

    /// Set uniform margin on all sides.
    pub fn m(mut self, value: u32) -> Self {
        self.margin = Margin::all(value);
        self
    }

    /// Set horizontal margin (left and right).
    pub fn mx(mut self, value: u32) -> Self {
        self.margin.left = value;
        self.margin.right = value;
        self
    }

    /// Set vertical margin (top and bottom).
    pub fn my(mut self, value: u32) -> Self {
        self.margin.top = value;
        self.margin.bottom = value;
        self
    }

    /// Set top margin.
    pub fn mt(mut self, value: u32) -> Self {
        self.margin.top = value;
        self
    }

    /// Set right margin.
    pub fn mr(mut self, value: u32) -> Self {
        self.margin.right = value;
        self
    }

    /// Set bottom margin.
    pub fn mb(mut self, value: u32) -> Self {
        self.margin.bottom = value;
        self
    }

    /// Set left margin.
    pub fn ml(mut self, value: u32) -> Self {
        self.margin.left = value;
        self
    }

    /// Set per-side margin using a [`Margin`] value.
    pub fn margin(mut self, margin: Margin) -> Self {
        self.margin = margin;
        self
    }

    // ── sizing (Tailwind: w, h, min-w, max-w, min-h, max-h) ────────

    /// Set a fixed width (sets both min and max width).
    pub fn w(mut self, value: u32) -> Self {
        self.constraints.min_width = Some(value);
        self.constraints.max_width = Some(value);
        self
    }

    define_breakpoint_methods!(
        base = w,
        arg = value: u32,
        xs = xs_w => [
            "Width applied only at Xs breakpoint (< 40 cols).",
            "",
            "# Example",
            "```ignore",
            "ui.container().w(20).md_w(40).lg_w(60).col(|ui| { ... });",
            "```"
        ],
        sm = sm_w => ["Width applied only at Sm breakpoint (40-79 cols)."],
        md = md_w => ["Width applied only at Md breakpoint (80-119 cols)."],
        lg = lg_w => ["Width applied only at Lg breakpoint (120-159 cols)."],
        xl = xl_w => ["Width applied only at Xl breakpoint (>= 160 cols)."],
        at = w_at => ["Width applied only at the given breakpoint."]
    );

    /// Set a fixed height (sets both min and max height).
    pub fn h(mut self, value: u32) -> Self {
        self.constraints.min_height = Some(value);
        self.constraints.max_height = Some(value);
        self
    }

    define_breakpoint_methods!(
        base = h,
        arg = value: u32,
        xs = xs_h => ["Height applied only at Xs breakpoint (< 40 cols)."],
        sm = sm_h => ["Height applied only at Sm breakpoint (40-79 cols)."],
        md = md_h => ["Height applied only at Md breakpoint (80-119 cols)."],
        lg = lg_h => ["Height applied only at Lg breakpoint (120-159 cols)."],
        xl = xl_h => ["Height applied only at Xl breakpoint (>= 160 cols)."],
        at = h_at => ["Height applied only at the given breakpoint."]
    );

    /// Set the minimum width constraint. Shorthand for [`min_width`](Self::min_width).
    pub fn min_w(mut self, value: u32) -> Self {
        self.constraints.min_width = Some(value);
        self
    }

    define_breakpoint_methods!(
        base = min_w,
        arg = value: u32,
        xs = xs_min_w => ["Minimum width applied only at Xs breakpoint (< 40 cols)."],
        sm = sm_min_w => ["Minimum width applied only at Sm breakpoint (40-79 cols)."],
        md = md_min_w => ["Minimum width applied only at Md breakpoint (80-119 cols)."],
        lg = lg_min_w => ["Minimum width applied only at Lg breakpoint (120-159 cols)."],
        xl = xl_min_w => ["Minimum width applied only at Xl breakpoint (>= 160 cols)."],
        at = min_w_at => ["Minimum width applied only at the given breakpoint."]
    );

    /// Set the maximum width constraint. Shorthand for [`max_width`](Self::max_width).
    pub fn max_w(mut self, value: u32) -> Self {
        self.constraints.max_width = Some(value);
        self
    }

    define_breakpoint_methods!(
        base = max_w,
        arg = value: u32,
        xs = xs_max_w => ["Maximum width applied only at Xs breakpoint (< 40 cols)."],
        sm = sm_max_w => ["Maximum width applied only at Sm breakpoint (40-79 cols)."],
        md = md_max_w => ["Maximum width applied only at Md breakpoint (80-119 cols)."],
        lg = lg_max_w => ["Maximum width applied only at Lg breakpoint (120-159 cols)."],
        xl = xl_max_w => ["Maximum width applied only at Xl breakpoint (>= 160 cols)."],
        at = max_w_at => ["Maximum width applied only at the given breakpoint."]
    );

    /// Set the minimum height constraint. Shorthand for [`min_height`](Self::min_height).
    pub fn min_h(mut self, value: u32) -> Self {
        self.constraints.min_height = Some(value);
        self
    }

    /// Set the maximum height constraint. Shorthand for [`max_height`](Self::max_height).
    pub fn max_h(mut self, value: u32) -> Self {
        self.constraints.max_height = Some(value);
        self
    }

    /// Set the minimum width constraint in cells.
    pub fn min_width(mut self, value: u32) -> Self {
        self.constraints.min_width = Some(value);
        self
    }

    /// Set the maximum width constraint in cells.
    pub fn max_width(mut self, value: u32) -> Self {
        self.constraints.max_width = Some(value);
        self
    }

    /// Set the minimum height constraint in rows.
    pub fn min_height(mut self, value: u32) -> Self {
        self.constraints.min_height = Some(value);
        self
    }

    /// Set the maximum height constraint in rows.
    pub fn max_height(mut self, value: u32) -> Self {
        self.constraints.max_height = Some(value);
        self
    }

    /// Set width as a percentage (1-100) of the parent container.
    pub fn w_pct(mut self, pct: u8) -> Self {
        self.constraints.width_pct = Some(pct.min(100));
        self
    }

    /// Set height as a percentage (1-100) of the parent container.
    pub fn h_pct(mut self, pct: u8) -> Self {
        self.constraints.height_pct = Some(pct.min(100));
        self
    }

    /// Set all size constraints at once using a [`Constraints`] value.
    pub fn constraints(mut self, constraints: Constraints) -> Self {
        self.constraints = constraints;
        self
    }

    // ── flex ─────────────────────────────────────────────────────────

    /// Set the gap (in cells) between child elements.
    pub fn gap(mut self, gap: u32) -> Self {
        self.gap = gap;
        self
    }

    /// Set the gap between children for column layouts (vertical spacing).
    /// Overrides `.gap()` when finalized with `.col()`.
    pub fn row_gap(mut self, value: u32) -> Self {
        self.row_gap = Some(value);
        self
    }

    /// Set the gap between children for row layouts (horizontal spacing).
    /// Overrides `.gap()` when finalized with `.row()`.
    pub fn col_gap(mut self, value: u32) -> Self {
        self.col_gap = Some(value);
        self
    }

    define_breakpoint_methods!(
        base = gap,
        arg = value: u32,
        xs = xs_gap => ["Gap applied only at Xs breakpoint (< 40 cols)."],
        sm = sm_gap => ["Gap applied only at Sm breakpoint (40-79 cols)."],
        md = md_gap => [
            "Gap applied only at Md breakpoint (80-119 cols).",
            "",
            "# Example",
            "```ignore",
            "ui.container().gap(0).md_gap(2).col(|ui| { ... });",
            "```"
        ],
        lg = lg_gap => ["Gap applied only at Lg breakpoint (120-159 cols)."],
        xl = xl_gap => ["Gap applied only at Xl breakpoint (>= 160 cols)."],
        at = gap_at => ["Gap applied only at the given breakpoint."]
    );

    /// Set the flex-grow factor. `1` means the container expands to fill available space.
    pub fn grow(mut self, grow: u16) -> Self {
        self.grow = grow;
        self
    }

    define_breakpoint_methods!(
        base = grow,
        arg = value: u16,
        xs = xs_grow => ["Grow factor applied only at Xs breakpoint (< 40 cols)."],
        sm = sm_grow => ["Grow factor applied only at Sm breakpoint (40-79 cols)."],
        md = md_grow => ["Grow factor applied only at Md breakpoint (80-119 cols)."],
        lg = lg_grow => ["Grow factor applied only at Lg breakpoint (120-159 cols)."],
        xl = xl_grow => ["Grow factor applied only at Xl breakpoint (>= 160 cols)."],
        at = grow_at => ["Grow factor applied only at the given breakpoint."]
    );

    define_breakpoint_methods!(
        base = p,
        arg = value: u32,
        xs = xs_p => ["Uniform padding applied only at Xs breakpoint (< 40 cols)."],
        sm = sm_p => ["Uniform padding applied only at Sm breakpoint (40-79 cols)."],
        md = md_p => ["Uniform padding applied only at Md breakpoint (80-119 cols)."],
        lg = lg_p => ["Uniform padding applied only at Lg breakpoint (120-159 cols)."],
        xl = xl_p => ["Uniform padding applied only at Xl breakpoint (>= 160 cols)."],
        at = p_at => ["Padding applied only at the given breakpoint."]
    );

    // ── alignment ───────────────────────────────────────────────────

    /// Set the cross-axis alignment of child elements.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Center children on the cross axis. Shorthand for `.align(Align::Center)`.
    pub fn center(self) -> Self {
        self.align(Align::Center)
    }

    /// Set the main-axis content distribution mode.
    pub fn justify(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }

    /// Distribute children with equal space between; first at start, last at end.
    pub fn space_between(self) -> Self {
        self.justify(Justify::SpaceBetween)
    }

    /// Distribute children with equal space around each child.
    pub fn space_around(self) -> Self {
        self.justify(Justify::SpaceAround)
    }

    /// Distribute children with equal space between all children and edges.
    pub fn space_evenly(self) -> Self {
        self.justify(Justify::SpaceEvenly)
    }

    /// Center children on both axes. Shorthand for `.justify(Justify::Center).align(Align::Center)`.
    pub fn flex_center(self) -> Self {
        self.justify(Justify::Center).align(Align::Center)
    }

    /// Override the parent's cross-axis alignment for this container only.
    /// Like CSS `align-self`.
    pub fn align_self(mut self, align: Align) -> Self {
        self.align_self_value = Some(align);
        self
    }

    // ── title ────────────────────────────────────────────────────────

    /// Set a plain-text title rendered in the top border.
    pub fn title(self, title: impl Into<String>) -> Self {
        self.title_styled(title, Style::new())
    }

    /// Set a styled title rendered in the top border.
    pub fn title_styled(mut self, title: impl Into<String>, style: Style) -> Self {
        self.title = Some((title.into(), style));
        self
    }

    // ── internal ─────────────────────────────────────────────────────

    /// Set the vertical scroll offset in rows. Used internally by [`Context::scrollable`].
    pub fn scroll_offset(mut self, offset: u32) -> Self {
        self.scroll_offset = Some(offset);
        self
    }

    pub(crate) fn group_name(mut self, name: String) -> Self {
        self.group_name = Some(name);
        self
    }

    /// Finalize the builder as a vertical (column) container.
    ///
    /// The closure receives a `&mut Context` for rendering children.
    /// Returns a [`Response`] with click/hover state for this container.
    pub fn col(self, f: impl FnOnce(&mut Context)) -> Response {
        self.finish(Direction::Column, f)
    }

    /// Finalize the builder as a horizontal (row) container.
    ///
    /// The closure receives a `&mut Context` for rendering children.
    /// Returns a [`Response`] with click/hover state for this container.
    pub fn row(self, f: impl FnOnce(&mut Context)) -> Response {
        self.finish(Direction::Row, f)
    }

    /// Finalize the builder as an inline text line.
    ///
    /// Like [`row`](ContainerBuilder::row) but gap is forced to zero
    /// for seamless inline rendering of mixed-style text.
    pub fn line(mut self, f: impl FnOnce(&mut Context)) -> Response {
        self.gap = 0;
        self.finish(Direction::Row, f)
    }

    /// Finalize the builder as a raw-draw region with direct buffer access.
    ///
    /// The closure receives `(&mut Buffer, Rect)` after layout is computed.
    /// Use `buf.set_char()`, `buf.set_string()`, `buf.get_mut()` to write
    /// directly into the terminal buffer. Writes outside `rect` are clipped.
    ///
    /// The closure must be `'static` because it is deferred until after layout.
    /// To capture local data, clone or move it into the closure:
    /// ```ignore
    /// let data = my_vec.clone();
    /// ui.container().w(40).h(20).draw(move |buf, rect| {
    ///     // use `data` here
    /// });
    /// ```
    pub fn draw(self, f: impl FnOnce(&mut crate::buffer::Buffer, Rect) + 'static) {
        let draw_id = self.ctx.deferred_draws.len();
        self.ctx.deferred_draws.push(Some(Box::new(f)));
        self.ctx.skip_interaction_slot();
        self.ctx.commands.push(Command::RawDraw {
            draw_id,
            constraints: self.constraints,
            grow: self.grow,
            margin: self.margin,
        });
    }

    /// Custom drawing with click and hover detection.
    ///
    /// Like [`draw`](Self::draw), but the returned [`Response`] reports
    /// `clicked` and `hovered` based on the laid-out region — exactly like
    /// `.col()` or `.row()`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let resp = ui.container()
    ///     .w(40).h(10)
    ///     .draw_interactive(|buf, rect| {
    ///         buf.set_string(rect.x, rect.y, "Click me!", slt::Style::new());
    ///     });
    /// if resp.clicked {
    ///     // handle click
    /// }
    /// # });
    /// ```
    pub fn draw_interactive(
        self,
        f: impl FnOnce(&mut crate::buffer::Buffer, Rect) + 'static,
    ) -> Response {
        let draw_id = self.ctx.deferred_draws.len();
        self.ctx.deferred_draws.push(Some(Box::new(f)));
        let interaction_id = self.ctx.next_interaction_id();
        self.ctx.commands.push(Command::RawDraw {
            draw_id,
            constraints: self.constraints,
            grow: self.grow,
            margin: self.margin,
        });
        self.ctx.response_for(interaction_id)
    }

    fn finish(mut self, direction: Direction, f: impl FnOnce(&mut Context)) -> Response {
        let interaction_id = self.ctx.next_interaction_id();
        let resolved_gap = match direction {
            Direction::Column => self.row_gap.unwrap_or(self.gap),
            Direction::Row => self.col_gap.unwrap_or(self.gap),
        };

        let in_hovered_group = self
            .group_name
            .as_ref()
            .map(|name| self.ctx.is_group_hovered(name))
            .unwrap_or(false)
            || self
                .ctx
                .rollback
                .group_stack
                .last()
                .map(|name| self.ctx.is_group_hovered(name))
                .unwrap_or(false);
        let in_focused_group = self
            .group_name
            .as_ref()
            .map(|name| self.ctx.is_group_focused(name))
            .unwrap_or(false)
            || self
                .ctx
                .rollback
                .group_stack
                .last()
                .map(|name| self.ctx.is_group_focused(name))
                .unwrap_or(false);

        let resolved_bg = if self.ctx.rollback.dark_mode {
            self.dark_bg.or(self.bg)
        } else {
            self.bg
        };
        let resolved_border_style = if self.ctx.rollback.dark_mode {
            self.dark_border_style.unwrap_or(self.border_style)
        } else {
            self.border_style
        };
        let bg_color = if in_hovered_group || in_focused_group {
            self.group_hover_bg.or(resolved_bg)
        } else {
            resolved_bg
        };
        let border_style = if in_hovered_group || in_focused_group {
            self.group_hover_border_style
                .unwrap_or(resolved_border_style)
        } else {
            resolved_border_style
        };
        let group_name = self.group_name.take();
        let is_group_container = group_name.is_some();

        if let Some(scroll_offset) = self.scroll_offset {
            self.ctx.commands.push(Command::BeginScrollable {
                grow: self.grow,
                border: self.border,
                border_sides: self.border_sides,
                border_style,
                padding: self.padding,
                margin: self.margin,
                constraints: self.constraints,
                title: self.title,
                scroll_offset,
            });
        } else {
            self.ctx.commands.push(Command::BeginContainer {
                direction,
                gap: resolved_gap,
                align: self.align,
                align_self: self.align_self_value,
                justify: self.justify,
                border: self.border,
                border_sides: self.border_sides,
                border_style,
                bg_color,
                padding: self.padding,
                margin: self.margin,
                constraints: self.constraints,
                title: self.title,
                grow: self.grow,
                group_name,
            });
        }
        self.ctx.rollback.text_color_stack.push(self.text_color);
        f(self.ctx);
        self.ctx.rollback.text_color_stack.pop();
        self.ctx.commands.push(Command::EndContainer);
        self.ctx.rollback.last_text_idx = None;

        if is_group_container {
            self.ctx.rollback.group_stack.pop();
            self.ctx.rollback.group_count = self.ctx.rollback.group_count.saturating_sub(1);
        }

        self.ctx.response_for(interaction_id)
    }
}
