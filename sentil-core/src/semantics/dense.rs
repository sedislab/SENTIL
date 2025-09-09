//! Dense-time robustness over piecewise-linear signals.
//!
//! The trace's samples are read as a continuous signal, linear between points.
//! Each subformula becomes a [`Pwl`] robustness signal, and the answer at any
//! time is read off that. For the single-operand operators dense and discrete
//! agree at the samples, parting only when a temporal window's edge or an
//! equality predicate's zero falls between them. `until` and `since` differ even
//! at the samples: the dense path takes the left operand's infimum over the
//! closed span up to the witness, the discrete path over the half-open span that
//! excludes it.
//!
//! Predicates must be linear in the signals. A linear term stays linear between
//! samples, so sampling it and joining the points reproduces it exactly; a
//! nonlinear term would curve between samples, so it is rejected rather than
//! silently approximated.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
#[cfg(feature = "std")]
use std::collections::{BTreeMap, VecDeque};

use super::eval::eval_expr;
use super::pwl::{combine, crossing, window, Pwl};
use crate::error::{Error, Result};
use crate::formula::{BinaryOp, ComparisonOp, Expr, Formula, Predicate};

/// Slack for comparing window edges against breakpoint times.
const EPS: f64 = 1e-9;

pub(crate) fn robustness_signal(
    formula: &Formula,
    times: &[f64],
    signals: &BTreeMap<String, Vec<f64>>,
) -> Result<Pwl> {
    match formula {
        Formula::Predicate(p) => predicate(p, times, signals),
        Formula::Not(f) => Ok(robustness_signal(f, times, signals)?.negate()),
        Formula::And(l, r) => Ok(combine(
            &robustness_signal(l, times, signals)?,
            &robustness_signal(r, times, signals)?,
            f64::min,
        )),
        Formula::Or(l, r) => Ok(combine(
            &robustness_signal(l, times, signals)?,
            &robustness_signal(r, times, signals)?,
            f64::max,
        )),
        Formula::Implies(l, r) => Ok(combine(
            &robustness_signal(l, times, signals)?.negate(),
            &robustness_signal(r, times, signals)?,
            f64::max,
        )),
        Formula::Always(interval, f) => Ok(window(
            &robustness_signal(f, times, signals)?,
            interval.lower(),
            interval.upper_or_infinity(),
            true,
        )),
        Formula::Eventually(interval, f) => Ok(window(
            &robustness_signal(f, times, signals)?,
            interval.lower(),
            interval.upper_or_infinity(),
            false,
        )),
        Formula::Historically(interval, f) => Ok(window(
            &robustness_signal(f, times, signals)?,
            -interval.upper_or_infinity(),
            -interval.lower(),
            true,
        )),
        Formula::Once(interval, f) => Ok(window(
            &robustness_signal(f, times, signals)?,
            -interval.upper_or_infinity(),
            -interval.lower(),
            false,
        )),
        Formula::Until(interval, l, r) => Ok(until_signal(
            &robustness_signal(l, times, signals)?,
            &robustness_signal(r, times, signals)?,
            interval.lower(),
            interval.upper_or_infinity(),
        )),
        Formula::Since(interval, l, r) => Ok(since_signal(
            &robustness_signal(l, times, signals)?,
            &robustness_signal(r, times, signals)?,
            interval.lower(),
            interval.upper_or_infinity(),
        )),
        Formula::Probabilistic(..) => Err(Error::ProbabilisticOperator),
        Formula::Next(_) => Err(Error::Unsupported {
            feature: "next in dense time; use discrete robustness for it",
        }),
    }
}

/// A shared breakpoint grid for two signals, with both sampled on it.
fn common_grid(phi: &Pwl, psi: &Pwl) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut times: Vec<f64> = phi.times().chain(psi.times()).collect();
    times.sort_by(f64::total_cmp);
    times.dedup();
    let phi_vals = times.iter().map(|&t| phi.at(t)).collect();
    let psi_vals = times.iter().map(|&t| psi.at(t)).collect();
    (times, phi_vals, psi_vals)
}

/// Dense `phi U_[a,b] psi`: at each breakpoint the best future instant `s` in the
/// window where `psi` holds and `phi` held throughout the closed span `[t, s]`.
fn until_signal(phi: &Pwl, psi: &Pwl, a: f64, b: f64) -> Pwl {
    let (times, phi_vals, psi_vals) = common_grid(phi, psi);
    let values = until_values(&times, &phi_vals, &psi_vals, a, b);
    Pwl::new(times.into_iter().zip(values).collect())
}

/// Dense `phi S_[a,b] psi`: the past dual of [`until_signal`], over the closed span.
fn since_signal(phi: &Pwl, psi: &Pwl, a: f64, b: f64) -> Pwl {
    let (times, phi_vals, psi_vals) = common_grid(phi, psi);
    let values = since_values(&times, &phi_vals, &psi_vals, a, b);
    Pwl::new(times.into_iter().zip(values).collect())
}

/// Dense until robustness over a shared breakpoint grid. With a zero lower bound a
/// monotonic deque answers in O(n) amortized; a positive lower bound falls back to
/// the exhaustive scan. The deque's domination drops a farther witness whenever a
/// nearer one already beats it, which is only sound when every future sample is in
/// the window. A positive lower bound breaks that: a sample the deque kept can sit
/// below the bound from a later start while the farther one it discarded was the
/// only valid witness, so the scan stays the reference there.
fn until_values(times: &[f64], phi_vals: &[f64], psi_vals: &[f64], a: f64, b: f64) -> Vec<f64> {
    if a <= EPS {
        until_deque(times, phi_vals, psi_vals, b)
    } else {
        until_naive(times, phi_vals, psi_vals, a, b)
    }
}

/// Dense since robustness over a shared breakpoint grid; the past dual of
/// [`until_values`], with the same zero lower bound split.
fn since_values(times: &[f64], phi_vals: &[f64], psi_vals: &[f64], a: f64, b: f64) -> Vec<f64> {
    if a <= EPS {
        since_deque(times, phi_vals, psi_vals, b)
    } else {
        since_naive(times, phi_vals, psi_vals, a, b)
    }
}

/// O(n) amortized dense until for a zero lower bound. Folding `phi` into the
/// witness value as `min(psi, phi)` turns the closed span the dense path uses
/// (phi held through the witness) into the half-open form the deque carries, so
/// the result equals [`until_naive`] sample for sample. The deque holds candidate
/// `(time, value)` pairs with values rising toward the back; sweeping `i` down, a
/// new sample folds `phi[i]` into the survivors and offers `min(psi[i], phi[i])`
/// as its own witness, and the answer for `i` is the back value once the entries
/// past `t + b` are dropped.
fn until_deque(times: &[f64], phi_vals: &[f64], psi_vals: &[f64], b: f64) -> Vec<f64> {
    let n = times.len();
    let mut result = vec![f64::NEG_INFINITY; n];
    let mut deque: VecDeque<(f64, f64)> = VecDeque::new();
    for i in (0..n).rev() {
        let t = times[i];
        let p = phi_vals[i];
        // Cap every retained witness by phi[i]; the dominated ones collapse into a
        // single entry at the nearest of their times carrying phi[i].
        let mut collapsed = None;
        while let Some(&(tb, vb)) = deque.back() {
            if vb >= p {
                collapsed = Some(tb);
                deque.pop_back();
            } else {
                break;
            }
        }
        if let Some(tc) = collapsed {
            deque.push_back((tc, p));
        }
        let witness = psi_vals[i].min(p);
        while deque.front().is_some_and(|&(_, vf)| vf <= witness) {
            deque.pop_front();
        }
        deque.push_front((t, witness));
        let window_end = t + b;
        while deque.back().is_some_and(|&(tb, _)| tb > window_end + EPS) {
            deque.pop_back();
        }
        result[i] = deque.back().map_or(f64::NEG_INFINITY, |&(_, v)| v);
    }
    result
}

/// O(n) amortized dense since for a zero lower bound; the forward-sweeping dual of
/// [`until_deque`], reading the front once entries before `t - b` are dropped.
fn since_deque(times: &[f64], phi_vals: &[f64], psi_vals: &[f64], b: f64) -> Vec<f64> {
    let n = times.len();
    let mut result = vec![f64::NEG_INFINITY; n];
    let mut deque: VecDeque<(f64, f64)> = VecDeque::new();
    for i in 0..n {
        let t = times[i];
        let p = phi_vals[i];
        let mut collapsed = None;
        while let Some(&(tf, vf)) = deque.front() {
            if vf >= p {
                collapsed = Some(tf);
                deque.pop_front();
            } else {
                break;
            }
        }
        if let Some(tc) = collapsed {
            deque.push_front((tc, p));
        }
        let witness = psi_vals[i].min(p);
        while deque.back().is_some_and(|&(_, vb)| vb <= witness) {
            deque.pop_back();
        }
        deque.push_back((t, witness));
        let window_start = t - b;
        while deque.front().is_some_and(|&(tf, _)| tf < window_start - EPS) {
            deque.pop_front();
        }
        result[i] = deque.front().map_or(f64::NEG_INFINITY, |&(_, v)| v);
    }
    result
}

/// The exhaustive dense until, the correctness reference and the path a positive
/// lower bound takes.
fn until_naive(times: &[f64], phi_vals: &[f64], psi_vals: &[f64], a: f64, b: f64) -> Vec<f64> {
    let n = times.len();
    let mut result = vec![f64::NEG_INFINITY; n];
    for i in 0..n {
        let mut best = f64::NEG_INFINITY;
        let mut phi_inf = f64::INFINITY;
        for s in i..n {
            phi_inf = phi_inf.min(phi_vals[s]);
            let dt = times[s] - times[i];
            if dt > b + EPS {
                break;
            }
            if dt >= a - EPS {
                best = best.max(psi_vals[s].min(phi_inf));
            }
        }
        result[i] = best;
    }
    result
}

/// The exhaustive dense since, the past dual of [`until_naive`].
fn since_naive(times: &[f64], phi_vals: &[f64], psi_vals: &[f64], a: f64, b: f64) -> Vec<f64> {
    let n = times.len();
    let mut result = vec![f64::NEG_INFINITY; n];
    for i in 0..n {
        let mut best = f64::NEG_INFINITY;
        let mut phi_inf = f64::INFINITY;
        for s in (0..=i).rev() {
            phi_inf = phi_inf.min(phi_vals[s]);
            let dt = times[i] - times[s];
            if dt > b + EPS {
                break;
            }
            if dt >= a - EPS {
                best = best.max(psi_vals[s].min(phi_inf));
            }
        }
        result[i] = best;
    }
    result
}

fn predicate(p: &Predicate, times: &[f64], signals: &BTreeMap<String, Vec<f64>>) -> Result<Pwl> {
    if !is_linear(&p.lhs) || !is_linear(&p.rhs) {
        return Err(Error::Unsupported {
            feature: "dense-time evaluation needs linear predicates; rewrite it or use discrete robustness",
        });
    }
    let mut diff = Vec::with_capacity(times.len());
    for i in 0..times.len() {
        let lookup = |name: &str| signals.get(name).and_then(|col| col.get(i)).copied();
        diff.push(eval_expr(&p.lhs, &lookup)? - eval_expr(&p.rhs, &lookup)?);
    }
    let signal = match p.op {
        ComparisonOp::Less | ComparisonOp::LessEqual => {
            Pwl::new(times.iter().zip(&diff).map(|(&t, &d)| (t, -d)).collect())
        }
        ComparisonOp::Greater | ComparisonOp::GreaterEqual => {
            Pwl::new(times.iter().zip(&diff).map(|(&t, &d)| (t, d)).collect())
        }
        ComparisonOp::Equal => abs_margin(times, &diff, true),
        ComparisonOp::NotEqual => abs_margin(times, &diff, false),
    };
    Ok(signal)
}

/// The robustness of an equality or inequality predicate, `-|diff|` or `|diff|`.
fn abs_margin(times: &[f64], diff: &[f64], equality: bool) -> Pwl {
    let raw = Pwl::new(times.iter().zip(diff).map(|(&t, &d)| (t, d)).collect());
    let mut breakpoints: Vec<f64> = times.to_vec();
    for i in 0..times.len().saturating_sub(1) {
        let (d0, d1) = (diff[i], diff[i + 1]);
        if d0 != 0.0 && d1 != 0.0 && (d0 < 0.0) != (d1 < 0.0) {
            breakpoints.push(times[i] + (times[i + 1] - times[i]) * d0 / (d0 - d1));
        }
    }
    breakpoints.sort_by(f64::total_cmp);
    breakpoints.dedup();
    Pwl::new(
        breakpoints
            .into_iter()
            .map(|t| {
                let magnitude = raw.at(t).abs();
                (t, if equality { -magnitude } else { magnitude })
            })
            .collect(),
    )
}

/// Whether the term is a sum of scaled signals and constants.
fn is_linear(expr: &Expr) -> bool {
    if is_constant(expr) {
        return true;
    }
    match expr {
        Expr::Variable(_) => true,
        Expr::Binary(BinaryOp::Add | BinaryOp::Sub, l, r) => is_linear(l) && is_linear(r),
        Expr::Binary(BinaryOp::Mul, l, r) => {
            (is_constant(l) && is_linear(r)) || (is_linear(l) && is_constant(r))
        }
        Expr::Binary(BinaryOp::Div, l, r) => is_linear(l) && is_constant(r),
        _ => false,
    }
}

/// Whether the term references no signal.
fn is_constant(expr: &Expr) -> bool {
    match expr {
        Expr::Literal(_) => true,
        Expr::Variable(_) => false,
        Expr::Binary(_, l, r) => is_constant(l) && is_constant(r),
        Expr::Call(_, args) => args.iter().all(is_constant),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "the dense values here are exact")]
    #![allow(
        clippy::cast_precision_loss,
        reason = "test trace indices are tiny, so the index-to-time cast is exact"
    )]

    use proptest::prelude::*;

    use super::*;
    use crate::formula::Formula;

    fn dense_at_start(formula: &str, times: &[f64], signals: &[(&str, &[f64])]) -> f64 {
        let phi = Formula::parse(formula).unwrap();
        let map: BTreeMap<String, Vec<f64>> = signals
            .iter()
            .map(|(n, v)| ((*n).to_string(), v.to_vec()))
            .collect();
        robustness_signal(&phi, times, &map).unwrap().at(times[0])
    }

    #[test]
    fn dense_catches_a_window_edge_between_samples() {
        let r = dense_at_start(
            "always[0, 1.5](x > 0)",
            &[0.0, 1.0, 2.0],
            &[("x", &[1.0, 1.0, -3.0])],
        );
        assert_eq!(r, -1.0);
    }

    #[test]
    fn dense_agrees_with_samples_when_the_window_lands_on_them() {
        let r = dense_at_start(
            "always[0, 2](x > 0)",
            &[0.0, 1.0, 2.0],
            &[("x", &[10.0, 5.0, 1.0])],
        );
        assert_eq!(r, 1.0);
    }

    #[test]
    fn equality_predicate_finds_the_crossing() {
        let r = dense_at_start(
            "eventually[0, 2](x == 0)",
            &[0.0, 2.0],
            &[("x", &[1.0, -1.0])],
        );
        assert_eq!(r, 0.0);
    }

    #[test]
    fn nonlinear_predicate_is_rejected() {
        let phi = Formula::parse("always[0, 1](x * y > 0)").unwrap();
        let map: BTreeMap<String, Vec<f64>> =
            [("x".to_string(), vec![1.0]), ("y".to_string(), vec![1.0])]
                .into_iter()
                .collect();
        assert!(matches!(
            robustness_signal(&phi, &[0.0], &map),
            Err(Error::Unsupported { .. })
        ));
    }

    fn dense_at(formula: &str, t: f64, times: &[f64], signals: &[(&str, &[f64])]) -> f64 {
        let phi = Formula::parse(formula).unwrap();
        let map: BTreeMap<String, Vec<f64>> = signals
            .iter()
            .map(|(n, v)| ((*n).to_string(), v.to_vec()))
            .collect();
        robustness_signal(&phi, times, &map).unwrap().at(t)
    }

    #[test]
    fn dense_until_takes_the_best_future_witness() {
        let r = dense_at(
            "(x > 0) until[0, 2] (y > 0)",
            0.0,
            &[0.0, 1.0, 2.0],
            &[("x", &[10.0, 10.0, 10.0]), ("y", &[-2.0, 0.0, 2.0])],
        );
        assert_eq!(r, 2.0);
    }

    #[test]
    fn dense_until_is_capped_by_a_dip_in_the_left_operand() {
        let r = dense_at(
            "(x > 0) until[1, 2] (y > 0)",
            0.0,
            &[0.0, 1.0, 2.0],
            &[("x", &[10.0, -5.0, 10.0]), ("y", &[2.0, 2.0, 2.0])],
        );
        assert_eq!(r, -5.0);
    }

    #[test]
    fn dense_since_takes_the_best_past_witness() {
        let r = dense_at(
            "(x > 0) since[0, 2] (y > 0)",
            2.0,
            &[0.0, 1.0, 2.0],
            &[("x", &[10.0, 10.0, 10.0]), ("y", &[2.0, 0.0, -2.0])],
        );
        assert_eq!(r, 2.0);
    }

    #[test]
    fn next_is_rejected_in_dense_time() {
        let phi = Formula::parse("next(x > 0)").unwrap();
        let map: BTreeMap<String, Vec<f64>> = [("x".to_string(), vec![1.0])].into_iter().collect();
        assert!(matches!(
            robustness_signal(&phi, &[0.0], &map),
            Err(Error::Unsupported { .. })
        ));
    }

    proptest! {
        #[test]
        fn dense_matches_discrete_on_aligned_windows(
            values in prop::collection::vec(-20.0f64..20.0, 1..30),
        ) {
            let times: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();
            let map: BTreeMap<String, Vec<f64>> =
                [("x".to_string(), values.clone())].into_iter().collect();
            for formula in [
                "always[0, 3](x > 0)",
                "eventually[0, 2](x > 5)",
                "historically[0, 2](x > -5)",
            ] {
                let phi = Formula::parse(formula).unwrap();
                let dense = robustness_signal(&phi, &times, &map).unwrap();
                let discrete = super::super::discrete::robustness_trace(&phi, &times, &map).unwrap();
                for (i, &t) in times.iter().enumerate() {
                    prop_assert_eq!(dense.at(t), discrete[i], "{} at t={}", formula, t);
                }
            }
        }
    }

    /// Strictly increasing times built from positive gaps, with paired phi and psi
    /// values, the shape the until and since deques sweep.
    fn grid_strategy() -> impl Strategy<Value = (Vec<f64>, Vec<f64>, Vec<f64>)> {
        prop::collection::vec((0.1f64..5.0, -20.0f64..20.0, -20.0f64..20.0), 1..30).prop_map(
            |rows| {
                let mut t = 0.0;
                let (mut times, mut phi, mut psi) = (Vec::new(), Vec::new(), Vec::new());
                for (gap, p, s) in rows {
                    t += gap;
                    times.push(t);
                    phi.push(p);
                    psi.push(s);
                }
                (times, phi, psi)
            },
        )
    }

    proptest! {
        /// The zero-lower deque reproduces the exhaustive scan for until and since,
        /// across finite and open upper bounds.
        #[test]
        fn deque_matches_the_scan_at_zero_lower_bound(
            (times, phi, psi) in grid_strategy(),
            span in 0.0f64..12.0,
            open in any::<bool>(),
        ) {
            let b = if open { f64::INFINITY } else { span };
            prop_assert_eq!(
                super::until_deque(&times, &phi, &psi, b),
                super::until_naive(&times, &phi, &psi, 0.0, b)
            );
            prop_assert_eq!(
                super::since_deque(&times, &phi, &psi, b),
                super::since_naive(&times, &phi, &psi, 0.0, b)
            );
        }

        /// The dispatcher sends a zero lower bound to the deque and a positive one
        /// to the scan, returning the scan's value in both regimes.
        #[test]
        fn values_match_the_scan_for_any_bound(
            (times, phi, psi) in grid_strategy(),
            a in prop_oneof![Just(0.0f64), 0.05f64..6.0],
            span in 0.0f64..12.0,
            open in any::<bool>(),
        ) {
            let b = if open { f64::INFINITY } else { a + span };
            prop_assert_eq!(
                super::until_values(&times, &phi, &psi, a, b),
                super::until_naive(&times, &phi, &psi, a, b)
            );
            prop_assert_eq!(
                super::since_values(&times, &phi, &psi, a, b),
                super::since_naive(&times, &phi, &psi, a, b)
            );
        }
    }

    #[test]
    fn the_zero_lower_until_stays_linear() {
        // The old scan was quadratic for a wide upper bound; a regression to that
        // would run for a minute or more, far past this budget, while the deque
        // clears a hundred thousand samples in milliseconds.
        let n = 100_000;
        let times: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let phi: Vec<f64> = (0..n).map(|i| ((i * 7) % 13) as f64 - 6.0).collect();
        let psi: Vec<f64> = (0..n).map(|i| ((i * 5) % 11) as f64 - 5.0).collect();
        let start = std::time::Instant::now();
        let out = super::until_deque(&times, &phi, &psi, 1e9);
        assert_eq!(out.len(), n);
        assert!(start.elapsed() < std::time::Duration::from_secs(60));
    }
}