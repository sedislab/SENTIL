//! Monitoring several formulas over one stream.

use crate::error::{Error, Result};
use crate::formula::Formula;
#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::signal::Trace;
#[cfg(feature = "statistical")]
use crate::stats::{LiftingRegistry, SmcConfig, SmcResult};

use super::{Robustness, StreamMonitor};

/// A bank of streaming monitors driven together.
pub struct MultiFormulaMonitor {
    monitors: Vec<(String, StreamMonitor)>,
}

impl MultiFormulaMonitor {
    /// Creates an empty bank.
    #[must_use]
    pub fn new() -> Self {
        Self {
            monitors: Vec::new(),
        }
    }

    /// Adds a formula, parsed from text, under `id`.
    ///
    /// # Errors
    ///
    /// Returns the parse or composition error if `formula` is not a valid
    /// streaming specification.
    pub fn add(&mut self, id: impl Into<String>, formula: &str) -> Result<()> {
        let monitor = StreamMonitor::new(formula)?;
        self.monitors.push((id.into(), monitor));
        Ok(())
    }

    /// Adds an already-parsed formula under `id`.
    ///
    /// # Errors
    ///
    /// Returns the composition error if the formula is not streaming-admissible.
    pub fn add_formula(&mut self, id: impl Into<String>, formula: &Formula) -> Result<()> {
        let monitor = StreamMonitor::from_formula(formula)?;
        self.monitors.push((id.into(), monitor));
        Ok(())
    }

    /// Adds a probabilistic formula `P~p(phi)` under `id`, monitored online by a
    /// lifted particle ensemble.
    ///
    /// # Errors
    ///
    /// Returns the composition error if the formula is not a streamable
    /// probabilistic specification.
    #[cfg(feature = "statistical")]
    pub fn add_probabilistic(
        &mut self,
        id: impl Into<String>,
        formula: &Formula,
        lifting: &LiftingRegistry,
        config: &SmcConfig,
    ) -> Result<()> {
        let monitor = StreamMonitor::with_lifting(formula, lifting, config)?;
        self.monitors.push((id.into(), monitor));
        Ok(())
    }

    /// Advances every monitor to `time` with the named `values` and returns each
    /// formula's robustness, in insertion order.
    ///
    /// # Errors
    ///
    /// Returns the first error any monitor reports, for example
    /// [`Error::UnknownVariable`] for a variable absent from `values`.
    pub fn update(
        &mut self,
        time: f64,
        values: &[(&str, f64)],
    ) -> Result<Vec<(String, Robustness)>> {
        let mut results = Vec::with_capacity(self.monitors.len());
        let mut first_error: Option<Error> = None;
        for (id, monitor) in &mut self.monitors {
            match monitor.update(time, values) {
                Ok(robustness) => results.push((id.clone(), robustness)),
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(results),
        }
    }

    /// Resets every monitor.
    pub fn reset(&mut self) {
        for (_, monitor) in &mut self.monitors {
            monitor.reset();
        }
    }

    /// Removes the first formula registered under `id`.
    pub fn remove(&mut self, id: &str) -> bool {
        if let Some(position) = self.monitors.iter().position(|(name, _)| name == id) {
            self.monitors.remove(position);
            true
        } else {
            false
        }
    }

    /// The ids currently monitored, in insertion order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.monitors.iter().map(|(id, _)| id.as_str())
    }

    /// The number of formulas in the bank.
    #[must_use]
    pub fn len(&self) -> usize {
        self.monitors.len()
    }

    /// Whether the bank holds no formulas.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.monitors.is_empty()
    }
}

impl Default for MultiFormulaMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// A bank of named formulas evaluated together over one complete trace.
///
/// ```
/// use sentil::{FormulaBank, Trace};
///
/// let mut bank = FormulaBank::new();
/// bank.add("safe", "always[0, 2](x > 0)")?;
/// bank.add("reaches", "eventually[0, 2](x > 4)")?;
/// let trace = Trace::from_signal([0.0, 1.0, 2.0], "x", [1.0, 3.0, 5.0])?;
/// let verdicts = bank.robustness(&trace);
/// assert_eq!(verdicts[0].0, "safe");
/// assert_eq!(verdicts[0].1.as_ref().unwrap(), &1.0);
/// # Ok::<(), sentil::Error>(())
/// ```
pub struct FormulaBank {
    formulas: Vec<(String, Formula)>,
}

impl FormulaBank {
    /// Creates an empty bank.
    #[must_use]
    pub fn new() -> Self {
        Self {
            formulas: Vec::new(),
        }
    }

    /// Adds a formula, parsed from text, under `id`.
    ///
    /// # Errors
    ///
    /// Returns the parse error if `formula` is malformed.
    pub fn add(&mut self, id: impl Into<String>, formula: &str) -> Result<()> {
        let formula = Formula::parse(formula)?;
        self.formulas.push((id.into(), formula));
        Ok(())
    }

    /// Adds an already-parsed formula under `id`.
    pub fn add_formula(&mut self, id: impl Into<String>, formula: &Formula) {
        self.formulas.push((id.into(), formula.clone()));
    }

    /// The discrete robustness of every formula over `trace`, paired with its id.
    #[must_use]
    pub fn robustness(&self, trace: &Trace) -> Vec<(String, Result<f64>)> {
        self.formulas
            .iter()
            .map(|(id, formula)| (id.clone(), formula.robustness(trace)))
            .collect()
    }

    /// The dense robustness of every formula over `trace`, paired with its id.
    #[must_use]
    pub fn robustness_dense(&self, trace: &Trace) -> Vec<(String, Result<f64>)> {
        self.formulas
            .iter()
            .map(|(id, formula)| (id.clone(), formula.robustness_dense(trace)))
            .collect()
    }

    /// The statistical check of every formula over the lifted trace ensemble,
    /// paired with its id.
    #[cfg(feature = "statistical")]
    #[must_use]
    pub fn check(
        &self,
        trace: &Trace,
        lifting: &LiftingRegistry,
        config: &SmcConfig,
    ) -> Vec<(String, Result<SmcResult>)> {
        self.formulas
            .iter()
            .map(|(id, formula)| (id.clone(), formula.check(trace, lifting, config)))
            .collect()
    }

    /// The ids in the bank, in insertion order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.formulas.iter().map(|(id, _)| id.as_str())
    }

    /// The number of formulas in the bank.
    #[must_use]
    pub fn len(&self) -> usize {
        self.formulas.len()
    }

    /// Whether the bank holds no formulas.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.formulas.is_empty()
    }
}

impl Default for FormulaBank {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "the asserted robustness values are exact")]

    use super::*;

    #[test]
    fn results_keep_insertion_order() {
        let mut bank = MultiFormulaMonitor::new();
        bank.add("low", "x > 0").unwrap();
        bank.add("high", "x > 10").unwrap();
        let out = bank.update(0.0, &[("x", 5.0)]).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, "low");
        assert_eq!(out[1].0, "high");
        assert_eq!(out[0].1.value(), 5.0);
        assert_eq!(out[1].1.value(), -5.0);
    }

    #[test]
    fn a_failing_formula_does_not_desync_the_others() {
        let mut bank = MultiFormulaMonitor::new();
        bank.add("a", "historically[0, 2](x > 0)").unwrap();
        bank.add("b", "historically[0, 2](z > 0)").unwrap();
        assert!(bank.update(0.0, &[("x", 1.0)]).is_err());
        bank.update(1.0, &[("x", 5.0), ("z", 1.0)]).unwrap();
        let out = bank.update(2.0, &[("x", 5.0), ("z", 1.0)]).unwrap();
        let a = out.iter().find(|(id, _)| id == "a").unwrap().1;
        assert_eq!(a.value(), 1.0);
    }

    #[cfg(feature = "statistical")]
    #[test]
    fn a_bank_mixes_deterministic_and_probabilistic_formulas() {
        use crate::stats::{NoiseInteraction, NoiseModel};
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 1.0).unwrap(),
            NoiseInteraction::Additive,
        );
        let config = SmcConfig {
            samples: 2000,
            seed: 7,
            ..Default::default()
        };
        let phi = Formula::parse("P>=0.5(x > 0)").unwrap();
        let mut bank = MultiFormulaMonitor::new();
        bank.add("plain", "x > 0").unwrap();
        bank.add_probabilistic("prob", &phi, &lifting, &config).unwrap();
        let out = bank.update(0.0, &[("x", 3.0)]).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, "plain");
        assert_eq!(out[1].0, "prob");
        assert!(out[0].1.value() > 0.0);
        assert!(out[1].1.value() > 0.0);
    }

    #[test]
    fn an_empty_bank_returns_no_results() {
        let mut bank = MultiFormulaMonitor::new();
        assert!(bank.is_empty());
        assert_eq!(bank.update(0.0, &[("x", 1.0)]).unwrap().len(), 0);
    }

    #[test]
    fn formulas_can_be_listed_and_removed() {
        let mut bank = MultiFormulaMonitor::new();
        bank.add("a", "x > 0").unwrap();
        bank.add("b", "x > 0").unwrap();
        assert_eq!(bank.len(), 2);
        assert_eq!(bank.ids().collect::<Vec<_>>(), ["a", "b"]);
        assert!(bank.remove("a"));
        assert!(!bank.remove("a"));
        assert_eq!(bank.ids().collect::<Vec<_>>(), ["b"]);
    }

    #[test]
    fn an_offline_bank_evaluates_every_formula_even_when_one_fails() {
        let mut bank = FormulaBank::new();
        bank.add("safe", "always[0, 2](x > 0)").unwrap();
        bank.add("missing", "y > 0").unwrap();
        let trace = Trace::from_signal([0.0, 1.0, 2.0], "x", [1.0, 3.0, 5.0]).unwrap();
        let out = bank.robustness(&trace);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, "safe");
        assert_eq!(*out[0].1.as_ref().unwrap(), 1.0);
        assert!(matches!(out[1].1, Err(Error::UnknownVariable { .. })));
    }

    #[test]
    fn an_offline_bank_reads_densely() {
        let mut bank = FormulaBank::new();
        bank.add("dip", "always[0, 1.5](x > 0)").unwrap();
        let trace = Trace::from_signal([0.0, 1.0, 2.0], "x", [1.0, 1.0, -3.0]).unwrap();
        let out = bank.robustness_dense(&trace);
        assert_eq!(*out[0].1.as_ref().unwrap(), -1.0);
    }

    #[cfg(feature = "statistical")]
    #[test]
    fn an_offline_bank_checks_probabilistic_formulas() {
        use crate::stats::{NoiseInteraction, NoiseModel};
        let mut bank = FormulaBank::new();
        bank.add("likely", "P>=0.9(x > 0)").unwrap();
        bank.add("plain", "x > 0").unwrap();
        let mut lifting = LiftingRegistry::new();
        lifting.register(
            "x",
            NoiseModel::gaussian(0.0, 0.5).unwrap(),
            NoiseInteraction::Additive,
        );
        let trace = Trace::from_signal([0.0], "x", [5.0]).unwrap();
        let out = bank.check(&trace, &lifting, &SmcConfig::default());
        assert!(out[0].1.as_ref().unwrap().holds);
        assert!(matches!(out[1].1, Err(Error::NotProbabilistic)));
    }
}