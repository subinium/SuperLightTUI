use super::*;

#[test]
fn static_output_accumulates_and_drains_new_lines() {
    let mut output = StaticOutput::new();
    output.println("Building crate...");
    output.println("Compiling foo v0.1.0");

    assert_eq!(
        output.lines(),
        &[
            "Building crate...".to_string(),
            "Compiling foo v0.1.0".to_string()
        ]
    );

    let first = output.drain_new();
    assert_eq!(
        first,
        vec![
            "Building crate...".to_string(),
            "Compiling foo v0.1.0".to_string()
        ]
    );
    assert!(output.drain_new().is_empty());

    output.println("Finished");
    assert_eq!(output.drain_new(), vec!["Finished".to_string()]);
}

#[test]
fn static_output_clear_resets_all_buffers() {
    let mut output = StaticOutput::new();
    output.println("line");
    output.clear();

    assert!(output.lines().is_empty());
    assert!(output.drain_new().is_empty());
}

#[test]
fn form_field_default_values() {
    let field = FormField::default();
    assert_eq!(field.label, "");
    assert_eq!(field.input.value, "");
    assert_eq!(field.input.cursor, 0);
    assert_eq!(field.error, None);
}

#[test]
fn toast_message_default_values() {
    let msg = ToastMessage::default();
    assert_eq!(msg.text, "");
    assert!(matches!(msg.level, ToastLevel::Info));
    assert_eq!(msg.created_tick, 0);
    assert_eq!(msg.duration_ticks, 30);
}

#[test]
fn list_state_default_values() {
    let state = ListState::default();
    assert!(state.items.is_empty());
    assert_eq!(state.selected, 0);
    assert_eq!(state.filter, "");
    assert!(state.visible_indices().is_empty());
    assert_eq!(state.selected_item(), None);
}

#[test]
fn file_entry_default_values() {
    let entry = FileEntry::default();
    assert_eq!(entry.name, "");
    assert_eq!(entry.path, PathBuf::new());
    assert!(!entry.is_dir);
    assert_eq!(entry.size, 0);
}

#[test]
fn tabs_state_default_values() {
    let state = TabsState::default();
    assert!(state.labels.is_empty());
    assert_eq!(state.selected, 0);
    assert_eq!(state.selected_label(), None);
}

#[test]
fn table_state_default_values() {
    let state = TableState::default();
    assert!(state.headers.is_empty());
    assert!(state.rows.is_empty());
    assert_eq!(state.selected, 0);
    assert_eq!(state.sort_column, None);
    assert!(state.sort_ascending);
    assert_eq!(state.filter, "");
    assert_eq!(state.page, 0);
    assert_eq!(state.page_size, 0);
    assert!(!state.zebra);
    assert!(state.visible_indices().is_empty());
    assert!(state.row_search_cache.is_empty());
    assert!(state.filter_tokens.is_empty());
}

#[test]
fn table_state_builds_lowercase_search_cache() {
    let state = TableState::new(vec!["Name"], vec![vec!["Alice Smith"], vec!["Bob"]]);

    assert_eq!(state.row_search_cache.len(), 2);
    assert_eq!(state.row_search_cache[0], "alice smith");
    assert_eq!(state.row_search_cache[1], "bob");
}

#[test]
fn table_filter_tokens_are_cached_and_reused() {
    let mut state = TableState::new(
        vec!["Name", "Role"],
        vec![vec!["Alice", "Admin"], vec!["Bob", "Viewer"]],
    );

    state.set_filter("AL admin");
    assert_eq!(
        state.filter_tokens,
        vec!["al".to_string(), "admin".to_string()]
    );
    assert_eq!(state.visible_indices(), &[0]);

    state.set_filter("AL admin");
    assert_eq!(
        state.filter_tokens,
        vec!["al".to_string(), "admin".to_string()]
    );
    assert_eq!(state.visible_indices(), &[0]);
}

#[test]
fn select_state_default_values() {
    let state = SelectState::default();
    assert!(state.items.is_empty());
    assert_eq!(state.selected, 0);
    assert!(!state.open);
    assert_eq!(state.placeholder, "");
    assert_eq!(state.selected_item(), None);
    assert_eq!(state.cursor(), 0);
}

#[test]
fn radio_state_default_values() {
    let state = RadioState::default();
    assert!(state.items.is_empty());
    assert_eq!(state.selected, 0);
    assert_eq!(state.selected_item(), None);
}

#[test]
fn text_input_state_default_uses_new() {
    let state = TextInputState::default();
    assert_eq!(state.value, "");
    assert_eq!(state.cursor, 0);
    assert_eq!(state.placeholder, "");
    assert_eq!(state.max_length, None);
    assert_eq!(state.validation_error, None);
    assert!(!state.masked);
}

#[test]
fn tabs_state_new_sets_labels() {
    let state = TabsState::new(vec!["a", "b"]);
    assert_eq!(state.labels, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(state.selected, 0);
    assert_eq!(state.selected_label(), Some("a"));
}

#[test]
fn list_state_new_selected_item_points_to_first_item() {
    let state = ListState::new(vec!["alpha", "beta"]);
    assert_eq!(state.items, vec!["alpha".to_string(), "beta".to_string()]);
    assert_eq!(state.selected, 0);
    assert_eq!(state.visible_indices(), &[0, 1]);
    assert_eq!(state.selected_item(), Some("alpha"));
}

#[test]
fn select_state_placeholder_builder_sets_value() {
    let state = SelectState::new(vec!["one", "two"]).placeholder("Pick one");
    assert_eq!(state.items, vec!["one".to_string(), "two".to_string()]);
    assert_eq!(state.placeholder, "Pick one");
    assert_eq!(state.selected_item(), Some("one"));
}

#[test]
fn select_filtered_indices_empty_query_returns_all() {
    let state = SelectState::new(vec!["Apple", "Banana", "Cherry"]);
    assert_eq!(state.filtered_indices(), vec![0, 1, 2]);
}

#[test]
fn select_filtered_indices_fuzzy_subsequence_preserves_order() {
    let mut state = SelectState::new(vec!["Apple", "Banana", "Cherry", "Mango"]);
    state.filter = "an".to_string();
    // "an" is a subsequence of Banana and Mango but not Apple/Cherry; original
    // order is preserved (no score reordering).
    assert_eq!(state.filtered_indices(), vec![1, 3]);
}

#[test]
fn select_filtered_indices_no_match_is_empty() {
    let mut state = SelectState::new(vec!["Apple", "Banana"]);
    state.filter = "zzz".to_string();
    assert!(state.filtered_indices().is_empty());
}

#[test]
fn radio_state_new_sets_items_and_selection() {
    let state = RadioState::new(vec!["red", "green"]);
    assert_eq!(state.items, vec!["red".to_string(), "green".to_string()]);
    assert_eq!(state.selected, 0);
    assert_eq!(state.selected_item(), Some("red"));
}

#[test]
fn table_state_new_sets_sort_ascending_true() {
    let state = TableState::new(vec!["Name"], vec![vec!["Alice"], vec!["Bob"]]);
    assert_eq!(state.headers, vec!["Name".to_string()]);
    assert_eq!(state.rows.len(), 2);
    assert!(state.sort_ascending);
    assert_eq!(state.sort_column, None);
    assert!(!state.zebra);
    assert_eq!(state.visible_indices(), &[0, 1]);
}

#[test]
fn command_palette_fuzzy_score_matches_gapped_pattern() {
    assert!(CommandPaletteState::fuzzy_score("sf", "Save File").is_some());
    assert!(CommandPaletteState::fuzzy_score("cmd", "Command Palette").is_some());
    assert_eq!(CommandPaletteState::fuzzy_score("xyz", "Save File"), None);
}

#[test]
fn command_palette_filtered_indices_uses_fuzzy_and_sorts() {
    let mut state = CommandPaletteState::new(vec![
        PaletteCommand::new("Save File", "Write buffer"),
        PaletteCommand::new("Search Files", "Find in workspace"),
        PaletteCommand::new("Quit", "Exit app"),
    ]);

    state.input = "sf".to_string();
    let filtered = state.filtered_indices();
    assert_eq!(filtered, vec![0, 1]);

    state.input = "buffer".to_string();
    let filtered = state.filtered_indices();
    assert_eq!(filtered, vec![0]);
}

#[test]
fn screen_state_push_pop_tracks_current_screen() {
    let mut screens = ScreenState::new("home");
    assert_eq!(screens.current(), "home");
    assert_eq!(screens.depth(), 1);
    assert!(!screens.can_pop());

    screens.push("settings");
    assert_eq!(screens.current(), "settings");
    assert_eq!(screens.depth(), 2);
    assert!(screens.can_pop());

    screens.push("profile");
    assert_eq!(screens.current(), "profile");
    assert_eq!(screens.depth(), 3);

    screens.pop();
    assert_eq!(screens.current(), "settings");
    assert_eq!(screens.depth(), 2);
}

#[test]
fn screen_state_pop_never_removes_root() {
    let mut screens = ScreenState::new("home");
    screens.push("settings");
    screens.pop();
    screens.pop();

    assert_eq!(screens.current(), "home");
    assert_eq!(screens.depth(), 1);
    assert!(!screens.can_pop());
}

#[test]
fn screen_state_reset_keeps_only_root() {
    let mut screens = ScreenState::new("home");
    screens.push("settings");
    screens.push("profile");
    assert_eq!(screens.current(), "profile");

    screens.reset();
    assert_eq!(screens.current(), "home");
    assert_eq!(screens.depth(), 1);
    assert!(!screens.can_pop());
}

#[test]
fn calendar_days_in_month_handles_leap_years() {
    assert_eq!(CalendarState::days_in_month(2024, 2), 29);
    assert_eq!(CalendarState::days_in_month(2023, 2), 28);
    assert_eq!(CalendarState::days_in_month(2024, 1), 31);
    assert_eq!(CalendarState::days_in_month(2024, 4), 30);
}

#[test]
fn calendar_first_weekday_known_dates() {
    assert_eq!(CalendarState::first_weekday(2024, 1), 0);
    assert_eq!(CalendarState::first_weekday(2023, 10), 6);
}

#[test]
fn calendar_prev_next_month_handles_year_boundary() {
    let mut state = CalendarState::from_ym(2024, 12);
    state.prev_month();
    assert_eq!((state.year, state.month), (2024, 11));

    let mut state = CalendarState::from_ym(2024, 1);
    state.prev_month();
    assert_eq!((state.year, state.month), (2023, 12));

    state.next_month();
    assert_eq!((state.year, state.month), (2024, 1));
}
