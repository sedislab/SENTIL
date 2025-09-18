//! Control-barrier-function safety filter.
//!
//! A safety filter shields a nominal controller: when the nominal input would
//! breach a barrier, it returns the closest input that does not, and otherwise
//! passes the nominal through unchanged. Each barrier is a linear inequality
//! `a · u >= b` on the input, the control-barrier condition for one step; the
//! filter solves the least-change quadratic program subject to those barriers and
//! the actuator bounds, so it overrides the controller as little as safety allows.

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

    /// Returns the input closest to `nominal` that satisfies every barrier
    /// `a · u >= b` and stays inside the bounds; a safe `nominal` passes through.
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
        let u = solve_qp(&p, &q, &g, &h, self.max_iters)?;
        // solve_qp can return a u satisfying no barrier when the primal is infeasible; reject rather
        // than pass it through. Tolerance scales with magnitude so solver slack does not read as a violation.
        for (k, (coefficients, bound)) in barriers.iter().enumerate() {
            let value: f64 = coefficients.iter().zip(&u).map(|(a, x)| a * x).sum();
            if value < bound - 1e-4 * (1.0 + bound.abs() + value.abs()) {
                return Err(Error::InvalidConfig {
                    context: "safety filter",
                    message: format!(
                        "barrier {k} cannot be met inside the bounds; the barriers and the actuator box have no common feasible input"
                    ),
                });
            }
        }
        Ok(u)
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