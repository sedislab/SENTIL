//! Receding-horizon online controller.
//!
//! At each step the controller plans a short input sequence from the live state by
//! climbing the smooth robustness, then hands back the first input to apply. The
//! plant advances on its own and the next call observes the new state, closing the
//! loop. The search is anytime: it runs gradient chunks until a wall-clock budget
//! expires and returns the best plan found, warm-started from the previous step so
//! a good plan is usually in hand long before the deadline.

use std::time::{Duration, Instant};

use super::model::{Bounds, SystemModel};
use super::pgrad::maximize;
use super::smooth::SmoothConfig;
use crate::error::{Error, Result};
use crate::formula::Formula;

const CHUNK: usize = 8;

const CONVERGED: f64 = 1e-9;

/// An online receding-horizon controller for a step-structured model.
pub struct Controller<'a, M: SystemModel> {
    model: &'a M,
    spec: &'a Formula,
    input_width: usize,
    bounds: Bounds,
    smooth: SmoothConfig,
    budget: Duration,
    warm_start: Vec<f64>,
}

impl<'a, M: SystemModel> Controller<'a, M> {
    /// Builds a controller that plans `spec` over `model`, applies `input_width`
    /// values per step, and spends at most `budget` planning each step.
    #[must_use]
    pub fn new(model: &'a M, spec: &'a Formula, input_width: usize, budget: Duration) -> Self {
        let dimension = model.input_dimension();
        Self {
            model,
            spec,
            input_width,
            bounds: Bounds::unbounded(dimension),
            smooth: SmoothConfig::default(),
            budget,
            warm_start: vec![0.0; dimension],
        }
    }

    /// Constrains the planned input to a box.
    #[must_use]
    pub fn with_bounds(mut self, bounds: Bounds) -> Self {
        self.bounds = bounds;
        self
    }

    /// Sets the smoothing temperature used while planning.
    #[must_use]
    pub fn with_smooth(mut self, smooth: SmoothConfig) -> Self {
        self.smooth = smooth;
        self
    }

    /// Plans from `state` and returns the first input to apply, of length
    /// `input_width`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if `input_width` is zero or wider than the
    /// model's input, and propagates any error from rolling the model or evaluating
    /// the specification.
    pub fn control(&mut self, state: &[f64]) -> Result<Vec<f64>> {
        let dimension = self.model.input_dimension();
        if self.input_width == 0 || self.input_width > dimension {
            return Err(Error::InvalidConfig {
                context: "controller",
                message: format!(
                    "input width {} must be in 1..={dimension}",
                    self.input_width
                ),
            });
        }

        let (model, spec, smooth) = (self.model, self.spec, self.smooth);
        let objective = |u: &[f64]| spec.smooth_gradient(model, state, u, smooth);
        let bounds = &self.bounds;
        let deadline = Instant::now() + self.budget;

        let (mut best, mut best_score) = maximize(objective, &self.warm_start, bounds, CHUNK)?;
        let mut plan = best.clone();
        while Instant::now() < deadline {
            let (next, score) = maximize(objective, &plan, bounds, CHUNK)?;
            let gain = score - best_score;
            if score > best_score {
                best_score = score;
                best.clone_from(&next);
            }
            plan = next;
            if gain < CONVERGED {
                break;
            }
        }

        self.warm_start = shifted(&best, self.input_width);
        Ok(best[..self.input_width].to_vec())
    }
}

fn shifted(plan: &[f64], width: usize) -> Vec<f64> {
    let mut next = Vec::with_capacity(plan.len());
    next.extend_from_slice(&plan[width..]);
    next.extend_from_slice(&plan[plan.len() - width..]);
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthesis::LinearModel;

    fn integrator(horizon: usize) -> LinearModel {
        LinearModel::new(
            vec![vec![1.0]],
            vec![vec![1.0]],
            [0.0],
            ["pos"],
            1.0,
            horizon,
        )
        .unwrap()
    }

    #[cfg(feature = "std")]
    #[test]
    fn the_closed_loop_reaches_the_target() {
        let model = integrator(5);
        let spec = Formula::parse("eventually[0, 5](pos > 2)").unwrap();
        let mut controller = Controller::new(&model, &spec, 1, Duration::from_millis(50))
            .with_bounds(Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap());

        let mut state = vec![0.0];
        for _ in 0..6 {
            let u = controller.control(&state).unwrap();
            assert!((-1.0..=1.0).contains(&u[0]), "input {} left the box", u[0]);
            state[0] += u[0];
        }
        assert!(state[0] > 2.0, "reached only {}", state[0]);
    }

    #[cfg(feature = "std")]
    #[test]
    fn a_bad_input_width_is_rejected() {
        let model = integrator(3);
        let spec = Formula::parse("pos > 0").unwrap();
        let mut controller = Controller::new(&model, &spec, 9, Duration::from_millis(5));
        assert!(controller.control(&[0.0]).is_err());
    }
}