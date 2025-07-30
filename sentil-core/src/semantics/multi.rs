//! Monitoring several formulas over one stream.
//!
//! A [`MultiFormulaMonitor`] drives a bank of independent [`StreamMonitor`]s from
//! the same timed samples, each keyed by a caller-chosen id. The monitors are
//! independent, one per formula; the value of the bank is the single drive point.

use crate::error::{Error, Result};
use crate::formula::Formula;

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
}

impl Default for MultiFormulaMonitor {
    fn default() -> Self {
        Self::new()
    }
}