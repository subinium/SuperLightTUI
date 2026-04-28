//! v0.20.0 lib top-level feature tests (issues #233, #236, #238).
//!
//! Covers:
//! - `ui.static_log(...)` accumulates lines on the active frame, drained
//!   into the runtime's scrollback channel after the frame closure runs.
//! - `ui.publish_keymap(...)` registers a widget's `key_help()` slice for
//!   the current frame; `ui.published_keymaps()` returns them.
//! - `RunConfig::handle_ctrl_c(true|false)` toggles whether Ctrl+C exits
//!   the loop (default `true`) or is delivered as a normal `Event::Key`
//!   (`false`, RataTUI parity).

use slt::keymap::WidgetKeyHelp;
use slt::{EventBuilder, KeyCode, KeyModifiers, PublishedKeymap, RunConfig, TestBackend};

// ─── #233: static_log ──────────────────────────────────────────────────

#[test]
fn static_log_appends_in_call_order() {
    let mut tb = TestBackend::new(40, 4);
    let mut captured: Vec<String> = Vec::new();
    tb.render(|ui| {
        ui.static_log("first");
        ui.static_log("second");
        ui.static_log("third");
        captured = ui.take_static_log();
        ui.text("dynamic");
    });
    assert_eq!(captured, vec!["first", "second", "third"]);
}

#[test]
fn static_log_accepts_into_string() {
    let mut tb = TestBackend::new(40, 4);
    let mut captured: Vec<String> = Vec::new();
    tb.render(|ui| {
        ui.static_log("&str works");
        ui.static_log(String::from("String works"));
        ui.static_log(format!("formatted: {}", 42));
        captured = ui.take_static_log();
    });
    assert_eq!(
        captured,
        vec!["&str works", "String works", "formatted: 42"]
    );
}

#[test]
fn static_log_take_resets_buffer() {
    let mut tb = TestBackend::new(40, 4);
    let mut second_drain: Vec<String> = Vec::new();
    tb.render(|ui| {
        ui.static_log("a");
        let _ = ui.take_static_log();
        ui.static_log("b");
        second_drain = ui.take_static_log();
    });
    assert_eq!(second_drain, vec!["b"]);
}

#[test]
fn static_log_pre_frame_dynamic_independent() {
    // Dynamic frame content rendering should be unaffected by static_log calls.
    let mut tb = TestBackend::new(40, 4);
    tb.render(|ui| {
        ui.static_log("scrollback noise");
        ui.text("dynamic line");
    });
    tb.assert_contains("dynamic line");
}

// ─── #236: keymap publishing ───────────────────────────────────────────

struct CounterWidget;
impl WidgetKeyHelp for CounterWidget {
    fn key_help(&self) -> &'static [(&'static str, &'static str)] {
        const HELP: &[(&str, &str)] = &[("↑/k", "increment"), ("↓/j", "decrement"), ("r", "reset")];
        HELP
    }
}

#[test]
fn publish_keymap_is_queryable_within_frame() {
    let mut tb = TestBackend::new(40, 4);
    let mut count: usize = 0;
    let mut name: Option<&'static str> = None;
    tb.render(|ui| {
        let counter = CounterWidget;
        ui.publish_keymap("counter", counter.key_help());
        let entries = ui.published_keymaps();
        count = entries.len();
        if let Some(first) = entries.first() {
            name = Some(first.name);
        }
    });
    assert_eq!(count, 1);
    assert_eq!(name, Some("counter"));
}

#[test]
fn publish_keymap_clears_between_frames() {
    let mut tb = TestBackend::new(40, 4);
    let mut len_first = 0usize;
    let mut len_second = 0usize;
    tb.render(|ui| {
        ui.publish_keymap("a", &[("k", "up")]);
        ui.publish_keymap("b", &[("j", "down")]);
        len_first = ui.published_keymaps().len();
    });
    assert_eq!(len_first, 2);
    tb.render(|ui| {
        ui.publish_keymap("c", &[("h", "left")]);
        len_second = ui.published_keymaps().len();
    });
    assert_eq!(len_second, 1);
}

#[test]
fn published_keymap_returns_const_slice_by_name() {
    let mut tb = TestBackend::new(40, 4);
    let mut bindings_len = 0usize;
    tb.render(|ui| {
        let counter = CounterWidget;
        ui.publish_keymap("counter", counter.key_help());
        let entries = ui.published_keymaps();
        bindings_len = entries[0].bindings.len();
    });
    assert_eq!(bindings_len, 3);
}

#[test]
fn keymap_help_overlay_renders_when_open() {
    let mut tb = TestBackend::new(80, 14);
    tb.render(|ui| {
        ui.publish_keymap("rich_log", &[("k", "scroll up"), ("j", "scroll down")]);
        ui.keymap_help_overlay(true);
    });
    let dump = tb.to_string_trimmed();
    assert!(
        dump.contains("Keyboard shortcuts"),
        "overlay missing title — dump:\n{dump}"
    );
    assert!(
        dump.contains("rich_log"),
        "overlay missing keymap name — dump:\n{dump}"
    );
    assert!(
        dump.contains("scroll up"),
        "overlay missing binding description — dump:\n{dump}"
    );
}

#[test]
fn keymap_help_overlay_no_op_when_closed() {
    let mut tb = TestBackend::new(60, 14);
    tb.render(|ui| {
        ui.publish_keymap("rich_log", &[("k", "scroll up")]);
        ui.keymap_help_overlay(false);
    });
    let dump = tb.to_string_trimmed();
    assert!(
        !dump.contains("Keyboard shortcuts"),
        "overlay should not render: {dump}"
    );
}

#[test]
fn published_keymap_construct_const() {
    const PK: PublishedKeymap = PublishedKeymap::new("scope", &[("k", "up")]);
    assert_eq!(PK.name, "scope");
    assert_eq!(PK.bindings.len(), 1);
}

// ─── #238: Ctrl+C opt-out ──────────────────────────────────────────────

#[test]
fn run_config_default_handles_ctrl_c() {
    let cfg = RunConfig::default();
    assert!(cfg.handle_ctrl_c, "default must preserve v0.19 behavior");
}

#[test]
fn run_config_builder_disables_ctrl_c() {
    let cfg = RunConfig::default().handle_ctrl_c(false);
    assert!(!cfg.handle_ctrl_c);
}

#[test]
fn run_config_handle_ctrl_c_round_trip() {
    let cfg = RunConfig::default()
        .handle_ctrl_c(false)
        .handle_ctrl_c(true);
    assert!(cfg.handle_ctrl_c);
}

// Integration-level test for the event flow: when handle_ctrl_c=false the
// frame closure is supposed to observe Ctrl+C as `KeyCode::Char('c')` +
// CONTROL modifier. We exercise this through `TestBackend::run_with_events`,
// which feeds events directly to the kernel without going through the
// `poll_events` Ctrl+C guard. This is the same code path the documented
// opt-out unblocks.
#[test]
fn ctrl_c_opt_out_delivers_event_to_closure() {
    let mut tb = TestBackend::new(40, 4);
    let events = EventBuilder::new()
        .key_with(KeyCode::Char('c'), KeyModifiers::CONTROL)
        .build();
    let mut observed = false;
    tb.run_with_events(events, |ui| {
        if ui.key_mod('c', KeyModifiers::CONTROL) {
            observed = true;
        }
    });
    assert!(
        observed,
        "frame closure should see Ctrl+C as a normal key event"
    );
}

#[test]
fn ctrl_c_opt_out_then_quit_works() {
    let mut tb = TestBackend::new(40, 4);
    let events = EventBuilder::new()
        .key_with(KeyCode::Char('c'), KeyModifiers::CONTROL)
        .build();
    let mut quit_called = false;
    tb.run_with_events(events, |ui| {
        if ui.key_mod('c', KeyModifiers::CONTROL) {
            ui.quit();
            quit_called = true;
        }
    });
    assert!(quit_called);
}

// ─── poll_events flag plumbing (smoke test on the gating logic) ────────
//
// The `poll_events` function is `#[cfg(feature = "crossterm")]` and private,
// so we cannot call it directly. Instead, exercise the public API surface
// to confirm `RunConfig::handle_ctrl_c` is wired through every entry point.

#[test]
fn run_config_clones_handle_ctrl_c_through_chain() {
    // Builders must be chainable without losing the flag.
    let cfg = RunConfig::default()
        .tick_rate(std::time::Duration::from_millis(8))
        .handle_ctrl_c(false)
        .max_fps(120);
    assert!(!cfg.handle_ctrl_c);
}
