//! Demo: spreadsheet-style table editor.
//!
//! Archetype: **Standard** (full-canvas, no overlay, no scrollback).
//!
//! §2 (Demo Guide): exposes `pub fn render(ui, &mut DemoState)` so a
//! composing demo can preserve the cursor position, edit-mode flag, the
//! typed edit value, and the scroll offset across tab switches. The
//! standalone `main()` is a thin wrapper.

use slt::{Border, Color, Context, ScrollState, Style, TextInputState, Theme};

pub struct Sheet {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub col_widths: Vec<usize>,
}

impl Sheet {
    pub fn new(headers: Vec<&str>, data: Vec<Vec<&str>>) -> Self {
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

    pub fn cell(&self, row: usize, col: usize) -> &str {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub fn set_cell(&mut self, row: usize, col: usize, val: String) {
        if row < self.rows.len() && col < self.headers.len() {
            self.rows[row][col] = val.clone();
            self.col_widths[col] = self.col_widths[col].max(val.len());
        }
    }

    pub fn total_rows(&self) -> usize {
        self.rows.len()
    }
    pub fn total_cols(&self) -> usize {
        self.headers.len()
    }
}

/// Persistent state for the spreadsheet demo.
pub struct DemoState {
    pub sheet: Sheet,
    pub editing: bool,
    pub edit_input: TextInputState,
    pub scroll: ScrollState,
    pub formula_bar: String,
    pub dark_mode: bool,
}

impl DemoState {
    pub fn new() -> Self {
        let sheet = Sheet::new(
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
        Self {
            sheet,
            editing: false,
            edit_input: TextInputState::new(),
            scroll: ScrollState::new(),
            formula_bar: String::new(),
            dark_mode: true,
        }
    }
}

impl Default for DemoState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render one frame of the spreadsheet demo.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    if ui.key_mod('t', slt::KeyModifiers::CONTROL) {
        state.dark_mode = !state.dark_mode;
    }
    ui.set_theme(if state.dark_mode {
        Theme::dark()
    } else {
        Theme::light()
    });

    if !state.editing {
        if ui.key_code(slt::KeyCode::Up) {
            state.sheet.cursor_row = state.sheet.cursor_row.saturating_sub(1);
        }
        if ui.key_code(slt::KeyCode::Down) {
            state.sheet.cursor_row = (state.sheet.cursor_row + 1).min(state.sheet.total_rows() - 1);
        }
        if ui.key_code(slt::KeyCode::Left) {
            state.sheet.cursor_col = state.sheet.cursor_col.saturating_sub(1);
        }
        if ui.key_code(slt::KeyCode::Right) {
            state.sheet.cursor_col = (state.sheet.cursor_col + 1).min(state.sheet.total_cols() - 1);
        }
        if ui.key_code(slt::KeyCode::Enter) {
            state.editing = true;
            state.edit_input.value = state
                .sheet
                .cell(state.sheet.cursor_row, state.sheet.cursor_col)
                .to_string();
            state.edit_input.cursor = state.edit_input.value.len();
        }
        state.formula_bar = state
            .sheet
            .cell(state.sheet.cursor_row, state.sheet.cursor_col)
            .to_string();
    } else {
        if ui.key_code(slt::KeyCode::Enter) {
            state.sheet.set_cell(
                state.sheet.cursor_row,
                state.sheet.cursor_col,
                state.edit_input.value.clone(),
            );
            state.editing = false;
        }
        if ui.key_code(slt::KeyCode::Esc) {
            state.editing = false;
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
        .p(1)
        .grow(1)
        .col(|ui| {
            // formula bar
            let _ = ui.row(|ui| {
                ui.text(format!(
                    "{}{}",
                    col_letter(state.sheet.cursor_col),
                    state.sheet.cursor_row + 1
                ))
                .bold()
                .fg(Color::Cyan);
                ui.text(" | ").dim();
                if state.editing {
                    let _ = ui.text_input(&mut state.edit_input);
                } else {
                    ui.text(&state.formula_bar);
                }
            });
            let _ = ui.separator();

            // column headers
            let _ = ui.scrollable(&mut state.scroll).grow(1).col(|ui| {
                let mut header_line = String::from("     ");
                for (c, h) in state.sheet.headers.iter().enumerate() {
                    let w = state.sheet.col_widths[c] + 2;
                    header_line.push_str(&format!("{:^w$}", h, w = w));
                    if c < state.sheet.total_cols() - 1 {
                        header_line.push('\u{2502}');
                    }
                }
                ui.text(&header_line).bold().fg(Color::Cyan);

                let mut sep_line = String::from("\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
                for (c, _) in state.sheet.headers.iter().enumerate() {
                    let w = state.sheet.col_widths[c] + 2;
                    sep_line.push_str(&"\u{2500}".repeat(w));
                    if c < state.sheet.total_cols() - 1 {
                        sep_line.push('\u{253C}');
                    }
                }
                ui.text(&sep_line).dim();

                for r in 0..state.sheet.total_rows() {
                    let row_num = format!("{:>4} ", r + 1);
                    let mut line = String::new();
                    for c in 0..state.sheet.total_cols() {
                        let w = state.sheet.col_widths[c] + 2;
                        let val = state.sheet.cell(r, c);
                        let formatted = if is_numeric(val) {
                            format!("{:>w$}", val, w = w)
                        } else {
                            format!(" {:<w$}", val, w = w - 1)
                        };
                        line.push_str(&formatted);
                        if c < state.sheet.total_cols() - 1 {
                            line.push('\u{2502}');
                        }
                    }

                    let is_current_row = r == state.sheet.cursor_row;
                    let _ = ui.row(|ui| {
                        let num_style = if is_current_row {
                            Style::new().fg(Color::Cyan).bold()
                        } else {
                            Style::new().fg(Color::Indexed(240))
                        };
                        ui.styled(&row_num, num_style);

                        if is_current_row {
                            // highlight current row, emphasize current cell
                            for c in 0..state.sheet.total_cols() {
                                let w = state.sheet.col_widths[c] + 2;
                                let val = state.sheet.cell(r, c);
                                let formatted = if is_numeric(val) {
                                    format!("{:>w$}", val, w = w)
                                } else {
                                    format!(" {:<w$}", val, w = w - 1)
                                };

                                if c == state.sheet.cursor_col {
                                    ui.styled(
                                        &formatted,
                                        Style::new().bg(Color::Cyan).fg(Color::Black).bold(),
                                    );
                                } else {
                                    ui.styled(&formatted, Style::new().fg(Color::White).bold());
                                }
                                if c < state.sheet.total_cols() - 1 {
                                    ui.styled("\u{2502}", Style::new().fg(Color::Indexed(240)));
                                }
                            }
                        } else {
                            ui.styled(&line, Style::new().fg(Color::Indexed(250)));
                        }
                    });
                }
            });

            let _ = ui.separator();
            // status bar
            let _ = ui.row(|ui| {
                ui.text(format!(
                    "Cell {}{} | {} rows x {} cols",
                    col_letter(state.sheet.cursor_col),
                    state.sheet.cursor_row + 1,
                    state.sheet.total_rows(),
                    state.sheet.total_cols(),
                ))
                .dim();
                ui.spacer();
                if state.editing {
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
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::new();
    slt::run_with(
        slt::RunConfig::default().mouse(true),
        move |ui: &mut Context| {
            if ui.key_mod('q', slt::KeyModifiers::CONTROL)
                || (ui.key_code(slt::KeyCode::Esc) && !state.editing)
            {
                ui.quit();
            }
            render(ui, &mut state);
        },
    )
}

fn is_numeric(s: &str) -> bool {
    s.parse::<f64>().is_ok()
}
