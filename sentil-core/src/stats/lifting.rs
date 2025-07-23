//! Stochastic signal lifting.

use std::collections::BTreeMap;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::noise::{NoiseInteraction, NoiseModel};
use crate::error::Result;
use crate::signal::Trace;

/// A mapping from signals to the noise that perturbs them.
#[derive(Debug, Clone, Default)]
pub struct LiftingRegistry {
    models: BTreeMap<String, (NoiseModel, NoiseInteraction)>,
}

impl LiftingRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches a noise model to a signal.
    pub fn register(
        &mut self,
        variable: &str,
        model: NoiseModel,
        interaction: NoiseInteraction,
    ) -> &mut Self {
        self.models
            .insert(variable.to_owned(), (model, interaction));
        self
    }

    /// The signals that carry a noise model, in sorted order.
    pub fn variables(&self) -> Vec<&str> {
        self.models.keys().map(String::as_str).collect()
    }

    /// Whether no signal has a noise model.
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// Produces one noisy realization of `trace`, seeded with `seed`.
    ///
    /// ```
    /// use sentil::{Formula, Trace, stats::{LiftingRegistry, NoiseModel, NoiseInteraction}};
    ///
    /// let mut trace = Trace::new([0.0, 1.0])?;
    /// trace.add_signal("x", [10.0, 20.0])?;
    /// let mut lifting = LiftingRegistry::new();
    /// lifting.register("x", NoiseModel::dirac(0.5)?, NoiseInteraction::Additive);
    /// let noisy = lifting.lift(&trace, 7)?;
    /// assert_eq!(Formula::parse("always[0, 1](x > 10)")?.robustness(&noisy)?, 0.5);
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if a lifted value is not finite.
    pub fn lift(&self, trace: &Trace, seed: u64) -> Result<Trace> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        self.lift_with(trace, &mut rng)
    }

    pub(crate) fn lift_with<R: Rng + ?Sized>(&self, trace: &Trace, rng: &mut R) -> Result<Trace> {
        let mut noisy = Trace::new(trace.times().to_vec())?;
        for (name, values) in trace.signals() {
            let lifted: Vec<f64> = match self.models.get(name) {
                Some((model, interaction)) => values
                    .iter()
                    .map(|&v| interaction.apply(v, model.sample(rng)))
                    .collect(),
                None => values.clone(),
            };
            noisy.add_signal(name, lifted)?;
        }
        Ok(noisy)
    }
}