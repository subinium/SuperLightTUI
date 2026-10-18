//! Issue #440: wheel events chain from an inner scrollable at its limit to
//! the enclosing scrollable, unless the inner one opts out with
//! `overscroll_contain()`.

use slt::event::{Event, KeyModifiers, MouseEvent, MouseKind};
use slt::{Border, Context, EventBuilder, ScrollState, TestBackend};

struct Block {
    open: bool,
    inner: ScrollState,
}

struct App {
    scroll: ScrollState,
    blocks: Vec<Block>,
    contain: bool,
}

impl App {
    fn new(contain: bool) -> Self {
        Self {
            scroll: ScrollState::new(),
            blocks: (0..30)
                .map(|i| Block {
                    open: i == 1,
                    inner: ScrollState::new(),
                })
                .collect(),
            contain,
        }
    }
}

fn view(ui: &mut Context, app: &mut App) {
    let contain = app.contain;
    let blocks = &mut app.blocks;
    let _ = ui.scrollable(&mut app.scroll).grow(1).col(|ui| {
        for (i, block) in blocks.iter_mut().enumerate() {
            ui.text(format!("header #{i:02}"));
            if block.open {
                let _ = ui
                    .scrollable(&mut block.inner)
                    .max_h(6)
                    .border(Border::Single)
                    .with_if(contain, |b| b.overscroll_contain())
                    .col(|ui| {
                        for line in 0..10 {
                            ui.text(format!("nested {i:02}.{line}"));
                        }
                    });
            }
        }
    });
}

fn wheel(kind: MouseKind, x: u32, y: u32, modifiers: KeyModifiers) -> Event {
    Event::Mouse(MouseEvent::new(kind, x, y, modifiers, None, None))
}

fn burst(kind: MouseKind, x: u32, y: u32, n: usize) -> Vec<Event> {
    (0..n)
        .map(|_| wheel(kind.clone(), x, y, KeyModifiers::NONE))
        .collect()
}

fn settle(tb: &mut TestBackend, app: &mut App) {
    tb.render(|ui| view(ui, app));
    tb.render(|ui| view(ui, app));
}

#[test]
fn wheel_burst_fills_inner_then_scrolls_outer() {
    let mut tb = TestBackend::new(80, 20);
    let mut app = App::new(false);
    settle(&mut tb, &mut app);
    let (x, y) = tb.find_text("nested 01.0").expect("nested region visible");

    tb.run_with_events(burst(MouseKind::ScrollDown, x, y, 30), |ui| {
        view(ui, &mut app)
    });
    let inner = &app.blocks[1].inner;
    assert_eq!(inner.offset, inner.max_offset(), "inner reaches its end");
    assert!(inner.max_offset() > 0);
    assert!(app.scroll.offset > 0, "the rest chains to the outer list");
    let outer_after_down = app.scroll.offset;
    assert_eq!(
        outer_after_down,
        (30 - inner.max_offset()).min(app.scroll.max_offset()),
        "each event moves exactly one region; the outer clamps at its end"
    );
}

#[test]
fn wheel_up_burst_reverses_the_chain() {
    let mut tb = TestBackend::new(80, 20);
    let mut app = App::new(false);
    settle(&mut tb, &mut app);
    // Inner scrolled to its end, outer scrolled a little: the nested block
    // is still on screen (block #01 starts at row 1, outer offset 1).
    app.blocks[1].inner.offset = 6;
    app.scroll.offset = 1;
    settle(&mut tb, &mut app);
    assert_eq!(app.blocks[1].inner.offset, 6);
    let (x, y) = tb.find_text("nested 01.").expect("nested region visible");
    tb.run_with_events(burst(MouseKind::ScrollUp, x, y, 30), |ui| {
        view(ui, &mut app)
    });
    assert_eq!(app.blocks[1].inner.offset, 0, "inner back at its top first");
    assert_eq!(app.scroll.offset, 0, "remaining wheel-ups scroll the outer");
}

#[test]
fn per_frame_wheel_chains_to_outer() {
    let mut tb = TestBackend::new(80, 20);
    let mut app = App::new(false);
    settle(&mut tb, &mut app);
    let (x, y) = tb.find_text("nested 01.0").expect("nested region visible");
    for _ in 0..8 {
        tb.run_with_events(EventBuilder::new().scroll_down(x, y).build(), |ui| {
            view(ui, &mut app)
        });
    }
    let inner = &app.blocks[1].inner;
    assert_eq!(inner.offset, inner.max_offset());
    assert!(
        app.scroll.offset > 0,
        "no dead zone once the inner list ends"
    );
}

#[test]
fn overscroll_contain_keeps_the_outer_still() {
    let mut tb = TestBackend::new(80, 20);
    let mut app = App::new(true);
    settle(&mut tb, &mut app);
    let (x, y) = tb.find_text("nested 01.0").expect("nested region visible");
    tb.run_with_events(burst(MouseKind::ScrollDown, x, y, 30), |ui| {
        view(ui, &mut app)
    });
    let inner = &app.blocks[1].inner;
    assert_eq!(inner.offset, inner.max_offset());
    assert_eq!(app.scroll.offset, 0, "contained region swallows the rest");
    // Further frames at the limit stay contained too.
    tb.run_with_events(burst(MouseKind::ScrollDown, x, y, 5), |ui| {
        view(ui, &mut app)
    });
    assert_eq!(app.scroll.offset, 0);
}

struct Axes {
    outer: ScrollState,
    row: ScrollState,
}

fn axes_view(ui: &mut Context, app: &mut Axes) {
    let row = &mut app.row;
    let _ = ui.scrollable(&mut app.outer).grow(1).col(|ui| {
        let _ = ui.scrollable(row).h(1).row(|ui| {
            for i in 0..40 {
                ui.text(format!("c{i:02} "));
            }
        });
        for i in 0..40 {
            ui.text(format!("line {i:02}"));
        }
    });
}

#[test]
fn horizontal_and_shift_wheel_pick_the_axis_capable_region() {
    let mut tb = TestBackend::new(30, 10);
    let mut app = Axes {
        outer: ScrollState::new(),
        row: ScrollState::new(),
    };
    tb.render(|ui| axes_view(ui, &mut app));
    tb.render(|ui| axes_view(ui, &mut app));
    let (x, y) = tb.find_text("c00").expect("row visible");

    // Shift+wheel over the horizontal row scrolls it, not the column.
    tb.run_with_events(
        vec![
            wheel(MouseKind::ScrollDown, x, y, KeyModifiers::SHIFT),
            wheel(MouseKind::ScrollRight, x, y, KeyModifiers::NONE),
        ],
        |ui| axes_view(ui, &mut app),
    );
    assert_eq!(app.row.offset_x, 2);
    assert_eq!(app.outer.offset, 0);

    // A vertical wheel over the row skips it (wrong axis) and scrolls the
    // column.
    tb.run_with_events(burst(MouseKind::ScrollDown, x, y, 2), |ui| {
        axes_view(ui, &mut app)
    });
    assert_eq!(app.outer.offset, 2);
    assert_eq!(app.row.offset_x, 2);

    // Horizontal motion past the row's end does not chain into vertical
    // scrolling of the column.
    app.outer.offset = 0;
    tb.render(|ui| axes_view(ui, &mut app));
    tb.render(|ui| axes_view(ui, &mut app));
    let (x, y) = tb.find_text("c").expect("row visible");
    let many: Vec<Event> = (0..400)
        .map(|_| wheel(MouseKind::ScrollRight, x, y, KeyModifiers::NONE))
        .collect();
    tb.run_with_events(many, |ui| axes_view(ui, &mut app));
    assert_eq!(app.row.offset_x, app.row.max_offset_x());
    assert_eq!(app.outer.offset, 0);
}
