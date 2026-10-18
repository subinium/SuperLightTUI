//! Text cursor shape (DECSCUSR) and pointer shape (OSC 22) requests (#452).
//!
//! Both are per-frame requests: a frame that does not ask for a shape gets
//! the terminal default again. The runtime writes an escape only when the
//! wanted shape changes, and the session restores the defaults on exit, in
//! the panic hook and on suspend, but only if a shape was ever written.

use super::*;

/// Text cursor shape requested with [`Context::set_cursor_shape`].
///
/// Written as DECSCUSR (`CSI n SP q`), which xterm, kitty, WezTerm, Ghostty,
/// iTerm2, Alacritty, foot, Windows Terminal and tmux understand. The shape
/// only matters while the text cursor is visible (a focused text input).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorShape {
    /// Full-cell block.
    Block,
    /// Underline.
    Underline,
    /// Vertical bar (I-beam), the usual shape for text entry.
    Bar,
}

impl CursorShape {
    /// DECSCUSR parameter for this shape.
    pub(crate) fn decscusr(self, blink: bool) -> u8 {
        match (self, blink) {
            (Self::Block, true) => 1,
            (Self::Block, false) => 2,
            (Self::Underline, true) => 3,
            (Self::Underline, false) => 4,
            (Self::Bar, true) => 5,
            (Self::Bar, false) => 6,
        }
    }
}

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

impl Context {
    /// Request a text cursor shape for this frame.
    ///
    /// The shape applies while the text cursor is visible (a focused text
    /// input). Call it every frame you want the shape; a frame without a
    /// request returns the cursor to the terminal's default shape. The
    /// runtime writes DECSCUSR only when the wanted shape changes and restores
    /// the default (`CSI 0 SP q`) on exit, in the panic hook and on suspend.
    /// Headless backends ignore it. Since 0.26.0.
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::{CursorShape, TextInputState};
    /// # let mut input = TextInputState::new();
    /// # slt::TestBackend::new(30, 3).render(|ui| {
    /// let r = ui.text_input(&mut input);
    /// if r.focused {
    ///     ui.set_cursor_shape(CursorShape::Bar, true);
    /// }
    /// # });
    /// ```
    pub fn set_cursor_shape(&mut self, shape: CursorShape, blink: bool) {
        self.pointer.cursor_request = Some((shape, blink));
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
