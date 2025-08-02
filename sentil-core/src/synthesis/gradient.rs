//! Central finite-difference gradient of smooth robustness over a model rollout.

use super::model::SystemModel;
use super::smooth::SmoothConfig;
use crate::error::Result;
use crate::formula::Formula;

const STEP: f64 = 1e-6;

impl Formula {
    /// The smooth robustness of the formula on the trace `model` rolls from
    /// `input`, with its gradient against each input coordinate.
    ///
    /// # Errors
    ///
    /// Propagates any error from rolling the model or evaluating the formula.
    pub fn smooth_gradient(
        &self,
        model: &impl SystemModel,
        input: &[f64],
        config: SmoothConfig,
    ) -> Result<(f64, Vec<f64>)> {
        let score = |u: &[f64]| self.smooth_robustness(&model.rollout(u)?, config);
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