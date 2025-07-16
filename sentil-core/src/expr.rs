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

impl Program {
    pub(crate) fn compile(formula: &Formula, symbols: &[String]) -> Result<Self> {
        let mut program = Program {
            ops: Vec::new(),
            terms: Vec::new(),
            depth: 0,
        };
        program.emit_formula(formula, symbols)?;
        program.depth = program.computed_depth();
        Ok(program)
    }

    pub(crate) fn scratch(&self) -> Vec<f64> {
        vec![0.0; self.depth]
    }

    pub(crate) fn eval(&self, stack: &mut [f64], state: &[f64]) -> Result<f64> {
        let mut sp = 0usize;
        for op in &self.ops {
            match *op {
                Op::Var(i) => {
                    stack[sp] = state[i];
                    sp += 1;
                }
                Op::Const(c) => {
                    stack[sp] = c;
                    sp += 1;
                }
                Op::Add => sp = fold(stack, sp, |a, b| a + b),
                Op::Sub => sp = fold(stack, sp, |a, b| a - b),
                Op::Mul => sp = fold(stack, sp, |a, b| a * b),
                Op::Pow => sp = fold(stack, sp, f64::powf),
                // `min`/`max` functions and boolean and/or share the same fold.
                Op::FnMin | Op::And => sp = fold(stack, sp, f64::min),
                Op::FnMax | Op::Or => sp = fold(stack, sp, f64::max),
                Op::Margin(cmp) => {
                    let rhs = stack[sp - 1];
                    stack[sp - 2] = predicate_margin(cmp, stack[sp - 2], rhs);
                    sp -= 1;
                }
                Op::Div(term) => {
                    let b = stack[sp - 1];
                    if b == 0.0 {
                        return Err(Error::DivisionByZero {
                            term: self.terms[term].clone(),
                        });
                    }
                    sp -= 1;
                    stack[sp - 1] /= b;
                }
                Op::Rem(term) => {
                    let b = stack[sp - 1];
                    if b == 0.0 {
                        return Err(Error::DivisionByZero {
                            term: self.terms[term].clone(),
                        });
                    }
                    sp -= 1;
                    stack[sp - 1] %= b;
                }
                Op::Abs => stack[sp - 1] = stack[sp - 1].abs(),
                Op::Sqrt => stack[sp - 1] = stack[sp - 1].sqrt(),
                Op::Exp => stack[sp - 1] = stack[sp - 1].exp(),
                Op::Ln => stack[sp - 1] = stack[sp - 1].ln(),
                Op::Log => stack[sp - 1] = stack[sp - 1].log10(),
                Op::Sin => stack[sp - 1] = stack[sp - 1].sin(),
                Op::Cos => stack[sp - 1] = stack[sp - 1].cos(),
                Op::Tan => stack[sp - 1] = stack[sp - 1].tan(),
                Op::Floor => stack[sp - 1] = stack[sp - 1].floor(),
                Op::Ceil => stack[sp - 1] = stack[sp - 1].ceil(),
                Op::Neg => stack[sp - 1] = -stack[sp - 1],
            }
        }
        Ok(stack[0])
    }

}