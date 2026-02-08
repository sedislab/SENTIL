//! The SENTIL runner. Emits one JSON record per measurement to standard output.
//! Run as `sentil_runner <deterministic|scalability|streaming>`. Heavy sizes belong
//! on a quiet compute node.

use std::env;
use std::hint::black_box;
use std::process::ExitCode;
use std::time::Instant;

use sentil::{Formula, StreamMonitor};
use sentil_benchmarks::measure::{hardware, peak_rss_bytes, summarize, time_runs};
use sentil_benchmarks::oracle::{trace, CANONICAL};
use sentil_benchmarks::schema::{Question, Record, Timing};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const SWEEP: &str = "always[0, 100](eventually[0, 10](x > 5))";

fn main() -> ExitCode {
    let suite = env::args().nth(1).unwrap_or_default();
    let records = match suite.as_str() {
        "deterministic" => deterministic(),
        "scalability" => scalability(),
        "dense" => dense(),
        "streaming" => streaming(),
        other => {
            eprintln!(
                "unknown suite `{other}`; use `deterministic`, `scalability`, `dense`, or `streaming`"
            );
            return ExitCode::FAILURE;
        }
    };
    for record in records {
        match serde_json::to_string(&record) {
            Ok(line) => println!("{line}"),
            Err(err) => {
                eprintln!("failed to serialize a record: {err}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

fn deterministic() -> Vec<Record> {
    let size = 2001u64;
    let tr = trace(size as usize);
    let runs = 50u64;
    let mut records = Vec::new();
    for (formula, _) in CANONICAL {
        let phi = Formula::parse(formula).expect("a valid oracle formula");
        records.push(measure_full("deterministic", formula, &phi, &tr, size, runs));
        records.push(measure_monitoring(
            "deterministic",
            formula,
            &phi,
            &tr,
            size,
            runs,
        ));
    }
    records
}

fn scalability() -> Vec<Record> {
    let phi = Formula::parse(SWEEP).expect("a valid formula");
    let sizes = [1_000u64, 10_000, 100_000, 1_000_000, 10_000_000];
    let mut records = Vec::new();
    for &size in &sizes {
        let tr = trace(size as usize);
        let runs = if size <= 100_000 { 30 } else { 5 };
        records.push(measure_full("scalability/length", SWEEP, &phi, &tr, size, runs));
        records.push(measure_monitoring(
            "scalability/length",
            SWEEP,
            &phi,
            &tr,
            size,
            runs,
        ));
    }

    let samples = 200_000usize;
    for depth in [1u64, 2, 4, 8, 16] {
        let mut nested = String::from("x > 5");
        for _ in 0..depth {
            nested = format!("eventually[0, 5]({nested})");
        }
        records.push(measure_stream("scalability/depth", &nested, samples, depth));
    }

    for w in [10u64, 100, 1_000, 10_000, 100_000] {
        let bounded = format!("always[0, {w}](x > 5)");
        records.push(measure_stream("scalability/bound", &bounded, samples, w));
    }

    records
}

fn measure_stream(benchmark: &str, formula: &str, samples: usize, param: u64) -> Record {
    let mut monitor = StreamMonitor::new(formula).expect("a streamable formula");
    let idx = monitor.symbol_index("x").expect("x is referenced");
    let mut packed = [0.0f64];
    let mut latencies = Vec::with_capacity(samples);
    let mut last = 0.0;
    for i in 0..samples {
        packed[idx] = 15.0 * (i as f64 * 0.1).sin();
        let start = Instant::now();
        let verdict = black_box(
            monitor
                .update_packed(i as f64, &packed)
                .expect("a streaming verdict"),
        );
        latencies.push(start.elapsed().as_secs_f64() * 1e3);
        last = verdict.lower();
    }
    let timing = summarize(&mut latencies);
    record(benchmark, formula, Question::Monitoring, param, last, timing, samples as u64)
}

fn dense() -> Vec<Record> {
    let phi = Formula::parse(SWEEP).expect("a valid formula");
    let sizes = [1_000u64, 10_000, 100_000, 1_000_000];
    let mut records = Vec::new();
    for &size in &sizes {
        let tr = trace(size as usize);
        let runs = if size <= 100_000 { 20 } else { 5 };
        let robustness = phi.robustness_dense_signal(&tr).expect("a finite dense signal")[0];
        let timing = time_runs(runs, || {
            phi.robustness_dense_signal(&tr).expect("a finite dense signal")
        });
        records.push(record(
            "dense/length",
            SWEEP,
            Question::FullSignal,
            size,
            robustness,
            timing,
            runs,
        ));
    }
    records
}

fn streaming() -> Vec<Record> {
    let n = 1_000_000usize;
    vec![measure_stream("streaming", SWEEP, n, n as u64)]
}

fn measure_full(
    benchmark: &str,
    formula: &str,
    phi: &Formula,
    tr: &sentil::Trace,
    size: u64,
    runs: u64,
) -> Record {
    let robustness = phi.robustness_signal(tr).expect("a finite signal")[0];
    let timing = time_runs(runs, || phi.robustness_signal(tr).expect("a finite signal"));
    record(benchmark, formula, Question::FullSignal, size, robustness, timing, runs)
}

fn measure_monitoring(
    benchmark: &str,
    formula: &str,
    phi: &Formula,
    tr: &sentil::Trace,
    size: u64,
    runs: u64,
) -> Record {
    let robustness = phi.robustness(tr).expect("a finite robustness");
    let timing = time_runs(runs, || phi.robustness(tr).expect("a finite robustness"));
    record(benchmark, formula, Question::Monitoring, size, robustness, timing, runs)
}

fn record(
    benchmark: &str,
    formula: &str,
    question: Question,
    size: u64,
    robustness: f64,
    timing: Timing,
    runs: u64,
) -> Record {
    Record {
        tool: "sentil".to_owned(),
        version: VERSION.to_owned(),
        language: "rust".to_owned(),
        benchmark: benchmark.to_owned(),
        formula: formula.to_owned(),
        question,
        size,
        robustness,
        timing,
        peak_rss_bytes: peak_rss_bytes(),
        runs,
        hardware: hardware(),
    }
}