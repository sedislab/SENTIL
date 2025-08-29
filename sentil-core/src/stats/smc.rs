//! Monte Carlo statistical model checking.

#![allow(
    clippy::cast_precision_loss,
    reason = "sample counts stay far below 2^53, so the count-to-float cast is exact"
)]

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use super::confidence::{wilson_interval, ConfidenceInterval};
use super::lifting::LiftingRegistry;
use crate::error::{Error, Result};
use crate::formula::{Formula, ProbabilityOp};
use crate::signal::Trace;

/// How a statistical check is run.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    if trace.is_empty() {
        return Err(Error::EmptyTrace);
    }

    #[cfg(feature = "gpu")]
    if let Some(result) = try_gpu_check(op, threshold, inner, trace, lifting, config) {
        return Ok(result);
    }

    let satisfies = |i: u64, buf: &mut Trace| -> Result<u64> {
        let mut rng = ChaCha8Rng::seed_from_u64(config.seed.wrapping_add(i));
        lifting.lift_into(trace, &mut rng, buf)?;
        Ok(u64::from(inner.robustness(buf)? >= 0.0))
    };

    #[cfg(feature = "parallel")]
    let satisfactions = {
        use rayon::prelude::*;
        (0..config.samples)
            .into_par_iter()
            .map_init(|| trace.clone(), |buf, i| satisfies(i, buf))
            .try_reduce(|| 0, |a, b| Ok(a + b))?
    };
    #[cfg(not(feature = "parallel"))]
    let satisfactions = {
        let mut buf = trace.clone();
        let mut count = 0u64;
        for i in 0..config.samples {
            count += satisfies(i, &mut buf)?;
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
#[cfg(feature = "gpu")]
#[allow(
    clippy::cast_precision_loss,
    reason = "samples and counts stay below 2^24"
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
    if symbols.is_empty() {
        return None;
    }
    let noise = crate::gpu::pack_noise_params(&symbols, lifting).ok()?;
    let satisfactions = if inner.has_temporal() {
        gpu_temporal_count(inner, &symbols, trace, &noise, config)?
    } else {
        gpu_atemporal_count(inner, &symbols, trace, &noise, config)?
    };
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

/// Counts satisfying realizations for an atemporal inner formula.
#[cfg(feature = "gpu")]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the seed narrowing to u32 is intentional"
)]
fn gpu_atemporal_count(
    inner: &Formula,
    symbols: &[String],
    trace: &Trace,
    noise: &[f32],
    config: &SmcConfig,
) -> Option<u64> {
    let (shader, state_size) = crate::gpu::build_count_shader(inner, symbols).ok()?;
    if state_size == 0 {
        return None;
    }
    let base = base_state_f32(trace, symbols)?;
    let context = crate::gpu::GpuMcContext::new(&shader, false).ok()?;
    context
        .gpu_satisfaction_count(&base, noise, None, config.samples, config.seed as u32)
        .ok()
}

/// Counts satisfying realizations for a temporal inner formula.
#[cfg(feature = "gpu")]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the seed narrowing to u32 is intentional"
)]
fn gpu_temporal_count(
    inner: &Formula,
    symbols: &[String],
    trace: &Trace,
    noise: &[f32],
    config: &SmcConfig,
) -> Option<u64> {
    let trace_len = trace.times().len();
    let (shader, _) = crate::gpu::build_temporal_shader(inner, symbols, trace_len).ok()?;
    let base_trace = base_trace_f32(trace, symbols)?;
    let times = times_f32(trace)?;
    let context = crate::gpu::GpuMcContext::new(&shader, true).ok()?;
    context
        .gpu_satisfaction_count(
            &base_trace,
            noise,
            Some(&times),
            config.samples,
            config.seed as u32,
        )
        .ok()
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

/// The base trace the temporal kernel perturbs, laid out variable major as `base[v * len + i]`.
#[cfg(feature = "gpu")]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the GPU evaluates in f32 by contract"
)]
fn base_trace_f32(trace: &Trace, symbols: &[String]) -> Option<Vec<f32>> {
    let len = trace.times().len();
    let mut out = Vec::with_capacity(symbols.len() * len);
    for name in symbols {
        let values = trace.signals().get(name)?;
        if values.len() != len {
            return None;
        }
        out.extend(values.iter().map(|&v| v as f32));
    }
    Some(out)
}

/// The time grid as f32, or `None` when a timestamp is not exactly representable in f32.
#[cfg(feature = "gpu")]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the GPU evaluates in f32 by contract"
)]
#[allow(
    clippy::float_cmp,
    reason = "the exact round-trip equality is the representability test itself"
)]
fn times_f32(trace: &Trace) -> Option<Vec<f32>> {
    trace
        .times()
        .iter()
        .map(|&t| {
            let f = t as f32;
            (f64::from(f) == t).then_some(f)
        })
        .collect()
}

#[cfg(test)]
mod cpu_tests {
    use super::*;
    use crate::stats::{NoiseInteraction, NoiseModel};

    #[test]
    fn count_matches_the_fresh_lift_full_robustness_baseline() {
        let inner = Formula::parse("always[0, 2](b > 0)").unwrap();
        let mut trace = Trace::new(vec![0.0, 1.0, 2.0, 3.0, 4.0]).unwrap();
        trace.add_signal("a", vec![5.0; 5]).unwrap();
        trace.add_signal("b", vec![1.0; 5]).unwrap();
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "a",
            NoiseModel::gaussian(0.0, 1.0).unwrap(),
            NoiseInteraction::Additive,
        );
        lifting.register(
            "b",
            NoiseModel::gaussian(0.0, 1.0).unwrap(),
            NoiseInteraction::Additive,
        );
        let config = SmcConfig {
            samples: 2000,
            confidence: 0.95,
            seed: 7,
        };

        let result = check(
            ProbabilityOp::GreaterEqual,
            0.5,
            &inner,
            &trace,
            &lifting,
            &config,
        )
        .unwrap();

        let baseline: u64 = (0..config.samples)
            .map(|i| {
                let mut rng = ChaCha8Rng::seed_from_u64(config.seed.wrapping_add(i));
                let noisy = lifting.lift_with(&trace, &mut rng).unwrap();
                u64::from(inner.robustness_signal(&noisy).unwrap()[0] >= 0.0)
            })
            .sum();

        assert_eq!(result.satisfactions, baseline);
    }

    #[test]
    fn an_empty_trace_is_rejected() {
        let inner = Formula::parse("x > 0").unwrap();
        let trace = Trace::new(Vec::new()).unwrap();
        let lifting = LiftingRegistry::new();
        let err = check(
            ProbabilityOp::GreaterEqual,
            0.5,
            &inner,
            &trace,
            &lifting,
            &SmcConfig::default(),
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyTrace));
    }
}

#[cfg(all(test, feature = "gpu"))]
mod tests {
    use super::*;
    use crate::stats::{NoiseInteraction, NoiseModel};

    #[test]
    #[ignore = "needs a GPU; run with --ignored on a GPU node"]
    fn temporal_smc_matches_cpu_and_analytic() {
        // Phi(1)^3 = 0.59557 is the analytic probability.
        let inner = Formula::parse("always[0, 2](x > 0)").unwrap();
        let mut trace = Trace::new(vec![0.0, 1.0, 2.0, 3.0]).unwrap();
        trace.add_signal("x", vec![1.0, 1.0, 1.0, 1.0]).unwrap();
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 1.0).unwrap(),
            NoiseInteraction::Additive,
        );
        let config = SmcConfig {
            samples: 500_000,
            confidence: 0.95,
            seed: 7,
        };

        let gpu = try_gpu_check(
            ProbabilityOp::GreaterEqual,
            0.5,
            &inner,
            &trace,
            &lifting,
            &config,
        )
        .expect("a GPU device should be present on this node");

        let satisfies = |i: u64| -> bool {
            let mut rng = ChaCha8Rng::seed_from_u64(config.seed.wrapping_add(i));
            let noisy = lifting.lift_with(&trace, &mut rng).unwrap();
            inner.robustness(&noisy).unwrap() >= 0.0
        };
        let cpu_count: u64 = (0..config.samples).map(|i| u64::from(satisfies(i))).sum();
        let cpu = cpu_count as f64 / config.samples as f64;

        let analytic = 0.595_57;
        assert!(
            (gpu.probability - cpu).abs() < 0.01,
            "gpu {} vs cpu {cpu}",
            gpu.probability
        );
        assert!(
            (gpu.probability - analytic).abs() < 0.01,
            "gpu {} vs analytic {analytic}",
            gpu.probability
        );
    }
}