//! v0.20.0 DX shorthand demo — four ergonomic helpers on a single screen.
//!
//! Demonstrates: #209 (Response::on_hover), #210 (animate_bool),
//! #220 (ContainerBuilder::fill), #221 (Rect::center_in).
//!
//! Run: `cargo run --example v020_dx_shortcuts`
//!
//! Keys:
//!   Space             — toggle the animated side panel (drives animate_bool)
//!   ? / h             — toggle the centered help overlay (drives Rect::center_in)
//!   Hover Save / Open — show the chained tooltip (drives Response::on_hover)
//!   q / Esc / Ctrl-Q  — quit (Ctrl-C may be bound to copy on macOS)
//!
//! Layout (80x24 minimum):
//!
//! ```text
//! +- v0.20 DX Shorthand Demo --------------------------------------+
//! | Press Space to toggle the panel, hover Save for a tooltip,     |
//! | press ? for the centered help overlay, Ctrl-Q to quit.         |
//! | +- Actions ----+ +- Status ---------------------------------+  |
//! | | [Save]       | | Shorthand helpers are about reading code |  |
//! | | [Open]       | | not writing it. fill() == grow(1).       |  |
//! | | panel_alpha  | |                                          |  |
//! | +--------------+ +------------------------------------------+  |
//! +----------------------------------------------------------------+
//! ```

use slt::{Border, Color, Context, KeyCode, KeyModifiers, Rect, Style};

/// Persistent state for the DX shorthand demo. Public so the v0.20 tour
/// can own a single [`DemoState`] across frames — clicks on the help
/// overlay or panel toggle would otherwise reset every frame.
#[derive(Default)]
pub struct DemoState {
    pub panel_open: bool,
    pub show_help: bool,
}

// Layout constants. Pinned here so the help overlay and the action column
// never drift between this demo, the snapshot test, and the doc-comment
// ASCII layout above.
const ACTIONS_W: u32 = 28;
const HELP_DIALOG_W: u32 = 44;
const HELP_DIALOG_H: u32 = 7;

// animate_bool fade thresholds. The color tier mirrors the standard
// "alarm-yellow-green" gauge palette so the demo composes cleanly with
// the showcase example.
const PANEL_ALPHA_HIGH: f64 = 0.5;
const PANEL_ALPHA_MID: f64 = 0.25;

fn main() -> std::io::Result<()> {
    let mut state = DemoState::default();

    slt::run_with(
        slt::RunConfig::default().mouse(true),
        move |ui: &mut Context| {
            // Standard exit-key policy: bare `q`, Esc, and Ctrl-Q. Ctrl-C is
            // intentionally NOT bound — many terminals (e.g. macOS Terminal,
            // iTerm2 with default copy-shortcut) intercept Ctrl-C, so it never
            // reaches the app reliably.
            if ui.key('q') || ui.key_code(KeyCode::Esc) || ui.key_mod('q', KeyModifiers::CONTROL) {
                ui.quit();
            }

            render(ui, &mut state);
        },
    )
}

/// Per-frame entry point. Handles Space (panel toggle) and ?/h (help
/// overlay toggle), then renders the demo body. Caller owns [`DemoState`]
/// so toggles persist across frames — this is the path the tour uses.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    if ui.key(' ') {
        state.panel_open = !state.panel_open;
    }
    if ui.key('?') || ui.key('h') {
        state.show_help = !state.show_help;
    }

    body(ui, state);
}

/// One-frame deterministic render entry point used by snapshot tests
/// (`tests/v020_dx_shortcuts_demo.rs`).
///
/// Stable defaults: panel closed, help overlay on. Reviewers can see the
/// centered help dialog (#221), the chained tooltip helpers (#209), and
/// the `fill()` status column (#220) all in one frame. The animated panel
/// alpha (#210) is exercised by the unit tests.
///
/// NEVER call this from a live loop or from another demo — toggles are
/// silently dropped because state never persists. Live embeddings should
/// call [`render`] with their own `&mut DemoState`.
pub fn render_snapshot(ui: &mut Context) {
    let mut snapshot = DemoState {
        panel_open: false,
        show_help: true,
    };
    body(ui, &mut snapshot);
}

fn body(ui: &mut Context, state: &mut DemoState) {
    let theme = ui.theme();
    let pad = theme.spacing.xs();
    let gap = theme.spacing.xs();

    // #210 — animate_bool: smooth 0..1 progress drives panel alpha and
    // width contribution. The id is keyed by demo so multiple animate_bool
    // calls in the same app don't collide.
    let panel_alpha = ui.animate_bool("dx_demo::panel_open", state.panel_open);

    let _ = ui
        .bordered(Border::Rounded)
        .title("v0.20 DX Shorthand Demo")
        .p(pad)
        .gap(gap)
        .col(|ui| {
            ui.text("Press Space to toggle the panel, hover Save for a tooltip,")
                .dim();
            ui.text("press ? for the centered help overlay, q / Esc / Ctrl-Q to quit.")
                .dim();

            let _ = ui.row(|ui| {
                render_actions_column(ui, panel_alpha);
                render_status_column(ui, panel_alpha);
            });
        });

    if state.show_help {
        render_centered_help(ui);
    }
}

fn render_actions_column(ui: &mut Context, panel_alpha: f64) {
    let gap = ui.theme().spacing.xs();
    let _ = ui.container().w(ACTIONS_W).gap(gap).col(|ui| {
        ui.text("Actions").bold();

        // #209 — on_hover: tooltip chained directly onto the button response.
        let _ = ui
            .button("Save")
            .on_hover(ui, "Saves the current document to disk.");
        let _ = ui
            .button("Open")
            .on_hover(ui, "Open an existing file from your project.");
        let _ = ui.button("Toggle Panel");

        ui.text(format!("panel_alpha = {panel_alpha:.2}")).dim();
    });
}

fn render_status_column(ui: &mut Context, panel_alpha: f64) {
    let pad = ui.theme().spacing.xs();
    // #220 — fill(): the right-hand column claims all remaining width
    // without writing `grow(1)`. Reads as plain English at the call site.
    let _ = ui
        .container()
        .fill()
        .border(Border::Single)
        .p(pad)
        .col(|ui| {
            ui.text("Status").bold();
            ui.text("Shorthand helpers are about reading code, not");
            ui.text("writing it. fill() == grow(1) — but plain English.");

            if panel_alpha > 0.0 {
                let pad = ui.theme().spacing.xs();
                let _ = ui.container().mt(pad).p(pad).col(|ui| {
                    let alpha_color = if panel_alpha > PANEL_ALPHA_HIGH {
                        Color::Green
                    } else if panel_alpha > PANEL_ALPHA_MID {
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
}

fn render_centered_help(ui: &mut Context) {
    // #221 — center_in: position a fixed-size help dialog dead-center on
    // the viewport using a raw-draw closure. The dotted outline shows the
    // geometry returned by `Rect::center_in`; in real apps the inner area
    // would host real widgets, but raw_draw keeps the demo geometry-only.
    let area_w = ui.width();
    let area_h = ui.height();
    let dot_style = Style::new().fg(Color::DarkGray);
    let label_style = Style::new().fg(Color::Cyan);

    let _ = ui.overlay(|ui| {
        ui.container().w(area_w).h(area_h).draw(move |buf, area| {
            let dialog = Rect::new(0, 0, HELP_DIALOG_W, HELP_DIALOG_H);
            let positioned = dialog.center_in(area);

            // Dotted outline traces the rect produced by center_in.
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

            // Inner label — confirms the geometry visually.
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
