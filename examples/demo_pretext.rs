/// Pretext-style demo: text reflows around the mouse cursor trail in real time.
///
/// Inspired by <https://github.com/chenglou/pretext> — demonstrates how fast
/// text relayout enables interactive, mouse-reactive typography in the terminal.
///
/// Move the mouse to create a caterpillar-shaped exclusion zone that text
/// flows around. The trail follows the cursor with smooth interpolation and
/// each segment pushes words aside independently.
use std::collections::VecDeque;
use std::time::Duration;

use slt::{Buffer, Color, Context, KeyCode, Rect, RunConfig, Style};
use unicode_width::UnicodeWidthChar;

const SAMPLE_TEXT: &str = "\
SuperLightTUI is an immediate-mode terminal UI library for Rust. It ships \
over 50 widgets out of the box and renders at 60 fps with a double-buffered \
diff engine that only flushes changed cells to the terminal. The layout \
system is a full flexbox implementation: rows, columns, grow, shrink, gap, \
padding, margin, justify, align — everything you expect from CSS flexbox, \
but running in your terminal. Widgets include text, buttons, checkboxes, \
toggles, radio groups, select dropdowns, multi-select, sliders, spinners, \
progress bars, text inputs, textareas, tables, tabs, lists, trees, grids, \
calendars, file pickers, modals, tooltips, breadcrumbs, badges, alerts, \
stats, code blocks, empty states, command palettes, virtual lists, and \
scrollable containers. For data visualization there are line charts, scatter \
plots, bar charts, stacked bar charts, sparklines, heatmaps, heatmap HD \
with half-block rendering, treemaps with squarified layout, candlestick \
charts, candlestick HD, and a braille-resolution canvas for freeform pixel \
drawing. Animations are first-class: Tween for eased transitions, Spring \
for physics-based motion, Keyframes for multi-step sequences, and Stagger \
for orchestrating groups. The theming system provides named presets — \
Catppuccin, Tokyo Night, Dracula, Gruvbox, Nord, Solarized, and more — \
plus a ThemeBuilder for custom palettes. Rich text supports per-character \
styling with gradients, bold, italic, underline, strikethrough, dim, and \
RGB colors. Images can be rendered via the Kitty graphics protocol or Sixel \
protocol, with automatic fallback to half-block approximation. Syntax \
highlighting is powered by tree-sitter with language-aware token coloring. \
The event system handles keyboard input with modifier detection, mouse \
clicks, mouse movement, mouse drag, and scroll events. Focus management \
supports tab navigation, focus groups, and programmatic focus control. \
State management uses hooks inspired by React: use_state for local state, \
use_effect for side effects. The buffer system uses CompactString for \
zero-allocation single-character cells and tracks style deltas to minimize \
ANSI escape sequence output. Synchronized output prevents flicker by \
batching all writes between BeginSynchronizedUpdate and \
EndSynchronizedUpdate markers. The architecture follows Rust 2018 module \
conventions with filename.rs plus filename/ directory patterns. Context is \
the central hub — your closure receives a mutable Context reference each \
frame and calls methods like ui.text(), ui.button(), ui.row(), ui.col() to \
build the UI tree. Commands are recorded during the closure, then \
build_tree() assembles them into LayoutNodes, compute() runs flexbox, \
collect_all() does a single DFS pass replacing seven separate traversals, \
render() writes cells to the buffer, and flush() diffs against the previous \
frame. Building a TUI app with SuperLightTUI is as simple as calling \
slt::run with a closure. No boilerplate, no trait implementations, no \
manual event loop setup. Just describe what you want to see and the library \
handles the rest. The container system supports nested layouts with \
ContainerBuilder providing methods for width, height, padding, margin, \
border style, background color, and grow factor. Containers can be rows or \
columns, and they nest arbitrarily deep. The border system offers six \
styles: None, Single, Double, Rounded, Thick, and Custom with user-defined \
corner and edge characters. Scrollable containers automatically handle \
overflow with configurable scrollbar appearance and mouse wheel support. \
Tables support sortable columns, row selection, striped rows, and custom \
cell rendering. The list widget provides single and multi-selection modes \
with keyboard navigation and optional filtering. File picker integrates \
with the filesystem for directory browsing with preview support. Calendar \
widget renders a monthly view with date selection and event markers. The \
command palette provides fuzzy-matched command execution with keyboard \
shortcuts display, inspired by VS Code. Virtual list efficiently renders \
thousands of items by only materializing visible rows, enabling smooth \
scrolling through massive datasets. Toast notifications appear briefly at \
configurable screen positions with auto-dismiss timers. Modal dialogs dim \
the background and trap focus within the overlay. Tooltips appear on hover \
with configurable delay and positioning. The chart system supports multiple \
datasets per chart with configurable colors, markers, and axis labels. \
Axis formatting is customizable with label callbacks for domain-specific \
display. Grid lines can be toggled independently for x and y axes. Braille \
rendering achieves 2x4 sub-cell resolution for smooth line and scatter \
plots. Bar charts support horizontal and vertical orientations with grouped \
and stacked variants. Sparklines render inline mini-charts within a single \
row of text. The heatmap widget maps 2D data to color intensity with \
configurable color ramps. Treemap uses the squarified algorithm to pack \
rectangular areas proportional to their values, with contrast-aware label \
positioning. Candlestick charts display OHLC financial data with \
configurable up/down colors and half-block precision for smoother wicks. \
The canvas widget exposes a braille-resolution pixel buffer with methods \
for points, lines, circles, rectangles, and text overlay. QR code \
generation is built in — pass a string and get a scannable code rendered \
in half-block characters. The animation system interpolates any numeric \
value over time. Tween supports 30 easing functions including linear, \
ease-in, ease-out, ease-in-out, bounce, elastic, back, and cubic-bezier. \
Spring simulation uses configurable stiffness, damping, and mass for \
natural motion. Keyframes define multi-stop animations with per-stop \
easing, pre-sorted at build time for zero per-frame overhead. Stagger \
orchestrates groups of animations with configurable delay between each \
element, perfect for list entrance effects. All animations integrate with \
the frame clock — no manual time tracking needed. The style system supports \
foreground color, background color, and eight modifiers: bold, dim, italic, \
underline, reverse, strikethrough, hidden, and rapid blink. Colors can be \
named (16 ANSI), indexed (256 palette), or RGB (16 million true colors). \
Color blending functions enable smooth gradients between any two colors. \
The theme system centralizes color decisions: primary, secondary, surface, \
background, text, error, warning, success, and info slots. Twelve built-in \
themes ship with the library, and ThemeBuilder allows creating custom \
themes with a fluent API. Theme switching is instant — change the theme \
and every widget picks up the new colors on the next frame. The terminal \
backend abstracts over platform differences. Full-screen mode uses the \
alternate screen buffer for clean enter/exit. Inline mode renders below \
the cursor for embedding TUI elements in CLI output. Both modes support \
synchronized output to eliminate flicker on modern terminals. Mouse capture \
enables click, drag, move, and scroll wheel detection with pixel-level \
coordinates on supported terminals. The test infrastructure provides \
TestBackend for headless rendering — create a virtual terminal of any size, \
run frames, and assert on the rendered buffer contents with methods like \
assert_contains, assert_at, and cell inspection. EventBuilder constructs \
synthetic input events for testing interactive flows without a real \
terminal. This demo itself demonstrates the raw power of the draw API. \
Every frame, the entire text corpus is reflowed around an exclusion zone \
centered on the mouse cursor. Word widths are computed once at the start \
of the frame — one cell per ASCII character, two per fullwidth CJK glyph — \
and then the layout phase is pure integer arithmetic: advance the cursor, \
check intersection with the exclusion circle, skip or place each word. \
No measurement is repeated. No layout tree is rebuilt. Just a flat loop \
over cached widths, exactly as Cheng Lou's pretext library does on the \
web. The result: text that flows like water around your cursor at a full \
sixty frames per second, with zero perceptible latency. This is what \
happens when layout is fast enough to be interactive. Typography becomes \
fluid. Text becomes alive. The terminal transforms from a static grid \
into a dynamic, responsive canvas. SuperLightTUI makes this possible with \
zero dependencies on ncurses, zero unsafe code in the widget layer, and a \
single cargo add superlighttui to get started. The minimum supported Rust \
version is 1.81. The library compiles to WebAssembly via the slt-wasm \
crate, bringing terminal UIs to the browser. Feature flags control optional \
functionality: async for tokio integration, serde for serialization. The \
test suite runs over 250 tests across widgets, layout, animation, and \
rendering. Move your mouse and watch the words part around it like a school \
of fish evading a predator. Then watch them settle back into perfect \
typographic order as you move away. This is SuperLightTUI.";

/// Number of trail segments (caterpillar body length).
const TRAIL_LEN: usize = 20;

/// Radius of the head segment.
const HEAD_RADIUS: f64 = 6.0;

/// Minimum distance between trail points before recording a new one.
const MIN_TRAIL_DIST: f64 = 1.5;

/// Smoothing factor for the cursor position.
const SMOOTH: f64 = 0.05;

fn main() {
    let mut smooth_mx: f64 = -100.0;
    let mut smooth_my: f64 = -100.0;
    let mut first_mouse = true;
    let mut trail: VecDeque<(f64, f64)> = VecDeque::with_capacity(TRAIL_LEN + 1);

    let _ = slt::run_with(
        RunConfig::default()
            .mouse(true)
            .tick_rate(Duration::from_millis(16))
            .max_fps(60),
        move |ui: &mut Context| {
            if ui.key('q') || ui.key_code(KeyCode::Esc) {
                ui.quit();
                return;
            }

            // Smooth mouse tracking
            if let Some((mx, my)) = ui.mouse_pos() {
                let mx = mx as f64;
                let my = my as f64;
                if first_mouse {
                    smooth_mx = mx;
                    smooth_my = my;
                    first_mouse = false;
                } else {
                    smooth_mx += (mx - smooth_mx) * (1.0 - SMOOTH);
                    smooth_my += (my - smooth_my) * (1.0 - SMOOTH);
                }
            }

            // Record trail points with minimum distance threshold
            if let Some(&(last_x, last_y)) = trail.back() {
                let dx = smooth_mx - last_x;
                let dy = smooth_my - last_y;
                if dx * dx + dy * dy >= MIN_TRAIL_DIST * MIN_TRAIL_DIST {
                    trail.push_back((smooth_mx, smooth_my));
                    if trail.len() > TRAIL_LEN {
                        trail.pop_front();
                    }
                }
            } else {
                trail.push_back((smooth_mx, smooth_my));
            }

            let tick = ui.tick();
            // Copy trail for the 'static closure
            let trail_snap: Vec<(f64, f64)> = trail.iter().copied().collect();

            ui.container()
                .grow(1)
                .draw(move |buf: &mut Buffer, rect: Rect| {
                    let words: Vec<&str> = SAMPLE_TEXT.split_whitespace().collect();
                    let word_widths: Vec<u32> = words
                        .iter()
                        .map(|w| {
                            w.chars()
                                .map(|c| UnicodeWidthChar::width(c).unwrap_or(1) as u32)
                                .sum()
                        })
                        .collect();

                    let area_x = rect.x + 1;
                    let area_y = rect.y + 1;
                    let area_w = rect.width.saturating_sub(2);
                    let area_h = rect.height.saturating_sub(2);

                    if area_w < 10 || area_h < 3 {
                        return;
                    }

                    // Draw subtle border
                    let border_style = Style::new().fg(Color::Indexed(238));
                    for x in rect.x..rect.right() {
                        buf.set_char(x, rect.y, '─', border_style);
                        buf.set_char(x, rect.bottom().saturating_sub(1), '─', border_style);
                    }
                    for y in rect.y..rect.bottom() {
                        buf.set_char(rect.x, y, '│', border_style);
                        buf.set_char(rect.right().saturating_sub(1), y, '│', border_style);
                    }
                    buf.set_char(rect.x, rect.y, '╭', border_style);
                    buf.set_char(rect.right().saturating_sub(1), rect.y, '╮', border_style);
                    buf.set_char(rect.x, rect.bottom().saturating_sub(1), '╰', border_style);
                    buf.set_char(
                        rect.right().saturating_sub(1),
                        rect.bottom().saturating_sub(1),
                        '╯',
                        border_style,
                    );

                    // Title
                    let title = " ✦ pretext reflow ";
                    let tx = area_x + (area_w.saturating_sub(title.len() as u32)) / 2;
                    buf.set_string(
                        tx,
                        rect.y,
                        title,
                        Style::new().fg(Color::Rgb(180, 140, 255)),
                    );

                    // Help text at bottom
                    let help = " q: quit │ move mouse to push text ";
                    let hx = area_x + (area_w.saturating_sub(help.len() as u32)) / 2;
                    buf.set_string(
                        hx,
                        rect.bottom().saturating_sub(1),
                        help,
                        Style::new().fg(Color::Indexed(243)),
                    );

                    let trail_len = trail_snap.len();

                    // Exclusion test: is (px, py) inside any trail segment?
                    // Each segment has a radius that shrinks toward the tail.
                    let is_excluded = |px: f64, py: f64| -> bool {
                        for (i, &(tx, ty)) in trail_snap.iter().enumerate() {
                            // Tail segments are smaller — linear falloff
                            let age = (trail_len - 1 - i) as f64 / TRAIL_LEN as f64;
                            let r = HEAD_RADIUS * (1.0 - age * 0.6);
                            let r_sq = r * r;
                            let dist_sq = (px - tx) * (px - tx) + (py - ty) * (py - ty) * 4.0;
                            if dist_sq < r_sq {
                                return true;
                            }
                        }
                        false
                    };

                    // Glow intensity at a point (0.0 = outside, up to 1.0 = center)
                    let glow_at = |px: f64, py: f64| -> (f64, f64) {
                        let mut best_t = 1.0_f64;
                        let mut best_age = 1.0_f64;
                        for (i, &(tx, ty)) in trail_snap.iter().enumerate() {
                            let age = (trail_len - 1 - i) as f64 / TRAIL_LEN as f64;
                            let r = HEAD_RADIUS * (1.0 - age * 0.6);
                            let r_sq = r * r;
                            let dist_sq = (px - tx) * (px - tx) + (py - ty) * (py - ty) * 4.0;
                            if dist_sq < r_sq {
                                let t = (dist_sq / r_sq).sqrt();
                                if t < best_t {
                                    best_t = t;
                                    best_age = age;
                                }
                            }
                        }
                        (best_t, best_age)
                    };

                    // Reflow text around the caterpillar trail
                    let mut word_idx = 0;
                    let mut cy = area_y;
                    let total_words = words.len();

                    while cy < area_y + area_h {
                        let mut cx = area_x;
                        let row_end = area_x + area_w;

                        if word_idx >= total_words {
                            word_idx = 0;
                        }

                        let row_start_word = word_idx;
                        let mut placed_any = false;

                        while cx < row_end {
                            let wi = word_idx % total_words;
                            let ww = word_widths[wi];
                            let space_needed = if cx == area_x { ww } else { ww + 1 };

                            // Does this word span intersect any trail segment?
                            let end_x = cx + space_needed;
                            let mut blocked = false;
                            for check_x in cx..end_x.min(row_end) {
                                if is_excluded(check_x as f64, cy as f64) {
                                    blocked = true;
                                    break;
                                }
                            }

                            if blocked {
                                // Skip past the excluded region on this row
                                let mut skip_x = cx + 1;
                                while skip_x < row_end {
                                    if !is_excluded(skip_x as f64, cy as f64) {
                                        break;
                                    }
                                    skip_x += 1;
                                }

                                if skip_x >= row_end || (row_end - skip_x) < ww {
                                    break;
                                }
                                cx = skip_x;
                                continue;
                            }

                            if cx + space_needed > row_end {
                                break;
                            }

                            if cx > area_x {
                                cx += 1;
                            }

                            // Color: subtle gradient based on word position
                            let progress = wi as f64 / total_words as f64;
                            let wave =
                                ((progress * 6.0 + tick as f64 * 0.02).sin() * 0.5 + 0.5) as f32;
                            let r = (140.0 + wave * 80.0) as u8;
                            let g = (160.0 + wave * 60.0) as u8;
                            let b = (200.0 + wave * 55.0) as u8;
                            let word_style = Style::new().fg(Color::Rgb(r, g, b));

                            let mut wx = cx;
                            for ch in words[wi].chars() {
                                let cw = UnicodeWidthChar::width(ch).unwrap_or(1) as u32;
                                if wx + cw <= row_end {
                                    buf.set_char(wx, cy, ch, word_style);
                                }
                                wx += cw;
                            }

                            cx = wx;
                            word_idx += 1;
                            placed_any = true;
                        }

                        if !placed_any && word_idx == row_start_word {
                            word_idx += 1;
                        }

                        cy += 1;
                    }

                    // Draw the caterpillar glow overlay
                    for dy in 0..area_h {
                        for dx in 0..area_w {
                            let px = (area_x + dx) as f64;
                            let py = (area_y + dy) as f64;
                            let (t, age) = glow_at(px, py);
                            if t < 1.0 {
                                let brightness = ((1.0 - t) * 40.0) as u8;
                                // Color shifts from purple (head) to blue (tail)
                                let head_mix = 1.0 - age;
                                let r_c = (100.0 + brightness as f64 * 2.0 * head_mix) as u8;
                                let g_c = (60.0 + brightness as f64 * head_mix * 0.5) as u8;
                                let b_c = (140.0 + brightness as f64 * (1.0 + age)) as u8;
                                let ch = if t < 0.3 {
                                    ' '
                                } else if t < 0.6 {
                                    '·'
                                } else {
                                    '∙'
                                };
                                buf.set_char(
                                    area_x + dx,
                                    area_y + dy,
                                    ch,
                                    Style::new().fg(Color::Rgb(r_c, g_c, b_c)),
                                );
                            }
                        }
                    }

                    // Draw trail segment centers (caterpillar spine)
                    for (i, &(tx, ty)) in trail_snap.iter().enumerate() {
                        let sx = tx.round() as u32;
                        let sy = ty.round() as u32;
                        if sx >= area_x
                            && sx < area_x + area_w
                            && sy >= area_y
                            && sy < area_y + area_h
                        {
                            let age = (trail_len - 1 - i) as f64 / TRAIL_LEN as f64;
                            let brightness = (255.0 * (1.0 - age * 0.7)) as u8;
                            let ch = if i == trail_len - 1 { '◉' } else { '○' };
                            buf.set_char(
                                sx,
                                sy,
                                ch,
                                Style::new().fg(Color::Rgb(
                                    brightness,
                                    (brightness as f64 * 0.7) as u8,
                                    255,
                                )),
                            );
                        }
                    }
                });
        },
    );
}
