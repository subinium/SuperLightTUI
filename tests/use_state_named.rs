//! Tests for `Context::use_state_named` /
//! `Context::use_state_named_default` (Issue #71, naming unified in #240).

use slt::TestBackend;

#[test]
fn named_state_persists_across_frames() {
    let mut backend = TestBackend::new(40, 4);

    // Frame 1: initialize, then mutate to 5.
    backend.render(|ui| {
        let v = ui.use_state_named::<i32>("counter::value", || 0);
        let cur = *v.get(ui);
        if cur < 5 {
            *v.get_mut(ui) = 5;
        }
        ui.text(format!("v={}", v.get(ui)));
    });
    assert!(backend.to_string_trimmed().contains("v=5"));

    // Frame 2: should still be 5 (init closure must NOT re-run).
    backend.render(|ui| {
        let v = ui.use_state_named::<i32>("counter::value", || 99);
        ui.text(format!("v={}", v.get(ui)));
    });
    assert!(
        backend.to_string_trimmed().contains("v=5"),
        "expected persisted v=5 in frame 2, got: {:?}",
        backend.to_string_trimmed()
    );
}

#[test]
fn two_different_ids_have_independent_state() {
    let mut backend = TestBackend::new(40, 4);
    backend.render(|ui| {
        let a = ui.use_state_named::<i32>("widget::a", || 1);
        let b = ui.use_state_named::<i32>("widget::b", || 2);
        assert_eq!(*a.get(ui), 1);
        assert_eq!(*b.get(ui), 2);
        *a.get_mut(ui) = 100;
        assert_eq!(*a.get(ui), 100);
        // Mutating a must not affect b.
        assert_eq!(*b.get(ui), 2);
    });
}

#[test]
fn same_id_in_same_scope_is_shared() {
    // Documented behavior: two calls with the same id share storage.
    let mut backend = TestBackend::new(40, 4);
    backend.render(|ui| {
        let first = ui.use_state_named::<i32>("shared", || 7);
        // Init closure ignored on second call because the id already exists.
        let second = ui.use_state_named::<i32>("shared", || 999);
        assert_eq!(*first.get(ui), 7);
        assert_eq!(*second.get(ui), 7);
        *first.get_mut(ui) = 42;
        assert_eq!(*second.get(ui), 42);
    });
}

#[test]
fn use_state_named_default_uses_default_impl() {
    let mut backend = TestBackend::new(40, 4);
    backend.render(|ui| {
        let v = ui.use_state_named_default::<i32>("default::int");
        assert_eq!(*v.get(ui), 0);
        let s = ui.use_state_named_default::<String>("default::str");
        assert!(s.get(ui).is_empty());
    });
}

#[test]
#[should_panic(expected = "use_state_named type mismatch")]
fn type_mismatch_on_get_panics() {
    // First create an i32 entry, then ask for a String at the same id.
    // The handle returned by the second call carries `String` as its type
    // tag, so `.get(ui)` will fail to downcast and panic with a clear message.
    let mut backend = TestBackend::new(40, 4);
    backend.render(|ui| {
        let _ = ui.use_state_named::<i32>("collide", || 5);
        let s = ui.use_state_named::<String>("collide", String::new);
        let _ = s.get(ui);
    });
}

#[test]
fn safe_inside_conditional_unlike_use_state() {
    // Demonstrates the key advantage over `use_state`: safe even when the
    // call is gated behind a runtime condition that flips between frames.
    let mut backend = TestBackend::new(40, 4);

    // Frame 1: branch taken, initialize to 10 then mutate to 42.
    backend.render(|ui| {
        let v = ui.use_state_named::<i32>("conditional::v", || 10);
        *v.get_mut(ui) = 42;
        ui.text("frame 1");
    });

    // Frame 2: branch NOT taken — no call to use_state_named at all.
    backend.render(|ui| {
        ui.text("frame 2");
    });

    // Frame 3: branch taken again, must observe the persisted 42.
    backend.render(|ui| {
        let v = ui.use_state_named::<i32>("conditional::v", || 99);
        ui.text(format!("v={}", v.get(ui)));
    });
    let out = backend.to_string_trimmed();
    assert!(out.contains("v=42"), "expected v=42, got: {out:?}");
}

#[test]
fn deprecated_use_state_named_with_alias_still_works() {
    // `use_state_named_with` is a #[deprecated(since = "0.21.0")] alias for the
    // new `use_state_named(id, init)`. It must remain functionally identical
    // until removed in v1.0. `#[allow(deprecated)]` keeps `-D warnings` green.
    #[allow(deprecated)]
    {
        let mut backend = TestBackend::new(40, 4);
        backend.render(|ui| {
            let v = ui.use_state_named_with::<i32>("alias::value", || 7);
            assert_eq!(*v.get(ui), 7);
            *v.get_mut(ui) = 11;
        });
        // Second frame: the alias shares the same `named_states` slot as the
        // canonical method, so the init closure must not re-run.
        backend.render(|ui| {
            let v = ui.use_state_named::<i32>("alias::value", || 99);
            assert_eq!(*v.get(ui), 11);
            ui.text(format!("v={}", v.get(ui)));
        });
        assert!(backend.to_string_trimmed().contains("v=11"));
    }
}
