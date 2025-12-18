//! `sentil fit` for fitting a noise model from paired ground-truth and sensor columns

use sentil::{NoiseInteraction, NoiseModel, Trace};
use serde_json::json;

use crate::cli::{FitModel, Interaction};
use crate::engine;
use crate::error::{code, CliError, Run};
use crate::output::Out;

const MIXTURE_ITERS: usize = 100;

#[allow(clippy::too_many_arguments)]
pub fn run(
    trace_path: &str,
    truth: &str,
    sensor: &str,
    interaction: Interaction,
    model: FitModel,
    components: usize,
    map: &[String],
    out: &Out,
) -> Run {
    let trace = engine::load_trace(trace_path, map)?;
    let truth_values = column(&trace, truth)?;
    let sensor_values = column(&trace, sensor)?;
    let coupling = match interaction {
        Interaction::Additive => NoiseInteraction::Additive,
        Interaction::Multiplicative => NoiseInteraction::Multiplicative,
    };

    let residuals = NoiseModel::residuals(truth_values, sensor_values, coupling)
        .map_err(|e| CliError::Input(format!("residuals: {e}"), None))?;
    let count = residuals.len();
    let fitted = match model {
        FitModel::Gaussian => NoiseModel::fit_gaussian(&residuals),
        FitModel::Bootstrap => NoiseModel::fit_bootstrap(&residuals),
        FitModel::Mixture => NoiseModel::fit_gaussian_mixture(&residuals, components, MIXTURE_ITERS),
    }
    .map_err(|e| CliError::Engine(e.to_string()))?;

    if out.is_text() {
        out.heading("fit");
        out.field("truth", truth);
        out.field("sensor", sensor);
        out.field("interaction", &interaction.to_string());
        out.field("model", &model.to_string());
        out.field("residuals", &count.to_string());
        if let Some(mean) = fitted.mean() {
            out.field("mean", &format!("{mean:.6}"));
        }
        if let Some(variance) = fitted.variance() {
            out.field("std", &format!("{:.6}", variance.sqrt()));
        }
    } else {
        let noise = serde_json::to_value(&fitted)
            .map_err(|e| CliError::Internal(format!("serializing the fitted model: {e}")))?;
        println!(
            "{}",
            json!({
                "schema_version": "1.0",
                "verb": "fit",
                "truth": truth,
                "sensor": sensor,
                "interaction": interaction.to_string(),
                "model": model.to_string(),
                "residuals": count,
                "noise": noise,
            })
        );
    }
    Ok(code::SUCCESS)
}

fn column<'a>(trace: &'a Trace, name: &str) -> Result<&'a [f64], CliError> {
    trace.signal(name).ok_or_else(|| {
        CliError::Input(
            format!("no column '{name}' in the calibration data"),
            Some(format!("the data has: {}", trace.variables().join(", "))),
        )
    })
}