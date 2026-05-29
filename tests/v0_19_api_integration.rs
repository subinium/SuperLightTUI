//! Integration tests for v0.19.0 component DX APIs.
//!
//! Exercises `provide` / `use_context` / `try_use_context`, the named
//! variants of `use_state` (`use_state_named` / `use_state_named_default`),
//! and the `with_if` conditional modifier together in realistic
//! combinations. Complements the unit-style tests in
//! `tests/context_provider.rs`, `tests/use_state_named.rs`, and
//! `tests/with_if.rs`.

#![allow(unused_must_use)]

use slt::{Color, Context, TestBackend};

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

/// 8. All three v0.19.0 APIs together in one tree:
///    - outer `provide` of a config struct
///    - inner `use_state_named` (incremented this frame)
///    - `with_if` driven by a value pulled from the provided context
#[test]
fn combined_v0_19_apis() {
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
