//! Visual styling primitives.
//!
//! Colors, themes, borders, padding, margin, constraints, alignment, and
//! text modifiers. Every widget inherits these through [`Theme`] automatically.

mod color;
mod theme;
#[cfg(feature = "serde")]
mod theme_io;
pub use color::{Color, ColorDepth};
pub use theme::{Spacing, Theme, ThemeBuilder, ThemeColor};
#[cfg(feature = "theme-watch")]
pub use theme_io::ThemeWatcher;
#[cfg(feature = "serde")]
pub use theme_io::{ThemeFile, ThemeLoadError};

/// Terminal size breakpoint for responsive layouts.
///
/// Based on the current terminal width. Use [`crate::Context::breakpoint`] to
/// get the active breakpoint.
#[non_exhaustive]
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
#[non_exhaustive]
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
    /// Top border visible.
    pub top: bool,
    /// Right border visible.
    pub right: bool,
    /// Bottom border visible.
    pub bottom: bool,
    /// Left border visible.
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

/// Width specification for a flexbox item.
///
/// Replaces the previous trio of `Option`-typed fields (`min_width`,
/// `max_width`, `width_pct`) with a single tagged enum. Resolution at
/// layout time dispatches on the variant.
///
/// `Constraints::default()` produces [`WidthSpec::Auto`].
///
/// # Variant semantics
///
/// - [`Auto`](Self::Auto) — no width constraint; the element sizes from
///   content and available space.
/// - [`Fixed(n)`](Self::Fixed) — exact cell width. Equivalent to
///   `MinMax { min: Some(n), max: Some(n) }`.
/// - [`Pct(p)`](Self::Pct) — percentage of parent width (clamped to 0..=100).
/// - [`Ratio(num, den)`](Self::Ratio) — exact integer fraction. For example
///   `Ratio(1, 3)` produces `area / 3`. Floor division: `area = 80, num = 1,
///   den = 3` → `26`. A `den` of `0` is treated as no constraint.
/// - [`MinMax { min, max }`](Self::MinMax) — bounds on each side independently.
///
/// # Example
///
/// ```
/// use slt::{Constraints, WidthSpec};
///
/// let c = Constraints::default().w_ratio(1, 3);
/// assert_eq!(c.width, WidthSpec::Ratio(1, 3));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum WidthSpec {
    /// Unconstrained — sizes from content and available space.
    Auto,
    /// Exact cell width.
    Fixed(u32),
    /// Percentage of parent width (`0..=100`).
    Pct(u8),
    /// Exact integer fraction of parent (numerator, denominator).
    ///
    /// `Ratio(1, 3)` produces `area / 3`. Floor division — for
    /// `area = 80, num = 1, den = 3` → `26`. A `den` of `0` is treated as
    /// no constraint.
    Ratio(u16, u16),
    /// Min and/or max bounds. Sentinels are used so that the variant fits
    /// in 12 bytes (24 bytes total for the two-axis [`Constraints`] struct):
    ///
    /// - `min = 0` means "no minimum" (equivalent to `Option::None`); since
    ///   a min of 0 is the same as no minimum, using `0` as the sentinel
    ///   does not lose any expressible state.
    /// - `max = u32::MAX` means "no maximum" (the natural `infinity`).
    ///
    /// Use the [`Constraints::min_w`] / [`Constraints::max_w`] /
    /// [`Constraints::w_minmax`] builders to construct this variant
    /// without thinking about sentinels.
    MinMax {
        /// Minimum width. `0` means unbounded below.
        min: u32,
        /// Maximum width. `u32::MAX` means unbounded above.
        max: u32,
    },
}

impl Default for WidthSpec {
    #[inline]
    fn default() -> Self {
        Self::Auto
    }
}

/// Height specification for a flexbox item.
///
/// Mirror of [`WidthSpec`] for the cross axis. See [`WidthSpec`] for full
/// variant semantics, including the sentinel encoding of [`Self::MinMax`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum HeightSpec {
    /// Unconstrained — sizes from content and available space.
    Auto,
    /// Exact cell height.
    Fixed(u32),
    /// Percentage of parent height (`0..=100`).
    Pct(u8),
    /// Exact integer fraction of parent (numerator, denominator).
    ///
    /// `Ratio(1, 3)` produces `area / 3`. Floor division — for
    /// `area = 80, num = 1, den = 3` → `26`. A `den` of `0` is treated as
    /// no constraint.
    Ratio(u16, u16),
    /// Min and/or max bounds. Sentinels: `min = 0` and `max = u32::MAX`
    /// represent "no bound". See [`WidthSpec::MinMax`] for full rationale.
    MinMax {
        /// Minimum height. `0` means unbounded below.
        min: u32,
        /// Maximum height. `u32::MAX` means unbounded above.
        max: u32,
    },
}

impl Default for HeightSpec {
    #[inline]
    fn default() -> Self {
        Self::Auto
    }
}

/// Size constraints for layout computation.
///
/// Holds a [`WidthSpec`] and a [`HeightSpec`] for the two axes. Use the
/// builder methods on `Constraints` to set individual bounds in a fluent
/// style; the builders pick the appropriate variant for you.
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
    /// Width specification.
    pub width: WidthSpec,
    /// Height specification.
    pub height: HeightSpec,
}

/// Compile-time regression guard for `Constraints` size.
///
/// The unified `WidthSpec`/`HeightSpec` representation is required to fit
/// in 24 bytes (12 + 12) — half the size of the v0.19 representation
/// (36 bytes). Layout state stores this struct on every `LayoutNode`, so
/// the cache footprint compounds.
const _ASSERT_CONSTRAINTS_SIZE: () = assert!(
    std::mem::size_of::<Constraints>() == 24,
    "Constraints must be 24 bytes"
);

impl Constraints {
    // ─── builder methods (preserved from v0.19, dispatch into enum) ───

    /// Set the minimum width constraint.
    ///
    /// If the current variant is [`WidthSpec::MinMax`], updates only the
    /// `min` side. Otherwise replaces the variant with `MinMax { min:
    /// min_width, max: u32::MAX }`.
    pub const fn min_w(mut self, min_width: u32) -> Self {
        let max = match self.width {
            WidthSpec::MinMax { max, .. } => max,
            WidthSpec::Fixed(v) => v,
            _ => u32::MAX,
        };
        self.width = WidthSpec::MinMax {
            min: min_width,
            max,
        };
        self
    }

    /// Set the maximum width constraint.
    ///
    /// If the current variant is [`WidthSpec::MinMax`], updates only the
    /// `max` side. Otherwise replaces the variant with `MinMax { min: 0,
    /// max: max_width }`.
    pub const fn max_w(mut self, max_width: u32) -> Self {
        let min = match self.width {
            WidthSpec::MinMax { min, .. } => min,
            WidthSpec::Fixed(v) => v,
            _ => 0,
        };
        self.width = WidthSpec::MinMax {
            min,
            max: max_width,
        };
        self
    }

    /// Set the minimum height constraint.
    ///
    /// If the current variant is [`HeightSpec::MinMax`], updates only the
    /// `min` side. Otherwise replaces the variant with `MinMax { min:
    /// min_height, max: u32::MAX }`.
    pub const fn min_h(mut self, min_height: u32) -> Self {
        let max = match self.height {
            HeightSpec::MinMax { max, .. } => max,
            HeightSpec::Fixed(v) => v,
            _ => u32::MAX,
        };
        self.height = HeightSpec::MinMax {
            min: min_height,
            max,
        };
        self
    }

    /// Set the maximum height constraint.
    ///
    /// If the current variant is [`HeightSpec::MinMax`], updates only the
    /// `max` side. Otherwise replaces the variant with `MinMax { min: 0,
    /// max: max_height }`.
    pub const fn max_h(mut self, max_height: u32) -> Self {
        let min = match self.height {
            HeightSpec::MinMax { min, .. } => min,
            HeightSpec::Fixed(v) => v,
            _ => 0,
        };
        self.height = HeightSpec::MinMax {
            min,
            max: max_height,
        };
        self
    }

    /// Set min and max width together.
    ///
    /// Equivalent to chaining `min_w(min)` and `max_w(max)` but in a single
    /// call, replacing the variant with `WidthSpec::MinMax`.
    pub const fn w_minmax(mut self, min: u32, max: u32) -> Self {
        self.width = WidthSpec::MinMax { min, max };
        self
    }

    /// Set min and max height together.
    pub const fn h_minmax(mut self, min: u32, max: u32) -> Self {
        self.height = HeightSpec::MinMax { min, max };
        self
    }

    /// Set a fixed width (replaces any existing width spec).
    pub const fn w(mut self, width: u32) -> Self {
        self.width = WidthSpec::Fixed(width);
        self
    }

    /// Set a fixed height (replaces any existing height spec).
    pub const fn h(mut self, height: u32) -> Self {
        self.height = HeightSpec::Fixed(height);
        self
    }

    /// Set width as a percentage (`0..=100`) of the parent container.
    pub const fn w_pct(mut self, pct: u8) -> Self {
        self.width = WidthSpec::Pct(pct);
        self
    }

    /// Set height as a percentage (`0..=100`) of the parent container.
    pub const fn h_pct(mut self, pct: u8) -> Self {
        self.height = HeightSpec::Pct(pct);
        self
    }

    /// Set width as an exact integer fraction of the parent (numerator, denominator).
    ///
    /// `w_ratio(1, 3)` produces `area / 3` — floor division. For `area = 80,
    /// num = 1, den = 3` → `26`.
    pub const fn w_ratio(mut self, num: u16, den: u16) -> Self {
        self.width = WidthSpec::Ratio(num, den);
        self
    }

    /// Set height as an exact integer fraction of the parent (numerator, denominator).
    ///
    /// `h_ratio(1, 3)` produces `area / 3` — floor division.
    pub const fn h_ratio(mut self, num: u16, den: u16) -> Self {
        self.height = HeightSpec::Ratio(num, den);
        self
    }

    // ─── derived accessors used by layout & widget code ────────────────

    /// Minimum width derived from the current [`WidthSpec`].
    ///
    /// Returns `Some(n)` for [`WidthSpec::Fixed`] (both min and max are `n`)
    /// and for [`WidthSpec::MinMax`] when the `min` side is non-zero.
    /// Returns `None` for [`WidthSpec::Auto`], [`WidthSpec::Pct`],
    /// [`WidthSpec::Ratio`], and for `MinMax { min: 0, .. }` (sentinel for
    /// "no minimum").
    pub const fn min_width(&self) -> Option<u32> {
        match self.width {
            WidthSpec::Fixed(v) => Some(v),
            WidthSpec::MinMax { min, .. } if min > 0 => Some(min),
            _ => None,
        }
    }

    /// Maximum width derived from the current [`WidthSpec`].
    ///
    /// Returns `Some(n)` for [`WidthSpec::Fixed`] and for
    /// [`WidthSpec::MinMax`] when the `max` side is not the sentinel
    /// `u32::MAX`. Returns `None` otherwise.
    pub const fn max_width(&self) -> Option<u32> {
        match self.width {
            WidthSpec::Fixed(v) => Some(v),
            WidthSpec::MinMax { max, .. } if max < u32::MAX => Some(max),
            _ => None,
        }
    }

    /// Minimum height derived from the current [`HeightSpec`].
    ///
    /// Mirror of [`min_width`](Self::min_width) for the cross axis.
    pub const fn min_height(&self) -> Option<u32> {
        match self.height {
            HeightSpec::Fixed(v) => Some(v),
            HeightSpec::MinMax { min, .. } if min > 0 => Some(min),
            _ => None,
        }
    }

    /// Maximum height derived from the current [`HeightSpec`].
    ///
    /// Mirror of [`max_width`](Self::max_width) for the cross axis.
    pub const fn max_height(&self) -> Option<u32> {
        match self.height {
            HeightSpec::Fixed(v) => Some(v),
            HeightSpec::MinMax { max, .. } if max < u32::MAX => Some(max),
            _ => None,
        }
    }

    /// Width percentage if the variant is [`WidthSpec::Pct`].
    pub const fn width_pct(&self) -> Option<u8> {
        match self.width {
            WidthSpec::Pct(p) => Some(p),
            _ => None,
        }
    }

    /// Height percentage if the variant is [`HeightSpec::Pct`].
    pub const fn height_pct(&self) -> Option<u8> {
        match self.height {
            HeightSpec::Pct(p) => Some(p),
            _ => None,
        }
    }

    // ─── imperative setters ─────────────────────────────────────────────
    //
    // These mutate `&mut Constraints` in-place. They exist alongside the
    // owning builder methods (`min_w`, `max_w`, …) for call sites that hold
    // a mutable borrow to a `Constraints` field embedded in a larger struct
    // — for those the builder's `mut self -> Self` shape would force a
    // `*c = c.min_w(v)` deref-assign. The setters keep that ergonomic.
    //
    // # Compatibility
    //
    // Public for downstream callers that adopted these from v0.19. New code
    // that owns a `Constraints` value should prefer the chainable builders
    // (`Constraints::default().min_w(10).max_w(40)`).

    /// Set the minimum width as `Option<u32>`.
    ///
    /// Promotes the variant to [`WidthSpec::MinMax`] preserving any existing
    /// `max` side. Passing `None` clears the minimum (sets it to `0`); if the
    /// resulting `MinMax` has no effective bounds (`min == 0` and
    /// `max == u32::MAX`) the variant collapses back to [`WidthSpec::Auto`].
    ///
    /// Prefer [`Constraints::min_w`] when you own the value; this setter is
    /// for in-place mutation through `&mut Constraints`.
    pub fn set_min_width(&mut self, value: Option<u32>) {
        let max = match self.width {
            WidthSpec::MinMax { max, .. } => max,
            WidthSpec::Fixed(v) => v,
            _ => u32::MAX,
        };
        let min = value.unwrap_or(0);
        self.width = if min == 0 && max == u32::MAX {
            WidthSpec::Auto
        } else {
            WidthSpec::MinMax { min, max }
        };
    }

    /// Set the maximum width as `Option<u32>`.
    pub fn set_max_width(&mut self, value: Option<u32>) {
        let min = match self.width {
            WidthSpec::MinMax { min, .. } => min,
            WidthSpec::Fixed(v) => v,
            _ => 0,
        };
        let max = value.unwrap_or(u32::MAX);
        self.width = if min == 0 && max == u32::MAX {
            WidthSpec::Auto
        } else {
            WidthSpec::MinMax { min, max }
        };
    }

    /// Set the minimum height as `Option<u32>`.
    pub fn set_min_height(&mut self, value: Option<u32>) {
        let max = match self.height {
            HeightSpec::MinMax { max, .. } => max,
            HeightSpec::Fixed(v) => v,
            _ => u32::MAX,
        };
        let min = value.unwrap_or(0);
        self.height = if min == 0 && max == u32::MAX {
            HeightSpec::Auto
        } else {
            HeightSpec::MinMax { min, max }
        };
    }

    /// Set the maximum height as `Option<u32>`.
    pub fn set_max_height(&mut self, value: Option<u32>) {
        let min = match self.height {
            HeightSpec::MinMax { min, .. } => min,
            HeightSpec::Fixed(v) => v,
            _ => 0,
        };
        let max = value.unwrap_or(u32::MAX);
        self.height = if min == 0 && max == u32::MAX {
            HeightSpec::Auto
        } else {
            HeightSpec::MinMax { min, max }
        };
    }

    /// Set the width percentage as `Option<u8>`.
    pub fn set_width_pct(&mut self, value: Option<u8>) {
        self.width = match value {
            Some(p) => WidthSpec::Pct(p),
            None => WidthSpec::Auto,
        };
    }

    /// Set the height percentage as `Option<u8>`.
    pub fn set_height_pct(&mut self, value: Option<u8>) {
        self.height = match value {
            Some(p) => HeightSpec::Pct(p),
            None => HeightSpec::Auto,
        };
    }
}

/// Cross-axis alignment within a container.
///
/// Controls how children are positioned along the axis perpendicular to the
/// container's main axis. For a `row()`, this is vertical alignment; for a
/// `col()`, this is horizontal alignment.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Align {
    /// Align children to the start of the cross axis (default).
    ///
    /// Unlike CSS `flex-start`, this variant fills the full cross-axis
    /// (equivalent to CSS `stretch`). Children are sized to the container's
    /// cross-axis dimension. Use [`Align::Center`] or [`Align::End`] to
    /// size children by their natural dimensions instead.
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
#[non_exhaustive]
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

    /// Unset all bits from `other`.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::Modifiers;
    ///
    /// let mut m = Modifiers::BOLD | Modifiers::ITALIC;
    /// m.remove(Modifiers::BOLD);
    /// assert!(!m.contains(Modifiers::BOLD));
    /// assert!(m.contains(Modifiers::ITALIC));
    /// ```
    #[inline]
    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
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
    /// Theme-aware background color. Takes precedence over [`Self::bg`] when set.
    pub theme_bg: Option<ThemeColor>,
    /// Theme-aware text color. Takes precedence over [`Self::text_color`] when set.
    pub theme_text_color: Option<ThemeColor>,
    /// Theme-aware border foreground color. Takes precedence over
    /// [`Self::border_style`]'s foreground when set.
    pub theme_border_fg: Option<ThemeColor>,
    /// Base style to inherit from. Fields in the base are applied first,
    /// then overridden by any `Some` fields in this style.
    ///
    /// Use [`ContainerStyle::extending`] to create a style that inherits.
    pub extends: Option<&'static ContainerStyle>,
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
            theme_bg: None,
            theme_text_color: None,
            theme_border_fg: None,
            extends: None,
        }
    }

    /// Create a style that inherits all fields from a base style.
    ///
    /// Only the fields you set on the returned style will override the base.
    /// The base must be a `&'static ContainerStyle` (typically a `const`).
    ///
    /// # Example
    ///
    /// ```ignore
    /// use slt::{ContainerStyle, Border, ThemeColor};
    ///
    /// const BUTTON: ContainerStyle = ContainerStyle::new()
    ///     .border(Border::Rounded)
    ///     .p(1);
    ///
    /// const BUTTON_DANGER: ContainerStyle = ContainerStyle::extending(&BUTTON)
    ///     .theme_bg(ThemeColor::Error);
    /// ```
    pub const fn extending(base: &'static ContainerStyle) -> Self {
        let mut s = Self::new();
        s.extends = Some(base);
        s
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

    /// Set horizontal margin (left + right). Top and bottom are preserved if
    /// margin was previously set, otherwise default to 0.
    ///
    /// ```
    /// use slt::ContainerStyle;
    /// let s = ContainerStyle::new().mx(2).py(1);
    /// assert_eq!(s.margin.unwrap().left, 2);
    /// assert_eq!(s.margin.unwrap().right, 2);
    /// assert_eq!(s.margin.unwrap().top, 0);
    /// ```
    pub const fn mx(mut self, value: u32) -> Self {
        let m = match self.margin {
            Some(m) => Margin {
                left: value,
                right: value,
                ..m
            },
            None => Margin {
                top: 0,
                bottom: 0,
                left: value,
                right: value,
            },
        };
        self.margin = Some(m);
        self
    }

    /// Set vertical margin (top + bottom). Left and right are preserved if
    /// margin was previously set, otherwise default to 0.
    pub const fn my(mut self, value: u32) -> Self {
        let m = match self.margin {
            Some(m) => Margin {
                top: value,
                bottom: value,
                ..m
            },
            None => Margin {
                top: value,
                bottom: value,
                left: 0,
                right: 0,
            },
        };
        self.margin = Some(m);
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

    /// Set a theme-aware background color that resolves at apply time.
    ///
    /// Takes precedence over [`Self::bg`] when set. The color is resolved
    /// against the active theme when [`crate::ContainerBuilder::apply`] is called.
    pub const fn theme_bg(mut self, color: ThemeColor) -> Self {
        self.theme_bg = Some(color);
        self
    }

    /// Set a theme-aware text color that resolves at apply time.
    ///
    /// Takes precedence over [`Self::text_color`] when set.
    pub const fn theme_text_color(mut self, color: ThemeColor) -> Self {
        self.theme_text_color = Some(color);
        self
    }

    /// Set a theme-aware border foreground color that resolves at apply time.
    ///
    /// Takes precedence over [`Self::border_style`]'s foreground when set.
    pub const fn theme_border_fg(mut self, color: ThemeColor) -> Self {
        self.theme_border_fg = Some(color);
        self
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
/// Per-widget color overrides that fall back to the active theme.
///
/// Literal [`Color`] fields and [`ThemeColor`] fields can be set independently.
/// Resolution order: `theme_*` field > literal field > theme default.
pub struct WidgetColors {
    /// Foreground color override.
    pub fg: Option<Color>,
    /// Background color override.
    pub bg: Option<Color>,
    /// Border color override.
    pub border: Option<Color>,
    /// Accent color override.
    pub accent: Option<Color>,
    /// Theme-aware foreground (takes precedence over [`Self::fg`]).
    pub theme_fg: Option<ThemeColor>,
    /// Theme-aware background (takes precedence over [`Self::bg`]).
    pub theme_bg: Option<ThemeColor>,
    /// Theme-aware border (takes precedence over [`Self::border`]).
    pub theme_border: Option<ThemeColor>,
    /// Theme-aware accent (takes precedence over [`Self::accent`]).
    pub theme_accent: Option<ThemeColor>,
}

impl WidgetColors {
    /// Create a new WidgetColors with all fields set to None (theme defaults).
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            border: None,
            accent: None,
            theme_fg: None,
            theme_bg: None,
            theme_border: None,
            theme_accent: None,
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

    /// Set a theme-aware foreground color.
    pub const fn theme_fg(mut self, color: ThemeColor) -> Self {
        self.theme_fg = Some(color);
        self
    }

    /// Set a theme-aware background color.
    pub const fn theme_bg(mut self, color: ThemeColor) -> Self {
        self.theme_bg = Some(color);
        self
    }

    /// Set a theme-aware border color.
    pub const fn theme_border(mut self, color: ThemeColor) -> Self {
        self.theme_border = Some(color);
        self
    }

    /// Set a theme-aware accent color.
    pub const fn theme_accent(mut self, color: ThemeColor) -> Self {
        self.theme_accent = Some(color);
        self
    }

    /// Resolve the foreground color, preferring theme color, then literal, then fallback.
    pub fn resolve_fg(&self, theme: &Theme, fallback: Color) -> Color {
        self.theme_fg
            .map(|tc| theme.resolve(tc))
            .or(self.fg)
            .unwrap_or(fallback)
    }

    /// Resolve the background color, preferring theme color, then literal, then fallback.
    pub fn resolve_bg(&self, theme: &Theme, fallback: Color) -> Color {
        self.theme_bg
            .map(|tc| theme.resolve(tc))
            .or(self.bg)
            .unwrap_or(fallback)
    }

    /// Resolve the border color, preferring theme color, then literal, then fallback.
    pub fn resolve_border(&self, theme: &Theme, fallback: Color) -> Color {
        self.theme_border
            .map(|tc| theme.resolve(tc))
            .or(self.border)
            .unwrap_or(fallback)
    }

    /// Resolve the accent color, preferring theme color, then literal, then fallback.
    pub fn resolve_accent(&self, theme: &Theme, fallback: Color) -> Color {
        self.theme_accent
            .map(|tc| theme.resolve(tc))
            .or(self.accent)
            .unwrap_or(fallback)
    }
}

/// Default widget colors applied to all instances of a widget type.
///
/// Set via [`crate::RunConfig::widget_theme`]. Each widget type reads its
/// defaults from this struct, then falls back to the active [`Theme`].
/// Per-callsite `_colored()` overrides still take precedence.
///
/// # Example
///
/// ```
/// use slt::{WidgetTheme, WidgetColors, Color};
///
/// let wt = WidgetTheme::new()
///     .button(WidgetColors::new().fg(Color::Cyan));
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct WidgetTheme {
    /// Default colors for buttons.
    pub button: WidgetColors,
    /// Default colors for tables.
    pub table: WidgetColors,
    /// Default colors for lists.
    pub list: WidgetColors,
    /// Default colors for tabs.
    pub tabs: WidgetColors,
    /// Default colors for select dropdowns.
    pub select: WidgetColors,
    /// Default colors for radio groups.
    pub radio: WidgetColors,
    /// Default colors for checkboxes.
    pub checkbox: WidgetColors,
    /// Default colors for toggles.
    pub toggle: WidgetColors,
    /// Default colors for text inputs.
    pub text_input: WidgetColors,
}

impl WidgetTheme {
    /// Create a WidgetTheme with all defaults (no overrides).
    pub const fn new() -> Self {
        Self {
            button: WidgetColors::new(),
            table: WidgetColors::new(),
            list: WidgetColors::new(),
            tabs: WidgetColors::new(),
            select: WidgetColors::new(),
            radio: WidgetColors::new(),
            checkbox: WidgetColors::new(),
            toggle: WidgetColors::new(),
            text_input: WidgetColors::new(),
        }
    }

    /// Set default button colors.
    pub const fn button(mut self, colors: WidgetColors) -> Self {
        self.button = colors;
        self
    }

    /// Set default table colors.
    pub const fn table(mut self, colors: WidgetColors) -> Self {
        self.table = colors;
        self
    }

    /// Set default list colors.
    pub const fn list(mut self, colors: WidgetColors) -> Self {
        self.list = colors;
        self
    }

    /// Set default tabs colors.
    pub const fn tabs(mut self, colors: WidgetColors) -> Self {
        self.tabs = colors;
        self
    }

    /// Set default select colors.
    pub const fn select(mut self, colors: WidgetColors) -> Self {
        self.select = colors;
        self
    }

    /// Set default radio colors.
    pub const fn radio(mut self, colors: WidgetColors) -> Self {
        self.radio = colors;
        self
    }

    /// Set default checkbox colors.
    pub const fn checkbox(mut self, colors: WidgetColors) -> Self {
        self.checkbox = colors;
        self
    }

    /// Set default toggle colors.
    pub const fn toggle(mut self, colors: WidgetColors) -> Self {
        self.toggle = colors;
        self
    }

    /// Set default text input colors.
    pub const fn text_input(mut self, colors: WidgetColors) -> Self {
        self.text_input = colors;
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
        assert_eq!(c.min_width(), Some(10));
        assert_eq!(c.max_width(), Some(40));
        assert_eq!(c.min_height(), Some(5));
        assert_eq!(c.max_height(), Some(20));
        assert_eq!(c.width, WidthSpec::MinMax { min: 10, max: 40 });
    }

    #[test]
    fn constraints_percentage_builder_sets_values() {
        let c = Constraints::default().w_pct(50).h_pct(80);
        assert_eq!(c.width_pct(), Some(50));
        assert_eq!(c.height_pct(), Some(80));
        assert_eq!(c.width, WidthSpec::Pct(50));
        assert_eq!(c.height, HeightSpec::Pct(80));
    }

    #[test]
    fn constraints_default_is_auto() {
        let c = Constraints::default();
        assert_eq!(c.width, WidthSpec::Auto);
        assert_eq!(c.height, HeightSpec::Auto);
    }

    #[test]
    fn constraints_fixed_w_h() {
        let c = Constraints::default().w(20).h(10);
        assert_eq!(c.width, WidthSpec::Fixed(20));
        assert_eq!(c.height, HeightSpec::Fixed(10));
        assert_eq!(c.min_width(), Some(20));
        assert_eq!(c.max_width(), Some(20));
    }

    #[test]
    fn constraints_size_24_bytes() {
        assert_eq!(std::mem::size_of::<Constraints>(), 24);
    }

    #[test]
    fn constraints_set_min_width_promotes_to_minmax() {
        let mut c = Constraints::default();
        c.set_min_width(Some(10));
        assert_eq!(
            c.width,
            WidthSpec::MinMax {
                min: 10,
                max: u32::MAX,
            }
        );
        c.set_max_width(Some(40));
        assert_eq!(c.width, WidthSpec::MinMax { min: 10, max: 40 });
    }

    #[test]
    fn constraints_w_ratio_builder() {
        let c = Constraints::default().w_ratio(1, 3);
        assert_eq!(c.width, WidthSpec::Ratio(1, 3));
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
