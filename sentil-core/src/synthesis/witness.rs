//! Counterexample synthesis: search for an input that violates a specification.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use super::cmaes::{cma_es, CmaConfig};
use super::model::{Bounds, SystemModel};
use super::pgrad::maximize;
use super::smooth::SmoothConfig;
use crate::error::Result;
use crate::formula::Formula;
use crate::signal::Trace;

/// A trajectory that witnesses a formula's truth value.
pub struct Witness {
    /// The input that produced the trace.
    pub input: Vec<f64>,
    /// The exact robustness of the formula on the trace.
    pub robustness: f64,
    /// The witnessing trace.
    pub trace: Trace,
}

impl Formula {
    /// Searches `bounds` for an input to `model` that violates the formula, by
    /// descending its smooth robustness.
    ///
    /// # Errors
    ///
    /// Propagates any error from rolling the model or scoring the formula.
    pub fn find_counterexample<M: SystemModel>(
        &self,
        model: &M,
        bounds: &Bounds,
        max_iters: usize,
        smooth: SmoothConfig,
    ) -> Result<Witness> {
        let initial = model.initial_state();
        let objective = |u: &[f64]| -> Result<(f64, Vec<f64>)> {
            let (value, gradient) = self.smooth_gradient(model, initial, u, smooth)?;
            Ok((-value, gradient.into_iter().map(|g| -g).collect()))
        };
        let start = vec![0.0; model.input_dimension()];
        let (input, _) = maximize(objective, &start, bounds, max_iters)?;
        let trace = model.rollout_from(initial, &input)?;
        let robustness = self.robustness(&trace)?;
        Ok(Witness {
            input,
            robustness,
            trace,
        })
    }

    /// Searches `bounds` for an input whose trace violates the formula, by
    /// minimizing the exact robustness with CMA-ES over up to `restarts` seeds.
    ///
    /// # Errors
    ///
    /// Propagates any error from rolling the model or scoring the formula.
    #[allow(
        clippy::needless_borrows_for_generic_args,
        reason = "the objective is scored by cma_es on every restart, so it is borrowed not moved"
    )]
    pub fn falsify<M: SystemModel>(
        &self,
        model: &M,
        bounds: &Bounds,
        config: CmaConfig,
        restarts: usize,
    ) -> Result<Witness> {
        let initial = model.initial_state();
        let objective = |u: &[f64]| -> Result<f64> {
            let trace = model.rollout_from(initial, u)?;
            Ok(-self.robustness(&trace)?)
        };
        let start = vec![0.0; model.input_dimension()];
        let (mut best_input, mut best_score) = cma_es(&objective, &start, bounds, config)?;
        for r in 1..restarts.max(1) {
            if best_score > 0.0 {
                break;
            }
            let restart = CmaConfig {
                seed: config.seed.wrapping_add(r as u64),
                ..config
            };
            let (input, score) = cma_es(&objective, &start, bounds, restart)?;
            if score > best_score {
                best_score = score;
                best_input = input;
            }
        }
        let trace = model.rollout_from(initial, &best_input)?;
        let robustness = self.robustness(&trace)?;
        Ok(Witness {
            input: best_input,
            robustness,
            trace,
        })
    }
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

    #[test]
    fn finds_an_input_that_violates_a_reachable_bound() {
        let model = integrator(5);
        let phi = Formula::parse("always[0, 5](pos < 1)").unwrap();
        let bounds = Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap();
        let witness = phi
            .find_counterexample(&model, &bounds, 400, SmoothConfig::default())
            .unwrap();
        assert!(
            witness.robustness < 0.0,
            "robustness {}",
            witness.robustness
        );
        assert!(witness.trace.signals()["pos"].iter().any(|&p| p >= 1.0));
    }

    #[test]
    fn reports_no_violation_when_the_bound_is_out_of_reach() {
        let model = integrator(5);
        let phi = Formula::parse("always[0, 5](pos < 100)").unwrap();
        let bounds = Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap();
        let witness = phi
            .find_counterexample(&model, &bounds, 400, SmoothConfig::default())
            .unwrap();
        assert!(witness.robustness >= 0.0);
    }

    #[test]
    fn falsify_finds_a_violation_by_global_search() {
        let model = integrator(5);
        let phi = Formula::parse("always[0, 5](pos < 1)").unwrap();
        let bounds = Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap();
        let witness = phi
            .falsify(&model, &bounds, CmaConfig::default(), 5)
            .unwrap();
        assert!(
            witness.robustness < 0.0,
            "robustness {}",
            witness.robustness
        );
        assert!(witness.trace.signals()["pos"].iter().any(|&p| p >= 1.0));
    }

    #[test]
    fn falsify_reports_no_violation_when_unreachable() {
        let model = integrator(5);
        let phi = Formula::parse("always[0, 5](pos < 100)").unwrap();
        let bounds = Bounds::new(vec![-1.0; 5], vec![1.0; 5]).unwrap();
        let witness = phi
            .falsify(&model, &bounds, CmaConfig::default(), 3)
            .unwrap();
        assert!(witness.robustness >= 0.0);
    }
}