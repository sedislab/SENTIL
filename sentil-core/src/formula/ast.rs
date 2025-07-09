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

/// The relation between an estimated satisfaction probability and the threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbabilityOp {
    /// The probability is at least the threshold.
    GreaterEqual,
    /// The probability exceeds the threshold.
    Greater,
    /// The probability is at most the threshold.
    LessEqual,
    /// The probability is below the threshold.
    Less,
}

/// A time interval `[lower, upper]` over which a temporal operator quantifies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: Option<f64>,
}

impl Interval {
    /// A bounded interval `[lower, upper]`.
    pub fn bounded(lower: f64, upper: f64) -> Self {
        Self {
            lower,
            upper: Some(upper),
        }
    }

    /// An interval `[lower, inf)` unbounded above.
    pub fn from_lower(lower: f64) -> Self {
        Self { lower, upper: None }
    }

    /// The interval `[0, inf)`.
    pub fn unbounded() -> Self {
        Self {
            lower: 0.0,
            upper: None,
        }
    }

    /// Whether the interval has a finite upper bound.
    pub fn is_bounded(&self) -> bool {
        self.upper.is_some()
    }

    /// Whether `t` lies in the interval.
    pub fn contains(&self, t: f64) -> bool {
        t >= self.lower && self.upper.is_none_or(|u| t <= u)
    }

    /// The upper bound, or positive infinity when unbounded.
    pub fn upper_or_infinity(&self) -> f64 {
        self.upper.unwrap_or(f64::INFINITY)
    }
}