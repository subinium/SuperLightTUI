/// State for the rich log viewer widget.
#[derive(Debug, Clone)]
pub struct RichLogState {
    /// Log entries to display.
    pub entries: Vec<RichLogEntry>,
    /// Scroll offset (0 = top).
    pub(crate) scroll_offset: usize,
    /// Whether to auto-scroll to bottom when new entries are added.
    pub auto_scroll: bool,
    /// Maximum number of entries to keep (None = unlimited).
    pub max_entries: Option<usize>,
}

/// A single entry in a RichLog.
#[derive(Debug, Clone)]
pub struct RichLogEntry {
    /// Styled text segments for this entry.
    pub segments: Vec<(String, Style)>,
}

impl RichLogState {
    /// Default maximum entry cap used by [`RichLogState::new`].
    ///
    /// Long-running apps that push log entries continuously would otherwise
    /// accumulate state without bound. Use [`RichLogState::new_unbounded`] to
    /// opt out explicitly.
    pub const DEFAULT_MAX_ENTRIES: usize = 10_000;

    /// Create an empty rich log state with the default entry cap
    /// ([`Self::DEFAULT_MAX_ENTRIES`]).
    pub fn new() -> Self {
        Self {
            max_entries: Some(Self::DEFAULT_MAX_ENTRIES),
            ..Self::new_unbounded()
        }
    }

    /// Create an empty rich log state without an entry cap.
    ///
    /// Prefer [`RichLogState::new`] in long-running apps. Use this constructor
    /// only when the host explicitly bounds growth elsewhere.
    pub fn new_unbounded() -> Self {
        Self {
            entries: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
            max_entries: None,
        }
    }

    /// Add a single-style entry to the log.
    pub fn push(&mut self, text: impl Into<String>, style: Style) {
        self.push_segments(vec![(text.into(), style)]);
    }

    /// Add a plain text entry using default style.
    pub fn push_plain(&mut self, text: impl Into<String>) {
        self.push(text, Style::new());
    }

    /// Add a multi-segment styled entry to the log.
    pub fn push_segments(&mut self, segments: Vec<(String, Style)>) {
        self.entries.push(RichLogEntry { segments });

        if let Some(max_entries) = self.max_entries
            && self.entries.len() > max_entries
        {
            let remove_count = self.entries.len() - max_entries;
            self.entries.drain(0..remove_count);
            self.scroll_offset = self.scroll_offset.saturating_sub(remove_count);
        }

        if self.auto_scroll {
            self.scroll_offset = usize::MAX;
        }
    }

    /// Clear all entries and reset scroll position.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.scroll_offset = 0;
    }

    /// Return number of entries in the log.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return true when no entries are present.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for RichLogState {
    fn default() -> Self {
        Self::new()
    }
}

/// An absolute calendar date `(year, month 1–12, day 1–31)`.
///
/// Used by [`CalendarState`] to represent range endpoints that can span
/// month and year boundaries (a `selected_day` alone is scoped to the
/// currently displayed month). Available since `0.21.0`.
///
/// # Example
///
/// ```no_run
/// use slt::CalDate;
///
/// let d = CalDate { year: 2024, month: 12, day: 31 };
/// assert_eq!((d.year, d.month, d.day), (2024, 12, 31));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalDate {
    /// Calendar year.
    pub year: i32,
    /// Month of year, `1`–`12`.
    pub month: u32,
    /// Day of month, `1`–`31`.
    pub day: u32,
}

impl CalDate {
    /// Sort key ordering this date against another by year, then month, then day.
    fn key(&self) -> (i32, u32, u32) {
        (self.year, self.month, self.day)
    }
}

/// Selection behavior for [`CalendarState`].
///
/// Defaults to [`Single`](CalendarSelect::Single), preserving the original
/// single-date pick. Switch to [`Range`](CalendarSelect::Range) via
/// [`CalendarState::with_range`] for start/end range selection. Available
/// since `0.21.0`.
///
/// # Example
///
/// ```no_run
/// use slt::{CalendarSelect, CalendarState};
///
/// let mut cal = CalendarState::from_ym(2024, 3);
/// assert_eq!(cal.mode(), CalendarSelect::Single);
/// cal.with_range();
/// assert_eq!(cal.mode(), CalendarSelect::Range);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalendarSelect {
    /// Pick exactly one date (default).
    #[default]
    Single,
    /// Pick a start/end date range via Shift-extend.
    Range,
}

/// State for the calendar date picker widget.
#[derive(Debug, Clone)]
pub struct CalendarState {
    /// Current display year.
    pub year: i32,
    /// Current display month (1–12).
    pub month: u32,
    /// Currently selected day, if any (single-date mode).
    pub selected_day: Option<u32>,
    pub(crate) cursor_day: u32,
    pub(crate) mode: CalendarSelect,
    pub(crate) anchor: Option<CalDate>,
    pub(crate) extent: Option<CalDate>,
    pub(crate) time_enabled: bool,
    pub(crate) hour: u8,
    pub(crate) minute: u8,
}

impl CalendarState {
    /// Create a new `CalendarState` initialized to the current month.
    pub fn new() -> Self {
        let (year, month) = Self::current_year_month();
        Self::from_ym(year, month)
    }

    /// Create a `CalendarState` for a specific year and month.
    pub fn from_ym(year: i32, month: u32) -> Self {
        let month = month.clamp(1, 12);
        Self {
            year,
            month,
            selected_day: None,
            cursor_day: 1,
            mode: CalendarSelect::Single,
            anchor: None,
            extent: None,
            time_enabled: false,
            hour: 0,
            minute: 0,
        }
    }

    /// Enable date-range selection (start/end via Shift-extend).
    ///
    /// Single-date mode remains the default; call this to opt in. Returns
    /// `&mut Self` for chaining. Available since `0.21.0`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::CalendarState;
    ///
    /// let mut cal = CalendarState::from_ym(2024, 3);
    /// cal.with_range();
    /// ```
    pub fn with_range(&mut self) -> &mut Self {
        self.mode = CalendarSelect::Range;
        self
    }

    /// Enable hour/minute selection, rendered as `HH:MM` below the grid.
    ///
    /// Off by default — no time row is rendered unless enabled. Returns
    /// `&mut Self` for chaining. Available since `0.21.0`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::CalendarState;
    ///
    /// let mut cal = CalendarState::from_ym(2024, 3);
    /// cal.with_time();
    /// ```
    pub fn with_time(&mut self) -> &mut Self {
        self.time_enabled = true;
        self
    }

    /// The active selection mode (`Single` by default).
    ///
    /// Available since `0.21.0`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::{CalendarSelect, CalendarState};
    ///
    /// let cal = CalendarState::from_ym(2024, 3);
    /// assert_eq!(cal.mode(), CalendarSelect::Single);
    /// ```
    pub fn mode(&self) -> CalendarSelect {
        self.mode
    }

    /// Returns the selected date as `(year, month, day)`, if any.
    pub fn selected_date(&self) -> Option<(i32, u32, u32)> {
        self.selected_day.map(|day| (self.year, self.month, day))
    }

    /// The normalized selected range as `(start, end)` with `start <= end`.
    ///
    /// Returns `None` in single-date mode or until an anchor has been set in
    /// range mode. Endpoints are absolute [`CalDate`]s, so a range may span
    /// month or year boundaries. Available since `0.21.0`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::CalendarState;
    ///
    /// let mut cal = CalendarState::from_ym(2024, 3);
    /// cal.with_range();
    /// assert!(cal.selected_range().is_none());
    /// ```
    pub fn selected_range(&self) -> Option<(CalDate, CalDate)> {
        if self.mode != CalendarSelect::Range {
            return None;
        }
        let anchor = self.anchor?;
        let extent = self.extent.unwrap_or(anchor);
        if anchor.key() <= extent.key() {
            Some((anchor, extent))
        } else {
            Some((extent, anchor))
        }
    }

    /// The selected `(hour, minute)` when time is enabled, else `None`.
    ///
    /// Available since `0.21.0`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::CalendarState;
    ///
    /// let mut cal = CalendarState::from_ym(2024, 3);
    /// assert!(cal.selected_time().is_none());
    /// cal.with_time();
    /// assert_eq!(cal.selected_time(), Some((0, 0)));
    /// ```
    pub fn selected_time(&self) -> Option<(u8, u8)> {
        self.time_enabled.then_some((self.hour, self.minute))
    }

    /// The cursor day as an absolute [`CalDate`] in the displayed month.
    pub(crate) fn cursor_date(&self) -> CalDate {
        CalDate {
            year: self.year,
            month: self.month,
            day: self.cursor_day,
        }
    }

    /// Set the range anchor to the cursor, clearing any prior extent.
    pub(crate) fn set_anchor_to_cursor(&mut self) {
        let cur = self.cursor_date();
        self.anchor = Some(cur);
        self.extent = None;
    }

    /// Set the range extent endpoint to the cursor.
    ///
    /// If no anchor exists yet, the cursor becomes the anchor.
    pub(crate) fn extend_to_cursor(&mut self) {
        let cur = self.cursor_date();
        if self.anchor.is_none() {
            self.anchor = Some(cur);
        }
        self.extent = Some(cur);
    }

    /// Whether the given absolute date falls inside the selected range
    /// (inclusive of both endpoints).
    pub(crate) fn in_range(&self, d: CalDate) -> bool {
        match self.selected_range() {
            Some((start, end)) => start.key() <= d.key() && d.key() <= end.key(),
            None => false,
        }
    }

    /// Whether the given absolute date is one of the range endpoints.
    pub(crate) fn is_range_endpoint(&self, d: CalDate) -> bool {
        match self.selected_range() {
            Some((start, end)) => d == start || d == end,
            None => false,
        }
    }

    /// Navigate to the previous month.
    pub fn prev_month(&mut self) {
        if self.month == 1 {
            self.month = 12;
            self.year -= 1;
        } else {
            self.month -= 1;
        }
        self.clamp_days();
    }

    /// Navigate to the next month.
    pub fn next_month(&mut self) {
        if self.month == 12 {
            self.month = 1;
            self.year += 1;
        } else {
            self.month += 1;
        }
        self.clamp_days();
    }

    pub(crate) fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::is_leap_year(year) {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        }
    }

    pub(crate) fn first_weekday(year: i32, month: u32) -> u32 {
        let month = month.clamp(1, 12);
        let offsets = [0_i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let mut y = year;
        if month < 3 {
            y -= 1;
        }
        let sunday_based = (y + y / 4 - y / 100 + y / 400 + offsets[(month - 1) as usize] + 1) % 7;
        ((sunday_based + 6) % 7) as u32
    }

    fn clamp_days(&mut self) {
        let max_day = Self::days_in_month(self.year, self.month);
        self.cursor_day = self.cursor_day.clamp(1, max_day);
        if let Some(day) = self.selected_day {
            self.selected_day = Some(day.min(max_day));
        }
    }

    fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    fn current_year_month() -> (i32, u32) {
        let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
            return (1970, 1);
        };
        let days_since_epoch = (duration.as_secs() / 86_400) as i64;
        let (year, month, _) = Self::civil_from_days(days_since_epoch);
        (year, month)
    }

    fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
        let z = days_since_epoch + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let mut year = (yoe as i32) + (era as i32) * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = (mp + if mp < 10 { 3 } else { -9 }) as u32;
        if month <= 2 {
            year += 1;
        }
        (year, month, day)
    }
}

impl Default for CalendarState {
    fn default() -> Self {
        Self::new()
    }
}

/// Visual variant for buttons.
///
/// Controls the color scheme used when rendering a button. Pass to
/// [`crate::Context::button_with`] to create styled button variants.
///
/// - `Default` — theme text color, primary when focused (same as `button()`)
/// - `Primary` — primary color background with contrasting text
/// - `Danger` — error/red color for destructive actions
/// - `Outline` — bordered appearance without fill
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ButtonVariant {
    /// Standard button style.
    #[default]
    Default,
    /// Filled button with primary background color.
    Primary,
    /// Filled button with error/danger background color.
    Danger,
    /// Bordered button without background fill.
    Outline,
}

/// Direction indicator for stat widgets.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    /// Positive movement.
    Up,
    /// Negative movement.
    Down,
}

// ── Frame-clock scheduler (issue #248) ────────────────────────────────

/// The kind of timer a [`SchedulerSlot`] holds.
///
/// Sampled once per frame against the frame's wall-clock
/// [`std::time::Instant`]. Intentionally **not** keyed on the frame tick:
/// `run_frame_kernel` does not advance `diagnostics.tick`, so a tick-based
/// deadline would never elapse under `TestBackend`. See issue #248.
pub(crate) enum SchedKind {
    /// One-shot timer that fires exactly once at/after `deadline`.
    Once {
        deadline: std::time::Instant,
        fired: bool,
    },
    /// Recurring timer that reports whole `interval`s elapsed since `last`.
    Every {
        interval: std::time::Duration,
        last: std::time::Instant,
    },
    /// Debounce timer: rearmed to `now + dur` on every dirty frame, fires
    /// once when the quiet window `dur` elapses.
    Debounce {
        dur: std::time::Duration,
        deadline: std::time::Instant,
        fired: bool,
    },
}

/// A single live timer in the [`SchedulerState`] table.
pub(crate) struct SchedulerSlot {
    /// Wall-clock instant the slot was first created. Backs [`Context::elapsed`].
    pub(crate) started: std::time::Instant,
    /// The timer behavior for this slot.
    pub(crate) kind: SchedKind,
    /// GC flag: set true every frame the slot is sampled; slots left `false`
    /// at frame end are dropped so abandoned timers do not leak.
    pub(crate) touched_this_frame: bool,
}

/// Persistent timer table backing the frame-clock scheduler (issue #248).
///
/// Round-tripped through the per-frame state exactly like the named-state
/// map: moved into [`Context`](crate::Context) at frame start and moved back
/// at frame end, where untouched slots are garbage-collected. Drives
/// [`Context::schedule`](crate::Context::schedule),
/// [`every`](crate::Context::every), [`debounce`](crate::Context::debounce),
/// [`exclusive`](crate::Context::exclusive), [`cancel`](crate::Context::cancel),
/// and [`elapsed`](crate::Context::elapsed).
///
/// This type is public so it appears in `cargo doc`, but all fields are
/// `pub(crate)`: you never construct or inspect it directly — the `Context`
/// timer methods are the entire API surface.
///
/// # Example
///
/// ```no_run
/// use std::time::Duration;
///
/// slt::run(|ui: &mut slt::Context| {
///     // The scheduler state is managed for you behind the timer methods.
///     if ui.schedule("greet", Duration::from_millis(500)) {
///         ui.text("Half a second has passed.");
///     }
/// })?;
/// # Ok::<_, std::io::Error>(())
/// ```
#[derive(Default)]
pub struct SchedulerState {
    /// `&'static str`-keyed slots (mirrors `named_states`).
    pub(crate) named: std::collections::HashMap<&'static str, SchedulerSlot>,
    /// Runtime-`String`-keyed slots for dynamic ids (mirrors `keyed_states`).
    pub(crate) keyed: std::collections::HashMap<String, SchedulerSlot>,
    /// Exclusive-group claim table: `group -> claim state` (issue #248).
    pub(crate) exclusive: std::collections::HashMap<String, ExclusiveGroup>,
}

/// Per-group claim state for [`Context::exclusive`](crate::Context::exclusive)
/// (issue #248). Tracks the current winning id plus ids that were superseded
/// and must stay stale (`false`) even if re-polled.
#[derive(Default)]
pub(crate) struct ExclusiveGroup {
    /// The most-recently-claimed id; the only id that returns `true`.
    pub(crate) winner: String,
    /// Ids that previously won the group and were superseded. They never win
    /// again, so stale work cancels permanently.
    pub(crate) retired: std::collections::HashSet<String>,
}

/// Pure interval-counting kernel for [`Context::every`](crate::Context::every)
/// (issue #248). Returns how many whole `interval`s fit into `elapsed`,
/// saturating at [`u32::MAX`]. Extracted so the no-drop / no-double-count
/// invariant can be proptested deterministically without real sleeps.
pub(crate) fn intervals_elapsed(
    elapsed: std::time::Duration,
    interval: std::time::Duration,
) -> u32 {
    let nanos = interval.as_nanos().max(1);
    let count = elapsed.as_nanos() / nanos;
    count.min(u32::MAX as u128) as u32
}

impl SchedulerState {
    /// Drop every slot that was not sampled this frame, then reset the
    /// per-frame `touched` flag on the survivors. Called at frame end from
    /// `run_frame_kernel`, mirroring the `named_states` writeback lifecycle.
    pub(crate) fn gc_untouched(&mut self) {
        self.named.retain(|_, slot| slot.touched_this_frame);
        self.keyed.retain(|_, slot| slot.touched_this_frame);
        for slot in self.named.values_mut() {
            slot.touched_this_frame = false;
        }
        for slot in self.keyed.values_mut() {
            slot.touched_this_frame = false;
        }
    }

    /// Total number of live timer slots (named + keyed). Test-only accessor
    /// used to assert GC of abandoned timers (issue #248).
    #[cfg(test)]
    pub(crate) fn slot_count(&self) -> usize {
        self.named.len() + self.keyed.len()
    }
}

// ── Select / Dropdown ─────────────────────────────────────────────────
