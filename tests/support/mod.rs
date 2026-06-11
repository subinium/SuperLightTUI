use std::io;

use slt::{AppState, Backend, Buffer, Context, Event, Rect, RunConfig, frame};

pub struct RecordingBackend {
    buffer: Buffer,
    width: u32,
    height: u32,
    last_rendered: String,
}

impl RecordingBackend {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
            width,
            height,
            last_rendered: String::new(),
        }
    }

    pub fn to_string_trimmed(&self) -> String {
        self.last_rendered.clone()
    }

    fn snapshot_current_buffer(&self) -> String {
        let mut lines = Vec::with_capacity(self.height as usize);
        for y in 0..self.height {
            let mut line = String::new();
            for x in 0..self.width {
                line.push_str(&self.buffer.get(x, y).symbol);
            }
            lines.push(line.trim_end().to_string());
        }
        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }
        lines.join("\n")
    }
}

impl Backend for RecordingBackend {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    fn flush(&mut self) -> io::Result<()> {
        self.last_rendered = self.snapshot_current_buffer();
        self.buffer.reset();
        Ok(())
    }
}

pub fn render_with_frame_backend(
    backend: &mut RecordingBackend,
    state: &mut AppState,
    events: &[Event],
    f: &mut impl FnMut(&mut Context),
) -> io::Result<String> {
    let keep_running = frame(backend, state, &RunConfig::default(), events, f)?;
    assert!(keep_running, "test frame unexpectedly requested quit");
    Ok(backend.to_string_trimmed())
}
