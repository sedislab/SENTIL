//! A tiny compiler for the non-temporal core of a formula.

use crate::error::{Error, Result};
use crate::formula::{BinaryOp, ComparisonOp, Expr, Formula};
#[cfg(not(feature = "std"))]
use crate::prelude::*;
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

    fn emit_formula(&mut self, formula: &Formula, symbols: &[String]) -> Result<()> {
        match formula {
            Formula::Predicate(p) => {
                self.emit_expr(&p.lhs, symbols)?;
                self.emit_expr(&p.rhs, symbols)?;
                self.ops.push(Op::Margin(p.op));
                Ok(())
            }
            Formula::Not(f) => {
                self.emit_formula(f, symbols)?;
                self.ops.push(Op::Neg);
                Ok(())
            }
            Formula::And(l, r) => self.emit_binary_formula(l, r, Op::And, symbols),
            Formula::Or(l, r) => self.emit_binary_formula(l, r, Op::Or, symbols),
            Formula::Implies(l, r) => {
                self.emit_formula(l, symbols)?;
                self.ops.push(Op::Neg);
                self.emit_formula(r, symbols)?;
                self.ops.push(Op::Or);
                Ok(())
            }
            _ => Err(Error::Unsupported {
                feature: "a temporal or probabilistic operator inside an atomic subformula",
            }),
        }
    }

    fn emit_binary_formula(
        &mut self,
        left: &Formula,
        right: &Formula,
        op: Op,
        symbols: &[String],
    ) -> Result<()> {
        self.emit_formula(left, symbols)?;
        self.emit_formula(right, symbols)?;
        self.ops.push(op);
        Ok(())
    }

    fn emit_expr(&mut self, expr: &Expr, symbols: &[String]) -> Result<()> {
        match expr {
            Expr::Literal(v) => self.ops.push(Op::Const(*v)),
            Expr::Variable(name) => {
                let slot = symbols
                    .iter()
                    .position(|n| n == name)
                    .ok_or_else(|| Error::UnknownVariable { name: name.clone() })?;
                self.ops.push(Op::Var(slot));
            }
            Expr::Binary(op, lhs, rhs) => {
                self.emit_expr(lhs, symbols)?;
                self.emit_expr(rhs, symbols)?;
                let op = match op {
                    BinaryOp::Add => Op::Add,
                    BinaryOp::Sub => Op::Sub,
                    BinaryOp::Mul => Op::Mul,
                    BinaryOp::Pow => Op::Pow,
                    BinaryOp::Div => Op::Div(self.push_term(format!("{lhs} / {rhs}"))),
                    BinaryOp::Mod => Op::Rem(self.push_term(format!("{lhs} % {rhs}"))),
                };
                self.ops.push(op);
            }
            Expr::Call(name, args) => self.emit_call(name, args, symbols)?,
        }
        Ok(())
    }

    fn emit_call(&mut self, name: &str, args: &[Expr], symbols: &[String]) -> Result<()> {
        let unary = |this: &mut Self, op: Op| -> Result<()> {
            let [arg] = args else {
                return Err(Error::UnknownFunction {
                    name: name.to_owned(),
                    arity: args.len(),
                });
            };
            this.emit_expr(arg, symbols)?;
            this.ops.push(op);
            Ok(())
        };
        match name {
            "abs" => unary(self, Op::Abs),
            "sqrt" => unary(self, Op::Sqrt),
            "exp" => unary(self, Op::Exp),
            "ln" => unary(self, Op::Ln),
            "log" => unary(self, Op::Log),
            "sin" => unary(self, Op::Sin),
            "cos" => unary(self, Op::Cos),
            "tan" => unary(self, Op::Tan),
            "floor" => unary(self, Op::Floor),
            "ceil" => unary(self, Op::Ceil),
            "min" | "max" => {
                let [lhs, rhs] = args else {
                    return Err(Error::UnknownFunction {
                        name: name.to_owned(),
                        arity: args.len(),
                    });
                };
                self.emit_expr(lhs, symbols)?;
                self.emit_expr(rhs, symbols)?;
                self.ops
                    .push(if name == "min" { Op::FnMin } else { Op::FnMax });
                Ok(())
            }
            _ => Err(Error::UnknownFunction {
                name: name.to_owned(),
                arity: args.len(),
            }),
        }
    }

    fn push_term(&mut self, term: String) -> usize {
        let index = self.terms.len();
        self.terms.push(term);
        index
    }

    fn computed_depth(&self) -> usize {
        let mut height: usize = 0;
        let mut max = 0usize;
        for op in &self.ops {
            height = match op {
                Op::Var(_) | Op::Const(_) => height + 1,
                Op::Add
                | Op::Sub
                | Op::Mul
                | Op::Pow
                | Op::Div(_)
                | Op::Rem(_)
                | Op::FnMin
                | Op::FnMax
                | Op::And
                | Op::Or
                | Op::Margin(_) => height - 1,
                _ => height,
            };
            max = max.max(height);
        }
        max
    }
}

#[inline]
fn fold(stack: &mut [f64], sp: usize, op: fn(f64, f64) -> f64) -> usize {
    let b = stack[sp - 1];
    stack[sp - 2] = op(stack[sp - 2], b);
    sp - 1
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "compiled programs return exact integer-valued f64 here"
    )]

    use super::*;

    fn run(formula: &str) -> Program {
        let f = Formula::parse(formula).unwrap();
        let symbols = f.variables();
        Program::compile(&f, &symbols).unwrap()
    }

    fn eval(formula: &str, values: &[(&str, f64)]) -> f64 {
        let f = Formula::parse(formula).unwrap();
        let symbols = f.variables();
        let program = Program::compile(&f, &symbols).unwrap();
        let state: Vec<f64> = symbols
            .iter()
            .map(|s| values.iter().find(|(n, _)| n == s).unwrap().1)
            .collect();
        let mut stack = program.scratch();
        program.eval(&mut stack, &state).unwrap()
    }

    #[test]
    fn compiled_margins_match_the_semantics() {
        assert_eq!(eval("x > 5", &[("x", 8.0)]), 3.0);
        assert_eq!(eval("x < 5", &[("x", 8.0)]), -3.0);
        assert_eq!(eval("x + y * 2 > 0", &[("x", 1.0), ("y", 3.0)]), 7.0);
        assert_eq!(eval("abs(x - 10) < 1", &[("x", 7.0)]), -2.0);
        assert_eq!(eval("2 ^ 3 ^ 2 > 0", &[]), 512.0);
    }

    #[test]
    fn compiled_boolean_structure() {
        assert_eq!(eval("x > 0 and y < 10", &[("x", 5.0), ("y", 2.0)]), 5.0);
        assert_eq!(eval("x > 0 or y > 0", &[("x", -1.0), ("y", 3.0)]), 3.0);
        assert_eq!(eval("not(x > 5)", &[("x", 3.0)]), 2.0);
        assert_eq!(
            eval("(x > 10) implies (y > 0)", &[("x", 15.0), ("y", 3.0)]),
            3.0
        );
    }

    #[test]
    fn division_by_zero_keeps_the_term() {
        let program = run("x / y > 0");
        let mut stack = program.scratch();
        let err = program.eval(&mut stack, &[1.0, 0.0]).unwrap_err();
        match err {
            Error::DivisionByZero { term } => assert_eq!(term, "x / y"),
            other => panic!("expected division by zero, got {other:?}"),
        }
    }

    #[test]
    fn unknown_variable_and_function_rejected_at_compile() {
        let f = Formula::parse("x > 0").unwrap();
        assert!(matches!(
            Program::compile(&f, &[]),
            Err(Error::UnknownVariable { .. })
        ));
        let g = Formula::parse("sin(x, y) > 0").unwrap();
        assert!(matches!(
            Program::compile(&g, &["x".to_string(), "y".to_string()]),
            Err(Error::UnknownFunction { arity: 2, .. })
        ));
    }

    #[test]
    fn temporal_operator_rejected() {
        let f = Formula::parse("always[0, 5](x > 0)").unwrap();
        assert!(matches!(
            Program::compile(&f, &["x".to_string()]),
            Err(Error::Unsupported { .. })
        ));
    }
}