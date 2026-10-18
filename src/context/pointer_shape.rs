//! Mouse pointer shape (OSC 22) requests (#452).
//!
//! The shape follows the hovered slot each frame. It travels with the other
//! terminal metadata (#468): the emitter writes OSC 22 only when the shape
//! changes and the session restores the default pointer on exit, in the
//! panic hook and on suspend, only if a shape was ever written. The text
//! cursor shape is [`Context::set_cursor_shape`].

use super::*;

/// Mouse pointer shape requested with [`ContainerBuilder::pointer_shape`].
///
/// Emitted as OSC 22 (`ESC ] 22 ; <name> ESC \`) with the CSS cursor name,
/// which kitty, foot, Ghostty and recent xterm builds understand. Terminals
/// that do not support OSC 22 may print the payload, so SLT only emits it
/// when the app opts in with
/// [`RunConfig::pointer_shapes(true)`](crate::RunConfig::pointer_shapes).
/// Shapes follow the hovered slot, so they need
/// [`MouseMode::Hover`](crate::MouseMode::Hover) to track plain pointer moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerShape {
    /// The terminal's default pointer.
    Default,
    /// A pointing hand, for clickable things (CSS `pointer`).
    Pointer,
    /// An I-beam over selectable or editable text.
    Text,
    /// An open hand over something that can be dragged.
    Grab,
    /// A closed hand while dragging.
    Grabbing,
    /// The action is not available.
    NotAllowed,
    /// Horizontal resize handle (vertical split divider).
    ColResize,
    /// Vertical resize handle (horizontal split divider).
    RowResize,
    /// Precise selection.
    Crosshair,
    /// Busy.
    Wait,
    /// Help is available.
    Help,
    /// Something can be moved.
    Move,
}

impl PointerShape {
    /// CSS cursor name sent in the OSC 22 payload.
    pub(crate) fn css_name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Pointer => "pointer",
            Self::Text => "text",
            Self::Grab => "grab",
            Self::Grabbing => "grabbing",
            Self::NotAllowed => "not-allowed",
            Self::ColResize => "col-resize",
            Self::RowResize => "row-resize",
            Self::Crosshair => "crosshair",
            Self::Wait => "wait",
            Self::Help => "help",
            Self::Move => "move",
        }
    }
}

impl ContainerBuilder<'_> {
    /// Show `shape` as the mouse pointer while this container is the
    /// innermost hovered slot (descendants without a shape inherit it, like
    /// CSS `cursor`).
    ///
    /// Only emitted when the app opts in with
    /// [`RunConfig::pointer_shapes(true)`](crate::RunConfig::pointer_shapes)
    /// on a terminal that supports OSC 22, and only when the shape under the
    /// pointer changes. Since 0.26.0.
    ///
    /// # Example
    ///
    /// ```
    /// # slt::TestBackend::new(30, 3).render(|ui| {
    /// let card = ui
    ///     .bordered(slt::Border::Rounded)
    ///     .pointer_shape(slt::PointerShape::Pointer)
    ///     .col(|ui| {
    ///         ui.text("Open");
    ///     });
    /// # let _ = card;
    /// # });
    /// ```
    pub fn pointer_shape(mut self, shape: PointerShape) -> Self {
        self.pointer_shape = Some(shape);
        self
    }
}
