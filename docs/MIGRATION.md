# SLT Migration Guide

A version-by-version upgrade guide for SuperLightTUI users. Pre-1.0, minor versions
(`0.X.0`) may contain breaking changes; patch versions (`0.X.Y`) must not.

For the full per-release notes, see `CHANGELOG.md`. For composition recipes that use
current APIs, see `docs/PATTERNS.md` and `docs/COOKBOOK.md`.

---

## 1. v0.22.x → v0.23.0

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

1. Set `superlighttui = "0.23.0"`.
2. Run `cargo check` and replace direct state-field mutation first.
3. Update async ownership and slider call sites reported by the compiler.
4. Run interaction tests for tables, selects, text input, and raw drawing.
5. Re-record visual snapshots containing CJK, emoji, RTL text, images, or clipped scroll regions.

---

## 2. v0.19.x → v0.19.3 (deprecations only — no code break)

These compiled fine in v0.19.x but now produce deprecation warnings. They will be
**removed in v0.20**:

| Old | New |
|---|---|
| `pad(n)` | `p(n)` |
| `min_width(n)` | `min_w(n)` |
| `max_width(n)` | `max_w(n)` |
| `min_height(n)` | `min_h(n)` |
| `max_height(n)` | `max_h(n)` |

Source-of-truth: `src/context/container.rs` lines ~1050–1069 carry the
`#[deprecated(since = "0.20.0", ...)]` attributes; `pad` is at ~835.

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
doesn't have an equivalent for SLT yet — see section 5.

---

## 3. v0.18.x → v0.19.0 (additive, no break)

v0.19.0 added component DX APIs in `src/context/runtime.rs` (`provide`, `use_context`,
`use_state_named`, `with_if`). All existing code keeps compiling. Adopt incrementally:

| Old pattern | New pattern (v0.19.0+) |
|---|---|
| Threading `&theme, &tick, &mut state` parameters through every render fn | `ui.provide(value, \|ui\| ...)` once, then `ui.use_context::<T>()` in nested fns |
| `use_state` inside `if`/`match` (panics on hook-order mismatch) | `use_state_named(id)` (id-keyed, safe in conditionals) |
| `if cond { t.bold(); t.fg(Color::Red); }` | `t.with_if(cond, \|t\| t.bold().fg(Color::Red))` |

See `docs/PATTERNS.md` for the full set of composition patterns.

---

## 4. v0.17.x → v0.18.x

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

## 5. Comparison: how other UI libraries handle migrations

| Framework | Tooling | Convention |
|---|---|---|
| **React** | `npx jscodeshift` AST codemods + RFC | Major every 2–3 years; deprecation warnings ≥1 minor before removal |
| **Vue** | `vue-codemod` AST tool + composition API guide | Major: 3.x → vue-next migration build |
| **Angular** | `ng update` schematic CLI (auto-migration) | Strict semver, deprecation warnings, automatic migration |
| **Flutter** | `dart fix --apply` | Annotated deprecations migrate via `dart fix` |
| **SLT** | sed-based codemods (no AST tool yet) + this guide + deprecation warnings | Strict semver, deprecation warnings ≥1 minor before removal in v0.x |

SLT does **not** yet have an AST-aware codemod tool. PRs welcome — `cargo-slt-migrate`
would be a high-impact contribution. Until then, the sed snippets in section 2 plus
`cargo build` errors are the migration toolchain.

---

## 6. Pre-1.0 versioning policy

- **Pre-1.0 (current)**: minor versions (`0.X.0`) MAY contain breaking changes; patch
  versions (`0.X.Y`) MUST NOT.
- **Post-1.0 (TBD)**: standard semver — major for breaking, minor for features, patch
  for fixes.
- CI runs `cargo-semver-checks` on every PR to verify patch releases stay non-breaking.
  Minor releases are allowed to fail this check; major/minor breakage is documented in
  the release notes and reflected in this file.

Deprecation policy: every API removal in a minor release is preceded by at least one
minor release that emits `#[deprecated(since = "X.Y.Z", note = "...")]` warnings. So if
you keep `cargo build` clean of deprecation warnings before each minor bump, you should
not hit surprise removals.

---

## 7. Reporting migration issues

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

## 8. Cross-references

- `CHANGELOG.md` — full per-release notes with every commit
- `docs/COOKBOOK.md` — recipes using current APIs
- `docs/PATTERNS.md` — composition patterns (`provide` / `use_context` / `with_if`)
- `docs/COMPLETE_REFERENCE.md` — full API surface
- `docs/STATE_APIS.md` — state-type method reference
