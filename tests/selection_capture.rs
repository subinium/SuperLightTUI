//! Issue #438: built-in text selection must not hijack widget drags or the
//! clipboard, and `RunConfig::text_selection(false)` turns it off.

use slt::event::Event;
use slt::rect::Rect;
use slt::{
    AppState, Backend, Buffer, Context, EventBuilder, Modifiers, RunConfig, ScrollState,
    SplitPaneState,
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

    fn reversed_cells(&self) -> usize {
        let area = self.buffer.area;
        (0..area.height)
            .flat_map(|y| (0..area.width).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                self.buffer
                    .get(x, y)
                    .style
                    .modifiers
                    .contains(Modifiers::REVERSED)
            })
            .count()
    }

    fn line(&self, y: u32) -> String {
        (0..self.buffer.area.width)
            .map(|x| self.buffer.get(x, y).symbol.to_string())
            .collect()
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
    fn new(width: u32, height: u32, config: RunConfig) -> Self {
        Self {
            backend: FrameBackend::new(width, height),
            state: AppState::new(),
            config,
        }
    }

    fn frame(&mut self, events: Vec<Event>, f: &mut impl FnMut(&mut Context)) {
        slt::frame_owned(&mut self.backend, &mut self.state, &self.config, events, f)
            .expect("frame");
    }
}

fn split_view(ui: &mut Context, split: &mut SplitPaneState) {
    let _ = ui.split_pane(
        split,
        |ui| {
            ui.text("LEFTPANE text");
        },
        |ui| {
            ui.text("RIGHTPANE text");
        },
    );
}

#[test]
fn splitter_drag_does_not_select_text() {
    let mut s = Session::new(40, 4, RunConfig::default());
    let mut split = SplitPaneState::new(0.5);
    s.frame(Vec::new(), &mut |ui| split_view(ui, &mut split));
    let line = s.backend.line(0);
    let x = line[..line.find('│').expect("divider")].chars().count() as u32;
    s.frame(
        EventBuilder::new()
            .click(x, 0)
            .drag(11, 0)
            .drag(9, 1)
            .build(),
        &mut |ui| split_view(ui, &mut split),
    );
    assert!(split.ratio < 0.5, "the drag still works");
    assert_eq!(s.backend.reversed_cells(), 0, "no selection highlight");
    s.frame(EventBuilder::new().mouse_up(9, 1).build(), &mut |ui| {
        split_view(ui, &mut split)
    });
    assert_eq!(s.backend.reversed_cells(), 0);
}

fn scroll_view(ui: &mut Context, scroll: &mut ScrollState) {
    let _ = ui.row(|ui| {
        let _ = ui.scrollable(scroll).grow(1).col(|ui| {
            for i in 0..40 {
                ui.text(format!("row {i:02} abcdef"));
            }
        });
        let _ = ui.scrollbar(scroll);
    });
}

#[test]
fn scrollbar_thumb_drag_does_not_select_text() {
    let mut s = Session::new(30, 6, RunConfig::default());
    let mut scroll = ScrollState::new();
    s.frame(Vec::new(), &mut |ui| scroll_view(ui, &mut scroll));
    s.frame(Vec::new(), &mut |ui| scroll_view(ui, &mut scroll));
    s.frame(
        EventBuilder::new()
            .click(29, 0)
            .drag(29, 2)
            .drag(27, 3)
            .build(),
        &mut |ui| scroll_view(ui, &mut scroll),
    );
    assert!(scroll.offset > 0, "the thumb drag still scrolls");
    assert_eq!(s.backend.reversed_cells(), 0);
}

#[test]
fn consumed_press_captures_the_whole_gesture() {
    // The press lands on the divider (consumed). The split pane then
    // disappears, so later drags over plain text are not consumed by anyone:
    // capture must still keep the selection idle until the release.
    let mut s = Session::new(40, 4, RunConfig::default());
    let mut split = SplitPaneState::new(0.5);
    s.frame(Vec::new(), &mut |ui| split_view(ui, &mut split));
    let line = s.backend.line(0);
    let x = line[..line.find('│').expect("divider")].chars().count() as u32;
    s.frame(EventBuilder::new().click(x, 0).build(), &mut |ui| {
        split_view(ui, &mut split)
    });
    let mut text_only = |ui: &mut Context| {
        let _ = ui.col(|ui| {
            ui.text("plain text that could be selected");
            ui.text("second line of plain text");
        });
    };
    s.frame(
        EventBuilder::new().drag(2, 0).drag(20, 1).build(),
        &mut text_only,
    );
    assert_eq!(s.backend.reversed_cells(), 0, "capture holds after leaving");

    // After the release a fresh plain drag selects again.
    s.frame(EventBuilder::new().mouse_up(20, 1).build(), &mut text_only);
    s.frame(
        EventBuilder::new().click(2, 0).drag(20, 1).build(),
        &mut text_only,
    );
    assert!(s.backend.reversed_cells() > 0, "plain drags still select");
}

fn text_view(ui: &mut Context) {
    let _ = ui.col(|ui| {
        ui.text("COPYME selectable text");
        ui.text("another line");
    });
}

#[test]
fn text_selection_false_disables_overlay_and_copy() {
    let mut on = Session::new(40, 4, RunConfig::default());
    on.frame(Vec::new(), &mut text_view);
    on.frame(
        EventBuilder::new().click(0, 0).drag(12, 0).build(),
        &mut text_view,
    );
    assert!(on.backend.reversed_cells() > 0, "default config selects");

    let mut off = Session::new(40, 4, RunConfig::default().text_selection(false));
    off.frame(Vec::new(), &mut text_view);
    off.frame(
        EventBuilder::new().click(0, 0).drag(12, 0).build(),
        &mut text_view,
    );
    assert_eq!(off.backend.reversed_cells(), 0, "no overlay when disabled");
    off.frame(EventBuilder::new().mouse_up(12, 0).build(), &mut text_view);
    assert_eq!(off.backend.reversed_cells(), 0);
}
