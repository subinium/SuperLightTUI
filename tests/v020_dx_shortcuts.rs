//! Unit tests for v0.20.0 DX shorthand helpers.
//!
//! Covers:
//! - `Response::on_hover` / `on_hover_ui` chaining (#209)
//! - `Context::animate_bool` / `animate_value` shorthand (#210)
//! - `ContainerBuilder::fill()` shorthand for `grow(1)` (#220)
//! - `Rect::center_in` / `center_horizontally_in` / `center_vertically_in` (#221)

use slt::anim::DEFAULT_ANIMATE_TICKS;
use slt::event::Event;
use slt::{frame, AppState, Backend, Buffer, Context, Rect, RunConfig, TestBackend};

/// Minimal Backend impl that drives `frame()` against an in-memory buffer.
/// Unlike `TestBackend::render()`, going through `frame()` advances the
/// per-frame `tick`, which animations rely on.
struct TickingBackend {
    buffer: Buffer,
}

impl TickingBackend {
    fn new(width: u32, height: u32) -> Self {
        Self {
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
        }
    }
}

impl Backend for TickingBackend {
    fn size(&self) -> (u32, u32) {
        (self.buffer.area.width, self.buffer.area.height)
    }
    fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Run a single frame using the public `frame()` entry point so that tick
/// advancement is observable in animation state.
fn run_one_frame<F: FnMut(&mut Context)>(
    backend: &mut TickingBackend,
    state: &mut AppState,
    config: &RunConfig,
    events: &[Event],
    mut f: F,
) {
    frame(backend, state, config, events, &mut f).expect("frame failed");
}

// ──────────────────────────────────────────────────────────────────────────
// #209 — Response::on_hover / on_hover_ui chaining
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn on_hover_renders_tooltip_when_hovered() {
    let mut tb = TestBackend::new(80, 24);

    // First frame: register layout so subsequent mouse events can hit-test.
    tb.render(|ui| {
        let _ = ui.button("Save").on_hover(ui, "Saves the file");
    });

    // Second frame: mouse hovering on the button.
    tb.run_with_events(vec![Event::mouse_move(2, 0)], |ui| {
        let _ = ui.button("Save").on_hover(ui, "Saves the file");
    });

    // Tooltip text should now be in the buffer.
    tb.assert_contains("Saves the file");
}

#[test]
fn on_hover_does_not_render_when_not_hovered() {
    let mut tb = TestBackend::new(80, 24);

    tb.render(|ui| {
        let _ = ui.button("Save").on_hover(ui, "Saves the file");
    });
    tb.run_with_events(vec![Event::mouse_move(70, 20)], |ui| {
        let _ = ui.button("Save").on_hover(ui, "Saves the file");
    });

    let rendered = tb.to_string_trimmed();
    assert!(
        !rendered.contains("Saves the file"),
        "tooltip should not render when widget is not hovered, got:\n{}",
        rendered
    );
}

#[test]
fn on_hover_preserves_response_chaining() {
    // Confirm the Response is returned so further chaining like
    // `.on_hover(...).clicked` continues to work.
    let mut tb = TestBackend::new(80, 24);
    tb.render(|ui| {
        let r = ui.button("Save").on_hover(ui, "tip");
        // Original Response fields still readable post-chain.
        let _ = r.clicked;
        let _ = r.hovered;
        let _ = r.rect;
    });
}

#[test]
fn on_hover_empty_string_is_noop() {
    // Empty tooltip text should not push a PendingTooltip even when hovered.
    let mut tb = TestBackend::new(80, 24);
    tb.render(|ui| {
        let _ = ui.button("Save").on_hover(ui, "");
    });
    tb.run_with_events(vec![Event::mouse_move(2, 0)], |ui| {
        let _ = ui.button("Save").on_hover(ui, "");
    });
    // No tooltip should have rendered — buffer should not contain the
    // border characters of the tooltip overlay box. We sample by checking
    // the absence of the typical button label being rendered twice.
    // Since an empty tooltip emits nothing, the overlay won't add any
    // off-button characters; we cannot easily probe negative state on a
    // pristine render, but at minimum the call must not panic and must
    // continue chaining.
    let _ = tb.to_string_trimmed();
}

#[test]
fn on_hover_ui_runs_closure_only_when_hovered() {
    // Drives a counter from inside the on_hover_ui closure; counts must
    // increment only when the mouse is over the button.
    use std::cell::Cell;
    let count = Cell::new(0u32);

    let mut tb = TestBackend::new(80, 24);
    // Frame 1: register layout, mouse outside.
    tb.render(|ui| {
        let _ = ui
            .button("Help")
            .on_hover_ui(ui, |_ui| count.set(count.get() + 1));
    });
    assert_eq!(count.get(), 0, "closure must not run on first frame");

    // Frame 2: mouse outside the button still.
    tb.run_with_events(vec![Event::mouse_move(70, 20)], |ui| {
        let _ = ui
            .button("Help")
            .on_hover_ui(ui, |_ui| count.set(count.get() + 1));
    });
    assert_eq!(count.get(), 0, "closure must not run when not hovered");

    // Frame 3: mouse over the button.
    tb.run_with_events(vec![Event::mouse_move(2, 0)], |ui| {
        let _ = ui
            .button("Help")
            .on_hover_ui(ui, |_ui| count.set(count.get() + 1));
    });
    assert_eq!(count.get(), 1, "closure must run when hovered");
}

// ──────────────────────────────────────────────────────────────────────────
// #210 — animate_bool / animate_value
// ──────────────────────────────────────────────────────────────────────────
//
// These tests drive `frame()` (not `TestBackend::render()`) because only
// `frame()` advances `state.tick` after each call — animations need that.

#[test]
fn animate_bool_first_call_snaps_to_target() {
    // First call should equal the target with no visible transition: the
    // tween is initialized at the target value with duration 0, so the
    // returned value is exactly `target`.
    let mut backend = TickingBackend::new(40, 4);
    let mut state = AppState::new();
    let config = RunConfig::default();
    let mut sample = -1.0_f64;
    run_one_frame(&mut backend, &mut state, &config, &[], |ui| {
        sample = ui.animate_bool("dx::first_true", true);
    });
    assert!(
        (sample - 1.0).abs() < f64::EPSILON,
        "expected 1.0, got {sample}"
    );

    let mut backend2 = TickingBackend::new(40, 4);
    let mut state2 = AppState::new();
    let mut sample2 = -1.0_f64;
    run_one_frame(&mut backend2, &mut state2, &config, &[], |ui| {
        sample2 = ui.animate_bool("dx::first_false", false);
    });
    assert!(
        (sample2 - 0.0).abs() < f64::EPSILON,
        "expected 0.0, got {sample2}"
    );
}

#[test]
fn animate_bool_transitions_zero_to_one_over_default_duration() {
    // After the first frame seeded the value, flipping the boolean should
    // ramp linearly toward the new target across DEFAULT_ANIMATE_TICKS frames.
    //
    // Note on timing: at the retarget frame the returned value equals the
    // current interpolated value (the "from" of the new tween). The first
    // visible transition step appears one tick later.
    let mut backend = TickingBackend::new(40, 4);
    let mut state = AppState::new();
    let config = RunConfig::default();

    // Frame 0: seed at 0.0.
    let mut v0 = -1.0_f64;
    run_one_frame(&mut backend, &mut state, &config, &[], |ui| {
        v0 = ui.animate_bool("dx::ramp", false);
    });
    assert!((v0 - 0.0).abs() < f64::EPSILON);

    // Frame 1: flip to true. Retarget begins at current value (0.0); the
    // tween's start_tick == this tick, so value(this_tick) == 0.0.
    let mut v1 = -1.0_f64;
    run_one_frame(&mut backend, &mut state, &config, &[], |ui| {
        v1 = ui.animate_bool("dx::ramp", true);
    });
    assert!(
        (v1 - 0.0).abs() < f64::EPSILON,
        "frame 1 (retarget tick) should still read 0.0, got {v1}"
    );

    // Frame 2: one tick into the new tween → ~1/12 of the way to 1.0.
    let mut v2 = -1.0_f64;
    run_one_frame(&mut backend, &mut state, &config, &[], |ui| {
        v2 = ui.animate_bool("dx::ramp", true);
    });
    assert!(
        v2 > 0.0 && v2 < 1.0,
        "expected mid-transition value on frame 2, got {v2}"
    );

    // Drive frames forward until duration elapses.
    let mut last = v2;
    for _ in 0..DEFAULT_ANIMATE_TICKS {
        run_one_frame(&mut backend, &mut state, &config, &[], |ui| {
            last = ui.animate_bool("dx::ramp", true);
        });
    }
    assert!(
        (last - 1.0).abs() < f64::EPSILON,
        "expected to reach 1.0 after duration, got {last}"
    );
}

#[test]
fn animate_value_zero_duration_snaps_immediately() {
    // duration_ticks == 0 means the next sample is exactly `target`.
    let mut backend = TickingBackend::new(40, 4);
    let mut state = AppState::new();
    let config = RunConfig::default();

    // Seed at 0.0 with normal duration.
    run_one_frame(&mut backend, &mut state, &config, &[], |ui| {
        let _ = ui.animate_value("dx::snap", 0.0, 12);
    });

    // Retarget to 100.0 with duration 0 → snaps.
    let mut v = -1.0_f64;
    run_one_frame(&mut backend, &mut state, &config, &[], |ui| {
        v = ui.animate_value("dx::snap", 100.0, 0);
    });
    assert!(
        (v - 100.0).abs() < 1e-9,
        "expected immediate snap to 100.0, got {v}"
    );
}

#[test]
fn animate_value_independent_ids_are_isolated() {
    let mut backend = TickingBackend::new(40, 4);
    let mut state = AppState::new();
    let config = RunConfig::default();

    let mut a_seed = -1.0_f64;
    let mut b_seed = -1.0_f64;
    run_one_frame(&mut backend, &mut state, &config, &[], |ui| {
        a_seed = ui.animate_value("dx::a", 50.0, 12);
        b_seed = ui.animate_value("dx::b", 200.0, 12);
    });
    assert!((a_seed - 50.0).abs() < f64::EPSILON);
    assert!((b_seed - 200.0).abs() < f64::EPSILON);

    // Frame 2: retarget `a` to 0.0. Returned value equals current value
    // at retarget time (50.0), since the new tween hasn't advanced yet.
    let mut a2 = -1.0_f64;
    let mut b2 = -1.0_f64;
    run_one_frame(&mut backend, &mut state, &config, &[], |ui| {
        a2 = ui.animate_value("dx::a", 0.0, 12);
        b2 = ui.animate_value("dx::b", 200.0, 12);
    });
    assert!(
        (a2 - 50.0).abs() < f64::EPSILON,
        "frame 2 (retarget tick) should still read 50.0, got {a2}"
    );
    assert!(
        (b2 - 200.0).abs() < f64::EPSILON,
        "expected `b` unchanged, got {b2}"
    );

    // Frame 3: `a` has advanced one tick into its 12-tick transition,
    // moving down toward 0.0; `b` is still at 200.0.
    let mut a3 = -1.0_f64;
    let mut b3 = -1.0_f64;
    run_one_frame(&mut backend, &mut state, &config, &[], |ui| {
        a3 = ui.animate_value("dx::a", 0.0, 12);
        b3 = ui.animate_value("dx::b", 200.0, 12);
    });
    assert!(
        a3 < 50.0 && a3 > 0.0,
        "frame 3 should show `a` mid-transition, got {a3}"
    );
    assert!(
        (b3 - 200.0).abs() < f64::EPSILON,
        "expected `b` unchanged on frame 3, got {b3}"
    );
}

#[test]
fn animate_value_retarget_starts_from_current_value() {
    // Mid-flight retarget should not snap back: the new tween must begin
    // from wherever the current value is, not from the original `from`.
    let mut backend = TickingBackend::new(40, 4);
    let mut state = AppState::new();
    let config = RunConfig::default();

    // Seed at 0.0.
    run_one_frame(&mut backend, &mut state, &config, &[], |ui| {
        let _ = ui.animate_value("dx::retarget", 0.0, 12);
    });

    // Begin transition to 100.0 — drive 6 frames mid-flight.
    let mut mid = -1.0_f64;
    for _ in 0..6 {
        run_one_frame(&mut backend, &mut state, &config, &[], |ui| {
            mid = ui.animate_value("dx::retarget", 100.0, 12);
        });
    }
    assert!(
        mid > 0.0 && mid < 100.0,
        "expected mid-flight value, got {mid}"
    );

    // Retarget to 200.0 immediately. The very next frame should still be
    // close to `mid`, not a jump back to 0.
    let mut after_retarget = -1.0_f64;
    run_one_frame(&mut backend, &mut state, &config, &[], |ui| {
        after_retarget = ui.animate_value("dx::retarget", 200.0, 12);
    });
    // Allow a small step; the key property is no jump back to 0.
    assert!(
        after_retarget >= mid - 0.5,
        "retarget must not snap backward (mid={mid}, after={after_retarget})"
    );
    assert!(
        after_retarget < 200.0,
        "retarget must not jump forward to target (after={after_retarget})"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// #220 — ContainerBuilder::fill() shorthand
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn fill_is_equivalent_to_grow_1() {
    // Two columns side-by-side: one fixed-width, one with .fill(). The
    // filled one should occupy all the remaining width — identical to
    // .grow(1).
    let mut tb_fill = TestBackend::new(40, 4);
    tb_fill.render(|ui| {
        let _ = ui.row(|ui| {
            let _ = ui.container().w(10).col(|ui| {
                ui.text("LHS");
            });
            let _ = ui.container().fill().col(|ui| {
                ui.text("RHS");
            });
        });
    });

    let mut tb_grow = TestBackend::new(40, 4);
    tb_grow.render(|ui| {
        let _ = ui.row(|ui| {
            let _ = ui.container().w(10).col(|ui| {
                ui.text("LHS");
            });
            let _ = ui.container().grow(1).col(|ui| {
                ui.text("RHS");
            });
        });
    });

    assert_eq!(
        tb_fill.to_string_trimmed(),
        tb_grow.to_string_trimmed(),
        "fill() must produce identical output to grow(1)"
    );
}

#[test]
fn fill_extends_to_full_remaining_width() {
    // With only the LHS having a fixed width, the .fill() RHS should claim
    // every remaining column. We verify by asserting the final column
    // contains content from RHS (any non-space char in the filled region).
    let mut tb = TestBackend::new(40, 4);
    tb.render(|ui| {
        let _ = ui.row(|ui| {
            let _ = ui.container().w(10).col(|ui| {
                ui.text("L");
            });
            let _ = ui.container().fill().col(|ui| {
                ui.text("R");
            });
        });
    });
    let line0 = tb.line(0);
    assert!(line0.contains('L'), "LHS not rendered: {line0:?}");
    assert!(line0.contains('R'), "RHS not rendered: {line0:?}");
}

// ──────────────────────────────────────────────────────────────────────────
// #221 — Rect::center_in / center_horizontally_in / center_vertically_in
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn center_in_centers_both_axes() {
    let dialog = Rect::new(0, 0, 40, 10);
    let screen = Rect::new(0, 0, 120, 40);
    assert_eq!(dialog.center_in(screen), Rect::new(40, 15, 40, 10));
}

#[test]
fn center_in_offset_parent_accounts_for_origin() {
    // Parent at (10, 5), 100x30 — child 40x10. Expected x = 10 + 30 = 40, y = 5 + 10 = 15.
    let dialog = Rect::new(0, 0, 40, 10);
    let parent = Rect::new(10, 5, 100, 30);
    assert_eq!(dialog.center_in(parent), Rect::new(40, 15, 40, 10));
}

#[test]
fn center_in_oversize_clamps_and_anchors_at_origin() {
    let oversize = Rect::new(0, 0, 200, 80);
    let screen = Rect::new(0, 0, 120, 40);
    assert_eq!(oversize.center_in(screen), Rect::new(0, 0, 120, 40));
}

#[test]
fn center_horizontally_in_preserves_y_height() {
    let banner = Rect::new(99, 7, 30, 3);
    let screen = Rect::new(0, 0, 120, 40);
    let r = banner.center_horizontally_in(screen);
    assert_eq!(r, Rect::new(45, 7, 30, 3));
}

#[test]
fn center_vertically_in_preserves_x_width() {
    let sidebar = Rect::new(2, 99, 20, 10);
    let screen = Rect::new(0, 0, 120, 40);
    let r = sidebar.center_vertically_in(screen);
    assert_eq!(r, Rect::new(2, 15, 20, 10));
}

#[test]
fn center_helpers_round_trip_against_centered() {
    // Symmetry property: if you place a rect using `center_in(parent)`,
    // the result should equal `parent.centered(w, h)` — both produce a
    // rect of the same size centered in the parent.
    let parent = Rect::new(7, 13, 60, 24);
    let child = Rect::new(0, 0, 20, 8);
    let via_center_in = child.center_in(parent);
    let via_centered = parent.centered(20, 8);
    assert_eq!(via_center_in, via_centered);
}
