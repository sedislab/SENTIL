//! The signal samples a formula is checked against.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
#[cfg(feature = "std")]
use std::collections::BTreeMap;

use crate::error::{Error, Result};

/// A set of signals sampled at a shared sequence of times.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Trace {
    times: Vec<f64>,
    signals: BTreeMap<String, Vec<f64>>,
}

impl Trace {
    /// Builds an empty trace over the given sample times.
    ///
    /// ```
    /// use sentil::Trace;
    ///
    /// let mut trace = Trace::new([0.0, 1.0, 2.0])?;
    /// trace.add_signal("x", [10.0, 5.0, 1.0])?;
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if a time is not finite or the times do not strictly
    /// increase.
    pub fn new(times: impl Into<Vec<f64>>) -> Result<Self> {
        let times = times.into();
        let mut previous: Option<f64> = None;
        for &t in &times {
            if !t.is_finite() {
                return Err(Error::NonFiniteSample {
                    kind: "time",
                    value: t,
                });
            }
            if let Some(p) = previous {
                if t <= p {
                    return Err(Error::NonMonotonicTime {
                        previous: p,
                        time: t,
                    });
                }
            }
            previous = Some(t);
        }
        Ok(Self {
            times,
            signals: BTreeMap::new(),
        })
    }

    /// Builds a trace with a single signal in one step.
    ///
    /// ```
    /// use sentil::Trace;
    ///
    /// let trace = Trace::from_signal([0.0, 1.0, 2.0], "x", [10.0, 5.0, 1.0])?;
    /// assert_eq!(trace.variables(), vec!["x"]);
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the times are not finite or strictly increasing, or
    /// if the signal's length does not match the number of time points.
    pub fn from_signal(
        times: impl Into<Vec<f64>>,
        name: &str,
        values: impl Into<Vec<f64>>,
    ) -> Result<Self> {
        let mut trace = Self::new(times)?;
        trace.add_signal(name, values)?;
        Ok(trace)
    }

    /// Builds an empty trace with unit-spaced integer times `0, 1, ..., len - 1`.
    pub fn indexed(len: usize) -> Self {
        #[allow(
            clippy::cast_precision_loss,
            reason = "trace lengths are far below 2^53, so the index-to-time cast is exact"
        )]
        let times = (0..len).map(|i| i as f64).collect();
        Self {
            times,
            signals: BTreeMap::new(),
        }
    }

    /// Adds a signal, one value per time point.
    ///
    /// # Errors
    ///
    /// Returns an error if the signal's length does not match the number of
    /// time points or if any value is not finite.
    pub fn add_signal(&mut self, name: &str, values: impl Into<Vec<f64>>) -> Result<()> {
        let values = values.into();
        if values.len() != self.times.len() {
            return Err(Error::SignalLengthMismatch {
                variable: name.to_owned(),
                expected: self.times.len(),
                found: values.len(),
            });
        }
        for &v in &values {
            if !v.is_finite() {
                return Err(Error::NonFiniteSample {
                    kind: "value",
                    value: v,
                });
            }
        }
        self.signals.insert(name.to_owned(), values);
        Ok(())
    }

    /// Overwrites an existing signal's values in place from an iterator, reusing
    /// the column's allocation so a trace replayed many times draws no fresh
    /// memory. The signal must already exist, the iterator must yield exactly one
    /// value per time point, and every value must be finite, as in
    /// [`add_signal`](Self::add_signal).
    pub(crate) fn refill_signal(
        &mut self,
        name: &str,
        values: impl IntoIterator<Item = f64>,
    ) -> Result<()> {
        let expected = self.times.len();
        let column = self
            .signals
            .get_mut(name)
            .ok_or_else(|| Error::UnknownVariable {
                name: name.to_owned(),
            })?;
        column.clear();
        for v in values {
            if !v.is_finite() {
                return Err(Error::NonFiniteSample {
                    kind: "value",
                    value: v,
                });
            }
            column.push(v);
        }
        if column.len() != expected {
            return Err(Error::SignalLengthMismatch {
                variable: name.to_owned(),
                expected,
                found: column.len(),
            });
        }
        Ok(())
    }

    /// The sample times, in increasing order.
    pub fn times(&self) -> &[f64] {
        &self.times
    }

    /// The number of time points.
    pub fn len(&self) -> usize {
        self.times.len()
    }

    /// Whether the trace has no time points.
    pub fn is_empty(&self) -> bool {
        self.times.is_empty()
    }

    /// The names of the signals in the trace, in sorted order.
    pub fn variables(&self) -> Vec<&str> {
        self.signals.keys().map(String::as_str).collect()
    }

    pub(crate) fn signals(&self) -> &BTreeMap<String, Vec<f64>> {
        &self.signals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_trace_and_lists_signals_sorted() {
        let mut trace = Trace::new([0.0, 1.0, 2.0]).unwrap();
        trace.add_signal("speed", [10.0, 5.0, 1.0]).unwrap();
        trace.add_signal("altitude", [100.0, 90.0, 80.0]).unwrap();
        assert_eq!(trace.len(), 3);
        assert_eq!(trace.variables(), ["altitude", "speed"]);
    }

    #[test]
    fn indexed_trace_uses_integer_times() {
        let trace = Trace::indexed(4);
        assert_eq!(trace.times(), &[0.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn non_monotonic_times_are_rejected() {
        let err = Trace::new([0.0, 2.0, 1.0]).unwrap_err();
        assert!(matches!(err, Error::NonMonotonicTime { .. }));
    }

    #[test]
    fn non_finite_time_is_rejected() {
        let err = Trace::new([0.0, f64::NAN]).unwrap_err();
        assert!(matches!(err, Error::NonFiniteSample { kind: "time", .. }));
    }

    #[test]
    fn mismatched_signal_length_is_rejected() {
        let mut trace = Trace::new([0.0, 1.0, 2.0]).unwrap();
        let err = trace.add_signal("x", [1.0, 2.0]).unwrap_err();
        assert!(matches!(
            err,
            Error::SignalLengthMismatch {
                expected: 3,
                found: 2,
                ..
            }
        ));
    }

    #[test]
    fn non_finite_value_is_rejected() {
        let mut trace = Trace::new([0.0, 1.0]).unwrap();
        let err = trace.add_signal("x", [1.0, f64::INFINITY]).unwrap_err();
        assert!(matches!(err, Error::NonFiniteSample { kind: "value", .. }));
    }
}