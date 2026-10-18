//! `Context` widget methods, grouped by widget family.
//!
//! Every family file here pairs with the state file of the same name in
//! [`crate::widgets`]: `context/widgets/selection.rs` renders the types in
//! `widgets/selection.rs`, `context/widgets/collections.rs` renders the types
//! in `widgets/collections.rs`, and so on. Families without persistent state
//! (`media`, `viz`, `events`) have no state file. See
//! `docs/ARCHITECTURE.md` for the full family table.

use super::*;

fn has_global_shortcut_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.contains(KeyModifiers::CONTROL)
        || modifiers.contains(KeyModifiers::SUPER)
        || modifiers.contains(KeyModifiers::META)
}

mod collections;
mod commanding;
mod display;
mod events;
mod feedback;
mod layout;
mod media;
mod overlay;
mod rich;
mod selection;
mod text_input;
mod viz;

pub use display::Breadcrumb;
pub use feedback::{Gauge, LineGauge};
pub use layout::GutterOpts;
#[allow(deprecated)]
pub use overlay::ModalOptions;
pub(super) use overlay::wrap_tooltip_text;
pub use overlay::{Anchor, ModalOpts, Placement};
pub use rich::{CodeBlock, DiffView};
pub use text_input::CodeEditor;
pub use viz::TreemapItem;
