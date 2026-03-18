use slt::{
    AlertLevel, Align, ApprovalAction, Border, BorderSides, Breakpoint, Color, CommandPaletteState,
    Context, ContextItem, FilePickerState, FormField, FormState, HalfBlockImage, Justify, KeyCode,
    KeyMap, KeyModifiers, ListState, MultiSelectState, PaletteCommand, RadioState, RunConfig,
    ScrollState, SelectState, SpinnerState, StreamingTextState, TableState, TabsState,
    TextInputState, TextareaState, Theme, ToastLevel, ToastState, ToolApprovalState, TreeNode,
    TreeState, Trend,
};

fn main() -> std::io::Result<()> {
    let mut page_tabs = TabsState::new(vec![
        "Core Widgets",
        "Data Viz",
        "Layout",
        "Forms",
        "IME/CJK",
        "Feedback",
        "Advanced",
        "v0.7.0",
        "v0.8.0",
        "v0.9.4",
        "v0.11.0",
        "v0.12.10",
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
    let mut accordion_general = true;
    let mut accordion_advanced = false;
    let mut alert_visible = true;
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
    let mut ime_name = TextInputState::with_placeholder("Type Korean/Japanese/Chinese...");
    let mut ime_search = TextInputState::with_placeholder("Search CJK terms...");
    let mut ime_message = TextareaState::new();
    let ime_items: Vec<String> = vec![
        "한글 입력 테스트",
        "日本語テスト",
        "中文测试",
        "English test",
        "Mixed 한글+English",
        "서울특별시",
        "부산광역시",
        "대구광역시",
        "인천광역시",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    let mut v11_button_clicks: u32 = 0;
    let mut v11_volume = 35.0_f64;
    let mut v11_brightness = 72.0_f64;
    let mut v11_confirm_delete = false;
    let mut v11_autocomplete = TextInputState::with_placeholder("Try: hel / dev / rust");
    v11_autocomplete.set_suggestions(vec![
        "hello".to_string(),
        "help".to_string(),
        "helm".to_string(),
        "developer".to_string(),
        "device".to_string(),
        "rust".to_string(),
        "runner".to_string(),
    ]);
    let mut v11_validated = TextInputState::with_placeholder("username (>=3 chars, alnum)");
    v11_validated.add_validator(|v| {
        if v.len() >= 3 {
            Ok(())
        } else {
            Err("Must be at least 3 characters".to_string())
        }
    });
    v11_validated.add_validator(|v| {
        if v.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            Ok(())
        } else {
            Err("Only [a-zA-Z0-9_] allowed".to_string())
        }
    });
    let mut v11_file_picker = FilePickerState::new(
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
    );
    let v11_keymap = KeyMap::new()
        .bind_mod('q', KeyModifiers::CONTROL, "quit")
        .bind_code(KeyCode::Tab, "focus next")
        .bind_code(KeyCode::Left, "slider -")
        .bind_code(KeyCode::Right, "slider +")
        .bind('y', "confirm yes")
        .bind('n', "confirm no");

    slt::run_with(
        RunConfig {
            mouse: true,
            kitty_keyboard: true,
            ..Default::default()
        },
        |ui: &mut Context| {
            let tick = ui.tick();

            if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
                ui.quit();
            }
            if ui.key_mod('t', slt::KeyModifiers::CONTROL) {
                theme_idx = (theme_idx + 1) % themes.len();
                toasts.info(format!("Theme: {}", theme_names[theme_idx]), tick);
            }
            if ui.key_mod('h', slt::KeyModifiers::CONTROL) {
                progress = (progress - 0.05).max(0.0);
            }
            if ui.key_mod('l', slt::KeyModifiers::CONTROL) {
                progress = (progress + 0.05).min(1.0);
            }
            if ui.key_mod('m', slt::KeyModifiers::CONTROL) {
                show_modal = !show_modal;
            }
            if ui.key_mod('o', slt::KeyModifiers::CONTROL) {
                show_overlay = !show_overlay;
            }
            if ui.key_mod('p', slt::KeyModifiers::CONTROL) {
                palette.open = !palette.open;
            }
            if ui.key_mod('g', slt::KeyModifiers::CONTROL) {
                scroll.offset = 0;
            }
            for i in 1..=9u8 {
                if ui.key_mod((b'0' + i) as char, slt::KeyModifiers::CONTROL) {
                    page_tabs.selected = (i - 1) as usize;
                }
            }

            ui.set_theme(themes[theme_idx]());
            ui.set_dark_mode(v8_dark_mode);

            let theme = *ui.theme();
            let _ = ui
                .container()
                .border(Border::Rounded)
                .pad(1)
                .grow(1)
                .col(|ui| {
                    let _ = ui.row(|ui| {
                        ui.text("SuperLightTUI").bold().fg(theme.primary);
                        ui.text(" widget showcase").fg(theme.text);
                        ui.spacer();
                        ui.text(theme_names[theme_idx]).fg(theme.text_dim);
                    });
                    ui.text("All widgets follow active theme tokens.")
                        .fg(theme.text_dim);
                    ui.separator();

                    let _ = ui.tabs(&mut page_tabs);
                    ui.separator();

                    let _ = ui
                        .scrollable(&mut scroll)
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
                            4 => render_ime(
                                ui,
                                &mut ime_name,
                                &mut ime_search,
                                &mut ime_message,
                                &ime_items,
                            ),
                            5 => render_feedback(ui, &spinner, progress),
                            6 => render_advanced(
                                ui,
                                &mut select,
                                &mut radio,
                                &mut multi,
                                &mut tree,
                                &mut vlist,
                            ),
                            7 => render_v070(
                                ui,
                                &mut v7_scroll,
                                &mut v7_stream,
                                &mut v7_tool,
                                &mut v7_stream_tick,
                            ),
                            8 => render_v080(
                                ui,
                                &mut list_with_filter,
                                &mut list_filter_input,
                                &mut v8_dark_mode,
                                &mut v8_tween,
                                &mut v8_anim_done,
                                tick,
                            ),
                            9 => render_v094(
                                ui,
                                &mut accordion_general,
                                &mut accordion_advanced,
                                &mut alert_visible,
                            ),
                            10 => render_v011(
                                ui,
                                &mut v11_button_clicks,
                                &mut v11_volume,
                                &mut v11_brightness,
                                &mut v11_confirm_delete,
                                &mut v11_autocomplete,
                                &mut v11_validated,
                                &mut v11_file_picker,
                                &v11_keymap,
                            ),
                            11 => render_v01210(ui),
                            _ => {}
                        });

                    ui.separator();
                    let _ = ui.help(&[
                        ("^Q/Esc", "quit"),
                        ("^T", "theme"),
                        ("^M", "modal"),
                        ("^O", "overlay"),
                        ("^H/^L", "progress"),
                        ("^P", "palette"),
                        ("^1-9", "tab"),
                        ("^G", "top"),
                        ("Tab", "focus"),
                        ("F12", "debug"),
                    ]);
                });

            if show_modal {
                let _ = ui.modal(|ui| {
                    let theme = *ui.theme();
                    let _ = ui
                        .container()
                        .bg(theme.surface)
                        .border(Border::Rounded)
                        .pad(2)
                        .col(|ui| {
                            ui.text("Modal Demo").bold().fg(theme.primary);
                            ui.text("This modal stays in the demo.")
                                .fg(theme.surface_text);
                            ui.text("Press m or click close.").fg(theme.surface_text);
                            if ui.button("Close").clicked {
                                show_modal = false;
                            }
                        });
                });
            }

            ui.toast(&mut toasts);

            let _cp = ui.command_palette(&mut palette);
            if let Some(idx) = palette.last_selected {
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
        let _ = ui.tabs(section_tabs);
        let _ = ui.row(|ui| {
            ui.text("Selected:").fg(theme.surface_text);
            match section_tabs.selected {
                0 => ui.text("Primary").fg(theme.primary),
                1 => ui.text("Secondary").fg(theme.secondary),
                _ => ui.text("Accent").fg(theme.accent),
            };
        });
    });

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Input").bold().fg(theme.primary);
            ui.text("Single-line editor").fg(theme.surface_text);
            let _ = ui.text_input(input);
            ui.text("Textarea").fg(theme.surface_text);
            let _ = ui.textarea(textarea, 4);
        });

        card(ui, |ui| {
            ui.text("Controls").bold().fg(theme.secondary);
            ui.text("Theme-aware toggles").fg(theme.surface_text);
            let _ = ui.checkbox("Dark mode", dark_mode);
            let _ = ui.checkbox("Notifications", notifications);
            let _ = ui.toggle("Auto-save", autosave);
            let _ = ui.toggle("Vim mode", vim_mode);
            ui.text("Semantic colors").fg(theme.surface_text);
            let _ = ui.row(|ui| {
                ui.text("success").fg(theme.success);
                ui.text("warning").fg(theme.warning);
                ui.text("error").fg(theme.error);
            });
        });

        card(ui, |ui| {
            ui.text("Buttons").bold().fg(theme.accent);
            ui.text("Primary actions").fg(theme.surface_text);
            let _ = ui.row(|ui| {
                if ui.button("Save").clicked {
                    *saves += 1;
                }
                if ui.button("Reset").clicked {
                    *saves = 0;
                }
            });
            let _ = ui.row(|ui| {
                ui.text("Clicks:").fg(theme.surface_text);
                ui.text(format!("{saves}")).fg(theme.primary);
            });
        });
    });

    let _ = ui.row(|ui| {
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

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Chart").bold().fg(theme.primary);
            ui.text("Line + markers").fg(theme.surface_text);
            let _ = ui.chart(
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
            let _ = ui.sparkline(&spark_data, 28);
            ui.text("Bar chart").fg(theme.surface_text);
            let _ = ui.bar_chart(&bars, 14);
        });
    });

    card(ui, |ui| {
        ui.text("Canvas").bold().fg(theme.accent);
        ui.text("Braille vector drawing").fg(theme.surface_text);
        let _ = ui.canvas(44, 8, |cv| {
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

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Grid").bold().fg(theme.primary);
            ui.text("3-column equal cells").fg(theme.surface_text);
            let _ = ui.grid(3, |ui| {
                for i in 1..=9 {
                    let _ = ui
                        .container()
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
            let _ = ui.list(list);
            let _ = ui.row(|ui| {
                ui.text("Current:").fg(theme.surface_text);
                ui.text(list.selected_item().unwrap_or("-"))
                    .fg(theme.primary);
            });
            ui.separator();
            ui.text("Sort: click header · Filter + Pagination").dim();
            let _ = ui.text_input(table_filter);
            table.set_filter(&table_filter.value);
            let _ = ui.table(table);
            if let Some(row) = table.selected_row() {
                let _ = ui.row(|ui| {
                    ui.text("Selected:").fg(theme.surface_text);
                    ui.text(row.join(", ")).fg(theme.primary);
                });
            } else {
                ui.text("No matching rows").dim();
            }
            let _ = ui.row(|ui| {
                ui.text(format!(
                    "{} / {} rows",
                    table.visible_indices().len(),
                    table.rows.len(),
                ))
                .dim();
                ui.spacer();
                if let Some(col) = table.sort_column {
                    let dir = if table.sort_ascending { "ASC" } else { "DESC" };
                    ui.text(format!("{} {dir}", table.headers[col]))
                        .fg(theme.text_dim);
                }
            });
        });
    });

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Align").bold().fg(theme.primary);
            ui.text("Start / Center / End").fg(theme.surface_text);
            let _ = ui
                .container()
                .bg(theme.surface_hover)
                .border(Border::Rounded)
                .pad(1)
                .align(Align::Start)
                .col(|ui| {
                    ui.text("Start").fg(theme.primary);
                });
            let _ = ui
                .container()
                .bg(theme.surface_hover)
                .border(Border::Rounded)
                .pad(1)
                .align(Align::Center)
                .col(|ui| {
                    ui.text("Center").fg(theme.secondary);
                });
            let _ = ui
                .container()
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
            let _ = ui
                .container()
                .bg(theme.surface_hover)
                .border(Border::Rounded)
                .pad(1)
                .justify(Justify::SpaceBetween)
                .row(|ui| {
                    ui.text("A").fg(theme.primary);
                    ui.text("B").fg(theme.secondary);
                    ui.text("C").fg(theme.accent);
                });
            let _ = ui
                .container()
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
        let _ = ui.overlay(|ui| {
            let theme = *ui.theme();
            let _ = ui.row(|ui| {
                ui.spacer();
                let _ = ui
                    .container()
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

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Sign In Form").bold().fg(theme.primary);
            ui.text("Modal/form showcase retained")
                .fg(theme.surface_text);
            for field in form.fields.iter_mut() {
                ui.form_field(field);
            }
            if ui.form_submit("Sign In").clicked {
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
            let _ = ui.text_input(password);
            let _ = ui.row(|ui| {
                ui.text("Length:").fg(theme.surface_text);
                ui.text(format!("{}", password.value.len()))
                    .fg(theme.primary);
            });
        });
    });
}

fn render_ime(
    ui: &mut Context,
    ime_name: &mut TextInputState,
    ime_search: &mut TextInputState,
    ime_message: &mut TextareaState,
    ime_items: &[String],
) {
    let theme = *ui.theme();
    section(ui, "IME / CJK INPUT");

    card(ui, |ui| {
        ui.text("Compose Korean/Japanese/Chinese text")
            .fg(theme.surface_text);
        let _ = ui.row(|ui| {
            let _ = ui.container().grow(1).col(|ui| {
                ui.text("Name").bold().fg(theme.primary);
                let _ = ui.text_input(ime_name);
                if !ime_name.value.is_empty() {
                    ui.line(|ui| {
                        ui.text("chars: ").fg(theme.surface_text);
                        ui.text(format!("{}", ime_name.value.chars().count()))
                            .fg(theme.accent);
                    });
                }
            });

            let _ = ui.container().grow(1).col(|ui| {
                ui.text("Search").bold().fg(theme.secondary);
                let _ = ui.text_input(ime_search);
                let query = ime_search.value.to_lowercase();
                let tokens: Vec<&str> = query.split_whitespace().collect();
                let matched = ime_items
                    .iter()
                    .filter(|item| {
                        let lower = item.to_lowercase();
                        tokens.is_empty() || tokens.iter().all(|t| lower.contains(t))
                    })
                    .count();
                ui.text(format!("{matched}/{} matches", ime_items.len()))
                    .fg(theme.surface_text);
            });
        });

        ui.separator();
        ui.text("Message").bold().fg(theme.primary);
        let rows = ui.height().saturating_sub(24).max(5);
        let _ = ui.textarea(ime_message, rows);
        let total_chars: usize = ime_message
            .lines
            .iter()
            .map(|line| line.chars().count())
            .sum();
        ui.text(format!(
            "{} lines, {} chars",
            ime_message.lines.len(),
            total_chars,
        ))
        .dim();
    });
}

#[allow(clippy::too_many_arguments)]
fn render_v011(
    ui: &mut Context,
    button_clicks: &mut u32,
    volume: &mut f64,
    brightness: &mut f64,
    confirm_delete: &mut bool,
    autocomplete: &mut TextInputState,
    validated: &mut TextInputState,
    file_picker: &mut FilePickerState,
    keymap: &KeyMap,
) {
    let theme = *ui.theme();
    section(ui, "v0.11.0 FEATURES");

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Response Pattern").bold().fg(theme.primary);
            let response = ui.button("Inspect Response");
            if response.clicked {
                *button_clicks = (*button_clicks).saturating_add(1);
            }
            ui.text(format!(
                "clicked={} hovered={} focused={} total_clicks={}",
                response.clicked, response.hovered, response.focused, *button_clicks,
            ))
            .fg(theme.surface_text);
        });

        card(ui, |ui| {
            ui.text("Slider").bold().fg(theme.secondary);
            if ui.slider("Volume", volume, 0.0..=100.0).changed {
                ui.notify("Volume updated", ToastLevel::Info);
            }
            if ui.slider("Brightness", brightness, 0.0..=100.0).changed {
                ui.notify("Brightness updated", ToastLevel::Success);
            }
            ui.text(format!(
                "Volume {:.0} / Brightness {:.0}",
                *volume, *brightness
            ))
            .fg(theme.surface_text);
        });
    });

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Confirm + Notify").bold().fg(theme.accent);
            let confirmed = ui.confirm("Delete selected file?", confirm_delete);
            if confirmed.clicked {
                if *confirm_delete {
                    ui.notify("Delete confirmed", ToastLevel::Warning);
                } else {
                    ui.notify("Delete canceled", ToastLevel::Info);
                }
            }
            if ui.button("Trigger success toast").clicked {
                ui.notify("Saved successfully", ToastLevel::Success);
            }
            ui.text(if *confirm_delete {
                "Last answer: Yes"
            } else {
                "Last answer: No"
            })
            .fg(theme.surface_text);
        });

        card(ui, |ui| {
            ui.text("LightDark + Tailwind Palette")
                .bold()
                .fg(theme.primary);
            let adaptive = ui.light_dark(
                slt::palette::tailwind::SLATE.c900,
                slt::palette::tailwind::SLATE.c100,
            );
            ui.text("Adaptive foreground sample").fg(adaptive);
            ui.line(|ui| {
                ui.text("██ RED500 ").fg(slt::palette::tailwind::RED.c500);
                ui.text("██ BLUE500 ").fg(slt::palette::tailwind::BLUE.c500);
                ui.text("██ GREEN500")
                    .fg(slt::palette::tailwind::GREEN.c500);
            });
            ui.text("Using slt::palette::tailwind::* constants").dim();
        });
    });

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Autocomplete + Validators")
                .bold()
                .fg(theme.secondary);
            ui.text("Autocomplete").fg(theme.surface_text);
            let _ = ui.text_input(autocomplete);
            let matches = autocomplete.matched_suggestions();
            ui.text(format!("matches: {}", matches.join(", "))).dim();

            ui.separator();
            ui.text("Validators").fg(theme.surface_text);
            let _ = ui.text_input(validated);
            validated.run_validators();
            if validated.errors().is_empty() {
                ui.text("All validators passed").fg(theme.success);
            } else {
                for err in validated.errors() {
                    ui.text(format!("• {err}")).fg(theme.error);
                }
            }
        });

        card(ui, |ui| {
            ui.text("File Picker + KeyMap").bold().fg(theme.accent);
            if ui.file_picker(file_picker).changed {
                if let Some(path) = file_picker.selected() {
                    let name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("selected file");
                    ui.notify(&format!("Picked: {name}"), ToastLevel::Success);
                }
            }
            ui.text(format!("Dir: {}", file_picker.current_dir.display()))
                .fg(theme.surface_text)
                .wrap();
            ui.separator();
            let _ = ui.help_from_keymap(keymap);
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

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Select").bold().fg(theme.primary);
            ui.text("Dropdown style preset").fg(theme.surface_text);
            let _changed = ui.select(select).changed;
            let _ = ui.row(|ui| {
                ui.text("Current:").fg(theme.surface_text);
                ui.text(&select.items[select.selected]).fg(theme.primary);
            });
        });

        card(ui, |ui| {
            ui.text("Radio").bold().fg(theme.secondary);
            ui.text("Theme preference").fg(theme.surface_text);
            let _changed = ui.radio(radio).changed;
            let _ = ui.row(|ui| {
                ui.text("Mode:").fg(theme.surface_text);
                ui.text(&radio.items[radio.selected]).fg(theme.secondary);
            });
        });

        card(ui, |ui| {
            ui.text("Multi Select").bold().fg(theme.accent);
            ui.text("Feature toggles").fg(theme.surface_text);
            let _ = ui.multi_select(multi);
            let _ = ui.row(|ui| {
                ui.text("Enabled:").fg(theme.surface_text);
                ui.text(format!("{}", multi.selected.len()))
                    .fg(theme.accent);
            });
        });
    });

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Tree").bold().fg(theme.primary);
            ui.text("Project structure").fg(theme.surface_text);
            let _ = ui.tree(tree);
        });

        card(ui, |ui| {
            ui.text("Virtual List").bold().fg(theme.secondary);
            ui.text("100 items, 8 visible").fg(theme.surface_text);
            let _ = ui.virtual_list(vlist, 8, |ui, idx| {
                ui.text(format!("Item {idx}")).fg(theme.surface_text);
            });
        });
    });

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Markdown").bold().fg(theme.primary);
            let _ = ui.markdown(
                "# v0.7.0\n\n**9 new features**: dashed borders, Kitty keyboard, color downsampling, scrollbar, breakpoints, clipboard, DevTools, half-block image, AI widgets.\n\n- Check the **v0.7.0** tab →\n- Press **F12** for DevTools\n\n---\n\n`Theme-aware` and production-ready.",
            );
        });

        card(ui, |ui| {
            ui.text("Rich Text").bold().fg(theme.secondary);
            ui.text("line() and line_wrap()").fg(theme.surface_text);

            ui.line(|ui| {
                ui.text("Status: ");
                ui.text("Online").bold().fg(theme.success);
                ui.text(" · ");
                ui.text("3 tasks").fg(theme.accent);
            });

            ui.line(|ui| {
                ui.text("Error: ").fg(theme.error);
                ui.text("file ").fg(theme.surface_text);
                ui.text("config.toml").bold().fg(theme.primary);
                ui.text(" not found").fg(theme.surface_text);
            });

            let _ = ui.container()
                .bg(theme.surface_hover)
                .border(Border::Rounded)
                .pad(1)
                .col(|ui| {
                    ui.text("line_wrap()").bold().fg(theme.accent);
                    ui.line_wrap(|ui| {
                        ui.text("This ");
                        ui.text("wraps ").bold();
                        ui.text("across lines while keeping ");
                        ui.text("styles").fg(theme.primary).bold();
                        ui.text(" on each segment.");
                    });
                });
        });
    });

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Borders + Percent Sizing").bold().fg(theme.accent);
            ui.text("Per-side borders and 30/70 layout").fg(theme.surface_text);

            let _ = ui.container()
                .bg(theme.surface_hover)
                .border(Border::Single)
                .border_sides(BorderSides::horizontal())
                .pad(1)
                .col(|ui| {
                    ui.text("Horizontal borders").fg(theme.surface_text);
                });

            let _ = ui.container()
                .bg(theme.surface_hover)
                .border(Border::Single)
                .border_sides(BorderSides::vertical())
                .pad(1)
                .col(|ui| {
                    ui.text("Vertical borders").fg(theme.surface_text);
                });

            let _ = ui.row(|ui| {
                let _ = ui.container()
                    .bg(theme.surface_hover)
                    .border(Border::Rounded)
                    .w_pct(30)
                    .pad(1)
                    .col(|ui| {
                        ui.text("30%").fg(theme.primary);
                    });
                let _ = ui.container()
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
            let _ = ui.markdown(
                "Inline: **bold text** and *italic text* and `code blocks` all render with proper styling.\n\n- List with **bold** items\n- And `inline code` too",
            );
        });
    });
}

fn render_feedback(ui: &mut Context, spinner: &SpinnerState, progress: f64) {
    let theme = *ui.theme();
    section(ui, "FEEDBACK");

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Progress").bold().fg(theme.primary);
            let _ = ui.row(|ui| {
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

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("Dashed Borders").bold().fg(theme.primary);
            ui.text("2 new border variants").fg(theme.surface_text);

            let _ = ui.container().border(Border::Dashed).pad(1).col(|ui| {
                ui.text("Border::Dashed").fg(theme.text);
            });
            let _ = ui.container().border(Border::DashedThick).pad(1).col(|ui| {
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
                let _ = ui.container().border(border).col(|ui| {
                    ui.text(name).fg(theme.surface_text);
                });
            }
        });
    });

    let _ = ui.row(|ui| {
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
            let _ = ui.row(|ui| {
                let _ = ui.scrollable(scroll).grow(1).h(8).col(|ui| {
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
            let _ = ui.row(|ui| {
                ui.text("Current: ").fg(theme.surface_text);
                ui.text(label).bold().fg(color);
            });
            ui.text(format!("Terminal: {}×{}", ui.width(), ui.height()))
                .dim();
            ui.text("Resize terminal to see changes").fg(theme.text_dim);
        });
    });

    let _ = ui.row(|ui| {
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
            let _ = ui.streaming_text(stream);

            if !stream.streaming && !stream.content.is_empty() && ui.button("↻ Replay").clicked {
                stream.content.clear();
            }
        });

        card(ui, |ui| {
            ui.text("ToolApproval").bold().fg(theme.secondary);
            ui.text("Human-in-the-loop gate").fg(theme.surface_text);
            let _ = ui.tool_approval(tool);
            if tool.action != ApprovalAction::Pending && ui.button("Reset").clicked {
                tool.reset();
            }
        });
    });

    let _ = ui.row(|ui| {
        card(ui, |ui| {
            ui.text("ContextBar").bold().fg(theme.primary);
            ui.text("Token usage indicator").fg(theme.surface_text);
            let items = vec![
                ContextItem::new("main.rs", 1200),
                ContextItem::new("lib.rs", 3400),
                ContextItem::new("README.md", 800),
            ];
            let _ = ui.context_bar(&items);
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
            let _ = ui.image(&img);
        });
    });

    card(ui, |ui| {
        ui.text("More v0.7.0").bold().fg(theme.accent);
        let _ = ui.row(|ui| {
            let _ = ui.container().grow(1).col(|ui| {
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
            let _ = ui.container().grow(1).col(|ui| {
                ui.text("Kitty Keyboard").bold().fg(theme.secondary);
                ui.text("Key release events enabled").fg(theme.surface_text);
                ui.text("kitty_keyboard: true").fg(theme.secondary);
                ui.separator();
                ui.text("OSC 52 Clipboard").bold().fg(theme.accent);
                ui.text("copy_to_clipboard()").fg(theme.surface_text);
                if ui.button("Copy 'SLT v0.7.0'").clicked {
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

    section(ui, "STYLE RECIPES");
    {
        const CARD: slt::ContainerStyle = slt::ContainerStyle::new().border(Border::Rounded).p(1);
        const ACCENT: slt::ContainerStyle =
            slt::ContainerStyle::new().bg(Color::Rgb(255, 107, 107));

        let _ = ui.row_gap(1, |ui| {
            let _ = ui.container().apply(&CARD).grow(1).col(|ui| {
                ui.text("Base card").bold();
                ui.text("ContainerStyle::new().border(..).p(1)").dim();
            });
            let _ = ui
                .container()
                .apply(&CARD)
                .apply(&ACCENT)
                .grow(1)
                .col(|ui| {
                    ui.text("Card + Accent").bold();
                    ui.text(".apply(&CARD).apply(&ACCENT)").dim();
                });
        });
    }

    section(ui, "ERROR BOUNDARY");
    let _ = ui.row_gap(1, |ui| {
        let _ = ui
            .container()
            .grow(1)
            .border(Border::Rounded)
            .p(1)
            .col(|ui| {
                ui.error_boundary(|ui| {
                    ui.text("Safe content").fg(theme.success);
                });
            });
        let _ = ui
            .container()
            .grow(1)
            .border(Border::Rounded)
            .p(1)
            .col(|ui| {
                ui.error_boundary_with(
                    |ui| {
                        ui.text("Protected zone");
                    },
                    |ui, _msg| {
                        ui.text("Caught a panic!").fg(theme.error);
                    },
                );
                ui.text("error_boundary_with catches panics").dim();
            });
    });

    section(ui, "DARK MODE");
    card(ui, |ui| {
        let _ = ui.row_gap(2, |ui| {
            let _ = ui
                .container()
                .bg(Color::Rgb(240, 240, 240))
                .dark_bg(Color::Rgb(30, 30, 46))
                .p(1)
                .col(|ui| {
                    ui.text("This background changes with dark/light mode");
                });
            if ui.button("Toggle Dark").clicked {
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
        let _ = ui.row_gap(1, |ui| {
            let _ = ui
                .container()
                .w(20)
                .md_w(30)
                .lg_w(40)
                .border(Border::Rounded)
                .p(1)
                .col(|ui| {
                    ui.text("Responsive width");
                });
            let _ = ui
                .container()
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
        let _ = ui.text_input(list_filter_input);
        if list_filter_input.value != list_with_filter.filter {
            list_with_filter.set_filter(&list_filter_input.value);
        }
        let _ = ui.list(list_with_filter);
        ui.text(format!(
            "{}/{} items shown",
            list_with_filter.visible_indices().len(),
            8
        ))
        .dim();
    });

    section(ui, "THEME BUILDER");
    card(ui, |ui| {
        let presets: &[(&str, slt::Theme)] = &[
            (
                "Coral",
                slt::Theme::builder()
                    .primary(Color::Rgb(255, 107, 107))
                    .secondary(Color::Rgb(78, 205, 196))
                    .accent(Color::Rgb(255, 230, 109))
                    .build(),
            ),
            (
                "Ocean",
                slt::Theme::builder()
                    .primary(Color::Rgb(86, 156, 214))
                    .secondary(Color::Rgb(78, 201, 176))
                    .accent(Color::Rgb(209, 154, 102))
                    .build(),
            ),
            (
                "Forest",
                slt::Theme::builder()
                    .primary(Color::Rgb(152, 195, 121))
                    .secondary(Color::Rgb(229, 192, 123))
                    .accent(Color::Rgb(198, 120, 221))
                    .build(),
            ),
        ];
        let idx_state = ui.use_state(|| 0usize);
        let idx = *idx_state.get(ui);
        let (_name, ref custom) = presets[idx % presets.len()];

        let _ = ui.row_gap(1, |ui| {
            for (i, (label, _)) in presets.iter().enumerate() {
                if i == idx {
                    ui.text(format!("● {label}")).bold().fg(custom.primary);
                } else if ui.button(*label).clicked {
                    *idx_state.get_mut(ui) = i;
                }
            }
            ui.text("  →  applies to entire app").dim();
        });
        let _ = ui.row_gap(1, |ui| {
            ui.text("■ Primary").fg(custom.primary);
            ui.text("■ Secondary").fg(custom.secondary);
            ui.text("■ Accent").fg(custom.accent);
            ui.text("■ Success").fg(custom.success);
            ui.text("■ Warning").fg(custom.warning);
            ui.text("■ Error").fg(custom.error);
        });
        ui.set_theme(presets[idx % presets.len()].1);
    });

    section(ui, "SCATTER PLOT");
    card(ui, |ui| {
        let _ = ui.scatter(
            &[(1.0, 2.0), (2.0, 5.0), (3.0, 3.0), (4.0, 7.0), (5.0, 4.0)],
            40,
            10,
        );
    });

    section(ui, "ANIMATION CALLBACK");
    card(ui, |ui| {
        let val = v8_tween.value(tick);
        let progress = val / 100.0;
        ui.progress(progress);

        let _ = ui.row_gap(1, |ui| {
            ui.text(format!("Value: {:.0}", val));
            if *v8_anim_done {
                ui.text("✓ on_complete fired!").fg(theme.success).bold();
            }
            if ui.button("Restart").clicked {
                v8_tween.reset(tick);
                *v8_anim_done = false;
            }
        });

        if v8_tween.is_done() && !*v8_anim_done {
            *v8_anim_done = true;
        }
    });

    section(ui, "GROUP HOVER");
    let _ = ui.row_gap(1, |ui| {
        for name in &["Card A", "Card B", "Card C"] {
            let _ = ui
                .group(name)
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

    section(ui, "HOOKS (use_state + use_memo)");
    card(ui, |ui| {
        let counter = ui.use_state(|| 0i32);
        let count_val = *counter.get(ui);
        let doubled = *ui.use_memo(&count_val, |c| c * 2);
        let tripled = *ui.use_memo(&count_val, |c| c * 3);
        let _ = ui.row_gap(1, |ui| {
            ui.text(format!("Count: {count_val}"));
            ui.text(format!("×2 = {doubled}")).fg(theme.primary);
            ui.text(format!("×3 = {tripled}")).fg(theme.success);
            if ui.button("+1").clicked {
                *counter.get_mut(ui) += 1;
            }
            if ui.button("-1").clicked {
                *counter.get_mut(ui) -= 1;
            }
            if ui.button("Reset").clicked {
                *counter.get_mut(ui) = 0;
            }
        });
        ui.text("use_memo recomputes only when deps change").dim();
    });
}

fn card(ui: &mut Context, f: impl FnOnce(&mut Context)) {
    let theme = *ui.theme();
    let _ = ui
        .container()
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

fn render_v01210(ui: &mut Context) {
    let theme = *ui.theme();

    section(ui, "v0.12.10 — TAILWIND-LEVEL ERGONOMICS");
    ui.text("");

    // ── 1. flex_center ────────────────────────────────────────────
    let _ = ui.divider_text("flex_center()");
    let _ = ui
        .container()
        .border(Border::Rounded)
        .h(5)
        .flex_center()
        .col(|ui| {
            ui.text("Perfectly centered with .flex_center()").bold();
        });

    ui.text("");

    // ── 2. border_x / border_y ────────────────────────────────────
    let _ = ui.divider_text("border_x() / border_y()");
    let _ = ui.row(|ui| {
        let _ = ui
            .container()
            .border(Border::Rounded)
            .border_x()
            .p(1)
            .grow(1)
            .col(|ui| {
                ui.text(".border_x()").bold().fg(theme.primary);
                ui.text("left + right only");
            });
        let _ = ui
            .container()
            .border(Border::Rounded)
            .border_y()
            .p(1)
            .grow(1)
            .col(|ui| {
                ui.text(".border_y()").bold().fg(theme.secondary);
                ui.text("top + bottom only");
            });
        let _ = ui
            .container()
            .border(Border::Rounded)
            .p(1)
            .grow(1)
            .col(|ui| {
                ui.text(".border() (all)").bold().fg(theme.accent);
                ui.text("all four sides");
            });
    });

    ui.text("");

    // ── 3. text_center / text_right ───────────────────────────────
    let _ = ui.divider_text("text_center() / text_right()");
    let _ = ui.container().border(Border::Single).p(1).col(|ui| {
        ui.text("Left (default)").fg(theme.primary);
        ui.text("Centered text").text_center().fg(theme.secondary);
        ui.text("Right-aligned text").text_right().fg(theme.accent);
    });

    ui.text("");

    // ── 4. text_color ─────────────────────────────────────────────
    let _ = ui.divider_text("text_color() — style inheritance");
    let _ = ui.row(|ui| {
        let _ = ui
            .container()
            .border(Border::Rounded)
            .text_color(Color::Rgb(255, 180, 50))
            .p(1)
            .grow(1)
            .col(|ui| {
                ui.text("Orange by default");
                ui.text("Still orange");
                ui.text("Overridden!").fg(Color::Rgb(100, 255, 100));
            });
        let _ = ui
            .container()
            .border(Border::Rounded)
            .text_color(Color::Rgb(130, 180, 255))
            .p(1)
            .grow(1)
            .col(|ui| {
                ui.text("Blue by default");
                ui.text("Nested containers inherit:");
                let _ = ui.container().p(1).col(|ui| {
                    ui.text("Still blue in child");
                });
            });
    });

    ui.text("");

    // ── 5. row_gap / col_gap ──────────────────────────────────────
    let _ = ui.divider_text("row_gap() / col_gap()");
    let _ = ui.row(|ui| {
        let _ = ui
            .container()
            .border(Border::Rounded)
            .row_gap(1)
            .p(1)
            .grow(1)
            .col(|ui| {
                ui.text(".row_gap(1)").bold().fg(theme.primary);
                ui.text("Row A");
                ui.text("Row B");
                ui.text("Row C");
            });
        let _ = ui
            .container()
            .border(Border::Rounded)
            .col_gap(4)
            .p(1)
            .grow(1)
            .row(|ui| {
                ui.text(".col_gap(4)").bold().fg(theme.secondary);
                ui.text("A");
                ui.text("B");
                ui.text("C");
            });
        let _ = ui
            .container()
            .border(Border::Rounded)
            .gap(0)
            .p(1)
            .grow(1)
            .col(|ui| {
                ui.text(".gap(0) — tight").bold().fg(theme.accent);
                ui.text("Row A");
                ui.text("Row B");
                ui.text("Row C");
            });
    });

    ui.text("");

    // ── 6. align_self ─────────────────────────────────────────────
    let _ = ui.divider_text("align_self() — per-child cross-axis override");
    let _ = ui
        .container()
        .border(Border::Rounded)
        .h(7)
        .gap(0)
        .col(|ui| {
            let _ = ui
                .container()
                .align_self(Align::Start)
                .border(Border::Single)
                .px(1)
                .row(|ui| {
                    ui.text("align_self(Start)").fg(theme.primary);
                });
            let _ = ui
                .container()
                .align_self(Align::Center)
                .border(Border::Single)
                .px(1)
                .row(|ui| {
                    ui.text("align_self(Center)").fg(theme.secondary);
                });
            let _ = ui
                .container()
                .align_self(Align::End)
                .border(Border::Single)
                .px(1)
                .row(|ui| {
                    ui.text("align_self(End)").fg(theme.accent);
                });
        });

    ui.text("");

    // ── 7. truncate ───────────────────────────────────────────────
    let _ = ui.divider_text("truncate() — text overflow with ellipsis");
    let _ = ui.container()
        .border(Border::Rounded)
        .p(1)
        .col(|ui| {
            ui.text("No truncation — this text is rendered at full length without any clipping applied")
                .fg(theme.text_dim);
            ui.text("With .truncate() — this text is way too long and will be truncated with an ellipsis character at the end")
                .truncate()
                .fg(theme.primary);
            ui.text("Truncate + bold — another long line that demonstrates truncation working with style chains together")
                .truncate()
                .bold()
                .fg(theme.accent);
        });
}

fn render_v094(
    ui: &mut Context,
    accordion_general: &mut bool,
    accordion_advanced: &mut bool,
    alert_visible: &mut bool,
) {
    section(ui, "v0.9.4 WIDGETS");
    ui.text("");

    if *alert_visible
        && ui
            .alert(
                "Deployment successful — all checks passed",
                AlertLevel::Success,
            )
            .clicked
    {
        *alert_visible = false;
    }

    let _ = ui.divider_text("Navigation");
    ui.breadcrumb(&["Home", "Settings", "Profile"]);

    let _ = ui.divider_text("Dashboard");
    let _ = ui.row(|ui| {
        card(ui, |ui| {
            let _ = ui.stat_trend("Revenue", "$12,400", Trend::Up);
        });
        card(ui, |ui| {
            let _ = ui.stat_trend("Errors", "3", Trend::Down);
        });
        card(ui, |ui| {
            let _ = ui.stat_colored("CPU", "72%", ui.theme().warning);
        });
        card(ui, |ui| {
            let _ = ui.stat("Uptime", "14d 3h");
        });
    });

    let _ = ui.divider_text("Inline Elements");
    ui.line(|ui| {
        let _ = ui.badge("v0.9.4");
        ui.text(" ");
        let _ = ui.badge_colored("Stable", ui.theme().success);
        ui.text(" ");
        let _ = ui.badge_colored("Rust", ui.theme().accent);
        ui.text("   ");
        let _ = ui.key_hint("Ctrl+S");
        ui.text(" save  ");
        let _ = ui.key_hint("Ctrl+Q");
        ui.text(" quit");
    });

    let _ = ui.divider_text("Accordions");
    let _ = ui.accordion("General Settings", accordion_general, |ui| {
        let _ = ui.definition_list(&[
            ("Theme", "Dark"),
            ("Language", "en-US"),
            ("Font Size", "14px"),
        ]);
    });
    let _ = ui.accordion("Advanced Settings", accordion_advanced, |ui| {
        let _ = ui.definition_list(&[
            ("Log Level", "debug"),
            ("Max Conn", "100"),
            ("Timeout", "30s"),
        ]);
    });

    let _ = ui.divider_text("Code Block");
    let _ = ui.code_block_numbered(
        "fn main() {\n    slt::run(|ui| {\n        ui.text(\"hello\");\n    });\n}",
    );

    let _ = ui.divider_text("Empty State");
    let _ = ui.container().h(3).col(|ui| {
        let _ = ui.empty_state("No items yet", "Items will appear here when added");
    });
}
