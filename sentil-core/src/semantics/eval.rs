//! Evaluating arithmetic terms and atomic predicates at a single instant.

use crate::error::{Error, Result};
use crate::formula::{BinaryOp, ComparisonOp, Expr, Predicate};

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