//! Turning a formula and a signal trace into a robustness margin.

mod dense;
mod discrete;
mod eval;
mod multi;
mod pwl;
mod robustness;
mod stream;
mod window;

/// The slack allowed when deciding whether a timestamp has reached a window edge.
pub(crate) const WINDOW_EPSILON: f64 = 1e-9;

#[cfg(not(feature = "std"))]
use crate::prelude::*;

pub use multi::{FormulaBank, MultiFormulaMonitor};
pub use robustness::Robustness;
pub use stream::StreamMonitor;

#[cfg(feature = "synthesis")]
pub(crate) use eval::eval_expr;
#[cfg(feature = "synthesis")]
pub(crate) use eval::eval_predicate;
pub(crate) use eval::predicate_margin;

use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::signal::Trace;

impl Formula {
    /// The robustness of the formula over a trace, measured at its start.
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
        let times = trace.times();
        let m = discrete::max_dep_index(self, 0, times) + 1;
        let values = discrete::robustness_trace(self, &times[..m], trace.signals())?;
        Ok(values[0])
    }

    /// The robustness over a trace read as a continuous, piecewise-linear signal.
    ///
    /// ```
    /// use sentil::{Formula, Trace};
    ///
    /// let phi = Formula::parse("always[0, 1.5](x > 0)")?;
    /// let mut trace = Trace::new([0.0, 1.0, 2.0])?;
    /// trace.add_signal("x", [1.0, 1.0, -3.0])?;
    /// assert_eq!(phi.robustness_dense(&trace)?, -1.0);
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyTrace`] for an empty trace,
    /// [`Error::UnknownVariable`] for a missing signal, and
    /// [`Error::Unsupported`] if a predicate is nonlinear in the signals.
    pub fn robustness_dense(&self, trace: &Trace) -> Result<f64> {
        if trace.is_empty() {
            return Err(Error::EmptyTrace);
        }
        let signal = dense::robustness_signal(self, trace.times(), trace.signals())?;
        Ok(signal.at(trace.times()[0]))
    }

    /// The discrete-time robustness at every sample.
    ///
    /// ```
    /// use sentil::{Formula, Trace};
    ///
    /// let phi = Formula::parse("x > 0")?;
    /// let mut trace = Trace::new([0.0, 1.0, 2.0])?;
    /// trace.add_signal("x", [1.0, -2.0, 3.0])?;
    /// assert_eq!(phi.robustness_signal(&trace)?, vec![1.0, -2.0, 3.0]);
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As for [`robustness`](Self::robustness).
    pub fn robustness_signal(&self, trace: &Trace) -> Result<Vec<f64>> {
        if trace.is_empty() {
            return Err(Error::EmptyTrace);
        }
        discrete::robustness_trace(self, trace.times(), trace.signals())
    }

    /// The dense-time robustness at every sample.
    ///
    /// ```
    /// use sentil::{Formula, Trace};
    ///
    /// let phi = Formula::parse("x > 0")?;
    /// let mut trace = Trace::new([0.0, 1.0])?;
    /// trace.add_signal("x", [2.0, -1.0])?;
    /// assert_eq!(phi.robustness_dense_signal(&trace)?, vec![2.0, -1.0]);
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As for [`robustness_dense`](Self::robustness_dense).
    pub fn robustness_dense_signal(&self, trace: &Trace) -> Result<Vec<f64>> {
        if trace.is_empty() {
            return Err(Error::EmptyTrace);
        }
        let signal = dense::robustness_signal(self, trace.times(), trace.signals())?;
        Ok(trace.times().iter().map(|&t| signal.at(t)).collect())
    }

    /// The time spans over which the formula is violated on the trace.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyTrace`] for an empty trace, and propagates any error
    /// from evaluating the formula.
    pub fn violations(&self, trace: &Trace) -> Result<Vec<(f64, f64)>> {
        let signal = self.robustness_signal(trace)?;
        Ok(violation_intervals(trace.times(), &signal))
    }
}

/// The time spans where `signal` is negative.
#[must_use]
pub fn violation_intervals(times: &[f64], signal: &[f64]) -> Vec<(f64, f64)> {
    let n = times.len().min(signal.len());
    let mut spans = Vec::new();
    let mut start: Option<usize> = None;
    for (i, &r) in signal.iter().take(n).enumerate() {
        if r < 0.0 {
            start.get_or_insert(i);
        } else if let Some(s) = start.take() {
            spans.push((times[s], times[i - 1]));
        }
    }
    if let Some(s) = start {
        spans.push((times[s], times[n - 1]));
    }
    spans
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "these robustness values are exact integer-valued f64 results"
    )]

    use crate::formula::Formula;
    use crate::signal::Trace;

    fn robustness(formula: &str, times: &[f64], signals: &[(&str, &[f64])]) -> f64 {
        let phi = Formula::parse(formula).unwrap();
        let mut trace = Trace::new(times.to_vec()).unwrap();
        for (name, values) in signals {
            trace.add_signal(name, values.to_vec()).unwrap();
        }
        phi.robustness(&trace).unwrap()
    }

    #[test]
    fn violation_intervals_finds_negative_runs() {
        let times = [0.0, 1.0, 2.0, 3.0, 4.0];
        let signal = [1.0, -1.0, -2.0, 0.5, -3.0];
        assert_eq!(
            super::violation_intervals(&times, &signal),
            vec![(1.0, 2.0), (4.0, 4.0)]
        );
    }

    #[test]
    fn a_formula_reports_its_violation_spans() {
        let phi = Formula::parse("x > 0").unwrap();
        let mut trace = Trace::new(vec![0.0, 1.0, 2.0]).unwrap();
        trace.add_signal("x", vec![1.0, -2.0, 3.0]).unwrap();
        assert_eq!(phi.violations(&trace).unwrap(), vec![(1.0, 1.0)]);
    }

    #[test]
    fn predicate_robustness_is_the_margin_at_the_start() {
        assert_eq!(robustness("x > 5", &[0.0], &[("x", &[8.0])]), 3.0);
        assert_eq!(robustness("x < 5", &[0.0], &[("x", &[8.0])]), -3.0);
    }

    #[test]
    fn always_takes_the_window_minimum() {
        assert_eq!(
            robustness(
                "always[0, 2](x > 0)",
                &[0.0, 1.0, 2.0],
                &[("x", &[10.0, 5.0, 1.0])]
            ),
            1.0
        );
    }

    #[test]
    fn eventually_takes_the_window_maximum() {
        assert_eq!(
            robustness(
                "eventually[0, 3](x > 10)",
                &[0.0, 1.0, 2.0, 3.0],
                &[("x", &[8.0, 8.0, 8.0, 8.0])]
            ),
            -2.0
        );
    }

    #[test]
    fn boolean_operators_combine_margins() {
        let times = [0.0];
        assert_eq!(
            robustness("x > 0 and y > 0", &times, &[("x", &[5.0]), ("y", &[3.0])]),
            3.0
        );
        assert_eq!(
            robustness("x > 0 or y > 0", &times, &[("x", &[-1.0]), ("y", &[3.0])]),
            3.0
        );
        assert_eq!(robustness("not(x > 5)", &times, &[("x", &[3.0])]), 2.0);
        assert_eq!(
            robustness(
                "(x > 10) implies (y > 0)",
                &times,
                &[("x", &[15.0]), ("y", &[3.0])]
            ),
            3.0
        );
    }

    #[test]
    fn historically_scans_the_past_window() {
        assert_eq!(
            robustness(
                "historically[0, 5](x > 0)",
                &[0.0, 1.0, 2.0],
                &[("x", &[2.0, -5.0, 3.0])]
            ),
            2.0
        );
    }

    #[test]
    fn nested_always_eventually() {
        let r = robustness(
            "always[0, 2](eventually[0, 1](p > 0))",
            &[0.0, 1.0, 2.0],
            &[("p", &[1.0, -1.0, 1.0])],
        );
        assert_eq!(r, 1.0);
    }

    #[test]
    fn unbounded_until() {
        let r = robustness(
            "p > 0 until q > 0",
            &[0.0, 1.0, 2.0],
            &[("p", &[1.0, 1.0, -3.0]), ("q", &[-1.0, -1.0, 1.0])],
        );
        assert_eq!(r, 1.0);
    }

    #[test]
    fn dense_reading_differs_from_discrete_at_a_sample() {
        let phi = Formula::parse("always[0, 1.5](x > 0)").unwrap();
        let mut trace = Trace::new(vec![0.0, 1.0, 2.0]).unwrap();
        trace.add_signal("x", vec![1.0, 1.0, -3.0]).unwrap();
        assert_eq!(phi.robustness(&trace).unwrap(), 1.0);
        assert_eq!(phi.robustness_dense(&trace).unwrap(), -1.0);
    }

    #[test]
    fn dense_until_signal_has_no_nan_next_to_an_empty_window() {
        let phi = Formula::parse("(y > 0) until[1, 3] (x > 0)").unwrap();
        let mut trace = Trace::new(vec![1.8, 3.5, 5.1, 5.5, 7.4, 7.8]).unwrap();
        trace
            .add_signal("x", vec![0.8, -0.1, -3.4, -1.6, -3.0, 3.8])
            .unwrap();
        trace
            .add_signal("y", vec![-1.5, 1.9, -3.3, -3.3, -1.7, -4.0])
            .unwrap();
        let dense = phi.robustness_dense_signal(&trace).unwrap();
        assert!(dense.iter().all(|v| !v.is_nan()), "dense had NaN: {dense:?}");
        assert_eq!(dense[4], f64::NEG_INFINITY);
        assert_eq!(dense[5], f64::NEG_INFINITY);
        assert!(dense[..4].iter().all(|v| v.is_finite()), "leading samples finite: {dense:?}");
    }

    #[test]
    fn probabilistic_operator_is_rejected_by_deterministic_robustness() {
        let phi = Formula::parse("P>=0.9(x > 0)").unwrap();
        let mut trace = Trace::new([0.0]).unwrap();
        trace.add_signal("x", [1.0]).unwrap();
        assert!(matches!(
            phi.robustness(&trace),
            Err(crate::Error::ProbabilisticOperator)
        ));
    }

    #[test]
    fn empty_trace_is_an_error() {
        let phi = Formula::parse("x > 0").unwrap();
        let trace = Trace::default();
        assert!(matches!(
            phi.robustness(&trace),
            Err(crate::Error::EmptyTrace)
        ));
    }
}