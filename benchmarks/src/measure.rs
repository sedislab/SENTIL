//! Timing helpers

use std::hint::black_box;
use std::time::Instant;

use crate::schema::{Hardware, Timing};

pub fn time_runs<T>(runs: u64, mut op: impl FnMut() -> T) -> Timing {
    let mut samples = Vec::with_capacity(runs as usize);
    for _ in 0..runs {
        let start = Instant::now();
        black_box(op());
        samples.push(start.elapsed().as_secs_f64() * 1e3);
    }
    summarize(&mut samples)
}

pub fn summarize(samples: &mut [f64]) -> Timing {
    samples.sort_by(f64::total_cmp);
    let n = samples.len();
    #[allow(
        clippy::cast_precision_loss,
        reason = "run counts are small, far below 2^53"
    )]
    let count = n as f64;
    let mean = samples.iter().sum::<f64>() / count;
    let var = if n > 1 {
        samples.iter().map(|&s| (s - mean).powi(2)).sum::<f64>() / (count - 1.0)
    } else {
        0.0
    };
    Timing {
        mean_ms: mean,
        std_ms: var.sqrt(),
        min_ms: samples[0],
        p50_ms: percentile(samples, 0.50),
        p99_ms: percentile(samples, 0.99),
    }
}

fn percentile(sorted: &[f64], q: f64) -> f64 {
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the index comes from a small length and a quantile in [0, 1]"
    )]
    let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[idx]
}

#[must_use]
pub fn hardware() -> Hardware {
    Hardware {
        cpu: cpu_model(),
        cores: std::thread::available_parallelism().map_or(1, std::num::NonZero::get),
    }
}

fn cpu_model() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|info| {
            info.lines()
                .find(|line| line.starts_with("model name"))
                .and_then(|line| line.split(':').nth(1))
                .map(|name| name.trim().to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

#[must_use]
pub fn peak_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let kib: u64 = status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    Some(kib * 1024)
}

#[cfg(test)]
mod tests {
    use super::{percentile, summarize, time_runs};

    #[test]
    fn percentile_picks_the_nearest_rank() {
        let sorted = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&sorted, 0.0), 1.0);
        assert_eq!(percentile(&sorted, 0.5), 3.0);
        assert_eq!(percentile(&sorted, 1.0), 5.0);
    }

    #[test]
    fn summarize_reports_min_and_order_statistics() {
        let mut samples = [5.0, 1.0, 3.0, 2.0, 4.0];
        let timing = super::summarize(&mut samples);
        assert_eq!(timing.min_ms, 1.0);
        assert_eq!(timing.p50_ms, 3.0);
        assert!((timing.mean_ms - 3.0).abs() < 1e-12);
    }

    #[test]
    fn time_runs_collects_the_requested_count() {
        let mut calls = 0u64;
        let timing = time_runs(8, || {
            calls += 1;
            calls
        });
        assert_eq!(calls, 8);
        assert!(timing.mean_ms >= 0.0);
    }

    #[test]
    fn a_single_run_has_zero_spread() {
        let mut one = [2.5];
        assert_eq!(summarize(&mut one).std_ms, 0.0);
    }
}