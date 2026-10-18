//! Code editor demo — edit a Rust file with highlighting, a line-number
//! gutter, auto-indent and undo.
//!
//! Run: `cargo run --example code_editor`
//! With Rust highlighting: `cargo run --example code_editor --features syntax-rust`
//!
//! Keys:
//!   type / arrows / Home / End / PageUp / PageDown — edit and move
//!   Shift + movement, mouse drag                     — select
//!   Tab / Shift+Tab                                  — indent / dedent (selection-aware)
//!   Enter                                            — new line keeping the indentation
//!   Ctrl+Z / Ctrl+Y                                  — undo / redo
//!   Ctrl+C / Ctrl+X                                  — copy / cut the selection
//!   Ctrl+W                                           — toggle soft wrap
//!   Ctrl+L                                           — toggle line numbers
//!   Esc                                              — quit

use slt::{Border, CodeEditorState, Context, KeyCode, KeyModifiers, RunConfig};

const SOURCE: &str = r#"use std::collections::HashMap;

/// Count how often each word appears in `text`.
pub fn word_counts(text: &str) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for word in text.split_whitespace() {
        let word = word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
        if word.is_empty() {
            continue;
        }
        *counts.entry(word).or_insert(0) += 1;
    }
    counts
}

fn main() {
    let text = "the quick brown fox jumps over the lazy dog, the end";
    let mut counts: Vec<_> = word_counts(text).into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (word, count) in counts.iter().take(5) {
        println!("{word:>8} {count}");
    }
}
"#;

fn main() -> std::io::Result<()> {
    let mut editor = CodeEditorState::new(SOURCE).lang("rust").tab_width(4);
    let mut wrap = false;
    let mut line_numbers = true;
    let mut edits = 0usize;

    slt::run_with(RunConfig::default().mouse(true), move |ui: &mut Context| {
        if ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        if ui.key_mod('w', KeyModifiers::CONTROL) {
            wrap = !wrap;
        }
        if ui.key_mod('l', KeyModifiers::CONTROL) {
            line_numbers = !line_numbers;
        }

        ui.bordered(Border::Rounded)
            .title("main.rs")
            .p(1)
            .grow(1)
            .col(|ui: &mut Context| {
                let rows = ui.height().saturating_sub(6);
                let response = ui
                    .code_editor(&mut editor)
                    .line_numbers(line_numbers)
                    .wrap(wrap)
                    .h(rows)
                    .show();
                if response.changed {
                    edits += 1;
                }

                let (row, col) = editor.cursor();
                let highlight = if slt::syntax::is_language_supported("rust") {
                    "rust"
                } else {
                    "plain (enable syntax-rust)"
                };
                ui.text(format!(
                    "Ln {}, Col {} · {} lines · {edits} edits · {highlight} · wrap {}",
                    row + 1,
                    col + 1,
                    editor.line_count(),
                    if wrap { "on" } else { "off" },
                ))
                .dim();
                ui.text(
                    "Tab indent · Shift+Tab dedent · Ctrl+Z/Y undo/redo · Ctrl+W wrap · Esc quit",
                )
                .dim();
            });
    })
}
