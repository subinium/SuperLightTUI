//! Minimal crossterm mouse event test — isolates whether mouse events arrive.
//! Run: cargo run --example test_mouse
//! Then click/drag in the terminal. Press 'q' to quit.
//! If you see "[MOUSE] ..." lines, crossterm mouse capture works.
//! If you only see key events, mouse capture is broken.

use std::io::{self, Write};

use crossterm::{
    cursor,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEventKind,
    },
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
    execute!(stdout, EnableMouseCapture)?;
    stdout.flush()?;
    execute!(
        stdout,
        cursor::MoveTo(0, 0),
        crossterm::style::Print("Mouse test: click/drag anywhere. Press 'q' to quit."),
        cursor::MoveTo(0, 1),
        crossterm::style::Print("If mouse works, you'll see events below:"),
        cursor::MoveTo(0, 2),
        crossterm::style::Print("─────────────────────────────────────────"),
    )?;
    stdout.flush()?;

    let mut line = 3u16;

    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            let ev = event::read()?;
            match &ev {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    if k.code == KeyCode::Char('q') {
                        break;
                    }
                    execute!(
                        stdout,
                        cursor::MoveTo(0, line),
                        crossterm::style::Print(format!("[KEY] {:?}     ", k.code)),
                    )?;
                    line += 1;
                }
                Event::Mouse(m) => {
                    let kind_str = match m.kind {
                        MouseEventKind::Down(btn) => format!("Down({:?})", btn),
                        MouseEventKind::Up(btn) => format!("Up({:?})", btn),
                        MouseEventKind::Drag(btn) => format!("Drag({:?})", btn),
                        MouseEventKind::Moved => "Moved".into(),
                        MouseEventKind::ScrollUp => "ScrollUp".into(),
                        MouseEventKind::ScrollDown => "ScrollDown".into(),
                        _ => "Other".into(),
                    };
                    execute!(
                        stdout,
                        cursor::MoveTo(0, line),
                        crossterm::style::Print(format!(
                            "[MOUSE] {} at ({}, {})          ",
                            kind_str, m.column, m.row
                        )),
                    )?;
                    line += 1;
                }
                Event::Resize(w, h) => {
                    execute!(
                        stdout,
                        cursor::MoveTo(0, line),
                        crossterm::style::Print(format!("[RESIZE] {}x{}     ", w, h)),
                    )?;
                    line += 1;
                }
                _ => {}
            }
            stdout.flush()?;

            let (_, rows) = terminal::size()?;
            if line >= rows - 1 {
                line = 3;
            }
        }
    }

    execute!(
        stdout,
        DisableMouseCapture,
        LeaveAlternateScreen,
        cursor::Show
    )?;
    terminal::disable_raw_mode()?;

    Ok(())
}
