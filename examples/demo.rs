use slt::{
    Align, Border, Color, Context, FormField, FormState, Justify, ListState, ScrollState,
    SpinnerState, TableState, TabsState, TextInputState, TextareaState, Theme, ToastState,
};

fn main() -> std::io::Result<()> {
    let mut page_tabs = TabsState::new(vec![
        "Core Widgets",
        "Data Viz",
        "Layout",
        "Forms",
        "Feedback",
    ]);
    let mut section_tabs = TabsState::new(vec!["Primary", "Secondary", "Accent"]);
    let mut scroll = ScrollState::new();
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
    let mut progress = 0.64_f64;
    let mut dark_mode = true;
    let mut notifications = true;
    let mut autosave = false;
    let mut vim_mode = false;
    let mut saves: u32 = 0;
    let mut show_modal = false;
    let mut show_overlay = true;
    let mut toasts = ToastState::new();
    let mut form = FormState::new()
        .field(FormField::new("Email").placeholder("you@example.com"))
        .field(FormField::new("Password").placeholder("********"));

    let themes: [fn() -> Theme; 7] = [
        Theme::dark,
        Theme::light,
        Theme::dracula,
        Theme::catppuccin,
        Theme::nord,
        Theme::solarized_dark,
        Theme::tokyo_night,
    ];
    let theme_names = [
        "Dark",
        "Light",
        "Dracula",
        "Catppuccin",
        "Nord",
        "Solarized",
        "Tokyo Night",
    ];
    let mut theme_idx: usize = 0;

    slt::run_with(
        slt::RunConfig {
            mouse: true,
            ..Default::default()
        },
        |ui: &mut Context| {
            let tick = ui.tick();

            if ui.key('q') {
                ui.quit();
            }
            if ui.key('t') {
                theme_idx = (theme_idx + 1) % themes.len();
                toasts.info(format!("Theme: {}", theme_names[theme_idx]), tick);
            }
            if ui.key('h') {
                progress = (progress - 0.05).max(0.0);
            }
            if ui.key('l') {
                progress = (progress + 0.05).min(1.0);
            }
            if ui.key('m') {
                show_modal = !show_modal;
            }
            if ui.key('o') {
                show_overlay = !show_overlay;
            }

            ui.set_theme(themes[theme_idx]());

            let theme = *ui.theme();
            ui.container()
                .border(Border::Rounded)
                .pad(1)
                .grow(1)
                .col(|ui| {
                    ui.row(|ui| {
                        ui.text("SuperLightTUI").bold().fg(theme.primary);
                        ui.text(" widget showcase").fg(theme.text);
                        ui.spacer();
                        ui.text(theme_names[theme_idx]).fg(theme.text_dim);
                    });
                    ui.text("All widgets follow active theme tokens.")
                        .fg(theme.text_dim);
                    ui.separator();

                    ui.tabs(&mut page_tabs);
                    ui.separator();

                    ui.scrollable(&mut scroll)
                        .grow(1)
                        .col(|ui| match page_tabs.selected {
                            0 => render_core(
                                ui,
                                &mut section_tabs,
                                &mut input,
                                &mut textarea,
                                &mut dark_mode,
                                &mut notifications,
                                &mut autosave,
                                &mut vim_mode,
                                &mut saves,
                            ),
                            1 => render_dataviz(ui),
                            2 => render_layout(ui, &mut list, &mut table, &mut show_overlay),
                            3 => render_forms(ui, &mut form),
                            4 => render_feedback(ui, &spinner, progress),
                            _ => {}
                        });

                    ui.separator();
                    ui.help(&[
                        ("q", "quit"),
                        ("t", "next theme"),
                        ("m", "toggle modal"),
                        ("o", "toggle overlay"),
                        ("h/l", "progress -/+"),
                        ("Tab", "focus"),
                        ("F12", "debug"),
                    ]);
                });

            if show_modal {
                ui.modal(|ui| {
                    let theme = *ui.theme();
                    ui.container()
                        .bg(theme.surface)
                        .border(Border::Rounded)
                        .pad(2)
                        .col(|ui| {
                            ui.text("Modal Demo").bold().fg(theme.primary);
                            ui.text("This modal stays in the demo.")
                                .fg(theme.surface_text);
                            ui.text("Press m or click close.").fg(theme.surface_text);
                            if ui.button("Close") {
                                show_modal = false;
                            }
                        });
                });
            }

            ui.toast(&mut toasts);
        },
    )
}

fn render_core(
    ui: &mut Context,
    section_tabs: &mut TabsState,
    input: &mut TextInputState,
    textarea: &mut TextareaState,
    dark_mode: &mut bool,
    notifications: &mut bool,
    autosave: &mut bool,
    vim_mode: &mut bool,
    saves: &mut u32,
) {
    let theme = *ui.theme();
    section(ui, "CORE WIDGETS");

    card(ui, |ui| {
        ui.text("Tabs").bold().fg(theme.primary);
        ui.text("Use Left/Right when focused.")
            .fg(theme.surface_text);
        ui.tabs(section_tabs);
        ui.row(|ui| {
            ui.text("Selected:").fg(theme.surface_text);
            match section_tabs.selected {
                0 => ui.text("Primary").fg(theme.primary),
                1 => ui.text("Secondary").fg(theme.secondary),
                _ => ui.text("Accent").fg(theme.accent),
            };
        });
    });

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Input").bold().fg(theme.primary);
            ui.text("Single-line editor").fg(theme.surface_text);
            ui.text_input(input);
            ui.text("Textarea").fg(theme.surface_text);
            ui.textarea(textarea, 4);
        });

        card(ui, |ui| {
            ui.text("Controls").bold().fg(theme.secondary);
            ui.text("Theme-aware toggles").fg(theme.surface_text);
            ui.checkbox("Dark mode", dark_mode);
            ui.checkbox("Notifications", notifications);
            ui.toggle("Auto-save", autosave);
            ui.toggle("Vim mode", vim_mode);
            ui.text("Semantic colors").fg(theme.surface_text);
            ui.row(|ui| {
                ui.text("success").fg(theme.success);
                ui.text("warning").fg(theme.warning);
                ui.text("error").fg(theme.error);
            });
        });

        card(ui, |ui| {
            ui.text("Buttons").bold().fg(theme.accent);
            ui.text("Primary actions").fg(theme.surface_text);
            ui.row(|ui| {
                if ui.button("Save") {
                    *saves += 1;
                }
                if ui.button("Reset") {
                    *saves = 0;
                }
            });
            ui.row(|ui| {
                ui.text("Clicks:").fg(theme.surface_text);
                ui.text(format!("{saves}")).fg(theme.primary);
            });
        });
    });

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Typography").bold().fg(theme.primary);
            ui.text("Text styles").fg(theme.surface_text);
            ui.text("Bold").bold();
            ui.text("Italic").italic();
            ui.text("Underline").underline();
            ui.text("Strike").strikethrough();
            ui.text("Reversed").reversed();
        });

        card(ui, |ui| {
            ui.text("Color Showcase").bold().fg(theme.primary);
            ui.text("Intentional explicit palette demo")
                .fg(theme.surface_text);
            ui.text("Red").fg(Color::Red);
            ui.text("Green").fg(Color::Green);
            ui.text("Yellow").fg(Color::Yellow);
            ui.text("Blue").fg(Color::Blue);
            ui.text("Magenta").fg(Color::Magenta);
            ui.text("Cyan").fg(Color::Cyan);
        });
    });
}

fn render_dataviz(ui: &mut Context) {
    let theme = *ui.theme();
    section(ui, "DATA VIZ");

    let line_data = [
        (0.0, 1.0),
        (1.0, 3.0),
        (2.0, 2.0),
        (3.0, 5.0),
        (4.0, 4.0),
        (5.0, 6.0),
        (6.0, 3.0),
    ];
    let spark_data = [2.0, 4.0, 3.0, 6.0, 5.0, 7.0, 6.0, 8.0, 7.0, 9.0];
    let bars = [("CPU", 72.0), ("MEM", 58.0), ("IO", 36.0), ("NET", 44.0)];

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Chart").bold().fg(theme.primary);
            ui.text("Line + markers").fg(theme.surface_text);
            ui.chart(
                |c| {
                    c.xlabel("Tick");
                    c.ylabel("Value");
                    c.line(&line_data).label("trend").color(theme.primary);
                    c.scatter(&line_data).label("points").color(theme.accent);
                    c.grid(true);
                },
                36,
                10,
            );
        });

        card(ui, |ui| {
            ui.text("Sparkline + Bars").bold().fg(theme.secondary);
            ui.text("Compact signals").fg(theme.surface_text);
            ui.sparkline(&spark_data, 28);
            ui.text("Bar chart").fg(theme.surface_text);
            ui.bar_chart(&bars, 14);
        });
    });

    card(ui, |ui| {
        ui.text("Canvas").bold().fg(theme.accent);
        ui.text("Braille vector drawing").fg(theme.surface_text);
        ui.canvas(44, 8, |cv| {
            cv.line(0, 0, cv.width() - 1, cv.height() - 1);
            cv.line(cv.width() - 1, 0, 0, cv.height() - 1);
            cv.circle(cv.width() / 2, cv.height() / 2, cv.height() / 3);
        });
    });
}

fn render_layout(
    ui: &mut Context,
    list: &mut ListState,
    table: &mut TableState,
    show_overlay: &mut bool,
) {
    let theme = *ui.theme();
    section(ui, "LAYOUT & DATA");

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Grid").bold().fg(theme.primary);
            ui.text("3-column equal cells").fg(theme.surface_text);
            ui.grid(3, |ui| {
                for i in 1..=9 {
                    ui.container()
                        .bg(theme.surface_hover)
                        .border(Border::Rounded)
                        .pad(1)
                        .col(|ui| {
                            ui.text(format!("Cell {i}")).fg(theme.surface_text);
                        });
                }
            });
        });

        card(ui, |ui| {
            ui.text("List + Table").bold().fg(theme.secondary);
            ui.text("Selection widgets").fg(theme.surface_text);
            ui.list(list);
            ui.row(|ui| {
                ui.text("Current:").fg(theme.surface_text);
                ui.text(list.selected_item().unwrap_or("-"))
                    .fg(theme.primary);
            });
            ui.separator();
            ui.table(table);
        });
    });

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Align").bold().fg(theme.primary);
            ui.text("Start / Center / End").fg(theme.surface_text);
            ui.container()
                .bg(theme.surface_hover)
                .border(Border::Rounded)
                .pad(1)
                .align(Align::Start)
                .col(|ui| {
                    ui.text("Start").fg(theme.primary);
                });
            ui.container()
                .bg(theme.surface_hover)
                .border(Border::Rounded)
                .pad(1)
                .align(Align::Center)
                .col(|ui| {
                    ui.text("Center").fg(theme.secondary);
                });
            ui.container()
                .bg(theme.surface_hover)
                .border(Border::Rounded)
                .pad(1)
                .align(Align::End)
                .col(|ui| {
                    ui.text("End").fg(theme.accent);
                });
        });

        card(ui, |ui| {
            ui.text("Justify").bold().fg(theme.accent);
            ui.text("Space modes").fg(theme.surface_text);
            ui.container()
                .bg(theme.surface_hover)
                .border(Border::Rounded)
                .pad(1)
                .justify(Justify::SpaceBetween)
                .row(|ui| {
                    ui.text("A").fg(theme.primary);
                    ui.text("B").fg(theme.secondary);
                    ui.text("C").fg(theme.accent);
                });
            ui.container()
                .bg(theme.surface_hover)
                .border(Border::Rounded)
                .pad(1)
                .space_around()
                .row(|ui| {
                    ui.text("A").fg(theme.primary);
                    ui.text("B").fg(theme.secondary);
                    ui.text("C").fg(theme.accent);
                });
        });
    });

    if *show_overlay {
        ui.overlay(|ui| {
            let theme = *ui.theme();
            ui.row(|ui| {
                ui.spacer();
                ui.container()
                    .bg(theme.surface)
                    .border(Border::Rounded)
                    .pad(1)
                    .col(|ui| {
                        ui.text("Overlay Active").fg(theme.warning);
                        ui.text("Press o to toggle").fg(theme.surface_text);
                    });
            });
        });
    }
}

fn render_forms(ui: &mut Context, form: &mut FormState) {
    let theme = *ui.theme();
    section(ui, "FORMS");

    card(ui, |ui| {
        ui.text("Sign In Form").bold().fg(theme.primary);
        ui.text("Modal/form showcase retained")
            .fg(theme.surface_text);
        for field in form.fields.iter_mut() {
            ui.form_field(field);
        }
        if ui.form_submit("Sign In") {
            let _valid = form.validate(&[
                |v| {
                    if v.contains('@') {
                        Ok(())
                    } else {
                        Err("invalid email".into())
                    }
                },
                |v| {
                    if v.len() >= 6 {
                        Ok(())
                    } else {
                        Err("min 6 chars".into())
                    }
                },
            ]);
        }
    });
}

fn render_feedback(ui: &mut Context, spinner: &SpinnerState, progress: f64) {
    let theme = *ui.theme();
    section(ui, "FEEDBACK");

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Progress").bold().fg(theme.primary);
            ui.row(|ui| {
                ui.spinner(spinner);
                ui.text(" Loading...").fg(theme.surface_text);
            });
            ui.progress(progress);
            ui.text(format!("{:.0}%", progress * 100.0))
                .fg(theme.surface_text);
        });

        card(ui, |ui| {
            ui.text("Text & Links").bold().fg(theme.secondary);
            ui.text("Secondary text uses theme tokens").fg(theme.surface_text);
            ui.text_wrap(
                "SLT wraps text at word boundaries. This panel uses surface text for readability on elevated surfaces.",
            )
            .fg(theme.surface_text);
            ui.link("Docs", "https://docs.rs/superlighttui");
            ui.link("GitHub", "https://github.com/subinium/SuperLightTUI");
        });
    });
}

fn card(ui: &mut Context, f: impl FnOnce(&mut Context)) {
    let theme = *ui.theme();
    ui.container()
        .bg(theme.surface)
        .border(Border::Rounded)
        .pad(1)
        .grow(1)
        .col(f);
}

fn section(ui: &mut Context, title: &str) {
    let theme = *ui.theme();
    ui.text(title).bold().fg(theme.text_dim);
}
