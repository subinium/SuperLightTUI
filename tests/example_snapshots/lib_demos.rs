//! Snapshots of the static log, keymap help, and Ctrl+C passthrough demos
//! (`examples/static_log.rs`, `examples/keymap_help.rs`,
//! `examples/ctrl_c_passthrough.rs`).
//!
//! Each example exposes a render function for a single deterministic frame.
//! The snapshots catch layout / styling regressions in the demos themselves
//! and serve as visual documentation for the features (issues #233, #236,
//! #238).
//!
//! Snapshot files live under `tests/snapshots/example_snapshots__*.snap`.
//!
//! Run with:
//! ```bash
//! cargo test --test example_snapshots lib_demos
//! cargo insta review  # to accept changes
//! ```

use slt::TestBackend;

#[allow(dead_code)]
#[path = "../../examples/static_log.rs"]
mod static_log;

#[allow(dead_code)]
#[path = "../../examples/keymap_help.rs"]
mod keymap_help;

#[allow(dead_code)]
#[path = "../../examples/ctrl_c_passthrough.rs"]
mod ctrl_c_passthrough;

fn snapshot_frame(name: &str, w: u32, h: u32, f: impl FnOnce(&mut slt::Context)) {
    let mut tb = TestBackend::new(w, h);
    tb.render(f);
    let body = tb.to_string_trimmed();
    insta::with_settings!({
        snapshot_path => "../snapshots",
        prepend_module_to_snapshot => false,
        omit_expression => true,
    }, {
        insta::assert_snapshot!(format!("example_snapshots__{name}"), body);
    });
}

#[test]
fn snapshot_static_log() {
    snapshot_frame("static_log", 80, 8, static_log::render);
}

#[test]
fn snapshot_keymap_help() {
    snapshot_frame("keymap_help", 80, 18, keymap_help::render_snapshot);
}

#[test]
fn snapshot_ctrl_c_passthrough() {
    snapshot_frame(
        "ctrl_c_passthrough",
        80,
        10,
        ctrl_c_passthrough::render_snapshot,
    );
}
