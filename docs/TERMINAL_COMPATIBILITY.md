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
| **Verified manually** | tmux 3.5a, Zellij 0.43.1, and GNU screen were run by hand during the v0.25 audit. For v0.26, tmux 3.5a was rerun for OSC 52 clipboard delivery (with and without passthrough) and for `ui.set_title` (results below). |
| **Verified in a scripted PTY** | `tests/terminal_caps_pty.rs` plays the terminal in a real Unix PTY and checks the startup probe bytes: the Kitty keyboard push with and without a `CSI ? u` reply, probe-confirmed truecolor, title / progress / cursor-shape emission and teardown, `TERM=dumb` suppression, OSC 52 output, and the width probe. |
| **Expected** | Everything else, including every GUI terminal listed below. Expectations come from what each terminal advertises and from SLT's detection rules, not from a physical test. |

If you confirm or refute an "expected" entry, an issue with your terminal,
version, and the output of the diagnostics below is very welcome.

## Feature matrix

| Feature | Requires | How SLT decides | When unavailable |
|---------|----------|-----------------|------------------|
| 24-bit color | Terminal truecolor support | `COLORTERM=truecolor` or `24bit`; a probe-confirmed XTGETTCAP `Tc` reply; terminal identity (`TERM` = `xterm-kitty`, `xterm-ghostty`, `wezterm`, or `TERM_PROGRAM` = `iTerm.app`, `WezTerm`, `ghostty`, `vscode`); a VT-enabled Windows console (Windows 10+, or `WT_SESSION`). `RunConfig::color_depth` overrides all of it | Colors are downsampled to 256 or 16 colors |
| 256 colors | `TERM` containing `256color` | Environment, when none of the 24-bit signals above apply | Downsampled to the 16 ANSI colors |
| No color | `NO_COLOR` set to any non-empty value, or `TERM=dumb` | Environment; wins over every detected signal. Only an explicit `RunConfig::color_depth` overrides it | n/a: no color codes are emitted |
| Mouse (click, hover, wheel, drag) | SGR mouse reporting | Opt-in: `RunConfig::default().mouse(true)`. Enables all-motion tracking | Keyboard focus (Tab) and activation (Enter/Space) still work |
| Text selection + copy | Mouse on, OSC 52 | Built in while the mouse is on; copies on release | Native terminal selection (mouse off, or the terminal's bypass modifier) |
| Clipboard write | OSC 52 | `ui.copy_to_clipboard(..)` writes it after the frame. Inside tmux / screen it is wrapped for passthrough only with `RunConfig::clipboard_passthrough(true)`. Payloads over `RunConfig::clipboard_max_bytes` (default 74,994 bytes) are not sent. `ui.clipboard_status()` reports `Sent`, `TooLarge`, or `WriteFailed` for one frame | Ignored by the terminal; `Sent` only means "written", OSC 52 has no acknowledgement |
| Hyperlinks | OSC 8 | `ui.link(..)` always emits it, and also opens the URL with the local system opener on click or Enter | Plain styled text; the local opener still runs |
| Bracketed paste | Bracketed paste mode | Always requested | Pasted text arrives as individual key presses |
| Focus in/out events | Focus reporting | Always requested | No `FocusGained` / `FocusLost` events |
| Synchronized output | DEC mode 2026 | Probed on identified direct terminals; off in multiplexers | Frames are written without the sync wrapper; fast redraws may tear |
| Key release, modifier-only keys, disambiguated keys | Kitty keyboard protocol | Opt-in: `RunConfig::kitty_keyboard(true)` (+ `report_all_keys(true)` for bare modifiers). Where startup queries run, SLT sends `CSI ? u` and pushes the protocol only when the terminal answers (`ui.capabilities().kitty_keyboard`) or `SLT_FORCE_KITTY_KEYBOARD` is set. Where queries are skipped (Zellij, Windows, unidentified PTYs) the multiplexer policy alone decides. Never pushed in tmux / screen unless forced | Press-only events; some combinations (such as Ctrl+I vs Tab) are indistinguishable |
| Images | Kitty graphics, Sixel, or iTerm2 inline images (`image` feature) | Probe, then known terminal identity; preference Kitty, then Sixel, then iTerm2 | Half-block cell art |
| Light/dark background | OSC 11 reply | `slt::detect_color_scheme()`, called before `run`; subject to the query rules below | Returns `ColorScheme::Unknown` |
| Window title | OSC 2 | `RunConfig::title(..)` sets the initial title; `ui.set_title(..)` changes it at runtime. Written only when it changes, with control characters replaced. The previous title is saved with XTWINOPS `22;2` and restored with `23;2` on exit | Ignored (and no restore on terminals without a title stack) |
| Taskbar / tab progress | OSC 9;4 | `ui.set_progress(Some(Progress::Normal(0.4)))`, written on change. Sent only to known hosts (Windows Terminal via `WT_SESSION`, Ghostty, ConEmu with `ConEmuANSI=ON`, iTerm2 3.6+) or with `SLT_FORCE_PROGRESS`; never inside tmux / screen / Zellij. Cleared on exit | Not sent: older terminals treat OSC 9 as a desktop notification |
| Program status | OSC 7501 | `ui.set_status(Some(ProgramStatus::new(ProgramState::Working)))`, written on change, direct terminals only (`SLT_FORCE_PROGRAM_STATUS` / `SLT_DISABLE_PROGRAM_STATUS`). On exit, `idle` / `working` / `blocked` records are cleared; `done` / `error` are left for the terminal to keep | Ignored by the terminal |
| Cursor shape | DECSCUSR (`CSI n SP q`) | `ui.set_cursor_shape(CursorShape::SteadyBar)`, written on change; reset to the terminal default on exit | Ignored |
| East Asian Ambiguous width | Terminal configured for wide ambiguous characters | Opt-in: `RunConfig::ambiguous_wide(true)` measures `→±①` as two columns each in the buffer, wrapping, and truncation | One column (the Unicode default) |
| Emoji clusters measured differently | n/a | `RunConfig::width_conservative(..)`: on by default under tmux / screen / Zellij; `RunConfig::width_probe(true)` measures VS16, ZWJ, skin-tone, and flag samples with `CSI 6 n` at startup (fullscreen, direct terminals only) and enables it on a mismatch. The policy is resolved today; the explicit cursor reposition in the diff flush lands with #441 | Rows after a wrongly measured cluster can shift on incremental updates |
| Clean Ctrl+Z | Unix job control | `RunConfig::handle_suspend` (default on, Unix only) | Ctrl+Z arrives as a key event (Windows) |
| Scrollback | Normal screen | `run` / `run_with` use the alternate screen, which hides scrollback | Use `run_inline` / `run_static` for CLI-style output that stays in scrollback |

Color depth precedence, when `RunConfig::color_depth` is not set:
`NO_COLOR` or `TERM=dumb` (no color), then `COLORTERM` or a probe-confirmed
`Tc` reply (24-bit), then terminal identity (24-bit), then a VT-enabled
Windows console (24-bit), then `TERM` containing `256color` (256), otherwise
16 colors. `ColorDepth::detect()` applies the same chain from the environment
alone; `ColorDepth::resolve(&slt::capabilities())` adds the probe. An explicit
`RunConfig::color_depth` always wins, including over `NO_COLOR`.

Title, progress, program status, and cursor shape are written by the built-in
terminal backends only, never for `TERM=dumb` or a non-terminal stdout, and
never by custom backends or `TestBackend` (where the getters
`ui.terminal_title()`, `ui.terminal_progress()`, `ui.program_status()`, and
`ui.cursor_shape()` expose the recorded values). Idle frames add no bytes.

## Terminals

Everything in this table is **expected** behavior, not verified on hardware.
"Detected colors" assumes the terminal's default environment; launching from
`sudo`, a service manager, or SSH can drop `COLORTERM` and lower the result.

| Terminal | Detected colors | Images | Kitty keyboard | Notes |
|----------|-----------------|--------|----------------|-------|
| Kitty | 24-bit (`COLORTERM`, or `TERM=xterm-kitty` alone) | Kitty graphics | Yes (confirmed by `CSI ? u`) | |
| Ghostty | 24-bit (`COLORTERM`, or `TERM=xterm-ghostty` / `TERM_PROGRAM=ghostty`) | Kitty graphics | Yes (confirmed by `CSI ? u`) | Receives OSC 9;4 progress |
| WezTerm | 24-bit (`COLORTERM`, or `TERM=wezterm` / `TERM_PROGRAM=WezTerm`) | Kitty graphics (also Sixel / iTerm2 capable) | When enabled in WezTerm's config | |
| iTerm2 | 24-bit (`COLORTERM`, or `TERM_PROGRAM=iTerm.app`) | iTerm2 inline images | Unknown: check with `demo_key_test --kitty` | Option-as-Meta setting affects Alt shortcuts; OSC 9;4 progress from 3.6 |
| Alacritty | 24-bit (sets `COLORTERM`) | None (half-block) | Recent versions | |
| Apple Terminal.app | 256 (`TERM=xterm-256color`, no `COLORTERM`) | None (half-block) | No | |
| Windows Terminal | 24-bit (VT-enabled console, `WT_SESSION`) | Half-block (no probing on Windows) | Not negotiated: with `kitty_keyboard(true)` it is pushed without a probe | Receives OSC 9;4 progress; see Windows notes |
| VS Code integrated terminal | 24-bit (`COLORTERM`, or `TERM_PROGRAM=vscode`) | Half-block (not identified) | Unknown | Image support in VS Code depends on its settings; SLT does not auto-detect it |

## Multiplexers

tmux and GNU screen sit between SLT and the real terminal and do not forward
most capability probes, so SLT turns probing off inside them. Zellij is
treated more permissively.

| Environment | Replies / probes | Sync output | Kitty graphics | Sixel | iTerm2 images | Kitty keyboard |
|-------------|------------------|-------------|----------------|-------|---------------|----------------|
| Direct terminal (identified) | Probed once at startup | Probe result | Probe or known identity | Probe or known identity | Known identity | Pushed when requested and confirmed by `CSI ? u` |
| tmux 3.2+ | Off | Off | Off | Off | Off | Off |
| Zellij 0.43.1+ | Off | Off | Off | On | Off | Pushed when requested (policy only, no probe) |
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
- **tmux detection**: tmux is recognized from `TMUX` even when its
  `default-terminal` is `screen-256color`, as long as `STY` (GNU screen) is
  not also set.
- **Clipboard through tmux** needs one of two setups. Either
  `set -g set-clipboard on` (tmux stores the copy and forwards it), or
  `RunConfig::clipboard_passthrough(true)` plus `set -g allow-passthrough on`,
  which wraps OSC 52 as `ESC P tmux; ... ESC \` with every inner `ESC` doubled
  and bypasses tmux's clipboard handling. Under GNU screen, passthrough splits
  the sequence into DCS chunks. Passthrough is off by default because it
  bypasses the multiplexer's security settings. Measured with tmux 3.5a and an
  attached client playing the outer terminal:

  | tmux configuration | tmux buffer | Outer terminal received OSC 52 |
  |--------------------|-------------|--------------------------------|
  | default (`set-clipboard external`) | none | no |
  | `set -g set-clipboard on` | `SLT-OSC52-PROBE` | yes |
  | `clipboard_passthrough(true)` + `allow-passthrough on` | none | yes |
  | `clipboard_passthrough(true)` + `allow-passthrough off` | none | no |

- **Window title through tmux**: with tmux 3.5a, `#{pane_title}` showed the
  `ui.set_title` value while the app ran and returned to the previous title
  after exit.
- **Progress and program status** are not sent inside tmux, screen, or Zellij.
  Title and cursor shape are, because multiplexers handle both themselves.
- **Images and kitty keyboard** are off in tmux and screen. The force flags
  below exist for setups with working passthrough, but they are escape
  hatches, not a promise that the protocol works there.
- **Emoji widths** can disagree between the multiplexer, the outer terminal,
  and SLT for ZWJ sequences, skin tones, and VS16 emoji, which can leave stray
  cells on that row. Width-conservative mode is on by default under
  multiplexers; the flush-side cursor reposition it drives lands with #441.

## SSH

- `COLORTERM` is often not forwarded by SSH, so a remote app usually sees
  only `TERM=xterm-256color`. A generic `TERM` does not start probes over SSH,
  so the result is usually 256 colors. When probes do run (an identified
  `TERM` such as `xterm-kitty`, or `SLT_FORCE_TERMINAL_QUERIES=1`), a `Tc`
  reply to XTGETTCAP selects 24-bit color. Otherwise add `SendEnv COLORTERM`
  on the client and `AcceptEnv COLORTERM` on the server, export
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
| `TERM` | `dumb` disables color and all title / progress / status / cursor-shape sequences; contains `256color` selects 256 colors; also used for terminal identity |
| `TERM_PROGRAM` / `TERM_PROGRAM_VERSION` | Terminal identity: truecolor (`iTerm.app`, `WezTerm`, `ghostty`, `vscode`), OSC 9;4 progress hosts (`ghostty`, `iTerm.app` 3.6+) |
| `WT_SESSION` / `ConEmuANSI` | Windows Terminal / ConEmu identity: 24-bit color (Windows Terminal) and OSC 9;4 progress |
| `SLT_DISABLE_TERMINAL_QUERIES` / `SLT_FORCE_TERMINAL_QUERIES` | Turn all startup probes off / on |
| `SLT_DISABLE_SYNC_OUTPUT` / `SLT_FORCE_SYNC_OUTPUT` | Synchronized output |
| `SLT_DISABLE_KITTY` / `SLT_FORCE_KITTY` | Kitty graphics |
| `SLT_DISABLE_SIXEL` / `SLT_FORCE_SIXEL` | Sixel graphics |
| `SLT_DISABLE_ITERM` / `SLT_FORCE_ITERM` | iTerm2 inline images |
| `SLT_DISABLE_KITTY_KEYBOARD` / `SLT_FORCE_KITTY_KEYBOARD` | Kitty keyboard protocol. Force also pushes without a `CSI ? u` reply; disable also clears `capabilities().kitty_keyboard` |
| `SLT_DISABLE_PROGRESS` / `SLT_FORCE_PROGRESS` | OSC 9;4 progress (force sends it to unknown hosts and multiplexers) |
| `SLT_DISABLE_PROGRAM_STATUS` / `SLT_FORCE_PROGRAM_STATUS` | OSC 7501 program status (force sends it inside multiplexers) |

`SLT_*` flags accept `1`, `true`, `yes`, or `on`. When both the disable and
the force flag for a protocol are set, **disable wins**.

To pin color depth in code, independent of the environment:

<!-- slt-check: {"name":"tc_color_depth","features":["crossterm"],"context":"module"} -->
```rust
use slt::{ColorDepth, Context, RunConfig};

fn main() -> std::io::Result<()> {
    // Example policy: an app-specific override wins; otherwise use the same
    // environment chain the run loop applies (without the startup probe).
    let depth = if std::env::var_os("MYAPP_TRUECOLOR").is_some() {
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

To report what a long-running app is doing in the terminal's tab or taskbar
(each setter persists until changed and is written only on change):

<!-- slt-check: {"name":"tc_terminal_meta","features":["crossterm"],"context":"ui"} -->
```rust
let done = 3;
let total = 10;
ui.set_title(format!("agent: running {done}/{total}"));
ui.set_progress(Some(Progress::Normal(done as f32 / total as f32)));
ui.set_status(Some(
    ProgramStatus::new(ProgramState::Working)
        .progress((done * 100 / total) as u8)
        .message("compiling"),
));
ui.set_cursor_shape(CursorShape::SteadyBar);
if let Some(ClipboardStatus::TooLarge { bytes, max }) = ui.clipboard_status() {
    ui.text(format!("copy skipped: {bytes} bytes > {max}"));
}
```

## Windows

- Raw terminal capability queries are disabled on Windows, so there is no
  probing for synchronized output, Kitty graphics, Sixel, or the Kitty
  keyboard protocol. SLT falls back to conservative defaults: half-block
  images, press-only key events unless `kitty_keyboard(true)` is set (then
  the push follows policy alone).
- A console with virtual-terminal processing enabled (Windows 10+, which
  includes Windows Terminal) is detected as 24-bit color even though `TERM`
  and `COLORTERM` are usually unset. Legacy conhost without VT stays at 16
  colors; set `RunConfig::color_depth` to pin a depth either way.
- Windows Terminal (`WT_SESSION`) receives OSC 9;4 taskbar progress.
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

1. Check `echo $COLORTERM $TERM $TERM_PROGRAM` and render
   `ui.capabilities().truecolor`. Without `truecolor` in `COLORTERM`, a `Tc`
   probe reply, or a known truecolor identity, RGB colors are downsampled to
   256 or 16 colors.
2. Inside tmux / screen or over SSH, `COLORTERM` is often missing and the
   probe does not run. Export it or set `RunConfig::color_depth`.
3. `NO_COLOR` set anywhere in the environment, or `TERM=dumb`, disables color
   completely unless `RunConfig::color_depth` is set explicitly.

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
   tmux/screen in between. On a direct terminal the protocol is pushed only
   when the terminal answers `CSI ? u`; `ui.capabilities().kitty_keyboard`
   shows the answer, and `SLT_FORCE_KITTY_KEYBOARD=1` pushes regardless.
   `demo_key_test -- --kitty` shows whether Tab and Shift+Tab still arrive
   with the protocol on.
3. Alt shortcuts on macOS depend on the terminal's Option-as-Meta setting.
4. A shortcut that fires while typing in a text field is an ordering bug in
   the app: check shortcuts after rendering widgets
   ([INTERACTION.md](INTERACTION.md)).

### Emoji or wide characters misalign

1. The terminal (or multiplexer) and SLT disagree on the character's width.
   This mostly affects ZWJ sequences, skin-tone modifiers, and VS16 emoji.
2. Try the app outside the multiplexer. If it is fine there, the multiplexer
   is the one measuring differently.
3. If arrows, `±`, or circled numbers are off by one column on a CJK-locale
   terminal set to "ambiguous width = wide", set
   `RunConfig::ambiguous_wide(true)`.
4. `RunConfig::width_probe(true)` measures VS16, ZWJ, skin-tone, and flag
   samples at startup on a direct fullscreen terminal and turns on
   width-conservative mode when one disagrees; `width_conservative(true)`
   forces it. The flush-side cursor reposition driven by this mode lands with
   #441.
5. For UI chrome, prefer characters with unambiguous widths (ASCII, box
   drawing, common symbols) over emoji.

### Copy does not reach the clipboard

1. Check `ui.clipboard_status()` on the frame after the copy. `TooLarge`
   means the text exceeded `RunConfig::clipboard_max_bytes` (default 74,994
   bytes) and nothing was sent; `WriteFailed` carries the I/O error kind.
   `Sent` only means the sequence was written.
2. The terminal must allow OSC 52. Some disable it by default or only allow
   writes (check the terminal's clipboard or security settings).
3. Inside tmux, set `set -g set-clipboard on`, or use
   `RunConfig::clipboard_passthrough(true)` with `set -g allow-passthrough on`
   (see the tmux table above).
4. With the mouse on, native selection is replaced by SLT's selection
   overlay. Hold the terminal's bypass modifier (often Shift, Option on macOS)
   to select natively, or turn the mouse off.

## Related docs

- [BACKENDS.md](BACKENDS.md) - backend contract, compatibility policy, CI matrix details
- [FEATURES.md](FEATURES.md) - feature flags and terminal query safety
- [INTERACTION.md](INTERACTION.md) - mouse, focus, and known limitations
- [DEBUGGING.md](DEBUGGING.md) - debugging techniques
