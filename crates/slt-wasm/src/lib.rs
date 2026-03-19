use std::cell::RefCell;
use std::io;
use std::rc::Rc;

use slt::{
    AppState, Backend, Buffer, Color, Context, Event, KeyCode, KeyModifiers, Modifiers,
    MouseButton, MouseEvent as SltMouseEvent, MouseKind, Rect, RunConfig,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement, HtmlPreElement, KeyboardEvent, MouseEvent, Window};

pub struct DomBackend {
    buffer: Buffer,
    container: HtmlElement,
    cells: Vec<HtmlElement>,
    initialized: bool,
    width: u32,
    height: u32,
}

impl DomBackend {
    pub fn new(container: HtmlElement, width: u32, height: u32) -> Self {
        Self {
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
            container,
            cells: Vec::new(),
            initialized: false,
            width,
            height,
        }
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
}

impl Backend for DomBackend {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.initialized {
            self.initialize_grid()?;
        }

        let ox = self.buffer.area.x;
        let oy = self.buffer.area.y;
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = (y * self.width + x) as usize;
                let cell = self.buffer.get(ox + x, oy + y);
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
    if style.modifiers.contains(Modifiers::UNDERLINE) {
        css.push_str("text-decoration:underline;");
    }
    if style.modifiers.contains(Modifiers::STRIKETHROUGH) {
        css.push_str("text-decoration:line-through;");
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

    let mut modifiers = KeyModifiers::NONE;
    if event.shift_key() {
        modifiers.0 |= KeyModifiers::SHIFT.0;
    }
    if event.ctrl_key() {
        modifiers.0 |= KeyModifiers::CONTROL.0;
    }
    if event.alt_key() {
        modifiers.0 |= KeyModifiers::ALT.0;
    }

    Some(Event::key_mod(code, modifiers))
}

fn mouse_button(button: i16) -> MouseButton {
    match button {
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        _ => MouseButton::Left,
    }
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

fn install_event_listeners(
    container: &HtmlElement,
    width: u32,
    height: u32,
    events: Rc<RefCell<Vec<Event>>>,
) -> Result<(), JsValue> {
    container.set_tab_index(0);

    let key_events = Rc::clone(&events);
    let keydown = Closure::wrap(Box::new(move |event: KeyboardEvent| {
        if let Some(slt_event) = keyboard_event_to_slt(&event) {
            key_events.borrow_mut().push(slt_event);
            event.prevent_default();
        }
    }) as Box<dyn FnMut(_)>);
    container.add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())?;
    keydown.forget();

    let move_events = Rc::clone(&events);
    let container_move = container.clone();
    let mousemove = Closure::wrap(Box::new(move |event: MouseEvent| {
        if let Some((x, y)) = mouse_cell_position(&event, &container_move, width, height) {
            move_events.borrow_mut().push(Event::Mouse(SltMouseEvent {
                kind: MouseKind::Moved,
                x,
                y,
                modifiers: KeyModifiers::NONE,
            }));
        }
    }) as Box<dyn FnMut(_)>);
    container.add_event_listener_with_callback("mousemove", mousemove.as_ref().unchecked_ref())?;
    mousemove.forget();

    let down_events = Rc::clone(&events);
    let container_down = container.clone();
    let mousedown = Closure::wrap(Box::new(move |event: MouseEvent| {
        if let Some((x, y)) = mouse_cell_position(&event, &container_down, width, height) {
            down_events.borrow_mut().push(Event::Mouse(SltMouseEvent {
                kind: MouseKind::Down(mouse_button(event.button())),
                x,
                y,
                modifiers: KeyModifiers::NONE,
            }));
        }
    }) as Box<dyn FnMut(_)>);
    container.add_event_listener_with_callback("mousedown", mousedown.as_ref().unchecked_ref())?;
    mousedown.forget();

    let up_events = Rc::clone(&events);
    let container_up = container.clone();
    let mouseup = Closure::wrap(Box::new(move |event: MouseEvent| {
        if let Some((x, y)) = mouse_cell_position(&event, &container_up, width, height) {
            up_events.borrow_mut().push(Event::Mouse(SltMouseEvent {
                kind: MouseKind::Up(mouse_button(event.button())),
                x,
                y,
                modifiers: KeyModifiers::NONE,
            }));
        }
    }) as Box<dyn FnMut(_)>);
    container.add_event_listener_with_callback("mouseup", mouseup.as_ref().unchecked_ref())?;
    mouseup.forget();

    Ok(())
}

pub fn run_wasm<F>(container: HtmlElement, width: u32, height: u32, app: F) -> Result<(), JsValue>
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

    install_event_listeners(&container, width, height, Rc::clone(&events))?;

    let raf: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let raf_for_assign = Rc::clone(&raf);
    let raf_for_loop = Rc::clone(&raf);
    let backend_ref = Rc::clone(&backend);
    let state_ref = Rc::clone(&state);
    let events_ref = Rc::clone(&events);
    let app_ref = Rc::clone(&app);
    let window_ref = window.clone();

    *raf_for_assign.borrow_mut() = Some(Closure::wrap(Box::new(move |_ts: f64| {
        let frame_events = {
            let mut queue = events_ref.borrow_mut();
            std::mem::take(&mut *queue)
        };

        let keep_going = {
            let mut backend = backend_ref.borrow_mut();
            let mut state = state_ref.borrow_mut();
            let mut app = app_ref.borrow_mut();
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
                if let Some(cb) = raf_for_loop.borrow().as_ref() {
                    let _ = window_ref.request_animation_frame(cb.as_ref().unchecked_ref());
                }
            }
            Ok(false) => {}
            Err(err) => {
                web_sys::console::error_1(&JsValue::from_str(&format!("slt frame error: {err}")));
            }
        }
    }) as Box<dyn FnMut(f64)>));

    {
        let borrow = raf.borrow();
        if let Some(cb) = borrow.as_ref() {
            window.request_animation_frame(cb.as_ref().unchecked_ref())?;
        } else {
            return Err(JsValue::from_str(
                "failed to initialize requestAnimationFrame loop",
            ));
        }
    }

    std::mem::forget(raf);
    Ok(())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn run_wasm_raw(container: HtmlElement, width: u32, height: u32) {
    let _ = run_wasm(container, width, height, |_ui| {});
}
