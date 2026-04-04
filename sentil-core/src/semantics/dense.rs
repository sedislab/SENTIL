//! Dense-time robustness over piecewise-linear signals.
//!
//! Predicates must be linear in the signals.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
#[cfg(feature = "std")]
use std::collections::BTreeMap;

use super::eval::eval_expr;
use super::pwl::{combine, crossing, window, Pwl};
use crate::error::{Error, Result};
use crate::formula::{BinaryOp, ComparisonOp, Expr, Formula, Predicate};

/// Slack for comparing window edges against breakpoint times.
const EPS: f64 = 1e-9;

pub(crate) fn robustness_signal(
    formula: &Formula,
    times: &[f64],
    signals: &BTreeMap<String, Vec<f64>>,
) -> Result<Pwl> {
    match formula {
        Formula::Predicate(p) => predicate(p, times, signals),
        Formula::Not(f) => Ok(robustness_signal(f, times, signals)?.negate()),
        Formula::And(l, r) => Ok(combine(
            &robustness_signal(l, times, signals)?,
            &robustness_signal(r, times, signals)?,
            f64::min,
        )),
        Formula::Or(l, r) => Ok(combine(
            &robustness_signal(l, times, signals)?,
            &robustness_signal(r, times, signals)?,
            f64::max,
        )),
        Formula::Implies(l, r) => Ok(combine(
            &robustness_signal(l, times, signals)?.negate(),
            &robustness_signal(r, times, signals)?,
            f64::max,
        )),
        Formula::Always(interval, f) => Ok(window(
            &robustness_signal(f, times, signals)?,
            interval.lower(),
            interval.upper_or_infinity(),
            true,
        )),
        Formula::Eventually(interval, f) => Ok(window(
            &robustness_signal(f, times, signals)?,
            interval.lower(),
            interval.upper_or_infinity(),
            false,
        )),
        Formula::Historically(interval, f) => Ok(window(
            &robustness_signal(f, times, signals)?,
            -interval.upper_or_infinity(),
            -interval.lower(),
            true,
        )),
        Formula::Once(interval, f) => Ok(window(
            &robustness_signal(f, times, signals)?,
            -interval.upper_or_infinity(),
            -interval.lower(),
            false,
        )),
        Formula::Until(interval, l, r) => Ok(until_signal(
            &robustness_signal(l, times, signals)?,
            &robustness_signal(r, times, signals)?,
            interval.lower(),
            interval.upper_or_infinity(),
        )),
        Formula::Since(interval, l, r) => Ok(since_signal(
            &robustness_signal(l, times, signals)?,
            &robustness_signal(r, times, signals)?,
            interval.lower(),
            interval.upper_or_infinity(),
        )),
        Formula::Probabilistic(..) => Err(Error::ProbabilisticOperator),
        Formula::Next(_) => Err(Error::Unsupported {
            feature: "next in dense time; use discrete robustness for it",
        }),
    }
}

/// A shared breakpoint grid for two signals, with both sampled on it.
fn common_grid(phi: &Pwl, psi: &Pwl) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut times: Vec<f64> = phi.times().chain(psi.times()).collect();
    times.sort_by(f64::total_cmp);
    times.dedup();
    let phi_vals = times.iter().map(|&t| phi.at(t)).collect();
    let psi_vals = times.iter().map(|&t| psi.at(t)).collect();
    (times, phi_vals, psi_vals)
}

/// Dense `phi U_[a,b] psi`.
fn until_signal(phi: &Pwl, psi: &Pwl, a: f64, b: f64) -> Pwl {
    let (times, phi_vals, psi_vals) = common_grid(phi, psi);
    let queries = result_grid(&times, a, b, true);
    let values: Vec<f64> = queries
        .iter()
        .map(|&t| until_at(t, &times, &phi_vals, &psi_vals, a, b))
        .collect();
    Pwl::new(queries.into_iter().zip(values).collect())
}

/// Dense `phi S_[a,b] psi`, the past dual of [`until_signal`].
fn since_signal(phi: &Pwl, psi: &Pwl, a: f64, b: f64) -> Pwl {
    let (times, phi_vals, psi_vals) = common_grid(phi, psi);
    let queries = result_grid(&times, a, b, false);
    let values: Vec<f64> = queries
        .iter()
        .map(|&t| since_at(t, &times, &phi_vals, &psi_vals, a, b))
        .collect();
    Pwl::new(queries.into_iter().zip(values).collect())
}

/// The times to evaluate an until or since result on.
fn result_grid(times: &[f64], a: f64, b: f64, future: bool) -> Vec<f64> {
    let (lo, hi) = (times[0], times[times.len() - 1]);
    let mut grid: Vec<f64> = times.to_vec();
    let sign = if future { -1.0 } else { 1.0 };
    for &g in times {
        for off in [a, b] {
            if off.is_finite() {
                let t = g + sign * off;
                if t >= lo && t <= hi {
                    grid.push(t);
                }
            }
        }
    }
    grid.sort_by(f64::total_cmp);
    grid.dedup();
    grid
}

/// The piecewise-linear value at `x`.
fn interp(times: &[f64], vals: &[f64], x: f64) -> f64 {
    if x <= times[0] {
        return vals[0];
    }
    let last = times.len() - 1;
    if x >= times[last] {
        return vals[last];
    }
    let upper = times.partition_point(|&t| t <= x);
    let (t0, v0) = (times[upper - 1], vals[upper - 1]);
    let (t1, v1) = (times[upper], vals[upper]);
    if !v0.is_finite() || !v1.is_finite() {
        return v0;
    }
    v0 + (v1 - v0) * (x - t0) / (t1 - t0)
}

/// The infimum of the piecewise-linear `phi` over `[lo, hi]`.
fn phi_inf_over(times: &[f64], phi_vals: &[f64], lo: f64, hi: f64) -> f64 {
    let mut inf = interp(times, phi_vals, lo).min(interp(times, phi_vals, hi));
    for (&g, &v) in times.iter().zip(phi_vals) {
        if g > lo && g < hi {
            inf = inf.min(v);
        }
    }
    inf
}

/// The maximum over one segment of `min(psi, phi)`, each linear from its value at
/// `u0` to its value at `u1`.
fn seg_max_of_min(psi0: f64, psi1: f64, phi0: f64, phi1: f64) -> f64 {
    let mut best = psi0.min(phi0).max(psi1.min(phi1));
    let (d0, d1) = (psi0 - phi0, psi1 - phi1);
    if d0.is_finite() && d1.is_finite() && d0 != 0.0 && d1 != 0.0 && (d0 < 0.0) != (d1 < 0.0) {
        let frac = d0 / (d0 - d1);
        let cross = psi0 + (psi1 - psi0) * frac;
        best = best.max(cross);
    }
    best
}

/// The exact `sup over s in [t+a, t+b] of min(psi(s), inf over [t, s) of phi)`.
fn until_at(t: f64, times: &[f64], phi_vals: &[f64], psi_vals: &[f64], a: f64, b: f64) -> f64 {
    let domain_end = times[times.len() - 1];
    let lo = t + a;
    if lo > domain_end + EPS {
        return f64::NEG_INFINITY;
    }
    let hi = if b.is_infinite() { domain_end } else { (t + b).min(domain_end) };
    windowed_sup(t, lo, hi, times, phi_vals, psi_vals, true)
}

/// The past dual of [`until_at`], over `s` in `[t-b, t-a]`.
fn since_at(t: f64, times: &[f64], phi_vals: &[f64], psi_vals: &[f64], a: f64, b: f64) -> f64 {
    let domain_start = times[0];
    let hi = t - a;
    if hi < domain_start - EPS {
        return f64::NEG_INFINITY;
    }
    let lo = if b.is_infinite() { domain_start } else { (t - b).max(domain_start) };
    windowed_sup(t, lo, hi, times, phi_vals, psi_vals, false)
}

/// The supremum of `min(psi(s), inf phi from t up to but not including s)` over `s`
/// in `[lo, hi]`, marching forward from `t` when `future`.
#[allow(clippy::too_many_arguments, reason = "the window, the two operand series, and the direction")]
fn windowed_sup(
    t: f64,
    lo: f64,
    hi: f64,
    times: &[f64],
    phi_vals: &[f64],
    psi_vals: &[f64],
    future: bool,
) -> f64 {
    if lo > hi + EPS {
        return f64::NEG_INFINITY;
    }
    let mut points: Vec<f64> = times
        .iter()
        .copied()
        .filter(|&g| g > lo + EPS && g < hi - EPS)
        .collect();
    let near = if future { lo } else { hi };
    points.push(lo);
    points.push(hi);
    points.sort_by(f64::total_cmp);
    points.dedup();
    if !future {
        points.reverse();
    }

    let mut floor = if future {
        phi_inf_over(times, phi_vals, t, near)
    } else {
        phi_inf_over(times, phi_vals, near, t)
    };
    // The infimum is half-open at the near edge.
    let near_floor = if (near - t).abs() <= EPS {
        f64::INFINITY
    } else {
        floor
    };
    let mut best = near_floor.min(interp(times, psi_vals, near));

    for pair in points.windows(2) {
        let (u0, u1) = (pair[0], pair[1]);
        let (psi0, psi1) = (interp(times, psi_vals, u0), interp(times, psi_vals, u1));
        let (phi0, phi1) = (interp(times, phi_vals, u0), interp(times, phi_vals, u1));
        let maxh = seg_max_of_min(psi0, psi1, phi0, phi1);
        best = best.max(floor.min(maxh));
        floor = floor.min(phi1.min(phi0));
    }
    best
}

fn predicate(p: &Predicate, times: &[f64], signals: &BTreeMap<String, Vec<f64>>) -> Result<Pwl> {
    if !is_linear(&p.lhs) || !is_linear(&p.rhs) {
        return Err(Error::Unsupported {
            feature: "dense-time evaluation needs linear predicates; rewrite it or use discrete robustness",
        });
    }
    let mut diff = Vec::with_capacity(times.len());
    for i in 0..times.len() {
        let lookup = |name: &str| signals.get(name).and_then(|col| col.get(i)).copied();
        diff.push(eval_expr(&p.lhs, &lookup)? - eval_expr(&p.rhs, &lookup)?);
    }
    let signal = match p.op {
        ComparisonOp::Less | ComparisonOp::LessEqual => {
            Pwl::new(times.iter().zip(&diff).map(|(&t, &d)| (t, -d)).collect())
        }
        ComparisonOp::Greater | ComparisonOp::GreaterEqual => {
            Pwl::new(times.iter().zip(&diff).map(|(&t, &d)| (t, d)).collect())
        }
        ComparisonOp::Equal => abs_margin(times, &diff, true),
        ComparisonOp::NotEqual => abs_margin(times, &diff, false),
    };
    Ok(signal)
}

/// The robustness of an equality or inequality predicate, `-|diff|` or `|diff|`.
fn abs_margin(times: &[f64], diff: &[f64], equality: bool) -> Pwl {
    let raw = Pwl::new(times.iter().zip(diff).map(|(&t, &d)| (t, d)).collect());
    let mut breakpoints: Vec<f64> = times.to_vec();
    for i in 0..times.len().saturating_sub(1) {
        breakpoints.extend(crossing(times[i], times[i + 1], diff[i], diff[i + 1]));
    }
    breakpoints.sort_by(f64::total_cmp);
    breakpoints.dedup();
    Pwl::new(
        breakpoints
            .into_iter()
            .map(|t| {
                let magnitude = raw.at(t).abs();
                (t, if equality { -magnitude } else { magnitude })
            })
            .collect(),
    )
}

/// Whether the term is a sum of scaled signals and constants.
fn is_linear(expr: &Expr) -> bool {
    if is_constant(expr) {
        return true;
    }
    match expr {
        Expr::Variable(_) => true,
        Expr::Binary(BinaryOp::Add | BinaryOp::Sub, l, r) => is_linear(l) && is_linear(r),
        Expr::Binary(BinaryOp::Mul, l, r) => {
            (is_constant(l) && is_linear(r)) || (is_linear(l) && is_constant(r))
        }
        Expr::Binary(BinaryOp::Div, l, r) => is_linear(l) && is_constant(r),
        _ => false,
    }
}

/// Whether the term references no signal.
fn is_constant(expr: &Expr) -> bool {
    match expr {
        Expr::Literal(_) => true,
        Expr::Variable(_) => false,
        Expr::Binary(_, l, r) => is_constant(l) && is_constant(r),
        Expr::Call(_, args) => args.iter().all(is_constant),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "the dense values here are exact")]
    #![allow(
        clippy::cast_precision_loss,
        reason = "test trace indices are tiny, so the index-to-time cast is exact"
    )]

    use proptest::prelude::*;

    use super::*;
    use crate::formula::Formula;

    fn dense_at_start(formula: &str, times: &[f64], signals: &[(&str, &[f64])]) -> f64 {
        let phi = Formula::parse(formula).unwrap();
        let map: BTreeMap<String, Vec<f64>> = signals
            .iter()
            .map(|(n, v)| ((*n).to_string(), v.to_vec()))
            .collect();
        robustness_signal(&phi, times, &map).unwrap().at(times[0])
    }

    #[test]
    fn dense_catches_a_window_edge_between_samples() {
        let r = dense_at_start(
            "always[0, 1.5](x > 0)",
            &[0.0, 1.0, 2.0],
            &[("x", &[1.0, 1.0, -3.0])],
        );
        assert_eq!(r, -1.0);
    }

    #[test]
    fn dense_agrees_with_samples_when_the_window_lands_on_them() {
        let r = dense_at_start(
            "always[0, 2](x > 0)",
            &[0.0, 1.0, 2.0],
            &[("x", &[10.0, 5.0, 1.0])],
        );
        assert_eq!(r, 1.0);
    }

    #[test]
    fn equality_predicate_finds_the_crossing() {
        let r = dense_at_start(
            "eventually[0, 2](x == 0)",
            &[0.0, 2.0],
            &[("x", &[1.0, -1.0])],
        );
        assert_eq!(r, 0.0);
    }

    #[test]
    fn nonlinear_predicate_is_rejected() {
        let phi = Formula::parse("always[0, 1](x * y > 0)").unwrap();
        let map: BTreeMap<String, Vec<f64>> =
            [("x".to_string(), vec![1.0]), ("y".to_string(), vec![1.0])]
                .into_iter()
                .collect();
        assert!(matches!(
            robustness_signal(&phi, &[0.0], &map),
            Err(Error::Unsupported { .. })
        ));
    }

    fn dense_at(formula: &str, t: f64, times: &[f64], signals: &[(&str, &[f64])]) -> f64 {
        let phi = Formula::parse(formula).unwrap();
        let map: BTreeMap<String, Vec<f64>> = signals
            .iter()
            .map(|(n, v)| ((*n).to_string(), v.to_vec()))
            .collect();
        robustness_signal(&phi, times, &map).unwrap().at(t)
    }

    #[test]
    fn dense_until_takes_the_best_future_witness() {
        let r = dense_at(
            "(x > 0) until[0, 2] (y > 0)",
            0.0,
            &[0.0, 1.0, 2.0],
            &[("x", &[10.0, 10.0, 10.0]), ("y", &[-2.0, 0.0, 2.0])],
        );
        assert_eq!(r, 2.0);
    }

    #[test]
    fn dense_until_is_capped_by_a_dip_in_the_left_operand() {
        let r = dense_at(
            "(x > 0) until[1, 2] (y > 0)",
            0.0,
            &[0.0, 1.0, 2.0],
            &[("x", &[10.0, -5.0, 10.0]), ("y", &[2.0, 2.0, 2.0])],
        );
        assert_eq!(r, -5.0);
    }

    #[test]
    fn dense_since_takes_the_best_past_witness() {
        let r = dense_at(
            "(x > 0) since[0, 2] (y > 0)",
            2.0,
            &[0.0, 1.0, 2.0],
            &[("x", &[10.0, 10.0, 10.0]), ("y", &[2.0, 0.0, -2.0])],
        );
        assert_eq!(r, 2.0);
    }

    #[test]
    fn next_is_rejected_in_dense_time() {
        let phi = Formula::parse("next(x > 0)").unwrap();
        let map: BTreeMap<String, Vec<f64>> = [("x".to_string(), vec![1.0])].into_iter().collect();
        assert!(matches!(
            robustness_signal(&phi, &[0.0], &map),
            Err(Error::Unsupported { .. })
        ));
    }

    proptest! {
        #[test]
        fn dense_matches_discrete_on_aligned_windows(
            values in prop::collection::vec(-20.0f64..20.0, 1..30),
        ) {
            let times: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();
            let map: BTreeMap<String, Vec<f64>> =
                [("x".to_string(), values.clone())].into_iter().collect();
            for formula in [
                "always[0, 3](x > 0)",
                "eventually[0, 2](x > 5)",
                "historically[0, 2](x > -5)",
            ] {
                let phi = Formula::parse(formula).unwrap();
                let dense = robustness_signal(&phi, &times, &map).unwrap();
                let discrete = super::super::discrete::robustness_trace(&phi, &times, &map).unwrap();
                for (i, &t) in times.iter().enumerate() {
                    prop_assert_eq!(dense.at(t), discrete[i], "{} at t={}", formula, t);
                }
            }
        }
    }

    #[test]
    fn dense_sees_a_bend_discrete_cannot() {
        let times = vec![0.0, 1.0, 2.0];
        let values = vec![13.139_237_358_246_93, 0.0, 10.350_972_832_119_19];
        let map: BTreeMap<String, Vec<f64>> =
            [("x".to_string(), values.clone())].into_iter().collect();
        let phi = Formula::parse("always[0, 4](eventually[0, 1](x > 0))").unwrap();

        let dense = robustness_signal(&phi, &times, &map).unwrap();
        let discrete = super::super::discrete::robustness_trace(&phi, &times, &map).unwrap();

        let bend = values[0] / ((values[2] - values[1]) + (values[0] - values[1]));
        let at_bend = values[2] * bend;
        assert!((dense.at(0.0) - at_bend).abs() < 1e-9);
        assert!((discrete[0] - values[2]).abs() < 1e-9);
        assert!(dense.at(0.0) < discrete[0]);
    }

    fn grid_strategy() -> impl Strategy<Value = (Vec<f64>, Vec<f64>, Vec<f64>)> {
        prop::collection::vec((0.4f64..5.0, -15.0f64..15.0, -15.0f64..15.0), 1..24).prop_map(
            |rows| {
                let mut t = 0.0;
                let (mut times, mut phi, mut psi) = (Vec::new(), Vec::new(), Vec::new());
                for (gap, p, s) in rows {
                    t += gap;
                    times.push(t);
                    phi.push(p);
                    psi.push(s);
                }
                (times, phi, psi)
            },
        )
    }

    /// The continuous until supremum approached by a fine grid.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation, reason = "a positive fine-step count")]
    fn fine_until(times: &[f64], phi: &[f64], psi: &[f64], t: f64, a: f64, b: f64) -> f64 {
        let end = times[times.len() - 1];
        let lo = t + a;
        if lo > end + super::EPS {
            return f64::NEG_INFINITY;
        }
        let hi = if b.is_infinite() { end } else { (t + b).min(end) };
        let steps = ((hi - lo) / 2e-3).ceil().max(1.0) as usize;
        let mut best = f64::NEG_INFINITY;
        for k in 0..=steps {
            let s = lo + (hi - lo) * (k as f64) / (steps as f64);
            let floor = if (s - t).abs() <= super::EPS {
                f64::INFINITY
            } else {
                super::phi_inf_over(times, phi, t, s)
            };
            best = best.max(super::interp(times, psi, s).min(floor));
        }
        best
    }

    proptest! {
        #[test]
        fn until_matches_the_continuous_limit(
            (times, phi, psi) in grid_strategy(),
            a in prop_oneof![Just(0.0f64), 0.3f64..5.0],
            span in 0.0f64..12.0,
            open in any::<bool>(),
        ) {
            let b = if open { f64::INFINITY } else { a + span };
            let t = times[0];
            let exact = super::until_at(t, &times, &phi, &psi, a, b);
            let reference = fine_until(&times, &phi, &psi, t, a, b);
            if exact.is_finite() && reference.is_finite() {
                prop_assert!(exact >= reference - 1e-6, "exact {} below reference {}", exact, reference);
                prop_assert!(exact - reference < 0.1, "exact {} exceeds reference {} by too much", exact, reference);
            } else {
                prop_assert_eq!(exact.is_finite(), reference.is_finite());
            }
        }
    }

    #[test]
    fn dense_until_finds_an_off_grid_edge_witness() {
        let times = [0.0, 1.0, 2.0, 3.0];
        let x: &[f64] = &[-3.0, -1.0, 3.0, -3.0];
        let map: BTreeMap<String, Vec<f64>> = [("x".to_string(), x.to_vec())].into_iter().collect();
        let ev = Formula::parse("eventually[0, 1.5](x > 0)").unwrap();
        let un = Formula::parse("(x > -1000) until[0, 1.5] (x > 0)").unwrap();
        let ev_sig = robustness_signal(&ev, &times, &map).unwrap();
        let un_sig = robustness_signal(&un, &times, &map).unwrap();
        assert!((ev_sig.at(0.0) - 1.0).abs() < 1e-9, "eventually {}", ev_sig.at(0.0));
        assert!((un_sig.at(0.0) - 1.0).abs() < 1e-9, "until {}", un_sig.at(0.0));
    }

    #[test]
    #[allow(clippy::cast_precision_loss, reason = "the tiny loop indices cast exactly")]
    fn dense_until_stays_within_time_and_memory() {
        let n = 2_000usize;
        let times: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let phi: Vec<f64> = (0..n).map(|i| ((i * 7) % 13) as f64 - 6.0).collect();
        let psi: Vec<f64> = (0..n).map(|i| ((i * 5) % 11) as f64 - 5.0).collect();
        let start = std::time::Instant::now();
        let out: Vec<f64> = (0..n)
            .map(|i| super::until_at(times[i], &times, &phi, &psi, 0.0, f64::INFINITY))
            .collect();
        assert_eq!(out.len(), n);
        assert!(start.elapsed() < std::time::Duration::from_secs(60));
    }
}