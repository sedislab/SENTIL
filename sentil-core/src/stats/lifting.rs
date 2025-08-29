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
    /// use sentil::{Formula, LiftingRegistry, NoiseInteraction, NoiseModel, Trace};
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

    pub(crate) fn lift_into<R: Rng + ?Sized>(
        &self,
        source: &Trace,
        rng: &mut R,
        dest: &mut Trace,
    ) -> Result<()> {
        for (name, values) in source.signals() {
            if let Some((model, interaction)) = self.models.get(name) {
                dest.refill_signal(
                    name,
                    values.iter().map(|&v| interaction.apply(v, model.sample(rng))),
                )?;
            }
        }
        Ok(())
    }

    pub(crate) fn model_for(&self, variable: &str) -> Option<(&NoiseModel, NoiseInteraction)> {
        self.models
            .get(variable)
            .map(|(model, kind)| (model, *kind))
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "Dirac noise gives exact, predictable shifts"
    )]

    use super::*;

    fn base() -> Trace {
        let mut trace = Trace::new([0.0, 1.0, 2.0]).unwrap();
        trace.add_signal("x", [10.0, 20.0, 30.0]).unwrap();
        trace.add_signal("y", [1.0, 2.0, 3.0]).unwrap();
        trace
    }

    #[test]
    fn additive_dirac_shifts_only_the_registered_signal() {
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::dirac(5.0).unwrap(),
            NoiseInteraction::Additive,
        );
        let noisy = lifting.lift(&base(), 1).unwrap();
        assert_eq!(noisy.signals()["x"], vec![15.0, 25.0, 35.0]);
        assert_eq!(noisy.signals()["y"], vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn multiplicative_dirac_scales_the_signal() {
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::dirac(2.0).unwrap(),
            NoiseInteraction::Multiplicative,
        );
        let noisy = lifting.lift(&base(), 1).unwrap();
        assert_eq!(noisy.signals()["x"], vec![20.0, 40.0, 60.0]);
    }

    #[test]
    fn lifting_is_reproducible_from_a_seed() {
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 1.0).unwrap(),
            NoiseInteraction::Additive,
        );
        let first = lifting.lift(&base(), 99).unwrap();
        let second = lifting.lift(&base(), 99).unwrap();
        assert_eq!(first.signals()["x"], second.signals()["x"]);
    }

    #[test]
    fn an_empty_registry_is_the_identity() {
        let lifting = LiftingRegistry::new();
        assert!(lifting.is_empty());
        let noisy = lifting.lift(&base(), 1).unwrap();
        assert_eq!(noisy.signals()["x"], vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn lift_into_matches_a_fresh_lift_for_multiple_signals() {
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 1.0).unwrap(),
            NoiseInteraction::Additive,
        );
        lifting.register(
            "y",
            NoiseModel::gaussian(0.0, 2.0).unwrap(),
            NoiseInteraction::Multiplicative,
        );
        let source = base();
        let mut fresh_rng = ChaCha8Rng::seed_from_u64(123);
        let fresh = lifting.lift_with(&source, &mut fresh_rng).unwrap();
        let mut reuse_rng = ChaCha8Rng::seed_from_u64(123);
        let mut dest = source.clone();
        lifting.lift_into(&source, &mut reuse_rng, &mut dest).unwrap();
        assert_eq!(fresh.signals()["x"], dest.signals()["x"]);
        assert_eq!(fresh.signals()["y"], dest.signals()["y"]);
    }
}