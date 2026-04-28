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
/// Boxed inside the variant so that the surrounding `Command` enum stays
/// small — a frame may contain hundreds of commands, and most variants
/// (for example `EndContainer`) carry no payload.
#[derive(Debug, Clone)]
pub(crate) struct BeginContainerArgs {
    pub direction: Direction,
    pub gap: u32,
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
    pub title: Option<(String, Style)>,
    pub grow: u16,
    pub group_name: Option<std::sync::Arc<str>>,
}

/// Arguments for [`Command::BeginScrollable`].
///
/// Boxed for the same reason as [`BeginContainerArgs`] — keeps the
/// `Command` enum from being dragged up to the width of this payload.
///
/// Note: `direction` is intentionally omitted — scrollable containers are
/// always `Direction::Column` (vertical scroll only).
#[derive(Debug, Clone)]
pub(crate) struct BeginScrollableArgs {
    pub grow: u16,
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
    /// Gap between children in pixels. Fixes #142.
    pub gap: u32,
    pub padding: Padding,
    pub margin: Margin,
    pub constraints: Constraints,
    pub title: Option<(String, Style)>,
    pub scroll_offset: u32,
    /// Group name for hover/focus registration. Fixes #141.
    pub group_name: Option<std::sync::Arc<str>>,
}

#[derive(Debug, Clone)]
pub(crate) enum Command {
    Text {
        content: String,
        cursor_offset: Option<usize>,
        style: Style,
        grow: u16,
        align: Align,
        wrap: bool,
        truncate: bool,
        margin: Margin,
        constraints: Constraints,
    },
    BeginContainer(Box<BeginContainerArgs>),
    BeginScrollable(Box<BeginScrollableArgs>),
    Link {
        text: String,
        url: String,
        style: Style,
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
    /// Fat variants (`BeginContainer`, `BeginScrollable`) are boxed to keep
    /// the common 0-payload variants (e.g. `EndContainer`) cheap.
    ///
    /// The current ceiling reflects the largest remaining inline variant
    /// (`Text`, which carries a `String` + `Constraints` + `Style` + small
    /// scalars). If this test fires after a refactor, either box the new
    /// fat variant or bump this bound with justification.
    #[test]
    fn command_enum_size_is_bounded() {
        const MAX_BYTES: usize = 128;
        let actual = std::mem::size_of::<Command>();
        assert!(
            actual <= MAX_BYTES,
            "Command enum grew to {actual} bytes (limit {MAX_BYTES}); \
             consider boxing the new fat variant"
        );
    }
}
