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
    semantics: Semantics,
    signal: bool,
    backend: Backend,
    out: &Out,
) -> Run {
    if matches!(backend, Backend::Gpu) {
        return Err(CliError::Backend(
            "the gpu backend does not run deterministic checking".into(),
            Some("drop --backend gpu here, or use `sentil smc --backend gpu` for simulation".into()),
        ));
    }

    let (formula, _builder) = engine::resolve_formula(formula, spec, variant, params, false)?;
    let parsed = engine::parse_or_diagnose(&formula)?;
    let spinner = out.spinner("evaluating");
    let trace = engine::load_trace(trace_path)?;

    let mode = match semantics {
        Semantics::Dense => TimeMode::Dense,
        Semantics::Discrete => TimeMode::Discrete,
    };
    let monitor = Monitor::from_formula(parsed, MonitorConfig::new().time(mode));

    let start = Instant::now();
    if signal {
        let values = monitor
            .robustness_signal(&trace)
            .map_err(|e| CliError::Engine(e.to_string()))?;
        if let Some(bar) = spinner {
            bar.finish_and_clear();
        }
        return emit_signal(&formula, trace_path, semantics, &values, out);
    }
    let robustness = monitor
        .robustness(&trace)
        .map_err(|e| CliError::Engine(e.to_string()))?;
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    output::clear_spinner(spinner);

    let satisfied = robustness >= 0.0;
    if out.is_text() {
        let verdict = if satisfied {
            out.paint("satisfied", output::good())
        } else {
            out.paint("violated", output::bad())
        };
        println!("{}", out.paint("check", output::heading()));
        println!("  formula     {formula}");
        println!("  trace       {trace_path}");
        println!("  semantics   {semantics}");
        println!("  verdict     {verdict}");
        println!("  robustness  {robustness:.6}");
        println!(
            "{}",
            out.paint(&format!("  evaluated in {elapsed_ms:.1} ms"), output::dim())
        );
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