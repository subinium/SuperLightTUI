//! Issue #452: tooltip dwell delay, hover timeout and pointer-leave clearing.

use std::time::Duration;

use slt::{Context, EventBuilder, RunConfig, TestBackend};

fn toolbar(ui: &mut Context) {
    let _ = ui.row(|ui| {
        let _ = ui.button("SAVE").on_hover(ui, "TIP-SAVE");
        let _ = ui.button("OPEN");
        ui.tooltip("TIP-OPEN");
    });
}

fn find(tb: &TestBackend, label: &str) -> (u32, u32) {
    tb.find_text(label).expect("label rendered")
}

#[test]
fn tooltip_waits_for_the_delay_then_appears_without_input() {
    let mut tb = TestBackend::new(60, 10)
        .with_run_config(RunConfig::default().tooltip_delay(Duration::from_millis(500)));
    tb.render(toolbar);
    let (x, y) = find(&tb, "SAVE");
    tb.advance_time(Duration::ZERO).expect("freeze clock");

    tb.run_with_events(EventBuilder::new().mouse_move(x, y).build(), toolbar);
    tb.assert_not_contains("TIP-SAVE");

    tb.advance_time(Duration::from_millis(300)).expect("clock");
    tb.render(toolbar);
    tb.assert_not_contains("TIP-SAVE");

    // No further input: the next frame after the deadline shows it.
    tb.advance_time(Duration::from_millis(200)).expect("clock");
    tb.render(toolbar);
    tb.assert_contains("TIP-SAVE");
}

#[test]
fn sweeping_to_another_widget_restarts_the_wait() {
    let mut tb = TestBackend::new(60, 10)
        .with_run_config(RunConfig::default().tooltip_delay(Duration::from_millis(500)));
    tb.render(toolbar);
    let (sx, sy) = find(&tb, "SAVE");
    let (ox, oy) = find(&tb, "OPEN");
    tb.advance_time(Duration::ZERO).expect("freeze clock");

    tb.run_with_events(EventBuilder::new().mouse_move(sx, sy).build(), toolbar);
    tb.advance_time(Duration::from_millis(400)).expect("clock");
    tb.run_with_events(EventBuilder::new().mouse_move(ox, oy).build(), toolbar);
    tb.assert_not_contains("TIP-SAVE");
    tb.assert_not_contains("TIP-OPEN");

    tb.advance_time(Duration::from_millis(400)).expect("clock");
    tb.render(toolbar);
    tb.assert_not_contains("TIP-OPEN");
    tb.advance_time(Duration::from_millis(100)).expect("clock");
    tb.render(toolbar);
    tb.assert_contains("TIP-OPEN");
    tb.assert_not_contains("TIP-SAVE");
}

#[test]
fn zero_delay_shows_on_the_first_hovered_frame() {
    let mut tb = TestBackend::new(60, 10)
        .with_run_config(RunConfig::default().tooltip_delay(Duration::ZERO));
    tb.render(toolbar);
    let (x, y) = find(&tb, "SAVE");
    tb.run_with_events(EventBuilder::new().mouse_move(x, y).build(), toolbar);
    tb.assert_contains("TIP-SAVE");
}

#[test]
fn default_delay_is_documented_500ms() {
    assert_eq!(
        RunConfig::default().tooltip_delay,
        Duration::from_millis(500)
    );
    assert_eq!(RunConfig::default().hover_timeout, None);
}

fn hover_probe(ui: &mut Context, hovered: &mut bool) {
    *hovered = ui.button("TARGET").hovered;
}

#[test]
fn hover_timeout_clears_hover_after_idle_time() {
    let mut tb = TestBackend::new(30, 4)
        .with_run_config(RunConfig::default().hover_timeout(Some(Duration::from_secs(1))));
    let mut hovered = false;
    tb.render(|ui| hover_probe(ui, &mut hovered));
    let (x, y) = find(&tb, "TARGET");
    tb.advance_time(Duration::ZERO).expect("freeze clock");
    tb.run_with_events(EventBuilder::new().mouse_move(x, y).build(), |ui| {
        hover_probe(ui, &mut hovered)
    });
    assert!(hovered);
    tb.advance_time(Duration::from_millis(600)).expect("clock");
    tb.render(|ui| hover_probe(ui, &mut hovered));
    assert!(hovered, "still within the timeout");
    tb.advance_time(Duration::from_millis(400)).expect("clock");
    tb.render(|ui| hover_probe(ui, &mut hovered));
    assert!(!hovered, "cleared after the timeout without motion");
    tb.render(|ui| hover_probe(ui, &mut hovered));
    assert!(!hovered, "stays cleared");

    // Motion brings it back.
    tb.run_with_events(EventBuilder::new().mouse_move(x, y).build(), |ui| {
        hover_probe(ui, &mut hovered)
    });
    assert!(hovered);
}

#[test]
fn no_hover_timeout_keeps_hover() {
    let mut tb = TestBackend::new(30, 4);
    let mut hovered = false;
    tb.render(|ui| hover_probe(ui, &mut hovered));
    let (x, y) = find(&tb, "TARGET");
    tb.run_with_events(EventBuilder::new().mouse_move(x, y).build(), |ui| {
        hover_probe(ui, &mut hovered)
    });
    tb.advance_time(Duration::from_secs(60)).expect("clock");
    tb.render(|ui| hover_probe(ui, &mut hovered));
    assert!(hovered, "terminals send no leave event; hover persists");
}

#[test]
fn focus_loss_and_resize_clear_hover() {
    let mut tb = TestBackend::new(30, 4);
    let mut hovered = false;
    tb.render(|ui| hover_probe(ui, &mut hovered));
    let (x, y) = find(&tb, "TARGET");
    tb.run_with_events(EventBuilder::new().mouse_move(x, y).build(), |ui| {
        hover_probe(ui, &mut hovered)
    });
    assert!(hovered);
    tb.run_with_events(EventBuilder::new().focus_lost().build(), |ui| {
        hover_probe(ui, &mut hovered)
    });
    assert!(!hovered, "focus loss clears hover");

    tb.run_with_events(EventBuilder::new().mouse_move(x, y).build(), |ui| {
        hover_probe(ui, &mut hovered)
    });
    assert!(hovered);
    tb.run_with_events(EventBuilder::new().resize(30, 4).build(), |ui| {
        hover_probe(ui, &mut hovered)
    });
    assert!(!hovered, "resize clears hover");
}
