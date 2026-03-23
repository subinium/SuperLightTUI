use std::io;

use slt::{frame, AppState, Backend, Buffer, Context, EventBuilder, Rect, RunConfig};

struct ContractBackend {
    buffer: Buffer,
    width: u32,
    height: u32,
    flush_count: usize,
    last_snapshot: String,
    fail_flush: Option<io::ErrorKind>,
}

impl ContractBackend {
    fn new(width: u32, height: u32) -> Self {
        Self {
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
            width,
            height,
            flush_count: 0,
            last_snapshot: String::new(),
            fail_flush: None,
        }
    }

    fn with_flush_error(mut self, kind: io::ErrorKind) -> Self {
        self.fail_flush = Some(kind);
        self
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.buffer.resize(Rect::new(0, 0, width, height));
    }

    fn snapshot(&self) -> &str {
        &self.last_snapshot
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

impl Backend for ContractBackend {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_count += 1;
        if let Some(kind) = self.fail_flush {
            return Err(io::Error::new(kind, "backend flush failed"));
        }

        self.last_snapshot = self.snapshot_current_buffer();
        self.buffer.reset();
        Ok(())
    }
}

#[test]
fn frame_propagates_backend_flush_errors() {
    let mut backend = ContractBackend::new(20, 2).with_flush_error(io::ErrorKind::Other);
    let mut state = AppState::new();

    let err = frame(
        &mut backend,
        &mut state,
        &RunConfig::default(),
        &[],
        &mut |ui: &mut Context| {
            ui.text("flush me");
        },
    )
    .unwrap_err();

    assert_eq!(err.kind(), io::ErrorKind::Other);
    assert_eq!(backend.flush_count, 1);
}

#[test]
fn frame_returns_false_after_quit() {
    let mut backend = ContractBackend::new(20, 2);
    let mut state = AppState::new();

    let keep_running = frame(
        &mut backend,
        &mut state,
        &RunConfig::default(),
        &[],
        &mut |ui: &mut Context| {
            ui.quit();
        },
    )
    .unwrap();

    assert!(!keep_running);
}

#[test]
fn frame_quit_still_persists_hook_state_for_callers() {
    let mut backend = ContractBackend::new(20, 2);
    let mut state = AppState::new();
    let config = RunConfig::default();

    let keep_running = frame(
        &mut backend,
        &mut state,
        &config,
        &[],
        &mut |ui: &mut Context| {
            let count = ui.use_state(|| 0i32);
            *count.get_mut(ui) = 7;
            ui.quit();
        },
    )
    .unwrap();

    assert!(!keep_running);

    let keep_running = frame(
        &mut backend,
        &mut state,
        &config,
        &[],
        &mut |ui: &mut Context| {
            let count = ui.use_state(|| 0i32);
            ui.text(format!("count={}", *count.get(ui)));
        },
    )
    .unwrap();

    assert!(keep_running);
    assert_eq!(backend.snapshot(), "count=7");
}

#[test]
fn app_state_persists_hooks_and_tick_across_frames() {
    let mut backend = ContractBackend::new(20, 2);
    let mut state = AppState::new();
    let config = RunConfig::default();

    let inc = EventBuilder::new().key('i').build();
    let keep_running = frame(
        &mut backend,
        &mut state,
        &config,
        &inc,
        &mut |ui: &mut Context| {
            let count = ui.use_state(|| 0i32);
            if ui.key('i') {
                *count.get_mut(ui) += 1;
            }
            let value = *count.get(ui);
            ui.text(format!("count={value}"));
        },
    )
    .unwrap();
    assert!(keep_running);
    assert_eq!(backend.snapshot(), "count=1");
    assert_eq!(state.tick(), 1);

    let keep_running = frame(
        &mut backend,
        &mut state,
        &config,
        &[],
        &mut |ui: &mut Context| {
            let count = ui.use_state(|| 0i32);
            let value = *count.get(ui);
            ui.text(format!("count={value}"));
        },
    )
    .unwrap();
    assert!(keep_running);
    assert_eq!(backend.snapshot(), "count=1");
    assert_eq!(state.tick(), 2);
}

#[test]
fn frame_uses_backend_size_after_resize() {
    let mut backend = ContractBackend::new(6, 1);
    let mut state = AppState::new();
    let config = RunConfig::default();

    frame(
        &mut backend,
        &mut state,
        &config,
        &[],
        &mut |ui: &mut Context| {
            ui.text("1234567890");
        },
    )
    .unwrap();
    assert_eq!(backend.snapshot(), "123456");

    backend.resize(10, 1);
    let events = EventBuilder::new().resize(10, 1).build();
    frame(
        &mut backend,
        &mut state,
        &config,
        &events,
        &mut |ui: &mut Context| {
            ui.text("1234567890");
        },
    )
    .unwrap();
    assert_eq!(backend.snapshot(), "1234567890");
}
