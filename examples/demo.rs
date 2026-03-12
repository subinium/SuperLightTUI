use slt::{
    Align, Border, Color, Context, ListState, ScrollState, SpinnerState, TableState,
    TextInputState, TextareaState, Theme,
};

fn main() -> std::io::Result<()> {
    let mut input = TextInputState::with_placeholder("Type here...");
    let mut textarea = TextareaState::new();
    let mut list = ListState::new(vec!["Rust", "Go", "Python", "TypeScript", "Zig", "C++"]);
    let mut table = TableState::new(
        vec!["Name", "Lang", "Arch"],
        vec![
            vec!["SLT", "Rust", "Immediate"],
            vec!["Ratatui", "Rust", "Retained"],
            vec!["Bubbletea", "Go", "Elm"],
            vec!["Ink", "JS/TS", "React"],
        ],
    );
    let spinner = SpinnerState::dots();
    let mut progress = 0.0_f64;
    let mut scroll = ScrollState::new();
    let mut dark = true;
    let mut notif = true;
    let mut autosave = false;
    let mut vim = false;
    let mut saves: u32 = 0;

    slt::run_with(
        slt::RunConfig {
            mouse: true,
            ..Default::default()
        },
        |ui: &mut Context| {
            if ui.key('q') {
                ui.quit();
            }
            if ui.key('t') {
                dark = !dark;
            }
            if ui.key('h') {
                progress = (progress - 0.05).max(0.0);
            }
            if ui.key('l') {
                progress = (progress + 0.05).min(1.0);
            }
            ui.set_theme(if dark { Theme::dark() } else { Theme::light() });

            ui.bordered(Border::Rounded)
                .title("SLT Demo")
                .pad(1)
                .grow(1)
                .col(|ui| {
                    ui.row(|ui| {
                        ui.text("Super Light TUI").bold().fg(Color::Cyan);
                        ui.spacer();
                        ui.text(if dark { "dark" } else { "light" }).dim();
                    });
                    ui.separator();

                    ui.scrollable(&mut scroll).grow(1).col(|ui| {
                        section(ui, "STYLING");
                        ui.row(|ui| {
                            ui.bordered(Border::Single)
                                .title("Text")
                                .pad(1)
                                .grow(1)
                                .col(|ui| {
                                    ui.text("Bold").bold();
                                    ui.text("Dim").dim();
                                    ui.text("Italic").italic();
                                    ui.text("Underline").underline();
                                    ui.text("Strike").strikethrough();
                                    ui.text("Reversed").reversed();
                                });
                            ui.bordered(Border::Single)
                                .title("Colors")
                                .pad(1)
                                .grow(1)
                                .col(|ui| {
                                    ui.text("Red").fg(Color::Red);
                                    ui.text("Green").fg(Color::Green);
                                    ui.text("Yellow").fg(Color::Yellow);
                                    ui.text("Blue").fg(Color::Blue);
                                    ui.text("Magenta").fg(Color::Magenta);
                                    ui.text("Cyan").fg(Color::Cyan);
                                });
                            ui.bordered(Border::Single)
                                .title("Borders")
                                .pad(1)
                                .grow(1)
                                .col(|ui| {
                                    ui.row(|ui| {
                                        ui.bordered(Border::Single).pad(1).grow(1).col(|ui| {
                                            ui.text("─");
                                        });
                                        ui.bordered(Border::Double).pad(1).grow(1).col(|ui| {
                                            ui.text("═");
                                        });
                                        ui.bordered(Border::Rounded).pad(1).grow(1).col(|ui| {
                                            ui.text("╭");
                                        });
                                        ui.bordered(Border::Thick).pad(1).grow(1).col(|ui| {
                                            ui.text("━");
                                        });
                                    });
                                });
                            ui.bordered(Border::Single)
                                .title("Align")
                                .pad(1)
                                .grow(1)
                                .col(|ui| {
                                    ui.bordered(Border::Single).align(Align::Start).pad(1).col(
                                        |ui| {
                                            ui.text("Start").fg(Color::Cyan);
                                        },
                                    );
                                    ui.bordered(Border::Single).align(Align::Center).pad(1).col(
                                        |ui| {
                                            ui.text("Center").fg(Color::Yellow);
                                        },
                                    );
                                    ui.bordered(Border::Single).align(Align::End).pad(1).col(
                                        |ui| {
                                            ui.text("End").fg(Color::Green);
                                        },
                                    );
                                });
                        });

                        section(ui, "WIDGETS");
                        ui.row(|ui| {
                            ui.bordered(Border::Single)
                                .title("Input")
                                .pad(1)
                                .grow(1)
                                .col(|ui| {
                                    ui.text("Name:").bold();
                                    ui.text_input(&mut input);
                                });
                            ui.bordered(Border::Single)
                                .title("Textarea")
                                .pad(1)
                                .grow(1)
                                .col(|ui| {
                                    ui.textarea(&mut textarea, 3);
                                });
                            ui.bordered(Border::Single)
                                .title("Controls")
                                .pad(1)
                                .grow(1)
                                .col(|ui| {
                                    ui.checkbox("Dark mode", &mut dark);
                                    ui.checkbox("Notifications", &mut notif);
                                    ui.toggle("Auto-save", &mut autosave);
                                    ui.toggle("Vim mode", &mut vim);
                                });
                        });
                        ui.row(|ui| {
                            ui.bordered(Border::Single)
                                .title("Buttons")
                                .pad(1)
                                .grow(1)
                                .col(|ui| {
                                    ui.row(|ui| {
                                        if ui.button("Save") {
                                            saves += 1;
                                        }
                                        if ui.button("Reset") {
                                            saves = 0;
                                        }
                                    });
                                    ui.text(format!("Clicked: {saves}")).dim();
                                });
                        });

                        section(ui, "DATA");
                        ui.row(|ui| {
                            ui.bordered(Border::Single)
                                .title("List")
                                .pad(1)
                                .grow(1)
                                .col(|ui| {
                                    ui.list(&mut list);
                                    ui.text(format!("=> {}", list.selected_item().unwrap_or("-")))
                                        .dim();
                                });
                            ui.bordered(Border::Single)
                                .title("Table")
                                .pad(1)
                                .grow(2)
                                .col(|ui| {
                                    ui.table(&mut table);
                                });
                        });

                        section(ui, "FEEDBACK");
                        ui.row(|ui| {
                            ui.bordered(Border::Single)
                                .title("Progress")
                                .pad(1)
                                .grow(1)
                                .col(|ui| {
                                    ui.row(|ui| {
                                        ui.spinner(&spinner);
                                        ui.text(" Loading...").dim();
                                    });
                                    ui.progress(progress);
                                    ui.text(format!("{:.0}%", progress * 100.0)).dim();
                                });
                            ui.bordered(Border::Single)
                                .title("Word Wrap")
                                .pad(1)
                                .grow(1)
                                .col(|ui| {
                                    ui.text_wrap(
                                        "SLT wraps text at word boundaries automatically. \
                                 This paragraph demonstrates how long content flows \
                                 inside a constrained container, just like CSS.",
                                    )
                                    .fg(Color::Magenta);
                                });
                        });
                    });

                    ui.separator();
                    ui.help(&[
                        ("q", "quit"),
                        ("t", "theme"),
                        ("h/l", "progress"),
                        ("Tab", "focus"),
                        ("F12", "debug"),
                    ]);
                });
        },
    )
}

fn section(ui: &mut Context, title: &str) {
    ui.text(title).bold().fg(Color::Indexed(245));
}
