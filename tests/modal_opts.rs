//! Issue #462: `modal()` is `modal_with(ModalOpts::default())`, the options
//! type lives at the crate root, and anchored placement folds into
//! `ModalOpts`.

use slt::{Anchor, Border, Context, EventBuilder, KeyCode, ModalOpts, TestBackend};

/// Background buttons, then a modal with two buttons. Records which label
/// reports `focused` each frame.
fn scene(
    ui: &mut Context,
    modal: &dyn Fn(&mut Context, &mut dyn FnMut(&mut Context)),
    log: &mut Vec<String>,
) {
    let mut focused = String::from("-");
    if ui.button("Back A").focused {
        focused = "Back A".into();
    }
    if ui.button("Back B").focused {
        focused = "Back B".into();
    }
    let mut body = |ui: &mut Context| {
        let _ = ui.bordered(Border::Rounded).col(|ui| {
            if ui.button("Yes").focused {
                focused = "Yes".into();
            }
            if ui.button("No").focused {
                focused = "No".into();
            }
        });
    };
    modal(ui, &mut body);
    log.push(focused);
}

/// Drive `modal` through Tab, Shift+Tab, a programmatic jump to a background
/// slot, and more Tabs. Returns the focus log, the final focus index and the
/// final buffer.
fn drive(
    modal: &dyn Fn(&mut Context, &mut dyn FnMut(&mut Context)),
) -> (Vec<String>, usize, String) {
    let mut tb = TestBackend::new(40, 12);
    let mut log = Vec::new();
    let mut index = 0usize;
    tb.render(|ui| scene(ui, modal, &mut log));
    let tab = || EventBuilder::new().key_code(KeyCode::Tab).build();
    let back_tab = || EventBuilder::new().key_code(KeyCode::BackTab).build();
    for events in [tab(), tab(), back_tab(), Vec::new()] {
        tb.run_with_events_keep_focus(events, |ui| {
            scene(ui, modal, &mut log);
            index = ui.focus_index();
        });
    }
    // Programmatic focus to a background slot index while the modal is open.
    tb.render(|ui| {
        scene(ui, modal, &mut log);
        ui.set_focus_index(7);
    });
    for events in [Vec::new(), tab(), tab()] {
        tb.run_with_events_keep_focus(events, |ui| {
            scene(ui, modal, &mut log);
            index = ui.focus_index();
        });
    }
    (log, index, tb.to_string_trimmed())
}

trait KeepFocus {
    fn run_with_events_keep_focus(&mut self, events: Vec<slt::Event>, f: impl FnOnce(&mut Context));
}

impl KeepFocus for TestBackend {
    fn run_with_events_keep_focus(
        &mut self,
        events: Vec<slt::Event>,
        f: impl FnOnce(&mut Context),
    ) {
        self.sequence().events(events, f).run();
    }
}

#[test]
fn modal_equals_modal_with_default_options() {
    let plain = drive(&|ui, body| {
        let _ = ui.modal(|ui| body(ui));
    });
    let with_default = drive(&|ui, body| {
        let _ = ui.modal_with(ModalOpts::default(), |ui| body(ui));
    });
    assert_eq!(plain, with_default);
}

#[test]
fn modal_traps_focus_by_default() {
    let (log, index, _) = drive(&|ui, body| {
        let _ = ui.modal(|ui| body(ui));
    });
    assert!(
        index < 2,
        "trapped modal focus index stays in range, got {index}"
    );
    assert!(
        log.iter()
            .all(|label| label != "Back A" && label != "Back B"),
        "background buttons never take focus while the modal is open: {log:?}"
    );
}

#[test]
fn tab_trap_off_reproduces_the_legacy_modal() {
    let (log, index, buffer) = drive(&|ui, body| {
        let _ = ui.modal_with(ModalOpts::new().tab_trap(false), |ui| body(ui));
    });
    // Fixture captured from v0.25 `modal()` (trap off) before the default
    // flipped.
    assert_eq!(log, LEGACY_LOG, "focus log");
    assert_eq!(index, LEGACY_INDEX, "final focus index");
    let (_, _, trapped_buffer) = drive(&|ui, body| {
        let _ = ui.modal(|ui| body(ui));
    });
    assert_eq!(
        buffer, trapped_buffer,
        "trap flag does not change rendering"
    );
}

const LEGACY_LOG: &[&str] = &["Yes", "Yes", "No", "Yes", "No", "No", "No", "No", "Yes"];
const LEGACY_INDEX: usize = 0;

#[test]
fn legacy_trap_off_leaves_stale_focus_untouched() {
    let mut tb = TestBackend::new(60, 8);
    tb.render(|ui| {
        let _ = ui.modal_with(ModalOpts::new().tab_trap(false), |ui| {
            let _ = ui.button("OK");
        });
    });
    let mut final_idx = usize::MAX;
    tb.render_with_events(Vec::new(), 99, 1, |ui| {
        let _ = ui.modal_with(ModalOpts::new().tab_trap(false), |ui| {
            let _ = ui.button("OK");
        });
        final_idx = ui.focus_index();
    });
    assert_eq!(final_idx, 99);

    // The default modal clamps the same stale index into its range.
    let mut tb = TestBackend::new(60, 8);
    tb.render(|ui| {
        let _ = ui.modal(|ui| {
            let _ = ui.button("OK");
        });
    });
    tb.render_with_events(Vec::new(), 99, 1, |ui| {
        let _ = ui.modal(|ui| {
            let _ = ui.button("OK");
        });
        final_idx = ui.focus_index();
    });
    assert_eq!(final_idx, 0);
}

fn anchored_body(ui: &mut Context) {
    let _ = ui.bordered(Border::Rounded).p(1).col(|ui| {
        ui.text("Saved!");
    });
}

#[test]
#[allow(deprecated)]
fn modal_opts_anchor_offset_matches_modal_at_offset() {
    let mut legacy = TestBackend::new(40, 10);
    legacy.render(|ui| {
        ui.text("background");
        let _ = ui.modal_at_offset(Anchor::TopRight, 2, 1, anchored_body);
    });
    let mut opts = TestBackend::new(40, 10);
    opts.render(|ui| {
        ui.text("background");
        let _ = ui.modal_with(
            ModalOpts::new().anchor(Anchor::TopRight).offset(2, 1),
            anchored_body,
        );
    });
    assert_eq!(legacy.to_string_trimmed(), opts.to_string_trimmed());
    // Pinned two cells in from the right edge and one row down.
    let (x, y) = opts.find_text("Saved!").expect("modal content");
    assert_eq!(y, 3, "border + padding below the 1-row offset");
    assert_eq!(
        x + "Saved!".len() as u32,
        40 - 2 - 2,
        "2-cell inset + border + padding"
    );

    let mut at = TestBackend::new(40, 10);
    at.render(|ui| {
        let _ = ui.modal_at(Anchor::BottomLeft, anchored_body);
    });
    let mut opts = TestBackend::new(40, 10);
    opts.render(|ui| {
        let _ = ui.modal_with(ModalOpts::new().anchor(Anchor::BottomLeft), anchored_body);
    });
    assert_eq!(at.to_string_trimmed(), opts.to_string_trimmed());
}

#[test]
#[allow(deprecated)]
fn deprecated_options_alias_still_compiles() {
    let opts: slt::context::ModalOptions = slt::context::ModalOptions::default();
    assert_eq!(opts, ModalOpts::new());
    assert!(opts.tab_trap);
    assert_eq!(opts.anchor, None);
    assert_eq!(opts.offset, (0, 0));
}
