//! A fixed-capacity ring buffer for streaming samples.

#[cfg(not(feature = "std"))]
use crate::prelude::*;

use crate::error::{Error, Result};

const TIME_EPSILON: f64 = 1e-9;

/// A fixed-capacity, time-ordered ring buffer of `(time, value)` samples.
///
/// ```
/// use sentil::RingBuffer;
///
/// let mut buffer = RingBuffer::new(2)?;
/// buffer.push(0.0, 10.0)?;
/// buffer.push(1.0, 20.0)?;
/// let evicted = buffer.push(2.0, 30.0)?;
/// assert_eq!(evicted, Some((0.0, 10.0)));
/// assert_eq!(buffer.front(), Some((1.0, 20.0)));
/// # Ok::<(), sentil::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct RingBuffer {
    data: Vec<(f64, f64)>,
    head: usize,
    len: usize,
    mean: f64,
    m2: f64,
}

impl RingBuffer {
    /// Creates an empty buffer that holds at most `capacity` samples.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if `capacity` is zero.
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 {
            return Err(Error::InvalidConfig {
                context: "ring buffer",
                message: "capacity must be at least 1".to_owned(),
            });
        }
        Ok(Self {
            data: vec![(0.0, 0.0); capacity],
            head: 0,
            len: 0,
            mean: 0.0,
            m2: 0.0,
        })
    }

    /// The capacity the buffer was created with.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    /// The number of samples currently held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the buffer holds no samples.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether the buffer is at capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.len == self.capacity()
    }

    fn slot(&self, i: usize) -> usize {
        (self.head + i) % self.capacity()
    }

    /// The sample at logical position `index`.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<(f64, f64)> {
        (index < self.len).then(|| self.data[self.slot(index)])
    }

    /// The oldest sample.
    #[must_use]
    pub fn front(&self) -> Option<(f64, f64)> {
        self.get(0)
    }

    /// The newest sample.
    #[must_use]
    pub fn back(&self) -> Option<(f64, f64)> {
        (self.len > 0).then(|| self.data[self.slot(self.len - 1)])
    }

    /// Drops every sample and resets the running statistics.
    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
        self.mean = 0.0;
        self.m2 = 0.0;
    }

    /// Appends a sample, evicting and returning the oldest if the buffer is full.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NonFiniteSample`] for a non-finite time or value, or
    /// [`Error::NonMonotonicTime`] if `time` precedes the newest sample.
    pub fn push(&mut self, time: f64, value: f64) -> Result<Option<(f64, f64)>> {
        if !time.is_finite() {
            return Err(Error::NonFiniteSample {
                kind: "time",
                value: time,
            });
        }
        if !value.is_finite() {
            return Err(Error::NonFiniteSample {
                kind: "value",
                value,
            });
        }
        if let Some((prev, _)) = self.back() {
            if time < prev - TIME_EPSILON {
                return Err(Error::NonMonotonicTime {
                    previous: prev,
                    time,
                });
            }
        }
        let evicted = if self.is_full() {
            let old = self.data[self.head];
            self.head = self.slot(1);
            self.len -= 1;
            self.welford_remove(old.1);
            Some(old)
        } else {
            None
        };
        let write = self.slot(self.len);
        self.data[write] = (time, value);
        self.len += 1;
        self.welford_add(value);
        Ok(evicted)
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "the sample count is bounded by the capacity and fits f64 exactly"
    )]
    fn welford_add(&mut self, value: f64) {
        let n = self.len as f64;
        let delta = value - self.mean;
        self.mean += delta / n;
        self.m2 += delta * (value - self.mean);
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "the sample count is bounded by the capacity and fits f64 exactly"
    )]
    fn welford_remove(&mut self, value: f64) {
        let n = self.len as f64;
        if n == 0.0 {
            self.mean = 0.0;
            self.m2 = 0.0;
            return;
        }
        let delta = value - self.mean;
        self.mean -= delta / n;
        self.m2 -= delta * (value - self.mean);
        if self.m2 < 0.0 {
            self.m2 = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "these buffer values are exact")]

    use super::*;

    #[test]
    fn pushes_and_evicts_in_order() {
        let mut buffer = RingBuffer::new(3).unwrap();
        assert!(buffer.is_empty());
        for (t, v) in [(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)] {
            assert_eq!(buffer.push(t, v).unwrap(), None);
        }
        assert!(buffer.is_full());
        assert_eq!(buffer.push(3.0, 4.0).unwrap(), Some((0.0, 1.0)));
        assert_eq!(buffer.front(), Some((1.0, 2.0)));
        assert_eq!(buffer.back(), Some((3.0, 4.0)));
        assert_eq!(buffer.get(1), Some((2.0, 3.0)));
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn rejects_a_zero_capacity_and_bad_samples() {
        assert!(RingBuffer::new(0).is_err());
        let mut buffer = RingBuffer::new(2).unwrap();
        buffer.push(1.0, 1.0).unwrap();
        assert!(buffer.push(0.5, 1.0).is_err());
        assert!(buffer.push(2.0, f64::NAN).is_err());
    }
}