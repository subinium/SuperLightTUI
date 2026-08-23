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
        self.width.saturating_mul(self.height)
    }

    /// Total area in cells without narrowing to `u32`.
    #[inline]
    pub const fn area_u64(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Exclusive right edge (`x + width`).
    ///
    /// This is one column past the last column in the rectangle.
    #[inline]
    pub const fn right(&self) -> u32 {
        self.x.saturating_add(self.width)
    }

    /// Checked exclusive right edge, or `None` when it is not representable.
    #[inline]
    pub const fn checked_right(&self) -> Option<u32> {
        self.x.checked_add(self.width)
    }

    /// Exclusive bottom edge (`y + height`).
    ///
    /// This is one row past the last row in the rectangle.
    #[inline]
    pub const fn bottom(&self) -> u32 {
        self.y.saturating_add(self.height)
    }

    /// Checked exclusive bottom edge, or `None` when it is not representable.
    #[inline]
    pub const fn checked_bottom(&self) -> Option<u32> {
        self.y.checked_add(self.height)
    }

    /// Return whether both exclusive edges are representable as `u32`.
    #[inline]
    pub const fn has_valid_edges(&self) -> bool {
        self.checked_right().is_some() && self.checked_bottom().is_some()
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
        let x = self.x.saturating_add((self.width.saturating_sub(w)) / 2);
        let y = self.y.saturating_add((self.height.saturating_sub(h)) / 2);
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
            width: right.saturating_sub(x),
            height: bottom.saturating_sub(y),
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
                width: right.saturating_sub(x),
                height: bottom.saturating_sub(y),
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
    pub fn rows(&self) -> impl Iterator<Item = u32> + use<> {
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
    pub fn positions(&self) -> impl Iterator<Item = (u32, u32)> + use<> {
        let x_start = self.x;
        let x_end = self.right();
        let y_start = self.y;
        let y_end = self.bottom();

        (y_start..y_end).flat_map(move |y| (x_start..x_end).map(move |x| (x, y)))
    }

    /// Position `self` centered both horizontally and vertically inside `parent`.
    ///
    /// Returns a [`Rect`] with the same `width`/`height` as `self`, but with
    /// `x`/`y` adjusted so the result is centered within `parent`. If `self`
    /// is wider or taller than `parent` on either axis, the corresponding
    /// dimension is clamped to `parent`'s extent on that axis (matching
    /// [`Rect::centered`]'s clamp policy). Self's existing `x`/`y` are
    /// ignored — only its dimensions matter.
    ///
    /// This is the inverse of [`Rect::centered`]: `centered` answers "give
    /// me an inner rect of size W×H centered in me," whereas `center_in`
    /// answers "position me centered inside parent."
    ///
    /// # Example
    /// ```
    /// use slt::Rect;
    /// let dialog = Rect::new(0, 0, 40, 10);
    /// let screen = Rect::new(0, 0, 120, 40);
    /// let r = dialog.center_in(screen);
    /// assert_eq!(r, Rect::new(40, 15, 40, 10));
    /// ```
    #[inline]
    pub const fn center_in(self, parent: Rect) -> Rect {
        let w = if self.width < parent.width {
            self.width
        } else {
            parent.width
        };
        let h = if self.height < parent.height {
            self.height
        } else {
            parent.height
        };
        let x = parent.x.saturating_add(parent.width.saturating_sub(w) / 2);
        let y = parent.y.saturating_add(parent.height.saturating_sub(h) / 2);
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    /// Position `self` centered horizontally inside `parent`; preserve `self.y` and `self.height`.
    ///
    /// If `self.width` exceeds `parent.width`, the returned rect's width is
    /// clamped to `parent.width` (matching [`Rect::centered`]).
    ///
    /// # Example
    /// ```
    /// use slt::Rect;
    /// let banner = Rect::new(0, 5, 30, 3);
    /// let screen = Rect::new(0, 0, 120, 40);
    /// let r = banner.center_horizontally_in(screen);
    /// assert_eq!(r, Rect::new(45, 5, 30, 3));
    /// ```
    #[inline]
    pub const fn center_horizontally_in(self, parent: Rect) -> Rect {
        let w = if self.width < parent.width {
            self.width
        } else {
            parent.width
        };
        let x = parent.x.saturating_add(parent.width.saturating_sub(w) / 2);
        Rect {
            x,
            y: self.y,
            width: w,
            height: self.height,
        }
    }

    /// Position `self` centered vertically inside `parent`; preserve `self.x` and `self.width`.
    ///
    /// If `self.height` exceeds `parent.height`, the returned rect's height
    /// is clamped to `parent.height` (matching [`Rect::centered`]).
    ///
    /// # Example
    /// ```
    /// use slt::Rect;
    /// let sidebar = Rect::new(2, 0, 20, 10);
    /// let screen = Rect::new(0, 0, 120, 40);
    /// let r = sidebar.center_vertically_in(screen);
    /// assert_eq!(r, Rect::new(2, 15, 20, 10));
    /// ```
    #[inline]
    pub const fn center_vertically_in(self, parent: Rect) -> Rect {
        let h = if self.height < parent.height {
            self.height
        } else {
            parent.height
        };
        let y = parent.y.saturating_add(parent.height.saturating_sub(h) / 2);
        Rect {
            x: self.x,
            y,
            width: self.width,
            height: h,
        }
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

    #[test]
    fn rect_area_no_overflow() {
        // u32::MAX * u32::MAX would wrap to 0 without saturating_mul
        let r = Rect::new(0, 0, u32::MAX, u32::MAX);
        assert_eq!(r.area(), u32::MAX);
        // Concrete case from issue #166: 65536 * 65536 wraps to 0 without fix
        let r2 = Rect::new(0, 0, 65536, 65536);
        assert_eq!(r2.area(), u32::MAX);
        assert_eq!(r2.area_u64(), 4_294_967_296);
    }

    #[test]
    fn checked_edges_report_unrepresentable_origins() {
        let invalid = Rect::new(u32::MAX, u32::MAX, 1, 1);
        assert_eq!(invalid.checked_right(), None);
        assert_eq!(invalid.checked_bottom(), None);
        assert!(!invalid.has_valid_edges());

        let valid = Rect::new(u32::MAX - 1, u32::MAX - 1, 1, 1);
        assert_eq!(valid.checked_right(), Some(u32::MAX));
        assert_eq!(valid.checked_bottom(), Some(u32::MAX));
        assert!(valid.has_valid_edges());
    }

    #[test]
    fn rect_edges_saturate_instead_of_wrapping() {
        let r = Rect::new(u32::MAX, u32::MAX - 1, 10, 10);
        assert_eq!(r.right(), u32::MAX);
        assert_eq!(r.bottom(), u32::MAX);
        assert!(!r.contains(0, 0), "saturated edge must not wrap to origin");
    }

    #[test]
    fn rect_union_and_intersection_do_not_wrap_at_u32_max() {
        let edge = Rect::new(u32::MAX - 1, u32::MAX - 1, 10, 10);
        let origin = Rect::new(0, 0, 1, 1);

        assert_eq!(edge.union(origin), Rect::new(0, 0, u32::MAX, u32::MAX));
        assert_eq!(edge.intersection(origin), None);
    }

    #[test]
    fn rect_centering_saturates_large_offsets() {
        let parent = Rect::new(u32::MAX - 2, u32::MAX - 2, 10, 10);
        let child = Rect::new(0, 0, 2, 2).center_in(parent);
        assert_eq!(child.x, u32::MAX);
        assert_eq!(child.y, u32::MAX);
    }

    #[test]
    fn test_center_in_basic() {
        let dialog = Rect::new(0, 0, 40, 10);
        let screen = Rect::new(0, 0, 120, 40);
        assert_eq!(dialog.center_in(screen), Rect::new(40, 15, 40, 10));
    }

    #[test]
    fn test_center_in_self_bigger_clamps() {
        // self larger than parent on both axes -> clamp to parent extent.
        let oversize = Rect::new(0, 0, 200, 80);
        let screen = Rect::new(0, 0, 120, 40);
        assert_eq!(oversize.center_in(screen), Rect::new(0, 0, 120, 40));
    }

    #[test]
    fn test_center_in_offset_parent() {
        // Parent at (10, 5) with size 100 x 30; centering 40 x 10 ->
        // x = 10 + (100 - 40) / 2 = 40, y = 5 + (30 - 10) / 2 = 15
        let dialog = Rect::new(999, 999, 40, 10); // self.x/self.y ignored
        let parent = Rect::new(10, 5, 100, 30);
        assert_eq!(dialog.center_in(parent), Rect::new(40, 15, 40, 10));
    }

    #[test]
    fn test_center_in_self_position_ignored() {
        // self.x/self.y must NOT influence the result — only dimensions.
        let a = Rect::new(0, 0, 10, 4).center_in(Rect::new(0, 0, 20, 10));
        let b = Rect::new(99, 99, 10, 4).center_in(Rect::new(0, 0, 20, 10));
        assert_eq!(a, b);
    }

    #[test]
    fn test_center_horizontally_in_preserves_y_height() {
        let banner = Rect::new(0, 5, 30, 3);
        let screen = Rect::new(0, 0, 120, 40);
        assert_eq!(
            banner.center_horizontally_in(screen),
            Rect::new(45, 5, 30, 3)
        );
    }

    #[test]
    fn test_center_horizontally_in_clamps_width() {
        let wide = Rect::new(0, 4, 200, 3);
        let screen = Rect::new(0, 0, 120, 40);
        // width clamped, x = 0 (saturating_sub(120, 120) = 0)
        assert_eq!(wide.center_horizontally_in(screen), Rect::new(0, 4, 120, 3));
    }

    #[test]
    fn test_center_vertically_in_preserves_x_width() {
        let sidebar = Rect::new(2, 0, 20, 10);
        let screen = Rect::new(0, 0, 120, 40);
        assert_eq!(
            sidebar.center_vertically_in(screen),
            Rect::new(2, 15, 20, 10)
        );
    }

    #[test]
    fn test_center_vertically_in_clamps_height() {
        let tall = Rect::new(3, 0, 8, 200);
        let screen = Rect::new(0, 0, 120, 40);
        assert_eq!(tall.center_vertically_in(screen), Rect::new(3, 0, 8, 40));
    }
}
