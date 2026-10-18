//! The braille `canvas` widget and its fallible / configured variants.

use super::*;

impl Context {
    /// Render a braille drawing canvas.
    ///
    /// The closure receives a [`CanvasContext`] for pixel-level drawing. Each
    /// terminal cell maps to a 2x4 braille dot matrix, giving `width*2` x
    /// `height*4` pixel resolution.
    /// Requested coordinates are retained even inside a smaller parent.
    ///
    /// # Panics
    /// Panics when backing storage exceeds the [`CanvasContext`] budgets or
    /// allocation fails. Use [`Self::try_canvas`] to handle construction errors.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.canvas(40, 10, |cv| {
    ///     cv.line(0, 0, cv.width() - 1, cv.height() - 1);
    ///     cv.circle(40, 20, 15);
    /// });
    /// # });
    /// ```
    pub fn canvas(
        &mut self,
        width: u32,
        height: u32,
        draw: impl FnOnce(&mut CanvasContext),
    ) -> Response {
        self.try_canvas(width, height, draw)
            .expect("canvas allocation failed")
    }

    /// Render a canvas with fallible backing-storage construction.
    ///
    /// Zero dimensions return an empty response without invoking `draw`.
    ///
    /// # Errors
    /// Returns [`CanvasError`] when dimensions or initial backing allocation
    /// fail, before invoking `draw` or recording any layout commands. Within
    /// `draw`, use [`CanvasContext::try_layer`] and [`CanvasContext::try_print`]
    /// to handle additional storage requests.
    pub fn try_canvas(
        &mut self,
        width: u32,
        height: u32,
        draw: impl FnOnce(&mut CanvasContext),
    ) -> Result<Response, CanvasError> {
        if width == 0 || height == 0 {
            return Ok(Response::none());
        }
        let mut canvas = CanvasContext::try_new(width, height)?;
        draw(&mut canvas);
        Ok(self.render_canvas(&mut canvas, width, height))
    }

    /// Render using caller-owned canvas storage, cleared before each draw.
    ///
    /// Retains the requested coordinate space, grids, label slots and scratch
    /// across frames. Zero dimensions do not invoke `draw`. Drop the canvas to
    /// release retained storage; construct another to change its dimensions.
    pub fn canvas_with(
        &mut self,
        canvas: &mut CanvasContext,
        draw: impl FnOnce(&mut CanvasContext),
    ) -> Response {
        let width = (canvas.width() / 2) as u32;
        let height = (canvas.height() / 4) as u32;
        canvas.clear();
        if width == 0 || height == 0 {
            return Response::none();
        }
        draw(canvas);
        self.render_canvas(canvas, width, height)
    }

    fn render_canvas(&mut self, canvas: &mut CanvasContext, width: u32, height: u32) -> Response {
        let response = self.begin_sized_viz_root(width, height);
        for segments in canvas.render() {
            self.skip_interaction_slot();
            self.commands
                .push(Command::BeginContainer(BeginContainerArgs {
                    direction: Direction::Row,
                    gap: 0,
                    align: Align::Start,
                    align_self: None,
                    justify: Justify::Start,
                    border: None,
                    border_sides: BorderSides::all(),
                    border_style: Style::new(),
                    bg_color: None,
                    padding: Padding::default(),
                    margin: Margin::default(),
                    constraints: Constraints::default(),
                    title: None,
                    grow: 0,
                    group_name: None,
                }));
            for (text, color) in segments {
                let c = if color == Color::Reset {
                    self.theme.primary
                } else {
                    color
                };
                self.styled(text, Style::new().fg(c));
            }
            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
        }
        self.end_viz_root();

        response
    }
}
