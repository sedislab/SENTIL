//! Discrete-time offline robustness over a sampled trace.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
#[cfg(feature = "std")]
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

/// The largest sample index the robustness of `formula` at index `i` reads.
pub(crate) fn max_dep_index(formula: &Formula, i: usize, times: &[f64]) -> usize {
    let last = times.len() - 1;
    // always and eventually admit the window up to `times[i] + upper` exactly;
    // until and since break one step past the edge, so they pass the EPSILON
    // slack to land on that step.
    let horizon = |bound: f64, slack: f64| -> usize {
        if bound.is_infinite() {
            last
        } else {
            times
                .partition_point(|&t| t <= times[i] + bound + slack)
                .saturating_sub(1)
                .min(last)
        }
    };
    match formula {
        Formula::Predicate(_) => i,
        Formula::Not(f) | Formula::Historically(_, f) | Formula::Once(_, f) => {
            max_dep_index(f, i, times)
        }
        Formula::And(l, r) | Formula::Or(l, r) | Formula::Implies(l, r) => {
            max_dep_index(l, i, times).max(max_dep_index(r, i, times))
        }
        Formula::Always(iv, f) | Formula::Eventually(iv, f) => i.max(max_dep_index(
            f,
            horizon(iv.upper_or_infinity(), 0.0),
            times,
        )),
        Formula::Until(iv, l, r) => {
            let hi = horizon(iv.upper_or_infinity(), EPSILON);
            i.max(max_dep_index(l, hi, times))
                .max(max_dep_index(r, hi, times))
        }
        Formula::Since(_, l, r) => max_dep_index(l, i, times).max(max_dep_index(r, i, times)),
        Formula::Next(f) => max_dep_index(f, (i + 1).min(last), times),
        Formula::Probabilistic(..) => last,
    }
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

#[cfg(test)]
mod tests {
    use super::{max_dep_index, robustness_trace};
    use crate::formula::{ComparisonOp, Expr, Formula, Interval, Predicate};
    use proptest::prelude::*;
    use std::collections::BTreeMap;

    fn dep(formula: &str) -> usize {
        let times = [0.0, 1.0, 2.0, 3.0, 4.0];
        max_dep_index(&Formula::parse(formula).unwrap(), 0, &times)
    }

    #[test]
    fn horizon_reaches_only_as_far_as_the_formula_needs() {
        assert_eq!(dep("x > 0"), 0);
        assert_eq!(dep("always[0, 2](x > 0)"), 2);
        assert_eq!(dep("eventually[0, 2](x > 0)"), 2);
        assert_eq!(dep("next(x > 0)"), 1);
        assert_eq!(dep("historically[0, 2](x > 0)"), 0);
        assert_eq!(dep("(x > 0) until[0, 2] (y > 0)"), 2);
        assert_eq!(dep("always[0, 2](eventually[0, 1](x > 0))"), 3);
        assert_eq!(dep("always(x > 0)"), 4);
    }

    fn arb_op() -> impl Strategy<Value = ComparisonOp> {
        prop_oneof![
            Just(ComparisonOp::Less),
            Just(ComparisonOp::LessEqual),
            Just(ComparisonOp::Greater),
            Just(ComparisonOp::GreaterEqual),
            Just(ComparisonOp::Equal),
            Just(ComparisonOp::NotEqual),
        ]
    }

    fn arb_predicate() -> impl Strategy<Value = Formula> {
        (prop::sample::select(vec!["x", "y"]), arb_op(), -12.0f64..12.0).prop_map(|(name, op, c)| {
            Formula::Predicate(Predicate {
                lhs: Expr::Variable(name.to_string()),
                op,
                rhs: Expr::Literal(c),
            })
        })
    }

    fn arb_interval() -> impl Strategy<Value = Interval> {
        (0.0f64..5.0, prop::option::of(0.0f64..8.0)).prop_map(|(lo, width)| match width {
            Some(w) => Interval::bounded(lo, lo + w),
            None => Interval::from_lower(lo),
        })
    }

    fn arb_formula() -> impl Strategy<Value = Formula> {
        arb_predicate().prop_recursive(4, 48, 2, |inner| {
            prop_oneof![
                inner.clone().prop_map(|f| Formula::Not(Box::new(f))),
                (inner.clone(), inner.clone())
                    .prop_map(|(l, r)| Formula::And(Box::new(l), Box::new(r))),
                (inner.clone(), inner.clone())
                    .prop_map(|(l, r)| Formula::Or(Box::new(l), Box::new(r))),
                (inner.clone(), inner.clone())
                    .prop_map(|(l, r)| Formula::Implies(Box::new(l), Box::new(r))),
                (arb_interval(), inner.clone())
                    .prop_map(|(iv, f)| Formula::Always(iv, Box::new(f))),
                (arb_interval(), inner.clone())
                    .prop_map(|(iv, f)| Formula::Eventually(iv, Box::new(f))),
                (arb_interval(), inner.clone())
                    .prop_map(|(iv, f)| Formula::Historically(iv, Box::new(f))),
                (arb_interval(), inner.clone())
                    .prop_map(|(iv, f)| Formula::Once(iv, Box::new(f))),
                (arb_interval(), inner.clone(), inner.clone())
                    .prop_map(|(iv, l, r)| Formula::Until(iv, Box::new(l), Box::new(r))),
                (arb_interval(), inner.clone(), inner.clone())
                    .prop_map(|(iv, l, r)| Formula::Since(iv, Box::new(l), Box::new(r))),
                inner.prop_map(|f| Formula::Next(Box::new(f))),
            ]
        })
    }

    fn arb_trace() -> impl Strategy<Value = (Vec<f64>, BTreeMap<String, Vec<f64>>)> {
        prop::collection::vec((0.1f64..5.0, -20.0f64..20.0, -20.0f64..20.0), 1..30).prop_map(
            |rows| {
                let mut t = 0.0;
                let mut times = Vec::with_capacity(rows.len());
                let mut xs = Vec::with_capacity(rows.len());
                let mut ys = Vec::with_capacity(rows.len());
                for (gap, x, y) in rows {
                    t += gap;
                    times.push(t);
                    xs.push(x);
                    ys.push(y);
                }
                let mut signals = BTreeMap::new();
                signals.insert("x".to_string(), xs);
                signals.insert("y".to_string(), ys);
                (times, signals)
            },
        )
    }

    proptest! {
        #[test]
        fn prefix_robustness_equals_full_at_index_zero(
            phi in arb_formula(),
            (times, signals) in arb_trace(),
        ) {
            let full = robustness_trace(&phi, &times, &signals).unwrap();
            let m = max_dep_index(&phi, 0, &times) + 1;
            let prefix = robustness_trace(&phi, &times[..m], &signals).unwrap();
            prop_assert_eq!(full[0].to_bits(), prefix[0].to_bits());
        }
    }
}