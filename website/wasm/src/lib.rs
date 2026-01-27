//! A thin browser surface over the SENTIL engine. The interface is deliberately
//! small: a handful of functions that take and return JSON strings, plus one
//! streaming handle. Everything heavy stays in the core; this only marshals.

use sentil::{
    LiftingRegistry, Monitor, MonitorConfig, NoiseInteraction, NoiseModel, SmcConfig, TimeMode,
    Trace,
};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

fn err(msg: impl std::fmt::Display) -> String {
    msg.to_string()
}

fn build_trace(req: &Value) -> Result<Trace, String> {
    let times: Vec<f64> = req
        .get("times")
        .and_then(|t| t.as_array())
        .ok_or("missing times array")?
        .iter()
        .map(|v| v.as_f64().unwrap_or(f64::NAN))
        .collect();
    let mut trace = Trace::new(times).map_err(|e| e.to_string())?;
    if let Some(signals) = req.get("signals").and_then(|s| s.as_object()) {
        for (name, values) in signals {
            let vals: Vec<f64> = values
                .as_array()
                .ok_or("signal values must be an array")?
                .iter()
                .map(|v| v.as_f64().unwrap_or(f64::NAN))
                .collect();
            trace.add_signal(name, vals).map_err(|e| e.to_string())?;
        }
    }
    Ok(trace)
}

fn noise_from(kind: &str, params: &[f64]) -> Result<NoiseModel, String> {
    let p = |i: usize| params.get(i).copied().unwrap_or(0.0);
    let model = match kind {
        "gaussian" => NoiseModel::gaussian(p(0), p(1)),
        "uniform" => NoiseModel::uniform(p(0), p(1)),
        "log_normal" | "lognormal" => NoiseModel::log_normal(p(0), p(1)),
        "exponential" => NoiseModel::exponential(p(0)),
        "gamma" => NoiseModel::gamma(p(0), p(1)),
        "beta" => NoiseModel::beta(p(0), p(1)),
        other => return Err(format!("unsupported noise kind '{other}'")),
    };
    model.map_err(|e| e.to_string())
}

/// Parse a formula and report its variables, or the parse error.
pub fn parse_formula(src: &str) -> String {
    match Monitor::new(src, MonitorConfig::new()) {
        Ok(m) => {
            let vars: Vec<String> = m.formula().variables().into_iter().collect();
            json!({ "ok": true, "variables": vars, "error": Value::Null }).to_string()
        }
        Err(e) => json!({ "ok": false, "variables": [], "error": e.to_string() }).to_string(),
    }
}

/// Evaluate robustness over a whole trace: the scalar, the per-sample series, and
/// the violation intervals. `dense` selects interpolated dense-time semantics.
/// Estimate satisfaction probability of a PrSTL property over one noisy channel,
/// with its confidence interval.
/// The Wilson score interval, a pure closed form with no sampling, so the docs
/// can animate an interval shrinking as the trial count grows.
#[wasm_bindgen]