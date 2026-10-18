//! The `code_block` builder: syntax-highlighted code with an optional
//! line-number gutter, rendered on `show()` or drop.

use super::*;

impl Context {
    /// Begin building a syntax-highlighted code block.
    ///
    /// Chain `.lang(...)` for language-aware highlighting and `.numbered()`
    /// for a line-number gutter. The returned [`CodeBlock`] auto-renders when
    /// dropped, so a bare `ui.code_block(code);` produces a default block.
    /// Call `.show()` (instead of dropping) to capture the [`Response`].
    ///
    /// This is the consuming-builder shape shared with [`Context::gauge`] /
    /// [`Context::breadcrumb`] — see the [API review rules](https://github.com/subinium/superlighttui/blob/main/docs/internals/design-principles.md#api-review-rules), Rule 1.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.code_block("let x = 1;");
    /// let r = ui.code_block("fn main() {}").lang("rust").numbered().show();
    /// if r.hovered { /* attach tooltip */ }
    /// # });
    /// ```
    pub fn code_block<'a>(&'a mut self, code: &'a str) -> CodeBlock<'a> {
        CodeBlock::new(self, code)
    }
}

/// Syntax-highlighted code block builder. Auto-renders on `Drop`.
///
/// Constructed via [`Context::code_block`]. Chain `.lang(...)` for
/// language-aware highlighting and `.numbered()` for a line-number gutter.
/// Drop the value to render without capturing a response, or call
/// [`Self::show`] to render and obtain a [`Response`].
///
/// Consuming-builder shape, mirroring [`Gauge`](super::Gauge) /
/// [`Breadcrumb`]: `Drop` is intentional so `ui.code_block(code);` is the
/// idiomatic form when the response isn't needed (egui's `ui.add(...)` idiom).
pub struct CodeBlock<'a> {
    ctx: Option<&'a mut Context>,
    code: &'a str,
    lang: &'a str,
    numbered: bool,
}

#[expect(
    clippy::return_self_not_must_use,
    reason = "the widget call already recorded the element (or renders on drop); modifiers edit it in place, so a statement-terminated chain is the intended use"
)]
impl<'a> CodeBlock<'a> {
    fn new(ctx: &'a mut Context, code: &'a str) -> Self {
        Self {
            ctx: Some(ctx),
            code,
            lang: "",
            numbered: false,
        }
    }

    /// Set the language for syntax highlighting (e.g. `"rust"`). Empty string
    /// (the default) falls back to keyword-based highlighting.
    pub fn lang(mut self, lang: &'a str) -> Self {
        self.lang = lang;
        self
    }

    /// Enable the line-number gutter.
    pub fn numbered(mut self) -> Self {
        self.numbered = true;
        self
    }

    /// Render now and return the [`Response`].
    ///
    /// # Panics
    ///
    /// Does not panic: `show` consumes the builder, so the context is taken
    /// exactly once.
    pub fn show(mut self) -> Response {
        // SAFETY: ctx is Some until Drop runs; show consumes self before Drop.
        let ctx = self.ctx.take().expect("CodeBlock::show called twice");
        render_code_block(ctx, self.code, self.lang, self.numbered)
    }
}

impl Drop for CodeBlock<'_> {
    fn drop(&mut self) {
        if let Some(ctx) = self.ctx.take() {
            let _ = render_code_block(ctx, self.code, self.lang, self.numbered);
        }
    }
}

/// Internal code-block rendering shared by the [`CodeBlock`] builder and the
/// deprecated `code_block_*` aliases. Folds the language-aware and
/// line-numbered paths on the `numbered` flag — no behavior change versus the
/// previous separate `code_block_lang` / `code_block_numbered_lang` bodies.
fn render_code_block(ctx: &mut Context, code: &str, lang: &str, numbered: bool) -> Response {
    if code.is_empty() {
        return Response::none();
    }

    let theme = ctx.theme;
    let pad = theme.spacing.xs();
    let highlighted = crate::syntax::highlight_code_cached(code, lang, &theme);

    if numbered {
        let lines: Vec<&str> = code.lines().collect();
        let gutter_w = (lines.len().max(1).ilog10() + 1) as usize;
        ctx.bordered(Border::Rounded)
            .bg(theme.surface)
            .p(pad)
            .col(|ui| {
                if let Some(ref hl_lines) = highlighted {
                    for (i, segs) in hl_lines.iter().enumerate() {
                        ui.line(|ui| {
                            ui.text(format!("{:>gutter_w$} │ ", i + 1))
                                .fg(theme.text_dim);
                            for (text, style) in segs {
                                ui.styled(text, *style);
                            }
                        });
                    }
                } else {
                    for (i, line) in lines.iter().enumerate() {
                        ui.line(|ui| {
                            ui.text(format!("{:>gutter_w$} │ ", i + 1))
                                .fg(theme.text_dim);
                            render_highlighted_line(ui, line);
                        });
                    }
                }
            })
    } else {
        ctx.bordered(Border::Rounded)
            .bg(theme.surface)
            .p(pad)
            .col(|ui| {
                if let Some(ref lines) = highlighted {
                    render_tree_sitter_lines(ui, lines);
                } else {
                    for line in code.lines() {
                        ui.line(|ui| render_highlighted_line(ui, line));
                    }
                }
            })
    }
}

#[cfg(test)]
mod code_block_tests {
    use crate::test_utils::TestBackend;
    use crate::{Rect, Response};

    #[test]
    fn code_block_builder_renders_lang_and_gutter() {
        let mut tb = TestBackend::new(40, 8);
        tb.render(|ui| {
            let _ = ui.code_block("let x = 1;").lang("rust").numbered().show();
        });
        tb.assert_contains("let");
        // Line-number gutter from the numbered path (`status.rs` render).
        tb.assert_contains("1 │");
    }

    #[test]
    fn code_block_default_drop_renders() {
        // Bare drop-render (no chain) must produce the same content as `.show()`.
        let mut tb_drop = TestBackend::new(40, 8);
        tb_drop.render(|ui| {
            ui.code_block("a\nb");
        });
        let mut tb_show = TestBackend::new(40, 8);
        tb_show.render(|ui| {
            let _ = ui.code_block("a\nb").show();
        });
        assert_eq!(tb_drop.to_string(), tb_show.to_string());
    }

    #[test]
    fn code_block_show_returns_outer_warm_frame_response() {
        let mut backend = TestBackend::new(30, 8);
        let mut response = Response::none();
        backend.render(|ui| {
            response = ui.code_block("let x = 1;").lang("rust").show();
        });
        assert_eq!(response.rect, Rect::default());

        backend.render(|ui| {
            response = ui.code_block("let x = 1;").lang("rust").show();
        });
        assert!(response.rect.width > 0);
        assert!(response.rect.height > 0);
    }

    #[test]
    fn empty_code_block_returns_none_without_rendering() {
        let mut backend = TestBackend::new(30, 8);
        let mut response = Response::none();
        backend.render(|ui| {
            response = ui.code_block("").show();
        });
        assert_eq!(response.rect, Rect::default());
        assert_eq!(backend.to_string().trim(), "");
    }

    #[test]
    fn code_block_deprecated_alias_byte_identical() {
        let code = "fn main() {}\nlet y = 2;";
        let mut tb_builder = TestBackend::new(40, 8);
        tb_builder.render(|ui| {
            let _ = ui.code_block(code).lang("rust").numbered().show();
        });
        let mut tb_alias = TestBackend::new(40, 8);
        tb_alias.render(|ui| {
            let _ = ui.code_block(code).lang("rust").numbered();
        });
        assert_eq!(
            tb_builder.to_string(),
            tb_alias.to_string(),
            "deprecated alias must be behavior-preserving"
        );
    }
}
