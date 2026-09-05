use slt::{CalendarState, TextInputState};
use slt_wasm::{WasmAppHandle, WasmOptions, run_wasm_with_options};
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

#[cfg(feature = "browser-tests")]
mod probe;

/// The embedding page owns this handle and disposes it when unmounting.
#[wasm_bindgen]
pub fn mount(container: HtmlElement) -> Result<WasmAppHandle, JsValue> {
    let mut name = TextInputState::with_placeholder("Name");
    let mut count = 0;
    let mut calendar = CalendarState::default();
    run_wasm_with_options(
        container,
        WasmOptions {
            width: 64,
            height: 22,
            ..WasmOptions::default()
        },
        move |ui| {
            ui.text("SuperLightTUI Browser").bold();
            let _ = ui.text_input(&mut name);
            if ui.button("Increment").clicked {
                count += 1;
            }
            ui.text(format!("Count: {count}  Frame: {}", ui.tick()));
            let _ = ui.calendar(&mut calendar);
            if ui.button("Stop").clicked {
                ui.quit();
            }
        },
    )
}
