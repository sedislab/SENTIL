//! Reading a signal between its samples.

#[cfg(not(feature = "std"))]
use crate::prelude::*;

/// How a signal is read between its samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// A straight line between neighbouring samples.
    #[default]
    Linear,
    /// The previous sample held constant until the next one.
    ZeroOrderHold,
    /// A natural cubic spline through the samples.
    CubicSpline,
}

pub(crate) fn read_at(times: &[f64], values: &[f64], interp: Interpolation, at: &[f64]) -> Vec<f64> {
    let segments = (interp == Interpolation::CubicSpline && times.len() >= 2)
        .then(|| cubic_segments(times, values));
    read_with_segments(times, values, segments.as_deref(), interp, at)
}

pub(crate) fn read_with_segments(
    times: &[f64],
    values: &[f64],
    segments: Option<&[[f64; 3]]>,
    interp: Interpolation,
    at: &[f64],
) -> Vec<f64> {
    at.iter()
        .map(|&t| read_one(times, values, segments, interp, t))
        .collect()
}

#[allow(
    clippy::many_single_char_names,
    reason = "i, t and the spline coefficients b, c, d read clearest short"
)]
fn read_one(
    times: &[f64],
    values: &[f64],
    segments: Option<&[[f64; 3]]>,
    interp: Interpolation,
    t: f64,
) -> f64 {
    let n = times.len();
    if t <= times[0] {
        return values[0];
    }
    if t >= times[n - 1] {
        return values[n - 1];
    }
    let i = match times.binary_search_by(|x| x.total_cmp(&t)) {
        Ok(j) => return values[j],
        Err(j) => j - 1,
    };
    let (t0, t1) = (times[i], times[i + 1]);
    let lerp = values[i] + (values[i + 1] - values[i]) * (t - t0) / (t1 - t0);
    match interp {
        Interpolation::ZeroOrderHold => values[i],
        Interpolation::Linear => lerp,
        // With fewer than two samples there is no spline to fit, so fall back to
        // the line; the endpoint guards above make that case unreachable here.
        Interpolation::CubicSpline => segments.map_or(lerp, |s| {
            let [b, c, d] = s[i];
            let dt = t - t0;
            values[i] + dt * (b + dt * (c + dt * d))
        }),
    }
}

/// The per-segment coefficients `[b, c, d]` of the natural cubic spline, where
/// segment `i` reads `values[i] + dt*(b + dt*(c + dt*d))` for `dt = t - times[i]`.
#[allow(
    clippy::many_single_char_names,
    reason = "h, b, c, d are the standard spline-coefficient names"
)]
pub(crate) fn cubic_segments(times: &[f64], values: &[f64]) -> Vec<[f64; 3]> {
    let n = times.len();
    let h: Vec<f64> = (0..n - 1).map(|i| times[i + 1] - times[i]).collect();
    if n == 2 {
        return vec![[(values[1] - values[0]) / h[0], 0.0, 0.0]];
    }
    let mut alpha = vec![0.0; n];
    for i in 1..n - 1 {
        alpha[i] =
            3.0 * ((values[i + 1] - values[i]) / h[i] - (values[i] - values[i - 1]) / h[i - 1]);
    }
    let mut c_prime = vec![0.0; n];
    let mut d_prime = vec![0.0; n];
    for i in 1..n - 1 {
        let denom = 2.0 * (h[i - 1] + h[i]) - h[i - 1] * c_prime[i - 1];
        c_prime[i] = h[i] / denom;
        d_prime[i] = (alpha[i] - h[i - 1] * d_prime[i - 1]) / denom;
    }
    let mut c = vec![0.0; n];
    for i in (1..n - 1).rev() {
        c[i] = d_prime[i] - c_prime[i] * c[i + 1];
    }
    (0..n - 1)
        .map(|i| {
            let b = (values[i + 1] - values[i]) / h[i] - h[i] * (c[i + 1] + 2.0 * c[i]) / 3.0;
            let d = (c[i + 1] - c[i]) / (3.0 * h[i]);
            [b, c[i], d]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "these interpolated values are exact")]

    use super::*;

    #[test]
    fn linear_reads_the_straight_line() {
        let times = [0.0, 5.0, 10.0];
        let values = [0.0, 10.0, 15.0];
        let out = read_at(&times, &values, Interpolation::Linear, &[2.5, 7.5]);
        assert_eq!(out, vec![5.0, 12.5]);
    }

    #[test]
    fn zero_order_hold_keeps_the_previous_sample() {
        let times = [0.0, 5.0, 10.0];
        let values = [1.0, 2.0, 3.0];
        let out = read_at(&times, &values, Interpolation::ZeroOrderHold, &[2.0, 5.0, 7.0, 10.0]);
        assert_eq!(out, vec![1.0, 2.0, 2.0, 3.0]);
    }

    #[test]
    fn cubic_spline_matches_the_hand_solved_value() {
        let times = [0.0, 1.0, 2.0, 3.0];
        let values = [0.0, 1.0, 4.0, 9.0];
        let out = read_at(&times, &values, Interpolation::CubicSpline, &[1.5]);
        assert_eq!(out, vec![2.2]);
    }

    #[test]
    fn every_mode_returns_the_samples_at_the_sample_times() {
        let times = [0.0, 1.0, 2.5, 4.0];
        let values = [3.0, -1.0, 2.0, 5.0];
        for interp in [
            Interpolation::Linear,
            Interpolation::ZeroOrderHold,
            Interpolation::CubicSpline,
        ] {
            assert_eq!(read_at(&times, &values, interp, &times), values.to_vec());
        }
    }

    #[test]
    fn cubic_spline_of_collinear_points_is_the_line() {
        let times = [0.0, 1.0, 2.0, 3.0, 4.0];
        let values = [1.0, 3.0, 5.0, 7.0, 9.0];
        let at = [0.3, 1.7, 2.5, 3.9];
        assert_eq!(
            read_at(&times, &values, Interpolation::CubicSpline, &at),
            read_at(&times, &values, Interpolation::Linear, &at)
        );
    }

    #[test]
    fn queries_outside_the_range_hold_the_ends() {
        let times = [0.0, 10.0];
        let values = [1.0, 2.0];
        for interp in [
            Interpolation::Linear,
            Interpolation::ZeroOrderHold,
            Interpolation::CubicSpline,
        ] {
            let out = read_at(&times, &values, interp, &[-5.0, 15.0]);
            assert_eq!(out, vec![1.0, 2.0]);
        }
    }

    #[test]
    fn a_single_sample_reads_as_a_constant() {
        let times = [5.0];
        let values = [10.0];
        let out = read_at(&times, &values, Interpolation::CubicSpline, &[0.0, 5.0, 9.0]);
        assert_eq!(out, vec![10.0, 10.0, 10.0]);
    }
}