//! A thin browser surface over the SENTIL engine. The interface is deliberately
//! small: a handful of functions that take and return JSON strings, plus one
//! streaming handle. Everything heavy stays in the core; this only marshals.

use sentil::stats::wilson_interval;
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
#[wasm_bindgen]
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
#[wasm_bindgen]
pub fn robustness(req_json: &str) -> String {
    let req: Value = match serde_json::from_str(req_json) {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let formula = match req.get("formula").and_then(|f| f.as_str()) {
        Some(f) => f,
        None => return err("missing formula"),
    };
    let dense = req.get("dense").and_then(|d| d.as_bool()).unwrap_or(false);
    let config = MonitorConfig::new().time(if dense {
        TimeMode::Dense
    } else {
        TimeMode::Discrete
    });
    let monitor = match Monitor::new(formula, config) {
        Ok(m) => m,
        Err(e) => return err(e),
    };
    let trace = match build_trace(&req) {
        Ok(t) => t,
        Err(e) => return err(e),
    };
    let value = match monitor.robustness(&trace) {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let series = monitor.robustness_signal(&trace).unwrap_or_default();
    let violations: Vec<[f64; 2]> = monitor
        .violations(&trace)
        .unwrap_or_default()
        .into_iter()
        .map(|(a, b)| [a, b])
        .collect();
    json!({
        "ok": true,
        "value": value,
        "series": series,
        "violations": violations,
        "error": Value::Null,
    })
    .to_string()
}

/// Estimate satisfaction probability of a PrSTL property over one noisy channel,
/// with its confidence interval.
#[wasm_bindgen]
pub fn check_prstl(req_json: &str) -> String {
    let req: Value = match serde_json::from_str(req_json) {
        Ok(v) => v,
        Err(e) => return err(e),
    };
    let formula = match req.get("formula").and_then(|f| f.as_str()) {
        Some(f) => f,
        None => return err("missing formula"),
    };
    let noise = match req.get("noise") {
        Some(n) => n,
        None => return err("missing noise spec"),
    };
    let variable = match noise.get("variable").and_then(|v| v.as_str()) {
        Some(v) => v,
        None => return err("missing noise.variable"),
    };
    let kind = noise.get("kind").and_then(|k| k.as_str()).unwrap_or("gaussian");
    let params: Vec<f64> = noise
        .get("params")
        .and_then(|p| p.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();
    let interaction = match noise.get("interaction").and_then(|i| i.as_str()) {
        Some("multiplicative") => NoiseInteraction::Multiplicative,
        _ => NoiseInteraction::Additive,
    };
    let model = match noise_from(kind, &params) {
        Ok(m) => m,
        Err(e) => return err(e),
    };
    let mut lifting = LiftingRegistry::new();
    lifting.register(variable, model, interaction);
    let samples = req.get("samples").and_then(|s| s.as_u64()).unwrap_or(2000);
    let seed = req.get("seed").and_then(|s| s.as_u64()).unwrap_or(1);
    let smc = SmcConfig {
        samples,
        seed,
        ..SmcConfig::default()
    };
    let monitor = match Monitor::new(formula, MonitorConfig::new().smc(smc)) {
        Ok(m) => m,
        Err(e) => return err(e),
    };
    let trace = match build_trace(&req) {
        Ok(t) => t,
        Err(e) => return err(e),
    };
    match monitor.check(&trace, &lifting) {
        Ok(r) => json!({
            "ok": true,
            "probability": r.probability,
            "lo": r.interval.lower,
            "hi": r.interval.upper,
            "holds": r.holds,
            "error": Value::Null,
        })
        .to_string(),
        Err(e) => err(e),
    }
}

/// The Wilson score interval, a pure closed form with no sampling, so the docs
/// can animate an interval shrinking as the trial count grows.
#[wasm_bindgen]
pub fn wilson(successes: u32, trials: u32, level: f64) -> String {
    let ci = wilson_interval(successes as u64, trials as u64, level);
    json!({ "lo": ci.lower, "hi": ci.upper }).to_string()
}

/// A streaming monitor that folds one timestamped sample at a time. Values are
/// passed in the variable order reported by `parse_formula`.
#[wasm_bindgen]
pub struct StreamMonitor {
    inner: Monitor,
}

#[wasm_bindgen]
impl StreamMonitor {
    #[wasm_bindgen(constructor)]
    pub fn new(src: &str) -> Result<StreamMonitor, JsError> {
        Monitor::new(src, MonitorConfig::new())
            .map(|inner| StreamMonitor { inner })
            .map_err(|e| JsError::new(&e.to_string()))
    }

    pub fn update(&mut self, time: f64, values: Vec<f64>) -> String {
        match self.inner.update_packed(time, &values) {
            Ok(r) => json!({
                "value": r.value(),
                "resolved": r.is_resolved(),
                "satisfied": r.is_satisfied(),
            })
            .to_string(),
            Err(e) => err(e),
        }
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }
}