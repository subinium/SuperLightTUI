//! Issue #466: keystroke-to-render latency of `Context::code_editor` on a
//! 2,000-line Rust file.
//!
//! Run highlighted (the configuration the issue's < 1 ms goal is about):
//!
//! ```text
//! cargo bench --bench code_editor --features syntax-rust
//! ```
//!
//! Without `syntax-rust` the same benches measure the plain-text fallback.
//! Before the criterion groups run, a percentile pass prints the median and
//! p99 of single keystroke frames.

use criterion::Criterion;
use slt::{CodeEditorState, EventBuilder, KeyCode, TestBackend};
use std::hint::black_box;
use std::time::{Duration, Instant};

const LINES: usize = 2_000;
const WIDTH: u32 = 120;
const HEIGHT: u32 = 40;
const PERCENTILE_SAMPLES: usize = 2_000;

/// A realistic Rust file: documented functions with nested blocks, plus a
/// struct and impl every fifth item.
fn rust_source(lines: usize) -> String {
    let mut out: Vec<String> = Vec::with_capacity(lines + 32);
    let mut item = 0usize;
    while out.len() < lines {
        out.push(format!("/// Compute value number {item}."));
        out.push(format!(
            "pub fn compute_{item}(input: &[u64], scale: f64) -> Result<u64, String> {{"
        ));
        out.push("    let mut total: u64 = 0;".into());
        out.push("    for (index, value) in input.iter().enumerate() {".into());
        out.push(format!("        if index % {} == 0 {{", item % 7 + 2));
        out.push("            total = total.wrapping_add(*value * 2);".into());
        out.push("        } else {".into());
        out.push("            total += value; // accumulate".into());
        out.push("        }".into());
        out.push("    }".into());
        out.push(format!("    let label = \"result-{item}\";"));
        out.push("    println!(\"{label}: {total} ({scale:.2})\");".into());
        out.push("    Ok(total)".into());
        out.push("}".into());
        out.push(String::new());
        if item % 5 == 0 {
            out.push("#[derive(Debug, Clone, Default)]".into());
            out.push(format!("pub struct Record{item} {{"));
            out.push("    pub id: u32,".into());
            out.push("    pub name: String,".into());
            out.push("}".into());
            out.push(String::new());
            out.push(format!("impl Record{item} {{"));
            out.push("    pub fn new(id: u32) -> Self {".into());
            out.push("        Self { id, name: String::new() }".into());
            out.push("    }".into());
            out.push("}".into());
            out.push(String::new());
        }
        item += 1;
    }
    out.truncate(lines);
    out.join("\n")
}

/// An editor on the 2,000-line file, focused and rendered once, with the
/// cursor in the middle of a line in the middle of the file.
fn setup() -> (TestBackend, CodeEditorState) {
    let source = rust_source(LINES);
    let mut editor = CodeEditorState::new(source).lang("rust");
    let row = (LINES / 2..LINES)
        .find(|&row| editor.lines()[row].starts_with("            total += value"))
        .expect("generated source has the target line");
    editor.set_cursor(row, 20);
    let mut tb = TestBackend::new(WIDTH, HEIGHT);
    tb.render(|ui| {
        let _ = ui.code_editor(&mut editor).show();
    });
    (tb, editor)
}

/// One frame carrying one keystroke. Alternates an insert and a Backspace so
/// the document stays the same size across iterations.
fn keystroke(tb: &mut TestBackend, editor: &mut CodeEditorState, insert: bool) {
    let code = if insert {
        KeyCode::Char('x')
    } else {
        KeyCode::Backspace
    };
    tb.sequence()
        .events(EventBuilder::new().key_code(code).build(), |ui| {
            black_box(ui.code_editor(editor).show());
        })
        .run();
}

fn mode() -> &'static str {
    if cfg!(feature = "syntax-rust") {
        "highlighted"
    } else {
        "plain"
    }
}

fn percentile_report() {
    let (mut tb, mut editor) = setup();
    for i in 0..200 {
        keystroke(&mut tb, &mut editor, i % 2 == 0);
    }
    let mut samples: Vec<Duration> = (0..PERCENTILE_SAMPLES)
        .map(|i| {
            let start = Instant::now();
            keystroke(&mut tb, &mut editor, i % 2 == 0);
            start.elapsed()
        })
        .collect();
    samples.sort_unstable();
    let at = |q: f64| samples[((samples.len() - 1) as f64 * q).round() as usize];
    println!(
        "code_editor keystroke ({}, {LINES} lines, {WIDTH}x{HEIGHT}, n={}): \
         median {:?}, p99 {:?}, max {:?}",
        mode(),
        samples.len(),
        at(0.5),
        at(0.99),
        samples[samples.len() - 1],
    );
}

fn bench_keystroke(c: &mut Criterion) {
    let (mut tb, mut editor) = setup();
    let mut insert = true;
    c.bench_function(
        &format!("code_editor_keystroke_2000_lines_{}", mode()),
        |b| {
            b.iter(|| {
                keystroke(&mut tb, &mut editor, insert);
                insert = !insert;
            });
        },
    );
}

fn bench_idle(c: &mut Criterion) {
    let (mut tb, mut editor) = setup();
    c.bench_function(
        &format!("code_editor_idle_frame_2000_lines_{}", mode()),
        |b| {
            b.iter(|| {
                tb.render(|ui| {
                    black_box(ui.code_editor(&mut editor).show());
                });
            });
        },
    );
}

/// Reference point: what re-highlighting the whole document per keystroke
/// (the naive approach the editor avoids) costs.
#[cfg(feature = "syntax-rust")]
fn bench_full_rehighlight(c: &mut Criterion) {
    let source = rust_source(LINES);
    let theme = slt::Theme::dark();
    let mut counter = 0u64;
    c.bench_function("full_document_highlight_2000_lines", |b| {
        b.iter(|| {
            // A fresh suffix defeats the shared preparation cache, as an
            // edit would.
            counter += 1;
            let code = format!("{source}\n// {counter}");
            black_box(slt::syntax::highlight_code(&code, "rust", &theme));
        });
    });
}

/// Reference point: the same document and keystrokes in a plain `textarea`
/// (the editing core the editor reuses), without gutter or highlighting.
fn bench_textarea_baseline(c: &mut Criterion) {
    let (mut tb, editor) = setup();
    let mut area = slt::TextareaState::new();
    area.set_value(editor.value());
    (area.cursor_row, area.cursor_col) = editor.cursor();
    tb.render(|ui| {
        let _ = ui.textarea(&mut area, HEIGHT);
    });
    let mut insert = true;
    c.bench_function("textarea_keystroke_2000_lines", |b| {
        b.iter(|| {
            let code = if insert {
                KeyCode::Char('x')
            } else {
                KeyCode::Backspace
            };
            insert = !insert;
            tb.sequence()
                .events(EventBuilder::new().key_code(code).build(), |ui| {
                    black_box(ui.textarea(&mut area, HEIGHT));
                })
                .run();
        });
    });
}

fn main() {
    percentile_report();
    let mut criterion = Criterion::default().configure_from_args();
    bench_keystroke(&mut criterion);
    bench_idle(&mut criterion);
    bench_textarea_baseline(&mut criterion);
    #[cfg(feature = "syntax-rust")]
    bench_full_rehighlight(&mut criterion);
    criterion.final_summary();
}
