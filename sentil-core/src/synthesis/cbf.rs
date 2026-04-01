//! Control-barrier-function safety filter.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use super::model::Bounds;
use super::qp::solve_qp;
use crate::error::{Error, Result};

/// A least-restrictive control-barrier safety filter over a fixed input box.
pub struct SafetyFilter {
    bounds: Bounds,
    max_iters: usize,
}

impl SafetyFilter {
    /// Builds a filter that keeps inputs inside `bounds`.
    #[must_use]
    pub fn new(bounds: Bounds) -> Self {
        Self {
            bounds,
            max_iters: 500,
        }
    }

    /// The input closest to `nominal` that satisfies every barrier `a . u >= b` and
    /// stays inside the bounds.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`](crate::Error::InvalidConfig) if `nominal` or a
    /// barrier does not match the width of the bounds, or if no input inside the
    /// bounds meets the barriers.
    #[allow(
        clippy::many_single_char_names,
        reason = "standard quadratic-program notation: cost P/q, constraints G/h"
    )]
    pub fn filter(&self, nominal: &[f64], barriers: &[(Vec<f64>, f64)]) -> Result<Vec<f64>> {
        let n = nominal.len();
        let p: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
            .collect();
        let q: Vec<f64> = nominal.iter().map(|&x| -x).collect();

        let mut g: Vec<Vec<f64>> = Vec::new();
        let mut h: Vec<f64> = Vec::new();
        for (i, (&lo, &hi)) in self
            .bounds
            .lower()
            .iter()
            .zip(self.bounds.upper())
            .enumerate()
        {
            if hi.is_finite() {
                let mut row = vec![0.0; n];
                row[i] = 1.0;
                g.push(row);
                h.push(hi);
            }
            if lo.is_finite() {
                let mut row = vec![0.0; n];
                row[i] = -1.0;
                g.push(row);
                h.push(-lo);
            }
        }
        for (k, (coefficients, bound)) in barriers.iter().enumerate() {
            if coefficients.len() != n {
                return Err(Error::InvalidConfig {
                    context: "safety filter",
                    message: format!(
                        "barrier {k} has {} coefficients but the input has width {n}",
                        coefficients.len()
                    ),
                });
            }
            g.push(coefficients.iter().map(|&x| -x).collect());
            h.push(-bound);
        }
        // The dual that solve_qp maximizes is unbounded when the box and the barriers
        // share no feasible input, and it converges slowly on an ill-conditioned
        // instance, so a fixed iteration budget can return a point that meets the
        // barriers but sits outside the box. Check every constraint, the box included,
        // and escalate the budget once before concluding the instance is infeasible,
        // so a slow solve is not mistaken for one with no answer and a point outside
        // the actuator box is never returned as safe.
        let u = solve_qp(&p, &q, &g, &h, self.max_iters)?;
        if let Some(safe) = self.accept(&g, &h, &u) {
            return Ok(safe);
        }
        let u = solve_qp(&p, &q, &g, &h, self.max_iters.saturating_mul(40))?;
        if let Some(safe) = self.accept(&g, &h, &u) {
            return Ok(safe);
        }
        Err(Error::InvalidConfig {
            context: "safety filter",
            message: "the barriers and the actuator box have no common feasible input".to_owned(),
        })
    }

    /// Accepts a solved input if it satisfies every constraint, the actuator box and
    /// the barriers alike, clamped to the box so the returned input is exactly inside
    /// the actuator limits rather than within the solver's tolerance of them. Returns
    /// `None` if a constraint is violated past that tolerance.
    fn accept(&self, g: &[Vec<f64>], h: &[f64], u: &[f64]) -> Option<Vec<f64>> {
        let holds = g.iter().zip(h).all(|(row, &limit)| {
            let value: f64 = row.iter().zip(u).map(|(a, x)| a * x).sum();
            value - limit <= 1e-4 * (1.0 + limit.abs() + value.abs())
        });
        if !holds {
            return None;
        }
        // The box is a hard actuator limit, so pin the point exactly inside it. It is
        // already within tolerance of the box, so this moves it by at most that slack.
        Some(
            u.iter()
                .enumerate()
                .map(|(i, &x)| {
                    let lo = self.bounds.lower().get(i).copied().unwrap_or(f64::NEG_INFINITY);
                    let hi = self.bounds.upper().get(i).copied().unwrap_or(f64::INFINITY);
                    x.max(lo).min(hi)
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unsafe_nominal_is_pulled_to_the_barrier() {
        let filter = SafetyFilter::new(Bounds::unbounded(1));
        let u = filter.filter(&[-1.0], &[(vec![1.0], -0.5)]).unwrap();
        assert!((u[0] + 0.5).abs() < 1e-3, "u = {}", u[0]);
    }

    #[test]
    fn a_safe_nominal_passes_through() {
        let filter = SafetyFilter::new(Bounds::unbounded(1));
        let u = filter.filter(&[-1.0], &[(vec![1.0], -3.0)]).unwrap();
        assert!((u[0] + 1.0).abs() < 1e-3);
    }

    #[test]
    fn the_actuator_bounds_are_respected() {
        let filter = SafetyFilter::new(Bounds::new([-2.0], [2.0]).unwrap());
        let u = filter.filter(&[5.0], &[]).unwrap();
        assert!((u[0] - 2.0).abs() < 1e-3);
    }

    #[test]
    fn infeasible_barriers_are_reported_not_silently_violated() {
        let filter = SafetyFilter::new(Bounds::unbounded(1));
        let err = filter
            .filter(&[0.0], &[(vec![1.0], 1.0), (vec![-1.0], 1.0)])
            .unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidConfig {
                context: "safety filter",
                ..
            }
        ));
    }

    #[test]
    fn an_ok_output_always_stays_inside_the_box() {
        let filter = SafetyFilter::new(Bounds::new([2.0, 2.0], [5.0, 5.0]).unwrap());
        let err = filter
            .filter(&[-5.0, -1.0], &[(vec![-2.75, -0.1], -2.6)])
            .unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidConfig {
                context: "safety filter",
                ..
            }
        ));

        let filter = SafetyFilter::new(Bounds::new([-1.0, -1.0], [1.0, 1.0]).unwrap());
        let u = filter
            .filter(&[-8.0, 6.0], &[(vec![3.0, 3.0], 1.0)])
            .unwrap();
        assert!(
            (-1.0..=1.0).contains(&u[0]) && (-1.0..=1.0).contains(&u[1]),
            "u = {u:?} left the box"
        );
        assert!(3.0 * u[0] + 3.0 * u[1] >= 1.0 - 1e-3, "u = {u:?} misses the barrier");
    }

    #[test]
    fn a_barrier_of_the_wrong_width_is_named_in_the_error() {
        let filter = SafetyFilter::new(Bounds::unbounded(2));
        let err = filter
            .filter(&[0.0, 0.0], &[(vec![1.0, 0.0], 0.0), (vec![1.0], 0.0)])
            .unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidConfig { context: "safety filter", ref message }
                if message.contains("barrier 1") && message.contains("width 2")
        ));
    }
}