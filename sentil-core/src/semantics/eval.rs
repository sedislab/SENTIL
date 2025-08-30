//! Evaluating arithmetic terms and atomic predicates at a single instant.

use crate::error::{Error, Result};
use crate::formula::{BinaryOp, ComparisonOp, Expr, Predicate};
#[cfg(not(feature = "std"))]
use crate::prelude::*;

/// The robustness margin of an atomic comparison `lhs op rhs`.
pub(crate) fn predicate_margin(op: ComparisonOp, lhs: f64, rhs: f64) -> f64 {
    match op {
        ComparisonOp::Less | ComparisonOp::LessEqual => rhs - lhs,
        ComparisonOp::Greater | ComparisonOp::GreaterEqual => lhs - rhs,
        ComparisonOp::Equal => -(lhs - rhs).abs(),
        ComparisonOp::NotEqual => (lhs - rhs).abs(),
    }
}

pub(crate) fn eval_predicate<F>(predicate: &Predicate, lookup: &F) -> Result<f64>
where
    F: Fn(&str) -> Option<f64>,
{
    let lhs = eval_expr(&predicate.lhs, lookup)?;
    let rhs = eval_expr(&predicate.rhs, lookup)?;
    Ok(predicate_margin(predicate.op, lhs, rhs))
}

pub(crate) fn eval_expr<F>(expr: &Expr, lookup: &F) -> Result<f64>
where
    F: Fn(&str) -> Option<f64>,
{
    match expr {
        Expr::Literal(v) => Ok(*v),
        Expr::Variable(name) => {
            lookup(name).ok_or_else(|| Error::UnknownVariable { name: name.clone() })
        }
        Expr::Binary(op, lhs, rhs) => {
            let a = eval_expr(lhs, lookup)?;
            let b = eval_expr(rhs, lookup)?;
            match op {
                BinaryOp::Add => Ok(a + b),
                BinaryOp::Sub => Ok(a - b),
                BinaryOp::Mul => Ok(a * b),
                BinaryOp::Pow => Ok(a.powf(b)),
                BinaryOp::Div => {
                    if b == 0.0 {
                        Err(Error::DivisionByZero {
                            term: format!("{lhs} / {rhs}"),
                        })
                    } else {
                        Ok(a / b)
                    }
                }
                BinaryOp::Mod => {
                    if b == 0.0 {
                        Err(Error::DivisionByZero {
                            term: format!("{lhs} % {rhs}"),
                        })
                    } else {
                        Ok(a % b)
                    }
                }
            }
        }
        Expr::Call(name, args) => eval_call(name, args, lookup),
    }
}

fn eval_call<F>(name: &str, args: &[Expr], lookup: &F) -> Result<f64>
where
    F: Fn(&str) -> Option<f64>,
{
    let unary = |f: fn(f64) -> f64| -> Result<f64> {
        if let [arg] = args {
            Ok(f(eval_expr(arg, lookup)?))
        } else {
            Err(Error::ArityMismatch {
                name: name.to_owned(),
                expected: 1,
                found: args.len(),
            })
        }
    };
    match name {
        "abs" => unary(f64::abs),
        "sqrt" => unary(f64::sqrt),
        "exp" => unary(f64::exp),
        "ln" => unary(f64::ln),
        "log" => unary(f64::log10),
        "sin" => unary(f64::sin),
        "cos" => unary(f64::cos),
        "tan" => unary(f64::tan),
        "floor" => unary(f64::floor),
        "ceil" => unary(f64::ceil),
        "min" | "max" => {
            if let [lhs, rhs] = args {
                let a = eval_expr(lhs, lookup)?;
                let b = eval_expr(rhs, lookup)?;
                Ok(if name == "min" { a.min(b) } else { a.max(b) })
            } else {
                Err(Error::ArityMismatch {
                    name: name.to_owned(),
                    expected: 2,
                    found: args.len(),
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
    #![allow(
        clippy::float_cmp,
        reason = "these arithmetic results are exact integer-valued f64 values"
    )]

    use super::*;
    use crate::formula::Formula;

    fn margin(formula: &str, values: &[(&str, f64)]) -> f64 {
        let f = Formula::parse(formula).unwrap();
        let Formula::Predicate(p) = f else {
            panic!("expected a predicate")
        };
        let lookup = |name: &str| values.iter().find(|(n, _)| *n == name).map(|(_, v)| *v);
        eval_predicate(&p, &lookup).unwrap()
    }

    #[test]
    fn predicate_margins_over_a_binding() {
        assert_eq!(margin("x > 5", &[("x", 8.0)]), 3.0);
        assert_eq!(margin("x < 5", &[("x", 8.0)]), -3.0);
        assert_eq!(margin("x + y < 10", &[("x", 3.0), ("y", 2.0)]), 5.0);
    }

    #[test]
    fn arithmetic_obeys_precedence_and_functions() {
        assert_eq!(margin("x + y * 2 > 0", &[("x", 1.0), ("y", 3.0)]), 7.0);
        assert_eq!(margin("abs(x - 10) < 1", &[("x", 7.0)]), -2.0);
        assert_eq!(margin("2 ^ 3 ^ 2 > 0", &[]), 512.0);
        assert_eq!(margin("max(x, y) > 0", &[("x", 4.0), ("y", 9.0)]), 9.0);
    }

    #[test]
    fn unknown_variable_is_an_error() {
        let f = Formula::parse("x > 0").unwrap();
        let Formula::Predicate(p) = f else {
            unreachable!()
        };
        let err = eval_predicate(&p, &|_: &str| None).unwrap_err();
        assert!(matches!(err, Error::UnknownVariable { .. }));
    }

    #[test]
    fn division_by_zero_names_the_term() {
        let f = Formula::parse("x / y > 0").unwrap();
        let Formula::Predicate(p) = f else {
            unreachable!()
        };
        let lookup = |name: &str| Some(if name == "x" { 1.0 } else { 0.0 });
        let err = eval_predicate(&p, &lookup).unwrap_err();
        assert!(matches!(err, Error::DivisionByZero { .. }));
    }

    #[test]
    fn wrong_arity_is_reported_separately_from_an_unknown_name() {
        let f = Formula::parse("sin(x, y) > 0").unwrap();
        let Formula::Predicate(p) = f else {
            unreachable!()
        };
        let err = eval_predicate(&p, &|_: &str| Some(1.0)).unwrap_err();
        assert!(matches!(
            err,
            Error::ArityMismatch {
                expected: 1,
                found: 2,
                ..
            }
        ));
        let g = Formula::parse("nope(x) > 0").unwrap();
        let Formula::Predicate(p) = g else {
            unreachable!()
        };
        let err = eval_predicate(&p, &|_: &str| Some(1.0)).unwrap_err();
        assert!(matches!(err, Error::UnknownFunction { .. }));
    }
}