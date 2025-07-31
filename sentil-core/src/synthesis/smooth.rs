//! Smooth, differentiable robustness operators for synthesis.
//!
//! Monitoring uses exact min and max, which are not differentiable at ties.
//! Synthesis instead needs a robustness that varies smoothly with the trace so an
//! optimizer can follow its gradient. The replacements here are a log-sum-exp soft
//! minimum and maximum controlled by a temperature: as the temperature rises they
//! approach the exact operators, and at every temperature the soft minimum stays
//! at or below the true minimum and the soft maximum at or above the true maximum.

use crate::error::{Error, Result};

/// The temperature for smooth robustness.
///
/// A higher temperature tracks the exact min and max more closely; a lower one is
/// smoother and easier for an optimizer to climb.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmoothConfig {
    temperature: f64,
}

impl SmoothConfig {
    /// Builds a configuration.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if `temperature` is not finite and
    /// positive.
    pub fn new(temperature: f64) -> Result<Self> {
        if temperature.is_finite() && temperature > 0.0 {
            Ok(Self { temperature })
        } else {
            Err(Error::InvalidConfig {
                context: "smooth robustness",
                message: format!("temperature must be finite and positive, got {temperature}"),
            })
        }
    }

    /// The temperature.
    #[must_use]
    pub fn temperature(&self) -> f64 {
        self.temperature
    }
}

impl Default for SmoothConfig {
    fn default() -> Self {
        Self { temperature: 10.0 }
    }
}

/// A smooth lower bound on the minimum of `values`, approaching the true minimum
/// as `beta` grows. An empty slice has minimum positive infinity.
#[must_use]
pub fn soft_min(values: &[f64], beta: f64) -> f64 {
    let shift = values.iter().copied().fold(f64::INFINITY, f64::min);
    if !shift.is_finite() {
        return shift;
    }
    let sum: f64 = values.iter().map(|&x| (-beta * (x - shift)).exp()).sum();
    shift - sum.ln() / beta
}

/// A smooth upper bound on the maximum of `values`, approaching the true maximum
/// as `beta` grows. An empty slice has maximum negative infinity.
#[must_use]
pub fn soft_max(values: &[f64], beta: f64) -> f64 {
    let shift = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !shift.is_finite() {
        return shift;
    }
    let sum: f64 = values.iter().map(|&x| (beta * (x - shift)).exp()).sum();
    shift + sum.ln() / beta
}