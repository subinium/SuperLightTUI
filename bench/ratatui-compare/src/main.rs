//! SLT vs ratatui matched-harness comparison CLI (issue #482).
//!
//! ```text
//! cargo run --release -- time [filter]                          # µs/frame, bytes/frame, flush share
//! cargo run --release --features count-alloc -- alloc [filter]  # allocations per steady frame
//! cargo run --release -- bytes [filter]                         # output bytes + one-cell/idle dump
//! cargo run --release -- show [filter] [--dump]                 # text equivalence per workload
//! ```
//!
//! `filter` is a substring of the case name, e.g. `sink`, `a_dashboard`,
//! `slt/sink`.

use std::hint::black_box;
use std::io;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use slt_ratatui_compare::{
    Case, Fixtures, Harness, Lib, Workload, build_cases, check_equivalence, env_header,
    measure_bytes,
};

#[cfg(feature = "count-alloc")]
#[global_allocator]
static GLOBAL: slt_ratatui_compare::alloc_count::CountingAlloc =
    slt_ratatui_compare::alloc_count::CountingAlloc;

/// Minimum warm-up frames and duration (whichever is longer).
const WARMUP_FRAMES: usize = 200;
const WARMUP_TIME: Duration = Duration::from_millis(300);
/// Sampling stops after this long or this many samples, but never below
/// `MIN_SAMPLES`.
const SAMPLE_TIME: Duration = Duration::from_millis(1500);
const MAX_SAMPLES: usize = 20_000;
const MIN_SAMPLES: usize = 300;
/// Frames run before dumping a steady-state frame in `bytes` mode.
const BYTES_DUMP_FRAME: u64 = 3;

const FEATURES: &str = if cfg!(feature = "count-alloc") {
    "count-alloc"
} else {
    "none"
};

struct TimeRow {
    workload: Workload,
    lib: Lib,
    harness: Harness,
    name: String,
    first_bytes: usize,
    steady_bytes: f64,
    median_ns: u64,
    p95_ns: u64,
    mean_ns: u64,
    flush_mean_ns: Option<u64>,
    samples: usize,
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx]
}

fn us(ns: u64) -> String {
    format!("{:.1}", ns as f64 / 1000.0)
}

fn time_case(case: &mut Case<'_>) -> io::Result<TimeRow> {
    let mut tick = 0u64;
    let first_bytes = case.frame(tick)?;
    let warm_start = Instant::now();
    let mut warm = 0usize;
    while warm < WARMUP_FRAMES || warm_start.elapsed() < WARMUP_TIME {
        tick += 1;
        black_box(case.frame(tick)?);
        warm += 1;
    }
    let flush_before = case.flush_ns();
    let mut samples = Vec::with_capacity(MAX_SAMPLES);
    let mut bytes_total = 0usize;
    let start = Instant::now();
    while samples.len() < MIN_SAMPLES
        || (start.elapsed() < SAMPLE_TIME && samples.len() < MAX_SAMPLES)
    {
        tick += 1;
        let t = Instant::now();
        let b = case.frame(tick)?;
        samples.push(t.elapsed().as_nanos() as u64);
        bytes_total += b;
    }
    let n = samples.len();
    let mean_ns = samples.iter().sum::<u64>() / n as u64;
    let flush_mean_ns = case
        .flush_ns()
        .zip(flush_before)
        .map(|(after, before)| (after - before) / n as u64);
    samples.sort_unstable();
    Ok(TimeRow {
        workload: case.workload,
        lib: case.lib,
        harness: case.harness,
        name: case.name(),
        first_bytes,
        steady_bytes: bytes_total as f64 / n as f64,
        median_ns: percentile(&samples, 0.5),
        p95_ns: percentile(&samples, 0.95),
        mean_ns,
        flush_mean_ns,
        samples: n,
    })
}

fn time_mode(filter: &str) -> io::Result<ExitCode> {
    println!("## time\n\n{}", env_header(FEATURES));
    println!(
        "| case | first-frame bytes | steady bytes/frame | median µs | p95 µs | mean µs | flush mean µs | n |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    let fx = Fixtures::new();
    let mut rows = Vec::new();
    for harness in [Harness::Mem, Harness::Sink] {
        for mut case in build_cases(&fx, harness)? {
            if !case.name().contains(filter) {
                continue;
            }
            let r = time_case(&mut case)?;
            let sink = r.harness == Harness::Sink;
            println!(
                "| {} | {} | {} | {} | {} | {} | {} | {} |",
                r.name,
                if sink {
                    r.first_bytes.to_string()
                } else {
                    "-".into()
                },
                if sink {
                    format!("{:.1}", r.steady_bytes)
                } else {
                    "-".into()
                },
                us(r.median_ns),
                us(r.p95_ns),
                us(r.mean_ns),
                r.flush_mean_ns.map(us).unwrap_or_else(|| "-".into()),
                r.samples
            );
            rows.push(r);
        }
    }
    print_summary(&rows);
    Ok(ExitCode::SUCCESS)
}

/// SLT vs ratatui medians on the sink harness, in the issue #482 layout.
fn print_summary(rows: &[TimeRow]) {
    let median = |w: Workload, lib: Lib| {
        rows.iter()
            .find(|r| r.workload == w && r.lib == lib && r.harness == Harness::Sink)
            .map(|r| r.median_ns)
    };
    if !rows.iter().any(|r| r.harness == Harness::Sink) {
        return;
    }
    println!("\n### Summary (sink harness, median µs/frame)\n");
    println!(
        "| Workload | SLT | ratatui {} | SLT / ratatui |",
        slt_ratatui_compare::RATATUI_VERSION
    );
    println!("|---|---|---|---|");
    for w in Workload::ALL {
        let Some(s) = median(w, Lib::Slt) else {
            continue;
        };
        let r = median(w, Lib::Ratatui);
        println!(
            "| {} | {} | {} | {} |",
            w.label(),
            us(s),
            r.map(us).unwrap_or_else(|| "—".into()),
            r.map(|r| format!("{:.1}x", s as f64 / r as f64))
                .unwrap_or_else(|| "—".into()),
        );
    }
}

#[cfg(feature = "count-alloc")]
fn alloc_mode(filter: &str) -> io::Result<ExitCode> {
    use slt_ratatui_compare::{
        ALLOC_MEASURE_FRAMES, ALLOC_WARMUP_FRAMES, alloc_count, measure_allocs,
    };
    if !alloc_count::is_installed() {
        eprintln!("counting allocator is not installed");
        return Ok(ExitCode::FAILURE);
    }
    println!("## alloc\n\n{}", env_header(FEATURES));
    println!(
        "Steady state: {ALLOC_WARMUP_FRAMES} warm-up frames, then the mean over {ALLOC_MEASURE_FRAMES} frames.\n"
    );
    println!("| case | allocs/frame | requested bytes/frame |");
    println!("|---|---|---|");
    let fx = Fixtures::new();
    for harness in [Harness::Mem, Harness::Sink] {
        for mut case in build_cases(&fx, harness)? {
            if !case.name().contains(filter) {
                continue;
            }
            let a = measure_allocs(&mut case)?;
            println!(
                "| {} | {:.1} | {:.0} |",
                case.name(),
                a.allocs_per_frame,
                a.bytes_per_frame
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(not(feature = "count-alloc"))]
fn alloc_mode(_filter: &str) -> io::Result<ExitCode> {
    eprintln!("alloc mode needs the counting allocator: re-run with --features count-alloc");
    Ok(ExitCode::from(2))
}

fn bytes_mode(filter: &str) -> io::Result<ExitCode> {
    println!("## bytes\n\n{}", env_header(FEATURES));
    println!("| case | first-frame bytes | steady bytes/frame |");
    println!("|---|---|---|");
    let fx = Fixtures::new();
    for mut case in build_cases(&fx, Harness::Sink)? {
        if !case.name().contains(filter) {
            continue;
        }
        let b = measure_bytes(&mut case)?;
        println!(
            "| {} | {} | {:.1} |",
            case.name(),
            b.first,
            b.steady_per_frame
        );
    }
    for w in [Workload::OneCell, Workload::Idle] {
        if !w.name().contains(filter) {
            continue;
        }
        let mut slt = slt_ratatui_compare::SltSinkRunner::new(w, &fx);
        let mut rat = slt_ratatui_compare::RatSinkRunner::new(w, &fx)?;
        for t in 0..=BYTES_DUMP_FRAME {
            slt.frame(t)?;
            rat.frame(t)?;
        }
        println!("\n### {} frame {BYTES_DUMP_FRAME}\n", w.name());
        let s = slt.sink().output();
        let r = rat.output();
        println!(
            "- slt ({} bytes): {:?}",
            s.len(),
            String::from_utf8_lossy(s)
        );
        println!(
            "- ratatui ({} bytes): {:?}",
            r.len(),
            String::from_utf8_lossy(r)
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn show_mode(filter: &str, dump: bool) -> io::Result<ExitCode> {
    println!("## show\n\n{}", env_header(FEATURES));
    println!("| SLT workload | ratatui arm | visible text | sink == mem |");
    println!("|---|---|---|---|");
    let fx = Fixtures::new();
    let mut details = Vec::new();
    let mut all_ok = true;
    for w in Workload::ALL {
        if !w.name().contains(filter) {
            continue;
        }
        let eq = check_equivalence(w, &fx)?;
        all_ok &= eq.is_match() && eq.sink_matches_mem;
        let unexpected = eq.unexpected().count();
        let documented = eq.diffs.len() - unexpected;
        let text = match (unexpected, documented) {
            (0, 0) => "identical".to_string(),
            (0, d) => format!("identical except {d} documented row(s)"),
            (u, _) => format!("{u} rows differ"),
        };
        let sink = if eq.sink_matches_mem { "yes" } else { "NO" };
        println!(
            "| {} | {} | {text} | {sink} |",
            w.name(),
            w.ratatui_counterpart().name(),
        );
        details.push(eq);
    }
    for eq in &details {
        if !eq.diffs.is_empty() {
            println!("\n### {} differences\n", eq.workload.name());
            let documented = slt_ratatui_compare::documented_exceptions(eq.workload);
            for d in &eq.diffs {
                let tag = if documented.contains(&d.row) {
                    " (documented)"
                } else {
                    ""
                };
                println!("row {:>3}{tag}", d.row);
                println!("row {:>3} slt     : {:?}", d.row, d.slt);
                println!("row {:>3} ratatui : {:?}", d.row, d.ratatui);
            }
        }
        if dump {
            let fence = "```";
            println!(
                "\n### {} (slt)\n{fence}text\n{}\n{fence}",
                eq.workload.name(),
                eq.slt.join("\n")
            );
            println!(
                "\n### {} (ratatui)\n{fence}text\n{}\n{fence}",
                eq.workload.ratatui_counterpart().name(),
                eq.ratatui.join("\n")
            );
        }
    }
    Ok(if all_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: slt-ratatui-compare <time|alloc|bytes|show> [filter] [--dump]\n  alloc needs --features count-alloc; --dump applies to show"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dump = args.iter().any(|a| a == "--dump");
    let mut positional = args.iter().filter(|a| !a.starts_with("--"));
    let mode = positional.next().map(String::as_str).unwrap_or("time");
    let filter = positional.next().map(String::as_str).unwrap_or("");
    let result = match mode {
        "time" => time_mode(filter),
        "alloc" => alloc_mode(filter),
        "bytes" => bytes_mode(filter),
        "show" => show_mode(filter, dump),
        _ => return usage(),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{mode} failed: {e}");
            ExitCode::FAILURE
        }
    }
}
