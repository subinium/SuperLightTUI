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
    BeginContainer {
        direction: Direction,
        gap: u32,
        align: Align,
        align_self: Option<Align>,
        justify: Justify,
        border: Option<Border>,
        border_sides: BorderSides,
        border_style: Style,
        bg_color: Option<Color>,
        padding: Padding,
        margin: Margin,
        constraints: Constraints,
        title: Option<(String, Style)>,
        grow: u16,
        group_name: Option<String>,
    },
    BeginScrollable {
        grow: u16,
        border: Option<Border>,
        border_sides: BorderSides,
        border_style: Style,
        padding: Padding,
        margin: Margin,
        constraints: Constraints,
        title: Option<(String, Style)>,
        scroll_offset: u32,
    },
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
    RawDraw {
        draw_id: usize,
        constraints: Constraints,
        grow: u16,
        margin: Margin,
    },
}
