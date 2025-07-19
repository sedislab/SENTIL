//! Dense-time robustness over continuous signals.
//!
//! Signals are read as continuous between their samples. This first version
//! resamples each signal onto a fine uniform grid and runs the discrete
//! evaluator over it, trading exactness for the grid step.

use std::collections::BTreeMap;

use super::discrete::robustness_trace;
use crate::error::Result;
use crate::formula::Formula;
use crate::signal::Trace;

/// How many grid points to place between two original samples.
const SUBDIVISIONS: usize = 16;

/// The dense robustness at the trace start, via a resampled grid.
pub(crate) fn robustness_eager(formula: &Formula, trace: &Trace) -> Result<f64> {
    let (times, signals) = resample(trace.times(), trace.signals());
    let values = robustness_trace(formula, &times, &signals)?;
    Ok(values[0])
}

/// Linearly interpolates every signal onto a uniform grid.
fn resample(
    times: &[f64],
    signals: &BTreeMap<String, Vec<f64>>,
) -> (Vec<f64>, BTreeMap<String, Vec<f64>>) {
    if times.len() < 2 {
        return (times.to_vec(), signals.clone());
    }
    let mut grid = Vec::new();
    for pair in times.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        for k in 0..SUBDIVISIONS {
            grid.push(a + (b - a) * (k as f64) / (SUBDIVISIONS as f64));
        }
    }
    grid.push(times[times.len() - 1]);
    let resampled = signals
        .iter()
        .map(|(name, values)| {
            let column = grid.iter().map(|&t| interpolate(times, values, t)).collect();
            (name.clone(), column)
        })
        .collect();
    (grid, resampled)
}

fn interpolate(times: &[f64], values: &[f64], t: f64) -> f64 {
    match times.binary_search_by(|x| x.partial_cmp(&t).unwrap()) {
        Ok(i) => values[i],
        Err(0) => values[0],
        Err(i) if i >= times.len() => values[values.len() - 1],
        Err(i) => {
            let (t0, t1) = (times[i - 1], times[i]);
            let (v0, v1) = (values[i - 1], values[i]);
            v0 + (v1 - v0) * (t - t0) / (t1 - t0)
        }
    }
}