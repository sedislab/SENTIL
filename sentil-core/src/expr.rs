//! A tiny compiler for the non-temporal core of a formula.

use crate::error::{Error, Result};
use crate::formula::{BinaryOp, ComparisonOp, Expr, Formula};
use crate::semantics::predicate_margin;

#[derive(Debug, Clone, Copy)]
enum Op {
    Var(usize),
    Const(f64),
    Add,
    Sub,
    Mul,
    Pow,
    Div(usize),
    Rem(usize),
    Abs,
    Sqrt,
    Exp,
    Ln,
    Log,
    Sin,
    Cos,
    Tan,
    Floor,
    Ceil,
    FnMin,
    FnMax,
    Margin(ComparisonOp),
    Neg,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub(crate) struct Program {
    ops: Vec<Op>,
    terms: Vec<String>,
    depth: usize,
}