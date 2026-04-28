//! v0.20.0 DX shorthand demo — exercises all four ergonomic helpers in a
//! single screen so reviewers can verify the API surface side-by-side.
//!
//! Features demoed:
//! 1. `Response::on_hover` — hover the "Save" button to see a chained tooltip.
//! 2. `Context::animate_bool` — press `Space` to toggle a panel; the visibility
//!    smoothly fades 0..1 over the default 12-tick duration.
//! 3. `ContainerBuilder::fill()` — the right-hand stats column uses `.fill()`
//!    instead of `.grow(1)` to claim the remaining width.
//! 4. `Rect::center_in` — the help-overlay rectangle is positioned via
//!    `dialog_rect.center_in(area)` inside a `draw_raw` callback.
//!
//! Run with: `cargo run --example v020_dx_shortcuts`

use slt::{Border, Color, Context, KeyCode, KeyModifiers, Rect, Style};

fn main() -> std::io::Result<()> {
    let mut panel_open = false;
    let mut show_help = false;

    slt::run_with(slt::RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        if ui.key(' ') {
            panel_open = !panel_open;
        }
        if ui.key('?') || ui.key('h') {
            show_help = !show_help;
        }

        render_demo(ui, panel_open, show_help);
    })
}

/// Render the demo screen for a given toggle state.
///
/// Exposed so the visual-snapshot test in `tests/v020_dx_shortcuts_demo.rs`
/// can pin a representative frame without touching the runtime event loop.
pub fn render(ui: &mut Context) {
    // Stable defaults for the snapshot: panel closed, help overlay on.
    // Reviewers can see the centered help dialog (#221), the chained
    // tooltip helpers (#209), and the fill() status column (#220) all in
    // one frame. The animated panel alpha (#210) is exercised by the
    // unit test suite.
    render_demo(ui, false, true);
}

fn render_demo(ui: &mut Context, panel_open: bool, show_help: bool) {
    // #210 — animate_bool: smooth 0..1 progress driving the side-panel
    // alpha and width contribution.
    let panel_alpha = ui.animate_bool("dx_demo::panel_open", panel_open);

    let _ = ui
        .bordered(Border::Rounded)
        .title("v0.20 DX Shorthand Demo")
        .p(1)
        .gap(1)
        .col(|ui| {
            ui.text("Press Space to toggle the panel, hover Save for a tooltip,")
                .dim();
            ui.text("press ? for the centered help overlay, Ctrl-Q to quit.")
                .dim();

            let _ = ui.row(|ui| {
                // Left column: fixed width with the interactive button row.
                let _ = ui.container().w(28).gap(1).col(|ui| {
                    ui.text("Actions").bold();
                    // #209 — on_hover: tooltip chained directly onto the
                    // button response.
                    let _ = ui
                        .button("Save")
                        .on_hover(ui, "Saves the current document to disk.");
                    let _ = ui
                        .button("Open")
                        .on_hover(ui, "Open an existing file from your project.");
                    let _ = ui.button("Toggle Panel");
                    ui.text(format!("panel_alpha = {:.2}", panel_alpha)).dim();
                });

                // #220 — fill(): the right-hand status column claims all
                // remaining width without writing `grow(1)`.
                let _ = ui.container().fill().border(Border::Single).p(1).col(|ui| {
                    ui.text("Status").bold();
                    ui.text("Shorthand helpers are about reading code, not");
                    ui.text("writing it. fill() == grow(1) — but plain English.");

                    if panel_alpha > 0.0 {
                        let _ = ui.container().mt(1).p(1).col(|ui| {
                            let alpha_color = if panel_alpha > 0.5 {
                                Color::Green
                            } else if panel_alpha > 0.25 {
                                Color::Yellow
                            } else {
                                Color::DarkGray
                            };
                            ui.text(format!("Animated panel ({:.0}%)", panel_alpha * 100.0))
                                .fg(alpha_color)
                                .bold();
                            ui.text("Smoothly tweened via animate_bool.");
                        });
                    }
                });
            });
        });

    // #221 — center_in: position a fixed-size help dialog dead-center on
    // the viewport using a raw-draw closure. The dotted outline shows the
    // geometry returned by `Rect::center_in`; in real apps you would draw
    // the help text inside that rect via `buf.set_char` or by calling
    // back into `Context` widgets.
    if show_help {
        let area_w = ui.width();
        let area_h = ui.height();
        let dot_style = Style::new().fg(Color::DarkGray);
        let label_style = Style::new().fg(Color::Cyan);
        let _ = ui.overlay(|ui| {
            ui.container().w(area_w).h(area_h).draw(move |buf, area| {
                let dialog = Rect::new(0, 0, 44, 7);
                let positioned = dialog.center_in(area);
                // Dotted outline shows where center_in placed the rect.
                for y in positioned.rows() {
                    for x in positioned.x..positioned.right() {
                        let on_edge = y == positioned.y
                            || y + 1 == positioned.bottom()
                            || x == positioned.x
                            || x + 1 == positioned.right();
                        if on_edge {
                            buf.set_char(x, y, '·', dot_style);
                        }
                    }
                }
                // Render a label inside the centered rect so reviewers
                // can confirm the geometry is what they expect.
                let label = "Help (centered via center_in)";
                let label_w = label.chars().count() as u32;
                if positioned.width >= label_w + 2 && positioned.height >= 3 {
                    let lx = positioned.x + (positioned.width - label_w) / 2;
                    let ly = positioned.y + positioned.height / 2;
                    for (i, ch) in label.chars().enumerate() {
                        buf.set_char(lx + i as u32, ly, ch, label_style);
                    }
                }
            });
        });
    }
}
