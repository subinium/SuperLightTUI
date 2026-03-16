use std::io::{self, Stdout, Write};

use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture,
};
use crossterm::style::{
    Attribute, Color as CtColor, Print, ResetColor, SetAttribute, SetBackgroundColor,
    SetForegroundColor,
};
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use crossterm::{cursor, execute, queue, terminal};

use unicode_width::UnicodeWidthStr;

use crate::buffer::Buffer;
use crate::rect::Rect;
use crate::style::{Color, ColorDepth, Modifiers, Style};

pub(crate) struct Terminal {
    stdout: Stdout,
    current: Buffer,
    previous: Buffer,
    mouse_enabled: bool,
    cursor_visible: bool,
    kitty_keyboard: bool,
    color_depth: ColorDepth,
    pub(crate) theme_bg: Option<Color>,
}

pub(crate) struct InlineTerminal {
    stdout: Stdout,
    current: Buffer,
    previous: Buffer,
    mouse_enabled: bool,
    cursor_visible: bool,
    height: u32,
    start_row: u16,
    reserved: bool,
    color_depth: ColorDepth,
    pub(crate) theme_bg: Option<Color>,
}

impl Terminal {
    pub fn new(mouse: bool, kitty_keyboard: bool, color_depth: ColorDepth) -> io::Result<Self> {
        let (cols, rows) = terminal::size()?;
        let area = Rect::new(0, 0, cols as u32, rows as u32);

        let mut stdout = io::stdout();
        terminal::enable_raw_mode()?;
        execute!(
            stdout,
            terminal::EnterAlternateScreen,
            cursor::Hide,
            EnableBracketedPaste
        )?;
        if mouse {
            execute!(stdout, EnableMouseCapture, EnableFocusChange)?;
        }
        if kitty_keyboard {
            use crossterm::event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
            let _ = execute!(
                stdout,
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                )
            );
        }

        Ok(Self {
            stdout,
            current: Buffer::empty(area),
            previous: Buffer::empty(area),
            mouse_enabled: mouse,
            cursor_visible: false,
            kitty_keyboard,
            color_depth,
            theme_bg: None,
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

        let mut last_style = Style::new();
        let mut first_style = true;
        let mut last_pos: Option<(u32, u32)> = None;
        let mut active_link: Option<&str> = None;
        let mut has_updates = false;

        for y in self.current.area.y..self.current.area.bottom() {
            for x in self.current.area.x..self.current.area.right() {
                let cur = self.current.get(x, y);
                let prev = self.previous.get(x, y);
                if cur == prev {
                    continue;
                }
                if cur.symbol.is_empty() {
                    continue;
                }
                has_updates = true;

                let need_move = last_pos.map_or(true, |(lx, ly)| ly != y || lx != x);
                if need_move {
                    queue!(self.stdout, cursor::MoveTo(x as u16, y as u16))?;
                }

                if cur.style != last_style {
                    if first_style {
                        queue!(self.stdout, ResetColor, SetAttribute(Attribute::Reset))?;
                        apply_style(&mut self.stdout, &cur.style, self.color_depth)?;
                        first_style = false;
                    } else {
                        apply_style_delta(
                            &mut self.stdout,
                            &last_style,
                            &cur.style,
                            self.color_depth,
                        )?;
                    }
                    last_style = cur.style;
                }

                let cell_link = cur.hyperlink.as_deref();
                if cell_link != active_link {
                    if let Some(url) = cell_link {
                        queue!(self.stdout, Print(format!("\x1b]8;;{url}\x07")))?;
                    } else {
                        queue!(self.stdout, Print("\x1b]8;;\x07"))?;
                    }
                    active_link = cell_link;
                }

                queue!(self.stdout, Print(&*cur.symbol))?;
                let char_width = UnicodeWidthStr::width(cur.symbol.as_str()).max(1) as u32;
                last_pos = Some((x + char_width, y));
            }
        }

        if has_updates {
            if active_link.is_some() {
                queue!(self.stdout, Print("\x1b]8;;\x07"))?;
            }
            queue!(self.stdout, ResetColor, SetAttribute(Attribute::Reset))?;
        }

        queue!(self.stdout, EndSynchronizedUpdate)?;

        let cursor_pos = find_cursor_marker(&self.current);
        match cursor_pos {
            Some((cx, cy)) => {
                if !self.cursor_visible {
                    queue!(self.stdout, cursor::Show)?;
                    self.cursor_visible = true;
                }
                queue!(self.stdout, cursor::MoveTo(cx as u16, cy as u16))?;
            }
            None => {
                if self.cursor_visible {
                    queue!(self.stdout, cursor::Hide)?;
                    self.cursor_visible = false;
                }
            }
        }

        self.stdout.flush()?;

        std::mem::swap(&mut self.current, &mut self.previous);
        if let Some(bg) = self.theme_bg {
            self.current.reset_with_bg(bg);
        } else {
            self.current.reset();
        }
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

impl crate::Backend for Terminal {
    fn size(&self) -> (u32, u32) {
        Terminal::size(self)
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        Terminal::buffer_mut(self)
    }

    fn flush(&mut self) -> io::Result<()> {
        Terminal::flush(self)
    }
}

impl InlineTerminal {
    pub fn new(height: u32, mouse: bool, color_depth: ColorDepth) -> io::Result<Self> {
        let (cols, _) = terminal::size()?;
        let area = Rect::new(0, 0, cols as u32, height);

        let mut stdout = io::stdout();
        terminal::enable_raw_mode()?;
        execute!(stdout, cursor::Hide, EnableBracketedPaste)?;
        if mouse {
            execute!(stdout, EnableMouseCapture, EnableFocusChange)?;
        }

        let (_, cursor_row) = cursor::position()?;
        Ok(Self {
            stdout,
            current: Buffer::empty(area),
            previous: Buffer::empty(area),
            mouse_enabled: mouse,
            cursor_visible: false,
            height,
            start_row: cursor_row,
            reserved: false,
            color_depth,
            theme_bg: None,
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
            let mut first_style = true;
            let mut last_pos: Option<(u32, u32)> = None;
            let mut active_link: Option<&str> = None;

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
                    if first_style {
                        queue!(self.stdout, ResetColor, SetAttribute(Attribute::Reset))?;
                        apply_style(&mut self.stdout, &cell.style, self.color_depth)?;
                        first_style = false;
                    } else {
                        apply_style_delta(
                            &mut self.stdout,
                            &last_style,
                            &cell.style,
                            self.color_depth,
                        )?;
                    }
                    last_style = cell.style;
                }

                let cell_link = cell.hyperlink.as_deref();
                if cell_link != active_link {
                    if let Some(url) = cell_link {
                        queue!(self.stdout, Print(format!("\x1b]8;;{url}\x07")))?;
                    } else {
                        queue!(self.stdout, Print("\x1b]8;;\x07"))?;
                    }
                    active_link = cell_link;
                }

                queue!(self.stdout, Print(&cell.symbol))?;
                let char_width = UnicodeWidthStr::width(cell.symbol.as_str()).max(1) as u32;
                last_pos = Some((x + char_width, abs_y));
            }

            if active_link.is_some() {
                queue!(self.stdout, Print("\x1b]8;;\x07"))?;
            }
            queue!(self.stdout, ResetColor, SetAttribute(Attribute::Reset))?;
        }

        queue!(self.stdout, EndSynchronizedUpdate)?;

        let cursor_pos = find_cursor_marker(&self.current);
        match cursor_pos {
            Some((cx, cy)) => {
                let abs_cy = self.start_row as u32 + cy;
                if !self.cursor_visible {
                    queue!(self.stdout, cursor::Show)?;
                    self.cursor_visible = true;
                }
                queue!(self.stdout, cursor::MoveTo(cx as u16, abs_cy as u16))?;
            }
            None => {
                if self.cursor_visible {
                    queue!(self.stdout, cursor::Hide)?;
                    self.cursor_visible = false;
                }
                let end_row = self.start_row + self.height.saturating_sub(1) as u16;
                queue!(self.stdout, cursor::MoveTo(0, end_row))?;
            }
        }

        self.stdout.flush()?;

        std::mem::swap(&mut self.current, &mut self.previous);
        reset_current_buffer(&mut self.current, self.theme_bg);
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

impl crate::Backend for InlineTerminal {
    fn size(&self) -> (u32, u32) {
        InlineTerminal::size(self)
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        InlineTerminal::buffer_mut(self)
    }

    fn flush(&mut self) -> io::Result<()> {
        InlineTerminal::flush(self)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if self.kitty_keyboard {
            use crossterm::event::PopKeyboardEnhancementFlags;
            let _ = execute!(self.stdout, PopKeyboardEnhancementFlags);
        }
        if self.mouse_enabled {
            let _ = execute!(self.stdout, DisableMouseCapture, DisableFocusChange);
        }
        let _ = execute!(
            self.stdout,
            ResetColor,
            SetAttribute(Attribute::Reset),
            cursor::Show,
            DisableBracketedPaste,
            terminal::LeaveAlternateScreen
        );
        let _ = terminal::disable_raw_mode();
    }
}

impl Drop for InlineTerminal {
    fn drop(&mut self) {
        if self.mouse_enabled {
            let _ = execute!(self.stdout, DisableMouseCapture, DisableFocusChange);
        }
        let _ = execute!(
            self.stdout,
            ResetColor,
            SetAttribute(Attribute::Reset),
            cursor::Show,
            DisableBracketedPaste
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

mod selection;
pub(crate) use selection::{apply_selection_overlay, extract_selection_text, SelectionState};
#[cfg(test)]
pub(crate) use selection::{find_innermost_rect, normalize_selection};

fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            CHARS[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            CHARS[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

pub(crate) fn copy_to_clipboard(w: &mut impl Write, text: &str) -> io::Result<()> {
    let encoded = base64_encode(text.as_bytes());
    write!(w, "\x1b]52;c;{encoded}\x1b\\")?;
    w.flush()
}

// ── Cursor marker ───────────────────────────────────────────────

const CURSOR_MARKER: &str = "▎";

fn find_cursor_marker(buffer: &Buffer) -> Option<(u32, u32)> {
    let area = buffer.area;
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if buffer.get(x, y).symbol == CURSOR_MARKER {
                return Some((x, y));
            }
        }
    }
    None
}

fn apply_style_delta(
    w: &mut impl Write,
    old: &Style,
    new: &Style,
    depth: ColorDepth,
) -> io::Result<()> {
    if old.fg != new.fg {
        match new.fg {
            Some(fg) => queue!(w, SetForegroundColor(to_crossterm_color(fg, depth)))?,
            None => queue!(w, SetForegroundColor(CtColor::Reset))?,
        }
    }
    if old.bg != new.bg {
        match new.bg {
            Some(bg) => queue!(w, SetBackgroundColor(to_crossterm_color(bg, depth)))?,
            None => queue!(w, SetBackgroundColor(CtColor::Reset))?,
        }
    }
    let removed = Modifiers(old.modifiers.0 & !new.modifiers.0);
    let added = Modifiers(new.modifiers.0 & !old.modifiers.0);
    if removed.contains(Modifiers::BOLD) || removed.contains(Modifiers::DIM) {
        queue!(w, SetAttribute(Attribute::NormalIntensity))?;
        if new.modifiers.contains(Modifiers::BOLD) {
            queue!(w, SetAttribute(Attribute::Bold))?;
        }
        if new.modifiers.contains(Modifiers::DIM) {
            queue!(w, SetAttribute(Attribute::Dim))?;
        }
    } else {
        if added.contains(Modifiers::BOLD) {
            queue!(w, SetAttribute(Attribute::Bold))?;
        }
        if added.contains(Modifiers::DIM) {
            queue!(w, SetAttribute(Attribute::Dim))?;
        }
    }
    if removed.contains(Modifiers::ITALIC) {
        queue!(w, SetAttribute(Attribute::NoItalic))?;
    }
    if added.contains(Modifiers::ITALIC) {
        queue!(w, SetAttribute(Attribute::Italic))?;
    }
    if removed.contains(Modifiers::UNDERLINE) {
        queue!(w, SetAttribute(Attribute::NoUnderline))?;
    }
    if added.contains(Modifiers::UNDERLINE) {
        queue!(w, SetAttribute(Attribute::Underlined))?;
    }
    if removed.contains(Modifiers::REVERSED) {
        queue!(w, SetAttribute(Attribute::NoReverse))?;
    }
    if added.contains(Modifiers::REVERSED) {
        queue!(w, SetAttribute(Attribute::Reverse))?;
    }
    if removed.contains(Modifiers::STRIKETHROUGH) {
        queue!(w, SetAttribute(Attribute::NotCrossedOut))?;
    }
    if added.contains(Modifiers::STRIKETHROUGH) {
        queue!(w, SetAttribute(Attribute::CrossedOut))?;
    }
    Ok(())
}

fn apply_style(w: &mut impl Write, style: &Style, depth: ColorDepth) -> io::Result<()> {
    if let Some(fg) = style.fg {
        queue!(w, SetForegroundColor(to_crossterm_color(fg, depth)))?;
    }
    if let Some(bg) = style.bg {
        queue!(w, SetBackgroundColor(to_crossterm_color(bg, depth)))?;
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

fn to_crossterm_color(color: Color, depth: ColorDepth) -> CtColor {
    let color = color.downsampled(depth);
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

fn reset_current_buffer(buffer: &mut Buffer, theme_bg: Option<Color>) {
    if let Some(bg) = theme_bg {
        buffer.reset_with_bg(bg);
    } else {
        buffer.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_current_buffer_applies_theme_background() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));

        reset_current_buffer(&mut buffer, Some(Color::Rgb(10, 20, 30)));
        assert_eq!(buffer.get(0, 0).style.bg, Some(Color::Rgb(10, 20, 30)));

        reset_current_buffer(&mut buffer, None);
        assert_eq!(buffer.get(0, 0).style.bg, None);
    }

    #[test]
    fn base64_encode_empty() {
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn base64_encode_hello() {
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
    }

    #[test]
    fn base64_encode_padding() {
        assert_eq!(base64_encode(b"a"), "YQ==");
        assert_eq!(base64_encode(b"ab"), "YWI=");
        assert_eq!(base64_encode(b"abc"), "YWJj");
    }

    #[test]
    fn base64_encode_unicode() {
        assert_eq!(base64_encode("한글".as_bytes()), "7ZWc6riA");
    }

    fn pair(r: Rect) -> (Rect, Rect) {
        (r, r)
    }

    #[test]
    fn find_innermost_rect_picks_smallest() {
        let rects = vec![
            pair(Rect::new(0, 0, 80, 24)),
            pair(Rect::new(5, 2, 30, 10)),
            pair(Rect::new(10, 4, 10, 5)),
        ];
        let result = find_innermost_rect(&rects, 12, 5);
        assert_eq!(result, Some(Rect::new(10, 4, 10, 5)));
    }

    #[test]
    fn find_innermost_rect_no_match() {
        let rects = vec![pair(Rect::new(10, 10, 5, 5))];
        assert_eq!(find_innermost_rect(&rects, 0, 0), None);
    }

    #[test]
    fn find_innermost_rect_empty() {
        assert_eq!(find_innermost_rect(&[], 5, 5), None);
    }

    #[test]
    fn find_innermost_rect_returns_content_rect() {
        let rects = vec![
            (Rect::new(0, 0, 80, 24), Rect::new(1, 1, 78, 22)),
            (Rect::new(5, 2, 30, 10), Rect::new(6, 3, 28, 8)),
        ];
        let result = find_innermost_rect(&rects, 10, 5);
        assert_eq!(result, Some(Rect::new(6, 3, 28, 8)));
    }

    #[test]
    fn normalize_selection_already_ordered() {
        let (s, e) = normalize_selection((2, 1), (5, 3));
        assert_eq!(s, (2, 1));
        assert_eq!(e, (5, 3));
    }

    #[test]
    fn normalize_selection_reversed() {
        let (s, e) = normalize_selection((5, 3), (2, 1));
        assert_eq!(s, (2, 1));
        assert_eq!(e, (5, 3));
    }

    #[test]
    fn normalize_selection_same_row() {
        let (s, e) = normalize_selection((10, 5), (3, 5));
        assert_eq!(s, (3, 5));
        assert_eq!(e, (10, 5));
    }

    #[test]
    fn selection_state_mouse_down_finds_rect() {
        let hit_map = vec![pair(Rect::new(0, 0, 80, 24)), pair(Rect::new(5, 2, 20, 10))];
        let mut sel = SelectionState::default();
        sel.mouse_down(10, 5, &hit_map);
        assert_eq!(sel.anchor, Some((10, 5)));
        assert_eq!(sel.current, Some((10, 5)));
        assert_eq!(sel.widget_rect, Some(Rect::new(5, 2, 20, 10)));
        assert!(!sel.active);
    }

    #[test]
    fn selection_state_drag_activates() {
        let hit_map = vec![pair(Rect::new(0, 0, 80, 24))];
        let mut sel = SelectionState {
            anchor: Some((10, 5)),
            current: Some((10, 5)),
            widget_rect: Some(Rect::new(0, 0, 80, 24)),
            ..Default::default()
        };
        sel.mouse_drag(10, 5, &hit_map);
        assert!(!sel.active, "no movement = not active");
        sel.mouse_drag(11, 5, &hit_map);
        assert!(!sel.active, "1 cell horizontal = not active yet");
        sel.mouse_drag(13, 5, &hit_map);
        assert!(sel.active, ">1 cell horizontal = active");
    }

    #[test]
    fn selection_state_drag_vertical_activates() {
        let hit_map = vec![pair(Rect::new(0, 0, 80, 24))];
        let mut sel = SelectionState {
            anchor: Some((10, 5)),
            current: Some((10, 5)),
            widget_rect: Some(Rect::new(0, 0, 80, 24)),
            ..Default::default()
        };
        sel.mouse_drag(10, 6, &hit_map);
        assert!(sel.active, "any vertical movement = active");
    }

    #[test]
    fn selection_state_drag_expands_widget_rect() {
        let hit_map = vec![
            pair(Rect::new(0, 0, 80, 24)),
            pair(Rect::new(5, 2, 30, 10)),
            pair(Rect::new(5, 2, 30, 3)),
        ];
        let mut sel = SelectionState {
            anchor: Some((10, 3)),
            current: Some((10, 3)),
            widget_rect: Some(Rect::new(5, 2, 30, 3)),
            ..Default::default()
        };
        sel.mouse_drag(10, 6, &hit_map);
        assert_eq!(sel.widget_rect, Some(Rect::new(5, 2, 30, 10)));
    }

    #[test]
    fn selection_state_clear_resets() {
        let mut sel = SelectionState {
            anchor: Some((1, 2)),
            current: Some((3, 4)),
            widget_rect: Some(Rect::new(0, 0, 10, 10)),
            active: true,
        };
        sel.clear();
        assert_eq!(sel.anchor, None);
        assert_eq!(sel.current, None);
        assert_eq!(sel.widget_rect, None);
        assert!(!sel.active);
    }

    #[test]
    fn extract_selection_text_single_line() {
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        buf.set_string(0, 0, "Hello World", Style::default());
        let sel = SelectionState {
            anchor: Some((0, 0)),
            current: Some((4, 0)),
            widget_rect: Some(area),
            active: true,
        };
        let text = extract_selection_text(&buf, &sel, &[]);
        assert_eq!(text, "Hello");
    }

    #[test]
    fn extract_selection_text_multi_line() {
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        buf.set_string(0, 0, "Line one", Style::default());
        buf.set_string(0, 1, "Line two", Style::default());
        buf.set_string(0, 2, "Line three", Style::default());
        let sel = SelectionState {
            anchor: Some((5, 0)),
            current: Some((3, 2)),
            widget_rect: Some(area),
            active: true,
        };
        let text = extract_selection_text(&buf, &sel, &[]);
        assert_eq!(text, "one\nLine two\nLine");
    }

    #[test]
    fn extract_selection_text_clamped_to_widget() {
        let area = Rect::new(0, 0, 40, 10);
        let widget = Rect::new(5, 2, 10, 3);
        let mut buf = Buffer::empty(area);
        buf.set_string(5, 2, "ABCDEFGHIJ", Style::default());
        buf.set_string(5, 3, "KLMNOPQRST", Style::default());
        let sel = SelectionState {
            anchor: Some((3, 1)),
            current: Some((20, 5)),
            widget_rect: Some(widget),
            active: true,
        };
        let text = extract_selection_text(&buf, &sel, &[]);
        assert_eq!(text, "ABCDEFGHIJ\nKLMNOPQRST");
    }

    #[test]
    fn extract_selection_text_inactive_returns_empty() {
        let area = Rect::new(0, 0, 10, 5);
        let buf = Buffer::empty(area);
        let sel = SelectionState {
            anchor: Some((0, 0)),
            current: Some((5, 2)),
            widget_rect: Some(area),
            active: false,
        };
        assert_eq!(extract_selection_text(&buf, &sel, &[]), "");
    }

    #[test]
    fn apply_selection_overlay_reverses_cells() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        buf.set_string(0, 0, "ABCDE", Style::default());
        let sel = SelectionState {
            anchor: Some((1, 0)),
            current: Some((3, 0)),
            widget_rect: Some(area),
            active: true,
        };
        apply_selection_overlay(&mut buf, &sel, &[]);
        assert!(!buf.get(0, 0).style.modifiers.contains(Modifiers::REVERSED));
        assert!(buf.get(1, 0).style.modifiers.contains(Modifiers::REVERSED));
        assert!(buf.get(2, 0).style.modifiers.contains(Modifiers::REVERSED));
        assert!(buf.get(3, 0).style.modifiers.contains(Modifiers::REVERSED));
        assert!(!buf.get(4, 0).style.modifiers.contains(Modifiers::REVERSED));
    }

    #[test]
    fn extract_selection_text_skips_border_cells() {
        // Simulate two bordered columns side by side:
        // Col1: full=(0,0,20,5) content=(1,1,18,3)
        // Col2: full=(20,0,20,5) content=(21,1,18,3)
        // Parent widget_rect covers both: (0,0,40,5)
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        // Col1 border characters
        buf.set_string(0, 0, "╭", Style::default());
        buf.set_string(0, 1, "│", Style::default());
        buf.set_string(0, 2, "│", Style::default());
        buf.set_string(0, 3, "│", Style::default());
        buf.set_string(0, 4, "╰", Style::default());
        buf.set_string(19, 0, "╮", Style::default());
        buf.set_string(19, 1, "│", Style::default());
        buf.set_string(19, 2, "│", Style::default());
        buf.set_string(19, 3, "│", Style::default());
        buf.set_string(19, 4, "╯", Style::default());
        // Col2 border characters
        buf.set_string(20, 0, "╭", Style::default());
        buf.set_string(20, 1, "│", Style::default());
        buf.set_string(20, 2, "│", Style::default());
        buf.set_string(20, 3, "│", Style::default());
        buf.set_string(20, 4, "╰", Style::default());
        buf.set_string(39, 0, "╮", Style::default());
        buf.set_string(39, 1, "│", Style::default());
        buf.set_string(39, 2, "│", Style::default());
        buf.set_string(39, 3, "│", Style::default());
        buf.set_string(39, 4, "╯", Style::default());
        // Content inside Col1
        buf.set_string(1, 1, "Hello Col1", Style::default());
        buf.set_string(1, 2, "Line2 Col1", Style::default());
        // Content inside Col2
        buf.set_string(21, 1, "Hello Col2", Style::default());
        buf.set_string(21, 2, "Line2 Col2", Style::default());

        let content_map = vec![
            (Rect::new(0, 0, 20, 5), Rect::new(1, 1, 18, 3)),
            (Rect::new(20, 0, 20, 5), Rect::new(21, 1, 18, 3)),
        ];

        // Select across both columns, rows 1-2
        let sel = SelectionState {
            anchor: Some((0, 1)),
            current: Some((39, 2)),
            widget_rect: Some(area),
            active: true,
        };
        let text = extract_selection_text(&buf, &sel, &content_map);
        // Should NOT contain border characters (│, ╭, ╮, etc.)
        assert!(!text.contains('│'), "Border char │ found in: {text}");
        assert!(!text.contains('╭'), "Border char ╭ found in: {text}");
        assert!(!text.contains('╮'), "Border char ╮ found in: {text}");
        // Should contain actual content
        assert!(
            text.contains("Hello Col1"),
            "Missing Col1 content in: {text}"
        );
        assert!(
            text.contains("Hello Col2"),
            "Missing Col2 content in: {text}"
        );
        assert!(text.contains("Line2 Col1"), "Missing Col1 line2 in: {text}");
        assert!(text.contains("Line2 Col2"), "Missing Col2 line2 in: {text}");
    }

    #[test]
    fn apply_selection_overlay_skips_border_cells() {
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);
        buf.set_string(0, 0, "│", Style::default());
        buf.set_string(1, 0, "ABC", Style::default());
        buf.set_string(19, 0, "│", Style::default());

        let content_map = vec![(Rect::new(0, 0, 20, 3), Rect::new(1, 0, 18, 3))];
        let sel = SelectionState {
            anchor: Some((0, 0)),
            current: Some((19, 0)),
            widget_rect: Some(area),
            active: true,
        };
        apply_selection_overlay(&mut buf, &sel, &content_map);
        // Border cells at x=0 and x=19 should NOT be reversed
        assert!(
            !buf.get(0, 0).style.modifiers.contains(Modifiers::REVERSED),
            "Left border cell should not be reversed"
        );
        assert!(
            !buf.get(19, 0).style.modifiers.contains(Modifiers::REVERSED),
            "Right border cell should not be reversed"
        );
        // Content cells should be reversed
        assert!(buf.get(1, 0).style.modifiers.contains(Modifiers::REVERSED));
        assert!(buf.get(2, 0).style.modifiers.contains(Modifiers::REVERSED));
        assert!(buf.get(3, 0).style.modifiers.contains(Modifiers::REVERSED));
    }

    #[test]
    fn copy_to_clipboard_writes_osc52() {
        let mut output: Vec<u8> = Vec::new();
        copy_to_clipboard(&mut output, "test").unwrap();
        let s = String::from_utf8(output).unwrap();
        assert!(s.starts_with("\x1b]52;c;"));
        assert!(s.ends_with("\x1b\\"));
        assert!(s.contains(&base64_encode(b"test")));
    }
}
