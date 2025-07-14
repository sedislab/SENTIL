//! Sliding-window minimum and maximum over a monotonic deque.
//!
//! The deque invariant is proved in `proofs/`.

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
    window: std::collections::VecDeque<(f64, f64)>,
}

impl MonotonicDeque {
    pub(crate) fn new() -> Self {
        Self::default()
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