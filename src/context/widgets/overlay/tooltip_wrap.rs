//! Word wrapping for tooltip bodies.

use super::*;

pub(in crate::context) fn wrap_tooltip_text(text: &str, max_width: usize) -> Vec<String> {
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
                let chunk_width = crate::width::unicode_width(chunk.as_str());

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
