//! Smooth, differentiable robustness operators for synthesis.
//!
//! Monitoring uses exact min and max, which are not differentiable at ties.
//! Synthesis instead needs a robustness that varies smoothly with the trace so an
//! optimizer can follow its gradient. Two smoothings are offered. The default is a
//! log-sum-exp soft minimum and maximum controlled by a temperature: as the
//! temperature rises they approach the exact operators, and at every temperature
//! the soft minimum stays at or below the true minimum and the soft maximum at or
//! above the true maximum. The alternative is an arithmetic-geometric mean
//! robustness, which is parameter-free and shares its sign with the exact value.

#![allow(
    clippy::cast_precision_loss,
    reason = "operand and window sizes stay far below 2^53, so the length cast is exact"
)]

use std::collections::BTreeMap;

use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::semantics::{eval_predicate, WINDOW_EPSILON};
use crate::signal::Trace;

const EPS: f64 = 1e-12;

/// Which smoothing the soft robustness uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SoftKind {
    /// Log-sum-exp soft min and max at the configuration's temperature.
    #[default]
    LogSumExp,
    /// Arithmetic-geometric mean robustness: the geometric mean of the satisfied
    /// margins, or the arithmetic mean of the violated ones. Parameter-free, so it
    /// ignores the temperature.
    ArithmeticGeometricMean,
}

/// The temperature and smoothing kind the smooth robustness uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmoothConfig {
    temperature: f64,
    kind: SoftKind,
}

impl SmoothConfig {
    /// Builds a log-sum-exp configuration at the given temperature.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if `temperature` is not finite and
    /// positive.
    pub fn new(temperature: f64) -> Result<Self> {
        if temperature.is_finite() && temperature > 0.0 {
            Ok(Self {
                temperature,
                kind: SoftKind::LogSumExp,
            })
        } else {
            Err(Error::InvalidConfig {
                context: "smooth robustness",
                message: format!("temperature must be finite and positive, got {temperature}"),
            })
        }
    }

    /// Selects the smoothing kind.
    #[must_use]
    pub fn with_kind(mut self, kind: SoftKind) -> Self {
        self.kind = kind;
        self
    }

    /// The temperature.
    #[must_use]
    pub fn temperature(&self) -> f64 {
        self.temperature
    }

    /// The smoothing kind.
    #[must_use]
    pub fn kind(&self) -> SoftKind {
        self.kind
    }
}

impl Default for SmoothConfig {
    fn default() -> Self {
        Self {
            temperature: 10.0,
            kind: SoftKind::LogSumExp,
        }
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

fn soft_min2(acc: f64, x: f64, beta: f64) -> f64 {
    let m = acc.min(x);
    if !m.is_finite() {
        return m;
    }
    m - ((-beta * (acc - m)).exp() + (-beta * (x - m)).exp()).ln() / beta
}

/// Arithmetic-geometric mean soft minimum. When every margin is positive it is the
/// geometric mean of the margins shifted by one, which recovers the exact value
/// when the margins are equal; otherwise it averages the violations, so it shares
/// its sign with the true minimum. An empty slice has minimum positive infinity.
fn agm_min(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::INFINITY;
    }
    let count = values.len() as f64;
    if values.iter().all(|&r| r > 0.0) {
        let log_mean = values.iter().map(|&r| (1.0 + r).ln()).sum::<f64>() / count;
        log_mean.exp() - 1.0
    } else {
        values.iter().filter(|&&r| r <= 0.0).sum::<f64>() / count
    }
}

/// Arithmetic-geometric mean soft maximum, the dual of [`agm_min`]. An empty slice
/// has maximum negative infinity.
fn agm_max(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::NEG_INFINITY;
    }
    let count = values.len() as f64;
    if values.iter().all(|&r| r < 0.0) {
        let log_mean = values.iter().map(|&r| (1.0 - r).ln()).sum::<f64>() / count;
        1.0 - log_mean.exp()
    } else {
        values.iter().filter(|&&r| r >= 0.0).sum::<f64>() / count
    }
}

fn reduce_min(values: &[f64], config: SmoothConfig) -> f64 {
    match config.kind {
        SoftKind::LogSumExp => soft_min(values, config.temperature),
        SoftKind::ArithmeticGeometricMean => agm_min(values),
    }
}

fn reduce_max(values: &[f64], config: SmoothConfig) -> f64 {
    match config.kind {
        SoftKind::LogSumExp => soft_max(values, config.temperature),
        SoftKind::ArithmeticGeometricMean => agm_max(values),
    }
}

impl Formula {
    /// The smooth robustness of the formula over a trace: a differentiable
    /// surrogate for [`robustness`](Self::robustness) that synthesis can climb.
    ///
    /// It mirrors the exact robustness but with the soft minimum and maximum from
    /// `config`, so the result varies smoothly with the trace.
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
        let signal = soft_eval(self, trace.times(), trace.signals(), config)?;
        Ok(signal[0])
    }
}

fn soft_eval(
    formula: &Formula,
    times: &[f64],
    signals: &BTreeMap<String, Vec<f64>>,
    config: SmoothConfig,
) -> Result<Vec<f64>> {
    match formula {
        Formula::Predicate(p) => (0..times.len())
            .map(|i| {
                let lookup = |name: &str| signals.get(name).and_then(|col| col.get(i)).copied();
                eval_predicate(p, &lookup)
            })
            .collect(),
        Formula::Not(f) => Ok(soft_eval(f, times, signals, config)?
            .into_iter()
            .map(|x| -x)
            .collect()),
        Formula::And(l, r) => soft_combine(l, r, times, signals, config, |x, y| {
            reduce_min(&[x, y], config)
        }),
        Formula::Or(l, r) => soft_combine(l, r, times, signals, config, |x, y| {
            reduce_max(&[x, y], config)
        }),
        Formula::Implies(l, r) => soft_combine(l, r, times, signals, config, |x, y| {
            reduce_max(&[-x, y], config)
        }),
        Formula::Always(interval, f) => Ok(soft_window(
            &soft_eval(f, times, signals, config)?,
            times,
            interval.lower,
            interval.upper_or_infinity(),
            config,
            reduce_min,
        )),
        Formula::Eventually(interval, f) => Ok(soft_window(
            &soft_eval(f, times, signals, config)?,
            times,
            interval.lower,
            interval.upper_or_infinity(),
            config,
            reduce_max,
        )),
        Formula::Historically(interval, f) => Ok(soft_window(
            &soft_eval(f, times, signals, config)?,
            times,
            -interval.upper_or_infinity(),
            -interval.lower,
            config,
            reduce_min,
        )),
        Formula::Once(interval, f) => Ok(soft_window(
            &soft_eval(f, times, signals, config)?,
            times,
            -interval.upper_or_infinity(),
            -interval.lower,
            config,
            reduce_max,
        )),
        Formula::Until(interval, l, r) => Ok(soft_until(
            &soft_eval(l, times, signals, config)?,
            &soft_eval(r, times, signals, config)?,
            times,
            interval.lower,
            interval.upper_or_infinity(),
            config,
        )),
        Formula::Since(interval, l, r) => Ok(soft_since(
            &soft_eval(l, times, signals, config)?,
            &soft_eval(r, times, signals, config)?,
            times,
            interval.lower,
            interval.upper_or_infinity(),
            config,
        )),
        Formula::Next(f) => Ok(soft_next(&soft_eval(f, times, signals, config)?)),
        Formula::Probabilistic(..) => Err(Error::ProbabilisticOperator),
    }
}

fn soft_until(
    phi: &[f64],
    psi: &[f64],
    times: &[f64],
    a: f64,
    b: f64,
    config: SmoothConfig,
) -> Vec<f64> {
    let n = phi.len();
    let mut result = vec![f64::NEG_INFINITY; n];
    let lse = matches!(config.kind, SoftKind::LogSumExp);
    let mut candidates = Vec::new();
    let mut prefix = Vec::new();
    for i in 0..n {
        let window_start = times[i] + a;
        let window_end = if b.is_infinite() {
            f64::INFINITY
        } else {
            times[i] + b
        };
        if window_start > times[n - 1] + EPS {
            continue;
        }
        let first = times.partition_point(|&t| t < window_start - EPS);
        candidates.clear();
        prefix.clear();
        let mut running = f64::INFINITY;
        for j in i..n {
            if j > i {
                if lse {
                    running = soft_min2(running, phi[j - 1], config.temperature);
                } else {
                    prefix.push(phi[j - 1]);
                }
            }
            if times[j] > window_end + EPS {
                break;
            }
            if j >= first {
                let phi_min = if lse { running } else { agm_min(&prefix) };
                candidates.push(reduce_min(&[psi[j], phi_min], config));
            }
        }
        result[i] = reduce_max(&candidates, config);
    }
    result
}

fn soft_since(
    phi: &[f64],
    psi: &[f64],
    times: &[f64],
    a: f64,
    b: f64,
    config: SmoothConfig,
) -> Vec<f64> {
    let n = phi.len();
    let mut result = vec![f64::NEG_INFINITY; n];
    let lse = matches!(config.kind, SoftKind::LogSumExp);
    let mut candidates = Vec::new();
    let mut prefix = Vec::new();
    for i in 0..n {
        let window_end = times[i] - a;
        let window_start = if b.is_infinite() {
            0.0
        } else {
            (times[i] - b).max(0.0)
        };
        let last = times.partition_point(|&t| t <= window_end + EPS);
        candidates.clear();
        prefix.clear();
        let mut running = f64::INFINITY;
        for j in (0..=i).rev() {
            if j < i {
                if lse {
                    running = soft_min2(running, phi[j + 1], config.temperature);
                } else {
                    prefix.push(phi[j + 1]);
                }
            }
            if times[j] < window_start - EPS {
                break;
            }
            if j < last {
                let phi_min = if lse { running } else { agm_min(&prefix) };
                candidates.push(reduce_min(&[psi[j], phi_min], config));
            }
        }
        result[i] = reduce_max(&candidates, config);
    }
    result
}

fn soft_next(inner: &[f64]) -> Vec<f64> {
    let n = inner.len();
    let mut result = vec![f64::NEG_INFINITY; n];
    if n > 1 {
        result[..n - 1].copy_from_slice(&inner[1..]);
    }
    result
}

fn soft_window(
    child: &[f64],
    times: &[f64],
    off_a: f64,
    off_b: f64,
    config: SmoothConfig,
    reduce: fn(&[f64], SmoothConfig) -> f64,
) -> Vec<f64> {
    let mut scratch = Vec::new();
    times
        .iter()
        .map(|&t| {
            let (lo, hi) = (t + off_a, t + off_b);
            let start = times.partition_point(|&tj| tj < lo - WINDOW_EPSILON);
            scratch.clear();
            for (&tj, &v) in times[start..].iter().zip(&child[start..]) {
                if tj > hi + WINDOW_EPSILON {
                    break;
                }
                scratch.push(v);
            }
            reduce(&scratch, config)
        })
        .collect()
}

fn soft_combine(
    left: &Formula,
    right: &Formula,
    times: &[f64],
    signals: &BTreeMap<String, Vec<f64>>,
    config: SmoothConfig,
    op: impl Fn(f64, f64) -> f64,
) -> Result<Vec<f64>> {
    let left = soft_eval(left, times, signals, config)?;
    let right = soft_eval(right, times, signals, config)?;
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
        assert_eq!(agm_min(&[]), f64::INFINITY);
        assert_eq!(agm_max(&[]), f64::NEG_INFINITY);
    }

    #[test]
    fn the_agm_recovers_equal_margins_and_keeps_the_sign() {
        assert!((agm_min(&[3.0, 3.0]) - 3.0).abs() < 1e-12);
        assert!((agm_max(&[-3.0, -3.0]) + 3.0).abs() < 1e-12);
        assert!(agm_min(&[5.0, -1.0]) < 0.0);
        assert!(agm_max(&[-5.0, 1.0]) > 0.0);
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
    fn the_agm_smoothing_keeps_the_verdict_sign() {
        let phi = Formula::parse("(x > 0) and (y > 0)").unwrap();
        let agm = SmoothConfig::default().with_kind(SoftKind::ArithmeticGeometricMean);

        let mut holds = Trace::new([0.0]).unwrap();
        holds.add_signal("x", [4.0]).unwrap();
        holds.add_signal("y", [4.0]).unwrap();
        assert!((phi.smooth_robustness(&holds, agm).unwrap() - 4.0).abs() < 1e-9);

        let mut fails = Trace::new([0.0]).unwrap();
        fails.add_signal("x", [4.0]).unwrap();
        fails.add_signal("y", [-2.0]).unwrap();
        assert!(phi.smooth_robustness(&fails, agm).unwrap() < 0.0);
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

    #[test]
    fn a_bounded_window_keeps_its_edge_sample_on_an_accumulated_grid() {
        let mut times = Vec::new();
        let mut t = 0.0;
        for _ in 0..8 {
            times.push(t);
            t += 0.1;
        }
        assert!(times[3] > 0.3);

        let phi = Formula::parse("always[0, 0.3](x > 0)").unwrap();
        let mut trace = Trace::new(times).unwrap();
        trace
            .add_signal("x", [3.0, 3.0, 3.0, 1.0, 3.0, 3.0, 3.0, 3.0])
            .unwrap();
        let exact = phi.robustness_signal(&trace).unwrap()[0];
        assert_eq!(exact, 1.0);

        for beta in [20.0, 200.0, 2000.0] {
            let smooth = phi
                .smooth_robustness(&trace, SmoothConfig::new(beta).unwrap())
                .unwrap();
            assert!(smooth <= exact + 1e-9, "beta {beta}: {smooth} over {exact}");
            assert!(
                smooth >= exact - 4.0_f64.ln() / beta,
                "beta {beta}: {smooth} under {exact}"
            );
        }
    }

    #[test]
    fn smooth_until_approaches_the_exact_value() {
        let phi = Formula::parse("(x > 0) until[0, 2] (y > 0)").unwrap();
        let mut trace = Trace::new([0.0, 1.0, 2.0]).unwrap();
        trace.add_signal("x", [3.0, 3.0, 3.0]).unwrap();
        trace.add_signal("y", [-1.0, 1.0, 2.0]).unwrap();
        let exact = phi.robustness(&trace).unwrap();
        let smooth = phi
            .smooth_robustness(&trace, SmoothConfig::new(200.0).unwrap())
            .unwrap();
        assert!((smooth - exact).abs() < 0.1);
    }

    #[test]
    fn smooth_until_with_a_long_prefix_tracks_the_exact_value() {
        let phi = Formula::parse("(x > 0) until[0, 5] (y > 0)").unwrap();
        let mut trace = Trace::new([0.0, 1.0, 2.0, 3.0, 4.0, 5.0]).unwrap();
        trace.add_signal("x", [3.0, 1.0, 2.0, 0.5, 4.0, 1.0]).unwrap();
        trace.add_signal("y", [-2.0, -1.0, -3.0, 2.0, -1.0, 1.0]).unwrap();
        let exact = phi.robustness(&trace).unwrap();
        let smooth = phi
            .smooth_robustness(&trace, SmoothConfig::new(200.0).unwrap())
            .unwrap();
        assert!((smooth - exact).abs() < 0.05, "smooth {smooth} exact {exact}");
    }

    #[test]
    fn smooth_next_shifts_one_step() {
        let phi = Formula::parse("next(x > 0)").unwrap();
        let mut trace = Trace::new([0.0, 1.0]).unwrap();
        trace.add_signal("x", [5.0, -3.0]).unwrap();
        assert_eq!(
            phi.smooth_robustness(&trace, SmoothConfig::default())
                .unwrap(),
            -3.0
        );
    }

    #[test]
    fn running_lse_fold_matches_the_whole_set_soft_reduction() {
        let soft_max2 = |acc: f64, x: f64, beta: f64| -> f64 {
            let m = acc.max(x);
            if !m.is_finite() {
                return m;
            }
            m + ((beta * (acc - m)).exp() + (beta * (x - m)).exp()).ln() / beta
        };
        let windows: &[&[f64]] = &[
            &[1.0],
            &[3.0, -1.0, 2.0, 0.5],
            &[-5.0, -5.0, -5.0],
            &[48.0, 50.0, 49.0, 52.0],
            &[0.0, -0.2, 0.3, -0.1, 0.05, 0.4],
        ];
        for &beta in &[2.0, 10.0, 50.0] {
            for w in windows {
                let lo = w
                    .iter()
                    .fold(f64::INFINITY, |acc, &x| soft_min2(acc, x, beta));
                let hi = w
                    .iter()
                    .fold(f64::NEG_INFINITY, |acc, &x| soft_max2(acc, x, beta));
                assert!(
                    (lo - soft_min(w, beta)).abs() < 1e-9,
                    "min fold at beta {beta} on {w:?}: {lo} vs {}",
                    soft_min(w, beta)
                );
                assert!(
                    (hi - soft_max(w, beta)).abs() < 1e-9,
                    "max fold at beta {beta} on {w:?}: {hi} vs {}",
                    soft_max(w, beta)
                );
                assert!(lo.is_finite() && hi.is_finite());
            }
        }
    }
}