//! A monitor: one handle that pairs a formula with how to check it.
//!
//! The engine exposes robustness, statistical checking, and streaming as separate
//! entry points on [`Formula`] and [`StreamMonitor`](crate::StreamMonitor). A
//! [`Monitor`] bundles a formula with a [`MonitorConfig`] so a caller chooses the
//! time mode and statistical settings once, then calls one obvious method per task.
//! It adds no semantics of its own: every method delegates to the engine.

use crate::error::Result;
use crate::formula::Formula;
use crate::signal::Trace;

/// How offline robustness reads between samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeMode {
    /// Evaluate at the sample points only.
    #[default]
    Discrete,
    /// Interpolate piecewise-linearly between samples.
    Dense,
}

/// How a [`Monitor`] checks its formula. For now this is the time mode; the
/// statistical settings join it with the `statistical` feature.
#[derive(Debug, Clone, Default)]
pub struct MonitorConfig {
    time: TimeMode,
}

impl MonitorConfig {
    /// The default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the time mode used by [`Monitor::robustness`].
    #[must_use]
    pub fn time(mut self, mode: TimeMode) -> Self {
        self.time = mode;
        self
    }
}

/// A formula paired with how to check it.
///
/// ```
/// use sentil::{Monitor, MonitorConfig, Trace};
///
/// let monitor = Monitor::new("always[0, 2](x > 0)", MonitorConfig::new())?;
/// let mut trace = Trace::new([0.0, 1.0, 2.0])?;
/// trace.add_signal("x", [3.0, 1.0, 2.0])?;
/// assert_eq!(monitor.robustness(&trace)?, 1.0);
/// # Ok::<(), sentil::Error>(())
/// ```
pub struct Monitor {
    formula: Formula,
    config: MonitorConfig,
}

impl Monitor {
    /// Parses `formula` and holds it under `config`.
    ///
    /// # Errors
    ///
    /// Returns a parse error if `formula` is malformed.
    pub fn new(formula: &str, config: MonitorConfig) -> Result<Self> {
        Ok(Self::from_formula(Formula::parse(formula)?, config))
    }

    /// Holds an already-parsed `formula` under `config`.
    #[must_use]
    pub fn from_formula(formula: Formula, config: MonitorConfig) -> Self {
        Self { formula, config }
    }

    /// The robustness of `trace`, read discretely or densely per the config.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ProbabilisticOperator`](crate::Error::ProbabilisticOperator)
    /// for a probabilistic formula, and propagates any evaluation error.
    pub fn robustness(&self, trace: &Trace) -> Result<f64> {
        match self.config.time {
            TimeMode::Discrete => self.formula.robustness(trace),
            TimeMode::Dense => self.formula.robustness_dense(trace),
        }
    }

    /// The formula being monitored.
    #[must_use]
    pub fn formula(&self) -> &Formula {
        &self.formula
    }

    /// The configuration in force.
    #[must_use]
    pub fn config(&self) -> &MonitorConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the monitor only delegates, so its result is bit-identical to the engine's"
    )]

    use super::*;
    use crate::error::Error;

    #[test]
    fn robustness_delegates_in_each_time_mode() {
        let mut trace = Trace::new([0.0, 1.0, 2.0]).unwrap();
        trace.add_signal("x", [2.0, 0.5, 3.0]).unwrap();
        let text = "always[0, 2](x > 1)";
        let phi = Formula::parse(text).unwrap();

        let discrete = Monitor::new(text, MonitorConfig::new()).unwrap();
        assert_eq!(
            discrete.robustness(&trace).unwrap(),
            phi.robustness(&trace).unwrap()
        );

        let dense = Monitor::new(text, MonitorConfig::new().time(TimeMode::Dense)).unwrap();
        assert_eq!(
            dense.robustness(&trace).unwrap(),
            phi.robustness_dense(&trace).unwrap()
        );
    }

    #[test]
    fn a_probabilistic_formula_cannot_be_read_as_robustness() {
        let monitor = Monitor::new("P>=0.9(always(x > 0))", MonitorConfig::new()).unwrap();
        let mut trace = Trace::new([0.0]).unwrap();
        trace.add_signal("x", [1.0]).unwrap();
        assert!(matches!(
            monitor.robustness(&trace),
            Err(Error::ProbabilisticOperator)
        ));
    }

    #[test]
    fn a_malformed_formula_is_rejected_at_construction() {
        assert!(Monitor::new("always(", MonitorConfig::new()).is_err());
    }
}