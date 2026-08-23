//! Input widgets — text input, textarea, sliders, spinners, toasts,
//! progress bars.
//!
//! Layer 3 widgets that consume key/mouse events when focused. State
//! types live in [`crate::widgets`] (e.g. [`crate::widgets::TextInputState`]).

use super::*;

fn has_global_shortcut_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.contains(KeyModifiers::CONTROL)
        || modifiers.contains(KeyModifiers::SUPER)
        || modifiers.contains(KeyModifiers::META)
}

mod feedback;
mod text_input;
mod textarea_progress;

#[cfg(test)]
mod tests;
