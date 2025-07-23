//! Noise models for stochastic signal lifting.

use rand::Rng;
use rand_distr::StandardNormal;

use crate::error::{Error, Result};

/// How a noise draw combines with a deterministic reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseInteraction {
    /// The noise is added to the reading: `reading + noise`.
    Additive,
    /// The noise scales the reading: `reading * noise`.
    Multiplicative,
}

impl NoiseInteraction {
    /// Combines a reading with a noise draw.
    pub fn apply(self, reading: f64, noise: f64) -> f64 {
        match self {
            NoiseInteraction::Additive => reading + noise,
            NoiseInteraction::Multiplicative => reading * noise,
        }
    }
}

/// A probability distribution that sensor noise is drawn from.
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseModel {
    kind: Kind,
}

#[derive(Debug, Clone, PartialEq)]
enum Kind {
    Dirac { value: f64 },
    Gaussian { mean: f64, std_dev: f64 },
    Uniform { low: f64, high: f64 },
}

impl NoiseModel {
    /// A point mass at `value`.
    ///
    /// # Errors
    ///
    /// Returns an error if `value` is not finite.
    pub fn dirac(value: f64) -> Result<Self> {
        finite("Dirac", "value", value)?;
        Ok(Self {
            kind: Kind::Dirac { value },
        })
    }

    /// A normal distribution.
    ///
    /// # Errors
    ///
    /// Returns an error if a parameter is not finite or the standard deviation is negative.
    pub fn gaussian(mean: f64, std_dev: f64) -> Result<Self> {
        finite("Gaussian", "mean", mean)?;
        finite("Gaussian", "standard deviation", std_dev)?;
        if std_dev < 0.0 {
            return Err(invalid(
                "Gaussian",
                format!("standard deviation must be non-negative, got {std_dev}"),
            ));
        }
        Ok(Self {
            kind: Kind::Gaussian { mean, std_dev },
        })
    }

    /// A uniform distribution over `[low, high]`.
    ///
    /// # Errors
    ///
    /// Returns an error if a bound is not finite or `low` exceeds `high`.
    pub fn uniform(low: f64, high: f64) -> Result<Self> {
        finite("Uniform", "lower bound", low)?;
        finite("Uniform", "upper bound", high)?;
        if low > high {
            return Err(invalid(
                "Uniform",
                format!("lower bound {low} exceeds upper bound {high}"),
            ));
        }
        Ok(Self {
            kind: Kind::Uniform { low, high },
        })
    }

    /// Draws one value from the distribution.
    pub fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        match self.kind {
            Kind::Dirac { value } => value,
            Kind::Gaussian { mean, std_dev } => {
                let z: f64 = rng.sample(StandardNormal);
                mean + std_dev * z
            }
            Kind::Uniform { low, high } => low + (high - low) * rng.random::<f64>(),
        }
    }
}

fn finite(model: &'static str, name: &str, value: f64) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(invalid(
            model,
            format!("{name} must be finite, got {value}"),
        ))
    }
}

fn invalid(model: &'static str, reason: String) -> Error {
    Error::InvalidNoiseModel { model, reason }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the degenerate cases produce exact values"
    )]

    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn dirac_is_deterministic() {
        let mut rng = StdRng::seed_from_u64(1);
        let model = NoiseModel::dirac(3.5).unwrap();
        assert_eq!(model.sample(&mut rng), 3.5);
        assert_eq!(model.sample(&mut rng), 3.5);
    }

    #[test]
    fn zero_variance_gaussian_returns_the_mean() {
        let mut rng = StdRng::seed_from_u64(2);
        let model = NoiseModel::gaussian(7.0, 0.0).unwrap();
        assert_eq!(model.sample(&mut rng), 7.0);
    }

    #[test]
    fn uniform_stays_within_its_bounds() {
        let mut rng = StdRng::seed_from_u64(3);
        let model = NoiseModel::uniform(-2.0, 5.0).unwrap();
        for _ in 0..1000 {
            let x = model.sample(&mut rng);
            assert!((-2.0..=5.0).contains(&x));
        }
    }

    #[test]
    fn sampling_is_reproducible_from_a_seed() {
        let model = NoiseModel::gaussian(0.0, 1.0).unwrap();
        let mut a = StdRng::seed_from_u64(42);
        let mut b = StdRng::seed_from_u64(42);
        for _ in 0..100 {
            assert_eq!(model.sample(&mut a), model.sample(&mut b));
        }
    }

    #[test]
    fn invalid_parameters_are_rejected() {
        assert!(matches!(
            NoiseModel::gaussian(0.0, -1.0),
            Err(Error::InvalidNoiseModel {
                model: "Gaussian",
                ..
            })
        ));
        assert!(matches!(
            NoiseModel::uniform(5.0, 1.0),
            Err(Error::InvalidNoiseModel {
                model: "Uniform",
                ..
            })
        ));
        assert!(NoiseModel::dirac(f64::NAN).is_err());
    }

    #[test]
    fn interaction_combines_reading_and_noise() {
        assert_eq!(NoiseInteraction::Additive.apply(10.0, 3.0), 13.0);
        assert_eq!(NoiseInteraction::Multiplicative.apply(10.0, 3.0), 30.0);
    }
}