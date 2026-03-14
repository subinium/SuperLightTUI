use slt::{
    Align, ApprovalAction, Border, BorderSides, Breakpoint, Color, CommandPaletteState, Context,
    ContextItem, FormField, FormState, HalfBlockImage, Justify, KeyCode, ListState,
    MultiSelectState, PaletteCommand, RadioState, RunConfig, ScrollState, SelectState,
    SpinnerState, StreamingTextState, TableState, TabsState, TextInputState, TextareaState, Theme,
    ToastState, ToolApprovalState, TreeNode, TreeState,
};

fn main() -> std::io::Result<()> {
    let mut page_tabs = TabsState::new(vec![
        "Core Widgets",
        "Data Viz",
        "Layout",
        "Forms",
        "Feedback",
        "Advanced",
        "v0.7.0",
        "v0.8.0",
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
    let mut v7_scroll = ScrollState::new();
    let mut v7_stream = StreamingTextState::new();
    let mut v7_tool = ToolApprovalState::new("read_file", "Read contents of config.toml");
    let mut v7_stream_tick: u64 = 0;
    let mut list_with_filter = ListState::new(vec![
        "Rust",
        "Go",
        "Python",
        "TypeScript",
        "JavaScript",
        "C++",
        "Zig",
        "Haskell",
    ]);
    let mut list_filter_input = TextInputState::with_placeholder("Filter list...");
    let mut v8_dark_mode = false;
    let mut v8_tween = slt::anim::Tween::new(0.0, 100.0, 120);
    let mut v8_anim_done = false;

    slt::run_with(
        RunConfig {
            mouse: true,
            kitty_keyboard: true,
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
            for i in 1..=8u8 {
                if ui.key((b'0' + i) as char) {
                    page_tabs.selected = (i - 1) as usize;
                }
            }

            ui.set_theme(themes[theme_idx]());
            ui.set_dark_mode(v8_dark_mode);

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
                            6 => render_v070(
                                ui,
                                &mut v7_scroll,
                                &mut v7_stream,
                                &mut v7_tool,
                                &mut v7_stream_tick,
                            ),
                            7 => render_v080(
                                ui,
                                &mut list_with_filter,
                                &mut list_filter_input,
                                &mut v8_dark_mode,
                                &mut v8_tween,
                                &mut v8_anim_done,
                                tick,
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
                        ("1-8", "tab"),
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
                "# v0.7.0\n\n**9 new features**: dashed borders, Kitty keyboard, color downsampling, scrollbar, breakpoints, clipboard, DevTools, half-block image, AI widgets.\n\n- Check the **v0.7.0** tab →\n- Press **F12** for DevTools\n\n---\n\n`Theme-aware` and production-ready.",
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

fn render_v070(
    ui: &mut Context,
    scroll: &mut ScrollState,
    stream: &mut StreamingTextState,
    tool: &mut ToolApprovalState,
    stream_tick: &mut u64,
) {
    let theme = *ui.theme();
    let tick = ui.tick();
    section(ui, "v0.7.0 FEATURES");

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Dashed Borders").bold().fg(theme.primary);
            ui.text("2 new border variants").fg(theme.surface_text);

            ui.container().border(Border::Dashed).pad(1).col(|ui| {
                ui.text("Border::Dashed").fg(theme.text);
            });
            ui.container().border(Border::DashedThick).pad(1).col(|ui| {
                ui.text("Border::DashedThick").fg(theme.text);
            });
        });

        card(ui, |ui| {
            ui.text("All 6 Border Styles").bold().fg(theme.secondary);
            let borders = [
                ("Single", Border::Single),
                ("Double", Border::Double),
                ("Rounded", Border::Rounded),
                ("Thick", Border::Thick),
                ("Dashed", Border::Dashed),
                ("DashedThick", Border::DashedThick),
            ];
            for (name, border) in borders {
                ui.container().border(border).col(|ui| {
                    ui.text(name).fg(theme.surface_text);
                });
            }
        });
    });

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Scrollbar").bold().fg(theme.primary);
            let focused = ui.register_focusable();
            if focused {
                ui.text("⇅ Use ↑↓ to scroll").fg(theme.secondary);
                if ui.key_code(KeyCode::Up) {
                    scroll.scroll_up(1);
                }
                if ui.key_code(KeyCode::Down) {
                    scroll.scroll_down(1);
                }
            } else {
                ui.text("Tab to focus, then ↑↓").fg(theme.text_dim);
            }
            ui.row(|ui| {
                ui.scrollable(scroll).grow(1).h(8).col(|ui| {
                    for i in 0..30 {
                        let fg = if focused && i == scroll.offset {
                            theme.primary
                        } else if i % 2 == 0 {
                            theme.text
                        } else {
                            theme.text_dim
                        };
                        ui.text(format!("  Line {i}")).fg(fg);
                    }
                });
                ui.scrollbar(scroll);
            });
        });

        card(ui, |ui| {
            ui.text("Responsive Breakpoint").bold().fg(theme.secondary);
            ui.text("Adapts to terminal width").fg(theme.surface_text);
            let bp = ui.breakpoint();
            let (label, color) = match bp {
                Breakpoint::Xs => ("Xs (<40)", theme.error),
                Breakpoint::Sm => ("Sm (40-79)", theme.warning),
                Breakpoint::Md => ("Md (80-119)", theme.secondary),
                Breakpoint::Lg => ("Lg (120-159)", theme.success),
                Breakpoint::Xl => ("Xl (160+)", theme.primary),
            };
            ui.row(|ui| {
                ui.text("Current: ").fg(theme.surface_text);
                ui.text(label).bold().fg(color);
            });
            ui.text(format!("Terminal: {}×{}", ui.width(), ui.height()))
                .dim();
            ui.text("Resize terminal to see changes").fg(theme.text_dim);
        });
    });

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("StreamingText").bold().fg(theme.primary);
            ui.text("AI response with cursor").fg(theme.surface_text);

            let full_text = "Hello! I'm an AI assistant powered by SLT. This text streams in character by character, just like a real LLM response.";

            if !stream.streaming && stream.content.is_empty() {
                stream.start();
                *stream_tick = tick;
            }
            if stream.streaming {
                let elapsed = tick.saturating_sub(*stream_tick);
                let chars_to_show = (elapsed / 4) as usize;
                stream.content = full_text.chars().take(chars_to_show).collect();
                if chars_to_show >= full_text.chars().count() {
                    stream.finish();
                }
            }
            ui.streaming_text(stream);

            if !stream.streaming && !stream.content.is_empty() && ui.button("↻ Replay") {
                stream.content.clear();
            }
        });

        card(ui, |ui| {
            ui.text("ToolApproval").bold().fg(theme.secondary);
            ui.text("Human-in-the-loop gate").fg(theme.surface_text);
            ui.tool_approval(tool);
            if tool.action != ApprovalAction::Pending && ui.button("Reset") {
                tool.reset();
            }
        });
    });

    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("ContextBar").bold().fg(theme.primary);
            ui.text("Token usage indicator").fg(theme.surface_text);
            let items = vec![
                ContextItem::new("main.rs", 1200),
                ContextItem::new("lib.rs", 3400),
                ContextItem::new("README.md", 800),
            ];
            ui.context_bar(&items);
        });

        card(ui, |ui| {
            ui.text("Half-Block Image").bold().fg(theme.secondary);
            ui.text("2x vertical resolution").fg(theme.surface_text);

            let w: u32 = 24;
            let h: u32 = 6;
            let pixel_h = h * 2;
            let mut rgb = Vec::with_capacity((w * pixel_h * 3) as usize);
            for y in 0..pixel_h {
                for x in 0..w {
                    let r = (x as f32 / w as f32 * 255.0) as u8;
                    let g = (y as f32 / pixel_h as f32 * 255.0) as u8;
                    let b = 128;
                    rgb.push(r);
                    rgb.push(g);
                    rgb.push(b);
                }
            }
            let img = HalfBlockImage::from_rgb(&rgb, w, h);
            ui.image(&img);
        });
    });

    card(ui, |ui| {
        ui.text("More v0.7.0").bold().fg(theme.accent);
        ui.row(|ui| {
            ui.container().grow(1).col(|ui| {
                ui.text("Color Downsampling").bold().fg(theme.primary);
                ui.text("RGB → 256 → 16 color").fg(theme.surface_text);
                let colors = [
                    ("Coral", Color::Rgb(255, 127, 80)),
                    ("Teal", Color::Rgb(0, 128, 128)),
                    ("Gold", Color::Rgb(255, 215, 0)),
                    ("Violet", Color::Rgb(138, 43, 226)),
                ];
                for (name, c) in colors {
                    ui.line(|ui| {
                        ui.text(format!("{name}: ")).fg(theme.surface_text);
                        ui.text("████").fg(c);
                        ui.text(" → ").dim();
                        ui.text("████").fg(c.downsampled(slt::ColorDepth::EightBit));
                        ui.text(" → ").dim();
                        ui.text("████").fg(c.downsampled(slt::ColorDepth::Basic));
                    });
                }
            });
            ui.container().grow(1).col(|ui| {
                ui.text("Kitty Keyboard").bold().fg(theme.secondary);
                ui.text("Key release events enabled").fg(theme.surface_text);
                ui.text("kitty_keyboard: true").fg(theme.secondary);
                ui.separator();
                ui.text("OSC 52 Clipboard").bold().fg(theme.accent);
                ui.text("copy_to_clipboard()").fg(theme.surface_text);
                if ui.button("Copy 'SLT v0.7.0'") {
                    ui.copy_to_clipboard("SLT v0.7.0");
                }
                ui.separator();
                ui.text("DevTools: Press F12").bold().fg(theme.warning);
            });
        });
    });
}

fn render_v080(
    ui: &mut Context,
    list_with_filter: &mut ListState,
    list_filter_input: &mut TextInputState,
    v8_dark_mode: &mut bool,
    v8_tween: &mut slt::anim::Tween,
    v8_anim_done: &mut bool,
    tick: u64,
) {
    let theme = *ui.theme();
    section(ui, "v0.8.0 FEATURES");

    section(ui, "DARK MODE");
    card(ui, |ui| {
        ui.row_gap(2, |ui| {
            ui.container()
                .bg(Color::Rgb(240, 240, 240))
                .dark_bg(Color::Rgb(30, 30, 46))
                .p(1)
                .col(|ui| {
                    ui.text("This background changes with dark/light mode");
                });
            if ui.button("Toggle Dark") {
                *v8_dark_mode = !*v8_dark_mode;
            }
            ui.text(if *v8_dark_mode {
                "Mode: Dark"
            } else {
                "Mode: Light"
            })
            .dim();
        });
    });

    section(ui, "RESPONSIVE LAYOUT");
    card(ui, |ui| {
        ui.text(format!("Breakpoint: {:?}", ui.breakpoint())).dim();
        ui.row_gap(1, |ui| {
            ui.container()
                .w(20)
                .md_w(30)
                .lg_w(40)
                .border(Border::Rounded)
                .p(1)
                .col(|ui| {
                    ui.text("Responsive width");
                });
            ui.container()
                .grow(1)
                .border(Border::Single)
                .p(1)
                .col(|ui| {
                    ui.text("Grows to fill");
                });
        });
    });

    section(ui, "LIST FILTER");
    card(ui, |ui| {
        ui.text("Type to filter (multi-token AND: 'ty script' matches TypeScript)")
            .dim();
        ui.text_input(list_filter_input);
        if list_filter_input.value != list_with_filter.filter {
            list_with_filter.set_filter(&list_filter_input.value);
        }
        ui.list(list_with_filter);
        ui.text(format!(
            "{}/{} items shown",
            list_with_filter.visible_indices().len(),
            8
        ))
        .dim();
    });

    section(ui, "THEME BUILDER");
    card(ui, |ui| {
        ui.text("Custom themes from Theme::builder()").dim();
        let custom = slt::Theme::builder()
            .primary(Color::Rgb(255, 107, 107))
            .secondary(Color::Rgb(78, 205, 196))
            .accent(Color::Rgb(255, 230, 109))
            .build();
        ui.row_gap(1, |ui| {
            ui.text("■ Primary").fg(custom.primary);
            ui.text("■ Secondary").fg(custom.secondary);
            ui.text("■ Accent").fg(custom.accent);
            ui.text("■ Success").fg(custom.success);
            ui.text("■ Warning").fg(custom.warning);
            ui.text("■ Error").fg(custom.error);
        });
        ui.text("").dim();
        ui.text("Theme::builder()").fg(custom.accent);
        ui.text("    .primary(Color::Rgb(255, 107, 107))").dim();
        ui.text("    .secondary(Color::Rgb(78, 205, 196))").dim();
        ui.text("    .accent(Color::Rgb(255, 230, 109))").dim();
        ui.text("    .build()").dim();
    });

    section(ui, "NEW CHARTS");
    ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Pie Chart").bold().fg(theme.primary);
            ui.pie_chart(&[("Rust", 45.0), ("Go", 30.0), ("Python", 25.0)], 6);
        });
        card(ui, |ui| {
            ui.text("Scatter Plot").bold().fg(theme.secondary);
            ui.scatter(
                &[(1.0, 2.0), (2.0, 5.0), (3.0, 3.0), (4.0, 7.0), (5.0, 4.0)],
                30,
                10,
            );
        });
    });

    section(ui, "ANIMATION CALLBACK");
    card(ui, |ui| {
        let val = v8_tween.value(tick);
        let progress = val / 100.0;
        ui.progress(progress);

        ui.row_gap(1, |ui| {
            ui.text(format!("Value: {:.0}", val));
            if *v8_anim_done {
                ui.text("✓ on_complete fired!").fg(Color::Green).bold();
            }
            if ui.button("Restart") {
                v8_tween.reset(tick);
                *v8_anim_done = false;
            }
        });

        if v8_tween.is_done() && !*v8_anim_done {
            *v8_anim_done = true;
        }
    });

    section(ui, "GROUP HOVER");
    ui.row_gap(1, |ui| {
        for name in &["Card A", "Card B", "Card C"] {
            ui.group(name)
                .border(Border::Rounded)
                .p(1)
                .grow(1)
                .group_hover_bg(Color::Indexed(238))
                .col(|ui| {
                    ui.text(*name).bold();
                    ui.text("Hover to highlight").dim();
                });
        }
    });

    section(ui, "AREA CHART");
    card(ui, |ui| {
        ui.chart(
            |c| {
                c.area(&[
                    (0.0, 1.0),
                    (1.0, 4.0),
                    (2.0, 2.0),
                    (3.0, 6.0),
                    (4.0, 3.0),
                    (5.0, 7.0),
                ]);
            },
            40,
            10,
        );
    });

    section(ui, "HOOKS (use_state + use_memo)");
    card(ui, |ui| {
        let counter = ui.use_state(|| 0i32);
        let count_val = *counter.get(ui);
        let doubled = *ui.use_memo(&count_val, |c| c * 2);
        let tripled = *ui.use_memo(&count_val, |c| c * 3);
        ui.row_gap(1, |ui| {
            ui.text(format!("Count: {count_val}"));
            ui.text(format!("×2 = {doubled}")).fg(Color::Cyan);
            ui.text(format!("×3 = {tripled}")).fg(Color::Green);
            if ui.button("+1") {
                *counter.get_mut(ui) += 1;
            }
            if ui.button("-1") {
                *counter.get_mut(ui) -= 1;
            }
            if ui.button("Reset") {
                *counter.get_mut(ui) = 0;
            }
        });
        ui.text("use_memo recomputes only when deps change").dim();
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
