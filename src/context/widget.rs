use super::*;

/// Trait for creating custom widgets.
///
/// Implement this trait to build reusable, composable widgets with full access
/// to the [`Context`] API — focus, events, theming, layout, and mouse interaction.
/// Choose `Self::Response` based on what the widget needs to report:
/// `()` for pure display, `bool` for simple changed/not-changed signals,
/// or [`Response`] for click/hover/focus-aware interaction.
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
