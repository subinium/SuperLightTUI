//! Idle frame skipping (#490) through a real PTY.
//!
//! Spawns this test binary as a PTY child (driven by
//! `tests/support/idle_pty.py`) and checks that `RunConfig::idle_redraw(false)`
//! renders nothing while idle, keeps animations ticking, wakes promptly for
//! keys, resizes and async messages, and still shows a delayed tooltip with
//! no further input. The `#[ignore]`d `idle_cpu_report` records the idle CPU,
//! output bytes and frame counts quoted in docs/PERFORMANCE.md.

#![cfg(all(unix, feature = "crossterm"))]

use std::time::{Duration, Instant};

use serde_json::{Value, json};
use slt::{Border, Context, MouseMode, RunConfig, SpinnerState, Tween};

const CHILD: &str = "idle_pty_child";

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

/// A static 200x60-class dashboard: bordered panels and a 40-row table.
fn dashboard(ui: &mut Context) {
    ui.bordered(Border::Rounded)
        .title("Idle dashboard")
        .grow(1)
        .col(|ui| {
            ui.row(|ui| {
                for index in 0..4u32 {
                    ui.bordered(Border::Single).grow(1).col(|ui| {
                        ui.text(format!("Metric {index}")).bold();
                        ui.text(format!("{} req/s", 1000 + index * 37));
                    });
                }
            });
            for row in 0..40u32 {
                ui.row(|ui| {
                    ui.text(format!("service-{row:02}"));
                    ui.spacer();
                    ui.text(format!("{:>9}", row * 1234));
                    ui.text(" healthy");
                });
            }
        });
}

#[test]
fn idle_pty_child() {
    let Ok(case) = std::env::var("SLT_IDLE_CASE") else {
        return;
    };
    let skip = env_u64("SLT_IDLE_SKIP", 1) == 1;
    let tick = Duration::from_millis(env_u64("SLT_IDLE_TICK_MS", 16));
    let quit_after = Duration::from_millis(env_u64("SLT_IDLE_QUIT_MS", 2000));
    let config = RunConfig::default()
        .idle_redraw(!skip)
        .tick_rate(tick)
        .handle_suspend(false);
    let start = Instant::now();
    let mut frames: Vec<f64> = Vec::new();
    let mut extra = json!({});
    match case.as_str() {
        // Static content; quits on `q` or when the scheduler timer fires.
        "static" | "keys" => {
            let mut keys: Vec<f64> = Vec::new();
            let mut sizes: Vec<(u32, u32)> = Vec::new();
            slt::run_with(config, |ui| {
                frames.push(ms(start));
                if ui.key('a') {
                    keys.push(ms(start));
                }
                if sizes.last() != Some(&(ui.width(), ui.height())) {
                    sizes.push((ui.width(), ui.height()));
                }
                dashboard(ui);
                if ui.key('q') || ui.schedule("quit", quit_after) {
                    ui.quit();
                }
            })
            .unwrap();
            extra = json!({"keys": keys, "sizes": sizes});
        }
        // A spinner and a 60-tick tween keep the loop ticking on their own.
        "animation" => {
            let spinner = SpinnerState::dots();
            let mut tween = Tween::new(0.0, 100.0, 60);
            let mut tween_done_tick = None;
            let mut spinning = true;
            slt::run_with(config, |ui| {
                frames.push(ms(start));
                let value = tween.value(ui.tick());
                ui.text(format!("tween {value:.0}"));
                if tween.is_done() && tween_done_tick.is_none() {
                    tween_done_tick = Some(ui.tick());
                }
                if spinning {
                    ui.spinner(&spinner);
                }
                if ui.schedule("stop-spinner", Duration::from_millis(1500)) {
                    spinning = false;
                }
                if ui.schedule("quit", quit_after) {
                    ui.quit();
                }
            })
            .unwrap();
            extra = json!({"tween_done_tick": tween_done_tick});
        }
        // The app itself asks for every frame.
        "request" => {
            slt::run_with(config, |ui| {
                frames.push(ms(start));
                ui.text(format!("frame {}", ui.tick()));
                ui.request_frame();
                if ui.schedule("quit", quit_after) {
                    ui.quit();
                }
            })
            .unwrap();
        }
        // Hovering a button shows its tooltip after the delay, with no input.
        "tooltip" => {
            let config = config
                .mouse_mode(MouseMode::Hover)
                .tooltip_delay(Duration::from_millis(300));
            slt::run_with(config, |ui| {
                frames.push(ms(start));
                let _ = ui.button("HOVERME");
                ui.tooltip("TIPTEXT");
                if ui.schedule("quit", quit_after) {
                    ui.quit();
                }
            })
            .unwrap();
        }
        #[cfg(feature = "async")]
        "async" => {
            extra = async_case(config, start, &mut frames, quit_after);
        }
        other => panic!("unknown case {other}"),
    }
    println!(
        "SLT_RESULT {}",
        json!({"frames": frames, "extra": extra, "skip": skip})
    );
}

/// `SLT_IDLE_MESSAGES` (default 20) messages, one every 50 ms from 0.5 s;
/// each records its delivery latency.
#[cfg(feature = "async")]
fn async_case(
    config: RunConfig,
    start: Instant,
    frames: &mut Vec<f64>,
    quit_after: Duration,
) -> Value {
    use std::sync::{Arc, Mutex};
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let latencies = Arc::new(Mutex::new(Vec::<f64>::new()));
    let frame_times = Arc::new(Mutex::new(Vec::<f64>::new()));
    runtime.block_on(async {
        let (seen, times) = (Arc::clone(&latencies), Arc::clone(&frame_times));
        let run = slt::run_async_with::<Instant>(config, move |ui, messages| {
            times.lock().unwrap().push(ms(start));
            for sent in messages.drain(..) {
                seen.lock()
                    .unwrap()
                    .push(sent.elapsed().as_secs_f64() * 1000.0);
            }
            dashboard(ui);
            if ui.schedule("quit", quit_after) {
                ui.quit();
            }
        })
        .unwrap();
        let sender = run.sender();
        tokio::time::sleep(Duration::from_millis(500)).await;
        for _ in 0..env_u64("SLT_IDLE_MESSAGES", 20) {
            sender.send(Instant::now()).await.unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        // The quit timer ends the loop; dropping the last sender first would
        // end it early through the disconnect path.
        let joined = run.join().await;
        drop(sender);
        joined.unwrap();
    });
    frames.extend(frame_times.lock().unwrap().iter().copied());
    let mut sorted = latencies.lock().unwrap().clone();
    sorted.sort_by(f64::total_cmp);
    let pick = |q: f64| {
        let last = sorted.len().saturating_sub(1) as f64;
        sorted
            .get((last * q).round() as usize)
            .copied()
            .unwrap_or(0.0)
    };
    json!({
        "count": sorted.len(),
        "p50_ms": pick(0.5),
        "p95_ms": pick(0.95),
        "max_ms": sorted.last().copied().unwrap_or(0.0),
    })
}

struct Run {
    report: Value,
}

impl Run {
    fn result(&self) -> &Value {
        &self.report["result"]
    }

    fn frames(&self) -> Vec<f64> {
        self.result()["frames"]
            .as_array()
            .expect("frames")
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    }

    fn frames_between(&self, from_ms: f64, to_ms: f64) -> usize {
        self.frames()
            .iter()
            .filter(|t| **t >= from_ms && **t < to_ms)
            .count()
    }

    /// Output bytes the terminal received between two driver times (seconds).
    fn bytes_between(&self, from: f64, to: f64) -> u64 {
        self.report["chunks"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|chunk| {
                let at = chunk[0].as_f64()?;
                (at >= from && at < to).then(|| chunk[1].as_u64().unwrap())
            })
            .sum()
    }

    /// First output chunk at or after `from` (seconds).
    fn first_output_after(&self, from: f64) -> Option<f64> {
        self.report["chunks"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|chunk| chunk[0].as_f64())
            .find(|at| *at >= from)
    }

    fn action_time(&self, index: usize) -> f64 {
        self.report["actions"][index][0]
            .as_f64()
            .expect("action ran")
    }
}

fn drive(case: &str, size: (u16, u16), env: &[(&str, &str)], script: Value, watch: &[&str]) -> Run {
    let mut command = std::process::Command::new("python3");
    command
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/idle_pty.py"
        ))
        .arg(std::env::current_exe().unwrap())
        .arg(CHILD)
        .arg(case)
        .arg(size.0.to_string())
        .arg(size.1.to_string())
        .arg("90")
        .arg(script.to_string())
        .args(watch);
    for (key, value) in env {
        command.env(key, value);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("driver report");
    assert_eq!(report["status"], 0, "child failed: {report}");
    assert!(report["result"].is_object(), "no child result: {report}");
    Run { report }
}

#[test]
fn idle_skip_renders_nothing_while_idle() {
    let run = drive(
        "static",
        (80, 24),
        &[("SLT_IDLE_QUIT_MS", "2500")],
        json!([]),
        &[],
    );
    // Startup frame plus one follow-up, then nothing until the quit timer.
    let early = run.frames_between(0.0, 300.0);
    assert!(
        (1..=3).contains(&early),
        "startup frames: {:?}",
        run.frames()
    );
    assert_eq!(
        run.frames_between(300.0, 2400.0),
        0,
        "idle frames rendered: {:?}",
        run.frames()
    );
    // The quit timer itself wakes the loop.
    assert!(
        run.frames_between(2400.0, 3000.0) >= 1,
        "{:?}",
        run.frames()
    );
    let start = run.report["chunks"][0][0].as_f64().unwrap();
    assert_eq!(
        run.bytes_between(start + 0.4, start + 2.3),
        0,
        "idle output: {}",
        run.report
    );
}

#[test]
fn default_config_still_redraws_every_tick() {
    let run = drive(
        "static",
        (80, 24),
        &[("SLT_IDLE_SKIP", "0"), ("SLT_IDLE_QUIT_MS", "1000")],
        json!([]),
        &[],
    );
    // 16 ms tick, 60 fps cap: roughly 60 frames in one second.
    let frames = run.frames_between(0.0, 1000.0);
    assert!(frames >= 40, "default cadence regressed: {frames} frames");
}

#[test]
fn idle_skip_keeps_animations_and_requests_ticking() {
    let run = drive(
        "animation",
        (80, 24),
        &[("SLT_IDLE_QUIT_MS", "2500")],
        json!([]),
        &[],
    );
    // Spinner (until 1.5 s) and tween keep the tick-rate cadence.
    let animating = run.frames_between(100.0, 1400.0);
    assert!(animating >= 50, "animation stalled: {animating} frames");
    let done = run.result()["extra"]["tween_done_tick"].as_u64();
    assert!(
        done.is_some_and(|tick| tick >= 60),
        "tween: {}",
        run.result()
    );
    // Spinner stopped, tween finished: the loop goes quiet.
    assert_eq!(run.frames_between(1700.0, 2400.0), 0, "{:?}", run.frames());

    let run = drive(
        "request",
        (80, 24),
        &[("SLT_IDLE_QUIT_MS", "1000")],
        json!([]),
        &[],
    );
    let frames = run.frames_between(0.0, 1000.0);
    assert!(frames >= 40, "request_frame cadence: {frames} frames");
}

#[test]
fn idle_skip_wakes_for_keys_and_resize_within_a_tick() {
    let run = drive(
        "keys",
        (80, 24),
        &[("SLT_IDLE_QUIT_MS", "10000")],
        json!([
            {"at": 1.0, "write": "a"},
            {"at": 1.6, "resize": [100, 30]},
            {"at": 2.2, "write": "q"}
        ]),
        &[],
    );
    let key_sent = run.action_time(0);
    let resized = run.action_time(1);
    let key_echo = run.first_output_after(key_sent).expect("key frame");
    let resize_echo = run.first_output_after(resized).expect("resize frame");
    let key_latency = key_echo - key_sent;
    let resize_latency = resize_echo - resized;
    eprintln!("key->bytes {key_latency:.4}s, resize->bytes {resize_latency:.4}s");
    assert!(key_latency < 0.1, "key latency {key_latency:.3}s");
    assert!(resize_latency < 0.1, "resize latency {resize_latency:.3}s");
    assert_eq!(
        run.result()["extra"]["keys"].as_array().unwrap().len(),
        1,
        "{}",
        run.result()
    );
    let sizes = run.result()["extra"]["sizes"].as_array().unwrap();
    assert_eq!(sizes.last().unwrap(), &json!([100, 30]), "{sizes:?}");
    // Quiet between the key's follow-up frames and the resize.
    assert_eq!(run.bytes_between(key_sent + 0.2, resized), 0);
}

#[test]
fn idle_skip_shows_tooltip_after_delay_without_input() {
    // Button at row 1; SGR motion report (button 35 = no button held).
    let run = drive(
        "tooltip",
        (60, 12),
        &[("SLT_IDLE_QUIT_MS", "1600")],
        json!([{"at": 0.5, "write": "\u{1b}[<35;3;1M"}]),
        &["TIPTEXT"],
    );
    let hovered = run.action_time(0);
    let shown = run.report["seen"]["TIPTEXT"]
        .as_f64()
        .unwrap_or_else(|| panic!("tooltip never shown: {}", run.report));
    let after = shown - hovered;
    eprintln!("tooltip shown {after:.3}s after hover (delay 0.300s)");
    assert!(after >= 0.28, "tooltip ignored the delay: {after:.3}s");
    assert!(after < 0.45, "tooltip late: {after:.3}s");
}

#[cfg(feature = "async")]
#[test]
fn idle_skip_delivers_async_messages_promptly() {
    let mut p95 = Vec::new();
    for skip in ["0", "1"] {
        let run = drive(
            "async",
            (80, 24),
            &[("SLT_IDLE_SKIP", skip), ("SLT_IDLE_QUIT_MS", "1800")],
            json!([]),
            &[],
        );
        let extra = &run.result()["extra"];
        assert_eq!(extra["count"], 20, "{extra}");
        let value = extra["p95_ms"].as_f64().unwrap();
        eprintln!(
            "idle_redraw={} async p95 {value:.2} ms {extra}",
            skip == "0"
        );
        p95.push(value);
        if skip == "1" {
            // Message frames only: no 60 Hz cadence before the first message.
            assert!(run.frames_between(100.0, 450.0) == 0, "{:?}", run.frames());
        }
    }
    assert!(p95[1] < 20.0, "skip-mode message p95 {:.2} ms", p95[1]);
}

/// Idle CPU, output bytes and frames over 30 s at 200x60 (docs/PERFORMANCE.md,
/// #414). Run with `cargo test --release --all-features --test pty_idle
/// idle_cpu_report -- --ignored --nocapture`; `SLT_IDLE_REPORT_SECS` changes
/// the window and `SLT_IDLE_REPORT_CASE=static|async` runs one case.
#[test]
#[ignore = "30 s per configuration; manual measurement"]
fn idle_cpu_report() {
    let secs = env_u64("SLT_IDLE_REPORT_SECS", 30);
    let quit = (secs * 1000).to_string();
    let configs = [
        ("static", "0", "16"),
        ("static", "1", "16"),
        ("static", "0", "1000"),
        ("static", "1", "1000"),
        #[cfg(feature = "async")]
        ("async", "0", "16"),
        #[cfg(feature = "async")]
        ("async", "1", "16"),
    ];
    let only = std::env::var("SLT_IDLE_REPORT_CASE").ok();
    for (case, skip, tick) in configs {
        if only.as_deref().is_some_and(|only| only != case) {
            continue;
        }
        let run = drive(
            case,
            (200, 60),
            &[
                ("SLT_IDLE_SKIP", skip),
                ("SLT_IDLE_TICK_MS", tick),
                ("SLT_IDLE_QUIT_MS", quit.as_str()),
                ("SLT_IDLE_MESSAGES", "0"),
            ],
            json!([]),
            &[],
        );
        let wall = run.report["wall"].as_f64().unwrap();
        let cpu = run.report["cpu"].as_f64().unwrap();
        let frames = run.frames().len();
        let start = run.report["chunks"][0][0].as_f64().unwrap();
        let idle_bytes = run.bytes_between(start + 1.0, start + secs as f64 - 0.5);
        let idle_secs = secs as f64 - 1.5;
        println!(
            "{case} idle_redraw={} tick={tick}ms wall={wall:.1}s cpu={cpu:.3}s ({:.2}% of a core) \
             frames={frames} ({:.1}/s) total_bytes={} idle_bytes/s={:.1}",
            skip == "0",
            cpu / wall * 100.0,
            frames as f64 / wall,
            run.report["bytes"],
            idle_bytes as f64 / idle_secs,
        );
    }
}
