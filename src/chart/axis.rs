#[allow(unused_imports)]
use super::*;

#[derive(Debug, Clone)]
pub(crate) struct TickSpec {
    pub(crate) values: Vec<f64>,
    pub(crate) step: f64,
}

pub(super) fn build_ticks(min: f64, max: f64, target: usize) -> TickSpec {
    let span = (max - min).abs().max(f64::EPSILON);
    let range = nice_number(span, false);
    let raw_step = range / (target.max(2) as f64 - 1.0);
    let step = nice_number(raw_step, true).max(f64::EPSILON);
    let nice_min = (min / step).floor() * step;
    let nice_max = (max / step).ceil() * step;

    let mut values = Vec::new();
    let mut value = nice_min;
    let limit = nice_max + step * 0.5;
    let mut guard = 0usize;
    while value <= limit && guard < 128 {
        values.push(value);
        value += step;
        guard = guard.saturating_add(1);
    }

    if values.is_empty() {
        values.push(min);
        values.push(max);
    }

    TickSpec { values, step }
}

/// TUI-aware tick generation: picks a nice step whose interval count
/// divides `cell_count - 1` as evenly as possible, with 3-8 intervals
/// and at least 2 rows per interval for readable spacing.
pub(super) fn build_tui_ticks(data_min: f64, data_max: f64, cell_count: usize) -> TickSpec {
    let last = cell_count.saturating_sub(1).max(1);
    let span = (data_max - data_min).abs().max(f64::EPSILON);
    let log = span.log10().floor();

    let mut candidates: Vec<(f64, f64, usize, usize)> = Vec::new();

    for exp_off in -1..=1i32 {
        let base = 10.0_f64.powf(log + f64::from(exp_off));
        for &mult in &[1.0, 2.0, 2.5, 5.0] {
            let step = base * mult;
            if step <= 0.0 || !step.is_finite() {
                continue;
            }
            let lo = (data_min / step).floor() * step;
            let hi = (data_max / step).ceil() * step;
            let n = ((hi - lo) / step + 0.5) as usize;
            if (3..=8).contains(&n) && last / n >= 2 {
                let rem = last % n;
                candidates.push((step, lo, n, rem));
            }
        }
    }

    candidates.sort_by(|a, b| {
        a.3.cmp(&b.3).then_with(|| {
            let da = (a.2 as i32 - 5).unsigned_abs();
            let db = (b.2 as i32 - 5).unsigned_abs();
            da.cmp(&db)
        })
    });

    if let Some(&(step, lo, n, _)) = candidates.first() {
        let values: Vec<f64> = (0..=n).map(|i| lo + step * i as f64).collect();
        return TickSpec { values, step };
    }

    build_ticks(data_min, data_max, 5)
}

pub(super) fn nice_number(value: f64, round: bool) -> f64 {
    if value <= 0.0 || !value.is_finite() {
        return 1.0;
    }
    let exponent = value.log10().floor();
    let power = 10.0_f64.powf(exponent);
    let fraction = value / power;

    let nice_fraction = if round {
        if fraction < 1.5 {
            1.0
        } else if fraction < 3.0 {
            2.0
        } else if fraction < 7.0 {
            5.0
        } else {
            10.0
        }
    } else if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };

    nice_fraction * power
}

pub(super) fn format_number(value: f64, step: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let abs_step = step.abs().max(f64::EPSILON);
    let precision = if abs_step >= 1.0 {
        0
    } else {
        (-abs_step.log10().floor() as i32 + 1).clamp(0, 6) as usize
    };
    format!("{value:.precision$}")
}

pub(super) fn resolve_bounds<I>(values: I, manual: Option<(f64, f64)>) -> (f64, f64)
where
    I: Iterator<Item = f64>,
{
    if let Some((min, max)) = manual {
        return normalize_bounds(min, max);
    }

    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for value in values {
        if !value.is_finite() {
            continue;
        }
        min = min.min(value);
        max = max.max(value);
    }

    if !min.is_finite() || !max.is_finite() {
        return (0.0, 1.0);
    }

    normalize_bounds(min, max)
}

pub(super) fn normalize_bounds(min: f64, max: f64) -> (f64, f64) {
    if (max - min).abs() < f64::EPSILON {
        let pad = if min.abs() < 1.0 {
            1.0
        } else {
            min.abs() * 0.1
        };
        (min - pad, max + pad)
    } else if min < max {
        (min, max)
    } else {
        (max, min)
    }
}
