//! Keyword-based fallback syntax highlighter and tree-sitter line renderer
//! for code blocks.

use super::*;

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

pub(super) fn render_tree_sitter_lines(
    ui: &mut Context,
    lines: &[Vec<(String, crate::style::Style)>],
) {
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

pub(super) fn render_highlighted_line(ui: &mut Context, line: &str) {
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
