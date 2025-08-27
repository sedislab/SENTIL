//! The SENTIL benchmark runner. It measures the engine on the shared oracle and
//! emits one JSON record per measurement to standard output, the same record
//! shape every other tool's runner emits.
//!
//! Two suites are available. `deterministic` times each canonical formula on a
//! fixed-size trace, on both the full-signal and the monitoring question.
//! `scalability` times one bounded formula across a range of trace lengths to
//! show how the cost grows, again on both questions.
//!
//! Run as `sentil_runner <suite>`, where `<suite>` is `deterministic` or
//! `scalability`. Heavy sizes belong on a quiet compute node.

use std::env;
use std::process::ExitCode;

use sentil::Formula;
use sentil_benchmarks::measure::{hardware, peak_rss_bytes, time_runs};
use sentil_benchmarks::oracle::{trace, CANONICAL};
use sentil_benchmarks::schema::{Question, Record};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const SWEEP: &str = "always[0, 100](eventually[0, 10](x > 5))";

fn main() -> ExitCode {
    let suite = env::args().nth(1).unwrap_or_default();
    let records = match suite.as_str() {
        "deterministic" => deterministic(),
        "scalability" => scalability(),
        other => {
            eprintln!("unknown suite `{other}`; use `deterministic` or `scalability`");
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

/// Times every canonical formula on a fixed trace, both questions.
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

/// Times one bounded formula as the trace grows, both questions, to show that
/// the monitoring cost stays flat while the full-signal cost tracks length.
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
    records
}

/// Measures the full-signal question: the robustness at every sample.
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
    record(
        benchmark,
        formula,
        Question::FullSignal,
        size,
        robustness,
        timing,
        runs,
    )
}

/// Measures the monitoring question: the robustness at the first sample.
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
    record(
        benchmark,
        formula,
        Question::Monitoring,
        size,
        robustness,
        timing,
        runs,
    )
}

fn record(
    benchmark: &str,
    formula: &str,
    question: Question,
    size: u64,
    robustness: f64,
    timing: sentil_benchmarks::schema::Timing,
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