//! Double-buffer grid of [`Cell`]s with clip-stack support.
//!
//! Two buffers are maintained per frame (current and previous). Only the diff
//! is flushed to the terminal, giving immediate-mode ergonomics with
//! retained-mode efficiency.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::cell::Cell;
use crate::rect::Rect;
use crate::style::Style;
use unicode_width::UnicodeWidthChar;

/// Maximum bytes allowed in a single cell's `symbol` field.
///
/// A grapheme cluster rarely exceeds ~16 bytes in the wild; anything
/// longer is typically an attempt to weaponize zero-width combining chars.
/// This cap bounds the worst case flush cost per cell.
const MAX_CELL_SYMBOL_BYTES: usize = 32;

/// Hard cap on pixel count processed by image decode/encode paths.
///
/// 16_777_216 ≈ 4096×4096 — well above any sane terminal image payload,
/// but guards 32-bit targets (WASM) from overflow and prevents a
/// hostile `width`/`height` pair from triggering multi-GiB allocations.
pub(crate) const MAX_IMAGE_PIXELS: u64 = 16_777_216;

/// Replace terminal-dangerous control characters with `U+FFFD`.
///
/// Unfiltered C0 (0x00–0x1F), DEL (0x7F), or C1 (0x80–0x9F) bytes can
/// break out of cell rendering and inject arbitrary escape sequences
/// (cursor moves, OSC 52 clipboard, title spoof, etc.) when flushed.
/// Replacing with the replacement character keeps byte counts sane and
/// makes the tampering visible.
#[inline]
fn sanitize_cell_char(ch: char) -> char {
    let c = ch as u32;
    if c < 0x20 || c == 0x7f || (0x80..=0x9f).contains(&c) {
        '\u{FFFD}'
    } else {
        ch
    }
}

/// Structured Kitty graphics protocol image placement.
///
/// Stored separately from raw escape sequences so the terminal can manage
/// image IDs, compression, and placement lifecycle. Images are deduplicated
/// by `content_hash` — identical pixel data is uploaded only once.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct KittyPlacement {
    /// Hash of the RGBA pixel data for dedup (avoids re-uploading).
    pub content_hash: u64,
    /// Reference-counted raw RGBA pixel data (shared across frames).
    pub rgba: Arc<Vec<u8>>,
    /// Source image width in pixels.
    pub src_width: u32,
    /// Source image height in pixels.
    pub src_height: u32,
    /// Screen cell position.
    pub x: u32,
    pub y: u32,
    /// Cell columns/rows to display.
    pub cols: u32,
    pub rows: u32,
    /// Source crop Y offset in pixels (for scroll clipping).
    pub crop_y: u32,
    /// Source crop height in pixels (0 = full height from crop_y).
    pub crop_h: u32,
}

/// Compute a content hash for RGBA pixel data.
pub(crate) fn hash_rgba(data: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

impl PartialEq for KittyPlacement {
    fn eq(&self, other: &Self) -> bool {
        self.content_hash == other.content_hash
            && self.x == other.x
            && self.y == other.y
            && self.cols == other.cols
            && self.rows == other.rows
            && self.crop_y == other.crop_y
            && self.crop_h == other.crop_h
    }
}

/// Scroll clip information applied to Kitty image placements emitted inside a
/// raw-draw callback.
///
/// Stored on a stack so that nested raw-draw regions restore the outer clip
/// info on pop, rather than silently clobbering it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KittyClipInfo {
    /// Rows of the source region already scrolled off the top.
    pub top_clip_rows: u32,
    /// Original total row count of the scrollable content.
    pub original_height: u32,
}

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
    pub(crate) raw_sequences: Vec<(u32, u32, String)>,
    pub(crate) kitty_placements: Vec<KittyPlacement>,
    pub(crate) cursor_pos: Option<(u32, u32)>,
    /// Stack of scroll clip infos set by the run loop before invoking draw
    /// closures. The top entry is the active clip; nested raw-draw regions
    /// push and pop without losing the outer clip.
    pub(crate) kitty_clip_info_stack: Vec<KittyClipInfo>,
}

impl Buffer {
    /// Create a buffer filled with blank cells covering `area`.
    pub fn empty(area: Rect) -> Self {
        let size = area.area() as usize;
        Self {
            area,
            content: vec![Cell::default(); size],
            clip_stack: Vec::new(),
            raw_sequences: Vec::new(),
            kitty_placements: Vec::new(),
            cursor_pos: None,
            kitty_clip_info_stack: Vec::new(),
        }
    }

    /// Push a scroll clip info frame. Paired with [`Buffer::pop_kitty_clip`].
    pub(crate) fn push_kitty_clip(&mut self, info: KittyClipInfo) {
        self.kitty_clip_info_stack.push(info);
    }

    /// Pop the most recently pushed scroll clip info frame.
    pub(crate) fn pop_kitty_clip(&mut self) -> Option<KittyClipInfo> {
        self.kitty_clip_info_stack.pop()
    }

    /// Peek the currently active scroll clip info, if any.
    pub(crate) fn current_kitty_clip(&self) -> Option<&KittyClipInfo> {
        self.kitty_clip_info_stack.last()
    }

    pub(crate) fn set_cursor_pos(&mut self, x: u32, y: u32) {
        self.cursor_pos = Some((x, y));
    }

    #[cfg(feature = "crossterm")]
    pub(crate) fn cursor_pos(&self) -> Option<(u32, u32)> {
        self.cursor_pos
    }

    /// Store a raw escape sequence to be written at position `(x, y)` during flush.
    ///
    /// Used for Sixel images and other passthrough sequences.
    /// Respects the clip stack: sequences fully outside the current clip are skipped.
    pub fn raw_sequence(&mut self, x: u32, y: u32, seq: String) {
        if let Some(clip) = self.effective_clip() {
            if x >= clip.right() || y >= clip.bottom() {
                return;
            }
        }
        self.raw_sequences.push((x, y, seq));
    }

    /// Store a structured Kitty graphics protocol placement.
    ///
    /// Unlike `raw_sequence`, Kitty placements are managed with image IDs,
    /// compression, and placement lifecycle by the terminal flush code.
    /// Scroll crop info is automatically applied from the top of the
    /// `kitty_clip_info_stack` (set via [`Buffer::push_kitty_clip`]).
    pub(crate) fn kitty_place(&mut self, mut p: KittyPlacement) {
        // Apply clip check
        if let Some(clip) = self.effective_clip() {
            if p.x >= clip.right()
                || p.y >= clip.bottom()
                || p.x + p.cols <= clip.x
                || p.y + p.rows <= clip.y
            {
                return;
            }
        }

        // Apply scroll crop info if any frame is active
        if let Some(info) = self.current_kitty_clip() {
            let top_clip_rows = info.top_clip_rows;
            let original_height = info.original_height;
            if original_height > 0 && (top_clip_rows > 0 || p.rows < original_height) {
                let ratio = p.src_height as f64 / original_height as f64;
                p.crop_y = (top_clip_rows as f64 * ratio) as u32;
                let bottom_clip = original_height.saturating_sub(top_clip_rows + p.rows);
                let bottom_pixels = (bottom_clip as f64 * ratio) as u32;
                p.crop_h = p.src_height.saturating_sub(p.crop_y + bottom_pixels);
            }
        }

        self.kitty_placements.push(p);
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
    /// Panics if `(x, y)` is out of bounds. Use [`Buffer::try_get`] when the
    /// coordinates may come from untrusted input.
    #[inline]
    pub fn get(&self, x: u32, y: u32) -> &Cell {
        assert!(
            self.in_bounds(x, y),
            "Buffer::get({x}, {y}) out of bounds for area {:?}",
            self.area
        );
        &self.content[self.index_of(x, y)]
    }

    /// Return a mutable reference to the cell at `(x, y)`.
    ///
    /// Panics if `(x, y)` is out of bounds. Use [`Buffer::try_get_mut`] when
    /// the coordinates may come from untrusted input.
    #[inline]
    pub fn get_mut(&mut self, x: u32, y: u32) -> &mut Cell {
        assert!(
            self.in_bounds(x, y),
            "Buffer::get_mut({x}, {y}) out of bounds for area {:?}",
            self.area
        );
        let idx = self.index_of(x, y);
        &mut self.content[idx]
    }

    /// Return a reference to the cell at `(x, y)`, or `None` if out of bounds.
    ///
    /// Non-panicking counterpart to [`Buffer::get`]. Prefer this inside
    /// `draw()` closures when coordinates are computed from mouse input,
    /// scroll offsets, or other sources that could land outside the buffer.
    #[inline]
    pub fn try_get(&self, x: u32, y: u32) -> Option<&Cell> {
        if self.in_bounds(x, y) {
            Some(&self.content[self.index_of(x, y)])
        } else {
            None
        }
    }

    /// Return a mutable reference to the cell at `(x, y)`, or `None` if out
    /// of bounds.
    ///
    /// Non-panicking counterpart to [`Buffer::get_mut`].
    #[inline]
    pub fn try_get_mut(&mut self, x: u32, y: u32) -> Option<&mut Cell> {
        if self.in_bounds(x, y) {
            let idx = self.index_of(x, y);
            Some(&mut self.content[idx])
        } else {
            None
        }
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
            let ch = sanitize_cell_char(ch);
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
                        let prev = self.get_mut(x - 1, y);
                        if prev.symbol.len() + ch.len_utf8() <= MAX_CELL_SYMBOL_BYTES {
                            prev.symbol.push(ch);
                        }
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
        let sanitized_url = sanitize_osc8_url(url);
        let link = sanitized_url.map(compact_str::CompactString::new);
        for ch in s.chars() {
            if x >= self.area.right() {
                break;
            }
            let ch = sanitize_cell_char(ch);
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
                        let prev = self.get_mut(x - 1, y);
                        if prev.symbol.len() + ch.len_utf8() <= MAX_CELL_SYMBOL_BYTES {
                            prev.symbol.push(ch);
                        }
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
        self.raw_sequences.clear();
        self.kitty_placements.clear();
        self.cursor_pos = None;
        self.kitty_clip_info_stack.clear();
    }

    /// Reset every cell and apply a background color to all cells.
    pub fn reset_with_bg(&mut self, bg: crate::style::Color) {
        for cell in &mut self.content {
            cell.reset();
            cell.style.bg = Some(bg);
        }
        self.clip_stack.clear();
        self.raw_sequences.clear();
        self.kitty_placements.clear();
        self.cursor_pos = None;
        self.kitty_clip_info_stack.clear();
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

/// Validate an OSC 8 hyperlink URL, returning `Some(url)` if safe to emit.
///
/// Rejects URLs containing control bytes, the BEL terminator, or an
/// embedded ST (`ESC \`). Those would let an attacker-controlled URL
/// prematurely close the OSC 8 sequence and inject arbitrary follow-up
/// commands (e.g., OSC 52 clipboard writes). Also caps length at 2048
/// bytes — longer than any legitimate URL and enough to prevent DoS via
/// balloon-sized hyperlinks.
pub(crate) fn sanitize_osc8_url(url: &str) -> Option<String> {
    const MAX_URL_BYTES: usize = 2048;
    if url.is_empty() || url.len() > MAX_URL_BYTES {
        return None;
    }
    let bytes = url.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        // Reject all C0 controls (incl. BEL, ESC), DEL, and C1 control range.
        if b < 0x20 || b == 0x7f {
            return None;
        }
        // Reject raw ESC \ (ST terminator) just in case something sneaked through.
        if b == 0x1b {
            return None;
        }
        i += 1;
    }
    Some(url.to_string())
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

    #[test]
    fn set_string_replaces_control_chars_with_replacement() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 6, 1));
        // ESC must never land in a cell — a flushed ESC would let the
        // string escape its cell and execute as a real terminal command.
        buf.set_string(0, 0, "a\x1bbc", Style::new());
        assert_eq!(buf.get(0, 0).symbol, "a");
        assert_eq!(buf.get(1, 0).symbol, "\u{FFFD}");
        assert_eq!(buf.get(2, 0).symbol, "b");
        assert_eq!(buf.get(3, 0).symbol, "c");
    }

    #[test]
    fn zero_width_combining_does_not_append_control_bytes() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        buf.set_char(0, 0, 'a', Style::new());
        // BEL is zero-width per unicode_width; the pre-fix code would have
        // pushed it onto cell(0,0).symbol. After sanitize_cell_char it is
        // replaced with U+FFFD and then appended (width 1, still fits).
        buf.set_string(1, 0, "\x07", Style::new());
        let symbol = buf.get(1, 0).symbol.as_str();
        assert!(!symbol.contains('\x07'), "BEL leaked into cell symbol");
    }

    #[test]
    fn set_string_caps_combining_overflow() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        buf.set_char(0, 0, 'a', Style::new());
        // 200 copies of an ASCII-printable zero-width-ish char would bypass
        // the byte cap. Use a legitimate zero-width combining character —
        // U+0301 (combining acute accent) — and confirm the cap kicks in.
        let combining: String = "\u{0301}".repeat(200);
        buf.set_string(1, 0, &combining, Style::new());
        assert!(
            buf.get(0, 0).symbol.len() <= MAX_CELL_SYMBOL_BYTES,
            "cell symbol exceeded MAX_CELL_SYMBOL_BYTES cap"
        );
    }

    #[test]
    fn sanitize_osc8_url_rejects_control_chars_and_esc() {
        assert!(sanitize_osc8_url("https://example.com").is_some());
        assert!(sanitize_osc8_url("https://example.com?q=1&r=2").is_some());
        // BEL — terminates OSC, would let follow-up text be interpreted.
        assert!(sanitize_osc8_url("https://example.com\x07attack").is_none());
        // ESC — can open ST (ESC \) or another OSC.
        assert!(sanitize_osc8_url("https://example.com\x1b]52;c;hi\x1b\\").is_none());
        // Empty / oversize.
        assert!(sanitize_osc8_url("").is_none());
        assert!(sanitize_osc8_url(&"a".repeat(2049)).is_none());
    }

    #[test]
    fn try_get_out_of_bounds_returns_none() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 2));
        assert!(buf.try_get(0, 0).is_some());
        assert!(buf.try_get(2, 0).is_none());
        assert!(buf.try_get(0, 2).is_none());
        assert!(buf.try_get_mut(5, 5).is_none());
    }

    #[test]
    fn kitty_clip_stack_restores_outer_on_pop() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 4));
        assert!(buf.current_kitty_clip().is_none());

        let outer = KittyClipInfo {
            top_clip_rows: 2,
            original_height: 10,
        };
        let inner = KittyClipInfo {
            top_clip_rows: 5,
            original_height: 20,
        };

        buf.push_kitty_clip(outer);
        assert_eq!(buf.current_kitty_clip(), Some(&outer));

        // Nested region pushes its own frame.
        buf.push_kitty_clip(inner);
        assert_eq!(buf.current_kitty_clip(), Some(&inner));

        // After inner pops, outer MUST still be active — the bug this
        // refactor fixes is exactly that the outer was previously clobbered.
        let popped_inner = buf.pop_kitty_clip();
        assert_eq!(popped_inner, Some(inner));
        assert_eq!(buf.current_kitty_clip(), Some(&outer));

        let popped_outer = buf.pop_kitty_clip();
        assert_eq!(popped_outer, Some(outer));
        assert!(buf.current_kitty_clip().is_none());
    }

    #[test]
    fn kitty_clip_stack_cleared_on_reset() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 2));
        buf.push_kitty_clip(KittyClipInfo {
            top_clip_rows: 1,
            original_height: 2,
        });
        buf.push_kitty_clip(KittyClipInfo {
            top_clip_rows: 3,
            original_height: 4,
        });
        buf.reset();
        assert!(buf.kitty_clip_info_stack.is_empty());
        assert!(buf.current_kitty_clip().is_none());
    }

    #[test]
    fn kitty_clip_pop_on_empty_stack_is_none() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 2));
        assert!(buf.pop_kitty_clip().is_none());
    }
}
