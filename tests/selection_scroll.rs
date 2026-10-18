//! Issue #445: the built-in selection works on any text, follows scrolled
//! content, and auto-scrolls at a region edge.

use slt::event::{Event, KeyModifiers, MouseEvent, MouseKind};
use slt::rect::Rect;
use slt::{
    AppState, Backend, Buffer, Context, EventBuilder, Modifiers, RunConfig, ScrollState,
    TestBackend,
};

struct FrameBackend {
    buffer: Buffer,
}

impl FrameBackend {
    fn reversed_rows(&self) -> Vec<u32> {
        let area = self.buffer.area;
        (0..area.height)
            .filter(|&y| {
                (0..area.width).any(|x| {
                    self.buffer
                        .get(x, y)
                        .style
                        .modifiers
                        .contains(Modifiers::REVERSED)
                })
            })
            .collect()
    }

    fn line(&self, y: u32) -> String {
        (0..self.buffer.area.width)
            .map(|x| self.buffer.get(x, y).symbol.to_string())
            .collect::<String>()
            .trim_end()
            .to_string()
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

struct Session {
    backend: FrameBackend,
    state: AppState,
    config: RunConfig,
}

impl Session {
    fn new(width: u32, height: u32) -> Self {
        Self {
            backend: FrameBackend {
                buffer: Buffer::empty(Rect::new(0, 0, width, height)),
            },
            state: AppState::new(),
            config: RunConfig::default(),
        }
    }

    fn frame(&mut self, events: Vec<Event>, f: &mut impl FnMut(&mut Context)) {
        // Fresh buffer per frame, like a terminal backend after its flush.
        self.backend.buffer.reset();
        slt::frame_owned(&mut self.backend, &mut self.state, &self.config, events, f)
            .expect("frame");
    }
}

#[test]
fn drag_across_top_level_text_selects() {
    let mut s = Session::new(40, 3);
    let mut view = |ui: &mut Context| {
        ui.text("COPYME selectable text");
    };
    s.frame(Vec::new(), &mut view);
    s.frame(Vec::new(), &mut view);
    s.frame(
        EventBuilder::new().click(0, 0).drag(10, 0).build(),
        &mut view,
    );
    assert_eq!(
        s.backend.reversed_rows(),
        [0],
        "top-level text is selectable"
    );
}

fn rows_view(ui: &mut Context, scroll: &mut ScrollState) {
    let _ = ui.scrollable(scroll).grow(1).col(|ui| {
        for i in 0..40 {
            ui.text(format!("row {i:02} abcdef"));
        }
    });
}

fn wheel(kind: MouseKind, n: usize) -> Vec<Event> {
    (0..n)
        .map(|_| {
            Event::Mouse(MouseEvent::new(
                kind.clone(),
                5,
                3,
                KeyModifiers::NONE,
                None,
                None,
            ))
        })
        .collect()
}

#[test]
fn selection_stays_attached_to_its_text_while_scrolling() {
    let mut s = Session::new(30, 6);
    let mut scroll = ScrollState::new();
    s.frame(Vec::new(), &mut |ui| rows_view(ui, &mut scroll));
    s.frame(Vec::new(), &mut |ui| rows_view(ui, &mut scroll));
    s.frame(
        EventBuilder::new().click(0, 1).drag(8, 2).build(),
        &mut |ui| rows_view(ui, &mut scroll),
    );
    assert_eq!(s.backend.reversed_rows(), [1, 2]);
    assert!(s.backend.line(1).starts_with("row 01"));

    // Scroll the selected lines out of view: nothing visible is highlighted.
    s.frame(wheel(MouseKind::ScrollDown, 3), &mut |ui| {
        rows_view(ui, &mut scroll)
    });
    assert_eq!(scroll.offset, 3);
    assert!(s.backend.line(1).starts_with("row 04"));
    assert!(
        s.backend.reversed_rows().is_empty(),
        "highlight left the screen with its text"
    );

    // Scroll back: the same rows light up over the same text.
    s.frame(wheel(MouseKind::ScrollUp, 3), &mut |ui| {
        rows_view(ui, &mut scroll)
    });
    assert_eq!(scroll.offset, 0);
    assert_eq!(s.backend.reversed_rows(), [1, 2]);
    assert!(s.backend.line(1).starts_with("row 01"));
}

#[test]
fn scrolling_shifts_a_partially_visible_selection_with_its_text() {
    let mut s = Session::new(30, 6);
    let mut scroll = ScrollState::new();
    s.frame(Vec::new(), &mut |ui| rows_view(ui, &mut scroll));
    s.frame(Vec::new(), &mut |ui| rows_view(ui, &mut scroll));
    s.frame(
        EventBuilder::new()
            .click(0, 2)
            .drag(8, 3)
            .mouse_up(8, 3)
            .build(),
        &mut |ui| rows_view(ui, &mut scroll),
    );
    assert_eq!(s.backend.reversed_rows(), [2, 3]);
    s.frame(wheel(MouseKind::ScrollDown, 1), &mut |ui| {
        rows_view(ui, &mut scroll)
    });
    assert_eq!(
        s.backend.reversed_rows(),
        [1, 2],
        "highlight moved up with rows 02-03"
    );
}

#[test]
fn dragging_below_the_bottom_edge_auto_scrolls_until_the_limit() {
    let mut tb = TestBackend::new(30, 6);
    let mut scroll = ScrollState::new();
    tb.render(|ui| rows_view(ui, &mut scroll));
    tb.render(|ui| rows_view(ui, &mut scroll));
    // Press inside, then hold the pointer below the region's last row.
    tb.run_with_events(EventBuilder::new().click(0, 1).drag(5, 9).build(), |ui| {
        rows_view(ui, &mut scroll)
    });
    let mut offsets = Vec::new();
    for _ in 0..40 {
        tb.render(|ui| rows_view(ui, &mut scroll));
        offsets.push(scroll.offset);
    }
    let max = scroll.max_offset();
    assert_eq!(max, 34);
    assert!(
        offsets.windows(2).all(|w| w[1] == (w[0] + 1).min(max)),
        "one row per frame: {offsets:?}"
    );
    assert_eq!(*offsets.last().expect("frames"), max, "stops at the limit");

    // Releasing stops the auto-scroll.
    tb.run_with_events(EventBuilder::new().mouse_up(5, 9).build(), |ui| {
        rows_view(ui, &mut scroll)
    });
    scroll.offset = 10;
    tb.render(|ui| rows_view(ui, &mut scroll));
    tb.render(|ui| rows_view(ui, &mut scroll));
    assert_eq!(scroll.offset, 10);
}

#[test]
fn dragging_above_the_top_edge_auto_scrolls_up() {
    let mut tb = TestBackend::new(30, 6);
    let mut scroll = ScrollState::new();
    scroll.offset = 5;
    tb.render(|ui| rows_view(ui, &mut scroll));
    tb.render(|ui| rows_view(ui, &mut scroll));
    tb.run_with_events(EventBuilder::new().click(0, 3).drag(4, 0).build(), |ui| {
        rows_view(ui, &mut scroll)
    });
    for _ in 0..10 {
        tb.render(|ui| rows_view(ui, &mut scroll));
    }
    assert_eq!(scroll.offset, 0);
}
