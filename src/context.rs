use crate::chart::{build_histogram_config, render_chart, Candle, ChartBuilder, HistogramBuilder};
use crate::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseKind};
use crate::halfblock::HalfBlockImage;
use crate::layout::{Command, Direction};
use crate::rect::Rect;
use crate::style::{
    Align, Border, BorderSides, Breakpoint, Color, Constraints, ContainerStyle, Justify, Margin,
    Modifiers, Padding, Style, Theme, WidgetColors,
};
use crate::widgets::{
    ApprovalAction, ButtonVariant, CommandPaletteState, ContextItem, FilePickerState, FormField,
    FormState, ListState, MultiSelectState, RadioState, ScrollState, SelectState, SpinnerState,
    StreamingTextState, TableState, TabsState, TextInputState, TextareaState, ToastLevel,
    ToastState, ToolApprovalState, TreeState,
};
use crate::FrameState;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[allow(dead_code)]
fn slt_assert(condition: bool, msg: &str) {
    if !condition {
        panic!("[SLT] {}", msg);
    }
}

#[cfg(debug_assertions)]
#[allow(dead_code, clippy::print_stderr)]
fn slt_warn(msg: &str) {
    eprintln!("\x1b[33m[SLT warning]\x1b[0m {}", msg);
}

#[cfg(not(debug_assertions))]
#[allow(dead_code)]
fn slt_warn(_msg: &str) {}

/// Handle to state created by `use_state()`. Access via `.get(ui)` / `.get_mut(ui)`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct State<T> {
    idx: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: 'static> State<T> {
    /// Read the current value.
    pub fn get<'a>(&self, ui: &'a Context) -> &'a T {
        ui.hook_states[self.idx]
            .downcast_ref::<T>()
            .unwrap_or_else(|| {
                panic!(
                    "use_state type mismatch at hook index {} — expected {}",
                    self.idx,
                    std::any::type_name::<T>()
                )
            })
    }

    /// Mutably access the current value.
    pub fn get_mut<'a>(&self, ui: &'a mut Context) -> &'a mut T {
        ui.hook_states[self.idx]
            .downcast_mut::<T>()
            .unwrap_or_else(|| {
                panic!(
                    "use_state type mismatch at hook index {} — expected {}",
                    self.idx,
                    std::any::type_name::<T>()
                )
            })
    }
}

/// Interaction response returned by all widgets.
///
/// Container methods return a [`Response`]. Check `.clicked`, `.changed`, etc.
/// to react to user interactions.
///
/// # Examples
///
/// ```
/// # use slt::*;
/// # TestBackend::new(80, 24).render(|ui| {
/// let r = ui.row(|ui| {
///     ui.text("Save");
/// });
/// if r.clicked {
///     // handle save
/// }
/// # });
/// ```
#[derive(Debug, Clone, Default)]
#[must_use = "Response contains interaction state — check .clicked, .hovered, or .changed"]
pub struct Response {
    /// Whether the widget was clicked this frame.
    pub clicked: bool,
    /// Whether the mouse is hovering over the widget.
    pub hovered: bool,
    /// Whether the widget's value changed this frame.
    pub changed: bool,
    /// Whether the widget currently has keyboard focus.
    pub focused: bool,
    /// The rectangle the widget occupies after layout.
    pub rect: Rect,
}

impl Response {
    /// Create a Response with all fields false/default.
    pub fn none() -> Self {
        Self::default()
    }
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
    pub text_value: Option<String>,
    pub value_style: Option<Style>,
}

impl Bar {
    /// Create a new bar with a label and value.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
            text_value: None,
            value_style: None,
        }
    }

    /// Set the bar color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn text_value(mut self, text: impl Into<String>) -> Self {
        self.text_value = Some(text.into());
        self
    }

    pub fn value_style(mut self, style: Style) -> Self {
        self.value_style = Some(style);
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BarChartConfig {
    pub direction: BarDirection,
    pub bar_width: u16,
    pub bar_gap: u16,
    pub group_gap: u16,
    pub max_value: Option<f64>,
}

impl Default for BarChartConfig {
    fn default() -> Self {
        Self {
            direction: BarDirection::Horizontal,
            bar_width: 1,
            bar_gap: 0,
            group_gap: 2,
            max_value: None,
        }
    }
}

impl BarChartConfig {
    pub fn direction(&mut self, direction: BarDirection) -> &mut Self {
        self.direction = direction;
        self
    }

    pub fn bar_width(&mut self, bar_width: u16) -> &mut Self {
        self.bar_width = bar_width.max(1);
        self
    }

    pub fn bar_gap(&mut self, bar_gap: u16) -> &mut Self {
        self.bar_gap = bar_gap;
        self
    }

    pub fn group_gap(&mut self, group_gap: u16) -> &mut Self {
        self.group_gap = group_gap;
        self
    }

    pub fn max_value(&mut self, max_value: f64) -> &mut Self {
        self.max_value = Some(max_value);
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
    // NOTE: If you add a mutable per-frame field, also add it to ContextSnapshot in error_boundary_with
    pub(crate) commands: Vec<Command>,
    pub(crate) events: Vec<Event>,
    pub(crate) consumed: Vec<bool>,
    pub(crate) should_quit: bool,
    pub(crate) area_width: u32,
    pub(crate) area_height: u32,
    pub(crate) tick: u64,
    pub(crate) focus_index: usize,
    pub(crate) focus_count: usize,
    pub(crate) hook_states: Vec<Box<dyn std::any::Any>>,
    pub(crate) hook_cursor: usize,
    prev_focus_count: usize,
    scroll_count: usize,
    prev_scroll_infos: Vec<(u32, u32)>,
    prev_scroll_rects: Vec<Rect>,
    interaction_count: usize,
    pub(crate) prev_hit_map: Vec<Rect>,
    pub(crate) group_stack: Vec<String>,
    pub(crate) prev_group_rects: Vec<(String, Rect)>,
    group_count: usize,
    prev_focus_groups: Vec<Option<String>>,
    _prev_focus_rects: Vec<(usize, Rect)>,
    mouse_pos: Option<(u32, u32)>,
    click_pos: Option<(u32, u32)>,
    last_text_idx: Option<usize>,
    overlay_depth: usize,
    pub(crate) modal_active: bool,
    prev_modal_active: bool,
    pub(crate) clipboard_text: Option<String>,
    debug: bool,
    theme: Theme,
    pub(crate) dark_mode: bool,
    pub(crate) is_real_terminal: bool,
    pub(crate) deferred_draws: Vec<Option<RawDrawCallback>>,
    pub(crate) notification_queue: Vec<(String, ToastLevel, u64)>,
    pub(crate) text_color_stack: Vec<Option<Color>>,
}

type RawDrawCallback = Box<dyn FnOnce(&mut crate::buffer::Buffer, Rect)>;

struct ContextSnapshot {
    cmd_count: usize,
    last_text_idx: Option<usize>,
    focus_count: usize,
    interaction_count: usize,
    scroll_count: usize,
    group_count: usize,
    group_stack_len: usize,
    overlay_depth: usize,
    modal_active: bool,
    hook_cursor: usize,
    hook_states_len: usize,
    dark_mode: bool,
    deferred_draws_len: usize,
    notification_queue_len: usize,
    text_color_stack_len: usize,
}

impl ContextSnapshot {
    fn capture(ctx: &Context) -> Self {
        Self {
            cmd_count: ctx.commands.len(),
            last_text_idx: ctx.last_text_idx,
            focus_count: ctx.focus_count,
            interaction_count: ctx.interaction_count,
            scroll_count: ctx.scroll_count,
            group_count: ctx.group_count,
            group_stack_len: ctx.group_stack.len(),
            overlay_depth: ctx.overlay_depth,
            modal_active: ctx.modal_active,
            hook_cursor: ctx.hook_cursor,
            hook_states_len: ctx.hook_states.len(),
            dark_mode: ctx.dark_mode,
            deferred_draws_len: ctx.deferred_draws.len(),
            notification_queue_len: ctx.notification_queue.len(),
            text_color_stack_len: ctx.text_color_stack.len(),
        }
    }

    fn restore(&self, ctx: &mut Context) {
        ctx.commands.truncate(self.cmd_count);
        ctx.last_text_idx = self.last_text_idx;
        ctx.focus_count = self.focus_count;
        ctx.interaction_count = self.interaction_count;
        ctx.scroll_count = self.scroll_count;
        ctx.group_count = self.group_count;
        ctx.group_stack.truncate(self.group_stack_len);
        ctx.overlay_depth = self.overlay_depth;
        ctx.modal_active = self.modal_active;
        ctx.hook_cursor = self.hook_cursor;
        ctx.hook_states.truncate(self.hook_states_len);
        ctx.dark_mode = self.dark_mode;
        ctx.deferred_draws.truncate(self.deferred_draws_len);
        ctx.notification_queue.truncate(self.notification_queue_len);
        ctx.text_color_stack.truncate(self.text_color_stack_len);
    }
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
#[must_use = "ContainerBuilder does nothing until .col(), .row(), .line(), or .draw() is called"]
pub struct ContainerBuilder<'a> {
    ctx: &'a mut Context,
    gap: u32,
    row_gap: Option<u32>,
    col_gap: Option<u32>,
    align: Align,
    align_self_value: Option<Align>,
    justify: Justify,
    border: Option<Border>,
    border_sides: BorderSides,
    border_style: Style,
    bg: Option<Color>,
    text_color: Option<Color>,
    dark_bg: Option<Color>,
    dark_border_style: Option<Style>,
    group_hover_bg: Option<Color>,
    group_hover_border_style: Option<Style>,
    group_name: Option<String>,
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
    pub fn apply(mut self, style: &ContainerStyle) -> Self {
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
        at = w_at => []
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
        at = h_at => []
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
        at = min_w_at => []
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
        at = max_w_at => []
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
        at = gap_at => []
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
        at = grow_at => []
    );

    define_breakpoint_methods!(
        base = p,
        arg = value: u32,
        xs = xs_p => ["Uniform padding applied only at Xs breakpoint (< 40 cols)."],
        sm = sm_p => ["Uniform padding applied only at Sm breakpoint (40-79 cols)."],
        md = md_p => ["Uniform padding applied only at Md breakpoint (80-119 cols)."],
        lg = lg_p => ["Uniform padding applied only at Lg breakpoint (120-159 cols)."],
        xl = xl_p => ["Uniform padding applied only at Xl breakpoint (>= 160 cols)."],
        at = p_at => []
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

    fn group_name(mut self, name: String) -> Self {
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
        self.ctx.interaction_count += 1;
        self.ctx.commands.push(Command::RawDraw {
            draw_id,
            constraints: self.constraints,
            grow: self.grow,
            margin: self.margin,
        });
    }

    fn finish(mut self, direction: Direction, f: impl FnOnce(&mut Context)) -> Response {
        let interaction_id = self.ctx.interaction_count;
        self.ctx.interaction_count += 1;
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
                .group_stack
                .last()
                .map(|name| self.ctx.is_group_focused(name))
                .unwrap_or(false);

        let resolved_bg = if self.ctx.dark_mode {
            self.dark_bg.or(self.bg)
        } else {
            self.bg
        };
        let resolved_border_style = if self.ctx.dark_mode {
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
        self.ctx.text_color_stack.push(self.text_color);
        f(self.ctx);
        self.ctx.text_color_stack.pop();
        self.ctx.commands.push(Command::EndContainer);
        self.ctx.last_text_idx = None;

        if is_group_container {
            self.ctx.group_stack.pop();
            self.ctx.group_count = self.ctx.group_count.saturating_sub(1);
        }

        self.ctx.response_for(interaction_id)
    }
}

impl Context {
    pub(crate) fn new(
        events: Vec<Event>,
        width: u32,
        height: u32,
        state: &mut FrameState,
        theme: Theme,
    ) -> Self {
        let consumed = vec![false; events.len()];

        let mut mouse_pos = state.last_mouse_pos;
        let mut click_pos = None;
        for event in &events {
            if let Event::Mouse(mouse) = event {
                mouse_pos = Some((mouse.x, mouse.y));
                if matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                    click_pos = Some((mouse.x, mouse.y));
                }
            }
        }

        let mut focus_index = state.focus_index;
        if let Some((mx, my)) = click_pos {
            let mut best: Option<(usize, u64)> = None;
            for &(fid, rect) in &state.prev_focus_rects {
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
            tick: state.tick,
            focus_index,
            focus_count: 0,
            hook_states: std::mem::take(&mut state.hook_states),
            hook_cursor: 0,
            prev_focus_count: state.prev_focus_count,
            scroll_count: 0,
            prev_scroll_infos: std::mem::take(&mut state.prev_scroll_infos),
            prev_scroll_rects: std::mem::take(&mut state.prev_scroll_rects),
            interaction_count: 0,
            prev_hit_map: std::mem::take(&mut state.prev_hit_map),
            group_stack: Vec::new(),
            prev_group_rects: std::mem::take(&mut state.prev_group_rects),
            group_count: 0,
            prev_focus_groups: std::mem::take(&mut state.prev_focus_groups),
            _prev_focus_rects: std::mem::take(&mut state.prev_focus_rects),
            mouse_pos,
            click_pos,
            last_text_idx: None,
            overlay_depth: 0,
            modal_active: false,
            prev_modal_active: state.prev_modal_active,
            clipboard_text: None,
            debug: state.debug_mode,
            theme,
            dark_mode: theme.is_dark,
            is_real_terminal: false,
            deferred_draws: Vec::new(),
            notification_queue: std::mem::take(&mut state.notification_queue),
            text_color_stack: Vec::new(),
        }
    }

    pub(crate) fn process_focus_keys(&mut self) {
        for (i, event) in self.events.iter().enumerate() {
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
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
        let snapshot = ContextSnapshot::capture(self);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            f(self);
        }));

        match result {
            Ok(()) => {}
            Err(panic_info) => {
                if self.is_real_terminal {
                    let _ = crossterm::terminal::enable_raw_mode();
                    let _ = crossterm::execute!(
                        std::io::stdout(),
                        crossterm::terminal::EnterAlternateScreen
                    );
                }

                snapshot.restore(self);

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
            return Response::none();
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

    /// Create persistent state that survives across frames.
    ///
    /// Returns a `State<T>` handle. Access with `state.get(ui)` / `state.get_mut(ui)`.
    ///
    /// # Rules
    /// - Must be called in the same order every frame (like React hooks)
    /// - Do NOT call inside if/else that changes between frames
    ///
    /// # Example
    /// ```ignore
    /// let count = ui.use_state(|| 0i32);
    /// let val = count.get(ui);
    /// ui.text(format!("Count: {val}"));
    /// if ui.button("+1").clicked {
    ///     *count.get_mut(ui) += 1;
    /// }
    /// ```
    pub fn use_state<T: 'static>(&mut self, init: impl FnOnce() -> T) -> State<T> {
        let idx = self.hook_cursor;
        self.hook_cursor += 1;

        if idx >= self.hook_states.len() {
            self.hook_states.push(Box::new(init()));
        }

        State {
            idx,
            _marker: std::marker::PhantomData,
        }
    }

    /// Memoize a computed value. Recomputes only when `deps` changes.
    ///
    /// # Example
    /// ```ignore
    /// let doubled = ui.use_memo(&count, |c| c * 2);
    /// ui.text(format!("Doubled: {doubled}"));
    /// ```
    pub fn use_memo<T: 'static, D: PartialEq + Clone + 'static>(
        &mut self,
        deps: &D,
        compute: impl FnOnce(&D) -> T,
    ) -> &T {
        let idx = self.hook_cursor;
        self.hook_cursor += 1;

        let should_recompute = if idx >= self.hook_states.len() {
            true
        } else {
            let (stored_deps, _) = self.hook_states[idx]
                .downcast_ref::<(D, T)>()
                .unwrap_or_else(|| {
                    panic!(
                        "Hook type mismatch at index {}: expected {}. Hooks must be called in the same order every frame.",
                        idx,
                        std::any::type_name::<(D, T)>()
                    )
                });
            stored_deps != deps
        };

        if should_recompute {
            let value = compute(deps);
            let slot = Box::new((deps.clone(), value));
            if idx < self.hook_states.len() {
                self.hook_states[idx] = slot;
            } else {
                self.hook_states.push(slot);
            }
        }

        let (_, value) = self.hook_states[idx]
            .downcast_ref::<(D, T)>()
            .unwrap_or_else(|| {
                panic!(
                    "Hook type mismatch at index {}: expected {}. Hooks must be called in the same order every frame.",
                    idx,
                    std::any::type_name::<(D, T)>()
                )
            });
        value
    }

    /// Returns `light` color if current theme is light mode, `dark` color if dark mode.
    pub fn light_dark(&self, light: Color, dark: Color) -> Color {
        if self.theme.is_dark {
            dark
        } else {
            light
        }
    }

    /// Show a toast notification without managing ToastState.
    ///
    /// # Examples
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(80, 24).render(|ui| {
    /// ui.notify("File saved!", ToastLevel::Success);
    /// # });
    /// ```
    pub fn notify(&mut self, message: &str, level: ToastLevel) {
        let tick = self.tick;
        self.notification_queue
            .push((message.to_string(), level, tick));
    }

    pub(crate) fn render_notifications(&mut self) {
        self.notification_queue
            .retain(|(_, _, created)| self.tick.saturating_sub(*created) < 180);
        if self.notification_queue.is_empty() {
            return;
        }

        let items: Vec<(String, Color)> = self
            .notification_queue
            .iter()
            .rev()
            .map(|(message, level, _)| {
                let color = match level {
                    ToastLevel::Info => self.theme.primary,
                    ToastLevel::Success => self.theme.success,
                    ToastLevel::Warning => self.theme.warning,
                    ToastLevel::Error => self.theme.error,
                };
                (message.clone(), color)
            })
            .collect();

        let _ = self.overlay(|ui| {
            let _ = ui.row(|ui| {
                ui.spacer();
                let _ = ui.col(|ui| {
                    for (message, color) in &items {
                        let mut line = String::with_capacity(2 + message.len());
                        line.push_str("● ");
                        line.push_str(message);
                        ui.styled(line, Style::new().fg(*color));
                    }
                });
            });
        });
    }
}

mod widgets_display;
mod widgets_input;
mod widgets_interactive;
mod widgets_viz;

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

fn format_token_count(count: usize) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

fn format_table_row(cells: &[String], widths: &[u32], separator: &str) -> String {
    let sep_width = UnicodeWidthStr::width(separator);
    let total_cells_width: usize = widths.iter().map(|w| *w as usize).sum();
    let mut row = String::with_capacity(
        total_cells_width + sep_width.saturating_mul(widths.len().saturating_sub(1)),
    );
    for (i, width) in widths.iter().enumerate() {
        if i > 0 {
            row.push_str(separator);
        }
        let cell = cells.get(i).map(String::as_str).unwrap_or("");
        let cell_width = UnicodeWidthStr::width(cell) as u32;
        let padding = (*width).saturating_sub(cell_width) as usize;
        row.push_str(cell);
        row.extend(std::iter::repeat(' ').take(padding));
    }
    row
}

fn table_visible_len(state: &TableState) -> usize {
    if state.page_size == 0 {
        return state.visible_indices().len();
    }

    let start = state
        .page
        .saturating_mul(state.page_size)
        .min(state.visible_indices().len());
    let end = (start + state.page_size).min(state.visible_indices().len());
    end.saturating_sub(start)
}

pub(crate) fn handle_vertical_nav(
    selected: &mut usize,
    max_index: usize,
    key_code: KeyCode,
) -> bool {
    match key_code {
        KeyCode::Up | KeyCode::Char('k') => {
            if *selected > 0 {
                *selected -= 1;
                true
            } else {
                false
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if *selected < max_index {
                *selected += 1;
                true
            } else {
                false
            }
        }
        _ => false,
    }
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
    let mut centered = String::with_capacity(width);
    centered.extend(std::iter::repeat(' ').take(left));
    centered.push_str(text);
    centered.extend(std::iter::repeat(' ').take(right));
    centered
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestBackend;

    #[test]
    fn use_memo_type_mismatch_includes_index_and_expected_type() {
        let mut state = FrameState::default();
        let mut ctx = Context::new(Vec::new(), 20, 5, &mut state, Theme::dark());
        ctx.hook_states.push(Box::new(42u32));
        ctx.hook_cursor = 0;

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let deps = 1u8;
            let _ = ctx.use_memo(&deps, |_| 7u8);
        }))
        .expect_err("use_memo should panic on type mismatch");

        let message = panic_message(panic);
        assert!(
            message.contains("Hook type mismatch at index 0"),
            "panic message should include hook index, got: {message}"
        );
        assert!(
            message.contains(std::any::type_name::<(u8, u8)>()),
            "panic message should include expected type, got: {message}"
        );
        assert!(
            message.contains("Hooks must be called in the same order every frame."),
            "panic message should explain hook ordering requirement, got: {message}"
        );
    }

    #[test]
    fn light_dark_uses_current_theme_mode() {
        let mut dark_backend = TestBackend::new(10, 2);
        dark_backend.render(|ui| {
            let color = ui.light_dark(Color::Red, Color::Blue);
            ui.text("X").fg(color);
        });
        assert_eq!(dark_backend.buffer().get(0, 0).style.fg, Some(Color::Blue));

        let mut light_backend = TestBackend::new(10, 2);
        light_backend.render(|ui| {
            ui.set_theme(Theme::light());
            let color = ui.light_dark(Color::Red, Color::Blue);
            ui.text("X").fg(color);
        });
        assert_eq!(light_backend.buffer().get(0, 0).style.fg, Some(Color::Red));
    }

    fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
        if let Some(s) = panic.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = panic.downcast_ref::<&str>() {
            (*s).to_string()
        } else {
            "<non-string panic payload>".to_string()
        }
    }
}
