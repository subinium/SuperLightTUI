//! Per-frame terminal side effects routed through the backend sink:
//! terminal metadata (title, progress, program status, cursor shape — issue
//! #468), `OSC 52` clipboard writes (issue #446), and the cluster-width
//! policy the diff flush consults (issue #449).
//!
//! `run_frame` stages one [`FrameEffects`] per frame for backends that own a
//! terminal session; the built-in [`Terminal`] / [`InlineTerminal`] flush
//! consumes it, so the bytes land in the same buffered stream (and in the PTY
//! harness capture) instead of a separate unbuffered stdout handle.

use super::*;
use crate::context::{ClipboardStatus, CursorShape, ProgramState, ProgramStatus, TerminalMeta};
use std::cell::{Cell, RefCell};

/// Side effects produced by one frame.
#[derive(Debug, Default)]
pub(crate) struct FrameEffects {
    /// Desired metadata (sticky `Context` setters, title falling back to
    /// [`crate::RunConfig::title`]).
    pub(crate) meta: TerminalMeta,
    /// Clipboard payloads in write order (selection copy, then
    /// `Context::copy_to_clipboard`).
    pub(crate) clipboard: Vec<String>,
    pub(crate) clipboard_options: ClipboardOptions,
}

thread_local! {
    static STAGED: RefCell<Option<FrameEffects>> = const { RefCell::new(None) };
    static CLIPBOARD_RESULT: Cell<Option<ClipboardStatus>> = const { Cell::new(None) };
}

/// Stage this frame's effects for the next terminal flush on this thread.
/// Replaces anything a failed flush left behind.
pub(crate) fn stage_frame_effects(effects: FrameEffects) {
    STAGED.with(|slot| *slot.borrow_mut() = Some(effects));
    CLIPBOARD_RESULT.with(|slot| slot.set(None));
}

fn take_staged_frame_effects() -> Option<FrameEffects> {
    STAGED.with(|slot| slot.borrow_mut().take())
}

/// Outcome of the clipboard writes performed by the last flush, if any.
pub(crate) fn take_clipboard_status() -> Option<ClipboardStatus> {
    CLIPBOARD_RESULT.with(Cell::take)
}

// ---------------------------------------------------------------------------
// Metadata policy and emission (issue #468)
// ---------------------------------------------------------------------------

/// Which metadata sequences a terminal receives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MetaPolicy {
    pub(crate) title: bool,
    pub(crate) cursor_shape: bool,
    pub(crate) progress: bool,
    pub(crate) status: bool,
}

impl MetaPolicy {
    /// Everything enabled: the PTY capture harness.
    #[cfg(any(test, feature = "pty-test"))]
    pub(crate) const ALL: Self = Self {
        title: true,
        cursor_shape: true,
        progress: true,
        status: true,
    };
    /// Nothing enabled.
    pub(crate) const NONE: Self = Self {
        title: false,
        cursor_shape: false,
        progress: false,
        status: false,
    };
}

/// Inputs to [`meta_policy_env`], injected so tests never touch process env.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct MetaEnv<'a> {
    pub(crate) term: &'a str,
    pub(crate) stdout_is_terminal: bool,
    pub(crate) multiplexer: Option<MultiplexerKind>,
    pub(crate) progress_host: bool,
    pub(crate) force_progress: bool,
    pub(crate) disable_progress: bool,
    pub(crate) force_status: bool,
    pub(crate) disable_status: bool,
}

/// Metadata policy: nothing for a non-TTY stdout or `TERM=dumb`; title and
/// cursor shape everywhere else (multiplexers handle both natively);
/// progress only on known `OSC 9;4` hosts outside tmux / screen / zellij;
/// program status on any direct terminal (unknown OSCs are ignored).
pub(super) fn meta_policy_env(env: &MetaEnv<'_>) -> MetaPolicy {
    if !env.stdout_is_terminal || env.term.eq_ignore_ascii_case("dumb") {
        return MetaPolicy::NONE;
    }
    MetaPolicy {
        title: true,
        cursor_shape: true,
        progress: (env.progress_host || env.force_progress)
            && terminal_protocol_allowed(
                env.multiplexer,
                TerminalProtocol::Progress,
                env.force_progress,
                env.disable_progress,
            ),
        status: terminal_protocol_allowed(
            env.multiplexer,
            TerminalProtocol::ProgramStatus,
            env.force_status,
            env.disable_status,
        ),
    }
}

#[cfg(feature = "crossterm")]
fn detect_meta_policy() -> MetaPolicy {
    let term = std::env::var("TERM").unwrap_or_default();
    meta_policy_env(&MetaEnv {
        term: &term,
        stdout_is_terminal: io::stdout().is_terminal(),
        multiplexer: terminal_multiplexer(),
        progress_host: terminal_progress_host(),
        force_progress: force_env_enabled("SLT_FORCE_PROGRESS"),
        disable_progress: force_env_enabled("SLT_DISABLE_PROGRESS"),
        force_status: force_env_enabled("SLT_FORCE_PROGRAM_STATUS"),
        disable_status: force_env_enabled("SLT_DISABLE_PROGRAM_STATUS"),
    })
}

/// What has to be undone when the session ends.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MetaRestore {
    pub(crate) title_pushed: bool,
    pub(crate) progress: bool,
    pub(crate) status: bool,
    pub(crate) cursor_shape: bool,
    pub(crate) pointer_shape: bool,
}

impl MetaRestore {
    fn any(self) -> bool {
        self.title_pushed || self.progress || self.status || self.cursor_shape || self.pointer_shape
    }
}

/// Process-global restore flags for production emitters, consulted by the
/// session exit writer (normal exit, panic cleanup and Ctrl+Z suspend).
/// `epoch` advances on every restore so emitters re-send their state after a
/// suspend / resume cycle.
struct GlobalRestore {
    flags: MetaRestore,
    epoch: u64,
}

static GLOBAL_RESTORE: std::sync::Mutex<GlobalRestore> = std::sync::Mutex::new(GlobalRestore {
    flags: MetaRestore {
        title_pushed: false,
        progress: false,
        status: false,
        cursor_shape: false,
        pointer_shape: false,
    },
    epoch: 0,
});

fn global_restore() -> std::sync::MutexGuard<'static, GlobalRestore> {
    GLOBAL_RESTORE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Write the sequences that undo `restore`. Writes nothing when nothing was
/// emitted.
pub(crate) fn write_meta_restore(w: &mut impl Write, restore: MetaRestore) -> io::Result<()> {
    if restore.cursor_shape {
        w.write_all(b"\x1b[0 q")?;
    }
    if restore.pointer_shape {
        w.write_all(b"\x1b]22;default\x1b\\")?;
    }
    if restore.progress {
        w.write_all(b"\x1b]9;4;0\x07")?;
    }
    if restore.status {
        w.write_all(b"\x1b]7501;state=clear\x1b\\")?;
    }
    if restore.title_pushed {
        w.write_all(b"\x1b[23;2t")?;
    }
    Ok(())
}

/// Undo every metadata change made by production emitters in this process.
pub(super) fn write_terminal_meta_restore(w: &mut impl Write) -> io::Result<()> {
    let flags = {
        let mut global = global_restore();
        let flags = std::mem::take(&mut global.flags);
        if flags.any() {
            global.epoch = global.epoch.wrapping_add(1);
        }
        flags
    };
    write_meta_restore(w, flags)
}

/// Maximum decoded `msg` length allowed by the OSC 7501 protocol.
const PROGRAM_STATUS_MSG_MAX: usize = 2048;

fn write_program_status(w: &mut impl Write, status: Option<&ProgramStatus>) -> io::Result<()> {
    let Some(status) = status else {
        return w.write_all(b"\x1b]7501;state=clear\x1b\\");
    };
    write!(w, "\x1b]7501;state={}", status.state.wire())?;
    if let Some(progress) = status.progress
        && matches!(status.state, ProgramState::Working | ProgramState::Blocked)
    {
        write!(w, ":progress={}", progress.min(100))?;
    }
    if let Some(message) = status.message.as_deref() {
        // Decoded text must not contain control characters.
        let mut clean: String = message
            .chars()
            .map(|ch| {
                if ch.is_control() || ('\u{80}'..='\u{9f}').contains(&ch) {
                    ' '
                } else {
                    ch
                }
            })
            .collect();
        if clean.len() > PROGRAM_STATUS_MSG_MAX {
            let mut end = PROGRAM_STATUS_MSG_MAX;
            while !clean.is_char_boundary(end) {
                end -= 1;
            }
            clean.truncate(end);
        }
        if !clean.is_empty() {
            write!(w, ":msg={}", base64_encode(clean.as_bytes()))?;
        }
    }
    w.write_all(b"\x1b\\")
}

/// Last-emitted metadata, compared against the desired state each frame so
/// an unchanged frame writes nothing.
#[derive(Debug, Default)]
pub(crate) struct MetaEmitter {
    title: Option<String>,
    progress: Option<(u8, u8)>,
    status: Option<ProgramStatus>,
    cursor_shape: CursorShape,
    pointer_shape: Option<crate::PointerShape>,
    restore: MetaRestore,
    /// Production emitters mirror `restore` into [`GLOBAL_RESTORE`] so the
    /// session exit writer can undo it; harness emitters keep it local.
    global: bool,
    epoch: u64,
}

impl MetaEmitter {
    pub(crate) fn new(global: bool) -> Self {
        Self {
            global,
            epoch: if global { global_restore().epoch } else { 0 },
            ..Self::default()
        }
    }

    /// Restore flags accumulated by this emitter.
    #[cfg(test)]
    pub(crate) fn restore_flags(&self) -> MetaRestore {
        self.restore
    }

    /// Emit whatever differs between `desired` and the last-emitted state.
    pub(crate) fn emit(
        &mut self,
        w: &mut impl Write,
        desired: &TerminalMeta,
        policy: MetaPolicy,
    ) -> io::Result<()> {
        if self.global {
            let epoch = global_restore().epoch;
            if epoch != self.epoch {
                // A session restore (suspend, nested session exit) undid our
                // state; start over so the next comparison re-sends it.
                *self = Self::new(true);
            }
        }
        let before = self.restore;

        if policy.title
            && let Some(title) = desired.title.as_deref()
            && self.title.as_deref() != Some(title)
        {
            if !self.restore.title_pushed {
                // XTWINOPS 22;2: save the window title for restore on exit.
                w.write_all(b"\x1b[22;2t")?;
                self.restore.title_pushed = true;
            }
            let safe = crate::sanitize_terminal_text(title);
            write!(w, "\x1b]2;{safe}\x07")?;
            self.title = Some(title.to_owned());
        }

        if policy.progress {
            let wanted = desired.progress.map(crate::Progress::osc_params);
            if wanted != self.progress {
                match wanted {
                    Some((3, _)) => w.write_all(b"\x1b]9;4;3\x07")?,
                    Some((state, percent)) => write!(w, "\x1b]9;4;{state};{percent}\x07")?,
                    None => w.write_all(b"\x1b]9;4;0\x07")?,
                }
                self.progress = wanted;
                self.restore.progress = wanted.is_some();
            }
        }

        if policy.status && desired.status != self.status {
            write_program_status(w, desired.status.as_ref())?;
            self.status = desired.status.clone();
            // `done` / `error` records are meant to outlive the program.
            self.restore.status = self.status.as_ref().is_some_and(|status| {
                matches!(
                    status.state,
                    ProgramState::Idle | ProgramState::Working | ProgramState::Blocked
                )
            });
        }

        if policy.cursor_shape && desired.cursor_shape != self.cursor_shape {
            write!(w, "\x1b[{} q", desired.cursor_shape.decscusr())?;
            self.cursor_shape = desired.cursor_shape;
            self.restore.cursor_shape = desired.cursor_shape != CursorShape::Default;
        }

        // #452: shape escapes share the cursor-shape policy (off for non-TTY
        // output and `TERM=dumb`); the app opts in per run.
        if policy.cursor_shape && desired.pointer_shape != self.pointer_shape {
            let name = desired
                .pointer_shape
                .map_or("default", crate::PointerShape::css_name);
            write!(w, "\x1b]22;{name}\x1b\\")?;
            self.pointer_shape = desired.pointer_shape;
            self.restore.pointer_shape = desired.pointer_shape.is_some();
        }

        if self.global && self.restore != before {
            global_restore().flags = self.restore;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Cluster width policy (issue #449)
// ---------------------------------------------------------------------------

/// Bit set of width-uncertain grapheme cluster classes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ClusterClasses(u8);

#[allow(dead_code)]
impl ClusterClasses {
    /// Emoji presentation selector U+FE0F.
    pub(crate) const VS16: Self = Self(1);
    /// Zero-width-joiner sequence.
    pub(crate) const ZWJ: Self = Self(1 << 1);
    /// Fitzpatrick skin-tone modifier U+1F3FB..U+1F3FF.
    pub(crate) const SKIN_TONE: Self = Self(1 << 2);
    /// Regional-indicator flag pair.
    pub(crate) const FLAG: Self = Self(1 << 3);
    /// Any other non-BMP pictograph.
    pub(crate) const NON_BMP: Self = Self(1 << 4);

    pub(crate) fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub(crate) fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0 && other.0 != 0
    }

    pub(crate) fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// Classify a grapheme cluster by the features terminals and multiplexers
/// are known to measure inconsistently. Plain ASCII, CJK and BMP symbols are
/// never width-uncertain.
pub(crate) fn classify_cluster(cluster: &str) -> ClusterClasses {
    let mut classes = ClusterClasses::default();
    if cluster.is_ascii() {
        return classes;
    }
    for ch in cluster.chars() {
        match ch {
            '\u{FE0F}' => classes.insert(ClusterClasses::VS16),
            '\u{200D}' => classes.insert(ClusterClasses::ZWJ),
            '\u{1F3FB}'..='\u{1F3FF}' => classes.insert(ClusterClasses::SKIN_TONE),
            '\u{1F1E6}'..='\u{1F1FF}' => classes.insert(ClusterClasses::FLAG),
            ch if u32::from(ch) > 0xFFFF => classes.insert(ClusterClasses::NON_BMP),
            _ => {}
        }
    }
    classes
}

/// Width handling the diff flush consults after each cluster.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WidthPolicy {
    /// Reposition the cursor explicitly after every width-uncertain cluster.
    pub(crate) conservative: bool,
    /// Classes the startup width probe measured differently from SLT.
    pub(crate) mismatched: ClusterClasses,
}

impl WidthPolicy {
    /// Resolve the policy: an explicit setting wins; otherwise a probe
    /// mismatch or a multiplexer turns conservative mode on.
    pub(super) fn resolve(
        explicit: Option<bool>,
        multiplexer: Option<MultiplexerKind>,
        probe: Option<&WidthProbe>,
    ) -> Self {
        let mismatched = probe.map(WidthProbe::mismatched).unwrap_or_default();
        let conservative = explicit.unwrap_or(multiplexer.is_some() || !mismatched.is_empty());
        Self {
            conservative,
            mismatched,
        }
    }

    /// Whether the flush must position the cursor explicitly for the cell
    /// after `cluster` instead of trusting the terminal's advance.
    pub(crate) fn reposition_after(&self, cluster: &str) -> bool {
        self.conservative && !classify_cluster(cluster).is_empty()
    }
}

/// One probed cluster class: SLT's width versus the terminal's advance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WidthSample {
    pub(crate) class: ClusterClasses,
    pub(crate) expected: u32,
    /// `None` when the terminal did not answer in time.
    pub(crate) measured: Option<u32>,
}

/// Result of the startup cursor-position width probe.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WidthProbe {
    pub(crate) samples: Vec<WidthSample>,
}

impl WidthProbe {
    /// Classes whose measured advance differs from SLT's width. A timed-out
    /// sample is not a mismatch: silence leaves the defaults unchanged.
    pub(crate) fn mismatched(&self) -> ClusterClasses {
        let mut classes = ClusterClasses::default();
        for sample in &self.samples {
            if sample
                .measured
                .is_some_and(|measured| measured != sample.expected)
            {
                classes.insert(sample.class);
            }
        }
        classes
    }
}

/// Representative clusters measured by the width probe.
pub(crate) const WIDTH_PROBE_SAMPLES: &[(ClusterClasses, &str)] = &[
    (ClusterClasses::VS16, "\u{2764}\u{FE0F}"),
    (
        ClusterClasses::ZWJ,
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
    ),
    (ClusterClasses::SKIN_TONE, "\u{1F44D}\u{1F3FD}"),
    (ClusterClasses::FLAG, "\u{1F1F0}\u{1F1F7}"),
];

/// Row (one-based) the probe measures on. Never row 1, so the reply
/// demultiplexer can tell a report from a legacy modified F3 key.
pub(crate) const WIDTH_PROBE_ROW: u32 = 2;

/// Build a [`WidthProbe`] from per-sample cursor-position reports (one-based
/// `(row, col)` after printing the sample at column 1).
pub(crate) fn width_probe_from_reports(reports: &[Option<(u32, u32)>]) -> WidthProbe {
    WidthProbe {
        samples: WIDTH_PROBE_SAMPLES
            .iter()
            .zip(reports.iter().chain(std::iter::repeat(&None)))
            .map(|(&(class, sample), report)| WidthSample {
                class,
                expected: crate::width::unicode_width(sample) as u32,
                measured: report
                    .filter(|&(row, _)| row == WIDTH_PROBE_ROW)
                    .map(|(_, col)| col.saturating_sub(1)),
            })
            .collect(),
    }
}

/// Run the width probe once per process on the alternate screen. Returns
/// `None` when queries are not allowed (Windows, multiplexers, unidentified
/// hosts, `SLT_DISABLE_TERMINAL_QUERIES`).
#[cfg(feature = "crossterm")]
fn run_width_probe() -> Option<WidthProbe> {
    static CACHED: std::sync::OnceLock<Option<WidthProbe>> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            if cfg!(windows)
                || terminal_multiplexer().is_some()
                || !automatic_terminal_queries_allowed()
            {
                return None;
            }
            let mut reports = Vec::with_capacity(WIDTH_PROBE_SAMPLES.len());
            for &(_, sample) in WIDTH_PROBE_SAMPLES {
                let request = format!("\x1b[{WIDTH_PROBE_ROW};1H{sample}\x1b[6n");
                let report = read_stdin_reply(&request, Duration::from_millis(30), |bytes| {
                    cpr_reply_complete(bytes)
                })
                .as_deref()
                .and_then(parse_cursor_position_report);
                reports.push(report);
            }
            // Erase the probe row; the first frame repaints from a blank
            // screen and only writes changed cells.
            let mut stdout = io::stdout();
            let _ = write!(stdout, "\x1b[{WIDTH_PROBE_ROW};1H\x1b[2K\x1b[H");
            let _ = stdout.flush();
            Some(width_probe_from_reports(&reports))
        })
        .clone()
}

// ---------------------------------------------------------------------------
// Per-terminal effect state
// ---------------------------------------------------------------------------

/// Effect state owned by one [`Terminal`] / [`InlineTerminal`].
#[derive(Debug)]
pub(crate) struct TerminalEffects {
    meta: MetaEmitter,
    policy: MetaPolicy,
    multiplexer: Option<MultiplexerKind>,
    pub(crate) width: WidthPolicy,
}

impl TerminalEffects {
    /// Production state: policy from the environment, restore flags shared
    /// with the session exit writer.
    #[cfg(feature = "crossterm")]
    pub(crate) fn detect() -> Self {
        let multiplexer = terminal_multiplexer();
        Self {
            meta: MetaEmitter::new(true),
            policy: detect_meta_policy(),
            multiplexer,
            width: WidthPolicy::resolve(None, multiplexer, None),
        }
    }

    /// PTY-harness state: every sequence enabled, nothing global, no
    /// multiplexer.
    #[cfg(any(test, feature = "pty-test"))]
    pub(crate) fn capture() -> Self {
        Self {
            meta: MetaEmitter::new(false),
            policy: MetaPolicy::ALL,
            multiplexer: None,
            width: WidthPolicy::default(),
        }
    }

    /// Override the multiplexer seen by clipboard passthrough (tests).
    #[cfg(any(test, feature = "pty-test"))]
    #[allow(dead_code)]
    pub(super) fn set_multiplexer(&mut self, multiplexer: Option<MultiplexerKind>) {
        self.multiplexer = multiplexer;
    }

    /// Resolve the width policy from run configuration, probing on a
    /// fullscreen session when asked to.
    #[cfg(feature = "crossterm")]
    pub(crate) fn configure_width(&mut self, config: &crate::RunConfig, fullscreen: bool) {
        let probe = (fullscreen && config.width_probe && config.width_conservative.is_none())
            .then(run_width_probe)
            .flatten();
        self.width =
            WidthPolicy::resolve(config.width_conservative, self.multiplexer, probe.as_ref());
    }

    /// Take the staged frame effects and emit metadata into `w`. Returns the
    /// staged effects so clipboard delivery can run after the frame flush.
    pub(crate) fn emit_meta(&mut self, w: &mut impl Write) -> io::Result<Option<FrameEffects>> {
        let Some(effects) = take_staged_frame_effects() else {
            return Ok(None);
        };
        self.meta.emit(w, &effects.meta, self.policy)?;
        Ok(Some(effects))
    }

    /// Write staged clipboard payloads, recording the last outcome for the
    /// next frame's [`crate::Context::clipboard_status`]. Never fails the
    /// frame.
    pub(crate) fn deliver_clipboard(&mut self, w: &mut impl Write, effects: &FrameEffects) {
        let mut status = None;
        for text in &effects.clipboard {
            status = Some(write_clipboard(
                w,
                text,
                effects.clipboard_options,
                self.multiplexer,
            ));
        }
        if status.is_some() {
            CLIPBOARD_RESULT.with(|slot| slot.set(status));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProgramStatus, Progress};

    fn emit_all(emitter: &mut MetaEmitter, meta: &TerminalMeta) -> Vec<u8> {
        let mut out = Vec::new();
        emitter.emit(&mut out, meta, MetaPolicy::ALL).unwrap();
        out
    }

    #[test]
    fn issue_452_pointer_shape_emits_on_change_and_restores() {
        let mut emitter = MetaEmitter::new(false);
        let mut meta = TerminalMeta::default();
        assert!(
            emit_all(&mut emitter, &meta).is_empty(),
            "default: no bytes"
        );

        meta.pointer_shape = Some(crate::PointerShape::Pointer);
        let out = emit_all(&mut emitter, &meta);
        assert_eq!(out, b"\x1b]22;pointer\x1b\\");
        assert!(emitter.restore_flags().pointer_shape);
        assert!(
            emit_all(&mut emitter, &meta).is_empty(),
            "unchanged: no bytes"
        );

        meta.pointer_shape = Some(crate::PointerShape::ColResize);
        assert_eq!(emit_all(&mut emitter, &meta), b"\x1b]22;col-resize\x1b\\");

        meta.pointer_shape = None;
        assert_eq!(emit_all(&mut emitter, &meta), b"\x1b]22;default\x1b\\");
        assert!(
            !emitter.restore_flags().pointer_shape,
            "default needs no restore"
        );

        let mut restore = Vec::new();
        write_meta_restore(
            &mut restore,
            MetaRestore {
                pointer_shape: true,
                ..MetaRestore::default()
            },
        )
        .unwrap();
        assert_eq!(restore, b"\x1b]22;default\x1b\\");
    }

    #[test]
    fn issue_468_title_emits_once_until_changed() {
        let mut emitter = MetaEmitter::new(false);
        let mut meta = TerminalMeta {
            title: Some("agent: idle".into()),
            ..TerminalMeta::default()
        };
        assert_eq!(
            emit_all(&mut emitter, &meta),
            b"\x1b[22;2t\x1b]2;agent: idle\x07"
        );
        for _ in 0..5 {
            assert!(emit_all(&mut emitter, &meta).is_empty());
        }
        meta.title = Some("agent: running".into());
        assert_eq!(emit_all(&mut emitter, &meta), b"\x1b]2;agent: running\x07");
        assert!(emitter.restore_flags().title_pushed);
    }

    #[test]
    fn issue_468_progress_states_and_restore() {
        let mut emitter = MetaEmitter::new(false);
        let mut meta = TerminalMeta::default();
        assert!(emit_all(&mut emitter, &meta).is_empty());
        for (progress, bytes) in [
            (Progress::Normal(0.4), &b"\x1b]9;4;1;40\x07"[..]),
            (Progress::Error(0.4), b"\x1b]9;4;2;40\x07"),
            (Progress::Indeterminate, b"\x1b]9;4;3\x07"),
            (Progress::Paused(1.0), b"\x1b]9;4;4;100\x07"),
        ] {
            meta.progress = Some(progress);
            assert_eq!(emit_all(&mut emitter, &meta), bytes);
            assert!(emit_all(&mut emitter, &meta).is_empty());
        }
        meta.progress = None;
        assert_eq!(emit_all(&mut emitter, &meta), b"\x1b]9;4;0\x07");
        assert!(!emitter.restore_flags().progress);

        meta.progress = Some(Progress::Normal(0.5));
        meta.cursor_shape = CursorShape::SteadyBar;
        assert_eq!(emit_all(&mut emitter, &meta), b"\x1b]9;4;1;50\x07\x1b[6 q");
        let mut teardown = Vec::new();
        write_meta_restore(&mut teardown, emitter.restore_flags()).unwrap();
        assert_eq!(teardown, b"\x1b[0 q\x1b]9;4;0\x07");
    }

    #[test]
    fn issue_468_title_sanitizes_control_and_c1_bytes() {
        let mut emitter = MetaEmitter::new(false);
        let meta = TerminalMeta {
            title: Some("a\x07b\x1b]0;evil\u{9b}c\nd".into()),
            ..TerminalMeta::default()
        };
        let out = emit_all(&mut emitter, &meta);
        assert_eq!(out, "\x1b[22;2t\x1b]2;a?b?]0;evil?c?d\x07".as_bytes());
    }

    #[test]
    fn issue_468_program_status_wire_format() {
        let mut emitter = MetaEmitter::new(false);
        let mut meta = TerminalMeta {
            status: Some(
                ProgramStatus::new(ProgramState::Working)
                    .progress(40)
                    .message("compiling\tnow"),
            ),
            ..TerminalMeta::default()
        };
        assert_eq!(
            emit_all(&mut emitter, &meta),
            format!(
                "\x1b]7501;state=working:progress=40:msg={}\x1b\\",
                base64_encode(b"compiling now")
            )
            .as_bytes()
        );
        assert!(emitter.restore_flags().status);
        meta.status = Some(ProgramStatus::new(ProgramState::Done).progress(100));
        assert_eq!(emit_all(&mut emitter, &meta), b"\x1b]7501;state=done\x1b\\");
        // `done` survives exit; nothing to clear.
        assert!(!emitter.restore_flags().status);
        meta.status = None;
        assert_eq!(
            emit_all(&mut emitter, &meta),
            b"\x1b]7501;state=clear\x1b\\"
        );
    }

    #[test]
    fn issue_468_dumb_or_non_tty_writes_nothing() {
        let base = MetaEnv {
            term: "xterm-256color",
            stdout_is_terminal: true,
            progress_host: true,
            ..MetaEnv::default()
        };
        let meta = TerminalMeta {
            title: Some("t".into()),
            progress: Some(Progress::Normal(0.1)),
            status: Some(ProgramStatus::new(ProgramState::Working)),
            cursor_shape: CursorShape::SteadyBar,
            pointer_shape: Some(crate::PointerShape::Pointer),
        };
        for env in [
            MetaEnv {
                term: "dumb",
                ..base
            },
            MetaEnv {
                term: "DUMB",
                force_progress: true,
                force_status: true,
                ..base
            },
            MetaEnv {
                stdout_is_terminal: false,
                ..base
            },
        ] {
            let policy = meta_policy_env(&env);
            assert_eq!(policy, MetaPolicy::NONE);
            let mut out = Vec::new();
            MetaEmitter::new(false)
                .emit(&mut out, &meta, policy)
                .unwrap();
            assert!(out.is_empty(), "{env:?} wrote {out:?}");
        }
        assert_eq!(meta_policy_env(&base), MetaPolicy::ALL);
    }

    #[test]
    fn issue_468_progress_and_status_gating() {
        let direct = MetaEnv {
            term: "xterm-256color",
            stdout_is_terminal: true,
            ..MetaEnv::default()
        };
        // Unknown host: OSC 9 may be a notification there, so no progress.
        assert!(!meta_policy_env(&direct).progress);
        assert!(meta_policy_env(&direct).status);
        let forced = MetaEnv {
            force_progress: true,
            ..direct
        };
        assert!(meta_policy_env(&forced).progress);
        let tmux = MetaEnv {
            progress_host: true,
            multiplexer: Some(MultiplexerKind::Tmux),
            ..direct
        };
        let policy = meta_policy_env(&tmux);
        assert!(policy.title && policy.cursor_shape);
        assert!(!policy.progress && !policy.status);
        let disabled = MetaEnv {
            progress_host: true,
            disable_progress: true,
            disable_status: true,
            ..direct
        };
        assert!(!meta_policy_env(&disabled).progress);
        assert!(!meta_policy_env(&disabled).status);

        assert!(progress_host_env("", "", true, false));
        assert!(progress_host_env("", "", false, true));
        assert!(progress_host_env("ghostty", "1.2.0", false, false));
        assert!(progress_host_env("iTerm.app", "3.6.1", false, false));
        assert!(!progress_host_env("iTerm.app", "3.5.14", false, false));
        assert!(!progress_host_env("WezTerm", "20240203", false, false));
        assert!(!progress_host_env("Apple_Terminal", "455", false, false));
    }

    #[test]
    fn issue_446_tmux_passthrough_doubles_every_inner_escape() {
        let bytes = osc52_sequence("hi", Some(MultiplexerKind::Tmux), true);
        assert_eq!(bytes, b"\x1bPtmux;\x1b\x1b]52;c;aGk=\x1b\x1b\\\x1b\\");
        let inner = &bytes[b"\x1bPtmux;".len()..bytes.len() - 2];
        // Every ESC inside the DCS body is doubled.
        let mut i = 0;
        while i < inner.len() {
            if inner[i] == 0x1b {
                assert_eq!(inner.get(i + 1), Some(&0x1b));
                i += 2;
            } else {
                i += 1;
            }
        }
        // Without passthrough (or without tmux) the bytes are unchanged.
        let bare = b"\x1b]52;c;aGk=\x1b\\".to_vec();
        assert_eq!(
            osc52_sequence("hi", Some(MultiplexerKind::Tmux), false),
            bare
        );
        assert_eq!(osc52_sequence("hi", None, true), bare);
        assert_eq!(
            osc52_sequence("hi", Some(MultiplexerKind::Zellij), true),
            bare
        );
    }

    #[test]
    fn issue_446_screen_passthrough_chunks_bel_terminated_osc() {
        let text = "x".repeat(200);
        let bytes = osc52_sequence(&text, Some(MultiplexerKind::Screen), true);
        let inner = format!("\x1b]52;c;{}\x07", base64_encode(text.as_bytes()));
        let mut rebuilt = Vec::new();
        let mut rest = &bytes[..];
        while !rest.is_empty() {
            assert!(rest.starts_with(b"\x1bP"));
            let end = rest
                .windows(2)
                .position(|w| w == b"\x1b\\")
                .expect("chunk terminated");
            let chunk = &rest[2..end];
            assert!(chunk.len() <= SCREEN_DCS_CHUNK);
            rebuilt.extend_from_slice(chunk);
            rest = &rest[end + 2..];
        }
        assert_eq!(rebuilt, inner.as_bytes());
    }

    #[test]
    fn issue_446_oversized_payload_is_not_written() {
        let mut out = Vec::new();
        let options = ClipboardOptions {
            passthrough: false,
            max_bytes: 4,
        };
        assert_eq!(
            write_clipboard(&mut out, "12345", options, None),
            ClipboardStatus::TooLarge { bytes: 5, max: 4 }
        );
        assert!(out.is_empty());
        assert_eq!(
            write_clipboard(&mut out, "1234", options, None),
            ClipboardStatus::Sent
        );
        assert_eq!(out, b"\x1b]52;c;MTIzNA==\x1b\\");
    }

    struct FailingWriter;
    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn issue_446_failing_writer_reports_status_without_aborting() {
        let mut effects = TerminalEffects::capture();
        let frame = FrameEffects {
            clipboard: vec!["copy me".into()],
            ..FrameEffects::default()
        };
        // `deliver_clipboard` returns `()`: a write failure cannot abort the
        // frame, and it is reported through the status slot, not stderr.
        effects.deliver_clipboard(&mut FailingWriter, &frame);
        assert_eq!(
            take_clipboard_status(),
            Some(ClipboardStatus::WriteFailed(io::ErrorKind::BrokenPipe))
        );
        assert_eq!(take_clipboard_status(), None);
    }

    fn count(haystack: &[u8], needle: &[u8]) -> usize {
        haystack
            .windows(needle.len())
            .filter(|w| *w == needle)
            .count()
    }

    /// Drive `frames` real frames through `frame_owned` into one captured
    /// sink terminal, so the emitter keeps its state across frames.
    fn render_frames(
        term: &mut Terminal,
        state: &mut crate::AppState,
        config: &crate::RunConfig,
        frames: usize,
        mut f: impl FnMut(&mut crate::Context),
    ) -> Vec<u8> {
        for _ in 0..frames {
            crate::frame_owned(term, state, config, Vec::new(), &mut f).unwrap();
        }
        term.take_sink_bytes()
    }

    #[test]
    fn issue_468_pty_title_progress_cursor_emit_only_on_change() {
        let mut term = Terminal::with_sink(20, 2, ColorDepth::TrueColor);
        let mut state = crate::AppState::new();
        let config = crate::RunConfig::default();
        let bytes = render_frames(&mut term, &mut state, &config, 6, |ui| {
            ui.text("x");
            ui.set_title("agent: idle");
            ui.set_progress(Some(Progress::Normal(0.4)));
            ui.set_cursor_shape(CursorShape::SteadyBar);
        });
        assert_eq!(count(&bytes, b"\x1b]2;agent: idle\x07"), 1);
        assert_eq!(count(&bytes, b"\x1b]9;4;1;40\x07"), 1);
        assert_eq!(count(&bytes, b"\x1b[6 q"), 1);
        let bytes = render_frames(&mut term, &mut state, &config, 3, |ui| {
            ui.set_title("agent: running 3/10");
            ui.set_progress(None);
        });
        assert_eq!(count(&bytes, b"\x1b]2;agent: running 3/10\x07"), 1);
        assert_eq!(count(&bytes, b"\x1b]9;4;0\x07"), 1);
        assert_eq!(count(&bytes, b"\x1b[22;2t"), 0, "title stack pushed once");
        // Idle frames add no metadata bytes.
        let bytes = render_frames(&mut term, &mut state, &config, 3, |_| {});
        assert_eq!(count(&bytes, b"\x1b]"), 0, "{bytes:?}");
        assert_eq!(count(&bytes, b" q"), 0);
        // The teardown sequence for what is still active.
        let mut teardown = Vec::new();
        write_meta_restore(&mut teardown, term.effects.meta.restore_flags()).unwrap();
        assert_eq!(teardown, b"\x1b[0 q\x1b[23;2t");
    }

    #[test]
    fn issue_468_pty_run_config_title_is_the_initial_value() {
        let mut term = Terminal::with_sink(10, 1, ColorDepth::TrueColor);
        let mut state = crate::AppState::new();
        let config = crate::RunConfig::default().title("from config");
        let bytes = render_frames(&mut term, &mut state, &config, 3, |_| {});
        assert_eq!(count(&bytes, b"\x1b]2;from config\x07"), 1);
        let bytes = render_frames(&mut term, &mut state, &config, 2, |ui| {
            ui.set_title("from app");
        });
        assert_eq!(count(&bytes, b"\x1b]2;from app\x07"), 1);
    }

    #[test]
    fn issue_446_pty_tmux_passthrough_and_status() {
        let mut term = Terminal::with_sink(10, 1, ColorDepth::TrueColor);
        term.effects.set_multiplexer(Some(MultiplexerKind::Tmux));
        let mut state = crate::AppState::new();
        let config = crate::RunConfig::default().clipboard_passthrough(true);
        let bytes = render_frames(&mut term, &mut state, &config, 1, |ui| {
            ui.copy_to_clipboard("hi");
        });
        assert_eq!(
            count(&bytes, b"\x1bPtmux;\x1b\x1b]52;c;aGk=\x1b\x1b\\\x1b\\"),
            1,
            "{bytes:?}"
        );
        let mut seen = Vec::new();
        render_frames(&mut term, &mut state, &config, 2, |ui| {
            seen.push(ui.clipboard_status());
        });
        assert_eq!(seen, [Some(ClipboardStatus::Sent), None]);

        // Without passthrough the bytes are unchanged from the bare sequence.
        let plain = crate::RunConfig::default();
        let bytes = render_frames(&mut term, &mut state, &plain, 1, |ui| {
            ui.copy_to_clipboard("hi");
        });
        assert_eq!(count(&bytes, b"\x1b]52;c;aGk=\x1b\\"), 1);
        assert_eq!(count(&bytes, b"tmux;"), 0);

        // Over the cap: nothing written, TooLarge reported next frame.
        let capped = crate::RunConfig::default().clipboard_max_bytes(1);
        let bytes = render_frames(&mut term, &mut state, &capped, 1, |ui| {
            ui.copy_to_clipboard("hi");
        });
        assert_eq!(count(&bytes, b"]52;"), 0);
        let mut seen = None;
        render_frames(&mut term, &mut state, &capped, 1, |ui| {
            seen = ui.clipboard_status();
        });
        assert_eq!(seen, Some(ClipboardStatus::TooLarge { bytes: 2, max: 1 }));
    }

    #[test]
    fn issue_449_classifies_width_uncertain_clusters() {
        assert!(classify_cluster("a").is_empty());
        assert!(classify_cluster("한").is_empty());
        assert!(classify_cluster("→").is_empty());
        assert!(classify_cluster("❤\u{FE0F}").contains(ClusterClasses::VS16));
        let family = classify_cluster("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}");
        assert!(family.contains(ClusterClasses::ZWJ));
        assert!(classify_cluster("\u{1F44D}\u{1F3FD}").contains(ClusterClasses::SKIN_TONE));
        assert!(classify_cluster("\u{1F1F0}\u{1F1F7}").contains(ClusterClasses::FLAG));
        assert!(classify_cluster("\u{1F600}").contains(ClusterClasses::NON_BMP));
    }

    #[test]
    fn issue_449_cpr_probe_parser_and_decisions() {
        assert_eq!(parse_cursor_position_report("\x1b[2;3R"), Some((2, 3)));
        assert_eq!(
            parse_cursor_position_report("x\x1b[?62c\x1b[12;40R"),
            Some((12, 40))
        );
        assert_eq!(parse_cursor_position_report("\x1b[2;R"), None);
        assert_eq!(parse_cursor_position_report("\x1b[2R"), None);
        assert_eq!(parse_cursor_position_report("\x1b[2;3"), None);
        assert!(cpr_reply_complete(b"\x1b[2;3R"));
        assert!(!cpr_reply_complete(b"\x1b[2;3"));

        // Every class matches: conservative mode stays off on a direct terminal.
        let matching = width_probe_from_reports(&[Some((2, 3)); 4]);
        assert!(matching.mismatched().is_empty());
        assert!(!WidthPolicy::resolve(None, None, Some(&matching)).conservative);

        // ZWJ advanced 6 columns (members drawn separately), flag only 1.
        let mixed =
            width_probe_from_reports(&[Some((2, 3)), Some((2, 7)), Some((2, 3)), Some((2, 2))]);
        let mismatched = mixed.mismatched();
        assert!(mismatched.contains(ClusterClasses::ZWJ));
        assert!(mismatched.contains(ClusterClasses::FLAG));
        assert!(!mismatched.intersects(ClusterClasses::VS16));
        let policy = WidthPolicy::resolve(None, None, Some(&mixed));
        assert!(policy.conservative);
        assert!(policy.reposition_after("\u{1F600}"));
        assert!(!policy.reposition_after("a"));

        // Timeouts (no reply, or a reply from another row) change nothing.
        let silent = width_probe_from_reports(&[None, Some((5, 9))]);
        assert!(silent.mismatched().is_empty());
        assert_eq!(
            WidthPolicy::resolve(None, None, Some(&silent)),
            WidthPolicy::default()
        );

        // Multiplexers default to conservative; explicit config wins.
        assert!(WidthPolicy::resolve(None, Some(MultiplexerKind::Zellij), None).conservative);
        assert!(
            !WidthPolicy::resolve(Some(false), Some(MultiplexerKind::Tmux), Some(&mixed))
                .conservative
        );
        assert!(WidthPolicy::resolve(Some(true), None, None).conservative);
    }
}
