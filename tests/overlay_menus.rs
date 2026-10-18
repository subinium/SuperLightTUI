//! Issue #459: element-anchored popovers, context menus, the menu bar, and
//! the select dropdown floating over the layout.

use slt::widgets::{MenuBarState, MenuItem, MenuState, SelectState};
use slt::{
    Context, Event, EventBuilder, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseKind,
    Placement, Rect, TestBackend,
};

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

fn key(code: KeyCode) -> Vec<Event> {
    EventBuilder::new().key_code(code).build()
}

/// Render one frame with `events`, keeping focus state across frames.
fn step(tb: &mut TestBackend, events: Vec<Event>, f: impl FnOnce(&mut Context)) {
    tb.sequence().events(events, f).run();
}

// ── popover placement ───────────────────────────────────────────────

#[test]
fn popover_below_near_the_bottom_edge_opens_above() {
    let mut tb = TestBackend::new(40, 10);
    let mut side = Placement::Below;
    tb.render(|ui| {
        let pop = ui.popover(Rect::new(2, 8, 6, 1), Placement::Below, |ui| {
            ui.text("hello");
            ui.text("world");
        });
        side = pop.placement;
    });
    assert_eq!(side, Placement::Above);
    let (_, y) = tb.find_text("world").expect("popover body");
    assert!(y < 8, "panel sits above the anchor row, got y={y}");
    let (_, top) = tb.find_text("hello").expect("popover body");
    assert_eq!(top, 8 - 4 + 1, "4-row panel ends right above the anchor");
}

#[test]
fn popover_near_the_right_edge_is_clamped_inside() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        let _ = ui.popover(Rect::new(36, 2, 4, 1), Placement::Below, |ui| {
            ui.text("a long popover");
        });
    });
    let (x, y) = tb.find_text("a long popover").expect("popover body");
    assert_eq!(y, 4, "below the anchor, inside the border");
    assert_eq!(
        x + 14 + 2,
        40,
        "padding and right border on the last columns"
    );
}

#[test]
fn popover_right_flips_left_and_above_flips_below() {
    let mut tb = TestBackend::new(40, 10);
    let mut sides = Vec::new();
    tb.render(|ui| {
        sides.push(
            ui.popover(Rect::new(34, 3, 4, 1), Placement::Right, |ui| {
                ui.text("r");
            })
            .placement,
        );
        sides.push(
            ui.popover(Rect::new(4, 0, 4, 1), Placement::Above, |ui| {
                ui.text("a");
            })
            .placement,
        );
        sides.push(
            ui.popover(Rect::new(1, 3, 1, 1), Placement::Left, |ui| {
                ui.text("l");
            })
            .placement,
        );
    });
    assert_eq!(sides, [Placement::Left, Placement::Below, Placement::Right]);
}

#[test]
fn popover_does_not_reflow_the_layout() {
    let scene = |ui: &mut Context, open: bool| {
        let info = ui.button("Info");
        if open {
            let _ = ui.popover(info.rect, Placement::Below, |ui| {
                ui.text("details");
            });
        }
        // Indented past the panel so the popover never covers it.
        ui.text(format!("{:20}after", ""));
    };
    let mut tb = TestBackend::new(30, 10);
    tb.render(|ui| scene(ui, false));
    let before = tb.find_text("after").expect("after");
    tb.render(|ui| scene(ui, true));
    tb.render(|ui| scene(ui, true));
    assert_eq!(tb.find_text("after"), Some(before));
    tb.assert_contains("details");
}

#[test]
fn popover_reports_light_dismiss() {
    let open = true;
    let mut dismissed = Vec::new();
    let mut tb = TestBackend::new(40, 10);
    let frame = |tb: &mut TestBackend, events: Vec<Event>, dismissed: &mut Vec<bool>| {
        step(tb, events, |ui| {
            let anchor = Rect::new(2, 1, 6, 1);
            if open {
                let pop = ui.popover(anchor, Placement::Below, |ui| {
                    ui.text("panel");
                });
                dismissed.push(pop.dismissed);
            }
        });
    };
    frame(&mut tb, vec![], &mut dismissed);
    // A press inside the panel or on the anchor does not dismiss.
    let (x, y) = tb.find_text("panel").expect("panel");
    frame(
        &mut tb,
        vec![press(MouseButton::Left, x, y)],
        &mut dismissed,
    );
    frame(
        &mut tb,
        vec![press(MouseButton::Left, 3, 1)],
        &mut dismissed,
    );
    // Outside press and Esc do.
    frame(
        &mut tb,
        vec![press(MouseButton::Left, 30, 8)],
        &mut dismissed,
    );
    frame(&mut tb, key(KeyCode::Esc), &mut dismissed);
    assert_eq!(dismissed, [false, false, false, true, true]);
}

// ── context menu ────────────────────────────────────────────────────

fn file_menu() -> MenuState {
    MenuState::new(vec![
        MenuItem::new("open", "Open"),
        MenuItem::new("rename", "Rename").shortcut("F2"),
        MenuItem::separator(),
        MenuItem::new("delete", "Delete").disabled(true),
    ])
}

#[derive(Default, Debug)]
struct MenuFrame {
    activated: Option<String>,
    trigger_focused: bool,
    other_clicked: bool,
}

fn menu_scene(ui: &mut Context, menu: &mut MenuState, out: &mut MenuFrame) {
    let file = ui.button("notes.md");
    out.trigger_focused = file.focused;
    if let Some(id) = ui.context_menu(menu, &file) {
        out.activated = Some(id);
    }
    for _ in 0..4 {
        ui.text("");
    }
    // Right of where the menu opens, so the menu never covers it.
    let _ = ui.row(|ui| {
        ui.text(format!("{:24}", ""));
        out.other_clicked = ui.button("Other").clicked;
    });
}

#[test]
fn right_click_opens_at_the_pointer_and_down_enter_activates() {
    let mut menu = file_menu();
    let mut tb = TestBackend::new(40, 12);
    let mut out = MenuFrame::default();
    step(&mut tb, vec![], |ui| menu_scene(ui, &mut menu, &mut out));
    let (x, y) = tb.find_text("notes.md").expect("trigger");
    step(&mut tb, vec![press(MouseButton::Right, x + 2, y)], |ui| {
        menu_scene(ui, &mut menu, &mut out)
    });
    assert!(menu.is_open());
    let (ox, oy) = tb.find_text("Open").expect("menu row");
    assert_eq!(
        (ox, oy),
        (x + 2 + 2, y + 1),
        "top-left corner at the pointer"
    );
    step(&mut tb, key(KeyCode::Down), |ui| {
        menu_scene(ui, &mut menu, &mut out)
    });
    let mut out = MenuFrame::default();
    step(&mut tb, key(KeyCode::Enter), |ui| {
        menu_scene(ui, &mut menu, &mut out)
    });
    assert_eq!(out.activated.as_deref(), Some("rename"));
    assert!(!menu.is_open());
}

#[test]
fn keyboard_skips_separators_and_disabled_entries() {
    let mut menu = file_menu();
    let mut tb = TestBackend::new(40, 12);
    let mut out = MenuFrame::default();
    step(&mut tb, vec![], |ui| menu_scene(ui, &mut menu, &mut out));
    step(
        &mut tb,
        EventBuilder::new()
            .key_with(KeyCode::F(10), KeyModifiers::SHIFT)
            .build(),
        |ui| menu_scene(ui, &mut menu, &mut out),
    );
    assert!(menu.is_open(), "Shift+F10 opens the focused trigger's menu");
    // Down twice wraps from Rename past the separator and disabled Delete.
    step(&mut tb, key(KeyCode::Down), |ui| {
        menu_scene(ui, &mut menu, &mut out)
    });
    step(&mut tb, key(KeyCode::Down), |ui| {
        menu_scene(ui, &mut menu, &mut out)
    });
    assert_eq!(menu.cursor(), Some(0));
}

#[test]
fn menu_key_opens_and_esc_returns_focus_to_the_trigger() {
    let mut menu = file_menu();
    let mut tb = TestBackend::new(40, 12);
    let mut out = MenuFrame::default();
    step(&mut tb, vec![], |ui| menu_scene(ui, &mut menu, &mut out));
    assert!(out.trigger_focused);
    step(&mut tb, key(KeyCode::Menu), |ui| {
        menu_scene(ui, &mut menu, &mut out)
    });
    assert!(menu.is_open());
    step(&mut tb, vec![], |ui| menu_scene(ui, &mut menu, &mut out));
    assert!(!out.trigger_focused, "the open menu owns the keyboard");
    step(&mut tb, key(KeyCode::Esc), |ui| {
        menu_scene(ui, &mut menu, &mut out)
    });
    assert!(!menu.is_open());
    let mut out = MenuFrame::default();
    step(&mut tb, vec![], |ui| menu_scene(ui, &mut menu, &mut out));
    assert!(out.trigger_focused, "focus returned to the trigger");
}

#[test]
fn outside_click_closes_without_clicking_the_widget_underneath() {
    let mut menu = file_menu();
    let mut tb = TestBackend::new(40, 12);
    let mut out = MenuFrame::default();
    step(&mut tb, vec![], |ui| menu_scene(ui, &mut menu, &mut out));
    let (x, y) = tb.find_text("notes.md").expect("trigger");
    step(&mut tb, vec![press(MouseButton::Right, x, y)], |ui| {
        menu_scene(ui, &mut menu, &mut out)
    });
    assert!(menu.is_open());
    let (bx, by) = tb.find_text("Other").expect("other button");
    let mut out = MenuFrame::default();
    step(&mut tb, vec![press(MouseButton::Left, bx, by)], |ui| {
        menu_scene(ui, &mut menu, &mut out)
    });
    assert!(!menu.is_open(), "outside click closes the menu");
    assert!(!out.other_clicked, "the click is not delivered underneath");
    let mut out = MenuFrame::default();
    step(&mut tb, vec![press(MouseButton::Left, bx, by)], |ui| {
        menu_scene(ui, &mut menu, &mut out)
    });
    assert!(out.other_clicked, "once closed, clicks reach widgets again");
}

#[test]
fn clicking_an_entry_activates_it_and_disabled_entries_ignore_clicks() {
    let mut menu = file_menu();
    let mut tb = TestBackend::new(40, 12);
    let mut out = MenuFrame::default();
    step(&mut tb, vec![], |ui| menu_scene(ui, &mut menu, &mut out));
    menu.open_at(Rect::new(10, 2, 0, 0));
    step(&mut tb, vec![], |ui| menu_scene(ui, &mut menu, &mut out));
    let (dx, dy) = tb.find_text("Delete").expect("disabled row");
    let mut out = MenuFrame::default();
    step(&mut tb, vec![press(MouseButton::Left, dx, dy)], |ui| {
        menu_scene(ui, &mut menu, &mut out)
    });
    assert_eq!(out.activated, None);
    assert!(menu.is_open());
    let (rx, ry) = tb.find_text("Rename").expect("row");
    let mut out = MenuFrame::default();
    step(&mut tb, vec![press(MouseButton::Left, rx, ry)], |ui| {
        menu_scene(ui, &mut menu, &mut out)
    });
    assert_eq!(out.activated.as_deref(), Some("rename"));
}

#[test]
fn submenus_open_with_right_and_close_with_left() {
    let mut menu = MenuState::new(vec![
        MenuItem::new("cut", "Cut"),
        MenuItem::submenu(
            "share",
            "Share",
            vec![
                MenuItem::new("mail", "Mail"),
                MenuItem::new("link", "Copy link"),
            ],
        ),
    ]);
    let mut tb = TestBackend::new(50, 12);
    let mut out = MenuFrame::default();
    step(&mut tb, vec![], |ui| menu_scene(ui, &mut menu, &mut out));
    menu.open_at(Rect::new(2, 2, 0, 0));
    step(&mut tb, vec![], |ui| menu_scene(ui, &mut menu, &mut out));
    for code in [KeyCode::Down, KeyCode::Right] {
        step(&mut tb, key(code), |ui| menu_scene(ui, &mut menu, &mut out));
    }
    tb.assert_contains("Copy link");
    let (sx, _) = tb.find_text("Mail").expect("submenu");
    let (px, _) = tb.find_text("Share").expect("parent");
    assert!(sx > px, "submenu opens to the right");
    step(&mut tb, key(KeyCode::Left), |ui| {
        menu_scene(ui, &mut menu, &mut out)
    });
    tb.assert_not_contains("Copy link");
    assert!(menu.is_open());
}

#[test]
fn snapshot_context_menu() {
    let mut menu = MenuState::new(vec![
        MenuItem::new("copy", "Copy").shortcut("Ctrl+C"),
        MenuItem::new("paste", "Paste")
            .shortcut("Ctrl+V")
            .disabled(true),
        MenuItem::separator(),
        MenuItem::submenu("share", "Share", vec![MenuItem::new("mail", "Mail")]),
    ]);
    let mut tb = TestBackend::new(40, 9);
    let mut out = MenuFrame::default();
    step(&mut tb, vec![], |ui| menu_scene(ui, &mut menu, &mut out));
    menu.open_at(Rect::new(4, 1, 0, 0));
    step(&mut tb, vec![], |ui| menu_scene(ui, &mut menu, &mut out));
    insta::assert_snapshot!(tb.to_string_trimmed());
}

// ── menu bar ────────────────────────────────────────────────────────

fn app_bar() -> MenuBarState {
    MenuBarState::new(vec![
        (
            "File",
            vec![
                MenuItem::new("new", "New").shortcut("Ctrl+N"),
                MenuItem::new("quit", "Quit"),
            ],
        ),
        (
            "Edit",
            vec![
                MenuItem::new("undo", "Undo").disabled(true),
                MenuItem::new("redo", "Redo"),
                MenuItem::submenu(
                    "find",
                    "Find",
                    vec![MenuItem::new("find-next", "Find next")],
                ),
            ],
        ),
        ("Help", vec![MenuItem::new("about", "About")]),
    ])
}

fn bar_scene(ui: &mut Context, bar: &mut MenuBarState, activated: &mut Option<String>) {
    if let Some(id) = ui.menu_bar(bar) {
        *activated = Some(id);
    }
    ui.text("document");
}

#[test]
fn menu_bar_keyboard_flow_activates_and_skips_disabled() {
    let mut bar = app_bar();
    let mut tb = TestBackend::new(40, 10);
    let mut activated = None;
    step(&mut tb, vec![], |ui| {
        bar_scene(ui, &mut bar, &mut activated)
    });
    for code in [KeyCode::F(10), KeyCode::Right, KeyCode::Down] {
        step(&mut tb, key(code), |ui| {
            bar_scene(ui, &mut bar, &mut activated)
        });
    }
    assert!(bar.is_open());
    assert_eq!(bar.selected(), 1);
    step(&mut tb, key(KeyCode::Enter), |ui| {
        bar_scene(ui, &mut bar, &mut activated)
    });
    assert_eq!(
        activated.as_deref(),
        Some("redo"),
        "disabled Undo is skipped"
    );
    assert!(!bar.is_active());
}

#[test]
fn menu_bar_alt_mnemonic_click_and_escape() {
    let mut bar = app_bar();
    let mut tb = TestBackend::new(40, 10);
    let mut activated = None;
    step(&mut tb, vec![], |ui| {
        bar_scene(ui, &mut bar, &mut activated)
    });
    step(
        &mut tb,
        EventBuilder::new()
            .key_with(KeyCode::Char('h'), KeyModifiers::ALT)
            .build(),
        |ui| bar_scene(ui, &mut bar, &mut activated),
    );
    assert!(bar.is_open());
    assert_eq!(bar.selected(), 2);
    tb.assert_contains("About");
    // Left switches the open dropdown to the previous menu.
    step(&mut tb, key(KeyCode::Left), |ui| {
        bar_scene(ui, &mut bar, &mut activated)
    });
    assert_eq!(bar.selected(), 1);
    tb.assert_contains("Redo");
    // Esc closes the dropdown, a second Esc leaves the bar.
    step(&mut tb, key(KeyCode::Esc), |ui| {
        bar_scene(ui, &mut bar, &mut activated)
    });
    assert!(bar.is_active() && !bar.is_open());
    step(&mut tb, key(KeyCode::Esc), |ui| {
        bar_scene(ui, &mut bar, &mut activated)
    });
    assert!(!bar.is_active());
    // Clicking a title opens it; clicking the document closes the bar.
    let (fx, fy) = tb.find_text("File").expect("title");
    step(&mut tb, vec![press(MouseButton::Left, fx, fy)], |ui| {
        bar_scene(ui, &mut bar, &mut activated)
    });
    assert!(bar.is_open());
    assert_eq!(bar.selected(), 0);
    step(&mut tb, vec![press(MouseButton::Left, 30, 8)], |ui| {
        bar_scene(ui, &mut bar, &mut activated)
    });
    assert!(!bar.is_active());
    assert_eq!(activated, None);
}

#[test]
fn snapshot_menu_bar_with_submenu() {
    let mut bar = app_bar();
    let mut tb = TestBackend::new(40, 8);
    let mut activated = None;
    step(&mut tb, vec![], |ui| {
        bar_scene(ui, &mut bar, &mut activated)
    });
    for code in [
        KeyCode::F(10),
        KeyCode::Right,
        KeyCode::Down,
        KeyCode::Down,
        KeyCode::Right,
    ] {
        step(&mut tb, key(code), |ui| {
            bar_scene(ui, &mut bar, &mut activated)
        });
    }
    insta::assert_snapshot!(tb.to_string_trimmed());
}

// ── select dropdown ─────────────────────────────────────────────────

#[test]
fn opening_a_select_does_not_move_the_widget_after_it() {
    let mut state = SelectState::new(vec!["Apple", "Banana", "Cherry"]);
    let mut tb = TestBackend::new(30, 12);
    let mut after = Rect::default();
    let mut scene = |ui: &mut Context, after: &mut Rect| {
        let _ = ui.select(&mut state);
        // The dropdown may paint over this row; its layout rect must not move.
        *after = ui
            .container()
            .row(|ui| {
                ui.text("after");
            })
            .rect;
    };
    step(&mut tb, vec![], |ui| scene(ui, &mut after));
    step(&mut tb, vec![], |ui| scene(ui, &mut after));
    let before = after;
    assert_eq!(before.y, 3, "right below the 3-row trigger");
    step(&mut tb, key(KeyCode::Enter), |ui| scene(ui, &mut after));
    step(&mut tb, vec![], |ui| scene(ui, &mut after));
    step(&mut tb, vec![], |ui| scene(ui, &mut after));
    tb.assert_contains("Banana");
    assert_eq!(after, before);
}

#[test]
fn select_dropdown_flips_above_at_the_bottom_and_outside_click_closes() {
    let mut state = SelectState::new(vec!["Apple", "Banana", "Cherry"]);
    let mut clicked = false;
    let mut tb = TestBackend::new(30, 10);
    let mut scene = |ui: &mut Context, clicked: &mut bool| {
        for _ in 0..4 {
            ui.text("");
        }
        let _ = ui.row(|ui| {
            let _ = ui.select(&mut state);
            ui.text(format!("{:6}", ""));
            *clicked |= ui.button("Top").clicked;
        });
    };
    step(&mut tb, vec![], |ui| scene(ui, &mut clicked));
    // The select is the first focusable: Enter opens it.
    step(&mut tb, key(KeyCode::Enter), |ui| scene(ui, &mut clicked));
    step(&mut tb, vec![], |ui| scene(ui, &mut clicked));
    let (_, apple) = tb.find_text("Apple").expect("dropdown row");
    let (_, trigger) = tb.find_text("▲").expect("open trigger");
    assert!(apple < trigger, "dropdown opened above the trigger");
    let (tx, ty) = tb.find_text("Top").expect("button");
    step(&mut tb, vec![press(MouseButton::Left, tx, ty)], |ui| {
        scene(ui, &mut clicked)
    });
    assert!(!clicked, "the outside click is swallowed");
    tb.assert_not_contains("▲");
}

#[test]
fn select_dropdown_inside_a_modal_paints_above_the_modal() {
    let mut state = SelectState::new(vec!["Apple", "Banana", "Cherry"]);
    let mut tb = TestBackend::new(40, 14);
    let mut scene = |ui: &mut Context| {
        let _ = ui.modal(|ui| {
            let _ = ui.bordered(slt::Border::Rounded).p(1).col(|ui| {
                let _ = ui.select(&mut state);
                ui.text("modal footer line");
            });
        });
    };
    step(&mut tb, vec![], &mut scene);
    step(&mut tb, key(KeyCode::Enter), &mut scene);
    step(&mut tb, vec![], &mut scene);
    // An overlay opened inside a modal paints over the modal's own content.
    tb.assert_contains("Banana");
    tb.assert_contains("Cherry");
    let (x, y) = tb.find_text("Cherry").expect("dropdown row");
    let dim_bg = tb.buffer().get(x, y).style;
    assert!(
        !dim_bg.modifiers.contains(slt::Modifiers::DIM),
        "the dropdown is not dimmed by the modal: {dim_bg:?}"
    );
}

#[test]
fn clicking_a_dropdown_row_does_not_focus_the_widget_underneath() {
    let mut state = SelectState::new(vec!["Apple", "Banana", "Cherry"]);
    let mut input = slt::widgets::TextInputState::new();
    let mut focused = (false, false);
    let mut tb = TestBackend::new(30, 12);
    let mut scene = |ui: &mut Context, focused: &mut (bool, bool)| {
        let select = ui.select(&mut state);
        let field = ui.text_input(&mut input);
        *focused = (select.focused, field.focused);
    };
    step(&mut tb, vec![], |ui| scene(ui, &mut focused));
    step(&mut tb, key(KeyCode::Enter), |ui| scene(ui, &mut focused));
    step(&mut tb, vec![], |ui| scene(ui, &mut focused));
    // "Banana" is drawn over the text input below the select.
    let (x, y) = tb.find_text("Banana").expect("dropdown row");
    step(&mut tb, vec![press(MouseButton::Left, x, y)], |ui| {
        scene(ui, &mut focused)
    });
    step(&mut tb, vec![], |ui| scene(ui, &mut focused));
    assert_eq!(focused, (true, false), "focus stays on the select");
    assert_eq!(state.selected_item(), Some("Banana"));
}

#[test]
fn a_programmatically_opened_select_draws_from_the_next_frame() {
    let mut state = SelectState::new(vec!["Apple", "Banana", "Cherry"]);
    state.open = true;
    let mut tb = TestBackend::new(30, 10);
    tb.render(|ui| {
        let _ = ui.select(&mut state);
    });
    tb.assert_not_contains("Banana");
    tb.render(|ui| {
        let _ = ui.select(&mut state);
    });
    tb.assert_contains("Banana");
}
