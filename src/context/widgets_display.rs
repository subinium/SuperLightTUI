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
pub use status::Breadcrumb;

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
    let is_light = matches!(
        theme.bg,
        Color::Reset | Color::White | Color::Rgb(255, 255, 255)
    );
    let keyword_color = if is_light {
        Color::Rgb(166, 38, 164)
    } else {
        Color::Rgb(198, 120, 221)
    };
    let string_color = if is_light {
        Color::Rgb(80, 161, 79)
    } else {
        Color::Rgb(152, 195, 121)
    };
    let comment_color = theme.text_dim;
    let number_color = if is_light {
        Color::Rgb(152, 104, 1)
    } else {
        Color::Rgb(209, 154, 102)
    };
    let fn_color = if is_light {
        Color::Rgb(64, 120, 242)
    } else {
        Color::Rgb(97, 175, 239)
    };
    let macro_color = if is_light {
        Color::Rgb(1, 132, 188)
    } else {
        Color::Rgb(86, 182, 194)
    };

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

        if ch == b'"' {
            if let Some(end) = trimmed[pos + 1..].find('"') {
                let s = &trimmed[pos..pos + end + 2];
                ui.text(s).fg(string_color);
                pos += end + 2;
                continue;
            }
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

fn normalize_rgba(data: &[u8], width: u32, height: u32) -> Vec<u8> {
    // Guard against overflow and resource exhaustion from attacker-controlled
    // dimensions. Returns an empty buffer for out-of-range inputs; the image
    // widgets treat an empty buffer as a no-op.
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels == 0 || pixels > crate::buffer::MAX_IMAGE_PIXELS {
        return Vec::new();
    }
    let Some(expected) = (pixels as usize).checked_mul(4) else {
        return Vec::new();
    };
    if data.len() >= expected {
        return data[..expected].to_vec();
    }
    let mut buf = Vec::with_capacity(expected);
    buf.extend_from_slice(data);
    buf.resize(expected, 0);
    buf
}

#[cfg(feature = "crossterm")]
fn terminal_supports_sixel() -> bool {
    let force = std::env::var("SLT_FORCE_SIXEL")
        .ok()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(force.as_str(), "1" | "true" | "yes" | "on") {
        return true;
    }

    let term = std::env::var("TERM")
        .ok()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();
    let term_program = std::env::var("TERM_PROGRAM")
        .ok()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();

    // Exact-match known sixel terminals; substring "sixel" catches custom builds.
    // The previous `term.contains("xterm")` check fired on `xterm-256color`,
    // which is the default for macOS Terminal.app, VS Code, and most SSH
    // clients — none of which support sixel. Patched-xterm-with-sixel users
    // can opt in with `SLT_FORCE_SIXEL=1`.
    const KNOWN_SIXEL_TERMS: &[&str] = &["mlterm", "foot", "yaft", "xterm-256color-sixel"];
    KNOWN_SIXEL_TERMS.iter().any(|&t| term == t)
        || term.contains("sixel")
        || term_program == "foot"
        || term_program == "mlterm"
}

#[cfg(all(test, feature = "crossterm"))]
mod sixel_detection_tests {
    use super::terminal_supports_sixel;
    use std::sync::Mutex;

    // Env vars are process-global. A test-local mutex serializes access so
    // concurrent test threads don't observe each other's set_var() calls.
    static ENV_GUARD: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce()>(term: Option<&str>, term_program: Option<&str>, force: bool, f: F) {
        let _g = ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        let prev_term = std::env::var("TERM").ok();
        let prev_program = std::env::var("TERM_PROGRAM").ok();
        let prev_force = std::env::var("SLT_FORCE_SIXEL").ok();

        match term {
            Some(v) => std::env::set_var("TERM", v),
            None => std::env::remove_var("TERM"),
        }
        match term_program {
            Some(v) => std::env::set_var("TERM_PROGRAM", v),
            None => std::env::remove_var("TERM_PROGRAM"),
        }
        if force {
            std::env::set_var("SLT_FORCE_SIXEL", "1");
        } else {
            std::env::remove_var("SLT_FORCE_SIXEL");
        }

        f();

        match prev_term {
            Some(v) => std::env::set_var("TERM", v),
            None => std::env::remove_var("TERM"),
        }
        match prev_program {
            Some(v) => std::env::set_var("TERM_PROGRAM", v),
            None => std::env::remove_var("TERM_PROGRAM"),
        }
        match prev_force {
            Some(v) => std::env::set_var("SLT_FORCE_SIXEL", v),
            None => std::env::remove_var("SLT_FORCE_SIXEL"),
        }
    }

    #[test]
    fn sixel_xterm_256color_no_false_positive() {
        // Regression: `term.contains("xterm")` previously matched
        // `xterm-256color` and printed raw escape sequences to screen on
        // macOS Terminal.app, VS Code, and most SSH clients.
        with_env(Some("xterm-256color"), None, false, || {
            assert!(!terminal_supports_sixel());
        });
    }

    #[test]
    fn sixel_mlterm_detected() {
        with_env(Some("mlterm"), None, false, || {
            assert!(terminal_supports_sixel());
        });
    }

    #[test]
    fn sixel_foot_detected_via_term() {
        with_env(Some("foot"), None, false, || {
            assert!(terminal_supports_sixel());
        });
    }

    #[test]
    fn sixel_foot_detected_via_term_program() {
        with_env(Some("xterm-256color"), Some("foot"), false, || {
            assert!(terminal_supports_sixel());
        });
    }

    #[test]
    fn sixel_force_env_overrides_negative_term() {
        with_env(Some("xterm-256color"), None, true, || {
            assert!(terminal_supports_sixel());
        });
    }

    #[test]
    fn sixel_substring_match_catches_custom_builds() {
        with_env(Some("custom-with-sixel"), None, false, || {
            assert!(terminal_supports_sixel());
        });
    }
}
