//! Piecewise-linear signals, the carrier for dense-time robustness.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
#[cfg(feature = "std")]
use std::collections::VecDeque;

/// A continuous piecewise-linear function of time.
#[derive(Debug, Clone)]
pub(crate) struct Pwl {
    points: Vec<(f64, f64)>,
}

impl Pwl {
    pub(crate) fn new(mut points: Vec<(f64, f64)>) -> Self {
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        #[allow(
            clippy::float_cmp,
            reason = "merging breakpoints that fall at the exact same time is intended"
        )]
        points.dedup_by(|a, b| a.0 == b.0);
        Self { points }
    }

    pub(crate) fn at(&self, t: f64) -> f64 {
        let first = self.points[0];
        let last = self.points[self.points.len() - 1];
        if t <= first.0 {
            return first.1;
        }
        if t >= last.0 {
            return last.1;
        }
        let upper = self.points.partition_point(|p| p.0 <= t);
        let (t0, v0) = self.points[upper - 1];
        let (t1, v1) = self.points[upper];
        // A segment touching an infinity is read as a left-held step.
        if !v0.is_finite() || !v1.is_finite() {
            return v0;
        }
        v0 + (v1 - v0) * (t - t0) / (t1 - t0)
    }

    pub(crate) fn domain(&self) -> (f64, f64) {
        (self.points[0].0, self.points[self.points.len() - 1].0)
    }

    pub(crate) fn negate(&self) -> Pwl {
        Pwl {
            points: self.points.iter().map(|&(t, v)| (t, -v)).collect(),
        }
    }

    pub(crate) fn times(&self) -> impl Iterator<Item = f64> + '_ {
        self.points.iter().map(|&(t, _)| t)
    }
}

/// Where two functions affine on `[t0, t1]` meet, from the signed gap `d0`, `d1`
/// between them at the ends.
pub(super) fn crossing(t0: f64, t1: f64, d0: f64, d1: f64) -> Option<f64> {
    if d0 != 0.0 && d1 != 0.0 && (d0 < 0.0) != (d1 < 0.0) {
        let t = t0 + (t1 - t0) * d0 / (d0 - d1);
        if t > t0 && t < t1 {
            return Some(t);
        }
    }
    None
}

/// Combines two signals pointwise with `op`, inserting the times where they cross.
pub(crate) fn combine(a: &Pwl, b: &Pwl, op: fn(f64, f64) -> f64) -> Pwl {
    let mut times: Vec<f64> = a.times().chain(b.times()).collect();
    times.sort_by(f64::total_cmp);
    times.dedup();

    let mut crossings = Vec::new();
    for pair in times.windows(2) {
        let (t0, t1) = (pair[0], pair[1]);
        let d0 = a.at(t0) - b.at(t0);
        let d1 = a.at(t1) - b.at(t1);
        crossings.extend(crossing(t0, t1, d0, d1));
    }
    times.extend(crossings);
    times.sort_by(f64::total_cmp);
    times.dedup();

    Pwl::new(
        times
            .into_iter()
            .map(|t| (t, op(a.at(t), b.at(t))))
            .collect(),
    )
}

fn window_queries(child: &Pwl, off_a: f64, off_b: f64) -> Vec<f64> {
    let (lo_t, hi_t) = child.domain();
    let mut queries = vec![lo_t, hi_t];
    for s in child.times() {
        for offset in [off_a, off_b] {
            if offset.is_finite() {
                let q = s - offset;
                if q >= lo_t && q <= hi_t {
                    queries.push(q);
                }
            }
        }
    }
    queries.sort_by(f64::total_cmp);
    queries.dedup();
    queries
}

/// The extremum over the breakpoints strictly inside the window at every query.
fn interior(child: &Pwl, queries: &[f64], off_a: f64, off_b: f64, take_min: bool) -> Vec<f64> {
    let pick = |a: f64, b: f64| if take_min { a.min(b) } else { a.max(b) };
    let none = if take_min {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    let pts = &child.points;
    let n = pts.len();
    let mut out = Vec::with_capacity(queries.len());
    match (off_a.is_finite(), off_b.is_finite()) {
        (true, true) => {
            let mut deque: VecDeque<(f64, f64)> = VecDeque::new();
            let mut right = 0;
            for &t in queries {
                let (lo, hi) = (t + off_a, t + off_b);
                while right < n && pts[right].0 < hi {
                    let v = pts[right].1;
                    while deque
                        .back()
                        .is_some_and(|&(_, bv)| if take_min { bv >= v } else { bv <= v })
                    {
                        deque.pop_back();
                    }
                    deque.push_back((pts[right].0, v));
                    right += 1;
                }
                while deque.front().is_some_and(|&(tb, _)| tb <= lo) {
                    deque.pop_front();
                }
                out.push(deque.front().map_or(none, |&(_, v)| v));
            }
        }
        (false, true) => {
            let mut right = 0;
            let mut acc = none;
            for &t in queries {
                let hi = t + off_b;
                while right < n && pts[right].0 < hi {
                    acc = pick(acc, pts[right].1);
                    right += 1;
                }
                out.push(acc);
            }
        }
        (true, false) => {
            let mut suffix = vec![pts[n - 1].1; n];
            for i in (0..n - 1).rev() {
                suffix[i] = pick(pts[i].1, suffix[i + 1]);
            }
            let mut j = 0;
            for &t in queries {
                let lo = t + off_a;
                while j < n && pts[j].0 <= lo {
                    j += 1;
                }
                out.push(if j < n { suffix[j] } else { none });
            }
        }
        (false, false) => {
            let g = pts.iter().fold(none, |a, &(_, v)| pick(a, v));
            out.resize(queries.len(), g);
        }
    }
    out
}

/// Reads `child` at `t + shift` for an ascending `times`.
fn sample_shifted(child: &Pwl, times: &[f64], shift: f64) -> Vec<f64> {
    let pts = &child.points;
    let (first, last) = (pts[0], pts[pts.len() - 1]);
    let mut upper = 1;
    let mut out = Vec::with_capacity(times.len());
    for &q in times {
        let t = q + shift;
        if t <= first.0 {
            out.push(first.1);
            continue;
        }
        if t >= last.0 {
            out.push(last.1);
            continue;
        }
        while pts[upper].0 <= t {
            upper += 1;
        }
        let (t0, v0) = pts[upper - 1];
        let (t1, v1) = pts[upper];
        out.push(if v0.is_finite() && v1.is_finite() {
            v0 + (v1 - v0) * (t - t0) / (t1 - t0)
        } else {
            v0
        });
    }
    out
}

/// The times where the windowed result bends between two neighbouring queries.
fn window_bends(
    child: &Pwl,
    queries: &[f64],
    lows: &[f64],
    highs: &[f64],
    off_a: f64,
    off_b: f64,
    take_min: bool,
) -> Vec<f64> {
    let mids: Vec<f64> = queries
        .windows(2)
        .map(|q| q[0] + (q[1] - q[0]) / 2.0)
        .collect();
    let levels = interior(child, &mids, off_a, off_b, take_min);
    let mut bends = Vec::new();
    for (i, c) in levels.into_iter().enumerate() {
        let (t0, t1) = (queries[i], queries[i + 1]);
        let (lo0, hi0) = (lows[i], highs[i]);
        let (lo1, hi1) = (lows[i + 1], highs[i + 1]);
        bends.extend(crossing(t0, t1, lo0 - hi0, lo1 - hi1));
        bends.extend(crossing(t0, t1, lo0 - c, lo1 - c));
        bends.extend(crossing(t0, t1, hi0 - c, hi1 - c));
    }
    bends
}

/// The sliding-window extremum of `child`, where the window at query time `t`
/// is `[t + off_a, t + off_b]`.
pub(crate) fn window(child: &Pwl, off_a: f64, off_b: f64, take_min: bool) -> Pwl {
    let pick = |a: f64, b: f64| if take_min { a.min(b) } else { a.max(b) };
    let mut queries = window_queries(child, off_a, off_b);
    let lows = sample_shifted(child, &queries, off_a);
    let highs = sample_shifted(child, &queries, off_b);
    let bends = window_bends(child, &queries, &lows, &highs, off_a, off_b, take_min);
    let (lows, highs) = if bends.is_empty() {
        (lows, highs)
    } else {
        queries.extend(bends);
        queries.sort_by(f64::total_cmp);
        queries.dedup();
        (
            sample_shifted(child, &queries, off_a),
            sample_shifted(child, &queries, off_b),
        )
    };
    let inside = interior(child, &queries, off_a, off_b, take_min);
    Pwl::new(
        queries
            .iter()
            .zip(inside)
            .enumerate()
            .map(|(i, (&t, c))| (t, pick(pick(lows[i], highs[i]), c)))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "these piecewise-linear values are exact")]

    use proptest::prelude::*;

    use super::*;

    fn naive_at(child: &Pwl, t: f64, off_a: f64, off_b: f64, take_min: bool) -> f64 {
        let pick = |a: f64, b: f64| if take_min { a.min(b) } else { a.max(b) };
        let (lo, hi) = (t + off_a, t + off_b);
        let mut acc = pick(child.at(lo), child.at(hi));
        for &(tb, v) in &child.points {
            if tb > lo && tb < hi {
                acc = pick(acc, v);
            }
        }
        acc
    }

    #[test]
    fn interpolates_between_breakpoints_and_holds_outside() {
        let p = Pwl::new(vec![(0.0, 0.0), (2.0, 4.0)]);
        assert_eq!(p.at(-1.0), 0.0);
        assert_eq!(p.at(1.0), 2.0);
        assert_eq!(p.at(2.0), 4.0);
        assert_eq!(p.at(5.0), 4.0);
    }

    #[test]
    fn pointwise_minimum_inserts_the_crossing() {
        let rising = Pwl::new(vec![(0.0, -1.0), (2.0, 1.0)]);
        let falling = Pwl::new(vec![(0.0, 1.0), (2.0, -1.0)]);
        let lower = combine(&rising, &falling, f64::min);
        assert_eq!(lower.at(1.0), 0.0);
        assert_eq!(lower.at(0.0), -1.0);
        assert_eq!(lower.at(2.0), -1.0);
    }

    #[test]
    fn window_minimum_sees_a_sub_sample_dip_at_the_edge() {
        let x = Pwl::new(vec![(0.0, 1.0), (1.0, 1.0), (2.0, -3.0)]);
        let always = window(&x, 0.0, 1.5, true);
        assert_eq!(always.at(0.0), -1.0);
    }

    #[test]
    fn window_minimum_bends_where_the_two_edges_cross() {
        let x = Pwl::new(vec![
            (0.0, -4.0),
            (1.0, -4.0),
            (2.0, -3.0),
            (3.0, 4.0),
            (4.0, -4.0),
            (5.0, -3.0),
            (6.0, 5.0),
        ]);
        let always = window(&x, 0.5, 1.5, true);
        assert_eq!(always.at(1.5), -3.0);
        assert_eq!(always.at(2.5), -4.0);
        assert_eq!(always.at(2.0), 0.0);
    }

    #[test]
    fn window_minimum_bends_where_an_edge_meets_the_interior() {
        let x = Pwl::new(vec![
            (0.0, 0.0),
            (0.5, 10.0),
            (1.0, 3.0),
            (1.5, 10.0),
            (2.0, 0.0),
        ]);
        let always = window(&x, 0.0, 1.5, true);
        assert_eq!(always.at(0.0), 0.0);
        assert_eq!(always.at(0.5), 0.0);
        assert_eq!(always.at(0.25), 3.0);
    }

    #[test]
    fn at_a_breakpoint_next_to_an_infinity_reads_the_breakpoint_not_nan() {
        let p = Pwl::new(vec![(0.0, -3.3), (1.0, f64::NEG_INFINITY)]);
        assert_eq!(p.at(0.0), -3.3);
        assert_eq!(p.at(1.0), f64::NEG_INFINITY);
        let q = Pwl::new(vec![(0.0, f64::NEG_INFINITY), (1.0, f64::NEG_INFINITY)]);
        assert_eq!(q.at(0.5), f64::NEG_INFINITY);
        assert_eq!(p.at(0.5), -3.3);
    }

    #[test]
    fn unbounded_window_takes_the_suffix_extremum() {
        let x = Pwl::new(vec![(0.0, 3.0), (1.0, -2.0), (2.0, 5.0)]);
        let eventually = window(&x, 0.0, f64::INFINITY, false);
        assert_eq!(eventually.at(0.0), 5.0);
        assert_eq!(eventually.at(1.5), 5.0);
    }

    proptest! {
        #[test]
        fn window_matches_the_naive_scan(
            raw in prop::collection::vec((-40.0f64..40.0, -40.0f64..40.0), 1..24),
            a in -8.0f64..8.0,
            span in 0.0f64..8.0,
            open_past in any::<bool>(),
            open_future in any::<bool>(),
            take_min in any::<bool>(),
        ) {
            let child = Pwl::new(raw);
            let off_a = if open_past { f64::NEG_INFINITY } else { a };
            let off_b = if open_future { f64::INFINITY } else { a + span };
            let fast = window(&child, off_a, off_b, take_min);
            for &(t, v) in &fast.points {
                prop_assert_eq!(v, naive_at(&child, t, off_a, off_b, take_min));
            }
        }

        #[test]
        fn the_grid_carries_every_bend(
            start in -20.0f64..20.0,
            steps in prop::collection::vec((0.25f64..4.0, -40.0f64..40.0), 2..14),
            a in -8.0f64..8.0,
            span in 0.0f64..8.0,
            open_past in any::<bool>(),
            open_future in any::<bool>(),
            take_min in any::<bool>(),
        ) {
            let mut t = start;
            let child = Pwl::new(steps.into_iter().map(|(dt, v)| { t += dt; (t, v) }).collect());
            let off_a = if open_past { f64::NEG_INFINITY } else { a };
            let off_b = if open_future { f64::INFINITY } else { a + span };
            let result = window(&child, off_a, off_b, take_min);
            for pair in result.points.windows(2) {
                let mid = pair[0].0 + (pair[1].0 - pair[0].0) / 2.0;
                let want = naive_at(&child, mid, off_a, off_b, take_min);
                prop_assert!((result.at(mid) - want).abs() < 1e-9, "at {mid}: {} wanted {want}", result.at(mid));
            }
        }
    }
}