//! Confidence intervals for an estimated satisfaction probability.

#![allow(
    clippy::cast_precision_loss,
    reason = "sample counts are far below 2^53, so the count-to-float casts are exact"
)]

use crate::error::{Error, Result};

/// An interval that contains the true probability at the stated confidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfidenceInterval {
    /// The lower bound, in `[0, 1]`.
    pub lower: f64,
    /// The upper bound, in `[0, 1]`.
    pub upper: f64,
    /// The confidence level the interval was built for, such as `0.95`.
    pub level: f64,
}

impl ConfidenceInterval {
    /// The width of the interval.
    pub fn width(&self) -> f64 {
        self.upper - self.lower
    }

    /// Whether a probability lies inside the interval.
    pub fn contains(&self, p: f64) -> bool {
        p >= self.lower && p <= self.upper
    }
}

fn whole_range(level: f64) -> ConfidenceInterval {
    ConfidenceInterval {
        lower: 0.0,
        upper: 1.0,
        level,
    }
}

/// Which interval a statistical check reports around its estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IntervalMethod {
    /// The Wilson score interval.
    #[default]
    Wilson,
    /// The Clopper-Pearson exact interval.
    ClopperPearson,
    /// The Jeffreys credible interval.
    Jeffreys,
    /// The Agresti-Coull interval.
    AgrestiCoull,
}

impl IntervalMethod {
    /// The interval this method gives for `successes` out of `trials` at `level`.
    #[must_use]
    pub fn interval(self, successes: u64, trials: u64, level: f64) -> ConfidenceInterval {
        match self {
            IntervalMethod::Wilson => wilson_interval(successes, trials, level),
            IntervalMethod::ClopperPearson => clopper_pearson(successes, trials, level),
            IntervalMethod::Jeffreys => jeffreys_interval(successes, trials, level),
            IntervalMethod::AgrestiCoull => agresti_coull(successes, trials, level),
        }
    }
}

/// The Wilson score interval for `successes` out of `trials` at a confidence level.
///
/// ```
/// use sentil::stats::wilson_interval;
///
/// let ci = wilson_interval(50, 100, 0.95);
/// assert!((ci.lower - 0.403_831).abs() < 1e-6);
/// assert!((ci.upper - 0.596_169).abs() < 1e-6);
/// ```
#[must_use]
pub fn wilson_interval(successes: u64, trials: u64, level: f64) -> ConfidenceInterval {
    if trials == 0 || !(0.0 < level && level < 1.0) {
        return whole_range(level);
    }
    let successes = successes.min(trials);
    let n = trials as f64;
    let p_hat = successes as f64 / n;
    let z = z_score(level);
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = (p_hat + z2 / (2.0 * n)) / denominator;
    let half_width = (z / denominator) * (p_hat * (1.0 - p_hat) / n + z2 / (4.0 * n * n)).sqrt();
    ConfidenceInterval {
        lower: (center - half_width).max(0.0),
        upper: (center + half_width).min(1.0),
        level,
    }
}

/// The two-sided z critical value for a confidence level, for example `1.95996`
/// at `0.95`.
#[must_use]
pub fn z_score(level: f64) -> f64 {
    normal_quantile(0.5 * (1.0 + level))
}

/// How many samples a fixed-sample estimate needs to land within `epsilon` of the
/// true probability with confidence `1 - delta`, by the Chernoff-Hoeffding bound
/// `ceil(ln(2 / delta) / (2 epsilon^2))`.
///
/// ```
/// use sentil::stats::chernoff_hoeffding_samples;
///
/// assert_eq!(chernoff_hoeffding_samples(0.1, 0.05)?, 185);
/// # Ok::<(), sentil::Error>(())
/// ```
///
/// # Errors
///
/// Returns [`Error::InvalidConfig`] if `epsilon` is not positive or `delta` is not in `(0, 1)`.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    reason = "the count is positive and finite, and clamped to u64::MAX"
)]
pub fn chernoff_hoeffding_samples(epsilon: f64, delta: f64) -> Result<u64> {
    if !epsilon.is_finite() || epsilon <= 0.0 {
        return Err(Error::InvalidConfig {
            context: "Chernoff-Hoeffding",
            message: format!("epsilon must be finite and positive, got {epsilon}"),
        });
    }
    if !delta.is_finite() || delta <= 0.0 || delta >= 1.0 {
        return Err(Error::InvalidConfig {
            context: "Chernoff-Hoeffding",
            message: format!("delta must be in (0, 1), got {delta}"),
        });
    }
    let n = ((2.0 / delta).ln() / (2.0 * epsilon * epsilon)).ceil();
    Ok(if n >= u64::MAX as f64 {
        u64::MAX
    } else {
        n as u64
    })
}

/// How many samples bound the Wilson (or normal) half-width by `epsilon` at the
/// given confidence `level`, by the worst-case `ceil(z^2 / (4 epsilon^2))` at
/// `p = 1/2`.
///
/// ```
/// use sentil::stats::wilson_samples;
///
/// assert_eq!(wilson_samples(0.01, 0.95)?, 9604);
/// # Ok::<(), sentil::Error>(())
/// ```
///
/// # Errors
///
/// Returns [`Error::InvalidConfig`] if `epsilon` is not positive or `level` is not in `(0, 1)`.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    reason = "the count is positive and finite, and clamped to u64::MAX"
)]
pub fn wilson_samples(epsilon: f64, level: f64) -> Result<u64> {
    if !epsilon.is_finite() || epsilon <= 0.0 {
        return Err(Error::InvalidConfig {
            context: "Wilson sample sizing",
            message: format!("epsilon must be finite and positive, got {epsilon}"),
        });
    }
    if !level.is_finite() || level <= 0.0 || level >= 1.0 {
        return Err(Error::InvalidConfig {
            context: "Wilson sample sizing",
            message: format!("level must be in (0, 1), got {level}"),
        });
    }
    let z = z_score(level);
    let n = (z * z / (4.0 * epsilon * epsilon)).ceil();
    Ok(if n >= u64::MAX as f64 {
        u64::MAX
    } else {
        n as u64
    })
}

/// Inverse standard normal CDF by Acklam's approximation with a Halley step.
fn normal_quantile(p: f64) -> f64 {
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239e0,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838e0,
        -2.549_732_539_343_734e0,
        4.374_664_141_464_968e0,
        2.938_163_982_698_783e0,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996e0,
        3.754_408_661_907_416e0,
    ];
    const LOW: f64 = 0.024_25;
    const HIGH: f64 = 1.0 - LOW;

    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    let x = if p < LOW {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= HIGH {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };

    let e = 0.5 * libm::erfc(-x * core::f64::consts::FRAC_1_SQRT_2) - p;
    let u = e * (2.0 * core::f64::consts::PI).sqrt() * (0.5 * x * x).exp();
    x - u / (1.0 + 0.5 * x * u)
}

/// The Clopper-Pearson exact interval for `successes` out of `trials`.
///
/// ```
/// use sentil::stats::clopper_pearson;
///
/// let ci = clopper_pearson(50, 100, 0.95);
/// assert!((ci.lower - 0.398_321).abs() < 1e-5);
/// assert!((ci.upper - 0.601_679).abs() < 1e-5);
/// ```
#[must_use]
pub fn clopper_pearson(successes: u64, trials: u64, level: f64) -> ConfidenceInterval {
    if trials == 0 || !(0.0 < level && level < 1.0) {
        return whole_range(level);
    }
    let successes = successes.min(trials);
    let k = successes as f64;
    let n = trials as f64;
    let alpha = 1.0 - level;
    let lower = if successes == 0 {
        0.0
    } else {
        beta_quantile(alpha / 2.0, k, n - k + 1.0)
    };
    let upper = if successes == trials {
        1.0
    } else {
        beta_quantile(1.0 - alpha / 2.0, k + 1.0, n - k)
    };
    ConfidenceInterval {
        lower,
        upper,
        level,
    }
}

/// The Jeffreys credible interval from a Beta(1/2, 1/2) prior.
///
/// ```
/// use sentil::stats::jeffreys_interval;
///
/// let ci = jeffreys_interval(8, 10, 0.95);
/// assert!(ci.lower < 0.8 && 0.8 < ci.upper);
/// ```
#[must_use]
pub fn jeffreys_interval(successes: u64, trials: u64, level: f64) -> ConfidenceInterval {
    if trials == 0 || !(0.0 < level && level < 1.0) {
        return whole_range(level);
    }
    let successes = successes.min(trials);
    let k = successes as f64;
    let n = trials as f64;
    let alpha = 1.0 - level;
    let a = k + 0.5;
    let b = n - k + 0.5;
    let lower = if successes == 0 {
        0.0
    } else {
        beta_quantile(alpha / 2.0, a, b)
    };
    let upper = if successes == trials {
        1.0
    } else {
        beta_quantile(1.0 - alpha / 2.0, a, b)
    };
    ConfidenceInterval {
        lower,
        upper,
        level,
    }
}

/// The Agresti-Coull interval for `successes` out of `trials`.
///
/// ```
/// use sentil::stats::agresti_coull;
///
/// let ci = agresti_coull(50, 100, 0.95);
/// assert!((ci.lower - 0.403_831).abs() < 1e-6);
/// assert!((ci.upper - 0.596_169).abs() < 1e-6);
/// ```
#[must_use]
pub fn agresti_coull(successes: u64, trials: u64, level: f64) -> ConfidenceInterval {
    if trials == 0 || !(0.0 < level && level < 1.0) {
        return whole_range(level);
    }
    let successes = successes.min(trials);
    let n = trials as f64;
    let k = successes as f64;
    let z = z_score(level);
    let z2 = z * z;
    let n_tilde = n + z2;
    let p_tilde = (k + z2 / 2.0) / n_tilde;
    let half_width = z * (p_tilde * (1.0 - p_tilde) / n_tilde).sqrt();
    ConfidenceInterval {
        lower: (p_tilde - half_width).max(0.0),
        upper: (p_tilde + half_width).min(1.0),
        level,
    }
}

fn beta_quantile(p: f64, a: f64, b: f64) -> f64 {
    if p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for _ in 0..100 {
        let mid = f64::midpoint(lo, hi);
        if regularized_incomplete_beta(a, b, mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    f64::midpoint(lo, hi)
}

pub(crate) fn regularized_incomplete_beta(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let front =
        (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_continued_fraction(a, b, x) / a
    } else {
        1.0 - front * beta_continued_fraction(b, a, 1.0 - x) / b
    }
}

fn nonzero(v: f64) -> f64 {
    const TINY: f64 = 1e-300;
    if v.abs() < TINY {
        TINY
    } else {
        v
    }
}

fn lentz_step(num: f64, c: &mut f64, d: &mut f64) -> f64 {
    *d = 1.0 / nonzero(1.0 + num * *d);
    *c = nonzero(1.0 + num / *c);
    *d * *c
}

#[allow(
    clippy::many_single_char_names,
    reason = "a, b, x, c, d, h are the standard names for Lentz's continued fraction"
)]
fn beta_continued_fraction(a: f64, b: f64, x: f64) -> f64 {
    let mut c = 1.0;
    let mut d = 1.0 / nonzero(1.0 - (a + b) * x / (a + 1.0));
    let mut h = d;
    for m in 1..200 {
        let m = f64::from(m);
        let even = m * (b - m) * x / ((a + 2.0 * m - 1.0) * (a + 2.0 * m));
        h *= lentz_step(even, &mut c, &mut d);
        let odd = -(a + m) * (a + b + m) * x / ((a + 2.0 * m) * (a + 2.0 * m + 1.0));
        let delta = lentz_step(odd, &mut c, &mut d);
        h *= delta;
        if (delta - 1.0).abs() < 1e-14 {
            break;
        }
    }
    h
}

/// Natural log of the gamma function by the Lanczos approximation.
fn ln_gamma(x: f64) -> f64 {
    const C: [f64; 8] = [
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    let x = x - 1.0;
    let mut a = 0.999_999_999_999_809_9;
    let t = x + 7.5;
    for (i, &c) in C.iter().enumerate() {
        a += c / (x + i as f64 + 1.0);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the clamped bounds 0.0 and 1.0 are produced exactly"
    )]

    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn z_scores_match_the_standard_values() {
        assert!((z_score(0.90) - 1.644_853_626_951_472).abs() < 1e-10);
        assert!((z_score(0.95) - 1.959_963_984_540_054).abs() < 1e-10);
        assert!((z_score(0.99) - 2.575_829_303_548_900).abs() < 1e-10);
    }

    #[test]
    fn wilson_matches_known_intervals() {
        let half = wilson_interval(50, 100, 0.95);
        assert!(close(half.lower, 0.403_831) && close(half.upper, 0.596_169));

        let rare = wilson_interval(5, 100, 0.95);
        assert!(close(rare.lower, 0.021_544) && close(rare.upper, 0.111_750));
    }

    #[test]
    fn wilson_stays_in_range_at_the_extremes() {
        let none = wilson_interval(0, 100, 0.95);
        assert_eq!(none.lower, 0.0);
        assert!(none.upper > 0.0 && none.upper < 0.05);

        let all = wilson_interval(100, 100, 0.95);
        assert_eq!(all.upper, 1.0);
        assert!(all.lower > 0.95 && all.lower < 1.0);
    }

    #[test]
    fn no_trials_gives_the_whole_range() {
        let ci = wilson_interval(0, 0, 0.95);
        assert_eq!((ci.lower, ci.upper), (0.0, 1.0));
        assert!(ci.contains(0.5));
        assert!(close(ci.width(), 1.0));
    }

    fn close3(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn clopper_pearson_matches_known_intervals() {
        // Values from R's binom.test at 95%.
        let half = clopper_pearson(50, 100, 0.95);
        assert!((half.lower - 0.398_321).abs() < 1e-5 && (half.upper - 0.601_679).abs() < 1e-5);
        let rare = clopper_pearson(5, 100, 0.95);
        assert!(close3(rare.lower, 0.016_43) && close3(rare.upper, 0.112_80));
    }

    #[test]
    fn clopper_pearson_is_at_least_as_wide_as_wilson() {
        let cp = clopper_pearson(20, 100, 0.95);
        let w = wilson_interval(20, 100, 0.95);
        assert!(cp.lower <= w.lower && cp.upper >= w.upper);
    }

    #[test]
    fn jeffreys_is_tighter_than_clopper_pearson() {
        let j = jeffreys_interval(20, 100, 0.95);
        let cp = clopper_pearson(20, 100, 0.95);
        assert!(j.lower >= cp.lower && j.upper <= cp.upper);
        assert!(j.lower < 0.2 && 0.2 < j.upper);
    }

    #[test]
    fn the_method_selector_dispatches_to_each_interval() {
        let (s, t, l) = (30, 100, 0.95);
        assert_eq!(IntervalMethod::Wilson.interval(s, t, l), wilson_interval(s, t, l));
        assert_eq!(
            IntervalMethod::ClopperPearson.interval(s, t, l),
            clopper_pearson(s, t, l)
        );
        assert_eq!(IntervalMethod::Jeffreys.interval(s, t, l), jeffreys_interval(s, t, l));
        assert_eq!(IntervalMethod::AgrestiCoull.interval(s, t, l), agresti_coull(s, t, l));
        assert_eq!(IntervalMethod::default(), IntervalMethod::Wilson);
    }

    #[test]
    fn agresti_coull_matches_known_intervals() {
        let half = agresti_coull(50, 100, 0.95);
        assert!(close(half.lower, 0.403_831) && close(half.upper, 0.596_169));
        let rare = agresti_coull(5, 100, 0.95);
        assert!(close(rare.lower, 0.018_676_36) && close(rare.upper, 0.114_617_79));
    }

    #[test]
    fn chernoff_hoeffding_sizes_the_sample_count() {
        assert_eq!(chernoff_hoeffding_samples(0.1, 0.05).unwrap(), 185);
        assert!(chernoff_hoeffding_samples(0.01, 0.01).unwrap() > 26_000);
        assert!(chernoff_hoeffding_samples(0.0, 0.05).is_err());
        assert!(chernoff_hoeffding_samples(0.1, 1.0).is_err());
        assert!(chernoff_hoeffding_samples(f64::NAN, 0.05).is_err());
    }

    #[test]
    fn wilson_sizing_is_about_half_the_distribution_free_count() {
        assert_eq!(wilson_samples(0.01, 0.95).unwrap(), 9604);
        assert_eq!(chernoff_hoeffding_samples(0.01, 0.05).unwrap(), 18445);
        assert!(wilson_samples(-1.0, 0.95).is_err());
        assert!(wilson_samples(0.01, 0.0).is_err());
    }

    #[test]
    fn clopper_pearson_handles_the_extremes() {
        let none = clopper_pearson(0, 10, 0.95);
        assert_eq!(none.lower, 0.0);
        assert!(close3(none.upper, 0.308_5));

        let all = clopper_pearson(100, 100, 0.95);
        assert_eq!(all.upper, 1.0);

        let empty = clopper_pearson(0, 0, 0.95);
        assert_eq!((empty.lower, empty.upper), (0.0, 1.0));
    }
}