# Positioning in SLT — A Migration Guide for Web & Mobile Devs

If you're coming from CSS, Flutter, or React Native, this guide maps SLT's `Anchor`
and overlay APIs to concepts you already know. SLT is an **immediate-mode** TUI
library, so positioning is a function call, not a styled element — but the mental
model is identical to `position: absolute` + `place-self`.

---

## 1. Quick Reference

| Intent | CSS | Flutter | React Native | SLT |
|---|---|---|---|---|
| Center on screen | `place-self: center; position: absolute` | `Center(child:)` | `position: absolute; top/left/right/bottom: 0; alignItems/justifyContent: 'center'` | `ui.overlay_at(Anchor::Center, \|ui\| ...)` |
| Bottom-right corner | `place-self: end end` | `Align(alignment: Alignment.bottomRight)` | `position: absolute; bottom: 0; right: 0` | `ui.overlay_at(Anchor::BottomRight, \|ui\| ...)` |
| Bottom-right with inset | `position: absolute; bottom: 16px; right: 16px` | `Positioned(bottom: 16, right: 16)` | `position: absolute; bottom: 16; right: 16` | `ui.overlay_at(Anchor::BottomRight, \|ui\| { ui.container().mr(2).mb(1).col(\|ui\| ...) })` |
| Top notification banner | `position: fixed; top: 0; left: 0; right: 0` | `Align(alignment: Alignment.topCenter)` | `position: absolute; top: 0; left: 0; right: 0` | `ui.overlay_at(Anchor::TopCenter, \|ui\| ...)` |
| Modal centered with backdrop | `<Modal />` w/ backdrop | `showDialog()` | `<Modal visible={true}>` | `ui.modal_at(Anchor::Center, \|ui\| ...)` |
| Tooltip near hovered widget | `position: absolute; top: cursorY; left: cursorX` | computed via offset | computed via offset | `ui.tooltip("...")` (uses previous-frame `Response.rect`) |

> **Units note**: SLT offsets are in **terminal cells**, not pixels.
> 1 cell ≈ 8 px wide × 16 px tall (font dependent). Use small numbers (1–3 cells).

---

## 2. The `Anchor` 9-Cell Compass

`Anchor` exposes 9 fixed positions:

```
TopLeft       TopCenter       TopRight
CenterLeft    Center          CenterRight
BottomLeft    BottomCenter    BottomRight
```

### Mapping to CSS `place-self`

| SLT | CSS `place-self` | Flutter `Alignment` | RN `justifyContent` × `alignItems` |
|---|---|---|---|
| `Anchor::TopLeft` | `start start` | `Alignment.topLeft` | `flex-start` × `flex-start` |
| `Anchor::TopCenter` | `start center` | `Alignment.topCenter` | `flex-start` × `center` |
| `Anchor::TopRight` | `start end` | `Alignment.topRight` | `flex-start` × `flex-end` |
| `Anchor::CenterLeft` | `center start` | `Alignment.centerLeft` | `center` × `flex-start` |
| `Anchor::Center` | `center center` | `Alignment.center` | `center` × `center` |
| `Anchor::CenterRight` | `center end` | `Alignment.centerRight` | `center` × `flex-end` |
| `Anchor::BottomLeft` | `end start` | `Alignment.bottomLeft` | `flex-end` × `flex-start` |
| `Anchor::BottomCenter` | `end center` | `Alignment.bottomCenter` | `flex-end` × `center` |
| `Anchor::BottomRight` | `end end` | `Alignment.bottomRight` | `flex-end` × `flex-end` |

Internally, `overlay_at` wraps your closure in a full-screen flex column with
`grow(1)`, then applies the matching `align`/`justify` pair. There's no magic —
it's the same flexbox you know.

---

## 3. Offsets — the SLT analog of `inset` / `top` / `right`

`overlay_at` pins to a corner with **zero inset**. To inset content (e.g. "16 px
from the right edge"), nest a `container()` with margin helpers (`ml`, `mt`, `mr`, `mb`)
inside the closure.

```
Without offset (Anchor::BottomRight):     With mr(2).mb(1) inside:

┌──────────────────────────────────┐     ┌──────────────────────────────────┐
│                                  │     │                                  │
│                                  │     │                                  │
│                                  │     │                                  │
│                                  │     │                                  │
│                                  │     │                                  │
│                                  │     │                                  │
│                                  │     │                                  │
│                          [BADGE] │     │                                  │
└──────────────────────────────────┘     │                       [BADGE]    │
                                          │                                  │
                                          └──────────────────────────────────┘
```

Pattern:

```rust
ui.overlay_at(Anchor::BottomRight, |ui| {
    ui.container()
        .mr(2)   // 2 cells from right edge
        .mb(1)   // 1 cell from bottom edge
        .col(|ui| {
            render_badge(ui);
        });
});
```

### Sign convention

Margins are non-negative (`u32`). They always **inset toward the center**,
regardless of which corner you anchored to:

| Anchor | `mr(2)` shifts | `mb(1)` shifts |
|---|---|---|
| `BottomRight` | left | up |
| `TopRight` | left | (no effect — already at top) |
| `BottomLeft` | (no effect — already at left) | up |

Rule of thumb: only the margin sides that face the **opposite** edge of your anchor
have visible effect. For `BottomRight`, use `mr` and `mb`. For `TopLeft`, use
`ml` and `mt`.

---

## 4. When NOT to use overlay_at

Most layout should be flow-based (`ui.row` / `ui.col`), not overlays. Overlays
exist for content that floats independently of the document flow.

| Use case | Right tool | Why |
|---|---|---|
| Toolbar at top of app | `ui.col(\|ui\| { toolbar(ui); main(ui); })` | Part of layout flow |
| Sidebar on the left | `ui.row(\|ui\| { sidebar(ui); main(ui); })` | Part of layout flow |
| Floating "Save" badge that doesn't push content | `ui.overlay_at(Anchor::BottomRight, ...)` | Floats above flow |
| Modal that dims background | `ui.modal_at(Anchor::Center, ...)` | Floats + dims |
| Tooltip near a hovered widget | `ui.tooltip("...")` | Anchored to widget rect |
| Toast notifications | `ui.overlay_at(Anchor::TopRight, ...)` | Floats above flow |

**Heuristic**: if removing the element would change where surrounding content sits,
it belongs in flow (`row`/`col`). If removing it leaves the rest of the layout
unchanged, it's an overlay.

---

## 5. Migrating from CSS / Flutter / React Native

### From CSS

```css
.badge {
  position: absolute;
  bottom: 16px;
  right: 16px;
}
```

```rust
ui.overlay_at(Anchor::BottomRight, |ui| {
    ui.container().mr(2).mb(1).col(|ui| {
        render_badge(ui);
    });
});
```

### From Flutter

```dart
Stack(
  children: [
    Align(
      alignment: Alignment.bottomRight,
      child: Padding(
        padding: EdgeInsets.only(bottom: 16, right: 16),
        child: Badge(),
      ),
    ),
  ],
)
```

```rust
ui.overlay_at(Anchor::BottomRight, |ui| {
    ui.container().mr(2).mb(1).col(|ui| {
        render_badge(ui);
    });
});
```

### From React Native

```jsx
<View style={{ position: 'absolute', bottom: 16, right: 16 }}>
  <Badge />
</View>
```

```rust
ui.overlay_at(Anchor::BottomRight, |ui| {
    ui.container().mr(2).mb(1).col(|ui| {
        render_badge(ui);
    });
});
```

### Modal with backdrop (CSS / Flutter / RN → SLT)

```css
/* CSS */
.modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.5); }
.modal { position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); }
```

```dart
// Flutter
showDialog(context: context, builder: (_) => AlertDialog(...));
```

```jsx
// React Native
<Modal visible transparent><View style={styles.center}>...</View></Modal>
```

```rust
// SLT — modal_at adds the dimmed backdrop automatically
ui.modal_at(Anchor::Center, |ui| {
    ui.text("Are you sure?");
    if ui.button("OK").clicked { confirm = true; }
});
```

---

## 6. Common Pitfalls

- **"My overlay collapses to top-left even though content has `align(End)`."**
  Use the typed entry points: `overlay_at` / `modal_at`. They internally apply
  `grow(1)` to the wrapper so flexbox has slack to push against. Hand-rolling
  with `align`/`justify` on a non-growing container will collapse.

- **"My margin doesn't shift the badge."** Margins only have visible effect on
  sides facing the **opposite** of your anchor. `Anchor::TopLeft` + `mr(2)` does
  nothing. Use `ml` and `mt` instead.

- **"Offsets are in pixels in my CSS but my badge is way off."** Terminal cells
  ≠ CSS px. 1 cell ≈ 8 px wide × 16 px tall. CSS `16px` ≈ 2 cells. Use small
  numbers (1–3) not 16.

- **"Multiple overlays — which one wins?"** Declaration order. Earlier
  `overlay_at` calls render below later ones. Render the most important overlay
  (e.g. modal) **last** so it stacks on top.

- **"Tooltip doesn't show."** `tooltip()` requires the previous widget to be
  hovered AND have a non-zero rect from the previous frame. Make sure the
  preceding widget call returned a `Response` and is interactive.

---

## 7. Cross-References

- **API reference**: `Anchor`, `overlay_at`, `modal_at` re-exported from the
  crate root — see `src/lib.rs` re-exports
- **Implementation**: `src/context/widgets_display/layout.rs` (`Anchor` enum,
  `anchor_to_align_justify`, `overlay_at`, `modal_at`)
- **Container margin helpers**: `ml`, `mt`, `mr`, `mb`, `mx`, `my`, `margin` in
  `src/context/container.rs`
- **Runnable demo**: `cargo run --example demo_overlay_anchor` — renders all 9
  anchors at once
- **Cookbook**: `docs/COOKBOOK.md` (Modal Confirmation with Toast, Real-time Dashboard)
- **Reference**: `docs/COMPLETE_REFERENCE.md` — full API surface

---

## 8. TL;DR

```rust
use slt::{Anchor, Context};

slt::run(|ui: &mut Context| {
    // Flow content
    ui.col(|ui| {
        ui.text("Main content here");
    });

    // Floating badge in the bottom-right with a 2x1 cell inset
    ui.overlay_at(Anchor::BottomRight, |ui| {
        ui.container().mr(2).mb(1).col(|ui| {
            ui.text("v0.19.2").dim();
        });
    });

    // Centered modal with backdrop
    if show_dialog {
        ui.modal_at(Anchor::Center, |ui| {
            ui.text("Confirm?");
            if ui.button("OK").clicked { show_dialog = false; }
        });
    }
});
```

That's the whole positioning system. Nine anchors, two entry points
(`overlay_at` / `modal_at`), and standard margin helpers for fine-tuning insets.
