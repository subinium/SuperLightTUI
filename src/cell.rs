//! Single terminal cell — the smallest unit of the render buffer.

use compact_str::CompactString;

use crate::style::Style;

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
/// Wide characters (e.g., CJK) occupy two adjacent cells; the second cell's
/// `symbol` is left empty by the buffer layer.
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
    pub fn set_symbol(&mut self, s: &str) -> &mut Self {
        self.symbol.clear();
        self.symbol.push_str(s);
        self
    }

    /// Replace the cell's symbol with a single character.
    pub fn set_char(&mut self, ch: char) -> &mut Self {
        self.symbol.clear();
        self.symbol.push(ch);
        self
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
}
