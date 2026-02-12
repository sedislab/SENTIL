//! The convex fast path for the online controller, over `and`, `always[a, b]`, and
//! `not` on affine predicates of an affine model.

#![allow(
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    reason = "the encoding uses the domain's short names (matrices a/b, cost P/q, constraints G/h), and step indices stay far below 2^53 so the time cast is exact"
)]

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use super::model::{AffineForm, Bounds};
use super::qp::solve_qp;
use crate::error::Result;
use crate::formula::{ComparisonOp, Expr, Formula, Predicate};

const TIME_EPS: f64 = 1e-9;

const MIN_MARGIN_WEIGHT: f64 = 100.0;

const QP_ITERS: usize = 4000;

/// A linear robustness term `coeffs . u + constant` over the packed input.
struct Term {
    coeffs: Vec<f64>,
    constant: f64,
}

/// An affine form over the packed input: `coeffs . u + constant`.
#[derive(Clone)]
struct Affine {
    coeffs: Vec<f64>,
    constant: f64,
}

impl Affine {
    fn constant(n: usize, c: f64) -> Self {
        Self {
            coeffs: vec![0.0; n],
            constant: c,
        }
    }

    fn add(&self, other: &Self) -> Self {
        Self {
            coeffs: self.coeffs.iter().zip(&other.coeffs).map(|(a, b)| a + b).collect(),
            constant: self.constant + other.constant,
        }
    }

    fn sub(&self, other: &Self) -> Self {
        Self {
            coeffs: self.coeffs.iter().zip(&other.coeffs).map(|(a, b)| a - b).collect(),
            constant: self.constant - other.constant,
        }
    }

    fn scaled(&self, factor: f64) -> Self {
        Self {
            coeffs: self.coeffs.iter().map(|c| c * factor).collect(),
            constant: self.constant * factor,
        }
    }

    /// The product, defined only when one side is a bare constant.
    fn mul(&self, other: &Self) -> Option<Self> {
        let constant_only = |a: &Self| a.coeffs.iter().all(|c| *c == 0.0);
        if constant_only(self) {
            Some(other.scaled(self.constant))
        } else if constant_only(other) {
            Some(self.scaled(other.constant))
        } else {
            None
        }
    }

    fn into_term(self) -> Term {
        Term {
            coeffs: self.coeffs,
            constant: self.constant,
        }
    }
}

/// The affine rollout `x_t = c_t + D_t u` from the live `state`.
struct Rollout {
    /// `state[t][s]` is the affine form of state component `s` at step `t`.
    state: Vec<Vec<Affine>>,
    dt: f64,
    horizon: usize,
    variables: Vec<String>,
}

impl Rollout {
    fn new(affine: &AffineForm, initial: &[f64]) -> Self {
        let n = affine.x0.len();
        let width = affine.b.first().map_or(0, Vec::len);
        let input_dim = width * affine.horizon;
        let mut state: Vec<Vec<Affine>> = Vec::with_capacity(affine.horizon + 1);
        state.push(
            (0..n)
                .map(|s| Affine::constant(input_dim, initial.get(s).copied().unwrap_or(0.0)))
                .collect(),
        );
        for t in 0..affine.horizon {
            let prev = &state[t];
            let next: Vec<Affine> = (0..n)
                .map(|s| {
                    let mut acc = Affine::constant(input_dim, 0.0);
                    for (k, &a) in affine.a[s].iter().enumerate() {
                        acc = acc.add(&prev[k].scaled(a));
                    }
                    for (k, &b) in affine.b[s].iter().enumerate() {
                        acc.coeffs[t * width + k] += b;
                    }
                    acc
                })
                .collect();
            state.push(next);
        }
        Self {
            state,
            dt: affine.dt,
            horizon: affine.horizon,
            variables: affine.variables.clone(),
        }
    }

    /// The affine form of `expr` over the state at step `t`, or `None` when the
    /// term is not affine in the state.
    fn term(&self, expr: &Expr, t: usize) -> Option<Affine> {
        let n = self.state[t].len();
        let input_dim = self.state[t].first().map_or(0, |a| a.coeffs.len());
        match expr {
            Expr::Literal(v) => Some(Affine::constant(input_dim, *v)),
            Expr::Variable(name) => {
                let s = self.variables.iter().position(|v| v == name)?;
                debug_assert!(s < n);
                Some(self.state[t][s].clone())
            }
            Expr::Binary(op, l, r) => {
                use crate::formula::BinaryOp;
                let a = self.term(l, t)?;
                let b = self.term(r, t)?;
                match op {
                    BinaryOp::Add => Some(a.add(&b)),
                    BinaryOp::Sub => Some(a.sub(&b)),
                    BinaryOp::Mul => a.mul(&b),
                    BinaryOp::Div => {
                        let divisor = (b.coeffs.iter().all(|c| *c == 0.0)).then_some(b.constant)?;
                        (divisor != 0.0).then(|| a.scaled(1.0 / divisor))
                    }
                    BinaryOp::Mod | BinaryOp::Pow => None,
                }
            }
            Expr::Call(..) => None,
        }
    }

    /// The step indices whose times fall in the window `[t*dt + a, t*dt + b]`.
    fn window(&self, t: usize, a: f64, b: f64) -> impl Iterator<Item = usize> {
        let lo = t as f64 * self.dt + a;
        let hi = t as f64 * self.dt + b;
        let dt = self.dt;
        (0..=self.horizon).filter(move |&j| {
            let tj = j as f64 * dt;
            tj >= lo - TIME_EPS && (hi.is_infinite() || tj <= hi + TIME_EPS)
        })
    }
}

/// The linear robustness terms of `spec` at step `t` whose minimum is its
/// robustness, or `None` when `spec` is outside the convex fragment.
fn terms(rollout: &Rollout, spec: &Formula, t: usize) -> Option<Vec<Term>> {
    match spec {
        Formula::Predicate(p) => predicate(rollout, p, t),
        Formula::Not(inner) => match inner.as_ref() {
            Formula::Predicate(p) if !is_equality(p) => {
                let mut out = predicate(rollout, p, t)?;
                let term = out.pop()?;
                out.is_empty().then(|| vec![Term {
                    coeffs: term.coeffs.iter().map(|c| -c).collect(),
                    constant: -term.constant,
                }])
            }
            _ => None,
        },
        Formula::And(l, r) => {
            let mut out = terms(rollout, l, t)?;
            out.extend(terms(rollout, r, t)?);
            Some(out)
        }
        Formula::Always(iv, inner) => {
            let mut out = Vec::new();
            for j in rollout.window(t, iv.lower(), iv.upper_or_infinity()) {
                out.extend(terms(rollout, inner, j)?);
            }
            Some(out)
        }
        _ => None,
    }
}

/// The robustness terms of `f(x_t) ~ c`, or `None` when the term is not affine.
fn predicate(rollout: &Rollout, p: &Predicate, t: usize) -> Option<Vec<Term>> {
    let lhs = rollout.term(&p.lhs, t)?;
    let rhs = rollout.term(&p.rhs, t)?;
    let margin = lhs.sub(&rhs);
    match p.op {
        ComparisonOp::Greater | ComparisonOp::GreaterEqual => Some(vec![margin.into_term()]),
        ComparisonOp::Less | ComparisonOp::LessEqual => Some(vec![margin.scaled(-1.0).into_term()]),
        ComparisonOp::Equal => Some(vec![margin.scaled(-1.0).into_term(), margin.into_term()]),
        ComparisonOp::NotEqual => None,
    }
}

fn is_equality(p: &Predicate) -> bool {
    matches!(p.op, ComparisonOp::Equal | ComparisonOp::NotEqual)
}

/// Solves the receding-horizon step as a convex program, returning the clamped
/// input to apply or `None` to defer to the gradient path.
pub(super) fn step(
    affine: &AffineForm,
    spec: &Formula,
    state: &[f64],
    bounds: &Bounds,
) -> Option<Vec<f64>> {
    let width = affine.b.first().map_or(0, Vec::len);
    let n = width * affine.horizon;
    if bounds.dimension() != n {
        return None;
    }
    let rollout = Rollout::new(affine, state);
    let constraints = terms(&rollout, spec, 0)?;

    let mut input = maximize_margin(&constraints, bounds, n).ok()?;
    bounds.clamp(&mut input);
    Some(input)
}

/// Maximizes the worst-case margin `rho` subject to every term clearing `rho` and
/// the box, over the stacked variable `[u, rho]`.
fn maximize_margin(constraints: &[Term], bounds: &Bounds, n: usize) -> Result<Vec<f64>> {
    let dim = n + 1;
    let p: Vec<Vec<f64>> = (0..dim)
        .map(|i| (0..dim).map(|j| f64::from(u8::from(i == j))).collect())
        .collect();
    let mut q = vec![0.0; dim];
    q[n] = -margin_weight(constraints, bounds);

    let (box_g, box_h) = bounds.constraint_rows();
    let mut g: Vec<Vec<f64>> = box_g
        .into_iter()
        .map(|mut row| {
            row.push(0.0);
            row
        })
        .collect();
    let mut h = box_h;
    for c in constraints {
        let mut row: Vec<f64> = c.coeffs.iter().map(|x| -x).collect();
        row.push(1.0);
        g.push(row);
        h.push(c.constant);
    }
    let z = solve_qp(&p, &q, &g, &h, QP_ITERS)?;
    Ok(z[..n].to_vec())
}

/// A reward weight that strictly exceeds the largest margin the box admits.
fn margin_weight(constraints: &[Term], bounds: &Bounds) -> f64 {
    let reach = |term: &Term| -> f64 {
        let span: f64 = term
            .coeffs
            .iter()
            .zip(bounds.lower().iter().zip(bounds.upper()))
            .map(|(c, (&lo, &hi))| c.abs() * lo.abs().max(hi.abs()))
            .sum();
        term.constant.abs() + span
    };
    let bound = constraints.iter().map(reach).fold(0.0_f64, f64::max);
    if bound.is_finite() {
        (2.0 * bound).max(MIN_MARGIN_WEIGHT)
    } else {
        MIN_MARGIN_WEIGHT
    }
}