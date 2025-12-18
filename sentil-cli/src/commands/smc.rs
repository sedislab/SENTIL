//! `sentil smc` performs the statistical model checking of a probabilistic specification over a base trace lifted into a noisy ensemble.
//! Monte Carlo and Chernoff are used to estimate the satisfaction probability and SPRT and Bayes can be used for the hypothesis testing.

use std::time::Instant;

use sentil::formula::ProbabilityOp;
use sentil::stats::chernoff_hoeffding_samples;
use sentil::{
    BayesConfig, BayesResult, Formula, IntervalMethod, LiftingRegistry, Monitor, MonitorConfig,
    SmcConfig, SprtConfig, SprtResult, Trace,
};
use serde_json::json;

use crate::cli::{Algo, Interval};
use crate::engine;
use crate::error::{code, CliError, Run};
use crate::output::{self, Out};

const BAYES_FACTOR: f64 = 100.0; // a bayes factor of 100 is solid on the Jeffreys scale

#[allow(clippy::too_many_arguments)]
pub fn run(
    algo: Algo,
    samples: &str,
    confidence: f64,
    interval: Interval,
    epsilon: f64,
    indifference: f64,
    seed: u64,
    formula: Option<&str>,
    spec: Option<&str>,
    variant: Option<&str>,
    params: &[String],
    noise: &[String],
    trace_path: &str,
    map: &[String],
    out: &Out,
) -> Run {
    if !(confidence > 0.0 && confidence < 1.0) {
        return Err(CliError::Input(
            format!("--confidence {confidence} must be between 0 and 1"),
            Some("for example --confidence 0.95".into()),
        ));
    }
    let budget = parse_count(samples)?;
    let (formula_text, builder) = engine::resolve_formula(formula, spec, variant, params, true)?;
    let parsed = engine::parse_or_diagnose(&formula_text)?;
    let (op, threshold) = match &parsed {
        Formula::Probabilistic(op, threshold, _) => (*op, *threshold),
        _ => {
            return Err(CliError::Input(
                "smc needs a probabilistic formula".into(),
                Some("wrap it in a probability operator, for example P>=0.9(always (x > 0))".into()),
            ))
        }
    };
    let trace = engine::load_trace(trace_path, map)?;
    let lifting = resolve_lifting(noise, builder.as_ref())?;

    let spinner = out.spinner("simulating");
    let start = Instant::now();
    let report = match algo {
        Algo::Smc => smc(&parsed, &trace, &lifting, budget, confidence, interval, seed)?,
        Algo::Chernoff => {
            let sized = chernoff_hoeffding_samples(epsilon, 1.0 - confidence)
                .map_err(|e| CliError::Input(format!("chernoff sample sizing: {e}"), None))?;
            smc(&parsed, &trace, &lifting, sized, confidence, interval, seed)?
        }
        Algo::Sprt => sprt(
            &parsed,
            &trace,
            &lifting,
            op,
            threshold,
            indifference,
            confidence,
            budget,
            seed,
        )?,
        Algo::Bayes => bayes(&parsed, &trace, &lifting, op, threshold, budget, seed)?,
    };
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    output::clear_spinner(spinner);

    if out.is_text() {
        out.heading("smc");
        out.field("formula", &formula_text);
        out.field("algorithm", &algo.to_string());
        out.field("samples", &report.samples.to_string());
        if let Some(satisfactions) = report.satisfactions {
            out.field("satisfied", &satisfactions.to_string());
        }
        if let Some(probability) = report.probability {
            out.field("probability", &format!("{probability:.6}"));
        }
        if let Some((low, high)) = report.interval {
            out.field(
                "interval",
                &format!("[{low:.6}, {high:.6}] at {:.0}%", confidence * 100.0),
            );
        }
        if let Some(decision) = report.decision {
            out.field("decision", decision);
        }
        let verdict = if report.holds {
            out.paint("holds", output::good())
        } else {
            out.paint("does not hold", output::bad())
        };
        out.field("verdict", &verdict);
        out.note(&format!("ran in {elapsed_ms:.1} ms"));
    } else {
        let mut object = json!({
            "schema_version": "1.0",
            "verb": "smc",
            "algorithm": algo.to_string(),
            "samples": report.samples,
            "holds": report.holds,
            "elapsed_ms": elapsed_ms,
        });
        if let Some(probability) = report.probability {
            object["probability"] = json!(probability);
        }
        if let Some(satisfactions) = report.satisfactions {
            object["satisfactions"] = json!(satisfactions);
        }
        if let Some((low, high)) = report.interval {
            object["interval"] = json!({
                "method": interval.to_string(),
                "confidence": confidence,
                "low": low,
                "high": high,
            });
        }
        if let Some(decision) = report.decision {
            object["decision"] = json!(decision);
        }
        println!("{object}");
    }
    Ok(if report.holds {
        code::SUCCESS
    } else {
        code::VIOLATED
    })
}

struct Report {
    probability: Option<f64>,
    satisfactions: Option<u64>,
    samples: u64,
    interval: Option<(f64, f64)>,
    decision: Option<&'static str>,
    holds: bool,
}

fn smc(
    formula: &Formula,
    trace: &Trace,
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
        probability: Some(result.probability),
        satisfactions: Some(result.satisfactions),
        samples: result.samples,
        interval: Some((result.interval.lower, result.interval.upper)),
        decision: None,
        holds: result.holds,
    })
}

#[allow(clippy::too_many_arguments)]
fn sprt(
    formula: &Formula,
    trace: &Trace,
    lifting: &LiftingRegistry,
    op: ProbabilityOp,
    threshold: f64,
    indifference: f64,
    confidence: f64,
    max_samples: u64,
    seed: u64,
) -> Result<Report, CliError> {
    let margin = 1e-6;
    let p0 = (threshold - indifference).clamp(margin, 1.0 - margin);
    let p1 = (threshold + indifference).clamp(margin, 1.0 - margin);
    if p0 >= p1 {
        return Err(CliError::Input(
            format!("the indifference band around p={threshold} collapsed at the [0,1] edge"),
            Some("lower --indifference, or pick a threshold away from 0 or 1".into()),
        ));
    }
    let error = (1.0 - confidence).clamp(margin, 0.5 - margin);
    let config = SprtConfig::new(p0, p1, error, error, max_samples)
        .map_err(|e| CliError::Input(format!("sprt configuration: {e}"), None))?
        .with_seed(seed);
    let monitor = Monitor::from_formula(formula.clone(), MonitorConfig::new());
    let result = monitor
        .check_sequential(trace, lifting, &config)
        .map_err(|e| CliError::Engine(e.to_string()))?;

    let lower_bound = matches!(op, ProbabilityOp::GreaterEqual | ProbabilityOp::Greater);
    let (decision, samples, accept_high) = match result {
        SprtResult::AcceptH1 { samples } => ("accept_h1", samples, Some(true)),
        SprtResult::AcceptH0 { samples } => ("accept_h0", samples, Some(false)),
        SprtResult::Inconclusive { samples, .. } => ("inconclusive", samples, None),
    };
    let holds = accept_high.is_some_and(|high| high == lower_bound);
    Ok(Report {
        probability: None,
        satisfactions: None,
        samples,
        interval: None,
        decision: Some(decision),
        holds,
    })
}

fn bayes(
    formula: &Formula,
    trace: &Trace,
    lifting: &LiftingRegistry,
    op: ProbabilityOp,
    threshold: f64,
    max_samples: u64,
    seed: u64,
) -> Result<Report, CliError> {
    let config = BayesConfig::new(threshold, BAYES_FACTOR, max_samples)
        .map_err(|e| CliError::Input(format!("bayes configuration: {e}"), None))?
        .with_seed(seed);
    let result = formula
        .check_bayesian(trace, lifting, &config)
        .map_err(|e| CliError::Engine(e.to_string()))?;
    let lower_bound = matches!(op, ProbabilityOp::GreaterEqual | ProbabilityOp::Greater);
    let (decision, samples, met) = match result {
        BayesResult::Holds { samples, .. } => ("holds", samples, Some(true)),
        BayesResult::Fails { samples, .. } => ("fails", samples, Some(false)),
        BayesResult::Inconclusive { samples, .. } => ("inconclusive", samples, None),
    };
    Ok(Report {
        probability: None,
        satisfactions: None,
        samples,
        interval: None,
        decision: Some(decision),
        holds: met.is_some_and(|high| high == lower_bound),
    })
}

fn resolve_lifting(
    noise: &[String],
    builder: Option<&sentil::SpecBuilder>,
) -> Result<LiftingRegistry, CliError> {
    if let Some(registry) = engine::parse_noise(noise)? {
        return Ok(registry);
    }
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