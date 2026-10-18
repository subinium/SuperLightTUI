//! Retained-memory measurement for large `ListState` / `TableState` (#416).
//!
//! Ignored by default: it prints numbers instead of asserting a budget.
//! Run in release so the allocator sees production layouts:
//!
//! ```sh
//! cargo test --release --test collections_retained_memory -- --ignored --nocapture --test-threads=1
//! ```
//!
//! "requested" is the sum of allocation sizes since the previous checkpoint
//! (including transient buffers); "live" is bytes currently allocated by the
//! process-wide counting allocator (alloc - dealloc, realloc as a delta), so a
//! live delta is what the step retained. RSS is read with `ps` and includes
//! allocator caching, so it only moves in page-sized, allocator-dependent steps.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use slt::TestBackend;
use slt::widgets::{ListState, TableState};

struct TrackingAlloc;

static LIVE: AtomicI64 = AtomicI64::new(0);
static REQUESTED: AtomicU64 = AtomicU64::new(0);

// SAFETY: every method forwards to `System` with the caller's arguments; the
// only addition is lock-free atomic counter updates, which never allocate.
unsafe impl GlobalAlloc for TrackingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size() as i64, Ordering::Relaxed);
        REQUESTED.fetch_add(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size() as i64, Ordering::Relaxed);
        REQUESTED.fetch_add(layout.size() as u64, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as i64, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LIVE.fetch_add(new_size as i64 - layout.size() as i64, Ordering::Relaxed);
        REQUESTED.fetch_add(new_size as u64, Ordering::Relaxed);
        // SAFETY: same contract as the caller's.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: TrackingAlloc = TrackingAlloc;

/// Resident set size in KiB, or `None` where `ps` is unavailable.
fn rss_kib() -> Option<u64> {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    String::from_utf8(out.stdout).ok()?.trim().parse().ok()
}

struct Probe {
    live: i64,
    requested: u64,
}

impl Probe {
    fn start() -> Self {
        Self {
            live: LIVE.load(Ordering::Relaxed),
            requested: REQUESTED.load(Ordering::Relaxed),
        }
    }

    /// Print the step's requested and retained bytes and reset the baseline.
    fn step(&mut self, label: &str) {
        let live = LIVE.load(Ordering::Relaxed);
        let requested = REQUESTED.load(Ordering::Relaxed);
        let rss = rss_kib().map_or_else(|| "n/a".to_string(), |kib| format!("{kib} KiB"));
        eprintln!(
            "RETAINED {label:<44} requested={:>11} live_delta={:>11} live_total={:>11} rss={rss}",
            requested - self.requested,
            live - self.live,
            live,
        );
        self.live = live;
        self.requested = requested;
    }
}

const N: usize = 100_000;

#[test]
#[ignore = "retained-memory measurement (#416); prints, does not assert"]
fn list_state_retained_memory_at_100k() {
    let mut tb = TestBackend::new(100, 50);
    tb.render(|ui| {
        ui.text("warm");
    });
    let items: Vec<String> = (0..N).map(|i| format!("item-{i:06}")).collect();
    let mut probe = Probe::start();

    let mut list = ListState::new(items);
    probe.step("list: new(100k) [input Vec moved in]");
    for _ in 0..3 {
        tb.render(|ui| {
            let _ = ui.list(&mut list);
        });
    }
    probe.step("list: 3 frames of ui.list");
    list.set_filter("item-01");
    probe.step("list: set_filter (10k match)");
    list.set_filter("");
    probe.step("list: clear filter");
    list.set_item_heights(vec![2; N]);
    for _ in 0..3 {
        tb.render(|ui| {
            let _ = ui.virtual_list_variable(&mut list, 48, |ui, idx| {
                ui.text(format!("row {idx}"));
            });
        });
    }
    probe.step("list: heights + prefix sums (virtual_list_variable)");
    list.set_items((0..10).map(|i| format!("small-{i}")).collect::<Vec<_>>());
    probe.step("list: set_items(10)");
    tb.render(|ui| {
        let _ = ui.list(&mut list);
    });
    probe.step("list: 1 frame after shrink");
    drop(list);
    probe.step("list: drop");
}

#[test]
#[ignore = "retained-memory measurement (#416); prints, does not assert"]
fn table_state_retained_memory_at_100k() {
    let mut tb = TestBackend::new(200, 60);
    tb.render(|ui| {
        ui.text("warm");
    });
    let rows: Vec<Vec<String>> = (0..N)
        .map(|i| vec![i.to_string(), format!("user-{i:06}"), format!("{}", i % 97)])
        .collect();
    let mut probe = Probe::start();

    let mut table = TableState::new(vec!["ID", "Name", "Score"], rows);
    probe.step("table: new(100k x 3) [input Vec moved in]");
    for _ in 0..3 {
        tb.render(|ui| {
            let _ = ui.table(&mut table);
        });
    }
    probe.step("table: 3 frames of ui.table");
    table.set_filter("user-01");
    probe.step("table: set_filter (10k match)");
    table.sort_by(2);
    probe.step("table: sort_by(2)");
    table.set_filter("");
    table.clear_sort();
    probe.step("table: clear filter + sort");
    table.set_rows(
        (0..10)
            .map(|i| vec![i.to_string(), format!("small-{i}"), "0".to_string()])
            .collect::<Vec<_>>(),
    );
    probe.step("table: set_rows(10)");
    tb.render(|ui| {
        let _ = ui.table(&mut table);
    });
    probe.step("table: 1 frame after shrink");
    drop(table);
    probe.step("table: drop");
}
