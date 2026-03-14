//! Double-buffer grid of [`Cell`]s with clip-stack support.
//!
//! Two buffers are maintained per frame (current and previous). Only the diff
//! is flushed to the terminal, giving immediate-mode ergonomics with
//! retained-mode efficiency.

use crate::cell::Cell;
use crate::rect::Rect;
use crate::style::Style;
use unicode_width::UnicodeWidthChar;

/// A 2D grid of [`Cell`]s backing the terminal display.
///
/// Two buffers are kept (current + previous); only the diff is flushed to the
/// terminal, giving immediate-mode ergonomics with retained-mode efficiency.
///
/// The buffer also maintains a clip stack. Push a [`Rect`] with
/// [`Buffer::push_clip`] to restrict writes to that region, and pop it with
/// [`Buffer::pop_clip`] when done.
pub struct Buffer {
    /// The area this buffer covers, in terminal coordinates.
    pub area: Rect,
    /// Flat row-major storage of all cells. Length equals `area.width * area.height`.
    pub content: Vec<Cell>,
    pub(crate) clip_stack: Vec<Rect>,
}

impl Buffer {
    /// Create a buffer filled with blank cells covering `area`.
    pub fn empty(area: Rect) -> Self {
        let size = area.area() as usize;
        Self {
            area,
            content: vec![Cell::default(); size],
            clip_stack: Vec::new(),
        }
    }

    /// Push a clipping rectangle onto the clip stack.
    ///
    /// Subsequent writes are restricted to the intersection of all active clip
    /// regions. Nested calls intersect with the current clip, so the effective
    /// clip can only shrink, never grow.
    pub fn push_clip(&mut self, rect: Rect) {
        let effective = if let Some(current) = self.clip_stack.last() {
            intersect_rects(*current, rect)
        } else {
            rect
        };
        self.clip_stack.push(effective);
    }

    /// Pop the most recently pushed clipping rectangle.
    ///
    /// After this call, writes are clipped to the previous region (or
    /// unclipped if the stack is now empty).
    pub fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }

    fn effective_clip(&self) -> Option<&Rect> {
        self.clip_stack.last()
    }

    #[inline]
    fn index_of(&self, x: u32, y: u32) -> usize {
        ((y - self.area.y) * self.area.width + (x - self.area.x)) as usize
    }

    /// Returns `true` if `(x, y)` is within the buffer's area.
    #[inline]
    pub fn in_bounds(&self, x: u32, y: u32) -> bool {
        x >= self.area.x && x < self.area.right() && y >= self.area.y && y < self.area.bottom()
    }

    /// Return a reference to the cell at `(x, y)`.
    ///
    /// Panics if `(x, y)` is out of bounds.
    #[inline]
    pub fn get(&self, x: u32, y: u32) -> &Cell {
        &self.content[self.index_of(x, y)]
    }

    /// Return a mutable reference to the cell at `(x, y)`.
    ///
    /// Panics if `(x, y)` is out of bounds.
    #[inline]
    pub fn get_mut(&mut self, x: u32, y: u32) -> &mut Cell {
        let idx = self.index_of(x, y);
        &mut self.content[idx]
    }

    /// Write a string into the buffer starting at `(x, y)`.
    ///
    /// Respects cell boundaries and Unicode character widths. Wide characters
    /// (e.g., CJK) occupy two columns; the trailing cell is blanked. Writes
    /// that fall outside the current clip region are skipped but still advance
    /// the cursor position.
    pub fn set_string(&mut self, mut x: u32, y: u32, s: &str, style: Style) {
        if y >= self.area.bottom() {
            return;
        }
        let clip = self.effective_clip().copied();
        for ch in s.chars() {
            if x >= self.area.right() {
                break;
            }
            let char_width = UnicodeWidthChar::width(ch).unwrap_or(0) as u32;
            if char_width == 0 {
                // Append zero-width char (combining mark, ZWJ, variation selector)
                // to the previous cell so grapheme clusters stay intact.
                if x > self.area.x {
                    let prev_in_clip = clip.map_or(true, |clip| {
                        (x - 1) >= clip.x
                            && (x - 1) < clip.right()
                            && y >= clip.y
                            && y < clip.bottom()
                    });
                    if prev_in_clip {
                        self.get_mut(x - 1, y).symbol.push(ch);
                    }
                }
                continue;
            }

            let in_clip = clip.map_or(true, |clip| {
                x >= clip.x && x < clip.right() && y >= clip.y && y < clip.bottom()
            });

            if !in_clip {
                x = x.saturating_add(char_width);
                continue;
            }

            let cell = self.get_mut(x, y);
            cell.set_char(ch);
            cell.set_style(style);

            // Wide characters occupy two cells; blank the trailing cell.
            if char_width > 1 {
                let next_x = x + 1;
                if next_x < self.area.right() {
                    let next = self.get_mut(next_x, y);
                    next.symbol.clear();
                    next.style = style;
                }
            }

            x = x.saturating_add(char_width);
        }
    }

    /// Write a hyperlinked string into the buffer starting at `(x, y)`.
    ///
    /// Like [`Buffer::set_string`] but attaches an OSC 8 hyperlink URL to each
    /// cell. The terminal renders these cells as clickable links.
    pub fn set_string_linked(&mut self, mut x: u32, y: u32, s: &str, style: Style, url: &str) {
        if y >= self.area.bottom() {
            return;
        }
        let clip = self.effective_clip().copied();
        let link = Some(url.to_string());
        for ch in s.chars() {
            if x >= self.area.right() {
                break;
            }
            let char_width = UnicodeWidthChar::width(ch).unwrap_or(0) as u32;
            if char_width == 0 {
                if x > self.area.x {
                    let prev_in_clip = clip.map_or(true, |clip| {
                        (x - 1) >= clip.x
                            && (x - 1) < clip.right()
                            && y >= clip.y
                            && y < clip.bottom()
                    });
                    if prev_in_clip {
                        self.get_mut(x - 1, y).symbol.push(ch);
                    }
                }
                continue;
            }

            let in_clip = clip.map_or(true, |clip| {
                x >= clip.x && x < clip.right() && y >= clip.y && y < clip.bottom()
            });

            if !in_clip {
                x = x.saturating_add(char_width);
                continue;
            }

            let cell = self.get_mut(x, y);
            cell.set_char(ch);
            cell.set_style(style);
            cell.hyperlink = link.clone();

            if char_width > 1 {
                let next_x = x + 1;
                if next_x < self.area.right() {
                    let next = self.get_mut(next_x, y);
                    next.symbol.clear();
                    next.style = style;
                    next.hyperlink = link.clone();
                }
            }

            x = x.saturating_add(char_width);
        }
    }

    /// Write a single character at `(x, y)` with the given style.
    ///
    /// No-ops if `(x, y)` is out of bounds or outside the current clip region.
    pub fn set_char(&mut self, x: u32, y: u32, ch: char, style: Style) {
        let in_clip = self.effective_clip().map_or(true, |clip| {
            x >= clip.x && x < clip.right() && y >= clip.y && y < clip.bottom()
        });
        if !self.in_bounds(x, y) || !in_clip {
            return;
        }
        let cell = self.get_mut(x, y);
        cell.set_char(ch);
        cell.set_style(style);
    }

    /// Compute the diff between `self` (current) and `other` (previous).
    ///
    /// Returns `(x, y, cell)` tuples for every cell that changed. The run loop
    /// uses this to emit only the minimal set of terminal escape sequences
    /// needed to update the display.
    pub fn diff<'a>(&'a self, other: &'a Buffer) -> Vec<(u32, u32, &'a Cell)> {
        let mut updates = Vec::new();
        for y in self.area.y..self.area.bottom() {
            for x in self.area.x..self.area.right() {
                let cur = self.get(x, y);
                let prev = other.get(x, y);
                if cur != prev {
                    updates.push((x, y, cur));
                }
            }
        }
        updates
    }

    /// Reset every cell to a blank space with default style, and clear the clip stack.
    pub fn reset(&mut self) {
        for cell in &mut self.content {
            cell.reset();
        }
        self.clip_stack.clear();
    }

    /// Resize the buffer to fit a new area, resetting all cells.
    ///
    /// If the new area is larger, new cells are initialized to blank. All
    /// existing content is discarded.
    pub fn resize(&mut self, area: Rect) {
        self.area = area;
        let size = area.area() as usize;
        self.content.resize(size, Cell::default());
        self.reset();
    }
}

fn intersect_rects(a: Rect, b: Rect) -> Rect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = a.right().min(b.right());
    let bottom = a.bottom().min(b.bottom());
    let width = right.saturating_sub(x);
    let height = bottom.saturating_sub(y);
    Rect::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_stack_intersects_nested_regions() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 5));
        buf.push_clip(Rect::new(1, 1, 6, 3));
        buf.push_clip(Rect::new(4, 0, 6, 4));

        buf.set_char(3, 2, 'x', Style::new());
        buf.set_char(4, 2, 'y', Style::new());

        assert_eq!(buf.get(3, 2).symbol, " ");
        assert_eq!(buf.get(4, 2).symbol, "y");
    }

    #[test]
    fn set_string_advances_even_when_clipped() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
        buf.push_clip(Rect::new(2, 0, 6, 1));

        buf.set_string(0, 0, "abcd", Style::new());

        assert_eq!(buf.get(2, 0).symbol, "c");
        assert_eq!(buf.get(3, 0).symbol, "d");
    }

    #[test]
    fn pop_clip_restores_previous_clip() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 6, 1));
        buf.push_clip(Rect::new(0, 0, 2, 1));
        buf.push_clip(Rect::new(4, 0, 2, 1));

        buf.set_char(1, 0, 'a', Style::new());
        buf.pop_clip();
        buf.set_char(1, 0, 'b', Style::new());

        assert_eq!(buf.get(1, 0).symbol, "b");
    }

    #[test]
    fn reset_clears_clip_stack() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        buf.push_clip(Rect::new(0, 0, 0, 0));
        buf.reset();
        buf.set_char(0, 0, 'z', Style::new());

        assert_eq!(buf.get(0, 0).symbol, "z");
    }
}
