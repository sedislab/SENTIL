//! Wald's sequential probability ratio test.

use crate::error::{Error, Result};

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