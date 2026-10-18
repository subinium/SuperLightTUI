# SLT Migration Guide

A version-by-version upgrade guide for SuperLightTUI users. Pre-1.0, minor versions
(`0.X.0`) may contain breaking changes; patch versions (`0.X.Y`) must not.

For the full per-release notes, see `CHANGELOG.md`. For composition recipes that use
current APIs, see `docs/PATTERNS.md` and `docs/COOKBOOK.md`.

---

## v0.25.x → v0.26.0

- **`modal()` traps focus (#462).** `ui.modal(f)` now equals
  `ui.modal_with(ModalOpts::default(), f)`: Tab cannot leave the modal and a
  stale `set_focus_index` pointing at the background is pulled back in. For
  the old behavior use `ui.modal_with(ModalOpts::new().tab_trap(false), f)`.
  `ModalOptions { tab_trap: true }` literals no longer compile (the type is
  now `#[non_exhaustive]` and renamed `ModalOpts`): write `ModalOpts::new()`.
  Replace `modal_at(a, f)` with `modal_with(ModalOpts::new().anchor(a), f)`
  and `modal_at_offset(a, dx, dy, f)` with
  `modal_with(ModalOpts::new().anchor(a).offset(dx, dy), f)`.
- **Group hover (#458).** `group_hover_bg` / `group_hover_border_style` light
  up only the hovered group instance and no longer react to keyboard focus.
  Drop the `format!("row-{i}")` unique names (or switch to `.hover_bg(..)`,
  which needs no group), and add `.focus_within_bg(..)` /
  `.focus_within_border_fg(..)` where the focus highlight is wanted.
- **Disabled widgets (#458).** Replace "looks disabled" `.with_if` styling
  plus handler guards with `ui.disabled(cond, |ui| ..)`. Code that builds
  `Response { .. }` literals must add `disabled` (or use `..Default::default()`).
- **Select dropdown (#459).** The open list no longer pushes content down, so
  snapshot tests of an open select change, and item rows sit one row lower
  than before (the dropdown has its own border, and opens above the trigger
  near the bottom edge). The list is placed from the trigger's previous-frame
  rect, so a select created with `open = true` shows its list from the second
  frame on; render twice in tests that assert on an initially open select. A click outside an open select closes it and is not
  delivered to the widget under the pointer.
- **Accessible mode (#469).** `SLT_ACCESSIBLE=1` now switches
  `RunConfig::default()` (and so `slt::run`) to a line-oriented prompt mode
  for screen readers; nothing changes unless a user sets it. If your app must
  always render visually (for example a full-screen game), pass
  `RunConfig::default().accessible(false)`, which wins over the variable.
  Consider `ui.a11y_label("...")` for inputs without a visible label and
  `ui.is_accessible()` to hide key legends.
- **Shortcuts vs focused text fields (#453).** While `text_input`, `textarea`,
  a form field or an open command palette has focus, `ui.key(c)`,
  `ui.key_code(..)`, `ui.key_mod(..)` and `ui.consume_key*(..)` skip the keys
  it edits with, in any call order. Shortcut checks no longer need to sit after
  the widgets. If your app read typed characters through `ui.key(..)` on
  purpose (for example a vim-style mode that also feeds an input), switch those
  reads to `ui.raw_key(c)` / `ui.raw_key_code(..)`. New `ui.shortcut(code,
  modifiers)` matches modifiers exactly.
- **Focus identity (#454).** Focus now stays on the same `text_input` /
  `textarea` / form field / named slot when focusables appear or disappear
  before it, instead of staying on the same slot number. Keep each widget's
  state alive across frames (create it outside the closure) to benefit;
  custom focusable widgets can opt in with `ui.register_focusable_keyed(id)`.
  `focus_index()` / `set_focus_index()` remain positional. Tests that assumed
  focus stays on slot N after inserting a focusable before it must follow the
  widget instead.
- **`TextInputState::validate` → `validate_now` (#461).** Same behaviour, new
  name; the old one is deprecated. Replace `s.value.clear(); s.cursor = 0;`
  with `s.clear()`, `s.masked = true` with `TextInputState::password()` /
  `.masked(true)`, and positional `form.value(i)` with `form.get("key")`
  (set keys with `FormField::key`, otherwise the label is used).
- **Text selection (#465).** Focused `text_input` / `textarea` now handle
  Ctrl+A, Ctrl+Z / Ctrl+Y and, with a selection, Ctrl+C / Ctrl+X. If your app
  binds those chords globally, they no longer fire while a text field has
  focus; opt out of select-all with `.ctrl_a_select_all(false)`, or check
  them with `ui.raw_key_mod(..)`. To let Ctrl+C copy, run with
  `RunConfig::default().handle_ctrl_c(false)` and quit on your own key.
- Color depth auto-detection now also uses terminal identity, a
  probe-confirmed XTGETTCAP `Tc` reply, and VT-enabled Windows consoles, so
  Kitty, Ghostty, WezTerm, iTerm2, VS Code, and Windows Terminal may get 24-bit
  color where 0.25 picked 16 or 256. `TERM=dumb` now resolves to
  `ColorDepth::NoColor`. Set `RunConfig::color_depth(..)` to pin a depth; an
  explicit depth still wins over everything, including `NO_COLOR` (#447).
- `RunConfig::kitty_keyboard(true)` no longer pushes the protocol on a direct
  terminal that answers startup queries but not `CSI ? u`.
  `ui.capabilities().kitty_keyboard` now reports the probe result instead of
  always `false`. Set `SLT_FORCE_KITTY_KEYBOARD=1` to push regardless (#448).
- `ui.copy_to_clipboard` payloads over 74,994 bytes are no longer sent; raise
  `RunConfig::clipboard_max_bytes(..)` if your terminal accepts more. Write
  failures are reported through `ui.clipboard_status()` instead of
  `eprintln!`, and a failed selection copy no longer aborts the frame (#446).
- The window title from `RunConfig::title` is now written through the frame
  flush and restored on exit (XTWINOPS title stack) instead of staying set
  after the app quits (#468).
- `RunConfig` gained `clipboard_passthrough`, `clipboard_max_bytes`,
  `ambiguous_wide`, `width_conservative`, and `width_probe`. `RunConfig` is
  `#[non_exhaustive]`; keep using the builder methods.
- Inside tmux with `default-terminal "screen-256color"`, SLT now identifies the
  multiplexer as tmux (it was GNU screen). Protocol policy for the two is the
  same; only OSC 52 passthrough wrapping differs.
- `ListState::selected` and `TableState::selected` are deprecated (#455). Reads
  become `selected_view_index()` (or `selected_index()` for the data index);
  writes become `set_selected_view_index(i)` / `set_selected_index(i)`. Replace
  `list.remove_item(list.selected)` and `table.remove_row(table.selected)` with
  `remove_selected()`, and `table.rows()[table.selected]` with
  `selected_row()`; the old forms act on the wrong row under a filter or sort.
- `list` and unpaged `table` scroll themselves (#487) and are at most as tall
  as the space their parent leaves below them. Lists that fit are unchanged.
  If you wrapped a long `ui.list` in a `scrollable` to make it scroll, you can
  drop the wrapper. To keep a footer visible under a long list, put the list
  in `ui.container().grow(1).col(...)` (or give it `.h(n)` / `.max_h(n)`).
- **One press, one target (#439).** A press activates only the topmost,
  innermost interactive slot: overlay content blocks what is drawn under it,
  and a child container wins over its parent. If an outer container relied on
  firing together with an inner widget, switch it to
  `ui.container().bubble_clicks()` (the ancestor also receives descendant
  clicks) or read `Response::clicked_any` (any press inside the rect). Layout
  wrappers count as slots: a plain `ui.row(..)` inside a clickable card now
  takes clicks that land on it, so either add `.bubble_clicks()` to the card or
  check `clicked_any`. `hovered` is unchanged; `hovered_top` is the new
  winner-only variant.
- **Pointer events (#442).** `clicked` still fires on press; opt into
  release-to-click with `RunConfig::default().click_on_release(true)`.
  `Response` has new public fields (`clicked_any`, `hovered_top`, `pressed`,
  `released`, `drag_started`, `dragging`, `drag_delta`, `middle_clicked`,
  `modifiers`): code that builds a `Response { .. }` literal must add
  `..Response::default()`. Several presses in one input batch are no longer
  collapsed into the last one: they are dispatched one per frame, so custom
  `slt::frame` loops must keep rendering while `AppState::has_pending_input()`
  is true (as they already must for queued Tab navigation).
- **Wheel chaining (#440).** A nested scrollable at its end now passes the
  wheel to the enclosing scrollable. Add `.overscroll_contain()` to an inner
  region (popups, pickers) that must keep the wheel. A wheel event that no
  region can move is left unconsumed, so `ui.scroll_up()` / `ui.scroll_down()`
  may now see it where the innermost region used to swallow it.
- **Mouse on by default (#444).** Plain `slt::run` now captures the mouse in
  `MouseMode::Drag` (or `Click` over SSH/mosh). Inline and static runs
  (`run_inline*`, `run_static*`) keep the mouse off unless you pass a mode, so
  their wheel still scrolls the terminal; an inline app that already called
  `.mouse(true)` is unchanged. Apps that relied on the
  terminal's native selection by default should set
  `RunConfig::default().mouse_mode(MouseMode::Off)` (or `.mouse(false)`);
  users can also Shift-drag to select natively on most emulators. Hover
  styling and tooltips need `.mouse_mode(MouseMode::Hover)`; the existing
  `.mouse(true)` still means Hover. Code that read or wrote the
  `RunConfig::mouse` field must use `mouse_mode` (`Option<MouseMode>`) or
  `effective_mouse_mode()`.
- **Tooltip delay (#452).** Tooltips now appear after the pointer rests for
  500 ms. Tests that assert a tooltip on the first hovered frame should
  either advance the clock (`TestBackend::advance_time`) or set
  `RunConfig::default().tooltip_delay(Duration::ZERO)` (with
  `TestBackend::with_run_config`).
- **Scroll anchoring (#443).** `ScrollState::anchor` defaults to `true`, so the
  offset moves when content above the viewport changes height. Apps that do
  their own offset math for that case should set `scroll.anchor = false` (or
  drop the manual correction). Use `ScrollState::new().stick_to_bottom(true)`
  instead of hand-rolled "follow the tail" logic.
- **No more `let _ =` on layout and display calls (#456).** `Response` is no
  longer `#[must_use]` as a type, so `let _ = ui.row(..)` /
  `let _ = ui.separator()` can become plain statements. Nothing breaks:
  existing `let _ =` keeps compiling. A bare `ui.button(..)` (and
  `button_colored` / `button_with`) still warns; check `.clicked` or chain
  `.on_click(ui, ..)`. Crates with `#![deny(unused_must_use)]` only lose
  warnings.
- **Scoped `Text` builder (#457).** `ui.text(..)`, `ui.styled(..)`,
  `ui.link(..)` and `ui.timer_display(..)` return `slt::Text<'_>` instead of
  `&mut Context`. Its chain options edit that one element, so
  `ui.text("a").bold().fg(c);` compiles and renders as before, while a
  modifier can no longer land on the wrong widget. `Text::show()` returns the
  element's `Response` (hover / click; for `link`, `clicked` is the
  activation). The `Context` "modify the last text" modifiers are deprecated
  and keep their 0.25 behaviour for this release:

  | 0.25 | 0.26 |
  |---|---|
  | `ui.text(s).bold().fg(c);` | unchanged |
  | `ui.text(s); ui.bold();` | `ui.text(s).bold();` |
  | `ui.text("a").text("b");` (chaining another Context call) | two statements: `ui.text("a"); ui.text("b");` |
  | `ui.text(s).with_if(c, \|t\| { t.bold(); });` | unchanged; `t` is a `Text` |
  | `fn dim(ui: &mut Context) { ui.dim(); }` used as `.with(dim)` | `fn dim(t: Text<'_>) { t.dim(); }` |
  | `ui.text(s).gradient_stops(&[(0.0f32, c)])` | `.gradient_stops_f64(&[(0.0, c)])` (the `f32` forms were removed in 0.26) |
  | `ui.separator(); ui.fg(c);` | `ui.separator().color(c);` |
  | `ui.with_if(..)` / `ui.with(..)` / `ui.w(..)` / `ui.m(..)` / `ui.grow(..)` / `ui.align(..)` / `ui.wrap()` / `ui.truncate()` / `ui.text_center()` | same names on `Text` |

  Code that kept the `&mut Context` returned by `text()` (for example
  `let ui = ui.text("x");`) should keep using the original `ui`.
- **`TestBackend::run_with_events` keeps focus (#464).** It no longer resets
  focus to slot 0 before each call. A test that pressed Tab in one call and
  typed in the next now types into the newly focused widget (as a user
  would). Tests that relied on the reset should call
  `render_with_events(events, 0, 0, f)`. For new multi-step tests prefer
  `tb.step(events, &mut f).press(KeyCode::Tab, &mut f).type_text("x", &mut f)`,
  which drives one `FnMut` over plain state instead of `RefCell` + `sequence()`.
- **Widget builders, one meaning per suffix (#460).** `button`, `badge`,
  `stat`, `help`, `separator`, `progress`, `progress_bar`, `slider`,
  `line_chart` and `area_chart` return builders (`Button`, `Badge`, `Stat`,
  `Help`, `Separator`, `ProgressBar`, `Slider`, `LineChart`). Options chain
  on the call and compose (`ui.button("Delete").variant(ButtonVariant::Danger)
  .colors(&c)`, impossible before), and each builder dereferences to its
  `Response`, so `if ui.button("x").clicked`, `ui.button("x").on_click(ui, ..)`
  and `ui.badge("x").on_hover(ui, ..)` compile unchanged (`Response` is now
  `Copy`). Call `.show()` where a `Response` *value* is needed:
  `vec![ui.button("a").show(), ..]`, a function or closure that returns it,
  or `let r = ui.button("x").show();` followed by other `ui.` calls before
  `r` is read (the builder borrows `ui` until then; the compiler reports
  E0499 / E0308). Builder arguments cannot borrow `ui` again, so
  `ui.badge("x").color(ui.theme().success)` becomes
  `let c = ui.theme().success; ui.badge("x").color(c);`. `slider` and the
  charts are recorded when their response is first read, on `.show()`, or
  at the end of the statement; the others are recorded immediately. In
  both cases the widget renders exactly where it is written. Rendering of
  every widget without options is unchanged. `_with` now always means "plus a
  closure parameter" and `_colored` always means "plus `&WidgetColors`"; the
  names that meant something else are deprecated (table below) and the
  `scripts/api_audit.sh` checks V8 / V9 keep it that way.
- **Removed items (#463).** Everything deprecated in 0.20, 0.21 and 0.22 is
  gone (table below); each replacement was named in its deprecation note.
  Items deprecated in 0.23 or later (`slider_with_step`, the 0.26 aliases)
  still compile with a warning.
- **`use_state_named` shares the keyed map (#463).** `use_state_named(id,
  init)` / `use_state_named_default(id)` are deprecated aliases of
  `use_state_keyed` / `use_state_keyed_default`, and both names now address
  one slot map: a `named` and a `keyed` call with the same id return the same
  value (before 0.26 they were separate), and `remove_state_keyed`,
  `retain_state_keyed` and `keyed_state_count` include ids created through
  `use_state_named`. Rename colliding ids if you relied on two independent
  slots.
- **`Context::line` returns `Response` (#463),** like
  `ContainerBuilder::line`. `ui.line(..);` statements are unchanged; code that
  chained a Context call after it (`ui.line(..).bold()`) splits into two
  statements.
- **`slt::prelude` (#463).** `use slt::prelude::*;` imports the run entry
  points, `Context`, the builders, the style vocabulary, key types and the
  common widget states, and nothing else. Prefer it over `use slt::*`: the
  root glob also imports SLT's `crossterm` re-export, which shadows your own
  `crossterm` dependency. `slt::crossterm` stays reachable by path.
  `StateHandle<T>` is a new alias of `State<T>`, the hook handle; "state" in
  the docs now means the widget state structs (`ListState`, ...).
- **New on `ContainerBuilder` (#463):** `fg(color)` (same as `text_color`)
  and `w_ratio(num, den)` / `h_ratio(num, den)` (same as `Constraints`).

  Renamed (deprecated alias kept for one release, warns):

  | 0.25 | 0.26 |
  |---|---|
  | `ui.button_with(label, variant)` | `ui.button(label).variant(variant)` |
  | `ui.button_colored(label, &colors)` | `ui.button(label).colors(&colors)` |
  | `ui.slider_with(&mut v, SliderOpts::new(label, range).step(s))` | `ui.slider(label, &mut v, range).step(s)` |
  | `ui.slider_with_step(label, &mut v, range, s)` (deprecated since 0.23) | `ui.slider(label, &mut v, range).step(s)` |
  | `ui.grid_with(&cols, f)` | `ui.grid_columns(&cols, f)` |
  | `ui.separator_colored(c)` | `ui.separator().color(c)` (full intensity, like before) |
  | `ui.badge_colored(label, c)` | `ui.badge(label).color(c)` |
  | `ui.stat_colored(label, value, c)` | `ui.stat(label, value).color(c)` |
  | `ui.progress_bar_colored(r, w, c)` | `ui.progress_bar(r, w).color(c)` |
  | `ui.line_chart_colored(d, w, h, c)` | `ui.line_chart(d, w, h).color(c)` |
  | `ui.area_chart_colored(d, w, h, c)` | `ui.area_chart(d, w, h).color(c)` |
  | `ui.help_colored(b, key, text)` | `ui.help(b).key_color(key).text_color(text)` |
  | `ui.use_state_named(id, init)` | `ui.use_state_keyed(id, init)` (same slot) |
  | `ui.use_state_named_default(id)` | `ui.use_state_keyed_default(id)` (same slot) |

  Removed (deprecated since 0.20-0.22; compile error now):

  | Removed | Use |
  |---|---|
  | `ContainerBuilder::pad(n)` | `.p(n)` |
  | `ContainerBuilder::min_width` / `max_width` / `min_height` / `max_height` | `.min_w` / `.max_w` / `.min_h` / `.max_h` (the `Constraints` getters of the same name stay) |
  | `ui.col_gap(n, f)` / `ui.row_gap(n, f)` | `ui.container().gap(n).col(f)` / `.row(f)` |
  | `FilePickerState::selected()` | `selected_file()` |
  | `ScrollState::progress() -> f32` | `progress_ratio() -> f64` |
  | `FormState::validate(&[FormValidator])` | `FormField::validate(..)` per field + `form.validate_all()` |
  | `ui.key_seq("gg")` | `ui.key_chord("gg")` |
  | `ui.code_block_lang(c, l)` | `ui.code_block(c).lang(l)` |
  | `ui.code_block_numbered(c)` | `ui.code_block(c).numbered()` |
  | `ui.code_block_numbered_lang(c, l)` | `ui.code_block(c).lang(l).numbered()` |
  | `ui.use_state_named_with(id, init)` | `ui.use_state_keyed(id, init)` |
  | `ui.use_memo_ref(&deps, f) -> &T` | `ui.use_memo(&deps, f)` -> `Memo<T>`; read with `.get(ui)` / `.copied(ui)` |
  | `ui.gradient_stops(&[(f32, Color)])` / `ui.bg_gradient_stops(..)` | `ui.text(..).gradient_stops_f64(&[(f64, Color)])` / `.bg_gradient_stops_f64(..)` |
  | `AppState::fps() -> f32` | `fps_f64()` |
  | `Color::luminance` / `blend` / `lighten` / `darken` / `contrast_ratio` / `from_hsl` / `from_hsv` / `rotate_hue` (`f32`) | the `_f64` method of the same name (`blend_f64`, ...) |
  | `Theme::overlay(color, f32)` | `overlay_f64(color, f64)` |

  `<widget>_colored(state, &WidgetColors)` on the stateful widgets
  (`text_input`, `list`, `list_reorderable`, `table`, `tabs`, `paginator`,
  `checkbox`, `toggle`, `select`, `radio`, `color_picker`) is the remaining
  `_colored` shape and is not deprecated. `bar_chart_with`, `histogram_with`,
  `table_with`, `canvas_with`, `error_boundary_with` and `modal_with` keep
  their names.

---

## 1. v0.24.x → v0.25.0

Existing `form_field`, `scrollable`, and native/browser entry points remain
available. Both packages are versioned at `0.25.0`.

- `ScrollState::follow_focus` defaults to `true`. Focus/caret/layout changes now
  reveal the relevant child through its ancestor scrollers. Set it to `false`
  for application-controlled offsets. Idle frames and manual scrolling do not
  continually snap back; editing the focused caret can reveal it again.
- Give each scrollable container its own `ScrollState`. State identity is
  preserved when moved and is independent of surrounding widget order; cloning
  produces an independent binding with copied offsets. Reusing the same mutable
  state for different viewports is not supported.
- Current-layout bounds and reveal corrections are applied before rendering.
  The public offsets are synchronized on the next binding. An explicit app
  offset change made in between takes precedence over the pending correction.
- `form_field_response` returns `FormFieldResponse`: field-level interaction,
  the input's own `Response`, and an optional full `layout_rect`. The old
  chainable `form_field` delegates to it. Labels/errors share the input's single
  focus slot; a viewport too small for the field prioritizes its caret.
- `focused_layout_rect()` and `measured_layout_rect(group_name)` return full
  previous-frame border boxes, before scrolling and clipping. Coordinates are
  root/overlay logical coordinates, not screen or local scroll coordinates.
  They reset on resize; use visible `Response.rect` for mouse hit testing.
- Initial focus is unique. Focus gain/loss follows the slot that actually
  rendered, not a later Tab or programmatic request. Screen navigation preserves
  its local slot without overwriting live Tab/mouse input; the first frame after
  returning uses the restored screen's focus rather than the old screen's count.
- Modal focus remains stable when background slots are suppressed, inactive
  lower modals do not capture input on settled frames, and closing the modal
  restores the background request. As with other previous-frame hit feedback,
  keep modal declaration order stable while interacting with stacked dialogs.
- Plain Tab can still accept input completion. Shift+Tab remains reverse focus
  navigation instead of accepting a completion.
- Input after a consecutive Tab/Shift+Tab group is queued for the following
  frame, so a fast `Tab` plus text/Enter cannot edit or activate the old field.
  Events stay ordered, paste payloads are moved without cloning, and callbacks
  still run once per frame. Custom backend loops must render again while
  `AppState::has_pending_input()` is true before waiting for more external input.
  `TestBackend::has_pending_input()` exposes the same condition in tests.
- `Buffer::cursor_position()` is temporarily `None` when a late focus request
  has not rendered its target yet. IME backends must treat that as an unknown
  caret and suppress preedit, rather than reuse the previous field's privacy.

These behavior changes are shared by the frame kernel, native run loops, and
WASM backend. They do not add new terminal-protocol or physical IME guarantees.

---

## 2. v0.23.x → v0.24.0

Existing native run and widget signatures remain available. Review these
intentional behavior changes:

- Headless `TestBackend` frames now advance `ui.tick()`, matching the real frame
  entry point. Use `advance_time(Duration::ZERO)` to freeze scheduler time and
  `advance_time(duration)` to drive deterministic deadlines without sleeping.
- FPS now measures actual frame cadence rather than the inverse of kernel-only
  processing time. Separate render/flush durations are available on `AppState`.
- Virtual lists use the `ListState` filtered view while callbacks retain raw
  item indices. Do not use the selected view index as a raw data index.
- `State<T>` and `Memo<T>` handle traits no longer require the stored value to
  implement those traits. `State` is still not `Copy`.
- C++ highlighting works with `syntax-cpp` alone without enabling public C
  highlighting. Applications do not need the unrelated grammar bundles.
- Browser integration has a real owned runtime and configuration surface. See
  [WASM.md](WASM.md) for the three-argument options entry point, the incremental
  direct-flush versus fresh-runtime-frame contract, and supported capabilities.
- Recoverable UI/task panics are reported through their boundary/outcome instead
  of using the terminal cleanup panic hook. Uncaught terminal-owner failures
  still restore owned terminal modes. With `panic = "abort"`, recovery is not
  possible; the hook restores all active terminal sessions before termination.
- After native event polling/reading begins, raw terminal queries are disabled
  for the process to preserve queued and partially parsed input.
  Startup probing remains available. A later `read_clipboard` returns `None`;
  size and capability refreshes use ioctl, cached data, or conservative fallbacks.
- Canvas requests have explicit budgets. Use the fallible APIs to handle
  rejected dimensions, layers or labels; `canvas_with` reuses caller-owned
  storage. This is not an unlimited offscreen bitmap API.

The root and companion package versions are synchronized at `0.24.0`; a version
number in a source checkout is not evidence that the registry release is live.

### Native input ownership

The Linux burst stall tracked in [issue #432](https://github.com/subinium/SuperLightTUI/issues/432)
is handled by SLT's owned Unix input source. It opens an independent nonblocking
terminal descriptor, retains readiness across full reads, and keeps partial
parsing in the same owner. No workspace-only dependency patch or renamed
Crossterm types are required. Applications using the built-in `run*` functions
receive the fix; applications supplying events to `frame()` still own their
external event source. Do not mix another reader with an active SLT loop.

Fatal input errors end the native input stream for the process. In particular,
a cursor-query timeout cannot safely correlate a later reply with a retry, so
subsequent native input returns an error rather than silently accepting stale
data. Restart the process after this failure. Normal quit/disposal can be
followed by another session. See [BACKENDS.md](BACKENDS.md) for limits and
resource ownership.

Browser IME preedit is presentation-only until composition commits. Backends
can inspect `Buffer::cursor_is_masked()` to suppress plaintext preview for a
masked input. The flag follows the actual rendered caret and resets with the
buffer; it is not inferred from bullet glyphs or styling.

---

## 3. v0.22.x → v0.23.0

v0.23 is a correctness-focused minor release. It intentionally makes breaking
changes where public mutable fields could invalidate private caches or where a
return type could not expose runtime failure safely.

| Change | Migration |
|---|---|
| Cache-coupled collection fields are encapsulated | `ListState::{items, filter}`, `TableState::{headers, rows, filter}`, `SelectState::items`, `MultiSelectState::items`, and `CommandPaletteState::commands` are now accessed through the same-name getter plus `set_items`, `set_filter`, `set_headers`, `set_rows`, or `set_commands`. |
| Async run ownership is explicit | Keep the returned run handle and use its sender accessor to send messages; await/join the handle to observe terminal errors or panic. |
| Rich-log retention uses bounded storage | Replace direct `entries` access with `entries()`, `entries_mut()`, `entry()`, `entry_mut()`, and `push_entry()`; storage is now a `VecDeque`. |
| `slider_with_step` is deprecated in favor of an opts API | Existing calls still compile. Migrate to `ui.slider_with(&mut value, SliderOpts::new(label, range).step(step))`; the three-argument `ui.slider(label, &mut value, range)` remains available. |
| Public state structs gained private invariant/cache fields | Construct `FileEntry`, `FilePickerState`, and `FormState` through `Default`/`new` and builders instead of external struct literals or functional update syntax. |
| Cell and buffer writes enforce grapheme/control invariants | Use `Buffer::set_string`, `set_char`, and checked cell APIs instead of assigning raw symbol bytes. |
| File-picker scanning is observable | Handle the scan status/error exposed by `FilePickerState`; an empty directory and a failed scan are distinct. |
| Non-TTY and zero-geometry runs are explicit | Handle the returned outcome/error rather than assuming an `Ok(())` means the closure ran. |

The exact method names above are compile-tested in the v0.23 rustdoc and examples.
Run this upgrade sequence:

1. Set `superlighttui = "0.25"` (or `"0.23"` to stop at this step); when going
   straight to current, also apply sections 1 and 2.
2. Run `cargo check` and replace direct state-field mutation first.
3. Update async ownership and slider call sites reported by the compiler.
4. Run interaction tests for tables, selects, text input, and raw drawing.
5. Re-record visual snapshots containing CJK, emoji, RTL text, images, or clipped scroll regions.

---

## 4. v0.21.x → v0.22.x (toolchain + f64 float surface)

v0.22.0 changed no public signatures; the break is the toolchain.

| Change | Migration |
|---|---|
| Edition 2024, MSRV 1.85 → 1.88 (0.22.0) | Build with Rust 1.88 or newer. Your own crate may stay on an older edition. |
| `f32` float APIs deprecated in favor of `*_f64` (0.22.2) | `Color::{luminance, blend, lighten, darken, from_hsl, from_hsv, rotate_hue}` → the same name with `_f64`; `Color::contrast_ratio` → `Color::contrast_ratio_f64`; `Theme::overlay` → `overlay_f64`; `ui.gradient_stops` / `ui.bg_gradient_stops` → `gradient_stops_f64` / `bg_gradient_stops_f64`; `AppState::fps` → `fps_f64`. The `f32` forms still compile with a warning. |

---

## 5. v0.20.x → v0.21.0

v0.21.0 is a breaking hook/API cleanup. Replacements listed below exist in v0.25;
the deprecated forms still compile with a warning.

| Change | Migration |
|---|---|
| `use_memo` returns a `Memo<T>` handle instead of `&T` | `let x = *ui.use_memo(&deps, f);` → `let x = ui.use_memo(&deps, f).copied(ui);` (or `.get(ui)` for `&T`). `use_memo_ref` keeps the old shape (deprecated). |
| `_named` hook family renamed to match `use_state(init)` | `use_state_named::<T>("id")` → `use_state_named_default::<T>("id")`; `use_state_named_with("id", init)` → `use_state_named("id", init)`. |
| `scrollbar` takes `&mut ScrollState`; `separator` / `separator_colored` return `Response` | Pass `&mut scroll`. Drop any `.separator().bold()`-style chains (they were no-ops). Statement-form `ui.separator();` still compiles but triggers the `must_use` warning; write `let _ = ui.separator();`. |
| Status widgets (`badge`, `key_hint`, `stat*`, `empty_state*`) return a populated `Response` | No code change; `hovered` / `right_clicked` / focus fields now carry real state. |
| `code_block` returns a `CodeBlock<'_>` builder (renders on `Drop`) | `code_block_numbered` / `code_block_lang` / `code_block_numbered_lang` → `ui.code_block(code).lang(l).numbered()`. |
| `ScrollState::progress() -> f32` deprecated | `scroll.progress_ratio()` (`f64`). |
| `key_seq` deprecated | `ui.key_chord(seq)` / `ui.key_chord_timeout(seq, ticks)` (matches across frames). |
| `FormState::validate(&[..])` deprecated | Attach `FormField::validate(...)` per field, then call `FormState::validate_all()`. |
| `ui.col_gap(n, f)` / `ui.row_gap(n, f)` deprecated (0.20.1) | `ui.container().gap(n).col(f)` / `.row(f)`. |

---

## 6. v0.19.3 → v0.20.0

v0.20.0 unified the new-widget surface. These names are **removed** (not deprecated):

| Removed | Replacement |
|---|---|
| `gauge_w(r, w)` | `ui.gauge(r).width(w)` |
| `gauge_colored(r, c)` | `ui.gauge(r).color(c)` |
| `line_gauge_with(r, opts)` / `LineGaugeOpts` | `ui.line_gauge(r).label(..).width(..)` (`LineGauge<'_>` builder) |
| `breadcrumb_sep(b, s)` / `breadcrumb_with(b, s)` | `ui.breadcrumb(b).separator(s)` |
| `breadcrumb_response(b)` | `ui.breadcrumb(b).show()` → `BreadcrumbResponse` (`clicked_segment`) |
| `HighlightRange::single(i)` | `HighlightRange::line(i)` |
| `label_owned(s)` | `label(s)` (accepts `impl Into<String>`) |

Other breaking changes:

| Change | Migration |
|---|---|
| `gauge` / `line_gauge` / `breadcrumb` are consuming builders (render on `Drop`) | `ui.gauge(0.42).label("CPU");` as a statement; call `.show()` to capture the response. |
| `scrollable_with_gutter(state, opts, body)` takes `GutterOpts<G>` | `GutterOpts::line_numbers(total, viewport)` for line numbers, `GutterOpts::new(total, viewport, gutter_fn)` for custom labels. |
| `Constraints` fields replaced by `WidthSpec` / `HeightSpec` | Struct literals → builders (`Constraints::default().min_w(10).max_w(40)` or `.w_minmax(10, 40)`); field reads → accessors (`c.min_width()`); writes → setters (`c.set_min_width(Some(v))`). Persisted serde `Constraints` from v0.19 does not deserialize. |
| Ratio APIs widened `f32` → `f64` (`gauge`, `line_gauge`, `SplitPaneState`, responses) | Remove `as f32` casts; use `f64`. |
| `State<T>` is no longer `Copy` | Call `.clone()` where a handle was implicitly copied. |
| `spinner` / `progress` / `progress_bar` / `progress_bar_colored` return `Response` | Use `let _ = ui.progress(0.5);`; split former `.fg(...)` chains into separate statements. |
| Built-in widgets derive padding/gap from `theme.spacing` | Default spacing is unchanged; custom `Spacing` values now take effect. |
| `FilePickerState::selected()` deprecated | `selected_file()`. |
| `Cargo.toml` uses `autoexamples = false` | Only listed `[[example]]` targets run; see [EXAMPLES.md](EXAMPLES.md). |

The long container names from section 7 (`pad`, `min_width`, …) were deprecated in
v0.20.0 and still compile with a warning in v0.25.

---

## 7. v0.19.x → v0.19.3 (deprecations only — no code break)

These compiled fine in v0.19.x and now produce deprecation warnings
(`#[deprecated(since = "0.20.0")]`). They are still present as aliases in v0.25
and will be removed in a future minor release:

| Old | New |
|---|---|
| `pad(n)` | `p(n)` |
| `min_width(n)` | `min_w(n)` |
| `max_width(n)` | `max_w(n)` |
| `min_height(n)` | `min_h(n)` |
| `max_height(n)` | `max_h(n)` |

Source-of-truth: the `#[deprecated(since = "0.20.0", ...)]` attributes on
`ContainerBuilder::{pad, min_width, max_width, min_height, max_height}` in
`src/context/container.rs`.

### Codemod (sed-based)

```bash
# Run from your project root. Backs up files before modifying.
find src/ -name "*.rs" -exec sed -i.bak -E '
  s/\.pad\(/\.p(/g;
  s/\.min_width\(/\.min_w(/g;
  s/\.max_width\(/\.max_w(/g;
  s/\.min_height\(/\.min_h(/g;
  s/\.max_height\(/\.max_h(/g;
' {} \;
```

**Warning**: this is a regex codemod, not AST-aware. False positives possible if your
project has unrelated `.pad()` / `.min_width()` etc. methods on non-`ContainerBuilder`
types (e.g. a custom `MyRect::pad`). Review the diff before committing:

```bash
git diff
# Looks good? Drop the backups:
find src/ -name "*.rs.bak" -delete
```

For comparison, React migrations use `npx jscodeshift` (AST-aware). The Rust ecosystem
doesn't have an equivalent for SLT yet — see section 10.

---

## 8. v0.18.x → v0.19.0 (additive, no break)

v0.19.0 added component DX APIs in `src/context/runtime.rs` (`provide`, `use_context`,
`use_state_named`, `with_if`). All existing code keeps compiling. Adopt incrementally:

| Old pattern | New pattern (v0.19.0+) |
|---|---|
| Threading `&theme, &tick, &mut state` parameters through every render fn | `ui.provide(value, \|ui\| ...)` once, then `ui.use_context::<T>()` in nested fns |
| `use_state` inside `if`/`match` (panics on hook-order mismatch) | `use_state_named(id)` (id-keyed, safe in conditionals) |
| `if cond { t.bold(); t.fg(Color::Red); }` | `t.with_if(cond, \|t\| { t.bold().fg(Color::Red); })` |

See `docs/PATTERNS.md` for the full set of composition patterns.

---

## 9. v0.17.x → v0.18.x

Major architecture refactor — most user-facing APIs unchanged but internal modules split:

- `src/context.rs` → facade (was monolithic)
- `src/widgets.rs` → facade
- `src/layout.rs` → facade

If you had `pub(crate)` references to specific module paths from inside the SLT crate
(forks, or downstream crates that depended on undocumented internals), they may have
moved. Most external users see no break.

Key APIs added in v0.18:

- `NO_COLOR` env var support — automatically disables ANSI colors when set
- `scroll_col` helper — vertical scroll for column layouts
- `draw_with(rect, |buf| { ... })` — escape hatch for raw buffer access
- `try_get(rect, x, y) -> Option<&Cell>` — bounds-safe cell access in raw-draw

v0.18.1 added doc improvements and safety hardening; v0.18.2 added flush coalescing,
the `Command` box pattern, and a flexbox scratch buffer (perf only).

---

## 10. Comparison: how other UI libraries handle migrations

| Framework | Tooling | Convention |
|---|---|---|
| **React** | `npx jscodeshift` AST codemods + RFC | Major every 2–3 years; deprecation warnings ≥1 minor before removal |
| **Vue** | `vue-codemod` AST tool + composition API guide | Major: 3.x → vue-next migration build |
| **Angular** | `ng update` schematic CLI (auto-migration) | Strict semver, deprecation warnings, automatic migration |
| **Flutter** | `dart fix --apply` | Annotated deprecations migrate via `dart fix` |
| **SLT** | sed-based codemods (no AST tool yet) + this guide + deprecation warnings | Patch releases stay compatible; callable APIs warn ≥1 minor before removal, while invariant-sensitive field encapsulation is documented in minor migrations |

SLT does **not** yet have an AST-aware codemod tool. PRs welcome — `cargo-slt-migrate`
would be a high-impact contribution. Until then, the sed snippets in section 7 plus
`cargo build` errors are the migration toolchain.

---

## 11. Pre-1.0 versioning policy

- **Pre-1.0 (current)**: minor versions (`0.X.0`) MAY contain breaking changes; patch
  versions (`0.X.Y`) MUST NOT.
- **Post-1.0 (TBD)**: standard semver — major for breaking, minor for features, patch
  for fixes.
- CI runs `cargo-semver-checks` on every PR to verify patch releases stay non-breaking.
  Minor releases are allowed to fail this check; major/minor breakage is documented in
  the release notes and reflected in this file.

Deprecation policy: callable methods and named types are deprecated for at least one
minor release before removal, with a concrete replacement in the warning. Before 1.0,
a minor release may also encapsulate public fields whose direct mutation can violate
cache or selection invariants; those changes must have explicit migration notes and
replacement accessors in this guide. Patch releases never introduce either form of
breakage, so always read the migration section before a `0.X.0` upgrade.

---

## 12. Reporting migration issues

If you find a migration step that doesn't work, errors out, or is missing from this
guide:

- File an issue: <https://github.com/subinium/SuperLightTUI/issues>
- Tag with `migration` label
- Include:
  - SLT version you're upgrading **from** and **to**
  - Minimal code snippet that reproduces the failure
  - Full error message (compile error, panic, or visual diff screenshot)

PRs that add migration entries here are welcome — keep entries to the same format as
the tables above.

---

## 13. Cross-references

- `CHANGELOG.md` — full per-release notes with every commit
- `docs/COOKBOOK.md` — recipes using current APIs
- `docs/PATTERNS.md` — composition patterns (`provide` / `use_context` / `with_if`)
- `docs/COMPLETE_REFERENCE.md` — full API surface
- `docs/STATE_APIS.md` — state-type method reference
