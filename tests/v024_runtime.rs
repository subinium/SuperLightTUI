use slt::{AppState, Backend, Buffer, Context, Event, EventBuilder, Rect, RunConfig, TestBackend};
use std::io;
use std::time::Duration;

struct Sink {
    buffer: Buffer,
    delay: Duration,
}

impl Sink {
    fn new(width: u32, height: u32) -> Self {
        Self {
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
            delay: Duration::ZERO,
        }
    }

    fn render(&mut self, state: &mut AppState, events: &[Event], mut f: impl FnMut(&mut Context)) {
        self.buffer.reset();
        assert!(slt::frame(self, state, &RunConfig::default(), events, &mut f).unwrap());
    }
}

impl Backend for Sink {
    fn size(&self) -> (u32, u32) {
        (self.buffer.area.width, self.buffer.area.height)
    }
    fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }
    fn flush(&mut self) -> io::Result<()> {
        std::thread::sleep(self.delay);
        Ok(())
    }
}

#[test]
fn test_backend_advances_ticks_like_frame() {
    let mut backend = TestBackend::new(20, 3);
    let mut seen = Vec::new();
    for _ in 0..3 {
        backend.render(|ui| seen.push(ui.tick()));
    }
    assert_eq!(seen, [0, 1, 2]);
    backend
        .sequence()
        .tick(|ui| assert_eq!(ui.tick(), 3))
        .tick(|ui| assert_eq!(ui.tick(), 4))
        .run();
}

#[test]
fn deterministic_clock_drives_scheduler_without_sleeping() {
    let mut backend = TestBackend::new(20, 3);
    backend.advance_time(Duration::ZERO).unwrap();
    backend.render(|ui| {
        assert!(!ui.schedule("once", Duration::from_millis(10)));
        assert_eq!(ui.every("repeat", Duration::from_millis(5)), 0);
        assert!(!ui.debounce("quiet", Duration::from_millis(10), true));
    });
    backend.advance_time(Duration::from_millis(12)).unwrap();
    backend.render(|ui| {
        assert!(ui.schedule("once", Duration::from_millis(10)));
        assert_eq!(ui.every("repeat", Duration::from_millis(5)), 2);
        assert!(ui.debounce("quiet", Duration::from_millis(10), false));
    });
    backend.render(|ui| {
        assert!(!ui.schedule("once", Duration::from_millis(10)));
        assert_eq!(ui.every("repeat", Duration::from_millis(5)), 0);
        assert!(!ui.debounce("quiet", Duration::from_millis(10), false));
    });
}

#[test]
fn custom_frame_retains_hover_and_reduces_focus_loss_in_order() {
    let mut sink = Sink::new(20, 3);
    let mut state = AppState::new();
    sink.render(&mut state, &EventBuilder::new().click(2, 1).build(), |ui| {
        assert_eq!(ui.mouse_pos(), Some((2, 1)));
    });
    sink.render(&mut state, &[], |ui| {
        assert_eq!(ui.mouse_pos(), Some((2, 1)))
    });
    sink.render(
        &mut state,
        &EventBuilder::new().click(2, 1).focus_lost().build(),
        |ui| {
            assert_eq!(ui.mouse_pos(), None);
        },
    );
    sink.render(
        &mut state,
        &EventBuilder::new().focus_lost().click(3, 1).build(),
        |ui| {
            assert_eq!(ui.mouse_pos(), Some((3, 1)));
        },
    );
}

#[test]
fn custom_frame_invalidates_old_hit_areas_on_explicit_or_implicit_resize() {
    for explicit in [false, true] {
        let mut sink = Sink::new(80, 4);
        let mut state = AppState::new();
        sink.render(&mut state, &[], |ui| {
            let _ = ui.row(|ui| {
                ui.spacer();
                let _ = ui.button("go");
            });
        });
        let old_x = (0..80)
            .find(|&x| sink.buffer.get(x, 0).symbol == "g")
            .unwrap();
        sink.buffer.resize(Rect::new(0, 0, 120, 4));
        let events = if explicit {
            EventBuilder::new().resize(120, 4).click(old_x, 0).build()
        } else {
            EventBuilder::new().click(old_x, 0).build()
        };
        sink.render(&mut state, &events, |ui| {
            let _ = ui.row(|ui| {
                ui.spacer();
                assert!(!ui.button("go").clicked);
            });
        });
        let new_x = (0..120)
            .find(|&x| sink.buffer.get(x, 0).symbol == "g")
            .unwrap();
        assert_ne!(old_x, new_x);
        sink.render(
            &mut state,
            &EventBuilder::new().click(new_x, 0).build(),
            |ui| {
                let _ = ui.row(|ui| {
                    ui.spacer();
                    assert!(ui.button("go").clicked);
                });
            },
        );
    }
}

#[test]
fn custom_frame_debug_keys_toggle_once() {
    let mut sink = Sink::new(20, 3);
    let mut state = AppState::new();
    sink.render(
        &mut state,
        &EventBuilder::new().key_code(slt::KeyCode::F(12)).build(),
        |ui| {
            assert!(ui.debug_enabled());
        },
    );
    sink.render(&mut state, &[], |ui| assert!(ui.debug_enabled()));
    sink.render(
        &mut state,
        &EventBuilder::new().key_code(slt::KeyCode::F(12)).build(),
        |ui| {
            assert!(!ui.debug_enabled());
        },
    );
}

#[test]
fn fps_measures_frame_cadence_including_backend_flush() {
    let mut sink = Sink::new(20, 3);
    sink.delay = Duration::from_millis(30);
    let mut state = AppState::new();
    for _ in 0..3 {
        sink.render(&mut state, &[], |ui| {
            ui.text("frame");
        });
    }
    assert!(
        state.fps_f64() > 0.0 && state.fps_f64() < 100.0,
        "fps={}",
        state.fps_f64()
    );
    assert!(state.flush_duration() >= Duration::from_millis(25));
    assert!(state.frame_interval() >= Duration::from_millis(25));
}
