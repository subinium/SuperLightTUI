# SLT DX Strategy — Work Plan

> **Goal**: Fix trust issues + build code-level DX features to achieve Tailwind-level developer experience
> **Constraints**: Solo developer, README only, zero external dependencies, low maintenance
> **Philosophy**: Zero Surprise, Full Visibility

---

## Hard Boundaries (MUST NOT violate)

1. **MUST NOT add external dependencies** — No `log`, `tracing`, `env_logger`. Use `std::fs`, `eprintln!`, `#[cfg(debug_assertions)]`
2. **MUST NOT change existing public API signatures** — Only ADD new methods/fields. `key()` stays `&self`, `key_code()` stays `&self`
3. **MUST NOT affect release build performance** — All debug features behind `#[cfg(debug_assertions)]` or env var (one-time startup check)
4. **MUST NOT change widget consumption internals** — Existing `consumed_indices` pattern in 12+ widgets is correct
5. **MUST NOT change `RunConfig::default()` behavior** — Existing apps must work identically
6. **MUST NOT add `#[must_use]` to `Response`** — Would warn on every `col()`/`row()` call (too noisy)

## Quality Gate (MANDATORY before every commit)

```bash
cargo fmt -- --check
cargo check --all-features
cargo clippy --all-features -- -D warnings
cargo test --all-features
cargo check --examples --all-features
```

---

## Wave 1: Trust (Zero Surprise) — Sequential, high-risk

### Task 1.1: error_boundary panic hook conflict fix
- **Category**: `deep`
- **Skills**: `['slt', 'systematic-debugging']`
- **Files**: `src/context.rs`, `src/lib.rs`
- **Depends on**: Nothing

**Problem**: Panic hook fires BEFORE `catch_unwind` catches, calling `disable_raw_mode()` + `LeaveAlternateScreen`. Terminal is destroyed but app keeps running.

**Design Decision**: Use **re-enter raw mode approach** (NOT thread-local guard). After `catch_unwind` returns `Err`, re-enter raw mode and alternate screen if we're in a real terminal context. This is more robust than suppressing the hook because:
- Works with third-party hooks (color-eyre, human-panic)
- Handles nesting naturally
- Doesn't modify panic hook behavior at all

**Implementation**:
1. Add `pub(crate) is_real_terminal: bool` field to Context (set by `run_with()`, false for TestBackend)
2. In `error_boundary_with()`, after `catch_unwind` returns `Err`:
   ```rust
   if self.is_real_terminal {
       let _ = crossterm::terminal::enable_raw_mode();
       let _ = crossterm::execute!(
           std::io::stdout(),
           crossterm::terminal::EnterAlternateScreen
       );
   }
   ```
3. This undoes the panic hook's cleanup, restoring terminal to working state

**Acceptance Criteria**:
- [x] After panic inside `error_boundary`, terminal remains in raw mode and alternate screen
- [x] Panic hook still fires correctly for panics OUTSIDE `error_boundary`
- [x] Nested `error_boundary` (boundary inside boundary) works correctly
- [x] TestBackend tests pass without regression
- [ ] Manual tmux smoke test: `examples/error_boundary_demo.rs` shows fallback UI, app continues

**QA**: Create `examples/error_boundary_demo.rs` that panics inside a boundary and renders a fallback. Run in tmux, verify terminal stays functional.

---

### Task 1.2: error_boundary rollback scope expansion
- **Category**: `deep`
- **Skills**: `['slt', 'systematic-debugging']`
- **Files**: `src/context.rs`
- **Depends on**: Task 1.1

**Problem**: Only `commands` and `last_text_idx` are restored. 10 other mutable fields are left corrupted.

**Implementation**:
1. Create `struct ContextSnapshot` capturing all rollback-needed fields:
   ```rust
   struct ContextSnapshot {
       cmd_count: usize,
       last_text_idx: Option<usize>,
       focus_count: usize,
       interaction_count: usize,
       scroll_count: usize,
       group_count: usize,
       group_stack_len: usize,
       overlay_depth: usize,
       modal_active: bool,
       hook_cursor: usize,
       hook_states_len: usize,
       dark_mode: bool,
       deferred_draws_len: usize,
   }
   ```
2. `ContextSnapshot::capture(&Context) -> Self`
3. `ContextSnapshot::restore(&self, &mut Context)` — truncates `commands`, `group_stack`, `hook_states`, `deferred_draws` and resets all counters
4. Add comment linking Context struct to ContextSnapshot: "// NOTE: If you add a mutable field here, also add it to ContextSnapshot"

**Acceptance Criteria** (one test per field):
- [x] `focus_count` restored after panic inside boundary that called `register_focusable()`
- [x] `interaction_count` restored after panic inside boundary with clickable containers
- [x] `hook_cursor` + `hook_states` restored after panic inside boundary that called `use_state()`
- [x] `group_stack` restored after panic inside `group()` within boundary
- [x] `overlay_depth` restored after panic inside `overlay()` within boundary
- [x] `modal_active` restored after panic inside `modal()` within boundary
- [x] Next frame after panic renders correctly (Tab cycling works, hooks access correct state)
- [x] Nested error_boundary: inner panic doesn't corrupt outer boundary's state

**QA**: `cargo test` — each field has a dedicated TestBackend test.

---

### Task 1.3: dark_mode / Theme linkage
- **Category**: `quick`
- **Skills**: `['slt']`
- **Files**: `src/style.rs` (or `src/style/theme.rs`), `src/context.rs`
- **Depends on**: Nothing (parallel with 1.1)

**Design Decision**: Add `is_dark: bool` to `Theme`. All 7 presets set it correctly. `ThemeBuilder` defaults to `true`. `Context::new()` reads `theme.is_dark` instead of hardcoding `true`.

**Implementation**:
1. Add `pub is_dark: bool` to `Theme` struct
2. Set `is_dark: true` for dark, dracula, catppuccin, nord, solarized, tokyo_night
3. Set `is_dark: false` for light
4. `ThemeBuilder::build()` defaults `is_dark` to `true`
5. `Context::new()`: change `dark_mode: true` → `dark_mode: theme.is_dark`
6. `#[cfg_attr(feature = "serde", ...)]` on `is_dark` if serde feature exists

**Acceptance Criteria**:
- [x] `Theme::light()` → `dark_mode == false` in Context
- [x] `Theme::dark()` → `dark_mode == true` in Context
- [x] All 7 theme presets have correct `is_dark` value
- [x] `ThemeBuilder::build()` without setting `is_dark` defaults to `true`
- [x] Custom `Theme { is_dark: false, .. }` works correctly
- [x] `ui.set_dark_mode()` still overrides at runtime
- [x] Existing apps that don't touch dark_mode behave identically (dark_mode defaults true, Theme::dark() is default)

**QA**: `cargo test` — TestBackend with Theme::light() verifies dark_mode is false.

---

### Task 1.4: Documentation drift fixes (5 items)
- **Category**: `quick`
- **Skills**: `['slt']`
- **Files**: `src/lib.rs`, `README.md`
- **Depends on**: Nothing (parallel)

**Items**:
1. `src/lib.rs:131` — RunConfig rustdoc: "100ms tick" → "16ms tick (60fps)"
2. `src/lib.rs:153` — tick_rate field doc: "Defaults to 100ms" → "Defaults to 16ms"
3. `README.md` — Widget example: `docs.rs/slt` → `docs.rs/superlighttui`
4. `README.md` — Styling section: "4 border styles" → "6 border styles"
5. `README.md` — Examples table: Remove `demo_v050` row (file doesn't exist)

**Acceptance Criteria**:
- [ ] `cargo doc` shows correct "16ms" in RunConfig docs
- [ ] README links point to correct docs.rs URL
- [ ] README border style count matches `Border` enum variant count (6)
- [ ] No references to non-existent examples in README

**QA**: `cargo doc --open` — verify RunConfig page. Manual README review.

---

### Task 1.5: Clippy --all-targets clean
- **Category**: `quick`
- **Skills**: `['slt']`
- **Files**: `examples/demo_website.rs`, `tests/widgets.rs`
- **Depends on**: Nothing (parallel)

**Current failures**:
- `examples/demo_website.rs:418` — `if_same_then_else`
- `examples/demo_website.rs:1739` — `too_many_arguments` (10/7)
- `tests/widgets.rs:929` — `len_zero` (use `!is_empty()`)

**Acceptance Criteria**:
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes clean

**QA**: Run the exact clippy command.

---

## Wave 2: Dev Warnings — Parallel, medium-risk

### Task 2.1: Dev warning framework
- **Category**: `unspecified-low`
- **Skills**: `['slt']`
- **Files**: `src/context.rs`
- **Depends on**: Wave 1 complete

**Implementation**:
1. Extend existing `slt_warn!()` macro at `context.rs:26` — ensure it uses `eprintln!` (stderr, not stdout)
2. All warnings gated with `#[cfg(debug_assertions)]`
3. Warning format: `[SLT warning] {message}` (matches existing pattern)
4. Add `dev_warnings()` method to Context, called at end of each frame (in `lib.rs` run loop)
5. Add `#[cfg(debug_assertions)] pub(crate) dev_warning_log: Vec<String>` to Context — accumulates all warnings in-memory for test inspection
6. `slt_warn!()` both `eprintln!`s AND pushes to `dev_warning_log`
7. Add `pub fn dev_warnings(&self) -> &[String]` to TestBackend for test access

**Acceptance Criteria**:
- [ ] Warnings print to stderr (not stdout — don't corrupt terminal)
- [ ] Warnings also accumulate in `dev_warning_log` (debug builds only)
- [ ] `cargo build --release` produces zero warning-related code (Vec field eliminated)
- [ ] `cargo test --release` passes with zero warning overhead
- [ ] TestBackend exposes `dev_warnings()` returning `&[String]` for assertion

**QA**: `cargo test` — TestBackend renders a frame, then asserts on `tb.dev_warnings()` contents. No stderr capture needed.

---

### Task 2.2: Hook call order change detection
- **Category**: `unspecified-low`
- **Skills**: `['slt']`
- **Files**: `src/context.rs`, `src/lib.rs`
- **Depends on**: Task 2.1

**Implementation**:
1. Add `prev_hook_count: usize` to Context (or FrameState)
2. At end of frame: compare `hook_cursor` with `prev_hook_count`
3. First frame: always exempt (no previous count)
4. On mismatch: `slt_warn!("Hook call count changed: expected {} hooks, got {}. This usually means a use_state/use_memo is inside an if/match block.", prev, current)`
5. Update `prev_hook_count` at end of frame

**Acceptance Criteria**:
- [ ] Warning fires when hook count changes between frames
- [ ] First frame never triggers warning
- [ ] Stable hook count (same every frame) produces no warning
- [ ] Release build: zero cost

**QA**: TestBackend test with conditional `use_state()` — render two frames (second with different hook count), assert `tb.dev_warnings()` contains expected message substring.

---

### Task 2.3: group_stack imbalance detection
- **Category**: `unspecified-low`
- **Skills**: `['slt']`
- **Files**: `src/context.rs`
- **Depends on**: Task 2.1

**Implementation**:
1. At end of frame (in `dev_warnings()`): check `self.group_stack.is_empty()`
2. If not empty: `slt_warn!("group_stack not empty at end of frame ({} unclosed groups). This usually means a group() was opened but the closure panicked or returned early.", self.group_stack.len())`

**Acceptance Criteria**:
- [ ] Warning fires when group_stack is non-empty at frame end
- [ ] Normal usage (matched push/pop) produces no warning
- [ ] Release build: zero cost

**QA**: TestBackend test.

---

### Task 2.4: #[must_use] message improvements
- **Category**: `quick`
- **Skills**: `['slt']`
- **Files**: `src/context.rs`, `src/widgets.rs`
- **Depends on**: Nothing (parallel with 2.1-2.3)

**Implementation**:
1. `ContainerBuilder`: `#[must_use = "ContainerBuilder does nothing until .col(), .row(), .line(), or .draw() is called"]`
2. `register_focusable()` return: ensure `#[must_use = "check this value to determine if the widget is focused"]` (if not already)
3. Do NOT add `#[must_use]` to `Response` (col/row return value) — too noisy

**Acceptance Criteria**:
- [ ] `#[must_use = "..."]` attribute present on `ContainerBuilder` with actionable message
- [ ] `#[must_use]` NOT present on `Response` type
- [ ] Normal usage (`.col(|ui| {...})`) compiles without warning
- [ ] `register_focusable()` return has `#[must_use]` if applicable

**QA**: `ast_grep` or `grep` to verify `#[must_use` attribute exists on `ContainerBuilder` with correct message string. Verify `Response` struct does NOT have `#[must_use]`. Run `cargo check --examples` to confirm no false-positive warnings in existing code.

---

## Wave 3: Input + Visibility — Parallel, medium-risk

### Task 3.1: consume_key() / consume_key_code() API
- **Category**: `unspecified-low`
- **Skills**: `['slt']`
- **Files**: `src/context/widgets_interactive.rs`, `src/lib.rs` (re-export)
- **Depends on**: Wave 2 complete

**Design Decision**: Expose existing `consumed[i] = true` mechanism to user code. Follow same `consumed_indices` pattern used by all widgets.

**Implementation**:
```rust
/// Check for key press and consume the event, preventing widgets from handling it.
/// Returns `true` if the key was found unconsumed and is now consumed.
/// Unlike `key()` which peeks without consuming, this claims exclusive ownership.
///
/// Call AFTER widgets if you want widgets to have priority over your handler.
pub fn consume_key(&mut self, c: char) -> bool {
    if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
        return false;
    }
    for (i, event) in self.events.iter().enumerate() {
        if self.consumed[i] { continue; }
        if matches!(event, Event::Key(k) if k.kind == KeyEventKind::Press && k.code == KeyCode::Char(c)) {
            self.consumed[i] = true;
            return true;
        }
    }
    false
}

/// Same as `consume_key` but for special keys (Enter, Esc, arrows, etc.)
pub fn consume_key_code(&mut self, code: KeyCode) -> bool { /* same pattern */ }
```

**Acceptance Criteria**:
- [ ] `consume_key('x')` returns `true` when 'x' event exists and is unconsumed
- [ ] `consume_key('x')` returns `false` when 'x' was already consumed by a widget
- [ ] `consume_key('x')` returns `false` when no 'x' event exists this frame
- [ ] After `consume_key('x')`, `key('x')` returns `false`
- [ ] After `consume_key('x')`, widget that handles 'x' does NOT react
- [ ] Modal gating works (returns false when modal is active and overlay_depth is 0)

**QA**: TestBackend with `render_with_events()` — inject key events, verify consumption.

---

### Task 3.2: F12 Detail Mode (activate dead code + counters)
- **Category**: `unspecified-low`
- **Skills**: `['slt']`
- **Files**: `src/layout/render.rs`, `src/lib.rs`
- **Depends on**: Nothing (parallel with 3.1)

**Implementation**:
1. Change F12 toggle from bool to cycle: `debug_mode: u8` (0=off, 1=layout, 2=detail)
2. Mode 1 (Layout): Current behavior (container borders + status bar)
3. Mode 2 (Detail): Layout + activate `draw_debug_padding_markers()` + `draw_debug_margin_markers()` (already exist as dead code) + extended status bar: `[w:60 h:20 p:2 | focus:3/8 | hooks:5 | groups:2]`
4. F12 cycles 0→1→2→0

**Acceptance Criteria**:
- [ ] F12 once: Layout mode (same as current)
- [ ] F12 twice: Detail mode (padding/margin markers visible, extended counters in status bar)
- [ ] F12 three times: OFF
- [ ] Padding/margin markers render correctly with nested containers
- [ ] Debug overlay renders on top of modals (topmost layer)

**QA**: Manual tmux verification with `examples/demo.rs`.

---

## Wave 4: Deep Debugging — Parallel, low-risk

### Task 4.1: SLT_LOG file logging
- **Category**: `unspecified-low`
- **Skills**: `['slt']`
- **Files**: `src/lib.rs` (new internal module or inline)
- **Depends on**: Wave 3 complete

**Implementation**:
1. At startup in `run_with()`: check `std::env::var("SLT_LOG")` once
2. If set: open file (append mode) with PID suffix: `slt_{pid}.log` (or custom path)
3. Store `Option<File>` in a `thread_local!` or pass through FrameState
4. `pub(crate) fn slt_log(msg: &str)` — write timestamped line if file is open
5. Gate everything with `#[cfg(debug_assertions)]`
6. Log: frame start/end, key events + consumption, focus changes, resize, panics

**Log format**:
```
[     0ms] SLT start: 120x40, theme=dark, fps=60
[    16ms] Frame 1: 42 widgets, 1.2ms render
[    32ms] Frame 2: Key(Enter) -> consumed by widget
[    48ms] Frame 3: Focus 2 -> 3 (Tab)
```

**Acceptance Criteria**:
- [ ] `SLT_LOG=1` creates `/tmp/slt_{pid}.log` with frame logs
- [ ] `SLT_LOG=/path/to/file` creates log at custom path
- [ ] Without env var: zero overhead (no file I/O, no string formatting)
- [ ] Release build: entire logging code eliminated
- [ ] Multiple SLT apps don't clobber each other's logs (PID suffix)
- [ ] File is created with append mode (safe for long-running apps)

**QA**: Integration test: set env var, run TestBackend frame, read log file, assert content, cleanup.

---

### Task 4.2: Widget path tracking for panic context
- **Category**: `unspecified-low`
- **Skills**: `['slt']`
- **Files**: `src/context.rs`, `src/context/widgets_display.rs`
- **Depends on**: Task 1.1 (error_boundary fix)

**Implementation**:
1. Add `#[cfg(debug_assertions)] widget_path: Vec<&'static str>` to Context
2. Push on container entry: `col()` → push "col", `row()` → push "row", `bordered().title("X")` → push "bordered(X)"
3. Pop on container exit
4. On panic (in panic hook): format and print widget path
5. On error_boundary panic: include path in fallback message

**Enhanced panic output**:
```
━━━ SLT Panic ━━━
src/my_app.rs:42:5
index out of bounds

Widget Path: root > col > row > bordered("Settings") > col
Frame: 847 | Focus: 3/8 | Hooks: 5
```

**Acceptance Criteria**:
- [ ] Panic message includes widget path when available
- [ ] Widget path is correct for nested containers
- [ ] Release build: widget_path field and tracking eliminated entirely
- [ ] Path format: `root > col > row > bordered("Title")`
- [ ] `error_boundary_with()` passes widget path string to fallback closure (as part of the message or separate parameter)

**QA**: TestBackend test with nested containers + `error_boundary_with()` — the fallback closure receives a message containing the widget path. Assert on the fallback message string directly (no stderr capture needed). For panic hook output, verify via `examples/error_boundary_demo.rs` manual tmux test.

---

### Task 4.3: TestBackend error output improvement
- **Category**: `quick`
- **Skills**: `['slt']`
- **Files**: `src/test_utils.rs`
- **Depends on**: Nothing (parallel)

**Implementation**:
1. `assert_contains()` on failure: print full buffer content with line numbers
2. `assert_line()` on failure: print expected vs actual with diff markers
3. Add `assert_styled(text, fg: Color)` — find text, verify its style
4. Add `debug_view()` → formatted buffer dump for eprintln debugging

**Acceptance Criteria**:
- [ ] `assert_contains("xyz")` failure shows full buffer: `Buffer contents:\n1: ...\n2: ...`
- [ ] `assert_line(2, "expected")` failure shows: `Expected: "expected"\nActual:   "actual text here"`
- [ ] `assert_styled("Error", Color::Red)` passes when text exists with that fg color
- [ ] `debug_view()` returns multi-line string suitable for `eprintln!`

**QA**: Write tests that intentionally fail, verify error output format (use `#[should_panic(expected = "...")]`).

---

## Execution Rules

### Commit Strategy
- RED → GREEN → REFACTOR per feature
- `test:` for RED, `feat:`/`fix:` for GREEN, `refactor:` for REFACTOR
- One logical change per commit
- Run all 5 quality gates before every commit

### Branch Strategy
- Each wave on a separate branch: `dx/wave-1-trust`, `dx/wave-2-warnings`, etc.
- Merge via PR after CI passes
- Wave N+1 starts only after Wave N is merged

### Parallel Execution Within Waves
- Wave 1: Tasks 1.1→1.2 (sequential), Tasks 1.3/1.4/1.5 (parallel with 1.1)
- Wave 2: Tasks 2.1→2.2→2.3 (sequential), Task 2.4 (parallel)
- Wave 3: Tasks 3.1 and 3.2 (parallel)
- Wave 4: Tasks 4.1, 4.2, 4.3 (parallel)
