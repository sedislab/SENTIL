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

    fn times(&self) -> impl Iterator<Item = f64> + '_ {
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