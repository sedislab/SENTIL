//! Reverse-mode differentiation of the smooth robustness.
//!
//! One backward pass over the formula tree gives the exact gradient of the smooth
//! robustness with respect to every trace value, where finite differences would
//! need a forward evaluation per value and carry a step-size error. This is the
//! differentiable signal-temporal-logic primitive for learning and gradient-based
//! trajectory optimization: a differentiable model that supplies its own Jacobian
//! chains this to reach the gradient with respect to its own inputs.

use std::collections::BTreeMap;

use super::smooth::{soft_eval, SmoothConfig, SoftKind};
use crate::error::{Error, Result};
use crate::formula::{BinaryOp, ComparisonOp, Expr, Formula, Predicate};
use crate::semantics::{eval_expr, WINDOW_EPSILON};
use crate::signal::Trace;

impl Formula {
    /// The smooth robustness and its exact gradient with respect to every trace
    /// value, keyed by signal name.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty trace, a non-log-sum-exp smoothing, an `until`,
    /// `since`, or probabilistic operator, or a missing signal.
    pub fn smooth_value_and_gradient(
        &self,
        trace: &Trace,
        config: SmoothConfig,
    ) -> Result<(f64, BTreeMap<String, Vec<f64>>)> {
        if trace.is_empty() {
            return Err(Error::EmptyTrace);
        }
        if config.kind() != SoftKind::LogSumExp {
            return Err(Error::InvalidConfig {
                context: "smooth gradient",
                message: "the reverse-mode gradient supports only the log-sum-exp smoothing"
                    .to_owned(),
            });
        }
        let times = trace.times();
        let signals = trace.signals();
        let value = soft_eval(self, times, signals, config)?[0];
        let mut out_adj = vec![0.0; times.len()];
        out_adj[0] = 1.0;
        let mut grad: BTreeMap<String, Vec<f64>> = signals
            .keys()
            .map(|name| (name.clone(), vec![0.0; times.len()]))
            .collect();
        backward(self, times, signals, config, &out_adj, &mut grad)?;
        Ok((value, grad))
    }
}

fn backward(
    formula: &Formula,
    times: &[f64],
    signals: &BTreeMap<String, Vec<f64>>,
    config: SmoothConfig,
    out_adj: &[f64],
    grad: &mut BTreeMap<String, Vec<f64>>,
) -> Result<()> {
    match formula {
        Formula::Predicate(p) => {
            for (t, &adj) in out_adj.iter().enumerate() {
                if adj == 0.0 {
                    continue;
                }
                let lookup = |name: &str| signals.get(name).and_then(|col| col.get(t)).copied();
                for (name, col) in grad.iter_mut() {
                    col[t] += adj * margin_grad(p, name, &lookup)?;
                }
            }
            Ok(())
        }
        Formula::Not(f) => {
            let inner: Vec<f64> = out_adj.iter().map(|&a| -a).collect();
            backward(f, times, signals, config, &inner, grad)
        }
        Formula::And(l, r) => combine_backward(l, r, times, signals, config, out_adj, grad, false, false),
        Formula::Or(l, r) => combine_backward(l, r, times, signals, config, out_adj, grad, true, false),
        Formula::Implies(l, r) => combine_backward(l, r, times, signals, config, out_adj, grad, true, true),
        Formula::Always(iv, f) => {
            window_backward(f, times, signals, config, out_adj, grad, iv.lower, iv.upper_or_infinity(), false)
        }
        Formula::Eventually(iv, f) => {
            window_backward(f, times, signals, config, out_adj, grad, iv.lower, iv.upper_or_infinity(), true)
        }
        Formula::Historically(iv, f) => {
            window_backward(f, times, signals, config, out_adj, grad, -iv.upper_or_infinity(), -iv.lower, false)
        }
        Formula::Once(iv, f) => {
            window_backward(f, times, signals, config, out_adj, grad, -iv.upper_or_infinity(), -iv.lower, true)
        }
        Formula::Next(f) => {
            let mut inner = vec![0.0; times.len()];
            for t in 0..times.len().saturating_sub(1) {
                inner[t + 1] += out_adj[t];
            }
            backward(f, times, signals, config, &inner, grad)
        }
        Formula::Until(..) | Formula::Since(..) => Err(Error::Unsupported {
            feature: "the reverse-mode gradient of until or since; use the finite-difference gradient",
        }),
        Formula::Probabilistic(..) => Err(Error::ProbabilisticOperator),
    }
}

#[allow(clippy::too_many_arguments, reason = "a backward step needs its full context")]
fn combine_backward(
    l: &Formula,
    r: &Formula,
    times: &[f64],
    signals: &BTreeMap<String, Vec<f64>>,
    config: SmoothConfig,
    out_adj: &[f64],
    grad: &mut BTreeMap<String, Vec<f64>>,
    is_max: bool,
    negate_left: bool,
) -> Result<()> {
    let beta = config.temperature();
    let l_sig = soft_eval(l, times, signals, config)?;
    let r_sig = soft_eval(r, times, signals, config)?;
    let mut l_adj = vec![0.0; times.len()];
    let mut r_adj = vec![0.0; times.len()];
    for t in 0..times.len() {
        if out_adj[t] == 0.0 {
            continue;
        }
        let left = if negate_left { -l_sig[t] } else { l_sig[t] };
        let weights = reduce_weights(&[left, r_sig[t]], beta, is_max);
        let left_contrib = out_adj[t] * weights[0];
        l_adj[t] += if negate_left { -left_contrib } else { left_contrib };
        r_adj[t] += out_adj[t] * weights[1];
    }
    backward(l, times, signals, config, &l_adj, grad)?;
    backward(r, times, signals, config, &r_adj, grad)
}

#[allow(clippy::too_many_arguments, reason = "a backward step needs its full context")]
fn window_backward(
    f: &Formula,
    times: &[f64],
    signals: &BTreeMap<String, Vec<f64>>,
    config: SmoothConfig,
    out_adj: &[f64],
    grad: &mut BTreeMap<String, Vec<f64>>,
    off_a: f64,
    off_b: f64,
    is_max: bool,
) -> Result<()> {
    let beta = config.temperature();
    let child = soft_eval(f, times, signals, config)?;
    let mut child_adj = vec![0.0; times.len()];
    for t in 0..times.len() {
        if out_adj[t] == 0.0 {
            continue;
        }
        let lo = times[t] + off_a;
        let hi = times[t] + off_b;
        let start = times.partition_point(|&tj| tj < lo - WINDOW_EPSILON);
        let end = times.partition_point(|&tj| tj <= hi + WINDOW_EPSILON);
        if start >= end {
            continue;
        }
        let members: Vec<f64> = (start..end).map(|j| child[j]).collect();
        let weights = reduce_weights(&members, beta, is_max);
        for (k, j) in (start..end).enumerate() {
            child_adj[j] += out_adj[t] * weights[k];
        }
    }
    backward(f, times, signals, config, &child_adj, grad)
}

fn reduce_weights(values: &[f64], beta: f64, is_max: bool) -> Vec<f64> {
    let sign = if is_max { 1.0 } else { -1.0 };
    let scaled: Vec<f64> = values.iter().map(|&x| beta * sign * x).collect();
    let shift = scaled.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !shift.is_finite() {
        return vec![0.0; values.len()];
    }
    let exps: Vec<f64> = scaled.iter().map(|&s| (s - shift).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}

fn margin_grad<F>(predicate: &Predicate, var: &str, lookup: &F) -> Result<f64>
where
    F: Fn(&str) -> Option<f64>,
{
    let dl = expr_grad(&predicate.lhs, var, lookup)?;
    let dr = expr_grad(&predicate.rhs, var, lookup)?;
    match predicate.op {
        ComparisonOp::Greater | ComparisonOp::GreaterEqual => Ok(dl - dr),
        ComparisonOp::Less | ComparisonOp::LessEqual => Ok(dr - dl),
        ComparisonOp::Equal => {
            let diff = eval_expr(&predicate.lhs, lookup)? - eval_expr(&predicate.rhs, lookup)?;
            Ok(-diff.signum() * (dl - dr))
        }
        ComparisonOp::NotEqual => {
            let diff = eval_expr(&predicate.lhs, lookup)? - eval_expr(&predicate.rhs, lookup)?;
            Ok(diff.signum() * (dl - dr))
        }
    }
}

fn expr_grad<F>(expr: &Expr, var: &str, lookup: &F) -> Result<f64>
where
    F: Fn(&str) -> Option<f64>,
{
    match expr {
        Expr::Literal(_) => Ok(0.0),
        Expr::Variable(name) => Ok(f64::from(u8::from(name == var))),
        Expr::Binary(op, l, r) => {
            let a = eval_expr(l, lookup)?;
            let b = eval_expr(r, lookup)?;
            let da = expr_grad(l, var, lookup)?;
            let db = expr_grad(r, var, lookup)?;
            match op {
                BinaryOp::Add => Ok(da + db),
                BinaryOp::Sub => Ok(da - db),
                BinaryOp::Mul => Ok(da * b + a * db),
                BinaryOp::Div => {
                    if b == 0.0 {
                        return Err(Error::DivisionByZero {
                            term: format!("{l} / {r}"),
                        });
                    }
                    Ok((da * b - a * db) / (b * b))
                }
                BinaryOp::Mod => {
                    if b == 0.0 {
                        return Err(Error::DivisionByZero {
                            term: format!("{l} % {r}"),
                        });
                    }
                    Ok(da - (a / b).floor() * db)
                }
                BinaryOp::Pow => {
                    if db == 0.0 {
                        Ok(b * a.powf(b - 1.0) * da)
                    } else {
                        Ok(a.powf(b) * (db * a.ln() + b * da / a))
                    }
                }
            }
        }
        Expr::Call(name, args) => call_grad(name, args, var, lookup),
    }
}

fn call_grad<F>(name: &str, args: &[Expr], var: &str, lookup: &F) -> Result<f64>
where
    F: Fn(&str) -> Option<f64>,
{
    let unary = |outer: fn(f64) -> f64| -> Result<f64> {
        if let [arg] = args {
            Ok(outer(eval_expr(arg, lookup)?) * expr_grad(arg, var, lookup)?)
        } else {
            Err(Error::UnknownFunction {
                name: name.to_owned(),
                arity: args.len(),
            })
        }
    };
    match name {
        "abs" => unary(f64::signum),
        "sqrt" => unary(|a| 0.5 / a.sqrt()),
        "exp" => unary(f64::exp),
        "ln" => unary(|a| 1.0 / a),
        "log" => unary(|a| 1.0 / (a * core::f64::consts::LN_10)),
        "sin" => unary(f64::cos),
        "cos" => unary(|a| -a.sin()),
        "tan" => unary(|a| 1.0 / (a.cos() * a.cos())),
        "floor" | "ceil" => unary(|_| 0.0),
        "min" | "max" => {
            if let [l, r] = args {
                let a = eval_expr(l, lookup)?;
                let b = eval_expr(r, lookup)?;
                let pick_left = if name == "min" { a <= b } else { a >= b };
                if pick_left {
                    expr_grad(l, var, lookup)
                } else {
                    expr_grad(r, var, lookup)
                }
            } else {
                Err(Error::UnknownFunction {
                    name: name.to_owned(),
                    arity: args.len(),
                })
            }
        }
        _ => Err(Error::UnknownFunction {
            name: name.to_owned(),
            arity: args.len(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(signals: &[(&str, &[f64])]) -> Trace {
        let n = signals[0].1.len();
        let mut tr = Trace::indexed(n);
        for (name, values) in signals {
            tr.add_signal(name, values.to_vec()).unwrap();
        }
        tr
    }

    fn finite_difference(phi: &Formula, signals: &[(&str, Vec<f64>)], v: usize, i: usize) -> f64 {
        let cfg = SmoothConfig::new(8.0).unwrap();
        let eps = 1e-5;
        let bump = |delta: f64| {
            let mut s: Vec<(&str, Vec<f64>)> = signals.to_vec();
            s[v].1[i] += delta;
            let refs: Vec<(&str, &[f64])> = s.iter().map(|(n, vs)| (*n, vs.as_slice())).collect();
            phi.smooth_robustness(&trace(&refs), cfg).unwrap()
        };
        (bump(eps) - bump(-eps)) / (2.0 * eps)
    }

    fn check(formula: &str, signals: &[(&str, Vec<f64>)]) {
        let phi = Formula::parse(formula).unwrap();
        let cfg = SmoothConfig::new(8.0).unwrap();
        let refs: Vec<(&str, &[f64])> = signals.iter().map(|(n, v)| (*n, v.as_slice())).collect();
        let (value, grad) = phi.smooth_value_and_gradient(&trace(&refs), cfg).unwrap();
        assert!((value - phi.smooth_robustness(&trace(&refs), cfg).unwrap()).abs() < 1e-12);
        for (v, (name, _)) in signals.iter().enumerate() {
            for (i, &ad) in grad[*name].iter().enumerate() {
                let fd = finite_difference(&phi, signals, v, i);
                assert!(
                    (ad - fd).abs() < 1e-4,
                    "{formula} d/d{name}[{i}]: autodiff {ad} vs finite diff {fd}"
                );
            }
        }
    }

    #[test]
    fn the_gradient_selects_the_same_window_as_the_value_on_a_drifted_grid() {
        let mut t = 0.0;
        let times: Vec<f64> = (0..6)
            .map(|_| {
                let now = t;
                t += 0.1;
                now
            })
            .collect();
        let values = vec![1.0, 0.4, 2.0, 0.8, 1.6, 0.2];
        let mut tr = Trace::new(times).unwrap();
        tr.add_signal("x", values.clone()).unwrap();
        let phi = Formula::parse("always[0, 0.3](x > 0)").unwrap();
        let cfg = SmoothConfig::new(8.0).unwrap();
        let (_, grad) = phi.smooth_value_and_gradient(&tr, cfg).unwrap();
        let eps = 1e-5;
        for i in 0..values.len() {
            let bump = |delta: f64| {
                let mut v = values.clone();
                v[i] += delta;
                let mut b = Trace::new(tr.times().to_vec()).unwrap();
                b.add_signal("x", v).unwrap();
                phi.smooth_robustness(&b, cfg).unwrap()
            };
            let fd = (bump(eps) - bump(-eps)) / (2.0 * eps);
            let ad = grad["x"][i];
            assert!(
                (ad - fd).abs() < 1e-4,
                "d/dx[{i}]: autodiff {ad} vs finite diff {fd}"
            );
        }
    }

    #[test]
    fn gradient_matches_finite_differences() {
        check("x > 0", &[("x", vec![1.5, -0.5, 2.0])]);
        check("x * 2 - y < 3", &[("x", vec![1.0, 2.0]), ("y", vec![0.5, -1.0])]);
        check("abs(x - 1) < 2", &[("x", vec![0.3, 2.4, -1.0])]);
        check("(x > 0) and (y > 0)", &[("x", vec![1.0, -2.0]), ("y", vec![0.5, 3.0])]);
        check("(x > 0) or (y > 0)", &[("x", vec![-1.0, 2.0]), ("y", vec![0.5, -3.0])]);
        check("(x > 0) implies (y > 1)", &[("x", vec![1.0, -1.0]), ("y", vec![2.0, 0.5])]);
        check("not(x > 0)", &[("x", vec![1.0, -0.5, 0.7])]);
        check("always[0, 2](x > 0)", &[("x", vec![1.0, 0.4, 2.0, 0.8])]);
        check("eventually[0, 2](x > 1)", &[("x", vec![0.2, 1.5, 0.6, 2.0])]);
        check("historically[0, 1](x > 0)", &[("x", vec![1.0, 0.3, 2.0])]);
        check("once[0, 1](x > 1)", &[("x", vec![0.2, 1.4, 0.6])]);
        check("next(x > 0)", &[("x", vec![1.0, 0.5, 2.0])]);
        check("always[0, 2](eventually[0, 1](x > 0))", &[("x", vec![-0.5, 1.0, 0.3, 2.0])]);
    }

    #[test]
    fn unsupported_smoothings_and_operators_are_rejected() {
        let phi = Formula::parse("(x > 0) until[0, 2] (y > 0)").unwrap();
        let tr = trace(&[("x", &[1.0, 2.0]), ("y", &[0.5, 1.0])]);
        assert!(matches!(
            phi.smooth_value_and_gradient(&tr, SmoothConfig::new(8.0).unwrap()),
            Err(Error::Unsupported { .. })
        ));

        let agm = SmoothConfig::new(8.0).unwrap().with_kind(SoftKind::ArithmeticGeometricMean);
        let phi = Formula::parse("x > 0").unwrap();
        let tr = trace(&[("x", &[1.0])]);
        assert!(matches!(
            phi.smooth_value_and_gradient(&tr, agm),
            Err(Error::InvalidConfig { .. })
        ));
    }
}