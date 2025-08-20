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
            samples: 10_000,
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
    /// Whether the point estimate meets the operator's threshold.
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
    #[cfg(feature = "gpu")]
    if let Some(result) = try_gpu_check(op, threshold, inner, trace, lifting, config) {
        return Ok(result);
    }

    // Sample `i` is seeded independently, so the count is the same however the
    // samples are scheduled, and a robustness of exactly zero counts as
    // satisfied, matching `Robustness::is_satisfied`.
    let satisfies = |i: u64| -> Result<bool> {
        let mut rng = ChaCha8Rng::seed_from_u64(config.seed.wrapping_add(i));
        let noisy = lifting.lift_with(trace, &mut rng)?;
        Ok(inner.robustness(&noisy)? >= 0.0)
    };

    #[cfg(feature = "parallel")]
    let satisfactions = {
        use rayon::prelude::*;
        (0..config.samples)
            .into_par_iter()
            .map(|i| satisfies(i).map(u64::from))
            .try_reduce(|| 0, |a, b| Ok(a + b))?
    };
    #[cfg(not(feature = "parallel"))]
    let satisfactions = {
        let mut count = 0u64;
        for i in 0..config.samples {
            count += u64::from(satisfies(i)?);
        }
        count
    };

    let samples = config.samples;
    let probability = satisfactions as f64 / samples as f64;
    let interval = wilson_interval(satisfactions, samples, config.confidence);
    Ok(SmcResult {
        probability,
        interval,
        satisfactions,
        samples,
        holds: super::decides(op, probability, threshold),
    })
}

/// The smallest sample count that repays the per-call device and shader setup.
#[cfg(feature = "gpu")]
const GPU_MIN_SAMPLES: u64 = 100_000;

/// Tries to run the check on the GPU, returning `None` to fall back to the CPU.
///
/// The GPU runs only when it gives the same answer faster: the formula is
/// atemporal and transpilable, mentions at least one variable, every noise family
/// has a GPU sampler, the base reading projects onto the variables, the run is
/// large enough to amortize setup, and a device is present. Any miss, and any
/// device error, falls back silently.
#[cfg(feature = "gpu")]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "samples and counts stay below 2^24, and the seed narrowing is intentional"
)]
fn try_gpu_check(
    op: ProbabilityOp,
    threshold: f64,
    inner: &Formula,
    trace: &Trace,
    lifting: &LiftingRegistry,
    config: &SmcConfig,
) -> Option<SmcResult> {
    if config.samples < GPU_MIN_SAMPLES {
        return None;
    }
    let symbols = inner.variables();
    let (shader, state_size) = crate::gpu::build_count_shader(inner, &symbols).ok()?;
    if state_size == 0 {
        return None;
    }
    let noise = crate::gpu::pack_noise_params(&symbols, lifting).ok()?;
    let base = base_state_f32(trace, &symbols)?;
    let context = crate::gpu::GpuMcContext::new(&shader).ok()?;
    let satisfactions = context
        .gpu_satisfaction_count(&base, &noise, config.samples, config.seed as u32)
        .ok()?;
    let samples = config.samples;
    let probability = satisfactions as f64 / samples as f64;
    Some(SmcResult {
        probability,
        interval: wilson_interval(satisfactions, samples, config.confidence),
        satisfactions,
        samples,
        holds: super::decides(op, probability, threshold),
    })
}

/// The first reading of each variable, in `symbols` order.
#[cfg(feature = "gpu")]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the GPU evaluates in f32 by contract"
)]
fn base_state_f32(trace: &Trace, symbols: &[String]) -> Option<Vec<f32>> {
    symbols
        .iter()
        .map(|name| {
            trace
                .signals()
                .get(name)
                .and_then(|values| values.first())
                .map(|&v| v as f32)
        })
        .collect()
}