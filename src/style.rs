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
/// Based on the current terminal width. Use [`Context::breakpoint`] to
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
    pub const fn all() -> Self {
        Self {
            top: true,
            right: true,
            bottom: true,
            left: true,
        }
    }

    pub const fn none() -> Self {
        Self {
            top: false,
            right: false,
            bottom: false,
            left: false,
        }
    }

    pub const fn horizontal() -> Self {
        Self {
            top: true,
            right: false,
            bottom: true,
            left: false,
        }
    }

    pub const fn vertical() -> Self {
        Self {
            top: false,
            right: true,
            bottom: false,
            left: true,
        }
    }

    pub fn has_horizontal(&self) -> bool {
        self.top || self.bottom
    }

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

/// Reusable container style recipe.
///
/// Define once, apply anywhere with [`ContainerBuilder::apply`]. All fields
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
    pub border: Option<Border>,
    pub border_sides: Option<BorderSides>,
    pub border_style: Option<Style>,
    pub bg: Option<Color>,
    pub text_color: Option<Color>,
    pub dark_bg: Option<Color>,
    pub dark_border_style: Option<Style>,
    pub padding: Option<Padding>,
    pub margin: Option<Margin>,
    pub gap: Option<u32>,
    pub row_gap: Option<u32>,
    pub col_gap: Option<u32>,
    pub grow: Option<u16>,
    pub align: Option<Align>,
    pub align_self: Option<Align>,
    pub justify: Option<Justify>,
    pub w: Option<u32>,
    pub h: Option<u32>,
    pub min_w: Option<u32>,
    pub max_w: Option<u32>,
    pub min_h: Option<u32>,
    pub max_h: Option<u32>,
    pub w_pct: Option<u8>,
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
pub struct WidgetColors {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub border: Option<Color>,
    pub accent: Option<Color>,
}

impl WidgetColors {
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            border: None,
            accent: None,
        }
    }

    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub const fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub const fn accent(mut self, color: Color) -> Self {
        self.accent = Some(color);
        self
    }
}
