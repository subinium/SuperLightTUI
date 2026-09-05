#![allow(unused_must_use)]

use slt::{
    Border, Color, Context, Event, EventBuilder, KeyCode, KeyModifiers, Rect, TestBackend,
    widgets::{
        FilePickerState, ListState, RichLogState, ScrollState, SplitPaneState, TableColumn,
        TableState,
    },
};
use std::cell::RefCell;

fn virtual_rows(state: &mut ListState, variable: bool, key: Option<KeyCode>) -> Vec<usize> {
    let rows = RefCell::new(Vec::new());
    let mut backend = TestBackend::new(40, 12);
    let events = key
        .map(|key| EventBuilder::new().key_code(key).build())
        .unwrap_or_default();
    backend.run_with_events(events, |ui| {
        let render = |ui: &mut Context, raw| {
            rows.borrow_mut().push(raw);
            ui.text(format!("item-{raw}"));
        };
        if variable {
            ui.virtual_list_variable(state, 4, render);
        } else {
            ui.virtual_list(state, 4, render);
        }
    });
    rows.into_inner()
}

#[test]
fn filtered_virtual_lists_use_raw_callback_indices_and_view_selection() {
    for variable in [false, true] {
        let mut state = ListState::new(vec!["apple", "banana"]).with_item_heights(vec![20, 2]);
        state.set_filter("banana");
        assert_eq!(virtual_rows(&mut state, variable, Some(KeyCode::Down)), [1]);
        assert_eq!(state.selected_item(), Some("banana"));
        state.set_filter("no match");
        assert!(virtual_rows(&mut state, variable, Some(KeyCode::End)).is_empty());
        assert_eq!(state.selected_item(), None);
        state.set_filter("");
        assert!(!virtual_rows(&mut state, variable, Some(KeyCode::End)).is_empty());
        assert_eq!(state.selected_item(), Some("banana"));
    }
}

#[test]
fn variable_filter_end_home_and_partial_heights_keep_mapping() {
    let mut state = ListState::new(vec!["hide", "show a", "show b", "show c"])
        .with_item_heights(vec![100, 2, 2, 3]);
    state.set_filter("show");
    assert_eq!(virtual_rows(&mut state, true, Some(KeyCode::End)), [3]);
    assert_eq!(state.selected_item(), Some("show c"));
    assert_eq!(virtual_rows(&mut state, true, Some(KeyCode::Home)), [1, 2]);
    assert_eq!(state.selected_item(), Some("show a"));
    assert_eq!(virtual_rows(&mut state, true, Some(KeyCode::PageDown)), [3]);
}

#[test]
fn variable_tall_item_is_clipped_to_declared_viewport() {
    let mut state = ListState::new(vec!["tall"]).with_item_heights(vec![10]);
    let mut backend = TestBackend::new(20, 12);
    backend.render(|ui| {
        ui.virtual_list_variable(&mut state, 3, |ui, _| {
            for i in 0..10 {
                ui.text(format!("row-{i}"));
            }
        });
        ui.text("after");
    });
    backend.assert_contains("row-2");
    backend.assert_not_contains("row-3");
    assert!(backend.line(3).contains("after"));
}

#[test]
fn list_append_matches_batch_with_filter_and_selection() {
    let data: Vec<String> = (0..1000).map(|i| format!("row-{i}")).collect();
    let mut incremental = ListState::default();
    incremental.set_filter("row 9");
    for item in &data {
        incremental.push_item(item);
    }
    let mut batch = ListState::new(data);
    batch.set_filter("row 9");
    assert_eq!(incremental.items(), batch.items());
    assert_eq!(incremental.visible_indices(), batch.visible_indices());
    assert_eq!(incremental.selected_item(), batch.selected_item());
}

#[test]
fn table_append_matches_batch_and_updates_auto_widths() {
    let mut state = TableState::new(vec!["H"], Vec::<Vec<String>>::new());
    state.set_filter("match");
    state.push_row(vec!["not included"]);
    state.push_row(vec!["match long content"]);
    assert_eq!(state.visible_indices(), [1]);
    let mut backend = TestBackend::new(40, 6);
    backend.render(|ui| {
        ui.table(&mut state);
    });
    backend.assert_contains("match long content");
}

#[test]
fn manual_list_and_table_clicks_are_blocked_by_modal() {
    for table_mode in [false, true] {
        let mut list = ListState::new(vec!["first", "second", "third"]);
        let mut table = TableState::new(vec!["H"], vec![vec!["first"], vec!["second"]]);
        let mut backend = TestBackend::new(40, 12);
        let mut scene = |ui: &mut Context| {
            let response = if table_mode {
                ui.table(&mut table)
            } else {
                ui.list(&mut list)
            };
            assert!(!response.changed);
            ui.modal(|ui| {
                ui.button("dialog");
            });
        };
        for _ in 0..3 {
            backend.render(&mut scene);
        }
        backend.run_with_events(
            EventBuilder::new()
                .click(0, if table_mode { 3 } else { 1 })
                .build(),
            &mut scene,
        );
        assert_eq!(list.selected, 0);
        assert_eq!(table.selected, 0);
    }
}

#[test]
fn table_percent_columns_fit_fixed_padded_parent_and_header_hits() {
    for screen_width in [40, 100] {
        let mut state = TableState::new(vec!["LEFT", "RIGHT"], vec![vec!["A", "B"]]);
        state.column_widths_spec(&[TableColumn::Percent(50), TableColumn::Percent(50)]);
        let mut backend = TestBackend::new(screen_width, 8);
        let mut table_rect = Rect::default();
        for _ in 0..3 {
            backend.render(|ui| {
                ui.container().w(24).border(Border::Single).px(1).col(|ui| {
                    table_rect = ui.table(&mut state).rect;
                });
            });
        }
        backend.assert_contains("RIGHT");
        assert!(table_rect.width <= 20, "{table_rect:?}");
        backend.run_with_events(
            EventBuilder::new()
                .click(table_rect.x + 12, table_rect.y)
                .build(),
            |ui| {
                ui.container().w(24).border(Border::Single).px(1).col(|ui| {
                    ui.table(&mut state);
                });
            },
        );
        assert_eq!(state.sort_column, Some(1));
    }
}

#[test]
fn line_wrap_preserves_gradient_plain_and_nested_text() {
    let mut backend = TestBackend::new(4, 8);
    backend.render(|ui| {
        ui.line_wrap(|ui| {
            ui.text("AB");
            ui.line_wrap(|ui| {
                ui.text("CDEF").gradient(Color::Red, Color::Blue);
            });
            ui.text("GH");
        });
    });
    backend.assert_contains("ABCD");
    backend.assert_contains("EFGH");
}

#[test]
fn line_wrap_with_link_keeps_gradient_and_url() {
    let mut backend = TestBackend::new(4, 8);
    backend.render(|ui| {
        ui.line_wrap(|ui| {
            ui.link("AB", "https://example.com");
            ui.text("CDEFGH").gradient(Color::Red, Color::Blue);
        });
    });
    let text = backend.to_string_trimmed();
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert_eq!(compact, "ABCDEFGH", "{text}");
}

fn nested_scroll(
    ui: &mut Context,
    outer: &mut ScrollState,
    inner: &mut ScrollState,
    horizontal: bool,
    overflow: bool,
) {
    ui.scrollable(outer).w(20).h(6).col(|ui| {
        if horizontal {
            ui.scrollable(inner).w(20).h(2).row(|ui| {
                ui.text("x".repeat(if overflow { 100 } else { 2 }));
            });
        } else {
            ui.scrollable(inner).w(20).h(2).col(|ui| {
                for _ in 0..if overflow { 20 } else { 1 } {
                    ui.text("inner");
                }
            });
        }
        for n in 0..20 {
            ui.text(format!("row-{n}"));
        }
    });
}

#[test]
fn nested_wheel_routes_by_axis_and_overflow_once() {
    for horizontal in [false, true] {
        for overflow in [false, true] {
            for shifted in [false, true] {
                let mut outer = ScrollState::new();
                let mut inner = ScrollState::new();
                let mut backend = TestBackend::new(30, 12);
                for _ in 0..3 {
                    backend.render(|ui| {
                        nested_scroll(ui, &mut outer, &mut inner, horizontal, overflow)
                    });
                }
                let mut events = EventBuilder::new().scroll_down(1, 0).build();
                if let Event::Mouse(mouse) = &mut events[0] {
                    if shifted {
                        mouse.modifiers = KeyModifiers::SHIFT;
                    }
                }
                backend.run_with_events(events, |ui| {
                    nested_scroll(ui, &mut outer, &mut inner, horizontal, overflow)
                });
                assert_eq!(outer.offset > 0, !shifted && (horizontal || !overflow));
                assert_eq!(inner.offset > 0, !shifted && !horizontal && overflow);
                assert_eq!(inner.offset_x > 0, shifted && horizontal && overflow);
            }
        }
    }
}

#[test]
fn nested_wheel_at_child_limit_does_not_bubble() {
    let mut outer = ScrollState::new();
    let mut inner = ScrollState::new();
    let mut backend = TestBackend::new(30, 12);
    for _ in 0..3 {
        backend.render(|ui| nested_scroll(ui, &mut outer, &mut inner, false, true));
    }
    inner.set_offset(usize::MAX);
    backend.run_with_events(EventBuilder::new().scroll_down(1, 0).build(), |ui| {
        nested_scroll(ui, &mut outer, &mut inner, false, true)
    });
    assert_eq!(outer.offset, 0);
}

#[test]
fn overlapping_scrollables_target_last_painted_and_consume_once() {
    let mut first = ScrollState::new();
    let mut last = ScrollState::new();
    let mut backend = TestBackend::new(30, 12);
    let render = |ui: &mut Context, first: &mut ScrollState, last: &mut ScrollState| {
        ui.container().gap_overlap(2).col(|ui| {
            ui.scrollable(first).w(20).h(4).col(|ui| {
                for _ in 0..20 {
                    ui.text("first");
                }
            });
            ui.scrollable(last).w(20).h(4).col(|ui| {
                for _ in 0..20 {
                    ui.text("last");
                }
            });
        });
    };
    for _ in 0..3 {
        backend.render(|ui| render(ui, &mut first, &mut last));
    }
    backend.run_with_events(EventBuilder::new().scroll_down(1, 2).build(), |ui| {
        render(ui, &mut first, &mut last)
    });
    assert_eq!(first.offset, 0);
    assert!(last.offset > 0);
}

#[test]
fn native_horizontal_wheel_and_modal_suppression() {
    for modal in [false, true] {
        let mut outer = ScrollState::new();
        let mut inner = ScrollState::new();
        let mut backend = TestBackend::new(30, 12);
        let render = |ui: &mut Context, outer: &mut ScrollState, inner: &mut ScrollState| {
            nested_scroll(ui, outer, inner, true, true);
            if modal {
                ui.modal(|ui| {
                    ui.button("modal");
                });
            }
        };
        for _ in 0..3 {
            backend.render(|ui| render(ui, &mut outer, &mut inner));
        }
        let mut events = EventBuilder::new().scroll_down(1, 0).build();
        if let Event::Mouse(mouse) = &mut events[0] {
            mouse.kind = slt::MouseKind::ScrollRight;
        }
        backend.run_with_events(events, |ui| render(ui, &mut outer, &mut inner));
        assert_eq!(outer.offset, 0);
        assert_eq!(inner.offset_x > 0, !modal);
    }
}

#[test]
fn table_in_growing_row_pane_tracks_parent_resize() {
    let mut state = TableState::new(vec!["LEFT", "RIGHT"], vec![vec!["A", "B"]]);
    state.column_widths_spec(&[TableColumn::Percent(50), TableColumn::Percent(50)]);
    let mut backend = TestBackend::new(100, 10);
    for width in [60, 40, 70] {
        for _ in 0..3 {
            backend.render(|ui| {
                ui.container().w(width).h(6).row(|ui| {
                    ui.container().w(10).col(|ui| {
                        ui.text("sidebar");
                    });
                    ui.container()
                        .grow(1)
                        .border(Border::Single)
                        .p(1)
                        .col(|ui| {
                            ui.table(&mut state);
                        });
                });
            });
        }
        backend.assert_contains("RIGHT");
        let line = backend.line(2);
        assert!(line.chars().skip(width as usize).all(|c| c == ' '));
    }
}

#[test]
fn log_in_scrolled_grow_pane_does_not_use_clipped_height() {
    let mut state = RichLogState::new();
    for i in 0..20 {
        state.push_plain(format!("entry-{i}"));
    }
    let mut scroll = ScrollState::new();
    let mut backend = TestBackend::new(40, 20);
    let render = |ui: &mut Context, scroll: &mut ScrollState, state: &mut RichLogState| {
        ui.scrollable(scroll).w(30).h(8).col(|ui| {
            ui.container().h(16).col(|ui| {
                ui.container().grow(1).col(|ui| {
                    ui.rich_log(state);
                });
            });
        });
    };
    for _ in 0..3 {
        backend.render(|ui| render(ui, &mut scroll, &mut state));
    }
    backend.assert_contains("entry-7");
    let snapshot = backend.to_string_trimmed();
    scroll.set_offset(4);
    for _ in 0..3 {
        backend.render(|ui| render(ui, &mut scroll, &mut state));
    }
    scroll.set_offset(0);
    for _ in 0..3 {
        backend.render(|ui| render(ui, &mut scroll, &mut state));
    }
    assert_eq!(backend.to_string_trimmed(), snapshot);
}

fn split_scene(
    ui: &mut Context,
    state: &mut SplitPaneState,
    list: &mut ListState,
    prefix: bool,
    vertical: bool,
) -> Rect {
    if prefix {
        ui.container().w(10).h(2).col(|ui| {
            ui.text("prefix");
        });
    }
    let mut rect = Rect::default();
    ui.container()
        .w(40)
        .h(12)
        .border(Border::Single)
        .p(1)
        .col(|ui| {
            rect = if vertical {
                ui.vsplit_pane(
                    state,
                    |ui| {
                        ui.list(list);
                    },
                    |ui| {
                        ui.text("bottom");
                    },
                )
                .rect
            } else {
                ui.split_pane(
                    state,
                    |ui| {
                        ui.list(list);
                    },
                    |ui| {
                        ui.text("right");
                    },
                )
                .rect
            };
        });
    rect
}

#[test]
fn splitter_only_grabs_divider_and_uses_own_geometry() {
    for vertical in [false, true] {
        let mut ratios = Vec::new();
        for prefix in [false, true] {
            let mut state = SplitPaneState::new(0.5);
            let mut list = ListState::new(vec!["first", "second", "third"]);
            let mut backend = TestBackend::new(60, 20);
            let mut rect = Rect::default();
            for _ in 0..3 {
                backend.render(|ui| {
                    rect = split_scene(ui, &mut state, &mut list, prefix, vertical);
                });
            }
            backend.run_with_events(
                EventBuilder::new().click(rect.x, rect.y + 1).build(),
                |ui| {
                    split_scene(ui, &mut state, &mut list, prefix, vertical);
                },
            );
            assert!(!state.dragging);
            assert_eq!(list.selected, 1);
            let (x, y) = if vertical {
                (rect.x, rect.y + (rect.height - 1) / 2)
            } else {
                (rect.x + (rect.width - 1) / 2, rect.y)
            };
            let (dx, dy) = if vertical {
                (x, rect.y + (rect.height - 1) * 3 / 4)
            } else {
                (rect.x + (rect.width - 1) * 3 / 4, y)
            };
            backend.run_with_events(
                EventBuilder::new()
                    .click(x, y)
                    .drag(dx, dy)
                    .mouse_up(dx, dy)
                    .build(),
                |ui| {
                    split_scene(ui, &mut state, &mut list, prefix, vertical);
                },
            );
            assert!(!state.dragging);
            assert!(
                state.ratio > 0.6,
                "vertical={vertical} rect={rect:?} ratio={}",
                state.ratio
            );
            ratios.push(state.ratio);
        }
        assert_eq!(ratios[0], ratios[1]);
    }
}

#[test]
fn nested_splitters_keep_drag_ownership_and_modal_cancels_capture() {
    let mut outer = SplitPaneState::new(0.5);
    let mut inner = SplitPaneState::new(0.5);
    let mut backend = TestBackend::new(60, 12);
    let mut inner_rect = Rect::default();
    let render = |ui: &mut Context,
                  outer: &mut SplitPaneState,
                  inner: &mut SplitPaneState,
                  inner_rect: &mut Rect,
                  modal| {
        ui.split_pane(
            outer,
            |ui| {
                *inner_rect = ui
                    .split_pane(
                        inner,
                        |ui| {
                            ui.button("child");
                        },
                        |ui| {
                            ui.text("inner");
                        },
                    )
                    .rect;
            },
            |ui| {
                ui.text("outer");
            },
        );
        if modal {
            ui.modal(|ui| {
                ui.button("modal");
            });
        }
    };
    for _ in 0..3 {
        backend.render(|ui| render(ui, &mut outer, &mut inner, &mut inner_rect, false));
    }
    let x = inner_rect.x + (inner_rect.width - 1) / 2;
    let dx = inner_rect.x + (inner_rect.width - 1) * 3 / 4;
    backend.run_with_events(
        EventBuilder::new()
            .click(x, inner_rect.y)
            .drag(dx, inner_rect.y)
            .build(),
        |ui| {
            render(ui, &mut outer, &mut inner, &mut inner_rect, false);
        },
    );
    assert_eq!(outer.ratio, 0.5);
    assert!(!outer.dragging);
    assert!(inner.dragging);
    assert!(inner.ratio > 0.6);
    for _ in 0..2 {
        backend.render(|ui| render(ui, &mut outer, &mut inner, &mut inner_rect, true));
    }
    assert!(!inner.dragging);
}

#[test]
fn clipped_scrolled_splitter_uses_full_logical_extent() {
    for vertical in [false, true] {
        let mut split = SplitPaneState::new(0.5);
        let mut scroll = ScrollState::new();
        let mut backend = TestBackend::new(40, 12);
        let render = |ui: &mut Context, split: &mut SplitPaneState, scroll: &mut ScrollState| {
            if vertical {
                ui.scrollable(scroll).w(30).h(8).col(|ui| {
                    ui.container().h(16).col(|ui| {
                        ui.vsplit_pane(
                            split,
                            |ui| {
                                ui.text("top");
                            },
                            |ui| {
                                ui.text("bottom");
                            },
                        );
                    });
                });
            } else {
                ui.scrollable(scroll).w(30).h(8).row(|ui| {
                    ui.container().w(60).h(8).col(|ui| {
                        ui.split_pane(
                            split,
                            |ui| {
                                ui.text("left");
                            },
                            |ui| {
                                ui.text("right");
                            },
                        );
                    });
                });
            }
        };
        for _ in 0..3 {
            backend.render(|ui| render(ui, &mut split, &mut scroll));
        }
        if vertical {
            scroll.set_offset(4);
        } else {
            scroll.scroll_right(10);
        }
        for _ in 0..3 {
            backend.render(|ui| render(ui, &mut split, &mut scroll));
        }
        let (x, y, dx, dy, expected) = if vertical {
            (0, 3, 0, 5, 9.0 / 15.0)
        } else {
            (19, 0, 24, 0, 34.0 / 59.0)
        };
        backend.run_with_events(
            EventBuilder::new()
                .click(x, y)
                .drag(dx, dy)
                .mouse_up(dx, dy)
                .build(),
            |ui| {
                render(ui, &mut split, &mut scroll);
            },
        );
        assert!(
            (split.ratio - expected).abs() < 1e-9,
            "vertical={vertical} ratio={}",
            split.ratio
        );
    }
}

#[test]
fn growing_log_matches_fresh_large_and_shrink_regrow() {
    let render = |backend: &mut TestBackend, state: &mut RichLogState, height| {
        for _ in 0..3 {
            backend.render(|ui| {
                ui.container().h(height).w(30).col(|ui| {
                    ui.rich_log(state);
                });
            });
        }
    };
    let mut grown = RichLogState::new();
    let mut backend = TestBackend::new(40, 16);
    render(&mut backend, &mut grown, 12);
    grown.push_plain("entry-0");
    render(&mut backend, &mut grown, 12);
    for i in 1..30 {
        grown.push_plain(format!("entry-{i}"));
    }
    render(&mut backend, &mut grown, 12);
    let snapshot = backend.to_string_trimmed();
    let mut fresh = RichLogState::new();
    for i in 0..30 {
        fresh.push_plain(format!("entry-{i}"));
    }
    render(&mut backend, &mut fresh, 12);
    assert_eq!(snapshot, backend.to_string_trimmed());
    render(&mut backend, &mut grown, 5);
    render(&mut backend, &mut grown, 12);
    assert!(backend.to_string_trimmed().matches("entry-").count() >= 8);
}

#[test]
fn growing_file_picker_matches_fresh_directory() {
    let root = std::env::temp_dir().join(format!(
        "slt-v024-picker-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let small = root.join("small");
    let large = root.join("large");
    std::fs::create_dir(&small).unwrap();
    std::fs::create_dir(&large).unwrap();
    for i in 0..20 {
        std::fs::write(large.join(format!("file-{i:02}")), "").unwrap();
    }
    let mut state = FilePickerState::new(&small);
    let mut backend = TestBackend::new(100, 12);
    for _ in 0..3 {
        backend.render(|ui| {
            ui.file_picker(&mut state);
        });
    }
    state.current_dir = large.clone();
    state.retry();
    for _ in 0..3 {
        backend.render(|ui| {
            ui.file_picker(&mut state);
        });
    }
    let grown = backend.to_string_trimmed();
    let mut fresh = FilePickerState::new(&large);
    for _ in 0..3 {
        backend.render(|ui| {
            ui.file_picker(&mut fresh);
        });
    }
    assert_eq!(grown, backend.to_string_trimmed());
    assert!(!grown.contains("1-0 /"));
    assert!(grown.matches("file-").count() >= 8);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "release-mode collection and idle-scroll workload measurements"]
fn collection_workloads() {
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    fn median(mut run: impl FnMut()) -> f64 {
        let mut times = Vec::new();
        for _ in 0..7 {
            let start = Instant::now();
            run();
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        times[3]
    }

    for count in [1_000, 10_000, 100_000] {
        let data: Vec<String> = (0..count).map(|i| format!("item-{i:06}")).collect();
        let mut state = ListState::default();
        let batch_ms = median(|| state.set_items(black_box(data.clone())));
        let mut appended = ListState::default();
        let start = Instant::now();
        let mut completed = 0;
        for item in &data {
            appended.push_item(black_box(item));
            completed += 1;
            if completed % 128 == 0 && start.elapsed() > Duration::from_secs(3) {
                break;
            }
        }
        let append_ms = start.elapsed().as_secs_f64() * 1000.0;
        let filter_ms = median(|| {
            state.set_filter("99");
            black_box(state.visible_indices());
            state.set_filter("");
        });
        state.set_item_heights((0..count).map(|i| (i % 4 + 1) as u32).collect());
        let mut backend = TestBackend::new(80, 24);
        backend.render(|ui| {
            ui.virtual_list_variable(&mut state, 20, |ui, raw| {
                ui.text(format!("row {raw}"));
            });
        });
        let navigation_ms = median(|| {
            backend.run_with_events(EventBuilder::new().key_code(KeyCode::Home).build(), |ui| {
                ui.virtual_list_variable(&mut state, 20, |ui, raw| {
                    ui.text(format!("row {raw}"));
                });
            });
            backend.run_with_events(EventBuilder::new().key_code(KeyCode::End).build(), |ui| {
                ui.virtual_list_variable(&mut state, 20, |ui, raw| {
                    ui.text(format!("row {raw}"));
                });
            });
        });
        let mut callback_count = 0;
        let callbacks = std::cell::Cell::new(0);
        backend.render(|ui| {
            ui.virtual_list_variable(&mut state, 20, |ui, raw| {
                callbacks.set(callbacks.get() + 1);
                ui.text(format!("row {raw}"));
            });
        });
        callback_count += callbacks.get();
        let full_list_ms = median(|| {
            backend.render(|ui| {
                ui.list(&mut state);
            })
        });
        let rows: Vec<Vec<String>> = data.iter().map(|s| vec![s.clone()]).collect();
        let mut table = TableState::new(vec!["Header"], rows);
        let table_full_ms = median(|| {
            backend.render(|ui| {
                ui.table(&mut table);
            })
        });
        table.page_size = 20;
        let table_page_ms = median(|| {
            backend.render(|ui| {
                ui.table(&mut table);
            })
        });
        eprintln!(
            "COLLECTION n={count} set_items_ms={batch_ms:.4} push_ms={append_ms:.4} push_completed={completed} filter_pair_ms={filter_ms:.4} variable_home_end_pair_ms={navigation_ms:.4} virtual_callbacks={callback_count} list_frame_ms={full_list_ms:.4} table_full_frame_ms={table_full_ms:.4} table_page_frame_ms={table_page_ms:.4}"
        );
    }

    for count in [128, 512, 2048] {
        let mut states = vec![ScrollState::new(); count];
        let mut backend = TestBackend::new(1, 4096);
        for _ in 0..3 {
            backend.render(|ui| {
                for state in &mut states {
                    ui.scrollable(state).h(1).col(|ui| {
                        ui.text("x");
                    });
                }
            });
        }
        let mut closure_times = Vec::new();
        let frame_ms = median(|| {
            backend.render(|ui| {
                let start = Instant::now();
                for state in &mut states {
                    ui.scrollable(state).h(1).col(|ui| {
                        ui.text("x");
                    });
                }
                closure_times.push(start.elapsed().as_secs_f64() * 1000.0);
            })
        });
        closure_times.sort_by(f64::total_cmp);
        let closure_ms = closure_times[3];
        eprintln!("IDLE_SCROLL regions={count} frame_ms={frame_ms:.4} closure_ms={closure_ms:.4}");
    }
}

#[test]
#[ignore = "release-mode repeated-table geometry scan regression measurement"]
fn repeated_table_geometry_workload() {
    use std::time::Instant;
    for count in [1_000, 10_000] {
        let mut tables = vec![TableState::new(vec!["H"], vec![vec!["cell"]]); count];
        let mut backend = TestBackend::new(80, 24);
        let mut closure_times = Vec::new();
        let mut frame_times = Vec::new();
        for sample in 0..9 {
            let frame = Instant::now();
            backend.render(|ui| {
                let start = Instant::now();
                for state in &mut tables {
                    ui.table(state);
                }
                if sample >= 2 {
                    closure_times.push(start.elapsed().as_secs_f64() * 1000.0);
                }
            });
            if sample >= 2 {
                frame_times.push(frame.elapsed().as_secs_f64() * 1000.0);
            }
        }
        closure_times.sort_by(f64::total_cmp);
        frame_times.sort_by(f64::total_cmp);
        eprintln!(
            "REPEATED_TABLES n={count} closure_median_ms={:.4} frame_median_ms={:.4}",
            closure_times[3], frame_times[3]
        );
    }
}
