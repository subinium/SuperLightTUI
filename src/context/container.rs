use super::*;

#[inline]
fn saturating_gap(value: u32) -> i32 {
    value.min(i32::MAX as u32) as i32
}

#[inline]
fn saturating_overlap(overlap: u32) -> i32 {
    -saturating_gap(overlap)
}

/// Options for [`Context::modal_with`].
///
/// Controls focus behavior when a modal overlay is active.
///
/// # Example
///
/// ```no_run
/// # let mut show = true;
/// # slt::run(|ui: &mut slt::Context| {
/// if show {
///     ui.modal_with(slt::context::ModalOptions { tab_trap: true }, |ui| {
///         ui.text("Are you sure?");
///         if ui.button("OK").clicked { show = false; }
///     });
/// }
/// # });
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ModalOptions {
    /// When `true`, Tab/Shift+Tab navigation cannot leave the modal's focus
    /// range, even if [`Context::set_focus_index`] or a mouse click moved
    /// focus outside.
    ///
    /// Default: `true` — aligned with WCAG 2.1 SC 2.4.3 (Focus Order),
    /// which recommends trapping focus inside modal dialogs.
    ///
    /// Set to `false` to preserve the legacy behavior where focus could
    /// escape via programmatic means.
    pub tab_trap: bool,
}

impl Default for ModalOptions {
    fn default() -> Self {
        Self { tab_trap: true }
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
///     .p(1)
///     .grow(1)
///     .col(|ui| {
///         ui.text("inside a bordered, padded, growing column");
///     });
/// # });
/// ```
#[must_use = "ContainerBuilder does nothing until .col(), .row(), .line(), or .draw() is called"]
pub struct ContainerBuilder<'a> {
    pub(crate) ctx: &'a mut Context,
    /// Resolved main-axis gap, in cells. Signed (#222): negative means
    /// adjacent children overlap, set via [`ContainerBuilder::gap_overlap`].
    /// The public [`ContainerBuilder::gap`] setter takes `u32` and is
    /// source-compatible; only `gap_overlap` can store a negative value.
    pub(crate) gap: i32,
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
    pub(crate) group_name: Option<std::sync::Arc<str>>,
    pub(crate) padding: Padding,
    pub(crate) margin: Margin,
    pub(crate) constraints: Constraints,
    pub(crate) title: Option<(String, Style)>,
    pub(crate) grow: u16,
    /// Opt-in flex-shrink flag. Set via [`ContainerBuilder::shrink`].
    ///
    /// When `true`, this container participates in proportional shrinking
    /// if its parent row/column overflows. Default `false` keeps the
    /// historic overflow-by-design behavior. Closes #161.
    pub(crate) shrink_flag: bool,
    /// Opt-in container-level flex-wrap flag. Set via
    /// [`ContainerBuilder::wrap`].
    ///
    /// When `true` on a row, children that overflow the available width flow
    /// onto subsequent lines instead of overflowing past the right edge.
    /// Default `false` keeps the historic single-line behavior. No-op on a
    /// column. Closes #258.
    pub(crate) wrap_flag: bool,
    /// Optional flex-basis (initial main-axis size, in cells). Set via
    /// [`ContainerBuilder::basis`]. `None` (default) falls back to the
    /// child's min size, preserving current behavior. Closes #258.
    pub(crate) basis: Option<u32>,
    pub(crate) scroll_offset: Option<u32>,
    /// Horizontal scroll offset for a scrollable row (#247). Set internally by
    /// [`crate::Context::scrollable`] from `ScrollState::offset_x`; carried into
    /// `BeginScrollableArgs` and applied by the tree builder only when the
    /// finalizing direction is `Direction::Row`.
    pub(crate) scroll_offset_x: Option<u32>,
    pub(crate) theme_override: Option<Theme>,
}

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
    grid: Vec<CanvasPixel>,
    labels: Vec<CanvasLabel>,
}

/// A rejected Canvas backing-storage request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasError {
    /// The requested cell dimensions exceed the canvas geometry budget.
    GeometryBudgetExceeded,
    /// The requested layer exceeds the layer count or aggregate cell budget.
    LayerBudgetExceeded,
    /// The requested label exceeds the text byte or label count budget.
    LabelBudgetExceeded,
    /// The allocator rejected a bounded backing-storage request.
    AllocationFailed,
}

impl std::fmt::Display for CanvasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::GeometryBudgetExceeded => "canvas geometry exceeds the cell budget",
            Self::LayerBudgetExceeded => "canvas layers exceed the layer budget",
            Self::LabelBudgetExceeded => "canvas labels exceed the text budget",
            Self::AllocationFailed => "canvas backing-storage allocation failed",
        })
    }
}

impl std::error::Error for CanvasError {}

/// Drawing context for the canvas widget, with bounded backing storage.
///
/// Dimensions describe the requested coordinate space, not the visible parent
/// area. Reuse an instance with [`Context::canvas_with`] to retain its grids
/// and composition scratch between frames. Drop it to release that storage.
pub struct CanvasContext {
    layers: Vec<CanvasLayer>,
    active_layers: usize,
    label_bytes: usize,
    cols: usize,
    rows: usize,
    px_w: usize,
    px_h: usize,
    current_color: Color,
    /// Flat scratch buffer for `render()` pixel composition.
    /// Capacity = `cols * rows`; flat index = `row * cols + col`.
    scratch_pixels: Vec<CanvasPixel>,
    /// Flat scratch buffer for `render()` label overlay.
    /// Capacity = `cols * rows`; flat index = `row * cols + col`.
    scratch_labels: Vec<Option<(String, Color)>>,
}

/// Integer square root for non-negative `i64` values, returning `isize`.
///
/// Uses the standard integer square root available below the crate's MSRV.
#[inline]
fn isqrt_i64(n: i64) -> isize {
    u64::try_from(n).map_or(0, |value| value.isqrt() as isize)
}

impl CanvasContext {
    /// Maximum cells in a single layer, or in either dimension (262,144).
    /// Each cell represents eight pixels, for at most 2,097,152 pixels.
    pub const MAX_CELLS: usize = 262_144;
    /// Maximum active or retained layers in one canvas.
    pub const MAX_LAYERS: usize = 32;
    /// Maximum aggregate cells across all retained layer grids.
    pub const MAX_LAYER_CELLS: usize = crate::buffer::MAX_BUFFER_CELLS;
    /// Maximum UTF-8 bytes owned by labels in one frame.
    pub const MAX_LABEL_BYTES: usize = 1_048_576;
    /// Maximum total label slots retained across all layers.
    pub const MAX_LABELS: usize = 16_384;

    #[cfg(test)]
    fn new(cols: u32, rows: u32) -> Self {
        Self::try_new(cols, rows).expect("valid test canvas")
    }

    /// Allocate a canvas in terminal-cell dimensions.
    ///
    /// # Errors
    /// Returns [`CanvasError`] before allocating when dimensions exceed
    /// [`Self::MAX_CELLS`], or when a backing allocation fails. Zero dimensions
    /// are allowed and produce an empty canvas. Pixel dimensions are always
    /// representable on both native and 32-bit WASM targets.
    pub fn try_new(cols: u32, rows: u32) -> Result<Self, CanvasError> {
        let count = u64::from(cols) * u64::from(rows);
        if count > Self::MAX_CELLS as u64
            || u64::from(cols) > Self::MAX_CELLS as u64
            || u64::from(rows) > Self::MAX_CELLS as u64
        {
            return Err(CanvasError::GeometryBudgetExceeded);
        }
        let (cols, rows, cell_count) = (cols as usize, rows as usize, count as usize);
        let mut layers = Vec::new();
        layers
            .try_reserve_exact(1)
            .map_err(|_| CanvasError::AllocationFailed)?;
        layers.push(Self::new_layer(cell_count)?);
        Ok(Self {
            layers,
            active_layers: 1,
            label_bytes: 0,
            cols,
            rows,
            px_w: cols * 2,
            px_h: rows * 4,
            current_color: Color::Reset,
            scratch_pixels: Self::filled_storage(cell_count, Self::empty_pixel())?,
            scratch_labels: Self::filled_storage(cell_count, None)?,
        })
    }

    fn empty_pixel() -> CanvasPixel {
        CanvasPixel {
            bits: 0,
            color: Color::Reset,
        }
    }

    fn filled_storage<T: Clone>(count: usize, value: T) -> Result<Vec<T>, CanvasError> {
        let mut storage = Vec::new();
        storage
            .try_reserve_exact(count)
            .map_err(|_| CanvasError::AllocationFailed)?;
        storage.resize(count, value);
        Ok(storage)
    }

    fn new_layer(cell_count: usize) -> Result<CanvasLayer, CanvasError> {
        Ok(CanvasLayer {
            grid: Self::filled_storage(cell_count, Self::empty_pixel())?,
            labels: Vec::new(),
        })
    }

    fn current_layer_mut(&mut self) -> Option<&mut CanvasLayer> {
        self.layers.get_mut(self.active_layers - 1)
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

        let index = char_row * self.cols + char_col;
        if let Some(layer) = self.current_layer_mut() {
            let cell = &mut layer.grid[index];
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
            let dx = isqrt_i64(span_sq as i64);
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
            // A triangle has exactly 3 edges -> at most 3 intersections per
            // scanline. A 4-element stack array avoids per-scanline heap
            // allocations from the previous Vec<f64>.
            let mut intersections = [0.0f64; 4];
            let mut isect_count = 0usize;

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
                if isect_count < intersections.len() {
                    intersections[isect_count] = x_start as f64 + t * (x_end - x_start) as f64;
                    isect_count += 1;
                }
            }

            intersections[..isect_count].sort_by(|a, b| a.total_cmp(b));
            let mut i = 0usize;
            while i + 1 < isect_count {
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
    ///
    /// # Panics
    /// Panics on a rejected label allocation. Use [`Self::try_print`] to
    /// handle the error. Empty and off-canvas labels are ignored.
    pub fn print(&mut self, x: usize, y: usize, text: &str) {
        self.try_print(x, y, text)
            .expect("canvas label allocation failed");
    }

    /// Place a label, reporting bounded-storage failures without changing it.
    ///
    /// # Errors
    /// Returns [`CanvasError::LabelBudgetExceeded`] for excessive label bytes
    /// or retained slots, or [`CanvasError::AllocationFailed`].
    pub fn try_print(&mut self, x: usize, y: usize, text: &str) -> Result<(), CanvasError> {
        if text.is_empty() || x >= self.px_w || y >= self.px_h {
            return Ok(());
        }
        if text.len() > Self::MAX_LABEL_BYTES.saturating_sub(self.label_bytes) {
            return Err(CanvasError::LabelBudgetExceeded);
        }
        let reserved: usize = self
            .layers
            .iter()
            .map(|layer| layer.labels.capacity())
            .sum();
        let layer = &self.layers[self.active_layers - 1];
        let extra = if layer.labels.len() == layer.labels.capacity() {
            (Self::MAX_LABELS.saturating_sub(reserved)).min(layer.labels.capacity().max(4))
        } else {
            0
        };
        if layer.labels.len() == layer.labels.capacity() && extra == 0 {
            return Err(CanvasError::LabelBudgetExceeded);
        }
        let color = self.current_color;
        let mut owned = String::new();
        owned
            .try_reserve_exact(text.len())
            .map_err(|_| CanvasError::AllocationFailed)?;
        owned.push_str(text);
        if let Some(layer) = self.current_layer_mut() {
            if extra > 0 {
                layer
                    .labels
                    .try_reserve_exact(extra)
                    .map_err(|_| CanvasError::AllocationFailed)?;
            }
            layer.labels.push(CanvasLabel {
                x,
                y,
                text: owned,
                color,
            });
        }
        self.label_bytes += text.len();
        Ok(())
    }

    /// Start a new drawing layer. Shapes on later layers overlay earlier ones.
    ///
    /// # Panics
    /// Panics when the layer budget or allocation fails. Use [`Self::try_layer`]
    /// to handle the failure without changing the current layer.
    pub fn layer(&mut self) {
        self.try_layer().expect("canvas layer allocation failed");
    }

    /// Start a new layer, reusing a retained grid when possible.
    ///
    /// # Errors
    /// Returns [`CanvasError::LayerBudgetExceeded`] if the next layer exceeds
    /// [`Self::MAX_LAYERS`] or [`Self::MAX_LAYER_CELLS`], or
    /// [`CanvasError::AllocationFailed`] on allocation failure.
    pub fn try_layer(&mut self) -> Result<(), CanvasError> {
        let next = self.active_layers + 1;
        let cell_count = self.cols * self.rows;
        if next > Self::MAX_LAYERS || next * cell_count > Self::MAX_LAYER_CELLS {
            return Err(CanvasError::LayerBudgetExceeded);
        }
        if self.active_layers == self.layers.len() {
            let layer = Self::new_layer(cell_count)?;
            self.layers
                .try_reserve_exact(1)
                .map_err(|_| CanvasError::AllocationFailed)?;
            self.layers.push(layer);
        }
        self.active_layers = next;
        Ok(())
    }

    /// Clear drawing content and color while retaining grids and scratch.
    /// The next draw begins on the first layer; extra layers stay inactive.
    pub fn clear(&mut self) {
        for layer in &mut self.layers {
            layer.grid.fill(Self::empty_pixel());
            layer.labels.clear();
        }
        self.active_layers = 1;
        self.label_bytes = 0;
        self.current_color = Color::Reset;
        self.scratch_labels.fill(None);
    }

    pub(crate) fn render(&mut self) -> Vec<Vec<(String, Color)>> {
        self.scratch_pixels.fill(Self::empty_pixel());
        self.scratch_labels.fill(None);

        let cols = self.cols;
        let rows = self.rows;

        for layer in &self.layers[..self.active_layers] {
            for (dst, src) in self.scratch_pixels.iter_mut().zip(&layer.grid) {
                if src.bits == 0 {
                    continue;
                }
                let merged = dst.bits | src.bits;
                if merged != dst.bits {
                    dst.bits = merged;
                    dst.color = src.color;
                }
            }

            for label in &layer.labels {
                let row = label.y / 4;
                if row >= rows {
                    continue;
                }
                let mut col = label.x / 2;
                let row_offset = row * cols;
                for grapheme in label.text.graphemes(true) {
                    if col >= cols {
                        break;
                    }
                    let width = UnicodeWidthStr::width(grapheme).max(1);
                    if width > cols - col {
                        break;
                    }
                    self.scratch_labels[row_offset + col] =
                        Some((grapheme.to_string(), label.color));
                    for continuation in 1..width {
                        self.scratch_labels[row_offset + col + continuation] =
                            Some((String::new(), label.color));
                    }
                    col += width;
                }
            }
        }

        let mut lines: Vec<Vec<(String, Color)>> = Vec::with_capacity(rows);
        for row in 0..rows {
            let row_offset = row * cols;
            let mut segments: Vec<(String, Color)> = Vec::new();
            let mut current_color: Option<Color> = None;
            let mut current_text = String::new();

            for col in 0..cols {
                let idx = row_offset + col;
                let (label, pixel_ch, color) =
                    if let Some((label, label_color)) = &self.scratch_labels[idx] {
                        if label.is_empty() {
                            continue;
                        }
                        (Some(label.as_str()), None, *label_color)
                    } else {
                        let pixel = self.scratch_pixels[idx];
                        let ch = char::from_u32(0x2800 + pixel.bits).unwrap_or(' ');
                        (None, Some(ch), pixel.color)
                    };
                let append_symbol = |text: &mut String| {
                    if let Some(label) = label {
                        text.push_str(label);
                    } else if let Some(ch) = pixel_ch {
                        text.push(ch);
                    }
                };

                match current_color {
                    Some(c) if c == color => {
                        append_symbol(&mut current_text);
                    }
                    Some(c) => {
                        segments.push((std::mem::take(&mut current_text), c));
                        append_symbol(&mut current_text);
                        current_color = Some(color);
                    }
                    None => {
                        append_symbol(&mut current_text);
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

#[cfg(test)]
mod v024_canvas_tests {
    use super::*;

    #[test]
    fn aggregate_layer_budget_rejects_before_allocating_another_grid() {
        let mut canvas = CanvasContext::try_new(512, 512).unwrap();
        for _ in 1..4 {
            canvas.try_layer().unwrap();
        }
        let count = canvas.layers.len();
        assert_eq!(canvas.try_layer(), Err(CanvasError::LayerBudgetExceeded));
        assert_eq!(canvas.layers.len(), count);
        assert_eq!(canvas.active_layers, count);
    }

    #[test]
    fn clear_retains_grids_and_scratch_and_reuses_extra_layers() {
        let mut canvas = CanvasContext::try_new(8, 4).unwrap();
        canvas.try_layer().unwrap();
        canvas.print(0, 0, "LABEL");
        let grid = canvas.layers[1].grid.as_ptr();
        let scratch = canvas.scratch_pixels.as_ptr();
        canvas.clear();
        canvas.try_layer().unwrap();
        assert_eq!(canvas.layers[1].grid.as_ptr(), grid);
        assert_eq!(canvas.scratch_pixels.as_ptr(), scratch);
        assert!(canvas.layers[1].labels.is_empty());
        assert_eq!(canvas.label_bytes, 0);
    }

    #[test]
    fn label_slot_and_byte_budgets_are_bounded_across_retained_layers() {
        let mut canvas = CanvasContext::try_new(2, 1).unwrap();
        for _ in 0..CanvasContext::MAX_LABELS {
            canvas.try_print(0, 0, "a").unwrap();
        }
        assert_eq!(
            canvas.try_print(0, 0, "b"),
            Err(CanvasError::LabelBudgetExceeded)
        );
        canvas.clear();
        let text = "x".repeat(CanvasContext::MAX_LABEL_BYTES);
        canvas.try_print(0, 0, &text).unwrap();
        assert_eq!(
            canvas.try_print(0, 0, "x"),
            Err(CanvasError::LabelBudgetExceeded)
        );
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
            // `ContainerStyle::gap` stays `Option<u32>` (positive only); only
            // `gap_overlap` produces a negative builder gap (#222).
            self.gap = saturating_gap(v);
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
            self.constraints = self.constraints.w(w);
        }
        if let Some(h) = style.h {
            self.constraints = self.constraints.h(h);
        }
        if let Some(v) = style.min_w {
            self.constraints.set_min_width(Some(v));
        }
        if let Some(v) = style.max_w {
            self.constraints.set_max_width(Some(v));
        }
        if let Some(v) = style.min_h {
            self.constraints.set_min_height(Some(v));
        }
        if let Some(v) = style.max_h {
            self.constraints.set_max_height(Some(v));
        }
        if let Some(v) = style.w_pct {
            self.constraints.set_width_pct(Some(v));
        }
        if let Some(v) = style.h_pct {
            self.constraints.set_height_pct(Some(v));
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

    /// Set uniform padding on all sides.
    pub fn p(mut self, value: u32) -> Self {
        self.padding = Padding::all(value);
        self
    }

    /// Set uniform padding on all sides. Deprecated alias for [`p`](Self::p).
    #[deprecated(since = "0.20.0", note = "Use `p()` instead")]
    pub fn pad(self, value: u32) -> Self {
        self.p(value)
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
        self.constraints = self.constraints.w(value);
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
        self.constraints = self.constraints.h(value);
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
        self.constraints.set_min_width(Some(value));
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
        self.constraints.set_max_width(Some(value));
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
        self.constraints.set_min_height(Some(value));
        self
    }

    define_breakpoint_methods!(
        base = min_h,
        arg = value: u32,
        xs = xs_min_h => ["Minimum height applied only at Xs breakpoint (< 40 cols)."],
        sm = sm_min_h => ["Minimum height applied only at Sm breakpoint (40-79 cols)."],
        md = md_min_h => ["Minimum height applied only at Md breakpoint (80-119 cols)."],
        lg = lg_min_h => ["Minimum height applied only at Lg breakpoint (120-159 cols)."],
        xl = xl_min_h => ["Minimum height applied only at Xl breakpoint (>= 160 cols)."],
        at = min_h_at => ["Minimum height applied only at the given breakpoint."]
    );

    /// Set the maximum height constraint. Shorthand for [`max_height`](Self::max_height).
    pub fn max_h(mut self, value: u32) -> Self {
        self.constraints.set_max_height(Some(value));
        self
    }

    define_breakpoint_methods!(
        base = max_h,
        arg = value: u32,
        xs = xs_max_h => ["Maximum height applied only at Xs breakpoint (< 40 cols)."],
        sm = sm_max_h => ["Maximum height applied only at Sm breakpoint (40-79 cols)."],
        md = md_max_h => ["Maximum height applied only at Md breakpoint (80-119 cols)."],
        lg = lg_max_h => ["Maximum height applied only at Lg breakpoint (120-159 cols)."],
        xl = xl_max_h => ["Maximum height applied only at Xl breakpoint (>= 160 cols)."],
        at = max_h_at => ["Maximum height applied only at the given breakpoint."]
    );

    /// Set the minimum width constraint in cells. Deprecated alias for [`min_w`](Self::min_w).
    #[deprecated(since = "0.20.0", note = "Use `min_w()` instead")]
    pub fn min_width(self, value: u32) -> Self {
        self.min_w(value)
    }

    /// Set the maximum width constraint in cells. Deprecated alias for [`max_w`](Self::max_w).
    #[deprecated(since = "0.20.0", note = "Use `max_w()` instead")]
    pub fn max_width(self, value: u32) -> Self {
        self.max_w(value)
    }

    /// Set the minimum height constraint in rows. Deprecated alias for [`min_h`](Self::min_h).
    #[deprecated(since = "0.20.0", note = "Use `min_h()` instead")]
    pub fn min_height(self, value: u32) -> Self {
        self.min_h(value)
    }

    /// Set the maximum height constraint in rows. Deprecated alias for [`max_h`](Self::max_h).
    #[deprecated(since = "0.20.0", note = "Use `max_h()` instead")]
    pub fn max_height(self, value: u32) -> Self {
        self.max_h(value)
    }

    /// Set width as a percentage (1-100) of the parent container.
    pub fn w_pct(mut self, pct: u8) -> Self {
        self.constraints.set_width_pct(Some(pct.min(100)));
        self
    }

    /// Set height as a percentage (1-100) of the parent container.
    pub fn h_pct(mut self, pct: u8) -> Self {
        self.constraints.set_height_pct(Some(pct.min(100)));
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
        self.gap = saturating_gap(gap);
        self
    }

    /// Set a *negative* gap, causing adjacent children to overlap by `overlap`
    /// cells on the main axis.
    ///
    /// This is SLT's analogue of ratatui's `Layout::spacing(-1)`. The common
    /// use is collapsing the duplicate border between two adjacent bordered
    /// panels: with `gap_overlap(1)` each panel's shared edge lands in the
    /// same column (row layout) or row (column layout), so the doubled border
    ///
    /// ```text
    /// ┌────┐┌────┐
    /// │    ││    │
    /// └────┘└────┘
    /// ```
    ///
    /// collapses to a single shared edge.
    ///
    /// `gap_overlap(0)` is identical to `gap(0)` (no overlap). It composes with
    /// the existing `gap` family: the last call wins, so call exactly one of
    /// `gap` / `gap_overlap` per builder.
    ///
    /// # Rendering note
    ///
    /// SLT does not (yet) merge the shared cells into junction glyphs (`┬`,
    /// `┼`, `┴`). When two bordered panels overlap, both write the shared
    /// column/row and the later panel's border character wins by buffer-diff
    /// order. To get a clean seam, give the panels compatible border styles or
    /// drop one panel's shared side (e.g. `border_sides` without the left edge).
    ///
    /// Large overlaps saturate gracefully — `gap_overlap(N)` past a child's
    /// extent never panics or wraps; positions clamp at 0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Border;
    /// // Two bordered panels sharing one border column.
    /// ui.container().gap_overlap(1).row(|ui| {
    ///     ui.bordered(Border::Single).w(10).col(|ui| {
    ///         ui.text("left");
    ///     });
    ///     ui.bordered(Border::Single).w(10).col(|ui| {
    ///         ui.text("right");
    ///     });
    /// });
    /// # });
    /// ```
    pub fn gap_overlap(mut self, overlap: u32) -> Self {
        self.gap = saturating_overlap(overlap);
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

    /// Expand to fill remaining space on the main axis. Shorthand for
    /// [`grow(1)`](Self::grow).
    ///
    /// Equivalent to CSS `flex: 1` and ratatui's `Constraint::Fill(1)`.
    /// This is the most common case in flex layouts and reads more
    /// naturally than `grow(1)` for new readers — the abstract "grow
    /// factor" terminology is replaced by a self-documenting verb.
    ///
    /// ```ignore
    /// ui.container().fill().col(|ui| { ... });
    /// // identical to:
    /// ui.container().grow(1).col(|ui| { ... });
    /// ```
    ///
    /// For other weights (e.g. a 2:1 split between two siblings), use
    /// `grow(N)` directly.
    pub fn fill(self) -> Self {
        self.grow(1)
    }

    /// Opt this container into proportional flex-shrink.
    ///
    /// Marks this container as a shrink participant. When the parent
    /// row / column overflows (its children's combined width or height
    /// exceeds available space), shrink-flagged children scale their
    /// fixed sizes by `available / fixed_total` (CSS `flex-shrink`-style).
    /// Children without `.shrink()` keep their historic
    /// overflow-by-design size and clip naturally.
    ///
    /// Default for every container is `false` — opt in per child.
    /// Equivalent to CSS `flex-shrink: 1` (vs the SLT default of `0`).
    /// Closes #161.
    ///
    /// # Example
    ///
    /// Two siblings with combined fixed width `60` placed inside a
    /// `40`-cell row. Without `.shrink()`, the row overflows; with
    /// `.shrink()` on both, each scales to `40 * 30/60 = 20`:
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// // Without shrink — overflows the parent.
    /// ui.row(|ui| {
    ///     ui.container().w(30).col(|ui| { ui.text("left"); });
    ///     ui.container().w(30).col(|ui| { ui.text("right"); });
    /// });
    ///
    /// // With shrink on both — proportional fit, no clipping.
    /// ui.row(|ui| {
    ///     ui.container().w(30).shrink().col(|ui| { ui.text("left"); });
    ///     ui.container().w(30).shrink().col(|ui| { ui.text("right"); });
    /// });
    /// # });
    /// ```
    ///
    /// # Layout
    ///
    /// Only fixed-width children with `grow == 0` participate. Grow
    /// children already absorb leftover space and ignore the shrink
    /// flag. Mixing shrink and non-shrink siblings is supported — only
    /// the flagged ones contribute to the shrink budget.
    pub fn shrink(mut self) -> Self {
        self.shrink_flag = true;
        self
    }

    /// Allow row children to wrap onto subsequent lines on main-axis overflow.
    ///
    /// When a `.row()` finalized with `wrap()` has children whose combined
    /// width exceeds the available width, the overflowing children flow onto
    /// the next line, and lines stack on the cross axis. This is the
    /// immediate-mode primitive for tag clouds, chip lists, wrapping toolbars,
    /// and responsive card grids that reflow as the terminal resizes — without
    /// per-frame breakpoint math. Equivalent to CSS `flex-wrap: wrap`.
    ///
    /// Spacing: within-line (main-axis) spacing uses `gap` / `col_gap` as
    /// usual; between-line (cross-axis) spacing uses `row_gap` when set, else
    /// `gap`. A child wider than the full available width occupies its own
    /// line (clipped, as a single-line row would clip) rather than producing
    /// an empty line.
    ///
    /// Row only. On `col()` this is a documented no-op (vertical-axis wrap is
    /// out of scope). Default: no wrap (single-line, current
    /// overflow-by-design behavior). Closes #258.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// // A chip list that reflows onto as many lines as the width needs.
    /// ui.container().wrap().gap(1).row(|ui| {
    ///     for tag in ["rust", "tui", "flexbox", "wrap", "immediate-mode"] {
    ///         ui.container().p(1).col(|ui| { ui.text(tag); });
    ///     }
    /// });
    /// # });
    /// ```
    #[doc(alias = "flex-wrap")]
    pub fn wrap(mut self) -> Self {
        self.wrap_flag = true;
        self
    }

    /// Set the flex-basis: the initial main-axis size (in cells) that `grow`
    /// grows from and `shrink` (#161) shrinks from.
    ///
    /// CSS resolves flex sizing as `basis` (initial) → distribute free space
    /// by `grow` → distribute the deficit by `shrink`. By default SLT uses a
    /// child's min size as that base; `basis(n)` overrides it so a child can
    /// say "start at `n` cells, then grow / shrink from there". `None`
    /// (default, i.e. not calling this) falls back to the min size, preserving
    /// current behavior. Equivalent to CSS `flex-basis: <n>`. Closes #258.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// // Two cards that each start at 10 cells, then split the leftover.
    /// ui.row(|ui| {
    ///     ui.container().basis(10).grow(1).col(|ui| { ui.text("a"); });
    ///     ui.container().basis(10).grow(1).col(|ui| { ui.text("b"); });
    /// });
    /// # });
    /// ```
    #[doc(alias = "flex-basis")]
    pub fn basis(mut self, cells: u32) -> Self {
        self.basis = Some(cells);
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

    // ── conditional / grouped builder helpers ───────────────────────

    /// Apply `f` only if `cond` is true. Returns the builder for chaining.
    ///
    /// Use this to attach a block of builder modifiers without breaking the
    /// fluent chain. The closure takes the builder by value and must return
    /// it (matching the rest of `ContainerBuilder`'s by-value API), so any
    /// builder method (`.border()`, `.title()`, `.bg()`, etc.) can be chained
    /// inside.
    ///
    /// Zero allocation: the closure is inlined and skipped entirely when
    /// `cond` is `false`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Border;
    /// let highlighted = true;
    /// ui.container()
    ///     .p(1)
    ///     .with_if(highlighted, |c| c.border(Border::Single).title("Active"))
    ///     .col(|ui| {
    ///         ui.text("body");
    ///     });
    /// # });
    /// ```
    pub fn with_if(self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond { f(self) } else { self }
    }

    /// Override the active theme for all widgets rendered inside this container.
    ///
    /// The override is scoped to the container body (the closure passed to
    /// `.col()`, `.row()`, or `.line()`). The parent theme is restored when
    /// the container closes — including on panic.
    ///
    /// All built-in widgets read `ctx.theme` directly for color decisions,
    /// so this swap propagates through every nested widget without requiring
    /// them to opt in. Nested `.theme(...)` calls correctly nest: the
    /// innermost theme wins inside its own subtree, and the outer theme
    /// resumes once it closes.
    ///
    /// Independent of [`Context::provide`] / [`Context::use_context`] —
    /// this directly mutates the active theme used by SLT-owned widgets,
    /// while `provide`/`use_context` is the general-purpose context
    /// injection mechanism for user code.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::{Border, Theme};
    /// ui.container()
    ///     .theme(Theme::light())
    ///     .border(Border::Rounded)
    ///     .col(|ui| {
    ///         ui.text("This subtree renders with the light theme");
    ///         ui.button("Click me"); // also uses light theme colors
    ///     });
    /// # });
    /// ```
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme_override = Some(theme);
        self
    }

    /// Apply `f` unconditionally. Useful for factoring out a block of builder
    /// modifier calls while keeping the fluent chain intact.
    ///
    /// The closure takes the builder by value and must return it.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Border;
    /// ui.container()
    ///     .with(|c| c.border(Border::Rounded).p(1))
    ///     .col(|ui| {
    ///         ui.text("body");
    ///     });
    /// # });
    /// ```
    pub fn with(self, f: impl FnOnce(Self) -> Self) -> Self {
        f(self)
    }

    // ── opt-in scoped cache (issue #273) ───────────────────────────────

    /// Opt-in: declare a subtree **stable** when `version_key` is unchanged
    /// from the previous frame at this call site.
    ///
    /// This is an **author-controlled cache, not reactive binding**. Your
    /// closure is still the app ([Principle 2 — "Your Closure IS the App"]):
    /// `f` runs **every frame** exactly like `.col(f)`, so the rendered output
    /// is **byte-for-byte identical** to an uncached container — there is no
    /// retained widget identity, no message passing, no reactive subscription,
    /// and no behavior change whatsoever when you do not call `cached`.
    ///
    /// What `cached` adds is a single, principle-preserving signal: it records
    /// the `version_key` you supply (a value you already own — e.g. a hash of
    /// the non-streaming inputs, or `StreamingTextState::version` of the
    /// *other* panes) and compares it to the key this call site recorded last
    /// frame. A match is a *cache hit* (the subtree is declared unchanged); a
    /// change, a new call site, the first frame, or a terminal resize is a
    /// *miss*. The hit/miss tally is exposed via
    /// [`Context::region_cache_hits`](crate::Context::region_cache_hits) /
    /// [`Context::region_cache_misses`](crate::Context::region_cache_misses).
    ///
    /// # Why output is identical even on a hit (current implementation)
    ///
    /// Skipping `f` on a hit would require splicing the prior frame's recorded
    /// `Command`s, replaying its focus / hit-map / scroll / raw-draw feedback,
    /// and reusing its rendered cells — without that full replay the immediate-
    /// mode invariant breaks (focus and interaction would silently drop). That
    /// replay is deliberately **out of scope** here (it risks reintroducing a
    /// retained tree, the thing Principle 2 forbids). So `cached` keeps the
    /// invariant absolute — `f` always runs — and instead lands the *safe,
    /// reversible* half: a measured, author-keyed stability gate plus
    /// diagnostics. The streaming benchmark `bench_streaming_append_chat`
    /// (`benches/benchmarks.rs`) quantifies the upstream cost this gate is
    /// designed to eventually elide; see `docs/PERFORMANCE.md`.
    ///
    /// # Pattern: cache the chrome, not the stream
    ///
    /// During token streaming, wrap the *static* surroundings (chat history,
    /// sidebar, status bar) keyed off everything *except* the stream, and
    /// leave the stream itself uncached — it changes every token:
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// # let history_version = 3u64;
    /// # let mut stream = slt::StreamingTextState::new();
    /// ui.container().cached(history_version, |ui| {
    ///     ui.text("…long chat transcript…"); // unchanged this token
    /// });
    /// ui.streaming_text(&mut stream);         // changes every token
    /// # });
    /// ```
    ///
    /// [Principle 2 — "Your Closure IS the App"]: https://docs.rs/slt
    pub fn cached(self, version_key: u64, f: impl FnOnce(&mut Context)) -> Response {
        // Record the key / classify hit-vs-miss BEFORE running the body so the
        // declaration order (and thus the per-call-site slot index) matches
        // the order regions are authored, exactly like the hook cursor.
        let _hit = self.ctx.record_cached_region(version_key);
        // Always run the body: byte-identical output, immediate-mode invariant
        // preserved. `_hit` is the gate a future cell-level cache would use.
        self.col(f)
    }

    // ── internal ─────────────────────────────────────────────────────

    /// Set the vertical scroll offset in rows. Used internally by [`Context::scrollable`].
    ///
    /// This is a crate-internal helper; external callers should use
    /// [`Context::scrollable`] together with a [`ScrollState`].
    ///
    /// Hidden from rustdoc with `#[doc(hidden)]` so it does not appear in the
    /// public API surface, while remaining callable for backwards compatibility
    /// (cargo-semver-checks still tracks the symbol). Promote to `pub(crate)`
    /// at v1.0.
    ///
    /// [`ScrollState`]: crate::widgets::ScrollState
    #[doc(hidden)]
    pub fn scroll_offset(mut self, offset: u32) -> Self {
        self.scroll_offset = Some(offset);
        self
    }

    /// Internal entry point that takes an already-shared `Arc<str>`.
    ///
    /// Used by `Context::group()` so the name allocated in the public path
    /// is pushed onto `group_stack` and threaded into `BeginContainerArgs`
    /// through a single `Arc::clone` instead of two `String` allocations.
    /// Closes #145 (double `to_string`) and completes the `Arc<str>`
    /// migration in #139.
    pub(crate) fn group_name_arc(mut self, name: std::sync::Arc<str>) -> Self {
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

    /// Like [`draw`](Self::draw), but carries owned per-frame `data` through
    /// to the deferred closure as a borrow.
    ///
    /// Raw-draw closures must be `'static` because they run after layout is
    /// computed — which normally forces callers to snapshot any borrowed
    /// state into an owned value before passing it in. `draw_with` makes
    /// that explicit: hand the snapshot over, borrow it inside the closure.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::{Buffer, Rect, Style};
    /// # slt::run(|ui: &mut slt::Context| {
    /// let points: Vec<(u32, u32)> = (0..20).map(|i| (i, i * 2)).collect();
    /// ui.container().w(40).h(20).draw_with(points, |buf, rect, points| {
    ///     for (x, y) in points {
    ///         if rect.contains(*x, *y) {
    ///             buf.set_char(*x, *y, '●', Style::new());
    ///         }
    ///     }
    /// });
    /// # });
    /// ```
    pub fn draw_with<D: 'static>(
        self,
        data: D,
        f: impl FnOnce(&mut crate::buffer::Buffer, Rect, &D) + 'static,
    ) {
        let draw_id = self.ctx.deferred_draws.len();
        self.ctx
            .deferred_draws
            .push(Some(Box::new(move |buf, rect| f(buf, rect, &data))));
        self.ctx.skip_interaction_slot();
        self.ctx.commands.push(Command::RawDraw {
            draw_id,
            constraints: self.constraints,
            grow: self.grow,
            margin: self.margin,
        });
    }

    /// Execute a borrowed-data draw closure immediately, then composite its
    /// owned cell snapshot after layout.
    ///
    /// Unlike [`draw`](Self::draw), `f` does not need to be `'static`: it runs
    /// before this method returns and may borrow local application state. The
    /// resulting source buffer is moved into the deferred layout callback.
    /// This is appropriate for cell-based custom drawing with known source
    /// dimensions; terminal protocol placements and raw escape sequences are
    /// intentionally not copied from the snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`BufferError`](crate::buffer::BufferError) when the requested
    /// source geometry exceeds the configured cell/row budget.
    pub fn draw_precomputed(
        self,
        width: u32,
        height: u32,
        f: impl FnOnce(&mut crate::buffer::Buffer, Rect),
    ) -> Result<(), crate::buffer::BufferError> {
        let source_rect = Rect::new(0, 0, width, height);
        let mut source = crate::buffer::Buffer::try_empty(source_rect)?;
        f(&mut source, source_rect);
        self.draw(move |destination, rect| {
            let (left, top) = destination.draw_source_offset();
            let copy_width = source.area.width.saturating_sub(left).min(rect.width);
            let copy_height = source.area.height.saturating_sub(top).min(rect.height);
            for dy in 0..copy_height {
                for dx in 0..copy_width {
                    let source_x = left + dx;
                    let source_y = top + dy;
                    let Some(cell) = source.try_get(source_x, source_y) else {
                        continue;
                    };
                    if cell.is_continuation() {
                        continue;
                    }
                    let symbol = cell.normalized_symbol();
                    if UnicodeWidthStr::width(symbol.as_str()) as u32 > copy_width - dx {
                        continue;
                    }
                    let x = rect.x.saturating_add(dx);
                    let y = rect.y.saturating_add(dy);
                    destination.set_grapheme_visual(
                        x,
                        y,
                        &symbol,
                        cell.style,
                        cell.hyperlink
                            .as_ref()
                            .filter(|url| crate::buffer::is_valid_osc8_url(url)),
                    );
                }
            }
        });
        Ok(())
    }

    /// Finalize a raw-draw region with a render-stage panic fallback.
    ///
    /// Deferred draw callbacks execute after the normal Context tree has
    /// finished layout, so [`Context::error_boundary`] cannot safely rebuild
    /// its fallback at that stage. This method catches the draw panic inside
    /// the laid-out region and invokes `fallback` with the panic message while
    /// the same clip is active.
    /// Like all Rust unwind boundaries, this cannot recover in `panic = "abort"`
    /// builds, including the browser target's default panic strategy.
    pub fn draw_with_fallback(
        self,
        draw: impl FnOnce(&mut crate::buffer::Buffer, Rect) + 'static,
        fallback: impl FnOnce(&mut crate::buffer::Buffer, Rect, &str) + 'static,
    ) {
        self.draw(move |buffer, rect| {
            let result = crate::catch_recoverable_unwind(|| {
                draw(buffer, rect);
            });
            if let Err(payload) = result {
                let message = if let Some(message) = payload.downcast_ref::<&str>() {
                    (*message).to_owned()
                } else if let Some(message) = payload.downcast_ref::<String>() {
                    message.clone()
                } else {
                    "draw callback panicked with a non-string payload".to_owned()
                };
                fallback(buffer, rect, &message);
            }
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
        // `row_gap` / `col_gap` are `Option<u32>` (positive override); fall back
        // to the signed builder `gap`, which alone can carry an overlap (#222).
        let resolved_gap: i32 = match direction {
            Direction::Column => self.row_gap.map(saturating_gap).unwrap_or(self.gap),
            Direction::Row => self.col_gap.map(saturating_gap).unwrap_or(self.gap),
        };
        // Cross-axis (between-line) gap for a wrapping row (#258): `row_gap`
        // when set, else the builder `gap`. Only consulted by the layout pass
        // when this container is a wrapping `Direction::Row`.
        let resolved_cross_gap: i32 = self.row_gap.map(saturating_gap).unwrap_or(self.gap);

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

        // Opt-in flex-shrink (#161). Push a marker the layout pass picks up
        // and applies to the next `BeginContainer` / `BeginScrollable`,
        // mirroring the existing `FocusMarker` / `InteractionMarker` pattern.
        // This avoids touching every `BeginContainerArgs` construction site
        // across the widget modules — only `ContainerBuilder.shrink()`
        // emits the marker, and `LayoutNode::shrink` defaults to `false`.
        if self.shrink_flag {
            self.ctx.commands.push(Command::ShrinkMarker);
        }

        // Opt-in flex-wrap / flex-basis (#258). Same marker pattern as shrink:
        // pushed just before the matching `Begin*`, picked up by the layout
        // pass and applied to the next node. Both default off / `None`, so
        // unflagged containers are byte-identical to pre-#258.
        if self.wrap_flag {
            self.ctx
                .commands
                .push(Command::WrapMarker(resolved_cross_gap));
        }
        if let Some(basis) = self.basis {
            self.ctx.commands.push(Command::BasisMarker(basis));
        }

        if let Some(scroll_offset) = self.scroll_offset {
            // #247: carry the finalizing `.row()` / `.col()` direction and both
            // axis offsets. The tree builder applies the offset matching
            // `direction`; the cross-axis offset is `0` for a single-axis
            // scroller (the common case).
            self.ctx
                .commands
                .push(Command::BeginScrollable(Box::new(BeginScrollableArgs {
                    grow: self.grow,
                    direction,
                    border: self.border,
                    border_sides: self.border_sides,
                    border_style,
                    bg_color,
                    align: self.align,
                    align_self: self.align_self_value,
                    justify: self.justify,
                    gap: resolved_gap,
                    padding: self.padding,
                    margin: self.margin,
                    constraints: self.constraints,
                    title: self.title,
                    scroll_offset,
                    scroll_offset_x: self.scroll_offset_x.unwrap_or(0),
                    group_name,
                })));
        } else {
            self.ctx
                .commands
                .push(Command::BeginContainer(Box::new(BeginContainerArgs {
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
                })));
        }
        self.ctx.rollback.text_color_stack.push(self.text_color);
        // Swap active theme if a per-subtree override was requested.
        // The previous theme is restored after `f` returns — including on
        // panic, so no widget ever sees a leaked override theme.
        let theme_save = self.theme_override.map(|t| {
            let prev = self.ctx.theme;
            self.ctx.theme = t;
            // Also keep dark_mode flag in sync so `dark_*` style variants
            // resolve to the new theme's brightness, not the stale flag.
            self.ctx.rollback.dark_mode = t.is_dark;
            (prev, prev.is_dark)
        });
        // catch_unwind guards the restore path against panics inside `f`.
        // The overlay/group bookkeeping that follows assumes `theme` reflects
        // the parent scope, so we must restore before propagating the panic.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(self.ctx)));
        if let Some((prev, prev_dark)) = theme_save {
            self.ctx.theme = prev;
            self.ctx.rollback.dark_mode = prev_dark;
        }
        self.ctx.rollback.text_color_stack.pop();
        self.ctx.commands.push(Command::EndContainer);
        self.ctx.rollback.last_text_idx = None;
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }

        if is_group_container {
            self.ctx.rollback.group_stack.pop();
            self.ctx.rollback.group_count = self.ctx.rollback.group_count.saturating_sub(1);
        }

        self.ctx.response_for(interaction_id)
    }
}

#[cfg(test)]
mod hotfix_tests {
    //! Regression tests for v0.19.1 A3 hotfixes (issues #143, #144, #146, #149).

    use super::*;

    // -- #143: filled_triangle stack-array intersections ----------------

    /// Filling a triangle must paint the same pixel set whether the
    /// previous Vec<f64> path or the new inline-array path is used.
    #[test]
    fn filled_triangle_paints_expected_interior() {
        let mut canvas = CanvasContext::new(20, 20);
        canvas.filled_triangle(2, 2, 18, 4, 6, 18);

        // Sample a point that must be filled (lies clearly inside the
        // triangle) and a point that must remain empty.
        let lines = canvas.render();
        // Pixel (8, 8) -> char cell (4, 2). Pull bits via re-render fallback.
        let inside_row = 8 / 4;
        let outside_row = 0;
        // Each row must be present in the rendered output.
        assert!(lines.len() > inside_row);
        assert!(lines.len() > outside_row);

        // Inside row must contain at least one non-blank braille glyph.
        let inside: String = lines[inside_row].iter().map(|(s, _)| s.as_str()).collect();
        assert!(
            inside.chars().any(|c| c != '\u{2800}' && c != ' '),
            "expected filled glyphs inside triangle, got: {inside:?}"
        );
    }

    /// Tall triangles previously allocated O(H) Vecs; the new path must
    /// still produce filled output for many scanlines without panicking.
    #[test]
    fn filled_triangle_handles_tall_triangle_without_panic() {
        let mut canvas = CanvasContext::new(8, 50);
        canvas.filled_triangle(0, 0, 15, 0, 8, 199);
        let lines = canvas.render();
        assert_eq!(lines.len(), 50);
    }

    /// Degenerate horizontal triangle (all three vertices on the same row)
    /// must not panic and must produce no fill (only the outline edges).
    #[test]
    fn filled_triangle_degenerate_horizontal_is_safe() {
        let mut canvas = CanvasContext::new(20, 20);
        canvas.filled_triangle(0, 0, 10, 0, 19, 0);
        let _ = canvas.render();
    }

    // -- #146: integer isqrt for filled_circle -------------------------

    #[test]
    fn isqrt_i64_matches_floor_sqrt_for_small_values() {
        for n in 0i64..=10_000 {
            let expected = (n as f64).sqrt().floor() as isize;
            assert_eq!(isqrt_i64(n), expected, "mismatch at n={n}");
        }
    }

    #[test]
    fn isqrt_i64_handles_perfect_squares_and_boundaries() {
        for k in 0i64..=4096 {
            assert_eq!(isqrt_i64(k * k), k as isize);
            if k > 0 {
                assert_eq!(isqrt_i64(k * k - 1), (k - 1) as isize);
            }
        }
    }

    #[test]
    fn isqrt_i64_clamps_non_positive_to_zero() {
        assert_eq!(isqrt_i64(0), 0);
        assert_eq!(isqrt_i64(-1), 0);
        assert_eq!(isqrt_i64(i64::MIN), 0);
    }

    /// `filled_circle` should produce a symmetric span around its center
    /// after switching from f64 sqrt to integer isqrt.
    #[test]
    fn filled_circle_renders_without_panic_and_is_non_empty() {
        let mut canvas = CanvasContext::new(20, 20);
        canvas.filled_circle(10, 10, 6);
        let lines = canvas.render();
        let any_filled = lines
            .iter()
            .flatten()
            .any(|(s, _)| s.chars().any(|c| c != '\u{2800}' && c != ' '));
        assert!(any_filled, "filled_circle produced empty output");
    }

    #[test]
    fn canvas_labels_respect_grapheme_cell_width() {
        let mut canvas = CanvasContext::new(4, 1);
        canvas.print(0, 0, "界A");

        let lines = canvas.render();
        let rendered: String = lines[0].iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(rendered, "界A\u{2800}");
        assert_eq!(UnicodeWidthStr::width(rendered.as_str()), 4);
    }

    #[test]
    fn canvas_labels_do_not_render_partial_wide_graphemes() {
        let mut canvas = CanvasContext::new(2, 1);
        canvas.print(2, 0, "A界");

        let lines = canvas.render();
        let rendered: String = lines[0].iter().map(|(text, _)| text.as_str()).collect();
        assert_eq!(rendered, "\u{2800}A");
        assert_eq!(UnicodeWidthStr::width(rendered.as_str()), 2);
    }

    // -- #149: scroll_offset visibility (compile-time check) -----------

    /// The `scroll_offset` helper must remain callable from inside the crate.
    /// It is `#[doc(hidden)] pub` (Option B from the issue) so it is removed
    /// from rustdoc but still semver-tracked; this test compiles only when
    /// the path is reachable.
    #[test]
    fn scroll_offset_is_crate_internal_api() {
        let _ = ContainerBuilder::scroll_offset;
    }

    #[test]
    fn draw_precomputed_accepts_borrowed_local_data() {
        let label = String::from("borrowed");
        let mut backend = crate::TestBackend::new(20, 3);
        backend.render(|ui| {
            ui.container()
                .w(8)
                .h(1)
                .draw_precomputed(8, 1, |buffer, rect| {
                    buffer.set_string(rect.x, rect.y, &label, crate::Style::new());
                })
                .expect("small snapshot geometry is valid");
        });
        backend.assert_contains("borrowed");
    }

    #[test]
    fn draw_precomputed_rejects_pathological_source_geometry() {
        let mut state = crate::FrameState::default();
        let mut ui = crate::Context::new(Vec::new(), 20, 3, &mut state, crate::Theme::dark());
        let result = ui
            .container()
            .draw_precomputed(u32::MAX, u32::MAX, |_, _| {});
        assert!(result.is_err());
    }

    #[test]
    fn draw_with_fallback_recovers_inside_the_raw_region() {
        let mut backend = crate::TestBackend::new(24, 3);
        backend.render(|ui| {
            ui.container().w(20).h(1).draw_with_fallback(
                |_, _| panic!("raw draw failed"),
                |buffer, rect, message| {
                    buffer.set_string(rect.x, rect.y, message, crate::Style::new());
                },
            );
        });
        backend.assert_contains("raw draw failed");
    }
}

#[cfg(test)]
mod flex_wrap_tests {
    //! Render-level regression tests for flex-wrap / flex-basis (#258).

    use crate::test_utils::TestBackend;

    /// A wrapping row of labels wider than the backend must flow the
    /// overflowing label onto the second terminal row, not clip it off the
    /// right edge. Each label is a 1-cell-tall text node, so a line is one
    /// cell tall and a wrap is visible as text on row 1.
    #[test]
    fn wrap_row_flows_overflow_to_second_line() {
        // Backend is 12 wide. `col_gap(1)` sets within-line spacing only, so
        // the cross-axis (between-line) gap falls back to 0. "alpha"(5) + 1 +
        // "bravo"(5) = 11 fits line 0; "gamma" overflows (11 + 1 + 5 = 17 >
        // 12) to line 1, immediately below with no blank gap row.
        let mut tb = TestBackend::new(12, 4);
        tb.render(|ui| {
            let _ = ui.container().wrap().col_gap(1).row(|ui| {
                ui.text("alpha");
                ui.text("bravo");
                ui.text("gamma");
            });
        });

        // Line 0 holds the first two labels; the third wrapped to line 1.
        tb.assert_line_contains(0, "alpha");
        tb.assert_line_contains(0, "bravo");
        tb.assert_line_contains(1, "gamma");
    }

    /// `wrap()` is opt-in: without it the overflowing label clips off the
    /// right edge rather than wrapping, so nothing appears on row 1.
    #[test]
    fn no_wrap_row_keeps_single_line() {
        let mut tb = TestBackend::new(12, 4);
        tb.render(|ui| {
            let _ = ui.container().col_gap(1).row(|ui| {
                ui.text("alpha");
                ui.text("bravo");
                ui.text("gamma");
            });
        });

        // Single line: first label on row 0, nothing wrapped to row 1.
        tb.assert_line_contains(0, "alpha");
        assert_eq!(tb.line(1), "");
    }
}

#[cfg(test)]
mod cached_region_tests {
    //! Issue #273 — opt-in scoped cached region.
    //!
    //! The invariant under test: `cached(key, f)` is byte-identical to an
    //! uncached container in EVERY case (the body always runs), and it
    //! correctly classifies each call site as a hit (key unchanged) or miss
    //! (key changed / new / first frame / post-resize) so the hit/miss
    //! diagnostics — and a future cell-level cache — have a sound gate.

    use crate::event::Event;
    use crate::test_utils::{EventBuilder, TestBackend};
    use std::cell::Cell;

    /// First frame is always a miss, output identical to a plain container.
    #[test]
    fn cached_region_byte_identical_on_first_frame() {
        let mut cached = TestBackend::new(40, 6);
        cached.render(|ui| {
            let _ = ui.container().cached(7, |ui| {
                ui.text("static chrome line one");
                ui.text("static chrome line two");
            });
        });

        let mut plain = TestBackend::new(40, 6);
        plain.render(|ui| {
            let _ = ui.container().col(|ui| {
                ui.text("static chrome line one");
                ui.text("static chrome line two");
            });
        });

        assert_eq!(
            cached.buffer().snapshot_format(),
            plain.buffer().snapshot_format(),
            "cached region must render byte-identically to an uncached container"
        );
    }

    /// An unchanged key is a hit on the second frame. The body still runs
    /// every frame (immediate-mode invariant), so the content stays visible
    /// and identical — `cached` only flips the hit classification.
    #[test]
    fn cached_region_hit_on_unchanged_key_body_still_runs() {
        let mut tb = TestBackend::new(40, 4);
        let runs = Cell::new(0u32);
        let hits = Cell::new(0u32);
        let misses = Cell::new(0u32);

        let frame = |tb: &mut TestBackend| {
            tb.render(|ui| {
                let _ = ui.container().cached(99, |ui| {
                    runs.set(runs.get() + 1);
                    ui.text("stable");
                });
                hits.set(ui.region_cache_hits());
                misses.set(ui.region_cache_misses());
            });
        };

        frame(&mut tb);
        assert_eq!(runs.get(), 1, "first frame runs the body");
        assert_eq!(misses.get(), 1, "first frame is a miss");
        assert_eq!(hits.get(), 0);
        tb.assert_contains("stable");

        frame(&mut tb);
        // Body STILL runs (byte-identical guarantee) even though the key
        // matched — the only observable change is the hit classification.
        assert_eq!(runs.get(), 2, "body re-runs every frame regardless of hit");
        assert_eq!(hits.get(), 1, "unchanged key on the second frame is a hit");
        assert_eq!(misses.get(), 0);
        tb.assert_contains("stable");
    }

    /// A changed key is a miss and the new content renders.
    #[test]
    fn cached_region_miss_on_key_change() {
        let mut tb = TestBackend::new(40, 4);
        let hits = Cell::new(0u32);
        let misses = Cell::new(0u32);

        tb.render(|ui| {
            let _ = ui.container().cached(1, |ui| {
                ui.text("first");
            });
            hits.set(ui.region_cache_hits());
            misses.set(ui.region_cache_misses());
        });
        assert_eq!(misses.get(), 1);
        tb.assert_contains("first");

        tb.render(|ui| {
            let _ = ui.container().cached(2, |ui| {
                ui.text("second");
            });
            hits.set(ui.region_cache_hits());
            misses.set(ui.region_cache_misses());
        });
        assert_eq!(hits.get(), 0, "changed key is not a hit");
        assert_eq!(misses.get(), 1, "changed key is a miss");
        tb.assert_contains("second");
    }

    /// A resize clears the persisted keys, forcing the next frame to miss even
    /// when the author passes the same key.
    #[test]
    fn cached_region_invalidates_on_resize() {
        let mut tb = TestBackend::new(40, 4);
        let hits = Cell::new(0u32);

        tb.render(|ui| {
            let _ = ui.container().cached(5, |ui| {
                ui.text("body");
            });
        });
        // Second frame, same key, no resize → hit.
        tb.render(|ui| {
            let _ = ui.container().cached(5, |ui| {
                ui.text("body");
            });
            hits.set(ui.region_cache_hits());
        });
        assert_eq!(hits.get(), 1, "same key without resize is a hit");

        // Now resize: the persisted region keys are cleared, so the SAME key
        // is treated as a fresh slot (miss) on the post-resize frame.
        tb.render_with_events(vec![Event::Resize(60, 8)], 0, 0, |ui| {
            let _ = ui.container().cached(5, |ui| {
                ui.text("body");
            });
            hits.set(ui.region_cache_hits());
        });
        assert_eq!(hits.get(), 0, "resize forces a cache miss for all regions");
    }

    /// Focus + hit-map continuity: a button inside a cached region keeps
    /// firing `clicked` across cached (hit) frames because the body always
    /// runs, so its focusable + hit-area are re-registered every frame.
    #[test]
    fn cached_region_preserves_focus_and_hit_map() {
        let mut tb = TestBackend::new(30, 5);
        let clicked = Cell::new(false);

        // Frame 1: register the button so its hit-area lands in the feedback
        // map for the next frame's click resolution. Same key both frames.
        tb.render(|ui| {
            let _ = ui.container().cached(3, |ui| {
                let _ = ui.button("Go");
            });
        });

        // Frame 2: click on the button's cell — even though the region is a
        // cache hit, the body re-ran and re-registered the hit-area, so the
        // click resolves.
        tb.render_with_events(EventBuilder::new().click(2, 0).build(), 0, 1, |ui| {
            let _ = ui.container().cached(3, |ui| {
                let resp = ui.button("Go");
                if resp.clicked {
                    clicked.set(true);
                }
            });
        });
        assert!(
            clicked.get(),
            "button inside a cached region must still receive clicks across hit frames"
        );
    }

    /// Raw-draw inside a cached region: the deferred draw runs on every frame
    /// including cache-hit frames (deferred draws are one-shot per frame, and
    /// the body always runs, so they re-register).
    #[test]
    fn cached_region_raw_draw_replays() {
        let mut tb = TestBackend::new(20, 3);

        let frame = |tb: &mut TestBackend| {
            tb.render(|ui| {
                let _ = ui.container().cached(8, |ui| {
                    ui.container().w(5).h(1).draw(|buf, rect| {
                        buf.set_string(rect.x, rect.y, "XXXXX", crate::style::Style::new());
                    });
                });
            });
        };

        frame(&mut tb);
        tb.assert_contains("XXXXX");

        // Second frame is a cache hit, but the raw draw must still paint.
        frame(&mut tb);
        tb.assert_contains("XXXXX");
    }

    /// Two adjacent cached regions get independent per-call-site slots; one
    /// changing its key does not disturb the other's hit classification.
    #[test]
    fn cached_regions_do_not_collide_per_call_site() {
        let mut tb = TestBackend::new(40, 6);
        let hits = Cell::new(0u32);
        let misses = Cell::new(0u32);

        // Frame 1: both new → 2 misses.
        tb.render(|ui| {
            let _ = ui.container().cached(10, |ui| {
                ui.text("region A");
            });
            let _ = ui.container().cached(20, |ui| {
                ui.text("region B");
            });
        });

        // Frame 2: A unchanged (hit), B changed (miss).
        tb.render(|ui| {
            let _ = ui.container().cached(10, |ui| {
                ui.text("region A");
            });
            let _ = ui.container().cached(21, |ui| {
                ui.text("region B2");
            });
            hits.set(ui.region_cache_hits());
            misses.set(ui.region_cache_misses());
        });
        assert_eq!(hits.get(), 1, "region A unchanged → exactly one hit");
        assert_eq!(misses.get(), 1, "region B changed → exactly one miss");
        tb.assert_contains("region A");
        tb.assert_contains("region B2");
    }
}

#[cfg(test)]
mod gap_saturation_tests {
    use super::*;
    use crate::test_utils::TestBackend;
    use proptest::prelude::*;

    #[test]
    fn every_public_gap_boundary_saturates() {
        let mut state = FrameState::default();
        let mut ui = Context::new(Vec::new(), 20, 5, &mut state, Theme::dark());

        assert_eq!(ui.container().gap(u32::MAX).gap, i32::MAX);
        assert_eq!(ui.container().gap_overlap(u32::MAX).gap, -i32::MAX);
        assert_eq!(ui.container().xs_gap(u32::MAX).gap, i32::MAX);

        // Row/column overrides are converted when the container is finalized.
        let _ = ui.container().row_gap(u32::MAX).col(|_| {});
        let _ = ui.container().col_gap(u32::MAX).row(|_| {});
        let begin_gaps: Vec<i32> = ui
            .commands
            .iter()
            .filter_map(|command| match command {
                Command::BeginContainer(args) => Some(args.gap),
                _ => None,
            })
            .collect();
        assert_eq!(begin_gaps, vec![i32::MAX, i32::MAX]);
    }

    proptest! {
        #[test]
        fn public_and_breakpoint_gaps_never_flip_sign(value in any::<u32>()) {
            let mut state = FrameState::default();
            let mut ui = Context::new(Vec::new(), 20, 5, &mut state, Theme::dark());
            let public = ui.container().gap(value).gap;
            let breakpoint = ui.container().xs_gap(value).gap;

            prop_assert!(public >= 0);
            prop_assert_eq!(public, breakpoint);
            prop_assert_eq!(public, value.min(i32::MAX as u32) as i32);
            prop_assert_eq!(saturating_overlap(value), -public);
        }

        #[test]
        fn arbitrary_large_overlaps_render_without_panicking(value in any::<u32>()) {
            let mut tb = TestBackend::new(8, 2);
            tb.render(|ui| {
                let _ = ui.container().gap_overlap(value).row(|ui| {
                    let _ = ui.container().w(4).col(|ui| { ui.text("left"); });
                    let _ = ui.container().w(4).col(|ui| { ui.text("right"); });
                });
            });
        }
    }
}
