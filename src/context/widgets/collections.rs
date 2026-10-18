//! Collections family — lists (plain, reorderable, virtual), tables,
//! paginators, trees, directory trees, calendars and file pickers.
//!
//! State types ([`crate::widgets::ListState`],
//! [`crate::widgets::TableState`], [`crate::widgets::TreeState`],
//! [`crate::widgets::CalendarState`], [`crate::widgets::FilePickerState`],
//! ...) live in `src/widgets/collections.rs`.

use super::*;

mod calendar;
mod file_picker;
mod list;
mod table;
mod tree;

impl Context {
    /// Rows a self-scrolling `list` / unpaged `table` body can draw this frame
    /// (#487).
    ///
    /// Uses the parent's available content height below this widget: the
    /// parent size is resolved for the current frame when it is known (root
    /// area, explicit heights), otherwise from the previous frame's layout, and
    /// this widget's previous-frame offset inside the parent is subtracted.
    /// The widget also never extends below the current terminal bottom from
    /// where it was last drawn, which covers offsets of outer ancestors and
    /// makes a shrinking resize take effect in the same frame. `reserved` rows
    /// (table header + rule) are removed, the result is capped by `total` and
    /// never drops below one row, so the selected row is always emitted. Must
    /// run after `begin_widget_interaction` and before the widget pushes any
    /// command, so the geometry cache attributes the offset to this widget's
    /// interaction marker.
    fn collection_viewport_rows(
        &mut self,
        interaction_id: usize,
        total: usize,
        reserved: u32,
    ) -> usize {
        if total == 0 {
            return 0;
        }
        let (_, mut height) = self.available_content_size();
        if let Some(prev) = self.prev_hit_map.get(interaction_id)
            && prev.height > 0
        {
            height = height.min(self.area_height.saturating_sub(prev.y));
        }
        (height.saturating_sub(reserved) as usize).clamp(1, total)
    }
}

/// Clamp a viewport `offset` so `selected` lies in `[offset, offset + rows)`
/// and the window never runs past the end of `total` items. Scrolls only when
/// the selection leaves the window (the same rule `virtual_list` uses).
fn keep_selection_in_window(offset: usize, selected: usize, rows: usize, total: usize) -> usize {
    if rows == 0 || total == 0 {
        return 0;
    }
    let mut offset = offset.min(total.saturating_sub(rows));
    if selected < offset {
        offset = selected;
    } else if selected >= offset + rows {
        offset = selected + 1 - rows;
    }
    offset
}

#[cfg(test)]
mod collection_command_count_tests {
    use super::*;

    #[test]
    #[ignore = "collection command-preparation workload measurements"]
    fn collection_command_workload() {
        for count in [1_000, 10_000, 100_000] {
            let mut list = ListState::new((0..count).map(|i| format!("item-{i:06}")).collect());
            list.set_item_heights(vec![2; count]);
            let mut table = TableState::new(
                vec!["Header"],
                list.items().iter().map(|s| vec![s.clone()]).collect(),
            );
            let mut backend = crate::TestBackend::new(80, 24);
            let mut counts = [0; 4];
            backend.render(|ui| {
                let _ = ui.list(&mut list);
                counts[0] = ui.commands.len();
            });
            backend.render(|ui| {
                let _ = ui.virtual_list_variable(&mut list, 20, |ui, raw| {
                    ui.text(format!("row-{raw}"));
                });
                counts[1] = ui.commands.len();
            });
            backend.render(|ui| {
                let _ = ui.table(&mut table);
                counts[2] = ui.commands.len();
            });
            table.page_size = 20;
            backend.render(|ui| {
                let _ = ui.table(&mut table);
                counts[3] = ui.commands.len();
            });
            eprintln!(
                "COLLECTION_COMMANDS n={count} list={} virtual={} table={} table_page={}",
                counts[0], counts[1], counts[2], counts[3]
            );
            assert!(counts[1] <= 60);
            assert!(counts[3] <= 35);
        }
    }
}

#[cfg(test)]
mod list_table_viewport_tests {
    //! Issue #487: `list` and unpaged `table` render only the rows that fit
    //! and keep the selected row inside the drawn viewport.
    use super::*;
    use crate::{EventBuilder, TestBackend};

    fn items(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("Item {i:05}")).collect()
    }

    fn users(n: usize) -> TableState {
        TableState::new(
            vec!["ID", "Name"],
            (0..n)
                .map(|i| vec![i.to_string(), format!("User {i}")])
                .collect::<Vec<_>>(),
        )
    }

    /// Exact cell text of a table row (`"25 │ User 25"`) for tables of at
    /// most 100 rows, whose ID column is two cells wide.
    fn user_row(i: usize) -> String {
        format!("{i:<2} │ User {i}")
    }

    fn screen(tb: &TestBackend) -> String {
        tb.to_string_trimmed()
    }

    #[test]
    fn programmatic_selection_is_rendered_for_list_and_table() {
        let mut list = ListState::new(items(100));
        assert!(list.set_selected_index(25));
        let mut tb = TestBackend::new(120, 20);
        tb.render(|ui| {
            let _ = ui.list(&mut list);
        });
        assert!(screen(&tb).contains("▸ Item 00025"), "{}", screen(&tb));
        assert_eq!(tb.to_string_trimmed().lines().count(), 20);

        let mut table = users(100);
        assert!(table.set_selected_index(25));
        let mut tb = TestBackend::new(120, 20);
        for _ in 0..3 {
            tb.render(|ui| {
                let _ = ui.table(&mut table);
            });
            tb.assert_contains(&user_row(25));
            // Header and rule stay pinned while the body scrolls.
            assert!(tb.line(0).starts_with("ID"), "{:?}", tb.line(0));
            assert!(tb.line(1).starts_with("──"), "{:?}", tb.line(1));
        }
    }

    #[test]
    fn keyboard_navigation_keeps_the_selection_visible_every_frame() {
        let mut list = ListState::new(items(100));
        let mut table = users(100);
        let mut list_tb = TestBackend::new(120, 20);
        let mut table_tb = TestBackend::new(120, 20);
        for (key, steps) in [(KeyCode::Down, 25), (KeyCode::Up, 25)] {
            for _ in 0..steps {
                let events = EventBuilder::new().key_code(key.clone()).build();
                list_tb.run_with_events(events.clone(), |ui| {
                    let _ = ui.list(&mut list);
                });
                let sel = list.selected_view_index();
                list_tb.assert_contains(&format!("▸ Item {sel:05}"));

                table_tb.run_with_events(events, |ui| {
                    let _ = ui.table(&mut table);
                });
                table_tb.assert_contains(&user_row(table.selected_view_index()));
            }
        }
        assert_eq!(list.selected_view_index(), 0);
        assert_eq!(table.selected_view_index(), 0);
        list_tb.assert_contains("▸ Item 00000");
    }

    #[test]
    fn clicks_after_scrolling_select_offset_plus_row() {
        let mut list = ListState::new(items(100));
        assert!(list.set_selected_index(50));
        let mut tb = TestBackend::new(120, 20);
        tb.render(|ui| {
            let _ = ui.list(&mut list);
        });
        // 20 rows with item 50 at the bottom: rows 31..=50 are drawn.
        assert!(tb.line(0).contains("Item 00031"), "{:?}", tb.line(0));
        tb.run_with_events(EventBuilder::new().click(4, 3).build(), |ui| {
            let _ = ui.list(&mut list);
        });
        assert_eq!(list.selected_index(), Some(34));

        let mut table = users(100);
        assert!(table.set_selected_index(50));
        let mut tb = TestBackend::new(120, 20);
        tb.render(|ui| {
            let _ = ui.table(&mut table);
        });
        // Header + rule leave 18 body rows: 33..=50.
        assert!(tb.line(2).starts_with("33 "), "{:?}", tb.line(2));
        tb.run_with_events(EventBuilder::new().click(1, 5).build(), |ui| {
            let _ = ui.table(&mut table);
        });
        assert_eq!(table.selected_index(), Some(36));
    }

    /// Persistent-frame-state harness so a terminal resize keeps the previous
    /// frame's geometry, like the real run loop.
    struct Resizable {
        state: crate::FrameState,
    }

    impl Resizable {
        fn frame(&mut self, w: u32, h: u32, f: &mut impl FnMut(&mut Context)) -> Vec<String> {
            let mut buffer = crate::buffer::Buffer::empty(Rect::new(0, 0, w, h));
            let _ = crate::run_frame_kernel(
                &mut buffer,
                &mut self.state,
                &crate::RunConfig::default(),
                (w, h),
                Vec::new(),
                false,
                f,
            );
            (0..h)
                .map(|y| {
                    (0..w)
                        .map(|x| buffer.get(x, y).symbol.as_str())
                        .collect::<String>()
                        .trim_end()
                        .to_string()
                })
                .collect()
        }
    }

    #[test]
    fn resize_shows_the_bottom_selection_within_one_frame() {
        let mut list = ListState::new(items(100));
        let mut table = users(100);
        let mut harness = Resizable {
            state: crate::FrameState::default(),
        };
        let render = |ui: &mut Context, list: &mut ListState, table: &mut TableState| {
            ui.text("header");
            let _ = ui.row(|ui| {
                let _ = ui.container().grow(1).col(|ui| {
                    let _ = ui.list(list);
                });
                let _ = ui.container().grow(1).col(|ui| {
                    let _ = ui.table(table);
                });
            });
        };
        // 40 rows: header + 39 list rows (0..=38); the table shows 37 rows.
        assert!(list.set_selected_index(38));
        assert!(table.set_selected_index(36));
        for _ in 0..2 {
            let lines = harness.frame(120, 40, &mut |ui| render(ui, &mut list, &mut table));
            assert!(lines[39].contains("Item 00038"), "{:?}", lines[39]);
            assert!(lines[39].contains(&user_row(36)), "{:?}", lines[39]);
        }

        // A resize clears all previous-frame geometry, so the resize frame
        // cannot know the header above the widgets: it may hide the bottom
        // row. The next frame must show the selection (one-frame bound).
        let _ = harness.frame(120, 10, &mut |ui| render(ui, &mut list, &mut table));
        let lines = harness.frame(120, 10, &mut |ui| render(ui, &mut list, &mut table));
        let text = lines.join("\n");
        assert!(lines[9].contains("▸ Item 00038"), "{text}");
        assert!(lines[9].contains(&user_row(36)), "{text}");
        assert!(lines[1].contains("ID │ Name"), "{text}");
    }

    #[test]
    fn resize_of_a_top_level_list_shows_the_selection_in_the_same_frame() {
        let mut list = ListState::new(items(100));
        let mut harness = Resizable {
            state: crate::FrameState::default(),
        };
        assert!(list.set_selected_index(39));
        let lines = harness.frame(120, 40, &mut |ui| {
            let _ = ui.list(&mut list);
        });
        assert!(lines[39].contains("▸ Item 00039"), "{:?}", lines[39]);
        let lines = harness.frame(120, 10, &mut |ui| {
            let _ = ui.list(&mut list);
        });
        assert!(lines[9].contains("▸ Item 00039"), "{}", lines.join("\n"));
    }

    #[test]
    fn short_lists_and_tables_render_every_row() {
        let mut list = ListState::new(items(5));
        let mut table = users(5);
        let mut tb = TestBackend::new(40, 20);
        tb.render(|ui| {
            let _ = ui.list(&mut list);
            let _ = ui.table(&mut table);
            ui.text("after");
        });
        for i in 0..5 {
            tb.assert_contains(&format!("Item {i:05}"));
            tb.assert_contains(&user_row(i));
        }
        assert_eq!(tb.line(12), "after");
    }

    #[test]
    fn list_command_count_tracks_viewport_not_items() {
        for n in [1_000usize, 10_000] {
            let mut list = ListState::new(items(n));
            assert!(list.set_selected_index(25));
            let mut table = users(n);
            assert!(table.set_selected_index(25));
            let mut tb = TestBackend::new(100, 50);
            let mut counts = (0, 0);
            tb.render(|ui| {
                let _ = ui.list(&mut list);
                counts.0 = ui.commands.len();
            });
            tb.render(|ui| {
                let _ = ui.table(&mut table);
                counts.1 = ui.commands.len();
            });
            // Begin/End + interaction marker + at most 50 rows.
            assert!(counts.0 <= 55, "list n={n}: {} commands", counts.0);
            assert!(counts.1 <= 55, "table n={n}: {} commands", counts.1);
        }
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(48))]

        #[test]
        fn viewport_always_contains_the_selection(
            n in 0usize..120,
            height in 1u32..30,
            keys in proptest::collection::vec(proptest::bool::ANY, 0..30),
            start in 0usize..120,
        ) {
            let mut list = ListState::new(items(n));
            list.set_selected_view_index(start.min(n.saturating_sub(1)));
            let mut table = users(n);
            table.set_selected_view_index(start.min(n.saturating_sub(1)));
            let mut list_tb = TestBackend::new(30, height);
            let mut table_tb = TestBackend::new(30, height + 2);
            let rows = n.min(height as usize);
            for down in keys {
                let key = if down { KeyCode::Down } else { KeyCode::Up };
                let events = EventBuilder::new().key_code(key).build();
                list_tb.run_with_events(events.clone(), |ui| {
                    let _ = ui.list(&mut list);
                });
                table_tb.run_with_events(events, |ui| {
                    let _ = ui.table(&mut table);
                });
                if n > 0 {
                    for (offset, selected) in [
                        (list.viewport_offset, list.selected_view_index()),
                        (table.viewport_offset, table.selected_view_index()),
                    ] {
                        proptest::prop_assert!(offset <= selected);
                        proptest::prop_assert!(selected < offset + rows);
                        proptest::prop_assert!(offset + rows <= n);
                    }
                }
            }
        }
    }
}
