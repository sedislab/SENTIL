//! Counterexample synthesis: search for an input that violates a specification.
//!
//! Where the synthesizer climbs toward satisfaction, this descends toward
//! violation, following the negated smooth-robustness gradient to the input that
//! breaks the formula hardest. A returned robustness below zero is a genuine
//! counterexample; otherwise it is the closest the search came over the bounded
//! inputs, evidence the formula may hold there.

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
}