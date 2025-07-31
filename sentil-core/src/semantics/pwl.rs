//! Piecewise-linear signals, the carrier for dense-time robustness.

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

    /// The extremum of the signal over the closed interval `[lo, hi]`, where
    /// either bound may be infinite. Because the signal is linear between
    /// breakpoints, the extremum sits at an interior breakpoint or at an edge.
    fn extremum_over(&self, lo: f64, hi: f64, take_min: bool) -> f64 {
        let pick = |a: f64, b: f64| if take_min { a.min(b) } else { a.max(b) };
        let mut acc = pick(self.at(lo), self.at(hi));
        for &(t, v) in &self.points {
            if t > lo && t < hi {
                acc = pick(acc, v);
            }
        }
        acc
    }
}

/// Combines two signals pointwise with `op`, inserting the times where they
/// cross so the result stays exact between the inputs' breakpoints.
pub(crate) fn combine(a: &Pwl, b: &Pwl, op: fn(f64, f64) -> f64) -> Pwl {
    let mut times: Vec<f64> = a.times().chain(b.times()).collect();
    times.sort_by(f64::total_cmp);
    times.dedup();

    let mut crossings = Vec::new();
    for pair in times.windows(2) {
        let (t0, t1) = (pair[0], pair[1]);
        let d0 = a.at(t0) - b.at(t0);
        let d1 = a.at(t1) - b.at(t1);
        if d0 != 0.0 && d1 != 0.0 && (d0 < 0.0) != (d1 < 0.0) {
            let crossing = t0 + (t1 - t0) * d0 / (d0 - d1);
            if crossing > t0 && crossing < t1 {
                crossings.push(crossing);
            }
        }
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

/// The sliding-window extremum of `child`, where the window at query time `t`
/// is `[t + off_a, t + off_b]`. Future operators pass `(a, b)`; past operators
/// pass `(-b, -a)`. An infinite offset opens that side of the window.
pub(crate) fn window(child: &Pwl, off_a: f64, off_b: f64, take_min: bool) -> Pwl {
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

    Pwl::new(
        queries
            .into_iter()
            .map(|t| (t, child.extremum_over(t + off_a, t + off_b, take_min)))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "these piecewise-linear values are exact")]

    use super::*;

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
    fn unbounded_window_takes_the_suffix_extremum() {
        let x = Pwl::new(vec![(0.0, 3.0), (1.0, -2.0), (2.0, 5.0)]);
        let eventually = window(&x, 0.0, f64::INFINITY, false);
        assert_eq!(eventually.at(0.0), 5.0);
        assert_eq!(eventually.at(1.5), 5.0);
    }
}