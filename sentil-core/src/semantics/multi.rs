//! Monitoring several formulas over one stream.
//!
//! A [`MultiFormulaMonitor`] drives a bank of independent [`StreamMonitor`]s from
//! the same timed samples, each keyed by a caller-chosen id. The monitors are
//! independent, one per formula; the value of the bank is the single drive point.

use crate::error::{Error, Result};
use crate::formula::Formula;
#[cfg(not(feature = "std"))]
use crate::prelude::*;

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

    /// Advances every monitor to `time` with the named `values` and returns each
    /// formula's robustness, in insertion order.
    ///
    /// Every monitor is advanced even when one errors, so a failing formula never
    /// leaves the others a step behind; if any fail, the first error is returned
    /// and the partial results are dropped.
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
}