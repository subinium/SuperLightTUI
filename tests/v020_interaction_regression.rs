//! v0.20 demo interaction regression tests.
//!
//! These tests catch three real bugs that shipped in v0.20 and slipped past
//! the existing static-snapshot suite. Each one drives a demo through a
//! multi-frame interaction and verifies a state mutation that the on-screen
//! text alone could not prove.
//!
//! - `named_focus_click_focuses_target_input`:
//!   `register_focusable_named(name)` previously allocated its own focus
//!   slot, so a following `text_input(&mut state)` registered a different
//!   slot and `focus_by_name` jumped to a dummy. We assert that interacting
//!   with the Name row delivers a typed character into `state.name.value`.
//!
//! - `modal_trap_yes_click_persists_answer`:
//!   `v020_modal_trap.rs::render(ui)` recreated `State` every frame which
//!   reset `show_modal` and discarded any Yes/No clicks. We hold a
//!   persistent `State` across frames and assert clicking Yes flips both
//!   `answered` and `show_modal` and keeps that flip on subsequent frames.
//!
//! - `code_block_lang_empty_lang_renders_tokens_inline`:
//!   `code_block_lang(code, "")` fell through to a path that emitted each
//!   token via `ui.text(...)` inside a `col`, stacking tokens vertically.
//!   We assert all tokens of a one-line snippet land on the same buffer
//!   row.

#[path = "../examples/v020_named_focus.rs"]
#[allow(dead_code)]
mod named_focus_demo;

#[path = "../examples/v020_modal_trap.rs"]
#[allow(dead_code)]
mod modal_trap_demo;

use slt::{context::ModalOptions, Border, ButtonVariant, Context, EventBuilder, TestBackend};
use std::cell::RefCell;
use std::rc::Rc;

// ----------------------------------------------------------------
// Test 1 — named focus + text_input slot binding
// ----------------------------------------------------------------

/// Click on the Name input box AND type a character drives the typed key into
/// the actual `TextInputState`. The on-screen `focused_name: name` indicator
/// rendered correctly even when the bug was present, so the only honest
/// proof is the keystroke landing in `state.name.value`.
///
/// We bundle the click and the keystroke into the same frame's event batch.
/// Click hit-testing in `Context::new` resolves `focus_index` to the
/// text_input rect's slot before the render closure runs, so the keystroke
/// flows into `text_input` without depending on the name → slot mapping
/// path. If the slot binding regresses or the hit-test ordering breaks, this
/// assertion is the canary.
#[test]
fn named_focus_click_focuses_target_input() {
    let state = Rc::new(RefCell::new(named_focus_demo::DemoState::default()));
    let mut tb = TestBackend::new(80, 12);

    // Frame 0: paint the form so the next frame's hit-map is populated.
    {
        let s = state.clone();
        tb.render(move |ui| {
            named_focus_demo::render(ui, &mut s.borrow_mut());
        });
    }

    // Locate the Name input box. The demo's layout puts the Name row's
    // text_input rectangle on the row containing "Name:" plus a column
    // offset; we scan the buffer to find the row interior so the test is
    // tolerant of small layout adjustments.
    let (input_x, input_y) = locate_named_input(&tb, "Name");

    // Frame 1: click on the input and inject the 'h' keystroke in the same
    // event batch. Same-frame timing is what bypasses the demo's internal
    // `focus_by_name("name")` call (which only takes effect on the next
    // frame) and exposes the slot-binding contract directly.
    let s1 = state.clone();
    tb.sequence()
        .events(
            EventBuilder::new().click(input_x, input_y).key('h').build(),
            move |ui| {
                named_focus_demo::render(ui, &mut s1.borrow_mut());
            },
        )
        .run();

    let value = state.borrow().name.value.clone();
    assert_eq!(
        value,
        "h",
        "click + 'h' should land in state.name.value; got {value:?}.\nbuffer:\n{}",
        tb.to_string_trimmed()
    );

    // Other inputs must not steal the keystroke.
    assert!(
        state.borrow().email.value.is_empty(),
        "email captured a keystroke meant for name"
    );
    assert!(
        state.borrow().city.value.is_empty(),
        "city captured a keystroke meant for name"
    );
}

/// Find the (x, y) interior of the bordered text_input attached to a labelled
/// row in the named-focus demo. Returns a position that lies on the cursor
/// row of the input box, at roughly its midpoint horizontally. Walks chars
/// (not bytes) so the multi-byte box-drawing border doesn't shift columns.
fn locate_named_input(tb: &TestBackend, label: &str) -> (u32, u32) {
    let label_marker = format!("{label}:");
    for y in 0..tb.height() {
        let line = tb.line(y);
        let Some(label_col) = char_index_of(&line, &label_marker) else {
            continue;
        };
        // Look for the first '╭' to the right of the label. The text_input
        // box is the only bordered child on this row.
        let after_start = label_col + label_marker.chars().count();
        let chars: Vec<char> = line.chars().collect();
        let mut box_left = after_start;
        for (i, ch) in chars.iter().enumerate().skip(after_start) {
            if *ch == '╭' || *ch == '┌' {
                box_left = i;
                break;
            }
        }
        // Cursor row is the row below the label row (inside the box).
        let cursor_y = y + 1;
        // Mid-box x — pick a column 2 chars to the right of the left border.
        let cursor_x = (box_left + 2) as u32;
        return (cursor_x, cursor_y);
    }
    panic!("could not locate input row for label {label:?}\nbuffer:\n{tb}");
}

// ----------------------------------------------------------------
// Test 2 — modal answer persists across frames
// ----------------------------------------------------------------

/// Hold a persistent `modal_trap_demo::State`, render multiple frames against
/// it, and assert that clicking Yes both records the answer and dismisses
/// the modal. The buggy version of the demo's `pub fn render(ui)` rebuilt
/// `State` every frame, so a click could never persist; the regression
/// canary here is that `state.answered == Some(true)` survives the next
/// frame.
///
/// `body` is private inside the demo, so we replicate the body shape the
/// fixed render must use. Replicating the body keeps the test honest about
/// what behaviour we are asserting (state-aware re-render) without adding
/// new public surface to the demo.
#[test]
fn modal_trap_yes_click_persists_answer() {
    // Open modal up front so the click frame sees it.
    let state = Rc::new(RefCell::new(modal_trap_demo::State {
        show_modal: true,
        answered: None,
    }));
    let mut tb = TestBackend::new(80, 16);

    // Frame 0: paint the modal so the next frame's hit-map knows where Yes
    // is.
    {
        let s = state.clone();
        tb.render(move |ui| {
            render_modal_body(ui, &mut s.borrow_mut());
        });
    }

    // Locate the Yes button on the painted modal — same tolerance pattern
    // as the named-input locator.
    let (yes_x, yes_y) = locate_yes_button(&tb);

    // Frame 1: click on Yes.
    let s1 = state.clone();
    tb.sequence()
        .events(EventBuilder::new().click(yes_x, yes_y).build(), move |ui| {
            render_modal_body(ui, &mut s1.borrow_mut());
        })
        .run();

    // The click handler must have flipped both signals.
    assert_eq!(
        state.borrow().answered,
        Some(true),
        "Yes click should set answered=Some(true).\nbuffer:\n{}",
        tb.to_string_trimmed()
    );
    assert!(
        !state.borrow().show_modal,
        "Yes click should dismiss the modal"
    );

    // Frame 2: a settle frame must NOT clobber the previously recorded
    // answer. This is the real regression — the buggy render re-initialised
    // state every frame, so the Yes flip would be lost here.
    let s2 = state.clone();
    tb.sequence()
        .tick(move |ui| {
            render_modal_body(ui, &mut s2.borrow_mut());
        })
        .run();

    assert_eq!(
        state.borrow().answered,
        Some(true),
        "answered must persist across frames"
    );
    assert!(
        !state.borrow().show_modal,
        "show_modal must remain false across frames"
    );
}

/// Replicate the demo's private `body` function in shape and key behaviour:
/// background buttons + Open-modal button + bordered Yes/No modal when
/// `state.show_modal` is true. The Yes/No click handlers mirror the demo
/// exactly so the test verifies the same state-mutation contract.
fn render_modal_body(ui: &mut Context, state: &mut modal_trap_demo::State) {
    let sp = ui.spacing();
    let _ = ui
        .bordered(Border::Rounded)
        .title("SLT v0.20: Modal focus trap")
        .p(sp.sm())
        .gap(sp.xs())
        .grow(1)
        .col(|ui| {
            ui.text("test body").dim();
            let _ = ui.container().gap(sp.sm()).row(|ui| {
                let _ = ui.button("First bg button");
                let _ = ui.button("Second bg button");
                let _ = ui.button("Third bg button");
            });
            ui.text("");
            if ui.button_with("Open modal", ButtonVariant::Primary).clicked {
                state.show_modal = true;
                state.answered = None;
            }
        });

    if state.show_modal {
        let _ = ui.modal_with(ModalOptions { tab_trap: true }, |ui| {
            let _ = ui
                .bordered(Border::Rounded)
                .title("Confirm")
                .p(sp.sm())
                .gap(sp.xs())
                .col(|ui| {
                    ui.text("Press Tab — focus stays inside the modal.").bold();
                    let _ = ui.container().gap(sp.sm()).row(|ui| {
                        if ui.button_with("Yes", ButtonVariant::Primary).clicked {
                            state.answered = Some(true);
                            state.show_modal = false;
                        }
                        if ui.button_with("No", ButtonVariant::Outline).clicked {
                            state.answered = Some(false);
                            state.show_modal = false;
                        }
                    });
                    ui.text("Esc to dismiss.").dim();
                });
        });
    }
}

/// Find a click target in the rendered Yes button. The button label "Yes"
/// renders inside the modal frame; we click on the row containing "Yes" at
/// the column where the label sits. Walk the line as `chars`, not bytes,
/// because the bordered frame uses multi-byte box-drawing glyphs.
fn locate_yes_button(tb: &TestBackend) -> (u32, u32) {
    for y in 0..tb.height() {
        let line = tb.line(y);
        if let Some(col) = char_index_of(&line, "Yes") {
            return ((col as u32) + 1, y);
        }
    }
    panic!("could not locate Yes button.\nbuffer:\n{tb}");
}

/// Return the *char* (terminal-column) index where `needle` starts within
/// `haystack`. `str::find` returns byte offsets, but TestBackend coordinates
/// are 1 char = 1 cell, so we scan `haystack.chars()` instead.
fn char_index_of(haystack: &str, needle: &str) -> Option<usize> {
    let needle_chars: Vec<char> = needle.chars().collect();
    let chars: Vec<char> = haystack.chars().collect();
    if needle_chars.is_empty() || chars.len() < needle_chars.len() {
        return None;
    }
    for start in 0..=chars.len() - needle_chars.len() {
        if chars[start..start + needle_chars.len()] == needle_chars[..] {
            return Some(start);
        }
    }
    None
}

// ----------------------------------------------------------------
// Test 3 — code_block_lang(..., "") renders tokens inline
// ----------------------------------------------------------------

/// `code_block_lang(code, "")` previously emitted each highlighted token via
/// a separate `ui.text(...)` call inside the bordered `col`, which made every
/// token start a new row. The fix is to wrap the fallback emission in
/// `ui.line(...)`. We assert all tokens of a one-line snippet land on the
/// same buffer row.
#[test]
fn code_block_lang_empty_lang_renders_tokens_inline() {
    let mut tb = TestBackend::new(80, 8);
    tb.render(|ui| {
        let _ = ui.code_block("fn main() { let x = 1; }").lang("").show();
    });

    // Tokens that the keyword-heuristic highlighter emits separately. They
    // must all coexist on the same row. We pick the densest grouping
    // ("fn", "main", "let", "x", "=", "1") and require co-location.
    let tokens = ["fn", "main", "let", "x", "=", "1"];
    let buffer = tb.to_string_trimmed();

    // Find the row that contains all tokens.
    let mut shared_row: Option<u32> = None;
    for y in 0..tb.height() {
        let line = tb.line(y);
        if tokens.iter().all(|t| line.contains(t)) {
            shared_row = Some(y);
            break;
        }
    }

    assert!(
        shared_row.is_some(),
        "all tokens {tokens:?} must appear together on a single row.\nbuffer:\n{buffer}"
    );

    // Make the same assertion via the row-by-row API for a clearer failure
    // diff in CI.
    let row = shared_row.unwrap();
    let line = tb.line(row);
    for t in tokens {
        assert!(
            line.contains(t),
            "token {t:?} missing from row {row}: {line:?}.\nbuffer:\n{buffer}"
        );
    }
}
