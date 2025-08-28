//! Sliding-window minimum and maximum over a monotonic deque.
//!
//! The deque invariant is proved in `proofs/`.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
#[cfg(feature = "std")]
use std::collections::VecDeque;

/// For each index `i`, the minimum of `values` over the inclusive window
/// `[t_i + off_a, t_i + off_b]`, where `t = times`.
pub(crate) fn sliding_window_min(
    values: &[f64],
    times: &[f64],
    off_a: f64,
    off_b: f64,
) -> Vec<f64> {
    sweep(values, times, off_a, off_b, Extremum::Min)
}

/// The dual of [`sliding_window_min`].
pub(crate) fn sliding_window_max(
    values: &[f64],
    times: &[f64],
    off_a: f64,
    off_b: f64,
) -> Vec<f64> {
    sweep(values, times, off_a, off_b, Extremum::Max)
}

#[derive(Clone, Copy)]
enum Extremum {
    Min,
    Max,
}

impl Extremum {
    fn neutral(self) -> f64 {
        match self {
            Extremum::Min => f64::INFINITY,
            Extremum::Max => f64::NEG_INFINITY,
        }
    }

    fn dominated(self, back: f64, incoming: f64) -> bool {
        match self {
            Extremum::Min => back >= incoming,
            Extremum::Max => back <= incoming,
        }
    }
}

fn sweep(values: &[f64], times: &[f64], off_a: f64, off_b: f64, extremum: Extremum) -> Vec<f64> {
    let n = values.len();
    let mut result = vec![extremum.neutral(); n];
    let mut candidates: VecDeque<usize> = VecDeque::new();
    let mut right = 0;

    for i in 0..n {
        let window_left = times[i] + off_a;
        let window_right = times[i] + off_b;

        while right < n && times[right] <= window_right {
            let v = values[right];
            while candidates.back().is_some_and(|&b| extremum.dominated(values[b], v)) {
                candidates.pop_back();
            }
            candidates.push_back(right);
            right += 1;
        }

        // Drop candidates that have fallen off the window's left edge.
        while let Some(&front) = candidates.front() {
            if times[front] < window_left {
                candidates.pop_front();
            } else {
                break;
            }
        }

        if let Some(&front) = candidates.front() {
            result[i] = values[front];
        }
    }

    result
}

/// The online counterpart of [`sliding_window_min`].
#[derive(Debug, Default)]
pub(crate) struct MonotonicDeque {
    window: VecDeque<(f64, f64)>,
}

impl MonotonicDeque {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            window: VecDeque::with_capacity(capacity),
        }
    }

    pub(crate) fn push_min(&mut self, time: f64, value: f64) {
        if value.is_nan() || time.is_nan() {
            return;
        }
        while self.window.back().is_some_and(|&(_, back)| back >= value) {
            self.window.pop_back();
        }
        self.window.push_back((time, value));
    }

    pub(crate) fn push_max(&mut self, time: f64, value: f64) {
        if value.is_nan() || time.is_nan() {
            return;
        }
        while self.window.back().is_some_and(|&(_, back)| back <= value) {
            self.window.pop_back();
        }
        self.window.push_back((time, value));
    }

    /// Drops front entries strictly before `time_limit`.
    pub(crate) fn evict_before(&mut self, time_limit: f64) {
        while self.window.front().is_some_and(|&(t, _)| t < time_limit) {
            self.window.pop_front();
        }
    }

    pub(crate) fn front_value(&self) -> Option<f64> {
        self.window.front().map(|&(_, v)| v)
    }

    pub(crate) fn clear(&mut self) {
        self.window.clear();
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the deque selects an exact value from the window, so equality with the naive scan is exact"
    )]

    use super::*;
    use proptest::prelude::*;

    fn naive(values: &[f64], times: &[f64], off_a: f64, off_b: f64, want_min: bool) -> Vec<f64> {
        times
            .iter()
            .map(|&center| {
                let (lower, upper) = (center + off_a, center + off_b);
                let init = if want_min {
                    f64::INFINITY
                } else {
                    f64::NEG_INFINITY
                };
                times
                    .iter()
                    .zip(values)
                    .filter(|(&t, _)| t >= lower && t <= upper)
                    .fold(
                        init,
                        |acc, (_, &v)| {
                            if want_min {
                                acc.min(v)
                            } else {
                                acc.max(v)
                            }
                        },
                    )
            })
            .collect()
    }

    #[test]
    fn matches_a_worked_example() {
        let times = [0.0, 1.0, 2.0, 3.0, 4.0];
        let values = [5.0, 2.0, 7.0, 1.0, 3.0];
        assert_eq!(
            sliding_window_min(&values, &times, 0.0, 2.0),
            vec![2.0, 1.0, 1.0, 1.0, 3.0]
        );
        assert_eq!(
            sliding_window_max(&values, &times, 0.0, 2.0),
            vec![7.0, 7.0, 7.0, 3.0, 3.0]
        );
    }

    fn times_and_values() -> impl Strategy<Value = (Vec<f64>, Vec<f64>)> {
        prop::collection::vec((0.1f64..5.0, -100.0f64..100.0), 1..40).prop_map(|pairs| {
            let mut t = 0.0;
            let mut times = Vec::with_capacity(pairs.len());
            let mut values = Vec::with_capacity(pairs.len());
            for (gap, value) in pairs {
                t += gap;
                times.push(t);
                values.push(value);
            }
            (times, values)
        })
    }

    proptest! {
        #[test]
        fn deque_equals_naive_for_bounded_windows(
            (times, values) in times_and_values(),
            a in 0.0f64..10.0,
            width in 0.0f64..20.0,
        ) {
            let b = a + width;
            prop_assert_eq!(sliding_window_min(&values, &times, a, b), naive(&values, &times, a, b, true));
            prop_assert_eq!(sliding_window_max(&values, &times, a, b), naive(&values, &times, a, b, false));
            prop_assert_eq!(sliding_window_min(&values, &times, -b, -a), naive(&values, &times, -b, -a, true));
            prop_assert_eq!(sliding_window_max(&values, &times, -b, -a), naive(&values, &times, -b, -a, false));
        }

        #[test]
        fn deque_equals_naive_for_unbounded_windows(
            (times, values) in times_and_values(),
            a in 0.0f64..10.0,
        ) {
            prop_assert_eq!(
                sliding_window_min(&values, &times, a, f64::INFINITY),
                naive(&values, &times, a, f64::INFINITY, true)
            );
            prop_assert_eq!(
                sliding_window_max(&values, &times, f64::NEG_INFINITY, -a),
                naive(&values, &times, f64::NEG_INFINITY, -a, false)
            );
        }
    }
}