//! Wald's sequential probability ratio test.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use super::lifting::LiftingRegistry;
use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::signal::Trace;

const DEFAULT_SEED: u64 = 42;

#[cfg(feature = "serde")]
fn default_seed() -> u64 {
    DEFAULT_SEED
}

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

/// The SPRT parameters.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "RawSprtConfig"))]
pub struct SprtConfig {
    p0: f64,
    p1: f64,
    alpha: f64,
    beta: f64,
    max_samples: u64,
    #[cfg_attr(feature = "serde", serde(default = "default_seed"))]
    seed: u64,
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
struct RawSprtConfig {
    p0: f64,
    p1: f64,
    alpha: f64,
    beta: f64,
    max_samples: u64,
    #[serde(default = "default_seed")]
    seed: u64,
}

#[cfg(feature = "serde")]
impl TryFrom<RawSprtConfig> for SprtConfig {
    type Error = String;

    fn try_from(raw: RawSprtConfig) -> core::result::Result<Self, Self::Error> {
        Self::new(raw.p0, raw.p1, raw.alpha, raw.beta, raw.max_samples)
            .map(|c| c.with_seed(raw.seed))
            .map_err(|e| e.to_string())
    }
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
            seed: DEFAULT_SEED,
        })
    }

    /// Sets the base seed for [`Formula::check_sequential`].
    #[must_use]
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// The base seed used to draw realizations.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
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
    let on_satisfied = config.p1.ln() - config.p0.ln();
    let on_unsatisfied = (1.0 - config.p1).ln() - (1.0 - config.p0).ln();

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
            let mut rng = ChaCha8Rng::seed_from_u64(config.seed.wrapping_add(n));
            n += 1;
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
    #[allow(clippy::cast_precision_loss, reason = "run counts are small, exact in f64")]
    fn the_error_rates_stay_within_the_bounds() {
        use rand::Rng;
        let cfg = SprtConfig::new(0.3, 0.7, 0.05, 0.05, 5000).unwrap();
        let runs = 400u64;
        let mut type_i = 0u64;
        let mut type_ii = 0u64;
        for r in 0..runs {
            let mut rng = ChaCha8Rng::seed_from_u64(r);
            if let Ok(SprtResult::AcceptH1 { .. }) =
                sequential_test(&cfg, || Ok(rng.random::<f64>() < 0.2))
            {
                type_i += 1;
            }
            let mut rng = ChaCha8Rng::seed_from_u64(1000 + r);
            if let Ok(SprtResult::AcceptH0 { .. }) =
                sequential_test(&cfg, || Ok(rng.random::<f64>() < 0.8))
            {
                type_ii += 1;
            }
        }
        let type_i_rate = type_i as f64 / runs as f64;
        let type_ii_rate = type_ii as f64 / runs as f64;
        assert!(type_i_rate <= 0.1, "type I rate {type_i_rate}");
        assert!(type_ii_rate <= 0.1, "type II rate {type_ii_rate}");
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

    fn samples_of(result: SprtResult) -> u64 {
        match result {
            SprtResult::AcceptH0 { samples }
            | SprtResult::AcceptH1 { samples }
            | SprtResult::Inconclusive { samples, .. } => samples,
        }
    }

    #[test]
    fn the_seed_varies_the_realization_stream() {
        let phi = Formula::parse("P>=0.5(x > 0)").unwrap();
        let (trace, lifting) = additive_gaussian(0.2);
        let counts: Vec<u64> = (1..=6)
            .map(|s| {
                samples_of(
                    phi.check_sequential(&trace, &lifting, &config().with_seed(s))
                        .unwrap(),
                )
            })
            .collect();
        assert!(
            counts.iter().any(|&c| c != counts[0]),
            "the seed must vary the draws, got {counts:?}"
        );
    }

    #[test]
    fn the_same_seed_reproduces_the_run() {
        let phi = Formula::parse("P>=0.5(x > 0)").unwrap();
        let (trace, lifting) = additive_gaussian(0.2);
        let cfg = config().with_seed(7);
        let a = phi.check_sequential(&trace, &lifting, &cfg).unwrap();
        let b = phi.check_sequential(&trace, &lifting, &cfg).unwrap();
        assert_eq!(a, b);
    }

    #[cfg(feature = "serde")]
    #[test]
    #[allow(clippy::float_cmp, reason = "the round trip preserves the exact values")]
    fn valid_config_round_trips_through_json() {
        let config = SprtConfig::new(0.3, 0.7, 0.05, 0.1, 500).unwrap();
        let json = serde_json::to_string(&config).unwrap();
        let back: SprtConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.p0(), 0.3);
        assert_eq!(back.p1(), 0.7);
        assert_eq!(back.alpha(), 0.05);
        assert_eq!(back.beta(), 0.1);
        assert_eq!(back.max_samples(), 500);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn deserialization_rejects_invalid_shapes() {
        let inverted = r#"{"p0":0.6,"p1":0.4,"alpha":0.05,"beta":0.05,"max_samples":100}"#;
        let zero_errors = r#"{"p0":0.4,"p1":0.6,"alpha":0.0,"beta":0.0,"max_samples":100}"#;
        let no_budget = r#"{"p0":0.4,"p1":0.6,"alpha":0.05,"beta":0.05,"max_samples":0}"#;
        assert!(serde_json::from_str::<SprtConfig>(inverted).is_err());
        assert!(serde_json::from_str::<SprtConfig>(zero_errors).is_err());
        assert!(serde_json::from_str::<SprtConfig>(no_budget).is_err());
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