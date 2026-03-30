use std::env;
use std::process::ExitCode;

use sentil_benchmarks::measure::hardware;
use sentil_benchmarks::probabilistic::PROBABILISTIC;
use sentil_benchmarks::schema::SmcRecord;
use sentil_benchmarks::smc::{estimate, SmcModel, THROUGHPUT};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const SEED: u64 = 7;

fn device() -> &'static str {
    #[cfg(feature = "gpu")]
    {
        if sentil::gpu::is_available() {
            "gpu"
        } else {
            "cpu"
        }
    }
    #[cfg(not(feature = "gpu"))]
    {
        "cpu"
    }
}

fn record(model: &SmcModel, samples: u64, runs: u64) -> SmcRecord {
    let out = estimate(model, samples, runs, SEED);
    SmcRecord {
        tool: "sentil".to_owned(),
        version: VERSION.to_owned(),
        language: "rust".to_owned(),
        device: device().to_owned(),
        model: model.id.to_owned(),
        formula: model.formula.to_owned(),
        samples,
        probability: out.result.probability,
        ci_lower: out.result.interval.lower,
        ci_upper: out.result.interval.upper,
        ground_truth: model.ground_truth,
        timing: out.timing,
        throughput_per_s: out.throughput_per_s,
        runs,
        hardware: hardware(),
    }
}

fn main() -> ExitCode {
    let suite = env::args().nth(1).unwrap_or_default();
    let samples_arg = env::args().nth(2).and_then(|s| s.parse::<u64>().ok());

    let runs_arg = env::args().nth(3).and_then(|s| s.parse::<u64>().ok());
    let records: Vec<SmcRecord> = match suite.as_str() {
        "accuracy" => {
            let samples = samples_arg.unwrap_or(100_000);
            let runs = runs_arg.unwrap_or(5);
            PROBABILISTIC
                .iter()
                .map(|case| {
                    let model = SmcModel {
                        id: case.id,
                        signals: case.signals,
                        noise: case.noise,
                        formula: case.formula,
                        ground_truth: Some(case.probability),
                    };
                    record(&model, samples, runs)
                })
                .collect()
        }
        "throughput" => {
            let samples = samples_arg.unwrap_or(1_000_000);
            let runs = runs_arg.unwrap_or(3);
            THROUGHPUT.iter().map(|m| record(m, samples, runs)).collect()
        }
        other => {
            eprintln!("usage: sentil_smc_runner <accuracy|throughput> [samples] [runs] (got `{other}`)");
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