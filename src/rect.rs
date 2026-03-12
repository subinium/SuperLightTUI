/// An axis-aligned rectangle with `u32` coordinates.
///
/// Uses `u32` rather than `u16` to avoid overflow bugs that affect other TUI
/// libraries on large terminals. All coordinates are in terminal columns and
/// rows, with `(0, 0)` at the top-left.
///
/// Note: [`Rect::right`] and [`Rect::bottom`] return **exclusive** bounds
/// (one past the last column/row), consistent with Rust range conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    /// Left edge column, inclusive.
    pub x: u32,
    /// Top edge row, inclusive.
    pub y: u32,
    /// Width in terminal columns.
    pub width: u32,
    /// Height in terminal rows.
    pub height: u32,
}

impl Rect {
    /// Create a new rectangle from position and size.
    #[inline]
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Total area in cells (`width * height`).
    #[inline]
    pub const fn area(&self) -> u32 {
        self.width * self.height
    }

    /// Exclusive right edge (`x + width`).
    ///
    /// This is one column past the last column in the rectangle.
    #[inline]
    pub const fn right(&self) -> u32 {
        self.x + self.width
    }

    /// Exclusive bottom edge (`y + height`).
    ///
    /// This is one row past the last row in the rectangle.
    #[inline]
    pub const fn bottom(&self) -> u32 {
        self.y + self.height
    }

    /// Returns `true` if the rectangle has zero area (width or height is zero).
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}
