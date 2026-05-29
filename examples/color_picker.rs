//! Interactive color picker demo.
//!
//! Run with `cargo run --example color_picker`.
//!
//! Arrow keys / `hjkl` move the swatch cursor, `Tab` toggles hex entry, and
//! `Enter` / `Space` confirms. The chosen color is shown live below the grid.

use slt::widgets::ColorPickerState;
use slt::{Border, Color, Context};

fn main() -> std::io::Result<()> {
    let mut picker = ColorPickerState::tailwind();
    let mut chosen: Color = picker.selected();

    slt::run(move |ui: &mut Context| {
        if ui.key('q') || ui.key_code(slt::KeyCode::Esc) {
            ui.quit();
        }

        let _ = ui
            .bordered(Border::Rounded)
            .p(1)
            .title("Color Picker")
            .col(|ui: &mut Context| {
                ui.text("Pick a color").bold();
                ui.text("←↑↓→ / hjkl move · Tab hex · Enter confirm · q quit")
                    .dim();

                if ui.color_picker(&mut picker).changed {
                    chosen = picker.selected();
                }

                ui.text(format!("selected: {chosen:?}")).fg(chosen).bold();
            });
    })
}
