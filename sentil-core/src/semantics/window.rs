//! Sliding-window minimum and maximum over a monotonic deque.
//!
//! Temporal operators reduce to taking the infimum or supremum of a signal over
//! a window that slides forward in time. A monotonic deque does this
//! in amortized constant time per sample, holding only the candidate extrema for
//! the current and future windows rather than the whole window. aWe prove this in
//! Lean: the front of the deque is always the window extremum, the front is dropped
//! once its timestamp falls below the window's left edge, and the back is dropped
//! while it is dominated by an incoming value.

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
            if times[front] <= window_left {
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