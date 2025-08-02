//! Smooth, differentiable robustness operators for synthesis.
//!
//! Monitoring uses exact min and max, which are not differentiable at ties.
//! Synthesis instead needs a robustness that varies smoothly with the trace so an
//! optimizer can follow its gradient. The replacements here are a log-sum-exp soft
//! minimum and maximum controlled by a temperature: as the temperature rises they
//! approach the exact operators, and at every temperature the soft minimum stays
//! at or below the true minimum and the soft maximum at or above the true maximum.

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::semantics::{eval_predicate, WINDOW_EPSILON};
use crate::signal::Trace;

/// The temperature for smooth robustness.
///
/// A higher temperature tracks the exact min and max more closely; a lower one is
/// smoother and easier for an optimizer to climb.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmoothConfig {
    temperature: f64,
}

impl SmoothConfig {
    /// Builds a configuration.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if `temperature` is not finite and
    /// positive.
    pub fn new(temperature: f64) -> Result<Self> {
        if temperature.is_finite() && temperature > 0.0 {
            Ok(Self { temperature })
        } else {
            Err(Error::InvalidConfig {
                context: "smooth robustness",
                message: format!("temperature must be finite and positive, got {temperature}"),
            })
        }
    }

    /// The temperature.
    #[must_use]
    pub fn temperature(&self) -> f64 {
        self.temperature
    }
}

impl Default for SmoothConfig {
    fn default() -> Self {
        Self { temperature: 10.0 }
    }
}

/// A smooth lower bound on the minimum of `values`, approaching the true minimum
/// as `beta` grows. An empty slice has minimum positive infinity.
#[must_use]
pub fn soft_min(values: &[f64], beta: f64) -> f64 {
    let shift = values.iter().copied().fold(f64::INFINITY, f64::min);
    if !shift.is_finite() {
        return shift;
    }
    let sum: f64 = values.iter().map(|&x| (-beta * (x - shift)).exp()).sum();
    shift - sum.ln() / beta
}

/// A smooth upper bound on the maximum of `values`, approaching the true maximum
/// as `beta` grows. An empty slice has maximum negative infinity.
#[must_use]
pub fn soft_max(values: &[f64], beta: f64) -> f64 {
    let shift = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !shift.is_finite() {
        return shift;
    }
    let sum: f64 = values.iter().map(|&x| (beta * (x - shift)).exp()).sum();
    shift + sum.ln() / beta
}

impl Formula {
    /// The smooth robustness of the formula over a trace: a differentiable
    /// surrogate for [`robustness`](Self::robustness) that synthesis can climb.
    ///
    /// It mirrors the exact robustness but with the soft minimum and maximum at
    /// the configured temperature, so the result varies smoothly with the trace.
    /// As the temperature rises it approaches the exact value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyTrace`] if the trace has no samples,
    /// [`Error::UnknownVariable`] if the formula names a missing signal, and
    /// [`Error::ProbabilisticOperator`] if the formula is probabilistic.
    pub fn smooth_robustness(&self, trace: &Trace, config: SmoothConfig) -> Result<f64> {
        if trace.is_empty() {
            return Err(Error::EmptyTrace);
        }
        let signal = soft_eval(self, trace.times(), trace.signals(), config.temperature())?;
        Ok(signal[0])
    }
}

/// The smooth robustness signal, mirroring `semantics::discrete::eval` with the
/// soft minimum and maximum in place of the exact ones.
fn soft_eval(
    formula: &Formula,
    times: &[f64],
    signals: &BTreeMap<String, Vec<f64>>,
    beta: f64,
) -> Result<Vec<f64>> {
    match formula {
        Formula::Predicate(p) => (0..times.len())
            .map(|i| {
                let lookup = |name: &str| signals.get(name).and_then(|col| col.get(i)).copied();
                eval_predicate(p, &lookup)
            })
            .collect(),
        Formula::Not(f) => Ok(soft_eval(f, times, signals, beta)?
            .into_iter()
            .map(|x| -x)
            .collect()),
        Formula::And(l, r) => {
            soft_combine(l, r, times, signals, beta, |x, y| soft_min(&[x, y], beta))
        }
        Formula::Or(l, r) => {
            soft_combine(l, r, times, signals, beta, |x, y| soft_max(&[x, y], beta))
        }
        Formula::Implies(l, r) => {
            soft_combine(l, r, times, signals, beta, |x, y| soft_max(&[-x, y], beta))
        }
        Formula::Always(interval, f) => Ok(soft_window(
            &soft_eval(f, times, signals, beta)?,
            times,
            interval.lower,
            interval.upper_or_infinity(),
            beta,
            soft_min,
        )),
        Formula::Eventually(interval, f) => Ok(soft_window(
            &soft_eval(f, times, signals, beta)?,
            times,
            interval.lower,
            interval.upper_or_infinity(),
            beta,
            soft_max,
        )),
        Formula::Historically(interval, f) => Ok(soft_window(
            &soft_eval(f, times, signals, beta)?,
            times,
            -interval.upper_or_infinity(),
            -interval.lower,
            beta,
            soft_min,
        )),
        Formula::Once(interval, f) => Ok(soft_window(
            &soft_eval(f, times, signals, beta)?,
            times,
            -interval.upper_or_infinity(),
            -interval.lower,
            beta,
            soft_max,
        )),
        Formula::Probabilistic(..) => Err(Error::ProbabilisticOperator),
        _ => Err(Error::Unsupported {
            feature: "until, since, and next in smooth robustness",
        }),
    }
}

fn soft_window(
    child: &[f64],
    times: &[f64],
    off_a: f64,
    off_b: f64,
    beta: f64,
    reduce: fn(&[f64], f64) -> f64,
) -> Vec<f64> {
    times
        .iter()
        .map(|&t| {
            let (lo, hi) = (t + off_a, t + off_b);
            let window: Vec<f64> = child
                .iter()
                .zip(times)
                .filter(|(_, &tj)| tj >= lo && tj <= hi)
                .map(|(&v, _)| v)
                .collect();
            reduce(&window, beta)
        })
        .collect()
}

fn soft_combine(
    left: &Formula,
    right: &Formula,
    times: &[f64],
    signals: &BTreeMap<String, Vec<f64>>,
    beta: f64,
    op: impl Fn(f64, f64) -> f64,
) -> Result<Vec<f64>> {
    let left = soft_eval(left, times, signals, beta)?;
    let right = soft_eval(right, times, signals, beta)?;
    Ok(left.iter().zip(&right).map(|(&x, &y)| op(x, y)).collect())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the asserted bounds and conventions are exact"
    )]

    use super::*;

    #[test]
    fn soft_operators_bound_the_exact_ones() {
        let values = [1.0, -2.0, 3.5, 0.25];
        assert!(soft_min(&values, 5.0) <= -2.0);
        assert!(soft_max(&values, 5.0) >= 3.5);
    }

    #[test]
    fn higher_temperature_approaches_exact_min_and_max() {
        let values = [1.0, -2.0, 3.5, 0.25];
        assert!((soft_min(&values, 200.0) - (-2.0)).abs() < 0.05);
        assert!((soft_max(&values, 200.0) - 3.5).abs() < 0.05);
    }

    #[test]
    fn an_empty_slice_matches_the_exact_conventions() {
        assert_eq!(soft_min(&[], 1.0), f64::INFINITY);
        assert_eq!(soft_max(&[], 1.0), f64::NEG_INFINITY);
    }

    #[test]
    fn config_rejects_a_non_positive_temperature() {
        assert!(SmoothConfig::new(0.0).is_err());
        assert!(SmoothConfig::new(-1.0).is_err());
        assert!(SmoothConfig::new(f64::NAN).is_err());
        assert_eq!(SmoothConfig::new(4.0).unwrap().temperature(), 4.0);
    }

    #[test]
    fn smooth_robustness_approaches_the_exact_value() {
        let phi = Formula::parse("(x > 0) and (y > 1)").unwrap();
        let mut trace = Trace::new([0.0]).unwrap();
        trace.add_signal("x", [3.0]).unwrap();
        trace.add_signal("y", [4.0]).unwrap();
        let exact = phi.robustness(&trace).unwrap();
        let smooth = phi
            .smooth_robustness(&trace, SmoothConfig::new(200.0).unwrap())
            .unwrap();
        assert!((smooth - exact).abs() < 0.05);
        assert!(smooth <= exact + 1e-9);
    }

    #[test]
    fn smooth_temporal_approaches_the_exact_value() {
        let phi = Formula::parse("always[0, 2](x > 0)").unwrap();
        let mut trace = Trace::new([0.0, 1.0, 2.0]).unwrap();
        trace.add_signal("x", [3.0, 1.0, 2.0]).unwrap();
        let exact = phi.robustness(&trace).unwrap();
        let smooth = phi
            .smooth_robustness(&trace, SmoothConfig::new(200.0).unwrap())
            .unwrap();
        assert!((smooth - exact).abs() < 0.1);
        assert!(smooth <= exact + 1e-9);
    }
}