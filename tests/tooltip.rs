use slt::event::{Event, KeyModifiers, MouseEvent, MouseKind};
use slt::rect::Rect;
use slt::{frame, Backend, Buffer, RunConfig};

struct FrameBackend {
    buffer: Buffer,
}

impl FrameBackend {
    fn new(width: u32, height: u32) -> Self {
        Self {
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
        }
    }

    fn contains(&self, needle: &str) -> bool {
        for y in 0..self.buffer.area.height {
            let mut line = String::new();
            for x in 0..self.buffer.area.width {
                line.push_str(&self.buffer.get(x, y).symbol);
            }
            if line.contains(needle) {
                return true;
            }
        }
        false
    }
}

impl Backend for FrameBackend {
    fn size(&self) -> (u32, u32) {
        (self.buffer.area.width, self.buffer.area.height)
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn tooltip_hidden_when_widget_not_hovered() {
    let mut backend = FrameBackend::new(80, 24);
    let mut state = slt::AppState::new();
    let config = RunConfig::default().mouse(true);

    frame(&mut backend, &mut state, &config, &[], &mut |ui| {
        let _ = ui.button("Save");
        ui.tooltip("tooltip text");
    })
    .unwrap();

    let events = vec![Event::mouse_move(79, 23)];

    frame(&mut backend, &mut state, &config, &events, &mut |ui| {
        let _ = ui.button("Save");
        ui.tooltip("tooltip text");
    })
    .unwrap();

    assert!(
        !backend.contains("tooltip text"),
        "Tooltip should be hidden when widget is not hovered"
    );
}

#[test]
fn tooltip_renders_when_widget_hovered() {
    let mut backend = FrameBackend::new(80, 24);
    let mut state = slt::AppState::new();
    let config = RunConfig::default().mouse(true);

    frame(&mut backend, &mut state, &config, &[], &mut |ui| {
        let _ = ui.button("Save");
        ui.tooltip("tooltip text");
    })
    .unwrap();

    let events = vec![Event::mouse_move(1, 0)];

    frame(&mut backend, &mut state, &config, &events, &mut |ui| {
        let _ = ui.button("Save");
        ui.tooltip("tooltip text");
    })
    .unwrap();

    assert!(
        backend.contains("tooltip text"),
        "Tooltip text should render when widget is hovered"
    );
}
