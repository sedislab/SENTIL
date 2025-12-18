//! `sentil synth` performs open-loop synthesis.

use sentil::{Backend, Bounds, LinearModel, SynthesisProblem, Synthesizer};
use serde::Deserialize;
use serde_json::json;

use crate::cli::Method;
use crate::engine;
use crate::error::{code, CliError, Run};
use crate::output::{self, Out};

/// The model file: a discrete linear system x[k+1] = A x[k] + B u[k].
#[derive(Deserialize)]
struct ModelSpec {
    a: Vec<Vec<f64>>,
    b: Vec<Vec<f64>>,
    x0: Vec<f64>,
    variables: Vec<String>,
    dt: f64,
    horizon: usize,
    #[serde(default)]
    bounds: Option<BoundsSpec>,
}

#[derive(Deserialize)]
struct BoundsSpec {
    lower: Vec<f64>,
    upper: Vec<f64>,
}

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
    let model_spec = load_model(model_path)?;

    let horizon = horizon.unwrap_or(model_spec.horizon);
    // The control vector spans every step, so a single [lo, hi] broadcasts to all
    // of them; full-length bounds are taken as written.
    let input_dim = horizon * model_spec.b.first().map_or(0, Vec::len);
    let bounds = model_spec
        .bounds
        .as_ref()
        .map(|b| build_bounds(b, input_dim))
        .transpose()?;
    let model = LinearModel::new(
        model_spec.a,
        model_spec.b,
        model_spec.x0,
        model_spec.variables,
        model_spec.dt,
        horizon,
    )
    .map_err(|e| CliError::Input(format!("model: {e}"), None))?;

    let mut problem = SynthesisProblem::new(&model, &spec_formula)
        .with_backend(backend_of(method))
        .with_budget(budget);
    if let Some(bounds) = bounds {
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

/// Builds the input bounds, broadcasting a single `[lo, hi]` across every step.
fn build_bounds(spec: &BoundsSpec, input_dim: usize) -> Result<Bounds, CliError> {
    let expand = |values: &[f64]| -> Result<Vec<f64>, CliError> {
        match values {
            [single] if input_dim > 0 => Ok(vec![*single; input_dim]),
            other => Ok(other.to_vec()),
        }
    };
    Bounds::new(expand(&spec.lower)?, expand(&spec.upper)?)
        .map_err(|e| CliError::Input(format!("bounds: {e}"), None))
}

fn backend_of(method: Method) -> Backend {
    match method {
        Method::Gradient => Backend::Gradient,
        Method::CmaEs => Backend::CmaEs,
        Method::Milp => Backend::Milp,
    }
}

fn load_model(path: &str) -> Result<ModelSpec, CliError> {
    if !std::path::Path::new(path).exists() {
        return Err(CliError::NotFound {
            path: path.to_string(),
        });
    }
    let text =
        std::fs::read_to_string(path).map_err(|e| CliError::Input(format!("{path}: {e}"), None))?;
    serde_json::from_str(&text).map_err(|e| {
        CliError::Input(
            format!("model {path}: {e}"),
            Some("the model needs a, b, x0, variables, dt, and horizon".into()),
        )
    })
}