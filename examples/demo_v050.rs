use slt::{
    anim::{ease_in_out_cubic, ease_out_quad},
    Border, ButtonVariant, Color, Context, FormField, FormState, Justify, Keyframes, ListState,
    LoopMode, ScrollState, Sequence, Stagger, Style, TableState, TabsState, TextInputState, Theme,
    ToastState,
};

fn main() -> std::io::Result<()> {
    let mut tabs = TabsState::new(vec![
        "Themes",
        "Buttons",
        "Containers",
        "Justify",
        "Links",
        "Validation",
        "Animation",
        "Modal",
        "Form",
    ]);
    let mut scroll = ScrollState::new();

    let mut email = TextInputState::with_placeholder("you@example.com");

    let mut keyframes = Keyframes::new(120)
        .stop(0.0, 0.0)
        .stop(0.3, 100.0)
        .stop(0.7, 30.0)
        .stop(1.0, 80.0)
        .loop_mode(LoopMode::PingPong);
    let mut sequence = Sequence::new()
        .then(0.0, 50.0, 30, ease_out_quad)
        .then(50.0, 100.0, 30, ease_in_out_cubic)
        .loop_mode(LoopMode::Repeat);
    let mut stagger = Stagger::new(0.0, 1.0, 40)
        .delay(8)
        .easing(ease_out_quad)
        .items(5)
        .loop_mode(slt::LoopMode::Repeat);

    let start_tick = 0;
    keyframes.reset(start_tick);
    sequence.reset(start_tick);
    stagger.reset(start_tick);

    let mut show_modal = false;
    let mut toasts = ToastState::new();
    let mut form = FormState::new()
        .field(FormField::new("Username").placeholder("john_doe"))
        .field(FormField::new("Email").placeholder("john@example.com"))
        .field(FormField::new("Password").placeholder("********"));
    let mut theme_idx: usize = 0;
    let theme_names = [
        "Dark",
        "Light",
        "Dracula",
        "Catppuccin",
        "Nord",
        "Solarized Dark",
        "Tokyo Night",
    ];
    let mut list = ListState::new(vec!["Alpha", "Beta", "Gamma", "Delta", "Epsilon"]);
    let mut table = TableState::new(
        vec!["Name", "Language", "Stars"],
        vec![
            vec!["SLT", "Rust", "★★★★★"],
            vec!["ratatui", "Rust", "★★★★"],
            vec!["blessed", "JS", "★★★"],
            vec!["tui-rs", "Rust", "★★★"],
        ],
    );

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
                theme_idx = (theme_idx + 1) % theme_names.len();
            }
            if ui.key('m') {
                show_modal = !show_modal;
            }
            if ui.key('r') {
                let tick = ui.tick();
                keyframes.reset(tick);
                sequence.reset(tick);
                stagger.reset(tick);
            }
            let mut theme_key_used = false;
            if tabs.selected == 0 {
                for (idx, key) in ['1', '2', '3', '4', '5', '6', '7'].iter().enumerate() {
                    if ui.key(*key) {
                        theme_idx = idx;
                        theme_key_used = true;
                    }
                }
            }

            for (idx, key) in ['1', '2', '3', '4', '5', '6', '7', '8', '9']
                .iter()
                .enumerate()
            {
                if tabs.selected == 0 && theme_key_used && idx < 7 {
                    continue;
                }
                if ui.key(*key) {
                    tabs.selected = idx;
                    scroll = ScrollState::new();
                }
            }

            let theme = match theme_idx {
                0 => Theme::dark(),
                1 => Theme::light(),
                2 => Theme::dracula(),
                3 => Theme::catppuccin(),
                4 => Theme::nord(),
                5 => Theme::solarized_dark(),
                6 => Theme::tokyo_night(),
                _ => Theme::dark(),
            };
            ui.set_theme(theme);

            ui.bordered(Border::Rounded)
                .title("SLT v0.5.0 Features")
                .pad(1)
                .grow(1)
                .col(|ui| {
                    ui.tabs(&mut tabs);
                    ui.separator();

                    ui.scrollable(&mut scroll)
                        .grow(1)
                        .col(|ui| match tabs.selected {
                            0 => render_themes(ui, &mut theme_idx, &theme_names),
                            1 => render_buttons(ui),
                            2 => render_containers(ui, &mut list, &mut table),
                            3 => render_justify(ui),
                            4 => render_links(ui),
                            5 => render_validation(ui, &mut email),
                            6 => render_animation(ui, &mut keyframes, &mut sequence, &mut stagger),
                            7 => {
                                render_modal(ui, &mut show_modal, &mut toasts);
                            }
                            8 => render_form(ui, &mut form, &mut toasts),
                            _ => {}
                        });

                    ui.separator();
                    ui.help(&[
                        ("q", "quit"),
                        ("t", "next theme"),
                        ("1-9", "tabs"),
                        ("1-7", "theme keys (Themes tab)"),
                        ("m", "modal"),
                        ("r", "restart animation"),
                        ("Tab", "focus"),
                    ]);
                });

            ui.toast(&mut toasts);
        },
    )
}

fn render_themes(ui: &mut Context, theme_idx: &mut usize, theme_names: &[&str]) {
    ui.text("Theme Palettes").bold().fg(Color::Cyan);
    ui.text("Click or press 1-7 to switch themes").dim();
    ui.text("");

    ui.row(|ui| {
        for (i, name) in theme_names.iter().enumerate() {
            if i == *theme_idx {
                ui.styled(format!(" ● {name} "), Style::new().fg(Color::Cyan).bold());
            } else if ui.button(*name) {
                *theme_idx = i;
            }
        }
    });
    ui.text("");

    let theme = match *theme_idx {
        0 => Theme::dark(),
        1 => Theme::light(),
        2 => Theme::dracula(),
        3 => Theme::catppuccin(),
        4 => Theme::nord(),
        5 => Theme::solarized_dark(),
        6 => Theme::tokyo_night(),
        _ => Theme::dark(),
    };

    ui.bordered(Border::Rounded)
        .title("Color Preview")
        .pad(1)
        .col(|ui| {
            ui.row(|ui| {
                ui.text("primary").fg(theme.primary);
                ui.text("secondary").fg(theme.secondary);
                ui.text("accent").fg(theme.accent);
            });
            ui.row(|ui| {
                ui.text("success").fg(theme.success);
                ui.text("warning").fg(theme.warning);
                ui.text("error").fg(theme.error);
            });
            ui.row(|ui| {
                ui.text("text").fg(theme.text);
                ui.text("text_dim").fg(theme.text_dim);
                ui.text("border").fg(theme.border);
            });
            ui.row(|ui| {
                ui.styled(
                    " selected ",
                    Style::new().bg(theme.selected_bg).fg(theme.selected_fg),
                );
                ui.styled(" surface ", Style::new().bg(theme.surface).fg(theme.text));
            });
        });

    ui.text("");
    ui.bordered(Border::Rounded)
        .title("Widget Preview")
        .pad(1)
        .col(|ui| {
            ui.text(format!("Active: {}", theme_names[*theme_idx]))
                .bold();
            ui.progress(0.65);
            ui.row(|ui| {
                ui.button("Default");
                ui.button_with("Primary", ButtonVariant::Primary);
                ui.button_with("Danger", ButtonVariant::Danger);
                ui.button_with("Outline", ButtonVariant::Outline);
            });
        });
}

fn render_buttons(ui: &mut Context) {
    ui.text("Button Variants").bold().fg(Color::Cyan);
    ui.text("").dim();

    ui.bordered(Border::Rounded)
        .title("Default")
        .pad(1)
        .col(|ui| {
            ui.row(|ui| {
                if ui.button("Click Me") {}
                if ui.button("Another") {}
            });
            ui.text("Standard bracket style: [ label ]").dim();
        });

    ui.bordered(Border::Rounded)
        .title("Primary")
        .pad(1)
        .col(|ui| {
            ui.row(|ui| {
                if ui.button_with("Save", ButtonVariant::Primary) {}
                if ui.button_with("Submit", ButtonVariant::Primary) {}
                if ui.button_with("Confirm", ButtonVariant::Primary) {}
            });
            ui.text("Filled with primary bg for call-to-action").dim();
        });

    ui.bordered(Border::Rounded)
        .title("Danger")
        .pad(1)
        .col(|ui| {
            ui.row(|ui| {
                if ui.button_with("Delete", ButtonVariant::Danger) {}
                if ui.button_with("Remove", ButtonVariant::Danger) {}
                if ui.button_with("Reset", ButtonVariant::Danger) {}
            });
            ui.text("Red/error color for destructive actions").dim();
        });

    ui.bordered(Border::Rounded)
        .title("Outline")
        .pad(1)
        .col(|ui| {
            ui.row(|ui| {
                if ui.button_with("Cancel", ButtonVariant::Outline) {}
                if ui.button_with("Skip", ButtonVariant::Outline) {}
                if ui.button_with("Later", ButtonVariant::Outline) {}
            });
            ui.text("Bordered style for secondary actions").dim();
        });

    ui.text("");
    ui.text("Hover over buttons to see bg highlight effect")
        .dim();
}

fn render_containers(ui: &mut Context, list: &mut ListState, table: &mut TableState) {
    ui.text("Container Background & Click Support")
        .bold()
        .fg(Color::Cyan);
    ui.text("");

    ui.text("Container .bg() color:").bold();
    ui.row(|ui| {
        ui.container()
            .bg(Color::Rgb(40, 42, 54))
            .p(1)
            .grow(1)
            .col(|ui| {
                ui.text("Dark bg").fg(Color::White);
            });
        ui.container()
            .bg(Color::Rgb(30, 30, 46))
            .p(1)
            .grow(1)
            .col(|ui| {
                ui.text("Catppuccin bg").fg(Color::Rgb(205, 214, 244));
            });
        ui.container()
            .bg(Color::Rgb(46, 52, 64))
            .p(1)
            .grow(1)
            .col(|ui| {
                ui.text("Nord bg").fg(Color::Rgb(236, 239, 244));
            });
    });

    ui.text("");
    ui.text("List (click to select):").bold();
    ui.bordered(Border::Rounded).pad(1).col(|ui| {
        ui.list(list);
    });
    let selected_item = list
        .items
        .get(list.selected)
        .map(|s| s.as_str())
        .unwrap_or("");
    ui.text(format!("Selected: {selected_item}")).dim();

    ui.text("");
    ui.text("Table (click row to select):").bold();
    ui.bordered(Border::Rounded).pad(1).col(|ui| {
        ui.table(table);
    });
    if let Some(row) = table.rows.get(table.selected) {
        ui.text(format!("Selected: {}", row.join(" | "))).dim();
    }
}

fn render_justify(ui: &mut Context) {
    ui.text("Container justify variants").bold().fg(Color::Cyan);

    ui.bordered(Border::Single)
        .title("SpaceBetween")
        .space_between()
        .pad(1)
        .row(|ui| {
            ui.text("A");
            ui.text("B");
            ui.text("C");
        });

    ui.bordered(Border::Single)
        .title("SpaceAround")
        .space_around()
        .pad(1)
        .row(|ui| {
            ui.text("A");
            ui.text("B");
            ui.text("C");
        });

    ui.bordered(Border::Single)
        .title("SpaceEvenly")
        .space_evenly()
        .pad(1)
        .row(|ui| {
            ui.text("A");
            ui.text("B");
            ui.text("C");
        });

    ui.row(|ui| {
        ui.bordered(Border::Single)
            .title("Justify::Center")
            .justify(Justify::Center)
            .pad(1)
            .grow(1)
            .row(|ui| {
                ui.text("A");
                ui.text("B");
                ui.text("C");
            });
        ui.bordered(Border::Single)
            .title("Justify::End")
            .justify(Justify::End)
            .pad(1)
            .grow(1)
            .row(|ui| {
                ui.text("A");
                ui.text("B");
                ui.text("C");
            });
    });
}

fn render_links(ui: &mut Context) {
    ui.text("OSC 8 hyperlinks").bold().fg(Color::Cyan);
    ui.link("SLT Documentation", "https://docs.rs/superlighttui");
    ui.link(
        "GitHub Repository",
        "https://github.com/subinium/SuperLightTUI",
    );
    ui.link("Bold Link", "https://example.com").bold();
    ui.text("Open links with Ctrl/Cmd+click in supporting terminals.")
        .dim();
}

fn render_validation(ui: &mut Context, email: &mut TextInputState) {
    ui.text("Live input validation").bold().fg(Color::Cyan);
    ui.text("Email:").dim();
    ui.text_input(email);
    email.validate(|v| {
        if v.is_empty() {
            return Ok(());
        }
        if v.contains('@') && v.contains('.') {
            Ok(())
        } else {
            Err("Enter a valid email".into())
        }
    });
}

fn render_animation(
    ui: &mut Context,
    keyframes: &mut Keyframes,
    sequence: &mut Sequence,
    stagger: &mut Stagger,
) {
    let tick = ui.tick();
    let kf_val = keyframes.value(tick) / 100.0;
    let seq_val = sequence.value(tick) / 100.0;

    ui.text("Keyframes / Sequence / Stagger")
        .bold()
        .fg(Color::Cyan);
    ui.bordered(Border::Single)
        .title("Keyframes (PingPong)")
        .pad(1)
        .col(|ui| {
            ui.progress(kf_val.clamp(0.0, 1.0));
            ui.text(format!("value: {:>5.1}", kf_val * 100.0)).dim();
        });

    ui.bordered(Border::Single)
        .title("Sequence (Repeat)")
        .pad(1)
        .col(|ui| {
            ui.progress(seq_val.clamp(0.0, 1.0));
            ui.text(format!("value: {:>5.1}", seq_val * 100.0)).dim();
        });

    ui.bordered(Border::Single)
        .title("Stagger (5 bars)")
        .pad(1)
        .col(|ui| {
            for i in 0..5 {
                let val = stagger.value(tick, i).clamp(0.0, 1.0);
                ui.progress(val);
            }
        });
}

fn render_modal(ui: &mut Context, show_modal: &mut bool, toasts: &mut ToastState) {
    ui.text("Overlay and modal APIs").bold().fg(Color::Cyan);
    if ui.button("Show Modal") {
        *show_modal = true;
    }

    ui.overlay(|ui| {
        ui.row(|ui| {
            ui.spacer();
            ui.text("Status: Online").fg(Color::Green);
        });
    });

    if *show_modal {
        ui.modal(|ui| {
            ui.bordered(Border::Rounded).pad(2).col(|ui| {
                ui.text("Confirm Action").bold();
                ui.text("Are you sure you want to proceed?");
                ui.row(|ui| {
                    if ui.button("Yes") {
                        *show_modal = false;
                        toasts.success("Action confirmed", ui.tick());
                    }
                    if ui.button("No") {
                        *show_modal = false;
                        toasts.info("Action cancelled", ui.tick());
                    }
                });
            });
        });
    }
}

fn render_form(ui: &mut Context, form: &mut FormState, toasts: &mut ToastState) {
    ui.text("Form widget and validators").bold().fg(Color::Cyan);
    ui.bordered(Border::Single)
        .title("Create Account")
        .pad(1)
        .col(|ui| {
            ui.form(form, |ui, form| {
                for field in form.fields.iter_mut() {
                    ui.form_field(field);
                }
                if ui.form_submit("Create Account") {
                    let valid = form.validate(&[
                        |v| {
                            if v.len() >= 3 {
                                Ok(())
                            } else {
                                Err("min 3 chars".into())
                            }
                        },
                        |v| {
                            if v.contains('@') {
                                Ok(())
                            } else {
                                Err("invalid email".into())
                            }
                        },
                        |v| {
                            if v.len() >= 8 {
                                Ok(())
                            } else {
                                Err("min 8 chars".into())
                            }
                        },
                    ]);
                    if valid {
                        form.submitted = true;
                        toasts.success("Account created", ui.tick());
                    } else {
                        form.submitted = false;
                        toasts.error("Please fix validation errors", ui.tick());
                    }
                }
                if form.submitted {
                    ui.text("Submitted successfully").fg(Color::Green);
                }
            });
        });
}
