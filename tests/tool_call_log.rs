//! The "expandable tool-call log" scenario from docs/INTERACTION.md, end to
//! end through the v0.26 pointer model:
//!
//! - a click on a header toggles that block (and only that block, #439);
//! - wheel motion over an expanded block's nested scrollable chains to the
//!   outer list once the block reaches its end (#440);
//! - collapsing a block above the viewport keeps the visible line (#443);
//! - dragging the scrollbar thumb never selects or copies text (#438).

use slt::event::{Event, KeyModifiers, MouseEvent, MouseKind};
use slt::rect::Rect;
use slt::{
    AppState, Backend, Border, Buffer, Color, Context, EventBuilder, KeyCode, Modifiers, RunConfig,
    ScrollState,
};

struct ToolCall {
    name: String,
    output: Vec<String>,
    open: bool,
    body: ScrollState,
}

struct App {
    calls: Vec<ToolCall>,
    scroll: ScrollState,
    toggles: usize,
}

impl App {
    fn new() -> Self {
        Self {
            calls: (0..30)
                .map(|i| ToolCall {
                    name: format!("#{i:02} {}", ["read_file", "grep", "bash"][i % 3]),
                    output: (0..8).map(|line| format!("out {i:02}.{line}")).collect(),
                    open: false,
                    body: ScrollState::new(),
                })
                .collect(),
            scroll: ScrollState::new(),
            toggles: 0,
        }
    }
}

fn tool_call(ui: &mut Context, call: &mut ToolCall, toggles: &mut usize) {
    let focused = ui.register_focusable();
    let pressed = focused && (ui.consume_key_code(KeyCode::Enter) || ui.consume_key(' '));
    let icon = if call.open { "v" } else { ">" };
    let header = ui
        .container()
        .px(1)
        .gap(1)
        .with_if(focused, |b| b.bg(Color::Indexed(237)))
        .row(|ui| {
            ui.text(icon).dim();
            ui.text(call.name.as_str()).bold();
        });
    if header.clicked || pressed {
        call.open = !call.open;
        *toggles += 1;
    }
    if call.open {
        let output = &call.output;
        let _ = ui.scrollable(&mut call.body).max_h(4).pl(4).col(|ui| {
            for line in output {
                ui.text(line.as_str());
            }
        });
    }
}

fn tool_log(ui: &mut Context, app: &mut App) {
    let App {
        calls,
        scroll,
        toggles,
    } = app;
    let _ = ui
        .bordered(Border::Rounded)
        .title("Tool calls")
        .grow(1)
        .row(|ui| {
            let _ = ui.scrollable(scroll).grow(1).col(|ui| {
                for call in calls.iter_mut() {
                    tool_call(ui, call, toggles);
                }
            });
            let _ = ui.scrollbar(scroll);
        });
}

struct Screen {
    buffer: Buffer,
}

impl Screen {
    fn line(&self, y: u32) -> String {
        (0..self.buffer.area.width)
            .map(|x| self.buffer.get(x, y).symbol.to_string())
            .collect()
    }

    fn find(&self, needle: &str) -> Option<(u32, u32)> {
        (0..self.buffer.area.height).find_map(|y| {
            let line = self.line(y);
            line.find(needle)
                .map(|byte| (line[..byte].chars().count() as u32, y))
        })
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
}

impl Backend for Screen {
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

struct Harness {
    screen: Screen,
    state: AppState,
    config: RunConfig,
    app: App,
}

impl Harness {
    fn new() -> Self {
        Self {
            screen: Screen {
                buffer: Buffer::empty(Rect::new(0, 0, 60, 16)),
            },
            state: AppState::new(),
            config: RunConfig::default(),
            app: App::new(),
        }
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.screen.buffer.reset();
        let app = &mut self.app;
        slt::frame_owned(
            &mut self.screen,
            &mut self.state,
            &self.config,
            events,
            &mut |ui| tool_log(ui, app),
        )
        .expect("frame");
    }

    /// Render until queued input (later presses of a batch) is drained.
    fn settle(&mut self) {
        while self.state.has_pending_input() {
            self.frame(Vec::new());
        }
        self.frame(Vec::new());
    }

    fn click_text(&mut self, needle: &str) {
        let (x, y) = self.screen.find(needle).expect("text on screen");
        self.frame(EventBuilder::new().click(x, y).mouse_up(x, y).build());
        self.frame(Vec::new());
    }
}

fn wheel_burst(x: u32, y: u32, n: usize) -> Vec<Event> {
    (0..n)
        .map(|_| {
            Event::Mouse(MouseEvent::new(
                MouseKind::ScrollDown,
                x,
                y,
                KeyModifiers::NONE,
                None,
                None,
            ))
        })
        .collect()
}

#[test]
fn expandable_tool_call_log_scenario() {
    let mut h = Harness::new();
    h.frame(Vec::new());
    h.frame(Vec::new());

    // 1. A click on a header toggles exactly that block, once.
    h.click_text("#01 grep");
    assert!(h.app.calls[1].open, "header click opened block #01");
    assert_eq!(h.app.toggles, 1, "one press, one activation");
    assert!(h.screen.find("out 01.0").is_some(), "body rendered");

    // Two header clicks in one batch both toggle (#442). Presses are
    // dispatched one per frame, like sequential DOM events: each is
    // hit-tested against the layout the previous one produced. Clicking the
    // lower header first keeps the upper one in place.
    let (x0, y0) = h.screen.find("#00 read_file").expect("header #00");
    let (x3, y3) = h.screen.find("#03 read_file").expect("header #03");
    h.frame(
        EventBuilder::new()
            .click(x3, y3)
            .mouse_up(x3, y3)
            .click(x0, y0)
            .mouse_up(x0, y0)
            .build(),
    );
    assert!(
        h.app.calls[3].open && !h.app.calls[0].open,
        "first press first"
    );
    h.settle();
    assert!(h.app.calls[0].open && h.app.calls[3].open);
    assert_eq!(h.app.toggles, 3);

    // 2. Wheel over block #01's nested scrollable: the block scrolls to its
    // end, then the remaining events scroll the outer list.
    let (bx, by) = h.screen.find("out 01.").expect("nested body visible");
    h.frame(wheel_burst(bx, by, 10));
    h.frame(Vec::new());
    let body = &h.app.calls[1].body;
    assert!(body.max_offset() > 0);
    assert_eq!(body.offset, body.max_offset(), "nested block at its end");
    assert!(h.app.scroll.offset > 0, "wheel chained to the outer list");

    // 3. Scroll the outer list past block #00, then collapse it from code
    // (it is above the viewport): the visible text stays put.
    let outer_max = h.app.scroll.max_offset();
    let target = 10.min(outer_max);
    h.app.scroll.offset = target;
    h.frame(Vec::new());
    h.frame(Vec::new());
    assert!(
        h.screen.find("#00 read_file").is_none(),
        "block #00 is above the viewport"
    );
    let before = h.screen.line(4);
    let offset_before = h.app.scroll.offset;
    h.app.calls[0].open = false;
    h.frame(Vec::new());
    assert_eq!(h.screen.line(4), before, "anchored on the collapse frame");
    h.frame(Vec::new());
    assert_eq!(h.screen.line(4), before);
    assert_eq!(
        h.app.scroll.offset,
        offset_before - 4,
        "offset shrank by the collapsed body's 4 rows"
    );

    // 4. Dragging the scrollbar thumb scrolls without selecting text.
    let (tx, ty) = h.screen.find("█").expect("scrollbar thumb");
    let offset = h.app.scroll.offset;
    h.frame(EventBuilder::new().click(tx, ty).drag(tx, ty + 6).build());
    assert!(h.app.scroll.offset > offset, "thumb drag scrolled");
    assert_eq!(h.screen.reversed_cells(), 0, "no selection highlight");
    h.frame(EventBuilder::new().drag(tx - 20, ty + 5).build());
    assert_eq!(h.screen.reversed_cells(), 0, "capture holds off the bar");
    h.frame(EventBuilder::new().mouse_up(tx - 20, ty + 5).build());
    h.frame(Vec::new());
    assert_eq!(h.screen.reversed_cells(), 0);
    assert_eq!(h.app.toggles, 3, "the drag toggled nothing");
}

/// The clipboard half of step 4 through the real flush pipeline.
#[cfg(feature = "pty-test")]
#[test]
fn scrollbar_drag_in_the_tool_log_never_writes_the_clipboard() {
    let mut app = App::new();
    let mut pb = slt::PtyBackend::new(60, 16);
    pb.render(|ui| tool_log(ui, &mut app));
    pb.render(|ui| tool_log(ui, &mut app));
    let mut probe = slt::TestBackend::new(60, 16);
    let mut probe_app = App::new();
    probe.render(|ui| tool_log(ui, &mut probe_app));
    probe.render(|ui| tool_log(ui, &mut probe_app));
    let (tx, ty) = probe.find_text("█").expect("scrollbar thumb");
    for events in [
        EventBuilder::new().click(tx, ty).build(),
        EventBuilder::new()
            .drag(tx, ty + 3)
            .drag(tx - 20, ty + 5)
            .build(),
        EventBuilder::new().mouse_up(tx - 20, ty + 5).build(),
        Vec::new(),
    ] {
        pb.render_with_events(events, |ui| tool_log(ui, &mut app));
    }
    assert!(app.scroll.offset > 0, "thumb drag scrolled");
    for raw in pb.frames_raw() {
        assert!(
            !raw.windows(5).any(|w| w == b"\x1b]52;"),
            "scrollbar drag wrote the clipboard"
        );
    }
}
