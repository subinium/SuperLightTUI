//! Terminal capability model (`Capabilities`, `BlitterSupport`, `Blitter`)
//! and the host / multiplexer / environment policy that gates protocol use.

use super::*;

// ---------------------------------------------------------------------------
// Runtime terminal capability probe (issue #264)
// ---------------------------------------------------------------------------
//
// Historically SLT decided whether a terminal could render images / accept the
// Kitty keyboard protocol / do truecolor *purely from environment-variable
// allowlists*, which silently degraded capable modern terminals (WezTerm,
// Ghostty) to an error string. This block adds a one-shot DA1/DA2/XTGETTCAP
// probe at session enter, parses the replies into a read-only [`Capabilities`]
// snapshot, and drives an automatic blitter ladder so app code never has to
// branch on terminal identity. The data types are always compiled (so the
// `Context` field exists on every build); only the runtime probe is
// `crossterm`-gated.

/// Image-rendering primitives the terminal can drive, used to build the
/// automatic blitter ladder. Each flag is conservative: when the runtime probe
/// returns no answer the defaults assume only the universally available
/// primitives (half-block + quadrants).
///
/// App code is **not** required to inspect this; it exists for diagnostics and
/// to feed [`Capabilities::best_blitter`].
///
/// # Example
///
/// ```no_run
/// # slt::run(|ui: &mut slt::Context| {
/// let blitters = ui.capabilities().blitters;
/// // Half-block is available on any ANSI terminal.
/// assert!(blitters.half);
/// # });
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlitterSupport {
    /// `▀` upper-half block — available on any ANSI terminal.
    pub half: bool,
    /// `▖▗▘▝` quadrant blocks — available on any Unicode-capable terminal.
    pub quad: bool,
    /// `🬀`..`🬻` sextants (Unicode 13+) — off by default until a renderer
    /// confirms support. This issue wires the capability slot; a sextant
    /// renderer is a separate feature.
    pub sextant: bool,
}

impl Default for BlitterSupport {
    fn default() -> Self {
        Self {
            half: true,
            quad: true,
            sextant: false,
        }
    }
}

/// Read-only snapshot of negotiated terminal capabilities, populated once at
/// session enter via DA1/DA2/XTGETTCAP.
///
/// App code **must not** be required to branch on this — it exists for
/// diagnostics and to drive the automatic blitter ladder (see
/// [`Capabilities::best_blitter`]). On a headless backend (TestBackend / piped
/// stdout) or when the probe gets no reply, every field falls back to a
/// conservative default.
///
/// Available since `0.21.0`.
///
/// # Example
///
/// ```no_run
/// # slt::run(|ui: &mut slt::Context| {
/// let caps = ui.capabilities();
/// if caps.sixel {
///     // Diagnostics only — image rendering already routes through the ladder.
/// }
/// # });
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capabilities {
    /// 24-bit color confirmed (XTGETTCAP `Tc`/`RGB` or `COLORTERM`).
    pub truecolor: bool,
    /// Sixel graphics confirmed (DA1 attribute `4`).
    pub sixel: bool,
    /// iTerm2 OSC 1337 inline-image protocol confirmed (env identity for
    /// iTerm2 / WezTerm / Tabby / mintty; issue #265).
    pub iterm2: bool,
    /// Kitty graphics protocol confirmed: the terminal answered the startup
    /// Kitty graphics query (`APC G a=q`) with an `OK` ack. When the query is
    /// not acknowledged, falls back to the `TERM` / `TERM_PROGRAM` identity of
    /// known Kitty-graphics hosts (Kitty, Ghostty, WezTerm), subject to
    /// multiplexer / remote-session gating; `SLT_FORCE_KITTY` forces it on and
    /// `SLT_DISABLE_KITTY` always clears it.
    pub kitty_graphics: bool,
    /// Kitty keyboard protocol confirmed: the terminal answered the startup
    /// progressive-enhancement query (`CSI ? u`) with `CSI ? <flags> u`
    /// (issue #448). `false` when the terminal stayed silent, when startup
    /// queries are skipped (Windows, multiplexers, unidentified PTYs), or when
    /// `SLT_DISABLE_KITTY_KEYBOARD=1` is set.
    pub kitty_keyboard: bool,
    /// Synchronized output (DECSET 2026) confirmed.
    pub sync_output: bool,
    /// Set of cell-art blitters the terminal can drive.
    pub blitters: BlitterSupport,
}

/// Descending image-render preference. The first capability that is available
/// wins; app code never selects a [`Blitter`] directly.
///
/// Ladder order: [`Kitty`](Blitter::Kitty) > [`Sixel`](Blitter::Sixel) >
/// [`Iterm2`](Blitter::Iterm2) > [`Sextant`](Blitter::Sextant) >
/// [`HalfBlock`](Blitter::HalfBlock).
///
/// Available since `0.21.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blitter {
    /// Kitty graphics protocol (highest fidelity).
    Kitty,
    /// Sixel graphics protocol.
    Sixel,
    /// iTerm2 OSC 1337 inline-image protocol (issue #265). Pixel-accurate on
    /// Tabby, older iTerm2, and WezTerm's iTerm2-compat mode.
    Iterm2,
    /// Unicode sextant cell art.
    Sextant,
    /// Half-block cell art (universal fallback).
    HalfBlock,
}

impl Capabilities {
    /// Resolve the best available image blitter for this terminal.
    ///
    /// Returns the first supported rung of the ladder
    /// (Kitty > Sixel > iTerm2 > Sextant > HalfBlock). This is total: it always
    /// returns a [`Blitter`], falling through to [`Blitter::HalfBlock`] which
    /// every terminal supports.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let _ = ui.capabilities().best_blitter();
    /// # });
    /// ```
    pub fn best_blitter(&self) -> Blitter {
        if self.kitty_graphics {
            Blitter::Kitty
        } else if self.sixel {
            Blitter::Sixel
        } else if self.iterm2 {
            Blitter::Iterm2
        } else if self.blitters.sextant {
            Blitter::Sextant
        } else {
            Blitter::HalfBlock
        }
    }
}

/// Return the process-global negotiated [`Capabilities`], probing the terminal
/// exactly once on first call and caching the result.
///
/// On an identified direct terminal, the probe issues the Kitty keyboard
/// query (`CSI ? u`), DA1 (`CSI c`), DA2 (`CSI > c`), the Kitty graphics
/// query, XTGETTCAP for the truecolor capname, and DECRQM 2026, reading replies
/// through the existing OSC round-trip infrastructure with a bounded total
/// timeout (≤180ms). Generic PTY wrappers, `TERM=dumb`, and tmux/screen skip
/// automatic queries to avoid leaking control bytes or racing user input;
/// environment-based fallbacks remain available. Set
/// `SLT_FORCE_TERMINAL_QUERIES=1` to opt in or
/// `SLT_DISABLE_TERMINAL_QUERIES=1` to disable all terminal queries. Repeated
/// calls are free.
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn capabilities() -> Capabilities {
    use std::sync::OnceLock;
    if !io::stdout().is_terminal() {
        return Capabilities::default();
    }
    static CACHED: OnceLock<Capabilities> = OnceLock::new();
    *CACHED.get_or_init(probe_capabilities)
}

/// Send DA1/DA2/XTGETTCAP and parse the replies into a [`Capabilities`].
///
/// Conservative on failure: any unread / unparsable reply leaves the
/// corresponding flag at its default. The total stdin wait is bounded to keep
/// startup latency within the same budget as the existing OSC 11 query.
#[cfg(feature = "crossterm")]
pub(super) fn probe_capabilities() -> Capabilities {
    let mut caps = Capabilities::default();
    if automatic_terminal_queries_allowed() {
        // Total stdin wait is bounded to ≤180ms (90 + 30 + 30 + 30) so a
        // silent terminal cannot stall startup beyond a small multiple of the
        // existing OSC-11 budget. A responsive terminal replies in well under
        // 10ms per query, so the common path adds negligible latency.
        // DA1 then DA2 in one write — both terminate with `c`, so a single
        // DA-aware read drains both replies (in order) when supported.
        // The Kitty keyboard query `CSI ? u` is sent in the same write, ahead
        // of DA1, so the DA1 reply doubles as its "unsupported" sentinel.
        if let Some(resp) = read_da_response(Duration::from_millis(90)) {
            parse_da1(&resp, &mut caps);
            parse_da2(&resp, &mut caps);
            caps.kitty_keyboard = parse_kitty_keyboard_reply(&resp).is_some();
        }

        // Kitty graphics query: APC G a=q (query) with a 1×1 RGB direct
        // payload. Base64 of three zero bytes = "AAAA".
        if let Some(resp) = read_osc_response(
            "\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\",
            Duration::from_millis(30),
            &[b"\x1b_G"],
        ) {
            parse_kitty_graphics_ack(&resp, &mut caps);
        }

        // XTGETTCAP for the `Tc` (truecolor) capname: `Tc` -> hex "5463".
        if let Some(resp) = read_osc_response(
            "\x1bP+q5463\x1b\\",
            Duration::from_millis(30),
            &[b"\x1bP1+r", b"\x1bP0+r"],
        ) {
            parse_xtgettcap_truecolor(&resp, &mut caps);
        }

        // DECRQM for synchronized output (mode ?2026): CSI ? 2026 $ p.
        if let Some(resp) = read_decrpm_response(Duration::from_millis(30)) {
            match parse_decrpm_sync_output(&resp) {
                Some(true) => {
                    caps.sync_output = true;
                    let _ = SYNC_OUTPUT_RESOLUTION.set(SyncOutputResolution::Supported);
                }
                Some(false) => {
                    let _ = SYNC_OUTPUT_RESOLUTION.set(SyncOutputResolution::Unsupported);
                }
                None => {}
            }
        }
    }

    // Env precedence chain stays authoritative for truecolor: a positive
    // COLORTERM/TERM signal confirms it even when the probe is silent.
    if matches!(ColorDepth::detect(), ColorDepth::TrueColor) {
        caps.truecolor = true;
    }

    if !caps.sixel && term_is_sixel_host() {
        caps.sixel = true;
    }

    // Env-fallback: when the runtime queries are silent (no reply within the
    // timeout), trust the terminal identity for the Kitty-graphics family so a
    // known-capable host (Kitty, Ghostty, WezTerm) still climbs the top rung.
    // The query above wins when it answers; this only fills an unknown.
    if !caps.kitty_graphics && term_is_kitty_graphics_host() {
        caps.kitty_graphics = true;
    }

    // iTerm2 OSC 1337 has no DA1/DA2 signal (issue #265): the protocol is
    // identified purely by terminal identity. Fill the capability slot from the
    // env so the blitter ladder can offer it below Kitty/Sixel.
    if term_is_iterm_host() {
        caps.iterm2 = true;
    }

    // Explicit disable flags have final precedence over probe acknowledgements,
    // inherited terminal identity, and force flags.
    if force_env_enabled("SLT_DISABLE_KITTY") {
        caps.kitty_graphics = false;
    }
    if force_env_enabled("SLT_DISABLE_SIXEL") {
        caps.sixel = false;
    }
    if force_env_enabled("SLT_DISABLE_ITERM") {
        caps.iterm2 = false;
    }
    if force_env_enabled("SLT_DISABLE_KITTY_KEYBOARD") {
        caps.kitty_keyboard = false;
    }

    caps
}

/// Whether to push the Kitty keyboard protocol at session enter (issue #448).
///
/// * `requested`: [`crate::RunConfig::kitty_keyboard`].
/// * `policy_allowed`: multiplexer / env policy
///   ([`kitty_keyboard_allowed_env`]); already folds in the force and disable
///   flags.
/// * `probe_ran`: startup queries ran, so `confirmed` is meaningful.
/// * `confirmed`: the terminal answered `CSI ? u`.
/// * `forced`: `SLT_FORCE_KITTY_KEYBOARD`.
///
/// Where queries cannot run (zellij, Windows, unidentified PTYs) the decision
/// falls back to policy alone so allowlisted hosts keep working.
pub(super) fn kitty_keyboard_push_decision(
    requested: bool,
    policy_allowed: bool,
    probe_ran: bool,
    confirmed: bool,
    forced: bool,
) -> bool {
    requested && policy_allowed && (forced || confirmed || !probe_ran)
}

/// Heuristic env-detection for iTerm2 OSC 1337 inline-image hosts (issue #265).
///
/// The protocol carries no DA reply, so detection is by `TERM_PROGRAM` identity
/// only: iTerm2, WezTerm (iTerm2-compat), Tabby, and mintty.
#[cfg(feature = "crossterm")]
pub(super) fn term_is_iterm_host() -> bool {
    if force_env_enabled("SLT_DISABLE_ITERM") {
        return false;
    }
    let term_program = std::env::var("TERM_PROGRAM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    term_is_iterm_host_env(
        &term_program,
        terminal_multiplexer(),
        terminal_is_remote(),
        force_env_enabled("SLT_FORCE_ITERM"),
    )
}

#[cfg(feature = "crossterm")]
pub(super) fn term_is_iterm_host_env(
    term_program: &str,
    multiplexer: Option<MultiplexerKind>,
    remote: bool,
    forced: bool,
) -> bool {
    if !terminal_protocol_allowed(multiplexer, TerminalProtocol::Iterm2, forced, false) {
        return false;
    }
    forced || (!remote && matches!(term_program, "iterm.app" | "wezterm" | "tabby" | "mintty"))
}

#[cfg(feature = "crossterm")]
pub(super) fn term_is_sixel_host() -> bool {
    if force_env_enabled("SLT_DISABLE_SIXEL") {
        return false;
    }
    let term = std::env::var("TERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let term_program = std::env::var("TERM_PROGRAM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    term_is_sixel_host_env(
        &term,
        &term_program,
        terminal_multiplexer(),
        terminal_is_remote(),
        force_env_enabled("SLT_FORCE_SIXEL"),
    )
}

#[cfg(feature = "crossterm")]
pub(super) fn term_is_sixel_host_env(
    term: &str,
    term_program: &str,
    multiplexer: Option<MultiplexerKind>,
    remote: bool,
    forced: bool,
) -> bool {
    if !terminal_protocol_allowed(multiplexer, TerminalProtocol::Sixel, forced, false) {
        return false;
    }
    if forced || matches!(multiplexer, Some(MultiplexerKind::Zellij)) {
        return true;
    }
    const KNOWN_SIXEL_TERMS: &[&str] = &["mlterm", "foot", "yaft", "xterm-256color-sixel"];
    const KNOWN_SIXEL_TERM_PROGRAMS: &[&str] = &["foot", "mlterm", "wezterm", "ghostty"];
    KNOWN_SIXEL_TERMS.contains(&term)
        || term.contains("sixel")
        || (!remote && KNOWN_SIXEL_TERM_PROGRAMS.contains(&term_program))
}

/// Heuristic env-fallback for Kitty-graphics hosts, consulted only when the
/// runtime Kitty graphics query returned no reply. Matches the documented
/// `TERM` / `TERM_PROGRAM` identities of terminals that implement the Kitty
/// graphics protocol.
#[cfg(feature = "crossterm")]
pub(super) fn term_is_kitty_graphics_host() -> bool {
    if force_env_enabled("SLT_DISABLE_KITTY") {
        return false;
    }
    let term = std::env::var("TERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let term_program = std::env::var("TERM_PROGRAM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    term_is_kitty_graphics_host_env(
        &term,
        &term_program,
        terminal_multiplexer(),
        terminal_is_remote(),
        force_env_enabled("SLT_FORCE_KITTY"),
    )
}

#[cfg(feature = "crossterm")]
pub(super) fn term_is_kitty_graphics_host_env(
    term: &str,
    term_program: &str,
    multiplexer: Option<MultiplexerKind>,
    remote: bool,
    forced: bool,
) -> bool {
    if !terminal_protocol_allowed(multiplexer, TerminalProtocol::KittyGraphics, forced, false) {
        return false;
    }
    // Kitty sets `TERM=xterm-kitty`; Ghostty/WezTerm advertise via TERM_PROGRAM.
    forced
        || term.contains("kitty")
        || (!remote && matches!(term_program, "ghostty" | "wezterm" | "kitty"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MultiplexerKind {
    Tmux,
    Screen,
    Zellij,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TerminalProtocol {
    Queries,
    SynchronizedOutput,
    KittyGraphics,
    Sixel,
    Iterm2,
    KittyKeyboard,
    /// Taskbar progress (`OSC 9;4`, issue #468).
    Progress,
    /// Program status (`OSC 7501`, issue #468).
    ProgramStatus,
}

pub(super) fn terminal_protocol_allowed(
    multiplexer: Option<MultiplexerKind>,
    protocol: TerminalProtocol,
    forced: bool,
    disabled: bool,
) -> bool {
    if disabled {
        return false;
    }
    if forced {
        return true;
    }
    match multiplexer {
        None => true,
        Some(MultiplexerKind::Zellij) => {
            matches!(
                protocol,
                TerminalProtocol::Sixel | TerminalProtocol::KittyKeyboard
            )
        }
        Some(MultiplexerKind::Tmux | MultiplexerKind::Screen) => false,
    }
}

#[cfg(feature = "crossterm")]
pub(super) fn terminal_multiplexer() -> Option<MultiplexerKind> {
    let term = std::env::var("TERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    terminal_multiplexer_env(
        &term,
        std::env::var_os("TMUX").is_some(),
        std::env::var_os("STY").is_some(),
        std::env::var_os("ZELLIJ").is_some(),
        std::env::var_os("ZELLIJ_SESSION_NAME").is_some(),
    )
}

pub(super) fn terminal_multiplexer_env(
    term: &str,
    has_tmux: bool,
    has_sty: bool,
    has_zellij: bool,
    has_zellij_session: bool,
) -> Option<MultiplexerKind> {
    let term = term.to_ascii_lowercase();
    if term.starts_with("tmux") {
        Some(MultiplexerKind::Tmux)
    } else if term.starts_with("screen") {
        // tmux's long-standing `default-terminal "screen-256color"` sets a
        // screen TERM; without `STY` the innermost multiplexer is tmux.
        // Passthrough wrapping (issue #446) depends on telling them apart.
        if has_tmux && !has_sty {
            Some(MultiplexerKind::Tmux)
        } else {
            Some(MultiplexerKind::Screen)
        }
    } else if has_zellij || has_zellij_session {
        Some(MultiplexerKind::Zellij)
    } else if has_tmux {
        Some(MultiplexerKind::Tmux)
    } else if has_sty {
        Some(MultiplexerKind::Screen)
    } else {
        None
    }
}

#[cfg(feature = "crossterm")]
fn terminal_is_remote() -> bool {
    // Shared with link opening (`Context::link`), which must not spawn a
    // browser on the remote host.
    crate::context::session_is_remote()
}

#[cfg(feature = "crossterm")]
pub(super) fn terminal_kitty_keyboard_allowed() -> bool {
    kitty_keyboard_allowed_env(
        terminal_multiplexer(),
        force_env_enabled("SLT_FORCE_KITTY_KEYBOARD"),
        force_env_enabled("SLT_DISABLE_KITTY_KEYBOARD"),
    )
}

pub(super) fn kitty_keyboard_allowed_env(
    multiplexer: Option<MultiplexerKind>,
    forced: bool,
    disabled: bool,
) -> bool {
    terminal_protocol_allowed(
        multiplexer,
        TerminalProtocol::KittyKeyboard,
        forced,
        disabled,
    )
}

#[cfg(feature = "crossterm")]
pub(super) fn force_env_enabled(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .is_some_and(|value| truthy_env_value(&value))
}

#[cfg(feature = "crossterm")]
fn truthy_env_value(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(feature = "crossterm")]
pub(super) fn terminal_queries_allowed() -> bool {
    // Console input records are not a VT byte stream on Windows. Until a
    // record-preserving query API is available, retain native input intact.
    if cfg!(windows) {
        return false;
    }
    let term = std::env::var("TERM").unwrap_or_default();
    terminal_query_allowed(
        io::stdout().is_terminal(),
        io::stdin().is_terminal(),
        &term,
        terminal_multiplexer(),
        force_env_enabled("SLT_FORCE_TERMINAL_QUERIES"),
        force_env_enabled("SLT_DISABLE_TERMINAL_QUERIES"),
    )
}

#[cfg(feature = "crossterm")]
pub(super) fn automatic_terminal_queries_allowed() -> bool {
    if !terminal_queries_allowed() {
        return false;
    }
    force_env_enabled("SLT_FORCE_TERMINAL_QUERIES") || terminal_query_host_is_identified()
}

#[cfg(feature = "crossterm")]
fn terminal_query_host_is_identified() -> bool {
    const IDENTITY_VARS: &[&str] = &[
        "TERM_PROGRAM",
        "WT_SESSION",
        "VTE_VERSION",
        "KONSOLE_VERSION",
        "KITTY_WINDOW_ID",
    ];
    let remote = terminal_is_remote();
    if !remote
        && IDENTITY_VARS
            .iter()
            .any(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()))
    {
        return true;
    }

    let term = std::env::var("TERM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    terminal_query_host_is_identified_env(&term, false, remote)
}

pub(super) fn terminal_query_host_is_identified_env(
    term: &str,
    has_identity_var: bool,
    remote: bool,
) -> bool {
    (has_identity_var && !remote)
        || matches!(
            term.to_ascii_lowercase().as_str(),
            "alacritty" | "foot" | "foot-extra" | "mlterm" | "wezterm" | "xterm-kitty"
        )
}

pub(super) fn terminal_query_allowed(
    stdout_is_terminal: bool,
    stdin_is_terminal: bool,
    term: &str,
    multiplexer: Option<MultiplexerKind>,
    forced: bool,
    disabled: bool,
) -> bool {
    if !stdout_is_terminal || !stdin_is_terminal {
        return false;
    }
    terminal_protocol_allowed(multiplexer, TerminalProtocol::Queries, forced, disabled)
        && (forced || (!term.is_empty() && !term.eq_ignore_ascii_case("dumb")))
}

/// Terminal identities known to render `OSC 9;4` progress instead of
/// treating `OSC 9` as a desktop notification (issue #468).
pub(super) fn progress_host_env(
    term_program: &str,
    term_program_version: &str,
    has_wt_session: bool,
    conemu_ansi: bool,
) -> bool {
    if has_wt_session || conemu_ansi {
        return true;
    }
    match term_program.to_ascii_lowercase().as_str() {
        "ghostty" => true,
        // iTerm2 3.6 added OSC 9;4; older versions post a notification.
        "iterm.app" => {
            let mut parts = term_program_version
                .split('.')
                .map(|part| part.parse::<u32>().unwrap_or(0));
            let major = parts.next().unwrap_or(0);
            let minor = parts.next().unwrap_or(0);
            (major, minor) >= (3, 6)
        }
        _ => false,
    }
}

#[cfg(feature = "crossterm")]
pub(super) fn terminal_progress_host() -> bool {
    let var = |name: &str| std::env::var(name).unwrap_or_default();
    progress_host_env(
        &var("TERM_PROGRAM"),
        &var("TERM_PROGRAM_VERSION"),
        std::env::var_os("WT_SESSION").is_some(),
        var("ConEmuANSI").eq_ignore_ascii_case("on"),
    )
}
