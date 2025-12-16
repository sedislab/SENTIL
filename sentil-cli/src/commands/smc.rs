//! `sentil smc`: statistical model checking of a probabilistic specification over
//! a base trace lifted into a noisy ensemble.

use std::time::Instant;

use sentil::stats::chernoff_hoeffding_samples;
use sentil::{
    IntervalMethod, LiftingRegistry, Monitor, MonitorConfig, SmcConfig, SprtConfig, SprtResult,
};
use serde_json::json;

use crate::cli::{Algo, Interval};
use crate::engine;
use crate::error::{code, CliError, Run};
use crate::output::{self, Out};

#[allow(clippy::too_many_arguments)]
pub fn run(
    algo: Algo,
    samples: &str,
    confidence: f64,
    interval: Interval,
    epsilon: f64,
    seed: u64,
    formula: Option<&str>,
    spec: Option<&str>,
    variant: Option<&str>,
    params: &[String],
    trace_path: &str,
    out: &Out,
) -> Run {
    let budget = parse_count(samples)?;
    let (formula, builder) = engine::resolve_formula(formula, spec, variant, params, true)?;
    let parsed = engine::parse_or_diagnose(&formula)?;
    let trace = engine::load_trace(trace_path)?;
    let lifting = lifting_for(builder.as_ref())?;

    let spinner = out.spinner("simulating");
    let start = Instant::now();
    let report = match algo {
        Algo::Smc => smc(&parsed, &trace, &lifting, budget, confidence, interval, seed)?,
        Algo::Chernoff => {
            let sized = chernoff_hoeffding_samples(epsilon, 1.0 - confidence).map_err(|e| {
                CliError::Input(format!("chernoff sample sizing: {e}"), None)
            })?;
            smc(&parsed, &trace, &lifting, sized, confidence, interval, seed)?
        }
        Algo::Sprt => sprt(&parsed, &trace, &lifting, budget, seed)?,
        Algo::Ams => {
            return Err(CliError::Input(
                "the ams algorithm needs a stochastic system, which the trace-based CLI cannot \
                 supply"
                    .into(),
                Some("use smc or sprt here, or call the library for adaptive multilevel splitting".into()),
            ))
        }
    };
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    output::clear_spinner(spinner);

    if out.is_text() {
        println!("{}", out.paint("smc", output::heading()));
        println!("  formula      {formula}");
        println!("  algorithm    {algo}");
        println!("  samples      {}", report.samples);
        println!("  satisfied    {}", report.satisfactions);
        println!("  probability  {:.6}", report.probability);
        if let Some((low, high)) = report.interval {
            println!("  {:.0}% interval [{low:.6}, {high:.6}]", confidence * 100.0);
        }
        println!(
            "{}",
            out.paint(&format!("  ran in {elapsed_ms:.1} ms"), output::dim())
        );
    } else {
        let mut object = json!({
            "schema_version": "1.0",
            "verb": "smc",
            "algorithm": algo.to_string(),
            "samples": report.samples,
            "satisfactions": report.satisfactions,
            "probability": report.probability,
            "elapsed_ms": elapsed_ms,
        });
        if let Some((low, high)) = report.interval {
            object["interval"] = json!({
                "method": interval.to_string(),
                "confidence": confidence,
                "low": low,
                "high": high,
            });
        }
        println!("{object}");
    }
    Ok(code::SUCCESS)
}

/// The estimate to report, shared across the algorithms.
struct Report {
    probability: f64,
    satisfactions: u64,
    samples: u64,
    interval: Option<(f64, f64)>,
}

fn smc(
    formula: &sentil::Formula,
    trace: &sentil::Trace,
    lifting: &LiftingRegistry,
    samples: u64,
    confidence: f64,
    interval: Interval,
    seed: u64,
) -> Result<Report, CliError> {
    let config = MonitorConfig::new().smc(SmcConfig {
        samples,
        confidence,
        seed,
        interval_method: match interval {
            Interval::Wilson => IntervalMethod::Wilson,
            Interval::ClopperPearson => IntervalMethod::ClopperPearson,
        },
    });
    let monitor = Monitor::from_formula(formula.clone(), config);
    let result = monitor
        .check(trace, lifting)
        .map_err(|e| CliError::Engine(e.to_string()))?;
    Ok(Report {
        probability: result.probability,
        satisfactions: result.satisfactions,
        samples: result.samples,
        interval: Some((result.interval.lower, result.interval.upper)),
    })
}

fn sprt(
    formula: &sentil::Formula,
    trace: &sentil::Trace,
    lifting: &LiftingRegistry,
    max_samples: u64,
    seed: u64,
) -> Result<Report, CliError> {
    let config = SprtConfig::new(0.90, 0.95, 0.01, 0.01, max_samples)
        .map_err(|e| CliError::Input(format!("sprt configuration: {e}"), None))?
        .with_seed(seed);
    let monitor = Monitor::from_formula(formula.clone(), MonitorConfig::new());
    let result = monitor
        .check_sequential(trace, lifting, &config)
        .map_err(|e| CliError::Engine(e.to_string()))?;
    let (probability, satisfactions, samples) = match result {
        SprtResult::AcceptH1 { samples } => (1.0, samples, samples),
        SprtResult::AcceptH0 { samples } => (0.0, 0, samples),
        SprtResult::Inconclusive { samples, .. } => (0.5, samples / 2, samples),
    };
    Ok(Report {
        probability,
        satisfactions,
        samples,
        interval: None,
    })
}

/// The noise models for lifting come from the spec; a raw formula runs with no
/// lifting. SENTIL carries noise models in the specification format rather than a
/// standalone file, so reach for `--spec` when an ensemble is wanted.
fn lifting_for(builder: Option<&sentil::SpecBuilder>) -> Result<LiftingRegistry, CliError> {
    match builder {
        Some(builder) => builder
            .build_lifting_registry()
            .map_err(|e| CliError::Input(format!("the spec's noise models: {e}"), None)),
        None => Ok(LiftingRegistry::new()),
    }
}

fn parse_count(text: &str) -> Result<u64, CliError> {
    let parsed: f64 = text
        .parse()
        .map_err(|_| CliError::Input(format!("--samples '{text}' is not a number"), None))?;
    if !parsed.is_finite() || parsed < 1.0 {
        return Err(CliError::Input(
            format!("--samples '{text}' must be at least 1"),
            None,
        ));
    }
    Ok(parsed as u64)
}