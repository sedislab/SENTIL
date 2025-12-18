//! `sentil falsify` searches a model's input space for a trajectory that violates a spec

use sentil::{CmaConfig, SmoothConfig};
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
    restarts: usize,
    out: &Out,
) -> Run {
    let (formula_text, _builder) = engine::resolve_formula(formula, spec, variant, params, false)?;
    let parsed = engine::parse_or_diagnose(&formula_text)?;
    let loaded = model::load(model_path, horizon)?;
    let bounds = loaded.bounds.ok_or_else(|| {
        CliError::Input(
            "falsify needs input bounds to search within".into(),
            Some("add a bounds {\"lower\": [...], \"upper\": [...]} block to the model file".into()),
        )
    })?;

    let spinner = out.spinner("searching for a counterexample");
    let witness = match method {
        Method::Gradient => {
            parsed.find_counterexample(&loaded.model, &bounds, budget, SmoothConfig::default())
        }
        Method::CmaEs => parsed.falsify(&loaded.model, &bounds, CmaConfig::default(), restarts),
        Method::Milp => {
            return Err(CliError::Input(
                "falsify searches with gradient or cmaes, not milp".into(),
                None,
            ))
        }
    }
    .map_err(|e| CliError::Engine(e.to_string()))?;
    output::clear_spinner(spinner);

    let found = witness.robustness < 0.0;
    if out.is_text() {
        let verdict = if found {
            out.paint("counterexample found", output::bad())
        } else {
            out.paint("no counterexample in budget", output::good())
        };
        let input = witness
            .input
            .iter()
            .map(|u| format!("{u:.4}"))
            .collect::<Vec<_>>()
            .join(", ");
        out.heading("falsify");
        out.field("spec", &formula_text);
        out.field("method", &method.to_string());
        out.field("result", &verdict);
        out.field("robustness", &format!("{:.6}", witness.robustness));
        out.field("input", &format!("[{input}]"));
    } else {
        println!(
            "{}",
            json!({
                "schema_version": "1.0",
                "verb": "falsify",
                "spec": formula_text,
                "method": method.to_string(),
                "found": found,
                "robustness": witness.robustness,
                "input": witness.input,
            })
        );
    }

    // A counterexample is a violation, so it exits like check
    Ok(if found {
        code::VIOLATED
    } else {
        code::SUCCESS
    })
}