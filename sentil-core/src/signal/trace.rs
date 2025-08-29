//! The signal samples a formula is checked against.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
#[cfg(feature = "std")]
use std::collections::BTreeMap;

use super::interpolation::{read_at, Interpolation};
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

    #[cfg(feature = "statistical")]
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

    /// Moves the trace onto a new set of times, reading every signal between its
    /// samples with `interpolation`.
    ///
    /// ```
    /// use sentil::{Interpolation, Trace};
    ///
    /// let trace = Trace::from_signal([0.0, 2.0, 4.0], "x", [0.0, 4.0, 8.0])?;
    /// let dense = trace.resample([0.0, 1.0, 2.0, 3.0, 4.0], Interpolation::Linear)?;
    /// assert_eq!(dense.signal("x"), Some(&[0.0, 2.0, 4.0, 6.0, 8.0][..]));
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::EmptyTrace`](crate::Error::EmptyTrace) if the trace has no
    /// samples, and propagates the validation of the new times and values.
    pub fn resample(
        &self,
        times: impl Into<Vec<f64>>,
        interpolation: Interpolation,
    ) -> Result<Trace> {
        if self.times.is_empty() {
            return Err(Error::EmptyTrace);
        }
        let mut out = Trace::new(times)?;
        let queries = out.times.clone();
        for (name, values) in &self.signals {
            out.add_signal(name, read_at(&self.times, values, interpolation, &queries))?;
        }
        Ok(out)
    }

    /// The value series for one signal, or `None` if the trace has no such signal.
    #[must_use]
    pub fn signal(&self, name: &str) -> Option<&[f64]> {
        self.signals.get(name).map(Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the refilled values are exact and integer-valued"
    )]

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
    fn resample_moves_every_signal_onto_the_new_grid() {
        let mut trace = Trace::new([0.0, 2.0, 4.0]).unwrap();
        trace.add_signal("x", [0.0, 4.0, 8.0]).unwrap();
        trace.add_signal("y", [1.0, 3.0, 5.0]).unwrap();
        let out = trace.resample([1.0, 3.0], Interpolation::ZeroOrderHold).unwrap();
        assert_eq!(out.times(), &[1.0, 3.0]);
        assert_eq!(out.signal("x"), Some(&[0.0, 4.0][..]));
        assert_eq!(out.signal("y"), Some(&[1.0, 3.0][..]));
    }

    #[test]
    fn resample_of_an_empty_trace_is_rejected() {
        let trace = Trace::new([]).unwrap();
        let err = trace.resample([0.0, 1.0], Interpolation::Linear).unwrap_err();
        assert!(matches!(err, Error::EmptyTrace));
    }

    #[test]
    fn resample_validates_the_new_times() {
        let trace = Trace::from_signal([0.0, 1.0], "x", [0.0, 1.0]).unwrap();
        let err = trace
            .resample([1.0, 0.0], Interpolation::Linear)
            .unwrap_err();
        assert!(matches!(err, Error::NonMonotonicTime { .. }));
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

    #[cfg(feature = "statistical")]
    #[test]
    fn refill_signal_overwrites_in_place_and_validates() {
        let mut trace = Trace::new([0.0, 1.0, 2.0]).unwrap();
        trace.add_signal("x", [1.0, 2.0, 3.0]).unwrap();
        trace.refill_signal("x", [4.0, 5.0, 6.0]).unwrap();
        assert_eq!(trace.signals()["x"], vec![4.0, 5.0, 6.0]);
        let err = trace
            .refill_signal("x", [1.0, f64::NAN, 3.0])
            .unwrap_err();
        assert!(matches!(err, Error::NonFiniteSample { kind: "value", .. }));
        let err = trace.refill_signal("z", [0.0, 0.0, 0.0]).unwrap_err();
        assert!(matches!(err, Error::UnknownVariable { .. }));
    }
}