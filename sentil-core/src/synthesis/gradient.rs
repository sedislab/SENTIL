//! Central finite-difference gradient of smooth robustness over a model rollout.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use super::model::SystemModel;
use super::smooth::SmoothConfig;
use crate::error::Result;
use crate::formula::Formula;

const STEP: f64 = 1e-6;

impl Formula {
    /// The smooth robustness of the formula on the trace `model` rolls from
    /// `initial` under `input`, with its gradient against each input coordinate.
    ///
    /// # Errors
    ///
    /// Propagates any error from rolling the model or evaluating the formula.
    pub fn smooth_gradient(
        &self,
        model: &impl SystemModel,
        initial: &[f64],
        input: &[f64],
        config: SmoothConfig,
    ) -> Result<(f64, Vec<f64>)> {
        let score = |u: &[f64]| self.smooth_robustness(&model.rollout_from(initial, u)?, config);
        let value = score(input)?;
        let mut gradient = vec![0.0; input.len()];
        let mut perturbed = input.to_vec();
        for i in 0..input.len() {
            let original = perturbed[i];
            perturbed[i] = original + STEP;
            let plus = score(&perturbed)?;
            perturbed[i] = original - STEP;
            let minus = score(&perturbed)?;
            perturbed[i] = original;
            gradient[i] = (plus - minus) / (2.0 * STEP);
        }
        Ok((value, gradient))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthesis::LinearModel;

    #[test]
    fn finite_difference_gradient_matches_the_analytic_one() {
        let model =
            LinearModel::new(vec![vec![1.0]], vec![vec![1.0]], [0.0], ["x"], 1.0, 1).unwrap();
        let phi = Formula::parse("eventually[0, 1](x > 0)").unwrap();
        let (beta, u) = (10.0, 0.5);
        let (_, gradient) = phi
            .smooth_gradient(&model, &[0.0], &[u], SmoothConfig::new(beta).unwrap())
            .unwrap();
        let analytic = 1.0 / (1.0 + (-beta * u).exp());
        assert!((gradient[0] - analytic).abs() < 1e-4);
    }

    #[test]
    fn a_rollout_error_propagates() {
        let model =
            LinearModel::new(vec![vec![1.0]], vec![vec![1.0]], [0.0], ["x"], 1.0, 1).unwrap();
        let phi = Formula::parse("eventually[0, 1](x > 0)").unwrap();
        assert!(phi
            .smooth_gradient(&model, &[0.0], &[1.0, 2.0], SmoothConfig::default())
            .is_err());
    }
}