//! Snapshot tests for the v0.20.0 lib top-level demos
//! (`examples/v020_*.rs`).
//!
//! Each example exposes `pub fn render(ui: &mut Context)` for a single
//! deterministic frame. The snapshots catch layout / styling regressions
//! in the demos themselves and serve as visual documentation for the new
//! features (issues #233, #236, #238).
//!
//! Snapshot files live under `tests/snapshots/v020_lib_demos__*.snap`.
//!
//! Run with:
//! ```bash
//! cargo test --test v020_lib_demos
//! cargo insta review  # to accept changes
//! ```

use slt::TestBackend;

#[allow(dead_code)]
#[path = "../examples/v020_static_log.rs"]
mod v020_static_log;

#[allow(dead_code)]
#[path = "../examples/v020_keymap_help.rs"]
mod v020_keymap_help;

#[allow(dead_code)]
#[path = "../examples/v020_ctrl_c_passthrough.rs"]
mod v020_ctrl_c_passthrough;

fn snapshot_frame(name: &str, w: u32, h: u32, f: impl FnOnce(&mut slt::Context)) {
    let mut tb = TestBackend::new(w, h);
    tb.render(f);
    let body = tb.to_string_trimmed();
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
        omit_expression => true,
    }, {
        insta::assert_snapshot!(format!("v020_lib_demos__{name}"), body);
    });
}

#[test]
fn snapshot_v020_static_log() {
    snapshot_frame("v020_static_log", 80, 8, v020_static_log::render);
}

#[test]
fn snapshot_v020_keymap_help() {
    snapshot_frame("v020_keymap_help", 80, 18, v020_keymap_help::render);
}

#[test]
fn snapshot_v020_ctrl_c_passthrough() {
    snapshot_frame(
        "v020_ctrl_c_passthrough",
        80,
        10,
        v020_ctrl_c_passthrough::render,
    );
}
