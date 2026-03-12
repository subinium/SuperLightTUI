use std::io::{self, Stdout, Write};

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::style::{
    Attribute, Color as CtColor, Print, ResetColor, SetAttribute, SetBackgroundColor,
    SetForegroundColor,
};
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use crossterm::{cursor, execute, queue, terminal};

use unicode_width::UnicodeWidthStr;

use crate::buffer::Buffer;
use crate::rect::Rect;
use crate::style::{Color, Modifiers, Style};

pub(crate) struct Terminal {
    stdout: Stdout,
    current: Buffer,
    previous: Buffer,
    mouse_enabled: bool,
}

pub(crate) struct InlineTerminal {
    stdout: Stdout,
    current: Buffer,
    previous: Buffer,
    mouse_enabled: bool,
    height: u32,
    start_row: u16,
    reserved: bool,
}

impl Terminal {
    pub fn new(mouse: bool) -> io::Result<Self> {
        let (cols, rows) = terminal::size()?;
        let area = Rect::new(0, 0, cols as u32, rows as u32);

        let mut stdout = io::stdout();
        terminal::enable_raw_mode()?;
        execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
        if mouse {
            execute!(stdout, EnableMouseCapture)?;
        }

        Ok(Self {
            stdout,
            current: Buffer::empty(area),
            previous: Buffer::empty(area),
            mouse_enabled: mouse,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.current.area.width, self.current.area.height)
    }

    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.current
    }

    pub fn flush(&mut self) -> io::Result<()> {
        let updates = self.current.diff(&self.previous);
        queue!(self.stdout, BeginSynchronizedUpdate)?;

        if !updates.is_empty() {
            let mut last_style = Style::new();
            let mut last_pos: Option<(u32, u32)> = None;

            for &(x, y, cell) in &updates {
                if cell.symbol.is_empty() {
                    continue;
                }

                let need_move = last_pos.map_or(true, |(lx, ly)| ly != y || lx != x);
                if need_move {
                    queue!(self.stdout, cursor::MoveTo(x as u16, y as u16))?;
                }

                if cell.style != last_style {
                    queue!(self.stdout, ResetColor, SetAttribute(Attribute::Reset))?;
                    apply_style(&mut self.stdout, &cell.style)?;
                    last_style = cell.style;
                }

                queue!(self.stdout, Print(&cell.symbol))?;
                let char_width = UnicodeWidthStr::width(cell.symbol.as_str()).max(1) as u32;
                last_pos = Some((x + char_width, y));
            }

            queue!(self.stdout, ResetColor, SetAttribute(Attribute::Reset))?;
        }

        queue!(self.stdout, EndSynchronizedUpdate)?;
        self.stdout.flush()?;

        std::mem::swap(&mut self.current, &mut self.previous);
        self.current.reset();
        Ok(())
    }

    pub fn handle_resize(&mut self) -> io::Result<()> {
        let (cols, rows) = terminal::size()?;
        let area = Rect::new(0, 0, cols as u32, rows as u32);
        self.current.resize(area);
        self.previous.resize(area);
        execute!(
            self.stdout,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
        Ok(())
    }
}

impl InlineTerminal {
    pub fn new(height: u32, mouse: bool) -> io::Result<Self> {
        let (cols, _) = terminal::size()?;
        let area = Rect::new(0, 0, cols as u32, height);

        let mut stdout = io::stdout();
        terminal::enable_raw_mode()?;
        execute!(stdout, cursor::Hide)?;
        if mouse {
            execute!(stdout, EnableMouseCapture)?;
        }

        let (_, cursor_row) = cursor::position()?;
        Ok(Self {
            stdout,
            current: Buffer::empty(area),
            previous: Buffer::empty(area),
            mouse_enabled: mouse,
            height,
            start_row: cursor_row,
            reserved: false,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.current.area.width, self.current.area.height)
    }

    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.current
    }

    pub fn flush(&mut self) -> io::Result<()> {
        queue!(self.stdout, BeginSynchronizedUpdate)?;

        if !self.reserved {
            queue!(self.stdout, cursor::MoveToColumn(0))?;
            for _ in 0..self.height {
                queue!(self.stdout, Print("\n"))?;
            }
            self.reserved = true;

            let (_, rows) = terminal::size()?;
            let bottom = self.start_row + self.height as u16;
            if bottom > rows {
                self.start_row = rows.saturating_sub(self.height as u16);
            }
        }

        let updates = self.current.diff(&self.previous);
        if !updates.is_empty() {
            let mut last_style = Style::new();
            let mut last_pos: Option<(u32, u32)> = None;

            for &(x, y, cell) in &updates {
                if cell.symbol.is_empty() {
                    continue;
                }

                let abs_y = self.start_row as u32 + y;
                let need_move = last_pos.map_or(true, |(lx, ly)| ly != abs_y || lx != x);
                if need_move {
                    queue!(self.stdout, cursor::MoveTo(x as u16, abs_y as u16))?;
                }

                if cell.style != last_style {
                    queue!(self.stdout, ResetColor, SetAttribute(Attribute::Reset))?;
                    apply_style(&mut self.stdout, &cell.style)?;
                    last_style = cell.style;
                }

                queue!(self.stdout, Print(&cell.symbol))?;
                let char_width = UnicodeWidthStr::width(cell.symbol.as_str()).max(1) as u32;
                last_pos = Some((x + char_width, abs_y));
            }

            queue!(self.stdout, ResetColor, SetAttribute(Attribute::Reset))?;
        }

        let end_row = self.start_row + self.height.saturating_sub(1) as u16;
        queue!(self.stdout, cursor::MoveTo(0, end_row))?;
        queue!(self.stdout, EndSynchronizedUpdate)?;
        self.stdout.flush()?;

        std::mem::swap(&mut self.current, &mut self.previous);
        self.current.reset();
        Ok(())
    }

    pub fn handle_resize(&mut self) -> io::Result<()> {
        let (cols, _) = terminal::size()?;
        let area = Rect::new(0, 0, cols as u32, self.height);
        self.current.resize(area);
        self.previous.resize(area);
        execute!(
            self.stdout,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        )?;
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if self.mouse_enabled {
            let _ = execute!(self.stdout, DisableMouseCapture);
        }
        let _ = execute!(
            self.stdout,
            ResetColor,
            SetAttribute(Attribute::Reset),
            cursor::Show,
            terminal::LeaveAlternateScreen
        );
        let _ = terminal::disable_raw_mode();
    }
}

impl Drop for InlineTerminal {
    fn drop(&mut self) {
        if self.mouse_enabled {
            let _ = execute!(self.stdout, DisableMouseCapture);
        }
        let _ = execute!(
            self.stdout,
            ResetColor,
            SetAttribute(Attribute::Reset),
            cursor::Show
        );
        if self.reserved {
            let _ = execute!(
                self.stdout,
                cursor::MoveToColumn(0),
                cursor::MoveDown(1),
                cursor::MoveToColumn(0),
                Print("\n")
            );
        } else {
            let _ = execute!(self.stdout, Print("\n"));
        }
        let _ = terminal::disable_raw_mode();
    }
}

fn apply_style(w: &mut impl Write, style: &Style) -> io::Result<()> {
    if let Some(fg) = style.fg {
        queue!(w, SetForegroundColor(to_crossterm_color(fg)))?;
    }
    if let Some(bg) = style.bg {
        queue!(w, SetBackgroundColor(to_crossterm_color(bg)))?;
    }
    let m = style.modifiers;
    if m.contains(Modifiers::BOLD) {
        queue!(w, SetAttribute(Attribute::Bold))?;
    }
    if m.contains(Modifiers::DIM) {
        queue!(w, SetAttribute(Attribute::Dim))?;
    }
    if m.contains(Modifiers::ITALIC) {
        queue!(w, SetAttribute(Attribute::Italic))?;
    }
    if m.contains(Modifiers::UNDERLINE) {
        queue!(w, SetAttribute(Attribute::Underlined))?;
    }
    if m.contains(Modifiers::REVERSED) {
        queue!(w, SetAttribute(Attribute::Reverse))?;
    }
    if m.contains(Modifiers::STRIKETHROUGH) {
        queue!(w, SetAttribute(Attribute::CrossedOut))?;
    }
    Ok(())
}

fn to_crossterm_color(color: Color) -> CtColor {
    match color {
        Color::Reset => CtColor::Reset,
        Color::Black => CtColor::Black,
        Color::Red => CtColor::DarkRed,
        Color::Green => CtColor::DarkGreen,
        Color::Yellow => CtColor::DarkYellow,
        Color::Blue => CtColor::DarkBlue,
        Color::Magenta => CtColor::DarkMagenta,
        Color::Cyan => CtColor::DarkCyan,
        Color::White => CtColor::White,
        Color::Rgb(r, g, b) => CtColor::Rgb { r, g, b },
        Color::Indexed(i) => CtColor::AnsiValue(i),
    }
}
