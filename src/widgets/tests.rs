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

// ── Form validators ──────────────────────────────────────────────────────

#[test]
fn validators_required_rejects_empty_and_whitespace() {
    let v = validators::required("required");
    assert_eq!(v(""), Err("required".to_string()));
    assert_eq!(v("   "), Err("required".to_string()));
    assert_eq!(v("\t\n"), Err("required".to_string()));
    assert!(v("a").is_ok());
    assert!(v(" x ").is_ok());
}

#[test]
fn validators_email_structural_check() {
    let v = validators::email();
    assert!(v("a@b.co").is_ok());
    assert!(v("jane.doe@example.com").is_ok());
    assert!(v("a").is_err());
    assert!(v("a@").is_err());
    assert!(v("@b.co").is_err());
    assert!(v("a@b").is_err()); // no dot in domain
    assert!(v("a@@b.co").is_err()); // two '@'
    assert!(v("a b@c.co").is_err()); // whitespace
    assert!(v("a@.co").is_err()); // empty host label
    assert!(v("a@b.").is_err()); // empty tld
}

#[test]
fn validators_range_i64_bounds_and_parse() {
    let v = validators::range_i64(1, 10, "1..=10");
    assert_eq!(v("0"), Err("1..=10".to_string()));
    assert_eq!(v("11"), Err("1..=10".to_string()));
    assert_eq!(v("x"), Err("1..=10".to_string()));
    assert!(v("5").is_ok());
    assert!(v("1").is_ok());
    assert!(v("10").is_ok());
    assert!(v(" 5 ").is_ok()); // trims
}

#[test]
fn validators_range_f64_rejects_nonfinite_and_out_of_range() {
    let v = validators::range_f64(0.0, 1.0, "0..=1");
    assert!(v("0.5").is_ok());
    assert!(v("0").is_ok());
    assert!(v("1").is_ok());
    assert!(v("-0.1").is_err());
    assert!(v("1.1").is_err());
    assert!(v("inf").is_err());
    assert!(v("nan").is_err());
    assert!(v("abc").is_err());
}

#[test]
fn validators_one_of_exact_match() {
    let v = validators::one_of(&["admin", "user"], "bad role");
    assert!(v("admin").is_ok());
    assert!(v("user").is_ok());
    assert_eq!(v("Admin"), Err("bad role".to_string())); // case-sensitive
    assert_eq!(v("guest"), Err("bad role".to_string()));
}

#[test]
fn validators_regex_glob_matcher() {
    // '.' = any char, '*' = zero-or-more, fully anchored.
    let v = validators::regex("a.c", "no match");
    assert!(v("abc").is_ok());
    assert!(v("axc").is_ok());
    assert!(v("ac").is_err()); // '.' needs exactly one char
    assert!(v("abcd").is_err()); // anchored to end

    // "a*" = literal 'a' then zero-or-more of any char.
    let star = validators::regex("a*", "no match");
    assert!(star("").is_err()); // missing the leading literal 'a'
    assert!(star("a").is_ok());
    assert!(star("aaaa").is_ok());
    assert!(star("abcd").is_ok()); // 'a' then '*' soaks the rest

    let anchored = validators::regex("^foo$", "no match");
    assert!(anchored("foo").is_ok());
    assert!(anchored("foobar").is_err());

    let prefix = validators::regex("foo*", "no match");
    assert!(prefix("foo").is_ok());
    assert!(prefix("foobar").is_ok());
}

#[test]
fn validator_closure_captures_outer_state() {
    // Proves the boxed closure form: captures `min`, impossible with a fn ptr.
    let min: usize = 4;
    let v = Validator::new(move |s: &str| {
        if s.chars().count() >= min {
            Ok(())
        } else {
            Err(format!("min {min}"))
        }
    });
    assert!(v.run("abcd").is_ok());
    assert_eq!(v.run("ab"), Err("min 4".to_string()));
}

#[test]
fn form_field_validate_builder_runs_first_failure() {
    let mut field = FormField::new("Email")
        .validate(validators::required("required"))
        .validate(validators::email());
    assert_eq!(field.validator_count(), 2);
    // Empty -> first validator (required) fails.
    assert!(!field.run_validators());
    assert_eq!(field.error.as_deref(), Some("required"));
    // Non-empty but not an email -> second validator fails.
    field.input.value = "abc".into();
    assert!(!field.run_validators());
    assert_eq!(field.error.as_deref(), Some("invalid email"));
    // Valid email -> clears.
    field.input.value = "a@b.co".into();
    assert!(field.run_validators());
    assert_eq!(field.error, None);
}

#[test]
fn form_field_default_trigger_is_on_blur() {
    let field = FormField::new("X");
    assert_eq!(field.trigger, ValidateTrigger::OnBlur);
    assert_eq!(
        FormField::new("Y").on_change().trigger,
        ValidateTrigger::OnChange
    );
    assert_eq!(
        FormField::new("Z").manual().trigger,
        ValidateTrigger::Manual
    );
}

#[test]
fn form_state_is_valid_and_errors() {
    let mut form = FormState::new()
        .field(FormField::new("Name").validate(validators::required("name required")))
        .field(FormField::new("Email").validate(validators::email()));
    // No validation run yet -> no errors -> valid.
    assert!(form.is_valid());
    assert!(form.errors().is_empty());

    // Run all -> both empty fields fail.
    assert!(!form.validate_all());
    assert!(!form.is_valid());
    let errors = form.errors();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0], (0, "name required"));
    assert_eq!(errors[1], (1, "invalid email"));

    // Fix both -> valid.
    form.fields[0].input.value = "Jane".into();
    form.fields[1].input.value = "jane@example.com".into();
    assert!(form.validate_all());
    assert!(form.is_valid());
    assert!(form.errors().is_empty());
}

#[test]
fn form_state_validate_with_cross_field_rule() {
    let mut form = FormState::new()
        .field(FormField::new("Password"))
        .field(FormField::new("Confirm"));
    form.fields[0].input.value = "secret".into();
    form.fields[1].input.value = "different".into();

    let ok = form.validate_with(|f| {
        if f.value(0) != f.value(1) {
            vec![(1, "passwords must match".to_string())]
        } else {
            vec![]
        }
    });
    assert!(!ok);
    assert_eq!(
        form.fields[1].error.as_deref(),
        Some("passwords must match")
    );
    assert_eq!(form.errors(), vec![(1, "passwords must match")]);

    // Make them match -> no cross-field error reported.
    form.fields[1].input.value = "secret".into();
    let ok = form.validate_with(|f| {
        if f.value(0) != f.value(1) {
            vec![(1, "passwords must match".to_string())]
        } else {
            vec![]
        }
    });
    assert!(ok);
}

#[test]
#[allow(deprecated)]
fn form_state_deprecated_positional_validate_still_works() {
    let mut form = FormState::new()
        .field(FormField::new("A"))
        .field(FormField::new("B"));
    let validators: &[FormValidator] = &[
        |v| {
            if v.is_empty() {
                Err("a empty".into())
            } else {
                Ok(())
            }
        },
        |_| Ok(()),
    ];
    assert!(!form.validate(validators));
    assert_eq!(form.fields[0].error.as_deref(), Some("a empty"));
    assert_eq!(form.fields[1].error, None);
}
