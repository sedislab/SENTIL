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
use std::collections::BTreeMap;

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
            interval.lower,
            interval.upper_or_infinity(),
            true,
        )),
        Formula::Eventually(interval, f) => Ok(window(
            &robustness_signal(f, times, signals)?,
            interval.lower,
            interval.upper_or_infinity(),
            false,
        )),
        Formula::Historically(interval, f) => Ok(window(
            &robustness_signal(f, times, signals)?,
            -interval.upper_or_infinity(),
            -interval.lower,
            true,
        )),
        Formula::Once(interval, f) => Ok(window(
            &robustness_signal(f, times, signals)?,
            -interval.upper_or_infinity(),
            -interval.lower,
            false,
        )),
        Formula::Until(interval, l, r) => Ok(until_signal(
            &robustness_signal(l, times, signals)?,
            &robustness_signal(r, times, signals)?,
            interval.lower,
            interval.upper_or_infinity(),
        )),
        Formula::Since(interval, l, r) => Ok(since_signal(
            &robustness_signal(l, times, signals)?,
            &robustness_signal(r, times, signals)?,
            interval.lower,
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

/// Dense `phi U_[a,b] psi`: at each time the best future instant `s` in the window
/// where `psi` holds and `phi` held throughout `[t, s]`.
fn until_signal(phi: &Pwl, psi: &Pwl, a: f64, b: f64) -> Pwl {
    let (times, phi_vals, psi_vals) = common_grid(phi, psi);
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
    Pwl::new(times.into_iter().zip(result).collect())
}

/// Dense `phi S_[a,b] psi`: the past dual of [`until_signal`], scanning backward.
fn since_signal(phi: &Pwl, psi: &Pwl, a: f64, b: f64) -> Pwl {
    let (times, phi_vals, psi_vals) = common_grid(phi, psi);
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
    Pwl::new(times.into_iter().zip(result).collect())
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
}