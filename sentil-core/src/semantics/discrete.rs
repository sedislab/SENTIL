//! Discrete-time offline robustness over a sampled trace.

use std::collections::BTreeMap;

use super::eval::eval_predicate;
use super::window::{sliding_window_max, sliding_window_min};
use crate::error::{Error, Result};
use crate::formula::Formula;

/// Tolerance for the time comparisons in `until` and `since`, which compare
/// shifted timestamps that can differ by a rounding step.
const EPSILON: f64 = 1e-12;

/// Robustness of `formula` at every sample index of the trace.
///
/// `signals` holds one value series per variable, each aligned with `times`.
pub(crate) fn robustness_trace(
    formula: &Formula,
    times: &[f64],
    signals: &BTreeMap<String, Vec<f64>>,
) -> Result<Vec<f64>> {
    eval(formula, times, signals)
}

fn eval(
    formula: &Formula,
    times: &[f64],
    signals: &BTreeMap<String, Vec<f64>>,
) -> Result<Vec<f64>> {
    let n = times.len();
    match formula {
        Formula::Predicate(p) => (0..n)
            .map(|i| {
                let lookup = |name: &str| signals.get(name).and_then(|col| col.get(i)).copied();
                eval_predicate(p, &lookup)
            })
            .collect(),
        Formula::Not(f) => Ok(eval(f, times, signals)?.into_iter().map(|x| -x).collect()),
        Formula::And(l, r) => Ok(combine(
            eval(l, times, signals)?,
            &eval(r, times, signals)?,
            f64::min,
        )),
        Formula::Or(l, r) => Ok(combine(
            eval(l, times, signals)?,
            &eval(r, times, signals)?,
            f64::max,
        )),
        Formula::Implies(l, r) => Ok(combine(
            eval(l, times, signals)?,
            &eval(r, times, signals)?,
            |x, y| (-x).max(y),
        )),
        Formula::Always(interval, f) => Ok(sliding_window_min(
            &eval(f, times, signals)?,
            times,
            interval.lower,
            interval.upper_or_infinity(),
        )),
        Formula::Eventually(interval, f) => Ok(sliding_window_max(
            &eval(f, times, signals)?,
            times,
            interval.lower,
            interval.upper_or_infinity(),
        )),
        Formula::Historically(interval, f) => Ok(sliding_window_min(
            &eval(f, times, signals)?,
            times,
            -interval.upper_or_infinity(),
            -interval.lower,
        )),
        Formula::Once(interval, f) => Ok(sliding_window_max(
            &eval(f, times, signals)?,
            times,
            -interval.upper_or_infinity(),
            -interval.lower,
        )),
        Formula::Until(interval, l, r) => Ok(until(
            &eval(l, times, signals)?,
            &eval(r, times, signals)?,
            times,
            interval.lower,
            interval.upper_or_infinity(),
        )),
        Formula::Since(interval, l, r) => Ok(since(
            &eval(l, times, signals)?,
            &eval(r, times, signals)?,
            times,
            interval.lower,
            interval.upper_or_infinity(),
        )),
        Formula::Next(f) => Ok(next(&eval(f, times, signals)?)),
        Formula::Probabilistic(..) => Err(Error::ProbabilisticOperator),
    }
}