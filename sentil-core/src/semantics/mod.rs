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