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