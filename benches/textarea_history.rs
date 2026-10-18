//! Issue #415: textarea editing cost on large documents — undo recording,
//! undo/redo, large pastes and soft-wrapped idle frames.
//!
//! ```text
//! cargo bench --bench textarea_history
//! ```
//!
//! Every edit bench is one full 80x24 frame (input handling, undo recording,
//! `changed` detection and rendering) on a fresh clone of a prepared state;
//! the clone is excluded from the timing. Prepared states already hold one
//! undo step so recording is measured in its steady state, not as the first
//! snapshot. Release p50/p95 latency and allocation counts for the full
//! paste matrix come from
//! `SLT_AUDIT_FILTER=textarea_paste cargo bench --bench alloc_diagnostics`.

use criterion::{BatchSize, Criterion};
use slt::{EventBuilder, KeyCode, KeyModifiers, TestBackend, TextareaState};
use std::hint::black_box;
use std::time::Duration;

const ROWS: u32 = 23;

/// About `bytes` of 40-glyph lines.
fn block(glyph: &str, bytes: usize) -> String {
    let row = format!("{}\n", glyph.repeat(40));
    let mut text = row.repeat((bytes / row.len()).max(1));
    text.pop();
    text
}

fn frame(tb: &mut TestBackend, state: &mut TextareaState, events: Vec<slt::event::Event>) {
    tb.render_with_events(events, 0, 1, |ui| {
        black_box(ui.textarea(state, ROWS));
    });
}

/// A 1 MB document whose history holds one edit, cursor at `(row, col)`.
fn prepared(glyph: &str, tb: &mut TestBackend, middle: bool) -> TextareaState {
    let mut state = TextareaState::new();
    state.set_value(block(glyph, 1_000_000));
    frame(tb, &mut state, EventBuilder::new().key('x').build());
    if middle {
        state.cursor_row = state.lines.len() / 2;
        state.cursor_col = 20;
    } else {
        state.cursor_row = state.lines.len() - 1;
        state.cursor_col = 40;
    }
    // Left + Right keeps the cursor but ends the typing burst, so the next
    // typed character records an undo step.
    frame(
        tb,
        &mut state,
        EventBuilder::new()
            .key_code(KeyCode::Left)
            .key_code(KeyCode::Right)
            .build(),
    );
    state
}

fn bench_edits(c: &mut Criterion) {
    let mut group = c.benchmark_group("textarea_history_1mb");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(4));
    let mut tb = TestBackend::new(80, 24);
    for (alphabet, glyph) in [("ascii", "a"), ("cjk", "\u{d55c}")] {
        for middle in [false, true] {
            let position = if middle { "middle" } else { "end" };
            let base = prepared(glyph, &mut tb, middle);

            // Start of a typing burst: records one undo step.
            group.bench_function(format!("{alphabet}/type_burst_start/{position}"), |b| {
                b.iter_batched(
                    || base.clone(),
                    |mut state| {
                        frame(&mut tb, &mut state, EventBuilder::new().key('y').build());
                        state
                    },
                    BatchSize::PerIteration,
                );
            });

            let payload = block(glyph, 100_000);
            group.bench_function(format!("{alphabet}/paste_100kb/{position}"), |b| {
                b.iter_batched(
                    || base.clone(),
                    |mut state| {
                        frame(
                            &mut tb,
                            &mut state,
                            EventBuilder::new().paste(&payload).build(),
                        );
                        state
                    },
                    BatchSize::PerIteration,
                );
            });

            // Undo / redo of a 1 KB paste.
            let mut pasted = base.clone();
            frame(
                &mut tb,
                &mut pasted,
                EventBuilder::new().paste(block(glyph, 1_000)).build(),
            );
            group.bench_function(format!("{alphabet}/undo/{position}"), |b| {
                b.iter_batched(
                    || pasted.clone(),
                    |mut state| {
                        frame(
                            &mut tb,
                            &mut state,
                            EventBuilder::new()
                                .key_with(KeyCode::Char('z'), KeyModifiers::CONTROL)
                                .build(),
                        );
                        state
                    },
                    BatchSize::PerIteration,
                );
            });
            let mut undone = pasted.clone();
            frame(
                &mut tb,
                &mut undone,
                EventBuilder::new()
                    .key_with(KeyCode::Char('z'), KeyModifiers::CONTROL)
                    .build(),
            );
            group.bench_function(format!("{alphabet}/redo/{position}"), |b| {
                b.iter_batched(
                    || undone.clone(),
                    |mut state| {
                        frame(
                            &mut tb,
                            &mut state,
                            EventBuilder::new()
                                .key_with(KeyCode::Char('y'), KeyModifiers::CONTROL)
                                .build(),
                        );
                        state
                    },
                    BatchSize::PerIteration,
                );
            });
        }
    }
    group.finish();
}

fn bench_wrapped_idle(c: &mut Criterion) {
    let mut group = c.benchmark_group("textarea_wrapped_idle_1mb");
    group.sample_size(30);
    for (alphabet, glyph) in [("ascii", "a"), ("cjk", "\u{d55c}")] {
        let mut tb = TestBackend::new(80, 24);
        let mut state = TextareaState::new().word_wrap(60);
        state.set_value(block(glyph, 1_000_000));
        state.cursor_row = state.lines.len() - 1;
        frame(&mut tb, &mut state, Vec::new());
        group.bench_function(format!("{alphabet}/wrap60"), |b| {
            b.iter(|| frame(&mut tb, &mut state, Vec::new()));
        });
    }
    group.finish();
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    bench_edits(&mut criterion);
    bench_wrapped_idle(&mut criterion);
    criterion.final_summary();
}
