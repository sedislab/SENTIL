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

/// The two-sided z critical value for a confidence level, for example `1.95996`
/// at `0.95`.
#[must_use]
pub fn z_score(level: f64) -> f64 {
    normal_quantile(0.5 * (1.0 + level))
}

/// The inverse of the standard normal cumulative distribution, by Acklam's
/// rational approximation. Accurate to a few parts in `1e9` across `(0, 1)`,
/// which is well past what a confidence bound needs.
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
    if p < LOW {
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
    }
}