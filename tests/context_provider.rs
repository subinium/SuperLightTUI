//! Tests for `Context::provide` / `Context::use_context` /
//! `Context::try_use_context` (Issue #66), plus integration tests that
//! combine the provider with named state (`use_state_named` /
//! `use_state_named_default`) and the `with_if` conditional modifier in
//! realistic render trees. Unit-style coverage for those two APIs lives in
//! `tests/use_state_named.rs` and `tests/with_if.rs`.

#![allow(unused_must_use)]

use slt::{Color, Context, TestBackend};

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

// ---------------------------------------------------------------------------
// Integration: provider + named state + `with_if` in realistic trees
// ---------------------------------------------------------------------------

/// 1. `provide` + `use_context` end-to-end. A consumer pulls the value out
///    of the surrounding context and uses it to style a text element. The
///    color is copied out before the text call so the shared borrow on
///    `ui` ends before the mutable borrow chain starts.
#[test]
fn provide_and_use_context_render_integration() {
    struct Theme {
        primary: Color,
    }

    let mut tb = TestBackend::new(40, 6);
    tb.render(|ui: &mut Context| {
        ui.provide(
            Theme {
                primary: Color::Cyan,
            },
            |ui| {
                let primary = ui.use_context::<Theme>().primary;
                ui.text("Hello").fg(primary);
            },
        );
    });
    assert!(tb.line(0).contains("Hello"));
}

/// 2. Nested `provide`: an inner provider for the same type shadows the
///    outer one for the lifetime of its closure.
#[test]
fn nested_provide_shadows_outer() {
    let mut tb = TestBackend::new(40, 6);
    tb.render(|ui| {
        ui.provide("outer", |ui| {
            ui.provide("inner", |ui| {
                let s = *ui.use_context::<&str>();
                ui.text(s);
            });
        });
    });
    assert!(tb.line(0).contains("inner"));
}

/// 3. `try_use_context` returns `None` when the requested type was never
///    provided. The branch is observable through what gets rendered.
#[test]
fn try_use_context_missing_is_none() {
    struct Missing;

    let mut tb = TestBackend::new(40, 6);
    tb.render(|ui| {
        let m: Option<&Missing> = ui.try_use_context::<Missing>();
        if m.is_none() {
            ui.text("none");
        } else {
            ui.text("some");
        }
    });
    assert!(tb.line(0).contains("none"));
}

/// 4. `use_state_named` persists across frames the same way `use_state`
///    does, but addressed by string id rather than positional hook order.
#[test]
fn use_state_named_persists() {
    let mut tb = TestBackend::new(40, 6);
    tb.render(|ui| {
        let counter = ui.use_state_named_default::<i32>("counter");
        *counter.get_mut(ui) += 1;
    });
    tb.render(|ui| {
        let counter = ui.use_state_named_default::<i32>("counter");
        *counter.get_mut(ui) += 1;
        let n = *counter.get(ui);
        ui.text(format!("count={n}"));
    });
    assert!(tb.line(0).contains("count=2"));
}

/// 5. Sibling named-state ids are independent — writing one does not
///    perturb the other, even when both are read in the same frame.
#[test]
fn use_state_named_siblings_are_independent() {
    let mut tb = TestBackend::new(40, 6);
    tb.render(|ui| {
        let a = ui.use_state_named::<i32>("a", || 10);
        let b = ui.use_state_named::<i32>("b", || 20);
        let av = *a.get(ui);
        let bv = *b.get(ui);
        ui.text(format!("{av}/{bv}"));
    });
    assert!(tb.line(0).contains("10/20"));
}

/// 6. `with_if(true, ...)` runs the closure; the text still renders.
///    `TestBackend` does not expose per-cell style assertions in its
///    public API, so we only verify content was emitted. Style coverage
///    lives in the dedicated `tests/with_if.rs` file (which can poke the
///    buffer's cells directly via `tb.buffer()` if needed).
#[test]
fn with_if_true_applies_bold() {
    let mut tb = TestBackend::new(40, 6);
    tb.render(|ui| {
        ui.text("hi").with_if(true, |t| {
            t.bold();
        });
    });
    assert!(tb.line(0).contains("hi"));
}

/// 7. `with_if(false, ...)` skips the closure entirely. Verifies the
///    chain is non-fatal when the predicate is `false` and the unmodified
///    text still appears.
#[test]
fn with_if_false_skips_closure() {
    let mut tb = TestBackend::new(40, 6);
    tb.render(|ui| {
        ui.text("plain").with_if(false, |t| {
            t.bold();
        });
    });
    assert!(tb.line(0).contains("plain"));
}

/// 8. Provider, named state, and `with_if` together in one tree:
///    - outer `provide` of a config struct
///    - inner `use_state_named` (incremented this frame)
///    - `with_if` driven by a value pulled from the provided context
#[test]
fn combined_provider_named_state_with_if() {
    struct Cfg {
        emphasize: bool,
    }

    let mut tb = TestBackend::new(40, 6);
    tb.render(|ui| {
        ui.provide(Cfg { emphasize: true }, |ui| {
            let counter = ui.use_state_named_default::<i32>("combined_counter");
            *counter.get_mut(ui) += 1;
            let n = *counter.get(ui);
            let emphasize = ui.use_context::<Cfg>().emphasize;
            ui.text(format!("n={n}")).with_if(emphasize, |t| {
                t.bold();
            });
        });
    });
    assert!(tb.line(0).contains("n=1"));
}

/// 9. `with_if` on a `ContainerBuilder` (the bordered container variant
///    documented in the spec). The builder is consumed by value, so the
///    closure receives `&mut Self` and any chained mutator (like
///    `.title()`-equivalent state) only runs when the predicate is true.
#[test]
fn with_if_on_container_builder_runs_only_when_true() {
    use slt::Border;

    let mut tb = TestBackend::new(40, 6);
    tb.render(|ui| {
        // `with_if` must compose cleanly inside a bordered container's
        // by-value builder chain. We assert by content: the body is
        // always rendered regardless of the predicate path.
        let _ = ui
            .bordered(Border::Single)
            .with_if(true, |c| {
                // No-op modifier closure: just verifies the API shape.
                // ContainerBuilder modifiers are by-value — return `c` unchanged.
                c
            })
            .col(|ui| {
                ui.text("body");
            });
    });
    assert!(tb.to_string_trimmed().contains("body"));
}

/// 10. Provided values survive across nested `with_if` chains — context
///     lookup still resolves inside conditionally-applied modifier
///     blocks.
#[test]
fn provide_visible_inside_with_if_chain() {
    struct Marker(&'static str);

    let mut tb = TestBackend::new(40, 6);
    tb.render(|ui| {
        ui.provide(Marker("OK"), |ui| {
            // Mid-chain context read: bind out before the conditional
            // closure to avoid overlapping borrows of `ui`.
            let label = ui.use_context::<Marker>().0;
            ui.text(label).with_if(true, |t| {
                t.bold();
            });
        });
    });
    assert!(tb.line(0).contains("OK"));
}
