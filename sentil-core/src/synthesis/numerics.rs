//! Dense linear-algebra primitives for synthesis.

use crate::error::{Error, Result};

const MAX_SWEEPS: usize = 100;
const TOLERANCE: f64 = 1e-28;

/// The eigendecomposition of a symmetric matrix by cyclic Jacobi rotations, as
/// eigenvalues paired with orthonormal `eigenvectors[j]`.
///
/// # Errors
///
/// Returns [`Error::InvalidConfig`] if `matrix` is not square.
#[allow(
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    reason = "standard linear-algebra notation and explicit row/column indexing read clearest here"
)]
pub fn symmetric_eigen(matrix: &[Vec<f64>]) -> Result<(Vec<f64>, Vec<Vec<f64>>)> {
    let n = matrix.len();
    if matrix.iter().any(|row| row.len() != n) {
        return Err(Error::InvalidConfig {
            context: "eigendecomposition",
            message: "matrix must be square".to_owned(),
        });
    }
    let mut a = matrix.to_vec();
    let mut v: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
        .collect();

    for _ in 0..MAX_SWEEPS {
        let mut off = 0.0;
        for p in 0..n {
            for q in (p + 1)..n {
                off += a[p][q] * a[p][q];
            }
        }
        if off <= TOLERANCE {
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                if a[p][q].abs() <= f64::MIN_POSITIVE {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + theta.mul_add(theta, 1.0).sqrt());
                let c = 1.0 / t.mul_add(t, 1.0).sqrt();
                let s = t * c;
                rotate(&mut a, &mut v, p, q, c, s);
            }
        }
    }

    let eigenvalues = (0..n).map(|i| a[i][i]).collect();
    let eigenvectors = (0..n).map(|j| (0..n).map(|i| v[i][j]).collect()).collect();
    Ok((eigenvalues, eigenvectors))
}

/// Applies the Jacobi rotation in the `(p, q)` plane to the working matrix `a`
/// (as `Jᵀ A J`) and accumulates it into the eigenvector matrix `v`, driving
/// `a[p][q]` to zero.
#[allow(
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    reason = "standard rotation notation: matrix a, eigenvectors v, plane p/q, cos c, sin s"
)]
fn rotate(a: &mut [Vec<f64>], v: &mut [Vec<f64>], p: usize, q: usize, c: f64, s: f64) {
    let n = a.len();
    for row in a.iter_mut() {
        let (kp, kq) = (row[p], row[q]);
        row[p] = c * kp - s * kq;
        row[q] = s * kp + c * kq;
    }
    for k in 0..n {
        let (pk, qk) = (a[p][k], a[q][k]);
        a[p][k] = c * pk - s * qk;
        a[q][k] = s * pk + c * qk;
    }
    for row in v.iter_mut() {
        let (kp, kq) = (row[p], row[q]);
        row[p] = c * kp - s * kq;
        row[q] = s * kp + c * kq;
    }
}