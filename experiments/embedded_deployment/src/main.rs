//! Per-cycle latency harness for the Raspberry Pi safety-monitoring deployment.

use std::time::Instant;

use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use sentil::{Formula, MultiFormulaMonitor};

const SIGNALS: [&str; 9] = [
    "speed",
    "gap",
    "ttc",
    "accel",
    "jerk",
    "lane",
    "fusion",
    "collision",
    "yaw",
];

struct Args {
    cycles: u64,
    rate: f64,
    seed: u64,
    warmup: u64,
    output: String,
    hardware: String,
}

fn main() {
    let args = parse_args();
    let deadline_ms = 1000.0 / args.rate;
    let dt = 1.0 / args.rate;

    let specs = workload();
    let mut monitor = MultiFormulaMonitor::new();
    for (id, text) in &specs {
        let formula = Formula::parse(text).unwrap_or_else(|e| panic!("{id}: {e}"));
        monitor
            .add_formula(id.clone(), &formula)
            .unwrap_or_else(|e| panic!("{id}: {e}"));
    }

    let mut gen = Generator::new(args.seed);
    for cycle in 0..args.warmup {
        let sample = gen.next_sample();
        let _ = monitor.update(cycle as f64 * dt, &borrow(&sample));
    }

    // A zeroed page stays on the kernel's shared zero page until it is written.
    let mut latencies = vec![f64::NAN; args.cycles as usize];
    let rss_start = rss_kb();
    let mut deadline_misses = 0u64;
    let mut violation_cycles = 0u64;
    for cycle in 0..args.cycles {
        let t = (args.warmup + cycle) as f64 * dt;
        let sample = gen.next_sample();
        let values = borrow(&sample);
        let start = Instant::now();
        let results = monitor
            .update(t, &values)
            .expect("the workload formulas evaluate over the synthetic signals");
        let elapsed = start.elapsed();
        let ms = elapsed.as_secs_f64() * 1e3;
        latencies[cycle as usize] = ms;
        if ms > deadline_ms {
            deadline_misses += 1;
        }
        if results.iter().any(|(_, r)| !r.is_satisfied()) {
            violation_cycles += 1;
        }
    }
    let rss_end = rss_kb();

    let stats = Stats::from(&mut latencies);
    let report = Report {
        tool: "sentil",
        language: "rust",
        benchmark: "embedded_deployment",
        specifications: specs.len(),
        rate_hz: args.rate,
        cycles: args.cycles,
        deadline_ms,
        deadline_misses,
        violation_cycles,
        latency_ms: stats,
        rss_steady_mb: rss_end.map(kb_to_mb),
        rss_growth_mb: match (rss_start, rss_end) {
            (Some(a), Some(b)) => Some(kb_to_mb(b.saturating_sub(a))),
            _ => None,
        },
        hardware: args.hardware,
        version: env!("CARGO_PKG_VERSION"),
        runs: 1,
    };

    let json = serde_json::to_string_pretty(&report).expect("the report serializes");
    if let Some(parent) = std::path::Path::new(&args.output).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&args.output, &json).unwrap_or_else(|e| panic!("{}: {e}", args.output));
    println!("{json}");
    eprintln!(
        "{} cycles, mean {:.3} ms, p99 {:.3} ms, {} deadline misses of {}",
        report.cycles,
        report.latency_ms.mean,
        report.latency_ms.p99,
        deadline_misses,
        report.cycles
    );
}

/// The 60 streaming safety specifications: bounds on speed, distance, acceleration,
/// jerk, lane offset, time to collision, and sensor fusion, and the nested response
/// requirements between them, with nesting depth running from 1 up to 10. These run
/// on the streaming monitor, the flat per-sample engine the latency claim rests on.
fn workload() -> Vec<(String, String)> {
    let mut specs: Vec<String> = Vec::new();

    for h in [5, 10, 20] {
        specs.push(format!("always[0,{h}] (speed < 35)"));
        specs.push(format!("always[0,{h}] (gap > 4)"));
        specs.push(format!("always[0,{h}] (ttc > 1.5)"));
        specs.push(format!("always[0,{h}] (accel < 3)"));
        specs.push(format!("always[0,{h}] (accel > -6)"));
        specs.push(format!("always[0,{h}] (jerk < 5)"));
        specs.push(format!("always[0,{h}] (lane < 0.5)"));
        specs.push(format!("always[0,{h}] (lane > -0.5)"));
        specs.push(format!("always[0,{h}] (yaw < 0.6)"));
        specs.push(format!("always[0,{h}] (fusion > 0.6)"));
    }

    specs.push("always[0,10] (speed > 30 implies eventually[0,3] (accel < 0))".into());
    specs.push("always[0,10] (gap < 5 implies eventually[0,2] (accel < -1))".into());
    specs.push("always[0,10] (ttc < 2 implies eventually[0,1] (accel < -2))".into());
    specs.push("always[0,15] (lane > 0.3 implies eventually[0,2] (yaw < 0))".into());
    specs.push("always[0,10] (fusion < 0.7 implies eventually[0,3] (fusion > 0.7))".into());
    specs.push("always[0,20] (eventually[0,5] (gap > 8))".into());
    specs.push("eventually[0,20] (always[0,5] (gap > 6))".into());
    specs.push("always[0,10] ((speed < 32) until[0,5] (gap > 10))".into());
    specs.push("always[0,10] (always[0,3] (speed < 36))".into());
    specs.push("always[0,10] ((accel > 2) implies eventually[0,2] (accel < 1))".into());

    specs
        .push("always[0,10] (speed > 28 implies eventually[0,3] (always[0,2] (accel < 0)))".into());
    specs.push("always[0,20] (gap < 4 implies eventually[0,2] (always[0,3] (gap > 5)))".into());
    specs.push(
        "always[0,10] ((speed < 30 and gap > 5) implies eventually[0,3] (always[0,2] (ttc > 2)))"
            .into(),
    );
    specs.push("always[0,15] (ttc < 1.5 implies eventually[0,2] (always[0,2] (ttc > 2.5)))".into());

    specs.push("always[0,10] (jerk > -5)".into());
    specs.push("always[0,5] (collision < 0.05)".into());
    specs.push("always[0,10] (fusion > 0.55)".into());
    specs.push("always[0,15] (speed > 31 implies eventually[0,2] (speed < 31))".into());
    specs.push("always[0,10] (gap < 6 implies eventually[0,3] (gap > 7))".into());
    specs.push("always[0,20] (ttc > 1 implies always[0,2] (ttc > 0.8))".into());

    specs.push("always[0,10] (collision < 0.08)".into());
    specs.push("always[0,15] (gap > 3.5)".into());
    specs.push("always[0,15] (ttc > 1.2)".into());
    specs.push("always[0,10] (speed < 30 or gap > 8)".into());
    specs.push("always[0,10] (accel < 2.5 and jerk < 4)".into());
    specs.push("always[0,20] (lane < 0.45)".into());
    specs.push("always[0,10] (eventually[0,2] (fusion > 0.75))".into());
    specs.push("always[0,15] (speed > 25 implies gap > 6)".into());
    specs.push("always[0,10] (yaw > -0.5 and yaw < 0.5)".into());
    specs.push("always[0,20] (eventually[0,4] (accel > -3))".into());

    assert_eq!(
        specs.len(),
        60,
        "the deployment workload is 60 specifications"
    );
    specs
        .into_iter()
        .enumerate()
        .map(|(i, text)| (format!("spec{i:02}"), text))
        .collect()
}

/// A seeded generator of driving signals.
struct Generator {
    rng: ChaCha8Rng,
    state: [f64; SIGNALS.len()],
}

impl Generator {
    fn new(seed: u64) -> Self {
        // Band centers, in SIGNALS order.
        let state = [23.0, 12.0, 4.5, -0.2, 0.0, 0.0, 0.85, 0.02, 0.0];
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            state,
        }
    }

    fn next_sample(&mut self) -> Vec<f64> {
        // (step, low, high) per signal, in SIGNALS order.
        const BANDS: [(f64, f64, f64); SIGNALS.len()] = [
            (0.3, 18.0, 27.0),
            (0.3, 8.0, 18.0),
            (0.1, 2.6, 7.0),
            (0.15, -2.4, 1.8),
            (0.25, -3.0, 3.0),
            (0.015, -0.35, 0.35),
            (0.008, 0.76, 0.97),
            (0.003, 0.0, 0.04),
            (0.015, -0.35, 0.35),
        ];
        for (value, &(step, lo, hi)) in self.state.iter_mut().zip(&BANDS) {
            *value += self.rng.random_range(-step..=step);
            *value = value.clamp(lo, hi);
        }
        self.state.to_vec()
    }
}

fn borrow(sample: &[f64]) -> Vec<(&'static str, f64)> {
    SIGNALS
        .iter()
        .copied()
        .zip(sample.iter().copied())
        .collect()
}

#[derive(serde::Serialize)]
struct Stats {
    mean: f64,
    median: f64,
    p95: f64,
    p99: f64,
    max: f64,
}

impl Stats {
    fn from(samples: &mut [f64]) -> Self {
        if samples.is_empty() {
            return Self {
                mean: f64::NAN,
                median: f64::NAN,
                p95: f64::NAN,
                p99: f64::NAN,
                max: f64::NAN,
            };
        }
        samples.sort_by(f64::total_cmp);
        let n = samples.len();
        let mean = samples.iter().sum::<f64>() / n as f64;
        let at = |q: f64| samples[((n as f64 * q) as usize).min(n - 1)];
        Self {
            mean,
            median: at(0.50),
            p95: at(0.95),
            p99: at(0.99),
            max: samples[n - 1],
        }
    }
}

#[derive(serde::Serialize)]
struct Report {
    tool: &'static str,
    language: &'static str,
    benchmark: &'static str,
    specifications: usize,
    rate_hz: f64,
    cycles: u64,
    deadline_ms: f64,
    deadline_misses: u64,
    violation_cycles: u64,
    latency_ms: Stats,
    rss_steady_mb: Option<f64>,
    rss_growth_mb: Option<f64>,
    hardware: String,
    version: &'static str,
    runs: u32,
}

fn kb_to_mb(kb: u64) -> f64 {
    kb as f64 / 1024.0
}

/// Resident set size in kilobytes.
fn rss_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|kb| kb.parse().ok())
}

fn parse_args() -> Args {
    let mut cycles: Option<u64> = None;
    let mut duration_min = 120.0;
    let mut rate = 85.0;
    let mut seed = 7;
    let mut warmup = 2000;
    let mut output = "results/embedded.json".to_string();
    let mut hardware = detect_hardware();

    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || {
            args.next()
                .unwrap_or_else(|| panic!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--cycles" => cycles = Some(value().parse().expect("--cycles is a whole number")),
            "--duration" => duration_min = value().parse().expect("--duration is in minutes"),
            "--rate" => rate = value().parse().expect("--rate is in hertz"),
            "--seed" => seed = value().parse().expect("--seed is a whole number"),
            "--warmup" => warmup = value().parse().expect("--warmup is a whole number"),
            "--output" => output = value(),
            "--hardware" => hardware = value(),
            other => panic!("unknown flag {other}"),
        }
    }
    let cycles = cycles.unwrap_or((duration_min * 60.0 * rate) as u64);
    Args {
        cycles,
        rate,
        seed,
        warmup,
        output,
        hardware,
    }
}

/// The CPU model.
fn detect_hardware() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|info| {
            info.lines()
                .find_map(|l| {
                    l.strip_prefix("Model")
                        .or_else(|| l.strip_prefix("model name"))
                })
                .and_then(|l| l.split_once(':'))
                .map(|(_, v)| v.trim().to_string())
        })
        .unwrap_or_else(|| std::env::consts::ARCH.to_string())
}