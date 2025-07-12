//! The signal samples a formula is checked against.

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

}