//! Layout family state — scroll state and search highlights, split panes,
//! grid columns, and the split-pane / gutter responses.
//!
//! Rendered by `context/widgets/layout.rs`.

use super::*;

/// A highlighted line range within a scrollable region.
///
/// Used with [`ScrollState::set_highlights`] to mark search results, error
/// lines, or any per-line emphasis. The `scrollable_with_gutter` widget reads
/// the active highlights and renders a background band on matching lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightRange {
    /// First line (0-based, relative to content top).
    pub start_line: usize,
    /// Number of lines in the range (1 = single line).
    pub line_count: usize,
}

impl HighlightRange {
    /// Create a single-line highlight at `line`.
    ///
    /// Field-name pairing: `start_line` + `line_count` → constructor named
    /// `line`. Use [`Self::span`] for multi-line ranges.
    pub fn line(line: usize) -> Self {
        Self {
            start_line: line,
            line_count: 1,
        }
    }

    /// Create a multi-line highlight starting at `start_line` covering `line_count` rows.
    pub fn span(start_line: usize, line_count: usize) -> Self {
        Self {
            start_line,
            line_count: line_count.max(1),
        }
    }

    /// Check whether the given absolute line index falls within this range.
    pub fn contains(&self, line: usize) -> bool {
        line >= self.start_line && line < self.start_line + self.line_count
    }
}

/// State for a scrollable container.
///
/// Pass a mutable reference to `Context::scrollable` each frame. The context
/// updates `offset` and the internal bounds automatically based on mouse wheel
/// and drag events.
///
/// Both axes are tracked (#247): the vertical axis (`offset`, [`scroll_up`] /
/// [`scroll_down`]) drives [`Context::scroll_col`], and the horizontal axis
/// (`offset_x`, [`scroll_left`] / [`scroll_right`]) drives
/// [`Context::scroll_row`]. A single [`ScrollState`] scrolls one axis per
/// container — nest a `scroll_row` inside a `scroll_col` for both. The vertical
/// API is unchanged from earlier versions.
///
/// [`scroll_up`]: ScrollState::scroll_up
/// [`scroll_down`]: ScrollState::scroll_down
/// [`scroll_left`]: ScrollState::scroll_left
/// [`scroll_right`]: ScrollState::scroll_right
/// [`Context::scroll_col`]: crate::Context::scroll_col
/// [`Context::scroll_row`]: crate::Context::scroll_row
#[derive(Debug)]
pub struct ScrollState {
    id: u64,
    /// Reveal newly focused children automatically. Defaults to `true`.
    ///
    /// Also follows changes to the focused child's layout or viewport size.
    /// Manual scrolling is preserved while focus and geometry stay unchanged.
    /// Set to `false` for application-controlled scrolling. Layout corrections
    /// are rendered immediately and synchronized into this state on its next
    /// `Context::scrollable` binding unless the application changed the offset.
    pub follow_focus: bool,
    /// Keep the visible content in place when content above the viewport
    /// changes height. Defaults to `true` (since 0.26).
    ///
    /// After layout, SLT finds the child that was at the top of the viewport
    /// on the previous frame and shifts `offset` by however far that child
    /// moved, like the browser's `overflow-anchor: auto`. Collapsing or
    /// growing a block above the viewport therefore no longer moves the text
    /// under the user's eyes. Children are matched by their
    /// [`group`](crate::Context::group) name when they have one, otherwise by
    /// position plus their leading text, so give repeated, identical-looking
    /// children a group name for reliable anchoring. Anchoring is skipped at
    /// offset 0, while the scrollbar thumb is dragged, on the frame a focused
    /// child is revealed, and while [`stick_to_bottom`](Self::stick_to_bottom)
    /// applies. Set to `false` to keep raw offsets exactly as before.
    pub anchor: bool,
    /// Follow the end of the content while the view is at the bottom.
    /// Defaults to `false`; see [`ScrollState::stick_to_bottom`] (the method).
    ///
    /// While the offset is at its maximum at the end of a frame, content that
    /// grows keeps the view pinned to the new maximum. Scrolling away from the
    /// bottom disengages it; scrolling back to the end re-engages it. Mirrors
    /// `RichLogState::auto_scroll` for any `scrollable`.
    pub stick_to_bottom: bool,
    /// Current vertical scroll offset in rows.
    pub offset: usize,
    /// Current horizontal scroll offset in columns (#247).
    pub offset_x: usize,
    /// Whether the scrollbar thumb is currently being dragged.
    ///
    /// Set to `true` by [`Context::scrollbar`] on a mouse-down inside the
    /// thumb and back to `false` on mouse-up, mirroring
    /// [`SplitPaneState::dragging`](crate::widgets::SplitPaneState). Persists
    /// across frames so cursor motion outside the thumb (or even outside the
    /// track on the x-axis) keeps scrolling while the button is held.
    ///
    /// [`Context::scrollbar`]: crate::Context::scrollbar
    pub dragging: bool,
    content_height: u32,
    viewport_height: u32,
    content_width: u32,
    viewport_width: u32,
    max_offset_y: u32,
    max_offset_x: u32,
    highlights: Vec<HighlightRange>,
    current_highlight: Option<usize>,
}

fn next_scroll_state_id() -> u64 {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl Clone for ScrollState {
    fn clone(&self) -> Self {
        Self {
            id: next_scroll_state_id(),
            follow_focus: self.follow_focus,
            anchor: self.anchor,
            stick_to_bottom: self.stick_to_bottom,
            offset: self.offset,
            offset_x: self.offset_x,
            dragging: self.dragging,
            content_height: self.content_height,
            viewport_height: self.viewport_height,
            content_width: self.content_width,
            viewport_width: self.viewport_width,
            max_offset_y: self.max_offset_y,
            max_offset_x: self.max_offset_x,
            highlights: self.highlights.clone(),
            current_highlight: self.current_highlight,
        }
    }
}

impl ScrollState {
    /// Create scroll state starting at offset 0.
    pub fn new() -> Self {
        Self {
            id: next_scroll_state_id(),
            follow_focus: true,
            anchor: true,
            stick_to_bottom: false,
            offset: 0,
            offset_x: 0,
            dragging: false,
            content_height: 0,
            viewport_height: 0,
            content_width: 0,
            viewport_width: 0,
            max_offset_y: 0,
            max_offset_x: 0,
            highlights: Vec::new(),
            current_highlight: None,
        }
    }

    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// Builder: follow the end of the content while scrolled to the bottom,
    /// for logs, chat transcripts and streaming output.
    ///
    /// Sets the [`stick_to_bottom`](ScrollState#structfield.stick_to_bottom)
    /// field. A wheel-up or any other scroll away from the end disengages it;
    /// returning to the end re-engages it. Since 0.26.0.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::ScrollState;
    /// let log = ScrollState::new().stick_to_bottom(true);
    /// assert!(log.stick_to_bottom);
    /// ```
    pub fn stick_to_bottom(mut self, enabled: bool) -> Self {
        self.stick_to_bottom = enabled;
        self
    }

    /// Check if scrolling upward is possible (offset is greater than 0).
    pub fn can_scroll_up(&self) -> bool {
        self.offset > 0
    }

    /// Check if scrolling downward is possible (content extends below the viewport).
    pub fn can_scroll_down(&self) -> bool {
        self.offset < self.max_offset()
    }

    /// Get the total content height in rows.
    pub fn content_height(&self) -> u32 {
        self.content_height
    }

    /// Get the viewport height in rows.
    pub fn viewport_height(&self) -> u32 {
        self.viewport_height
    }

    /// Maximum vertical offset from the most recent viewport binding.
    /// Accounts for leading clipping; it need not equal content minus visible height.
    pub fn max_offset(&self) -> usize {
        self.max_offset_y as usize
    }

    /// Maximum horizontal offset from the most recent viewport binding.
    pub fn max_offset_x(&self) -> usize {
        self.max_offset_x as usize
    }

    /// Get the scroll progress as a ratio in `[0.0, 1.0]`.
    ///
    /// Returns `f64` to match the rest of the ratio surface unified in v0.20
    /// (`Gauge::ratio`, `SplitPaneState::ratio`, `progress(ratio)`,
    /// `progress_bar(ratio)`). Feed the value straight into [`Context::gauge`]
    /// or [`Context::progress_bar`] without a cast.
    ///
    /// [`Context::gauge`]: crate::Context::gauge
    /// [`Context::progress_bar`]: crate::Context::progress_bar
    ///
    /// ```no_run
    /// # use slt::ScrollState;
    /// let scroll = ScrollState::new();
    /// // Bounds are populated by the `scrollable` widget each frame; a fresh
    /// // state with no content reports 0.0.
    /// let ratio: f64 = scroll.progress_ratio();
    /// assert!((0.0..=1.0).contains(&ratio));
    /// ```
    pub fn progress_ratio(&self) -> f64 {
        let max = self.max_offset();
        if max == 0 {
            0.0
        } else {
            self.offset.min(max) as f64 / max as f64
        }
    }

    /// Scroll up by the given number of rows, clamped to 0.
    pub fn scroll_up(&mut self, amount: usize) {
        self.offset = self.offset.saturating_sub(amount);
    }

    /// Scroll down by the given number of rows, clamped to the maximum offset.
    pub fn scroll_down(&mut self, amount: usize) {
        let max_offset = self.max_offset();
        self.offset = self.offset.saturating_add(amount).min(max_offset);
    }

    /// Set the absolute scroll offset, clamped to `[0, content - viewport]`.
    ///
    /// Uses the same `max_offset` semantics as [`scroll_down`](Self::scroll_down).
    /// Click-to-jump and thumb-drag in [`Context::scrollbar`] route through
    /// this so an out-of-range target row never leaves the offset past the
    /// last full screen of content. Direct `state.offset = …` writes keep
    /// working; this is the clamping-safe alternative.
    ///
    /// [`Context::scrollbar`]: crate::Context::scrollbar
    ///
    /// ```no_run
    /// # use slt::widgets::ScrollState;
    /// let mut scroll = ScrollState::new();
    /// // Bounds are populated by the `scrollable` widget each frame; on a
    /// // fresh state max_offset is 0 so any target clamps to 0.
    /// scroll.set_offset(999);
    /// assert_eq!(scroll.offset, 0);
    /// ```
    pub fn set_offset(&mut self, offset: usize) {
        let max_offset = self.max_offset();
        self.offset = offset.min(max_offset);
    }

    pub(crate) fn set_bounds(&mut self, content_height: u32, viewport_height: u32) {
        self.content_height = content_height;
        self.viewport_height = viewport_height;
        self.max_offset_y = content_height.saturating_sub(viewport_height);
    }

    pub(crate) fn set_clipped_bounds(
        &mut self,
        content: u32,
        viewport: u32,
        max: u32,
        horizontal: bool,
    ) {
        if horizontal {
            self.set_bounds_x(content, viewport);
            self.max_offset_x = max;
        } else {
            self.set_bounds(content, viewport);
            self.max_offset_y = max;
        }
    }

    /// Update the horizontal (x-axis) bounds (#247).
    ///
    /// Called by [`Context::scroll_row`] / [`Context::scrollable`] each frame
    /// when the bound scrollable scrolls horizontally. The vertical
    /// [`set_bounds`](Self::set_bounds) is left untouched, keeping the two axes
    /// independent.
    ///
    /// [`Context::scroll_row`]: crate::Context::scroll_row
    /// [`Context::scrollable`]: crate::Context::scrollable
    pub(crate) fn set_bounds_x(&mut self, content_width: u32, viewport_width: u32) {
        self.content_width = content_width;
        self.viewport_width = viewport_width;
        self.max_offset_x = content_width.saturating_sub(viewport_width);
    }

    /// Check if scrolling left is possible (`offset_x` is greater than 0, #247).
    ///
    /// ```no_run
    /// # use slt::ScrollState;
    /// let scroll = ScrollState::new();
    /// assert!(!scroll.can_scroll_left());
    /// ```
    pub fn can_scroll_left(&self) -> bool {
        self.offset_x > 0
    }

    /// Check if scrolling right is possible (content extends past the right
    /// edge of the viewport, #247).
    ///
    /// ```no_run
    /// # use slt::ScrollState;
    /// let scroll = ScrollState::new();
    /// // A fresh state with no content cannot scroll right.
    /// assert!(!scroll.can_scroll_right());
    /// ```
    pub fn can_scroll_right(&self) -> bool {
        self.offset_x < self.max_offset_x()
    }

    /// Total horizontal content width in columns (#247).
    pub fn content_width(&self) -> u32 {
        self.content_width
    }

    /// Horizontal viewport width in columns (#247).
    pub fn viewport_width(&self) -> u32 {
        self.viewport_width
    }

    /// Horizontal scroll progress as a ratio in `[0.0, 1.0]` (#247).
    ///
    /// The x-axis mirror of [`progress_ratio`](Self::progress_ratio). Returns
    /// `0.0` when the content fits the viewport (no horizontal overflow). Feed
    /// it to a future horizontal scrollbar, a position readout, or a minimap.
    ///
    /// ```no_run
    /// # use slt::ScrollState;
    /// let scroll = ScrollState::new();
    /// let p: f64 = scroll.progress_x();
    /// assert!((0.0..=1.0).contains(&p));
    /// ```
    pub fn progress_x(&self) -> f64 {
        let max = self.max_offset_x();
        if max == 0 {
            0.0
        } else {
            self.offset_x.min(max) as f64 / max as f64
        }
    }

    /// Scroll left by the given number of columns, clamped to 0 (#247).
    ///
    /// ```no_run
    /// # use slt::ScrollState;
    /// let mut scroll = ScrollState::new();
    /// scroll.scroll_left(4); // clamps at 0 with no content
    /// assert_eq!(scroll.offset_x, 0);
    /// ```
    pub fn scroll_left(&mut self, amount: usize) {
        self.offset_x = self.offset_x.saturating_sub(amount);
    }

    /// Scroll right by the given number of columns, clamped to the maximum
    /// horizontal offset (#247).
    ///
    /// ```no_run
    /// # use slt::ScrollState;
    /// let mut scroll = ScrollState::new();
    /// scroll.scroll_right(4); // clamps to content bounds (0 with no content)
    /// assert_eq!(scroll.offset_x, 0);
    /// ```
    pub fn scroll_right(&mut self, amount: usize) {
        let max_offset = self.max_offset_x();
        self.offset_x = self.offset_x.saturating_add(amount).min(max_offset);
    }

    /// Set the active highlight ranges. Replaces any previous highlights.
    ///
    /// Selecting the first highlight automatically when the list is non-empty
    /// matches the behavior of search-result navigation in code editors.
    pub fn set_highlights(&mut self, ranges: &[HighlightRange]) {
        self.highlights.clear();
        self.highlights.extend_from_slice(ranges);
        self.current_highlight = if self.highlights.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Read-only access to the active highlight ranges.
    pub fn highlights(&self) -> &[HighlightRange] {
        &self.highlights
    }

    /// Index of the currently focused highlight, if any.
    pub fn current_highlight(&self) -> Option<usize> {
        self.current_highlight
    }

    /// Clear all highlights and reset the current index.
    pub fn clear_highlights(&mut self) {
        self.highlights.clear();
        self.current_highlight = None;
    }

    /// Advance to the next highlight, scrolling the viewport to show it.
    /// Wraps from last to first.
    pub fn highlight_next(&mut self) {
        if self.highlights.is_empty() {
            return;
        }
        let next = match self.current_highlight {
            Some(i) => (i + 1) % self.highlights.len(),
            None => 0,
        };
        self.current_highlight = Some(next);
        self.scroll_to_current_highlight();
    }

    /// Move to the previous highlight, scrolling the viewport to show it.
    /// Wraps from first to last.
    pub fn highlight_previous(&mut self) {
        if self.highlights.is_empty() {
            return;
        }
        let next = match self.current_highlight {
            Some(i) => {
                if i == 0 {
                    self.highlights.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.current_highlight = Some(next);
        self.scroll_to_current_highlight();
    }

    /// Scroll the viewport so the currently focused highlight is visible
    /// with one line of context above when possible.
    pub fn scroll_to_current_highlight(&mut self) {
        let Some(idx) = self.current_highlight else {
            return;
        };
        let Some(range) = self.highlights.get(idx).copied() else {
            return;
        };
        let target = range.start_line;
        let viewport = self.viewport_height as usize;
        let max_offset = self.max_offset();
        if target < self.offset {
            self.offset = target.saturating_sub(1).min(max_offset);
        } else if viewport > 0 && target >= self.offset + viewport {
            let desired = target + 2;
            let new_offset = desired.saturating_sub(viewport);
            self.offset = new_offset.min(max_offset);
        } else if self.offset > max_offset {
            self.offset = max_offset;
        }
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for a [`crate::Context::split_pane`] /
/// [`crate::Context::vsplit_pane`] container.
///
/// Tracks the split ratio and drag state. Pass a mutable reference each frame
/// — the widget updates `ratio` in place when the user drags the handle or
/// presses arrow keys with the handle focused.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitPaneState {
    /// Fraction of space given to the first pane. Clamped to
    /// `[min_ratio, 1.0 - min_ratio]`.
    pub ratio: f64,
    /// Whether the handle is currently being dragged.
    pub dragging: bool,
    /// Minimum fraction allocated to either pane. Default: `0.10`.
    pub min_ratio: f64,
}

/// Default minimum fraction of either pane, used by [`SplitPaneState::new`].
///
/// Crate-internal: there is no public path that benefits from constructing
/// with this constant — call [`SplitPaneState::new`] for the default (0.10)
/// or [`SplitPaneState::with_min_ratio`] to override per-instance.
pub(crate) const DEFAULT_SPLIT_MIN_RATIO: f64 = 0.10;

impl SplitPaneState {
    /// Create split state with the given initial ratio, clamped to
    /// `[DEFAULT_SPLIT_MIN_RATIO, 1.0 - DEFAULT_SPLIT_MIN_RATIO]` (default
    /// `[0.10, 0.90]`).
    pub fn new(ratio: f64) -> Self {
        let min_ratio = DEFAULT_SPLIT_MIN_RATIO;
        let clamped = ratio.clamp(min_ratio, 1.0 - min_ratio);
        Self {
            ratio: clamped,
            dragging: false,
            min_ratio,
        }
    }

    /// Override the minimum ratio for either pane (clamped to `[0.0, 0.49]`).
    pub fn with_min_ratio(mut self, min: f64) -> Self {
        self.min_ratio = min.clamp(0.0, 0.49);
        self.ratio = self.ratio.clamp(self.min_ratio, 1.0 - self.min_ratio);
        self
    }

    /// Set the ratio, clamped to `[min_ratio, 1.0 - min_ratio]`.
    pub fn set_ratio(&mut self, ratio: f64) {
        self.ratio = ratio.clamp(self.min_ratio, 1.0 - self.min_ratio);
    }
}

impl Default for SplitPaneState {
    fn default() -> Self {
        Self::new(0.5)
    }
}

/// Column specification for [`crate::Context::grid_with()`].
///
/// Controls the width allocation of individual columns in a grid layout.
///
/// # Example
///
/// ```no_run
/// use slt::GridColumn;
/// # slt::run(|ui: &mut slt::Context| {
/// ui.grid_columns(&[
///     GridColumn::Fixed(8),   // label column: exactly 8 chars
///     GridColumn::Grow(1),    // flexible column
///     GridColumn::Grow(1),    // flexible column
///     GridColumn::Fixed(4),   // status column: exactly 4 chars
/// ], |ui| {
///     // children placed left-to-right, wrapping to next row
/// });
/// # });
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GridColumn {
    /// Equal-width column with grow weight 1 (default `grid()` behavior).
    Auto,
    /// Fixed-width column in character cells. Does not grow or shrink.
    Fixed(u32),
    /// Flexible column with a custom grow weight. Higher values take
    /// proportionally more space.
    Grow(u16),
    /// Column sized as a percentage (1–100) of the grid width.
    Percent(u8),
}

/// Response from [`Context::split_pane`](crate::Context::split_pane) and
/// [`Context::vsplit_pane`](crate::Context::vsplit_pane).
///
/// Wraps the row-level [`Response`] of the outer container plus drag state.
/// Implements `Deref<Target = Response>` so `r.hovered` etc. work directly.
#[derive(Debug, Clone, Default)]
#[must_use = "SplitPaneResponse contains interaction state — check .ratio, .drag_active, or .hovered"]
pub struct SplitPaneResponse {
    /// The row/column-level interaction response.
    pub response: Response,
    /// Current ratio after this frame's interaction (mirrors `state.ratio`).
    pub ratio: f64,
    /// Whether the handle was actively being dragged this frame.
    pub drag_active: bool,
}

impl std::ops::Deref for SplitPaneResponse {
    type Target = Response;
    fn deref(&self) -> &Response {
        &self.response
    }
}

/// Response from [`Context::scrollable_with_gutter`](crate::Context::scrollable_with_gutter).
///
/// Wraps the [`Response`] for the scrollable region plus search-result
/// metadata: the index of the currently focused highlight (if any) and the
/// total count of registered highlights.
#[derive(Debug, Clone, Default)]
#[must_use = "GutterResponse contains interaction state — check .current_highlight or .hovered"]
pub struct GutterResponse {
    /// The scrollable region's interaction response.
    pub response: Response,
    /// Index of the currently focused highlight, if any.
    pub current_highlight: Option<usize>,
    /// Total number of active highlights.
    pub total_highlights: usize,
}

impl std::ops::Deref for GutterResponse {
    type Target = Response;
    fn deref(&self) -> &Response {
        &self.response
    }
}

#[cfg(test)]
mod scroll_state_progress_tests {
    use super::ScrollState;

    /// Build a state with the bounds the `scrollable` widget would set, plus an
    /// offset, so `progress_ratio` exercises a realistic non-zero ratio.
    fn scrolled(content_height: u32, viewport_height: u32, offset: usize) -> ScrollState {
        let mut state = ScrollState::new();
        state.set_bounds(content_height, viewport_height);
        state.offset = offset;
        state
    }

    #[test]
    fn progress_ratio_returns_f64_in_unit_range() {
        // Top of a scrollable region → 0.0.
        let top = scrolled(100, 20, 0);
        let ratio: f64 = top.progress_ratio();
        assert_eq!(ratio, 0.0);

        // Halfway through the scrollable range (offset 40 of max 80) → 0.5.
        let mid = scrolled(100, 20, 40);
        assert_eq!(mid.progress_ratio(), 0.5);

        // Fully scrolled (offset == max) → 1.0.
        let bottom = scrolled(100, 20, 80);
        assert_eq!(bottom.progress_ratio(), 1.0);
    }

    #[test]
    fn progress_ratio_is_zero_when_content_fits_viewport() {
        // No overflow → no scroll range → 0.0 (and no divide-by-zero).
        let fits = scrolled(20, 20, 0);
        assert_eq!(fits.progress_ratio(), 0.0);

        let smaller = scrolled(10, 20, 5);
        assert_eq!(smaller.progress_ratio(), 0.0);
    }

    #[test]
    fn progress_ratio_preserves_f64_precision() {
        // 1/3 is lossy in f32; the f64 surface keeps more digits than `as f32`.
        let third = scrolled(40, 10, 10); // max = 30, offset = 10 → 1/3
        let ratio = third.progress_ratio();
        assert!((ratio - 1.0 / 3.0).abs() < 1e-12);
    }
}
