//! Single terminal cell — the smallest unit of the render buffer.

use crate::style::Style;

/// A single terminal cell containing a character and style.
///
/// Each cell holds one grapheme cluster (stored as a `String` to support
/// multi-byte Unicode) and the [`Style`] to render it with. Wide characters
/// (e.g., CJK) occupy two adjacent cells; the second cell's `symbol` is left
/// empty by the buffer layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The grapheme cluster displayed in this cell. Defaults to a single space.
    pub symbol: String,
    /// The visual style (colors and modifiers) for this cell.
    pub style: Style,
    /// Optional OSC 8 hyperlink URL. When set, the terminal renders this cell
    /// as a clickable link.
    pub hyperlink: Option<String>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            symbol: " ".into(),
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
