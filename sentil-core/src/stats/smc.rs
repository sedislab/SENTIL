//! Monte Carlo statistical model checking.

#![allow(
    clippy::cast_precision_loss,
    reason = "sample counts stay far below 2^53, so the count-to-float cast is exact"
)]

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use super::confidence::{wilson_interval, ConfidenceInterval};
use super::lifting::LiftingRegistry;
use crate::error::Result;
use crate::formula::{Formula, ProbabilityOp};
use crate::signal::Trace;

/// How a statistical check is run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmcConfig {
    /// How many noisy realizations to draw.
    pub samples: u64,
    /// The confidence level for the reported interval, such as `0.95`.
    pub confidence: f64,
    /// The base seed.
    pub seed: u64,
}

impl Default for SmcConfig {
    fn default() -> Self {
        Self {
            samples: 1_000,
            confidence: 0.95,
            seed: 42,
        }
    }
}

/// The outcome of a statistical check.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmcResult {
    /// The estimated satisfaction probability.
    pub probability: f64,
    /// A confidence interval around the estimate.
    pub interval: ConfidenceInterval,
    /// How many realizations satisfied the inner formula.
    pub satisfactions: u64,
    /// How many realizations were drawn.
    pub samples: u64,
    /// Whether the estimate meets the operator's threshold.
    pub holds: bool,
}

pub(crate) fn check(
    op: ProbabilityOp,
    threshold: f64,
    inner: &Formula,
    trace: &Trace,
    lifting: &LiftingRegistry,
    config: &SmcConfig,
) -> Result<SmcResult> {
    let mut satisfactions = 0u64;
    for i in 0..config.samples {
        let mut rng = ChaCha8Rng::seed_from_u64(config.seed.wrapping_add(i));
        let noisy = lifting.lift_with(trace, &mut rng)?;
        if inner.robustness(&noisy)? > 0.0 {
            satisfactions += 1;
        }
    }
    let samples = config.samples;
    let probability = satisfactions as f64 / samples as f64;
    let interval = wilson_interval(satisfactions, samples, config.confidence);
    Ok(SmcResult {
        probability,
        interval,
        satisfactions,
        samples,
        holds: decides(op, probability, threshold),
    })
}

/// Whether an estimated probability meets the operator's threshold.
fn decides(op: ProbabilityOp, probability: f64, threshold: f64) -> bool {
    match op {
        ProbabilityOp::GreaterEqual => probability >= threshold,
        ProbabilityOp::Greater => probability > threshold,
        ProbabilityOp::LessEqual => probability <= threshold,
        ProbabilityOp::Less => probability < threshold,
    }
}