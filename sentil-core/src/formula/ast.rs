//! The syntax tree of a temporal-logic formula.

use core::fmt;

/// A Signal Temporal Logic formula, optionally wrapped in a probabilistic operator to form PrSTL.
#[derive(Debug, Clone, PartialEq)]
pub enum Formula {
    /// An atomic comparison between two arithmetic terms, e.g. `x + 1 < 5`.
    Predicate(Predicate),
    /// Logical negation.
    Not(Box<Formula>),
    /// Logical conjunction.
    And(Box<Formula>, Box<Formula>),
    /// Logical disjunction.
    Or(Box<Formula>, Box<Formula>),
    /// Material implication.
    Implies(Box<Formula>, Box<Formula>),
    /// `always[a, b] phi`.
    Always(Interval, Box<Formula>),
    /// `eventually[a, b] phi`.
    Eventually(Interval, Box<Formula>),
    /// `phi until[a, b] psi`.
    Until(Interval, Box<Formula>, Box<Formula>),
    /// `next phi`.
    Next(Box<Formula>),
    /// `phi since[a, b] psi`.
    Since(Interval, Box<Formula>, Box<Formula>),
    /// `historically[a, b] phi`.
    Historically(Interval, Box<Formula>),
    /// `once[a, b] phi`.
    Once(Interval, Box<Formula>),
    /// `P~p(phi)`.
    Probabilistic(ProbabilityOp, f64, Box<Formula>),
}

/// An atomic predicate.
#[derive(Debug, Clone, PartialEq)]
pub struct Predicate {
    /// The left-hand term.
    pub lhs: Expr,
    /// The comparison.
    pub op: ComparisonOp,
    /// The right-hand term.
    pub rhs: Expr,
}

/// A comparison between two terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
}

/// A binary arithmetic operator inside a predicate term.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,
    /// `^`
    Pow,
}

/// An arithmetic term over signals and constants.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A binary operation between two sub-terms.
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    /// A function applied to arguments, e.g. `abs(x)` or `max(x, y)`.
    Call(String, Vec<Expr>),
    /// A literal constant.
    Literal(f64),
    /// A reference to a named signal.
    Variable(String),
}