//! Hot-reload a TOML theme file while the app runs.
//!
//! Requires the `serde` and `theme-watch` features:
//!
//! ```sh
//! cargo run --example theme_hot_reload --features theme-watch
//! ```
//!
//! On first launch this writes a sample `theme.toml` next to your working
//! directory (if one does not already exist). Edit any color in that file and
//! save — the running TUI restyles instantly, no recompile. Try changing
//! `primary` to `"#ff00ff"` or `bg` to `"indexed:236"`.

use slt::{Border, Color, Context, ThemeWatcher};

const THEME_PATH: &str = "theme.toml";

const SAMPLE_THEME: &str = r##"# SLT theme — edit any value and save to hot-reload.
# Colors accept "#rrggbb"/"#rgb" hex, named colors ("cyan"),
# or palette indices ("indexed:245").

[theme]
primary  = "#7aa2f7"
secondary = "#7dcfff"
accent   = "#bb9af7"
text     = "#a9b1d6"
text_dim = "indexed:245"
border   = "#3b4261"
bg       = "reset"
success  = "#9ece6a"
warning  = "#e0af68"
error    = "#f7768e"
is_dark  = true

# Optional per-widget overrides:
# [widgets.button]
# fg = "#ffffff"
"##;

fn main() -> std::io::Result<()> {
    // Seed a sample theme file the first time so the demo has something to watch.
    if !std::path::Path::new(THEME_PATH).exists() {
        std::fs::write(THEME_PATH, SAMPLE_THEME)?;
    }

    let mut watcher = ThemeWatcher::new(THEME_PATH).map_err(std::io::Error::other)?;
    let mut last_primary: Color = watcher.current().theme.primary;

    slt::run_with(
        slt::RunConfig::default().theme(watcher.current().theme),
        move |ui: &mut Context| {
            if ui.key('q') || ui.key_code(slt::KeyCode::Esc) {
                ui.quit();
            }

            // Poll for an edited theme.toml and apply it live.
            if let Some(tf) = watcher.poll() {
                last_primary = tf.theme.primary;
                ui.set_theme(tf.theme);
            }

            let _ = ui
                .bordered(Border::Rounded)
                .p(1)
                .title("Theme Hot Reload")
                .col(|ui: &mut Context| {
                    let text_color = ui.color(slt::ThemeColor::Text);
                    ui.text("Edit theme.toml and save — colors update live.")
                        .fg(text_color);
                    ui.text(format!("current primary: {}", last_primary.to_hex()))
                        .dim();
                    let _ = ui.button("Themed Button");
                    ui.text("Press 'q' or Esc to quit").dim();
                });
        },
    )
}
