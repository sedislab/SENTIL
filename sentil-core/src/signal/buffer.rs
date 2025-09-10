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
            self.pop_front()
        } else {
            None
        };
        let write = self.slot(self.len);
        self.data[write] = (time, value);
        self.len += 1;
        self.welford_add(value);
        Ok(evicted)
    }

    /// Removes and returns the oldest sample.
    pub fn pop_front(&mut self) -> Option<(f64, f64)> {
        if self.len == 0 {
            return None;
        }
        let old = self.data[self.head];
        self.head = self.slot(1);
        self.len -= 1;
        self.welford_remove(old.1);
        Some(old)
    }

    /// Removes and returns the newest sample.
    pub fn pop_back(&mut self) -> Option<(f64, f64)> {
        if self.len == 0 {
            return None;
        }
        let last = self.data[self.slot(self.len - 1)];
        self.len -= 1;
        self.welford_remove(last.1);
        Some(last)
    }

    /// The earliest and latest sample times.
    #[must_use]
    pub fn time_range(&self) -> Option<(f64, f64)> {
        match (self.front(), self.back()) {
            (Some((first, _)), Some((last, _))) => Some((first, last)),
            _ => None,
        }
    }

    /// The running mean of the held values.
    #[must_use]
    pub fn mean(&self) -> Option<f64> {
        (self.len > 0).then_some(self.mean)
    }

    /// The Bessel-corrected sample variance, or `None` with fewer than two samples.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        reason = "the sample count is bounded by the capacity and fits f64 exactly"
    )]
    pub fn variance(&self) -> Option<f64> {
        (self.len >= 2).then(|| (self.m2 / (self.len as f64 - 1.0)).max(0.0))
    }

    /// The sample standard deviation.
    #[must_use]
    pub fn std_dev(&self) -> Option<f64> {
        self.variance().map(f64::sqrt)
    }

    /// Recomputes the running mean and squared deviation from the held samples.
    #[allow(
        clippy::cast_precision_loss,
        reason = "the sample count is bounded by the capacity and fits f64 exactly"
    )]
    pub fn recompute_statistics(&mut self) {
        if self.len == 0 {
            self.mean = 0.0;
            self.m2 = 0.0;
            return;
        }
        let n = self.len as f64;
        let mean = self.values().sum::<f64>() / n;
        self.m2 = self.values().map(|v| (v - mean) * (v - mean)).sum();
        self.mean = mean;
    }

    /// The smallest value held.
    #[must_use]
    pub fn min(&self) -> Option<f64> {
        self.values().reduce(f64::min)
    }

    /// The largest value held.
    #[must_use]
    pub fn max(&self) -> Option<f64> {
        self.values().reduce(f64::max)
    }

    /// The samples in time order.
    pub fn iter(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        (0..self.len).map(|i| self.data[self.slot(i)])
    }

    /// The values in time order.
    pub fn values(&self) -> impl Iterator<Item = f64> + '_ {
        self.iter().map(|(_, v)| v)
    }

    /// The sample times in time order.
    pub fn times(&self) -> impl Iterator<Item = f64> + '_ {
        self.iter().map(|(t, _)| t)
    }

    /// The most recent `count` samples in time order.
    pub fn recent(&self, count: usize) -> impl Iterator<Item = (f64, f64)> + '_ {
        let start = self.len.saturating_sub(count);
        (start..self.len).map(move |i| self.data[self.slot(i)])
    }

    fn lower_bound(&self, time: f64) -> usize {
        let mut lo = 0;
        let mut hi = self.len;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.data[self.slot(mid)].0 < time - TIME_EPSILON {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }

    /// The value sampled at `time`, within a small tolerance.
    #[must_use]
    pub fn at_time(&self, time: f64) -> Option<f64> {
        let idx = self.lower_bound(time);
        for candidate in [idx, idx.wrapping_sub(1)] {
            if candidate < self.len {
                let (t, v) = self.data[self.slot(candidate)];
                if (t - time).abs() < TIME_EPSILON {
                    return Some(v);
                }
            }
        }
        None
    }

    /// The sample whose time is closest to `time`.
    #[must_use]
    pub fn closest_to_time(&self, time: f64) -> Option<(f64, f64)> {
        if self.is_empty() {
            return None;
        }
        let idx = self.lower_bound(time);
        if idx == 0 {
            return self.front();
        }
        if idx >= self.len {
            return self.back();
        }
        let prev = self.data[self.slot(idx - 1)];
        let curr = self.data[self.slot(idx)];
        if (time - prev.0).abs() <= (curr.0 - time).abs() {
            Some(prev)
        } else {
            Some(curr)
        }
    }

    /// Every sample whose time lies in `[start, end]`, oldest first.
    #[must_use]
    pub fn between(&self, start: f64, end: f64) -> Vec<(f64, f64)> {
        if self.is_empty() || start > end {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut i = self.lower_bound(start);
        while i < self.len {
            let sample = self.data[self.slot(i)];
            if sample.0 > end + TIME_EPSILON {
                break;
            }
            out.push(sample);
            i += 1;
        }
        out
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

    #[test]
    fn time_lookups_and_stats() {
        let mut buffer = RingBuffer::new(4).unwrap();
        for (t, v) in [(0.0, 2.0), (1.0, 4.0), (2.0, 6.0), (3.0, 8.0)] {
            buffer.push(t, v).unwrap();
        }
        assert_eq!(buffer.at_time(2.0), Some(6.0));
        assert_eq!(buffer.at_time(2.5), None);
        assert_eq!(buffer.closest_to_time(2.4), Some((2.0, 6.0)));
        assert_eq!(buffer.between(1.0, 2.0), vec![(1.0, 4.0), (2.0, 6.0)]);
        assert_eq!(buffer.time_range(), Some((0.0, 3.0)));
        assert_eq!(buffer.mean(), Some(5.0));
        assert_eq!(buffer.min(), Some(2.0));
        assert_eq!(buffer.max(), Some(8.0));
        assert!((buffer.variance().unwrap() - 20.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn statistics_track_eviction() {
        let mut buffer = RingBuffer::new(2).unwrap();
        buffer.push(0.0, 10.0).unwrap();
        buffer.push(1.0, 20.0).unwrap();
        buffer.push(2.0, 30.0).unwrap();
        assert_eq!(buffer.mean(), Some(25.0));
        assert!((buffer.variance().unwrap() - 50.0).abs() < 1e-9);
        assert_eq!(buffer.between(1.0, 2.0), vec![(1.0, 20.0), (2.0, 30.0)]);
    }

    #[test]
    fn pops_from_either_end_and_tracks_statistics() {
        let mut buffer = RingBuffer::new(4).unwrap();
        for (t, v) in [(0.0, 2.0), (1.0, 4.0), (2.0, 6.0), (3.0, 8.0)] {
            buffer.push(t, v).unwrap();
        }
        assert_eq!(buffer.pop_back(), Some((3.0, 8.0)));
        assert_eq!(buffer.pop_front(), Some((0.0, 2.0)));
        assert_eq!(buffer.iter().collect::<Vec<_>>(), vec![(1.0, 4.0), (2.0, 6.0)]);
        assert_eq!(buffer.mean(), Some(5.0));
        buffer.push(4.0, 10.0).unwrap();
        assert_eq!(buffer.back(), Some((4.0, 10.0)));
        let mut empty = RingBuffer::new(2).unwrap();
        assert_eq!(empty.pop_front(), None);
        assert_eq!(empty.pop_back(), None);
    }

    #[test]
    fn recent_yields_the_last_samples_for_a_windowed_query() {
        let mut buffer = RingBuffer::new(5).unwrap();
        for (t, v) in [(0.0, 3.0), (1.0, 1.0), (2.0, 4.0), (3.0, 1.0), (4.0, 5.0)] {
            buffer.push(t, v).unwrap();
        }
        assert_eq!(buffer.recent(2).collect::<Vec<_>>(), vec![(3.0, 1.0), (4.0, 5.0)]);
        assert_eq!(buffer.recent(10).count(), 5);
        assert_eq!(buffer.recent(3).map(|(_, v)| v).reduce(f64::min), Some(1.0));
        assert_eq!(buffer.recent(3).map(|(_, v)| v).reduce(f64::max), Some(5.0));
    }

    #[test]
    fn recompute_statistics_matches_the_incremental_values() {
        let mut buffer = RingBuffer::new(8).unwrap();
        for i in 0..8 {
            buffer.push(f64::from(i), f64::from(i) * 1.5 - 3.0).unwrap();
        }
        let (mean, variance) = (buffer.mean(), buffer.variance());
        buffer.recompute_statistics();
        assert!((buffer.mean().unwrap() - mean.unwrap()).abs() < 1e-12);
        assert!((buffer.variance().unwrap() - variance.unwrap()).abs() < 1e-12);
    }
}