//! `sentil check` does offline robustness

use std::time::Instant;

use sentil::{Monitor, MonitorConfig, TimeMode};
use serde_json::json;

use crate::cli::{Backend, Semantics};
use crate::engine;
use crate::error::{code, CliError, Run};
use crate::output::{self, Out};

#[allow(clippy::too_many_arguments)]
pub fn run(
    formula: Option<&str>,
    spec: Option<&str>,
    variant: Option<&str>,
    params: &[String],
    trace_path: &str,
    map: &[String],
    semantics: Semantics,
    signal: bool,
    violations: bool,
    backend: Backend,
    out: &Out,
) -> Run {
    if matches!(backend, Backend::Gpu) {
        return Err(CliError::Backend(
            "the gpu backend does not run deterministic checking".into(),
            Some("drop --backend gpu; deterministic robustness runs on the CPU".into()),
        ));
    }

    let (formula, _builder) = engine::resolve_formula(formula, spec, variant, params, false)?;
    let parsed = engine::parse_or_diagnose(&formula)?;
    let spinner = out.spinner("evaluating");
    let trace = engine::load_trace(trace_path, map)?;
    engine::check_variables(&parsed, &trace)?;

    let mode = match semantics {
        Semantics::Dense => TimeMode::Dense,
        Semantics::Discrete => TimeMode::Discrete,
    };
    let monitor = Monitor::from_formula(parsed, MonitorConfig::new().time(mode));

    let start = Instant::now();
    if violations {
        let intervals = monitor.violations(&trace).map_err(eval_error)?;
        output::clear_spinner(spinner);
        return emit_violations(&formula, trace_path, &intervals, out);
    }
    if signal {
        let values = monitor.robustness_signal(&trace).map_err(eval_error)?;
        output::clear_spinner(spinner);
        return emit_signal(&formula, trace_path, semantics, &values, out);
    }
    let robustness = monitor.robustness(&trace).map_err(eval_error)?;
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    output::clear_spinner(spinner);

    let satisfied = robustness >= 0.0;
    if out.is_text() {
        let verdict = if satisfied {
            out.paint("satisfied", output::good())
        } else {
            out.paint("violated", output::bad())
        };
        out.heading("check");
        out.field("formula", &formula);
        out.field("trace", trace_path);
        out.field("semantics", &semantics.to_string());
        out.field("verdict", &verdict);
        out.field("robustness", &format!("{robustness:.6}"));
        out.note(&format!("evaluated in {elapsed_ms:.1} ms"));
    } else {
        println!(
            "{}",
            json!({
                "schema_version": "1.0",
                "verb": "check",
                "formula": formula,
                "trace": trace_path,
                "semantics": semantics.to_string(),
                "verdict": if satisfied { "satisfied" } else { "violated" },
                "robustness": robustness,
                "backend": backend.to_string(),
                "elapsed_ms": elapsed_ms,
            })
        );
    }

    Ok(if satisfied {
        code::SUCCESS
    } else {
        code::VIOLATED
    })
}

fn eval_error(e: sentil::Error) -> CliError {
    let message = e.to_string();
    if message.contains("next in dense time") {
        return CliError::Input(
            "the `next` operator is not defined in dense time".into(),
            Some("add --semantics discrete; next advances one sample step, which dense time has no fixed step for".into()),
        );
    }
    CliError::Engine(message)
}

fn emit_violations(formula: &str, trace_path: &str, intervals: &[(f64, f64)], out: &Out) -> Run {
    if out.is_text() {
        out.heading("violations");
        out.field("formula", formula);
        out.field("trace", trace_path);
        if intervals.is_empty() {
            out.note("none");
        } else {
            for (start, end) in intervals {
                println!("  [{start:.3}, {end:.3}]");
            }
        }
    } else {
        let spans: Vec<[f64; 2]> = intervals.iter().map(|(s, e)| [*s, *e]).collect();
        println!(
            "{}",
            json!({
                "schema_version": "1.0",
                "verb": "check",
                "formula": formula,
                "trace": trace_path,
                "violations": spans,
            })
        );
    }
    Ok(if intervals.is_empty() {
        code::SUCCESS
    } else {
        code::VIOLATED
    })
}

/// The per-sample robustness. Text prints one value per line in a round-trip form to preserve the infinities that a bounded operator can produce
fn emit_signal(
    formula: &str,
    trace_path: &str,
    semantics: Semantics,
    values: &[f64],
    out: &Out,
) -> Run {
    if out.is_text() {
        for value in values {
            println!("{value}");
        }
    } else {
        println!(
            "{}",
            json!({
                "schema_version": "1.0",
                "verb": "check",
                "formula": formula,
                "trace": trace_path,
                "semantics": semantics.to_string(),
                "signal": values,
            })
        );
    }
    let satisfied = values.first().is_some_and(|&v| v >= 0.0);
    Ok(if satisfied {
        code::SUCCESS
    } else {
        code::VIOLATED
    })
}