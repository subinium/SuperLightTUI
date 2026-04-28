//! Visual regression snapshot for the v0.20.0 DX shorthand demo.
//!
//! Pins one frame of `examples/v020_dx_shortcuts.rs` (panel closed, help
//! overlay open) so layout regressions in any of the four shorthand APIs
//! show up as a snapshot diff:
//!
//! - `Response::on_hover` — the "Save" tooltip stays absent when the mouse
//!   isn't over the button (snapshot is mouse-less).
//! - `Context::animate_bool` — `panel_alpha` reads `0.00` on first call
//!   for a closed panel.
//! - `ContainerBuilder::fill()` — Status column fills the remaining row
//!   width identically to `.grow(1)`.
//! - `Rect::center_in` — the dotted border + themed Help panel are
//!   centered horizontally and vertically on the area.
//!
//! Update procedure:
//!
//! ```bash
//! cargo insta review
//! ```

use slt::TestBackend;

#[allow(dead_code)]
#[path = "../examples/v020_dx_shortcuts.rs"]
mod v020_dx_shortcuts;

#[test]
fn visual_v020_dx_shortcuts() {
    let mut tb = TestBackend::new(80, 20);
    tb.render(v020_dx_shortcuts::render_snapshot);
    let body = tb.to_string_trimmed();
    insta::with_settings!({
        snapshot_path => "snapshots",
        prepend_module_to_snapshot => false,
        omit_expression => true,
    }, {
        insta::assert_snapshot!("visual__v020_dx_shortcuts", body);
    });
}
