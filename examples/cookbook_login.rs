//! Cookbook: login form with validation.
//!
//! Archetype: **Standard** (full-canvas, no overlay, no scrollback).
//!
//! Demonstrates:
//! - two `TextInputState` fields (Tab cycles focus automatically)
//! - masked password input
//! - validation: username non-empty, password length >= 6
//! - inline error rendering below the form
//! - Ctrl+Q or Esc to quit
//!
//! §2 (Demo Guide): exposes `pub fn render(ui, &mut DemoState)` so a
//! composing demo can preserve the typed username/password and the
//! `logged_in` welcome state across tab switches. The standalone
//! `main()` is a thin wrapper.

use slt::{Border, Color, Context, KeyCode, KeyModifiers, TextInputState};

/// Persistent form state. `error` mirrors the inline validation
/// message; `logged_in` flips after a successful submit so subsequent
/// frames render the welcome view.
pub struct DemoState {
    pub username: TextInputState,
    pub password: TextInputState,
    pub error: Option<String>,
    pub logged_in: bool,
    pub current_user: String,
}

impl DemoState {
    pub fn new() -> Self {
        let mut password = TextInputState::with_placeholder("at least 6 chars");
        password.masked = true;
        Self {
            username: TextInputState::with_placeholder("your name"),
            password,
            error: None,
            logged_in: false,
            current_user: String::new(),
        }
    }
}

impl Default for DemoState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render one frame of the login demo. Caller owns `DemoState` so the
/// typed text and `logged_in` flag persist across frames.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    if state.logged_in {
        let _ = ui
            .bordered(Border::Rounded)
            .title("Cookbook: Login")
            .p(2)
            .grow(1)
            .center()
            .col(|ui| {
                ui.text(format!("Welcome, {}!", state.current_user))
                    .bold()
                    .fg(Color::Green);
                ui.text("").dim();
                ui.text("Ctrl+Q or Esc to quit.").dim();
            });
        return;
    }

    let submitted = ui.key_code(KeyCode::Enter);

    let _ = ui
        .bordered(Border::Rounded)
        .title("Cookbook: Login")
        .p(2)
        .gap(1)
        .grow(1)
        .col(|ui| {
            ui.text("Sign in").bold().fg(Color::Cyan);
            ui.text("Tab to switch fields. Enter to submit.").dim();

            let _ = ui.col(|ui| {
                ui.text("Username").dim();
                let _ = ui.text_input(&mut state.username);
            });

            let _ = ui.col(|ui| {
                ui.text("Password").dim();
                let _ = ui.text_input(&mut state.password);
            });

            let clicked_login = ui.button("Login").clicked;

            if submitted || clicked_login {
                if state.username.value.trim().is_empty() {
                    state.error = Some("Username is required.".into());
                } else if state.password.value.chars().count() < 6 {
                    state.error = Some("Password must be at least 6 characters.".into());
                } else {
                    state.error = None;
                    state.current_user = state.username.value.trim().to_string();
                    state.logged_in = true;
                }
            }

            if let Some(err) = &state.error {
                ui.text(err.as_str()).fg(Color::Red).bold();
            }

            ui.text("").dim();
            ui.text("Ctrl+Q or Esc to quit.").dim();
        });
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::new();
    slt::run(move |ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        render(ui, &mut state);
    })
}
