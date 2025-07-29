//! Rare-event PrSTL by adaptive multilevel splitting.

use std::collections::BTreeSet;

use rand::RngCore;

use super::{adaptive_multilevel_splitting, RareEventSimulator};
use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::signal::Trace;

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

/// A trajectory prefix. The monitor is rebuilt and replayed when scoring rather
/// than carried, since [`StreamMonitor`] is not cloneable.
#[derive(Clone)]
struct WalkState {
    samples: Vec<Vec<f64>>,
    last_value: Vec<f64>,
    step_index: usize,
    time: f64,
}

struct PrstlWalk<'a> {
    inner: &'a Formula,
    system: &'a StochasticSystem,
    margin: f64,
}

impl RareEventSimulator for PrstlWalk<'_> {
    type State = WalkState;

    fn initial_state(&self, rng: &mut dyn RngCore) -> WalkState {
        let v0 = self.system.initial(rng);
        WalkState {
            samples: vec![v0.clone()],
            last_value: v0,
            step_index: 0,
            time: 0.0,
        }
    }

    fn step(&self, state: &WalkState, rng: &mut dyn RngCore) -> WalkState {
        if state.step_index >= self.system.horizon() {
            return state.clone();
        }
        let next = self.system.advance(&state.last_value, state.time, rng);
        let mut samples = state.samples.clone();
        samples.push(next.clone());
        WalkState {
            samples,
            last_value: next,
            step_index: state.step_index + 1,
            time: state.time + self.system.dt(),
        }
    }

    fn is_terminal(&self, state: &WalkState) -> (bool, bool) {
        let violated = self.score(state) >= self.margin;
        (violated, violated)
    }

    /// The current violation: the negated robustness of the inner formula over the
    /// prefix treated as a complete trace, which only ever sees what has happened
    /// so far. A malformed system or evaluation error maps to NaN so the splitter
    /// reports [`Error::Splitting`] rather than scoring on garbage.
    fn score(&self, state: &WalkState) -> f64 {
        let vars = self.system.variables();
        let mut times = Vec::with_capacity(state.samples.len());
        let mut t = 0.0;
        for _ in &state.samples {
            times.push(t);
            t += self.system.dt();
        }
        let Ok(mut trace) = Trace::new(times) else {
            return f64::NAN;
        };
        for (i, var) in vars.iter().enumerate() {
            let column: Vec<f64> = state
                .samples
                .iter()
                .map(|s| s.get(i).copied().unwrap_or(f64::NAN))
                .collect();
            if trace.add_signal(var, column).is_err() {
                return f64::NAN;
            }
        }
        match self.inner.robustness(&trace) {
            Ok(rho) => (-rho).clamp(-1e12, 1e12),
            Err(_) => f64::NAN,
        }
    }
}

impl Formula {
    /// Estimates `P~p(phi)` over a user-defined stochastic `system` by adaptive
    /// multilevel splitting, for satisfaction probabilities too small for plain
    /// Monte Carlo to resolve. The inner formula should be safety-shaped, like
    /// `always`. `probability` is the satisfaction probability.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotProbabilistic`] unless the formula is `P~p(phi)`, [`Error::InvalidConfig`] if the inner formula is not safety shaped, and any [`Error::Splitting`] from the run.
    pub fn check_rare_event(
        &self,
        system: &StochasticSystem,
        config: &RareEventConfig,
    ) -> Result<RareEventResult> {
        let Formula::Probabilistic(op, threshold, inner) = self else {
            return Err(Error::NotProbabilistic);
        };
        let max_steps = if config.max_steps_per_level == 0 {
            (system.horizon() as u64 / 10).max(1)
        } else {
            config.max_steps_per_level
        };
        let walk = PrstlWalk {
            inner,
            system,
            margin: config.margin,
        };
        let est = adaptive_multilevel_splitting(
            &walk,
            config.particles,
            config.margin,
            max_steps,
            config.seed,
        )?;
        let probability = 1.0 - est.probability;
        Ok(RareEventResult {
            probability,
            violation_probability: est.probability,
            holds: super::decides(*op, probability, *threshold),
            simulations: est.simulations,
        })
    }
}

fn config_error(message: String) -> Error {
    Error::InvalidConfig {
        context: "stochastic system",
        message,
    }
}