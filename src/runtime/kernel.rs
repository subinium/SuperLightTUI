//! Shared frame kernel used by every run loop and by `slt::frame`.

use super::*;

use std::io;

use crate::clock::Instant;
#[cfg(feature = "crossterm")]
use crate::terminal;
use crate::{Backend, Buffer, Context, Event, KeyCode, context, event, layout};

/// Reserved sentinel key used by [`Context::static_log`] (issue #233).
/// Re-exported into `context::runtime` so reads/writes never drift.
pub(crate) const STATIC_LOG_NAMED_STATE_KEY: &str = "__slt_static_log_pending";

/// Reserved sentinel key used by [`Context::publish_keymap`] (issue #236).
/// Re-exported into `context::runtime` so reads/writes never drift.
pub(crate) const KEYMAP_REGISTRY_NAMED_STATE_KEY: &str = "__slt_keymap_registry";

/// Clear the per-frame keymap registry stored in [`FrameState::named_states`]
/// (issue #236). Called at the start of every kernel iteration so that
/// `Context::publish_keymap` always sees a fresh empty buffer. Capacity is
/// preserved by clearing the inner `Vec` rather than removing the entry.
pub(crate) fn clear_keymap_registry(state: &mut FrameState) {
    if let Some(boxed) = state.named_states.get_mut(KEYMAP_REGISTRY_NAMED_STATE_KEY)
        && let Some(vec) = boxed.downcast_mut::<Vec<crate::keymap::PublishedKeymap>>()
    {
        vec.clear();
    }
}

/// Apply a single terminal event to `FrameState`, mutating tracked
/// diagnostics fields (debug overlay toggle, mouse position cache,
/// resize flag) accordingly.
///
/// Issue #201: handles **F12** (toggle overlay on/off) and **Shift+F12**
/// (cycle [`DebugLayer`] across `All → TopMost → BaseOnly`). The two
/// keybindings are independent — toggling the overlay does not change
/// the active layer.
///
/// Extracted from `poll_events` so the keybinding behavior can be
/// exercised by unit tests without standing up a real crossterm event
/// stream.
pub(crate) fn process_run_loop_event(ev: &Event, state: &mut FrameState, has_resize: &mut bool) {
    match ev {
        Event::Mouse(m) => {
            state.layout_feedback.last_mouse_pos = Some((m.x, m.y));
        }
        Event::FocusLost => {
            state.layout_feedback.last_mouse_pos = None;
            state.layout_feedback.last_click_at = None;
            state.layout_feedback.last_click_pos = None;
        }
        // Issue #268: Ctrl+F12 toggles the devtools inspector panel
        // independently of the F12 outline overlay and the Shift+F12 layer
        // cycle. Match before the Shift/NONE arms so the Control branch wins.
        Event::Key(event::KeyEvent {
            code: KeyCode::F(12),
            kind: event::KeyEventKind::Press,
            modifiers,
        }) if modifiers.contains(event::KeyModifiers::CONTROL) => {
            state.diagnostics.inspector_mode = !state.diagnostics.inspector_mode;
        }
        // Issue #201: Shift+F12 cycles the active `DebugLayer`. Match
        // before the plain-F12 arm so the modifier branch wins. Plain
        // F12 keeps its legacy on/off toggle when no modifiers are
        // held; we explicitly require `KeyModifiers::NONE` so the two
        // arms do not double-fire on the same press.
        Event::Key(event::KeyEvent {
            code: KeyCode::F(12),
            kind: event::KeyEventKind::Press,
            modifiers,
        }) if modifiers.contains(event::KeyModifiers::SHIFT) => {
            state.diagnostics.debug_layer = match state.diagnostics.debug_layer {
                DebugLayer::All => DebugLayer::TopMost,
                DebugLayer::TopMost => DebugLayer::BaseOnly,
                DebugLayer::BaseOnly => DebugLayer::All,
            };
        }
        Event::Key(event::KeyEvent {
            code: KeyCode::F(12),
            kind: event::KeyEventKind::Press,
            modifiers,
        }) if *modifiers == event::KeyModifiers::NONE => {
            state.diagnostics.debug_mode = !state.diagnostics.debug_mode;
        }
        Event::Resize(_, _) => {
            *has_resize = true;
        }
        _ => {}
    }
}

pub(crate) struct FrameKernelResult {
    pub(crate) should_quit: bool,
    #[cfg(feature = "crossterm")]
    clipboard_text: Option<String>,
    #[cfg(feature = "crossterm")]
    should_copy_selection: bool,
}

pub(crate) fn input_frame(state: &mut FrameState, mut events: Vec<Event>) -> Vec<Event> {
    fn navigation(event: &Event) -> bool {
        matches!(event, Event::Key(key) if key.kind == event::KeyEventKind::Press
            && matches!(key.code, event::KeyCode::Tab | event::KeyCode::BackTab))
    }
    if !state.pending_input.is_empty() {
        state.pending_input.extend(events.drain(..));
        let mut navigated = false;
        while let Some(event) = state.pending_input.front() {
            let is_navigation = navigation(event);
            if navigated && !is_navigation {
                break;
            }
            navigated |= is_navigation;
            if let Some(event) = state.pending_input.pop_front() {
                events.push(event);
            }
        }
        return events;
    }
    if let Some(first) = events.iter().position(navigation) {
        let end = first
            + events[first..]
                .iter()
                .take_while(|event| navigation(event))
                .count();
        // Keep consecutive navigation keys together, but do not let a later
        // character/activation edit the widget that is about to lose focus.
        // Moving events preserves large paste payloads without cloning them.
        state.pending_input.extend(events.drain(end..));
    }
    events
}

pub(crate) fn run_frame_kernel(
    buffer: &mut Buffer,
    state: &mut FrameState,
    config: &RunConfig,
    size: (u32, u32),
    events: Vec<event::Event>,
    is_real_terminal: bool,
    f: &mut impl FnMut(&mut context::Context),
) -> FrameKernelResult {
    let events = input_frame(state, events);
    let frame_start = Instant::now();
    let now = state.diagnostics.clock_override.unwrap_or(frame_start);
    if let Some(previous) = state.diagnostics.frame_started {
        let interval = now.saturating_duration_since(previous);
        state.diagnostics.frame_interval = interval;
        if !interval.is_zero() {
            let fps = 1.0 / interval.as_secs_f32();
            state.diagnostics.fps_ema = if state.diagnostics.fps_ema == 0.0 {
                fps
            } else {
                state.diagnostics.fps_ema * 0.9 + fps * 0.1
            };
        }
    }
    state.diagnostics.frame_started = Some(now);
    let (w, h) = size;
    if state.layout_feedback.size != Some(size)
        || events
            .iter()
            .any(|event| matches!(event, Event::Resize(_, _)))
    {
        clear_frame_layout_cache(state);
    }
    state.layout_feedback.size = Some(size);
    let mut resized = false;
    for event in &events {
        process_run_loop_event(event, state, &mut resized);
    }
    // Issue #236: reset the per-frame keymap registry before constructing
    // `Context`. Widgets that call `publish_keymap` accumulate fresh
    // entries; entries from the previous frame must not leak through
    // `named_states` persistence.
    clear_keymap_registry(state);
    // Issue #273: invalidate every `cached` region's persisted version key on a
    // resize. The real run loop also clears region keys via
    // `clear_frame_layout_cache` (driven by its `has_resize` flag), but the
    // headless `TestBackend` / `frame_owned` paths feed the kernel directly
    // and never run that flag, so we detect the resize event here too. This
    // keeps the "resize forces a cache miss for all cached regions" invariant
    // path-independent: a geometry change cannot be silently treated as a hit.
    // Cheap when unused — `region_versions` is empty for apps without `cached`.
    if !state.region_versions.is_empty() && events.iter().any(|e| matches!(e, Event::Resize(_, _)))
    {
        state.region_versions.clear();
    }
    let mut ctx = Context::new(events, w, h, state, config.theme);
    ctx.is_real_terminal = is_real_terminal;
    // Issue #264: surface the negotiated capability snapshot read-only. The
    // probe ran once at session enter (cached in a `OnceLock`); on a headless
    // backend it never ran, so we keep the conservative default rather than
    // forcing a probe that would block on stdin.
    #[cfg(feature = "crossterm")]
    if is_real_terminal {
        ctx.capabilities = terminal::capabilities();
    }
    ctx.set_scroll_speed(config.scroll_speed);
    ctx.widget_theme = config.widget_theme;

    f(&mut ctx);
    if ctx.prev_modal_active && !ctx.rollback.modal_active {
        ctx.focus_index = ctx.rollback.modal_restore_index.take().unwrap_or(0);
    } else if ctx.rollback.modal_active
        && let Some((requested, resolved)) = ctx.rollback.modal_focus_rebase
        && ctx.focus_index == requested
    {
        ctx.focus_index = resolved;
    }
    ctx.process_focus_keys();
    ctx.finish_screen_focus();
    ctx.finish_focus_identity(&mut state.focus);
    ctx.render_notifications();
    ctx.emit_pending_tooltips();

    debug_assert_eq!(
        ctx.rollback.overlay_depth, 0,
        "overlay depth must settle back to zero before layout"
    );
    debug_assert_eq!(
        ctx.rollback.group_count, 0,
        "group count must settle back to zero before layout"
    );
    debug_assert!(
        ctx.rollback.group_stack.is_empty(),
        "group stack must be empty before layout"
    );
    debug_assert!(
        ctx.rollback.text_color_stack.is_empty(),
        "text color stack must be empty before layout"
    );
    debug_assert!(
        ctx.pending_tooltips.is_empty(),
        "pending tooltips must be emitted before layout"
    );

    if ctx.should_quit {
        state.pending_input.clear();
        reclaim_feedback(&mut ctx, state);
        reclaim_event_scratch(&mut ctx, state);
        state.hook_states = ctx.hook_states;
        state.named_states = ctx.named_states;
        state.keyed_states = ctx.keyed_states;
        // Issue #262: persist the partial-chord buffer on quit too (TestBackend
        // reuses `FrameState` across `render()` calls — same rationale as the
        // keyed-state reclaim).
        state.chord_states = ctx.chord;
        // Issue #248: hand the scheduler table back and GC abandoned timers.
        let mut scheduler = ctx.scheduler;
        scheduler.gc_untouched();
        state.scheduler = scheduler;
        // Issue #234: hand the async task registry back so in-flight tasks and
        // pending results survive to the next frame (TestBackend reuses
        // `FrameState` across `render()` calls — same rationale as the
        // scheduler reclaim).
        #[cfg(feature = "async")]
        {
            // Pump the registry every frame so a handle dropped on a frame that
            // calls neither spawn nor poll still has its cancellation processed
            // (and completed results moved in) before the round-trip.
            ctx.async_tasks.maintain();
            state.async_tasks = ctx.async_tasks;
        }
        state.screen_hook_map = ctx.screen_hook_map;
        state.diagnostics.notification_queue = ctx.rollback.notification_queue;
        state.diagnostics.debug_layer = ctx.debug_layer;
        // Issue #268: persist any in-frame `set_inspector` change on quit too.
        state.diagnostics.inspector_mode = ctx.inspector_mode;
        // Issue #208 / #217: persist focus tracking state on quit so a later
        // resumed run starts in a sensible place. (Real TUI exits before
        // resuming, but tests reuse `FrameState` across calls.)
        state.focus.prev_focus_index = ctx.rollback.focused_widget_id;
        state.focus.focus_name_map_prev = ctx.focus_name_map;
        state.focus.pending_focus_name = ctx.pending_focus_name;
        // Issue #204: reclaim the 6 alloc-reuse buffers on the quit path
        // too. Real TUI exits ignore this, but TestBackend reuses the same
        // FrameState across `render()` calls — without the reclaim the next
        // frame's `Context::new` `mem::take`s an empty Vec and silently
        // reverts to v0.19 per-frame allocation.
        ctx.deferred_draws.clear();
        state.context_stack_buf = std::mem::take(&mut ctx.context_stack);
        state.deferred_draws_buf = std::mem::take(&mut ctx.deferred_draws);
        state.group_stack_buf = std::mem::take(&mut ctx.rollback.group_stack);
        state.text_color_stack_buf = std::mem::take(&mut ctx.rollback.text_color_stack);
        state.pending_tooltips_buf = std::mem::take(&mut ctx.pending_tooltips);
        state.hovered_groups_buf = std::mem::take(&mut ctx.hovered_groups);
        // Issue #273: reclaim the region-cache key buffers on quit too
        // (TestBackend reuses `FrameState` across `render()` calls — same
        // rationale as #204). The quit path skips `build_tree`, but the keys
        // recorded by any `cached` regions before `quit()` are still valid as
        // next frame's baseline.
        state.region_versions = std::mem::take(&mut ctx.region_versions_cur);
        state.region_versions_buf = std::mem::take(&mut ctx.region_versions_prev);
        // Issue #150: reclaim `commands` on quit too (TestBackend reuses
        // `FrameState` across `render()` calls — same rationale as #204).
        // The Vec was never `build_tree`'d on the quit path so it may still
        // hold the recorded commands; clearing here drops them and keeps
        // capacity for the next frame.
        ctx.commands.clear();
        state.commands_buf = std::mem::take(&mut ctx.commands);
        #[cfg(feature = "crossterm")]
        let clipboard_text = ctx.clipboard_text.take();
        #[cfg(feature = "crossterm")]
        let should_copy_selection = false;
        return FrameKernelResult {
            should_quit: true,
            #[cfg(feature = "crossterm")]
            clipboard_text,
            #[cfg(feature = "crossterm")]
            should_copy_selection,
        };
    }
    state.focus.prev_modal_active = ctx.rollback.modal_active;
    state.focus.prev_modal_focus_start = ctx.rollback.modal_focus_start;
    state.focus.prev_modal_focus_count = ctx.rollback.modal_focus_count;
    state.focus.prev_modal_id = ctx.rollback.modal_id;
    state.focus.modal_restore_index = ctx.rollback.modal_restore_index;
    #[cfg(feature = "crossterm")]
    let clipboard_text = ctx.clipboard_text.take();
    #[cfg(not(feature = "crossterm"))]
    let _clipboard_text = ctx.clipboard_text.take();

    #[cfg(feature = "crossterm")]
    let mut should_copy_selection = false;
    #[cfg(feature = "crossterm")]
    for ev in &ctx.events {
        if let Event::Mouse(mouse) = ev {
            match mouse.kind {
                event::MouseKind::Down(event::MouseButton::Left) => {
                    state.selection.mouse_down(
                        mouse.x,
                        mouse.y,
                        &state.layout_feedback.prev_content_map,
                    );
                }
                event::MouseKind::Drag(event::MouseButton::Left) => {
                    state.selection.mouse_drag(
                        mouse.x,
                        mouse.y,
                        &state.layout_feedback.prev_content_map,
                    );
                }
                event::MouseKind::Up(event::MouseButton::Left) => {
                    should_copy_selection = state.selection.active;
                }
                _ => {}
            }
        }
    }

    state.focus.focus_index = ctx.focus_index;
    state.focus.prev_focus_count = ctx.rollback.focus_count;

    // Issue #150: `state.commands_buf` is swapped into `ctx.commands` on
    // entry (see `Context::new`), so the per-frame `Vec::new()` allocation
    // for the command list is amortized to one allocation across the
    // session. `build_tree` now takes `&mut Vec<Command>` and `drain`s it,
    // leaving the Vec at `len == 0` with capacity preserved. We reclaim
    // that Vec into `state.commands_buf` after the frame so the next call
    // to `Context::new` can pick it up via `mem::take` (matches the #204
    // pattern for the other six recycled buffers).
    let mut tree = layout::build_tree(&mut ctx.commands);
    let area = crate::rect::Rect::new(0, 0, w, h);
    layout::compute(&mut tree, area);

    // Recover the previous feedback vectors as this frame's collection
    // scratch, then publish the newly collected vectors with a second swap.
    // This keeps both sides' capacities warm instead of `mem::take`-ing every
    // filled vector and leaving an empty FrameData behind.
    reclaim_feedback(&mut ctx, state);
    let mut fd = std::mem::take(&mut state.frame_data);
    fd.swap_feedback(&mut state.layout_feedback);
    layout::collect_all(&tree, &mut fd);
    if state
        .focus
        .scroll
        .prepare(ctx.rollback.focused_widget_id, &fd.geometry)
    {
        layout::apply_scroll_adjustments(&mut tree, &state.focus.scroll.pending);
        // Recollect only when a focus transition or fresh bounds moved a
        // viewport, so rendering and mouse feedback share the adjusted tree.
        layout::collect_all(&tree, &mut fd);
    }
    debug_assert_eq!(
        fd.scroll_infos.len(),
        fd.scroll_rects.len(),
        "scroll feedback vectors must stay aligned"
    );
    fd.swap_feedback(&mut state.layout_feedback);
    let mut raw_rects = std::mem::take(&mut fd.raw_draw_rects);
    layout::render(&tree, buffer);
    if ctx
        .rollback
        .focused_widget_id
        .is_some_and(|rendered| rendered != ctx.focus_index % ctx.rollback.focus_count.max(1))
    {
        // A late focus request has not rendered its privacy policy yet. Do not
        // let an IME backend preview new input at the previous plain caret.
        buffer.cursor_pos = None;
    }
    let mut deferred_draw_panic = None;
    for rdr in raw_rects.drain(..) {
        if rdr.rect.width == 0 || rdr.rect.height == 0 {
            continue;
        }
        let Some(cb) = ctx
            .deferred_draws
            .get_mut(rdr.draw_id)
            .and_then(|c| c.take())
        else {
            continue;
        };
        if let Err(panic) = context::invoke_deferred_draw(
            buffer,
            rdr.rect,
            rdr.left_clip_cols,
            rdr.top_clip_rows,
            rdr.original_width,
            rdr.original_height,
            cb,
        ) {
            deferred_draw_panic = Some(panic);
            break;
        }
    }
    raw_rects.clear();
    fd.raw_draw_rects = raw_rects;
    state.frame_data = fd;
    debug_assert!(
        buffer.kitty_clip_info_stack.is_empty(),
        "kitty_clip_info_stack must be empty at end of frame"
    );
    debug_assert!(
        buffer.kitty_horizontal_clip_stack.is_empty(),
        "kitty_horizontal_clip_stack must be empty at end of frame"
    );
    reclaim_event_scratch(&mut ctx, state);
    state.hook_states = ctx.hook_states;
    state.named_states = ctx.named_states;
    // Issue #215: hand the keyed-state map back to FrameState so the next
    // frame can pick it up via `Context::new`. Mirrors the `named_states`
    // round-trip exactly.
    state.keyed_states = ctx.keyed_states;
    // Issue #262: hand the partial-chord buffer back so a chord spanning
    // multiple frames survives between them. Same round-trip as `keyed_states`.
    state.chord_states = ctx.chord;
    // Issue #248: hand the scheduler table back and GC any timer slot that was
    // not sampled this frame (mirrors the `named_states` round-trip lifecycle).
    let mut scheduler = ctx.scheduler;
    scheduler.gc_untouched();
    state.scheduler = scheduler;
    // Issue #234: hand the async task registry back so in-flight tasks and
    // pending results survive to the next frame (same round-trip lifecycle as
    // the scheduler table).
    #[cfg(feature = "async")]
    {
        // Pump the registry every frame (see the quit-path note): drains
        // completed results and honours handle-drop cancellations even on a
        // frame that called neither spawn nor poll.
        ctx.async_tasks.maintain();
        state.async_tasks = ctx.async_tasks;
    }
    state.screen_hook_map = ctx.screen_hook_map;
    state.diagnostics.notification_queue = ctx.rollback.notification_queue;
    // Issue #201: persist any in-frame `set_debug_layer` change.
    state.diagnostics.debug_layer = ctx.debug_layer;
    // Issue #268: persist any in-frame `set_inspector` change.
    state.diagnostics.inspector_mode = ctx.inspector_mode;
    // Issue #208: remember the focus index that finished this frame so the
    // next frame can compute `Response::gained_focus` / `lost_focus`.
    state.focus.prev_focus_index = ctx.rollback.focused_widget_id;
    // Issue #217: swap the freshly-built focus name map into the previous
    // slot for next-frame resolution; carry forward any unresolved pending
    // name (deferred until the named widget exists).
    state.focus.focus_name_map_prev = ctx.focus_name_map;
    state.focus.pending_focus_name = ctx.pending_focus_name;

    // Issue #204: reclaim the six per-frame `Vec`/`HashSet` allocations so the
    // next frame reuses the existing capacity instead of allocating fresh.
    // Frame-end invariants (asserted above at lines 1102–1121):
    //   - `rollback.group_stack` and `rollback.text_color_stack` are empty
    //   - `pending_tooltips` is empty
    // `context_stack` is asserted-empty by the consumers in `widgets_*`
    // modules (provider/use_context); on the rare panic-rollback path the
    // checkpoint truncates it back to the saved length, so we still
    // recover capacity.
    //
    // `deferred_draws`: most slots are emptied by the `take()` above, but
    // entries whose `RawDrawRect` had `width == 0 || height == 0` are
    // skipped at the loop guard and remain `Some(_)`. We explicitly
    // `clear()` to drop those callbacks here so they don't outlive the
    // frame; capacity is preserved. (Leaving them would not cause UB —
    // `Context::new` calls `.clear()` on the reclaimed Vec — but dropping
    // promptly matches user expectation that one-shot callbacks don't
    // survive past their frame.)
    //
    // `hovered_groups`: `clear()`-ed at the start of every frame inside
    // `build_hovered_groups`, so the existing entries are harmless to
    // reclaim with content; capacity is preserved.
    ctx.deferred_draws.clear();
    state.context_stack_buf = std::mem::take(&mut ctx.context_stack);
    state.deferred_draws_buf = std::mem::take(&mut ctx.deferred_draws);
    state.group_stack_buf = std::mem::take(&mut ctx.rollback.group_stack);
    state.text_color_stack_buf = std::mem::take(&mut ctx.rollback.text_color_stack);
    state.pending_tooltips_buf = std::mem::take(&mut ctx.pending_tooltips);
    state.hovered_groups_buf = std::mem::take(&mut ctx.hovered_groups);
    // Issue #273: this frame's recorded `cached` keys become next frame's
    // comparison baseline; the (now-stale) previous keys are reclaimed as the
    // recycled scratch buffer. Same alloc-reuse discipline as `commands_buf`.
    state.region_versions = std::mem::take(&mut ctx.region_versions_cur);
    state.region_versions_buf = std::mem::take(&mut ctx.region_versions_prev);
    // Issue #150: reclaim the drained command Vec so the next `Context::new`
    // picks it up via `mem::take(&mut state.commands_buf)`. After
    // `build_tree(&mut ctx.commands)` the Vec is at `len == 0` with capacity
    // preserved; mirror the #204 reclamation pattern for the other six
    // per-frame buffers.
    state.commands_buf = std::mem::take(&mut ctx.commands);

    let frame_time = frame_start.elapsed();
    state.diagnostics.render_duration = frame_time;
    let frame_time_us = frame_time.as_micros().min(u128::from(u64::MAX)) as u64;
    if state.diagnostics.debug_mode {
        layout::render_debug_overlay(
            &tree,
            buffer,
            frame_time_us,
            state.diagnostics.fps_ema,
            state.diagnostics.debug_layer,
        );
    }
    // Issue #268: render the devtools inspector panel (Ctrl+F12) on top of the
    // frame. Reuses the already-built tree and the focus snapshot threaded in
    // from `FrameState` (no new traversal beyond one focused-node DFS). The
    // name map was already swapped into `focus_name_map_prev` above, so it
    // reflects this frame's registrations.
    if state.diagnostics.inspector_mode {
        let focus = layout::InspectorFocus {
            focus_index: state.focus.focus_index,
            focus_count: state.focus.prev_focus_count,
            names: &state.focus.focus_name_map_prev,
            theme: &config.theme,
        };
        layout::render_inspector(&tree, buffer, &focus);
    }

    // The callback executed after layout, so no Context fallback can safely
    // run here. Restore every persistent frame field first, then preserve the
    // original panic for the owning runtime or outer catch_unwind boundary.
    if let Some(panic) = deferred_draw_panic {
        std::panic::resume_unwind(panic);
    }

    FrameKernelResult {
        should_quit: false,
        #[cfg(feature = "crossterm")]
        clipboard_text,
        #[cfg(feature = "crossterm")]
        should_copy_selection,
    }
}

fn reclaim_feedback(ctx: &mut Context, state: &mut FrameState) {
    state.layout_feedback.geometry = std::mem::take(&mut ctx.prev_geometry);
    state.focus.scroll.pending = std::mem::take(&mut ctx.pending_scroll);
    state.layout_feedback.prev_scroll_infos = std::mem::take(&mut ctx.prev_scroll_infos);
    state.layout_feedback.prev_scroll_rects = std::mem::take(&mut ctx.prev_scroll_rects);
    state.layout_feedback.prev_hit_map = std::mem::take(&mut ctx.prev_hit_map);
    state.layout_feedback.prev_allocated_areas = std::mem::take(&mut ctx.prev_allocated_areas);
    state.layout_feedback.prev_group_rects = std::mem::take(&mut ctx.prev_group_rects);
    state.layout_feedback.prev_focus_groups = std::mem::take(&mut ctx.prev_focus_groups);
}

fn reclaim_event_scratch(ctx: &mut Context, state: &mut FrameState) {
    state.focus.screen_ranges = std::mem::take(&mut ctx.screen_focus_ranges);
    ctx.screen_focus_scopes.clear();
    state.focus.screen_scopes_buf = std::mem::take(&mut ctx.screen_focus_scopes);
    ctx.events.clear();
    ctx.consumed.clear();
    state.events_buf = std::mem::take(&mut ctx.events);
    state.consumed_buf = std::mem::take(&mut ctx.consumed);
    state.focus.focus_name_map_buf = std::mem::take(&mut ctx.focus_name_map_prev);
    ctx.geometry_stack.clear();
    state.geometry_stack_buf = std::mem::take(&mut ctx.geometry_stack);
}

pub(crate) fn run_frame(
    term: &mut impl Backend,
    state: &mut FrameState,
    config: &RunConfig,
    events: Vec<event::Event>,
    terminal_side_effects: bool,
    f: &mut impl FnMut(&mut context::Context),
) -> io::Result<bool> {
    let size = term.size();
    let kernel = run_frame_kernel(
        term.buffer_mut(),
        state,
        config,
        size,
        events,
        terminal_side_effects,
        f,
    );
    if kernel.should_quit {
        return Ok(false);
    }

    #[cfg(feature = "crossterm")]
    if state.selection.active {
        terminal::apply_selection_overlay(
            term.buffer_mut(),
            &state.selection,
            &state.layout_feedback.prev_content_map,
        );
    }
    #[cfg(feature = "crossterm")]
    if terminal_side_effects && kernel.should_copy_selection {
        let text = terminal::extract_selection_text(
            term.buffer_mut(),
            &state.selection,
            &state.layout_feedback.prev_content_map,
        );
        if !text.is_empty() {
            terminal::copy_to_clipboard(&mut io::stdout(), &text)?;
        }
        state.selection.clear();
    }

    let flush_start = Instant::now();
    term.flush()?;
    state.diagnostics.flush_duration = flush_start.elapsed();
    #[cfg(feature = "crossterm")]
    if terminal_side_effects && let Some(text) = kernel.clipboard_text {
        #[allow(clippy::print_stderr)]
        if let Err(e) = terminal::copy_to_clipboard(&mut io::stdout(), &text) {
            eprintln!("[slt] failed to copy to clipboard: {e}");
        }
    }
    state.diagnostics.tick = state.diagnostics.tick.wrapping_add(1);

    Ok(true)
}

pub(crate) fn clear_frame_layout_cache(state: &mut FrameState) {
    state.layout_feedback.geometry.clear();
    state.focus.scroll.invalidate();
    state.layout_feedback.prev_hit_map.clear();
    state.layout_feedback.prev_allocated_areas.clear();
    state.layout_feedback.prev_group_rects.clear();
    state.layout_feedback.prev_content_map.clear();
    state.layout_feedback.prev_focus_rects.clear();
    state.layout_feedback.prev_focus_groups.clear();
    state.layout_feedback.prev_scroll_infos.clear();
    state.layout_feedback.prev_scroll_rects.clear();
    state.layout_feedback.last_mouse_pos = None;
    state.layout_feedback.last_click_at = None;
    state.layout_feedback.last_click_pos = None;
    #[cfg(feature = "crossterm")]
    state.selection.clear();
    // Issue #273: a resize may change the geometry of every cached region, so
    // the previous frame's version keys are no longer a safe stability signal.
    // Dropping them forces a cache miss for all `cached` regions on the next
    // frame, matching the layout-feedback invalidation above.
    state.region_versions.clear();
}
