use slt::widgets::*;
use slt::*;

#[test]
fn render_demo_basic_layout() {
    let mut tb = TestBackend::new(80, 24);
    let mut input = TextInputState::with_placeholder("Type here...");
    let spinner = SpinnerState::dots();
    let mut dark = true;
    let mut notif = true;

    tb.render(|ui| {
        ui.bordered(Border::Rounded)
            .title("SLT Demo")
            .pad(1)
            .grow(1)
            .col(|ui| {
                ui.row(|ui| {
                    ui.text("Super Light TUI").bold().fg(Color::Cyan);
                    ui.spacer();
                    ui.text("dark").dim();
                });
                ui.separator();
                ui.bordered(Border::Single)
                    .title("Input")
                    .pad(1)
                    .grow(1)
                    .col(|ui| {
                        ui.text("Name:").bold();
                        ui.text_input(&mut input);
                    });
                ui.row(|ui| {
                    ui.spinner(&spinner);
                    ui.text(" Loading...").dim();
                });
            });
    });

    let output = tb.to_string_trimmed();
    println!("{}", output);

    tb.assert_contains("SLT Demo");
    tb.assert_contains("Super Light TUI");
    tb.assert_contains("Name:");
    tb.assert_contains("Type here...");
}

#[test]
fn render_justify_layout() {
    let mut tb = TestBackend::new(60, 10);
    tb.render(|ui| {
        ui.bordered(Border::Single)
            .space_between()
            .pad(1)
            .row(|ui| {
                ui.text("A");
                ui.text("B");
                ui.text("C");
            });
    });
    let output = tb.to_string_trimmed();
    println!("Justify output:\n{}", output);
    tb.assert_contains("A");
    tb.assert_contains("B");
    tb.assert_contains("C");
}

#[test]
fn render_link() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.link("Click here", "https://example.com");
    });
    tb.assert_contains("Click here");
}

#[test]
fn render_form_field() {
    let mut tb = TestBackend::new(40, 10);
    let mut field = FormField::new("Email").placeholder("you@example.com");
    tb.render(|ui| {
        ui.form_field(&mut field);
    });
    let output = tb.to_string_trimmed();
    println!("Form field output:\n{}", output);
    tb.assert_contains("Email");
    tb.assert_contains("you@example.com");
}

#[test]
fn render_modal() {
    let mut tb = TestBackend::new(60, 20);
    tb.render(|ui| {
        ui.text("Background content");
        ui.modal(|ui| {
            ui.bordered(Border::Rounded).pad(1).col(|ui| {
                ui.text("Modal title");
                ui.text("Modal body");
            });
        });
    });
    let output = tb.to_string_trimmed();
    println!("Modal output:\n{}", output);
    tb.assert_contains("Modal title");
}

#[test]
fn render_overlay() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.text("Base text");
        ui.overlay(|ui| {
            ui.text("Overlay text");
        });
    });
    let output = tb.to_string_trimmed();
    println!("Overlay output:\n{}", output);
    tb.assert_contains("Overlay text");
}
