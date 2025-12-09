//! Receding-horizon online controller.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
#[cfg(feature = "std")]
use std::time::{Duration, Instant};

use super::convex;
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
    #[cfg(feature = "std")]
    budget: Option<Duration>,
    max_iters: usize,
    warm_start: Vec<f64>,
}

impl<'a, M: SystemModel> Controller<'a, M> {
    /// Builds a controller that plans `spec` over `model`, applies `input_width`
    /// values per step, and spends at most `budget` planning each step.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn new(model: &'a M, spec: &'a Formula, input_width: usize, budget: Duration) -> Self {
        let dimension = model.input_dimension();
        Self {
            model,
            spec,
            input_width,
            bounds: Bounds::unbounded(dimension),
            smooth: SmoothConfig::default(),
            budget: Some(budget),
            max_iters: usize::MAX,
            warm_start: vec![0.0; dimension],
        }
    }

    /// Builds a controller bounded by a gradient-step count rather than a clock.
    #[must_use]
    pub fn with_iterations(
        model: &'a M,
        spec: &'a Formula,
        input_width: usize,
        max_iters: usize,
    ) -> Self {
        let dimension = model.input_dimension();
        Self {
            model,
            spec,
            input_width,
            bounds: Bounds::unbounded(dimension),
            smooth: SmoothConfig::default(),
            #[cfg(feature = "std")]
            budget: None,
            max_iters: max_iters.max(1),
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

        if let Some(affine) = self.model.affine_form() {
            if let Some(input) = convex::step(&affine, self.spec, state, &self.bounds) {
                if input.len() >= self.input_width {
                    self.warm_start = shifted(&input, self.input_width);
                    return Ok(input[..self.input_width].to_vec());
                }
            }
        }

        let (model, spec, smooth) = (self.model, self.spec, self.smooth);
        let objective = |u: &[f64]| spec.smooth_gradient(model, state, u, smooth);
        let bounds = &self.bounds;
        #[cfg(feature = "std")]
        let start = Instant::now();
        #[cfg(feature = "std")]
        let deadline = self.budget.map(|budget| start + budget);

        let (mut best, mut best_score) = maximize(objective, &self.warm_start, bounds, 1)?;
        let mut plan = best.clone();
        let mut steps = 1usize;
        loop {
            #[cfg(feature = "std")]
            if let Some(deadline) = deadline {
                if Instant::now() >= deadline {
                    break;
                }
            }
            if steps >= self.max_iters {
                break;
            }
            #[cfg(feature = "std")]
            let chunk = match deadline {
                Some(deadline) => {
                    #[allow(
                        clippy::cast_precision_loss,
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "steps and CHUNK are tiny counts, and the ratio is clamped to [1, CHUNK] before the cast"
                    )]
                    {
                        let per_step = start.elapsed().as_secs_f64() / steps as f64;
                        let remaining = (deadline - Instant::now()).as_secs_f64();
                        if per_step > 0.0 {
                            (remaining / per_step).clamp(1.0, CHUNK as f64) as usize
                        } else {
                            CHUNK
                        }
                    }
                }
                None => CHUNK.min(self.max_iters.saturating_sub(steps)).max(1),
            };
            #[cfg(not(feature = "std"))]
            let chunk = CHUNK.min(self.max_iters.saturating_sub(steps)).max(1);

            let (next, score) = maximize(objective, &plan, bounds, chunk)?;
            steps += chunk;
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
    fn a_tiny_budget_still_returns_a_valid_input() {
        let model = integrator(4);
        let spec = Formula::parse("eventually[0, 4](pos > 1)").unwrap();
        let mut controller = Controller::new(&model, &spec, 1, Duration::from_nanos(1))
            .with_bounds(Bounds::new(vec![-1.0; 4], vec![1.0; 4]).unwrap());
        let u = controller.control(&[0.0]).unwrap();
        assert_eq!(u.len(), 1);
        assert!((-1.0..=1.0).contains(&u[0]), "input {} left the box", u[0]);
    }

    #[cfg(feature = "std")]
    #[test]
    fn a_bad_input_width_is_rejected() {
        let model = integrator(3);
        let spec = Formula::parse("pos > 0").unwrap();
        let mut controller = Controller::new(&model, &spec, 9, Duration::from_millis(5));
        assert!(controller.control(&[0.0]).is_err());
    }

    fn exact(model: &LinearModel, spec: &Formula, state: &[f64], input: &[f64]) -> f64 {
        let trace = model.rollout_from(state, input).unwrap();
        spec.robustness(&trace).unwrap()
    }

    #[cfg(feature = "std")]
    #[test]
    fn the_convex_path_satisfies_a_conjunctive_affine_spec() {
        let model = integrator(6);
        let spec = Formula::parse("always[3, 5](pos > 2) and always[0, 6](pos < 10)").unwrap();
        let mut controller = Controller::new(&model, &spec, 6, Duration::from_millis(50))
            .with_bounds(Bounds::new(vec![-1.0; 6], vec![1.0; 6]).unwrap());
        let plan = controller.control(&[0.0]).unwrap();
        assert!(plan.iter().all(|&u| (-1.0..=1.0).contains(&u)), "plan {plan:?}");
        assert!(exact(&model, &spec, &[0.0], &plan) >= -1e-6, "plan {plan:?}");
    }

    #[cfg(feature = "std")]
    #[test]
    fn the_convex_closed_loop_reaches_and_holds_the_target() {
        let model = integrator(6);
        let spec = Formula::parse("always[3, 5](pos > 2)").unwrap();
        let mut controller = Controller::new(&model, &spec, 1, Duration::from_millis(50))
            .with_bounds(Bounds::new(vec![-1.0; 6], vec![1.0; 6]).unwrap());

        let mut state = vec![0.0];
        for _ in 0..10 {
            let u = controller.control(&state).unwrap();
            assert!((-1.0..=1.0).contains(&u[0]), "input {} left the box", u[0]);
            state[0] += u[0];
        }
        assert!(state[0] >= 1.9, "reached only {}", state[0]);
    }

    fn gradient_rho(model: &LinearModel, spec: &Formula, state: &[f64], bounds: &Bounds) -> f64 {
        let start = vec![0.0; model.input_dimension()];
        let input = maximize(
            |u: &[f64]| spec.smooth_gradient(model, state, u, SmoothConfig::default()),
            &start,
            bounds,
            800,
        )
        .unwrap()
        .0;
        exact(model, spec, state, &input)
    }

    #[test]
    fn the_convex_path_satisfies_a_feasible_spec() {
        let model = integrator(6);
        let spec = Formula::parse("always[3, 5](pos > 2) and always[0, 6](pos > -4)").unwrap();
        let bounds = Bounds::new(vec![-1.0; 6], vec![1.0; 6]).unwrap();
        let input = convex::step(&model.affine_form().unwrap(), &spec, &[0.0], &bounds).unwrap();
        let qp_rho = exact(&model, &spec, &[0.0], &input);
        assert!(qp_rho >= -1e-6, "qp robustness {qp_rho}");
        assert!(gradient_rho(&model, &spec, &[0.0], &bounds) >= -1e-6);
    }

    #[test]
    fn the_convex_path_handles_a_wide_margin_spec() {
        let model = integrator(6);
        let spec = Formula::parse("always[5, 6](pos > 1000)").unwrap();
        let bounds = Bounds::new(vec![-200.0; 6], vec![200.0; 6]).unwrap();
        let input = convex::step(&model.affine_form().unwrap(), &spec, &[0.0], &bounds).unwrap();
        assert!(input.iter().all(|&u| (-200.0..=200.0).contains(&u)), "input {input:?}");
        let rho = exact(&model, &spec, &[0.0], &input);
        assert!(rho >= -1e-3, "wide-margin spec should hold, rho {rho}");
    }

    #[test]
    fn the_qp_margin_matches_the_exact_robustness() {
        let model = integrator(6);
        let bounds = Bounds::new(vec![-1.0; 6], vec![1.0; 6]).unwrap();
        let affine = model.affine_form().unwrap();
        for (text, expected) in [
            ("always[3, 5](pos > 2)", 1.0),
            ("always[0, 6](pos > 10)", -10.0),
            ("always[0, 6](not(pos > 1))", 1.0),
        ] {
            let spec = Formula::parse(text).unwrap();
            let input = convex::step(&affine, &spec, &[0.0], &bounds).unwrap();
            let scored = exact(&model, &spec, &[0.0], &input);
            assert!((scored - expected).abs() < 1e-3, "{text}: scored {scored} vs {expected}");
        }
    }

    #[test]
    fn the_convex_path_beats_the_gradient_on_an_infeasible_spec() {
        let model = integrator(6);
        let spec = Formula::parse("always[0, 6](pos > 10)").unwrap();
        let bounds = Bounds::new(vec![-1.0; 6], vec![1.0; 6]).unwrap();
        let input = convex::step(&model.affine_form().unwrap(), &spec, &[0.0], &bounds).unwrap();
        let qp_rho = exact(&model, &spec, &[0.0], &input);
        let grad_rho = gradient_rho(&model, &spec, &[0.0], &bounds);
        assert!(qp_rho < 0.0, "spec should be infeasible, got {qp_rho}");
        assert!(qp_rho >= grad_rho - 1e-6, "qp {qp_rho} below gradient {grad_rho}");
    }

    #[cfg(feature = "std")]
    #[test]
    fn a_disjunctive_spec_falls_back_to_the_gradient_path() {
        let model = integrator(5);
        let spec = Formula::parse("eventually[0, 5](pos > 2) or eventually[0, 5](pos < -8)").unwrap();
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
    fn an_infeasible_conjunctive_spec_returns_an_in_box_input() {
        let model = integrator(6);
        let spec = Formula::parse("always[0, 6](pos > 10) and always[0, 6](pos > -3)").unwrap();
        let mut controller = Controller::new(&model, &spec, 6, Duration::from_millis(50))
            .with_bounds(Bounds::new(vec![-1.0; 6], vec![1.0; 6]).unwrap());
        let plan = controller.control(&[0.0]).unwrap();
        assert_eq!(plan.len(), 6);
        assert!(plan.iter().all(|&u| (-1.0..=1.0).contains(&u)), "plan {plan:?}");
        let rho = exact(&model, &spec, &[0.0], &plan);
        let idle = exact(&model, &spec, &[0.0], &[0.0; 6]);
        assert!(rho >= idle - 1e-6, "plan rho {rho} below idle {idle}");
    }

    #[test]
    fn the_convex_and_gradient_paths_agree() {
        let model = integrator(6);
        let spec = Formula::parse("always[2, 4](pos > 1) and always[0, 6](pos < 8)").unwrap();
        let bounds = Bounds::new(vec![-1.0; 6], vec![1.0; 6]).unwrap();
        let input = convex::step(&model.affine_form().unwrap(), &spec, &[0.0], &bounds).unwrap();
        let qp_rho = exact(&model, &spec, &[0.0], &input);
        let grad_rho = gradient_rho(&model, &spec, &[0.0], &bounds);
        assert!(qp_rho >= -1e-6 && grad_rho >= -1e-6);
        assert!((qp_rho - grad_rho).abs() < 5e-2, "qp {qp_rho} vs gradient {grad_rho}");
    }

    #[test]
    fn empty_window_always_does_not_panic() {
        let model = integrator(6);
        let spec = Formula::parse("always[10, 12](pos > 2)").unwrap();
        let bounds = Bounds::new(vec![-1.0; 6], vec![1.0; 6]).unwrap();
        let input = convex::step(&model.affine_form().unwrap(), &spec, &[0.0], &bounds).unwrap();
        assert_eq!(input.len(), 6);
        assert!(input.iter().all(|&u| (-1.0..=1.0).contains(&u)), "input {input:?}");
    }

    #[test]
    fn the_convex_path_respects_a_tight_box() {
        let model = integrator(6);
        let spec = Formula::parse("always[5, 6](pos > 2)").unwrap();
        let bounds = Bounds::new(vec![-0.2; 6], vec![0.2; 6]).unwrap();
        let input = convex::step(&model.affine_form().unwrap(), &spec, &[0.0], &bounds).unwrap();
        assert!(input.iter().all(|&u| (-0.2..=0.2).contains(&u)), "input {input:?}");
        let rho = exact(&model, &spec, &[0.0], &input);
        assert!((rho + 1.0).abs() < 1e-3, "rho {rho} should sit on -1.0");
    }
}