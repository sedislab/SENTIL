//! `sentil synth` performs open-loop synthesis.

use sentil::{Backend, SynthesisProblem, Synthesizer};
use serde_json::json;

use crate::cli::Method;
use crate::error::{code, CliError, Run};
use crate::output::{self, Out};
use crate::{engine, model};

#[allow(clippy::too_many_arguments)]
pub fn run(
    method: Method,
    model_path: &str,
    formula: Option<&str>,
    spec: Option<&str>,
    variant: Option<&str>,
    params: &[String],
    horizon: Option<usize>,
    budget: usize,
    out: &Out,
) -> Run {
    let (formula_text, _builder) = engine::resolve_formula(formula, spec, variant, params, false)?;
    let spec_formula = engine::parse_or_diagnose(&formula_text)?;
    let loaded = model::load(model_path, horizon)?;

    let mut problem = SynthesisProblem::new(&loaded.model, &spec_formula)
        .with_backend(backend_of(method))
        .with_budget(budget);
    if let Some(bounds) = loaded.bounds {
        problem = problem.with_bounds(bounds);
    }

    let spinner = out.spinner("synthesizing");
    let result = Synthesizer::solve(&problem).map_err(|e| CliError::Engine(e.to_string()))?;
    output::clear_spinner(spinner);

    if out.is_text() {
        let feasible = if result.holds {
            out.paint("feasible", output::good())
        } else {
            out.paint("infeasible (minimally violating)", output::bad())
        };
        let input = result
            .input
            .iter()
            .map(|u| format!("{u:.4}"))
            .collect::<Vec<_>>()
            .join(", ");
        out.heading("synth");
        out.field("spec", &formula_text);
        out.field("method", &method.to_string());
        out.field("result", &feasible);
        out.field("robustness", &format!("{:.6}", result.robustness));
        out.field("input", &format!("[{input}]"));
    } else {
        println!(
            "{}",
            json!({
                "schema_version": "1.0",
                "verb": "synth",
                "spec": formula_text,
                "method": method.to_string(),
                "feasible": result.holds,
                "robustness": result.robustness,
                "input": result.input,
            })
        );
    }

    Ok(if result.holds {
        code::SUCCESS
    } else {
        code::VIOLATED
    })
}

fn backend_of(method: Method) -> Backend {
    match method {
        Method::Gradient => Backend::Gradient,
        Method::CmaEs => Backend::CmaEs,
        Method::Milp => Backend::Milp,
    }
}