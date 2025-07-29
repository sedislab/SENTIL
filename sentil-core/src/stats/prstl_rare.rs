//! Rare-event PrSTL by adaptive multilevel splitting.

use std::collections::BTreeSet;

use rand::RngCore;

use crate::error::{Error, Result};

/// Draws an initial packed sample.
type InitFn = Box<dyn Fn(&mut dyn RngCore) -> Vec<f64>>;
type StepFn = Box<dyn Fn(&[f64], f64, &mut dyn RngCore) -> Vec<f64>>;

/// A user-defined stochastic system the splitter can drive.
pub struct StochasticSystem {
    variables: Vec<String>,
    dt: f64,
    horizon: usize,
    init: InitFn,
    step: StepFn,
}

impl StochasticSystem {
    /// Builds a system.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if `dt` is not finite and positive, `horizon` is zero, or `variables` is empty or holds a duplicate.
    pub fn new(
        variables: impl IntoIterator<Item = impl Into<String>>,
        dt: f64,
        horizon: usize,
        init: impl Fn(&mut dyn RngCore) -> Vec<f64> + 'static,
        step: impl Fn(&[f64], f64, &mut dyn RngCore) -> Vec<f64> + 'static,
    ) -> Result<Self> {
        let variables: Vec<String> = variables.into_iter().map(Into::into).collect();
        if !(dt.is_finite() && dt > 0.0) {
            return Err(config_error(format!(
                "dt must be finite and positive, got {dt}"
            )));
        }
        if horizon == 0 {
            return Err(config_error("horizon must be positive".to_owned()));
        }
        if variables.is_empty() {
            return Err(config_error("at least one variable is required".to_owned()));
        }
        if variables.iter().collect::<BTreeSet<_>>().len() != variables.len() {
            return Err(config_error("variable names must be unique".to_owned()));
        }
        Ok(Self {
            variables,
            dt,
            horizon,
            init: Box::new(init),
            step: Box::new(step),
        })
    }

    /// The packed signal order both closures emit.
    #[must_use]
    pub fn variables(&self) -> &[String] {
        &self.variables
    }

    /// The spacing between successive steps.
    #[must_use]
    pub fn dt(&self) -> f64 {
        self.dt
    }

    /// The trajectory length, in steps.
    #[must_use]
    pub fn horizon(&self) -> usize {
        self.horizon
    }

    /// Draws an initial packed sample from the system.
    pub fn initial(&self, rng: &mut dyn RngCore) -> Vec<f64> {
        (self.init)(rng)
    }

    /// Advances the system one step from `previous` at `time`.
    pub fn advance(&self, previous: &[f64], time: f64, rng: &mut dyn RngCore) -> Vec<f64> {
        (self.step)(previous, time, rng)
    }
}

/// Tuning for a rare-event run.
///
/// The defaults suit a first run: a few thousand particles, an automatic
/// per-level step budget, a zero violation margin, and a fixed seed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RareEventConfig {
    /// The particle population.
    pub particles: usize,
    /// How far a particle advances per level. `0` means automatic, about a tenth
    /// of the horizon, which keeps each level partial so the splitting ladder can
    /// form; setting it to the full horizon would collapse the run to plain Monte
    /// Carlo.
    pub max_steps_per_level: u64,
    /// The violation margin; the rare event is robustness at or below `-margin`.
    pub margin: f64,
    /// The seed.
    pub seed: u64,
}

impl Default for RareEventConfig {
    fn default() -> Self {
        Self {
            particles: 4096,
            max_steps_per_level: 0,
            margin: 0.0,
            seed: 42,
        }
    }
}

/// The outcome of a rare-event run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RareEventResult {
    /// The estimated satisfaction probability, `1 - violation_probability`.
    pub probability: f64,
    /// The raw splitting estimate of the tail violation probability.
    pub violation_probability: f64,
    /// Whether the satisfaction probability meets the operator's threshold.
    pub holds: bool,
    /// The total simulation steps run.
    pub simulations: u64,
}

fn config_error(message: String) -> Error {
    Error::InvalidConfig {
        context: "stochastic system",
        message,
    }
}