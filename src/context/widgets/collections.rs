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

#[cfg(test)]
mod v024_collection_command_tests {
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
