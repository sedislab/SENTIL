//! Robustness semantics: turning a formula and a signal trace into a margin.

mod discrete;
mod eval;
mod robustness;
mod window;

pub use robustness::Robustness;

use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::signal::Trace;

impl Formula {
    /// The robustness of the formula over a trace, measured at its start.
    ///
    /// A positive result means the trace satisfies the formula with that much
    /// margin; a negative result is the depth of the worst violation.
    ///
    /// ```
    /// use sentil::{Formula, Trace};
    ///
    /// let phi = Formula::parse("always[0, 2](x > 0)")?;
    /// let mut trace = Trace::new([0.0, 1.0, 2.0])?;
    /// trace.add_signal("x", [10.0, 5.0, 1.0])?;
    /// assert_eq!(phi.robustness(&trace)?, 1.0);
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyTrace`] if the trace has no samples,
    /// [`Error::UnknownVariable`] if the formula names a signal the trace lacks,
    /// and [`Error::ProbabilisticOperator`] if the formula is probabilistic.
    pub fn robustness(&self, trace: &Trace) -> Result<f64> {
        if trace.is_empty() {
            return Err(Error::EmptyTrace);
        }
        let values = discrete::robustness_trace(self, trace.times(), trace.signals())?;
        Ok(values[0])
    }
}