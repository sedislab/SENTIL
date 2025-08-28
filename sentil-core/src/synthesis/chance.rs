//! Validating a chance constraint by Monte Carlo.
//!
//! A chance constraint asks that a specification hold with at least some target
//! probability over a stochastic system. This validates one the conservative way:
//! it simulates the system, counts the runs the formula holds on, and requires the
//! Wilson lower confidence bound, not the point estimate, to clear the target. An
//! optional tightening raises the bar further, turning a probabilistic guarantee
//! into a risk margin a synthesized controller can be checked against as it runs.

#![allow(
    clippy::cast_precision_loss,
    reason = "sample counts stay far below 2^53, so the count-to-float cast is exact"
)]

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::stats::{wilson_interval, StochasticSystem};

/// A requirement that `formula` hold with probability at least `probability` over a
/// stochastic system.
pub struct ChanceConstraint {
    formula: Formula,
    probability: f64,
    level: f64,
    tightening: f64,
}

/// The outcome of validating a [`ChanceConstraint`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChanceReport {
    /// The point estimate of the satisfaction probability.
    pub estimate: f64,
    /// The Wilson lower confidence bound the verdict is taken from.
    pub lower_bound: f64,
    /// The number of trajectories simulated.
    pub samples: u64,
    /// Whether the lower bound clears the target plus any tightening.
    pub holds: bool,
}

impl ChanceConstraint {
    /// A constraint that `formula` hold with probability at least `probability`, at
    /// a default 95% confidence and no extra tightening.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] unless `probability` lies in `[0, 1]`.
    pub fn new(formula: Formula, probability: f64) -> Result<Self> {
        if !(0.0..=1.0).contains(&probability) {
            return Err(config_error(format!(
                "target probability {probability} must be in [0, 1]"
            )));
        }
        Ok(Self {
            formula,
            probability,
            level: 0.95,
            tightening: 0.0,
        })
    }

    /// Sets the confidence level of the lower bound, such as `0.99`.
    #[must_use]
    pub fn with_confidence(mut self, level: f64) -> Self {
        self.level = level;
        self
    }

    /// Adds a conservative margin to the target, so the lower bound must clear
    /// `probability + tightening`.
    #[must_use]
    pub fn with_tightening(mut self, tightening: f64) -> Self {
        self.tightening = tightening;
        self
    }

    /// Simulates `samples` trajectories of `system` from `seed` and reports whether
    /// the constraint holds conservatively.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if `samples` is zero or the confidence level
    /// is not in `(0, 1)`, and propagates any error from simulating the system or
    /// scoring the formula.
    pub fn validate(
        &self,
        system: &StochasticSystem,
        samples: u64,
        seed: u64,
    ) -> Result<ChanceReport> {
        if samples == 0 {
            return Err(config_error("samples must be positive".to_owned()));
        }
        if !(0.0 < self.level && self.level < 1.0) {
            return Err(config_error(format!(
                "confidence level {} must be in (0, 1)",
                self.level
            )));
        }

        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut successes = 0u64;
        for _ in 0..samples {
            let trace = system.simulate(&mut rng)?;
            if self.formula.robustness(&trace)? >= 0.0 {
                successes += 1;
            }
        }
        let interval = wilson_interval(successes, samples, self.level);
        Ok(ChanceReport {
            estimate: successes as f64 / samples as f64,
            lower_bound: interval.lower,
            samples,
            holds: interval.lower >= self.probability + self.tightening,
        })
    }
}

fn config_error(message: String) -> Error {
    Error::InvalidConfig {
        context: "chance constraint",
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_distr::{Distribution, StandardNormal};

    fn standard_normal_start() -> StochasticSystem {
        StochasticSystem::new(
            ["x"],
            1.0,
            2,
            |rng| vec![StandardNormal.sample(rng)],
            |prev, _t, _rng| prev.to_vec(),
        )
        .unwrap()
    }

    #[test]
    fn a_reachable_target_holds_on_the_lower_bound() {
        let phi = Formula::parse("x > 0").unwrap();
        let report = ChanceConstraint::new(phi, 0.4)
            .unwrap()
            .validate(&standard_normal_start(), 4000, 7)
            .unwrap();
        assert!(
            (report.estimate - 0.5).abs() < 0.05,
            "estimate {}",
            report.estimate
        );
        assert!(report.holds);
    }

    #[test]
    fn a_target_above_the_lower_bound_fails() {
        let phi = Formula::parse("x > 0").unwrap();
        let report = ChanceConstraint::new(phi, 0.55)
            .unwrap()
            .validate(&standard_normal_start(), 4000, 7)
            .unwrap();
        assert!(!report.holds);
    }

    #[test]
    fn an_out_of_range_target_is_rejected() {
        assert!(ChanceConstraint::new(Formula::parse("x > 0").unwrap(), 1.5).is_err());
    }

    #[test]
    fn zero_samples_is_rejected() {
        let constraint = ChanceConstraint::new(Formula::parse("x > 0").unwrap(), 0.5).unwrap();
        assert!(constraint.validate(&standard_normal_start(), 0, 1).is_err());
    }
}