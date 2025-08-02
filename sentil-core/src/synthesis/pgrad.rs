//! Projected gradient ascent over a box.

use super::model::Bounds;
use crate::error::Result;

/// Maximizes `objective` over the box `bounds`, starting from `start`, for at most
/// `max_iters` steps.
///
/// # Errors
///
/// Propagates any error the objective returns.
pub fn maximize<F>(
    objective: F,
    start: &[f64],
    bounds: &Bounds,
    max_iters: usize,
) -> Result<(Vec<f64>, f64)>
where
    F: Fn(&[f64]) -> Result<(f64, Vec<f64>)>,
{
    let mut current = start.to_vec();
    bounds.clamp(&mut current);
    let (mut value, mut gradient) = objective(&current)?;
    let mut best = current.clone();
    let mut best_value = value;
    let mut step = 1.0;
    for _ in 0..max_iters {
        let mut candidate = current.clone();
        for (c, g) in candidate.iter_mut().zip(&gradient) {
            *c += step * g;
        }
        bounds.clamp(&mut candidate);
        let (candidate_value, candidate_gradient) = objective(&candidate)?;
        if candidate_value > value {
            current = candidate;
            value = candidate_value;
            gradient = candidate_gradient;
            if value > best_value {
                best_value = value;
                best.clone_from(&current);
            }
            step *= 1.2;
        } else {
            step *= 0.5;
            if step < 1e-12 {
                break;
            }
        }
    }
    Ok((best, best_value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_an_interior_optimum() {
        let objective = |u: &[f64]| {
            let d = u[0] - 3.0;
            Ok((-d * d, vec![-2.0 * d]))
        };
        let (best, value) =
            maximize(objective, &[0.0], &Bounds::new([0.0], [10.0]).unwrap(), 200).unwrap();
        assert!((best[0] - 3.0).abs() < 1e-3);
        assert!(value > -1e-4);
    }

    #[test]
    fn projects_an_exterior_optimum_to_the_boundary() {
        let objective = |u: &[f64]| {
            let d = u[0] - 20.0;
            Ok((-d * d, vec![-2.0 * d]))
        };
        let (best, _) =
            maximize(objective, &[0.0], &Bounds::new([0.0], [10.0]).unwrap(), 200).unwrap();
        assert!((best[0] - 10.0).abs() < 1e-3);
    }
}