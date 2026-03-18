use slt::{Border, Color, Context, ScrollState, Style, TextInputState, Theme};

struct Sheet {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    cursor_row: usize,
    cursor_col: usize,
    col_widths: Vec<usize>,
}

impl Sheet {
    fn new(headers: Vec<&str>, data: Vec<Vec<&str>>) -> Self {
        let headers: Vec<String> = headers.into_iter().map(String::from).collect();
        let rows: Vec<Vec<String>> = data
            .into_iter()
            .map(|r| r.into_iter().map(String::from).collect())
            .collect();
        let mut col_widths = vec![0usize; headers.len()];
        for (i, h) in headers.iter().enumerate() {
            col_widths[i] = h.len().max(4);
        }
        for row in &rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() {
                    col_widths[i] = col_widths[i].max(cell.len());
                }
            }
        }
        Self {
            headers,
            rows,
            cursor_row: 0,
            cursor_col: 0,
            col_widths,
        }
    }

    fn cell(&self, row: usize, col: usize) -> &str {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    fn set_cell(&mut self, row: usize, col: usize, val: String) {
        if row < self.rows.len() && col < self.headers.len() {
            self.rows[row][col] = val.clone();
            self.col_widths[col] = self.col_widths[col].max(val.len());
        }
    }

    fn total_rows(&self) -> usize {
        self.rows.len()
    }
    fn total_cols(&self) -> usize {
        self.headers.len()
    }
}

fn main() -> std::io::Result<()> {
    let mut sheet = Sheet::new(
        vec![
            "ID",
            "Name",
            "Department",
            "Salary",
            "Start Date",
            "Status",
            "Rating",
        ],
        vec![
            vec![
                "1001",
                "Alice Kim",
                "Engineering",
                "125000",
                "2021-03-15",
                "Active",
                "4.8",
            ],
            vec![
                "1002",
                "Bob Chen",
                "Marketing",
                "95000",
                "2020-07-22",
                "Active",
                "4.2",
            ],
            vec![
                "1003",
                "Carol Wu",
                "Engineering",
                "132000",
                "2019-11-01",
                "Active",
                "4.9",
            ],
            vec![
                "1004",
                "Dan Park",
                "Design",
                "105000",
                "2022-01-10",
                "Active",
                "4.5",
            ],
            vec![
                "1005",
                "Eve Liu",
                "Engineering",
                "128000",
                "2020-05-18",
                "On Leave",
                "4.7",
            ],
            vec![
                "1006",
                "Frank Lee",
                "Sales",
                "88000",
                "2023-02-14",
                "Active",
                "3.9",
            ],
            vec![
                "1007",
                "Grace Cho",
                "Engineering",
                "140000",
                "2018-09-30",
                "Active",
                "5.0",
            ],
            vec![
                "1008",
                "Hank Yun",
                "Marketing",
                "92000",
                "2021-08-05",
                "Active",
                "4.1",
            ],
            vec![
                "1009",
                "Ivy Song",
                "Design",
                "108000",
                "2022-06-20",
                "Active",
                "4.6",
            ],
            vec![
                "1010",
                "Jack Oh",
                "Sales",
                "91000",
                "2023-04-01",
                "Probation",
                "3.5",
            ],
            vec![
                "1011",
                "Kate Ryu",
                "Engineering",
                "135000",
                "2019-01-15",
                "Active",
                "4.8",
            ],
            vec![
                "1012",
                "Leo Bae",
                "HR",
                "98000",
                "2020-11-22",
                "Active",
                "4.3",
            ],
            vec![
                "1013",
                "Mia Jang",
                "Engineering",
                "130000",
                "2021-06-01",
                "Active",
                "4.7",
            ],
            vec![
                "1014",
                "Noah Shin",
                "Finance",
                "115000",
                "2022-03-08",
                "Active",
                "4.4",
            ],
            vec![
                "1015",
                "Olive Han",
                "Design",
                "102000",
                "2023-01-20",
                "Active",
                "4.0",
            ],
            vec![
                "1016",
                "Paul Lim",
                "Engineering",
                "142000",
                "2017-04-10",
                "Active",
                "4.9",
            ],
            vec![
                "1017",
                "Quinn Jung",
                "Sales",
                "87000",
                "2023-07-15",
                "Probation",
                "3.2",
            ],
            vec![
                "1018",
                "Rose Ahn",
                "Marketing",
                "96000",
                "2021-09-12",
                "Active",
                "4.3",
            ],
            vec![
                "1019",
                "Sam Kang",
                "Engineering",
                "138000",
                "2018-12-01",
                "Active",
                "4.8",
            ],
            vec![
                "1020",
                "Tina Moon",
                "HR",
                "95000",
                "2022-08-25",
                "Active",
                "4.1",
            ],
        ],
    );

    let mut editing = false;
    let mut edit_input = TextInputState::new();
    let mut scroll = ScrollState::new();
    let mut formula_bar = String::new();
    let mut dark_mode = true;

    slt::run_with(
        slt::RunConfig {
            mouse: true,
            ..Default::default()
        },
        |ui: &mut Context| {
            if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(slt::KeyCode::Esc) {
                if editing {
                    editing = false;
                } else {
                    ui.quit();
                }
            }
            if ui.key_mod('t', slt::KeyModifiers::CONTROL) {
                dark_mode = !dark_mode;
            }
            ui.set_theme(if dark_mode {
                Theme::dark()
            } else {
                Theme::light()
            });

            if !editing {
                if ui.key_code(slt::KeyCode::Up) {
                    sheet.cursor_row = sheet.cursor_row.saturating_sub(1);
                }
                if ui.key_code(slt::KeyCode::Down) {
                    sheet.cursor_row = (sheet.cursor_row + 1).min(sheet.total_rows() - 1);
                }
                if ui.key_code(slt::KeyCode::Left) {
                    sheet.cursor_col = sheet.cursor_col.saturating_sub(1);
                }
                if ui.key_code(slt::KeyCode::Right) {
                    sheet.cursor_col = (sheet.cursor_col + 1).min(sheet.total_cols() - 1);
                }
                if ui.key_code(slt::KeyCode::Enter) {
                    editing = true;
                    edit_input.value = sheet.cell(sheet.cursor_row, sheet.cursor_col).to_string();
                    edit_input.cursor = edit_input.value.len();
                }
                formula_bar = sheet.cell(sheet.cursor_row, sheet.cursor_col).to_string();
            } else {
                if ui.key_code(slt::KeyCode::Enter) {
                    sheet.set_cell(sheet.cursor_row, sheet.cursor_col, edit_input.value.clone());
                    editing = false;
                }
                if ui.key_code(slt::KeyCode::Esc) {
                    editing = false;
                }
            }

            let col_letter = |c: usize| -> String {
                if c < 26 {
                    format!("{}", (b'A' + c as u8) as char)
                } else {
                    format!(
                        "{}{}",
                        (b'A' + (c / 26 - 1) as u8) as char,
                        (b'A' + (c % 26) as u8) as char
                    )
                }
            };

            let _ = ui
                .bordered(Border::Rounded)
                .title("Spreadsheet")
                .pad(1)
                .grow(1)
                .col(|ui| {
                    // formula bar
                    let _ = ui.row(|ui| {
                        ui.text(format!(
                            "{}{}",
                            col_letter(sheet.cursor_col),
                            sheet.cursor_row + 1
                        ))
                        .bold()
                        .fg(Color::Cyan);
                        ui.text(" │ ").dim();
                        if editing {
                            let _ = ui.text_input(&mut edit_input);
                        } else {
                            ui.text(&formula_bar);
                        }
                    });
                    ui.separator();

                    // column headers
                    let _ = ui.scrollable(&mut scroll).grow(1).col(|ui| {
                        let mut header_line = String::from("     ");
                        for (c, h) in sheet.headers.iter().enumerate() {
                            let w = sheet.col_widths[c] + 2;
                            header_line.push_str(&format!("{:^w$}", h, w = w));
                            if c < sheet.total_cols() - 1 {
                                header_line.push('│');
                            }
                        }
                        ui.text(&header_line).bold().fg(Color::Cyan);

                        let mut sep_line = String::from("─────");
                        for (c, _) in sheet.headers.iter().enumerate() {
                            let w = sheet.col_widths[c] + 2;
                            sep_line.push_str(&"─".repeat(w));
                            if c < sheet.total_cols() - 1 {
                                sep_line.push('┼');
                            }
                        }
                        ui.text(&sep_line).dim();

                        for r in 0..sheet.total_rows() {
                            let row_num = format!("{:>4} ", r + 1);
                            let mut line = String::new();
                            for c in 0..sheet.total_cols() {
                                let w = sheet.col_widths[c] + 2;
                                let val = sheet.cell(r, c);
                                let formatted = if is_numeric(val) {
                                    format!("{:>w$}", val, w = w)
                                } else {
                                    format!(" {:<w$}", val, w = w - 1)
                                };
                                line.push_str(&formatted);
                                if c < sheet.total_cols() - 1 {
                                    line.push('│');
                                }
                            }

                            let is_current_row = r == sheet.cursor_row;
                            let _ = ui.row(|ui| {
                                let num_style = if is_current_row {
                                    Style::new().fg(Color::Cyan).bold()
                                } else {
                                    Style::new().fg(Color::Indexed(240))
                                };
                                ui.styled(&row_num, num_style);

                                if is_current_row {
                                    // highlight current row, emphasize current cell
                                    let mut pos = 0;
                                    for c in 0..sheet.total_cols() {
                                        let w = sheet.col_widths[c] + 2;
                                        let val = sheet.cell(r, c);
                                        let formatted = if is_numeric(val) {
                                            format!("{:>w$}", val, w = w)
                                        } else {
                                            format!(" {:<w$}", val, w = w - 1)
                                        };

                                        if c == sheet.cursor_col {
                                            ui.styled(
                                                &formatted,
                                                Style::new()
                                                    .bg(Color::Cyan)
                                                    .fg(Color::Black)
                                                    .bold(),
                                            );
                                        } else {
                                            ui.styled(
                                                &formatted,
                                                Style::new().fg(Color::White).bold(),
                                            );
                                        }
                                        if c < sheet.total_cols() - 1 {
                                            ui.styled("│", Style::new().fg(Color::Indexed(240)));
                                        }
                                        pos += w + 1;
                                    }
                                    let _ = pos;
                                } else {
                                    ui.styled(&line, Style::new().fg(Color::Indexed(250)));
                                }
                            });
                        }
                    });

                    ui.separator();
                    // status bar
                    let _ = ui.row(|ui| {
                        ui.text(format!(
                            "Cell {}{} | {} rows x {} cols",
                            col_letter(sheet.cursor_col),
                            sheet.cursor_row + 1,
                            sheet.total_rows(),
                            sheet.total_cols(),
                        ))
                        .dim();
                        ui.spacer();
                        if editing {
                            ui.text("EDIT").bold().fg(Color::Yellow);
                        } else {
                            ui.text("NAV").bold().fg(Color::Green);
                        }
                    });
                    let _ = ui.help(&[
                        ("Ctrl+Q", "quit"),
                        ("Ctrl+T", "theme"),
                        ("Arrows", "navigate"),
                        ("e/Enter", "edit"),
                        ("Esc", "cancel"),
                    ]);
                });
        },
    )
}

fn is_numeric(s: &str) -> bool {
    s.parse::<f64>().is_ok()
}
