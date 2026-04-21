//! Tests for `.with_if(cond, f)` and `.with(f)` on text style chains and
//! `ContainerBuilder`. These conditional/grouping helpers should be free of
//! side effects when the condition is false and equivalent to inline calls
//! when the condition is true.

use slt::{Border, Color, Modifiers, TestBackend};

/// Read the style of the first non-empty cell on row `y`.
fn first_styled_cell(tb: &TestBackend, y: u32) -> slt::Style {
    let buf = tb.buffer();
    for x in 0..tb.width() {
        let cell = buf.get(x, y);
        if cell.symbol.as_str() != " " && !cell.symbol.is_empty() {
            return cell.style;
        }
    }
    panic!("no non-empty cell on row {y}");
}

// ── text helpers ────────────────────────────────────────────────────────

#[test]
fn with_if_true_applies_modifier() {
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        ui.text("hello").with_if(true, |t| {
            t.bold();
        });
    });
    let style = first_styled_cell(&tb, 0);
    assert!(
        style.modifiers.contains(Modifiers::BOLD),
        "expected BOLD modifier when cond=true, got {:?}",
        style.modifiers
    );
}

#[test]
fn with_if_false_skips() {
    let mut tb_skip = TestBackend::new(20, 3);
    tb_skip.render(|ui| {
        ui.text("hello").with_if(false, |t| {
            t.bold();
        });
    });
    let skipped = first_styled_cell(&tb_skip, 0);
    assert!(
        !skipped.modifiers.contains(Modifiers::BOLD),
        "expected BOLD to NOT be set when cond=false, got {:?}",
        skipped.modifiers
    );

    // Equivalence: skipping with_if(false, ...) should match the no-op chain.
    let mut tb_plain = TestBackend::new(20, 3);
    tb_plain.render(|ui| {
        ui.text("hello");
    });
    let plain = first_styled_cell(&tb_plain, 0);
    assert_eq!(
        skipped.modifiers, plain.modifiers,
        "with_if(false) should leave modifiers identical to no-op text"
    );
    assert_eq!(
        skipped.fg, plain.fg,
        "with_if(false) should leave fg identical to no-op text"
    );
}

#[test]
fn with_always_applies() {
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        ui.text("hello").with(|t| {
            t.fg(Color::Red);
        });
    });
    let style = first_styled_cell(&tb, 0);
    assert_eq!(
        style.fg,
        Some(Color::Red),
        "expected Red fg, got {:?}",
        style.fg
    );
}

#[test]
fn with_if_chain_independently_applies_each_branch() {
    // Three independent conditions: only the true ones should apply.
    // True/False/True → expect BOLD set, ITALIC unset, fg = Cyan.
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        ui.text("chain")
            .with_if(true, |t| {
                t.bold();
            })
            .with_if(false, |t| {
                t.italic();
            })
            .with_if(true, |t| {
                t.fg(Color::Cyan);
            });
    });
    let style = first_styled_cell(&tb, 0);
    assert!(
        style.modifiers.contains(Modifiers::BOLD),
        "branch 1 (true) should have set BOLD, modifiers={:?}",
        style.modifiers
    );
    assert!(
        !style.modifiers.contains(Modifiers::ITALIC),
        "branch 2 (false) should NOT have set ITALIC, modifiers={:?}",
        style.modifiers
    );
    assert_eq!(
        style.fg,
        Some(Color::Cyan),
        "branch 3 (true) should have set Cyan fg, got {:?}",
        style.fg
    );
}

// ── ContainerBuilder helpers ────────────────────────────────────────────

#[test]
fn container_with_if_true_applies_modifier() {
    // When cond=true, container should have a Single-line border drawn around it.
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        let _ = ui
            .container()
            .w(10)
            .h(3)
            .with_if(true, |c| c.border(Border::Single))
            .col(|ui| {
                ui.text("X");
            });
    });
    // Border::Single uses '┌' at the top-left corner.
    let line0 = tb.line(0);
    assert!(
        line0.contains('┌'),
        "expected top-left border corner '┌' on line 0, got {line0:?}"
    );
}

#[test]
fn container_with_if_false_skips() {
    // When cond=false, no border should be drawn.
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        let _ = ui
            .container()
            .w(10)
            .h(3)
            .with_if(false, |c| c.border(Border::Single))
            .col(|ui| {
                ui.text("X");
            });
    });
    let line0 = tb.line(0);
    assert!(
        !line0.contains('┌'),
        "expected NO border corner '┌' when cond=false, got {line0:?}"
    );
}

#[test]
fn container_with_always_applies() {
    // .with always runs — verify a title appears in the top border.
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        let _ = ui
            .container()
            .w(12)
            .h(3)
            .border(Border::Single)
            .with(|c| c.title("Hi"))
            .col(|ui| {
                ui.text("X");
            });
    });
    tb.assert_contains("Hi");
}

#[test]
fn container_with_if_chain_independently_applies_each_branch() {
    // Three independent conditions on the container builder.
    // True/False/True → border drawn (true), title "SKIP" never set (false),
    // title "OK" set (true).
    let mut tb = TestBackend::new(30, 5);
    tb.render(|ui| {
        let _ = ui
            .container()
            .w(20)
            .h(3)
            .with_if(true, |c| c.border(Border::Single))
            .with_if(false, |c| c.title("SKIP"))
            .with_if(true, |c| c.title("OK"))
            .col(|ui| {
                ui.text("X");
            });
    });

    let line0 = tb.line(0);
    assert!(
        line0.contains('┌'),
        "branch 1 (true) should draw border corner '┌', got {line0:?}"
    );
    tb.assert_contains("OK");
    // SKIP must not appear anywhere.
    for y in 0..tb.height() {
        let l = tb.line(y);
        assert!(
            !l.contains("SKIP"),
            "branch 2 (false) should not have set title 'SKIP', found on line {y}: {l:?}"
        );
    }
}
