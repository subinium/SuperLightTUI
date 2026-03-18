//! Visual styling primitives.
//!
//! Colors, themes, borders, padding, margin, constraints, alignment, and
//! text modifiers. Every widget inherits these through [`Theme`] automatically.

mod color;
mod theme;
pub use color::{Color, ColorDepth};
pub use theme::{Theme, ThemeBuilder};

/// Terminal size breakpoint for responsive layouts.
///
/// Based on the current terminal width. Use [`crate::Context::breakpoint`] to
/// get the active breakpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Breakpoint {
    /// Width < 40 columns (phone-sized)
    Xs,
    /// Width 40-79 columns (small terminal)
    Sm,
    /// Width 80-119 columns (standard terminal)
    Md,
    /// Width 120-159 columns (wide terminal)
    Lg,
    /// Width >= 160 columns (ultra-wide)
    Xl,
}

/// Border style for containers.
///
/// Pass to `Context::bordered()` to draw a box around a container.
/// Each variant uses a different set of Unicode box-drawing characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Border {
    /// Single-line box: `┌─┐│└─┘`
    Single,
    /// Double-line box: `╔═╗║╚═╝`
    Double,
    /// Rounded corners: `╭─╮│╰─╯`
    Rounded,
    /// Thick single-line box: `┏━┓┃┗━┛`
    Thick,
    /// Dashed border using light dash characters: ┄╌┄╌
    Dashed,
    /// Heavy dashed border: ┅╍┅╍
    DashedThick,
}

/// Character set for a specific border style.
///
/// Returned by [`Border::chars`]. Contains the six box-drawing characters
/// needed to render a complete border: four corners and two line segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BorderChars {
    /// Top-left corner character.
    pub tl: char,
    /// Top-right corner character.
    pub tr: char,
    /// Bottom-left corner character.
    pub bl: char,
    /// Bottom-right corner character.
    pub br: char,
    /// Horizontal line character.
    pub h: char,
    /// Vertical line character.
    pub v: char,
}

/// Controls which sides of a border are visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BorderSides {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

impl BorderSides {
    /// All four sides visible (default).
    pub const fn all() -> Self {
        Self {
            top: true,
            right: true,
            bottom: true,
            left: true,
        }
    }

    /// No sides visible.
    pub const fn none() -> Self {
        Self {
            top: false,
            right: false,
            bottom: false,
            left: false,
        }
    }

    /// Top and bottom sides only.
    pub const fn horizontal() -> Self {
        Self {
            top: true,
            right: false,
            bottom: true,
            left: false,
        }
    }

    /// Left and right sides only.
    pub const fn vertical() -> Self {
        Self {
            top: false,
            right: true,
            bottom: false,
            left: true,
        }
    }

    /// Returns true if top or bottom is visible.
    pub fn has_horizontal(&self) -> bool {
        self.top || self.bottom
    }

    /// Returns true if left or right is visible.
    pub fn has_vertical(&self) -> bool {
        self.left || self.right
    }
}

impl Default for BorderSides {
    fn default() -> Self {
        Self::all()
    }
}

impl Border {
    /// Return the [`BorderChars`] for this border style.
    pub const fn chars(self) -> BorderChars {
        match self {
            Self::Single => BorderChars {
                tl: '┌',
                tr: '┐',
                bl: '└',
                br: '┘',
                h: '─',
                v: '│',
            },
            Self::Double => BorderChars {
                tl: '╔',
                tr: '╗',
                bl: '╚',
                br: '╝',
                h: '═',
                v: '║',
            },
            Self::Rounded => BorderChars {
                tl: '╭',
                tr: '╮',
                bl: '╰',
                br: '╯',
                h: '─',
                v: '│',
            },
            Self::Thick => BorderChars {
                tl: '┏',
                tr: '┓',
                bl: '┗',
                br: '┛',
                h: '━',
                v: '┃',
            },
            Self::Dashed => BorderChars {
                tl: '┌',
                tr: '┐',
                bl: '└',
                br: '┘',
                h: '┄',
                v: '┆',
            },
            Self::DashedThick => BorderChars {
                tl: '┏',
                tr: '┓',
                bl: '┗',
                br: '┛',
                h: '┅',
                v: '┇',
            },
        }
    }
}

/// Padding inside a container border.
///
/// Shrinks the content area inward from each edge. All values are in terminal
/// columns/rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Padding {
    /// Padding on the top edge.
    pub top: u32,
    /// Padding on the right edge.
    pub right: u32,
    /// Padding on the bottom edge.
    pub bottom: u32,
    /// Padding on the left edge.
    pub left: u32,
}

impl Padding {
    /// Create uniform padding on all four sides.
    pub const fn all(v: u32) -> Self {
        Self::new(v, v, v, v)
    }

    /// Create padding with `x` on left/right and `y` on top/bottom.
    pub const fn xy(x: u32, y: u32) -> Self {
        Self::new(y, x, y, x)
    }

    /// Create padding with explicit values for each side.
    pub const fn new(top: u32, right: u32, bottom: u32, left: u32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Total horizontal padding (`left + right`).
    pub const fn horizontal(self) -> u32 {
        self.left + self.right
    }

    /// Total vertical padding (`top + bottom`).
    pub const fn vertical(self) -> u32 {
        self.top + self.bottom
    }
}

/// Margin outside a container.
///
/// Adds space around the outside of a container's border. All values are in
/// terminal columns/rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Margin {
    /// Margin on the top edge.
    pub top: u32,
    /// Margin on the right edge.
    pub right: u32,
    /// Margin on the bottom edge.
    pub bottom: u32,
    /// Margin on the left edge.
    pub left: u32,
}

impl Margin {
    /// Create uniform margin on all four sides.
    pub const fn all(v: u32) -> Self {
        Self::new(v, v, v, v)
    }

    /// Create margin with `x` on left/right and `y` on top/bottom.
    pub const fn xy(x: u32, y: u32) -> Self {
        Self::new(y, x, y, x)
    }

    /// Create margin with explicit values for each side.
    pub const fn new(top: u32, right: u32, bottom: u32, left: u32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Total horizontal margin (`left + right`).
    pub const fn horizontal(self) -> u32 {
        self.left + self.right
    }

    /// Total vertical margin (`top + bottom`).
    pub const fn vertical(self) -> u32 {
        self.top + self.bottom
    }
}

/// Size constraints for layout computation.
///
/// All fields are optional. Unset constraints are unconstrained. Use the
/// builder methods to set individual bounds in a fluent style.
///
/// # Example
///
/// ```
/// use slt::Constraints;
///
/// let c = Constraints::default().min_w(10).max_w(40);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use = "configure constraints using the returned value"]
pub struct Constraints {
    /// Minimum width in terminal columns, if any.
    pub min_width: Option<u32>,
    /// Maximum width in terminal columns, if any.
    pub max_width: Option<u32>,
    /// Minimum height in terminal rows, if any.
    pub min_height: Option<u32>,
    /// Maximum height in terminal rows, if any.
    pub max_height: Option<u32>,
    /// Width as a percentage (1-100) of the parent container.
    pub width_pct: Option<u8>,
    /// Height as a percentage (1-100) of the parent container.
    pub height_pct: Option<u8>,
}

impl Constraints {
    /// Set the minimum width constraint.
    pub const fn min_w(mut self, min_width: u32) -> Self {
        self.min_width = Some(min_width);
        self
    }

    /// Set the maximum width constraint.
    pub const fn max_w(mut self, max_width: u32) -> Self {
        self.max_width = Some(max_width);
        self
    }

    /// Set the minimum height constraint.
    pub const fn min_h(mut self, min_height: u32) -> Self {
        self.min_height = Some(min_height);
        self
    }

    /// Set the maximum height constraint.
    pub const fn max_h(mut self, max_height: u32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    /// Set width as a percentage (1-100) of the parent container.
    pub const fn w_pct(mut self, pct: u8) -> Self {
        self.width_pct = Some(pct);
        self
    }

    /// Set height as a percentage (1-100) of the parent container.
    pub const fn h_pct(mut self, pct: u8) -> Self {
        self.height_pct = Some(pct);
        self
    }
}

/// Cross-axis alignment within a container.
///
/// Controls how children are positioned along the axis perpendicular to the
/// container's main axis. For a `row()`, this is vertical alignment; for a
/// `col()`, this is horizontal alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Align {
    /// Align children to the start of the cross axis (default).
    #[default]
    Start,
    /// Center children on the cross axis.
    Center,
    /// Align children to the end of the cross axis.
    End,
}

/// Main-axis content distribution within a container.
///
/// Controls how children are distributed along the main axis. For a `row()`,
/// this is horizontal distribution; for a `col()`, this is vertical.
///
/// When children have `grow > 0`, they consume remaining space before justify
/// distribution applies. Justify modes only affect the leftover space after
/// flex-grow allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Justify {
    /// Pack children at the start (default). Uses `gap` for spacing.
    #[default]
    Start,
    /// Center children along the main axis with `gap` spacing.
    Center,
    /// Pack children at the end with `gap` spacing.
    End,
    /// First child at start, last at end, equal space between.
    SpaceBetween,
    /// Equal space around each child (half-size space at edges).
    SpaceAround,
    /// Equal space between all children and at both edges.
    SpaceEvenly,
}

/// Text modifier bitflags stored as a `u8`.
///
/// Combine modifiers with `|` or [`Modifiers::insert`]. Check membership with
/// [`Modifiers::contains`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Modifiers(pub u8);

impl Modifiers {
    /// No modifiers set.
    pub const NONE: Self = Self(0);
    /// Enable bold text.
    pub const BOLD: Self = Self(1 << 0);
    /// Enable dimmed/faint text.
    pub const DIM: Self = Self(1 << 1);
    /// Enable italic text.
    pub const ITALIC: Self = Self(1 << 2);
    /// Enable underlined text.
    pub const UNDERLINE: Self = Self(1 << 3);
    /// Enable reversed foreground/background colors.
    pub const REVERSED: Self = Self(1 << 4);
    /// Enable strikethrough text.
    pub const STRIKETHROUGH: Self = Self(1 << 5);

    /// Returns `true` if all bits in `other` are set in `self`.
    #[inline]
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Set all bits from `other` into `self`.
    #[inline]
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Returns `true` if no modifiers are set.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for Modifiers {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for Modifiers {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Visual style for a terminal cell (foreground, background, modifiers).
///
/// Styles are applied to text via the builder methods on `Context` widget
/// calls (e.g., `.bold()`, `.fg(Color::Cyan)`). All fields are optional;
/// `None` means "inherit from the terminal default."
///
/// # Example
///
/// ```
/// use slt::{Style, Color};
///
/// let style = Style::new().fg(Color::Cyan).bold();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use = "build and pass the returned Style value"]
pub struct Style {
    /// Foreground color, or `None` to use the terminal default.
    pub fg: Option<Color>,
    /// Background color, or `None` to use the terminal default.
    pub bg: Option<Color>,
    /// Text modifiers (bold, italic, underline, etc.).
    pub modifiers: Modifiers,
}

impl Style {
    /// Create a new style with no color or modifiers set.
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            modifiers: Modifiers::NONE,
        }
    }

    /// Set the foreground color.
    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    /// Set the background color.
    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Add the bold modifier.
    pub fn bold(mut self) -> Self {
        self.modifiers |= Modifiers::BOLD;
        self
    }

    /// Add the dim modifier.
    pub fn dim(mut self) -> Self {
        self.modifiers |= Modifiers::DIM;
        self
    }

    /// Add the italic modifier.
    pub fn italic(mut self) -> Self {
        self.modifiers |= Modifiers::ITALIC;
        self
    }

    /// Add the underline modifier.
    pub fn underline(mut self) -> Self {
        self.modifiers |= Modifiers::UNDERLINE;
        self
    }

    /// Add the reversed (inverted colors) modifier.
    pub fn reversed(mut self) -> Self {
        self.modifiers |= Modifiers::REVERSED;
        self
    }

    /// Add the strikethrough modifier.
    pub fn strikethrough(mut self) -> Self {
        self.modifiers |= Modifiers::STRIKETHROUGH;
        self
    }
}

/// Reusable container style recipe.
///
/// Define once, apply anywhere with [`crate::ContainerBuilder::apply`]. All fields
/// are optional — only set fields override the builder's current values.
/// Styles compose: apply multiple recipes in sequence, last write wins.
///
/// # Example
///
/// ```ignore
/// use slt::{ContainerStyle, Border, Color};
///
/// const CARD: ContainerStyle = ContainerStyle::new()
///     .border(Border::Rounded)
///     .p(1)
///     .bg(Color::Indexed(236));
///
/// const DANGER: ContainerStyle = ContainerStyle::new()
///     .bg(Color::Red);
///
/// // Apply one or compose multiple:
/// ui.container().apply(&CARD).col(|ui| { ... });
/// ui.container().apply(&CARD).apply(&DANGER).col(|ui| { ... });
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct ContainerStyle {
    /// Border style for the container.
    pub border: Option<Border>,
    /// Which sides of the border are visible.
    pub border_sides: Option<BorderSides>,
    /// Style (color and modifiers) for the border.
    pub border_style: Option<Style>,
    /// Background color.
    pub bg: Option<Color>,
    /// Foreground (text) color.
    pub text_color: Option<Color>,
    /// Background color in dark mode.
    pub dark_bg: Option<Color>,
    /// Border style in dark mode.
    pub dark_border_style: Option<Style>,
    /// Padding inside the container.
    pub padding: Option<Padding>,
    /// Margin outside the container.
    pub margin: Option<Margin>,
    /// Gap between children (both row and column).
    pub gap: Option<u32>,
    /// Gap between rows.
    pub row_gap: Option<u32>,
    /// Gap between columns.
    pub col_gap: Option<u32>,
    /// Flex grow factor.
    pub grow: Option<u16>,
    /// Cross-axis alignment.
    pub align: Option<Align>,
    /// Self alignment (overrides parent align).
    pub align_self: Option<Align>,
    /// Main-axis content distribution.
    pub justify: Option<Justify>,
    /// Fixed width.
    pub w: Option<u32>,
    /// Fixed height.
    pub h: Option<u32>,
    /// Minimum width.
    pub min_w: Option<u32>,
    /// Maximum width.
    pub max_w: Option<u32>,
    /// Minimum height.
    pub min_h: Option<u32>,
    /// Maximum height.
    pub max_h: Option<u32>,
    /// Width as percentage of parent.
    pub w_pct: Option<u8>,
    /// Height as percentage of parent.
    pub h_pct: Option<u8>,
}

impl ContainerStyle {
    /// Create an empty container style with no overrides.
    pub const fn new() -> Self {
        Self {
            border: None,
            border_sides: None,
            border_style: None,
            bg: None,
            text_color: None,
            dark_bg: None,
            dark_border_style: None,
            padding: None,
            margin: None,
            gap: None,
            row_gap: None,
            col_gap: None,
            grow: None,
            align: None,
            align_self: None,
            justify: None,
            w: None,
            h: None,
            min_w: None,
            max_w: None,
            min_h: None,
            max_h: None,
            w_pct: None,
            h_pct: None,
        }
    }

    /// Set the border style.
    pub const fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }

    /// Set which border sides to render.
    pub const fn border_sides(mut self, sides: BorderSides) -> Self {
        self.border_sides = Some(sides);
        self
    }

    /// Set the background color.
    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Set default text color inherited by child text widgets.
    pub const fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Set the dark-mode background color.
    pub const fn dark_bg(mut self, color: Color) -> Self {
        self.dark_bg = Some(color);
        self
    }

    /// Set uniform padding on all sides.
    pub const fn p(mut self, value: u32) -> Self {
        self.padding = Some(Padding {
            top: value,
            bottom: value,
            left: value,
            right: value,
        });
        self
    }

    /// Set horizontal padding.
    pub const fn px(mut self, value: u32) -> Self {
        let p = match self.padding {
            Some(p) => Padding {
                left: value,
                right: value,
                ..p
            },
            None => Padding {
                top: 0,
                bottom: 0,
                left: value,
                right: value,
            },
        };
        self.padding = Some(p);
        self
    }

    /// Set vertical padding.
    pub const fn py(mut self, value: u32) -> Self {
        let p = match self.padding {
            Some(p) => Padding {
                top: value,
                bottom: value,
                ..p
            },
            None => Padding {
                top: value,
                bottom: value,
                left: 0,
                right: 0,
            },
        };
        self.padding = Some(p);
        self
    }

    /// Set uniform margin on all sides.
    pub const fn m(mut self, value: u32) -> Self {
        self.margin = Some(Margin {
            top: value,
            bottom: value,
            left: value,
            right: value,
        });
        self
    }

    /// Set the gap between children.
    pub const fn gap(mut self, value: u32) -> Self {
        self.gap = Some(value);
        self
    }

    /// Set row gap for column layouts.
    pub const fn row_gap(mut self, value: u32) -> Self {
        self.row_gap = Some(value);
        self
    }

    /// Set column gap for row layouts.
    pub const fn col_gap(mut self, value: u32) -> Self {
        self.col_gap = Some(value);
        self
    }

    /// Set the flex-grow factor.
    pub const fn grow(mut self, value: u16) -> Self {
        self.grow = Some(value);
        self
    }

    /// Set fixed width.
    pub const fn w(mut self, value: u32) -> Self {
        self.w = Some(value);
        self
    }

    /// Set fixed height.
    pub const fn h(mut self, value: u32) -> Self {
        self.h = Some(value);
        self
    }

    /// Set minimum width.
    pub const fn min_w(mut self, value: u32) -> Self {
        self.min_w = Some(value);
        self
    }

    /// Set maximum width.
    pub const fn max_w(mut self, value: u32) -> Self {
        self.max_w = Some(value);
        self
    }

    /// Set cross-axis alignment.
    pub const fn align(mut self, value: Align) -> Self {
        self.align = Some(value);
        self
    }

    /// Set per-child cross-axis alignment override.
    pub const fn align_self(mut self, value: Align) -> Self {
        self.align_self = Some(value);
        self
    }

    /// Set main-axis justification.
    pub const fn justify(mut self, value: Justify) -> Self {
        self.justify = Some(value);
        self
    }

    /// Set minimum height.
    pub const fn min_h(mut self, value: u32) -> Self {
        self.min_h = Some(value);
        self
    }

    /// Set maximum height.
    pub const fn max_h(mut self, value: u32) -> Self {
        self.max_h = Some(value);
        self
    }

    /// Set width as percentage of parent (1-100).
    pub const fn w_pct(mut self, value: u8) -> Self {
        self.w_pct = Some(value);
        self
    }

    /// Set height as percentage of parent (1-100).
    pub const fn h_pct(mut self, value: u8) -> Self {
        self.h_pct = Some(value);
        self
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Per-widget color overrides that fall back to the active theme.
pub struct WidgetColors {
    /// Foreground color override.
    pub fg: Option<Color>,
    /// Background color override.
    pub bg: Option<Color>,
    /// Border color override.
    pub border: Option<Color>,
    /// Accent color override.
    pub accent: Option<Color>,
}

impl WidgetColors {
    /// Create a new WidgetColors with all fields set to None (theme defaults).
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            border: None,
            accent: None,
        }
    }

    /// Set the foreground color override.
    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    /// Set the background color override.
    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Set the border color override.
    pub const fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    /// Set the accent color override.
    pub const fn accent(mut self, color: Color) -> Self {
        self.accent = Some(color);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_new_is_default() {
        let style = Style::new();
        assert_eq!(style.fg, None);
        assert_eq!(style.bg, None);
        assert_eq!(style.modifiers, Modifiers::NONE);
        assert_eq!(style, Style::default());
    }

    #[test]
    fn style_bold_and_fg_set_expected_fields() {
        let style = Style::new().bold().fg(Color::Red);
        assert_eq!(style.fg, Some(Color::Red));
        assert_eq!(style.bg, None);
        assert!(style.modifiers.contains(Modifiers::BOLD));
    }

    #[test]
    fn style_multiple_modifiers_accumulate() {
        let style = Style::new().italic().underline().dim();
        assert!(style.modifiers.contains(Modifiers::ITALIC));
        assert!(style.modifiers.contains(Modifiers::UNDERLINE));
        assert!(style.modifiers.contains(Modifiers::DIM));
    }

    #[test]
    fn style_repeated_fg_overrides_previous_color() {
        let style = Style::new().fg(Color::Blue).fg(Color::Green);
        assert_eq!(style.fg, Some(Color::Green));
    }

    #[test]
    fn style_repeated_bg_overrides_previous_color() {
        let style = Style::new().bg(Color::Blue).bg(Color::Green);
        assert_eq!(style.bg, Some(Color::Green));
    }

    #[test]
    fn style_override_preserves_existing_modifiers() {
        let style = Style::new().bold().fg(Color::Red).fg(Color::Yellow);
        assert_eq!(style.fg, Some(Color::Yellow));
        assert!(style.modifiers.contains(Modifiers::BOLD));
    }

    #[test]
    fn padding_all_sets_all_sides() {
        let p = Padding::all(3);
        assert_eq!(p.top, 3);
        assert_eq!(p.right, 3);
        assert_eq!(p.bottom, 3);
        assert_eq!(p.left, 3);
    }

    #[test]
    fn padding_xy_sets_axis_values() {
        let p = Padding::xy(4, 2);
        assert_eq!(p.top, 2);
        assert_eq!(p.bottom, 2);
        assert_eq!(p.left, 4);
        assert_eq!(p.right, 4);
    }

    #[test]
    fn padding_new_and_totals_are_correct() {
        let p = Padding::new(1, 2, 3, 4);
        assert_eq!(p.top, 1);
        assert_eq!(p.right, 2);
        assert_eq!(p.bottom, 3);
        assert_eq!(p.left, 4);
        assert_eq!(p.horizontal(), 6);
        assert_eq!(p.vertical(), 4);
    }

    #[test]
    fn margin_all_and_xy_are_correct() {
        let all = Margin::all(5);
        assert_eq!(all, Margin::new(5, 5, 5, 5));

        let xy = Margin::xy(7, 1);
        assert_eq!(xy.top, 1);
        assert_eq!(xy.bottom, 1);
        assert_eq!(xy.left, 7);
        assert_eq!(xy.right, 7);
    }

    #[test]
    fn margin_new_and_totals_are_correct() {
        let m = Margin::new(2, 4, 6, 8);
        assert_eq!(m.horizontal(), 12);
        assert_eq!(m.vertical(), 8);
    }

    #[test]
    fn constraints_min_max_builder_sets_values() {
        let c = Constraints::default()
            .min_w(10)
            .max_w(40)
            .min_h(5)
            .max_h(20);
        assert_eq!(c.min_width, Some(10));
        assert_eq!(c.max_width, Some(40));
        assert_eq!(c.min_height, Some(5));
        assert_eq!(c.max_height, Some(20));
    }

    #[test]
    fn constraints_percentage_builder_sets_values() {
        let c = Constraints::default().w_pct(50).h_pct(80);
        assert_eq!(c.width_pct, Some(50));
        assert_eq!(c.height_pct, Some(80));
    }

    #[test]
    fn border_sides_all_has_both_axes() {
        let sides = BorderSides::all();
        assert!(sides.top && sides.right && sides.bottom && sides.left);
        assert!(sides.has_horizontal());
        assert!(sides.has_vertical());
    }

    #[test]
    fn border_sides_none_has_no_axes() {
        let sides = BorderSides::none();
        assert!(!sides.top && !sides.right && !sides.bottom && !sides.left);
        assert!(!sides.has_horizontal());
        assert!(!sides.has_vertical());
    }

    #[test]
    fn border_sides_horizontal_only() {
        let sides = BorderSides::horizontal();
        assert!(sides.top);
        assert!(sides.bottom);
        assert!(!sides.left);
        assert!(!sides.right);
        assert!(sides.has_horizontal());
        assert!(!sides.has_vertical());
    }

    #[test]
    fn border_sides_vertical_only() {
        let sides = BorderSides::vertical();
        assert!(!sides.top);
        assert!(!sides.bottom);
        assert!(sides.left);
        assert!(sides.right);
        assert!(!sides.has_horizontal());
        assert!(sides.has_vertical());
    }

    #[test]
    fn container_style_new_is_empty() {
        let s = ContainerStyle::new();
        assert_eq!(s.border, None);
        assert_eq!(s.bg, None);
        assert_eq!(s.padding, None);
        assert_eq!(s.margin, None);
        assert_eq!(s.gap, None);
        assert_eq!(s.align, None);
        assert_eq!(s.justify, None);
    }

    #[test]
    fn container_style_const_construction_and_fields() {
        const CARD: ContainerStyle = ContainerStyle::new()
            .border(Border::Rounded)
            .border_sides(BorderSides::horizontal())
            .p(2)
            .m(1)
            .gap(3)
            .align(Align::Center)
            .justify(Justify::SpaceBetween)
            .w(60)
            .h(20);

        assert_eq!(CARD.border, Some(Border::Rounded));
        assert_eq!(CARD.border_sides, Some(BorderSides::horizontal()));
        assert_eq!(CARD.padding, Some(Padding::all(2)));
        assert_eq!(CARD.margin, Some(Margin::all(1)));
        assert_eq!(CARD.gap, Some(3));
        assert_eq!(CARD.align, Some(Align::Center));
        assert_eq!(CARD.justify, Some(Justify::SpaceBetween));
        assert_eq!(CARD.w, Some(60));
        assert_eq!(CARD.h, Some(20));
    }

    #[test]
    fn widget_colors_new_is_empty() {
        let colors = WidgetColors::new();
        assert_eq!(colors.fg, None);
        assert_eq!(colors.bg, None);
        assert_eq!(colors.border, None);
        assert_eq!(colors.accent, None);

        let defaults = WidgetColors::default();
        assert_eq!(defaults.fg, None);
        assert_eq!(defaults.bg, None);
        assert_eq!(defaults.border, None);
        assert_eq!(defaults.accent, None);
    }

    #[test]
    fn widget_colors_builder_sets_all_fields() {
        let colors = WidgetColors::new()
            .fg(Color::White)
            .bg(Color::Black)
            .border(Color::Cyan)
            .accent(Color::Yellow);

        assert_eq!(colors.fg, Some(Color::White));
        assert_eq!(colors.bg, Some(Color::Black));
        assert_eq!(colors.border, Some(Color::Cyan));
        assert_eq!(colors.accent, Some(Color::Yellow));
    }

    #[test]
    fn align_default_is_start() {
        assert_eq!(Align::default(), Align::Start);
    }

    #[test]
    fn justify_default_is_start() {
        assert_eq!(Justify::default(), Justify::Start);
    }

    #[test]
    fn align_and_justify_variants_are_distinct() {
        assert_ne!(Align::Start, Align::Center);
        assert_ne!(Align::Center, Align::End);

        assert_ne!(Justify::Start, Justify::Center);
        assert_ne!(Justify::Center, Justify::End);
        assert_ne!(Justify::SpaceBetween, Justify::SpaceAround);
        assert_ne!(Justify::SpaceAround, Justify::SpaceEvenly);
    }
}
