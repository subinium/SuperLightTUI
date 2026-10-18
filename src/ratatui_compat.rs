//! Render [ratatui](https://ratatui.rs) widgets inside SLT containers.
//!
//! Enabled by the opt-in `ratatui` feature, which depends only on
//! [`ratatui-core`](https://docs.rs/ratatui-core) 0.1 (the widget/buffer half
//! of ratatui 0.30). No ratatui backend or second crossterm is pulled in, and
//! the feature is not part of `full`.
//!
//! The bridge is render-only: a ratatui [`Widget`] draws into a scratch
//! ratatui [`Buffer`](ratatui_core::buffer::Buffer) sized to the laid-out SLT
//! region, and the result is copied cell by cell into SLT's
//! [`Buffer`](crate::Buffer) through the normal clipped, grapheme-safe write
//! path. Input handling, focus, and `StatefulWidget` state stay with the
//! caller.
//!
//! ```no_run
//! use slt::ratatui_compat::ratatui_core::style::{Color, Style};
//! # slt::run(|ui: &mut slt::Context| {
//! let text = format!("frame {}", ui.tick());
//! ui.container().h(3).ratatui(move |area, buf| {
//!     buf.set_string(area.x, area.y, &text, Style::new().fg(Color::Cyan));
//! });
//! # });
//! ```
//!
//! # Coordinates and clipping
//!
//! The closure receives an `area` whose origin is `(0, 0)` and whose size is
//! the region's full laid-out size, even when a scroll container or a parent
//! clip hides part of it. Only the visible part is copied back, so a
//! [`Block`](https://docs.rs/ratatui-widgets/latest/ratatui_widgets/block/struct.Block.html)
//! scrolled half out of view keeps its geometry instead of being re-laid-out
//! into the visible slice. Regions larger than [`MAX_SCRATCH_CELLS`] (or wider
//! / taller than `u16::MAX`) are rendered at their visible size instead.
//!
//! Before the closure runs, the visible cells already in SLT's buffer (for
//! example a parent container's background) are copied into the scratch
//! buffer, so ratatui widgets compose over SLT content the same way they
//! compose over each other in ratatui. Cells the widget leaves untouched are
//! not written back, which preserves SLT-only attributes such as hyperlinks
//! and `OVERLINE`.
//!
//! # Style mapping
//!
//! | ratatui | SLT | Notes |
//! |---------|-----|-------|
//! | `Color::Reset` | `Color::Reset` | |
//! | `Color::Black` … `Color::Cyan` | same name | indices 0–6 |
//! | `Color::Gray` | `Color::White` | index 7 |
//! | `Color::DarkGray` | `Color::DarkGray` | index 8 |
//! | `Color::LightRed` … `Color::LightCyan` | same name | indices 9–14 |
//! | `Color::White` | `Color::LightWhite` | index 15 |
//! | `Color::Rgb` / `Color::Indexed` | `Color::Rgb` / `Color::Indexed` | |
//! | `BOLD`, `DIM`, `ITALIC`, `REVERSED` | same name | |
//! | `UNDERLINED` | `UNDERLINE` | |
//! | `CROSSED_OUT` | `STRIKETHROUGH` | |
//! | `SLOW_BLINK`, `RAPID_BLINK` | `BLINK` | lossy: SLT has one blink |
//! | `HIDDEN` | *(dropped)* | the cell's symbol is drawn as spaces |
//! | *(none)* | `OVERLINE` | dropped when converting to ratatui |
//! | `Style::sub_modifier` | *(dropped)* | SLT styles are additive |
//!
//! Colour conversion is lossless in both directions. SLT's
//! [`UnderlineStyle`](crate::style::UnderlineStyle) has no ratatui
//! counterpart and becomes `Straight` when converting from ratatui. Cells
//! marked `CellDiffOption::Skip` / `ForcedWidth` (escape-sequence
//! passthrough) are not copied.

use std::cell::RefCell;

use ratatui_core::buffer::{Buffer as RatBuffer, Cell as RatCell, CellDiffOption};
use ratatui_core::layout::Rect as RatRect;
use ratatui_core::style::{Color as RatColor, Modifier as RatModifier, Style as RatStyle};
use ratatui_core::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use crate::buffer::Buffer;
use crate::cell::Cell;
use crate::context::ContainerBuilder;
use crate::rect::Rect;
use crate::style::{Color, Modifiers, Style};

/// The `ratatui-core` crate this bridge is built against.
///
/// Use this re-export (or a matching `ratatui-core = "0.1"` / `ratatui =
/// "0.30"` dependency) so your widgets implement the same `Widget` trait.
pub use ratatui_core;

/// Upper bound on the scratch buffer rendered at the region's full logical
/// size. Larger regions are rendered at their visible size.
pub const MAX_SCRATCH_CELLS: usize = crate::buffer::MAX_BUFFER_CELLS;

/// Scratch buffers up to this many cells are kept for reuse by the next
/// region; larger ones are freed after use so one huge region cannot pin
/// memory for the rest of the process.
const RETAIN_SCRATCH_CELLS: usize = 65_536;

thread_local! {
    static SCRATCH: RefCell<Option<RatBuffer>> = const { RefCell::new(None) };
}

#[cfg_attr(docsrs, doc(cfg(feature = "ratatui")))]
impl ContainerBuilder<'_> {
    /// Finalize the builder as a region rendered by ratatui code.
    ///
    /// `f` receives a ratatui [`Rect`](ratatui_core::layout::Rect) with origin
    /// `(0, 0)` covering the region's laid-out size and a scratch ratatui
    /// [`Buffer`](ratatui_core::buffer::Buffer). Anything it draws is copied
    /// into the SLT frame, clipped to the region's visible part. See the
    /// [module docs](crate::ratatui_compat) for coordinate, clipping and style
    /// mapping rules.
    ///
    /// Like [`draw`](Self::draw), the closure is deferred until after layout,
    /// so it must be `'static`: move owned data (or a `String`-backed
    /// `Paragraph<'static>`) into it.
    ///
    /// ```no_run
    /// use slt::ratatui_compat::ratatui_core::style::Style;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let label = String::from("rendered by ratatui-core");
    /// ui.container().w(30).h(1).ratatui(move |area, buf| {
    ///     buf.set_string(area.x, area.y, &label, Style::new());
    /// });
    /// # });
    /// ```
    pub fn ratatui(self, f: impl FnOnce(RatRect, &mut RatBuffer) + 'static) {
        self.draw(move |buf, rect| render_region(buf, rect, f));
    }

    /// Finalize the builder as a region rendered by one ratatui [`Widget`].
    ///
    /// Shorthand for `.ratatui(move |area, buf| widget.render(area, buf))`.
    ///
    /// ```no_run
    /// // `ratatui-widgets = "0.3"` (or `ratatui = "0.30"`) provides the widgets.
    /// use ratatui_widgets::{block::Block, paragraph::Paragraph};
    /// # slt::run(|ui: &mut slt::Context| {
    /// let count = 3;
    /// ui.container().h(5).ratatui_widget(
    ///     Paragraph::new(format!("count: {count}")).block(Block::bordered()),
    /// );
    /// # });
    /// ```
    pub fn ratatui_widget<W: Widget + 'static>(self, widget: W) {
        self.ratatui(move |area, buf| widget.render(area, buf));
    }
}

/// Run `f` against a scratch ratatui buffer and copy the visible result into
/// `buf` at `rect` (the visible, already-clipped region).
fn render_region(buf: &mut Buffer, rect: Rect, f: impl FnOnce(RatRect, &mut RatBuffer)) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    let (left, top, logical_w, logical_h) = logical_geometry(buf, rect);
    let area = RatRect::new(0, 0, logical_w, logical_h);

    SCRATCH.with(|slot| {
        // The closure cannot reach SLT, so re-entry is not expected; fall back
        // to a fresh buffer rather than panicking if it ever happens.
        let mut fallback = None;
        let mut guard = slot.try_borrow_mut().ok();
        let scratch = match guard.as_deref_mut() {
            Some(slot) => slot.get_or_insert_with(|| RatBuffer::empty(RatRect::ZERO)),
            None => fallback.insert(RatBuffer::empty(RatRect::ZERO)),
        };
        scratch.resize(area);
        scratch.reset();

        prefill(scratch, buf, rect, left, top);
        f(area, scratch);
        copy_back(scratch, buf, rect, left, top);

        if scratch.content.len() > RETAIN_SCRATCH_CELLS
            && let Some(slot) = guard.as_deref_mut()
        {
            *slot = None;
        }
    });
}

/// Resolve the scratch-buffer geometry for a visible `rect`.
///
/// Returns `(left, top, width, height)`: the source offset of the visible part
/// inside the logical region and the logical size handed to ratatui.
fn logical_geometry(buf: &Buffer, rect: Rect) -> (u16, u16, u16, u16) {
    let (left, top) = buf.draw_source_offset();
    // The raw-draw invoker pushes the region's unclipped size onto the Kitty
    // crop stacks; reuse it so the widget sees its full laid-out geometry.
    let original_w = buf
        .kitty_horizontal_clip_stack
        .last()
        .map_or(0, |clip| clip.original_width);
    let original_h = buf
        .current_kitty_clip()
        .map_or(0, |clip| clip.original_height);
    let full_w = original_w.max(left.saturating_add(rect.width));
    let full_h = original_h.max(top.saturating_add(rect.height));

    let fits = |v: u32| u16::try_from(v).ok();
    let cells = u64::from(full_w) * u64::from(full_h);
    if cells <= MAX_SCRATCH_CELLS as u64
        && let (Some(l), Some(t), Some(w), Some(h)) =
            (fits(left), fits(top), fits(full_w), fits(full_h))
    {
        return (l, t, w, h);
    }
    // Pathologically large logical region: render only what is on screen.
    let visible_w = u16::try_from(rect.width).unwrap_or(u16::MAX);
    let visible_h = u16::try_from(rect.height).unwrap_or(u16::MAX);
    (0, 0, visible_w, visible_h)
}

/// Visible width/height actually copied (bounded by the scratch area).
fn copy_extent(scratch: &RatBuffer, rect: Rect, left: u16, top: u16) -> (u16, u16) {
    let w = scratch
        .area
        .width
        .saturating_sub(left)
        .min(u16::try_from(rect.width).unwrap_or(u16::MAX));
    let h = scratch
        .area
        .height
        .saturating_sub(top)
        .min(u16::try_from(rect.height).unwrap_or(u16::MAX));
    (w, h)
}

/// Seed the visible part of the scratch buffer with SLT's current cells.
fn prefill(scratch: &mut RatBuffer, buf: &Buffer, rect: Rect, left: u16, top: u16) {
    let (w, h) = copy_extent(scratch, rect, left, top);
    for dy in 0..h {
        for dx in 0..w {
            let Some(cell) = buf.try_get(rect.x + u32::from(dx), rect.y + u32::from(dy)) else {
                continue;
            };
            if let Some(target) = scratch.cell_mut((left + dx, top + dy)) {
                *target = cell_to_ratatui(cell);
            }
        }
    }
}

/// Copy changed visible scratch cells into SLT's buffer.
fn copy_back(scratch: &RatBuffer, buf: &mut Buffer, rect: Rect, left: u16, top: u16) {
    let (w, h) = copy_extent(scratch, rect, left, top);
    for dy in 0..h {
        let y = rect.y + u32::from(dy);
        let mut dx: u16 = 0;
        // A wide grapheme that starts left of the visible slice hides the
        // first visible column(s). Ratatui does not reset hidden cells, so
        // they may still hold stale prefill content: draw them blank.
        if let Some((hidden, style)) = hidden_by_clipped_wide(scratch, left, top + dy) {
            let hidden = hidden.min(w);
            buf.set_string(rect.x, y, &" ".repeat(usize::from(hidden)), style);
            dx = hidden;
        }
        while dx < w {
            let x = rect.x + u32::from(dx);
            let Some(src) = scratch.cell((left + dx, top + dy)) else {
                break;
            };
            let always_update = match diff_option(src) {
                CellDiffOption::Skip => {
                    dx += 1;
                    continue;
                }
                CellDiffOption::ForcedWidth(forced) => {
                    dx = dx.saturating_add(forced.get());
                    continue;
                }
                CellDiffOption::AlwaysUpdate => true,
                CellDiffOption::None => false,
            };

            let symbol = src.symbol();
            let width = symbol_width(src);
            let unchanged = !always_update
                && buf
                    .try_get(x, y)
                    .is_some_and(|dest| cell_to_ratatui(dest) == *src);
            if !unchanged {
                let style = cell_style_to_slt(src);
                let remaining = w - dx;
                if width > remaining || src.modifier.contains(RatModifier::HIDDEN) {
                    // A wide grapheme cut by the visible edge, or hidden text:
                    // keep the cells' styling but draw blanks.
                    let blanks = " ".repeat(usize::from(width.min(remaining)));
                    buf.set_string(x, y, &blanks, style);
                } else {
                    buf.set_string(x, y, symbol, style);
                }
            }
            // Cells covered by a wide grapheme are hidden, as in ratatui's
            // own diff: skip them instead of overwriting the continuation.
            dx = dx.saturating_add(width);
        }
    }
}

/// Display width of a ratatui cell's symbol, at least one column.
fn symbol_width(cell: &RatCell) -> u16 {
    u16::try_from(UnicodeWidthStr::width(cell.symbol()))
        .unwrap_or(u16::MAX)
        .max(1)
}

/// How many columns starting at `left` on scratch row `y` are covered by a
/// wide grapheme whose leading cell lies left of `left`, plus that
/// grapheme's style. Mirrors the hidden-cell rule of ratatui's buffer diff.
fn hidden_by_clipped_wide(scratch: &RatBuffer, left: u16, y: u16) -> Option<(u16, Style)> {
    let mut covered: u16 = 0;
    let mut style = Style::new();
    for x in 0..left {
        let cell = scratch.cell((x, y))?;
        if covered > 0 {
            covered -= 1;
            continue;
        }
        let width = match diff_option(cell) {
            CellDiffOption::ForcedWidth(forced) => forced.get(),
            _ => symbol_width(cell),
        };
        covered = width - 1;
        style = cell_style_to_slt(cell);
    }
    (covered > 0).then_some((covered, style))
}

/// Read a cell's diff option, honouring the deprecated `skip` flag that older
/// widgets may still set.
#[expect(
    deprecated,
    reason = "Cell::skip is deprecated since ratatui-core 0.30.1 but older widgets still set it"
)]
fn diff_option(cell: &RatCell) -> CellDiffOption {
    if cell.skip {
        CellDiffOption::Skip
    } else {
        cell.diff_option
    }
}

/// Convert an SLT cell into the ratatui cell a widget would see.
///
/// Continuation cells become blanks, matching how ratatui's `set_stringn`
/// resets the cells hidden by a wide grapheme.
fn cell_to_ratatui(cell: &Cell) -> RatCell {
    let mut out = RatCell::EMPTY;
    if !cell.is_continuation() {
        out.set_symbol(cell.symbol.as_str());
    }
    out.fg = cell.style.fg.map_or(RatColor::Reset, RatColor::from);
    out.bg = cell.style.bg.map_or(RatColor::Reset, RatColor::from);
    out.underline_color = cell
        .style
        .underline_color
        .map_or(RatColor::Reset, RatColor::from);
    out.modifier = RatModifier::from(cell.style.modifiers);
    out
}

/// Map a ratatui cell's colours and modifiers to an SLT style. `Reset` maps
/// to `None` (terminal default), which is what an unstyled SLT cell holds.
fn cell_style_to_slt(cell: &RatCell) -> Style {
    let opt = |c: RatColor| (c != RatColor::Reset).then(|| Color::from(c));
    let mut style = Style::new();
    style.fg = opt(cell.fg);
    style.bg = opt(cell.bg);
    style.underline_color = opt(cell.underline_color);
    style.modifiers = Modifiers::from(cell.modifier);
    style
}

impl From<RatColor> for Color {
    fn from(color: RatColor) -> Self {
        match color {
            RatColor::Reset => Color::Reset,
            RatColor::Black => Color::Black,
            RatColor::Red => Color::Red,
            RatColor::Green => Color::Green,
            RatColor::Yellow => Color::Yellow,
            RatColor::Blue => Color::Blue,
            RatColor::Magenta => Color::Magenta,
            RatColor::Cyan => Color::Cyan,
            RatColor::Gray => Color::White,
            RatColor::DarkGray => Color::DarkGray,
            RatColor::LightRed => Color::LightRed,
            RatColor::LightGreen => Color::LightGreen,
            RatColor::LightYellow => Color::LightYellow,
            RatColor::LightBlue => Color::LightBlue,
            RatColor::LightMagenta => Color::LightMagenta,
            RatColor::LightCyan => Color::LightCyan,
            RatColor::White => Color::LightWhite,
            RatColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
            RatColor::Indexed(i) => Color::Indexed(i),
        }
    }
}

impl From<Color> for RatColor {
    fn from(color: Color) -> Self {
        match color {
            Color::Reset => RatColor::Reset,
            Color::Black => RatColor::Black,
            Color::Red => RatColor::Red,
            Color::Green => RatColor::Green,
            Color::Yellow => RatColor::Yellow,
            Color::Blue => RatColor::Blue,
            Color::Magenta => RatColor::Magenta,
            Color::Cyan => RatColor::Cyan,
            Color::White => RatColor::Gray,
            Color::DarkGray => RatColor::DarkGray,
            Color::LightRed => RatColor::LightRed,
            Color::LightGreen => RatColor::LightGreen,
            Color::LightYellow => RatColor::LightYellow,
            Color::LightBlue => RatColor::LightBlue,
            Color::LightMagenta => RatColor::LightMagenta,
            Color::LightCyan => RatColor::LightCyan,
            Color::LightWhite => RatColor::White,
            Color::Rgb(r, g, b) => RatColor::Rgb(r, g, b),
            Color::Indexed(i) => RatColor::Indexed(i),
        }
    }
}

/// Lossless pairs; `SLOW_BLINK`/`RAPID_BLINK`, `HIDDEN` and `OVERLINE` are
/// handled explicitly in the conversions below.
const MODIFIER_PAIRS: [(RatModifier, Modifiers); 6] = [
    (RatModifier::BOLD, Modifiers::BOLD),
    (RatModifier::DIM, Modifiers::DIM),
    (RatModifier::ITALIC, Modifiers::ITALIC),
    (RatModifier::UNDERLINED, Modifiers::UNDERLINE),
    (RatModifier::REVERSED, Modifiers::REVERSED),
    (RatModifier::CROSSED_OUT, Modifiers::STRIKETHROUGH),
];

impl From<RatModifier> for Modifiers {
    /// `SLOW_BLINK` and `RAPID_BLINK` both map to [`Modifiers::BLINK`];
    /// `HIDDEN` has no SLT equivalent and is dropped.
    fn from(modifier: RatModifier) -> Self {
        let mut out = Modifiers::NONE;
        for (rat, slt) in MODIFIER_PAIRS {
            if modifier.contains(rat) {
                out.insert(slt);
            }
        }
        if modifier.intersects(RatModifier::SLOW_BLINK | RatModifier::RAPID_BLINK) {
            out.insert(Modifiers::BLINK);
        }
        out
    }
}

impl From<Modifiers> for RatModifier {
    /// [`Modifiers::BLINK`] maps to `SLOW_BLINK`; [`Modifiers::OVERLINE`] has
    /// no ratatui equivalent and is dropped.
    fn from(modifiers: Modifiers) -> Self {
        let mut out = RatModifier::empty();
        for (rat, slt) in MODIFIER_PAIRS {
            if modifiers.contains(slt) {
                out.insert(rat);
            }
        }
        if modifiers.contains(Modifiers::BLINK) {
            out.insert(RatModifier::SLOW_BLINK);
        }
        out
    }
}

impl From<RatStyle> for Style {
    /// Maps colours and `add_modifier`; `sub_modifier` is dropped because SLT
    /// styles carry no subtractive modifiers.
    fn from(style: RatStyle) -> Self {
        let mut out = Style::new();
        out.fg = style.fg.map(Color::from);
        out.bg = style.bg.map(Color::from);
        out.underline_color = style.underline_color.map(Color::from);
        out.modifiers = Modifiers::from(style.add_modifier);
        out
    }
}

impl From<Style> for RatStyle {
    /// Maps colours and modifiers; SLT's underline style and `OVERLINE` are
    /// dropped.
    fn from(style: Style) -> Self {
        let mut out = RatStyle::new();
        out.fg = style.fg.map(RatColor::from);
        out.bg = style.bg.map(RatColor::from);
        out.underline_color = style.underline_color.map(RatColor::from);
        out.add_modifier = RatModifier::from(style.modifiers);
        out
    }
}
