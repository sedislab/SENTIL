//! A small dense convex quadratic program.
//!
//! The control-barrier safety filter projects a nominal input onto the safe set,
//! which is a quadratic program: minimize `½ uᵀP u + qᵀu` subject to `G u <= h`.
//! Rather than a combinatorial active-set search with its own feasibility phase,
//! this solves the Lagrangian dual, which is a concave maximization over the
//! non-negative multipliers and so is always feasible at `λ = 0`. The dual reuses
//! the projected-gradient ascent and the Cholesky solve already in the crate.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use super::model::Bounds;
use super::numerics::solve_spd;
use super::pgrad::maximize;
use crate::error::{Error, Result};

/// Minimizes `½ uᵀP u + qᵀu` subject to `G u <= h`, for a symmetric
/// positive-definite `P`.
///
/// # Errors
///
/// Returns [`Error::InvalidConfig`] if the shapes are inconsistent or `P` is not
/// positive-definite.
#[allow(
    clippy::many_single_char_names,
    reason = "standard quadratic-program notation: cost P/q, constraints G/h"
)]
pub fn solve_qp(
    p: &[Vec<f64>],
    q: &[f64],
    g: &[Vec<f64>],
    h: &[f64],
    max_iters: usize,
) -> Result<Vec<f64>> {
    let n = q.len();
    let m = h.len();
    if p.len() != n || p.iter().any(|row| row.len() != n) {
        return Err(Error::InvalidConfig {
            context: "quadratic program",
            message: format!(
                "P must be {n}x{n} to match q of length {n}, but it is {}x{}",
                p.len(),
                p.first().map_or(0, Vec::len)
            ),
        });
    }
    if g.len() != m || g.iter().any(|row| row.len() != n) {
        return Err(Error::InvalidConfig {
            context: "quadratic program",
            message: format!(
                "G must be {m}x{n}, one row per constraint in h and one column per variable, but it is {}x{}",
                g.len(),
                g.first().map_or(0, Vec::len)
            ),
        });
    }

    let inverse: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            let mut unit = vec![0.0; n];
            unit[i] = 1.0;
            solve_spd(p, &unit)
        })
        .collect::<Result<_>>()?;
    let apply_inverse = |w: &[f64]| -> Vec<f64> { inverse.iter().map(|row| dot(row, w)).collect() };
    let residual = |lambda: &[f64]| -> Vec<f64> {
        (0..n)
            .map(|i| q[i] + (0..m).map(|j| g[j][i] * lambda[j]).sum::<f64>())
            .collect()
    };

    // Maximize the dual g(λ) = -½ wᵀP⁻¹w - hᵀλ over λ >= 0, where w = q + Gᵀλ.
    let dual = |lambda: &[f64]| -> Result<(f64, Vec<f64>)> {
        let w = residual(lambda);
        let pinv_w = apply_inverse(&w);
        let value = -0.5 * dot(&w, &pinv_w) - dot(h, lambda);
        let gradient = (0..m).map(|j| -dot(&g[j], &pinv_w) - h[j]).collect();
        Ok((value, gradient))
    };
    let bounds = Bounds::new(vec![0.0; m], vec![f64::INFINITY; m])?;
    let (lambda, _) = maximize(dual, &vec![0.0; m], &bounds, max_iters)?;

    // Recover u = -P⁻¹(q + Gᵀλ).
    let u = apply_inverse(&residual(&lambda));
    Ok(u.iter().map(|x| -x).collect())
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_the_origin_onto_a_half_space() {
        // min ½|u|^2 s.t. u0 + u1 >= 2, written as -u0 - u1 <= -2. The closest point
        // to the origin on that half-space is (1, 1).
        let p = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let u = solve_qp(&p, &[0.0, 0.0], &[vec![-1.0, -1.0]], &[-2.0], 400).unwrap();
        assert!((u[0] - 1.0).abs() < 1e-3 && (u[1] - 1.0).abs() < 1e-3);
    }

    #[test]
    fn an_inactive_constraint_leaves_the_unconstrained_minimum() {
        // min ½(u - 5)^2 s.t. u <= 3: the unconstrained minimum 5 is infeasible, so
        // the solution sits on the active bound, u = 3.
        let u = solve_qp(&[vec![1.0]], &[-5.0], &[vec![1.0]], &[3.0], 400).unwrap();
        assert!((u[0] - 3.0).abs() < 1e-3);
    }

    #[test]
    fn inconsistent_shapes_are_rejected() {
        assert!(solve_qp(&[vec![1.0]], &[0.0], &[vec![1.0, 2.0]], &[1.0], 10).is_err());
    }
}