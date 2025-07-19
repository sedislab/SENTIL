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