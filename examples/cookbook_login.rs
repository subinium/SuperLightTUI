//! Cookbook: login form with validation.
//!
//! Demonstrates:
//! - two `TextInputState` fields (Tab cycles focus automatically)
//! - masked password input
//! - validation: username non-empty, password length >= 6
//! - inline error rendering below the form
//! - Ctrl+Q or Esc to quit

use slt::{Border, Color, Context, KeyCode, KeyModifiers, TextInputState};

fn main() -> std::io::Result<()> {
    let mut username = TextInputState::with_placeholder("your name");
    let mut password = TextInputState::with_placeholder("at least 6 chars");
    password.masked = true;

    let mut error: Option<String> = None;
    let mut logged_in = false;
    let mut current_user = String::new();

    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        if logged_in {
            let _ = ui
                .bordered(Border::Rounded)
                .title("Cookbook — Login")
                .pad(2)
                .grow(1)
                .center()
                .col(|ui| {
                    ui.text(format!("Welcome, {current_user}!"))
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
            .title("Cookbook — Login")
            .pad(2)
            .gap(1)
            .grow(1)
            .col(|ui| {
                ui.text("Sign in").bold().fg(Color::Cyan);
                ui.text("Tab to switch fields. Enter to submit.").dim();

                let _ = ui.col(|ui| {
                    ui.text("Username").dim();
                    let _ = ui.text_input(&mut username);
                });

                let _ = ui.col(|ui| {
                    ui.text("Password").dim();
                    let _ = ui.text_input(&mut password);
                });

                let clicked_login = ui.button("Login").clicked;

                if submitted || clicked_login {
                    if username.value.trim().is_empty() {
                        error = Some("Username is required.".into());
                    } else if password.value.chars().count() < 6 {
                        error = Some("Password must be at least 6 characters.".into());
                    } else {
                        error = None;
                        current_user = username.value.trim().to_string();
                        logged_in = true;
                    }
                }

                if let Some(err) = &error {
                    ui.text(err).fg(Color::Red).bold();
                }

                ui.text("").dim();
                ui.text("Ctrl+Q or Esc to quit.").dim();
            });
    })
}
