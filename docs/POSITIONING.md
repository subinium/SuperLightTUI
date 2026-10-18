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
| Bottom-right with inset | `position: absolute; bottom: 16px; right: 16px` | `Positioned(bottom: 16, right: 16)` | `position: absolute; bottom: 16; right: 16` | `ui.overlay_at_offset(Anchor::BottomRight, 2, 1, \|ui\| ...)` |
| Top notification banner | `position: fixed; top: 0; left: 0; right: 0` | `Align(alignment: Alignment.topCenter)` | `position: absolute; top: 0; left: 0; right: 0` | `ui.overlay_at(Anchor::TopCenter, \|ui\| ...)` |
| Modal centered with backdrop | `<Modal />` w/ backdrop | `showDialog()` | `<Modal visible={true}>` | `ui.modal_at(Anchor::Center, \|ui\| ...)` |
| Modal pinned near a corner | `position: fixed; top: 16px; right: 16px` + backdrop | `showDialog` + `Align` + `Padding` | `<Modal>` + absolute child | `ui.modal_at_offset(Anchor::TopRight, 2, 1, \|ui\| ...)` |
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

`overlay_at` / `modal_at` pin to an anchor with **zero inset**. To inset the
content (e.g. "16 px from the right edge"), use the offset variants:

```rust
pub fn overlay_at_offset(&mut self, anchor: Anchor, dx: i32, dy: i32, f: impl FnOnce(&mut Context)) -> Response
pub fn modal_at_offset(&mut self, anchor: Anchor, dx: i32, dy: i32, f: impl FnOnce(&mut Context)) -> Response
```

`dx` / `dy` are in terminal cells. Both return the overlay's `Response`.

```
overlay_at(BottomRight)                overlay_at_offset(BottomRight, 2, 1)

┌──────────────────────────────────┐   ┌──────────────────────────────────┐
│                                  │   │                                  │
│                                  │   │                                  │
│                                  │   │                                  │
│                                  │   │                                  │
│                                  │   │                        [BADGE]   │
│                           [BADGE]│   │                                  │
└──────────────────────────────────┘   └──────────────────────────────────┘
```

Pattern:

```rust
// 2 cells from the right edge, 1 row from the bottom edge
ui.overlay_at_offset(Anchor::BottomRight, 2, 1, |ui| {
    render_badge(ui);
});
```

### Sign convention

Positive `dx` / `dy` always **inset toward the center**, regardless of which
corner or edge you anchored to. Internally the offset becomes a margin on the
anchoring column (`src/context/widgets/overlay.rs`,
`anchor_offset_to_margin`):

| Anchor axis | Positive value | Negative value |
|---|---|---|
| Left / right edge (`*Left`, `*Right`) | insets by exactly `dx` cells | clamped to 0 (would push off-screen) |
| Top / bottom edge (`Top*`, `Bottom*`) | insets by exactly `dy` rows | clamped to 0 |
| Centered axis (`*Center`, `Center*`) | shifts right / down | shifts left / up |

On a **centered** axis the offset shrinks the area the content is centered in,
so the visible shift is about **half** the value, rounded (measured on a 40×11
`TestBackend`: `Anchor::Center` with `(4, 2)` moves a badge 2 cells right and
1 row down; `(-4, -2)` moves it 2 left and 1 up). Use even values on centered
axes when you need an exact shift.

To escape an edge in the other direction, pick a different anchor instead of a
negative offset.

### Manual margins still work

Nesting a margin container inside `overlay_at` produces the same position for a
single child, and is useful when you also want padding, borders, or a
background on the wrapper:

```rust
ui.overlay_at(Anchor::BottomRight, |ui| {
    ui.container().mr(2).mb(1).col(|ui| {
        render_badge(ui);
    });
});
```

Only the margin sides that face the **opposite** edge of your anchor have
visible effect: for `BottomRight` use `mr` and `mb`; for `TopLeft` use `ml`
and `mt`. `overlay_at_offset` picks the right sides for you, so prefer it.

---

## 4. When NOT to use overlay_at

Most layout should be flow-based (`ui.row` / `ui.col`), not overlays. Overlays
exist for content that floats independently of the document flow.

| Use case | Right tool | Why |
|---|---|---|
| Toolbar at top of app | `ui.col(\|ui\| { toolbar(ui); main(ui); })` | Part of layout flow |
| Sidebar on the left | `ui.row(\|ui\| { sidebar(ui); main(ui); })` | Part of layout flow |
| Floating "Save" badge that doesn't push content | `ui.overlay_at_offset(Anchor::BottomRight, 2, 1, ...)` | Floats above flow |
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
ui.overlay_at_offset(Anchor::BottomRight, 2, 1, |ui| {
    render_badge(ui);
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
ui.overlay_at_offset(Anchor::BottomRight, 2, 1, |ui| {
    render_badge(ui);
});
```

### From React Native

```jsx
<View style={{ position: 'absolute', bottom: 16, right: 16 }}>
  <Badge />
</View>
```

```rust
ui.overlay_at_offset(Anchor::BottomRight, 2, 1, |ui| {
    render_badge(ui);
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
  Use the typed entry points: `overlay_at` / `modal_at` (and their `_offset`
  variants). They internally apply
  `grow(1)` to the wrapper so flexbox has slack to push against. Hand-rolling
  with `align`/`justify` on a non-growing container will collapse.

- **"My margin doesn't shift the badge."** Manual margins only have visible
  effect on sides facing the **opposite** of your anchor. `Anchor::TopLeft` +
  `mr(2)` does nothing. Use `overlay_at_offset(Anchor::TopLeft, 2, 0, ..)` (or
  `ml` / `mt`) instead.

- **"My negative offset is ignored."** On edge and corner anchors negative
  `dx` / `dy` are clamped to 0. Negative values only move content on centered
  axes. Pick the opposite anchor instead.

- **"My centered offset moved half as far."** Expected: on centered axes the
  offset narrows the centering area, so the shift is about `value / 2`.

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

- **API reference**: `Anchor` is re-exported from the crate root (`src/lib.rs`);
  `overlay_at`, `overlay_at_offset`, `modal_at`, `modal_at_offset` are
  `Context` methods
- **Implementation**: `src/context/widgets/overlay.rs` (`Anchor` enum,
  `anchor_to_align_justify`, `anchor_offset_to_margin`, `overlay_at`,
  `modal_at`, `overlay_at_offset`, `modal_at_offset`)
- **Container margin helpers**: `ml`, `mt`, `mr`, `mb`, `mx`, `my`, `margin` in
  `src/context/container.rs`
- **Runnable demo**: `cargo run --example system_tour --features async` —
  the overlay-anchor tab (`examples/demo_overlay_anchor.rs`) renders all 9
  anchors with both `overlay_at` and `overlay_at_offset(anchor, 2, 1, ..)`
- **Cookbook**: `docs/COOKBOOK.md` (Modal Confirmation with Toast, Real-time Dashboard)
- **Reference**: `docs/COMPLETE_REFERENCE.md` — full API surface

---

## 8. TL;DR

```rust
use slt::{Anchor, Context};

let mut show_dialog = true;
slt::run(|ui: &mut Context| {
    // Flow content
    ui.col(|ui| {
        ui.text("Main content here");
    });

    // Floating badge in the bottom-right with a 2x1 cell inset
    ui.overlay_at_offset(Anchor::BottomRight, 2, 1, |ui| {
        ui.text("Saved").dim();
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

That's the whole positioning system. Nine anchors, four entry points
(`overlay_at` / `modal_at` for zero inset, `overlay_at_offset` /
`modal_at_offset` for a `(dx, dy)` cell inset), and standard margin helpers
when the wrapper also needs padding or a border.
