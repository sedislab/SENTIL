//! Dense-time robustness over piecewise-linear signals.
//!
//! The trace's samples are read as a continuous signal, linear between points.
//! Each subformula becomes a [`Pwl`] robustness signal, and the answer at any
//! time is read off that. Dense and discrete agree at the samples; they part
//! when a temporal window's edge, or an equality predicate's zero, falls between
//! samples, which dense captures exactly.
//!
//! Predicates must be linear in the signals. A linear term stays linear between
//! samples, so sampling it and joining the points reproduces it exactly; a
//! nonlinear term would curve between samples, so it is rejected rather than
//! silently approximated.

use std::collections::BTreeMap;

use super::eval::eval_expr;
use super::pwl::{combine, crossing, window, Pwl};
use crate::error::{Error, Result};
use crate::formula::{BinaryOp, ComparisonOp, Expr, Formula, Predicate};

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
        Formula::Probabilistic(..) => Err(Error::ProbabilisticOperator),
        _ => Err(Error::Unsupported {
            feature: "the until, since, and next operators in dense time (added next)",
        }),
    }
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