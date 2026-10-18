//! The [`Command`] stream that `Context` records each frame and the layout
//! tree is built from.

use super::*;

/// Main axis direction for a container's children.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Lay out children horizontally (left to right).
    Row,
    /// Lay out children vertically (top to bottom).
    Column,
}

/// Arguments for [`Command::BeginContainer`].
///
/// Stored inline in the variant so a container costs no heap allocation
/// per frame (it was boxed under #64); inlining it and
/// boxing only the rare `title` keeps `Command` at 136 bytes with no
/// per-container allocation.
#[derive(Debug, Clone)]
pub(crate) struct BeginContainerArgs {
    pub direction: Direction,
    /// Signed inter-child gap (negative = overlap). See
    /// [`crate::ContainerBuilder::gap_overlap`] (#222).
    pub gap: i32,
    pub align: Align,
    pub align_self: Option<Align>,
    pub justify: Justify,
    pub border: Option<Border>,
    pub border_sides: BorderSides,
    pub border_style: Style,
    pub bg_color: Option<Color>,
    pub padding: Padding,
    pub margin: Margin,
    pub constraints: Constraints,
    /// Boxed: titles are rare, and inline they would add 32 bytes to every
    /// `Command`. A titled container already allocates its `String`.
    pub title: Option<Box<(String, Style)>>,
    pub grow: u16,
    pub group_name: Option<std::sync::Arc<str>>,
}

/// Arguments for [`Command::BeginScrollable`].
///
/// Stored inline like [`BeginContainerArgs`]; this is the widest variant
/// and sets the size of `Command`.
///
/// `direction` selects the scroll axis (#247): `Direction::Column` scrolls
/// vertically (the historic default), `Direction::Row` scrolls horizontally.
/// Both vertical and horizontal offsets are carried so the tree builder can
/// apply the one matching `direction` without re-querying state mid-frame.
#[derive(Debug, Clone)]
pub(crate) struct BeginScrollableArgs {
    pub grow: u16,
    /// Scroll axis: `Column` => vertical scroll, `Row` => horizontal (#247).
    pub direction: Direction,
    pub border: Option<Border>,
    pub border_sides: BorderSides,
    pub border_style: Style,
    /// Background color (dark-mode resolved). Fixes #142.
    pub bg_color: Option<Color>,
    /// Main-axis child alignment. Fixes #142.
    pub align: Align,
    /// Cross-axis self alignment. Fixes #142.
    pub align_self: Option<Align>,
    /// Main-axis justification. Fixes #142.
    pub justify: Justify,
    /// Signed gap between children in cells (negative = overlap, #222).
    /// Fixes #142.
    pub gap: i32,
    pub padding: Padding,
    pub margin: Margin,
    pub constraints: Constraints,
    /// Boxed: titles are rare, and inline they would add 32 bytes to every
    /// `Command`. A titled container already allocates its `String`.
    pub title: Option<Box<(String, Style)>>,
    /// Vertical scroll offset in rows (used when `direction == Column`).
    pub scroll_offset: u32,
    /// Horizontal scroll offset in columns (used when `direction == Row`, #247).
    pub scroll_offset_x: u32,
    pub scroll_follow_focus: Option<bool>,
    pub scroll_state_id: u64,
    /// Group name for hover/focus registration. Fixes #141.
    pub group_name: Option<std::sync::Arc<str>>,
}

#[derive(Debug, Clone)]
pub(crate) enum Command {
    Text {
        content: String,
        /// Grapheme index of the visible caret marker in `content`.
        cursor_offset: Option<usize>,
        cursor_masked: bool,
        style: Style,
        grow: u16,
        align: Align,
        wrap: bool,
        truncate: bool,
        margin: Margin,
        constraints: Constraints,
    },
    BeginContainer(BeginContainerArgs),
    BeginScrollable(BeginScrollableArgs),
    Link {
        text: String,
        url: String,
        style: Style,
        wrap: bool,
        margin: Margin,
        constraints: Constraints,
    },
    RichText {
        segments: Vec<(String, Style)>,
        wrap: bool,
        align: Align,
        margin: Margin,
        constraints: Constraints,
    },
    EndContainer,
    BeginOverlay {
        modal: bool,
    },
    EndOverlay,
    Spacer {
        grow: u16,
    },
    FocusMarker(usize),
    InteractionMarker(usize),
    /// Marks the next container / scrollable as opt-in flex-shrink.
    ///
    /// Pushed by [`crate::context::ContainerBuilder::shrink`] just before the
    /// matching `BeginContainer` / `BeginScrollable`. Consumed by
    /// `build_children` like [`Command::FocusMarker`] (buffered into
    /// `pending_shrink`, applied to the next built [`super::tree::LayoutNode`]).
    /// Closes #161.
    ShrinkMarker,
    /// Marks the next container as a wrapping (multi-line) row, carrying the
    /// signed cross-axis (between-line) gap.
    ///
    /// Pushed by [`crate::context::ContainerBuilder::wrap`] just before the
    /// matching `BeginContainer` / `BeginScrollable`. Consumed by
    /// `build_children` like [`Command::ShrinkMarker`] (buffered into
    /// `pending_wrap`, applied to the next built
    /// [`super::tree::LayoutNode`]). On a `Row` container the child layout
    /// flows children onto subsequent lines on main-axis overflow, with the
    /// payload as the between-line gap; on a `Column` container it is a
    /// documented no-op. Kept a scalar variant so the `Command` enum stays
    /// small. Closes #258.
    WrapMarker(i32),
    /// Sets the flex-basis (initial main-axis size, in cells) on the next
    /// container.
    ///
    /// Pushed by [`crate::context::ContainerBuilder::basis`] just before the
    /// matching `BeginContainer` / `BeginScrollable`. Buffered into
    /// `pending_basis` and applied to the next built
    /// [`super::tree::LayoutNode::flex_basis`]. A scalar variant so the
    /// `Command` enum stays small. Closes #258.
    BasisMarker(u32),
    RawDraw {
        draw_id: usize,
        constraints: Constraints,
        grow: u16,
        margin: Margin,
    },
}

#[cfg(test)]
mod size_tests {
    use super::Command;

    /// Regression guard for the `Command` enum size.
    ///
    /// A frame may push hundreds of `Command` values into a single `Vec`,
    /// so every byte in the enum variant-union multiplies across the frame.
    /// The container variants are inline (no allocation per
    /// container per frame); the widest one, `BeginScrollable`, sets the
    /// size. In the matched harness the larger copy costs less than the
    /// malloc/free pair it replaces (every workload equal or faster within
    /// noise). If this test fires after a refactor, box a rare
    /// field (as `title` is) or bump this bound with a measurement.
    #[test]
    fn command_enum_size_is_bounded() {
        const MAX_BYTES: usize = 136;
        let actual = std::mem::size_of::<Command>();
        assert!(
            actual <= MAX_BYTES,
            "Command enum grew to {actual} bytes (limit {MAX_BYTES}); \
             consider boxing the new fat variant"
        );
    }
}
