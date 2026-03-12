use crate::buffer::Buffer;
use crate::context::Context;
use crate::event::Event;
use crate::layout;
use crate::rect::Rect;
use crate::style::Theme;

pub struct TestBackend {
    buffer: Buffer,
    width: u32,
    height: u32,
}

impl TestBackend {
    pub fn new(width: u32, height: u32) -> Self {
        let area = Rect::new(0, 0, width, height);
        Self {
            buffer: Buffer::empty(area),
            width,
            height,
        }
    }

    /// Run a closure as if it were one frame, render to internal buffer
    pub fn render(&mut self, f: impl FnOnce(&mut Context)) {
        let mut ctx = Context::new(
            Vec::new(),
            self.width,
            self.height,
            0,
            0,
            0,
            Vec::new(),
            Vec::new(),
            false,
            Theme::dark(),
            None,
        );
        f(&mut ctx);
        let mut tree = layout::build_tree(&ctx.commands);
        let area = Rect::new(0, 0, self.width, self.height);
        layout::compute(&mut tree, area);
        self.buffer.reset();
        layout::render(&tree, &mut self.buffer);
    }

    /// Render with specific events (for testing keyboard/mouse interaction)
    pub fn render_with_events(
        &mut self,
        events: Vec<Event>,
        focus_index: usize,
        prev_focus_count: usize,
        f: impl FnOnce(&mut Context),
    ) {
        let mut ctx = Context::new(
            events,
            self.width,
            self.height,
            0,
            focus_index,
            prev_focus_count,
            Vec::new(),
            Vec::new(),
            false,
            Theme::dark(),
            None,
        );
        ctx.process_focus_keys();
        f(&mut ctx);
        let mut tree = layout::build_tree(&ctx.commands);
        let area = Rect::new(0, 0, self.width, self.height);
        layout::compute(&mut tree, area);
        self.buffer.reset();
        layout::render(&tree, &mut self.buffer);
    }

    /// Get the rendered text content of row y (trimmed trailing spaces)
    pub fn line(&self, y: u32) -> String {
        let mut s = String::new();
        for x in 0..self.width {
            s.push_str(&self.buffer.get(x, y).symbol);
        }
        s.trim_end().to_string()
    }

    /// Assert that row y contains `expected` as a substring
    pub fn assert_line(&self, y: u32, expected: &str) {
        let line = self.line(y);
        assert_eq!(
            line, expected,
            "Line {y}: expected {expected:?}, got {line:?}"
        );
    }

    /// Assert that row y contains `expected` as a substring
    pub fn assert_line_contains(&self, y: u32, expected: &str) {
        let line = self.line(y);
        assert!(
            line.contains(expected),
            "Line {y}: expected to contain {expected:?}, got {line:?}"
        );
    }

    /// Assert that any line in the buffer contains `expected`
    pub fn assert_contains(&self, expected: &str) {
        for y in 0..self.height {
            if self.line(y).contains(expected) {
                return;
            }
        }
        let mut all_lines = String::new();
        for y in 0..self.height {
            all_lines.push_str(&format!("{}: {}\n", y, self.line(y)));
        }
        panic!("Buffer does not contain {expected:?}.\nBuffer:\n{all_lines}");
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}
