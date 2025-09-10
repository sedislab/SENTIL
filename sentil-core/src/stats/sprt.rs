//! Wald's sequential probability ratio test.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use super::lifting::LiftingRegistry;
use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::signal::Trace;

/// The outcome of a sequential test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SprtResult {
    /// Accepted `H0` (`p <= p0`) after `samples` draws.
    AcceptH0 {
        /// Samples drawn before deciding.
        samples: u64,
    },
    /// Accepted `H1` (`p >= p1`) after `samples` draws.
    AcceptH1 {
        /// Samples drawn before deciding.
        samples: u64,
    },
    /// Neither hypothesis was reached within the sample budget.
    Inconclusive {
        /// The number of samples drawn.
        samples: u64,
        /// The final log-likelihood ratio.
        log_likelihood: f64,
    },
}

/// The test parameters: the indifference region `(p0, p1)`, the error rates, and
/// the sample cap.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SprtConfig {
    p0: f64,
    p1: f64,
    alpha: f64,
    beta: f64,
    max_samples: u64,
}

impl SprtConfig {
    /// Builds a configuration.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] unless `0 < p0 < p1 < 1`, both error rates are in `(0, 1)`, and `max_samples` is positive.
    pub fn new(p0: f64, p1: f64, alpha: f64, beta: f64, max_samples: u64) -> Result<Self> {
        unit("p0", p0)?;
        unit("p1", p1)?;
        if p0 >= p1 {
            return Err(config_error(format!("p0 ({p0}) must be below p1 ({p1})")));
        }
        unit("alpha", alpha)?;
        unit("beta", beta)?;
        if max_samples == 0 {
            return Err(config_error("max_samples must be positive".to_owned()));
        }
        Ok(Self {
            p0,
            p1,
            alpha,
            beta,
            max_samples,
        })
    }

    /// The lower bound `p0` of the indifference region.
    #[must_use]
    pub fn p0(&self) -> f64 {
        self.p0
    }

    /// The upper bound `p1` of the indifference region.
    #[must_use]
    pub fn p1(&self) -> f64 {
        self.p1
    }

    /// The bound on the false-positive rate.
    #[must_use]
    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    /// The bound on the false-negative rate.
    #[must_use]
    pub fn beta(&self) -> f64 {
        self.beta
    }

    /// The sample budget.
    #[must_use]
    pub fn max_samples(&self) -> u64 {
        self.max_samples
    }
}

/// Runs the test over a Bernoulli source, calling `draw` for each fresh sample.
///
/// # Errors
///
/// Propagates any error returned by `draw`.
pub fn sequential_test<F>(config: &SprtConfig, mut draw: F) -> Result<SprtResult>
where
    F: FnMut() -> Result<bool>,
{
    let accept_h0 = (config.beta / (1.0 - config.alpha)).ln();
    let accept_h1 = ((1.0 - config.beta) / config.alpha).ln();
    let on_satisfied = (config.p1 / config.p0).ln();
    let on_unsatisfied = ((1.0 - config.p1) / (1.0 - config.p0)).ln();

    let mut log_likelihood = 0.0;
    for n in 1..=config.max_samples {
        log_likelihood += if draw()? {
            on_satisfied
        } else {
            on_unsatisfied
        };
        if log_likelihood >= accept_h1 {
            return Ok(SprtResult::AcceptH1 { samples: n });
        }
        if log_likelihood <= accept_h0 {
            return Ok(SprtResult::AcceptH0 { samples: n });
        }
    }
    Ok(SprtResult::Inconclusive {
        samples: config.max_samples,
        log_likelihood,
    })
}

impl Formula {
    /// Decides a probabilistic specification sequentially.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotProbabilistic`] if the formula is not probabilistic.
    pub fn check_sequential(
        &self,
        trace: &Trace,
        lifting: &LiftingRegistry,
        config: &SprtConfig,
    ) -> Result<SprtResult> {
        let Formula::Probabilistic(_, _, inner) = self else {
            return Err(Error::NotProbabilistic);
        };
        if trace.is_empty() {
            return Err(Error::EmptyTrace);
        }
        let mut buf = trace.clone();
        let mut n = 0u64;
        sequential_test(config, || {
            n += 1;
            let mut rng = ChaCha8Rng::seed_from_u64(n);
            lifting.lift_into(trace, &mut rng, &mut buf)?;
            Ok(inner.robustness(&buf)? >= 0.0)
        })
    }
}

fn unit(name: &str, value: f64) -> Result<()> {
    if value.is_finite() && value > 0.0 && value < 1.0 {
        Ok(())
    } else {
        Err(config_error(format!(
            "{name} must be in (0, 1), got {value}"
        )))
    }
}

fn config_error(message: String) -> Error {
    Error::InvalidConfig {
        context: "SPRT",
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::{LiftingRegistry, NoiseInteraction, NoiseModel};

    fn config() -> SprtConfig {
        SprtConfig::new(0.4, 0.6, 0.05, 0.05, 5000).unwrap()
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
            sequential_test(&config(), || Ok(true)),
            Ok(SprtResult::AcceptH1 { .. })
        ));
        assert!(matches!(
            sequential_test(&config(), || Ok(false)),
            Ok(SprtResult::AcceptH0 { .. })
        ));
    }

    #[test]
    fn check_sequential_decides_clear_properties_early() {
        let phi = Formula::parse("P>=0.5(x > 0)").unwrap();
        let (trace, lifting) = additive_gaussian(5.0);
        let held = phi.check_sequential(&trace, &lifting, &config()).unwrap();
        assert!(matches!(held, SprtResult::AcceptH1 { samples } if samples < 5000));

        let (trace, lifting) = additive_gaussian(-5.0);
        let failed = phi.check_sequential(&trace, &lifting, &config()).unwrap();
        assert!(matches!(failed, SprtResult::AcceptH0 { .. }));
    }

    #[test]
    fn config_rejects_bad_parameters() {
        assert!(SprtConfig::new(0.6, 0.4, 0.05, 0.05, 100).is_err());
        assert!(SprtConfig::new(0.4, 0.6, 0.0, 0.05, 100).is_err());
        assert!(SprtConfig::new(0.4, 0.6, 0.05, 0.05, 0).is_err());
    }

    #[test]
    #[allow(clippy::float_cmp, reason = "the getters return the exact values set")]
    fn config_reads_back_its_parameters() {
        let config = SprtConfig::new(0.3, 0.7, 0.05, 0.1, 500).unwrap();
        assert_eq!(config.p0(), 0.3);
        assert_eq!(config.p1(), 0.7);
        assert_eq!(config.alpha(), 0.05);
        assert_eq!(config.beta(), 0.1);
        assert_eq!(config.max_samples(), 500);
    }

    #[test]
    fn non_probabilistic_is_rejected() {
        let phi = Formula::parse("x > 0").unwrap();
        let (trace, lifting) = additive_gaussian(1.0);
        assert!(matches!(
            phi.check_sequential(&trace, &lifting, &config()),
            Err(Error::NotProbabilistic)
        ));
    }

    #[test]
    fn an_empty_trace_is_rejected() {
        let phi = Formula::parse("P>=0.5(x > 0)").unwrap();
        let trace = Trace::new(Vec::new()).unwrap();
        let lifting = LiftingRegistry::new();
        assert!(matches!(
            phi.check_sequential(&trace, &lifting, &config()),
            Err(Error::EmptyTrace)
        ));
    }
}