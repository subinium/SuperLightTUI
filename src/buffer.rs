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

/// Returns `true` if `s` contains any codepoint that can trigger
/// right-to-left or explicit bidirectional reordering under the Unicode
/// Bidirectional Algorithm (UAX #9).
///
/// Pure-LTR strings (ASCII, Latin, CJK, …) return `false` and take the
/// zero-allocation fast path in [`Buffer::set_string`]: no `String` is
/// allocated and `unicode-bidi` is never invoked. Only strings that carry
/// Hebrew, Arabic, Syriac, Thaana, Arabic presentation forms, or the
/// explicit bidi control characters (RLM/LRM, RLE/LRE, RLO/LRO, PDF,
/// RLI/LRI/FSI/PDI) need the full reorder pass.
///
/// This is intentionally a cheap, conservative character-class scan rather
/// than a full UAX #9 resolution: a `true` here only *gates* the (possibly
/// no-op) reorder, so over-inclusion costs at worst one extra reorder call,
/// never incorrect output. Under-inclusion would silently mirror RTL text,
/// so the ranges err toward inclusion.
#[cfg(feature = "bidi")]
#[inline]
fn needs_bidi_reorder(s: &str) -> bool {
    s.chars().any(|c| {
        let u = c as u32;
        matches!(u,
            0x0590..=0x05FF | // Hebrew
            0x0600..=0x06FF | // Arabic
            0x0700..=0x074F | // Syriac
            0x0750..=0x077F | // Arabic Supplement
            0x0780..=0x07BF | // Thaana
            0x08A0..=0x08FF | // Arabic Extended-A
            0xFB1D..=0xFDFF | // Hebrew/Arabic presentation forms-A
            0xFE70..=0xFEFF   // Arabic presentation forms-B
        )
        // explicit bidi controls: LRM, RLM, RLE/LRE/PDF/LRO/RLO, RLI/LRI/FSI/PDI
        || matches!(u, 0x200E | 0x200F | 0x202A..=0x202E | 0x2066..=0x2069)
    })
}

/// Reorder one logical-order line into visual (display) order per UAX #9.
///
/// The input is treated as a single paragraph (callers already split on
/// `\n` upstream — see [`Buffer::set_string`]). The base paragraph
/// direction is resolved from the first strong character (no override),
/// matching default UAX #9 behavior. Returns the visually-ordered string.
///
/// Only ever called after [`needs_bidi_reorder`] returns `true`, so the
/// `String` allocation here is incurred solely on the RTL path; pure-LTR
/// input never reaches this function.
#[cfg(feature = "bidi")]
fn reorder_line_visual(s: &str) -> String {
    use unicode_bidi::BidiInfo;
    // No paragraph override: let the first strong char set base direction.
    let info = BidiInfo::new(s, None);
    match info.paragraphs.first() {
        // A single input line is a single paragraph; reorder its full range.
        Some(para) => info.reorder_line(para, para.range.clone()).into_owned(),
        None => s.to_string(), // empty input → no paragraph
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
    /// Per-row digest of every cell on row `y`, used by `flush_buffer_diff`
    /// to skip the per-cell scan when both the dirty flag and the hash
    /// match the previous frame (issue #171).
    ///
    /// Length equals `area.height`. Stale until
    /// [`Buffer::recompute_line_hashes`] is called — `flush_buffer_diff` is
    /// the only call site that relies on these being up to date.
    pub(crate) line_hashes: Vec<u64>,
    /// Per-row dirty flag. Set by every cell-write path
    /// ([`Buffer::set_string`], [`Buffer::set_string_linked`],
    /// [`Buffer::set_char`], [`Buffer::reset`], [`Buffer::reset_with_bg`]).
    /// Cleared by [`Buffer::recompute_line_hashes`] after the row hash is
    /// refreshed.
    ///
    /// A `false` entry means the row has not been touched since the last
    /// hash refresh, so `flush_buffer_diff` can short-circuit the cell
    /// scan when its hash also matches `previous.line_hashes[y]`.
    pub(crate) line_dirty: Vec<bool>,
}

impl Buffer {
    /// Create a buffer filled with blank cells covering `area`.
    pub fn empty(area: Rect) -> Self {
        let size = area.area() as usize;
        let height = area.height as usize;
        Self {
            area,
            content: vec![Cell::default(); size],
            clip_stack: Vec::new(),
            raw_sequences: Vec::new(),
            kitty_placements: Vec::new(),
            cursor_pos: None,
            kitty_clip_info_stack: Vec::new(),
            // Empty buffers start with default cells on every row; their
            // hashes are equal across two empty buffers, so initialise to
            // 0 with `line_dirty=true` so the first flush still recomputes.
            line_hashes: vec![0; height],
            line_dirty: vec![true; height],
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
    pub fn set_string(&mut self, x: u32, y: u32, s: &str, style: Style) {
        self.set_string_inner(x, y, s, style, None);
    }

    /// Write a hyperlinked string into the buffer starting at `(x, y)`.
    ///
    /// Like [`Buffer::set_string`] but attaches an OSC 8 hyperlink URL to each
    /// cell. The terminal renders these cells as clickable links.
    pub fn set_string_linked(&mut self, x: u32, y: u32, s: &str, style: Style, url: &str) {
        let link = sanitize_osc8_url(url).map(compact_str::CompactString::new);
        self.set_string_inner(x, y, s, style, link.as_ref());
    }

    /// Shared implementation for [`Self::set_string`] and
    /// [`Self::set_string_linked`].
    ///
    /// `link` is `Some` only for the OSC 8 path; both paths share clip,
    /// wide-char, and zero-width grapheme handling. Keeping a single
    /// implementation prevents the two call sites from drifting on edge cases
    /// (e.g., `MAX_CELL_SYMBOL_BYTES` checks, wide-char blanking).
    fn set_string_inner(
        &mut self,
        mut x: u32,
        y: u32,
        s: &str,
        style: Style,
        link: Option<&compact_str::CompactString>,
    ) {
        if y >= self.area.bottom() {
            return;
        }
        // Issue #171: mark this row dirty so the next flush refreshes its
        // hash. Marking unconditionally here keeps the write paths cheap;
        // false positives only cost one redundant hash recompute, never a
        // correctness issue.
        self.mark_row_dirty(y);
        // Bidi (UAX #9) reorder: convert this logical-order line into visual
        // (display) order before the positional cell-write loop below. The
        // loop is purely left-to-right by column, so RTL runs must be
        // reordered *here* or they render mirrored. `needs_bidi_reorder`
        // gates the work so pure-LTR input neither allocates nor calls into
        // `unicode-bidi` — its output is byte-identical to skipping this
        // block entirely. Width/clip/zero-width/hyperlink handling below is
        // order-independent and applies unchanged to the reordered glyphs.
        #[cfg(feature = "bidi")]
        let reordered;
        #[cfg(feature = "bidi")]
        let s: &str = if needs_bidi_reorder(s) {
            reordered = reorder_line_visual(s);
            &reordered
        } else {
            s
        };
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
            cell.hyperlink = link.cloned();

            // Wide characters occupy two cells; blank the trailing cell.
            if char_width > 1 {
                let next_x = x + 1;
                if next_x < self.area.right() {
                    let next = self.get_mut(next_x, y);
                    next.symbol.clear();
                    next.style = style;
                    next.hyperlink = link.cloned();
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
        // Issue #171: mark this row dirty so the next flush refreshes its
        // hash before deciding whether to skip the per-cell scan.
        self.mark_row_dirty(y);
        let cell = self.get_mut(x, y);
        cell.set_char(ch);
        cell.set_style(style);
    }

    /// Mark row `y` as dirty so the next flush recomputes its line hash.
    ///
    /// `y` is in the buffer's coordinate space (i.e. `area.y..area.bottom()`).
    /// Out-of-range values are ignored so callers don't need to bounds-check
    /// before invoking this on every cell write.
    #[inline]
    pub(crate) fn mark_row_dirty(&mut self, y: u32) {
        if y < self.area.y {
            return;
        }
        let idx = (y - self.area.y) as usize;
        if let Some(slot) = self.line_dirty.get_mut(idx) {
            *slot = true;
        }
    }

    /// Recompute the per-row digest for every row currently flagged dirty.
    ///
    /// This is the only call site that updates [`Self::line_hashes`]; once
    /// a row's hash is refreshed its `line_dirty` entry is cleared. Hashes
    /// derive from each cell's `(symbol, style, hyperlink)` tuple via
    /// [`std::collections::hash_map::DefaultHasher`] — sufficient for
    /// equality detection with no extra dependency.
    ///
    /// Called by `flush_buffer_diff` once per frame, before the per-row
    /// skip check (issue #171).
    ///
    /// Gated on `crossterm` (the only flush call site) and `test`. Without
    /// the gate it shows as `dead_code` under `--no-default-features`.
    #[cfg(any(feature = "crossterm", test))]
    pub(crate) fn recompute_line_hashes(&mut self) {
        let height = self.area.height;
        if height == 0 {
            return;
        }
        // `line_hashes` / `line_dirty` are sized at construction / resize;
        // an interior mutation (e.g. resize before reset) could leave them
        // out of step with `area.height`. Repair lazily here so callers
        // never observe a stale length.
        let expected_len = height as usize;
        if self.line_hashes.len() != expected_len {
            self.line_hashes.resize(expected_len, 0);
        }
        if self.line_dirty.len() != expected_len {
            self.line_dirty.resize(expected_len, true);
        }

        let width = self.area.width as usize;
        for (idx, dirty) in self.line_dirty.iter_mut().enumerate() {
            if !*dirty {
                continue;
            }
            let row_start = idx * width;
            let row_end = row_start + width;
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            for cell in &self.content[row_start..row_end] {
                cell.symbol.as_str().hash(&mut hasher);
                cell.style.hash(&mut hasher);
                cell.hyperlink.as_deref().hash(&mut hasher);
            }
            self.line_hashes[idx] = hasher.finish();
            *dirty = false;
        }
    }

    /// Returns `true` if row `y` (buffer-space) was not touched since the
    /// last [`Self::recompute_line_hashes`] call.
    ///
    /// Gated on `crossterm` (consumed by `flush_buffer_diff`) and `test`.
    ///
    /// Used by `flush_buffer_diff` to short-circuit the per-cell scan when
    /// combined with a hash match against the previous frame (issue #171).
    /// Out-of-range rows report as dirty so callers fall back to the
    /// existing per-cell path on edge inputs.
    #[inline]
    #[cfg(any(feature = "crossterm", test))]
    pub(crate) fn row_clean(&self, y: u32) -> bool {
        if y < self.area.y {
            return false;
        }
        let idx = (y - self.area.y) as usize;
        self.line_dirty
            .get(idx)
            .copied()
            .map(|d| !d)
            .unwrap_or(false)
    }

    /// Read row `y`'s cached digest, or `None` if out of range.
    ///
    /// Pairs with [`Self::row_clean`] inside `flush_buffer_diff`: only the
    /// hash for clean rows is used as a short-circuit signal, so callers
    /// must check `row_clean` first.
    #[inline]
    #[cfg(any(feature = "crossterm", test))]
    pub(crate) fn row_hash(&self, y: u32) -> Option<u64> {
        if y < self.area.y {
            return None;
        }
        let idx = (y - self.area.y) as usize;
        self.line_hashes.get(idx).copied()
    }

    /// Compute the diff between `self` (current) and `other` (previous).
    ///
    /// Returns `(x, y, cell)` tuples for every cell that changed. Useful for
    /// custom backends or tests that need to inspect changed cells directly.
    ///
    /// # Allocation
    ///
    /// Allocates a new [`Vec`] on every call. For high-frequency use
    /// (per-frame diffing in a render loop), prefer the internal
    /// `flush_buffer_diff` path used by [`crate::run`], which streams updates
    /// directly to the backend without an intermediate `Vec`. Calling
    /// `diff()` on every frame in a 60 fps loop adds one heap allocation
    /// (sized to the changed-cell count) per frame.
    ///
    /// # Benchmarks
    ///
    /// `benches/benchmarks.rs` exercises this path in `bench_buffer_diff`.
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
        // Issue #171: every row is now blank — flag them all dirty so the
        // next flush refreshes the digest before any skip check.
        for d in &mut self.line_dirty {
            *d = true;
        }
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
        // Issue #171: every cell was just rewritten — mark all rows dirty.
        for d in &mut self.line_dirty {
            *d = true;
        }
    }

    /// Resize the buffer to fit a new area, resetting all cells.
    ///
    /// If the new area is larger, new cells are initialized to blank. All
    /// existing content is discarded.
    pub fn resize(&mut self, area: Rect) {
        self.area = area;
        let size = area.area() as usize;
        self.content.resize(size, Cell::default());
        // Issue #171: keep the per-row tracking arrays sized to the new
        // height. `reset()` re-marks every row dirty so initial values
        // here don't affect correctness.
        let height = area.height as usize;
        self.line_hashes.resize(height, 0);
        self.line_dirty.resize(height, true);
        self.reset();
    }

    /// Serialize the buffer into a stable, styled-snapshot format suitable for
    /// snapshot testing (e.g. with `insta::assert_snapshot!`).
    ///
    /// # Format
    ///
    /// One line per buffer row, joined with `\n`. Within a row, runs of cells
    /// that share an identical [`Style`] are grouped. The default style (no
    /// foreground, no background, no modifiers) emits **unannotated** text —
    /// no `[...]` markers. Any non-default run is wrapped:
    ///
    /// ```text
    /// [fg=...,bg=...,mods]"text"[/]
    /// ```
    ///
    /// Trailing whitespace per row is preserved in the styled segment but
    /// trailing default-style spaces at the end of a row are emitted verbatim
    /// (they are visually invisible in diffs). Empty cells render as a single
    /// space. The terminating `[/]` marker only appears when a styled run is
    /// in effect at the end of a row.
    ///
    /// # Color formatting
    ///
    /// Named palette colors use short lowercase codes:
    /// `reset`, `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`,
    /// `white`, `dark_gray`, `light_red`, `light_green`, `light_yellow`,
    /// `light_blue`, `light_magenta`, `light_cyan`, `light_white`. RGB colors
    /// emit `#rrggbb`. Indexed palette colors emit `idx<N>` (decimal).
    ///
    /// # Modifier formatting
    ///
    /// Modifiers are emitted as comma-separated lowercase tokens in a fixed
    /// canonical order: `bold`, `dim`, `italic`, `underline`, `reversed`,
    /// `strikethrough`. Order is independent of the bit pattern, so two
    /// equivalent `Modifiers` values always serialize identically.
    ///
    /// # Stability
    ///
    /// The output format is stable across patch and minor versions of SLT.
    /// Names use a hand-rolled formatter (not `Debug`) so derives changing
    /// upstream cannot accidentally break locked snapshots. A breaking change
    /// to the format would be reserved for a major version bump.
    ///
    /// # Determinism
    ///
    /// Identical input buffers always produce byte-equal output. This is a
    /// hard requirement — snapshot tests rely on it.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::{Buffer, Color, Rect, Style};
    ///
    /// let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
    /// buf.set_string(0, 0, "ab", Style::new().fg(Color::Red).bold());
    /// buf.set_string(2, 0, "cd", Style::new());
    /// let snap = buf.snapshot_format();
    /// assert!(snap.starts_with("[fg=red,bold]\"ab\"[/]cd"));
    /// ```
    pub fn snapshot_format(&self) -> String {
        let mut out = String::new();
        let width = self.area.width;
        let height = self.area.height;
        if width == 0 || height == 0 {
            return out;
        }

        for y in self.area.y..self.area.bottom() {
            if y > self.area.y {
                out.push('\n');
            }

            // Walk the row, grouping consecutive cells by Style.
            let mut current_style: Option<Style> = None;
            let mut run_text = String::new();

            for x in self.area.x..self.area.right() {
                let cell = self.get(x, y);
                let style = cell.style;
                // Empty cell symbol → single space (e.g. trailing wide-char cell).
                let sym: &str = if cell.symbol.is_empty() {
                    " "
                } else {
                    cell.symbol.as_str()
                };

                match current_style {
                    Some(s) if s == style => {
                        run_text.push_str(sym);
                    }
                    _ => {
                        if let Some(s) = current_style.take() {
                            flush_run(&mut out, s, &run_text);
                            run_text.clear();
                        }
                        current_style = Some(style);
                        run_text.push_str(sym);
                    }
                }
            }

            if let Some(s) = current_style {
                flush_run(&mut out, s, &run_text);
            }
        }

        out
    }
}

/// Flush a single style-run into the snapshot output.
///
/// Default style → unannotated raw text (no markers, escape only embedded `"`).
/// Non-default style → `[fg=...,bg=...,mods]"text"[/]` form. Embedded `"` and
/// `\` characters in cell symbols are escaped so the snapshot remains
/// unambiguous.
fn flush_run(out: &mut String, style: Style, text: &str) {
    if style == Style::default() {
        out.push_str(text);
        return;
    }
    out.push('[');
    let mut first = true;
    if let Some(fg) = style.fg {
        out.push_str("fg=");
        write_color(out, fg);
        first = false;
    }
    if let Some(bg) = style.bg {
        if !first {
            out.push(',');
        }
        out.push_str("bg=");
        write_color(out, bg);
        first = false;
    }
    let mods = style.modifiers;
    // Canonical order: bold, dim, italic, underline, reversed, strikethrough.
    let pairs: [(crate::style::Modifiers, &str); 6] = [
        (crate::style::Modifiers::BOLD, "bold"),
        (crate::style::Modifiers::DIM, "dim"),
        (crate::style::Modifiers::ITALIC, "italic"),
        (crate::style::Modifiers::UNDERLINE, "underline"),
        (crate::style::Modifiers::REVERSED, "reversed"),
        (crate::style::Modifiers::STRIKETHROUGH, "strikethrough"),
    ];
    for (bit, name) in pairs {
        if mods.contains(bit) {
            if !first {
                out.push(',');
            }
            out.push_str(name);
            first = false;
        }
    }
    out.push(']');
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
    out.push('"');
    out.push_str("[/]");
}

/// Format a [`crate::style::Color`] using the stable snapshot vocabulary.
///
/// Hand-rolled instead of `Debug` so upstream derive changes can't silently
/// break snapshot stability.
fn write_color(out: &mut String, color: crate::style::Color) {
    use crate::style::Color;
    match color {
        Color::Reset => out.push_str("reset"),
        Color::Black => out.push_str("black"),
        Color::Red => out.push_str("red"),
        Color::Green => out.push_str("green"),
        Color::Yellow => out.push_str("yellow"),
        Color::Blue => out.push_str("blue"),
        Color::Magenta => out.push_str("magenta"),
        Color::Cyan => out.push_str("cyan"),
        Color::White => out.push_str("white"),
        Color::DarkGray => out.push_str("dark_gray"),
        Color::LightRed => out.push_str("light_red"),
        Color::LightGreen => out.push_str("light_green"),
        Color::LightYellow => out.push_str("light_yellow"),
        Color::LightBlue => out.push_str("light_blue"),
        Color::LightMagenta => out.push_str("light_magenta"),
        Color::LightCyan => out.push_str("light_cyan"),
        Color::LightWhite => out.push_str("light_white"),
        Color::Rgb(r, g, b) => {
            use std::fmt::Write;
            let _ = write!(out, "#{:02x}{:02x}{:02x}", r, g, b);
        }
        Color::Indexed(idx) => {
            use std::fmt::Write;
            let _ = write!(out, "idx{}", idx);
        }
    }
}

/// Maximum byte length for OSC 8 hyperlink URLs.
///
/// Longer than any legitimate URL and enough to prevent DoS via
/// balloon-sized hyperlinks. Shared by [`is_valid_osc8_url`] and
/// [`sanitize_osc8_url`] so both gates agree on acceptance.
const MAX_OSC8_URL_BYTES: usize = 2048;

/// Returns `true` if `url` is safe to emit as an OSC 8 hyperlink payload.
///
/// Equivalent to `sanitize_osc8_url(url).is_some()` but avoids the `String`
/// allocation when callers only need a boolean validity check (e.g.,
/// defense-in-depth validation of a public `Cell::hyperlink` field on the
/// flush path).
#[inline]
pub(crate) fn is_valid_osc8_url(url: &str) -> bool {
    if url.is_empty() || url.len() > MAX_OSC8_URL_BYTES {
        return false;
    }
    // Reject all C0 controls (incl. BEL 0x07, ESC 0x1b), DEL 0x7f, and
    // anything below 0x20. ESC enables the ST (ESC \) terminator trick;
    // BEL is the legacy OSC terminator. Either would let an
    // attacker-controlled URL prematurely close the OSC 8 sequence and
    // inject arbitrary follow-up commands (e.g., OSC 52 clipboard writes).
    url.bytes().all(|b| b >= 0x20 && b != 0x7f)
}

/// Validate an OSC 8 hyperlink URL, returning `Some(url)` if safe to emit.
///
/// Rejects URLs containing control bytes, the BEL terminator, or an
/// embedded ST (`ESC \`). Those would let an attacker-controlled URL
/// prematurely close the OSC 8 sequence and inject arbitrary follow-up
/// commands (e.g., OSC 52 clipboard writes). Also caps length at
/// [`MAX_OSC8_URL_BYTES`] (2048).
///
/// For boolean validation (no allocation), use [`is_valid_osc8_url`].
pub(crate) fn sanitize_osc8_url(url: &str) -> Option<String> {
    if is_valid_osc8_url(url) {
        Some(url.to_string())
    } else {
        None
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
    fn is_valid_osc8_url_matches_sanitize() {
        // is_valid_osc8_url must agree with sanitize_osc8_url on every input.
        // If the two ever drift, the OSC 8 flush path either rejects
        // legitimate URLs (silent) or admits dangerous ones (security).
        let oversize = "x".repeat(2049);
        let cases: &[&str] = &[
            "https://example.com",
            "http://localhost:8080/path?q=1#frag",
            "ftp://[::1]/file",
            "",
            &oversize,
            "https://evil.com\x1b]52;c;inject\x1b\\",
            "https://evil.com\x07bel",
            "https://example.com\x7f",
            "https://example.com\x00",
        ];
        for url in cases {
            assert_eq!(
                is_valid_osc8_url(url),
                sanitize_osc8_url(url).is_some(),
                "is_valid_osc8_url and sanitize_osc8_url disagree on {url:?}"
            );
        }
    }

    #[test]
    fn set_string_inner_parity_no_link() {
        // set_string and set_string_linked with an invalid URL must produce
        // identical buffer state (link rejected → None).
        let area = Rect::new(0, 0, 20, 1);
        let mut buf_a = Buffer::empty(area);
        let mut buf_b = Buffer::empty(area);
        let style = Style::new();

        buf_a.set_string(0, 0, "Hello wide世界", style);
        buf_b.set_string_linked(0, 0, "Hello wide世界", style, "");

        for x in 0..20 {
            let ca = buf_a.get(x, 0);
            let cb = buf_b.get(x, 0);
            assert_eq!(ca.symbol, cb.symbol, "symbol mismatch at x={x}");
            assert_eq!(ca.style, cb.style, "style mismatch at x={x}");
            assert_eq!(
                cb.hyperlink, None,
                "invalid URL must produce None hyperlink at x={x}"
            );
        }
    }

    #[test]
    fn set_string_linked_attaches_hyperlink_to_wide_char_pair() {
        // Wide chars span two cells; both must carry the same hyperlink.
        let area = Rect::new(0, 0, 4, 1);
        let mut buf = Buffer::empty(area);
        buf.set_string_linked(0, 0, "世", Style::new(), "https://example.com");
        let leading = buf.get(0, 0);
        let trailing = buf.get(1, 0);
        assert_eq!(leading.symbol, "世");
        assert!(trailing.symbol.is_empty(), "wide-char trailing must blank");
        assert!(leading.hyperlink.is_some());
        assert_eq!(leading.hyperlink, trailing.hyperlink);
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

    // ---- snapshot_format tests (#231) -------------------------------------

    #[test]
    fn snapshot_format_default_style_unannotated() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 1));
        buf.set_string(0, 0, "abc", Style::new());
        // Two trailing default cells render as raw spaces.
        assert_eq!(buf.snapshot_format(), "abc  ");
    }

    #[test]
    fn snapshot_format_color_runs_grouped() {
        use crate::style::Color;
        let mut buf = Buffer::empty(Rect::new(0, 0, 6, 1));
        buf.set_string(0, 0, "abc", Style::new().fg(Color::Red));
        buf.set_string(3, 0, "def", Style::new().fg(Color::Blue));
        let snap = buf.snapshot_format();
        assert_eq!(snap, "[fg=red]\"abc\"[/][fg=blue]\"def\"[/]");
    }

    #[test]
    fn snapshot_format_modifier_transitions() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 6, 1));
        buf.set_string(0, 0, "ab", Style::new().bold());
        // gap with default style
        buf.set_string(2, 0, "cd", Style::new());
        buf.set_string(4, 0, "ef", Style::new().bold());
        let snap = buf.snapshot_format();
        assert_eq!(snap, "[bold]\"ab\"[/]cd[bold]\"ef\"[/]");
    }

    #[test]
    fn snapshot_format_deterministic() {
        use crate::style::Color;
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 2));
        buf.set_string(0, 0, "hello", Style::new().fg(Color::Cyan).bold());
        buf.set_string(0, 1, "world", Style::new().bg(Color::Rgb(10, 20, 30)));
        let a = buf.snapshot_format();
        let b = buf.snapshot_format();
        assert_eq!(a, b, "snapshot_format must be deterministic");
        // Verify byte length equality as a stronger anti-flake guarantee.
        assert_eq!(a.len(), b.len());
    }

    #[test]
    fn snapshot_format_empty_buffer_is_spaces() {
        let buf = Buffer::empty(Rect::new(0, 0, 4, 2));
        // 4 default-style spaces per row, joined by '\n'.
        assert_eq!(buf.snapshot_format(), "    \n    ");
    }

    #[test]
    fn snapshot_format_zero_dim_returns_empty() {
        let buf_a = Buffer::empty(Rect::new(0, 0, 0, 4));
        let buf_b = Buffer::empty(Rect::new(0, 0, 4, 0));
        assert_eq!(buf_a.snapshot_format(), "");
        assert_eq!(buf_b.snapshot_format(), "");
    }

    #[test]
    fn snapshot_format_rgb_uses_hex_codes() {
        use crate::style::Color;
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        buf.set_string(0, 0, "x", Style::new().fg(Color::Rgb(0xff, 0x00, 0xab)));
        let snap = buf.snapshot_format();
        assert!(
            snap.contains("fg=#ff00ab"),
            "expected hex RGB code, got {snap:?}"
        );
    }

    #[test]
    fn snapshot_format_indexed_color() {
        use crate::style::Color;
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        buf.set_string(0, 0, "x", Style::new().fg(Color::Indexed(42)));
        assert!(buf.snapshot_format().contains("fg=idx42"));
    }

    #[test]
    fn snapshot_format_modifiers_canonical_order() {
        // Insert in reverse order; output must still be canonical.
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        let style = Style::new().strikethrough().italic().bold();
        buf.set_string(0, 0, "x", style);
        let snap = buf.snapshot_format();
        // Order in output: bold, italic, strikethrough.
        let bold_idx = snap.find("bold").expect("bold present");
        let italic_idx = snap.find("italic").expect("italic present");
        let strike_idx = snap.find("strikethrough").expect("strikethrough present");
        assert!(bold_idx < italic_idx);
        assert!(italic_idx < strike_idx);
    }

    #[test]
    fn snapshot_format_escapes_quote_and_backslash() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        buf.set_string(0, 0, "a\"b\\", Style::new().bold());
        let snap = buf.snapshot_format();
        // Embedded quote → \" and backslash → \\
        assert!(
            snap.contains("\"a\\\"b\\\\\""),
            "expected escapes, got {snap:?}"
        );
    }

    #[test]
    fn snapshot_format_multi_row_uses_newlines() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 3, 3));
        buf.set_string(0, 0, "aaa", Style::new());
        buf.set_string(0, 1, "bbb", Style::new());
        buf.set_string(0, 2, "ccc", Style::new());
        assert_eq!(buf.snapshot_format(), "aaa\nbbb\nccc");
    }

    // ---- per-row hash skip (#171) -----------------------------------------

    #[test]
    fn line_dirty_initial_state_is_all_dirty() {
        // Fresh buffer must start with every row dirty so the first flush
        // refreshes hashes before the per-row skip ever fires.
        let buf = Buffer::empty(Rect::new(0, 0, 4, 3));
        assert_eq!(buf.line_dirty.len(), 3);
        assert!(buf.line_dirty.iter().all(|d| *d));
    }

    #[test]
    fn set_string_marks_row_dirty() {
        // After a recompute every row is clean. A subsequent write must
        // re-mark the touched row as dirty so its hash gets refreshed.
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 4));
        buf.recompute_line_hashes();
        assert!(buf.line_dirty.iter().all(|d| !*d));

        buf.set_string(0, 1, "hello", Style::new());
        assert!(!buf.line_dirty[0]);
        assert!(buf.line_dirty[1]);
        assert!(!buf.line_dirty[2]);
        assert!(!buf.line_dirty[3]);
    }

    #[test]
    fn set_char_marks_row_dirty() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 3));
        buf.recompute_line_hashes();
        buf.set_char(2, 2, 'X', Style::new());
        assert!(!buf.line_dirty[0]);
        assert!(!buf.line_dirty[1]);
        assert!(buf.line_dirty[2]);
    }

    #[test]
    fn recompute_line_hashes_clears_dirty_and_caches_hashes() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 2));
        buf.set_string(0, 0, "abcd", Style::new());
        buf.set_string(0, 1, "wxyz", Style::new());
        buf.recompute_line_hashes();

        assert!(buf.line_dirty.iter().all(|d| !*d));
        // Different content → different hashes.
        assert_ne!(buf.line_hashes[0], buf.line_hashes[1]);
        assert!(buf.row_clean(0));
        assert!(buf.row_clean(1));
    }

    #[test]
    fn row_clean_returns_false_for_unrecomputed_or_dirty_row() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 2));
        // Initial state — every row dirty until recompute.
        assert!(!buf.row_clean(0));
        buf.recompute_line_hashes();
        assert!(buf.row_clean(0));
        // Touching the row re-marks it dirty.
        buf.set_string(0, 0, "z", Style::new());
        assert!(!buf.row_clean(0));
    }

    #[test]
    fn identical_buffers_share_line_hashes_after_recompute() {
        // Foundation of the flush short-circuit: two buffers with the same
        // cells must produce equal per-row digests.
        let area = Rect::new(0, 0, 5, 3);
        let mut a = Buffer::empty(area);
        let mut b = Buffer::empty(area);
        a.set_string(0, 0, "hello", Style::new());
        b.set_string(0, 0, "hello", Style::new());
        a.set_string(0, 1, "world", Style::new());
        b.set_string(0, 1, "world", Style::new());
        a.recompute_line_hashes();
        b.recompute_line_hashes();

        assert_eq!(a.row_hash(0), b.row_hash(0));
        assert_eq!(a.row_hash(1), b.row_hash(1));
        // Untouched row 2 — both buffers have it as default-cell row.
        assert_eq!(a.row_hash(2), b.row_hash(2));
    }

    #[test]
    fn different_styles_yield_different_line_hashes() {
        // Identical glyph but different style must still hash distinctly —
        // the flush would otherwise emit the wrong style if it skipped a
        // "matching" row.
        use crate::style::Color;
        let area = Rect::new(0, 0, 3, 1);
        let mut a = Buffer::empty(area);
        let mut b = Buffer::empty(area);
        a.set_string(0, 0, "abc", Style::new().fg(Color::Red));
        b.set_string(0, 0, "abc", Style::new().fg(Color::Blue));
        a.recompute_line_hashes();
        b.recompute_line_hashes();

        assert_ne!(a.row_hash(0), b.row_hash(0));
    }

    #[test]
    fn resize_keeps_line_arrays_in_sync() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 3));
        buf.recompute_line_hashes();
        // Grow → all rows dirty + arrays sized to new height.
        buf.resize(Rect::new(0, 0, 4, 5));
        assert_eq!(buf.line_dirty.len(), 5);
        assert_eq!(buf.line_hashes.len(), 5);
        assert!(buf.line_dirty.iter().all(|d| *d));
        // Shrink — same invariants.
        buf.resize(Rect::new(0, 0, 4, 2));
        assert_eq!(buf.line_dirty.len(), 2);
        assert_eq!(buf.line_hashes.len(), 2);
        assert!(buf.line_dirty.iter().all(|d| *d));
    }

    // ── Bidi (UAX #9) reordering ────────────────────────────────────────
    //
    // `line_visual` reads a buffer row left-to-right by column and trims
    // trailing blanks — exactly the visual order a reader sees, which is the
    // correct oracle for asserting reorder output.
    #[cfg(feature = "bidi")]
    fn line_visual(buf: &Buffer, y: u32) -> String {
        let mut s = String::new();
        for x in buf.area.x..buf.area.right() {
            let sym = buf.get(x, y).symbol.as_str();
            if sym.is_empty() {
                continue; // wide-char trailing cell
            }
            s.push_str(sym);
        }
        s.trim_end().to_string()
    }

    #[cfg(feature = "bidi")]
    #[test]
    fn needs_bidi_reorder_false_for_pure_ltr() {
        // Pure-LTR strings take the zero-allocation fast path.
        assert!(!needs_bidi_reorder("Hello, world 123"));
        assert!(!needs_bidi_reorder(""));
        assert!(!needs_bidi_reorder("café résumé"));
        assert!(!needs_bidi_reorder("世界 CJK wide"));
    }

    #[cfg(feature = "bidi")]
    #[test]
    fn needs_bidi_reorder_true_for_rtl_and_controls() {
        assert!(needs_bidi_reorder("שלום")); // Hebrew
        assert!(needs_bidi_reorder("شكرا")); // Arabic
        assert!(needs_bidi_reorder("abc אבג def")); // mixed
        assert!(needs_bidi_reorder("a\u{202E}bc")); // RLO control
        assert!(needs_bidi_reorder("\u{200F}")); // RLM
    }

    #[cfg(feature = "bidi")]
    #[test]
    fn set_string_ltr_unchanged_by_reorder_path() {
        // Regression guard: LTR text must NOT be reordered.
        let mut buf = Buffer::empty(Rect::new(0, 0, 6, 1));
        buf.set_string(0, 0, "abcde", Style::new());
        assert_eq!(buf.get(0, 0).symbol, "a");
        assert_eq!(buf.get(1, 0).symbol, "b");
        assert_eq!(buf.get(2, 0).symbol, "c");
        assert_eq!(buf.get(3, 0).symbol, "d");
        assert_eq!(buf.get(4, 0).symbol, "e");
    }

    #[cfg(feature = "bidi")]
    #[test]
    fn set_string_pure_rtl_reverses_to_visual_order() {
        // Hebrew "שלום" is logical ש,ל,ו,ם. In visual order the first
        // logical char (ש) lands on the rightmost column and the last (ם)
        // on the leftmost — i.e. the row reads "םולש" left-to-right.
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        buf.set_string(0, 0, "\u{05E9}\u{05DC}\u{05D5}\u{05DD}", Style::new());
        // column 0 == last logical char, last column == first logical char
        assert_eq!(buf.get(0, 0).symbol, "\u{05DD}"); // ם
        assert_eq!(buf.get(3, 0).symbol, "\u{05E9}"); // ש
        assert_eq!(line_visual(&buf, 0), "\u{05DD}\u{05D5}\u{05DC}\u{05E9}");
    }

    #[cfg(feature = "bidi")]
    #[test]
    fn set_string_mixed_ltr_rtl_run() {
        // Per UAX #9 (unicode-bidi reference vectors): "abc אבג" → "abc גבא".
        // The Latin segment keeps LTR order; the Hebrew segment reverses.
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
        buf.set_string(0, 0, "abc \u{05D0}\u{05D1}\u{05D2}", Style::new());
        assert_eq!(line_visual(&buf, 0), "abc \u{05D2}\u{05D1}\u{05D0}");
    }

    #[cfg(feature = "bidi")]
    #[test]
    fn set_string_numbers_inside_rtl_stay_ltr() {
        // "123 אבג" → "גבא 123": European numbers are weak LTR and cannot
        // reorder a strong RTL run, so the digits stay "123" left-to-right
        // while the Hebrew reverses (unicode-bidi reference vector).
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
        buf.set_string(0, 0, "123 \u{05D0}\u{05D1}\u{05D2}", Style::new());
        assert_eq!(line_visual(&buf, 0), "\u{05D2}\u{05D1}\u{05D0} 123");
    }

    #[cfg(feature = "bidi")]
    #[test]
    fn set_string_wide_char_with_rtl_blanks_trailing_cell() {
        // A CJK wide glyph mixed with Hebrew: after reorder the wide char's
        // trailing cell must still be blanked at the correct visual column.
        // Logical "世 אב" → the wide 世 stays leftmost (LTR base), Hebrew
        // reverses to "בא". Visual: 世 (cols 0-1), space (col 2), ב (3) א (4).
        let mut buf = Buffer::empty(Rect::new(0, 0, 6, 1));
        buf.set_string(0, 0, "\u{4E16} \u{05D0}\u{05D1}", Style::new());
        assert_eq!(buf.get(0, 0).symbol, "\u{4E16}"); // 世 leading
        assert!(buf.get(1, 0).symbol.is_empty(), "wide trailing must blank");
        assert_eq!(buf.get(3, 0).symbol, "\u{05D1}"); // ב
        assert_eq!(buf.get(4, 0).symbol, "\u{05D0}"); // א
    }

    #[cfg(feature = "bidi")]
    #[test]
    fn set_string_linked_hyperlink_survives_reorder() {
        // Every non-blank emitted cell of an RTL link must carry the URL,
        // regardless of its new visual column.
        let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
        buf.set_string_linked(
            0,
            0,
            "\u{05E9}\u{05DC}\u{05D5}\u{05DD}",
            Style::new(),
            "https://example.com",
        );
        for x in 0..4 {
            let cell = buf.get(x, 0);
            assert!(
                cell.hyperlink.is_some(),
                "hyperlink missing at visual column {x}"
            );
        }
    }

    #[cfg(feature = "bidi")]
    #[test]
    fn set_string_control_chars_filtered_in_rtl() {
        // An ESC embedded in an RTL string must still be replaced with
        // U+FFFD — the reorder path must not bypass sanitize_cell_char.
        let mut buf = Buffer::empty(Rect::new(0, 0, 6, 1));
        buf.set_string(0, 0, "\u{05D0}\x1b\u{05D1}", Style::new());
        let mut found_replacement = false;
        for x in 0..6 {
            let sym = buf.get(x, 0).symbol.as_str();
            assert!(!sym.contains('\x1b'), "ESC leaked into a cell");
            if sym.contains('\u{FFFD}') {
                found_replacement = true;
            }
        }
        assert!(found_replacement, "ESC was not replaced with U+FFFD");
    }

    #[cfg(feature = "bidi")]
    #[test]
    fn reorder_line_visual_empty_is_noop() {
        assert_eq!(reorder_line_visual(""), "");
    }

    #[cfg(feature = "bidi")]
    mod bidi_proptest {
        use super::{needs_bidi_reorder, reorder_line_visual};
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(256))]

            /// Fast-path no-op: arbitrary ASCII strings never need reordering.
            #[test]
            fn ascii_takes_fast_path_and_reorder_is_identity(s in "[ -~]{0,64}") {
                prop_assert!(!needs_bidi_reorder(&s));
                // Even if forced through the reorder, ASCII is a no-op permutation.
                prop_assert_eq!(reorder_line_visual(&s), s);
            }

            /// Reorder is a permutation, not a width change: the total display
            /// width of the reordered line equals the original's width.
            #[test]
            fn reorder_preserves_total_display_width(
                s in "[a-z\\x{05D0}-\\x{05EA}\\x{0627}-\\x{064A}0-9 ]{0,48}"
            ) {
                use unicode_width::UnicodeWidthStr;
                let before = UnicodeWidthStr::width(s.as_str());
                let after = UnicodeWidthStr::width(reorder_line_visual(&s).as_str());
                prop_assert_eq!(before, after);
            }
        }
    }
}
