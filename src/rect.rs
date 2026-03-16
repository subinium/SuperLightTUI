//! Axis-aligned rectangle type used throughout SLT for layout regions,
//! clipping bounds, and hit-test areas.

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

    /// Returns a smaller Rect centered within self.
    ///
    /// If the inner dimensions exceed self's dimensions, they are clamped to self's size.
    /// The returned rectangle is positioned such that it is centered both horizontally
    /// and vertically within self.
    ///
    /// # Example
    /// ```
    /// use slt::Rect;
    /// let outer = Rect::new(0, 0, 10, 10);
    /// let inner = outer.centered(4, 4);
    /// assert_eq!(inner, Rect::new(3, 3, 4, 4));
    /// ```
    #[inline]
    pub fn centered(&self, inner_w: u32, inner_h: u32) -> Rect {
        let w = inner_w.min(self.width);
        let h = inner_h.min(self.height);
        let x = self.x + (self.width.saturating_sub(w)) / 2;
        let y = self.y + (self.height.saturating_sub(h)) / 2;
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    /// Returns the smallest Rect containing both self and other.
    ///
    /// The union encompasses all cells in both rectangles. If either rectangle is empty,
    /// the result may have unexpected dimensions; use `is_empty()` to check.
    ///
    /// # Example
    /// ```
    /// use slt::Rect;
    /// let r1 = Rect::new(0, 0, 5, 5);
    /// let r2 = Rect::new(3, 3, 5, 5);
    /// let union = r1.union(r2);
    /// assert_eq!(union, Rect::new(0, 0, 8, 8));
    /// ```
    #[inline]
    pub fn union(&self, other: Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Rect {
            x,
            y,
            width: right - x,
            height: bottom - y,
        }
    }

    /// Returns the overlapping region between self and other, or None if they don't overlap.
    ///
    /// Two rectangles overlap if they share at least one cell. Adjacent rectangles
    /// (touching at an edge but not overlapping) return None.
    ///
    /// # Example
    /// ```
    /// use slt::Rect;
    /// let r1 = Rect::new(0, 0, 5, 5);
    /// let r2 = Rect::new(3, 3, 5, 5);
    /// let overlap = r1.intersection(r2);
    /// assert_eq!(overlap, Some(Rect::new(3, 3, 2, 2)));
    /// ```
    #[inline]
    pub fn intersection(&self, other: Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        if x < right && y < bottom {
            Some(Rect {
                x,
                y,
                width: right - x,
                height: bottom - y,
            })
        } else {
            None
        }
    }

    /// Returns true if the point (x, y) is inside the rectangle.
    ///
    /// A point is considered inside if it is within the inclusive left/top bounds
    /// and exclusive right/bottom bounds (consistent with Rust range conventions).
    ///
    /// # Example
    /// ```
    /// use slt::Rect;
    /// let r = Rect::new(5, 5, 10, 10);
    /// assert!(r.contains(5, 5));   // top-left corner
    /// assert!(r.contains(14, 14)); // inside
    /// assert!(!r.contains(15, 15)); // outside (exclusive right/bottom)
    /// ```
    #[inline]
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// Returns an iterator over row y-coordinates in this rectangle.
    ///
    /// Yields values from `self.y` to `self.bottom() - 1` (inclusive).
    ///
    /// # Example
    /// ```
    /// use slt::Rect;
    /// let r = Rect::new(0, 2, 5, 3);
    /// let rows: Vec<u32> = r.rows().collect();
    /// assert_eq!(rows, vec![2, 3, 4]);
    /// ```
    #[inline]
    pub fn rows(&self) -> impl Iterator<Item = u32> {
        self.y..self.bottom()
    }

    /// Returns an iterator over all (x, y) positions in this rectangle, row by row.
    ///
    /// Iterates from top-left to bottom-right, filling each row left-to-right before
    /// moving to the next row. Total count is `width * height`.
    ///
    /// # Example
    /// ```
    /// use slt::Rect;
    /// let r = Rect::new(0, 0, 2, 2);
    /// let positions: Vec<(u32, u32)> = r.positions().collect();
    /// assert_eq!(positions, vec![(0, 0), (1, 0), (0, 1), (1, 1)]);
    /// ```
    #[inline]
    pub fn positions(&self) -> impl Iterator<Item = (u32, u32)> {
        let x_start = self.x;
        let x_end = self.right();
        let y_start = self.y;
        let y_end = self.bottom();

        (y_start..y_end).flat_map(move |y| (x_start..x_end).map(move |x| (x, y)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_centered_normal() {
        let outer = Rect::new(0, 0, 10, 10);
        let inner = outer.centered(4, 4);
        assert_eq!(inner, Rect::new(3, 3, 4, 4));
    }

    #[test]
    fn test_centered_larger_than_self() {
        let outer = Rect::new(0, 0, 10, 10);
        let inner = outer.centered(20, 20);
        assert_eq!(inner, Rect::new(0, 0, 10, 10));
    }

    #[test]
    fn test_centered_zero_size() {
        let outer = Rect::new(5, 5, 10, 10);
        let inner = outer.centered(0, 0);
        assert_eq!(inner, Rect::new(10, 10, 0, 0));
    }

    #[test]
    fn test_centered_offset() {
        let outer = Rect::new(10, 20, 20, 20);
        let inner = outer.centered(10, 10);
        assert_eq!(inner, Rect::new(15, 25, 10, 10));
    }

    #[test]
    fn test_union_overlapping() {
        let r1 = Rect::new(0, 0, 5, 5);
        let r2 = Rect::new(3, 3, 5, 5);
        let union = r1.union(r2);
        assert_eq!(union, Rect::new(0, 0, 8, 8));
    }

    #[test]
    fn test_union_non_overlapping() {
        let r1 = Rect::new(0, 0, 5, 5);
        let r2 = Rect::new(10, 10, 5, 5);
        let union = r1.union(r2);
        assert_eq!(union, Rect::new(0, 0, 15, 15));
    }

    #[test]
    fn test_union_same_rect() {
        let r = Rect::new(5, 5, 10, 10);
        let union = r.union(r);
        assert_eq!(union, r);
    }

    #[test]
    fn test_intersection_overlapping() {
        let r1 = Rect::new(0, 0, 5, 5);
        let r2 = Rect::new(3, 3, 5, 5);
        let overlap = r1.intersection(r2);
        assert_eq!(overlap, Some(Rect::new(3, 3, 2, 2)));
    }

    #[test]
    fn test_intersection_non_overlapping() {
        let r1 = Rect::new(0, 0, 5, 5);
        let r2 = Rect::new(10, 10, 5, 5);
        let overlap = r1.intersection(r2);
        assert_eq!(overlap, None);
    }

    #[test]
    fn test_intersection_adjacent() {
        let r1 = Rect::new(0, 0, 5, 5);
        let r2 = Rect::new(5, 0, 5, 5);
        let overlap = r1.intersection(r2);
        assert_eq!(overlap, None);
    }

    #[test]
    fn test_intersection_same_rect() {
        let r = Rect::new(5, 5, 10, 10);
        let overlap = r.intersection(r);
        assert_eq!(overlap, Some(r));
    }

    #[test]
    fn test_contains_inside() {
        let r = Rect::new(5, 5, 10, 10);
        assert!(r.contains(5, 5));
        assert!(r.contains(10, 10));
        assert!(r.contains(14, 14));
    }

    #[test]
    fn test_contains_outside() {
        let r = Rect::new(5, 5, 10, 10);
        assert!(!r.contains(4, 5));
        assert!(!r.contains(5, 4));
        assert!(!r.contains(15, 15));
        assert!(!r.contains(15, 10));
    }

    #[test]
    fn test_contains_on_edge() {
        let r = Rect::new(5, 5, 10, 10);
        assert!(r.contains(5, 5)); // top-left inclusive
        assert!(!r.contains(15, 5)); // right exclusive
        assert!(!r.contains(5, 15)); // bottom exclusive
    }

    #[test]
    fn test_rows_correct_range() {
        let r = Rect::new(0, 2, 5, 3);
        let rows: Vec<u32> = r.rows().collect();
        assert_eq!(rows, vec![2, 3, 4]);
    }

    #[test]
    fn test_rows_single_row() {
        let r = Rect::new(0, 5, 10, 1);
        let rows: Vec<u32> = r.rows().collect();
        assert_eq!(rows, vec![5]);
    }

    #[test]
    fn test_rows_empty() {
        let r = Rect::new(0, 5, 10, 0);
        let rows: Vec<u32> = r.rows().collect();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_positions_correct_count() {
        let r = Rect::new(0, 0, 3, 2);
        let positions: Vec<(u32, u32)> = r.positions().collect();
        assert_eq!(positions.len(), 6);
    }

    #[test]
    fn test_positions_order() {
        let r = Rect::new(0, 0, 2, 2);
        let positions: Vec<(u32, u32)> = r.positions().collect();
        assert_eq!(positions, vec![(0, 0), (1, 0), (0, 1), (1, 1)]);
    }

    #[test]
    fn test_positions_offset() {
        let r = Rect::new(5, 3, 2, 2);
        let positions: Vec<(u32, u32)> = r.positions().collect();
        assert_eq!(positions, vec![(5, 3), (6, 3), (5, 4), (6, 4)]);
    }

    #[test]
    fn test_positions_empty() {
        let r = Rect::new(0, 0, 0, 5);
        let positions: Vec<(u32, u32)> = r.positions().collect();
        assert!(positions.is_empty());
    }
}
