//! Display family state — stat trends and the breadcrumb response.
//!
//! Rendered by `context/widgets/display.rs`.

use super::*;

/// Direction indicator for stat widgets.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    /// Positive movement.
    Up,
    /// Negative movement.
    Down,
}

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
