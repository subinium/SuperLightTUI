#![cfg(target_arch = "wasm32")]

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use slt::{CalendarState, Color};
use slt_wasm::{WasmAppHandle, WasmOptions, run_wasm_with_handle, run_wasm_with_options};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::HtmlElement;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen(inline_js = "
export async function frames(n) { while(n--) await new Promise(requestAnimationFrame); }
export function failStyleOnce() {
  const original = Element.prototype.setAttribute;
  Element.prototype.setAttribute = function(name, value) {
    if (this.tagName === 'SPAN' && name === 'style') {
      Element.prototype.setAttribute = original;
      throw new Error('injected DOM write failure');
    }
    return original.call(this, name, value);
  };
}
export function dispatchDuringFrame(host) {
  const rect = host.querySelector('pre').getBoundingClientRect();
  host.dispatchEvent(new PointerEvent('pointerdown', { bubbles:true, clientX:rect.left+2, clientY:rect.top+2, pointerId:7 }));
  window.dispatchEvent(new Event('resize'));
}
export function commitText(host) {
  const sink = host.querySelector('textarea'); host.focus();
  if (document.activeElement !== sink) throw new Error('host focus did not activate input sink');
  sink.dispatchEvent(new InputEvent('input', {data:'a',inputType:'insertText',bubbles:true}));
  sink.dispatchEvent(new CompositionEvent('compositionstart', {bubbles:true}));
  const enter = new KeyboardEvent('keydown', {key:'Enter',isComposing:true,bubbles:true,cancelable:true});
  sink.dispatchEvent(enter);
  if (enter.defaultPrevented) throw new Error('composing Enter was cancelled');
  sink.dispatchEvent(new InputEvent('input', {data:'x',isComposing:true,inputType:'insertCompositionText',bubbles:true}));
  sink.dispatchEvent(new CompositionEvent('compositionend', {data:'b',bubbles:true}));
  sink.dispatchEvent(new InputEvent('input', {data:'b',inputType:'insertText',bubbles:true}));
  sink.dispatchEvent(new InputEvent('beforeinput', {inputType:'deleteContentBackward',bubbles:true,cancelable:true}));
  sink.dispatchEvent(new InputEvent('input', {data:'c',inputType:'insertText',bubbles:true}));
}
export function compose(host, type, text) {
  host.querySelector('textarea').dispatchEvent(new CompositionEvent(type, { data:text, bubbles:true }));
}
export function trailingCompositionInput(host, text) {
  host.querySelector('textarea').dispatchEvent(new InputEvent('input', { data:text, inputType:'insertText', bubbles:true }));
}
export function composingInput(host, text) {
  host.querySelector('textarea').dispatchEvent(new InputEvent('input', { data:text, inputType:'insertCompositionText', isComposing:true, bubbles:true }));
}
")]
extern "C" {
    fn frames(n: u32) -> js_sys::Promise;
    #[wasm_bindgen(js_name = failStyleOnce)]
    fn fail_style_once();
    #[wasm_bindgen(js_name = dispatchDuringFrame)]
    fn dispatch_during_frame(host: &HtmlElement);
    #[wasm_bindgen(js_name = commitText)]
    fn commit_text(host: &HtmlElement);
    fn compose(host: &HtmlElement, event_type: &str, text: &str);
    #[wasm_bindgen(js_name = trailingCompositionInput)]
    fn trailing_composition_input(host: &HtmlElement, text: &str);
    #[wasm_bindgen(js_name = composingInput)]
    fn composing_input(host: &HtmlElement, text: &str);
}

async fn wait_frames() {
    JsFuture::from(frames(6)).await.expect("RAF frames");
}

fn host() -> HtmlElement {
    let document = web_sys::window()
        .expect("window")
        .document()
        .expect("document");
    let host = document
        .create_element("div")
        .expect("create host")
        .dyn_into::<HtmlElement>()
        .expect("host");
    host.set_attribute(
        "style",
        "width:240px;height:128px;padding:8px;border:2px solid black;",
    )
    .expect("style");
    document
        .body()
        .expect("body")
        .append_child(&host)
        .expect("append");
    host
}

fn text(host: &HtmlElement) -> String {
    host.query_selector("pre")
        .expect("query")
        .expect("grid")
        .text_content()
        .expect("text")
}

struct Dropped(Rc<Cell<bool>>);
impl Drop for Dropped {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

#[wasm_bindgen_test(async)]
async fn real_frames_clear_removed_text_styles_wide_cells_and_modal() {
    let host = host();
    let phase = Rc::new(Cell::new(0));
    let current = Rc::clone(&phase);
    let handle = run_wasm_with_handle(host.clone(), 16, 6, move |ui| match current.get() {
        0 => {
            ui.text("ABCDEFGH").bold().fg(Color::Red);
        }
        1 => {
            ui.text("X");
        }
        2 => {
            ui.text("\u{754c}A");
        }
        3 => {
            ui.text("base");
            let _ = ui.modal(|ui| {
                ui.text("MODAL");
            });
        }
        4 => {
            ui.text("base");
        }
        _ => {}
    })
    .expect("mount");
    wait_frames().await;
    assert!(text(&host).contains("ABCDEFGH"));
    phase.set(1);
    wait_frames().await;
    assert!(text(&host).starts_with("X "));
    assert!(!text(&host).contains('B'));
    let style = host
        .query_selector("pre span")
        .expect("query")
        .expect("cell")
        .get_attribute("style")
        .expect("style");
    assert!(!style.contains("bold"));
    assert!(!style.contains("#cd3131"));
    phase.set(2);
    wait_frames().await;
    assert!(text(&host).starts_with("\u{754c}A"));
    phase.set(3);
    wait_frames().await;
    assert!(text(&host).contains("MODAL"));
    phase.set(4);
    wait_frames().await;
    assert!(!text(&host).contains("MODAL"));
    phase.set(5);
    wait_frames().await;
    assert!(text(&host).trim().is_empty());
    assert!(handle.error().is_none());
    handle.dispose();
    wait_frames().await;
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn clocks_quit_and_post_raf_drop_release_the_app() {
    let host = host();
    let count = Rc::new(Cell::new(0));
    let observed = Rc::clone(&count);
    let dropped = Rc::new(Cell::new(false));
    let sentinel = Dropped(Rc::clone(&dropped));
    let mut scheduled = false;
    let mut repeated = false;
    let mut debounced = false;
    let handle = run_wasm_with_handle(host.clone(), 16, 4, move |ui| {
        let _sentinel = &sentinel;
        let _calendar = CalendarState::default();
        let n = observed.get() + 1;
        observed.set(n);
        scheduled |= ui.schedule("schedule", Duration::from_millis(10));
        repeated |= ui.every("every", Duration::from_millis(10)) > 0;
        debounced |= ui.debounce("debounce", Duration::from_millis(10), n == 1);
        ui.text(format!("frame {n}"));
        if scheduled && repeated && debounced && n >= 3 {
            ui.quit();
        }
    })
    .expect("mount");
    wait_frames().await;
    wait_frames().await;
    assert!(count.get() >= 3);
    assert!(!handle.is_running());
    assert!(handle.error().is_none());
    assert!(dropped.get());
    let stopped_at = count.get();
    handle.dispose();
    handle.dispose();
    wait_frames().await;
    assert_eq!(count.get(), stopped_at);
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn synchronous_events_and_dispose_inside_frame_do_not_reborrow_backend() {
    let host = host();
    let container = host.clone();
    let holder = Rc::new(RefCell::new(None::<WasmAppHandle>));
    let owner = Rc::clone(&holder);
    let dropped = Rc::new(Cell::new(false));
    let sentinel = Dropped(Rc::clone(&dropped));
    let mut n = 0;
    let handle = run_wasm_with_handle(host.clone(), 16, 4, move |ui| {
        let _sentinel = &sentinel;
        n += 1;
        ui.text(format!("frame {n}"));
        if n == 3 {
            dispatch_during_frame(&container);
            owner.borrow().as_ref().expect("handle").dispose();
        }
    })
    .expect("mount");
    *holder.borrow_mut() = Some(handle);
    wait_frames().await;
    assert!(!holder.borrow().as_ref().expect("handle").is_running());
    assert!(dropped.get());
    holder.borrow_mut().take();
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn returned_dom_error_stops_and_reports_failure() {
    let host = host();
    let phase = Rc::new(Cell::new(false));
    let current = Rc::clone(&phase);
    let handle = run_wasm_with_handle(host.clone(), 16, 4, move |ui| {
        ui.text(if current.get() { "changed" } else { "initial" });
    })
    .expect("mount");
    wait_frames().await;
    fail_style_once();
    phase.set(true);
    wait_frames().await;
    assert!(!handle.is_running());
    assert!(
        handle
            .error()
            .expect("error")
            .contains("injected DOM write failure")
    );
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn browser_options_apply_on_first_frame_and_fit_host_only_resize() {
    let host = host();
    let sizes = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&sizes);
    let theme = slt::Theme::light();
    let expected_bg = theme.bg;
    let options = WasmOptions {
        width: 4,
        height: 2,
        theme,
        widget_theme: slt::WidgetTheme::new().button(
            slt::WidgetColors::new()
                .fg(Color::Rgb(12, 34, 56))
                .accent(Color::Rgb(12, 34, 56)),
        ),
        scroll_speed: 4,
        auto_fit: true,
        input: false,
        max_fps: None,
        ..WasmOptions::default()
    };
    let handle = run_wasm_with_options(host.clone(), options, move |ui| {
        assert_eq!(ui.theme().bg, expected_bg);
        assert_eq!(ui.scroll_speed(), 4);
        observed.borrow_mut().push((ui.width(), ui.height()));
        let _ = ui.button("stable");
    })
    .expect("mount");
    wait_frames().await;
    let before = *sizes.borrow().last().expect("frame");
    assert!(host.inner_html().contains("color:#0c2238"));
    host.style().set_property("width", "360px").expect("resize");
    wait_frames().await;
    let after = *sizes.borrow().last().expect("frame");
    assert!(after.0 > before.0);
    assert_eq!(after.1, before.1);
    assert!(host.query_selector("textarea").expect("query").is_none());
    drop(handle);
    wait_frames().await;
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn editing_bridge_commits_once_and_keeps_mount_focus_independent() {
    let first = host();
    let second = host();
    let first_text = Rc::new(RefCell::new(String::new()));
    let second_text = Rc::new(RefCell::new(String::new()));
    let mount = |container: HtmlElement, value: Rc<RefCell<String>>| {
        let mut input = slt::TextInputState::new();
        run_wasm_with_handle(container, 16, 4, move |ui| {
            assert!(!ui.events().any(
                |event| matches!(event, slt::Event::Key(key) if key.code == slt::KeyCode::Enter)
            ));
            let _ = ui.text_input(&mut input);
            *value.borrow_mut() = input.value.clone();
        })
        .expect("mount")
    };
    let one = mount(first.clone(), Rc::clone(&first_text));
    let two = mount(second.clone(), Rc::clone(&second_text));
    wait_frames().await;
    commit_text(&first);
    wait_frames().await;
    assert_eq!(&*first_text.borrow(), "ac");
    assert!(second_text.borrow().is_empty());
    commit_text(&second);
    wait_frames().await;
    assert_eq!(&*first_text.borrow(), "ac");
    assert_eq!(&*second_text.borrow(), "ac");
    one.dispose();
    two.dispose();
    wait_frames().await;
    first.remove();
    second.remove();
}

fn assert_overlay_matches_grid(host: &HtmlElement) {
    let grid = host
        .query_selector("pre")
        .expect("grid query")
        .expect("grid")
        .get_bounding_client_rect();
    let input = host
        .query_selector("textarea")
        .expect("input query")
        .expect("input")
        .get_bounding_client_rect();
    for (name, actual, expected) in [
        ("left", input.left(), grid.left()),
        ("top", input.top(), grid.top()),
        ("width", input.width(), grid.width()),
        ("height", input.height(), grid.height()),
    ] {
        assert!(
            (actual - expected).abs() < 0.05,
            "{name}: input {actual}, grid {expected}"
        );
    }
}

#[wasm_bindgen_test(async)]
async fn transformed_host_keeps_input_grid_and_resize_focus_in_one_coordinate_space() {
    let host = host();
    host.style()
        .set_property("transform-origin", "0 0")
        .expect("origin");
    host.style()
        .set_property("transform", "translate(50px, 30px) scale(1.5)")
        .expect("transform");
    let widths = Rc::new(Cell::new(0));
    let observed = Rc::clone(&widths);
    let mut input_state = slt::TextInputState::new();
    let handle = run_wasm_with_options(
        host.clone(),
        WasmOptions {
            auto_fit: true,
            ..WasmOptions::default()
        },
        move |ui| {
            observed.set(ui.width());
            let _ = ui.text_input(&mut input_state);
        },
    )
    .expect("mount");
    wait_frames().await;
    assert_overlay_matches_grid(&host);
    let input = host
        .query_selector("textarea")
        .expect("input query")
        .expect("input");
    host.focus().expect("focus");
    let before_width = widths.get();
    host.style().set_property("width", "360px").expect("resize");
    wait_frames().await;
    assert!(widths.get() > before_width);
    assert_overlay_matches_grid(&host);
    assert_eq!(
        host.query_selector("textarea").expect("input query"),
        Some(input.clone())
    );
    assert_eq!(
        host.owner_document().expect("document").active_element(),
        Some(input)
    );
    handle.dispose();
    wait_frames().await;
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn same_tick_remount_retains_its_tab_stop_and_caller_attributes() {
    for (original, changed) in [
        (None, None),
        (Some("-1"), None),
        (Some("3"), None),
        (None, Some("7")),
    ] {
        let host = host();
        if let Some(value) = original {
            host.set_attribute("tabindex", value)
                .expect("caller tabindex");
        }
        let old = run_wasm_with_handle(host.clone(), 16, 4, |ui| {
            ui.text("old");
        })
        .expect("old mount");
        wait_frames().await;
        if let Some(value) = changed {
            host.set_attribute("tabindex", value)
                .expect("caller changes mounted tabindex");
        }
        let expected = changed.or(original);
        old.dispose();
        assert_eq!(host.get_attribute("tabindex").as_deref(), expected);
        let new = run_wasm_with_handle(host.clone(), 16, 4, |ui| {
            ui.text("new");
        })
        .expect("same tick remount");
        drop(old);
        wait_frames().await;
        assert_eq!(
            host.get_attribute("tabindex").as_deref(),
            expected.or(Some("0"))
        );
        host.focus().expect("focus remounted host");
        assert_eq!(
            host.owner_document().expect("document").active_element(),
            host.query_selector("textarea").expect("input query")
        );
        new.dispose();
        wait_frames().await;
        assert_eq!(host.get_attribute("tabindex").as_deref(), expected);
        host.remove();
    }
}

#[wasm_bindgen_test(async)]
async fn preedit_is_visible_without_editing_then_commits_once_or_cancels() {
    let host = host();
    let value = Rc::new(RefCell::new(String::new()));
    let observed = Rc::clone(&value);
    let mut input = slt::TextInputState::new();
    let handle = run_wasm_with_options(
        host.clone(),
        WasmOptions {
            width: 16,
            height: 4,
            widget_theme: slt::WidgetTheme::new()
                .text_input(slt::WidgetColors::new().fg(Color::Rgb(12, 34, 56))),
            ..WasmOptions::default()
        },
        move |ui| {
            let _ = ui.container().bg(Color::Rgb(200, 210, 220)).col(|ui| {
                let _ = ui.text_input(&mut input);
            });
            *observed.borrow_mut() = input.value.clone();
        },
    )
    .expect("mount");
    wait_frames().await;
    host.focus().expect("focus");
    compose(&host, "compositionstart", "");
    for stage in ["\u{314e}", "\u{d558}", "\u{d55c}"] {
        compose(&host, "compositionupdate", stage);
        wait_frames().await;
        assert!(
            value.borrow().is_empty(),
            "preedit must not mutate application state"
        );
        let preview = host
            .query_selector("[data-slt-preedit]")
            .expect("query")
            .expect("visible preedit overlay");
        assert_eq!(preview.text_content().as_deref(), Some(stage));
        assert!(preview.get_bounding_client_rect().width() > 0.0);
        let glyph = preview
            .query_selector("span")
            .expect("query")
            .expect("preedit glyph");
        let style = glyph.get_attribute("style").expect("style");
        assert!(style.contains("underline"));
        assert!(style.contains("color:#0c2238"), "foreground: {style}");
        assert!(
            style.contains("background-color:#c8d2dc"),
            "background: {style}"
        );
    }
    compose(&host, "compositionend", "\u{d55c}");
    trailing_composition_input(&host, "\u{d55c}");
    wait_frames().await;
    assert_eq!(&*value.borrow(), "\u{d55c}");
    compose(&host, "compositionstart", "");
    compose(&host, "compositionupdate", "discard");
    compose(&host, "compositionend", "");
    wait_frames().await;
    assert_eq!(&*value.borrow(), "\u{d55c}");
    let preview = host
        .query_selector("[data-slt-preedit]")
        .expect("query")
        .expect("overlay");
    assert_eq!(preview.text_content().as_deref(), Some(""));
    assert_eq!(preview.get_bounding_client_rect().height(), 0.0);
    composing_input(&host, "\u{d558}");
    wait_frames().await;
    assert_eq!(preview.text_content().as_deref(), Some("\u{d558}"));
    compose(&host, "compositionend", "");
    handle.dispose();
    wait_frames().await;
    assert!(
        host.query_selector("[data-slt-preedit]")
            .expect("query")
            .is_none()
    );
    assert!(host.query_selector("textarea").expect("query").is_none());
    host.remove();
}

#[wasm_bindgen_test(async)]
async fn masked_input_never_exposes_raw_preedit_even_when_masking_changes_mid_composition() {
    let host = host();
    let masked = Rc::new(Cell::new(true));
    let mask = Rc::clone(&masked);
    let value = Rc::new(RefCell::new(String::new()));
    let observed = Rc::clone(&value);
    let mut input = slt::TextInputState::new();
    let handle = run_wasm_with_handle(host.clone(), 16, 4, move |ui| {
        input.masked = mask.get();
        let _ = ui.text_input(&mut input);
        *observed.borrow_mut() = input.value.clone();
    })
    .expect("masked mount");
    wait_frames().await;
    host.focus().expect("focus");
    let secret = "\u{c554}\u{d638}";
    compose(&host, "compositionstart", "");
    compose(&host, "compositionupdate", secret);
    wait_frames().await;
    assert!(value.borrow().is_empty());
    assert!(
        !text(&host).contains(secret),
        "masked preedit exposed raw text in the DOM"
    );
    let preview = host
        .query_selector("[data-slt-preedit]")
        .expect("query")
        .expect("preedit");
    assert_eq!(preview.text_content().as_deref(), Some(""));
    assert_eq!(preview.get_bounding_client_rect().height(), 0.0);
    compose(&host, "compositionend", secret);
    trailing_composition_input(&host, secret);
    wait_frames().await;
    assert_eq!(&*value.borrow(), secret);
    assert!(!text(&host).contains(secret));
    assert!(text(&host).contains('\u{2022}'));

    let second_secret = "\u{be44}\u{bc00}";
    composing_input(&host, second_secret);
    masked.set(false);
    wait_frames().await;
    compose(&host, "compositionupdate", second_secret);
    wait_frames().await;
    assert!(
        !text(&host).contains(second_secret),
        "a masked composition must not become a plaintext preview"
    );
    compose(&host, "compositionend", "");

    compose(&host, "compositionstart", "");
    compose(&host, "compositionupdate", "\u{d55c}");
    wait_frames().await;
    assert_eq!(preview.text_content().as_deref(), Some("\u{d55c}"));
    masked.set(true);
    wait_frames().await;
    assert_eq!(preview.text_content().as_deref(), Some(""));
    assert!(!text(&host).contains('\u{d55c}'));
    masked.set(false);
    wait_frames().await;
    compose(&host, "compositionupdate", "\u{d558}");
    wait_frames().await;
    assert_eq!(preview.text_content().as_deref(), Some(""));
    compose(&host, "compositionend", "");
    wait_frames().await;
    assert_eq!(&*value.borrow(), secret);
    compose(&host, "compositionstart", "");
    masked.set(true);
    compose(&host, "compositionupdate", "leak");
    assert!(
        !text(&host).contains("leak"),
        "pending privacy changes must be checked before painting preedit"
    );
    wait_frames().await;
    assert!(!text(&host).contains("leak"));
    compose(&host, "compositionend", "");
    handle.dispose();
    wait_frames().await;
    host.remove();
}
