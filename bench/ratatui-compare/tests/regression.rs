//! Deterministic regression gate for the matched harness (issue #482).
//!
//! No wall-clock assertions: only properties that are identical on every
//! run and host for a given source tree.
//!
//! 1. Visible-text equivalence: each SLT workload draws the same text as its
//!    ratatui arm (documented exceptions only), and the timed sink harness
//!    draws the same text as the in-memory one.
//! 2. Allocation ceilings: steady-state allocation calls and requested bytes
//!    per SLT frame stay under generous fixed ceilings.
//! 3. Output-byte ceilings: first-frame and steady bytes per SLT frame.
//!
//! Ceilings are upper bounds, so improvements pass. When a fix lands
//! (#483-#491), lower the matching ceiling in the same PR so the gain is
//! locked in; raising one must be an intentional, reviewed diff.
//!
//! Run: `cargo test --release` (or plain `cargo test`) in
//! `bench/ratatui-compare`.

use slt_ratatui_compare::alloc_count::{self, CountingAlloc};
use slt_ratatui_compare::{
    Case, Fixtures, Harness, Lib, Workload, check_equivalence, measure_allocs_with, measure_bytes,
};

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

/// Short windows keep the debug-profile run fast; counts are per-thread and
/// steady after the first couple of frames.
const WARMUP_FRAMES: u64 = 10;
const MEASURE_FRAMES: u64 = 20;

struct AllocCeiling {
    workload: Workload,
    /// Allocation calls per frame.
    allocs: f64,
    /// Requested bytes per frame.
    bytes: f64,
}

/// SLT sink harness baselines in parentheses: v0.25.0 (2eb49b6) first, then
/// v0.26 (inline container commands, widget rows formatted into strings
/// recycled from the previous frame). What v0.26 still allocates is text the
/// workload hands to `ui.text` (`format!` rows, `&str` labels), so those
/// ceilings sit ~10% above the baseline; the widgets that format their own
/// rows must stay at zero.
const ALLOC_CEILINGS: &[AllocCeiling] = &[
    // (54 / 27_664 -> 21 / 172): 20 `format!` rows and the `"Header"` `&str`.
    AllocCeiling {
        workload: Workload::Dashboard,
        allocs: 23.0,
        bytes: 260.0,
    },
    // (109 / 55_958 after #487 -> 0 / 0); was 20_018 / 13_798_852 when every
    // row was built each frame.
    AllocCeiling {
        workload: Workload::List,
        allocs: 0.0,
        bytes: 0.0,
    },
    // (160 / 56_166 -> 98 / 2_064): two `format!` growths per visible row in
    // the workload's own row closure.
    AllocCeiling {
        workload: Workload::VirtualList,
        allocs: 108.0,
        bytes: 2_400.0,
    },
    // (129 / 60_222 after T10 and #487 -> 0 / 0); was 9_038 / 1_148_024.
    AllocCeiling {
        workload: Workload::Table,
        allocs: 0.0,
        bytes: 0.0,
    },
    // (131 / 60_156 after T10 -> 0 / 0); was 551 / 71_013.
    AllocCeiling {
        workload: Workload::TablePaged,
        allocs: 0.0,
        bytes: 0.0,
    },
    // (285 / 168_254 -> 60 / 350): one `String` per `&str` label.
    AllocCeiling {
        workload: Workload::Nested,
        allocs: 66.0,
        bytes: 420.0,
    },
    // (56 / 27_674 -> 21 / 172)
    AllocCeiling {
        workload: Workload::OneCell,
        allocs: 23.0,
        bytes: 260.0,
    },
    // (54 / 27_664 -> 21 / 172)
    AllocCeiling {
        workload: Workload::Idle,
        allocs: 23.0,
        bytes: 260.0,
    },
];

struct ByteCeiling {
    workload: Workload,
    /// Bytes of the first (full-paint) frame.
    first: usize,
    /// Mean bytes per steady frame.
    steady: f64,
}

/// Baseline at v0.25.0 (2eb49b6) in parentheses. Output bytes are exact, so
/// steady ceilings equal the baseline; first-frame ceilings allow ~5% for
/// theme or SGR-encoding changes. 16 steady bytes = BSU + ESU only.
const BYTE_CEILINGS: &[ByteCeiling] = &[
    // (1_008 / 16)
    ByteCeiling {
        workload: Workload::Dashboard,
        first: 1_060,
        steady: 16.0,
    },
    // (2_309 / 16)
    ByteCeiling {
        workload: Workload::List,
        first: 2_430,
        steady: 16.0,
    },
    // (2_300 / 16)
    ByteCeiling {
        workload: Workload::VirtualList,
        first: 2_420,
        steady: 16.0,
    },
    // (5_292 / 16)
    ByteCeiling {
        workload: Workload::Table,
        first: 5_560,
        steady: 16.0,
    },
    // (5_238 / 16)
    ByteCeiling {
        workload: Workload::TablePaged,
        first: 5_500,
        steady: 16.0,
    },
    // (26_430 / 16)
    ByteCeiling {
        workload: Workload::Nested,
        first: 27_760,
        steady: 16.0,
    },
    // (1_008 / 44) — 36 since #489 dropped the duplicate `CSI 0m` before the
    // first styled run and at the end of the frame (ratatui: 32).
    ByteCeiling {
        workload: Workload::OneCell,
        first: 1_060,
        steady: 36.0,
    },
    // (1_008 / 16)
    ByteCeiling {
        workload: Workload::Idle,
        first: 1_060,
        steady: 16.0,
    },
];

#[test]
fn counting_allocator_is_installed() {
    assert!(alloc_count::is_installed());
}

#[test]
fn every_workload_has_ceilings() {
    for w in Workload::ALL {
        assert!(
            ALLOC_CEILINGS.iter().any(|c| c.workload == w),
            "no alloc ceiling for {w:?}"
        );
        assert!(
            BYTE_CEILINGS.iter().any(|c| c.workload == w),
            "no byte ceiling for {w:?}"
        );
    }
}

#[test]
fn rendered_text_matches_ratatui_per_workload() {
    let fx = Fixtures::new();
    let mut failures = Vec::new();
    for w in Workload::ALL {
        let eq = check_equivalence(w, &fx).expect("render");
        if !eq.sink_matches_mem {
            failures.push(format!(
                "{}: sink harness text differs from TestBackend text",
                w.name()
            ));
        }
        for d in eq.unexpected() {
            failures.push(format!(
                "{} row {}:\n  slt     {:?}\n  ratatui {:?}",
                w.name(),
                d.row,
                d.slt,
                d.ratatui
            ));
        }
        assert!(!eq.slt.is_empty(), "{}: SLT drew nothing", w.name());
    }
    assert!(
        failures.is_empty(),
        "text mismatch:\n{}",
        failures.join("\n")
    );
}

#[test]
fn slt_steady_allocations_within_ceilings() {
    let fx = Fixtures::new();
    let mut failures = Vec::new();
    for c in ALLOC_CEILINGS {
        let mut case = Case::new(c.workload, Lib::Slt, Harness::Sink, &fx).expect("case");
        let got = measure_allocs_with(&mut case, WARMUP_FRAMES, MEASURE_FRAMES).expect("frame");
        eprintln!(
            "{}: {:.1} allocs/frame (ceiling {}), {:.0} B/frame (ceiling {})",
            case.name(),
            got.allocs_per_frame,
            c.allocs,
            got.bytes_per_frame,
            c.bytes
        );
        if got.allocs_per_frame > c.allocs || got.bytes_per_frame > c.bytes {
            failures.push(format!(
                "{}: {:.1} allocs/frame (ceiling {}), {:.0} B/frame (ceiling {})",
                case.name(),
                got.allocs_per_frame,
                c.allocs,
                got.bytes_per_frame,
                c.bytes
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "allocation ceiling exceeded:\n{}",
        failures.join("\n")
    );
}

#[test]
fn slt_output_bytes_within_ceilings() {
    let fx = Fixtures::new();
    let mut failures = Vec::new();
    for c in BYTE_CEILINGS {
        let mut case = Case::new(c.workload, Lib::Slt, Harness::Sink, &fx).expect("case");
        let got = measure_bytes(&mut case).expect("frame");
        eprintln!(
            "{}: first {} B (ceiling {}), steady {:.1} B/frame (ceiling {})",
            case.name(),
            got.first,
            c.first,
            got.steady_per_frame,
            c.steady
        );
        if got.first > c.first || got.steady_per_frame > c.steady {
            failures.push(format!(
                "{}: first {} B (ceiling {}), steady {:.1} B/frame (ceiling {})",
                case.name(),
                got.first,
                c.first,
                got.steady_per_frame,
                c.steady
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "output byte ceiling exceeded:\n{}",
        failures.join("\n")
    );
}

#[test]
fn idle_frame_emits_only_sync_guards() {
    // An unchanged frame must not repaint anything: exactly BSU + ESU.
    let fx = Fixtures::new();
    let mut r = slt_ratatui_compare::SltSinkRunner::new(Workload::Idle, &fx);
    for t in 0..3 {
        r.frame(t).expect("frame");
    }
    assert_eq!(r.sink().output(), b"\x1b[?2026h\x1b[?2026l");
}
