use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use slt::{CalendarState, Color, Event, SplitPaneState, TextInputState};
use slt_wasm::{WasmAppHandle, WasmOptions, run_wasm_with_options};
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

#[derive(Default)]
struct Evidence {
    frames: Cell<u32>,
    phase: Cell<u32>,
    events: RefCell<Vec<String>>,
    text: RefCell<String>,
    ratio: Cell<f64>,
    dropped: Cell<bool>,
    timers: Cell<bool>,
    double_clicked: Cell<bool>,
}

struct Sentinel(Rc<Evidence>);
impl Drop for Sentinel {
    fn drop(&mut self) {
        self.0.dropped.set(true);
    }
}

/// Compiled Rust fixture used by browser.test.cjs, never a JS projection.
#[wasm_bindgen]
pub struct Probe {
    runtime: Option<WasmAppHandle>,
    evidence: Rc<Evidence>,
}

#[wasm_bindgen]
impl Probe {
    #[wasm_bindgen(constructor)]
    pub fn new(host: HtmlElement, mode: u32, fps: u32, auto_fit: bool) -> Result<Probe, JsValue> {
        let evidence = Rc::new(Evidence::default());
        let data = Rc::clone(&evidence);
        let sentinel = Sentinel(Rc::clone(&evidence));
        let mut input = TextInputState::new();
        let mut split = SplitPaneState::new(0.5);
        let calendar = CalendarState::default();
        let mut scheduled = false;
        let mut repeated = false;
        let mut debounced = false;
        let mut options = WasmOptions {
            width: 16,
            height: 6,
            max_fps: (fps > 0).then_some(fps),
            auto_fit,
            ..WasmOptions::default()
        };
        options.scroll_speed = 3;
        let runtime = run_wasm_with_options(host.clone(), options, move |ui| {
            let _keep_alive = &sentinel;
            let _calendar = &calendar;
            let frame = data.frames.get() + 1;
            data.frames.set(frame);
            for event in ui.events() {
                data.events.borrow_mut().push(format!("{event:?}"));
            }
            scheduled |= ui.schedule("once", Duration::from_millis(20));
            repeated |= ui.every("repeat", Duration::from_millis(20)) > 0;
            debounced |= ui.debounce("debounce", Duration::from_millis(20), frame == 1);
            data.timers.set(scheduled && repeated && debounced);
            match mode {
                0 => match data.phase.get() {
                    0 => {
                        ui.text("ABCDEFGH").bold().fg(Color::Red);
                    }
                    1 => {
                        ui.text("X");
                    }
                    2 => {}
                    3 => {
                        ui.text("\u{754c}A\u{1f469}\u{200d}\u{1f4bb}B");
                    }
                    4 => {
                        ui.text("short");
                    }
                    5 => {
                        ui.text("base");
                        let _ = ui.modal(|ui| {
                            ui.text("MODAL");
                        });
                    }
                    _ => {
                        ui.text("base");
                    }
                },
                1 => {
                    let _ = ui.text_input(&mut input);
                    *data.text.borrow_mut() = input.value.clone();
                    let response = ui.button("Click");
                    if response.double_clicked {
                        data.double_clicked.set(true);
                    }
                    if response.clicked {
                        data.phase.set(data.phase.get() + 1);
                    }
                    ui.text("\u{754c}A\u{1f469}\u{200d}\u{1f4bb}B");
                }
                2 => {
                    let _ = ui.split_pane(
                        &mut split,
                        |ui| {
                            ui.text("L");
                        },
                        |ui| {
                            ui.text("R");
                        },
                    );
                    data.ratio.set(split.ratio);
                }
                3 if frame >= 3 => ui.quit(),
                4 if frame >= 3 => panic!("intentional isolated fatal-frame test"),
                5 if frame == 3 => {
                    let event = web_sys::Event::new("resize").expect("resize event");
                    web_sys::window()
                        .expect("window")
                        .dispatch_event(&event)
                        .expect("dispatch");
                    let event = web_sys::Event::new("slt-dispose").expect("custom event");
                    host.dispatch_event(&event).expect("dispatch");
                }
                _ => {
                    ui.text(format!("frame {frame}"));
                }
            }
            if ui.events().any(|event| matches!(event, Event::FocusLost)) {
                data.events.borrow_mut().push("focus-lost-observed".into());
            }
        })?;
        Ok(Self {
            runtime: Some(runtime),
            evidence,
        })
    }

    pub fn frames(&self) -> u32 {
        self.evidence.frames.get()
    }
    pub fn phase(&self) -> u32 {
        self.evidence.phase.get()
    }
    pub fn set_phase(&self, phase: u32) {
        self.evidence.phase.set(phase);
    }
    pub fn events(&self) -> String {
        self.evidence.events.borrow().join("\n")
    }
    pub fn text(&self) -> String {
        self.evidence.text.borrow().clone()
    }
    pub fn ratio(&self) -> f64 {
        self.evidence.ratio.get()
    }
    pub fn dropped(&self) -> bool {
        self.evidence.dropped.get()
    }
    pub fn timers(&self) -> bool {
        self.evidence.timers.get()
    }
    pub fn double_clicked(&self) -> bool {
        self.evidence.double_clicked.get()
    }
    pub fn running(&self) -> bool {
        self.runtime.as_ref().is_some_and(WasmAppHandle::is_running)
    }
    pub fn error(&self) -> Option<String> {
        self.runtime.as_ref().and_then(WasmAppHandle::error)
    }
    pub fn dispose(&self) {
        if let Some(runtime) = &self.runtime {
            runtime.dispose();
        }
    }
    pub fn drop_handle(&mut self) {
        self.runtime = None;
    }
}
