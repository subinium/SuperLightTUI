//! Demo: Design System features (v0.17 preview)
//!
//! Tests ThemeColor, Spacing tokens, ContainerStyle extends,
//! WidgetTheme, and new theme presets.
//!
//! Archetype: **Standard** (full-canvas, no overlay, no scrollback).
//!
//! §2 (Demo Guide): exposes `pub fn render(ui, &mut DemoState)` so a
//! composing demo can preserve the selected theme index, the
//! `show_themes` toggle, the typed input value, the list cursor, and
//! the counter across tab switches.
//!
//! Run: cargo run --example demo_design_system --features crossterm

use slt::{
    Border, Color, ContainerStyle, Context, ListState, RunConfig, Spacing, TextInputState, Theme,
    ThemeColor, WidgetColors, WidgetTheme,
};

// ── Theme-aware styles using ThemeColor ──────────────────────────────

const CARD: ContainerStyle = ContainerStyle::new()
    .border(Border::Rounded)
    .p(1)
    .gap(1)
    .theme_bg(ThemeColor::Surface)
    .theme_border_fg(ThemeColor::Border);

const CARD_PRIMARY: ContainerStyle =
    ContainerStyle::extending(&CARD).theme_border_fg(ThemeColor::Primary);

const CARD_ERROR: ContainerStyle =
    ContainerStyle::extending(&CARD).theme_border_fg(ThemeColor::Error);

const CARD_SUCCESS: ContainerStyle =
    ContainerStyle::extending(&CARD).theme_border_fg(ThemeColor::Success);

fn build_themes() -> Vec<(&'static str, Theme)> {
    vec![
        ("Dark", Theme::dark()),
        ("Light", Theme::light()),
        ("Dracula", Theme::dracula()),
        ("Catppuccin", Theme::catppuccin()),
        ("Nord", Theme::nord()),
        ("Tokyo Night", Theme::tokyo_night()),
        ("Gruvbox Dark", Theme::gruvbox_dark()),
        ("One Dark", Theme::one_dark()),
        ("Solarized Dark", Theme::solarized_dark()),
        ("Solarized Light", Theme::solarized_light()),
    ]
}

/// Persistent state: theme cursor, theme-browser toggle, plus the
/// inputs/list/counter the showcase exposes.
pub struct DemoState {
    pub themes: Vec<(&'static str, Theme)>,
    pub theme_idx: usize,
    pub show_themes: bool,
    pub input: TextInputState,
    pub list: ListState,
    pub counter: u32,
}

impl DemoState {
    pub fn new() -> Self {
        let mut input = TextInputState::new();
        input.placeholder = "Type here...".into();
        let list = ListState::new(vec![
            "Alpha".to_string(),
            "Beta".to_string(),
            "Gamma".to_string(),
            "Delta".to_string(),
        ]);
        Self {
            themes: build_themes(),
            theme_idx: 0,
            show_themes: false,
            input,
            list,
            counter: 0,
        }
    }
}

impl Default for DemoState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render one frame of the design-system demo.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    let (theme_name, theme) = state.themes[state.theme_idx];
    ui.set_theme(theme);

    // Cache colors before any container calls
    let primary = ui.color(ThemeColor::Primary);
    let text_dim = ui.color(ThemeColor::TextDim);
    let surface_text = ui.color(ThemeColor::SurfaceText);
    let sp = ui.spacing();

    if ui.key_code(slt::KeyCode::Right) {
        state.theme_idx = (state.theme_idx + 1) % state.themes.len();
    }
    if ui.key_code(slt::KeyCode::Left) {
        state.theme_idx = state
            .theme_idx
            .checked_sub(1)
            .unwrap_or(state.themes.len() - 1);
    }
    if ui.key('t') {
        state.show_themes = !state.show_themes;
    }

    // ── Header ───────────────────────────────────────────────────
    let _ = ui.container().gap(sp.xs()).col(|ui| {
        ui.text("Design System Demo (v0.17)").bold().fg(primary);
        ui.text(format!(
            "Theme: {} | Left/Right cycle themes | t: toggle theme view | q: quit",
            theme_name
        ))
        .fg(text_dim);
        let _ = ui.separator();
    });

    if state.show_themes {
        // ── Theme browser ────────────────────────────────────────
        let _ = ui.container().gap(sp.sm()).col(|ui| {
            ui.text("All Theme Presets").bold();
            for (i, (name, t)) in state.themes.iter().enumerate() {
                let marker = if i == state.theme_idx { "> " } else { "  " };
                let _ = ui.container().gap(1).row(|ui| {
                    ui.text(format!("{}{}", marker, name)).fg(t.primary).bold();
                    for (label, color) in [
                        ("pri", t.primary),
                        ("sec", t.secondary),
                        ("acc", t.accent),
                        ("suc", t.success),
                        ("wrn", t.warning),
                        ("err", t.error),
                    ] {
                        let fg = Color::contrast_fg(color);
                        let _ = ui.container().bg(color).px(1).col(|ui| {
                            ui.text(label).fg(fg);
                        });
                    }
                });
            }
        });
    } else {
        // ── Showcase ─────────────────────────────────────────────
        let _ = ui.container().gap(sp.sm()).col(|ui| {
            // Row 1: Style extends + ThemeColor
            ui.text("Style Extends + ThemeColor").bold();
            let _ = ui.container().gap(sp.xs()).row(|ui| {
                let _ = ui.container().apply(&CARD).grow(1).col(|ui| {
                    ui.text("CARD (base)").fg(surface_text);
                    ui.text("theme_bg: Surface").fg(text_dim);
                });
                let _ = ui.container().apply(&CARD_PRIMARY).grow(1).col(|ui| {
                    ui.text("CARD_PRIMARY").fg(surface_text);
                    ui.text("extends CARD").fg(text_dim);
                });
                let _ = ui.container().apply(&CARD_ERROR).grow(1).col(|ui| {
                    ui.text("CARD_ERROR").fg(surface_text);
                    ui.text("extends CARD").fg(text_dim);
                });
                let _ = ui.container().apply(&CARD_SUCCESS).grow(1).col(|ui| {
                    ui.text("CARD_SUCCESS").fg(surface_text);
                    ui.text("extends CARD").fg(text_dim);
                });
            });

            // Row 2: Spacing tokens
            ui.text("Spacing Tokens").bold();
            let _ = ui.container().gap(sp.xs()).row(|ui| {
                let scale = Spacing::new(1);
                for (name, val) in [
                    ("xs", scale.xs()),
                    ("sm", scale.sm()),
                    ("md", scale.md()),
                    ("lg", scale.lg()),
                    ("xl", scale.xl()),
                ] {
                    let _ = ui.container().apply(&CARD).p(val).grow(1).col(|ui| {
                        ui.text(format!("sp.{}() = {}", name, val)).fg(surface_text);
                    });
                }
            });

            // Row 3: WidgetTheme + interactive widgets
            ui.text("WidgetTheme (buttons have cyan accent)").bold();
            let _ = ui.container().gap(sp.sm()).row(|ui| {
                let _ = ui.container().apply(&CARD).grow(1).col(|ui| {
                    if ui.button("Increment").clicked {
                        state.counter += 1;
                    }
                    ui.text(format!("Counter: {}", state.counter));
                    if ui.button("Reset").clicked {
                        state.counter = 0;
                    }
                });
                let _ = ui.container().apply(&CARD).grow(1).col(|ui| {
                    ui.text("Text Input:");
                    let _ = ui.text_input(&mut state.input);
                    ui.text(format!("Value: \"{}\"", state.input.value))
                        .fg(text_dim);
                });
                let _ = ui.container().apply(&CARD).grow(1).col(|ui| {
                    ui.text("List:");
                    let _ = ui.list(&mut state.list);
                });
            });

            // Row 4: Contrast helpers
            ui.text("Contrast Helpers").bold();
            let test_bgs = [
                ("Primary", theme.primary),
                ("Error", theme.error),
                ("Success", theme.success),
                ("Surface", theme.surface),
            ];
            let _ = ui.container().gap(sp.xs()).row(|ui| {
                for (label, bg_color) in test_bgs {
                    let fg = Color::contrast_fg(bg_color);
                    let ratio = Color::contrast_ratio_f64(fg, bg_color);
                    let _ = ui.container().bg(bg_color).p(1).grow(1).col(|ui| {
                        ui.text(label).fg(fg).bold();
                        ui.text(format!("ratio: {:.1}", ratio)).fg(fg);
                    });
                }
            });
        });
    }
}

fn main() -> std::io::Result<()> {
    let widget_theme = WidgetTheme::new().button(WidgetColors::new().accent(Color::Cyan));
    let config = RunConfig::default().mouse(true).widget_theme(widget_theme);

    let mut state = DemoState::new();
    slt::run_with(config, move |ui: &mut Context| {
        if ui.key('q') || ui.key_code(slt::KeyCode::Esc) {
            ui.quit();
        }
        render(ui, &mut state);
    })
}
