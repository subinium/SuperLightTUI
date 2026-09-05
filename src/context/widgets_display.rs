//! Display widgets — text, alerts, badges, gauges, code blocks,
//! breadcrumbs, gutters, splits, and the `bordered`/`col`/`row` layout
//! shortcuts.
//!
//! These are Layer 3 widgets that produce visual output without
//! consuming events. For event-consuming widgets see
//! [`super::widgets_interactive`]; for input widgets see
//! [`super::widgets_input`].

use super::*;

mod gauge;
mod gutter;
mod layout;
mod rich_output;
mod split;
mod status;
mod text;

pub use gauge::{Gauge, LineGauge};
pub use gutter::GutterOpts;
pub use layout::Anchor;
pub use status::{Breadcrumb, CodeBlock};

#[cfg(test)]
mod line_wrap_tests;
#[cfg(test)]
mod tests;

pub(super) fn wrap_tooltip_text(text: &str, max_width: usize) -> Vec<String> {
    let max_width = max_width.max(1);
    let mut lines = Vec::new();

    for paragraph in text.lines() {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current = String::new();
        let mut current_width = 0usize;

        for word in paragraph.split_whitespace() {
            for chunk in split_word_for_width(word, max_width) {
                let chunk_width = UnicodeWidthStr::width(chunk.as_str());

                if current.is_empty() {
                    current = chunk;
                    current_width = chunk_width;
                    continue;
                }

                if current_width + 1 + chunk_width <= max_width {
                    current.push(' ');
                    current.push_str(&chunk);
                    current_width += 1 + chunk_width;
                } else {
                    lines.push(std::mem::take(&mut current));
                    current = chunk;
                    current_width = chunk_width;
                }
            }
        }

        if !current.is_empty() {
            lines.push(current);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn split_word_for_width(word: &str, max_width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for ch in word.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if !current.is_empty() && current_width + ch_width > max_width {
            chunks.push(std::mem::take(&mut current));
            current_width = 0;
        }
        current.push(ch);
        current_width += ch_width;

        if current_width >= max_width {
            chunks.push(std::mem::take(&mut current));
            current_width = 0;
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    if chunks.is_empty() {
        chunks.push(String::new());
    }

    chunks
}

fn glyph_8x8(ch: char) -> [u8; 8] {
    if ch.is_ascii() {
        let code = ch as u8;
        if (32..=126).contains(&code) {
            return FONT_8X8_PRINTABLE[(code - 32) as usize];
        }
    }

    FONT_8X8_PRINTABLE[(b'?' - 32) as usize]
}

const FONT_8X8_PRINTABLE: [[u8; 8]; 95] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x18, 0x3C, 0x3C, 0x18, 0x18, 0x00, 0x18, 0x00],
    [0x36, 0x36, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x36, 0x36, 0x7F, 0x36, 0x7F, 0x36, 0x36, 0x00],
    [0x0C, 0x3E, 0x03, 0x1E, 0x30, 0x1F, 0x0C, 0x00],
    [0x00, 0x63, 0x33, 0x18, 0x0C, 0x66, 0x63, 0x00],
    [0x1C, 0x36, 0x1C, 0x6E, 0x3B, 0x33, 0x6E, 0x00],
    [0x06, 0x06, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x18, 0x0C, 0x06, 0x06, 0x06, 0x0C, 0x18, 0x00],
    [0x06, 0x0C, 0x18, 0x18, 0x18, 0x0C, 0x06, 0x00],
    [0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00],
    [0x00, 0x0C, 0x0C, 0x3F, 0x0C, 0x0C, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x06],
    [0x00, 0x00, 0x00, 0x3F, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x00],
    [0x60, 0x30, 0x18, 0x0C, 0x06, 0x03, 0x01, 0x00],
    [0x3E, 0x63, 0x73, 0x7B, 0x6F, 0x67, 0x3E, 0x00],
    [0x0C, 0x0E, 0x0C, 0x0C, 0x0C, 0x0C, 0x3F, 0x00],
    [0x1E, 0x33, 0x30, 0x1C, 0x06, 0x33, 0x3F, 0x00],
    [0x1E, 0x33, 0x30, 0x1C, 0x30, 0x33, 0x1E, 0x00],
    [0x38, 0x3C, 0x36, 0x33, 0x7F, 0x30, 0x78, 0x00],
    [0x3F, 0x03, 0x1F, 0x30, 0x30, 0x33, 0x1E, 0x00],
    [0x1C, 0x06, 0x03, 0x1F, 0x33, 0x33, 0x1E, 0x00],
    [0x3F, 0x33, 0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x00],
    [0x1E, 0x33, 0x33, 0x1E, 0x33, 0x33, 0x1E, 0x00],
    [0x1E, 0x33, 0x33, 0x3E, 0x30, 0x18, 0x0E, 0x00],
    [0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x0C, 0x00],
    [0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x0C, 0x06],
    [0x18, 0x0C, 0x06, 0x03, 0x06, 0x0C, 0x18, 0x00],
    [0x00, 0x00, 0x3F, 0x00, 0x00, 0x3F, 0x00, 0x00],
    [0x06, 0x0C, 0x18, 0x30, 0x18, 0x0C, 0x06, 0x00],
    [0x1E, 0x33, 0x30, 0x18, 0x0C, 0x00, 0x0C, 0x00],
    [0x3E, 0x63, 0x7B, 0x7B, 0x7B, 0x03, 0x1E, 0x00],
    [0x0C, 0x1E, 0x33, 0x33, 0x3F, 0x33, 0x33, 0x00],
    [0x3F, 0x66, 0x66, 0x3E, 0x66, 0x66, 0x3F, 0x00],
    [0x3C, 0x66, 0x03, 0x03, 0x03, 0x66, 0x3C, 0x00],
    [0x1F, 0x36, 0x66, 0x66, 0x66, 0x36, 0x1F, 0x00],
    [0x7F, 0x46, 0x16, 0x1E, 0x16, 0x46, 0x7F, 0x00],
    [0x7F, 0x46, 0x16, 0x1E, 0x16, 0x06, 0x0F, 0x00],
    [0x3C, 0x66, 0x03, 0x03, 0x73, 0x66, 0x7C, 0x00],
    [0x33, 0x33, 0x33, 0x3F, 0x33, 0x33, 0x33, 0x00],
    [0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00],
    [0x78, 0x30, 0x30, 0x30, 0x33, 0x33, 0x1E, 0x00],
    [0x67, 0x66, 0x36, 0x1E, 0x36, 0x66, 0x67, 0x00],
    [0x0F, 0x06, 0x06, 0x06, 0x46, 0x66, 0x7F, 0x00],
    [0x63, 0x77, 0x7F, 0x7F, 0x6B, 0x63, 0x63, 0x00],
    [0x63, 0x67, 0x6F, 0x7B, 0x73, 0x63, 0x63, 0x00],
    [0x1C, 0x36, 0x63, 0x63, 0x63, 0x36, 0x1C, 0x00],
    [0x3F, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x0F, 0x00],
    [0x1E, 0x33, 0x33, 0x33, 0x3B, 0x1E, 0x38, 0x00],
    [0x3F, 0x66, 0x66, 0x3E, 0x36, 0x66, 0x67, 0x00],
    [0x1E, 0x33, 0x07, 0x0E, 0x38, 0x33, 0x1E, 0x00],
    [0x3F, 0x2D, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00],
    [0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x3F, 0x00],
    [0x33, 0x33, 0x33, 0x33, 0x33, 0x1E, 0x0C, 0x00],
    [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00],
    [0x63, 0x63, 0x36, 0x1C, 0x1C, 0x36, 0x63, 0x00],
    [0x33, 0x33, 0x33, 0x1E, 0x0C, 0x0C, 0x1E, 0x00],
    [0x7F, 0x63, 0x31, 0x18, 0x4C, 0x66, 0x7F, 0x00],
    [0x1E, 0x06, 0x06, 0x06, 0x06, 0x06, 0x1E, 0x00],
    [0x03, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00],
    [0x1E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x1E, 0x00],
    [0x08, 0x1C, 0x36, 0x63, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF],
    [0x0C, 0x0C, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x1E, 0x30, 0x3E, 0x33, 0x6E, 0x00],
    [0x07, 0x06, 0x06, 0x3E, 0x66, 0x66, 0x3B, 0x00],
    [0x00, 0x00, 0x1E, 0x33, 0x03, 0x33, 0x1E, 0x00],
    [0x38, 0x30, 0x30, 0x3E, 0x33, 0x33, 0x6E, 0x00],
    [0x00, 0x00, 0x1E, 0x33, 0x3F, 0x03, 0x1E, 0x00],
    [0x1C, 0x36, 0x06, 0x0F, 0x06, 0x06, 0x0F, 0x00],
    [0x00, 0x00, 0x6E, 0x33, 0x33, 0x3E, 0x30, 0x1F],
    [0x07, 0x06, 0x36, 0x6E, 0x66, 0x66, 0x67, 0x00],
    [0x0C, 0x00, 0x0E, 0x0C, 0x0C, 0x0C, 0x1E, 0x00],
    [0x30, 0x00, 0x30, 0x30, 0x30, 0x33, 0x33, 0x1E],
    [0x07, 0x06, 0x66, 0x36, 0x1E, 0x36, 0x67, 0x00],
    [0x0E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00],
    [0x00, 0x00, 0x33, 0x7F, 0x7F, 0x6B, 0x63, 0x00],
    [0x00, 0x00, 0x1F, 0x33, 0x33, 0x33, 0x33, 0x00],
    [0x00, 0x00, 0x1E, 0x33, 0x33, 0x33, 0x1E, 0x00],
    [0x00, 0x00, 0x3B, 0x66, 0x66, 0x3E, 0x06, 0x0F],
    [0x00, 0x00, 0x6E, 0x33, 0x33, 0x3E, 0x30, 0x78],
    [0x00, 0x00, 0x3B, 0x6E, 0x66, 0x06, 0x0F, 0x00],
    [0x00, 0x00, 0x3E, 0x03, 0x1E, 0x30, 0x1F, 0x00],
    [0x08, 0x0C, 0x3E, 0x0C, 0x0C, 0x2C, 0x18, 0x00],
    [0x00, 0x00, 0x33, 0x33, 0x33, 0x33, 0x6E, 0x00],
    [0x00, 0x00, 0x33, 0x33, 0x33, 0x1E, 0x0C, 0x00],
    [0x00, 0x00, 0x63, 0x6B, 0x7F, 0x7F, 0x36, 0x00],
    [0x00, 0x00, 0x63, 0x36, 0x1C, 0x36, 0x63, 0x00],
    [0x00, 0x00, 0x33, 0x33, 0x33, 0x3E, 0x30, 0x1F],
    [0x00, 0x00, 0x3F, 0x19, 0x0C, 0x26, 0x3F, 0x00],
    [0x38, 0x0C, 0x0C, 0x07, 0x0C, 0x0C, 0x38, 0x00],
    [0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x18, 0x00],
    [0x07, 0x0C, 0x0C, 0x38, 0x0C, 0x0C, 0x07, 0x00],
    [0x6E, 0x3B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
];

const KEYWORDS: &[&str] = &[
    "fn",
    "let",
    "mut",
    "pub",
    "use",
    "impl",
    "struct",
    "enum",
    "trait",
    "type",
    "const",
    "static",
    "if",
    "else",
    "match",
    "for",
    "while",
    "loop",
    "return",
    "break",
    "continue",
    "where",
    "self",
    "super",
    "crate",
    "mod",
    "async",
    "await",
    "move",
    "ref",
    "in",
    "as",
    "true",
    "false",
    "Some",
    "None",
    "Ok",
    "Err",
    "Self",
    "def",
    "class",
    "import",
    "from",
    "pass",
    "lambda",
    "yield",
    "with",
    "try",
    "except",
    "raise",
    "finally",
    "elif",
    "del",
    "global",
    "nonlocal",
    "assert",
    "is",
    "not",
    "and",
    "or",
    "function",
    "var",
    "const",
    "export",
    "default",
    "switch",
    "case",
    "throw",
    "catch",
    "typeof",
    "instanceof",
    "new",
    "delete",
    "void",
    "this",
    "null",
    "undefined",
    "func",
    "package",
    "defer",
    "go",
    "chan",
    "select",
    "range",
    "map",
    "interface",
    "fallthrough",
    "nil",
];

fn render_tree_sitter_lines(ui: &mut Context, lines: &[Vec<(String, crate::style::Style)>]) {
    for segs in lines {
        if segs.is_empty() {
            ui.text(" ");
        } else {
            ui.line(|ui| {
                for (text, style) in segs {
                    ui.styled(text, *style);
                }
            });
        }
    }
}

fn render_highlighted_line(ui: &mut Context, line: &str) {
    let theme = ui.theme;
    let syntax = theme.syntax;
    let keyword_color = syntax.keyword;
    let string_color = syntax.string;
    let comment_color = theme.text_dim;
    let number_color = syntax.number;
    let fn_color = syntax.function;
    let macro_color = syntax.macro_;

    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    if !indent.is_empty() {
        ui.text(indent);
    }

    if trimmed.starts_with("//") {
        ui.text(trimmed).fg(comment_color).italic();
        return;
    }

    let mut pos = 0;

    while pos < trimmed.len() {
        let ch = trimmed.as_bytes()[pos];

        if ch == b'"'
            && let Some(end) = trimmed[pos + 1..].find('"')
        {
            let s = &trimmed[pos..pos + end + 2];
            ui.text(s).fg(string_color);
            pos += end + 2;
            continue;
        }

        if ch.is_ascii_digit() && (pos == 0 || !trimmed.as_bytes()[pos - 1].is_ascii_alphanumeric())
        {
            let end = trimmed[pos..]
                .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '_')
                .map_or(trimmed.len(), |e| pos + e);
            ui.text(&trimmed[pos..end]).fg(number_color);
            pos = end;
            continue;
        }

        if ch.is_ascii_alphabetic() || ch == b'_' {
            let end = trimmed[pos..]
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .map_or(trimmed.len(), |e| pos + e);
            let word = &trimmed[pos..end];

            if end < trimmed.len() && trimmed.as_bytes()[end] == b'!' {
                ui.text(&trimmed[pos..end + 1]).fg(macro_color);
                pos = end + 1;
            } else if end < trimmed.len()
                && trimmed.as_bytes()[end] == b'('
                && !KEYWORDS.contains(&word)
            {
                ui.text(word).fg(fn_color);
                pos = end;
            } else if KEYWORDS.contains(&word) {
                ui.text(word).fg(keyword_color);
                pos = end;
            } else {
                ui.text(word);
                pos = end;
            }
            continue;
        }

        let end = trimmed[pos..]
            .find(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '"')
            .map_or(trimmed.len(), |e| pos + e);
        ui.text(&trimmed[pos..end]);
        pos = end;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreparedRgbaKey {
    content_hash: u64,
    width: u32,
    height: u32,
    bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreparedHalfblockKey {
    content_hash: u64,
    width: u32,
    height: u32,
    cells: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(feature = "crossterm")]
struct PreparedSixelKey {
    content_hash: u64,
    width: u32,
    height: u32,
    colors: u32,
    target_width: u32,
    target_height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(feature = "crossterm")]
struct PreparedItermKey {
    content_hash: u64,
    bytes: usize,
    cols: u32,
    rows: u32,
    preserve_aspect: bool,
}

struct PreparationCacheEntry<K, V> {
    key: K,
    value: V,
    weight: usize,
}

struct PreparationCache<K, V> {
    entries: std::collections::VecDeque<PreparationCacheEntry<K, V>>,
    total_weight: usize,
}

impl<K: PartialEq, V: Clone> PreparationCache<K, V> {
    const MAX_ENTRIES: usize = 16;
    const MAX_WEIGHT: usize = 32 * 1024 * 1024;

    fn new() -> Self {
        Self {
            entries: std::collections::VecDeque::new(),
            total_weight: 0,
        }
    }

    fn get(&mut self, key: &K) -> Option<V> {
        let index = self.entries.iter().position(|entry| &entry.key == key)?;
        let entry = self.entries.remove(index)?;
        let value = entry.value.clone();
        self.entries.push_back(entry);
        Some(value)
    }

    fn insert(&mut self, key: K, value: V, weight: usize) {
        if weight > Self::MAX_WEIGHT {
            return;
        }
        while self.entries.len() >= Self::MAX_ENTRIES
            || self.total_weight.saturating_add(weight) > Self::MAX_WEIGHT
        {
            let Some(entry) = self.entries.pop_front() else {
                break;
            };
            self.total_weight = self.total_weight.saturating_sub(entry.weight);
        }
        self.total_weight = self.total_weight.saturating_add(weight);
        self.entries
            .push_back(PreparationCacheEntry { key, value, weight });
    }
}

fn prepare_rgba(data: &[u8], width: u32, height: u32) -> Option<(u64, std::sync::Arc<Vec<u8>>)> {
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels == 0 || pixels > crate::buffer::MAX_IMAGE_PIXELS {
        return None;
    }
    let expected = usize::try_from(pixels).ok()?.checked_mul(4)?;
    if data.len() < expected {
        return None;
    }
    let normalized = &data[..expected];
    let content_hash = crate::buffer::hash_rgba(normalized);
    let key = PreparedRgbaKey {
        content_hash,
        width,
        height,
        bytes: expected,
    };

    static CACHE: std::sync::OnceLock<
        std::sync::Mutex<PreparationCache<PreparedRgbaKey, std::sync::Arc<Vec<u8>>>>,
    > = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(PreparationCache::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(prepared) = cache.get(&key)
        && prepared.as_slice() == normalized
    {
        return Some((content_hash, prepared));
    }

    let prepared = std::sync::Arc::new(normalized.to_vec());
    cache.insert(key, std::sync::Arc::clone(&prepared), expected);
    Some((content_hash, prepared))
}

fn prepare_halfblock(
    pixels: &[(Color, Color)],
    width: u32,
    height: u32,
) -> Option<std::sync::Arc<Vec<(Color, Color)>>> {
    use std::hash::{Hash, Hasher};

    let cells = usize::try_from(u64::from(width).saturating_mul(u64::from(height))).ok()?;
    if cells == 0 || pixels.len() < cells {
        return None;
    }
    let pixels = &pixels[..cells];
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pixels.hash(&mut hasher);
    let key = PreparedHalfblockKey {
        content_hash: hasher.finish(),
        width,
        height,
        cells,
    };
    type HalfblockPreparationCache = std::sync::Mutex<
        PreparationCache<PreparedHalfblockKey, std::sync::Arc<Vec<(Color, Color)>>>,
    >;
    static CACHE: std::sync::OnceLock<HalfblockPreparationCache> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(PreparationCache::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(prepared) = cache.get(&key)
        && prepared.as_slice() == pixels
    {
        return Some(prepared);
    }

    let prepared = std::sync::Arc::new(pixels.to_vec());
    let weight = cells.saturating_mul(std::mem::size_of::<(Color, Color)>());
    cache.insert(key, std::sync::Arc::clone(&prepared), weight);
    Some(prepared)
}

#[cfg(feature = "crossterm")]
type EncodedMedia = (u64, std::sync::Arc<str>);
#[cfg(feature = "crossterm")]
type EncodedMediaCache<K> = std::sync::Mutex<PreparationCache<K, EncodedMedia>>;

#[cfg(feature = "crossterm")]
fn prepare_sixel(
    data: &[u8],
    width: u32,
    height: u32,
    colors: u32,
    target_width: u32,
    target_height: u32,
) -> Option<EncodedMedia> {
    let (content_hash, rgba) = prepare_rgba(data, width, height)?;
    let key = PreparedSixelKey {
        content_hash,
        width,
        height,
        colors,
        target_width,
        target_height,
    };
    static CACHE: std::sync::OnceLock<EncodedMediaCache<PreparedSixelKey>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(PreparationCache::new()));
    let cached = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&key);
    if let Some(prepared) = cached {
        return Some(prepared);
    }

    let scaled = crate::sixel::resize_rgba(&rgba, width, height, target_width, target_height)?;
    let encoded: std::sync::Arc<str> =
        crate::sixel::encode_sixel(&scaled, target_width, target_height, colors).into();
    if encoded.is_empty() {
        return None;
    }
    let payload_hash = crate::buffer::hash_rgba(encoded.as_bytes());
    cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(key, (payload_hash, encoded.clone()), encoded.len());
    Some((payload_hash, encoded))
}

#[cfg(feature = "crossterm")]
fn prepare_iterm(data: &[u8], cols: u32, rows: u32, preserve_aspect: bool) -> Option<EncodedMedia> {
    encoded_image_dimensions(data)?;
    let content_hash = crate::buffer::hash_rgba(data);
    let key = PreparedItermKey {
        content_hash,
        bytes: data.len(),
        cols,
        rows,
        preserve_aspect,
    };
    static CACHE: std::sync::OnceLock<EncodedMediaCache<PreparedItermKey>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(PreparationCache::new()));
    let cached = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&key);
    if let Some(prepared) = cached {
        return Some(prepared);
    }

    let encoded: std::sync::Arc<str> =
        crate::iterm::encode_iterm_osc1337(data, cols, rows, preserve_aspect).into();
    if encoded.is_empty() {
        return None;
    }
    let payload_hash = crate::buffer::hash_rgba(encoded.as_bytes());
    cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(key, (payload_hash, encoded.clone()), encoded.len());
    Some((payload_hash, encoded))
}

fn encoded_image_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() >= 24
        && data[..8] == [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
        && data[12..16] == *b"IHDR"
    {
        let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return (width > 0 && height > 0).then_some((width, height));
    }

    if data.len() >= 10 && (&data[..6] == b"GIF87a" || &data[..6] == b"GIF89a") {
        let width = u32::from(u16::from_le_bytes([data[6], data[7]]));
        let height = u32::from(u16::from_le_bytes([data[8], data[9]]));
        return (width > 0 && height > 0).then_some((width, height));
    }

    if data.starts_with(&[0xff, 0xd8]) {
        let mut offset = 2usize;
        let scan_limit = data.len().min(1024 * 1024);
        while offset + 3 < scan_limit {
            if data[offset] != 0xff {
                offset += 1;
                continue;
            }
            while offset < scan_limit && data[offset] == 0xff {
                offset += 1;
            }
            if offset >= scan_limit {
                break;
            }
            let marker = data[offset];
            offset += 1;
            if marker == 0xd9 || marker == 0xda {
                break;
            }
            if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
                continue;
            }
            if offset + 1 >= scan_limit {
                break;
            }
            let segment_len = usize::from(u16::from_be_bytes([data[offset], data[offset + 1]]));
            if segment_len < 2 || offset.saturating_add(segment_len) > scan_limit {
                break;
            }
            let is_start_of_frame = matches!(
                marker,
                0xc0 | 0xc1
                    | 0xc2
                    | 0xc3
                    | 0xc5
                    | 0xc6
                    | 0xc7
                    | 0xc9
                    | 0xca
                    | 0xcb
                    | 0xcd
                    | 0xce
                    | 0xcf
            );
            if is_start_of_frame && segment_len >= 7 {
                let height = u32::from(u16::from_be_bytes([data[offset + 3], data[offset + 4]]));
                let width = u32::from(u16::from_be_bytes([data[offset + 5], data[offset + 6]]));
                return (width > 0 && height > 0).then_some((width, height));
            }
            offset = offset.saturating_add(segment_len);
        }
    }

    None
}

fn image_fit_rows(src_width: u32, src_height: u32, cols: u32, cell_w: u32, cell_h: u32) -> u32 {
    if src_width == 0 || src_height == 0 || cell_w == 0 || cell_h == 0 {
        return 1;
    }

    ((cols as f64 * src_height as f64 * cell_w as f64) / (src_width as f64 * cell_h as f64))
        .ceil()
        .max(1.0) as u32
}

fn sample_rgba_color(
    data: &[u8],
    src_width: u32,
    src_height: u32,
    dst_x: u32,
    dst_y: u32,
    dst_width: u32,
    dst_height: u32,
) -> Option<Color> {
    if src_width == 0 || src_height == 0 || dst_width == 0 || dst_height == 0 {
        return None;
    }

    let src_x = (u64::from(dst_x) * u64::from(src_width) / u64::from(dst_width))
        .min(u64::from(src_width.saturating_sub(1)));
    let src_y = (u64::from(dst_y) * u64::from(src_height) / u64::from(dst_height))
        .min(u64::from(src_height.saturating_sub(1)));
    let pixel = src_y
        .saturating_mul(u64::from(src_width))
        .saturating_add(src_x);
    let idx = usize::try_from(pixel).ok().and_then(|p| p.checked_mul(4))?;
    let px = data.get(idx..idx + 4)?;
    if px[3] == 0 {
        None
    } else {
        Some(Color::Rgb(px[0], px[1], px[2]))
    }
}

fn draw_halfblock_cell(
    buf: &mut crate::Buffer,
    x: u32,
    y: u32,
    upper: Option<Color>,
    lower: Option<Color>,
) {
    match (upper, lower) {
        (Some(upper), Some(lower)) => {
            buf.set_char(x, y, '▀', Style::new().fg(upper).bg(lower));
        }
        (Some(upper), None) => {
            buf.set_char(x, y, '▀', Style::new().fg(upper));
        }
        (None, Some(lower)) => {
            buf.set_char(x, y, '▄', Style::new().fg(lower));
        }
        (None, None) => {
            buf.set_char(x, y, ' ', Style::new());
        }
    }
}

fn draw_rgba_fallback(
    buf: &mut crate::Buffer,
    rect: Rect,
    rgba: &[u8],
    width: u32,
    height: u32,
    cols: u32,
    rows: u32,
) {
    let (source_x, source_y) = buf.draw_source_offset();
    let pixel_rows = rows.saturating_mul(2).max(1);
    for y in 0..rect.height {
        for x in 0..rect.width {
            let sx = source_x.saturating_add(x);
            let sy = source_y.saturating_add(y).saturating_mul(2);
            let upper = sample_rgba_color(rgba, width, height, sx, sy, cols, pixel_rows);
            let lower = sample_rgba_color(
                rgba,
                width,
                height,
                sx,
                sy.saturating_add(1),
                cols,
                pixel_rows,
            );
            draw_halfblock_cell(buf, rect.x + x, rect.y + y, upper, lower);
        }
    }
}

#[cfg(all(test, feature = "crossterm"))]
fn terminal_supports_kitty() -> bool {
    crate::terminal::GraphicsEmissionSupport::for_context(
        crate::terminal::Capabilities::default(),
        true,
    )
    .should_emit_kitty()
}

#[cfg(all(test, feature = "crossterm"))]
fn terminal_supports_sixel() -> bool {
    crate::terminal::GraphicsEmissionSupport::for_context(
        crate::terminal::Capabilities::default(),
        true,
    )
    .should_emit_sprixel(crate::terminal::SprixelProtocol::Sixel)
}

/// Env-fallback detection for the iTerm2 OSC 1337 inline-image protocol
/// (issue #265).
///
/// Consulted as the env-fallback when the runtime capability probe returned
/// unknown, mirroring [`terminal_supports_sixel`]. Matches the `TERM_PROGRAM`
/// identities of terminals that implement OSC 1337 inline images: iTerm2
/// itself, WezTerm (iTerm2-compat mode), Tabby, and mintty. `SLT_FORCE_ITERM=1`
/// forces a positive. Never fires on `xterm-256color`, matching the sixel
/// regression-test parity.
#[cfg(all(test, feature = "crossterm"))]
fn terminal_supports_iterm() -> bool {
    crate::terminal::GraphicsEmissionSupport::for_context(
        crate::terminal::Capabilities::default(),
        true,
    )
    .should_emit_sprixel(crate::terminal::SprixelProtocol::Iterm2)
}

#[cfg(all(test, feature = "crossterm"))]
#[derive(Default)]
struct GraphicsEnv<'a> {
    term: Option<&'a str>,
    term_program: Option<&'a str>,
    force_sixel: bool,
    force_iterm: bool,
    force_kitty: bool,
    tmux: bool,
    sty: bool,
}

#[cfg(all(test, feature = "crossterm"))]
static GRAPHICS_ENV_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(all(test, feature = "crossterm"))]
#[allow(unsafe_code)]
fn with_graphics_env<F: FnOnce()>(env: GraphicsEnv<'_>, f: F) {
    let _g = GRAPHICS_ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());
    let prev_term = std::env::var("TERM").ok();
    let prev_program = std::env::var("TERM_PROGRAM").ok();
    let prev_sixel = std::env::var("SLT_FORCE_SIXEL").ok();
    let prev_iterm = std::env::var("SLT_FORCE_ITERM").ok();
    let prev_kitty = std::env::var("SLT_FORCE_KITTY").ok();
    let prev_tmux = std::env::var("TMUX").ok();
    let prev_sty = std::env::var("STY").ok();

    // SAFETY (edition 2024): set_var/remove_var are unsafe because env
    // mutation races concurrent reads. GRAPHICS_ENV_GUARD serializes these
    // tests so no other detection test observes a torn update.
    unsafe {
        match env.term {
            Some(v) => std::env::set_var("TERM", v),
            None => std::env::remove_var("TERM"),
        }
        match env.term_program {
            Some(v) => std::env::set_var("TERM_PROGRAM", v),
            None => std::env::remove_var("TERM_PROGRAM"),
        }
        if env.force_sixel {
            std::env::set_var("SLT_FORCE_SIXEL", "1");
        } else {
            std::env::remove_var("SLT_FORCE_SIXEL");
        }
        if env.force_iterm {
            std::env::set_var("SLT_FORCE_ITERM", "1");
        } else {
            std::env::remove_var("SLT_FORCE_ITERM");
        }
        if env.force_kitty {
            std::env::set_var("SLT_FORCE_KITTY", "1");
        } else {
            std::env::remove_var("SLT_FORCE_KITTY");
        }
        if env.tmux {
            std::env::set_var("TMUX", "/tmp/tmux-1000/default,1,0");
        } else {
            std::env::remove_var("TMUX");
        }
        if env.sty {
            std::env::set_var("STY", "1234.pts-0.host");
        } else {
            std::env::remove_var("STY");
        }
    }

    f();

    unsafe {
        match prev_term {
            Some(v) => std::env::set_var("TERM", v),
            None => std::env::remove_var("TERM"),
        }
        match prev_program {
            Some(v) => std::env::set_var("TERM_PROGRAM", v),
            None => std::env::remove_var("TERM_PROGRAM"),
        }
        match prev_sixel {
            Some(v) => std::env::set_var("SLT_FORCE_SIXEL", v),
            None => std::env::remove_var("SLT_FORCE_SIXEL"),
        }
        match prev_iterm {
            Some(v) => std::env::set_var("SLT_FORCE_ITERM", v),
            None => std::env::remove_var("SLT_FORCE_ITERM"),
        }
        match prev_kitty {
            Some(v) => std::env::set_var("SLT_FORCE_KITTY", v),
            None => std::env::remove_var("SLT_FORCE_KITTY"),
        }
        match prev_tmux {
            Some(v) => std::env::set_var("TMUX", v),
            None => std::env::remove_var("TMUX"),
        }
        match prev_sty {
            Some(v) => std::env::set_var("STY", v),
            None => std::env::remove_var("STY"),
        }
    }
}

#[cfg(all(test, feature = "crossterm"))]
mod kitty_detection_tests {
    use super::{GraphicsEnv, terminal_supports_kitty, with_graphics_env};

    #[test]
    fn kitty_xterm_kitty_detected() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-kitty"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_kitty());
            },
        );
    }

    #[test]
    fn kitty_ghostty_detected_via_term_program() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("ghostty"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_kitty());
            },
        );
    }

    #[test]
    fn kitty_xterm_256color_no_false_positive() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_kitty());
            },
        );
    }

    #[test]
    fn kitty_tmux_blocks_env_fallback() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("wezterm"),
                tmux: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_kitty());
            },
        );
    }

    #[test]
    fn kitty_screen_term_blocks_env_fallback() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("screen-256color"),
                term_program: Some("ghostty"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_kitty());
            },
        );
    }

    #[test]
    fn kitty_force_env_overrides_tmux() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("screen-256color"),
                term_program: Some("wezterm"),
                force_kitty: true,
                tmux: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_kitty());
            },
        );
    }
}

#[cfg(all(test, feature = "crossterm"))]
mod sixel_detection_tests {
    use super::{GraphicsEnv, terminal_supports_sixel, with_graphics_env};

    #[test]
    fn sixel_xterm_256color_no_false_positive() {
        // Regression: `term.contains("xterm")` previously matched
        // `xterm-256color` and printed raw escape sequences to screen on
        // macOS Terminal.app, VS Code, and most SSH clients.
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_mlterm_detected() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("mlterm"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_foot_detected_via_term() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("foot"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_foot_detected_via_term_program() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("foot"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_force_env_overrides_negative_term() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                force_sixel: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_substring_match_catches_custom_builds() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("custom-with-sixel"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_wezterm_detected_via_term_program() {
        // Issue #264: WezTerm reports `TERM=xterm-256color` but is a capable
        // image host; it must no longer fall through to `[sixel unsupported]`.
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("wezterm"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_ghostty_detected_via_term_program() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("ghostty"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_mlterm_still_detected_via_term_program() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("mlterm"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_tmux_blocks_env_fallback() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("ghostty"),
                tmux: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_force_env_overrides_tmux() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("screen-256color"),
                term_program: Some("wezterm"),
                force_sixel: true,
                tmux: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }
}

#[cfg(all(test, feature = "crossterm"))]
mod iterm_detection_tests {
    use super::{GraphicsEnv, terminal_supports_iterm, with_graphics_env};

    #[test]
    fn iterm_xterm_256color_no_false_positive() {
        // Parity with `sixel_xterm_256color_no_false_positive`: a plain xterm
        // must never be mistaken for an OSC 1337 host.
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_app_detected() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("iTerm.app"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_wezterm_detected() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("WezTerm"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_tabby_detected() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("Tabby"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_mintty_detected() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("mintty"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_force_env_overrides_negative_term() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                force_iterm: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_unknown_term_program_negative() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("Apple_Terminal"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_tmux_blocks_env_fallback() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("iTerm.app"),
                tmux: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_screen_term_blocks_env_fallback() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("screen.xterm-256color"),
                term_program: Some("Tabby"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_force_env_overrides_tmux() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("screen-256color"),
                term_program: Some("iTerm.app"),
                force_iterm: true,
                tmux: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_iterm());
            },
        );
    }
}

#[cfg(test)]
mod fallback_syntax_theme_tests {
    use super::*;
    use crate::TestBackend;

    #[test]
    fn fallback_highlighter_uses_custom_semantic_palette_with_reset_background() {
        let mut theme = Theme::dark();
        theme.bg = Color::Reset;
        theme.is_dark = true;
        theme.syntax.keyword = Color::Rgb(1, 2, 3);
        theme.syntax.string = Color::Rgb(4, 5, 6);
        theme.syntax.number = Color::Rgb(7, 8, 9);
        theme.syntax.function = Color::Rgb(10, 11, 12);
        theme.syntax.macro_ = Color::Rgb(13, 14, 15);

        let mut backend = TestBackend::new(40, 2);
        backend.render(|ui| {
            ui.set_theme(theme);
            ui.line(|ui| render_highlighted_line(ui, "let value = call(42, \"x\");"));
        });

        let buffer = backend.buffer();
        let colors: Vec<Option<Color>> = (0..backend.width())
            .map(|x| buffer.get(x, 0).style.fg)
            .collect();
        assert!(colors.contains(&Some(theme.syntax.keyword)));
        assert!(colors.contains(&Some(theme.syntax.string)));
        assert!(colors.contains(&Some(theme.syntax.number)));
        assert!(colors.contains(&Some(theme.syntax.function)));
    }
}

#[cfg(test)]
mod media_preparation_tests {
    use super::*;

    #[cfg(feature = "crossterm")]
    #[test]
    fn v024_sixel_preparation_shares_payload_and_keys_final_geometry() {
        let rgba = [255, 0, 0, 255].repeat(4);
        let (_, first) = prepare_sixel(&rgba, 2, 2, 256, 8, 16).unwrap();
        let (_, second) = prepare_sixel(&rgba, 2, 2, 256, 8, 16).unwrap();
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        let (_, resized) = prepare_sixel(&rgba, 2, 2, 256, 16, 8).unwrap();
        assert_ne!(first, resized);
    }

    #[test]
    fn rgba_preparation_is_strict_and_reuses_cached_allocation() {
        let rgba = [255u8, 0, 0, 255, 0, 255, 0, 255];
        let (_, first) = prepare_rgba(&rgba, 2, 1).expect("valid rgba");
        let (_, second) = prepare_rgba(&rgba, 2, 1).expect("cached rgba");
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        assert!(prepare_rgba(&rgba[..7], 2, 1).is_none());
        assert!(prepare_rgba(&rgba, 0, 1).is_none());
    }

    #[test]
    fn halfblock_preparation_reuses_cached_allocation() {
        let pixels = vec![(Color::Red, Color::Blue); 4];
        let first = prepare_halfblock(&pixels, 2, 2).expect("valid halfblock image");
        let second = prepare_halfblock(&pixels, 2, 2).expect("cached halfblock image");
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        assert!(prepare_halfblock(&pixels[..3], 2, 2).is_none());
    }

    #[test]
    fn encoded_image_dimensions_reads_png_gif_and_jpeg_headers() {
        let mut png = vec![0u8; 24];
        png[..8].copy_from_slice(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&320u32.to_be_bytes());
        png[20..24].copy_from_slice(&200u32.to_be_bytes());
        assert_eq!(encoded_image_dimensions(&png), Some((320, 200)));

        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&64u16.to_le_bytes());
        gif.extend_from_slice(&32u16.to_le_bytes());
        assert_eq!(encoded_image_dimensions(&gif), Some((64, 32)));

        let jpeg = [
            0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 0x08, 0x00, 0x30, 0x00, 0x40, 0x03, 0x01, 0x11,
            0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00,
        ];
        assert_eq!(encoded_image_dimensions(&jpeg), Some((64, 48)));
        assert_eq!(encoded_image_dimensions(b"not an image"), None);
    }

    #[test]
    fn image_fit_rows_uses_source_aspect_ratio() {
        assert_eq!(image_fit_rows(400, 200, 20, 8, 16), 5);
        assert_eq!(image_fit_rows(200, 400, 20, 8, 16), 20);
    }

    #[test]
    fn preparation_cache_evicts_old_entries_at_fixed_capacity() {
        let mut cache = PreparationCache::new();
        for key in 0..(PreparationCache::<usize, usize>::MAX_ENTRIES + 2) {
            cache.insert(key, key, 1);
        }
        assert_eq!(
            cache.entries.len(),
            PreparationCache::<usize, usize>::MAX_ENTRIES
        );
        assert!(cache.get(&0).is_none());
        assert!(
            cache
                .get(&(PreparationCache::<usize, usize>::MAX_ENTRIES + 1))
                .is_some()
        );
    }
}
