//! Bayesian sequential model checking.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use super::confidence::regularized_incomplete_beta;
use super::lifting::LiftingRegistry;
use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::signal::Trace;

/// The outcome of a Bayesian sequential test against a probability threshold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BayesResult {
    /// The probability is at least the threshold, decided after `samples` draws.
    Holds {
        /// Samples drawn before deciding.
        samples: u64,
        /// The posterior probability that the threshold is met.
        posterior: f64,
    },
    /// The probability is below the threshold, decided after `samples` draws.
    Fails {
        /// Samples drawn before deciding.
        samples: u64,
        /// The posterior probability that the threshold is met.
        posterior: f64,
    },
    /// The Bayes factor stayed indecisive within the sample budget.
    Inconclusive {
        /// The number of samples drawn.
        samples: u64,
        /// The posterior probability that the threshold is met.
        posterior: f64,
    },
}

/// The test parameters: the probability `threshold`, the Bayes-factor cutoff that
/// makes a decision, and the sample cap. The prior is the uniform Beta(1, 1).
#[derive(Debug, Clone, Copy)]
pub struct BayesConfig {
    threshold: f64,
    bayes_factor: f64,
    max_samples: u64,
}

impl BayesConfig {
    /// Builds a configuration.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] unless `threshold` is in `(0, 1)`, `bayes_factor` exceeds one, and `max_samples` is positive.
    pub fn new(threshold: f64, bayes_factor: f64, max_samples: u64) -> Result<Self> {
        if !(threshold.is_finite() && 0.0 < threshold && threshold < 1.0) {
            return Err(config_error(format!(
                "threshold must be in (0, 1), got {threshold}"
            )));
        }
        if !(bayes_factor.is_finite() && bayes_factor > 1.0) {
            return Err(config_error(format!(
                "bayes_factor must exceed 1, got {bayes_factor}"
            )));
        }
        if max_samples == 0 {
            return Err(config_error("max_samples must be positive".to_owned()));
        }
        Ok(Self {
            threshold,
            bayes_factor,
            max_samples,
        })
    }
}

/// Runs the test over a Bernoulli source, calling `draw` for each fresh sample.
///
/// The posterior over the satisfaction probability is Beta(1 + successes,
/// 1 + failures). The test weighs the posterior mass above the threshold against
/// the mass below it; once one outweighs the other by the configured Bayes factor
/// it decides. The comparison is written without division, so a posterior mass of
/// zero is handled cleanly.
///
/// # Errors
///
/// Propagates any error returned by `draw`.
#[allow(
    clippy::cast_precision_loss,
    reason = "sample counts stay far below 2^53, so the count-to-float cast is exact"
)]
pub fn bayes_sequential_test<F>(config: &BayesConfig, mut draw: F) -> Result<BayesResult>
where
    F: FnMut() -> Result<bool>,
{
    let mut successes = 0u64;
    for n in 1..=config.max_samples {
        if draw()? {
            successes += 1;
        }
        let a = 1.0 + successes as f64;
        let b = 1.0 + (n - successes) as f64;
        let below = regularized_incomplete_beta(a, b, config.threshold);
        let posterior = 1.0 - below;
        if posterior >= config.bayes_factor * below {
            return Ok(BayesResult::Holds { samples: n, posterior });
        }
        if below >= config.bayes_factor * posterior {
            return Ok(BayesResult::Fails { samples: n, posterior });
        }
    }
    let a = 1.0 + successes as f64;
    let b = 1.0 + (config.max_samples - successes) as f64;
    let posterior = 1.0 - regularized_incomplete_beta(a, b, config.threshold);
    Ok(BayesResult::Inconclusive {
        samples: config.max_samples,
        posterior,
    })
}

impl Formula {
    /// Decides a probabilistic specification with Bayesian sequential testing.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotProbabilistic`] if the formula is not probabilistic and [`Error::EmptyTrace`] for an empty trace.
    pub fn check_bayesian(
        &self,
        trace: &Trace,
        lifting: &LiftingRegistry,
        config: &BayesConfig,
    ) -> Result<BayesResult> {
        let Formula::Probabilistic(_, _, inner) = self else {
            return Err(Error::NotProbabilistic);
        };
        if trace.is_empty() {
            return Err(Error::EmptyTrace);
        }
        let mut buf = trace.clone();
        let mut n = 0u64;
        bayes_sequential_test(config, || {
            n += 1;
            let mut rng = ChaCha8Rng::seed_from_u64(n);
            lifting.lift_into(trace, &mut rng, &mut buf)?;
            Ok(inner.robustness(&buf)? >= 0.0)
        })
    }
}

fn config_error(message: String) -> Error {
    Error::InvalidConfig {
        context: "Bayesian SMC",
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::{LiftingRegistry, NoiseInteraction, NoiseModel};

    fn config() -> BayesConfig {
        BayesConfig::new(0.5, 100.0, 5000).unwrap()
    }

    fn additive_gaussian(x: f64) -> (Trace, LiftingRegistry) {
        let trace = Trace::from_signal([0.0], "x", [x]).unwrap();
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 0.5).unwrap(),
            NoiseInteraction::Additive,
        );
        (trace, lifting)
    }

    #[test]
    fn a_constant_source_decides_each_way() {
        assert!(matches!(
            bayes_sequential_test(&config(), || Ok(true)),
            Ok(BayesResult::Holds { .. })
        ));
        assert!(matches!(
            bayes_sequential_test(&config(), || Ok(false)),
            Ok(BayesResult::Fails { .. })
        ));
    }

    #[test]
    fn check_bayesian_decides_clear_properties_early() {
        let phi = Formula::parse("P>=0.5(x > 0)").unwrap();
        let (trace, lifting) = additive_gaussian(5.0);
        let held = phi.check_bayesian(&trace, &lifting, &config()).unwrap();
        assert!(matches!(held, BayesResult::Holds { samples, .. } if samples < 5000));

        let (trace, lifting) = additive_gaussian(-5.0);
        let failed = phi.check_bayesian(&trace, &lifting, &config()).unwrap();
        assert!(matches!(failed, BayesResult::Fails { .. }));
    }

    #[test]
    fn config_rejects_bad_parameters() {
        assert!(BayesConfig::new(0.0, 100.0, 100).is_err());
        assert!(BayesConfig::new(0.5, 1.0, 100).is_err());
        assert!(BayesConfig::new(0.5, 100.0, 0).is_err());
    }

    #[test]
    fn non_probabilistic_is_rejected() {
        let phi = Formula::parse("x > 0").unwrap();
        let (trace, lifting) = additive_gaussian(1.0);
        assert!(matches!(
            phi.check_bayesian(&trace, &lifting, &config()),
            Err(Error::NotProbabilistic)
        ));
    }
}