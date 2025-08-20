//! GPU Monte Carlo for the atemporal statistical model checking case.
//!
//! For a probabilistic formula `P~p(phi)` whose inner `phi` is atemporal, the
//! satisfaction probability is estimated by drawing many noisy realizations of
//! the reading and counting how many satisfy `phi`. This module runs that count
//! on a GPU: each thread draws one realization, evaluates the transpiled
//! formula, and a tree reduction counts the satisfied ones.
//!
//! The GPU works in f32, so its estimate agrees with the CPU only within Monte
//! Carlo and single-precision tolerance. The device returns only an integer
//! count, and the confidence interval and the verdict are computed on the host
//! in f64, identical to the CPU path. The path runs only for the closed-form
//! noise families and falls back to the CPU for everything else, so a result is
//! always available.

// The pieces are built bottom-up and become reachable from the statistical layer
// once the SMC entry wires in the fallback path.
#![allow(dead_code)]

use crate::error::Error;
use crate::stats::{GpuSampler, LiftingRegistry, NoiseInteraction};

/// The width, in f32 slots, of one variable's noise record in the device buffer.
pub(crate) const NOISE_RECORD: usize = 8;

/// A failure on the GPU Monte Carlo path.
///
/// A capability or policy miss (no device, an unsupported family, too many
/// samples) is handled by falling back to the CPU, not by surfacing one of
/// these. A variant that does reach the caller becomes [`Error::Gpu`].
#[derive(Debug)]
pub(crate) enum GpuMcError {
    AdapterNotFound,
    DeviceRequest(String),
    Readback(String),
    InvalidWgsl(String),
    /// More samples were requested than the f32-exact count path allows. The
    /// CPU path, which counts in `u64`, handles larger runs.
    SampleCountOverflow {
        /// The requested sample count.
        requested: u64,
        /// The largest count the GPU path accepts.
        max: u64,
    },
    /// A noise family has no closed-form GPU sampler. The caller runs on the CPU.
    UnsupportedNoiseFamily {
        /// The family that has no GPU sampler.
        family: &'static str,
    },
}

impl core::fmt::Display for GpuMcError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GpuMcError::AdapterNotFound => write!(f, "no compatible GPU adapter is present"),
            GpuMcError::DeviceRequest(e) => write!(f, "could not create a GPU device: {e}"),
            GpuMcError::Readback(e) => write!(f, "could not read GPU results back: {e}"),
            GpuMcError::InvalidWgsl(e) => write!(f, "the GPU shader did not compile: {e}"),
            GpuMcError::SampleCountOverflow { requested, max } => write!(
                f,
                "{requested} samples exceeds the GPU limit of {max}; the CPU path handles larger runs"
            ),
            GpuMcError::UnsupportedNoiseFamily { family } => write!(
                f,
                "the {family} noise family has no GPU sampler; this runs on the CPU"
            ),
        }
    }
}

impl From<GpuMcError> for Error {
    fn from(error: GpuMcError) -> Self {
        Error::Gpu {
            message: error.to_string(),
        }
    }
}

/// Packs each variable's noise parameters into the device buffer, in `symbols` order.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "the device runs in f32; the small family tag and the parameters fit it"
)]
pub(crate) fn pack_noise_params(
    symbols: &[String],
    lifting: &LiftingRegistry,
) -> Result<Vec<f32>, GpuMcError> {
    let mut packed = vec![0.0f32; symbols.len() * NOISE_RECORD];
    for (slot, name) in symbols.iter().enumerate() {
        let Some((model, interaction)) = lifting.model_for(name) else {
            continue;
        };
        let (family, p0, p1) = match model.gpu_sampler() {
            GpuSampler::Closed { family, p0, p1 } => (family, p0, p1),
            GpuSampler::Cpu { family } => {
                return Err(GpuMcError::UnsupportedNoiseFamily { family })
            }
        };
        let base = slot * NOISE_RECORD;
        packed[base] = family as f32;
        packed[base + 1] = match interaction {
            NoiseInteraction::Additive => 0.0,
            NoiseInteraction::Multiplicative => 1.0,
        };
        packed[base + 2] = p0 as f32;
        packed[base + 3] = p1 as f32;
    }
    Ok(packed)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "the packed records are exact f32 values")]

    use super::*;
    use crate::stats::NoiseModel;

    #[test]
    fn packs_supported_families_by_slot() {
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 2.0).unwrap(),
            NoiseInteraction::Additive,
        );
        lifting.register(
            "y",
            NoiseModel::uniform(1.0, 3.0).unwrap(),
            NoiseInteraction::Multiplicative,
        );
        let symbols = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        let packed = pack_noise_params(&symbols, &lifting).unwrap();
        assert_eq!(packed.len(), 3 * NOISE_RECORD);
        // x: Gaussian (1), additive (0), mean 0, std 2.
        assert_eq!(&packed[0..4], &[1.0f32, 0.0, 0.0, 2.0]);
        // y: Uniform (4), multiplicative (1), low 1, high 3.
        assert_eq!(&packed[8..12], &[4.0f32, 1.0, 1.0, 3.0]);
        assert_eq!(&packed[16..24], &[0.0f32; 8]);
    }

    #[test]
    fn an_unsupported_family_declines_for_cpu_fallback() {
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gamma(2.0, 1.0).unwrap(),
            NoiseInteraction::Additive,
        );
        let err = pack_noise_params(&["x".to_string()], &lifting).unwrap_err();
        assert!(matches!(
            err,
            GpuMcError::UnsupportedNoiseFamily { family: "Gamma" }
        ));
    }
}