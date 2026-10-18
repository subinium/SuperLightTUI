//! Issue #443: scroll anchoring and `ScrollState::stick_to_bottom`.

use slt::event::{Event, KeyModifiers, MouseEvent, MouseKind};
use slt::rect::Rect;
use slt::{AppState, Backend, Buffer, Context, RunConfig, ScrollState, TestBackend};

struct FrameBackend {
    buffer: Buffer,
}

impl FrameBackend {
    fn line(&self, y: u32) -> String {
        let mut line = String::new();
        for x in 0..self.buffer.area.width {
            line.push_str(&self.buffer.get(x, y).symbol);
        }
        line.trim_end().to_string()
    }
}

impl Backend for FrameBackend {
    fn size(&self) -> (u32, u32) {
        (self.buffer.area.width, self.buffer.area.height)
    }
    fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Log {
    scroll: ScrollState,
    open: Vec<bool>,
}

impl Log {
    fn new(anchor: bool) -> Self {
        let mut scroll = ScrollState::new();
        scroll.anchor = anchor;
        Self {
            scroll,
            open: vec![true; 30],
        }
    }
}

fn log_view(ui: &mut Context, app: &mut Log) {
    let open = &app.open;
    let _ = ui.scrollable(&mut app.scroll).grow(1).col(|ui| {
        for (i, &is_open) in open.iter().enumerate() {
            let _ = ui.col(|ui| {
                ui.text(format!("block #{i:02}"));
                if is_open {
                    for line in 0..5 {
                        ui.text(format!("out[{i:02}:{line:02}] test result line {line}"));
                    }
                }
            });
        }
    });
}

fn wheel_down(n: usize) -> Vec<Event> {
    (0..n)
        .map(|_| {
            Event::Mouse(MouseEvent::new(
                MouseKind::ScrollDown,
                10,
                5,
                KeyModifiers::NONE,
                None,
                None,
            ))
        })
        .collect()
}

struct Harness {
    backend: FrameBackend,
    state: AppState,
    config: RunConfig,
}

impl Harness {
    fn new() -> Self {
        Self {
            backend: FrameBackend {
                buffer: Buffer::empty(Rect::new(0, 0, 80, 20)),
            },
            state: AppState::new(),
            config: RunConfig::default(),
        }
    }

    fn frame(&mut self, events: Vec<Event>, app: &mut Log) {
        self.backend.buffer.reset();
        slt::frame_owned(
            &mut self.backend,
            &mut self.state,
            &self.config,
            events,
            &mut |ui| log_view(ui, app),
        )
        .expect("frame");
    }
}

#[test]
fn collapsing_a_block_above_the_viewport_keeps_the_visible_line() {
    let mut h = Harness::new();
    let mut app = Log::new(true);
    h.frame(Vec::new(), &mut app);
    h.frame(Vec::new(), &mut app);
    h.frame(wheel_down(60), &mut app);
    h.frame(Vec::new(), &mut app);
    assert_eq!(app.scroll.offset, 60);
    let before = h.backend.line(3);
    assert!(before.starts_with("out[10:"), "row 3 before: {before}");

    app.open[0] = false;
    h.frame(Vec::new(), &mut app);
    assert_eq!(h.backend.line(3), before, "same line on the collapse frame");
    h.frame(Vec::new(), &mut app);
    assert_eq!(h.backend.line(3), before);
    assert_eq!(app.scroll.offset, 55, "offset shrank by the 5 removed rows");
}

#[test]
fn expanding_above_grows_the_offset_and_changes_below_do_not() {
    let mut h = Harness::new();
    let mut app = Log::new(true);
    app.open[0] = false;
    app.open[29] = false;
    h.frame(Vec::new(), &mut app);
    h.frame(Vec::new(), &mut app);
    h.frame(wheel_down(30), &mut app);
    h.frame(Vec::new(), &mut app);
    let offset = app.scroll.offset;
    assert_eq!(offset, 30);
    let before = h.backend.line(3);

    app.open[0] = true;
    h.frame(Vec::new(), &mut app);
    h.frame(Vec::new(), &mut app);
    assert_eq!(h.backend.line(3), before);
    assert_eq!(
        app.scroll.offset,
        offset + 5,
        "offset grew by the added rows"
    );

    // Growing a block below the viewport leaves the offset alone.
    app.open[29] = true;
    h.frame(Vec::new(), &mut app);
    h.frame(Vec::new(), &mut app);
    assert_eq!(app.scroll.offset, offset + 5);
    assert_eq!(h.backend.line(3), before);
}

#[test]
fn anchor_false_keeps_the_raw_offset() {
    let mut h = Harness::new();
    let mut app = Log::new(false);
    h.frame(Vec::new(), &mut app);
    h.frame(Vec::new(), &mut app);
    h.frame(wheel_down(60), &mut app);
    h.frame(Vec::new(), &mut app);
    let before = h.backend.line(3);
    app.open[0] = false;
    h.frame(Vec::new(), &mut app);
    h.frame(Vec::new(), &mut app);
    assert_eq!(app.scroll.offset, 60, "raw offset unchanged");
    assert_ne!(h.backend.line(3), before, "content shifts like before 0.26");
}

fn tail_view(ui: &mut Context, scroll: &mut ScrollState, lines: usize) {
    let _ = ui.scrollable(scroll).grow(1).col(|ui| {
        for i in 0..lines {
            ui.text(format!("log line {i:03}"));
        }
    });
}

#[test]
fn stick_to_bottom_follows_appends_until_the_user_scrolls_up() {
    let mut tb = TestBackend::new(30, 5);
    let mut scroll = ScrollState::new().stick_to_bottom(true);
    let mut lines = 3;
    for _ in 0..3 {
        tb.render(|ui| tail_view(ui, &mut scroll, lines));
    }
    // Content grows past the viewport: the view follows the end.
    for _ in 0..10 {
        lines += 1;
        tb.render(|ui| tail_view(ui, &mut scroll, lines));
    }
    tb.render(|ui| tail_view(ui, &mut scroll, lines));
    assert_eq!(scroll.offset, scroll.max_offset());
    assert_eq!(scroll.offset, lines - 5);
    tb.assert_contains(&format!("log line {:03}", lines - 1));

    // One wheel-up disengages it.
    let up = vec![Event::Mouse(MouseEvent::new(
        MouseKind::ScrollUp,
        2,
        2,
        KeyModifiers::NONE,
        None,
        None,
    ))];
    tb.run_with_events(up, |ui| tail_view(ui, &mut scroll, lines));
    let parked = scroll.offset;
    assert_eq!(parked, lines - 6);
    for _ in 0..3 {
        lines += 1;
        tb.render(|ui| tail_view(ui, &mut scroll, lines));
    }
    assert_eq!(scroll.offset, parked, "disengaged while scrolled up");

    // Scrolling back to the end re-engages it.
    tb.render(|ui| tail_view(ui, &mut scroll, lines));
    let down: Vec<Event> = (0..20)
        .map(|_| {
            Event::Mouse(MouseEvent::new(
                MouseKind::ScrollDown,
                2,
                2,
                KeyModifiers::NONE,
                None,
                None,
            ))
        })
        .collect();
    tb.run_with_events(down, |ui| tail_view(ui, &mut scroll, lines));
    assert_eq!(scroll.offset, scroll.max_offset(), "back at the end");
    for _ in 0..2 {
        lines += 1;
        tb.render(|ui| tail_view(ui, &mut scroll, lines));
    }
    tb.render(|ui| tail_view(ui, &mut scroll, lines));
    assert_eq!(scroll.offset, scroll.max_offset());
    assert_eq!(scroll.offset, lines - 5);
}

fn ticking_view(ui: &mut Context, app: &mut Log, tick: u64) {
    let open = &app.open;
    let _ = ui.scrollable(&mut app.scroll).grow(1).col(|ui| {
        for (i, &is_open) in open.iter().enumerate() {
            let _ = ui.col(|ui| {
                // Every header carries live text (a timer), so the anchor's
                // content differs on every frame.
                ui.text(format!("block #{i:02} running {tick}s"));
                if is_open {
                    for line in 0..5 {
                        ui.text(format!("out[{i:02}:{line:02}] {tick}"));
                    }
                }
            });
        }
    });
}

#[test]
fn anchoring_survives_text_that_changes_every_frame() {
    let mut backend = FrameBackend {
        buffer: Buffer::empty(Rect::new(0, 0, 80, 20)),
    };
    let mut state = AppState::new();
    let config = RunConfig::default();
    let mut app = Log::new(true);
    let mut tick = 0u64;
    let mut frame = |events: Vec<Event>, app: &mut Log, backend: &mut FrameBackend| {
        tick += 1;
        backend.buffer.reset();
        slt::frame_owned(backend, &mut state, &config, events, &mut |ui| {
            ticking_view(ui, app, tick)
        })
        .expect("frame");
    };
    frame(Vec::new(), &mut app, &mut backend);
    frame(Vec::new(), &mut app, &mut backend);
    frame(wheel_down(60), &mut app, &mut backend);
    frame(Vec::new(), &mut app, &mut backend);
    assert_eq!(app.scroll.offset, 60);
    let before = backend.line(3);
    assert!(before.starts_with("out[10:"), "row 3 before: {before}");

    app.open[0] = false;
    frame(Vec::new(), &mut app, &mut backend);
    frame(Vec::new(), &mut app, &mut backend);
    assert_eq!(app.scroll.offset, 55, "anchored despite the changing text");
    assert!(
        backend.line(3).starts_with(&before[..8]),
        "same line stays at row 3"
    );
}
