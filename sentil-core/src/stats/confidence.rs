//! Confidence intervals for an estimated satisfaction probability.

#![allow(
    clippy::cast_precision_loss,
    reason = "sample counts are far below 2^53, so the count-to-float casts are exact"
)]

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