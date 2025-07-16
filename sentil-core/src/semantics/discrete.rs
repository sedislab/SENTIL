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

fn combine(mut left: Vec<f64>, right: &[f64], op: fn(f64, f64) -> f64) -> Vec<f64> {
    for (l, &r) in left.iter_mut().zip(right) {
        *l = op(*l, r);
    }
    left
}

/// Robustness of `phi until[a, b] psi`.
fn until(phi: &[f64], psi: &[f64], times: &[f64], a: f64, b: f64) -> Vec<f64> {
    let n = phi.len();
    if n == 0 {
        return Vec::new();
    }
    if a <= 0.0 && b.is_infinite() {
        let mut carried = f64::NEG_INFINITY;
        let mut result = vec![0.0; n];
        for j in (0..n).rev() {
            carried = psi[j].max(phi[j].min(carried));
            result[j] = carried;
        }
        return result;
    }

    let mut result = vec![f64::NEG_INFINITY; n];
    for i in 0..n {
        let window_start = times[i] + a;
        let window_end = if b.is_infinite() {
            f64::INFINITY
        } else {
            times[i] + b
        };
        if window_start > times[n - 1] + EPSILON {
            continue;
        }
        let first = times.partition_point(|&t| t < window_start - EPSILON);
        let mut min_phi = f64::INFINITY;
        let mut best = f64::NEG_INFINITY;
        for j in i..n {
            if j > i {
                min_phi = min_phi.min(phi[j - 1]);
            }
            if times[j] > window_end + EPSILON {
                break;
            }
            if j >= first {
                best = best.max(psi[j].min(min_phi));
            }
        }
        result[i] = best;
    }
    result
}

/// Robustness of `phi since[a, b] psi`, the past-time mirror of `until`.
fn since(phi: &[f64], psi: &[f64], times: &[f64], a: f64, b: f64) -> Vec<f64> {
    let n = phi.len();
    if n == 0 {
        return Vec::new();
    }
    let mut result = vec![f64::NEG_INFINITY; n];
    for i in 0..n {
        let window_end = times[i] - a;
        let window_start = if b.is_infinite() {
            0.0
        } else {
            (times[i] - b).max(0.0)
        };
        let last = times.partition_point(|&t| t <= window_end + EPSILON);
        let mut min_phi = f64::INFINITY;
        let mut best = f64::NEG_INFINITY;
        for j in (0..=i).rev() {
            if j < i {
                min_phi = min_phi.min(phi[j + 1]);
            }
            if times[j] < window_start - EPSILON {
                break;
            }
            if j < last {
                best = best.max(psi[j].min(min_phi));
            }
        }
        result[i] = best;
    }
    result
}

/// Robustness of `next phi`.
fn next(inner: &[f64]) -> Vec<f64> {
    let n = inner.len();
    let mut result = vec![f64::NEG_INFINITY; n];
    if n > 1 {
        result[..n - 1].copy_from_slice(&inner[1..]);
    }
    result
}