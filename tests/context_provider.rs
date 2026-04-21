//! Tests for `Context::provide` / `Context::use_context` /
//! `Context::try_use_context` (Issue #66).

use slt::TestBackend;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ThemeAccent(&'static str);

#[derive(Debug, Clone, PartialEq, Eq)]
struct UserName(String);

#[test]
fn provide_and_use_context_round_trip() {
    let mut backend = TestBackend::new(40, 4);
    backend.render(|ui| {
        ui.provide(ThemeAccent("red"), |ui| {
            let theme = ui.use_context::<ThemeAccent>();
            assert_eq!(theme.0, "red");
            ui.text(format!("accent={}", theme.0));
        });
    });
    assert!(backend.to_string_trimmed().contains("accent=red"));
}

#[test]
fn nested_provide_inner_shadows_outer_for_same_type() {
    let mut backend = TestBackend::new(40, 4);
    backend.render(|ui| {
        ui.provide(ThemeAccent("outer"), |ui| {
            assert_eq!(ui.use_context::<ThemeAccent>().0, "outer");
            ui.provide(ThemeAccent("inner"), |ui| {
                assert_eq!(ui.use_context::<ThemeAccent>().0, "inner");
            });
            // After inner pops, outer is visible again.
            assert_eq!(ui.use_context::<ThemeAccent>().0, "outer");
        });
    });
}

#[test]
fn two_different_types_coexist_on_stack() {
    let mut backend = TestBackend::new(40, 4);
    backend.render(|ui| {
        ui.provide(ThemeAccent("blue"), |ui| {
            ui.provide(UserName("alice".to_string()), |ui| {
                assert_eq!(ui.use_context::<ThemeAccent>().0, "blue");
                assert_eq!(ui.use_context::<UserName>().0, "alice");
            });
        });
    });
}

#[test]
fn try_use_context_returns_none_when_not_provided() {
    let mut backend = TestBackend::new(40, 4);
    backend.render(|ui| {
        assert!(ui.try_use_context::<ThemeAccent>().is_none());
    });
}

#[test]
fn try_use_context_returns_some_when_provided() {
    let mut backend = TestBackend::new(40, 4);
    backend.render(|ui| {
        ui.provide(ThemeAccent("ok"), |ui| {
            assert_eq!(ui.try_use_context::<ThemeAccent>().map(|t| t.0), Some("ok"));
        });
    });
}

#[test]
#[should_panic(expected = "no context of type")]
fn use_context_panics_when_not_provided() {
    let mut backend = TestBackend::new(40, 4);
    backend.render(|ui| {
        let _ = ui.use_context::<ThemeAccent>();
    });
}

#[test]
fn context_stack_pops_after_provide_returns() {
    let mut backend = TestBackend::new(40, 4);
    backend.render(|ui| {
        ui.provide(ThemeAccent("temp"), |ui| {
            assert!(ui.try_use_context::<ThemeAccent>().is_some());
        });
        // Outside the provide closure, the value must be gone.
        assert!(ui.try_use_context::<ThemeAccent>().is_none());
    });
}

#[test]
fn provide_returns_body_value() {
    let mut backend = TestBackend::new(40, 4);
    backend.render(|ui| {
        let doubled = ui.provide(42i32, |ui| *ui.use_context::<i32>() * 2);
        assert_eq!(doubled, 84);
    });
}
