//! WASM/browser backend for SuperLightTUI.
//!
//! Renders an SLT [`Context`] into a grid of `<span>` elements inside a host
//! container and drives it from `requestAnimationFrame`, translating DOM
//! keyboard/mouse/wheel/resize/paste events into SLT [`Event`]s.

// Mirror the library-only hygiene lints kept out of [workspace.lints]; this
// crate has no example targets so they apply cleanly to its single lib.
#![warn(missing_docs)]
#![warn(unreachable_pub)]
#![deny(clippy::unwrap_in_result)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::dbg_macro)]
#![warn(clippy::print_stdout)]
#![warn(clippy::print_stderr)]
#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::io;
use std::rc::{Rc, Weak};

use slt::{
    AppState, Backend, Buffer, Color, Context, Event, KeyCode, KeyModifiers, Modifiers,
    MouseButton, MouseEvent as SltMouseEvent, MouseKind, Rect, RunConfig,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{
    Document, EventTarget, HtmlElement, HtmlPreElement, KeyboardEvent, MouseEvent, Window,
};

type RafCallback = Closure<dyn FnMut(f64)>;

thread_local! {
    static ACTIVE_APPS: RefCell<Vec<WasmAppHandle>> = const { RefCell::new(Vec::new()) };
}

struct EventListener {
    target: EventTarget,
    event_type: &'static str,
    callback: Closure<dyn FnMut(web_sys::Event)>,
}

impl EventListener {
    fn install(
        target: EventTarget,
        event_type: &'static str,
        callback: Closure<dyn FnMut(web_sys::Event)>,
    ) -> Result<Self, JsValue> {
        target.add_event_listener_with_callback(event_type, callback.as_ref().unchecked_ref())?;
        Ok(Self {
            target,
            event_type,
            callback,
        })
    }
}

impl Drop for EventListener {
    fn drop(&mut self) {
        let _ = self.target.remove_event_listener_with_callback(
            self.event_type,
            self.callback.as_ref().unchecked_ref(),
        );
    }
}

struct WasmAppInner {
    window: Window,
    listeners: Vec<EventListener>,
    events: Rc<RefCell<Vec<Event>>>,
    raf: Option<RafCallback>,
    raf_id: Option<i32>,
    running: bool,
}

impl WasmAppInner {
    fn new(window: Window, listeners: Vec<EventListener>, events: Rc<RefCell<Vec<Event>>>) -> Self {
        Self {
            window,
            listeners,
            events,
            raf: None,
            raf_id: None,
            running: true,
        }
    }
}

/// Owned browser runtime handle returned by [`run_wasm_with_handle`].
///
/// Dropping or explicitly disposing the handle removes DOM event listeners,
/// cancels any pending `requestAnimationFrame`, and releases the app closure.
/// The compatibility [`run_wasm`] API stores this handle internally until the
/// app exits so existing fire-and-forget callers continue to work.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct WasmAppHandle {
    inner: Rc<RefCell<WasmAppInner>>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl WasmAppHandle {
    /// Stop the app and release all browser resources owned by this handle.
    pub fn dispose(&self) {
        dispose_inner_now(&self.inner);
    }

    /// Return whether the app is still scheduled to render future frames.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.inner.borrow().running
    }
}

impl WasmAppHandle {
    fn new(inner: Rc<RefCell<WasmAppInner>>) -> Self {
        Self { inner }
    }
}

impl Drop for WasmAppHandle {
    fn drop(&mut self) {
        dispose_inner_now(&self.inner);
    }
}

/// SLT [`Backend`] that paints into a DOM `<pre>`/`<span>` grid and diffs
/// against the previously flushed frame so only changed cells are rewritten.
pub struct DomBackend {
    buffer: Buffer,
    /// Snapshot of the buffer as it was last flushed to the DOM. Used to diff
    /// against the live `buffer` so `flush` only mutates spans whose cell
    /// actually changed — mirroring the native ANSI diff in `src/buffer.rs`.
    prev: Buffer,
    container: HtmlElement,
    cells: Vec<HtmlElement>,
    initialized: bool,
    width: u32,
    height: u32,
}

impl DomBackend {
    /// Create a backend that renders a `width`×`height` cell grid into
    /// `container`. The DOM grid is built lazily on the first flush.
    pub fn new(container: HtmlElement, width: u32, height: u32) -> Self {
        Self {
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
            prev: Buffer::empty(Rect::new(0, 0, width, height)),
            container,
            cells: Vec::new(),
            initialized: false,
            width,
            height,
        }
    }

    /// Resize the backend to a new cell grid, discarding the existing DOM grid.
    ///
    /// The next [`flush`](DomBackend::flush) rebuilds the `<span>` grid and
    /// repaints every cell. Both the live and previous buffers are resized so
    /// the diff baseline stays consistent with the new dimensions. A no-op when
    /// the dimensions are unchanged or either axis is zero.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 || (width == self.width && height == self.height) {
            return;
        }
        self.width = width;
        self.height = height;
        let area = Rect::new(0, 0, width, height);
        self.buffer.resize(area);
        self.prev.resize(area);
        // Force a full DOM rebuild + repaint on the next flush.
        self.initialized = false;
    }

    fn document(&self) -> Result<Document, io::Error> {
        self.container
            .owner_document()
            .ok_or_else(|| io::Error::other("owner document unavailable"))
    }

    fn initialize_grid(&mut self) -> io::Result<()> {
        self.container.set_inner_html("");

        let document = self.document()?;
        let pre = document
            .create_element("pre")
            .map_err(|e| io::Error::other(format!("create pre failed: {e:?}")))?
            .dyn_into::<HtmlPreElement>()
            .map_err(|_| io::Error::other("failed to cast pre element"))?;

        pre.set_attribute(
            "style",
            "margin:0;padding:0;line-height:1;font-family:monospace;font-size:14px;white-space:pre;",
        )
        .map_err(|e| io::Error::other(format!("set pre style failed: {e:?}")))?;

        self.cells.clear();
        for y in 0..self.height {
            for _x in 0..self.width {
                let span = document
                    .create_element("span")
                    .map_err(|e| io::Error::other(format!("create span failed: {e:?}")))?
                    .dyn_into::<HtmlElement>()
                    .map_err(|_| io::Error::other("failed to cast span element"))?;
                span.set_text_content(Some(" "));
                pre.append_child(&span)
                    .map_err(|e| io::Error::other(format!("append span failed: {e:?}")))?;
                self.cells.push(span);
            }

            if y + 1 < self.height {
                let newline = document.create_text_node("\n");
                pre.append_child(&newline)
                    .map_err(|e| io::Error::other(format!("append newline failed: {e:?}")))?;
            }
        }

        self.container
            .append_child(&pre)
            .map_err(|e| io::Error::other(format!("append pre failed: {e:?}")))?;
        self.initialized = true;
        Ok(())
    }

    /// Measure the rendered pixel size of a single cell `<span>`.
    ///
    /// Returns `None` when the grid has not been built yet or the first span
    /// reports a zero-area box (e.g. the container is `display:none`). Used to
    /// translate a container pixel resize into a new cell grid.
    fn cell_pixel_size(&self) -> Option<(f64, f64)> {
        let span = self.cells.first()?;
        let rect = web_sys::Element::from(span.clone()).get_bounding_client_rect();
        let (w, h) = (rect.width(), rect.height());
        if w > 0.0 && h > 0.0 {
            Some((w, h))
        } else {
            None
        }
    }

    /// Compute the cell grid `(cols, rows)` that fits the container's current
    /// pixel size, given a measured per-cell pixel size.
    ///
    /// Returns `None` when either the container or a cell reports a zero size so
    /// the caller can keep the existing dimensions instead of collapsing to a
    /// degenerate grid.
    fn fit_grid_to_container(&self) -> Option<(u32, u32)> {
        let (cell_w, cell_h) = self.cell_pixel_size()?;
        let rect = web_sys::Element::from(self.container.clone()).get_bounding_client_rect();
        let (cont_w, cont_h) = (rect.width(), rect.height());
        if cont_w <= 0.0 || cont_h <= 0.0 {
            return None;
        }
        let cols = (cont_w / cell_w).floor().max(1.0) as u32;
        let rows = (cont_h / cell_h).floor().max(1.0) as u32;
        Some((cols, rows))
    }
}

impl Backend for DomBackend {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    fn flush(&mut self) -> io::Result<()> {
        // When the grid is (re)built every span starts blank, so the previous
        // snapshot must be treated as empty to force a full repaint. Otherwise
        // we diff against the cells we actually painted last frame and skip the
        // unchanged ones — the common steady-state case writes almost nothing.
        let full_repaint = !self.initialized;
        if full_repaint {
            self.initialize_grid()?;
        }

        let ox = self.buffer.area.x;
        let oy = self.buffer.area.y;
        let pox = self.prev.area.x;
        let poy = self.prev.area.y;
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                let cell = self.buffer.get(ox + x, oy + y);

                // `Cell` derives `PartialEq` over (symbol, style, hyperlink), so
                // an equality check captures every visible difference. After a
                // grid rebuild `full_repaint` is set and we always write.
                if !full_repaint && cell == self.prev.get(pox + x, poy + y) {
                    continue;
                }

                let span = self
                    .cells
                    .get(idx)
                    .ok_or_else(|| io::Error::other("dom cell index out of bounds"))?;

                span.set_attribute("style", &style_to_css(cell.style))
                    .map_err(|e| io::Error::other(format!("set span style failed: {e:?}")))?;
                let symbol = if cell.symbol.is_empty() {
                    " "
                } else {
                    cell.symbol.as_str()
                };
                span.set_text_content(Some(symbol));
            }
        }

        // Record what is now on screen so the next frame can diff against it.
        self.prev.resize(self.buffer.area);
        self.prev.content.clone_from(&self.buffer.content);

        Ok(())
    }
}

fn style_to_css(style: slt::Style) -> String {
    let mut css = String::new();

    if let Some(fg) = color_to_css(style.fg) {
        css.push_str("color:");
        css.push_str(&fg);
        css.push(';');
    }
    if let Some(bg) = color_to_css(style.bg) {
        css.push_str("background-color:");
        css.push_str(&bg);
        css.push(';');
    }

    if style.modifiers.contains(Modifiers::BOLD) {
        css.push_str("font-weight:bold;");
    }
    if style.modifiers.contains(Modifiers::DIM) {
        css.push_str("opacity:0.7;");
    }
    if style.modifiers.contains(Modifiers::ITALIC) {
        css.push_str("font-style:italic;");
    }
    let mut text_decorations = Vec::new();
    if style.modifiers.contains(Modifiers::UNDERLINE) {
        text_decorations.push("underline");
    }
    if style.modifiers.contains(Modifiers::STRIKETHROUGH) {
        text_decorations.push("line-through");
    }
    if !text_decorations.is_empty() {
        css.push_str("text-decoration-line:");
        css.push_str(&text_decorations.join(" "));
        css.push(';');
    }
    if style.modifiers.contains(Modifiers::REVERSED) {
        css.push_str("filter:invert(100%);");
    }

    css
}

fn color_to_css(color: Option<Color>) -> Option<String> {
    match color? {
        Color::Reset => None,
        Color::Black => Some("#000000".to_string()),
        Color::Red => Some("#cd3131".to_string()),
        Color::Green => Some("#0dbc79".to_string()),
        Color::Yellow => Some("#e5e510".to_string()),
        Color::Blue => Some("#2472c8".to_string()),
        Color::Magenta => Some("#bc3fbc".to_string()),
        Color::Cyan => Some("#11a8cd".to_string()),
        Color::White => Some("#e5e5e5".to_string()),
        Color::DarkGray => Some("#808080".to_string()),
        Color::LightRed => Some("#ff0000".to_string()),
        Color::LightGreen => Some("#00ff00".to_string()),
        Color::LightYellow => Some("#ffff00".to_string()),
        Color::LightBlue => Some("#0000ff".to_string()),
        Color::LightMagenta => Some("#ff00ff".to_string()),
        Color::LightCyan => Some("#00ffff".to_string()),
        Color::LightWhite => Some("#ffffff".to_string()),
        Color::Rgb(r, g, b) => Some(format!("#{r:02x}{g:02x}{b:02x}")),
        Color::Indexed(i) => {
            let (r, g, b) = indexed_to_rgb(i);
            Some(format!("#{r:02x}{g:02x}{b:02x}"))
        }
        _ => None,
    }
}

fn indexed_to_rgb(i: u8) -> (u8, u8, u8) {
    if i < 16 {
        return match i {
            0 => (0, 0, 0),
            1 => (128, 0, 0),
            2 => (0, 128, 0),
            3 => (128, 128, 0),
            4 => (0, 0, 128),
            5 => (128, 0, 128),
            6 => (0, 128, 128),
            7 => (192, 192, 192),
            8 => (128, 128, 128),
            9 => (255, 0, 0),
            10 => (0, 255, 0),
            11 => (255, 255, 0),
            12 => (0, 0, 255),
            13 => (255, 0, 255),
            14 => (0, 255, 255),
            _ => (255, 255, 255),
        };
    }

    if (16..=231).contains(&i) {
        let idx = i - 16;
        let r = idx / 36;
        let g = (idx % 36) / 6;
        let b = idx % 6;
        let comp = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
        return (comp(r), comp(g), comp(b));
    }

    let gray = 8 + (i - 232) * 10;
    (gray, gray, gray)
}

fn modifiers_from_bools(shift: bool, control: bool, alt: bool, super_key: bool) -> KeyModifiers {
    let mut modifiers = KeyModifiers::NONE;
    if shift {
        modifiers.0 |= KeyModifiers::SHIFT.0;
    }
    if control {
        modifiers.0 |= KeyModifiers::CONTROL.0;
    }
    if alt {
        modifiers.0 |= KeyModifiers::ALT.0;
    }
    if super_key {
        modifiers.0 |= KeyModifiers::SUPER.0;
    }
    modifiers
}

fn keyboard_event_modifiers(event: &KeyboardEvent) -> KeyModifiers {
    modifiers_from_bools(
        event.shift_key(),
        event.ctrl_key(),
        event.alt_key(),
        event.meta_key(),
    )
}

fn mouse_event_modifiers(event: &MouseEvent) -> KeyModifiers {
    modifiers_from_bools(
        event.shift_key(),
        event.ctrl_key(),
        event.alt_key(),
        event.meta_key(),
    )
}

fn reflect_bool(target: &JsValue, key: &str) -> bool {
    js_sys::Reflect::get(target, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn reflect_event_modifiers(target: &JsValue) -> KeyModifiers {
    modifiers_from_bools(
        reflect_bool(target, "shiftKey"),
        reflect_bool(target, "ctrlKey"),
        reflect_bool(target, "altKey"),
        reflect_bool(target, "metaKey"),
    )
}

fn keyboard_event_to_slt(event: &KeyboardEvent) -> Option<Event> {
    let key = event.key();
    let code = match key.as_str() {
        "Enter" => KeyCode::Enter,
        "Escape" => KeyCode::Esc,
        "Tab" => {
            if event.shift_key() {
                KeyCode::BackTab
            } else {
                KeyCode::Tab
            }
        }
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "ArrowUp" => KeyCode::Up,
        "ArrowDown" => KeyCode::Down,
        "ArrowLeft" => KeyCode::Left,
        "ArrowRight" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        s => {
            if s.chars().count() == 1 {
                KeyCode::Char(s.chars().next()?)
            } else {
                return None;
            }
        }
    };

    Some(Event::key_mod(code, keyboard_event_modifiers(event)))
}

fn mouse_button(button: i16) -> MouseButton {
    match button {
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        _ => MouseButton::Left,
    }
}

fn mouse_pixel_position(event: &MouseEvent) -> (Option<u16>, Option<u16>) {
    let px = u16::try_from(event.offset_x()).ok();
    let py = u16::try_from(event.offset_y()).ok();
    (px, py)
}

fn mouse_cell_position(
    event: &MouseEvent,
    container: &HtmlElement,
    width: u32,
    height: u32,
) -> Option<(u32, u32)> {
    let rect = web_sys::Element::from(container.clone()).get_bounding_client_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return None;
    }

    let rel_x = event.client_x() as f64 - rect.left();
    let rel_y = event.client_y() as f64 - rect.top();
    if rel_x < 0.0 || rel_y < 0.0 {
        return None;
    }

    let cell_w = rect.width() / width.max(1) as f64;
    let cell_h = rect.height() / height.max(1) as f64;
    if cell_w <= 0.0 || cell_h <= 0.0 {
        return None;
    }

    let x = (rel_x / cell_w).floor() as u32;
    let y = (rel_y / cell_h).floor() as u32;
    Some((
        x.min(width.saturating_sub(1)),
        y.min(height.saturating_sub(1)),
    ))
}

/// Read a numeric property off a JS value via reflection.
///
/// `web-sys` typed wrappers for `WheelEvent` / `ClipboardEvent` are not enabled
/// in this crate's feature set, so the wheel and paste listeners receive the
/// generic [`web_sys::Event`] and pull the fields they need by name. Returns
/// `None` when the property is missing or not a number.
fn reflect_f64(target: &JsValue, key: &str) -> Option<f64> {
    js_sys::Reflect::get(target, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_f64())
}

/// Translate a `wheel` event's scroll deltas into an SLT [`MouseKind`].
///
/// The dominant axis wins so a mostly-vertical gesture maps to
/// `ScrollUp`/`ScrollDown` and a mostly-horizontal one to
/// `ScrollLeft`/`ScrollRight`, mirroring how crossterm reports terminal wheel
/// events. Returns `None` for a zero-delta event so we never enqueue a no-op.
fn wheel_delta_to_kind(delta_x: f64, delta_y: f64) -> Option<MouseKind> {
    if delta_y.abs() >= delta_x.abs() {
        if delta_y < 0.0 {
            Some(MouseKind::ScrollUp)
        } else if delta_y > 0.0 {
            Some(MouseKind::ScrollDown)
        } else {
            None
        }
    } else if delta_x < 0.0 {
        Some(MouseKind::ScrollLeft)
    } else {
        Some(MouseKind::ScrollRight)
    }
}

/// Compute the cell `(x, y)` under a pointer event from its `clientX`/`clientY`,
/// read reflectively so this works for the generic `wheel` event too.
fn reflect_cell_position(
    target: &JsValue,
    container: &HtmlElement,
    width: u32,
    height: u32,
) -> Option<(u32, u32)> {
    let client_x = reflect_f64(target, "clientX")?;
    let client_y = reflect_f64(target, "clientY")?;
    let rect = web_sys::Element::from(container.clone()).get_bounding_client_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    let rel_x = client_x - rect.left();
    let rel_y = client_y - rect.top();
    if rel_x < 0.0 || rel_y < 0.0 {
        return None;
    }
    let cell_w = rect.width() / width.max(1) as f64;
    let cell_h = rect.height() / height.max(1) as f64;
    if cell_w <= 0.0 || cell_h <= 0.0 {
        return None;
    }
    let x = (rel_x / cell_w).floor() as u32;
    let y = (rel_y / cell_h).floor() as u32;
    Some((
        x.min(width.saturating_sub(1)),
        y.min(height.saturating_sub(1)),
    ))
}

/// Extract pasted text from a `paste` event's `clipboardData`.
///
/// `clipboardData.getData("text")` returns the plain-text flavor of the
/// clipboard. Read reflectively because the `ClipboardEvent` / `DataTransfer`
/// web-sys wrappers are not enabled. Returns `None` when there is no text
/// payload so we never enqueue an empty paste.
fn paste_event_text(target: &JsValue) -> Option<String> {
    let clipboard_data = js_sys::Reflect::get(target, &JsValue::from_str("clipboardData")).ok()?;
    if clipboard_data.is_null() || clipboard_data.is_undefined() {
        return None;
    }
    let get_data = js_sys::Reflect::get(&clipboard_data, &JsValue::from_str("getData")).ok()?;
    let get_data: js_sys::Function = get_data.dyn_into().ok()?;
    let text = get_data
        .call1(&clipboard_data, &JsValue::from_str("text"))
        .ok()?
        .as_string()?;
    if text.is_empty() { None } else { Some(text) }
}

fn event_target<T: JsCast>(target: &T) -> EventTarget {
    target.unchecked_ref::<EventTarget>().clone()
}

fn dispose_inner_now(inner: &Rc<RefCell<WasmAppInner>>) {
    let mut app = inner.borrow_mut();
    app.running = false;
    if let Some(raf_id) = app.raf_id.take() {
        let _ = app.window.cancel_animation_frame(raf_id);
    }
    app.listeners.clear();
    app.events.borrow_mut().clear();
    app.raf = None;
}

fn schedule_raf(inner: &Rc<RefCell<WasmAppInner>>) -> Result<(), JsValue> {
    let raf_id = {
        let app = inner.borrow();
        if !app.running {
            return Ok(());
        }
        let Some(raf) = app.raf.as_ref() else {
            return Err(JsValue::from_str(
                "failed to initialize requestAnimationFrame loop",
            ));
        };
        app.window
            .request_animation_frame(raf.as_ref().unchecked_ref())?
    };
    inner.borrow_mut().raf_id = Some(raf_id);
    Ok(())
}

fn schedule_deferred_dispose(window: &Window, inner: Rc<RefCell<WasmAppInner>>) {
    let cleanup = Closure::once_into_js(move || {
        dispose_inner_now(&inner);
        ACTIVE_APPS.with(|apps| {
            apps.borrow_mut().retain(WasmAppHandle::is_running);
        });
    });
    let _ =
        window.set_timeout_with_callback_and_timeout_and_arguments_0(cleanup.unchecked_ref(), 0);
}

fn stop_after_current_frame(inner: Rc<RefCell<WasmAppInner>>) {
    let window = {
        let mut app = inner.borrow_mut();
        if !app.running && app.raf.is_none() {
            return;
        }
        app.running = false;
        if let Some(raf_id) = app.raf_id.take() {
            let _ = app.window.cancel_animation_frame(raf_id);
        }
        app.listeners.clear();
        app.events.borrow_mut().clear();
        app.window.clone()
    };
    schedule_deferred_dispose(&window, inner);
}

fn install_event_listeners(
    container: &HtmlElement,
    window: &Window,
    backend: Rc<RefCell<DomBackend>>,
    events: Rc<RefCell<Vec<Event>>>,
) -> Result<Vec<EventListener>, JsValue> {
    container.set_tab_index(0);
    let mut listeners = Vec::new();

    // The grid can be re-sized by the `resize` listener, so every
    // position-dependent listener reads the live `(width, height)` from the
    // backend at dispatch time rather than capturing the initial dimensions.
    // JS callbacks are non-reentrant, so a short-lived `borrow()` here can never
    // overlap the `borrow_mut()` in the RAF loop or the `resize` listener.

    let key_events = Rc::clone(&events);
    let keydown = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let Some(event) = event.dyn_ref::<KeyboardEvent>() else {
            return;
        };
        if let Some(slt_event) = keyboard_event_to_slt(event) {
            key_events.borrow_mut().push(slt_event);
            event.prevent_default();
        }
    }) as Box<dyn FnMut(_)>);
    listeners.push(EventListener::install(
        event_target(container),
        "keydown",
        keydown,
    )?);

    let move_events = Rc::clone(&events);
    let container_move = container.clone();
    let move_backend = Rc::clone(&backend);
    let mousemove = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let Some(event) = event.dyn_ref::<MouseEvent>() else {
            return;
        };
        let (width, height) = move_backend.borrow().size();
        if let Some((x, y)) = mouse_cell_position(event, &container_move, width, height) {
            let (pixel_x, pixel_y) = mouse_pixel_position(event);
            move_events
                .borrow_mut()
                .push(Event::Mouse(SltMouseEvent::new(
                    MouseKind::Moved,
                    x,
                    y,
                    mouse_event_modifiers(event),
                    pixel_x,
                    pixel_y,
                )));
        }
    }) as Box<dyn FnMut(_)>);
    listeners.push(EventListener::install(
        event_target(container),
        "mousemove",
        mousemove,
    )?);

    let down_events = Rc::clone(&events);
    let container_down = container.clone();
    let down_backend = Rc::clone(&backend);
    let mousedown = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let Some(event) = event.dyn_ref::<MouseEvent>() else {
            return;
        };
        let (width, height) = down_backend.borrow().size();
        if let Some((x, y)) = mouse_cell_position(event, &container_down, width, height) {
            let (pixel_x, pixel_y) = mouse_pixel_position(event);
            down_events
                .borrow_mut()
                .push(Event::Mouse(SltMouseEvent::new(
                    MouseKind::Down(mouse_button(event.button())),
                    x,
                    y,
                    mouse_event_modifiers(event),
                    pixel_x,
                    pixel_y,
                )));
        }
    }) as Box<dyn FnMut(_)>);
    listeners.push(EventListener::install(
        event_target(container),
        "mousedown",
        mousedown,
    )?);

    let up_events = Rc::clone(&events);
    let container_up = container.clone();
    let up_backend = Rc::clone(&backend);
    let mouseup = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let Some(event) = event.dyn_ref::<MouseEvent>() else {
            return;
        };
        let (width, height) = up_backend.borrow().size();
        if let Some((x, y)) = mouse_cell_position(event, &container_up, width, height) {
            let (pixel_x, pixel_y) = mouse_pixel_position(event);
            up_events.borrow_mut().push(Event::Mouse(SltMouseEvent::new(
                MouseKind::Up(mouse_button(event.button())),
                x,
                y,
                mouse_event_modifiers(event),
                pixel_x,
                pixel_y,
            )));
        }
    }) as Box<dyn FnMut(_)>);
    listeners.push(EventListener::install(
        event_target(container),
        "mouseup",
        mouseup,
    )?);

    // Mouse wheel -> ScrollUp/Down/Left/Right at the cell under the cursor.
    // The `WheelEvent` web-sys wrapper is not enabled, so the deltas and the
    // pointer position are read reflectively off the generic event.
    let wheel_events = Rc::clone(&events);
    let container_wheel = container.clone();
    let wheel_backend = Rc::clone(&backend);
    let wheel = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let value: &JsValue = event.as_ref();
        let delta_x = reflect_f64(value, "deltaX").unwrap_or(0.0);
        let delta_y = reflect_f64(value, "deltaY").unwrap_or(0.0);
        let Some(kind) = wheel_delta_to_kind(delta_x, delta_y) else {
            return;
        };
        let (width, height) = wheel_backend.borrow().size();
        let (x, y) =
            reflect_cell_position(value, &container_wheel, width, height).unwrap_or((0, 0));
        wheel_events
            .borrow_mut()
            .push(Event::Mouse(SltMouseEvent::new(
                kind,
                x,
                y,
                reflect_event_modifiers(value),
                None,
                None,
            )));
        event.prevent_default();
    }) as Box<dyn FnMut(_)>);
    listeners.push(EventListener::install(
        event_target(container),
        "wheel",
        wheel,
    )?);

    // Window resize -> recompute the cell grid from the container's pixel size
    // and emit a `Resize` event. The backend re-sizes its buffer (and rebuilds
    // the DOM grid on the next flush) so the core run loop lays out against the
    // new dimensions, mirroring crossterm's `Resize`.
    let resize_events = Rc::clone(&events);
    let resize_backend = Rc::clone(&backend);
    let resize = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        let mut backend = resize_backend.borrow_mut();
        let Some((cols, rows)) = backend.fit_grid_to_container() else {
            return;
        };
        if (cols, rows) == backend.size() {
            return;
        }
        backend.resize(cols, rows);
        resize_events.borrow_mut().push(Event::Resize(cols, rows));
    }) as Box<dyn FnMut(_)>);
    listeners.push(EventListener::install(
        event_target(window),
        "resize",
        resize,
    )?);

    // Focus blur -> FocusLost / FocusGained so widgets can clear hover state,
    // matching crossterm's focus events.
    let blur_events = Rc::clone(&events);
    let blur = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        blur_events.borrow_mut().push(Event::FocusLost);
    }) as Box<dyn FnMut(_)>);
    listeners.push(EventListener::install(
        event_target(container),
        "blur",
        blur,
    )?);

    let focus_events = Rc::clone(&events);
    let focus = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        focus_events.borrow_mut().push(Event::FocusGained);
    }) as Box<dyn FnMut(_)>);
    listeners.push(EventListener::install(
        event_target(container),
        "focus",
        focus,
    )?);

    // Clipboard paste -> Paste(text). `ClipboardEvent`/`DataTransfer` wrappers
    // are not enabled, so the text flavor is pulled reflectively from
    // `clipboardData.getData("text")`.
    let paste_events = Rc::clone(&events);
    let paste = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if let Some(text) = paste_event_text(event.as_ref()) {
            paste_events.borrow_mut().push(Event::Paste(text));
            event.prevent_default();
        }
    }) as Box<dyn FnMut(_)>);
    listeners.push(EventListener::install(
        event_target(container),
        "paste",
        paste,
    )?);

    Ok(listeners)
}

/// Mount `app` into `container` as a `width`×`height` cell grid and return an
/// owned handle for the browser runtime.
///
/// Installs the DOM event listeners (keyboard, mouse, wheel, resize, focus,
/// paste) that feed SLT [`Event`]s into the run loop. The returned
/// [`WasmAppHandle`] owns those listeners and the `requestAnimationFrame`
/// callback; drop it or call [`WasmAppHandle::dispose`] to stop the app.
///
/// # Errors
///
/// Returns a [`JsValue`] error when the `window` is unavailable, a listener
/// fails to install, or the initial frame cannot be scheduled.
pub fn run_wasm_with_handle<F>(
    container: HtmlElement,
    width: u32,
    height: u32,
    app: F,
) -> Result<WasmAppHandle, JsValue>
where
    F: FnMut(&mut Context) + 'static,
{
    let window: Window =
        web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    let backend = Rc::new(RefCell::new(DomBackend::new(
        container.clone(),
        width,
        height,
    )));
    let state = Rc::new(RefCell::new(AppState::new()));
    let config = RunConfig::default();
    let events = Rc::new(RefCell::new(Vec::<Event>::new()));
    let app = Rc::new(RefCell::new(app));

    let listeners =
        install_event_listeners(&container, &window, Rc::clone(&backend), Rc::clone(&events))?;
    let inner = Rc::new(RefCell::new(WasmAppInner::new(
        window.clone(),
        listeners,
        Rc::clone(&events),
    )));

    let inner_weak: Weak<RefCell<WasmAppInner>> = Rc::downgrade(&inner);
    let backend_ref = Rc::clone(&backend);
    let state_ref = Rc::clone(&state);
    let events_ref = Rc::clone(&events);
    let app_ref = Rc::clone(&app);

    let raf = Closure::wrap(Box::new(move |_ts: f64| {
        let Some(inner) = inner_weak.upgrade() else {
            return;
        };
        {
            let mut app = inner.borrow_mut();
            app.raf_id = None;
            if !app.running {
                return;
            }
        }

        let frame_events = {
            let mut queue = events_ref.borrow_mut();
            std::mem::take(&mut *queue)
        };

        let keep_going = {
            let mut backend = backend_ref.borrow_mut();
            let mut state = state_ref.borrow_mut();
            let mut app = app_ref.borrow_mut();
            // `RefMut<T>` derefs to `T`, but the generic `&mut impl Backend`
            // bound has no coercion site, so the explicit reborrow is required
            // to hand `frame` a `&mut DomBackend` rather than `&mut RefMut<_>`.
            #[allow(clippy::explicit_auto_deref)]
            slt::frame(
                &mut *backend,
                &mut *state,
                &config,
                &frame_events,
                &mut *app,
            )
        };

        match keep_going {
            Ok(true) => {
                if let Err(err) = schedule_raf(&inner) {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "slt requestAnimationFrame error: {err:?}"
                    )));
                    stop_after_current_frame(inner);
                }
            }
            Ok(false) => stop_after_current_frame(inner),
            Err(err) => {
                web_sys::console::error_1(&JsValue::from_str(&format!("slt frame error: {err}")));
                stop_after_current_frame(inner);
            }
        }
    }) as Box<dyn FnMut(f64)>);

    inner.borrow_mut().raf = Some(raf);
    schedule_raf(&inner)?;

    Ok(WasmAppHandle::new(inner))
}

/// Mount `app` into `container` and run it until it requests exit.
///
/// This compatibility API preserves the original fire-and-forget behavior by
/// storing the owned [`WasmAppHandle`] internally. New integrations that need
/// explicit unmount/dispose control should prefer [`run_wasm_with_handle`].
///
/// # Errors
///
/// Returns a [`JsValue`] error when the `window` is unavailable, a listener
/// fails to install, or the initial frame cannot be scheduled.
pub fn run_wasm<F>(container: HtmlElement, width: u32, height: u32, app: F) -> Result<(), JsValue>
where
    F: FnMut(&mut Context) + 'static,
{
    let handle = run_wasm_with_handle(container, width, height, app)?;
    ACTIVE_APPS.with(|apps| {
        let mut apps = apps.borrow_mut();
        apps.retain(WasmAppHandle::is_running);
        apps.push(handle);
    });
    Ok(())
}

/// `wasm-bindgen` entry point that mounts an empty SLT app into `container`.
///
/// Thin wrapper over [`run_wasm`] with a no-op closure, exported to JS so the
/// browser harness can smoke-test the backend wiring. Errors are dropped.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn run_wasm_raw(container: HtmlElement, width: u32, height: u32) {
    let _ = run_wasm(container, width, height, |_ui| {});
}

/// `wasm-bindgen` entry point that mounts an empty SLT app and returns a
/// disposable handle.
///
/// This is the JS-friendly counterpart to [`run_wasm_with_handle`] for browser
/// harnesses that want to unmount the backend explicitly.
///
/// # Errors
///
/// Returns a [`JsValue`] error when mounting the empty app fails.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn run_wasm_raw_handle(
    container: HtmlElement,
    width: u32,
    height: u32,
) -> Result<WasmAppHandle, JsValue> {
    run_wasm_with_handle(container, width, height, |_ui| {})
}

#[cfg(test)]
mod tests {
    //! Logic-level tests for the DOM backend's pure helpers. The DOM-touching
    //! paths (`DomBackend`, the event listeners) require a browser and are
    //! exercised via the example harness; these cover the platform-independent
    //! translation logic that runs on the host.
    use super::*;

    #[test]
    fn wheel_vertical_maps_to_scroll_up_down() {
        // Negative deltaY scrolls up, positive scrolls down (crossterm parity).
        assert_eq!(wheel_delta_to_kind(0.0, -1.0), Some(MouseKind::ScrollUp));
        assert_eq!(wheel_delta_to_kind(0.0, 1.0), Some(MouseKind::ScrollDown));
    }

    #[test]
    fn wheel_horizontal_maps_to_scroll_left_right() {
        assert_eq!(wheel_delta_to_kind(-3.0, 0.0), Some(MouseKind::ScrollLeft));
        assert_eq!(wheel_delta_to_kind(3.0, 0.0), Some(MouseKind::ScrollRight));
    }

    #[test]
    fn wheel_dominant_axis_wins() {
        // A mostly-vertical diagonal gesture resolves to vertical scroll.
        assert_eq!(wheel_delta_to_kind(2.0, 10.0), Some(MouseKind::ScrollDown));
        // A mostly-horizontal diagonal gesture resolves to horizontal scroll.
        assert_eq!(wheel_delta_to_kind(-10.0, 2.0), Some(MouseKind::ScrollLeft));
    }

    #[test]
    fn wheel_zero_delta_is_ignored() {
        // Edge case: a wheel event with no movement must not enqueue a scroll.
        assert_eq!(wheel_delta_to_kind(0.0, 0.0), None);
    }

    #[test]
    fn wheel_ties_prefer_vertical() {
        // Equal magnitudes (|dy| >= |dx|) resolve to the vertical axis.
        assert_eq!(wheel_delta_to_kind(5.0, 5.0), Some(MouseKind::ScrollDown));
        assert_eq!(wheel_delta_to_kind(5.0, -5.0), Some(MouseKind::ScrollUp));
    }

    #[test]
    fn rgb_color_renders_as_hex() {
        assert_eq!(
            color_to_css(Some(Color::Rgb(0x12, 0x34, 0x56))),
            Some("#123456".to_string())
        );
    }

    #[test]
    fn reset_color_is_transparent() {
        assert_eq!(color_to_css(Some(Color::Reset)), None);
        assert_eq!(color_to_css(None), None);
    }

    #[test]
    fn indexed_color_cube_and_grayscale() {
        // 16-color base, the 6x6x6 cube, and the grayscale ramp must all map.
        assert_eq!(indexed_to_rgb(0), (0, 0, 0));
        assert_eq!(indexed_to_rgb(15), (255, 255, 255));
        assert_eq!(indexed_to_rgb(16), (0, 0, 0)); // cube origin
        assert_eq!(indexed_to_rgb(231), (255, 255, 255)); // cube far corner
        assert_eq!(indexed_to_rgb(232), (8, 8, 8)); // grayscale start
        assert_eq!(indexed_to_rgb(255), (238, 238, 238)); // grayscale end
    }

    #[test]
    fn style_css_includes_modifiers() {
        let style = slt::Style::new()
            .fg(Color::Red)
            .bg(Color::Black)
            .bold()
            .underline();
        let css = style_to_css(style);
        assert!(css.contains("color:#cd3131;"));
        assert!(css.contains("background-color:#000000;"));
        assert!(css.contains("font-weight:bold;"));
        assert!(css.contains("text-decoration-line:underline;"));
    }

    #[test]
    fn style_css_combines_text_decorations() {
        let style = slt::Style::new().underline().strikethrough();
        let css = style_to_css(style);
        assert!(css.contains("text-decoration-line:underline line-through;"));
        assert!(!css.contains("text-decoration:underline;"));
        assert!(!css.contains("text-decoration:line-through;"));
    }

    #[test]
    fn dom_modifier_bits_match_slt_modifiers() {
        let modifiers = modifiers_from_bools(true, true, true, true);
        assert!(modifiers.contains(KeyModifiers::SHIFT));
        assert!(modifiers.contains(KeyModifiers::CONTROL));
        assert!(modifiers.contains(KeyModifiers::ALT));
        assert!(modifiers.contains(KeyModifiers::SUPER));

        assert_eq!(
            modifiers_from_bools(false, false, false, false),
            KeyModifiers::NONE
        );
    }

    #[test]
    fn default_style_emits_no_css() {
        // Edge case: a plain default style produces no inline CSS at all.
        assert_eq!(style_to_css(slt::Style::new()), "");
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod browser_tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn test_container() -> HtmlElement {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("browser document");
        let container = document
            .create_element("div")
            .expect("create test container")
            .dyn_into::<HtmlElement>()
            .expect("container is HtmlElement");
        document
            .body()
            .expect("document body")
            .append_child(&container)
            .expect("append test container");
        container
    }

    #[wasm_bindgen_test]
    fn dom_backend_flush_builds_grid_and_updates_text() {
        let container = test_container();
        let mut backend = DomBackend::new(container.clone(), 4, 1);
        backend
            .buffer_mut()
            .set_string(0, 0, "SLT", slt::Style::new().fg(Color::Red));

        backend.flush().expect("flush DOM backend");

        assert!(container.inner_html().contains("<pre"));
        assert_eq!(container.text_content().as_deref(), Some("SLT "));
        container.remove();
    }

    #[wasm_bindgen_test]
    fn run_wasm_handle_mounts_and_disposes_runtime() {
        let container = test_container();
        let handle = run_wasm_raw_handle(container.clone(), 4, 1).expect("mount wasm runtime");

        assert!(handle.is_running());
        handle.dispose();
        assert!(!handle.is_running());
        container.remove();
    }
}
