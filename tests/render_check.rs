#![allow(unused_must_use)]

use slt::widgets::*;
use slt::*;

#[test]
fn render_demo_basic_layout() {
    let mut tb = TestBackend::new(80, 24);
    let mut input = TextInputState::with_placeholder("Type here...");
    let spinner = SpinnerState::dots();

    tb.render(|ui| {
        ui.bordered(Border::Rounded)
            .title("SLT Demo")
            .p(1)
            .grow(1)
            .col(|ui| {
                ui.row(|ui| {
                    ui.text("Super Light TUI").bold().fg(Color::Cyan);
                    ui.spacer();
                    ui.text("dark").dim();
                });
                ui.separator();
                ui.bordered(Border::Single)
                    .title("Input")
                    .p(1)
                    .grow(1)
                    .col(|ui| {
                        ui.text("Name:").bold();
                        ui.text_input(&mut input);
                    });
                ui.row(|ui| {
                    ui.spinner(&spinner);
                    ui.text(" Loading...").dim();
                });
            });
    });

    let output = tb.to_string_trimmed();
    println!("{}", output);

    tb.assert_contains("SLT Demo");
    tb.assert_contains("Super Light TUI");
    tb.assert_contains("Name:");
    tb.assert_contains("Type here...");
}

#[test]
fn render_justify_layout() {
    let mut tb = TestBackend::new(60, 10);
    tb.render(|ui| {
        ui.bordered(Border::Single).space_between().p(1).row(|ui| {
            ui.text("A");
            ui.text("B");
            ui.text("C");
        });
    });
    let output = tb.to_string_trimmed();
    println!("Justify output:\n{}", output);
    tb.assert_contains("A");
    tb.assert_contains("B");
    tb.assert_contains("C");
}

#[test]
fn render_link() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.link("Click here", "https://example.com");
    });
    tb.assert_contains("Click here");
}

#[test]
fn render_form_field() {
    let mut tb = TestBackend::new(40, 10);
    let mut field = FormField::new("Email").placeholder("you@example.com");
    tb.render(|ui| {
        ui.form_field(&mut field);
    });
    let output = tb.to_string_trimmed();
    println!("Form field output:\n{}", output);
    tb.assert_contains("Email");
    tb.assert_contains("you@example.com");
}

#[test]
fn render_modal() {
    let mut tb = TestBackend::new(60, 20);
    tb.render(|ui| {
        ui.text("Background content");
        ui.modal(|ui| {
            ui.bordered(Border::Rounded).p(1).col(|ui| {
                ui.text("Modal title");
                ui.text("Modal body");
            });
        });
    });
    let output = tb.to_string_trimmed();
    println!("Modal output:\n{}", output);
    tb.assert_contains("Modal title");
}

#[test]
fn render_overlay() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.text("Base text");
        ui.overlay(|ui| {
            ui.text("Overlay text");
        });
    });
    let output = tb.to_string_trimmed();
    println!("Overlay output:\n{}", output);
    tb.assert_contains("Overlay text");
}

/// Regression for issue #200 Part 2: `align(End)`/`justify(End)` inside an
/// overlay must reach the bottom-right corner. Pre-fix the overlay wrapper
/// shrunk to content min-size and centered, so `.grow(1)` had no slack and
/// the inner `align/justify End` had nothing to push against.
#[test]
fn overlay_align_end_justify_end_renders_at_corner() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.text("base");
        ui.overlay(|ui| {
            ui.container()
                .grow(1)
                .align(Align::End)
                .justify(Justify::End)
                .col(|ui| {
                    ui.text("HELLO");
                });
        });
    });
    let output = tb.to_string_trimmed();
    println!("AlignEnd/JustifyEnd overlay output:\n{}", output);
    let bottom = tb.line(9);
    assert!(
        bottom.ends_with("HELLO"),
        "expected bottom row to end with 'HELLO', got {bottom:?}"
    );
}

/// Regression for issue #200 Part 3: a `container.grow(1).draw(|buf, rect|)`
/// raw-draw inside an overlay must actually render. Pre-fix the overlay
/// wrapper shrunk to content min-size — for a constraint-less raw-draw that
/// is 0×0, so the run loop skipped the empty-rect callback entirely.
#[test]
fn overlay_raw_draw_writes_to_buffer() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.text("base");
        ui.overlay(|ui| {
            ui.container().grow(1).draw(move |buf, rect| {
                buf.set_string(rect.x, rect.y, "HELLO", Style::new().fg(Color::Red));
            });
        });
    });
    let output = tb.to_string_trimmed();
    println!("Overlay raw-draw output:\n{}", output);
    tb.assert_contains("HELLO");
}

/// Regression for `overlay_at_offset(Anchor::*, dx, dy, ...)` — positive
/// `dx`/`dy` must inset the rendered content toward the viewport center,
/// matching the CSS `inset` shorthand convention.
///
/// On a 40x10 buffer with `Anchor::BottomRight` and `(dx=2, dy=1)`, the
/// glyph "X" must land at column `40 - 1 - 2 = 37` and row `10 - 1 - 1 = 8`.
#[test]
fn overlay_at_offset_insets_from_corner() {
    use slt::Anchor;

    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.overlay_at_offset(Anchor::BottomRight, 2, 1, |ui| {
            ui.text("X");
        });
    });
    let output = tb.to_string_trimmed();
    println!("overlay_at_offset BottomRight (2,1):\n{}", output);

    // Row 8 (the inset row) should end with "X" at column 37 — i.e. the
    // trimmed line is 38 chars long (cols 0..=37) and the last char is 'X'.
    let row8 = tb.line(8);
    assert_eq!(
        row8.len(),
        38,
        "row 8 should end at col 37 (40 - 1 - dx 2), got len {} ({:?})",
        row8.len(),
        row8
    );
    assert!(
        row8.ends_with('X'),
        "Anchor::BottomRight + (2,1) must place 'X' at col 37, got {row8:?}"
    );

    // Row 9 (the bottom edge) and row 10 (oob) must NOT contain 'X' — the
    // inset pushed the glyph upward and leftward.
    assert!(
        !tb.line(9).contains('X'),
        "row 9 must be empty after inset, got {:?}",
        tb.line(9)
    );

    // Row 7 must be empty too — the glyph is on row 8 only.
    assert!(
        !tb.line(7).contains('X'),
        "row 7 must be empty (glyph is on row 8), got {:?}",
        tb.line(7)
    );
}

/// Top-left mirror of `overlay_at_offset_insets_from_corner` — verifies the
/// sign convention is consistent across opposite anchors.
#[test]
fn overlay_at_offset_insets_from_top_left() {
    use slt::Anchor;

    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.overlay_at_offset(Anchor::TopLeft, 2, 1, |ui| {
            ui.text("X");
        });
    });
    let output = tb.to_string_trimmed();
    println!("overlay_at_offset TopLeft (2,1):\n{}", output);

    // Row 1 (top + dy 1) should start with two spaces then 'X'.
    let row1 = tb.line(1);
    assert!(
        row1.starts_with("  X"),
        "Anchor::TopLeft + (2,1) must place 'X' at col 2 row 1, got {row1:?}"
    );
    assert!(
        !tb.line(0).contains('X'),
        "row 0 must be empty after dy=1 inset, got {:?}",
        tb.line(0)
    );
}

/// Regression for issue #200 Part 1: `overlay_at(Anchor::*)` must place
/// content at the requested compass position by composing align+justify
/// inside the wrapper. Verified for two corners + center.
#[test]
fn overlay_at_anchor_positions() {
    use slt::Anchor;

    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.overlay_at(Anchor::TopLeft, |ui| {
            ui.text("TL");
        });
        ui.overlay_at(Anchor::BottomRight, |ui| {
            ui.text("BR");
        });
        ui.overlay_at(Anchor::Center, |ui| {
            ui.text("CC");
        });
    });
    let output = tb.to_string_trimmed();
    println!("overlay_at output:\n{}", output);
    // Top-left: row 0 starts with "TL".
    let top = tb.line(0);
    assert!(
        top.starts_with("TL"),
        "Anchor::TopLeft must place 'TL' at the start of row 0, got {top:?}"
    );
    // Bottom-right: row 9 ends with "BR".
    let bottom = tb.line(9);
    assert!(
        bottom.ends_with("BR"),
        "Anchor::BottomRight must place 'BR' at the end of row 9, got {bottom:?}"
    );
    // Center: 'CC' should be near the middle of the buffer (rows 4 or 5).
    let center_row_a = tb.line(4);
    let center_row_b = tb.line(5);
    assert!(
        center_row_a.contains("CC") || center_row_b.contains("CC"),
        "Anchor::Center must place 'CC' near the buffer center (row 4 or 5), got rows {center_row_a:?} / {center_row_b:?}"
    );
}
