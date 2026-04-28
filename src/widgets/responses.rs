// Widget response types introduced in v0.20.0.
//
// These wrap a [`Response`] alongside widget-specific interaction data
// (clicked segment index, drag state, search highlight info). Each implements
// [`Deref<Target = Response>`] so callers can read the standard `hovered`,
// `clicked`, `rect`, and `focused` fields without explicit field navigation.

/// Response from [`Context::breadcrumb`](crate::Context::breadcrumb).
///
/// Wraps the row-level [`Response`] and exposes the index of the clicked
/// segment (if any). Implements `Deref<Target = Response>` so `r.hovered`,
/// `r.rect`, etc. work directly.
///
/// # Example
///
/// ```no_run
/// # slt::run(|ui: &mut slt::Context| {
/// let r = ui.breadcrumb(&["Home", "Settings", "Profile"]).show();
/// if let Some(i) = r.clicked_segment {
///     // navigate to segment `i`
/// }
/// if r.hovered {
///     // whole bar hovered
/// }
/// # });
/// ```
#[derive(Debug, Clone, Default)]
#[must_use = "BreadcrumbResponse contains interaction state — check .clicked_segment, .hovered, or .rect"]
pub struct BreadcrumbResponse {
    /// The row-level interaction response (hover, rect, focus).
    pub response: Response,
    /// Index of the clicked segment, if any.
    pub clicked_segment: Option<usize>,
}

impl std::ops::Deref for BreadcrumbResponse {
    type Target = Response;
    fn deref(&self) -> &Response {
        &self.response
    }
}

/// Response from [`Context::gauge`](crate::Context::gauge) and
/// [`Context::line_gauge`](crate::Context::line_gauge).
///
/// Wraps the row-level [`Response`] plus the rendered ratio so callers can
/// confirm the displayed value (clamped to `0.0..=1.0`). Implements
/// `Deref<Target = Response>` so `r.hovered` etc. work directly.
///
/// Note: `ratio` was widened from `f32` to `f64` in v0.20.0 so the gauge
/// family aligns with `animate_value`, chart APIs, and `progress_bar`.
#[derive(Debug, Clone, Default)]
#[must_use = "GaugeResponse contains interaction state — check .hovered or .ratio"]
pub struct GaugeResponse {
    /// The row-level interaction response.
    pub response: Response,
    /// The clamped ratio that was rendered (always `0.0..=1.0`).
    pub ratio: f64,
}

impl std::ops::Deref for GaugeResponse {
    type Target = Response;
    fn deref(&self) -> &Response {
        &self.response
    }
}

/// Options struct for the deprecated
/// [`Context::line_gauge_with`](crate::Context::line_gauge_with) shim.
///
/// New code should use the chainable builder
/// `ui.line_gauge(ratio).label(...).width(...).filled(...).empty(...)`.
/// This struct stays around for one minor cycle to ease migration of v0.19
/// call sites that constructed it directly.
#[derive(Debug, Clone)]
pub struct LineGaugeOpts {
    /// Fill character. Default: `'━'`.
    pub filled: char,
    /// Empty character. Default: `'─'`.
    pub empty: char,
    /// Width in terminal cells. `None` falls back to a default of 20 cells.
    pub width: Option<u32>,
    /// Optional label appended after the bar (e.g., `"60%"`).
    pub label: Option<String>,
}

impl Default for LineGaugeOpts {
    fn default() -> Self {
        Self {
            filled: '━',
            empty: '─',
            width: None,
            label: None,
        }
    }
}

impl LineGaugeOpts {
    /// Set the label appended after the bar.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set an explicit bar width in terminal cells.
    pub fn width(mut self, width: u32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set a custom fill character.
    pub fn filled(mut self, ch: char) -> Self {
        self.filled = ch;
        self
    }

    /// Set a custom empty character.
    pub fn empty(mut self, ch: char) -> Self {
        self.empty = ch;
        self
    }
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
    pub ratio: f32,
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
