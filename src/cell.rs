//! Single terminal cell — the smallest unit of the render buffer.

use compact_str::CompactString;
use unicode_segmentation::UnicodeSegmentation;

use crate::style::Style;

/// Maximum UTF-8 bytes retained for one terminal grapheme.
///
/// This bounds output work for pathological combining-mark sequences while
/// leaving enough room for common ZWJ emoji clusters.
pub(crate) const MAX_CELL_SYMBOL_BYTES: usize = 32;

/// Replace terminal control code points with the visible replacement glyph.
#[inline]
pub(crate) fn sanitize_cell_char(ch: char) -> char {
    let value = ch as u32;
    if value < 0x20 || value == 0x7f || (0x80..=0x9f).contains(&value) {
        '\u{FFFD}'
    } else {
        ch
    }
}

/// Normalize an arbitrary string into one bounded, terminal-safe grapheme.
///
/// An empty result is reserved for continuation cells. Additional graphemes
/// are discarded because a `Cell` represents exactly one display atom.
pub(crate) fn normalize_cell_symbol(symbol: &str) -> CompactString {
    let Some(grapheme) = symbol.graphemes(true).next() else {
        return CompactString::new("");
    };

    let mut normalized = CompactString::new("");
    for ch in grapheme.chars() {
        let ch = sanitize_cell_char(ch);
        if normalized.len().saturating_add(ch.len_utf8()) > MAX_CELL_SYMBOL_BYTES {
            break;
        }
        normalized.push(ch);
    }
    normalized
}

// Compile-time size assertion for `Cell`.
//
// `Cell` is composed of `symbol: CompactString` + `style: Style` +
// `hyperlink: Option<CompactString>`. Upstream changes to any of these
// (e.g., `CompactString` inline-storage tweaks, `Style` field additions,
// or hyperlink type swaps) can silently grow the struct until runtime.
// A 64-byte budget keeps each cell within one cache line.
//
// If an intentional growth pushes us past 64 B, raise this bound and
// document why — but do not silently let it drift.
const _: () = assert!(
    std::mem::size_of::<Cell>() <= 64,
    "Cell exceeds one cache line (64 B). If the size increase is intentional, update this bound and document why."
);

/// A single terminal cell containing a character and style.
///
/// Each cell holds one grapheme cluster (stored as a [`CompactString`] for
/// inline storage of short strings — no heap allocation for ≤24 bytes).
/// Wide graphemes occupy adjacent cells. The leading cell stores the
/// grapheme and every continuation cell has an empty `symbol`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The grapheme cluster displayed in this cell. Defaults to a single space.
    pub symbol: CompactString,
    /// The visual style (colors and modifiers) for this cell.
    pub style: Style,
    /// Optional OSC 8 hyperlink URL. When set, the terminal renders this cell
    /// as a clickable link.
    pub hyperlink: Option<CompactString>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            symbol: CompactString::const_new(" "),
            style: Style::new(),
            hyperlink: None,
        }
    }
}

impl Cell {
    /// Replace the cell's symbol with the given string slice.
    ///
    /// Only the first extended grapheme cluster is retained. C0, DEL, and C1
    /// controls are replaced with `U+FFFD`, and the optional hyperlink is
    /// cleared so direct symbol replacement cannot inherit stale link state.
    pub fn set_symbol(&mut self, s: &str) -> &mut Self {
        self.symbol = normalize_cell_symbol(s);
        self.hyperlink = None;
        self
    }

    /// Replace the cell's symbol with a single character.
    pub fn set_char(&mut self, ch: char) -> &mut Self {
        self.symbol.clear();
        self.symbol.push(sanitize_cell_char(ch));
        self.hyperlink = None;
        self
    }

    /// Return whether this cell continues a grapheme stored in a prior cell.
    ///
    /// Empty symbols are reserved as continuation metadata; ordinary blank
    /// cells contain a single space.
    #[inline]
    pub fn is_continuation(&self) -> bool {
        self.symbol.is_empty()
    }

    /// Mark this cell as a continuation of a preceding wide grapheme.
    pub(crate) fn set_continuation(&mut self, style: Style) -> &mut Self {
        self.symbol.clear();
        self.style = style;
        self.hyperlink = None;
        self
    }

    /// Return a defensively normalized symbol for terminal output.
    ///
    /// This is required at the flush boundary because `symbol` remains public
    /// for compatibility and callers can mutate it without using the setters.
    pub(crate) fn normalized_symbol(&self) -> CompactString {
        normalize_cell_symbol(&self.symbol)
    }

    /// Set the cell's style.
    pub fn set_style(&mut self, style: Style) -> &mut Self {
        self.style = style;
        self
    }

    /// Reset the cell to a blank space with default style.
    pub fn reset(&mut self) {
        self.symbol.clear();
        self.symbol.push(' ');
        self.style = Style::new();
        self.hyperlink = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_size_within_cache_line() {
        let size = std::mem::size_of::<Cell>();
        assert!(
            size <= 64,
            "Cell size = {size}B; exceeds 64B cache-line budget. If intentional, update the const-assert and this test together."
        );
    }

    #[test]
    fn setters_keep_one_safe_grapheme_and_clear_links() {
        let mut cell = Cell::default();
        cell.hyperlink = Some(CompactString::new("https://example.com"));
        cell.set_symbol("👩‍💻tail\x1b");

        assert_eq!(cell.symbol, "👩‍💻");
        assert!(cell.hyperlink.is_none());

        cell.set_char('\x1b');
        assert_eq!(cell.symbol, "\u{FFFD}");
    }

    #[test]
    fn empty_symbol_is_explicit_continuation_state() {
        let mut cell = Cell::default();
        assert!(!cell.is_continuation());
        cell.set_continuation(Style::new());
        assert!(cell.is_continuation());
    }

    #[test]
    fn normalized_symbol_defends_against_direct_public_mutation() {
        let mut cell = Cell::default();
        cell.symbol = CompactString::new("\x1b]52;c;payload");
        assert_eq!(cell.normalized_symbol(), "\u{FFFD}");
    }
}
