//! Demonstrates `overlay_at(Anchor::*)` and `overlay_at_offset(Anchor::*, dx, dy)` —
//! pin floating content to any of the 9 compass positions, with an optional
//! cell-offset inset toward the viewport center.
//!
//! Each badge sits in its own overlay with the corresponding [`Anchor`].
//! The base layer renders a centered title behind them; the overlays float
//! on top without dimming (use [`modal_at`] for a dimmed variant).
//!
//! - Outer flush badges (`TL`..`BR`) call [`Context::overlay_at`] — the
//!   widget sits flush against the screen edge / corner.
//! - Inset badges (`tl*`, `tr*`, `bl*`, `br*`) call
//!   [`Context::overlay_at_offset`] with `(dx=2, dy=1)` — 2 cells
//!   horizontally and 1 row vertically inset toward the center, the SLT
//!   analog of CSS `place-self: end end; bottom: 1px; right: 2px;`.
//!
//! Press `q` or `Esc` to quit.

use slt::{Anchor, Border, Color, Context, KeyCode};

/// Render one frame of the overlay-anchor demo.
///
/// Exposed as a free function so that visual snapshot tests in
/// `tests/visual_snapshots.rs` can drive the same rendering logic
/// through a `TestBackend` without going through `slt::run`.
pub fn render(ui: &mut Context) {
    if ui.key('q') || ui.key_code(KeyCode::Esc) {
        ui.quit();
    }

    // Base layer — visible behind every overlay.
    let _ = ui
        .bordered(Border::Rounded)
        .title("overlay_at + overlay_at_offset demo")
        .p(2)
        .grow(1)
        .col(|ui| {
            ui.text("Press q to quit.").dim();
            ui.spacer();
            ui.text("Outer flush badges = overlay_at(anchor)")
                .fg(Color::Cyan)
                .bold();
            ui.text("Inner inset badges = overlay_at_offset(anchor, 2, 1)")
                .fg(Color::Magenta)
                .bold();
            ui.text("CSS analog: place-self + top/right/bottom/left inset.")
                .dim();
            ui.spacer();
        });

    // Flush badges (no inset). Each lives in its own overlay, so flexbox
    // doesn't compete with the base layer.
    for (anchor, label, color) in ANCHORS {
        let _ = ui.overlay_at(*anchor, |ui| {
            ui.text(*label).fg(Color::Black).bg(*color);
        });
    }

    // Inset badges — same anchors, shifted (dx=2, dy=1) toward the
    // viewport center. Compare visually with the flush corners above.
    for (anchor, label) in INSET_BADGES {
        let _ = ui.overlay_at_offset(*anchor, 2, 1, |ui| {
            ui.text(*label).fg(Color::White).bg(Color::Magenta);
        });
    }
}

fn main() -> std::io::Result<()> {
    slt::run(render)
}

const ANCHORS: &[(Anchor, &str, Color)] = &[
    (Anchor::TopLeft, " TL ", Color::Cyan),
    (Anchor::TopCenter, " TC ", Color::Cyan),
    (Anchor::TopRight, " TR ", Color::Cyan),
    (Anchor::CenterLeft, " ML ", Color::Yellow),
    (Anchor::Center, " CC ", Color::Magenta),
    (Anchor::CenterRight, " MR ", Color::Yellow),
    (Anchor::BottomLeft, " BL ", Color::Green),
    (Anchor::BottomCenter, " BC ", Color::Green),
    (Anchor::BottomRight, " BR ", Color::Green),
];

// Same compass corners, but inset by (2 cols, 1 row) toward the center —
// the typical "16px inset corner badge" pattern from CSS layouts.
const INSET_BADGES: &[(Anchor, &str)] = &[
    (Anchor::TopLeft, " tl* "),
    (Anchor::TopRight, " tr* "),
    (Anchor::BottomLeft, " bl* "),
    (Anchor::BottomRight, " br* "),
];
