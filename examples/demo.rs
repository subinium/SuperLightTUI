use slt::{
    Align, Border, BorderSides, Color, CommandPaletteState, Context, FormField, FormState, Justify,
    ListState, MultiSelectState, PaletteCommand, RadioState, ScrollState, SelectState,
    SpinnerState, TableState, TabsState, TextInputState, TextareaState, Theme, ToastState,
    TreeNode, TreeState,
};

fn main() -> std::io::Result<()> {
    let mut page_tabs = TabsState::new(vec![
        "Core Widgets",
        "Data Viz",
        "Layout",
        "Forms",
        "Feedback",
        "Advanced",
    ]);
    let mut section_tabs = TabsState::new(vec!["Primary", "Secondary", "Accent"]);
    let mut scroll = ScrollState::new();
    let mut input = TextInputState::with_placeholder("Type here...");
    let mut textarea = TextareaState::new();
    let mut list = ListState::new(vec!["Rust", "Go", "Python", "TypeScript", "Zig", "C++"]);
    let mut table = TableState::new(
        vec!["Name", "Lang", "Stars"],
        vec![
            vec!["SLT", "Rust", "500"],
            vec!["Ratatui", "Rust", "12000"],
            vec!["Bubbletea", "Go", "30000"],
            vec!["Ink", "JS/TS", "8000"],
            vec!["Textual", "Python", "26000"],
            vec!["Cursive", "Rust", "4200"],
        ],
    );
    table.page_size = 3;
    let mut table_filter = TextInputState::with_placeholder("Filter table...");
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
    let mut select = SelectState::new(vec!["Rounded", "Single", "Double", "Thick"]);
    let mut radio = RadioState::new(vec!["Dark", "Light", "System"]);
    let mut multi = MultiSelectState::new(vec![
        "Vim motions",
        "Mouse support",
        "Clipboard",
        "Unicode",
        "Async",
    ]);
    let mut tree = TreeState::new(vec![
        TreeNode::new("src").expanded().children(vec![
            TreeNode::new("lib.rs"),
            TreeNode::new("context.rs"),
            TreeNode::new("layout.rs"),
            TreeNode::new("style.rs"),
            TreeNode::new("widgets.rs"),
        ]),
        TreeNode::new("examples")
            .children(vec![TreeNode::new("demo.rs"), TreeNode::new("counter.rs")]),
        TreeNode::new("tests").children(vec![
            TreeNode::new("widgets.rs"),
            TreeNode::new("snapshots.rs"),
        ]),
    ]);
    let mut vlist = ListState::new((0..100).map(|i| format!("Item {i}")).collect());
    let mut password = TextInputState::with_placeholder("Password");
    password.masked = true;
    let mut palette = CommandPaletteState::new(vec![
        PaletteCommand::new("Switch Theme", "Cycle to next theme"),
        PaletteCommand::new("Toggle Modal", "Show/hide modal dialog"),
        PaletteCommand::new("Toggle Overlay", "Show/hide overlay"),
        PaletteCommand::new("Quit", "Exit the application"),
    ]);

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
            if ui.key_mod('p', slt::KeyModifiers::CONTROL) {
                palette.open = !palette.open;
            }
            if ui.key_seq("gg") {
                scroll.offset = 0;
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
                            2 => render_layout(
                                ui,
                                &mut list,
                                &mut table,
                                &mut table_filter,
                                &mut show_overlay,
                            ),
                            3 => render_forms(ui, &mut form, &mut password),
                            4 => render_feedback(ui, &spinner, progress),
                            5 => render_advanced(
                                ui,
                                &mut select,
                                &mut radio,
                                &mut multi,
                                &mut tree,
                                &mut vlist,
                            ),
                            _ => {}
                        });

                    ui.separator();
                    ui.help(&[
                        ("q", "quit"),
                        ("t", "next theme"),
                        ("m", "toggle modal"),
                        ("o", "toggle overlay"),
                        ("h/l", "progress -/+"),
                        ("Ctrl+P", "palette"),
                        ("gg", "top"),
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

            if let Some(idx) = ui.command_palette(&mut palette) {
                match idx {
                    0 => {
                        theme_idx = (theme_idx + 1) % themes.len();
                        toasts.info(format!("Theme: {}", theme_names[theme_idx]), tick);
                    }
                    1 => show_modal = !show_modal,
                    2 => show_overlay = !show_overlay,
                    3 => ui.quit(),
                    _ => {}
                }
            }
        },
    )
}

#[allow(clippy::too_many_arguments)]
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
    table_filter: &mut TextInputState,
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
            ui.text("Sort: click header · Filter + Pagination").dim();
            ui.text_input(table_filter);
            table.set_filter(&table_filter.value);
            ui.table(table);
            if let Some(row) = table.selected_row() {
                ui.row(|ui| {
                    ui.text("Selected:").fg(theme.surface_text);
                    ui.text(row.join(", ")).fg(theme.primary);
                });
            }
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

fn render_forms(ui: &mut Context, form: &mut FormState, password: &mut TextInputState) {
    let theme = *ui.theme();
    section(ui, "FORMS");

    ui.row(|ui| {
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

        card(ui, |ui| {
            ui.text("Password Input").bold().fg(theme.secondary);
            ui.text("Masked text input widget").fg(theme.surface_text);
            ui.text_input(password);
            ui.row(|ui| {
                ui.text("Length:").fg(theme.surface_text);
                ui.text(format!("{}", password.value.len()))
                    .fg(theme.primary);
            });
        });
    });
}

fn render_advanced(
    ui: &mut Context,
    select: &mut SelectState,
    radio: &mut RadioState,
    multi: &mut MultiSelectState,
    tree: &mut TreeState,
    vlist: &mut ListState,
) {
    let theme = *ui.theme();
    section(ui, "ADVANCED");

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Select").bold().fg(theme.primary);
            ui.text("Dropdown style preset").fg(theme.surface_text);
            let _changed = ui.select(select);
            ui.row(|ui| {
                ui.text("Current:").fg(theme.surface_text);
                ui.text(&select.items[select.selected]).fg(theme.primary);
            });
        });

        card(ui, |ui| {
            ui.text("Radio").bold().fg(theme.secondary);
            ui.text("Theme preference").fg(theme.surface_text);
            let _changed = ui.radio(radio);
            ui.row(|ui| {
                ui.text("Mode:").fg(theme.surface_text);
                ui.text(&radio.items[radio.selected]).fg(theme.secondary);
            });
        });

        card(ui, |ui| {
            ui.text("Multi Select").bold().fg(theme.accent);
            ui.text("Feature toggles").fg(theme.surface_text);
            ui.multi_select(multi);
            ui.row(|ui| {
                ui.text("Enabled:").fg(theme.surface_text);
                ui.text(format!("{}", multi.selected.len()))
                    .fg(theme.accent);
            });
        });
    });

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Tree").bold().fg(theme.primary);
            ui.text("Project structure").fg(theme.surface_text);
            ui.tree(tree);
        });

        card(ui, |ui| {
            ui.text("Virtual List").bold().fg(theme.secondary);
            ui.text("100 items, 8 visible").fg(theme.surface_text);
            ui.virtual_list(vlist, 8, |ui, idx| {
                ui.text(format!("Item {idx}")).fg(theme.surface_text);
            });
        });
    });

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Markdown").bold().fg(theme.primary);
            ui.markdown(
                "# v0.6.0\n\n**New widgets**: select, radio, multi-select, tree, virtual list.\n\n- Command palette (`Ctrl+P`)\n- Key sequence demo (`gg`)\n\n---\n\n`Theme-aware` and production-ready.",
            );
        });

        card(ui, |ui| {
            ui.text("Rich Text").bold().fg(theme.secondary);
            ui.text("line() and line_wrap()").fg(theme.surface_text);

            ui.line(|ui| {
                ui.text("Status: ");
                ui.text("Online").bold().fg(Color::Green);
                ui.text(" · ");
                ui.text("3 tasks").fg(theme.accent);
            });

            ui.line(|ui| {
                ui.text("Error: ").fg(Color::Red);
                ui.text("file ").fg(theme.surface_text);
                ui.text("config.toml").bold().fg(theme.primary);
                ui.text(" not found").fg(theme.surface_text);
            });

            ui.container()
                .bg(theme.surface_hover)
                .border(Border::Rounded)
                .pad(1)
                .col(|ui| {
                    ui.text("line_wrap()").bold().fg(theme.accent);
                    ui.line_wrap(|ui| {
                        ui.text("This ");
                        ui.text("wraps ").bold();
                        ui.text("across lines while keeping ");
                        ui.text("styles").fg(Color::Cyan).bold();
                        ui.text(" on each segment.");
                    });
                });
        });
    });

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Borders + Percent Sizing").bold().fg(theme.accent);
            ui.text("Per-side borders and 30/70 layout").fg(theme.surface_text);

            ui.container()
                .bg(theme.surface_hover)
                .border(Border::Single)
                .border_sides(BorderSides::horizontal())
                .pad(1)
                .col(|ui| {
                    ui.text("Horizontal borders").fg(theme.surface_text);
                });

            ui.container()
                .bg(theme.surface_hover)
                .border(Border::Single)
                .border_sides(BorderSides::vertical())
                .pad(1)
                .col(|ui| {
                    ui.text("Vertical borders").fg(theme.surface_text);
                });

            ui.row(|ui| {
                ui.container()
                    .bg(theme.surface_hover)
                    .border(Border::Rounded)
                    .w_pct(30)
                    .pad(1)
                    .col(|ui| {
                        ui.text("30%").fg(theme.primary);
                    });
                ui.container()
                    .bg(theme.surface_hover)
                    .border(Border::Rounded)
                    .w_pct(70)
                    .pad(1)
                    .col(|ui| {
                        ui.text("70%").fg(theme.secondary);
                    });
            });
        });

        card(ui, |ui| {
            ui.text("Markdown Inline Styles").bold().fg(theme.primary);
            ui.text("**bold**, *italic*, `code` now styled").fg(theme.surface_text);
            ui.markdown(
                "Inline: **bold text** and *italic text* and `code blocks` all render with proper styling.\n\n- List with **bold** items\n- And `inline code` too",
            );
        });
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
