//! Retained undo-history memory for large `TextareaState` / `TextInputState`
//! documents (#415).
//!
//! Ignored by default: it prints numbers instead of asserting a budget (the
//! bounded-memory contract itself is asserted by unit tests next to the
//! history implementation). Run in release so the allocator sees production
//! layouts:
//!
//! ```sh
//! cargo test --release --test textarea_retained_memory -- --ignored --nocapture --test-threads=1
//! ```
//!
//! "live_delta" is the change in bytes currently allocated by the
//! process-wide counting allocator (alloc - dealloc, realloc as a delta)
//! across one step, so it is what the step retained: the edits themselves
//! plus whatever undo history they recorded. Compare a capped row with the
//! `history=0` row of the same document to isolate the history share.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicI64, Ordering};

use slt::event::Event;
use slt::widgets::{TextInputState, TextareaState};
use slt::{EventBuilder, KeyCode, KeyModifiers, TestBackend};

struct TrackingAlloc;

static LIVE: AtomicI64 = AtomicI64::new(0);

// SAFETY: every method forwards to `System` with the caller's arguments; the
// only addition is a lock-free atomic counter update, which never allocates.
unsafe impl GlobalAlloc for TrackingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size() as i64, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size() as i64, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as i64, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LIVE.fetch_add(new_size as i64 - layout.size() as i64, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: TrackingAlloc = TrackingAlloc;

const DOC_BYTES: usize = 1_000_000;
const EDITS: usize = 150;

fn live() -> i64 {
    LIVE.load(Ordering::Relaxed)
}

/// About `DOC_BYTES` of 40-glyph lines.
fn document(glyph: &str) -> String {
    let row = format!("{}\n", glyph.repeat(40));
    let mut text = row.repeat((DOC_BYTES / row.len()).max(1));
    text.pop();
    text
}

fn chord(ch: char) -> Event {
    EventBuilder::new()
        .key_with(KeyCode::Char(ch), KeyModifiers::CONTROL)
        .build()
        .remove(0)
}

fn repeat(event: Event, count: usize) -> Vec<Event> {
    vec![event; count]
}

fn report(label: &str, before: i64, history_len: usize) {
    eprintln!(
        "RETAINED {label:<52} live_delta={:>12} history_len={history_len}",
        live() - before
    );
}

fn textarea_case(alphabet: &str, glyph: &str, cap: usize) {
    let mut tb = TestBackend::new(80, 24);
    let mut state = TextareaState::new().history_max(cap);
    state.set_value(document(glyph));
    state.cursor_row = state.lines.len() - 1;
    state.cursor_col = 0;
    for _ in 0..2 {
        tb.render(|ui| {
            let _ = ui.textarea(&mut state, 23);
        });
    }
    let label = format!("textarea {alphabet} 1MB history={cap}");

    let before = live();
    let paste = EventBuilder::new().paste("x").build().remove(0);
    tb.render_with_events(repeat(paste, EDITS), 0, 1, |ui| {
        let _ = ui.textarea(&mut state, 23);
    });
    report(
        &format!("{label}: {EDITS} one-byte pastes"),
        before,
        state.history_len(),
    );

    let before = live();
    tb.render_with_events(repeat(chord('z'), EDITS), 0, 1, |ui| {
        let _ = ui.textarea(&mut state, 23);
    });
    report(&format!("{label}: undo all"), before, state.history_len());

    let before = live();
    tb.render_with_events(repeat(chord('y'), EDITS), 0, 1, |ui| {
        let _ = ui.textarea(&mut state, 23);
    });
    report(&format!("{label}: redo all"), before, state.history_len());

    let before = live();
    drop(state);
    report(&format!("{label}: drop state"), before, 0);
}

fn textarea_large_paste_case(cap: usize) {
    let mut tb = TestBackend::new(80, 24);
    let mut state = TextareaState::new().history_max(cap);
    state.set_value(document("a"));
    state.cursor_row = state.lines.len() - 1;
    state.cursor_col = 0;
    tb.render(|ui| {
        let _ = ui.textarea(&mut state, 23);
    });
    let payload = document("b")[..100_000].to_string();
    let before = live();
    let paste = EventBuilder::new().paste(&payload).build().remove(0);
    tb.render_with_events(repeat(paste, 10), 0, 1, |ui| {
        let _ = ui.textarea(&mut state, 23);
    });
    report(
        &format!("textarea ascii 1MB history={cap}: 10 x 100KB pastes"),
        before,
        state.history_len(),
    );
}

fn text_input_case() {
    let mut tb = TestBackend::new(80, 3);
    let mut state = TextInputState::new();
    state.set_value("a".repeat(DOC_BYTES));
    for _ in 0..2 {
        tb.render(|ui| {
            let _ = ui.text_input(&mut state);
        });
    }
    let before = live();
    let backspace = EventBuilder::new().key_code(KeyCode::Backspace).build();
    tb.render_with_events(repeat(backspace[0].clone(), 100), 0, 1, |ui| {
        let _ = ui.text_input(&mut state);
    });
    report("text_input 1MB single line: 100 backspaces", before, 0);
    let before = live();
    drop(state);
    report("text_input 1MB single line: drop state", before, 0);
}

#[test]
#[ignore = "retained-memory measurement (#415); prints, does not assert"]
fn textarea_history_retained_memory_at_1mb() {
    let mut tb = TestBackend::new(80, 24);
    tb.render(|ui| {
        ui.text("warm");
    });
    for (alphabet, glyph) in [("ascii", "a"), ("cjk", "\u{d55c}")] {
        for cap in [0, 8, 100] {
            textarea_case(alphabet, glyph, cap);
        }
    }
    textarea_large_paste_case(0);
    textarea_large_paste_case(100);
    text_input_case();
}
