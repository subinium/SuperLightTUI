//! Real-terminal probe for the CI terminal jobs (issue #450).
//!
//! One deterministic screen, three modes:
//!
//! * no arguments — run the interactive probe (alternate screen, mouse on).
//!   `t` flips the per-row variant cell between `x` and `y` (an incremental,
//!   one-cell diff per row), a left click on `[ ] TARGET` toggles it, `q`
//!   quits with status 0. Every mouse-down is echoed as pane-relative,
//!   zero-based `down=(x,y)` so a driver can assert the coordinates SLT saw.
//! * `--expect COLS ROWS VARIANT TOGGLED HITS [X Y]` — render the same view
//!   through `TestBackend` and print the expected cell grid as JSON
//!   (`[[[x, symbol, buffer_width], ...], ...]`, one array per row). A driver
//!   compares a terminal capture against this instead of hand-written rows.
//! * `--measure OUT.json` — in raw mode, print every distinct non-ASCII
//!   symbol of the view and read the terminal's cursor advance back with a
//!   CPR (`ESC [ 6 n`) query. The JSON map `{symbol: columns}` is what the
//!   terminal really does with each grapheme, so geometry checks stay correct
//!   on terminals that render, for example, U+2764 U+FE0F as one column.
//!
//! With `SLT_PROBE_REPORT=PATH` the interactive probe also mirrors its state
//! (size, variant, toggle, hits, last mouse-down) into a one-line JSON file
//! for drivers that cannot read the screen back (xterm under Xvfb).
//!
//! Before entering the alternate screen the probe prints
//! `SLT-MAIN-SCREEN-MARKER` on the main screen, so drivers can prove the
//! alternate screen was entered (marker hidden) and left (marker back, probe
//! rows gone) without depending on how a pseudo-console forwards `?1049h`.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::time::Duration;

use slt::crossterm::{cursor, execute, terminal};
use slt::{Border, Color, Context, KeyCode, RunConfig, TestBackend};

/// Printed on the main screen before the alternate screen is entered.
const MAIN_SCREEN_MARKER: &str = "SLT-MAIN-SCREEN-MARKER";
/// When set, the interactive probe mirrors its state into this JSON file.
const REPORT_ENV: &str = "SLT_PROBE_REPORT";
/// Width of the clickable target in cells (`[ ] TARGET`).
const TARGET_WIDTH: u32 = 10;

#[derive(Clone, Debug, Default)]
struct ProbeState {
    /// `false` renders `x` in every variant cell, `true` renders `y`.
    variant: bool,
    toggled: bool,
    hits: u32,
    last_down: Option<(u32, u32)>,
}

fn view(ui: &mut Context, state: &mut ProbeState) {
    // Read the raw mouse-down before any widget can consume it, so the echo
    // reports exactly the coordinates the input parser produced.
    if let Some(position) = ui.mouse_down() {
        state.last_down = Some(position);
    }
    if ui.key('t') {
        state.variant = !state.variant;
    }
    if ui.key('q') || ui.key_code(KeyCode::Esc) {
        ui.quit();
    }

    let t = if state.variant { "y" } else { "x" };
    let (width, height) = (ui.width(), ui.height());
    let _ = ui.col(|ui| {
        ui.text(format!("SLT-PROBE {width}x{height} variant={t}"));
        let _ = ui.bordered(Border::Single).w(16).h(3).col(|ui| {
            ui.text(format!("BOX-{t}"));
        });
        ui.text(format!("CJK  |\u{D55C}\u{AE00}\u{6F22}\u{5B57}B{t}|"));
        ui.text(format!("VS16 |A\u{2764}\u{FE0F}B{t}|"));
        ui.text(format!(
            "ZWJ  |A\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}B{t}|"
        ));
        let _ = ui.row(|ui| {
            ui.text("STYLE ");
            ui.text("STYLED").bold().fg(Color::Red);
        });
        ui.text("");
        let _ = ui.row(|ui| {
            ui.text("HIT  ");
            let mark = if state.toggled { "x" } else { " " };
            let response = ui.container().w(TARGET_WIDTH).h(1).row(|ui| {
                ui.text(format!("[{mark}] TARGET"));
            });
            if response.clicked {
                state.toggled = !state.toggled;
                state.hits += 1;
            }
        });
        let down = match state.last_down {
            Some((x, y)) => format!("({x},{y})"),
            None => "none".to_string(),
        };
        ui.text(format!("MOUSE down={down} hits={}", state.hits));
        ui.text("keys: t=flip variant  q=quit");
    });
}

fn json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Render `state` headlessly and return, per row, `(x, symbol, buffer_width)`
/// for every non-continuation cell.
fn expected_cells(cols: u32, rows: u32, state: &ProbeState) -> Vec<Vec<(u32, String, u32)>> {
    let mut backend = TestBackend::new(cols, rows);
    let mut frame_state = state.clone();
    backend.render(|ui| view(ui, &mut frame_state));
    let buffer = backend.buffer();
    let mut grid = Vec::with_capacity(rows as usize);
    for y in 0..rows {
        let mut cells: Vec<(u32, String, u32)> = Vec::new();
        for x in 0..cols {
            let symbol = buffer.get(x, y).symbol.to_string();
            if symbol.is_empty() {
                // Continuation cell of the previous wide grapheme.
                if let Some(last) = cells.last_mut() {
                    last.2 += 1;
                }
                continue;
            }
            cells.push((x, symbol, 1));
        }
        grid.push(cells);
    }
    grid
}

fn parse_u32(value: Option<&String>, name: &str) -> io::Result<u32> {
    value
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("bad {name}")))
}

fn run_expect(args: &[String]) -> io::Result<()> {
    let cols = parse_u32(args.first(), "COLS")?;
    let rows = parse_u32(args.get(1), "ROWS")?;
    let state = ProbeState {
        variant: args.get(2).map(String::as_str) == Some("y"),
        toggled: args.get(3).map(String::as_str) == Some("1"),
        hits: parse_u32(args.get(4), "HITS")?,
        last_down: match (args.get(5), args.get(6)) {
            (Some(_), Some(_)) => {
                Some((parse_u32(args.get(5), "X")?, parse_u32(args.get(6), "Y")?))
            }
            _ => None,
        },
    };
    let mut out = String::from("[");
    for (row_index, row) in expected_cells(cols, rows, &state).iter().enumerate() {
        if row_index > 0 {
            out.push(',');
        }
        out.push('[');
        for (cell_index, (x, symbol, width)) in row.iter().enumerate() {
            if cell_index > 0 {
                out.push(',');
            }
            let _ = write!(out, "[{x},");
            json_string(&mut out, symbol);
            let _ = write!(out, ",{width}]");
        }
        out.push(']');
    }
    out.push_str("]\n");
    io::stdout().write_all(out.as_bytes())
}

fn run_measure(out_path: &str) -> io::Result<()> {
    let mut symbols = BTreeSet::new();
    for row in expected_cells(80, 24, &ProbeState::default()) {
        for (_, symbol, _) in row {
            if !symbol.is_ascii() {
                symbols.insert(symbol);
            }
        }
    }

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    let mut measured = Vec::new();
    let mut failure = None;
    for symbol in &symbols {
        let result = (|| -> io::Result<u16> {
            execute!(
                stdout,
                cursor::MoveTo(0, 0),
                terminal::Clear(terminal::ClearType::CurrentLine)
            )?;
            stdout.write_all(symbol.as_bytes())?;
            stdout.flush()?;
            let (column, _) = cursor::position()?;
            Ok(column)
        })();
        match result {
            Ok(column) => measured.push((symbol.clone(), column)),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    terminal::disable_raw_mode()?;
    if let Some(error) = failure {
        return Err(io::Error::other(format!("CPR width probe failed: {error}")));
    }

    let mut out = String::from("{");
    for (index, (symbol, column)) in measured.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        json_string(&mut out, symbol);
        let _ = write!(out, ":{column}");
    }
    out.push_str("}\n");
    // Write atomically so a polling driver never parses a partial file.
    let tmp = format!("{out_path}.tmp");
    std::fs::write(&tmp, out)?;
    std::fs::rename(&tmp, out_path)
}

fn run_interactive() -> io::Result<()> {
    println!("{MAIN_SCREEN_MARKER}");
    io::stdout().flush()?;
    let mut state = ProbeState::default();
    let report_path = std::env::var_os(REPORT_ENV).map(std::path::PathBuf::from);
    let mut last_report = String::new();
    let config = RunConfig::default()
        .mouse(true)
        .tick_rate(Duration::from_millis(50))
        .max_fps(30);
    slt::run_with(config, |ui| {
        view(ui, &mut state);
        if let Some(path) = &report_path {
            let report = state_report(ui.width(), ui.height(), &state);
            // Atomic replace so a polling driver never reads half a file; a
            // failed write is retried on the next frame.
            let tmp = path.with_extension("tmp");
            if report != last_report
                && std::fs::write(&tmp, &report)
                    .and_then(|()| std::fs::rename(&tmp, path))
                    .is_ok()
            {
                last_report = report;
            }
        }
    })
}

/// One-line JSON view of what the app received.
fn state_report(cols: u32, rows: u32, state: &ProbeState) -> String {
    let down = match state.last_down {
        Some((x, y)) => format!("[{x},{y}]"),
        None => "null".to_string(),
    };
    format!(
        "{{\"cols\":{cols},\"rows\":{rows},\"variant\":\"{}\",\"toggled\":{},\"hits\":{},\"down\":{down}}}\n",
        if state.variant { "y" } else { "x" },
        u8::from(state.toggled),
        state.hits,
    )
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--expect") => run_expect(&args[1..]),
        Some("--measure") => match args.get(1) {
            Some(path) => run_measure(path),
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--measure needs an output path",
            )),
        },
        Some(other) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown argument {other:?}"),
        )),
        None => run_interactive(),
    }
}
