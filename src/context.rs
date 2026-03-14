use crate::chart::{build_histogram_config, render_chart, ChartBuilder, HistogramBuilder};
use crate::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseKind};
use crate::layout::{Command, Direction};
use crate::rect::Rect;
use crate::style::{
    Align, Border, Color, Constraints, Justify, Margin, Modifiers, Padding, Style, Theme,
};
use crate::widgets::{
    ButtonVariant, FormField, FormState, ListState, ScrollState, SpinnerState, TableState,
    TabsState, TextInputState, TextareaState, ToastLevel, ToastState,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[allow(dead_code)]
fn slt_assert(condition: bool, msg: &str) {
    if !condition {
        panic!("[SLT] {}", msg);
    }
}

#[cfg(debug_assertions)]
#[allow(dead_code)]
fn slt_warn(msg: &str) {
    eprintln!("\x1b[33m[SLT warning]\x1b[0m {}", msg);
}

#[cfg(not(debug_assertions))]
#[allow(dead_code)]
fn slt_warn(_msg: &str) {}

/// Result of a container mouse interaction.
///
/// Returned by [`Context::col`], [`Context::row`], and [`ContainerBuilder::col`] /
/// [`ContainerBuilder::row`] so you can react to clicks and hover without a separate
/// event loop.
#[derive(Debug, Clone, Copy, Default)]
pub struct Response {
    /// Whether the container was clicked this frame.
    pub clicked: bool,
    /// Whether the mouse is over the container.
    pub hovered: bool,
}

/// Direction for bar chart rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarDirection {
    /// Bars grow horizontally (default, current behavior).
    Horizontal,
    /// Bars grow vertically from bottom to top.
    Vertical,
}

/// A single bar in a styled bar chart.
#[derive(Debug, Clone)]
pub struct Bar {
    /// Display label for this bar.
    pub label: String,
    /// Numeric value.
    pub value: f64,
    /// Bar color. If None, uses theme.primary.
    pub color: Option<Color>,
}

impl Bar {
    /// Create a new bar with a label and value.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
        }
    }

    /// Set the bar color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// A group of bars rendered together (for grouped bar charts).
#[derive(Debug, Clone)]
pub struct BarGroup {
    /// Group label displayed below the bars.
    pub label: String,
    /// Bars in this group.
    pub bars: Vec<Bar>,
}

impl BarGroup {
    /// Create a new bar group with a label and bars.
    pub fn new(label: impl Into<String>, bars: Vec<Bar>) -> Self {
        Self {
            label: label.into(),
            bars,
        }
    }
}

/// Trait for creating custom widgets.
///
/// Implement this trait to build reusable, composable widgets with full access
/// to the [`Context`] API — focus, events, theming, layout, and mouse interaction.
///
/// # Examples
///
/// A simple rating widget:
///
/// ```no_run
/// use slt::{Context, Widget, Color};
///
/// struct Rating {
///     value: u8,
///     max: u8,
/// }
///
/// impl Rating {
///     fn new(value: u8, max: u8) -> Self {
///         Self { value, max }
///     }
/// }
///
/// impl Widget for Rating {
///     type Response = bool;
///
///     fn ui(&mut self, ui: &mut Context) -> bool {
///         let focused = ui.register_focusable();
///         let mut changed = false;
///
///         if focused {
///             if ui.key('+') && self.value < self.max {
///                 self.value += 1;
///                 changed = true;
///             }
///             if ui.key('-') && self.value > 0 {
///                 self.value -= 1;
///                 changed = true;
///             }
///         }
///
///         let stars: String = (0..self.max).map(|i| {
///             if i < self.value { '★' } else { '☆' }
///         }).collect();
///
///         let color = if focused { Color::Yellow } else { Color::White };
///         ui.styled(stars, slt::Style::new().fg(color));
///
///         changed
///     }
/// }
///
/// fn main() -> std::io::Result<()> {
///     let mut rating = Rating::new(3, 5);
///     slt::run(|ui| {
///         if ui.key('q') { ui.quit(); }
///         ui.text("Rate this:");
///         ui.widget(&mut rating);
///     })
/// }
/// ```
pub trait Widget {
    /// The value returned after rendering. Use `()` for widgets with no return,
    /// `bool` for widgets that report changes, or [`Response`] for click/hover.
    type Response;

    /// Render the widget into the given context.
    ///
    /// Use [`Context::register_focusable`] to participate in Tab focus cycling,
    /// [`Context::key`] / [`Context::key_code`] to handle keyboard input,
    /// and [`Context::interaction`] to detect clicks and hovers.
    fn ui(&mut self, ctx: &mut Context) -> Self::Response;
}

/// The main rendering context passed to your closure each frame.
///
/// Provides all methods for building UI: text, containers, widgets, and event
/// handling. You receive a `&mut Context` on every frame and describe what to
/// render by calling its methods. SLT collects those calls, lays them out with
/// flexbox, diffs against the previous frame, and flushes only changed cells.
///
/// # Example
///
/// ```no_run
/// slt::run(|ui: &mut slt::Context| {
///     if ui.key('q') { ui.quit(); }
///     ui.text("Hello, world!").bold();
/// });
/// ```
pub struct Context {
    pub(crate) commands: Vec<Command>,
    pub(crate) events: Vec<Event>,
    pub(crate) consumed: Vec<bool>,
    pub(crate) should_quit: bool,
    pub(crate) area_width: u32,
    pub(crate) area_height: u32,
    pub(crate) tick: u64,
    pub(crate) focus_index: usize,
    pub(crate) focus_count: usize,
    prev_focus_count: usize,
    scroll_count: usize,
    prev_scroll_infos: Vec<(u32, u32)>,
    interaction_count: usize,
    pub(crate) prev_hit_map: Vec<Rect>,
    _prev_focus_rects: Vec<(usize, Rect)>,
    mouse_pos: Option<(u32, u32)>,
    click_pos: Option<(u32, u32)>,
    last_text_idx: Option<usize>,
    overlay_depth: usize,
    pub(crate) modal_active: bool,
    prev_modal_active: bool,
    debug: bool,
    theme: Theme,
}

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
#[must_use = "configure and finalize with .col() or .row()"]
pub struct ContainerBuilder<'a> {
    ctx: &'a mut Context,
    gap: u32,
    align: Align,
    justify: Justify,
    border: Option<Border>,
    border_style: Style,
    bg_color: Option<Color>,
    padding: Padding,
    margin: Margin,
    constraints: Constraints,
    title: Option<(String, Style)>,
    grow: u16,
    scroll_offset: Option<u32>,
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

pub struct CanvasContext {
    layers: Vec<CanvasLayer>,
    cols: usize,
    rows: usize,
    px_w: usize,
    px_h: usize,
    current_color: Color,
}

impl CanvasContext {
    fn new(cols: usize, rows: usize) -> Self {
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

impl<'a> ContainerBuilder<'a> {
    // ── border ───────────────────────────────────────────────────────

    /// Set the border style.
    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
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

    pub fn bg(mut self, color: Color) -> Self {
        self.bg_color = Some(color);
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

    /// Set a fixed height (sets both min and max height).
    pub fn h(mut self, value: u32) -> Self {
        self.constraints.min_height = Some(value);
        self.constraints.max_height = Some(value);
        self
    }

    /// Set the minimum width constraint. Shorthand for [`min_width`](Self::min_width).
    pub fn min_w(mut self, value: u32) -> Self {
        self.constraints.min_width = Some(value);
        self
    }

    /// Set the maximum width constraint. Shorthand for [`max_width`](Self::max_width).
    pub fn max_w(mut self, value: u32) -> Self {
        self.constraints.max_width = Some(value);
        self
    }

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

    /// Set the flex-grow factor. `1` means the container expands to fill available space.
    pub fn grow(mut self, grow: u16) -> Self {
        self.grow = grow;
        self
    }

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

    fn finish(self, direction: Direction, f: impl FnOnce(&mut Context)) -> Response {
        let interaction_id = self.ctx.interaction_count;
        self.ctx.interaction_count += 1;

        if let Some(scroll_offset) = self.scroll_offset {
            self.ctx.commands.push(Command::BeginScrollable {
                grow: self.grow,
                border: self.border,
                border_style: self.border_style,
                padding: self.padding,
                margin: self.margin,
                constraints: self.constraints,
                title: self.title,
                scroll_offset,
            });
        } else {
            self.ctx.commands.push(Command::BeginContainer {
                direction,
                gap: self.gap,
                align: self.align,
                justify: self.justify,
                border: self.border,
                border_style: self.border_style,
                bg_color: self.bg_color,
                padding: self.padding,
                margin: self.margin,
                constraints: self.constraints,
                title: self.title,
                grow: self.grow,
            });
        }
        f(self.ctx);
        self.ctx.commands.push(Command::EndContainer);
        self.ctx.last_text_idx = None;

        self.ctx.response_for(interaction_id)
    }
}

impl Context {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        events: Vec<Event>,
        width: u32,
        height: u32,
        tick: u64,
        mut focus_index: usize,
        prev_focus_count: usize,
        prev_scroll_infos: Vec<(u32, u32)>,
        prev_hit_map: Vec<Rect>,
        prev_focus_rects: Vec<(usize, Rect)>,
        debug: bool,
        theme: Theme,
        last_mouse_pos: Option<(u32, u32)>,
        prev_modal_active: bool,
    ) -> Self {
        let consumed = vec![false; events.len()];

        let mut mouse_pos = last_mouse_pos;
        let mut click_pos = None;
        for event in &events {
            if let Event::Mouse(mouse) = event {
                mouse_pos = Some((mouse.x, mouse.y));
                if matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                    click_pos = Some((mouse.x, mouse.y));
                }
            }
        }

        if let Some((mx, my)) = click_pos {
            let mut best: Option<(usize, u64)> = None;
            for &(fid, rect) in &prev_focus_rects {
                if mx >= rect.x && mx < rect.right() && my >= rect.y && my < rect.bottom() {
                    let area = rect.width as u64 * rect.height as u64;
                    if best.map_or(true, |(_, ba)| area < ba) {
                        best = Some((fid, area));
                    }
                }
            }
            if let Some((fid, _)) = best {
                focus_index = fid;
            }
        }

        Self {
            commands: Vec::new(),
            events,
            consumed,
            should_quit: false,
            area_width: width,
            area_height: height,
            tick,
            focus_index,
            focus_count: 0,
            prev_focus_count,
            scroll_count: 0,
            prev_scroll_infos,
            interaction_count: 0,
            prev_hit_map,
            _prev_focus_rects: prev_focus_rects,
            mouse_pos,
            click_pos,
            last_text_idx: None,
            overlay_depth: 0,
            modal_active: false,
            prev_modal_active,
            debug,
            theme,
        }
    }

    pub(crate) fn process_focus_keys(&mut self) {
        for (i, event) in self.events.iter().enumerate() {
            if let Event::Key(key) = event {
                if key.code == KeyCode::Tab && !key.modifiers.contains(KeyModifiers::SHIFT) {
                    if self.prev_focus_count > 0 {
                        self.focus_index = (self.focus_index + 1) % self.prev_focus_count;
                    }
                    self.consumed[i] = true;
                } else if (key.code == KeyCode::Tab && key.modifiers.contains(KeyModifiers::SHIFT))
                    || key.code == KeyCode::BackTab
                {
                    if self.prev_focus_count > 0 {
                        self.focus_index = if self.focus_index == 0 {
                            self.prev_focus_count - 1
                        } else {
                            self.focus_index - 1
                        };
                    }
                    self.consumed[i] = true;
                }
            }
        }
    }

    /// Render a custom [`Widget`].
    ///
    /// Calls [`Widget::ui`] with this context and returns the widget's response.
    pub fn widget<W: Widget>(&mut self, w: &mut W) -> W::Response {
        w.ui(self)
    }

    /// Wrap child widgets in a panic boundary.
    ///
    /// If the closure panics, the panic is caught and an error message is
    /// rendered in place of the children. The app continues running.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.error_boundary(|ui| {
    ///     ui.text("risky widget");
    /// });
    /// # });
    /// ```
    pub fn error_boundary(&mut self, f: impl FnOnce(&mut Context)) {
        self.error_boundary_with(f, |ui, msg| {
            ui.styled(
                format!("⚠ Error: {msg}"),
                Style::new().fg(ui.theme.error).bold(),
            );
        });
    }

    /// Like [`error_boundary`](Self::error_boundary), but renders a custom
    /// fallback instead of the default error message.
    ///
    /// The fallback closure receives the panic message as a [`String`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.error_boundary_with(
    ///     |ui| {
    ///         ui.text("risky widget");
    ///     },
    ///     |ui, msg| {
    ///         ui.text(format!("Recovered from panic: {msg}"));
    ///     },
    /// );
    /// # });
    /// ```
    pub fn error_boundary_with(
        &mut self,
        f: impl FnOnce(&mut Context),
        fallback: impl FnOnce(&mut Context, String),
    ) {
        let cmd_count = self.commands.len();
        let last_text_idx = self.last_text_idx;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            f(self);
        }));

        match result {
            Ok(()) => {}
            Err(panic_info) => {
                self.commands.truncate(cmd_count);
                self.last_text_idx = last_text_idx;

                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "widget panicked".to_string()
                };

                fallback(self, msg);
            }
        }
    }

    /// Allocate a click/hover interaction slot and return the [`Response`].
    ///
    /// Use this in custom widgets to detect mouse clicks and hovers without
    /// wrapping content in a container. Each call reserves one slot in the
    /// hit-test map, so the call order must be stable across frames.
    pub fn interaction(&mut self) -> Response {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return Response::default();
        }
        let id = self.interaction_count;
        self.interaction_count += 1;
        self.response_for(id)
    }

    /// Register a widget as focusable and return whether it currently has focus.
    ///
    /// Call this in custom widgets that need keyboard focus. Each call increments
    /// the internal focus counter, so the call order must be stable across frames.
    pub fn register_focusable(&mut self) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        let id = self.focus_count;
        self.focus_count += 1;
        self.commands.push(Command::FocusMarker(id));
        if self.prev_focus_count == 0 {
            return true;
        }
        self.focus_index % self.prev_focus_count == id
    }

    // ── text ──────────────────────────────────────────────────────────

    /// Render a text element. Returns `&mut Self` for style chaining.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// ui.text("hello").bold().fg(Color::Cyan);
    /// # });
    /// ```
    pub fn text(&mut self, s: impl Into<String>) -> &mut Self {
        let content = s.into();
        self.commands.push(Command::Text {
            content,
            style: Style::new(),
            grow: 0,
            align: Align::Start,
            wrap: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render a clickable hyperlink.
    ///
    /// The link is interactive: clicking it (or pressing Enter/Space when
    /// focused) opens the URL in the system browser. OSC 8 is also emitted
    /// for terminals that support native hyperlinks.
    pub fn link(&mut self, text: impl Into<String>, url: impl Into<String>) -> &mut Self {
        let url_str = url.into();
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);

        let mut activated = response.clicked;
        if focused {
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        activated = true;
                        self.consumed[i] = true;
                    }
                }
            }
        }

        if activated {
            let _ = open_url(&url_str);
        }

        let style = if focused {
            Style::new()
                .fg(self.theme.primary)
                .bg(self.theme.surface_hover)
                .underline()
                .bold()
        } else if response.hovered {
            Style::new()
                .fg(self.theme.accent)
                .bg(self.theme.surface_hover)
                .underline()
        } else {
            Style::new().fg(self.theme.primary).underline()
        };

        self.commands.push(Command::Link {
            text: text.into(),
            url: url_str,
            style,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render a text element with word-boundary wrapping.
    ///
    /// Long lines are broken at word boundaries to fit the container width.
    /// Style chaining works the same as [`Context::text`].
    pub fn text_wrap(&mut self, s: impl Into<String>) -> &mut Self {
        let content = s.into();
        self.commands.push(Command::Text {
            content,
            style: Style::new(),
            grow: 0,
            align: Align::Start,
            wrap: true,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    // ── style chain (applies to last text) ───────────────────────────

    /// Apply bold to the last rendered text element.
    pub fn bold(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::BOLD);
        self
    }

    /// Apply dim styling to the last rendered text element.
    ///
    /// Also sets the foreground color to the theme's `text_dim` color if no
    /// explicit foreground has been set.
    pub fn dim(&mut self) -> &mut Self {
        let text_dim = self.theme.text_dim;
        self.modify_last_style(|s| {
            s.modifiers |= Modifiers::DIM;
            if s.fg.is_none() {
                s.fg = Some(text_dim);
            }
        });
        self
    }

    /// Apply italic to the last rendered text element.
    pub fn italic(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::ITALIC);
        self
    }

    /// Apply underline to the last rendered text element.
    pub fn underline(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::UNDERLINE);
        self
    }

    /// Apply reverse-video to the last rendered text element.
    pub fn reversed(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::REVERSED);
        self
    }

    /// Apply strikethrough to the last rendered text element.
    pub fn strikethrough(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::STRIKETHROUGH);
        self
    }

    /// Set the foreground color of the last rendered text element.
    pub fn fg(&mut self, color: Color) -> &mut Self {
        self.modify_last_style(|s| s.fg = Some(color));
        self
    }

    /// Set the background color of the last rendered text element.
    pub fn bg(&mut self, color: Color) -> &mut Self {
        self.modify_last_style(|s| s.bg = Some(color));
        self
    }

    /// Render a text element with an explicit [`Style`] applied immediately.
    ///
    /// Equivalent to calling `text(s)` followed by style-chain methods, but
    /// more concise when you already have a `Style` value.
    pub fn styled(&mut self, s: impl Into<String>, style: Style) -> &mut Self {
        self.commands.push(Command::Text {
            content: s.into(),
            style,
            grow: 0,
            align: Align::Start,
            wrap: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Enable word-boundary wrapping on the last rendered text element.
    pub fn wrap(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text { wrap, .. } = &mut self.commands[idx] {
                *wrap = true;
            }
        }
        self
    }

    fn modify_last_style(&mut self, f: impl FnOnce(&mut Style)) {
        if let Some(idx) = self.last_text_idx {
            match &mut self.commands[idx] {
                Command::Text { style, .. } | Command::Link { style, .. } => f(style),
                _ => {}
            }
        }
    }

    // ── containers ───────────────────────────────────────────────────

    /// Create a vertical (column) container.
    ///
    /// Children are stacked top-to-bottom. Returns a [`Response`] with
    /// click/hover state for the container area.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.col(|ui| {
    ///     ui.text("line one");
    ///     ui.text("line two");
    /// });
    /// # });
    /// ```
    pub fn col(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        self.push_container(Direction::Column, 0, f)
    }

    /// Create a vertical (column) container with a gap between children.
    ///
    /// `gap` is the number of blank rows inserted between each child.
    pub fn col_gap(&mut self, gap: u32, f: impl FnOnce(&mut Context)) -> Response {
        self.push_container(Direction::Column, gap, f)
    }

    /// Create a horizontal (row) container.
    ///
    /// Children are placed left-to-right. Returns a [`Response`] with
    /// click/hover state for the container area.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.row(|ui| {
    ///     ui.text("left");
    ///     ui.spacer();
    ///     ui.text("right");
    /// });
    /// # });
    /// ```
    pub fn row(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        self.push_container(Direction::Row, 0, f)
    }

    /// Create a horizontal (row) container with a gap between children.
    ///
    /// `gap` is the number of blank columns inserted between each child.
    pub fn row_gap(&mut self, gap: u32, f: impl FnOnce(&mut Context)) -> Response {
        self.push_container(Direction::Row, gap, f)
    }

    pub fn modal(&mut self, f: impl FnOnce(&mut Context)) {
        self.commands.push(Command::BeginOverlay { modal: true });
        self.overlay_depth += 1;
        self.modal_active = true;
        f(self);
        self.overlay_depth = self.overlay_depth.saturating_sub(1);
        self.commands.push(Command::EndOverlay);
        self.last_text_idx = None;
    }

    pub fn overlay(&mut self, f: impl FnOnce(&mut Context)) {
        self.commands.push(Command::BeginOverlay { modal: false });
        self.overlay_depth += 1;
        f(self);
        self.overlay_depth = self.overlay_depth.saturating_sub(1);
        self.commands.push(Command::EndOverlay);
        self.last_text_idx = None;
    }

    /// Create a container with a fluent builder.
    ///
    /// Use this for borders, padding, grow, constraints, and titles. Chain
    /// configuration methods on the returned [`ContainerBuilder`], then call
    /// `.col()` or `.row()` to finalize.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Border;
    /// ui.container()
    ///     .border(Border::Rounded)
    ///     .pad(1)
    ///     .title("My Panel")
    ///     .col(|ui| {
    ///         ui.text("content");
    ///     });
    /// # });
    /// ```
    pub fn container(&mut self) -> ContainerBuilder<'_> {
        let border = self.theme.border;
        ContainerBuilder {
            ctx: self,
            gap: 0,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            scroll_offset: None,
        }
    }

    /// Create a scrollable container. Handles wheel scroll and drag-to-scroll automatically.
    ///
    /// Pass a [`ScrollState`] to persist scroll position across frames. The state
    /// is updated in-place with the current scroll offset and bounds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::ScrollState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut scroll = ScrollState::new();
    /// ui.scrollable(&mut scroll).col(|ui| {
    ///     for i in 0..100 {
    ///         ui.text(format!("Line {i}"));
    ///     }
    /// });
    /// # });
    /// ```
    pub fn scrollable(&mut self, state: &mut ScrollState) -> ContainerBuilder<'_> {
        let index = self.scroll_count;
        self.scroll_count += 1;
        if let Some(&(ch, vh)) = self.prev_scroll_infos.get(index) {
            state.set_bounds(ch, vh);
            let max = ch.saturating_sub(vh) as usize;
            state.offset = state.offset.min(max);
        }

        let next_id = self.interaction_count;
        if let Some(rect) = self.prev_hit_map.get(next_id).copied() {
            self.auto_scroll(&rect, state);
        }

        self.container().scroll_offset(state.offset as u32)
    }

    fn auto_scroll(&mut self, rect: &Rect, state: &mut ScrollState) {
        let mut to_consume: Vec<usize> = Vec::new();

        for (i, event) in self.events.iter().enumerate() {
            if self.consumed[i] {
                continue;
            }
            if let Event::Mouse(mouse) = event {
                let in_bounds = mouse.x >= rect.x
                    && mouse.x < rect.right()
                    && mouse.y >= rect.y
                    && mouse.y < rect.bottom();
                if !in_bounds {
                    continue;
                }
                match mouse.kind {
                    MouseKind::ScrollUp => {
                        state.scroll_up(1);
                        to_consume.push(i);
                    }
                    MouseKind::ScrollDown => {
                        state.scroll_down(1);
                        to_consume.push(i);
                    }
                    MouseKind::Drag(MouseButton::Left) => {
                        // Left-drag is reserved for text selection.
                        // Scroll via mouse wheel instead.
                    }
                    _ => {}
                }
            }
        }

        for i in to_consume {
            self.consumed[i] = true;
        }
    }

    /// Shortcut for `container().border(border)`.
    ///
    /// Returns a [`ContainerBuilder`] pre-configured with the given border style.
    pub fn bordered(&mut self, border: Border) -> ContainerBuilder<'_> {
        self.container().border(border)
    }

    fn push_container(
        &mut self,
        direction: Direction,
        gap: u32,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let border = self.theme.border;

        self.commands.push(Command::BeginContainer {
            direction,
            gap,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        f(self);
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self.response_for(interaction_id)
    }

    fn response_for(&self, interaction_id: usize) -> Response {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return Response::default();
        }
        if let Some(rect) = self.prev_hit_map.get(interaction_id) {
            let clicked = self
                .click_pos
                .map(|(mx, my)| {
                    mx >= rect.x && mx < rect.right() && my >= rect.y && my < rect.bottom()
                })
                .unwrap_or(false);
            let hovered = self
                .mouse_pos
                .map(|(mx, my)| {
                    mx >= rect.x && mx < rect.right() && my >= rect.y && my < rect.bottom()
                })
                .unwrap_or(false);
            Response { clicked, hovered }
        } else {
            Response::default()
        }
    }

    /// Set the flex-grow factor of the last rendered text element.
    ///
    /// A value of `1` causes the element to expand and fill remaining space
    /// along the main axis.
    pub fn grow(&mut self, value: u16) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text { grow, .. } = &mut self.commands[idx] {
                *grow = value;
            }
        }
        self
    }

    /// Set the text alignment of the last rendered text element.
    pub fn align(&mut self, align: Align) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text {
                align: text_align, ..
            } = &mut self.commands[idx]
            {
                *text_align = align;
            }
        }
        self
    }

    /// Render an invisible spacer that expands to fill available space.
    ///
    /// Useful for pushing siblings to opposite ends of a row or column.
    pub fn spacer(&mut self) -> &mut Self {
        self.commands.push(Command::Spacer { grow: 1 });
        self.last_text_idx = None;
        self
    }

    /// Render a form that groups input fields vertically.
    ///
    /// Use [`Context::form_field`] inside the closure to render each field.
    pub fn form(
        &mut self,
        state: &mut FormState,
        f: impl FnOnce(&mut Context, &mut FormState),
    ) -> &mut Self {
        self.col(|ui| {
            f(ui, state);
        });
        self
    }

    /// Render a single form field with label and input.
    ///
    /// Shows a validation error below the input when present.
    pub fn form_field(&mut self, field: &mut FormField) -> &mut Self {
        self.col(|ui| {
            ui.styled(field.label.clone(), Style::new().bold().fg(ui.theme.text));
            ui.text_input(&mut field.input);
            if let Some(error) = field.error.as_deref() {
                ui.styled(error.to_string(), Style::new().dim().fg(ui.theme.error));
            }
        });
        self
    }

    /// Render a submit button.
    ///
    /// Returns `true` when the button is clicked or activated.
    pub fn form_submit(&mut self, label: impl Into<String>) -> bool {
        self.button(label)
    }

    /// Render a single-line text input. Auto-handles cursor, typing, and backspace.
    ///
    /// The widget claims focus via [`Context::register_focusable`]. When focused,
    /// it consumes character, backspace, arrow, Home, and End key events.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::TextInputState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut input = TextInputState::with_placeholder("Search...");
    /// ui.text_input(&mut input);
    /// // input.value holds the current text
    /// # });
    /// ```
    pub fn text_input(&mut self, state: &mut TextInputState) -> &mut Self {
        slt_assert(
            !state.value.contains('\n'),
            "text_input got a newline — use textarea instead",
        );
        let focused = self.register_focusable();
        state.cursor = state.cursor.min(state.value.chars().count());

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char(ch) => {
                            if let Some(max) = state.max_length {
                                if state.value.chars().count() >= max {
                                    continue;
                                }
                            }
                            let index = byte_index_for_char(&state.value, state.cursor);
                            state.value.insert(index, ch);
                            state.cursor += 1;
                            consumed_indices.push(i);
                        }
                        KeyCode::Backspace => {
                            if state.cursor > 0 {
                                let start = byte_index_for_char(&state.value, state.cursor - 1);
                                let end = byte_index_for_char(&state.value, state.cursor);
                                state.value.replace_range(start..end, "");
                                state.cursor -= 1;
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Left => {
                            state.cursor = state.cursor.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Right => {
                            state.cursor = (state.cursor + 1).min(state.value.chars().count());
                            consumed_indices.push(i);
                        }
                        KeyCode::Home => {
                            state.cursor = 0;
                            consumed_indices.push(i);
                        }
                        KeyCode::Delete => {
                            let len = state.value.chars().count();
                            if state.cursor < len {
                                let start = byte_index_for_char(&state.value, state.cursor);
                                let end = byte_index_for_char(&state.value, state.cursor + 1);
                                state.value.replace_range(start..end, "");
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::End => {
                            state.cursor = state.value.chars().count();
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
                if let Event::Paste(ref text) = event {
                    for ch in text.chars() {
                        if let Some(max) = state.max_length {
                            if state.value.chars().count() >= max {
                                break;
                            }
                        }
                        let index = byte_index_for_char(&state.value, state.cursor);
                        state.value.insert(index, ch);
                        state.cursor += 1;
                    }
                    consumed_indices.push(i);
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        let show_cursor = focused && (self.tick / 30) % 2 == 0;

        let input_text = if state.value.is_empty() {
            if state.placeholder.len() > 100 {
                slt_warn(
                    "text_input placeholder is very long (>100 chars) — consider shortening it",
                );
            }
            state.placeholder.clone()
        } else {
            let mut rendered = String::new();
            for (idx, ch) in state.value.chars().enumerate() {
                if show_cursor && idx == state.cursor {
                    rendered.push('▎');
                }
                rendered.push(ch);
            }
            if show_cursor && state.cursor >= state.value.chars().count() {
                rendered.push('▎');
            }
            rendered
        };
        let input_style = if state.value.is_empty() {
            Style::new().dim().fg(self.theme.text_dim)
        } else {
            Style::new().fg(self.theme.text)
        };

        let border_color = if focused {
            self.theme.primary
        } else if state.validation_error.is_some() {
            self.theme.error
        } else {
            self.theme.border
        };

        self.bordered(Border::Rounded)
            .border_style(Style::new().fg(border_color))
            .px(1)
            .col(|ui| {
                ui.styled(input_text, input_style);
            });

        if let Some(error) = state.validation_error.clone() {
            self.styled(
                format!("⚠ {error}"),
                Style::new().dim().fg(self.theme.error),
            );
        }
        self
    }

    /// Render an animated spinner.
    ///
    /// The spinner advances one frame per tick. Use [`SpinnerState::dots`] or
    /// [`SpinnerState::line`] to create the state, then chain style methods to
    /// color it.
    pub fn spinner(&mut self, state: &SpinnerState) -> &mut Self {
        self.styled(
            state.frame(self.tick).to_string(),
            Style::new().fg(self.theme.primary),
        )
    }

    /// Render toast notifications. Calls `state.cleanup(tick)` automatically.
    ///
    /// Expired messages are removed before rendering. If there are no active
    /// messages, nothing is rendered and `self` is returned unchanged.
    pub fn toast(&mut self, state: &mut ToastState) -> &mut Self {
        state.cleanup(self.tick);
        if state.messages.is_empty() {
            return self;
        }

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        for message in state.messages.iter().rev() {
            let color = match message.level {
                ToastLevel::Info => self.theme.primary,
                ToastLevel::Success => self.theme.success,
                ToastLevel::Warning => self.theme.warning,
                ToastLevel::Error => self.theme.error,
            };
            self.styled(format!("  ● {}", message.text), Style::new().fg(color));
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a multi-line text area with the given number of visible rows.
    ///
    /// When focused, handles character input, Enter (new line), Backspace,
    /// arrow keys, Home, and End. The cursor is rendered as a block character.
    ///
    /// Set [`TextareaState::word_wrap`] to enable soft-wrapping at a given
    /// display-column width. Up/Down then navigate visual lines.
    pub fn textarea(&mut self, state: &mut TextareaState, visible_rows: u32) -> &mut Self {
        if state.lines.is_empty() {
            state.lines.push(String::new());
        }
        state.cursor_row = state.cursor_row.min(state.lines.len().saturating_sub(1));
        state.cursor_col = state
            .cursor_col
            .min(state.lines[state.cursor_row].chars().count());

        let focused = self.register_focusable();
        let wrap_w = state.wrap_width.unwrap_or(u32::MAX);
        let wrapping = state.wrap_width.is_some();

        let pre_vlines = textarea_build_visual_lines(&state.lines, wrap_w);

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char(ch) => {
                            if let Some(max) = state.max_length {
                                let total: usize =
                                    state.lines.iter().map(|line| line.chars().count()).sum();
                                if total >= max {
                                    continue;
                                }
                            }
                            let index = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            state.lines[state.cursor_row].insert(index, ch);
                            state.cursor_col += 1;
                            consumed_indices.push(i);
                        }
                        KeyCode::Enter => {
                            let split_index = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            let remainder = state.lines[state.cursor_row].split_off(split_index);
                            state.cursor_row += 1;
                            state.lines.insert(state.cursor_row, remainder);
                            state.cursor_col = 0;
                            consumed_indices.push(i);
                        }
                        KeyCode::Backspace => {
                            if state.cursor_col > 0 {
                                let start = byte_index_for_char(
                                    &state.lines[state.cursor_row],
                                    state.cursor_col - 1,
                                );
                                let end = byte_index_for_char(
                                    &state.lines[state.cursor_row],
                                    state.cursor_col,
                                );
                                state.lines[state.cursor_row].replace_range(start..end, "");
                                state.cursor_col -= 1;
                            } else if state.cursor_row > 0 {
                                let current = state.lines.remove(state.cursor_row);
                                state.cursor_row -= 1;
                                state.cursor_col = state.lines[state.cursor_row].chars().count();
                                state.lines[state.cursor_row].push_str(&current);
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Left => {
                            if state.cursor_col > 0 {
                                state.cursor_col -= 1;
                            } else if state.cursor_row > 0 {
                                state.cursor_row -= 1;
                                state.cursor_col = state.lines[state.cursor_row].chars().count();
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Right => {
                            let line_len = state.lines[state.cursor_row].chars().count();
                            if state.cursor_col < line_len {
                                state.cursor_col += 1;
                            } else if state.cursor_row + 1 < state.lines.len() {
                                state.cursor_row += 1;
                                state.cursor_col = 0;
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Up => {
                            if wrapping {
                                let (vrow, vcol) = textarea_logical_to_visual(
                                    &pre_vlines,
                                    state.cursor_row,
                                    state.cursor_col,
                                );
                                if vrow > 0 {
                                    let (lr, lc) =
                                        textarea_visual_to_logical(&pre_vlines, vrow - 1, vcol);
                                    state.cursor_row = lr;
                                    state.cursor_col = lc;
                                }
                            } else if state.cursor_row > 0 {
                                state.cursor_row -= 1;
                                state.cursor_col = state
                                    .cursor_col
                                    .min(state.lines[state.cursor_row].chars().count());
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Down => {
                            if wrapping {
                                let (vrow, vcol) = textarea_logical_to_visual(
                                    &pre_vlines,
                                    state.cursor_row,
                                    state.cursor_col,
                                );
                                if vrow + 1 < pre_vlines.len() {
                                    let (lr, lc) =
                                        textarea_visual_to_logical(&pre_vlines, vrow + 1, vcol);
                                    state.cursor_row = lr;
                                    state.cursor_col = lc;
                                }
                            } else if state.cursor_row + 1 < state.lines.len() {
                                state.cursor_row += 1;
                                state.cursor_col = state
                                    .cursor_col
                                    .min(state.lines[state.cursor_row].chars().count());
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Home => {
                            state.cursor_col = 0;
                            consumed_indices.push(i);
                        }
                        KeyCode::Delete => {
                            let line_len = state.lines[state.cursor_row].chars().count();
                            if state.cursor_col < line_len {
                                let start = byte_index_for_char(
                                    &state.lines[state.cursor_row],
                                    state.cursor_col,
                                );
                                let end = byte_index_for_char(
                                    &state.lines[state.cursor_row],
                                    state.cursor_col + 1,
                                );
                                state.lines[state.cursor_row].replace_range(start..end, "");
                            } else if state.cursor_row + 1 < state.lines.len() {
                                let next = state.lines.remove(state.cursor_row + 1);
                                state.lines[state.cursor_row].push_str(&next);
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::End => {
                            state.cursor_col = state.lines[state.cursor_row].chars().count();
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
                if let Event::Paste(ref text) = event {
                    for ch in text.chars() {
                        if ch == '\n' || ch == '\r' {
                            let split_index = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            let remainder = state.lines[state.cursor_row].split_off(split_index);
                            state.cursor_row += 1;
                            state.lines.insert(state.cursor_row, remainder);
                            state.cursor_col = 0;
                        } else {
                            if let Some(max) = state.max_length {
                                let total: usize =
                                    state.lines.iter().map(|l| l.chars().count()).sum();
                                if total >= max {
                                    break;
                                }
                            }
                            let index = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            state.lines[state.cursor_row].insert(index, ch);
                            state.cursor_col += 1;
                        }
                    }
                    consumed_indices.push(i);
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        let vlines = textarea_build_visual_lines(&state.lines, wrap_w);
        let (cursor_vrow, cursor_vcol) =
            textarea_logical_to_visual(&vlines, state.cursor_row, state.cursor_col);

        if cursor_vrow < state.scroll_offset {
            state.scroll_offset = cursor_vrow;
        }
        if cursor_vrow >= state.scroll_offset + visible_rows as usize {
            state.scroll_offset = cursor_vrow + 1 - visible_rows as usize;
        }

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });

        let show_cursor = focused && (self.tick / 30) % 2 == 0;
        for vi in 0..visible_rows as usize {
            let actual_vi = state.scroll_offset + vi;
            let (seg_text, is_cursor_line) = if let Some(vl) = vlines.get(actual_vi) {
                let line = &state.lines[vl.logical_row];
                let text: String = line
                    .chars()
                    .skip(vl.char_start)
                    .take(vl.char_count)
                    .collect();
                (text, actual_vi == cursor_vrow)
            } else {
                (String::new(), false)
            };

            let mut rendered = seg_text.clone();
            let mut style = if seg_text.is_empty() {
                Style::new().fg(self.theme.text_dim)
            } else {
                Style::new().fg(self.theme.text)
            };

            if is_cursor_line {
                rendered.clear();
                for (idx, ch) in seg_text.chars().enumerate() {
                    if show_cursor && idx == cursor_vcol {
                        rendered.push('▎');
                    }
                    rendered.push(ch);
                }
                if show_cursor && cursor_vcol >= seg_text.chars().count() {
                    rendered.push('▎');
                }
                style = Style::new().fg(self.theme.text);
            }

            self.styled(rendered, style);
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a progress bar (20 chars wide). `ratio` is clamped to `0.0..=1.0`.
    ///
    /// Uses block characters (`█` filled, `░` empty). For a custom width use
    /// [`Context::progress_bar`].
    pub fn progress(&mut self, ratio: f64) -> &mut Self {
        self.progress_bar(ratio, 20)
    }

    /// Render a progress bar with a custom character width.
    ///
    /// `ratio` is clamped to `0.0..=1.0`. `width` is the total number of
    /// characters rendered.
    pub fn progress_bar(&mut self, ratio: f64, width: u32) -> &mut Self {
        let clamped = ratio.clamp(0.0, 1.0);
        let filled = (clamped * width as f64).round() as u32;
        let empty = width.saturating_sub(filled);
        let mut bar = String::new();
        for _ in 0..filled {
            bar.push('█');
        }
        for _ in 0..empty {
            bar.push('░');
        }
        self.text(bar)
    }

    /// Render a horizontal bar chart from `(label, value)` pairs.
    ///
    /// Bars are normalized against the largest value and rendered with `█` up to
    /// `max_width` characters.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # slt::run(|ui: &mut slt::Context| {
    /// let data = [
    ///     ("Sales", 160.0),
    ///     ("Revenue", 120.0),
    ///     ("Users", 220.0),
    ///     ("Costs", 60.0),
    /// ];
    /// ui.bar_chart(&data, 24);
    ///
    /// For styled bars with per-bar colors, see [`bar_chart_styled`].
    /// # });
    /// ```
    pub fn bar_chart(&mut self, data: &[(&str, f64)], max_width: u32) -> &mut Self {
        if data.is_empty() {
            return self;
        }

        let max_label_width = data
            .iter()
            .map(|(label, _)| UnicodeWidthStr::width(*label))
            .max()
            .unwrap_or(0);
        let max_value = data
            .iter()
            .map(|(_, value)| *value)
            .fold(f64::NEG_INFINITY, f64::max);
        let denom = if max_value > 0.0 { max_value } else { 1.0 };

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });

        for (label, value) in data {
            let label_width = UnicodeWidthStr::width(*label);
            let label_padding = " ".repeat(max_label_width.saturating_sub(label_width));
            let normalized = (*value / denom).clamp(0.0, 1.0);
            let bar_len = (normalized * max_width as f64).round() as usize;
            let bar = "█".repeat(bar_len);

            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
                direction: Direction::Row,
                gap: 1,
                align: Align::Start,
                justify: Justify::Start,
                border: None,
                border_style: Style::new().fg(self.theme.border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
            });
            self.styled(
                format!("{label}{label_padding}"),
                Style::new().fg(self.theme.text),
            );
            self.styled(bar, Style::new().fg(self.theme.primary));
            self.styled(
                format_compact_number(*value),
                Style::new().fg(self.theme.text_dim),
            );
            self.commands.push(Command::EndContainer);
            self.last_text_idx = None;
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a styled bar chart with per-bar colors, grouping, and direction control.
    ///
    /// # Example
    /// ```ignore
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::{Bar, Color};
    /// let bars = vec![
    ///     Bar::new("Q1", 32.0).color(Color::Cyan),
    ///     Bar::new("Q2", 46.0).color(Color::Green),
    ///     Bar::new("Q3", 28.0).color(Color::Yellow),
    ///     Bar::new("Q4", 54.0).color(Color::Red),
    /// ];
    /// ui.bar_chart_styled(&bars, 30, slt::BarDirection::Horizontal);
    /// # });
    /// ```
    pub fn bar_chart_styled(
        &mut self,
        bars: &[Bar],
        max_width: u32,
        direction: BarDirection,
    ) -> &mut Self {
        if bars.is_empty() {
            return self;
        }

        let max_value = bars
            .iter()
            .map(|bar| bar.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let denom = if max_value > 0.0 { max_value } else { 1.0 };

        match direction {
            BarDirection::Horizontal => {
                let max_label_width = bars
                    .iter()
                    .map(|bar| UnicodeWidthStr::width(bar.label.as_str()))
                    .max()
                    .unwrap_or(0);

                self.interaction_count += 1;
                self.commands.push(Command::BeginContainer {
                    direction: Direction::Column,
                    gap: 0,
                    align: Align::Start,
                    justify: Justify::Start,
                    border: None,
                    border_style: Style::new().fg(self.theme.border),
                    bg_color: None,
                    padding: Padding::default(),
                    margin: Margin::default(),
                    constraints: Constraints::default(),
                    title: None,
                    grow: 0,
                });

                for bar in bars {
                    let label_width = UnicodeWidthStr::width(bar.label.as_str());
                    let label_padding = " ".repeat(max_label_width.saturating_sub(label_width));
                    let normalized = (bar.value / denom).clamp(0.0, 1.0);
                    let bar_len = (normalized * max_width as f64).round() as usize;
                    let bar_text = "█".repeat(bar_len);
                    let color = bar.color.unwrap_or(self.theme.primary);

                    self.interaction_count += 1;
                    self.commands.push(Command::BeginContainer {
                        direction: Direction::Row,
                        gap: 1,
                        align: Align::Start,
                        justify: Justify::Start,
                        border: None,
                        border_style: Style::new().fg(self.theme.border),
                        bg_color: None,
                        padding: Padding::default(),
                        margin: Margin::default(),
                        constraints: Constraints::default(),
                        title: None,
                        grow: 0,
                    });
                    self.styled(
                        format!("{}{label_padding}", bar.label),
                        Style::new().fg(self.theme.text),
                    );
                    self.styled(bar_text, Style::new().fg(color));
                    self.styled(
                        format_compact_number(bar.value),
                        Style::new().fg(self.theme.text_dim),
                    );
                    self.commands.push(Command::EndContainer);
                    self.last_text_idx = None;
                }

                self.commands.push(Command::EndContainer);
                self.last_text_idx = None;
            }
            BarDirection::Vertical => {
                const FRACTION_BLOCKS: [char; 8] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇'];

                let chart_height = max_width.max(1) as usize;
                let value_labels: Vec<String> = bars
                    .iter()
                    .map(|bar| format_compact_number(bar.value))
                    .collect();
                let col_width = bars
                    .iter()
                    .zip(value_labels.iter())
                    .map(|(bar, value)| {
                        UnicodeWidthStr::width(bar.label.as_str())
                            .max(UnicodeWidthStr::width(value.as_str()))
                            .max(1)
                    })
                    .max()
                    .unwrap_or(1);

                let bar_units: Vec<usize> = bars
                    .iter()
                    .map(|bar| {
                        let normalized = (bar.value / denom).clamp(0.0, 1.0);
                        (normalized * chart_height as f64 * 8.0).round() as usize
                    })
                    .collect();

                self.interaction_count += 1;
                self.commands.push(Command::BeginContainer {
                    direction: Direction::Column,
                    gap: 0,
                    align: Align::Start,
                    justify: Justify::Start,
                    border: None,
                    border_style: Style::new().fg(self.theme.border),
                    bg_color: None,
                    padding: Padding::default(),
                    margin: Margin::default(),
                    constraints: Constraints::default(),
                    title: None,
                    grow: 0,
                });

                self.interaction_count += 1;
                self.commands.push(Command::BeginContainer {
                    direction: Direction::Row,
                    gap: 1,
                    align: Align::Start,
                    justify: Justify::Start,
                    border: None,
                    border_style: Style::new().fg(self.theme.border),
                    bg_color: None,
                    padding: Padding::default(),
                    margin: Margin::default(),
                    constraints: Constraints::default(),
                    title: None,
                    grow: 0,
                });
                for value in &value_labels {
                    self.styled(
                        center_text(value, col_width),
                        Style::new().fg(self.theme.text_dim),
                    );
                }
                self.commands.push(Command::EndContainer);
                self.last_text_idx = None;

                for row in (0..chart_height).rev() {
                    self.interaction_count += 1;
                    self.commands.push(Command::BeginContainer {
                        direction: Direction::Row,
                        gap: 1,
                        align: Align::Start,
                        justify: Justify::Start,
                        border: None,
                        border_style: Style::new().fg(self.theme.border),
                        bg_color: None,
                        padding: Padding::default(),
                        margin: Margin::default(),
                        constraints: Constraints::default(),
                        title: None,
                        grow: 0,
                    });

                    let row_base = row * 8;
                    for (bar, units) in bars.iter().zip(bar_units.iter()) {
                        let fill = if *units <= row_base {
                            ' '
                        } else {
                            let delta = *units - row_base;
                            if delta >= 8 {
                                '█'
                            } else {
                                FRACTION_BLOCKS[delta]
                            }
                        };

                        self.styled(
                            center_text(&fill.to_string(), col_width),
                            Style::new().fg(bar.color.unwrap_or(self.theme.primary)),
                        );
                    }

                    self.commands.push(Command::EndContainer);
                    self.last_text_idx = None;
                }

                self.interaction_count += 1;
                self.commands.push(Command::BeginContainer {
                    direction: Direction::Row,
                    gap: 1,
                    align: Align::Start,
                    justify: Justify::Start,
                    border: None,
                    border_style: Style::new().fg(self.theme.border),
                    bg_color: None,
                    padding: Padding::default(),
                    margin: Margin::default(),
                    constraints: Constraints::default(),
                    title: None,
                    grow: 0,
                });
                for bar in bars {
                    self.styled(
                        center_text(&bar.label, col_width),
                        Style::new().fg(self.theme.text),
                    );
                }
                self.commands.push(Command::EndContainer);
                self.last_text_idx = None;

                self.commands.push(Command::EndContainer);
                self.last_text_idx = None;
            }
        }

        self
    }

    /// Render a grouped bar chart.
    ///
    /// Each group contains multiple bars rendered side by side. Useful for
    /// comparing categories across groups (e.g., quarterly revenue by product).
    ///
    /// # Example
    /// ```ignore
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::{Bar, BarGroup, Color};
    /// let groups = vec![
    ///     BarGroup::new("2023", vec![Bar::new("Rev", 100.0).color(Color::Cyan), Bar::new("Cost", 60.0).color(Color::Red)]),
    ///     BarGroup::new("2024", vec![Bar::new("Rev", 140.0).color(Color::Cyan), Bar::new("Cost", 80.0).color(Color::Red)]),
    /// ];
    /// ui.bar_chart_grouped(&groups, 40);
    /// # });
    /// ```
    pub fn bar_chart_grouped(&mut self, groups: &[BarGroup], max_width: u32) -> &mut Self {
        if groups.is_empty() {
            return self;
        }

        let all_bars: Vec<&Bar> = groups.iter().flat_map(|group| group.bars.iter()).collect();
        if all_bars.is_empty() {
            return self;
        }

        let max_label_width = all_bars
            .iter()
            .map(|bar| UnicodeWidthStr::width(bar.label.as_str()))
            .max()
            .unwrap_or(0);
        let max_value = all_bars
            .iter()
            .map(|bar| bar.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let denom = if max_value > 0.0 { max_value } else { 1.0 };

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 1,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });

        for group in groups {
            self.styled(group.label.clone(), Style::new().bold().fg(self.theme.text));

            for bar in &group.bars {
                let label_width = UnicodeWidthStr::width(bar.label.as_str());
                let label_padding = " ".repeat(max_label_width.saturating_sub(label_width));
                let normalized = (bar.value / denom).clamp(0.0, 1.0);
                let bar_len = (normalized * max_width as f64).round() as usize;
                let bar_text = "█".repeat(bar_len);

                self.interaction_count += 1;
                self.commands.push(Command::BeginContainer {
                    direction: Direction::Row,
                    gap: 1,
                    align: Align::Start,
                    justify: Justify::Start,
                    border: None,
                    border_style: Style::new().fg(self.theme.border),
                    bg_color: None,
                    padding: Padding::default(),
                    margin: Margin::default(),
                    constraints: Constraints::default(),
                    title: None,
                    grow: 0,
                });
                self.styled(
                    format!("  {}{label_padding}", bar.label),
                    Style::new().fg(self.theme.text),
                );
                self.styled(
                    bar_text,
                    Style::new().fg(bar.color.unwrap_or(self.theme.primary)),
                );
                self.styled(
                    format_compact_number(bar.value),
                    Style::new().fg(self.theme.text_dim),
                );
                self.commands.push(Command::EndContainer);
                self.last_text_idx = None;
            }
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a single-line sparkline from numeric data.
    ///
    /// Uses the last `width` points (or fewer if the data is shorter) and maps
    /// each point to one of `▁▂▃▄▅▆▇█`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # slt::run(|ui: &mut slt::Context| {
    /// let samples = [12.0, 9.0, 14.0, 18.0, 16.0, 21.0, 20.0, 24.0];
    /// ui.sparkline(&samples, 16);
    ///
    /// For per-point colors and missing values, see [`sparkline_styled`].
    /// # });
    /// ```
    pub fn sparkline(&mut self, data: &[f64], width: u32) -> &mut Self {
        const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        let w = width as usize;
        let window = if data.len() > w {
            &data[data.len() - w..]
        } else {
            data
        };

        if window.is_empty() {
            return self;
        }

        let min = window.iter().copied().fold(f64::INFINITY, f64::min);
        let max = window.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;

        let line: String = window
            .iter()
            .map(|&value| {
                let normalized = if range == 0.0 {
                    0.5
                } else {
                    (value - min) / range
                };
                let idx = (normalized * 7.0).round() as usize;
                BLOCKS[idx.min(7)]
            })
            .collect();

        self.styled(line, Style::new().fg(self.theme.primary))
    }

    /// Render a sparkline with per-point colors.
    ///
    /// Each point can have its own color via `(f64, Option<Color>)` tuples.
    /// Use `f64::NAN` for absent values (rendered as spaces).
    ///
    /// # Example
    /// ```ignore
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// let data: Vec<(f64, Option<Color>)> = vec![
    ///     (12.0, Some(Color::Green)),
    ///     (9.0, Some(Color::Red)),
    ///     (14.0, Some(Color::Green)),
    ///     (f64::NAN, None),
    ///     (18.0, Some(Color::Cyan)),
    /// ];
    /// ui.sparkline_styled(&data, 16);
    /// # });
    /// ```
    pub fn sparkline_styled(&mut self, data: &[(f64, Option<Color>)], width: u32) -> &mut Self {
        const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        let w = width as usize;
        let window = if data.len() > w {
            &data[data.len() - w..]
        } else {
            data
        };

        if window.is_empty() {
            return self;
        }

        let mut finite_values = window
            .iter()
            .map(|(value, _)| *value)
            .filter(|value| !value.is_nan());
        let Some(first) = finite_values.next() else {
            return self.styled(
                " ".repeat(window.len()),
                Style::new().fg(self.theme.text_dim),
            );
        };

        let mut min = first;
        let mut max = first;
        for value in finite_values {
            min = f64::min(min, value);
            max = f64::max(max, value);
        }
        let range = max - min;

        let mut cells: Vec<(char, Color)> = Vec::with_capacity(window.len());
        for (value, color) in window {
            if value.is_nan() {
                cells.push((' ', self.theme.text_dim));
                continue;
            }

            let normalized = if range == 0.0 {
                0.5
            } else {
                ((*value - min) / range).clamp(0.0, 1.0)
            };
            let idx = (normalized * 7.0).round() as usize;
            cells.push((BLOCKS[idx.min(7)], color.unwrap_or(self.theme.primary)));
        }

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 0,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });

        let mut seg = String::new();
        let mut seg_color = cells[0].1;
        for (ch, color) in cells {
            if color != seg_color {
                self.styled(seg, Style::new().fg(seg_color));
                seg = String::new();
                seg_color = color;
            }
            seg.push(ch);
        }
        if !seg.is_empty() {
            self.styled(seg, Style::new().fg(seg_color));
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a multi-row line chart using braille characters.
    ///
    /// `width` and `height` are terminal cell dimensions. Internally this uses
    /// braille dot resolution (`width*2` x `height*4`) for smoother plotting.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # slt::run(|ui: &mut slt::Context| {
    /// let data = [1.0, 3.0, 2.0, 5.0, 4.0, 6.0, 3.0, 7.0];
    /// ui.line_chart(&data, 40, 8);
    /// # });
    /// ```
    pub fn line_chart(&mut self, data: &[f64], width: u32, height: u32) -> &mut Self {
        if data.is_empty() || width == 0 || height == 0 {
            return self;
        }

        let cols = width as usize;
        let rows = height as usize;
        let px_w = cols * 2;
        let px_h = rows * 4;

        let min = data.iter().copied().fold(f64::INFINITY, f64::min);
        let max = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let range = if (max - min).abs() < f64::EPSILON {
            1.0
        } else {
            max - min
        };

        let points: Vec<usize> = (0..px_w)
            .map(|px| {
                let data_idx = if px_w <= 1 {
                    0.0
                } else {
                    px as f64 * (data.len() - 1) as f64 / (px_w - 1) as f64
                };
                let idx = data_idx.floor() as usize;
                let frac = data_idx - idx as f64;
                let value = if idx + 1 < data.len() {
                    data[idx] * (1.0 - frac) + data[idx + 1] * frac
                } else {
                    data[idx.min(data.len() - 1)]
                };

                let normalized = (value - min) / range;
                let py = ((1.0 - normalized) * (px_h - 1) as f64).round() as usize;
                py.min(px_h - 1)
            })
            .collect();

        const LEFT_BITS: [u32; 4] = [0x01, 0x02, 0x04, 0x40];
        const RIGHT_BITS: [u32; 4] = [0x08, 0x10, 0x20, 0x80];

        let mut grid = vec![vec![0u32; cols]; rows];

        for i in 0..points.len() {
            let px = i;
            let py = points[i];
            let char_col = px / 2;
            let char_row = py / 4;
            let sub_col = px % 2;
            let sub_row = py % 4;

            if char_col < cols && char_row < rows {
                grid[char_row][char_col] |= if sub_col == 0 {
                    LEFT_BITS[sub_row]
                } else {
                    RIGHT_BITS[sub_row]
                };
            }

            if i + 1 < points.len() {
                let py_next = points[i + 1];
                let (y_start, y_end) = if py <= py_next {
                    (py, py_next)
                } else {
                    (py_next, py)
                };
                for y in y_start..=y_end {
                    let cell_row = y / 4;
                    let sub_y = y % 4;
                    if char_col < cols && cell_row < rows {
                        grid[cell_row][char_col] |= if sub_col == 0 {
                            LEFT_BITS[sub_y]
                        } else {
                            RIGHT_BITS[sub_y]
                        };
                    }
                }
            }
        }

        let style = Style::new().fg(self.theme.primary);
        for row in grid {
            let line: String = row
                .iter()
                .map(|&bits| char::from_u32(0x2800 + bits).unwrap_or(' '))
                .collect();
            self.styled(line, style);
        }

        self
    }

    /// Render a braille drawing canvas.
    ///
    /// The closure receives a [`CanvasContext`] for pixel-level drawing. Each
    /// terminal cell maps to a 2x4 braille dot matrix, giving `width*2` x
    /// `height*4` pixel resolution.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.canvas(40, 10, |cv| {
    ///     cv.line(0, 0, cv.width() - 1, cv.height() - 1);
    ///     cv.circle(40, 20, 15);
    /// });
    /// # });
    /// ```
    pub fn canvas(
        &mut self,
        width: u32,
        height: u32,
        draw: impl FnOnce(&mut CanvasContext),
    ) -> &mut Self {
        if width == 0 || height == 0 {
            return self;
        }

        let mut canvas = CanvasContext::new(width as usize, height as usize);
        draw(&mut canvas);

        for segments in canvas.render() {
            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
                direction: Direction::Row,
                gap: 0,
                align: Align::Start,
                justify: Justify::Start,
                border: None,
                border_style: Style::new(),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
            });
            for (text, color) in segments {
                let c = if color == Color::Reset {
                    self.theme.primary
                } else {
                    color
                };
                self.styled(text, Style::new().fg(c));
            }
            self.commands.push(Command::EndContainer);
            self.last_text_idx = None;
        }

        self
    }

    /// Render a multi-series chart with axes, legend, and auto-scaling.
    pub fn chart(
        &mut self,
        configure: impl FnOnce(&mut ChartBuilder),
        width: u32,
        height: u32,
    ) -> &mut Self {
        if width == 0 || height == 0 {
            return self;
        }

        let axis_style = Style::new().fg(self.theme.text_dim);
        let mut builder = ChartBuilder::new(width, height, axis_style, axis_style);
        configure(&mut builder);

        let config = builder.build();
        let rows = render_chart(&config);

        for row in rows {
            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
                direction: Direction::Row,
                gap: 0,
                align: Align::Start,
                justify: Justify::Start,
                border: None,
                border_style: Style::new().fg(self.theme.border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
            });
            for (text, style) in row.segments {
                self.styled(text, style);
            }
            self.commands.push(Command::EndContainer);
            self.last_text_idx = None;
        }

        self
    }

    /// Render a histogram from raw data with auto-binning.
    pub fn histogram(&mut self, data: &[f64], width: u32, height: u32) -> &mut Self {
        self.histogram_with(data, |_| {}, width, height)
    }

    /// Render a histogram with configuration options.
    pub fn histogram_with(
        &mut self,
        data: &[f64],
        configure: impl FnOnce(&mut HistogramBuilder),
        width: u32,
        height: u32,
    ) -> &mut Self {
        if width == 0 || height == 0 {
            return self;
        }

        let mut options = HistogramBuilder::default();
        configure(&mut options);
        let axis_style = Style::new().fg(self.theme.text_dim);
        let config = build_histogram_config(data, &options, width, height, axis_style);
        let rows = render_chart(&config);

        for row in rows {
            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
                direction: Direction::Row,
                gap: 0,
                align: Align::Start,
                justify: Justify::Start,
                border: None,
                border_style: Style::new().fg(self.theme.border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
            });
            for (text, style) in row.segments {
                self.styled(text, style);
            }
            self.commands.push(Command::EndContainer);
            self.last_text_idx = None;
        }

        self
    }

    /// Render children in a fixed grid with the given number of columns.
    ///
    /// Children are placed left-to-right, top-to-bottom. Each cell has equal
    /// width (`area_width / cols`). Rows wrap automatically.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.grid(3, |ui| {
    ///     for i in 0..9 {
    ///         ui.text(format!("Cell {i}"));
    ///     }
    /// });
    /// # });
    /// ```
    pub fn grid(&mut self, cols: u32, f: impl FnOnce(&mut Context)) -> Response {
        slt_assert(cols > 0, "grid() requires at least 1 column");
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let border = self.theme.border;

        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });

        let children_start = self.commands.len();
        f(self);
        let child_commands: Vec<Command> = self.commands.drain(children_start..).collect();

        let mut elements: Vec<Vec<Command>> = Vec::new();
        let mut iter = child_commands.into_iter().peekable();
        while let Some(cmd) = iter.next() {
            match cmd {
                Command::BeginContainer { .. } | Command::BeginScrollable { .. } => {
                    let mut depth = 1_u32;
                    let mut element = vec![cmd];
                    for next in iter.by_ref() {
                        match next {
                            Command::BeginContainer { .. } | Command::BeginScrollable { .. } => {
                                depth += 1;
                            }
                            Command::EndContainer => {
                                depth = depth.saturating_sub(1);
                            }
                            _ => {}
                        }
                        let at_end = matches!(next, Command::EndContainer) && depth == 0;
                        element.push(next);
                        if at_end {
                            break;
                        }
                    }
                    elements.push(element);
                }
                Command::EndContainer => {}
                _ => elements.push(vec![cmd]),
            }
        }

        let cols = cols.max(1) as usize;
        for row in elements.chunks(cols) {
            self.interaction_count += 1;
            self.commands.push(Command::BeginContainer {
                direction: Direction::Row,
                gap: 0,
                align: Align::Start,
                justify: Justify::Start,
                border: None,
                border_style: Style::new().fg(border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
            });

            for element in row {
                self.interaction_count += 1;
                self.commands.push(Command::BeginContainer {
                    direction: Direction::Column,
                    gap: 0,
                    align: Align::Start,
                    justify: Justify::Start,
                    border: None,
                    border_style: Style::new().fg(border),
                    bg_color: None,
                    padding: Padding::default(),
                    margin: Margin::default(),
                    constraints: Constraints::default(),
                    title: None,
                    grow: 1,
                });
                self.commands.extend(element.iter().cloned());
                self.commands.push(Command::EndContainer);
            }

            self.commands.push(Command::EndContainer);
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self.response_for(interaction_id)
    }

    /// Render a selectable list. Handles Up/Down (and `k`/`j`) navigation when focused.
    ///
    /// The selected item is highlighted with the theme's primary color. If the
    /// list is empty, nothing is rendered.
    pub fn list(&mut self, state: &mut ListState) -> &mut Self {
        if state.items.is_empty() {
            state.selected = 0;
            return self;
        }

        state.selected = state.selected.min(state.items.len().saturating_sub(1));

        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.selected = state.selected.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            state.selected =
                                (state.selected + 1).min(state.items.len().saturating_sub(1));
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if let Some(rect) = self.prev_hit_map.get(interaction_id).copied() {
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Mouse(mouse) = event {
                    if !matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                        continue;
                    }
                    let in_bounds = mouse.x >= rect.x
                        && mouse.x < rect.right()
                        && mouse.y >= rect.y
                        && mouse.y < rect.bottom();
                    if !in_bounds {
                        continue;
                    }
                    let clicked_idx = (mouse.y - rect.y) as usize;
                    if clicked_idx < state.items.len() {
                        state.selected = clicked_idx;
                        self.consumed[i] = true;
                    }
                }
            }
        }

        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });

        for (idx, item) in state.items.iter().enumerate() {
            if idx == state.selected {
                if focused {
                    self.styled(
                        format!("▸ {item}"),
                        Style::new().bold().fg(self.theme.primary),
                    );
                } else {
                    self.styled(format!("▸ {item}"), Style::new().fg(self.theme.primary));
                }
            } else {
                self.styled(format!("  {item}"), Style::new().fg(self.theme.text));
            }
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a data table with column headers. Handles Up/Down selection when focused.
    ///
    /// Column widths are computed automatically from header and cell content.
    /// The selected row is highlighted with the theme's selection colors.
    pub fn table(&mut self, state: &mut TableState) -> &mut Self {
        if state.is_dirty() {
            state.recompute_widths();
        }

        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;

        if focused && !state.rows.is_empty() {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.selected = state.selected.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            state.selected =
                                (state.selected + 1).min(state.rows.len().saturating_sub(1));
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }
            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if !state.rows.is_empty() {
            if let Some(rect) = self.prev_hit_map.get(interaction_id).copied() {
                for (i, event) in self.events.iter().enumerate() {
                    if self.consumed[i] {
                        continue;
                    }
                    if let Event::Mouse(mouse) = event {
                        if !matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                            continue;
                        }
                        let in_bounds = mouse.x >= rect.x
                            && mouse.x < rect.right()
                            && mouse.y >= rect.y
                            && mouse.y < rect.bottom();
                        if !in_bounds {
                            continue;
                        }
                        if mouse.y < rect.y + 2 {
                            continue;
                        }
                        let clicked_idx = (mouse.y - rect.y - 2) as usize;
                        if clicked_idx < state.rows.len() {
                            state.selected = clicked_idx;
                            self.consumed[i] = true;
                        }
                    }
                }
            }
        }

        state.selected = state.selected.min(state.rows.len().saturating_sub(1));

        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });

        let header_line = format_table_row(&state.headers, state.column_widths(), " │ ");
        self.styled(header_line, Style::new().bold().fg(self.theme.text));

        let separator = state
            .column_widths()
            .iter()
            .map(|w| "─".repeat(*w as usize))
            .collect::<Vec<_>>()
            .join("─┼─");
        self.text(separator);

        for (idx, row) in state.rows.iter().enumerate() {
            let line = format_table_row(row, state.column_widths(), " │ ");
            if idx == state.selected {
                let mut style = Style::new()
                    .bg(self.theme.selected_bg)
                    .fg(self.theme.selected_fg);
                if focused {
                    style = style.bold();
                }
                self.styled(line, style);
            } else {
                self.styled(line, Style::new().fg(self.theme.text));
            }
        }

        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a tab bar. Handles Left/Right navigation when focused.
    ///
    /// The active tab is rendered in the theme's primary color. If the labels
    /// list is empty, nothing is rendered.
    pub fn tabs(&mut self, state: &mut TabsState) -> &mut Self {
        if state.labels.is_empty() {
            state.selected = 0;
            return self;
        }

        state.selected = state.selected.min(state.labels.len().saturating_sub(1));
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Left => {
                            state.selected = if state.selected == 0 {
                                state.labels.len().saturating_sub(1)
                            } else {
                                state.selected - 1
                            };
                            consumed_indices.push(i);
                        }
                        KeyCode::Right => {
                            state.selected = (state.selected + 1) % state.labels.len();
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if let Some(rect) = self.prev_hit_map.get(interaction_id).copied() {
            for (i, event) in self.events.iter().enumerate() {
                if self.consumed[i] {
                    continue;
                }
                if let Event::Mouse(mouse) = event {
                    if !matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                        continue;
                    }
                    let in_bounds = mouse.x >= rect.x
                        && mouse.x < rect.right()
                        && mouse.y >= rect.y
                        && mouse.y < rect.bottom();
                    if !in_bounds {
                        continue;
                    }

                    let mut x_offset = 0u32;
                    let rel_x = mouse.x - rect.x;
                    for (idx, label) in state.labels.iter().enumerate() {
                        let tab_width = UnicodeWidthStr::width(label.as_str()) as u32 + 4;
                        if rel_x >= x_offset && rel_x < x_offset + tab_width {
                            state.selected = idx;
                            self.consumed[i] = true;
                            break;
                        }
                        x_offset += tab_width + 1;
                    }
                }
            }
        }

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 1,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        for (idx, label) in state.labels.iter().enumerate() {
            let style = if idx == state.selected {
                let s = Style::new().fg(self.theme.primary).bold();
                if focused {
                    s.underline()
                } else {
                    s
                }
            } else {
                Style::new().fg(self.theme.text_dim)
            };
            self.styled(format!("[ {label} ]"), style);
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a clickable button. Returns `true` when activated via Enter, Space, or mouse click.
    ///
    /// The button is styled with the theme's primary color when focused and the
    /// accent color when hovered.
    pub fn button(&mut self, label: impl Into<String>) -> bool {
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);

        let mut activated = response.clicked;
        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        activated = true;
                        consumed_indices.push(i);
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        let hovered = response.hovered;
        let style = if focused {
            Style::new().fg(self.theme.primary).bold()
        } else if hovered {
            Style::new().fg(self.theme.accent)
        } else {
            Style::new().fg(self.theme.text)
        };
        let hover_bg = if hovered || focused {
            Some(self.theme.surface_hover)
        } else {
            None
        };

        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 0,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            bg_color: hover_bg,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        self.styled(format!("[ {} ]", label.into()), style);
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        activated
    }

    /// Render a styled button variant. Returns `true` when activated.
    ///
    /// Use [`ButtonVariant::Primary`] for call-to-action, [`ButtonVariant::Danger`]
    /// for destructive actions, or [`ButtonVariant::Outline`] for secondary actions.
    pub fn button_with(&mut self, label: impl Into<String>, variant: ButtonVariant) -> bool {
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);

        let mut activated = response.clicked;
        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        activated = true;
                        consumed_indices.push(i);
                    }
                }
            }
            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        let label = label.into();
        let hover_bg = if response.hovered || focused {
            Some(self.theme.surface_hover)
        } else {
            None
        };
        let (text, style, bg_color, border) = match variant {
            ButtonVariant::Default => {
                let style = if focused {
                    Style::new().fg(self.theme.primary).bold()
                } else if response.hovered {
                    Style::new().fg(self.theme.accent)
                } else {
                    Style::new().fg(self.theme.text)
                };
                (format!("[ {label} ]"), style, hover_bg, None)
            }
            ButtonVariant::Primary => {
                let style = if focused {
                    Style::new().fg(self.theme.bg).bg(self.theme.primary).bold()
                } else if response.hovered {
                    Style::new().fg(self.theme.bg).bg(self.theme.accent)
                } else {
                    Style::new().fg(self.theme.bg).bg(self.theme.primary)
                };
                (format!(" {label} "), style, hover_bg, None)
            }
            ButtonVariant::Danger => {
                let style = if focused {
                    Style::new().fg(self.theme.bg).bg(self.theme.error).bold()
                } else if response.hovered {
                    Style::new().fg(self.theme.bg).bg(self.theme.warning)
                } else {
                    Style::new().fg(self.theme.bg).bg(self.theme.error)
                };
                (format!(" {label} "), style, hover_bg, None)
            }
            ButtonVariant::Outline => {
                let border_color = if focused {
                    self.theme.primary
                } else if response.hovered {
                    self.theme.accent
                } else {
                    self.theme.border
                };
                let style = if focused {
                    Style::new().fg(self.theme.primary).bold()
                } else if response.hovered {
                    Style::new().fg(self.theme.accent)
                } else {
                    Style::new().fg(self.theme.text)
                };
                (
                    format!(" {label} "),
                    style,
                    hover_bg,
                    Some((Border::Rounded, Style::new().fg(border_color))),
                )
            }
        };

        let (btn_border, btn_border_style) = border.unwrap_or((Border::Rounded, Style::new()));
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 0,
            align: Align::Center,
            justify: Justify::Center,
            border: if border.is_some() {
                Some(btn_border)
            } else {
                None
            },
            border_style: btn_border_style,
            bg_color,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        self.styled(text, style);
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        activated
    }

    /// Render a checkbox. Toggles the bool on Enter, Space, or click.
    ///
    /// The checked state is shown with the theme's success color. When focused,
    /// a `▸` prefix is added.
    pub fn checkbox(&mut self, label: impl Into<String>, checked: &mut bool) -> &mut Self {
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);
        let mut should_toggle = response.clicked;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        should_toggle = true;
                        consumed_indices.push(i);
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if should_toggle {
            *checked = !*checked;
        }

        let hover_bg = if response.hovered || focused {
            Some(self.theme.surface_hover)
        } else {
            None
        };
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 1,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            bg_color: hover_bg,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        let marker_style = if *checked {
            Style::new().fg(self.theme.success)
        } else {
            Style::new().fg(self.theme.text_dim)
        };
        let marker = if *checked { "[x]" } else { "[ ]" };
        let label_text = label.into();
        if focused {
            self.styled(format!("▸ {marker}"), marker_style.bold());
            self.styled(label_text, Style::new().fg(self.theme.text).bold());
        } else {
            self.styled(marker, marker_style);
            self.styled(label_text, Style::new().fg(self.theme.text));
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render an on/off toggle switch.
    ///
    /// Toggles `on` when activated via Enter, Space, or click. The switch
    /// renders as `●━━ ON` or `━━● OFF` colored with the theme's success or
    /// dim color respectively.
    pub fn toggle(&mut self, label: impl Into<String>, on: &mut bool) -> &mut Self {
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);
        let mut should_toggle = response.clicked;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        should_toggle = true;
                        consumed_indices.push(i);
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if should_toggle {
            *on = !*on;
        }

        let hover_bg = if response.hovered || focused {
            Some(self.theme.surface_hover)
        } else {
            None
        };
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 2,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            bg_color: hover_bg,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        let label_text = label.into();
        let switch = if *on { "●━━ ON" } else { "━━● OFF" };
        let switch_style = if *on {
            Style::new().fg(self.theme.success)
        } else {
            Style::new().fg(self.theme.text_dim)
        };
        if focused {
            self.styled(
                format!("▸ {label_text}"),
                Style::new().fg(self.theme.text).bold(),
            );
            self.styled(switch, switch_style.bold());
        } else {
            self.styled(label_text, Style::new().fg(self.theme.text));
            self.styled(switch, switch_style);
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a horizontal divider line.
    ///
    /// The line is drawn with the theme's border color and expands to fill the
    /// container width.
    pub fn separator(&mut self) -> &mut Self {
        self.commands.push(Command::Text {
            content: "─".repeat(200),
            style: Style::new().fg(self.theme.border).dim(),
            grow: 0,
            align: Align::Start,
            wrap: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render a help bar showing keybinding hints.
    ///
    /// `bindings` is a slice of `(key, action)` pairs. Keys are rendered in the
    /// theme's primary color; actions in the dim text color. Pairs are separated
    /// by a `·` character.
    pub fn help(&mut self, bindings: &[(&str, &str)]) -> &mut Self {
        if bindings.is_empty() {
            return self;
        }

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 2,
            align: Align::Start,
            justify: Justify::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        for (idx, (key, action)) in bindings.iter().enumerate() {
            if idx > 0 {
                self.styled("·", Style::new().fg(self.theme.text_dim));
            }
            self.styled(*key, Style::new().bold().fg(self.theme.primary));
            self.styled(*action, Style::new().fg(self.theme.text_dim));
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    // ── events ───────────────────────────────────────────────────────

    /// Check if a character key was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    pub fn key(&self, c: char) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i] && matches!(e, Event::Key(k) if k.code == KeyCode::Char(c))
        })
    }

    /// Check if a specific key code was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    pub fn key_code(&self, code: KeyCode) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        self.events
            .iter()
            .enumerate()
            .any(|(i, e)| !self.consumed[i] && matches!(e, Event::Key(k) if k.code == code))
    }

    /// Check if a character key with specific modifiers was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    pub fn key_mod(&self, c: char, modifiers: KeyModifiers) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.code == KeyCode::Char(c) && k.modifiers.contains(modifiers))
        })
    }

    /// Return the position of a left mouse button down event this frame, if any.
    ///
    /// Returns `None` if no unconsumed mouse-down event occurred.
    pub fn mouse_down(&self) -> Option<(u32, u32)> {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return None;
        }
        self.events.iter().enumerate().find_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            if let Event::Mouse(mouse) = event {
                if matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                    return Some((mouse.x, mouse.y));
                }
            }
            None
        })
    }

    /// Return the current mouse cursor position, if known.
    ///
    /// The position is updated on every mouse move or click event. Returns
    /// `None` until the first mouse event is received.
    pub fn mouse_pos(&self) -> Option<(u32, u32)> {
        self.mouse_pos
    }

    /// Return the first unconsumed paste event text, if any.
    pub fn paste(&self) -> Option<&str> {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return None;
        }
        self.events.iter().enumerate().find_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            if let Event::Paste(ref text) = event {
                return Some(text.as_str());
            }
            None
        })
    }

    /// Check if an unconsumed scroll-up event occurred this frame.
    pub fn scroll_up(&self) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        self.events.iter().enumerate().any(|(i, event)| {
            !self.consumed[i]
                && matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollUp))
        })
    }

    /// Check if an unconsumed scroll-down event occurred this frame.
    pub fn scroll_down(&self) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        self.events.iter().enumerate().any(|(i, event)| {
            !self.consumed[i]
                && matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollDown))
        })
    }

    /// Signal the run loop to exit after this frame.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Get the current theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Change the theme for subsequent rendering.
    ///
    /// All widgets rendered after this call will use the new theme's colors.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    // ── info ─────────────────────────────────────────────────────────

    /// Get the terminal width in cells.
    pub fn width(&self) -> u32 {
        self.area_width
    }

    /// Get the terminal height in cells.
    pub fn height(&self) -> u32 {
        self.area_height
    }

    /// Get the current tick count (increments each frame).
    ///
    /// Useful for animations and time-based logic. The tick starts at 0 and
    /// increases by 1 on every rendered frame.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Return whether the layout debugger is enabled.
    ///
    /// The debugger is toggled with F12 at runtime.
    pub fn debug_enabled(&self) -> bool {
        self.debug
    }
}

#[inline]
fn byte_index_for_char(value: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    value
        .char_indices()
        .nth(char_index)
        .map_or(value.len(), |(idx, _)| idx)
}

fn format_table_row(cells: &[String], widths: &[u32], separator: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (i, width) in widths.iter().enumerate() {
        let cell = cells.get(i).map(String::as_str).unwrap_or("");
        let cell_width = UnicodeWidthStr::width(cell) as u32;
        let padding = (*width).saturating_sub(cell_width) as usize;
        parts.push(format!("{cell}{}", " ".repeat(padding)));
    }
    parts.join(separator)
}

fn format_compact_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        return format!("{value:.0}");
    }

    let mut s = format!("{value:.2}");
    while s.contains('.') && s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

fn center_text(text: &str, width: usize) -> String {
    let text_width = UnicodeWidthStr::width(text);
    if text_width >= width {
        return text.to_string();
    }

    let total = width - text_width;
    let left = total / 2;
    let right = total - left;
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
}

struct TextareaVLine {
    logical_row: usize,
    char_start: usize,
    char_count: usize,
}

fn textarea_build_visual_lines(lines: &[String], wrap_width: u32) -> Vec<TextareaVLine> {
    let mut out = Vec::new();
    for (row, line) in lines.iter().enumerate() {
        if line.is_empty() || wrap_width == u32::MAX {
            out.push(TextareaVLine {
                logical_row: row,
                char_start: 0,
                char_count: line.chars().count(),
            });
            continue;
        }
        let mut seg_start = 0usize;
        let mut seg_chars = 0usize;
        let mut seg_width = 0u32;
        for (idx, ch) in line.chars().enumerate() {
            let cw = UnicodeWidthChar::width(ch).unwrap_or(0) as u32;
            if seg_width + cw > wrap_width && seg_chars > 0 {
                out.push(TextareaVLine {
                    logical_row: row,
                    char_start: seg_start,
                    char_count: seg_chars,
                });
                seg_start = idx;
                seg_chars = 0;
                seg_width = 0;
            }
            seg_chars += 1;
            seg_width += cw;
        }
        out.push(TextareaVLine {
            logical_row: row,
            char_start: seg_start,
            char_count: seg_chars,
        });
    }
    out
}

fn textarea_logical_to_visual(
    vlines: &[TextareaVLine],
    logical_row: usize,
    logical_col: usize,
) -> (usize, usize) {
    for (i, vl) in vlines.iter().enumerate() {
        if vl.logical_row != logical_row {
            continue;
        }
        let seg_end = vl.char_start + vl.char_count;
        if logical_col >= vl.char_start && logical_col < seg_end {
            return (i, logical_col - vl.char_start);
        }
        if logical_col == seg_end {
            let is_last_seg = vlines
                .get(i + 1)
                .map_or(true, |next| next.logical_row != logical_row);
            if is_last_seg {
                return (i, logical_col - vl.char_start);
            }
        }
    }
    (vlines.len().saturating_sub(1), 0)
}

fn textarea_visual_to_logical(
    vlines: &[TextareaVLine],
    visual_row: usize,
    visual_col: usize,
) -> (usize, usize) {
    if let Some(vl) = vlines.get(visual_row) {
        let logical_col = vl.char_start + visual_col.min(vl.char_count);
        (vl.logical_row, logical_col)
    } else {
        (0, 0)
    }
}

fn open_url(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .spawn()?;
    }
    Ok(())
}
