//! Tree-sitter based syntax highlighting.
//!
//! When one of the `syntax-*` features is enabled,
//! [`crate::syntax::highlight_code`] uses tree-sitter grammars for accurate,
//! language-aware highlighting. Without those features the function always
//! returns `None` so callers can fall back to the built-in keyword
//! highlighter.

use crate::style::{Style, Theme};

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
use crate::style::SyntaxPalette;

type HighlightedLines = Vec<Vec<(String, Style)>>;

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
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                HighlightConfiguration::new(
                    tree_sitter_rust::LANGUAGE.into(),
                    "rust",
                    tree_sitter_rust::HIGHLIGHTS_QUERY,
                    tree_sitter_rust::INJECTIONS_QUERY,
                    "",
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-python")]
        "python" | "py" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                HighlightConfiguration::new(
                    tree_sitter_python::LANGUAGE.into(),
                    "python",
                    tree_sitter_python::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-javascript")]
        "javascript" | "js" | "jsx" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                HighlightConfiguration::new(
                    tree_sitter_javascript::LANGUAGE.into(),
                    "javascript",
                    tree_sitter_javascript::HIGHLIGHT_QUERY,
                    tree_sitter_javascript::INJECTIONS_QUERY,
                    tree_sitter_javascript::LOCALS_QUERY,
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-go")]
        "go" | "golang" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                HighlightConfiguration::new(
                    tree_sitter_go::LANGUAGE.into(),
                    "go",
                    tree_sitter_go::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-bash")]
        "bash" | "sh" | "shell" | "zsh" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                HighlightConfiguration::new(
                    tree_sitter_bash::LANGUAGE.into(),
                    "bash",
                    tree_sitter_bash::HIGHLIGHT_QUERY,
                    "",
                    "",
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-json")]
        "json" | "jsonc" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                HighlightConfiguration::new(
                    tree_sitter_json::LANGUAGE.into(),
                    "json",
                    tree_sitter_json::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-toml")]
        "toml" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                HighlightConfiguration::new(
                    tree_sitter_toml_ng::LANGUAGE.into(),
                    "toml",
                    tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-c")]
        "c" | "h" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                HighlightConfiguration::new(
                    tree_sitter_c::LANGUAGE.into(),
                    "c",
                    tree_sitter_c::HIGHLIGHT_QUERY,
                    "",
                    "",
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-cpp")]
        "cpp" | "c++" | "cxx" | "cc" | "hpp" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
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

                HighlightConfiguration::new(
                    tree_sitter_cpp::LANGUAGE.into(),
                    "cpp",
                    &highlights,
                    "",
                    "",
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-typescript")]
        "typescript" | "ts" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                let highlights = [
                    tree_sitter_javascript::HIGHLIGHT_QUERY,
                    tree_sitter_typescript::HIGHLIGHTS_QUERY,
                ]
                .join("\n");
                HighlightConfiguration::new(
                    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                    "typescript",
                    &highlights,
                    "",
                    tree_sitter_typescript::LOCALS_QUERY,
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-typescript")]
        "tsx" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                let highlights = [
                    tree_sitter_javascript::HIGHLIGHT_QUERY,
                    tree_sitter_javascript::JSX_HIGHLIGHT_QUERY,
                    tree_sitter_typescript::HIGHLIGHTS_QUERY,
                ]
                .join("\n");
                HighlightConfiguration::new(
                    tree_sitter_typescript::LANGUAGE_TSX.into(),
                    "tsx",
                    &highlights,
                    "",
                    tree_sitter_typescript::LOCALS_QUERY,
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-java")]
        "java" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                HighlightConfiguration::new(
                    tree_sitter_java::LANGUAGE.into(),
                    "java",
                    tree_sitter_java::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-ruby")]
        "ruby" | "rb" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                HighlightConfiguration::new(
                    tree_sitter_ruby::LANGUAGE.into(),
                    "ruby",
                    tree_sitter_ruby::HIGHLIGHTS_QUERY,
                    "",
                    tree_sitter_ruby::LOCALS_QUERY,
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-css")]
        "css" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                HighlightConfiguration::new(
                    tree_sitter_css::LANGUAGE.into(),
                    "css",
                    tree_sitter_css::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-html")]
        "html" | "htm" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                HighlightConfiguration::new(
                    tree_sitter_html::LANGUAGE.into(),
                    "html",
                    tree_sitter_html::HIGHLIGHTS_QUERY,
                    tree_sitter_html::INJECTIONS_QUERY,
                    "",
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        #[cfg(feature = "syntax-yaml")]
        "yaml" | "yml" => {
            static CFG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CFG.get_or_init(|| {
                HighlightConfiguration::new(
                    tree_sitter_yaml::LANGUAGE.into(),
                    "yaml",
                    tree_sitter_yaml::HIGHLIGHTS_QUERY,
                    "",
                    "",
                )
                .ok()
                .map(|mut c| {
                    c.configure(HIGHLIGHT_NAMES);
                    c
                })
            })
            .as_ref()
        }

        _ => None,
    }
}

/// Map a tree-sitter highlight capture name to an SLT [`Style`].
///
/// Colorful tokens resolve through the active theme's
/// [`SyntaxPalette`](crate::SyntaxPalette) (`theme.syntax.*`), so code blocks
/// adopt the selected theme instead of a hardcoded scheme. Neutral tokens
/// (comments, operators, plain variables, punctuation) resolve through
/// [`Theme::text`] / [`Theme::text_dim`].
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
    let syntax = &theme.syntax;
    match name {
        "keyword" => Style::new().fg(syntax.keyword),
        "string" | "string.special" => Style::new().fg(syntax.string),
        "comment" => Style::new().fg(theme.text_dim).italic(),
        "number" => Style::new().fg(syntax.number),
        "constant" | "constant.builtin" => Style::new().fg(syntax.constant),
        "function" | "function.builtin" => Style::new().fg(syntax.function),
        "function.macro" => Style::new().fg(syntax.macro_),
        "type" | "type.builtin" | "constructor" => Style::new().fg(syntax.type_),
        "variable.builtin" => Style::new().fg(syntax.tag),
        "property" | "property.builtin" => Style::new().fg(syntax.property),
        "tag" => Style::new().fg(syntax.tag),
        "attribute" => Style::new().fg(syntax.constant),
        "module" | "embedded" | "operator" | "variable" | "variable.parameter" => {
            Style::new().fg(theme.text)
        }
        "punctuation" | "punctuation.bracket" | "punctuation.delimiter" | "punctuation.special" => {
            Style::new().fg(theme.text_dim)
        }
        _ => Style::new().fg(theme.text),
    }
}

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
thread_local! {
    // SAFETY: SLT runs a single-threaded synchronous event loop.
    // Re-entrant highlight calls are architecturally impossible.
    // If an async runtime is added later, revisit this (see issue #113).
    static HIGHLIGHTER: std::cell::RefCell<tree_sitter_highlight::Highlighter> =
        std::cell::RefCell::new(tree_sitter_highlight::Highlighter::new());
}

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
#[derive(Clone, Copy, PartialEq, Eq)]
struct SyntaxThemeKey {
    text: crate::style::Color,
    text_dim: crate::style::Color,
    syntax: SyntaxPalette,
}

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
struct SyntaxCacheEntry {
    content_hash: u64,
    code: Box<str>,
    lang: Box<str>,
    theme: SyntaxThemeKey,
    lines: std::sync::Arc<HighlightedLines>,
}

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
#[derive(Default)]
struct SyntaxCache {
    entries: std::collections::VecDeque<SyntaxCacheEntry>,
    source_bytes: usize,
}

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
impl SyntaxCache {
    const MAX_ENTRIES: usize = 64;
    const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;

    fn get(
        &mut self,
        content_hash: u64,
        code: &str,
        lang: &str,
        theme: SyntaxThemeKey,
    ) -> Option<std::sync::Arc<HighlightedLines>> {
        let index = self.entries.iter().position(|entry| {
            entry.content_hash == content_hash
                && entry.code.as_ref() == code
                && entry.lang.as_ref() == lang
                && entry.theme == theme
        })?;
        let entry = self.entries.remove(index)?;
        let lines = std::sync::Arc::clone(&entry.lines);
        self.entries.push_back(entry);
        Some(lines)
    }

    fn insert(&mut self, entry: SyntaxCacheEntry) {
        let source_bytes = entry.code.len().saturating_add(entry.lang.len());
        if source_bytes > Self::MAX_SOURCE_BYTES {
            return;
        }
        while self.entries.len() >= Self::MAX_ENTRIES
            || self.source_bytes.saturating_add(source_bytes) > Self::MAX_SOURCE_BYTES
        {
            let Some(evicted) = self.entries.pop_front() else {
                break;
            };
            self.source_bytes = self
                .source_bytes
                .saturating_sub(evicted.code.len().saturating_add(evicted.lang.len()));
        }
        self.source_bytes = self.source_bytes.saturating_add(source_bytes);
        self.entries.push_back(entry);
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
fn highlight_code_uncached(code: &str, lang: &str, theme: &Theme) -> Option<HighlightedLines> {
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
        use tree_sitter_highlight::HighlightEvent;

        let config = get_config(lang)?;
        let highlights = HIGHLIGHTER.with(|cell| {
            let mut highlighter = cell.borrow_mut();
            highlighter
                .highlight(config, code.as_bytes(), None, |_| None)
                .ok()
                .map(|iter| iter.collect::<Vec<_>>())
        })?;
        let highlights = highlights.into_iter();

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

/// Highlight source code using the active theme, reusing a bounded preparation
/// cache for repeated content/language/theme combinations.
pub fn highlight_code(code: &str, lang: &str, theme: &Theme) -> Option<Vec<Vec<(String, Style)>>> {
    highlight_code_cached(code, lang, theme).map(|lines| lines.as_ref().clone())
}

pub(crate) fn highlight_code_cached(
    code: &str,
    lang: &str,
    theme: &Theme,
) -> Option<std::sync::Arc<HighlightedLines>> {
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
        use std::hash::{Hash, Hasher};
        use std::sync::{Mutex, OnceLock};

        static CACHE: OnceLock<Mutex<SyntaxCache>> = OnceLock::new();
        const MAX_CACHEABLE_CODE_BYTES: usize = 256 * 1024;

        let theme_key = SyntaxThemeKey {
            text: theme.text,
            text_dim: theme.text_dim,
            syntax: theme.syntax,
        };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        code.hash(&mut hasher);
        let content_hash = hasher.finish();

        if code.len() <= MAX_CACHEABLE_CODE_BYTES {
            let cache = CACHE.get_or_init(|| Mutex::new(SyntaxCache::default()));
            let mut cache = cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(lines) = cache.get(content_hash, code, lang, theme_key) {
                return Some(lines);
            }
        }

        let lines = std::sync::Arc::new(highlight_code_uncached(code, lang, theme)?);
        if code.len() <= MAX_CACHEABLE_CODE_BYTES {
            let cache = CACHE.get_or_init(|| Mutex::new(SyntaxCache::default()));
            let mut cache = cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            cache.insert(SyntaxCacheEntry {
                content_hash,
                code: code.into(),
                lang: lang.into(),
                theme: theme_key,
                lines: std::sync::Arc::clone(&lines),
            });
        }
        Some(lines)
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
        let _ = (code, lang, theme);
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
    #![allow(clippy::unwrap_used)]
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
    #[test]
    fn number_and_constant_captures_use_distinct_palette_entries() {
        let mut theme = Theme::dark();
        theme.syntax.number = crate::style::Color::Rgb(1, 2, 3);
        theme.syntax.constant = crate::style::Color::Rgb(4, 5, 6);

        assert_eq!(
            highlight_name_to_style("number", &theme).fg,
            Some(theme.syntax.number)
        );
        assert_eq!(
            highlight_name_to_style("constant", &theme).fg,
            Some(theme.syntax.constant)
        );
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
    fn highlight_keyword_uses_theme_palette() {
        // The `let` keyword should adopt each theme's syntax palette rather
        // than a hardcoded One Dark color.
        let nord = Theme::nord();
        let catppuccin = Theme::catppuccin();

        let kw_fg = |theme: &Theme| -> crate::style::Color {
            let line = highlight_code("let x = 1;", "rust", theme).unwrap();
            line[0]
                .iter()
                .find_map(|(text, style)| (text.as_str() == "let").then_some(style.fg.unwrap()))
                .expect("`let` keyword segment present")
        };

        assert_eq!(kw_fg(&nord), nord.syntax.keyword);
        assert_eq!(kw_fg(&catppuccin), catppuccin.syntax.keyword);
        // The two themes resolve to different keyword colors — proving the
        // old hardcoded One Dark purple is no longer used.
        assert_ne!(nord.syntax.keyword, catppuccin.syntax.keyword);
    }

    #[cfg(feature = "syntax-rust")]
    #[test]
    fn code_block_renders_with_theme_syntax_palette() {
        use crate::style::Theme;
        use crate::test_utils::TestBackend;

        let theme = Theme::tokyo_night();
        let mut tb = TestBackend::new(40, 8);
        tb.render(|ui| {
            ui.set_theme(theme);
            let _ = ui.code_block("fn main() {}").lang("rust").show();
        });

        // The code text still renders.
        tb.assert_contains("fn");
        tb.assert_contains("main");

        // Some keyword cell adopts Tokyo Night's keyword color, and the old
        // hardcoded One Dark purple is absent from the buffer.
        let one_dark_keyword = crate::style::Color::Rgb(198, 120, 221);
        let buffer = tb.buffer();
        let mut saw_theme_keyword = false;
        for y in 0..tb.height() {
            for x in 0..tb.width() {
                let fg = buffer.get(x, y).style.fg;
                assert_ne!(
                    fg,
                    Some(one_dark_keyword),
                    "One Dark keyword color must not appear under Tokyo Night"
                );
                if fg == Some(theme.syntax.keyword) {
                    saw_theme_keyword = true;
                }
            }
        }
        assert!(
            saw_theme_keyword,
            "expected a cell colored with Tokyo Night's keyword color"
        );
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
        let lines = highlight_code("const x: number = 42;", "ts", &theme).unwrap();
        let number = lines
            .iter()
            .flatten()
            .find(|(text, _)| text == "42")
            .expect("numeric capture");
        assert_eq!(number.1.fg, Some(theme.syntax.number));
    }

    #[cfg(feature = "syntax-typescript")]
    #[test]
    fn highlight_tsx_basic() {
        let theme = Theme::dark();
        let lines = highlight_code(
            "const App = () => <div data-count={42}>hello</div>;",
            "tsx",
            &theme,
        )
        .unwrap();
        let flat: String = lines
            .iter()
            .flatten()
            .map(|(text, _)| text.as_str())
            .collect();
        assert!(flat.contains("data-count"));
        assert!(flat.contains("42"));
    }

    #[cfg(feature = "syntax-java")]
    #[test]
    fn highlight_java_basic() {
        let theme = Theme::dark();
        assert!(
            highlight_code(
                "public class Main { public static void main(String[] args) {} }",
                "java",
                &theme
            )
            .is_some()
        );
    }

    #[cfg(feature = "syntax-ruby")]
    #[test]
    fn highlight_ruby_basic() {
        let theme = Theme::dark();
        let lines = highlight_code(
            "answer = 42\ndef hello\n  puts 'world'\nend",
            "ruby",
            &theme,
        )
        .unwrap();
        let flat: String = lines
            .iter()
            .flatten()
            .map(|(text, _)| text.as_str())
            .collect();
        assert!(flat.contains("answer"));
        assert!(flat.contains("42"));
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

    /// Regression test for issue #113:
    /// `highlight_code()` must not panic on repeated calls (thread_local HIGHLIGHTER reuse).
    #[cfg(feature = "syntax-rust")]
    #[test]
    fn highlight_reuse_does_not_panic() {
        let theme = Theme::dark();
        // Call twice with the same language — exercises HIGHLIGHTER.with borrow_mut reuse.
        let first = highlight_code("let x = 1;", "rust", &theme);
        let second = highlight_code("fn foo() {}", "rust", &theme);
        assert!(first.is_some(), "first call should succeed");
        assert!(second.is_some(), "second call should succeed");
    }

    #[cfg(feature = "syntax-rust")]
    #[test]
    fn highlight_preparation_cache_reuses_arc_and_invalidates_on_theme() {
        let dark = Theme::dark();
        let light = Theme::light();
        let first = highlight_code_cached("let cache_probe = 17;", "rust", &dark).unwrap();
        let second = highlight_code_cached("let cache_probe = 17;", "rust", &dark).unwrap();
        let themed = highlight_code_cached("let cache_probe = 17;", "rust", &light).unwrap();

        // Other parallel syntax tests may legitimately evict this bounded
        // global cache between calls. Verify value reuse semantics here and
        // pointer reuse deterministically in the cache-unit test below.
        assert_eq!(first.as_ref(), second.as_ref());
        assert_ne!(first.as_ref(), themed.as_ref());
    }

    #[cfg(feature = "syntax-rust")]
    #[test]
    fn syntax_cache_returns_the_stored_arc() {
        let theme = Theme::dark();
        let theme_key = SyntaxThemeKey {
            text: theme.text,
            text_dim: theme.text_dim,
            syntax: theme.syntax,
        };
        let lines = std::sync::Arc::new(vec![vec![(
            "cached".to_string(),
            Style::new().fg(theme.text),
        )]]);
        let mut cache = SyntaxCache::default();
        cache.insert(SyntaxCacheEntry {
            content_hash: 7,
            code: "probe".into(),
            lang: "rust".into(),
            theme: theme_key,
            lines: std::sync::Arc::clone(&lines),
        });

        let cached = cache.get(7, "probe", "rust", theme_key).unwrap();
        assert!(std::sync::Arc::ptr_eq(&lines, &cached));
    }

    /// Regression test for issue #113:
    /// Multiple calls across different languages must all return Some.
    #[cfg(all(feature = "syntax-rust", feature = "syntax-python"))]
    #[test]
    fn highlight_reuse_across_languages() {
        let theme = Theme::dark();
        let r1 = highlight_code("let x = 1;", "rust", &theme);
        let r2 = highlight_code("def foo(): pass", "python", &theme);
        let r3 = highlight_code("fn bar() {}", "rust", &theme);
        assert!(r1.is_some());
        assert!(r2.is_some());
        assert!(r3.is_some());
    }
}
