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

use std::cell::{Cell, RefCell};
use std::io;
use std::rc::{Rc, Weak};

use slt::{
    AppState, Backend, Buffer, Color, Context, Event, KeyCode, KeyModifiers, Modifiers,
    MouseButton, MouseEvent as SltMouseEvent, MouseKind, Rect, RunConfig,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{
    Document, EventTarget, HtmlElement, HtmlPreElement, HtmlTextAreaElement, KeyboardEvent, Window,
};

type RafCallback = Closure<dyn FnMut(f64)>;

#[wasm_bindgen(
    inline_js = "export function guardedFrame(frame, failed) { return ts => { try { frame(ts); } catch (error) { failed(String(error)); } }; } export function deferCleanup(callback) { queueMicrotask(callback); }"
)]
extern "C" {
    #[wasm_bindgen(js_name = guardedFrame)]
    fn guarded_frame(frame: &js_sys::Function, failed: &js_sys::Function) -> js_sys::Function;
    #[wasm_bindgen(js_name = deferCleanup)]
    fn defer_cleanup(callback: &js_sys::Function);
}

/// Mount-time options specific to the browser runtime, not the terminal.
#[derive(Clone)]
pub struct WasmOptions {
    /// Initial grid width in cells. Default: 80. Must be positive.
    pub width: u32,
    /// Initial grid height in cells. Default: 24. Must be positive.
    pub height: u32,
    /// Initial colors. Can be overridden by `Context::set_theme`.
    pub theme: slt::Theme,
    /// Initial widget color overrides.
    pub widget_theme: slt::WidgetTheme,
    /// Lines scrolled for each browser wheel event.
    pub scroll_speed: u32,
    /// Maximum rendered frames per second; `None` renders every RAF.
    /// `Some(0)` is rejected. Input remains queued while throttled.
    pub max_fps: Option<u32>,
    /// Fit the host content box on mount and element resize. The host must
    /// have an independently sized width and height. Default: fixed grid.
    pub auto_fit: bool,
    /// Install focused keyboard, editable text, pointer and wheel listeners.
    /// Disable for a display-only mount. Never captures global keyboard input.
    pub input: bool,
}

impl Default for WasmOptions {
    fn default() -> Self {
        let config = RunConfig::default();
        Self {
            width: 80,
            height: 24,
            theme: config.theme,
            widget_theme: config.widget_theme,
            scroll_speed: config.scroll_speed,
            max_fps: Some(60),
            auto_fit: false,
            input: true,
        }
    }
}

thread_local! {
    static ACTIVE_APPS: RefCell<Vec<WasmAppHandle>> = const { RefCell::new(Vec::new()) };
}

struct EventListener {
    target: EventTarget,
    event_type: &'static str,
    callback: Closure<dyn Fn(web_sys::Event)>,
}

impl EventListener {
    fn install(
        target: EventTarget,
        event_type: &'static str,
        callback: Closure<dyn Fn(web_sys::Event)>,
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
    guarded_raf: Option<js_sys::Function>,
    failure_callback: Option<Closure<dyn FnMut(String)>>,
    raf_id: Option<i32>,
    running: bool,
    input_active: Rc<Cell<bool>>,
    error: Option<String>,
    input: Option<InputSurface>,
    observer: Option<web_sys::ResizeObserver>,
    observer_callback: Option<Closure<dyn FnMut()>>,
    pointer: Rc<Cell<Option<(i32, MouseButton)>>>,
    container: HtmlElement,
    added_tab_stop: bool,
}

impl WasmAppInner {
    fn new(window: Window, container: HtmlElement, events: Rc<RefCell<Vec<Event>>>) -> Self {
        Self {
            window,
            listeners: Vec::new(),
            events,
            raf: None,
            guarded_raf: None,
            failure_callback: None,
            raf_id: None,
            running: true,
            input_active: Rc::new(Cell::new(true)),
            error: None,
            input: None,
            observer: None,
            observer_callback: None,
            pointer: Rc::new(Cell::new(None)),
            container,
            added_tab_stop: false,
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

    /// The last fatal frame/scheduling error, or `None` after a normal stop.
    /// A trapped WASM instance must be discarded, not restarted.
    #[must_use]
    pub fn error(&self) -> Option<String> {
        self.inner.borrow().error.clone()
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
    grid: Option<HtmlPreElement>,
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
            grid: None,
            cells: Vec::new(),
            initialized: false,
            width,
            height,
        }
    }

    /// Resize the backend to a new cell grid, rebuilding its painted cells.
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
        // Keep the grid and editable sink mounted while rebuilding cells.
        self.initialized = false;
    }

    fn document(&self) -> Result<Document, io::Error> {
        self.container
            .owner_document()
            .ok_or_else(|| io::Error::other("owner document unavailable"))
    }

    fn initialize_grid(&mut self) -> io::Result<()> {
        let document = self.document()?;
        let pre = if let Some(grid) = &self.grid {
            grid.clone()
        } else {
            document
                .create_element("pre")
                .map_err(|e| io::Error::other(format!("create pre failed: {e:?}")))?
                .dyn_into::<HtmlPreElement>()
                .map_err(|_| io::Error::other("failed to cast pre element"))?
        };

        for cell in self.cells.drain(..) {
            cell.remove();
        }
        let mut child = pre.first_child();
        while let Some(node) = child {
            child = node.next_sibling();
            if node.node_type() == web_sys::Node::TEXT_NODE {
                pre.remove_child(&node)
                    .map_err(|e| io::Error::other(format!("remove row break failed: {e:?}")))?;
            }
        }

        pre.set_attribute(
            "style",
            &format!("position:relative;margin:0;padding:0;border:0;line-height:16px;font-family:monospace;font-size:14px;white-space:pre;direction:ltr;box-sizing:content-box;width:{}ch;height:{}px;", self.width, self.height * 16),
        )
        .map_err(|e| io::Error::other(format!("set pre style failed: {e:?}")))?;
        pre.set_attribute("data-slt-grid", "")
            .map_err(|e| io::Error::other(format!("mark grid failed: {e:?}")))?;

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

        if self.grid.is_none() {
            self.container
                .append_child(&pre)
                .map_err(|e| io::Error::other(format!("append pre failed: {e:?}")))?;
        }
        self.initialized = true;
        self.grid = Some(pre);
        Ok(())
    }

    /// Compute the cell grid `(cols, rows)` that fits the container's current
    /// pixel size, given a measured per-cell pixel size.
    ///
    /// Returns `None` when either the container or a cell reports a zero size so
    /// the caller can keep the existing dimensions instead of collapsing to a
    /// degenerate grid.
    fn fit_grid_to_container(&self) -> Option<(u32, u32)> {
        let grid = self.grid.as_ref()?;
        let style = web_sys::window()?
            .get_computed_style(&self.container)
            .ok()??;
        let padding = |name| {
            style
                .get_property_value(name)
                .ok()
                .and_then(|v| v.trim_end_matches("px").parse::<f64>().ok())
                .unwrap_or(0.0)
        };
        let cont_w = self.container.client_width() as f64
            - padding("padding-left")
            - padding("padding-right");
        let cont_h = self.container.client_height() as f64
            - padding("padding-top")
            - padding("padding-bottom");
        let cell_w = grid.get_bounding_client_rect().width() / self.width.max(1) as f64;
        let host_scale = self.container.get_bounding_client_rect().width()
            / self.container.offset_width().max(1) as f64;
        let cell_w = cell_w / host_scale;
        let cell_h = 16.0;
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

                let continuation = cell.is_continuation();
                let occupied = if x + 1 < self.width
                    && self.buffer.get(ox + x + 1, oy + y).is_continuation()
                {
                    2
                } else {
                    1
                };
                span.set_attribute("style", &format!("position:absolute;display:{};left:{}%;top:{}px;width:{}%;height:16px;line-height:16px;overflow:hidden;unicode-bidi:isolate;{}", if continuation { "none" } else { "block" }, x as f64 * 100.0 / self.width.max(1) as f64, y * 16, occupied as f64 * 100.0 / self.width.max(1) as f64, style_to_css(cell.style)))
                    .map_err(|e| io::Error::other(format!("set span style failed: {e:?}")))?;
                let symbol = cell.symbol.as_str();
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

#[derive(Clone)]
struct GridGeometry {
    grid: HtmlPreElement,
    width: u32,
    height: u32,
}

impl GridGeometry {
    fn position(&self, event: &JsValue, outside: bool) -> Option<(u32, u32)> {
        let rect = self.grid.get_bounding_client_rect();
        let x = reflect_f64(event, "clientX")? - rect.left();
        let y = reflect_f64(event, "clientY")? - rect.top();
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return None;
        }
        if x < 0.0 || y < 0.0 || x >= rect.width() || y >= rect.height() {
            // An outside release must never become a click on an edge widget.
            return outside.then_some((u32::MAX, u32::MAX));
        }
        Some((
            (x * self.width as f64 / rect.width()).floor() as u32,
            (y * self.height as f64 / rect.height()).floor() as u32,
        ))
    }
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
    let (container, added_tab_stop, pointer, input) = {
        let mut app = inner.borrow_mut();
        app.running = false;
        app.input_active.set(false);
        if let Some(raf_id) = app.raf_id.take() {
            let _ = app.window.cancel_animation_frame(raf_id);
        }
        (
            app.container.clone(),
            std::mem::take(&mut app.added_tab_stop),
            app.pointer.take(),
            app.input.clone(),
        )
    };
    // Release shared host ownership before a synchronous remount can claim it.
    // Deferred cleanup below only destroys this runtime's private resources.
    if added_tab_stop && container.get_attribute("tabindex").as_deref() == Some("0") {
        let _ = container.remove_attribute("tabindex");
    }
    if let Some((id, _)) = pointer {
        let _ = container.release_pointer_capture(id);
    }
    if let Some(input) = input {
        input.preedit.clear();
    }
    // Also safe when called synchronously from a user frame or DOM callback:
    // the currently executing wasm-bindgen closure is not destroyed on its stack.
    let inner = Rc::clone(inner);
    let cleanup = Closure::once_into_js(move || {
        let mut app = inner.borrow_mut();
        let listeners = std::mem::take(&mut app.listeners);
        let observer = app.observer.take();
        let observer_callback = app.observer_callback.take();
        let input = app.input.take();
        app.events.borrow_mut().clear();
        let raf = app.raf.take();
        let guarded = app.guarded_raf.take();
        let failure_callback = app.failure_callback.take();
        drop(app);
        drop(listeners);
        if let Some(observer) = observer {
            observer.disconnect();
        }
        if let Some(input) = input {
            input.sink.remove();
            input.preedit.element.remove();
        }
        drop((observer_callback, raf, guarded, failure_callback));
        ACTIVE_APPS.with(|apps| apps.borrow_mut().retain(WasmAppHandle::is_running));
    });
    defer_cleanup(cleanup.unchecked_ref());
}

fn fail_runtime(inner: &Rc<RefCell<WasmAppInner>>, error: String) {
    web_sys::console::error_1(&JsValue::from_str(&error));
    inner.borrow_mut().error = Some(error);
    dispose_inner_now(inner);
}

fn schedule_raf(inner: &Rc<RefCell<WasmAppInner>>) -> Result<(), JsValue> {
    let raf_id = {
        let app = inner.borrow();
        if !app.running {
            return Ok(());
        }
        let callback = app
            .guarded_raf
            .as_ref()
            .ok_or_else(|| JsValue::from_str("RAF callback unavailable"))?;
        app.window.request_animation_frame(callback)?
    };
    inner.borrow_mut().raf_id = Some(raf_id);
    Ok(())
}

#[derive(Default)]
struct TextBridge {
    composing: bool,
    committed: Option<String>,
    printable_modifiers: Option<KeyModifiers>,
    focused: bool,
}

#[derive(Clone)]
struct InputSurface {
    sink: HtmlTextAreaElement,
    preedit: Rc<Preedit>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PreeditAnchor {
    x: u32,
    y: u32,
    width: u32,
    style: slt::Style,
}

struct Preedit {
    element: HtmlElement,
    grid: HtmlPreElement,
    anchor: Cell<Option<PreeditAnchor>>,
    suppressed: Cell<bool>,
    dirty: Cell<bool>,
    text: RefCell<String>,
    foreground: Color,
    background: Color,
    runtime: Weak<RefCell<WasmAppInner>>,
}

impl Preedit {
    fn new(
        grid: HtmlPreElement,
        foreground: Color,
        background: Color,
        runtime: Weak<RefCell<WasmAppInner>>,
    ) -> Result<Self, JsValue> {
        let document = grid
            .owner_document()
            .ok_or_else(|| JsValue::from_str("preedit document unavailable"))?;
        let element = document.create_element("div")?.dyn_into::<HtmlElement>()?;
        element.set_attribute("data-slt-preedit", "")?;
        element.set_attribute("aria-hidden", "true")?;
        element.set_attribute("style", "display:none;pointer-events:none;")?;
        Ok(Self {
            element,
            grid,
            anchor: Cell::new(None),
            suppressed: Cell::new(true),
            dirty: Cell::new(false),
            text: RefCell::new(String::new()),
            foreground,
            background,
            runtime,
        })
    }

    fn clear(&self) {
        self.dirty.set(false);
        let had_text = !self.text.borrow().is_empty();
        self.text.borrow_mut().clear();
        if had_text {
            self.element.set_text_content(None);
            let _ = self.element.style().set_property("display", "none");
        }
    }

    fn begin(&self) {
        self.clear();
        self.suppressed.set(self.anchor.get().is_none());
    }

    fn update(&self, text: &str) {
        if self.suppressed.get() || self.anchor.get().is_none() {
            return;
        }
        if text.is_empty() {
            self.clear();
            return;
        }
        if *self.text.borrow() == text {
            return;
        }
        {
            let mut current = self.text.borrow_mut();
            current.clear();
            current.push_str(text);
        }
        // The next runtime frame checks the current caret privacy before
        // painting; queued focus/masking changes must not use a stale policy.
        self.dirty.set(true);
    }

    fn sync(&self, buffer: &Buffer) {
        let anchor = buffer
            .cursor_position()
            .filter(|&(x, y)| !buffer.cursor_is_masked() && buffer.area.contains(x, y))
            .map(|(x, y)| PreeditAnchor {
                x,
                y,
                width: buffer.area.width,
                style: buffer.get(x, y).style,
            });
        if anchor.is_none() {
            self.anchor.set(None);
            // Once a composition touches a private/unknown caret, do not
            // reveal its text even if focus or masking changes before it ends.
            self.suppressed.set(true);
            self.clear();
            return;
        }
        let moved = self.anchor.replace(anchor) != anchor;
        let changed = self.dirty.replace(false);
        if !self.suppressed.get() && (moved || changed) && !self.text.borrow().is_empty() {
            self.redraw();
        }
    }

    fn redraw(&self) {
        if let Err(error) = self.paint()
            && let Some(runtime) = self.runtime.upgrade()
        {
            fail_runtime(&runtime, format!("slt preedit error: {error:?}"));
        }
    }

    fn paint(&self) -> Result<(), JsValue> {
        let text = self.text.borrow().clone();
        let Some(anchor) = self.anchor.get() else {
            self.element.style().set_property("display", "none")?;
            return Ok(());
        };
        let available = anchor.width - anchor.x;
        // Reuse the core's grapheme, width, bidi and clipping rules without
        // writing any preedit text into the application's presented buffer.
        let mut buffer = Buffer::empty(Rect::new(0, 0, available, 1));
        buffer.set_string(0, 0, &text, slt::Style::new().underline());
        let end = buffer
            .content
            .iter()
            .rposition(|cell| cell.style.modifiers.contains(Modifiers::UNDERLINE))
            .map_or(0, |i| i + 1);
        let document = self
            .grid
            .owner_document()
            .ok_or_else(|| JsValue::from_str("preedit document unavailable"))?;
        let mut style = anchor.style.underline();
        if color_to_css(style.fg).is_none() {
            style.fg = Some(self.foreground);
        }
        let mut css = style_to_css(style);
        if color_to_css(style.bg).is_none() {
            css.push_str(&format!("background-color:{};", self.background_css()));
        }
        self.element.set_text_content(None);
        self.element.set_attribute("style", &format!("position:absolute;display:block;left:{}%;top:{}px;width:{}%;height:16px;line-height:16px;z-index:2;overflow:hidden;pointer-events:none;", anchor.x as f64 * 100.0 / anchor.width as f64, anchor.y * 16, available as f64 * 100.0 / anchor.width as f64))?;
        for x in 0..end {
            let cell = &buffer.content[x];
            if cell.is_continuation() {
                continue;
            }
            let occupied = if buffer
                .content
                .get(x + 1)
                .is_some_and(slt::Cell::is_continuation)
            {
                2
            } else {
                1
            };
            let glyph = document.create_element("span")?;
            glyph.set_attribute("style", &format!("position:absolute;left:{}%;top:0;width:{}%;height:16px;line-height:16px;white-space:pre;overflow:hidden;text-underline-offset:2px;{}", x as f64 * 100.0 / available as f64, occupied as f64 * 100.0 / available as f64, css))?;
            glyph.set_text_content(Some(cell.symbol.as_str()));
            self.element.append_child(&glyph)?;
        }
        Ok(())
    }

    fn background_css(&self) -> String {
        let mut element = Some(web_sys::Element::from(self.grid.clone()));
        if let Some(window) = web_sys::window() {
            while let Some(current) = element {
                if let Ok(Some(style)) = window.get_computed_style(&current)
                    && let Ok(color) = style.get_property_value("background-color")
                    && !color.is_empty()
                    && color != "transparent"
                    && color != "rgba(0, 0, 0, 0)"
                {
                    return color;
                }
                element = current.parent_element();
            }
        }
        color_to_css(Some(self.background)).unwrap_or_else(|| "#000000".into())
    }
}

fn owns_focus(container: &HtmlElement, input: &HtmlTextAreaElement) -> bool {
    container
        .owner_document()
        .and_then(|document| document.active_element())
        .is_some_and(|active| {
            &active == container.unchecked_ref::<web_sys::Element>()
                || &active == input.unchecked_ref::<web_sys::Element>()
        })
}

fn enqueue_mouse(
    events: &RefCell<Vec<Event>>,
    geometry: &RefCell<GridGeometry>,
    event: &web_sys::Event,
    kind: MouseKind,
    outside: bool,
) {
    if let Some((x, y)) = geometry.borrow().position(event.as_ref(), outside) {
        events.borrow_mut().push(Event::Mouse(SltMouseEvent::new(
            kind,
            x,
            y,
            reflect_event_modifiers(event.as_ref()),
            None,
            None,
        )));
    }
}

fn cancel_pointer(
    pointer: &Cell<Option<(i32, MouseButton)>>,
    events: &RefCell<Vec<Event>>,
    host: &HtmlElement,
) {
    if let Some((id, button)) = pointer.take() {
        events.borrow_mut().push(Event::Mouse(SltMouseEvent::new(
            MouseKind::Up(button),
            u32::MAX,
            u32::MAX,
            KeyModifiers::NONE,
            None,
            None,
        )));
        let _ = host.release_pointer_capture(id);
    }
}

fn install_event_listeners(
    container: &HtmlElement,
    surface: &InputSurface,
    window: &Window,
    geometry: Rc<RefCell<GridGeometry>>,
    events: Rc<RefCell<Vec<Event>>>,
    pointer: Rc<Cell<Option<(i32, MouseButton)>>>,
    running: Rc<Cell<bool>>,
) -> Result<Vec<EventListener>, JsValue> {
    let input = &surface.sink;
    let mut listeners = Vec::new();
    let bridge = Rc::new(RefCell::new(TextBridge::default()));

    for event_type in [
        "keydown",
        "beforeinput",
        "input",
        "compositionstart",
        "compositionupdate",
        "compositionend",
        "paste",
        "focusin",
        "focusout",
    ] {
        let host = container.clone();
        let sink = input.clone();
        let events = Rc::clone(&events);
        let bridge = Rc::clone(&bridge);
        let pointer = Rc::clone(&pointer);
        let running = Rc::clone(&running);
        let preedit = Rc::clone(&surface.preedit);
        let callback = Closure::wrap(Box::new(move |event: web_sys::Event| {
            if !running.get() {
                return;
            }
            if event_type == "focusin" {
                if !owns_focus(&host, &sink) {
                    return;
                }
                let was_focused = bridge.borrow().focused;
                bridge.borrow_mut().focused = true;
                if !was_focused {
                    events.borrow_mut().push(Event::FocusGained);
                }
                if event
                    .target()
                    .is_some_and(|target| target == event_target(&host))
                {
                    let _ = sink.focus();
                }
                return;
            }
            if event_type == "focusout" {
                if !bridge.borrow().focused {
                    return;
                }
                let related = event
                    .dyn_ref::<web_sys::FocusEvent>()
                    .and_then(|e| e.related_target());
                if related.is_some_and(|target| {
                    target == event_target(&sink) || target == event_target(&host)
                }) {
                    return;
                }
                *bridge.borrow_mut() = TextBridge::default();
                sink.set_value("");
                preedit.clear();
                cancel_pointer(&pointer, &events, &host);
                events.borrow_mut().push(Event::FocusLost);
                return;
            }
            if !owns_focus(&host, &sink) {
                return;
            }
            match event_type {
                "keydown" => {
                    let Some(key) = event.dyn_ref::<KeyboardEvent>() else {
                        return;
                    };
                    if key.is_composing() || key.key_code() == 229 || bridge.borrow().composing {
                        return;
                    }
                    bridge.borrow_mut().committed = None;
                    bridge.borrow_mut().printable_modifiers = None;
                    let name = key.key();
                    if (key.ctrl_key() || key.meta_key())
                        && matches!(name.to_ascii_lowercase().as_str(), "v" | "c" | "x")
                    {
                        return;
                    }
                    // Printable/dead-key/AltGraph text is committed by the editable sink.
                    if ((!key.ctrl_key() && !key.meta_key()) || key.get_modifier_state("AltGraph"))
                        && (name.chars().count() == 1 || name == "Dead")
                    {
                        if !key.alt_key() && name.chars().count() == 1 {
                            bridge.borrow_mut().printable_modifiers =
                                Some(keyboard_event_modifiers(key));
                        }
                        return;
                    }
                    if let Some(mapped) = keyboard_event_to_slt(key) {
                        events.borrow_mut().push(mapped);
                        key.prevent_default();
                    }
                }
                "compositionstart" => {
                    let mut state = bridge.borrow_mut();
                    state.composing = true;
                    state.committed = None;
                    state.printable_modifiers = None;
                    drop(state);
                    preedit.begin();
                }
                "compositionupdate" => {
                    if bridge.borrow().composing {
                        let text = event
                            .dyn_ref::<web_sys::CompositionEvent>()
                            .and_then(|e| e.data())
                            .unwrap_or_default();
                        preedit.update(&text);
                    }
                }
                "compositionend" => {
                    if !bridge.borrow().composing {
                        return;
                    }
                    let text = event
                        .dyn_ref::<web_sys::CompositionEvent>()
                        .and_then(|e| e.data())
                        .unwrap_or_default();
                    let mut state = bridge.borrow_mut();
                    state.composing = false;
                    state.committed = (!text.is_empty()).then(|| text.clone());
                    if !text.is_empty() {
                        events.borrow_mut().push(Event::Paste(text));
                    }
                    drop(state);
                    preedit.clear();
                    sink.set_value("");
                }
                "beforeinput" => {
                    let Some(input_event) = event.dyn_ref::<web_sys::InputEvent>() else {
                        return;
                    };
                    if input_event.is_composing() || bridge.borrow().composing {
                        return;
                    }
                    let code = match input_event.input_type().as_str() {
                        "deleteContentBackward" => Some(KeyCode::Backspace),
                        "deleteContentForward" => Some(KeyCode::Delete),
                        "insertLineBreak" | "insertParagraph" => Some(KeyCode::Enter),
                        _ => None,
                    };
                    if let Some(code) = code {
                        events.borrow_mut().push(Event::key(code));
                        if event.cancelable() {
                            event.prevent_default();
                        }
                        sink.set_value("");
                    }
                }
                "input" => {
                    let Some(input_event) = event.dyn_ref::<web_sys::InputEvent>() else {
                        return;
                    };
                    if input_event.is_composing() || bridge.borrow().composing {
                        if !bridge.borrow().composing {
                            preedit.begin();
                        }
                        bridge.borrow_mut().composing = true;
                        preedit.update(&input_event.data().unwrap_or_else(|| sink.value()));
                        return;
                    }
                    let kind = input_event.input_type();
                    let text = input_event
                        .data()
                        .filter(|text| !text.is_empty())
                        .unwrap_or_else(|| sink.value());
                    let duplicate = bridge.borrow_mut().committed.take().as_ref() == Some(&text);
                    let modifiers = bridge.borrow_mut().printable_modifiers.take();
                    if !duplicate
                        && kind.starts_with("insert")
                        && !text.is_empty()
                        && kind != "insertLineBreak"
                        && kind != "insertParagraph"
                    {
                        let mapped = if text.chars().count() == 1 {
                            modifiers.and_then(|modifiers| {
                                text.chars()
                                    .next()
                                    .map(|ch| Event::key_mod(KeyCode::Char(ch), modifiers))
                            })
                        } else {
                            None
                        };
                        events
                            .borrow_mut()
                            .push(mapped.unwrap_or(Event::Paste(text)));
                    }
                    sink.set_value("");
                }
                "paste" => {
                    if let Some(text) = paste_event_text(event.as_ref()) {
                        events.borrow_mut().push(Event::Paste(text));
                        event.prevent_default();
                        sink.set_value("");
                    }
                }
                _ => {}
            }
        }) as Box<dyn Fn(_)>);
        listeners.push(EventListener::install(
            event_target(container),
            event_type,
            callback,
        )?);
    }

    for event_type in [
        "pointerdown",
        "pointermove",
        "pointerup",
        "pointercancel",
        "lostpointercapture",
        "wheel",
    ] {
        let host = container.clone();
        let sink = input.clone();
        let events = Rc::clone(&events);
        let geometry = Rc::clone(&geometry);
        let pointer = Rc::clone(&pointer);
        let running = Rc::clone(&running);
        let callback = Closure::wrap(Box::new(move |event: web_sys::Event| {
            if !running.get() {
                return;
            }
            if event_type == "wheel" {
                let value = event.as_ref();
                if let Some(kind) = wheel_delta_to_kind(
                    reflect_f64(value, "deltaX").unwrap_or(0.0),
                    reflect_f64(value, "deltaY").unwrap_or(0.0),
                ) && geometry.borrow().position(value, false).is_some()
                {
                    enqueue_mouse(&events, &geometry, &event, kind, false);
                    event.prevent_default();
                }
                return;
            }
            let Some(pe) = event.dyn_ref::<web_sys::PointerEvent>() else {
                return;
            };
            match event_type {
                "pointerdown" => {
                    if pointer.get().is_some()
                        || geometry.borrow().position(event.as_ref(), false).is_none()
                    {
                        return;
                    }
                    let _ = sink.focus();
                    let button = mouse_button(pe.button());
                    pointer.set(Some((pe.pointer_id(), button)));
                    let _ = host.set_pointer_capture(pe.pointer_id());
                    enqueue_mouse(&events, &geometry, &event, MouseKind::Down(button), false);
                    // Keep right-button defaults so browser context-menu paste remains available.
                    if button == MouseButton::Left {
                        event.prevent_default();
                    }
                }
                "pointermove" => {
                    let kind = match pointer.get() {
                        Some((id, button)) if id == pe.pointer_id() => MouseKind::Drag(button),
                        Some(_) => return,
                        None => {
                            if !event
                                .target()
                                .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
                                .is_some_and(|target| host.contains(Some(&target)))
                            {
                                return;
                            }
                            MouseKind::Moved
                        }
                    };
                    enqueue_mouse(&events, &geometry, &event, kind, false);
                }
                "pointerup" => {
                    if let Some((id, button)) = pointer.get() {
                        if id != pe.pointer_id() || mouse_button(pe.button()) != button {
                            return;
                        }
                        pointer.set(None);
                        enqueue_mouse(&events, &geometry, &event, MouseKind::Up(button), true);
                        let _ = host.release_pointer_capture(id);
                    }
                }
                _ => {
                    if pointer.get().is_some_and(|(id, _)| id == pe.pointer_id()) {
                        cancel_pointer(&pointer, &events, &host);
                    }
                }
            }
        }) as Box<dyn Fn(_)>);
        let target = if matches!(event_type, "pointermove" | "pointerup") {
            event_target(window)
        } else {
            event_target(container)
        };
        listeners.push(EventListener::install(target, event_type, callback)?);
    }
    for event_type in ["blur", "focus"] {
        let events = Rc::clone(&events);
        let bridge = Rc::clone(&bridge);
        let pointer = Rc::clone(&pointer);
        let running = Rc::clone(&running);
        let host = container.clone();
        let sink = input.clone();
        let preedit = Rc::clone(&surface.preedit);
        let callback = Closure::wrap(Box::new(move |_: web_sys::Event| {
            if !running.get() {
                return;
            }
            if event_type == "blur" && bridge.borrow().focused {
                *bridge.borrow_mut() = TextBridge::default();
                sink.set_value("");
                preedit.clear();
                cancel_pointer(&pointer, &events, &host);
                events.borrow_mut().push(Event::FocusLost);
            } else if event_type == "focus" && owns_focus(&host, &sink) && !bridge.borrow().focused
            {
                bridge.borrow_mut().focused = true;
                events.borrow_mut().push(Event::FocusGained);
            }
        }) as Box<dyn Fn(_)>);
        listeners.push(EventListener::install(
            event_target(window),
            event_type,
            callback,
        )?);
    }
    Ok(listeners)
}

/// Mount a fixed grid with default browser options and retain its runtime owner.
///
/// # Errors
/// Returns an error for invalid dimensions or unavailable browser resources.
pub fn run_wasm_with_handle<F>(
    container: HtmlElement,
    width: u32,
    height: u32,
    app: F,
) -> Result<WasmAppHandle, JsValue>
where
    F: FnMut(&mut Context) + 'static,
{
    run_wasm_with_options(
        container,
        WasmOptions {
            width,
            height,
            ..WasmOptions::default()
        },
        app,
    )
}

/// Mount a browser application with explicit mount-time configuration.
///
/// Every rendered frame starts with a fresh live buffer. Direct
/// `DomBackend::buffer_mut`/`flush` users keep incremental drawing semantics.
/// The host is not owned: disposal retains its content and last rendered grid.
/// A detached host keeps running until disposal, drop, quit, or failure.
///
/// # Errors
/// Returns an error for zero dimensions/FPS or unavailable browser resources.
pub fn run_wasm_with_options<F>(
    container: HtmlElement,
    options: WasmOptions,
    mut app: F,
) -> Result<WasmAppHandle, JsValue>
where
    F: FnMut(&mut Context) + 'static,
{
    let (width, height) = (options.width, options.height);
    if width == 0 || height == 0 || options.max_fps == Some(0) {
        return Err(JsValue::from_str(
            "grid dimensions and max_fps must be positive",
        ));
    }
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    // A disposed runtime deliberately leaves its last frame in place. A new
    // mount replaces that grid only, without deleting caller-owned children.
    if let Some(previous_grid) = container.query_selector("pre[data-slt-grid]")? {
        previous_grid.remove();
    }
    let mut backend = DomBackend::new(container.clone(), width, height);
    backend
        .initialize_grid()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    if options.auto_fit
        && let Some((cols, rows)) = backend.fit_grid_to_container()
    {
        backend.resize(cols, rows);
        if !backend.initialized {
            backend
                .initialize_grid()
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
        }
    }
    let geometry = Rc::new(RefCell::new(GridGeometry {
        grid: backend
            .grid
            .clone()
            .ok_or_else(|| JsValue::from_str("grid unavailable"))?,
        width: backend.width,
        height: backend.height,
    }));
    let events = Rc::new(RefCell::new(Vec::new()));
    let inner = Rc::new(RefCell::new(WasmAppInner::new(
        window.clone(),
        container.clone(),
        Rc::clone(&events),
    )));
    let handle = WasmAppHandle::new(Rc::clone(&inner));
    let running = Rc::clone(&inner.borrow().input_active);
    if options.input {
        if !container.has_attribute("tabindex") {
            container.set_tab_index(0);
            inner.borrow_mut().added_tab_stop = true;
        }
        let document = container
            .owner_document()
            .ok_or_else(|| JsValue::from_str("document unavailable"))?;
        let input = document
            .create_element("textarea")?
            .dyn_into::<HtmlTextAreaElement>()?;
        input.set_attribute("data-slt-input", "")?;
        input.set_attribute("aria-label", "Terminal input")?;
        input.set_attribute("autocapitalize", "off")?;
        input.set_attribute("autocomplete", "off")?;
        input.set_attribute("spellcheck", "false")?;
        input.set_attribute("style", "position:absolute;left:0;top:0;width:100%;height:100%;z-index:1;opacity:0.01;background:transparent;color:transparent;caret-color:transparent;outline:none;padding:0;margin:0;border:0;box-sizing:border-box;resize:none;overflow:hidden;pointer-events:auto;font:14px/16px monospace;white-space:pre;")?;
        input.set_tab_index(-1);
        let grid = geometry.borrow().grid.clone();
        let preedit = Rc::new(Preedit::new(
            grid.clone(),
            options.theme.text,
            options.theme.bg,
            Rc::downgrade(&inner),
        )?);
        let surface = InputSurface {
            sink: input.clone(),
            preedit,
        };
        inner.borrow_mut().input = Some(surface.clone());
        grid.append_child(&input)?;
        grid.append_child(&surface.preedit.element)?;
        let listeners = install_event_listeners(
            &container,
            &surface,
            &window,
            Rc::clone(&geometry),
            Rc::clone(&events),
            Rc::clone(&inner.borrow().pointer),
            Rc::clone(&running),
        )?;
        inner.borrow_mut().listeners = listeners;
    }
    let resize_pending = Rc::new(Cell::new(false));
    if options.auto_fit {
        let pending = Rc::clone(&resize_pending);
        let callback = Closure::wrap(Box::new(move || pending.set(true)) as Box<dyn FnMut()>);
        let observer = web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref())?;
        observer.observe(&container);
        inner.borrow_mut().observer = Some(observer);
        inner.borrow_mut().observer_callback = Some(callback);
    }
    let mut config = RunConfig::default();
    config.theme = options.theme;
    config.widget_theme = options.widget_theme;
    config.scroll_speed = options.scroll_speed;
    config.handle_ctrl_c = false;
    let mut state = AppState::new();
    let mut last_frame = None;
    let mut last_input_cursor = None;
    let interval = options.max_fps.map(|fps| 1000.0 / fps as f64);
    let weak: Weak<RefCell<WasmAppInner>> = Rc::downgrade(&inner);
    let raf = Closure::wrap(Box::new(move |timestamp: f64| {
        let Some(inner) = weak.upgrade() else {
            return;
        };
        {
            let mut runtime = inner.borrow_mut();
            runtime.raf_id = None;
            if !runtime.running {
                running.set(false);
                return;
            }
        }
        if !frame_due(&mut last_frame, timestamp, interval) {
            if let Err(error) = schedule_raf(&inner) {
                fail_runtime(&inner, format!("{error:?}"));
            }
            return;
        }
        if resize_pending.replace(false)
            && let Some((cols, rows)) = backend.fit_grid_to_container()
            && backend.size() != (cols, rows)
        {
            backend.resize(cols, rows);
            events.borrow_mut().push(Event::Resize(cols, rows));
        }
        let frame_events = std::mem::take(&mut *events.borrow_mut());
        backend.buffer_mut().reset();
        let result = slt::frame_owned(&mut backend, &mut state, &config, frame_events, &mut app);
        if let Some(grid) = backend.grid.clone() {
            *geometry.borrow_mut() = GridGeometry {
                grid,
                width: backend.width,
                height: backend.height,
            };
        }
        let input = inner.borrow().input.clone();
        if let Some(surface) = input {
            surface.preedit.sync(&backend.buffer);
            let input = &surface.sink;
            let (x, y) = backend.buffer.cursor_position().unwrap_or((0, 0));
            // Paint and editing share local grid coordinates, so ancestor
            // transforms apply exactly once to both the overlay and IME caret.
            let cursor = (x, y, backend.width);
            if last_input_cursor != Some(cursor) {
                let _ = input.style().set_property(
                    "padding-left",
                    &format!("{}%", x as f64 * 100.0 / backend.width as f64),
                );
                let _ = input
                    .style()
                    .set_property("padding-top", &format!("{}px", y * 16));
                last_input_cursor = Some(cursor);
            }
        }
        match result {
            Ok(true) if inner.borrow().running => {
                if let Err(error) = schedule_raf(&inner) {
                    fail_runtime(&inner, format!("{error:?}"));
                }
            }
            Ok(_) => {
                running.set(false);
                dispose_inner_now(&inner);
            }
            Err(error) => {
                running.set(false);
                fail_runtime(&inner, error.to_string());
            }
        }
    }) as Box<dyn FnMut(f64)>);
    let weak = Rc::downgrade(&inner);
    let failed = Closure::wrap(Box::new(move |message: String| {
        if let Some(inner) = weak.upgrade() {
            fail_runtime(
                &inner,
                format!("fatal browser frame: {message}; discard this WASM instance"),
            );
        }
    }) as Box<dyn FnMut(String)>);
    let guarded = guarded_frame(
        raf.as_ref().unchecked_ref(),
        failed.as_ref().unchecked_ref(),
    );
    {
        let mut runtime = inner.borrow_mut();
        runtime.raf = Some(raf);
        runtime.guarded_raf = Some(guarded);
        runtime.failure_callback = Some(failed);
    }
    schedule_raf(&inner)?;
    Ok(handle)
}

fn frame_due(last: &mut Option<f64>, timestamp: f64, interval: Option<f64>) -> bool {
    if let (Some(previous), Some(interval)) = (*last, interval) {
        if timestamp >= previous && timestamp - previous + 0.01 < interval {
            return false;
        }
        // Retain fractional RAF remainder; do not catch up a suspended tab.
        *last = Some(if timestamp - previous < interval * 2.0 {
            previous + interval
        } else {
            timestamp
        });
    } else {
        *last = Some(timestamp);
    }
    true
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

    #[test]
    fn pacing_caps_high_refresh_without_losing_fractional_remainder() {
        for refresh in [60, 120, 144] {
            let mut last = None;
            let frames = (0..refresh)
                .filter(|n| {
                    frame_due(
                        &mut last,
                        *n as f64 * 1000.0 / refresh as f64,
                        Some(1000.0 / 60.0),
                    )
                })
                .count();
            assert_eq!(frames, 60, "refresh rate {refresh}");
        }
    }

    #[test]
    fn pacing_handles_uncapped_suspension_and_timestamp_reset() {
        let mut last = None;
        assert!(frame_due(&mut last, 0.0, Some(50.0)));
        assert!(!frame_due(&mut last, 49.0, Some(50.0)));
        assert!(frame_due(&mut last, 50.0, Some(50.0)));
        assert!(frame_due(&mut last, 10_000.0, Some(50.0)));
        assert!(!frame_due(&mut last, 10_001.0, Some(50.0)));
        assert!(frame_due(&mut last, 0.0, Some(50.0)));
        assert!(frame_due(&mut last, 1.0, None));
        assert!(frame_due(&mut last, 2.0, None));
        assert_eq!(WasmOptions::default().max_fps, Some(60));
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
