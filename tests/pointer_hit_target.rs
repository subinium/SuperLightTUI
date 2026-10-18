//! Issue #439: each press resolves to the topmost, innermost hit target.
//!
//! Overlays block the widgets underneath, a child wins over its parent, and
//! ancestors opt back in with `ContainerBuilder::bubble_clicks`.

use proptest::prelude::*;
use slt::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseKind};
use slt::{Context, EventBuilder, TestBackend};

fn press(button: MouseButton, x: u32, y: u32) -> Event {
    Event::Mouse(MouseEvent::new(
        MouseKind::Down(button),
        x,
        y,
        KeyModifiers::NONE,
        None,
        None,
    ))
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
struct Hits {
    under: u32,
    popup: u32,
}

fn overlay_view(ui: &mut Context, hits: &mut Hits) {
    if ui
        .container()
        .w(40)
        .h(10)
        .col(|ui| {
            ui.text("UNDER");
        })
        .clicked
    {
        hits.under += 1;
    }
    let _ = ui.overlay(|ui| {
        if ui.button("POPUP").clicked {
            hits.popup += 1;
        }
    });
}

#[test]
fn overlay_button_blocks_the_container_underneath() {
    let mut tb = TestBackend::new(40, 10);
    let mut hits = Hits::default();
    tb.render(|ui| overlay_view(ui, &mut hits));

    let (x, y) = tb.find_text("POPUP").expect("popup button rendered");
    tb.run_with_events(EventBuilder::new().click(x, y).build(), |ui| {
        overlay_view(ui, &mut hits)
    });
    assert_eq!(hits, Hits { under: 0, popup: 1 });

    // Outside the button but inside the container: the container wins.
    tb.run_with_events(EventBuilder::new().click(20, 6).build(), |ui| {
        overlay_view(ui, &mut hits)
    });
    assert_eq!(hits, Hits { under: 1, popup: 1 });
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
struct Nested {
    inner: u32,
    outer: u32,
    inner_any: bool,
    outer_any: bool,
    inner_right: u32,
    outer_right: u32,
    inner_double: u32,
    outer_double: u32,
}

fn nested_view(ui: &mut Context, bubble: bool, n: &mut Nested) {
    let outer = ui.container().p(1).with_if(bubble, |b| b.bubble_clicks());
    let r = outer.col(|ui| {
        let inner = ui.container().col(|ui| {
            ui.text("INNER");
        });
        n.inner += u32::from(inner.clicked);
        n.inner_any |= inner.clicked_any;
        n.inner_right += u32::from(inner.right_clicked);
        n.inner_double += u32::from(inner.double_clicked);
    });
    n.outer += u32::from(r.clicked);
    n.outer_any |= r.clicked_any;
    n.outer_right += u32::from(r.right_clicked);
    n.outer_double += u32::from(r.double_clicked);
}

#[test]
fn nested_clickable_containers_fire_innermost_only() {
    let mut tb = TestBackend::new(20, 5);
    let mut n = Nested::default();
    tb.render(|ui| nested_view(ui, false, &mut n));
    let (x, y) = tb.find_text("INNER").expect("inner rendered");

    tb.run_with_events(EventBuilder::new().click(x, y).build(), |ui| {
        nested_view(ui, false, &mut n)
    });
    assert_eq!((n.inner, n.outer), (1, 0), "child wins over parent");
    assert!(n.inner_any && n.outer_any, "clicked_any keeps containment");

    // Padding cell of the outer container: no inner slot there.
    tb.run_with_events(EventBuilder::new().click(0, 0).build(), |ui| {
        nested_view(ui, false, &mut n)
    });
    assert_eq!((n.inner, n.outer), (1, 1));
}

#[test]
fn bubble_clicks_lets_the_ancestor_fire_too() {
    let mut tb = TestBackend::new(20, 5);
    let mut n = Nested::default();
    tb.render(|ui| nested_view(ui, true, &mut n));
    let (x, y) = tb.find_text("INNER").expect("inner rendered");
    tb.run_with_events(EventBuilder::new().click(x, y).build(), |ui| {
        nested_view(ui, true, &mut n)
    });
    assert_eq!((n.inner, n.outer), (1, 1));
    assert!(n.inner_any && n.outer_any);
}

#[test]
fn right_and_double_clicks_follow_the_winner_rule() {
    let mut tb = TestBackend::new(20, 5);
    let mut n = Nested::default();
    tb.render(|ui| nested_view(ui, false, &mut n));
    let (x, y) = tb.find_text("INNER").expect("inner rendered");

    tb.run_with_events(vec![press(MouseButton::Right, x, y)], |ui| {
        nested_view(ui, false, &mut n)
    });
    assert_eq!((n.inner_right, n.outer_right), (1, 0));

    // Two left presses on the same cell inside the double-click window.
    let double = EventBuilder::new()
        .click(x, y)
        .mouse_up(x, y)
        .click(x, y)
        .build();
    tb.run_with_events(double, |ui| nested_view(ui, false, &mut n));
    assert_eq!((n.inner_double, n.outer_double), (1, 0));

    // With bubbling the ancestor also sees right and double clicks.
    let mut b = Nested::default();
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| nested_view(ui, true, &mut b));
    tb.run_with_events(vec![press(MouseButton::Right, x, y)], |ui| {
        nested_view(ui, true, &mut b)
    });
    assert_eq!((b.inner_right, b.outer_right), (1, 1));
}

#[test]
fn hovered_top_marks_only_the_winner() {
    let mut tb = TestBackend::new(20, 5);
    let mut tops = (false, false);
    let mut hovers = (false, false);
    let view = |ui: &mut Context, tops: &mut (bool, bool), hovers: &mut (bool, bool)| {
        let outer = ui.container().p(1).col(|ui| {
            let inner = ui.container().col(|ui| {
                ui.text("INNER");
            });
            tops.0 = inner.hovered_top;
            hovers.0 = inner.hovered;
        });
        tops.1 = outer.hovered_top;
        hovers.1 = outer.hovered;
    };
    tb.render(|ui| view(ui, &mut tops, &mut hovers));
    let (x, y) = tb.find_text("INNER").expect("inner rendered");
    tb.run_with_events(vec![Event::mouse_move(x, y)], |ui| {
        view(ui, &mut tops, &mut hovers)
    });
    assert_eq!(hovers, (true, true), "hovered stays containment-based");
    assert_eq!(tops, (true, false), "hovered_top marks the winner only");
}

#[test]
fn tooltip_overlay_does_not_steal_clicks() {
    // A tooltip panel spans the full area as an overlay; it must stay
    // transparent so the next press still reaches widgets underneath.
    let mut tb = TestBackend::new(40, 10);
    let mut clicks = 0;
    let view = |ui: &mut Context, clicks: &mut u32| {
        let _ = ui.button("SAVE").on_hover(ui, "Write the file");
        if ui.button("OTHER").clicked {
            *clicks += 1;
        }
    };
    tb.render(|ui| view(ui, &mut clicks));
    let (sx, sy) = tb.find_text("SAVE").expect("save rendered");
    let (ox, oy) = tb.find_text("OTHER").expect("other rendered");
    tb.run_with_events(vec![Event::mouse_move(sx, sy)], |ui| view(ui, &mut clicks));
    tb.assert_contains("Write the file");
    tb.run_with_events(EventBuilder::new().click(ox, oy).build(), |ui| {
        view(ui, &mut clicks)
    });
    assert_eq!(clicks, 1);
}

/// A generated container: parent index (or root), padding, overlay flag.
#[derive(Debug, Clone)]
struct NodeSpec {
    parent: Option<usize>,
    pad: u32,
    overlay: bool,
}

fn node_specs() -> impl Strategy<Value = Vec<NodeSpec>> {
    prop::collection::vec(
        (any::<prop::sample::Index>(), 0u32..2, any::<bool>()),
        1..10,
    )
    .prop_map(|raw| {
        raw.into_iter()
            .enumerate()
            .map(|(i, (parent, pad, overlay))| NodeSpec {
                // Roughly half the nodes nest under an earlier node.
                parent: (i > 0 && parent.index(2) == 1).then(|| parent.index(i)),
                pad,
                overlay,
            })
            .collect()
    })
}

fn render_node(ui: &mut Context, specs: &[NodeSpec], index: usize, clicked: &mut [u32]) {
    let r = ui.container().p(specs[index].pad).col(|ui| {
        ui.text(format!("N{index}"));
        for child in (0..specs.len()).filter(|&c| specs[c].parent == Some(index)) {
            render_node(ui, specs, child, clicked);
        }
    });
    clicked[index] += u32::from(r.clicked);
}

fn render_tree(ui: &mut Context, specs: &[NodeSpec], clicked: &mut [u32]) {
    for root in (0..specs.len()).filter(|&i| specs[i].parent.is_none()) {
        if specs[root].overlay {
            let _ = ui.overlay(|ui| render_node(ui, specs, root, clicked));
        } else {
            render_node(ui, specs, root, clicked);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn at_most_one_slot_reports_clicked_per_press(
        specs in node_specs(),
        x in 0u32..30,
        y in 0u32..12,
    ) {
        let mut tb = TestBackend::new(30, 12);
        let mut clicked = vec![0u32; specs.len()];
        tb.render(|ui| render_tree(ui, &specs, &mut clicked));
        clicked.iter_mut().for_each(|c| *c = 0);
        tb.run_with_events(EventBuilder::new().click(x, y).build(), |ui| {
            render_tree(ui, &specs, &mut clicked)
        });
        let total: u32 = clicked.iter().sum();
        prop_assert!(total <= 1, "one press activated {total} slots: {clicked:?}");
    }
}
