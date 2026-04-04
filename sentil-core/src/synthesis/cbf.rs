//! Control-barrier-function safety filter.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use super::model::Bounds;
use super::qp::solve_qp;
use crate::error::{Error, Result};

const BARRIER_SLACK: f64 = 32.0 * f64::EPSILON;

const RESTORE_MARGIN: f64 = 8.0 * f64::EPSILON;

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
        let n = self.bounds.dimension();
        if nominal.len() != n {
            let actual = nominal.len();
            return Err(Error::InvalidConfig {
                context: "safety filter",
                message: format!(
                    "the nominal input has width {actual} but the filter's bounds have \
                     width {n}; pass a nominal input of width {n}, or build the filter \
                     with bounds of width {actual}"
                ),
            });
        }
        if let Some((i, value)) = nominal.iter().copied().enumerate().find(|(_, x)| !x.is_finite())
        {
            return Err(Error::InvalidConfig {
                context: "safety filter",
                message: format!(
                    "the nominal input is {value} at index {i}; every coordinate must be finite"
                ),
            });
        }
        let p: Vec<Vec<f64>> = (0..n)
            .map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
            .collect();
        let q: Vec<f64> = nominal.iter().map(|&x| -x).collect();

        let (mut g, mut h) = self.bounds.constraint_rows();
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
            if let Some((j, value)) = coefficients
                .iter()
                .copied()
                .enumerate()
                .find(|(_, x)| !x.is_finite())
            {
                return Err(Error::InvalidConfig {
                    context: "safety filter",
                    message: format!(
                        "barrier {k} is {value} at coefficient {j}; every coefficient \
                         and bound must be finite"
                    ),
                });
            }
            if !bound.is_finite() {
                return Err(Error::InvalidConfig {
                    context: "safety filter",
                    message: format!(
                        "barrier {k} has a bound of {bound}; every coefficient and \
                         bound must be finite"
                    ),
                });
            }
            g.push(coefficients.iter().map(|&x| -x).collect());
            h.push(-bound);
        }
        for iters in [self.max_iters, self.max_iters.saturating_mul(40)] {
            let u = solve_qp(&p, &q, &g, &h, iters)?;
            if let Some(safe) = self.accept(barriers, &u, iters) {
                return Ok(safe);
            }
        }
        Err(Error::InvalidConfig {
            context: "safety filter",
            message: "the barriers and the actuator box have no common feasible input".to_owned(),
        })
    }

    /// The clamped and restored input, when it clears every barrier to within
    /// [`BARRIER_SLACK`].
    fn accept(&self, barriers: &[(Vec<f64>, f64)], u: &[f64], passes: usize) -> Option<Vec<f64>> {
        let mut safe = u.to_vec();
        self.bounds.clamp(&mut safe);
        self.restore(barriers, &mut safe, passes);
        let holds = safe.iter().all(|x| x.is_finite())
            && barriers.iter().all(|(coefficients, bound)| {
                let value: f64 = coefficients.iter().zip(&safe).map(|(a, x)| a * x).sum();
                bound - value <= BARRIER_SLACK * (1.0 + bound.abs() + value.abs())
            });
        if !holds {
            return None;
        }
        Some(safe)
    }

    /// Projects a clamped point onto each barrier it misses, within the active face,
    /// for at most `passes` rounds.
    fn restore(&self, barriers: &[(Vec<f64>, f64)], safe: &mut [f64], passes: usize) {
        let (lower, upper) = (self.bounds.lower(), self.bounds.upper());
        let free = |i: usize, x: f64, a: f64| (a > 0.0 && x < upper[i]) || (a < 0.0 && x > lower[i]);
        for _ in 0..passes {
            let mut moved = false;
            for (coefficients, bound) in barriers {
                let value: f64 = coefficients.iter().zip(safe.iter()).map(|(a, x)| a * x).sum();
                let gap = bound - value;
                let mut square = 0.0;
                for (i, &a) in coefficients.iter().enumerate() {
                    if free(i, safe[i], a) {
                        square += a * a;
                    }
                }
                if gap > 0.0 && square > 0.0 {
                    let overshoot = RESTORE_MARGIN * (1.0 + bound.abs() + value.abs());
                    let step = (gap + overshoot) / square;
                    for (i, &a) in coefficients.iter().enumerate() {
                        if free(i, safe[i], a) {
                            safe[i] += step * a;
                        }
                    }
                    moved = true;
                }
            }
            self.bounds.clamp(safe);
            if !moved {
                break;
            }
        }
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
    fn a_nominal_of_the_wrong_width_is_named_in_the_error() {
        let wide = SafetyFilter::new(Bounds::new([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]).unwrap());
        let err = wide.filter(&[0.0], &[]).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidConfig { context: "safety filter", ref message }
                if message.contains("nominal input has width 1")
                    && message.contains("bounds have width 3")
        ));

        let narrow = SafetyFilter::new(Bounds::new([-1.0], [1.0]).unwrap());
        let err = narrow.filter(&[0.0, 0.0], &[(vec![1.0, 1.0], 0.0)]).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidConfig { context: "safety filter", ref message }
                if message.contains("nominal input has width 2")
                    && message.contains("bounds have width 1")
        ));
    }

    #[test]
    fn a_wide_box_never_reports_success_for_an_input_that_misses_a_barrier() {
        let filter = SafetyFilter::new(Bounds::new([-1e6, -1e6], [1e6, 1e6]).unwrap());
        for &(gap, push) in &[(5.0, 10.0), (50.0, 2.0), (50.0, 100.0), (500.0, 2.0), (500.0, 100.0)]
        {
            match filter.filter(&[push * 1e6, 1e6], &[(vec![1.0, -1.0], gap)]) {
                Ok(u) => {
                    let value = u[0] - u[1];
                    assert!(
                        gap - value <= BARRIER_SLACK * (1.0 + gap + value.abs()),
                        "gap {gap}, push {push}: {u:?} gives u0 - u1 = {value}"
                    );
                }
                Err(Error::InvalidConfig {
                    context: "safety filter",
                    ..
                }) => {}
                Err(e) => panic!("gap {gap}, push {push}: unexpected error: {e}"),
            }
        }
    }

    #[test]
    fn a_barrier_far_from_the_origin_is_met_and_not_merely_approached() {
        let filter = SafetyFilter::new(Bounds::new([-1e6, -1e6], [1e6, 1e6]).unwrap());
        let u = filter.filter(&[2e6, -1.5e6], &[(vec![2.0, 1.5], 8e5)]).unwrap();
        assert!(2.0 * u[0] + 1.5 * u[1] >= 8e5, "u = {u:?} misses the barrier");
        assert!((u[0] - 1e6).abs() < 1e-6 && (u[1] + 8e5).abs() < 1e-6, "u = {u:?}");
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

    #[test]
    fn a_non_finite_nominal_is_refused_rather_than_clamped() {
        let filter =
            SafetyFilter::new(Bounds::new([0.0, 0.0, -100.0], [100.0, 100.0, 100.0]).unwrap());
        for nominal in [
            [f64::NAN, 0.5, 0.0],
            [0.0, f64::INFINITY, 0.0],
            [0.0, 0.5, f64::NEG_INFINITY],
        ] {
            let err = filter.filter(&nominal, &[]).unwrap_err();
            assert!(
                matches!(
                    err,
                    Error::InvalidConfig { context: "safety filter", ref message }
                        if message.contains("nominal input")
                ),
                "a non-finite nominal must be refused, got {err:?}"
            );
        }
    }

    #[test]
    fn a_shallow_barrier_is_met_against_an_actuator_face() {
        let filter = SafetyFilter::new(Bounds::new([-1.0, -1.0], [1.0, 1.0]).unwrap());
        let u = filter.filter(&[0.0, 0.0], &[(vec![-1.28, 0.02], 1.29)]).unwrap();
        assert!(-1.28 * u[0] + 0.02 * u[1] >= 1.29, "u = {u:?} misses the barrier");
        assert!((-1.0..=1.0).contains(&u[0]) && (-1.0..=1.0).contains(&u[1]), "u = {u:?}");
    }

    #[test]
    fn a_non_finite_barrier_is_refused() {
        let filter = SafetyFilter::new(Bounds::unbounded(2));
        let err = filter
            .filter(&[0.0, 0.0], &[(vec![1.0, f64::NAN], 0.0)])
            .unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidConfig { context: "safety filter", ref message }
                if message.contains("barrier 0") && message.contains("coefficient 1")
        ));
        let err = filter
            .filter(&[0.0, 0.0], &[(vec![1.0, 0.0], f64::INFINITY)])
            .unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidConfig { context: "safety filter", ref message }
                if message.contains("barrier 0") && message.contains("bound")
        ));
    }
}