/// Terminal color.
///
/// Covers the standard 16 named colors, 256-color palette indices, and
/// 24-bit RGB true color. Use [`Color::Reset`] to restore the terminal's
/// default foreground or background.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Color {
    /// Reset to the terminal's default color.
    Reset,
    /// Standard black (color index 0).
    Black,
    /// Standard red (color index 1).
    Red,
    /// Standard green (color index 2).
    Green,
    /// Standard yellow (color index 3).
    Yellow,
    /// Standard blue (color index 4).
    Blue,
    /// Standard magenta (color index 5).
    Magenta,
    /// Standard cyan (color index 6).
    Cyan,
    /// Standard white (color index 7).
    White,
    /// 24-bit true color.
    Rgb(u8, u8, u8),
    /// 256-color palette index.
    Indexed(u8),
}

/// A color theme that flows through all widgets automatically.
///
/// Construct with [`Theme::dark()`] or [`Theme::light()`], or build a custom
/// theme by filling in the fields directly. Pass the theme via [`crate::RunConfig`]
/// and every widget will pick up the colors without any extra wiring.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Theme {
    /// Primary accent color, used for focused borders and highlights.
    pub primary: Color,
    /// Secondary accent color, used for less prominent highlights.
    pub secondary: Color,
    /// Accent color for decorative elements.
    pub accent: Color,
    /// Default foreground text color.
    pub text: Color,
    /// Dimmed text color for secondary labels and hints.
    pub text_dim: Color,
    /// Border color for unfocused containers.
    pub border: Color,
    /// Background color. Typically [`Color::Reset`] to inherit the terminal background.
    pub bg: Color,
    /// Color for success states (e.g., toast notifications).
    pub success: Color,
    /// Color for warning states.
    pub warning: Color,
    /// Color for error states.
    pub error: Color,
    /// Background color for selected list/table rows.
    pub selected_bg: Color,
    /// Foreground color for selected list/table rows.
    pub selected_fg: Color,
}

impl Theme {
    /// Create a dark theme with cyan primary and white text.
    pub fn dark() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Blue,
            accent: Color::Magenta,
            text: Color::White,
            text_dim: Color::Indexed(245),
            border: Color::Indexed(240),
            bg: Color::Reset,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            selected_bg: Color::Cyan,
            selected_fg: Color::Black,
        }
    }

    /// Create a light theme with blue primary and black text.
    pub fn light() -> Self {
        Self {
            primary: Color::Blue,
            secondary: Color::Cyan,
            accent: Color::Magenta,
            text: Color::Black,
            text_dim: Color::Indexed(240),
            border: Color::Indexed(245),
            bg: Color::Reset,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            selected_bg: Color::Blue,
            selected_fg: Color::White,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
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
    /// Bold text.
    pub const BOLD: Self = Self(1 << 0);
    /// Dimmed/faint text.
    pub const DIM: Self = Self(1 << 1);
    /// Italic text.
    pub const ITALIC: Self = Self(1 << 2);
    /// Underlined text.
    pub const UNDERLINE: Self = Self(1 << 3);
    /// Reversed foreground/background colors.
    pub const REVERSED: Self = Self(1 << 4);
    /// Strikethrough text.
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
