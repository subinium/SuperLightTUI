//! Matched-harness SLT vs ratatui comparison (issue #482).
//!
//! The library half holds everything the CLI (`src/main.rs`) and the
//! deterministic regression test (`tests/regression.rs`) share:
//!
//! - [`Fixtures`]: the input data for every workload, built once.
//! - [`Workload`]: workloads a-f plus the SLT-only variants b' and c'.
//! - [`Case`]: one runnable (workload, library, harness) triple.
//! - [`SltSink`]: an [`slt::Backend`] whose `flush` mirrors
//!   `slt::Terminal::flush` into a `Vec<u8>`.
//! - [`alloc_count`]: a thread-local counting allocator.
//! - Text extraction for the equivalence check ([`check_equivalence`]).

use std::hint::black_box;
use std::io;
use std::process::Command;
use std::time::Instant;

use ratatui::backend::CrosstermBackend;
use ratatui::{Terminal, TerminalOptions, Viewport};
use unicode_width::UnicodeWidthStr;

/// Unwraps ratatui `TestBackend` results, whose error type is uninhabited.
fn infallible<T>(r: Result<T, std::convert::Infallible>) -> T {
    match r {
        Ok(v) => v,
        Err(never) => match never {},
    }
}

/// ratatui version pinned in `Cargo.toml`. Keep in sync with the manifest.
pub const RATATUI_VERSION: &str = "0.30.2";

/// Selected row for the list workloads.
pub const LIST_SELECTED: usize = 25;
/// Selected row for the table workloads.
pub const TABLE_SELECTED: usize = 25;
/// Rows per page for the paged SLT table variant (c'): 60 rows minus the
/// header, separator and pager rows.
pub const TABLE_PAGE_SIZE: usize = 57;
/// Visible rows handed to `ui.virtual_list` for b' (50-row viewport minus one).
pub const VIRTUAL_LIST_ROWS: u32 = 49;

const LIST_LEN: usize = 10_000;
const TABLE_LEN: usize = 1000;
const BOX_COUNT: usize = 60;
const SINK_CAPACITY: usize = 1 << 20;
const RUN_BUF_CAPACITY: usize = 4096;

// ───────────────────────── fixtures ─────────────────────────

/// Input data shared by both libraries. Built once; workloads borrow it.
pub struct Fixtures {
    /// 10k list rows (b, b').
    pub list_items: Vec<String>,
    /// Table header (c, c').
    pub table_headers: Vec<String>,
    /// 1000 x 6 table body (c, c').
    pub table_rows: Vec<Vec<String>>,
    /// Labels for the 60 nested boxes (d).
    pub box_labels: Vec<String>,
}

impl Default for Fixtures {
    fn default() -> Self {
        Self::new()
    }
}

impl Fixtures {
    /// Build every fixture.
    pub fn new() -> Self {
        Self {
            list_items: (0..LIST_LEN)
                .map(|i| format!("Item {i:05} — some descriptive text"))
                .collect(),
            table_headers: ["ID", "Name", "Email", "Role", "Status", "Updated"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            table_rows: (0..TABLE_LEN)
                .map(|i| {
                    vec![
                        format!("{i}"),
                        format!("User {i}"),
                        format!("user{i}@example.com"),
                        ["Admin", "Editor", "Viewer"][i % 3].to_string(),
                        ["Active", "Idle", "Offline"][i % 3].to_string(),
                        format!("2026-10-{:02}", i % 28 + 1),
                    ]
                })
                .collect(),
            box_labels: (0..BOX_COUNT).map(|i| format!("box {i}")).collect(),
        }
    }
}

// ───────────────────────── workloads ─────────────────────────

/// One workload. b' and c' are SLT-only variants, compared against the
/// ratatui arm of b and c respectively.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Workload {
    /// a. static dashboard, 200x60.
    Dashboard,
    /// b. 10k-row `ui.list`, 100x50.
    List,
    /// b'. 10k rows through `ui.virtual_list`, 100x50 (SLT only).
    VirtualList,
    /// c. 1000x6 table, 200x60.
    Table,
    /// c'. 1000x6 table with `page_size = 57`, 200x60 (SLT only).
    TablePaged,
    /// d. 60 nested bordered boxes, 240x80.
    Nested,
    /// e. dashboard with one changing cell per frame, 200x60.
    OneCell,
    /// f. idle dashboard (nothing changes), 200x60.
    Idle,
}

impl Workload {
    /// Every workload, in report order.
    pub const ALL: [Workload; 8] = [
        Workload::Dashboard,
        Workload::List,
        Workload::VirtualList,
        Workload::Table,
        Workload::TablePaged,
        Workload::Nested,
        Workload::OneCell,
        Workload::Idle,
    ];

    /// Stable case-name prefix (matches the audit prototype).
    pub fn name(self) -> &'static str {
        match self {
            Workload::Dashboard => "a_dashboard_200x60",
            Workload::List => "b_list10k_100x50",
            Workload::VirtualList => "b_list10k_100x50(virtual_list)",
            Workload::Table => "c_table1000x6_200x60",
            Workload::TablePaged => "c_table1000x6_200x60(page_size=57)",
            Workload::Nested => "d_nested60_240x80",
            Workload::OneCell => "e_onecell_200x60",
            Workload::Idle => "f_idle_200x60",
        }
    }

    /// Human-readable label for summary tables.
    pub fn label(self) -> &'static str {
        match self {
            Workload::Dashboard => "a. static dashboard 200x60",
            Workload::List => "b. 10k `ui.list`, 100x50",
            Workload::VirtualList => "b'. 10k `ui.virtual_list`, 100x50",
            Workload::Table => "c. table 1000x6, 200x60",
            Workload::TablePaged => "c'. table 1000x6, `page_size = 57`, 200x60",
            Workload::Nested => "d. 60 nested bordered boxes, 240x80",
            Workload::OneCell => "e. one cell changes per frame, 200x60",
            Workload::Idle => "f. idle (unchanged), 200x60",
        }
    }

    /// Terminal size `(width, height)` in cells.
    pub fn size(self) -> (u16, u16) {
        match self {
            Workload::List | Workload::VirtualList => (100, 50),
            Workload::Nested => (240, 80),
            _ => (200, 60),
        }
    }

    /// Whether ratatui has its own arm for this workload.
    pub fn has_ratatui_arm(self) -> bool {
        !matches!(self, Workload::VirtualList | Workload::TablePaged)
    }

    /// The workload whose ratatui arm this SLT workload is compared to.
    pub fn ratatui_counterpart(self) -> Workload {
        match self {
            Workload::VirtualList => Workload::List,
            Workload::TablePaged => Workload::Table,
            w => w,
        }
    }

    /// The frame tick fed to the dashboard; only e varies it.
    fn tick(self, tick: u64) -> Option<u64> {
        (self == Workload::OneCell).then_some(tick)
    }
}

/// SLT side of every workload.
pub mod slt_w {
    use slt::{Border, Context, Style};

    /// Dashboard: header, separator, 20 text rows, a progress bar. `tick =
    /// Some(t)` changes one digit in the first body row per frame.
    pub fn dashboard(ui: &mut Context, tick: Option<u64>) {
        let _ = ui.col(|ui| {
            ui.text("Header").bold();
            let _ = ui.separator();
            for i in 0..20 {
                if i == 0 {
                    ui.text(format!("Row {i} [{}]", tick.unwrap_or(0) % 10));
                } else {
                    ui.text(format!("Row {i}"));
                }
            }
            let _ = ui.progress(0.75);
        });
    }

    /// 10k-row list widget.
    pub fn list(ui: &mut Context, st: &mut slt::ListState) {
        let _ = ui.list(st);
    }

    /// Same rows through `virtual_list`, styled like `ui.list`.
    pub fn virtual_list(ui: &mut Context, st: &mut slt::ListState, items: &[String]) {
        let sel = st.selected_index();
        let _ = ui.virtual_list(st, super::VIRTUAL_LIST_ROWS, |ui, idx| {
            let item = &items[idx];
            if Some(idx) == sel {
                ui.styled(format!("▸ {item}"), Style::new().bold().reversed());
            } else {
                ui.text(format!("  {item}"));
            }
        });
    }

    /// 1000x6 table widget.
    pub fn table(ui: &mut Context, st: &mut slt::TableState) {
        let _ = ui.table(st);
    }

    /// 3 rows x 4 columns x 5 rounded boxes, each with a one-line label.
    pub fn nested(ui: &mut Context, labels: &[String]) {
        let _ = ui.container().grow(1).col(|ui| {
            for r in 0..3 {
                let _ = ui.container().grow(1).row(|ui| {
                    for c in 0..4 {
                        let _ = ui.container().grow(1).col(|ui| {
                            for b in 0..5 {
                                let label = &labels[r * 20 + c * 5 + b];
                                let _ = ui.bordered(Border::Rounded).grow(1).col(|ui| {
                                    ui.text(label.as_str());
                                });
                            }
                        });
                    }
                });
            }
        });
    }
}

/// ratatui side of every workload, written to draw the same visible text as
/// the SLT side (verified by [`check_equivalence`](crate::check_equivalence)).
///
/// Where a stock ratatui widget draws different glyphs or layout than the
/// SLT widget, the ratatui arm is adjusted to SLT's output, never the other
/// way round, so the SLT arm measures what SLT users get by default:
///
/// - progress bar: SLT draws `█` x round(ratio * 20) then `░`; ratatui's
///   `Gauge` / `LineGauge` cannot draw that, so the same 20-glyph line is
///   rendered as a styled `Paragraph` (also how SLT builds it).
/// - table: SLT sizes columns to content and draws ` │ ` column rules and
///   a `─┼─` header rule. ratatui gets precomputed `Length` widths, rule
///   columns and a two-line header.
/// - nested boxes: `Fill(1)` spreads the rounding remainder, SLT gives it
///   to the trailing items; `Length` constraints reproduce SLT's split.
pub mod rat_w {
    use ratatui::Frame;
    use ratatui::layout::{Constraint, Layout, Rect};
    use ratatui::style::{Color, Style, Stylize};
    use ratatui::text::{Line, Text};
    use ratatui::widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Row, Table, TableState,
    };

    /// Width of SLT's `ui.progress` bar.
    pub const PROGRESS_WIDTH: u16 = 20;
    /// Column rule SLT's table draws between columns.
    pub const COLUMN_RULE: &str = " │ ";
    /// Header-rule junction under [`COLUMN_RULE`].
    pub const HEADER_JUNCTION: &str = "─┼─";
    const COLUMN_RULE_WIDTH: u16 = 3;
    const HEADER_HEIGHT: u16 = 2;

    /// SLT's progress glyphs: `█` x round(ratio * width), then `░`.
    pub fn progress_line(ratio: f64, width: u16) -> String {
        let filled = (ratio.clamp(0.0, 1.0) * f64::from(width)).round() as usize;
        let empty = usize::from(width).saturating_sub(filled);
        let mut bar = String::with_capacity(usize::from(width) * 3);
        bar.extend(std::iter::repeat_n('█', filled));
        bar.extend(std::iter::repeat_n('░', empty));
        bar
    }

    /// Dashboard, see [`crate::slt_w::dashboard`].
    pub fn dashboard(f: &mut Frame<'_>, tick: Option<u64>) {
        use Constraint::{Fill, Length};
        let [header, sep, body, bar, _rest] =
            Layout::vertical([Length(1), Length(1), Length(20), Length(1), Fill(1)])
                .areas(f.area());
        f.render_widget(Paragraph::new("Header").bold(), header);
        f.render_widget(Block::new().borders(Borders::TOP), sep);
        let lines: Vec<Line<'_>> = (0..20)
            .map(|i| {
                if i == 0 {
                    Line::from(format!("Row {i} [{}]", tick.unwrap_or(0) % 10))
                } else {
                    Line::from(format!("Row {i}"))
                }
            })
            .collect();
        f.render_widget(Paragraph::new(lines), body);
        let bar_line = Line::styled(
            progress_line(0.75, PROGRESS_WIDTH),
            Style::new().fg(Color::Cyan),
        );
        f.render_widget(Paragraph::new(bar_line), bar);
    }

    /// 10k-row list with a `▸ ` highlight symbol.
    pub fn list(f: &mut Frame<'_>, items: &[String], st: &mut ListState) {
        let list = List::new(items.iter().map(|s| ListItem::new(s.as_str())))
            .highlight_style(Style::new().bold().reversed())
            .highlight_symbol("▸ ");
        f.render_stateful_widget(list, f.area(), st);
    }

    /// Column layout matching SLT's table, computed once per table.
    pub struct TableLayout {
        constraints: Vec<Constraint>,
        rules: Vec<String>,
    }

    impl TableLayout {
        /// Content-sized columns (max display width of header and cells).
        pub fn new(headers: &[String], rows: &[Vec<String>]) -> Self {
            let widths: Vec<u16> = (0..headers.len())
                .map(|c| {
                    let cells = std::iter::once(&headers[c]).chain(rows.iter().map(|r| &r[c]));
                    let w = cells
                        .map(|s| unicode_width::UnicodeWidthStr::width(s.as_str()))
                        .max();
                    u16::try_from(w.unwrap_or(0)).unwrap_or(u16::MAX)
                })
                .collect();
            let mut constraints = Vec::with_capacity(widths.len() * 2);
            for (i, w) in widths.iter().enumerate() {
                if i > 0 {
                    constraints.push(Constraint::Length(COLUMN_RULE_WIDTH));
                }
                constraints.push(Constraint::Length(*w));
            }
            let rules = widths.iter().map(|w| "─".repeat(usize::from(*w))).collect();
            Self { constraints, rules }
        }
    }

    /// Interleave `cells` with `rule` between neighbours.
    fn ruled<'a, C: Into<ratatui::widgets::Cell<'a>>>(
        cells: impl Iterator<Item = C>,
        rule: impl Fn() -> C,
    ) -> impl Iterator<Item = C> {
        cells
            .enumerate()
            .flat_map(move |(i, c)| (i > 0).then(&rule).into_iter().chain(std::iter::once(c)))
    }

    /// 1000x6 table drawn like SLT's `ui.table`.
    pub fn table(
        f: &mut Frame<'_>,
        headers: &[String],
        rows: &[Vec<String>],
        layout: &TableLayout,
        st: &mut TableState,
    ) {
        let header_cells = headers
            .iter()
            .zip(&layout.rules)
            .map(|(h, rule)| Text::from(vec![Line::from(h.as_str()), Line::from(rule.as_str())]));
        let header = Row::new(ruled(header_cells, || {
            Text::from(vec![Line::from(COLUMN_RULE), Line::from(HEADER_JUNCTION)])
        }))
        .height(HEADER_HEIGHT)
        .bold();
        let rows = rows
            .iter()
            .map(|r| Row::new(ruled(r.iter().map(String::as_str), || COLUMN_RULE)));
        let t = Table::new(rows, layout.constraints.iter().copied())
            .header(header)
            .column_spacing(0)
            .row_highlight_style(Style::new().bold().reversed());
        f.render_stateful_widget(t, f.area(), st);
    }

    /// `n` lengths summing to `total`, remainder on the trailing items
    /// (SLT's flex split for equal `grow`).
    pub fn trailing_split(total: u16, n: u16) -> impl Iterator<Item = Constraint> {
        let base = total / n;
        let extra_from = n - total % n;
        (0..n).map(move |i| Constraint::Length(base + u16::from(i >= extra_from)))
    }

    fn split_vertical(area: Rect, n: u16) -> std::rc::Rc<[Rect]> {
        Layout::vertical(trailing_split(area.height, n)).split(area)
    }

    /// 60 nested rounded boxes, see [`crate::slt_w::nested`].
    pub fn nested(f: &mut Frame<'_>, labels: &[String]) {
        let rows = split_vertical(f.area(), 3);
        for (r, ra) in rows.iter().enumerate() {
            let cols = Layout::horizontal(trailing_split(ra.width, 4)).split(*ra);
            for (c, ca) in cols.iter().enumerate() {
                let boxes = split_vertical(*ca, 5);
                for (b, ba) in boxes.iter().enumerate() {
                    let block = Block::bordered().border_type(BorderType::Rounded);
                    let inner = block.inner(*ba);
                    f.render_widget(block, *ba);
                    f.render_widget(Paragraph::new(labels[r * 20 + c * 5 + b].as_str()), inner);
                }
            }
        }
    }
}

/// Per-instance SLT workload state.
pub struct SltWorkload<'a> {
    kind: Workload,
    fx: &'a Fixtures,
    list: Option<slt::ListState>,
    table: Option<slt::TableState>,
}

impl<'a> SltWorkload<'a> {
    /// Fresh state for `kind`.
    pub fn new(kind: Workload, fx: &'a Fixtures) -> Self {
        let list = matches!(kind, Workload::List | Workload::VirtualList).then(|| {
            let mut st = slt::ListState::new(fx.list_items.clone());
            st.set_selected_index(LIST_SELECTED);
            st
        });
        let table = matches!(kind, Workload::Table | Workload::TablePaged).then(|| {
            let mut st = slt::TableState::new(fx.table_headers.clone(), fx.table_rows.clone());
            st.set_selected_index(TABLE_SELECTED);
            if kind == Workload::TablePaged {
                st.page_size = TABLE_PAGE_SIZE;
            }
            st
        });
        Self {
            kind,
            fx,
            list,
            table,
        }
    }

    /// Render one frame.
    pub fn render(&mut self, ui: &mut slt::Context, tick: u64) {
        match self.kind {
            Workload::Dashboard | Workload::OneCell | Workload::Idle => {
                slt_w::dashboard(ui, self.kind.tick(tick));
            }
            Workload::List => {
                if let Some(st) = self.list.as_mut() {
                    slt_w::list(ui, st);
                }
            }
            Workload::VirtualList => {
                if let Some(st) = self.list.as_mut() {
                    slt_w::virtual_list(ui, st, &self.fx.list_items);
                }
            }
            Workload::Table | Workload::TablePaged => {
                if let Some(st) = self.table.as_mut() {
                    slt_w::table(ui, st);
                }
            }
            Workload::Nested => slt_w::nested(ui, &self.fx.box_labels),
        }
    }
}

/// Per-instance ratatui workload state.
pub struct RatWorkload<'a> {
    kind: Workload,
    fx: &'a Fixtures,
    list: ratatui::widgets::ListState,
    table: ratatui::widgets::TableState,
    table_layout: Option<rat_w::TableLayout>,
}

impl<'a> RatWorkload<'a> {
    /// Fresh state for `kind`. The table column layout is computed here,
    /// once, as an app would cache it.
    ///
    /// # Panics
    ///
    /// If `kind` has no ratatui arm (b', c').
    pub fn new(kind: Workload, fx: &'a Fixtures) -> Self {
        assert!(kind.has_ratatui_arm(), "{kind:?} has no ratatui arm");
        let table_layout = (kind == Workload::Table)
            .then(|| rat_w::TableLayout::new(&fx.table_headers, &fx.table_rows));
        Self {
            kind,
            fx,
            list: ratatui::widgets::ListState::default().with_selected(Some(LIST_SELECTED)),
            table: ratatui::widgets::TableState::default().with_selected(Some(TABLE_SELECTED)),
            table_layout,
        }
    }

    /// Render one frame.
    pub fn render(&mut self, f: &mut ratatui::Frame<'_>, tick: u64) {
        match self.kind {
            Workload::Dashboard | Workload::OneCell | Workload::Idle => {
                rat_w::dashboard(f, self.kind.tick(tick));
            }
            Workload::List => rat_w::list(f, &self.fx.list_items, &mut self.list),
            Workload::Table => {
                if let Some(layout) = self.table_layout.as_ref() {
                    rat_w::table(
                        f,
                        &self.fx.table_headers,
                        &self.fx.table_rows,
                        layout,
                        &mut self.table,
                    );
                }
            }
            Workload::Nested => rat_w::nested(f, &self.fx.box_labels),
            Workload::VirtualList | Workload::TablePaged => {
                unreachable!("rejected in RatWorkload::new")
            }
        }
    }
}

// ───────────────────────── SLT ANSI sink ─────────────────────────

/// [`slt::Backend`] writing ANSI into a `Vec<u8>`.
///
/// `flush` mirrors `slt::Terminal::flush` (src/terminal.rs) for the paths
/// these workloads reach: BSU, the written-span row skip plus the
/// run-coalesced diff (`__bench_flush_buffer_diff_mut_with_buf`, which calls
/// the same `flush_buffer_diff` kernel production uses), ESU, swap, then
/// `reset_with_bg` with the theme background exactly as `run_with`
/// configures it. Omitted because they are no-ops for these workloads:
/// width-shrink clear, sprixel row invalidation, kitty / raw / sprixel
/// passes, cursor (no cursor is placed and it starts hidden).
///
/// This is a mirror, not the production code path. Replacing it with a
/// persistent `#[doc(hidden)]` sink `Terminal` is tracked in #482.
pub struct SltSink {
    cur: slt::Buffer,
    prev: slt::Buffer,
    out: Vec<u8>,
    run_buf: String,
    bg: Option<slt::Color>,
    flush_ns: u64,
}

const BSU: &[u8] = b"\x1b[?2026h";
const ESU: &[u8] = b"\x1b[?2026l";

impl SltSink {
    /// Sink of `w` x `h` cells with the `run_with` theme background policy
    /// for `cfg`.
    pub fn new(w: u32, h: u32, cfg: &slt::RunConfig) -> Self {
        let area = slt::Rect::new(0, 0, w, h);
        // Mirrors run_with: theme_bg is only set when theme.bg != Reset.
        let bg = Some(cfg.theme.bg).filter(|c| *c != slt::Color::Reset);
        Self {
            cur: slt::Buffer::empty(area),
            prev: slt::Buffer::empty(area),
            out: Vec::with_capacity(SINK_CAPACITY),
            run_buf: String::with_capacity(RUN_BUF_CAPACITY),
            bg,
            flush_ns: 0,
        }
    }

    /// Bytes emitted since the last [`clear_output`](Self::clear_output).
    pub fn output(&self) -> &[u8] {
        &self.out
    }

    /// Drop captured bytes (capacity is kept).
    pub fn clear_output(&mut self) {
        self.out.clear();
    }

    /// The most recently flushed frame.
    pub fn last_frame(&self) -> &slt::Buffer {
        &self.prev
    }

    /// Cumulative nanoseconds spent in `flush`.
    pub fn flush_ns(&self) -> u64 {
        self.flush_ns
    }
}

impl slt::Backend for SltSink {
    fn size(&self) -> (u32, u32) {
        (self.cur.area.width, self.cur.area.height)
    }

    fn buffer_mut(&mut self) -> &mut slt::Buffer {
        &mut self.cur
    }

    fn flush(&mut self) -> io::Result<()> {
        let t = Instant::now();
        self.out.extend_from_slice(BSU);
        slt::__bench_flush_buffer_diff_mut_with_buf(
            &mut self.out,
            &mut self.cur,
            &mut self.prev,
            slt::ColorDepth::TrueColor,
            &mut self.run_buf,
        )?;
        self.out.extend_from_slice(ESU);
        std::mem::swap(&mut self.cur, &mut self.prev);
        match self.bg {
            Some(bg) => self.cur.reset_with_bg(bg),
            None => self.cur.reset(),
        }
        self.flush_ns += t.elapsed().as_nanos() as u64;
        Ok(())
    }
}

/// SLT sink runner: `slt::frame_owned` into an [`SltSink`].
pub struct SltSinkRunner<'a> {
    sink: SltSink,
    app: slt::AppState,
    cfg: slt::RunConfig,
    work: SltWorkload<'a>,
}

impl<'a> SltSinkRunner<'a> {
    /// New runner for `kind`.
    pub fn new(kind: Workload, fx: &'a Fixtures) -> Self {
        let (w, h) = kind.size();
        let cfg = slt::RunConfig::default();
        Self {
            sink: SltSink::new(u32::from(w), u32::from(h), &cfg),
            app: slt::AppState::new(),
            cfg,
            work: SltWorkload::new(kind, fx),
        }
    }

    /// Run one frame; returns the bytes it emitted.
    pub fn frame(&mut self, tick: u64) -> io::Result<usize> {
        self.sink.clear_output();
        let work = &mut self.work;
        slt::frame_owned(
            &mut self.sink,
            &mut self.app,
            &self.cfg,
            Vec::new(),
            &mut |ui: &mut slt::Context| work.render(ui, tick),
        )?;
        Ok(self.sink.output().len())
    }

    /// The sink.
    pub fn sink(&self) -> &SltSink {
        &self.sink
    }
}

type RatSinkTerminal = Terminal<CrosstermBackend<Vec<u8>>>;

/// ratatui sink runner: `Terminal<CrosstermBackend<Vec<u8>>>` with a fixed
/// viewport. One frame is exactly `Terminal::try_draw` (its `autoresize` is
/// a no-op for `Viewport::Fixed`): `get_frame`, render, then
/// `apply_buffer` (diff + hide cursor + swap + backend flush). The two halves
/// are called separately only so the flush phase can be timed.
pub struct RatSinkRunner<'a> {
    term: RatSinkTerminal,
    work: RatWorkload<'a>,
    flush_ns: u64,
    last_frame: Option<ratatui::buffer::Buffer>,
    keep_frame: bool,
}

impl<'a> RatSinkRunner<'a> {
    /// New runner for `kind`.
    pub fn new(kind: Workload, fx: &'a Fixtures) -> io::Result<Self> {
        let (w, h) = kind.size();
        let term = Terminal::with_options(
            CrosstermBackend::new(Vec::<u8>::with_capacity(SINK_CAPACITY)),
            TerminalOptions {
                viewport: Viewport::Fixed(ratatui::layout::Rect::new(0, 0, w, h)),
            },
        )?;
        Ok(Self {
            term,
            work: RatWorkload::new(kind, fx),
            flush_ns: 0,
            last_frame: None,
            keep_frame: false,
        })
    }

    /// Keep a copy of each completed frame. Equivalence checks only: the
    /// clone would distort timing and allocation numbers.
    pub fn keep_frames(mut self) -> Self {
        self.keep_frame = true;
        self
    }

    /// Run one frame; returns the bytes it emitted.
    pub fn frame(&mut self, tick: u64) -> io::Result<usize> {
        self.term.backend_mut().writer_mut().clear();
        {
            let mut f = self.term.get_frame();
            self.work.render(&mut f, tick);
        }
        let t = Instant::now();
        let completed = self.term.apply_buffer()?;
        self.flush_ns += t.elapsed().as_nanos() as u64;
        if self.keep_frame {
            self.last_frame = Some(completed.buffer.clone());
        }
        Ok(self.term.backend().writer().len())
    }

    /// Bytes emitted by the last frame.
    pub fn output(&self) -> &[u8] {
        self.term.backend().writer()
    }

    /// Last completed frame, if [`keep_frames`](Self::keep_frames) is set.
    pub fn last_frame(&self) -> Option<&ratatui::buffer::Buffer> {
        self.last_frame.as_ref()
    }
}

// ───────────────────────── cases ─────────────────────────

/// Which library a case drives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lib {
    /// SuperLightTUI.
    Slt,
    /// ratatui.
    Ratatui,
}

impl Lib {
    /// Short name used in case names.
    pub fn name(self) -> &'static str {
        match self {
            Lib::Slt => "slt",
            Lib::Ratatui => "ratatui",
        }
    }
}

/// Where a frame ends up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Harness {
    /// In-memory `TestBackend` (render only, no ANSI flush).
    Mem,
    /// ANSI sink (render + diff + ANSI emission into a `Vec<u8>`).
    Sink,
}

impl Harness {
    /// Short name used in case names.
    pub fn name(self) -> &'static str {
        match self {
            Harness::Mem => "mem",
            Harness::Sink => "sink",
        }
    }
}

trait Runner {
    fn frame(&mut self, tick: u64) -> io::Result<usize>;
    fn flush_ns(&self) -> Option<u64>;
}

impl Runner for SltSinkRunner<'_> {
    fn frame(&mut self, tick: u64) -> io::Result<usize> {
        SltSinkRunner::frame(self, tick)
    }
    fn flush_ns(&self) -> Option<u64> {
        Some(self.sink.flush_ns())
    }
}

impl Runner for RatSinkRunner<'_> {
    fn frame(&mut self, tick: u64) -> io::Result<usize> {
        RatSinkRunner::frame(self, tick)
    }
    fn flush_ns(&self) -> Option<u64> {
        Some(self.flush_ns)
    }
}

struct SltMemRunner<'a> {
    tb: slt::TestBackend,
    work: SltWorkload<'a>,
}

impl Runner for SltMemRunner<'_> {
    fn frame(&mut self, tick: u64) -> io::Result<usize> {
        let work = &mut self.work;
        self.tb.render(|ui| work.render(ui, tick));
        black_box(&self.tb);
        Ok(0)
    }
    fn flush_ns(&self) -> Option<u64> {
        None
    }
}

struct RatMemRunner<'a> {
    term: Terminal<ratatui::backend::TestBackend>,
    work: RatWorkload<'a>,
}

impl Runner for RatMemRunner<'_> {
    fn frame(&mut self, tick: u64) -> io::Result<usize> {
        let work = &mut self.work;
        infallible(self.term.draw(|f| work.render(f, tick)));
        Ok(0)
    }
    fn flush_ns(&self) -> Option<u64> {
        None
    }
}

/// One runnable (workload, library, harness) triple.
pub struct Case<'a> {
    /// Workload.
    pub workload: Workload,
    /// Library.
    pub lib: Lib,
    /// Harness.
    pub harness: Harness,
    runner: Box<dyn Runner + 'a>,
}

impl<'a> Case<'a> {
    /// Build one case.
    ///
    /// # Panics
    ///
    /// If `lib` is ratatui and `workload` has no ratatui arm.
    pub fn new(
        workload: Workload,
        lib: Lib,
        harness: Harness,
        fx: &'a Fixtures,
    ) -> io::Result<Self> {
        let (w, h) = workload.size();
        let runner: Box<dyn Runner + 'a> = match (lib, harness) {
            (Lib::Slt, Harness::Sink) => Box::new(SltSinkRunner::new(workload, fx)),
            (Lib::Ratatui, Harness::Sink) => Box::new(RatSinkRunner::new(workload, fx)?),
            (Lib::Slt, Harness::Mem) => Box::new(SltMemRunner {
                tb: slt::TestBackend::new(u32::from(w), u32::from(h)),
                work: SltWorkload::new(workload, fx),
            }),
            (Lib::Ratatui, Harness::Mem) => Box::new(RatMemRunner {
                term: infallible(Terminal::new(ratatui::backend::TestBackend::new(w, h))),
                work: RatWorkload::new(workload, fx),
            }),
        };
        Ok(Self {
            workload,
            lib,
            harness,
            runner,
        })
    }

    /// `<workload>/<lib>/<harness>`, e.g. `a_dashboard_200x60/slt/sink`.
    pub fn name(&self) -> String {
        format!(
            "{}/{}/{}",
            self.workload.name(),
            self.lib.name(),
            self.harness.name()
        )
    }

    /// Run one frame; returns bytes emitted (0 for [`Harness::Mem`]).
    pub fn frame(&mut self, tick: u64) -> io::Result<usize> {
        self.runner.frame(tick)
    }

    /// Cumulative flush nanoseconds (sink harness only).
    pub fn flush_ns(&self) -> Option<u64> {
        self.runner.flush_ns()
    }
}

/// Every case for `harness`, in report order (SLT arm, then ratatui arm).
pub fn build_cases(fx: &Fixtures, harness: Harness) -> io::Result<Vec<Case<'_>>> {
    let mut cases = Vec::new();
    for w in Workload::ALL {
        cases.push(Case::new(w, Lib::Slt, harness, fx)?);
        if w.has_ratatui_arm() {
            cases.push(Case::new(w, Lib::Ratatui, harness, fx)?);
        }
    }
    Ok(cases)
}

// ───────────────────────── measurements ─────────────────────────

/// Frames run before allocation counting starts.
pub const ALLOC_WARMUP_FRAMES: u64 = 50;
/// Frames whose allocations are averaged.
pub const ALLOC_MEASURE_FRAMES: u64 = 200;

/// Steady-state allocation profile of a case.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameAllocs {
    /// Allocation calls (alloc + alloc_zeroed + realloc) per frame.
    pub allocs_per_frame: f64,
    /// Requested bytes per frame.
    pub bytes_per_frame: f64,
}

/// Warm `case` up, then count allocations over [`ALLOC_MEASURE_FRAMES`].
/// Requires [`alloc_count::CountingAlloc`] as the global allocator, and
/// counts only the calling thread.
pub fn measure_allocs(case: &mut Case<'_>) -> io::Result<FrameAllocs> {
    measure_allocs_with(case, ALLOC_WARMUP_FRAMES, ALLOC_MEASURE_FRAMES)
}

/// [`measure_allocs`] with explicit warm-up and measured frame counts.
///
/// # Panics
///
/// If `frames` is zero.
pub fn measure_allocs_with(
    case: &mut Case<'_>,
    warmup: u64,
    frames: u64,
) -> io::Result<FrameAllocs> {
    assert!(frames > 0, "measure at least one frame");
    for t in 0..warmup {
        black_box(case.frame(t)?);
    }
    let before = alloc_count::snapshot();
    for t in warmup..warmup + frames {
        black_box(case.frame(t)?);
    }
    let d = alloc_count::snapshot().since(before);
    let n = frames as f64;
    Ok(FrameAllocs {
        allocs_per_frame: d.calls as f64 / n,
        bytes_per_frame: d.bytes as f64 / n,
    })
}

/// Frames averaged for steady output bytes.
pub const BYTES_MEASURE_FRAMES: u64 = 20;

/// Output-byte profile of a sink case.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameBytes {
    /// Bytes emitted by the first frame (full paint).
    pub first: usize,
    /// Mean bytes over the next [`BYTES_MEASURE_FRAMES`] frames.
    pub steady_per_frame: f64,
}

/// Measure first-frame and steady bytes of a fresh sink case.
pub fn measure_bytes(case: &mut Case<'_>) -> io::Result<FrameBytes> {
    let first = case.frame(0)?;
    let mut total = 0usize;
    for t in 1..=BYTES_MEASURE_FRAMES {
        total += case.frame(t)?;
    }
    Ok(FrameBytes {
        first,
        steady_per_frame: total as f64 / BYTES_MEASURE_FRAMES as f64,
    })
}

// ───────────────────────── text equivalence ─────────────────────────

/// Visible text of an SLT buffer: one string per row, trailing spaces
/// trimmed, trailing empty rows dropped. Wide-glyph continuation cells
/// (empty symbol) contribute nothing.
pub fn slt_lines(buf: &slt::Buffer) -> Vec<String> {
    let a = buf.area;
    let rows = (a.y..a.y + a.height).map(|y| {
        let mut s = String::new();
        for x in a.x..a.x + a.width {
            s.push_str(&buf.get(x, y).symbol);
        }
        s.trim_end().to_string()
    });
    trim_trailing_empty(rows.collect())
}

/// Visible text of a ratatui buffer, same normalization as [`slt_lines`].
/// The cells a wide glyph covers after its first column contribute nothing
/// (ratatui stores them as blanks, SLT as empty continuation cells).
pub fn ratatui_lines(buf: &ratatui::buffer::Buffer) -> Vec<String> {
    let a = buf.area;
    let rows = (a.y..a.y + a.height).map(|y| {
        let mut s = String::new();
        let mut covered = 0usize;
        for x in a.x..a.x + a.width {
            if covered > 0 {
                covered -= 1;
                continue;
            }
            let symbol = buf[(x, y)].symbol();
            s.push_str(symbol);
            covered = UnicodeWidthStr::width(symbol).saturating_sub(1);
        }
        s.trim_end().to_string()
    });
    trim_trailing_empty(rows.collect())
}

fn trim_trailing_empty(mut lines: Vec<String>) -> Vec<String> {
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

/// SLT text of the first frame through the in-memory `TestBackend`.
pub fn slt_mem_text(kind: Workload, fx: &Fixtures) -> Vec<String> {
    let (w, h) = kind.size();
    let mut tb = slt::TestBackend::new(u32::from(w), u32::from(h));
    let mut work = SltWorkload::new(kind, fx);
    tb.render(|ui| work.render(ui, 0));
    slt_lines(tb.buffer())
}

/// SLT text of the first frame through the timed sink harness.
pub fn slt_sink_text(kind: Workload, fx: &Fixtures) -> io::Result<Vec<String>> {
    let mut r = SltSinkRunner::new(kind, fx);
    r.frame(0)?;
    Ok(slt_lines(r.sink().last_frame()))
}

/// ratatui text of the first frame through `Terminal<TestBackend>`.
pub fn ratatui_mem_text(kind: Workload, fx: &Fixtures) -> Vec<String> {
    let (w, h) = kind.size();
    let mut term = infallible(Terminal::new(ratatui::backend::TestBackend::new(w, h)));
    let mut work = RatWorkload::new(kind, fx);
    infallible(term.draw(|f| work.render(f, 0)));
    ratatui_lines(term.backend().buffer())
}

/// ratatui text of the first frame through the timed sink harness.
pub fn ratatui_sink_text(kind: Workload, fx: &Fixtures) -> io::Result<Vec<String>> {
    let mut r = RatSinkRunner::new(kind, fx)?.keep_frames();
    r.frame(0)?;
    Ok(r.last_frame().map(ratatui_lines).unwrap_or_default())
}

/// A row where the two renderings differ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowDiff {
    /// Row index.
    pub row: usize,
    /// SLT text of the row.
    pub slt: String,
    /// ratatui text of the row.
    pub ratatui: String,
}

/// Result of comparing one SLT workload with its ratatui counterpart.
#[derive(Clone, Debug)]
pub struct Equivalence {
    /// SLT workload (b' / c' compare against the ratatui arm of b / c).
    pub workload: Workload,
    /// SLT rows (in-memory harness).
    pub slt: Vec<String>,
    /// ratatui rows (in-memory harness).
    pub ratatui: Vec<String>,
    /// Rows that differ.
    pub diffs: Vec<RowDiff>,
    /// The sink harness drew the same text as the in-memory one, for both
    /// libraries, so the timed path draws what was checked.
    pub sink_matches_mem: bool,
}

impl Equivalence {
    /// Rows that differ and are not in [`documented_exceptions`].
    pub fn unexpected(&self) -> impl Iterator<Item = &RowDiff> {
        let allowed = documented_exceptions(self.workload);
        self.diffs.iter().filter(move |d| !allowed.contains(&d.row))
    }

    /// Whether every row matches, apart from documented exceptions.
    pub fn is_match(&self) -> bool {
        self.unexpected().next().is_none()
    }
}

/// Rows allowed to differ, per SLT workload. Only the SLT-only variants
/// have any: they are compared against the plain ratatui arm and add one
/// SLT-specific row each.
///
/// - b' (`virtual_list`): row 49 is the `↓ N more` overflow hint.
/// - c' (`page_size = 57`): row 59 is the `Page 1/18` pager.
pub fn documented_exceptions(kind: Workload) -> &'static [usize] {
    match kind {
        Workload::VirtualList => &[49],
        Workload::TablePaged => &[59],
        _ => &[],
    }
}

/// Compare the first frame of `kind` (SLT) with its ratatui counterpart.
pub fn check_equivalence(kind: Workload, fx: &Fixtures) -> io::Result<Equivalence> {
    let counterpart = kind.ratatui_counterpart();
    let slt = slt_mem_text(kind, fx);
    let ratatui = ratatui_mem_text(counterpart, fx);
    let sink_matches_mem =
        slt_sink_text(kind, fx)? == slt && ratatui_sink_text(counterpart, fx)? == ratatui;
    let rows = slt.len().max(ratatui.len());
    let diffs = (0..rows)
        .filter_map(|row| {
            let a = slt.get(row).map(String::as_str).unwrap_or_default();
            let b = ratatui.get(row).map(String::as_str).unwrap_or_default();
            (a != b).then(|| RowDiff {
                row,
                slt: a.to_string(),
                ratatui: b.to_string(),
            })
        })
        .collect();
    Ok(Equivalence {
        workload: kind,
        slt,
        ratatui,
        diffs,
        sink_matches_mem,
    })
}

// ───────────────────────── environment header ─────────────────────────

/// Markdown bullet list recording host, toolchain, profile, features and
/// the superlighttui revision.
pub fn env_header(bench_features: &str) -> String {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    format!(
        "- host: {} ({}-{})\n- rustc: {}\n- profile: {profile}\n- superlighttui: {} (default features)\n- ratatui: {RATATUI_VERSION}\n- bench features: {bench_features}\n",
        cpu_brand(),
        std::env::consts::ARCH,
        std::env::consts::OS,
        env!("SLT_COMPARE_RUSTC"),
        source_revision(),
    )
}

fn command_stdout(cmd: &mut Command) -> Option<String> {
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

fn cpu_brand() -> String {
    if cfg!(target_os = "macos")
        && let Some(s) =
            command_stdout(Command::new("sysctl").args(["-n", "machdep.cpu.brand_string"]))
    {
        return s;
    }
    if let Ok(info) = std::fs::read_to_string("/proc/cpuinfo")
        && let Some(name) = info
            .lines()
            .find(|l| l.starts_with("model name"))
            .and_then(|l| l.split(':').nth(1))
    {
        return name.trim().to_string();
    }
    "unknown CPU".to_string()
}

/// Short SHA (plus `-dirty` when tracked files are modified) and
/// `describe --tags` of the superlighttui checkout this crate builds against.
pub fn source_revision() -> String {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let vcs = |args: &[&str]| command_stdout(Command::new("git").arg("-C").arg(root).args(args));
    let sha = vcs(&["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let describe = vcs(&["describe", "--tags", "--always"]).unwrap_or_default();
    let dirty = vcs(&["status", "--porcelain", "--untracked-files=no"]).is_some();
    let suffix = if dirty { "-dirty" } else { "" };
    format!("{sha}{suffix} ({describe})")
}

// ───────────────────────── counting allocator ─────────────────────────

/// Counting global allocator with thread-local counters, so parallel test
/// threads never pollute each other's numbers.
pub mod alloc_count {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static CALLS: Cell<u64> = const { Cell::new(0) };
        static BYTES: Cell<u64> = const { Cell::new(0) };
    }

    /// Install with `#[global_allocator] static A: CountingAlloc = CountingAlloc;`.
    pub struct CountingAlloc;

    #[inline]
    fn record(size: usize) {
        // `try_with`: never panic inside the allocator (thread teardown).
        let _ = CALLS.try_with(|c| c.set(c.get() + 1));
        let _ = BYTES.try_with(|b| b.set(b.get() + size as u64));
    }

    // SAFETY: every method forwards to `System` with the caller's arguments
    // unchanged; the only addition is a const-initialized, destructor-free
    // thread-local counter bump, which never allocates.
    unsafe impl GlobalAlloc for CountingAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record(layout.size());
            // SAFETY: same contract as the caller's.
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record(layout.size());
            // SAFETY: same contract as the caller's.
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            // SAFETY: same contract as the caller's.
            unsafe { System.dealloc(ptr, layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            record(new_size);
            // SAFETY: same contract as the caller's.
            unsafe { System.realloc(ptr, layout, new_size) }
        }
    }

    /// Counter snapshot for the current thread.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct AllocStats {
        /// Allocation calls.
        pub calls: u64,
        /// Requested bytes.
        pub bytes: u64,
    }

    impl AllocStats {
        /// Difference `self - earlier`.
        pub fn since(self, earlier: AllocStats) -> AllocStats {
            AllocStats {
                calls: self.calls - earlier.calls,
                bytes: self.bytes - earlier.bytes,
            }
        }
    }

    /// Current thread's counters.
    pub fn snapshot() -> AllocStats {
        AllocStats {
            calls: CALLS.with(Cell::get),
            bytes: BYTES.with(Cell::get),
        }
    }

    /// Whether [`CountingAlloc`] is the active global allocator.
    pub fn is_installed() -> bool {
        let before = snapshot();
        std::hint::black_box(Vec::<u8>::with_capacity(16));
        snapshot().calls > before.calls
    }
}
