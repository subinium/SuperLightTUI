//! Tree-sitter based syntax highlighting.
//!
//! When one of the `syntax-*` features is enabled, [`highlight_code`] uses
//! tree-sitter grammars for accurate, language-aware highlighting.
//! Without those features the function always returns `None` so callers
//! can fall back to the built-in keyword highlighter.

use crate::style::{Color, Style, Theme};

/// Ordered list of tree-sitter highlight capture names.
///
/// The index of each name corresponds to the `Highlight` index
/// returned by `HighlightEvent::HighlightStart`.
#[cfg(any(
    feature = "syntax-rust",
    feature = "syntax-python",
    feature = "syntax-javascript",
    feature = "syntax-typescript",
    feature = "syntax-go",
    feature = "syntax-bash",
    feature = "syntax-json",
    feature = "syntax-toml",
    feature = "syntax-c",
    feature = "syntax-cpp",
    feature = "syntax-java",
    feature = "syntax-ruby",
    feature = "syntax-css",
    feature = "syntax-html",
    feature = "syntax-yaml",
))]
const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "comment",
    "constant",
    "constant.builtin",
    "constructor",
    "embedded",
    "function",
    "function.builtin",
    "function.macro",
    "keyword",
    "module",
    "number",
    "operator",
    "property",
    "property.builtin",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.special",
    "string",
    "string.special",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
];

#[cfg(any(
    feature = "syntax-rust",
    feature = "syntax-python",
    feature = "syntax-javascript",
    feature = "syntax-typescript",
    feature = "syntax-go",
    feature = "syntax-bash",
    feature = "syntax-json",
    feature = "syntax-toml",
    feature = "syntax-c",
    feature = "syntax-cpp",
    feature = "syntax-java",
    feature = "syntax-ruby",
    feature = "syntax-css",
    feature = "syntax-html",
    feature = "syntax-yaml",
))]
use std::sync::OnceLock;

#[cfg(any(
    feature = "syntax-rust",
    feature = "syntax-python",
    feature = "syntax-javascript",
    feature = "syntax-typescript",
    feature = "syntax-go",
    feature = "syntax-bash",
    feature = "syntax-json",
    feature = "syntax-toml",
    feature = "syntax-c",
    feature = "syntax-cpp",
    feature = "syntax-java",
    feature = "syntax-ruby",
    feature = "syntax-css",
    feature = "syntax-html",
    feature = "syntax-yaml",
))]
use tree_sitter_highlight::HighlightConfiguration;

/// Return a cached `HighlightConfiguration` for `lang`, or `None` if the
/// language is unsupported or the corresponding feature is not enabled.
#[cfg(any(
    feature = "syntax-rust",
    feature = "syntax-python",
    feature = "syntax-javascript",
    feature = "syntax-typescript",
    feature = "syntax-go",
    feature = "syntax-bash",
    feature = "syntax-json",
    feature = "syntax-toml",
    feature = "syntax-c",
    feature = "syntax-cpp",
    feature = "syntax-java",
    feature = "syntax-ruby",
    feature = "syntax-css",
    feature = "syntax-html",
    feature = "syntax-yaml",
))]
fn get_config(lang: &str) -> Option<&'static HighlightConfiguration> {
    match lang {
        #[cfg(feature = "syntax-rust")]
        "rust" | "rs" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_rust::LANGUAGE.into(),
                    "rust",
                    tree_sitter_rust::HIGHLIGHTS_QUERY,
                    tree_sitter_rust::INJECTIONS_QUERY,
                    "",
                )
                .expect("valid rust highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-python")]
        "python" | "py" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_python::LANGUAGE.into(),
                    "python",
                    tree_sitter_python::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .expect("valid python highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-javascript")]
        "javascript" | "js" | "jsx" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_javascript::LANGUAGE.into(),
                    "javascript",
                    tree_sitter_javascript::HIGHLIGHT_QUERY,
                    tree_sitter_javascript::INJECTIONS_QUERY,
                    tree_sitter_javascript::LOCALS_QUERY,
                )
                .expect("valid javascript highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-go")]
        "go" | "golang" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_go::LANGUAGE.into(),
                    "go",
                    tree_sitter_go::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .expect("valid go highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-bash")]
        "bash" | "sh" | "shell" | "zsh" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_bash::LANGUAGE.into(),
                    "bash",
                    tree_sitter_bash::HIGHLIGHT_QUERY,
                    "",
                    "",
                )
                .expect("valid bash highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-json")]
        "json" | "jsonc" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_json::LANGUAGE.into(),
                    "json",
                    tree_sitter_json::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .expect("valid json highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-toml")]
        "toml" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_toml_ng::LANGUAGE.into(),
                    "toml",
                    tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .expect("valid toml highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-c")]
        "c" | "h" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_c::LANGUAGE.into(),
                    "c",
                    tree_sitter_c::HIGHLIGHT_QUERY,
                    "",
                    "",
                )
                .expect("valid c highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-cpp")]
        "cpp" | "c++" | "cxx" | "cc" | "hpp" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                #[cfg(feature = "syntax-c")]
                let highlights = {
                    let mut combined = String::with_capacity(
                        tree_sitter_c::HIGHLIGHT_QUERY.len()
                            + tree_sitter_cpp::HIGHLIGHT_QUERY.len()
                            + 1,
                    );
                    combined.push_str(tree_sitter_c::HIGHLIGHT_QUERY);
                    combined.push('\n');
                    combined.push_str(tree_sitter_cpp::HIGHLIGHT_QUERY);
                    combined
                };
                #[cfg(not(feature = "syntax-c"))]
                let highlights = tree_sitter_cpp::HIGHLIGHT_QUERY.to_string();

                let mut c = HighlightConfiguration::new(
                    tree_sitter_cpp::LANGUAGE.into(),
                    "cpp",
                    &highlights,
                    "",
                    "",
                )
                .expect("valid cpp highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-typescript")]
        "typescript" | "ts" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                    "typescript",
                    tree_sitter_typescript::HIGHLIGHTS_QUERY,
                    tree_sitter_typescript::LOCALS_QUERY,
                    "",
                )
                .expect("valid typescript highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-typescript")]
        "tsx" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_typescript::LANGUAGE_TSX.into(),
                    "tsx",
                    tree_sitter_typescript::HIGHLIGHTS_QUERY,
                    tree_sitter_typescript::LOCALS_QUERY,
                    "",
                )
                .expect("valid tsx highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-java")]
        "java" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_java::LANGUAGE.into(),
                    "java",
                    tree_sitter_java::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .expect("valid java highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-ruby")]
        "ruby" | "rb" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_ruby::LANGUAGE.into(),
                    "ruby",
                    tree_sitter_ruby::HIGHLIGHTS_QUERY,
                    tree_sitter_ruby::LOCALS_QUERY,
                    "",
                )
                .expect("valid ruby highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-css")]
        "css" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_css::LANGUAGE.into(),
                    "css",
                    tree_sitter_css::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .expect("valid css highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-html")]
        "html" | "htm" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_html::LANGUAGE.into(),
                    "html",
                    tree_sitter_html::HIGHLIGHTS_QUERY,
                    tree_sitter_html::INJECTIONS_QUERY,
                    "",
                )
                .expect("valid html highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        #[cfg(feature = "syntax-yaml")]
        "yaml" | "yml" => {
            static CFG: OnceLock<HighlightConfiguration> = OnceLock::new();
            Some(CFG.get_or_init(|| {
                let mut c = HighlightConfiguration::new(
                    tree_sitter_yaml::LANGUAGE.into(),
                    "yaml",
                    tree_sitter_yaml::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .expect("valid yaml highlight config");
                c.configure(HIGHLIGHT_NAMES);
                c
            }))
        }

        _ => None,
    }
}

/// Map a tree-sitter highlight capture name to an SLT [`Style`].
///
/// Colors follow the One Dark palette and flip between light/dark variants
/// based on [`Theme::is_dark`].
#[cfg(any(
    feature = "syntax-rust",
    feature = "syntax-python",
    feature = "syntax-javascript",
    feature = "syntax-typescript",
    feature = "syntax-go",
    feature = "syntax-bash",
    feature = "syntax-json",
    feature = "syntax-toml",
    feature = "syntax-c",
    feature = "syntax-cpp",
    feature = "syntax-java",
    feature = "syntax-ruby",
    feature = "syntax-css",
    feature = "syntax-html",
    feature = "syntax-yaml",
))]
fn highlight_name_to_style(name: &str, theme: &Theme) -> Style {
    let dark = theme.is_dark;
    match name {
        "keyword" => Style::new().fg(if dark {
            Color::Rgb(198, 120, 221)
        } else {
            Color::Rgb(166, 38, 164)
        }),
        "string" | "string.special" => Style::new().fg(if dark {
            Color::Rgb(152, 195, 121)
        } else {
            Color::Rgb(80, 161, 79)
        }),
        "comment" => Style::new().fg(theme.text_dim).italic(),
        "number" | "constant" | "constant.builtin" => Style::new().fg(if dark {
            Color::Rgb(209, 154, 102)
        } else {
            Color::Rgb(152, 104, 1)
        }),
        "function" | "function.builtin" => Style::new().fg(if dark {
            Color::Rgb(97, 175, 239)
        } else {
            Color::Rgb(64, 120, 242)
        }),
        "function.macro" => Style::new().fg(if dark {
            Color::Rgb(86, 182, 194)
        } else {
            Color::Rgb(1, 132, 188)
        }),
        "type" | "type.builtin" | "constructor" => Style::new().fg(if dark {
            Color::Rgb(229, 192, 123)
        } else {
            Color::Rgb(152, 104, 1)
        }),
        "variable.builtin" => Style::new().fg(if dark {
            Color::Rgb(224, 108, 117)
        } else {
            Color::Rgb(166, 38, 164)
        }),
        "property" | "property.builtin" => Style::new().fg(if dark {
            Color::Rgb(97, 175, 239)
        } else {
            Color::Rgb(64, 120, 242)
        }),
        "tag" => Style::new().fg(if dark {
            Color::Rgb(224, 108, 117)
        } else {
            Color::Rgb(166, 38, 164)
        }),
        "attribute" => Style::new().fg(if dark {
            Color::Rgb(209, 154, 102)
        } else {
            Color::Rgb(152, 104, 1)
        }),
        "module" | "embedded" | "operator" | "variable" | "variable.parameter" => {
            Style::new().fg(theme.text)
        }
        "punctuation" | "punctuation.bracket" | "punctuation.delimiter" | "punctuation.special" => {
            Style::new().fg(theme.text_dim)
        }
        _ => Style::new().fg(theme.text),
    }
}

/// Highlight source code using tree-sitter.
///
/// Returns `Some(lines)` where each line is a `Vec<(text, style)>` of
/// styled segments, or `None` if:
/// - The language is not recognised
/// - The corresponding `syntax-*` feature is not enabled
/// - Parsing fails
///
/// Callers should fall back to the built-in keyword highlighter when
/// `None` is returned.
///
/// # Example
///
/// ```ignore
/// let lines = slt::syntax::highlight_code("let x = 1;", "rust", &theme);
/// ```
#[allow(unused_variables)]
pub fn highlight_code(code: &str, lang: &str, theme: &Theme) -> Option<Vec<Vec<(String, Style)>>> {
    #[cfg(any(
        feature = "syntax-rust",
        feature = "syntax-python",
        feature = "syntax-javascript",
        feature = "syntax-typescript",
        feature = "syntax-go",
        feature = "syntax-bash",
        feature = "syntax-json",
        feature = "syntax-toml",
        feature = "syntax-c",
        feature = "syntax-cpp",
        feature = "syntax-java",
        feature = "syntax-ruby",
        feature = "syntax-css",
        feature = "syntax-html",
        feature = "syntax-yaml",
    ))]
    {
        use tree_sitter_highlight::{HighlightEvent, Highlighter};

        let config = get_config(lang)?;
        let mut highlighter = Highlighter::new();
        let highlights = highlighter
            .highlight(config, code.as_bytes(), None, |_| None)
            .ok()?;

        let default_style = Style::new().fg(theme.text);
        let mut result: Vec<Vec<(String, Style)>> = Vec::new();
        let mut current_line: Vec<(String, Style)> = Vec::new();
        let mut style_stack: Vec<Style> = vec![default_style];

        for event in highlights {
            match event.ok()? {
                HighlightEvent::Source { start, end } => {
                    let text = &code[start..end];
                    let style = *style_stack.last().unwrap_or(&default_style);
                    // Split by newlines to produce per-line segments
                    for (i, part) in text.split('\n').enumerate() {
                        if i > 0 {
                            result.push(std::mem::take(&mut current_line));
                        }
                        if !part.is_empty() {
                            current_line.push((part.to_string(), style));
                        }
                    }
                }
                HighlightEvent::HighlightStart(highlight) => {
                    let name = HIGHLIGHT_NAMES.get(highlight.0).copied().unwrap_or("");
                    let style = highlight_name_to_style(name, theme);
                    style_stack.push(style);
                }
                HighlightEvent::HighlightEnd => {
                    style_stack.pop();
                }
            }
        }

        if !current_line.is_empty() {
            result.push(current_line);
        }

        Some(result)
    }

    #[cfg(not(any(
        feature = "syntax-rust",
        feature = "syntax-python",
        feature = "syntax-javascript",
        feature = "syntax-typescript",
        feature = "syntax-go",
        feature = "syntax-bash",
        feature = "syntax-json",
        feature = "syntax-toml",
        feature = "syntax-c",
        feature = "syntax-cpp",
        feature = "syntax-java",
        feature = "syntax-ruby",
        feature = "syntax-css",
        feature = "syntax-html",
        feature = "syntax-yaml",
    )))]
    {
        None
    }
}

/// Returns `true` if tree-sitter highlighting is available for `lang`.
///
/// This checks both that the corresponding `syntax-*` feature is enabled
/// and that the language string is recognised.
#[allow(unused_variables)]
pub fn is_language_supported(lang: &str) -> bool {
    #[cfg(any(
        feature = "syntax-rust",
        feature = "syntax-python",
        feature = "syntax-javascript",
        feature = "syntax-typescript",
        feature = "syntax-go",
        feature = "syntax-bash",
        feature = "syntax-json",
        feature = "syntax-toml",
        feature = "syntax-c",
        feature = "syntax-cpp",
        feature = "syntax-java",
        feature = "syntax-ruby",
        feature = "syntax-css",
        feature = "syntax-html",
        feature = "syntax-yaml",
    ))]
    {
        get_config(lang).is_some()
    }
    #[cfg(not(any(
        feature = "syntax-rust",
        feature = "syntax-python",
        feature = "syntax-javascript",
        feature = "syntax-typescript",
        feature = "syntax-go",
        feature = "syntax-bash",
        feature = "syntax-json",
        feature = "syntax-toml",
        feature = "syntax-c",
        feature = "syntax-cpp",
        feature = "syntax-java",
        feature = "syntax-ruby",
        feature = "syntax-css",
        feature = "syntax-html",
        feature = "syntax-yaml",
    )))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Theme;

    #[test]
    fn highlight_returns_none_for_unknown_lang() {
        let theme = Theme::dark();
        assert!(highlight_code("let x = 1;", "brainfuck", &theme).is_none());
    }

    #[test]
    fn is_language_supported_unknown() {
        assert!(!is_language_supported("haskell"));
    }

    #[cfg(feature = "syntax-rust")]
    #[test]
    fn highlight_rust_basic() {
        let theme = Theme::dark();
        let result = highlight_code("let x = 1;", "rust", &theme);
        assert!(result.is_some());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 1);
        // "let" should be in the first line's segments
        let flat: String = lines[0].iter().map(|(t, _)| t.as_str()).collect();
        assert!(flat.contains("let"));
        assert!(flat.contains("1"));
    }

    #[cfg(feature = "syntax-rust")]
    #[test]
    fn highlight_rust_multiline() {
        let theme = Theme::dark();
        let code = "fn main() {\n    println!(\"hello\");\n}";
        let result = highlight_code(code, "rust", &theme).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[cfg(feature = "syntax-rust")]
    #[test]
    fn highlight_rust_rs_alias() {
        let theme = Theme::dark();
        assert!(highlight_code("let x = 1;", "rs", &theme).is_some());
    }

    #[cfg(feature = "syntax-python")]
    #[test]
    fn highlight_python_basic() {
        let theme = Theme::dark();
        let result = highlight_code("def foo():\n    return 42", "python", &theme);
        assert!(result.is_some());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 2);
    }

    #[cfg(feature = "syntax-javascript")]
    #[test]
    fn highlight_javascript_basic() {
        let theme = Theme::dark();
        let result = highlight_code("const x = () => 42;", "js", &theme);
        assert!(result.is_some());
    }

    #[cfg(feature = "syntax-bash")]
    #[test]
    fn highlight_bash_basic() {
        let theme = Theme::dark();
        let result = highlight_code("echo \"hello\"", "sh", &theme);
        assert!(result.is_some());
    }

    #[cfg(feature = "syntax-json")]
    #[test]
    fn highlight_json_basic() {
        let theme = Theme::dark();
        let result = highlight_code("{\"key\": 42}", "json", &theme);
        assert!(result.is_some());
    }

    #[cfg(feature = "syntax-toml")]
    #[test]
    fn highlight_toml_basic() {
        let theme = Theme::dark();
        let result = highlight_code("[package]\nname = \"slt\"", "toml", &theme);
        assert!(result.is_some());
    }

    #[cfg(feature = "syntax-go")]
    #[test]
    fn highlight_go_basic() {
        let theme = Theme::dark();
        let result = highlight_code("package main\nfunc main() {}", "go", &theme);
        assert!(result.is_some());
    }

    #[cfg(feature = "syntax-rust")]
    #[test]
    fn highlight_light_theme_differs() {
        let dark = Theme::dark();
        let light = Theme::light();
        let dark_result = highlight_code("let x = 1;", "rust", &dark).unwrap();
        let light_result = highlight_code("let x = 1;", "rust", &light).unwrap();
        // Keyword styles should differ between dark and light
        let dark_styles: Vec<Style> = dark_result[0].iter().map(|(_, s)| *s).collect();
        let light_styles: Vec<Style> = light_result[0].iter().map(|(_, s)| *s).collect();
        assert_ne!(dark_styles, light_styles);
    }

    #[cfg(feature = "syntax-rust")]
    #[test]
    fn highlight_incomplete_code_does_not_panic() {
        let theme = Theme::dark();
        let result = highlight_code("fn main( {", "rust", &theme);
        assert!(result.is_some());
    }

    #[cfg(feature = "syntax-c")]
    #[test]
    fn highlight_c_basic() {
        let theme = Theme::dark();
        assert!(
            highlight_code("#include <stdio.h>\nint main() { return 0; }", "c", &theme).is_some()
        );
    }

    #[cfg(feature = "syntax-cpp")]
    #[test]
    fn highlight_cpp_basic() {
        let theme = Theme::dark();
        assert!(highlight_code("class Foo { public: void bar(); };", "cpp", &theme).is_some());
    }

    #[cfg(feature = "syntax-typescript")]
    #[test]
    fn highlight_typescript_basic() {
        let theme = Theme::dark();
        assert!(highlight_code("const x: number = 42;", "ts", &theme).is_some());
    }

    #[cfg(feature = "syntax-typescript")]
    #[test]
    fn highlight_tsx_basic() {
        let theme = Theme::dark();
        assert!(highlight_code("const App = () => <div>hello</div>;", "tsx", &theme).is_some());
    }

    #[cfg(feature = "syntax-java")]
    #[test]
    fn highlight_java_basic() {
        let theme = Theme::dark();
        assert!(highlight_code(
            "public class Main { public static void main(String[] args) {} }",
            "java",
            &theme
        )
        .is_some());
    }

    #[cfg(feature = "syntax-ruby")]
    #[test]
    fn highlight_ruby_basic() {
        let theme = Theme::dark();
        assert!(highlight_code("def hello\n  puts 'world'\nend", "ruby", &theme).is_some());
    }

    #[cfg(feature = "syntax-css")]
    #[test]
    fn highlight_css_basic() {
        let theme = Theme::dark();
        assert!(highlight_code("body { color: red; }", "css", &theme).is_some());
    }

    #[cfg(feature = "syntax-html")]
    #[test]
    fn highlight_html_basic() {
        let theme = Theme::dark();
        assert!(highlight_code("<div class=\"test\">hello</div>", "html", &theme).is_some());
    }

    #[cfg(feature = "syntax-yaml")]
    #[test]
    fn highlight_yaml_basic() {
        let theme = Theme::dark();
        assert!(highlight_code("name: slt\nversion: 0.14", "yaml", &theme).is_some());
    }
}
