# Terminal Compatibility

SLT runs anywhere crossterm runs, but terminals and multiplexers differ in
what they support. This page lists what each feature needs, how SLT decides,
what happens when the feature is missing, and how to diagnose problems.

<!-- slt-check-default: illustrative fragments require application context -->

## How much of this is verified

Be precise about what "works in X" means here:

| Level | What it covers |
|-------|----------------|
| **Verified in CI** | Real tmux (3.2 and the current Ubuntu package), Zellij (0.43.1 and 0.45.0), and GNU screen (Ubuntu package) sessions on Ubuntu. Each run checks startup with default and forced queries, resize, bracketed paste, Tab, Escape, focus and SGR mouse sequence parsing, suspend/resume, and Ctrl+C cleanup. macOS and Windows CI runs `cargo test` only. |
| **Verified manually** | tmux 3.5a, Zellij 0.43.1, and GNU screen were run by hand during the v0.25 audit (including the tmux clipboard behavior below). |
| **Expected** | Everything else, including every GUI terminal listed below. Expectations come from what each terminal advertises and from SLT's detection rules, not from a physical test. |

If you confirm or refute an "expected" entry, an issue with your terminal,
version, and the output of the diagnostics below is very welcome.

## Feature matrix

| Feature | Requires | How SLT decides | When unavailable |
|---------|----------|-----------------|------------------|
| 24-bit color | Terminal truecolor support | `COLORTERM=truecolor` or `24bit` in the environment, or `RunConfig::color_depth` | Colors are downsampled to 256 or 16 colors |
| 256 colors | `TERM` containing `256color` | Environment only | Downsampled to the 16 ANSI colors |
| No color | `NO_COLOR` set to any non-empty value | Environment (wins over everything else) | n/a: no color codes are emitted |
| Mouse (click, hover, wheel, drag) | SGR mouse reporting | Opt-in: `RunConfig::default().mouse(true)`. Enables all-motion tracking | Keyboard focus (Tab) and activation (Enter/Space) still work |
| Text selection + copy | Mouse on, OSC 52 | Built in while the mouse is on; copies on release. Widget drags (split handles, scrollbar thumbs) never select. `RunConfig::text_selection(false)` turns it off | Native terminal selection (mouse off, or the terminal's bypass modifier) |
| Clipboard write | OSC 52 | `ui.copy_to_clipboard(..)` always emits it | Silently ignored by the terminal |
| Hyperlinks | OSC 8 | `ui.link(..)` always emits it, and also opens the URL with the local system opener on click or Enter | Plain styled text; the local opener still runs |
| Bracketed paste | Bracketed paste mode | Always requested | Pasted text arrives as individual key presses |
| Focus in/out events | Focus reporting | Always requested | No `FocusGained` / `FocusLost` events |
| Synchronized output | DEC mode 2026 | Probed on identified direct terminals; off in multiplexers | Frames are written without the sync wrapper; fast redraws may tear |
| Key release, modifier-only keys, disambiguated keys | Kitty keyboard protocol | Opt-in: `RunConfig::kitty_keyboard(true)` (+ `report_all_keys(true)` for bare modifiers) | Press-only events; some combinations (such as Ctrl+I vs Tab) are indistinguishable |
| Images | Kitty graphics, Sixel, or iTerm2 inline images (`image` feature) | Probe, then known terminal identity; preference Kitty, then Sixel, then iTerm2 | Half-block cell art |
| Light/dark background | OSC 11 reply | `slt::detect_color_scheme()`, called before `run`; subject to the query rules below | Returns `ColorScheme::Unknown` |
| Window title | OSC 2 | `RunConfig::title(..)` | Ignored |
| Clean Ctrl+Z | Unix job control | `RunConfig::handle_suspend` (default on, Unix only) | Ctrl+Z arrives as a key event (Windows) |
| Scrollback | Normal screen | `run` / `run_with` use the alternate screen, which hides scrollback | Use `run_inline` / `run_static` for CLI-style output that stays in scrollback |

Color depth is decided from the environment only. SLT also probes truecolor
support on some terminals (visible in `ui.capabilities().truecolor`), but the
probe result does not change the output depth.

## Terminals

Everything in this table is **expected** behavior, not verified on hardware.
"Detected colors" assumes the terminal's default environment; launching from
`sudo`, a service manager, or SSH can drop `COLORTERM` and lower the result.

| Terminal | Detected colors | Images | Kitty keyboard | Notes |
|----------|-----------------|--------|----------------|-------|
| Kitty | 24-bit (sets `COLORTERM`) | Kitty graphics | Yes | `TERM=xterm-kitty` alone, without `COLORTERM`, is detected as 16 colors |
| Ghostty | 24-bit (sets `COLORTERM`) | Kitty graphics | Yes | `TERM=xterm-ghostty` alone is detected as 16 colors |
| WezTerm | 24-bit (sets `COLORTERM`) | Kitty graphics (also Sixel / iTerm2 capable) | When enabled in WezTerm's config | |
| iTerm2 | 24-bit (sets `COLORTERM`) | iTerm2 inline images | Unknown: check with `demo_key_test --kitty` | Option-as-Meta setting affects Alt shortcuts |
| Alacritty | 24-bit (sets `COLORTERM`) | None (half-block) | Recent versions | |
| Apple Terminal.app | 256 (`TERM=xterm-256color`, no `COLORTERM`) | None (half-block) | No | |
| Windows Terminal | Usually 16: `TERM` and `COLORTERM` are typically unset | Half-block (no probing on Windows) | No | Set `color_depth` explicitly; see Windows notes |
| VS Code integrated terminal | 24-bit (sets `COLORTERM`) | Half-block (not identified) | Unknown | Image support in VS Code depends on its settings; SLT does not auto-detect it |

## Multiplexers

tmux and GNU screen sit between SLT and the real terminal and do not forward
most capability probes, so SLT turns probing off inside them. Zellij is
treated more permissively.

| Environment | Replies / probes | Sync output | Kitty graphics | Sixel | iTerm2 images | Kitty keyboard |
|-------------|------------------|-------------|----------------|-------|---------------|----------------|
| Direct terminal (identified) | Probed once at startup | Probe result | Probe or known identity | Probe or known identity | Known identity | Allowed when requested |
| tmux 3.2+ | Off | Off | Off | Off | Off | Off |
| Zellij 0.43.1+ | Off | Off | Off | On | Off | Allowed when requested |
| GNU screen 4.09+ | Off | Off | Off | Off | Off | Off |
| SSH session | Direct-terminal rules, but a generic `TERM=xterm-256color` is not enough to start probes | Direct-terminal policy | Outer identity not trusted | Outer identity not trusted | Off unless forced | Depends on the remote endpoint |
| Unknown PTY bridge | Off | Historical default | Off | Off | Off | Not negotiated |

Practical consequences:

- **Colors inside tmux / screen** follow the `TERM` and `COLORTERM` they set,
  not those of your outer terminal. tmux usually sets `TERM=tmux-256color` or
  `screen-256color` (256 colors). For 24-bit color, export
  `COLORTERM=truecolor` in the session (and enable RGB in tmux, for example
  `set -as terminal-features ',xterm-256color:RGB'`), or set
  `RunConfig::color_depth`.
- **Mouse** works through tmux, Zellij, and screen when the multiplexer itself
  has mouse mode on (`set -g mouse on` in tmux, `mousetrack on` in screen;
  Zellij enables it by default). Multiplexer key bindings and selection may
  take precedence over the app.
- **Clipboard through tmux** needs `set -g set-clipboard on`. With tmux's
  default (`external`), SLT's OSC 52 did not reach a tmux buffer or the outer
  terminal in the v0.25 tests (tmux 3.5a); with `on` it reached both. SLT does
  not wrap OSC 52 in a tmux passthrough sequence.
- **Images and kitty keyboard** are off in tmux and screen. The force flags
  below exist for setups with working passthrough, but they are escape
  hatches, not a promise that the protocol works there.
- **Emoji widths** can disagree between the multiplexer, the outer terminal,
  and SLT for ZWJ sequences, skin tones, and VS16 emoji, which can leave stray
  cells on that row.

## SSH

- `COLORTERM` is often not forwarded by SSH, so a remote app usually sees
  only `TERM=xterm-256color` (256 colors). Either add `SendEnv COLORTERM` on
  the client and `AcceptEnv COLORTERM` on the server, export
  `COLORTERM=truecolor` remotely, or set `RunConfig::color_depth`.
- With the mouse on, every pointer move is sent over the connection. On slow
  links, consider leaving the mouse off.
- OSC 52 copy goes to the **local** terminal's clipboard, which is usually
  what you want over SSH (if the terminal allows OSC 52).
- `ui.link(..)` activation runs the system opener on the **remote** host. Over
  SSH that usually does nothing useful; rely on the OSC 8 hyperlink instead.
- Image identity inherited from the outer terminal is not trusted remotely, so
  images fall back to half-block unless forced.

## Environment variables

| Variable | Effect |
|----------|--------|
| `NO_COLOR` | Any non-empty value disables color output |
| `COLORTERM` | `truecolor` or `24bit` selects 24-bit color |
| `TERM` | Contains `256color` selects 256 colors; also used for terminal identity |
| `SLT_DISABLE_TERMINAL_QUERIES` / `SLT_FORCE_TERMINAL_QUERIES` | Turn all startup probes off / on |
| `SLT_DISABLE_SYNC_OUTPUT` / `SLT_FORCE_SYNC_OUTPUT` | Synchronized output |
| `SLT_DISABLE_KITTY` / `SLT_FORCE_KITTY` | Kitty graphics |
| `SLT_DISABLE_SIXEL` / `SLT_FORCE_SIXEL` | Sixel graphics |
| `SLT_DISABLE_ITERM` / `SLT_FORCE_ITERM` | iTerm2 inline images |
| `SLT_DISABLE_KITTY_KEYBOARD` / `SLT_FORCE_KITTY_KEYBOARD` | Kitty keyboard protocol |

`SLT_*` flags accept `1`, `true`, `yes`, or `on`. When both the disable and
the force flag for a protocol are set, **disable wins**.

To pin color depth in code, independent of the environment:

<!-- slt-check: {"name":"tc_color_depth","features":["crossterm"],"context":"module"} -->
```rust
use slt::{ColorDepth, Context, RunConfig};

fn main() -> std::io::Result<()> {
    // Example policy: trust Windows Terminal with 24-bit color, otherwise
    // use the normal environment detection.
    let depth = if std::env::var_os("WT_SESSION").is_some() {
        ColorDepth::TrueColor
    } else {
        ColorDepth::detect()
    };

    slt::run_with(RunConfig::default().color_depth(depth), |ui: &mut Context| {
        ui.text("colors").fg(slt::Color::Rgb(255, 140, 0));
        if ui.key('q') {
            ui.quit();
        }
    })
}
```

## Windows

- Raw terminal capability queries are disabled on Windows, so there is no
  probing for synchronized output, Kitty graphics, or Sixel. SLT falls back
  to conservative defaults: half-block images, press-only key events.
- Windows Terminal does not usually set `TERM` or `COLORTERM`, so detection
  lands on 16 colors. Set `RunConfig::color_depth(ColorDepth::TrueColor)` (it
  supports 24-bit color) or export `COLORTERM=truecolor`.
- Ctrl+Z suspend handling is Unix-only; on Windows it is a normal key event.
- Mouse and focus work through crossterm's console input. They are
  **expected** to work but are not verified beyond `cargo test` in CI.

## Troubleshooting

Two diagnostic examples ship with the crate:

- `cargo run --example demo_key_test` runs SLT with the mouse on and logs a
  fixed set of inputs: Tab, Shift+Tab, Esc, F1, F2, Ctrl+T, Ctrl+C, and a
  click on the mode badge. It also has a text input for typing tests. Add
  `-- --kitty` to request the Kitty keyboard protocol.
- `cargo run --example mouse_probe` bypasses SLT and prints raw crossterm mouse
  events, to tell a terminal problem from an app problem.

For any other key, log every event SLT receives:

<!-- slt-check: {"name":"tc_event_log","features":["crossterm"],"context":"module"} -->
```rust
use slt::{Context, KeyCode, RunConfig};

fn main() -> std::io::Result<()> {
    let mut log: Vec<String> = Vec::new();
    let config = RunConfig::default().mouse(true).kitty_keyboard(true);

    slt::run_with(config, |ui: &mut Context| {
        let batch: Vec<String> = ui.raw_events().map(|e| format!("{e:?}")).collect();
        log.extend(batch);
        if log.len() > 200 {
            log.drain(..log.len() - 200);
        }
        for line in log.iter().rev().take(30) {
            ui.text(line);
        }
        if ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
    })
}
```

For capabilities, render `ui.capabilities()` in a debug panel:

<!-- slt-check: {"name":"tc_capabilities","features":["crossterm"],"context":"ui"} -->
```rust
let caps = *ui.capabilities();
ui.text(format!(
    "truecolor={} kitty_graphics={} sixel={} iterm2={} kitty_keyboard={} sync={}",
    caps.truecolor,
    caps.kitty_graphics,
    caps.sixel,
    caps.iterm2,
    caps.kitty_keyboard,
    caps.sync_output,
));
```

### The mouse does nothing

1. Did you enable it? `RunConfig::default().mouse(true)`. It is off by
   default.
2. Run `mouse_probe`. If it prints no mouse events, the terminal or
   multiplexer is not reporting them: check multiplexer mouse mode
   (`set -g mouse on` in tmux). If `mouse_probe` works but your app does not,
   use the event logger above to see what reaches SLT.
3. Clicks arrive but hit the wrong widget: hit-testing uses last frame's
   layout, so the first frame after a layout change still sees the old
   geometry. See [PREVIOUS_FRAME_GUIDE.md](PREVIOUS_FRAME_GUIDE.md).
4. Clicks fire on press and also reach clickable parents or widgets under a
   non-modal overlay. See the known limitations in
   [INTERACTION.md](INTERACTION.md).

### Colors look wrong or washed out

1. Check `echo $COLORTERM $TERM`. No `truecolor` means RGB colors are
   downsampled to 256 or 16 colors.
2. Inside tmux / screen or over SSH, `COLORTERM` is often missing. Export it
   or set `RunConfig::color_depth`.
3. `NO_COLOR` set anywhere in the environment disables color completely.

### Images are missing or blocky

1. Build with the `image` feature.
2. Blocky (half-block) output means no graphics protocol was selected. Inside
   tmux / screen, over SSH, on Windows, and in terminals SLT does not
   recognize, that is the expected fallback.
3. If you know your path supports a protocol (for example tmux with
   passthrough configured), try `SLT_FORCE_KITTY=1` or `SLT_FORCE_SIXEL=1`.
   If output becomes garbled, unset it.

### Keys are missing or merged

1. Run the event logger above and press the key. If nothing shows up, the
   terminal or multiplexer is eating it (common for Ctrl+Tab, Ctrl+Shift
   combos, and multiplexer prefixes).
2. Key releases and bare modifiers need `kitty_keyboard(true)` (plus
   `report_all_keys(true)` for bare modifiers), a supporting terminal, and no
   tmux/screen in between. `demo_key_test -- --kitty` shows whether Tab and
   Shift+Tab still arrive with the protocol on.
3. Alt shortcuts on macOS depend on the terminal's Option-as-Meta setting.
4. A shortcut that fires while typing in a text field is an ordering bug in
   the app: check shortcuts after rendering widgets
   ([INTERACTION.md](INTERACTION.md)).

### Emoji or wide characters misalign

1. The terminal (or multiplexer) and SLT disagree on the character's width.
   This mostly affects ZWJ sequences, skin-tone modifiers, and VS16 emoji.
2. Try the app outside the multiplexer. If it is fine there, the multiplexer
   is the one measuring differently.
3. For UI chrome, prefer characters with unambiguous widths (ASCII, box
   drawing, common symbols) over emoji.

### Copy does not reach the clipboard

1. The terminal must allow OSC 52. Some disable it by default or only allow
   writes (check the terminal's clipboard or security settings).
2. Inside tmux, set `set -g set-clipboard on`.
3. With the mouse on, native selection is replaced by SLT's selection
   overlay. Hold the terminal's bypass modifier (often Shift, Option on macOS)
   to select natively, or turn the mouse off.

## Related docs

- [BACKENDS.md](BACKENDS.md) - backend contract, compatibility policy, CI matrix details
- [FEATURES.md](FEATURES.md) - feature flags and terminal query safety
- [INTERACTION.md](INTERACTION.md) - mouse, focus, and known limitations
- [DEBUGGING.md](DEBUGGING.md) - debugging techniques
