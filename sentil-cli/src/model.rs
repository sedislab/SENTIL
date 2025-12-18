//! Loading a system model from a JSON file, shared by `synth` and `falsify`.

use sentil::{Bounds, LinearModel};
use serde::Deserialize;

use crate::error::CliError;

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

pub struct Loaded {
    pub model: LinearModel,
    pub bounds: Option<Bounds>,
}

pub fn load(path: &str, horizon_override: Option<usize>) -> Result<Loaded, CliError> {
    let spec = read(path)?;
    let horizon = horizon_override.unwrap_or(spec.horizon);
    let input_dim = horizon * spec.b.first().map_or(0, Vec::len);
    let bounds = spec
        .bounds
        .as_ref()
        .map(|b| build_bounds(b, input_dim))
        .transpose()?;
    let model = LinearModel::new(spec.a, spec.b, spec.x0, spec.variables, spec.dt, horizon)
        .map_err(|e| CliError::Input(format!("model: {e}"), None))?;
    Ok(Loaded { model, bounds })
}

fn read(path: &str) -> Result<ModelSpec, CliError> {
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

fn build_bounds(spec: &BoundsSpec, input_dim: usize) -> Result<Bounds, CliError> {
    let expand = |values: &[f64]| match values {
        [single] if input_dim > 0 => vec![*single; input_dim],
        other => other.to_vec(),
    };
    Bounds::new(expand(&spec.lower), expand(&spec.upper))
        .map_err(|e| CliError::Input(format!("bounds: {e}"), None))
}