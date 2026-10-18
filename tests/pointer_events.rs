//! Issue #442: pointer event model. Every press per frame is kept, clicks can
//! activate on release, `Response` carries press/release/drag data, and drag
//! gestures end when the terminal loses focus.

use slt::event::Event;
use slt::rect::Rect;
use slt::{
    AppState, Backend, Buffer, Context, EventBuilder, Response, RunConfig, ScrollState,
    SplitPaneState, TestBackend,
};

struct FrameBackend {
    buffer: Buffer,
}

impl FrameBackend {
    fn new(width: u32, height: u32) -> Self {
        Self {
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
        }
    }

    fn find(&self, needle: &str) -> Option<(u32, u32)> {
        for y in 0..self.buffer.area.height {
            let mut line = String::new();
            for x in 0..self.buffer.area.width {
                line.push_str(&self.buffer.get(x, y).symbol);
            }
            if let Some(col) = line.find(needle) {
                return Some((line[..col].chars().count() as u32, y));
            }
        }
        None
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

struct Headers {
    open: [bool; 4],
    toggles: u32,
}

fn headers_view(ui: &mut Context, app: &mut Headers) {
    for i in 0..app.open.len() {
        let header = ui.container().row(|ui| {
            ui.text(format!("#{i:02} header"));
        });
        if header.clicked {
            app.open[i] = !app.open[i];
            app.toggles += 1;
        }
    }
}

#[test]
fn two_click_release_pairs_in_one_frame_toggle_both() {
    let mut backend = FrameBackend::new(30, 6);
    let mut state = AppState::new();
    let config = RunConfig::default();
    let mut app = Headers {
        open: [false; 4],
        toggles: 0,
    };
    slt::frame_owned(&mut backend, &mut state, &config, Vec::new(), &mut |ui| {
        headers_view(ui, &mut app)
    })
    .expect("frame");
    let (x1, y1) = backend.find("#01").expect("header 1");
    let (x3, y3) = backend.find("#03").expect("header 3");
    let batch = EventBuilder::new()
        .click(x1, y1)
        .mouse_up(x1, y1)
        .click(x3, y3)
        .mouse_up(x3, y3)
        .build();
    slt::frame_owned(&mut backend, &mut state, &config, batch, &mut |ui| {
        headers_view(ui, &mut app)
    })
    .expect("frame");
    // One press per frame: the second click is carried to the next frame so
    // it is hit-tested against geometry that reflects the first one.
    assert_eq!(app.toggles, 1);
    assert!(
        state.has_pending_input(),
        "second press queued, not dropped"
    );
    while state.has_pending_input() {
        slt::frame_owned(&mut backend, &mut state, &config, Vec::new(), &mut |ui| {
            headers_view(ui, &mut app)
        })
        .expect("frame");
    }
    assert_eq!(app.toggles, 2);
    assert_eq!(app.open, [false, true, false, true]);
}

#[test]
fn click_on_release_cancels_when_released_outside() {
    let mut tb =
        TestBackend::new(30, 5).with_run_config(RunConfig::default().click_on_release(true));
    let mut clicks = Vec::new();
    let view = |ui: &mut Context, clicks: &mut Vec<bool>| {
        clicks.push(ui.button("PRESSME").clicked);
    };
    tb.render(|ui| view(ui, &mut clicks));
    let (x, y) = tb.find_text("PRESSME").expect("button rendered");

    // Press inside, release far outside: no click on either frame.
    tb.run_with_events(EventBuilder::new().click(x, y).build(), |ui| {
        view(ui, &mut clicks)
    });
    tb.run_with_events(
        EventBuilder::new().drag(25, 4).mouse_up(25, 4).build(),
        |ui| view(ui, &mut clicks),
    );
    assert_eq!(clicks, [false, false, false]);

    // Press and release inside: clicked on the release frame only.
    clicks.clear();
    tb.run_with_events(EventBuilder::new().click(x, y).build(), |ui| {
        view(ui, &mut clicks)
    });
    tb.run_with_events(EventBuilder::new().mouse_up(x, y).build(), |ui| {
        view(ui, &mut clicks)
    });
    assert_eq!(clicks, [false, true]);
}

#[test]
fn default_click_still_fires_on_press() {
    let mut tb = TestBackend::new(30, 5);
    let mut clicked = false;
    tb.render(|ui| {
        let _ = ui.button("PRESSME");
    });
    let (x, y) = tb.find_text("PRESSME").expect("button rendered");
    tb.run_with_events(EventBuilder::new().click(x, y).build(), |ui| {
        clicked = ui.button("PRESSME").clicked;
    });
    assert!(clicked, "press activates by default");
}

fn split_view(ui: &mut Context, split: &mut SplitPaneState) {
    let _ = ui.split_pane(
        split,
        |ui| {
            ui.text("LEFTPANE");
        },
        |ui| {
            ui.text("RIGHTPANE");
        },
    );
}

fn divider_column(tb: &TestBackend) -> u32 {
    let line = tb.line(0);
    let col = line.find('│').expect("split divider rendered");
    line[..col].chars().count() as u32
}

#[test]
fn split_drag_ends_on_focus_lost() {
    let mut tb = TestBackend::new(40, 4);
    let mut split = SplitPaneState::new(0.5);
    tb.render(|ui| split_view(ui, &mut split));
    let x = divider_column(&tb);

    tb.run_with_events(EventBuilder::new().click(x, 0).drag(10, 0).build(), |ui| {
        split_view(ui, &mut split)
    });
    assert!(split.dragging, "divider grabbed");
    let ratio = split.ratio;
    assert!(ratio < 0.5, "drag moved the divider");

    tb.run_with_events(EventBuilder::new().focus_lost().build(), |ui| {
        split_view(ui, &mut split)
    });
    assert!(!split.dragging, "focus loss ends the drag");

    tb.run_with_events(EventBuilder::new().drag(30, 0).build(), |ui| {
        split_view(ui, &mut split)
    });
    assert_eq!(split.ratio, ratio, "stale motion must not move the divider");
    assert!(!split.dragging);
}

fn scroll_view(ui: &mut Context, scroll: &mut ScrollState) {
    let _ = ui.row(|ui| {
        let _ = ui.scrollable(scroll).grow(1).col(|ui| {
            for i in 0..40 {
                ui.text(format!("row {i:02}"));
            }
        });
        let _ = ui.scrollbar(scroll);
    });
}

#[test]
fn scrollbar_drag_ends_on_focus_lost() {
    let mut tb = TestBackend::new(30, 6);
    let mut scroll = ScrollState::new();
    tb.render(|ui| scroll_view(ui, &mut scroll));
    tb.render(|ui| scroll_view(ui, &mut scroll));

    tb.run_with_events(EventBuilder::new().click(29, 0).drag(29, 3).build(), |ui| {
        scroll_view(ui, &mut scroll)
    });
    assert!(scroll.dragging, "thumb grabbed");
    let offset = scroll.offset;
    assert!(offset > 0, "thumb drag scrolled");

    tb.run_with_events(EventBuilder::new().focus_lost().build(), |ui| {
        scroll_view(ui, &mut scroll)
    });
    assert!(!scroll.dragging, "focus loss ends the thumb drag");

    tb.run_with_events(EventBuilder::new().drag(29, 5).build(), |ui| {
        scroll_view(ui, &mut scroll)
    });
    assert_eq!(scroll.offset, offset, "stale motion must not scroll");
}

#[test]
fn split_drag_ends_on_motion_without_button() {
    let mut tb = TestBackend::new(40, 4);
    let mut split = SplitPaneState::new(0.5);
    tb.render(|ui| split_view(ui, &mut split));
    let x = divider_column(&tb);
    tb.run_with_events(EventBuilder::new().click(x, 0).drag(12, 0).build(), |ui| {
        split_view(ui, &mut split)
    });
    assert!(split.dragging);
    tb.run_with_events(vec![Event::mouse_move(20, 2)], |ui| {
        split_view(ui, &mut split)
    });
    assert!(
        !split.dragging,
        "button-less motion means the release was lost"
    );
}

fn handle_view(ui: &mut Context, log: &mut Vec<Response>) {
    let r = ui.container().w(6).h(1).row(|ui| {
        ui.text("HANDLE");
    });
    log.push(r);
}

#[test]
fn drag_delta_accumulates_and_released_fires_once() {
    let mut tb = TestBackend::new(30, 6);
    let mut log = Vec::new();
    tb.render(|ui| handle_view(ui, &mut log));
    let (x, y) = tb.find_text("HANDLE").expect("handle rendered");

    let frames = [
        EventBuilder::new().click(x, y).build(),
        EventBuilder::new()
            .drag(x + 2, y + 1)
            .drag(x + 3, y + 1)
            .build(),
        EventBuilder::new().drag(x + 5, y + 3).build(),
        EventBuilder::new().mouse_up(x + 5, y + 3).build(),
        Vec::new(),
    ];
    log.clear();
    for events in frames {
        tb.run_with_events(events, |ui| handle_view(ui, &mut log));
    }
    let pressed: Vec<bool> = log.iter().map(|r| r.pressed).collect();
    let started: Vec<bool> = log.iter().map(|r| r.drag_started).collect();
    let dragging: Vec<bool> = log.iter().map(|r| r.dragging).collect();
    let released: Vec<bool> = log.iter().map(|r| r.released).collect();
    assert_eq!(pressed, [true, false, false, false, false]);
    assert_eq!(started, [false, true, false, false, false]);
    assert_eq!(dragging, [false, true, true, false, false]);
    assert_eq!(released, [false, false, false, true, false]);
    let total = log.iter().fold((0, 0), |acc, r| {
        (acc.0 + r.drag_delta.0, acc.1 + r.drag_delta.1)
    });
    assert_eq!(total, (5, 3));
    assert_eq!(log[1].drag_delta, (3, 1));
}

#[test]
fn focus_lost_cancels_capture_without_release() {
    let mut tb = TestBackend::new(30, 6);
    let mut log = Vec::new();
    tb.render(|ui| handle_view(ui, &mut log));
    let (x, y) = tb.find_text("HANDLE").expect("handle rendered");
    log.clear();
    for events in [
        EventBuilder::new().click(x, y).drag(x + 1, y).build(),
        EventBuilder::new().focus_lost().build(),
        EventBuilder::new()
            .drag(x + 4, y)
            .mouse_up(x + 4, y)
            .build(),
    ] {
        tb.run_with_events(events, |ui| handle_view(ui, &mut log));
    }
    assert!(log[0].dragging);
    assert!(!log[1].dragging && !log[1].released);
    assert_eq!(log[2].drag_delta, (0, 0));
    assert!(
        !log[2].released,
        "a cancelled gesture never reports release"
    );
}

#[test]
fn middle_click_and_modifiers_follow_the_target() {
    use slt::event::{KeyModifiers, MouseButton, MouseEvent, MouseKind};
    let mut tb = TestBackend::new(30, 6);
    let mut log = Vec::new();
    tb.render(|ui| handle_view(ui, &mut log));
    let (x, y) = tb.find_text("HANDLE").expect("handle rendered");
    log.clear();
    let middle = Event::Mouse(MouseEvent::new(
        MouseKind::Down(MouseButton::Middle),
        x,
        y,
        KeyModifiers::NONE,
        None,
        None,
    ));
    tb.run_with_events(vec![middle], |ui| handle_view(ui, &mut log));
    tb.run_with_events(
        EventBuilder::new()
            .click_with(x, y, KeyModifiers::SHIFT)
            .build(),
        |ui| handle_view(ui, &mut log),
    );
    assert!(log[0].middle_clicked && !log[0].clicked);
    assert!(log[1].clicked && log[1].modifiers.contains(KeyModifiers::SHIFT));
}
