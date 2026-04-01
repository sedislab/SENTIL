//! The syntax tree of a temporal-logic formula.

use crate::error::{Error, Result};
#[cfg(not(feature = "std"))]
use crate::prelude::*;
use core::fmt;

/// A Signal Temporal Logic formula, optionally wrapped in a probabilistic operator to form PrSTL.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Interval {
    lower: f64,
    upper: Option<f64>,
}

impl Interval {
    /// Builds `[lower, upper]`, where `upper` is `None` for an interval unbounded
    /// above.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if the bounds are not a valid interval.
    pub fn new(lower: f64, upper: Option<f64>) -> Result<Self> {
        if !lower.is_finite() || lower < 0.0 {
            return Err(interval_error(format!(
                "an interval lower bound must be finite and at least 0, found {lower}"
            )));
        }
        if let Some(u) = upper {
            if !u.is_finite() {
                return Err(interval_error(format!(
                    "an interval upper bound must be finite; leave it unbounded for inf, found {u}"
                )));
            }
            if lower > u {
                return Err(interval_error(format!(
                    "interval lower bound {lower} is greater than upper bound {u}"
                )));
            }
        }
        Ok(Self { lower, upper })
    }

    pub(crate) fn new_unchecked(lower: f64, upper: Option<f64>) -> Self {
        Self { lower, upper }
    }

    /// A bounded interval `[lower, upper]`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if the bounds are not a valid interval.
    pub fn bounded(lower: f64, upper: f64) -> Result<Self> {
        Self::new(lower, Some(upper))
    }

    /// An interval `[lower, inf)` unbounded above.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] if `lower` is not finite and non-negative.
    pub fn from_lower(lower: f64) -> Result<Self> {
        Self::new(lower, None)
    }

    /// The interval `[0, inf)`.
    pub fn unbounded() -> Self {
        Self {
            lower: 0.0,
            upper: None,
        }
    }

    /// The lower bound.
    pub fn lower(&self) -> f64 {
        self.lower
    }

    /// The upper bound, or `None` when the interval is unbounded above.
    pub fn upper(&self) -> Option<f64> {
        self.upper
    }

    /// Whether the interval has a finite upper bound.
    pub fn is_bounded(&self) -> bool {
        self.upper.is_some()
    }

    /// Whether the interval is the whole future `[0, inf)`.
    pub fn is_unbounded(&self) -> bool {
        self.upper.is_none() && self.lower <= 0.0
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

fn interval_error(message: String) -> Error {
    Error::InvalidConfig {
        context: "interval",
        message,
    }
}

impl Formula {
    /// The signals the formula mentions, sorted and deduplicated.
    pub fn variables(&self) -> Vec<String> {
        let mut vars = Vec::new();
        self.collect_variables(&mut vars);
        vars.sort();
        vars.dedup();
        vars
    }

    /// The nesting depth.
    pub fn depth(&self) -> usize {
        match self {
            Formula::Predicate(_) => 1,
            Formula::Not(f)
            | Formula::Next(f)
            | Formula::Always(_, f)
            | Formula::Eventually(_, f)
            | Formula::Historically(_, f)
            | Formula::Once(_, f)
            | Formula::Probabilistic(_, _, f) => 1 + f.depth(),
            Formula::And(l, r)
            | Formula::Or(l, r)
            | Formula::Implies(l, r)
            | Formula::Until(_, l, r)
            | Formula::Since(_, l, r) => 1 + l.depth().max(r.depth()),
        }
    }

    /// Whether the formula contains any temporal operator.
    pub fn has_temporal(&self) -> bool {
        match self {
            Formula::Predicate(_) => false,
            Formula::Not(f) | Formula::Probabilistic(_, _, f) => f.has_temporal(),
            Formula::And(l, r) | Formula::Or(l, r) | Formula::Implies(l, r) => {
                l.has_temporal() || r.has_temporal()
            }
            Formula::Always(..)
            | Formula::Eventually(..)
            | Formula::Until(..)
            | Formula::Since(..)
            | Formula::Historically(..)
            | Formula::Once(..)
            | Formula::Next(_) => true,
        }
    }

    fn collect_variables(&self, vars: &mut Vec<String>) {
        match self {
            Formula::Predicate(p) => {
                p.lhs.collect_variables(vars);
                p.rhs.collect_variables(vars);
            }
            Formula::Not(f)
            | Formula::Next(f)
            | Formula::Always(_, f)
            | Formula::Eventually(_, f)
            | Formula::Historically(_, f)
            | Formula::Once(_, f)
            | Formula::Probabilistic(_, _, f) => f.collect_variables(vars),
            Formula::And(l, r)
            | Formula::Or(l, r)
            | Formula::Implies(l, r)
            | Formula::Until(_, l, r)
            | Formula::Since(_, l, r) => {
                l.collect_variables(vars);
                r.collect_variables(vars);
            }
        }
    }
}

impl Expr {
    /// The nesting depth of the term.
    pub fn depth(&self) -> usize {
        match self {
            Expr::Binary(_, l, r) => 1 + l.depth().max(r.depth()),
            Expr::Call(_, args) => 1 + args.iter().map(Expr::depth).max().unwrap_or(0),
            Expr::Variable(_) | Expr::Literal(_) => 1,
        }
    }

    fn collect_variables(&self, vars: &mut Vec<String>) {
        match self {
            Expr::Binary(_, l, r) => {
                l.collect_variables(vars);
                r.collect_variables(vars);
            }
            Expr::Call(_, args) => {
                for arg in args {
                    arg.collect_variables(vars);
                }
            }
            Expr::Variable(name) => vars.push(name.clone()),
            Expr::Literal(_) => {}
        }
    }
}

impl fmt::Display for Formula {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Formula::Predicate(p) => write!(f, "{p}"),
            Formula::Not(inner) => write!(f, "not({inner})"),
            Formula::And(l, r) => write!(f, "({l} and {r})"),
            Formula::Or(l, r) => write!(f, "({l} or {r})"),
            Formula::Implies(l, r) => write!(f, "({l} implies {r})"),
            Formula::Always(i, inner) => write!(f, "always{i}({inner})"),
            Formula::Eventually(i, inner) => write!(f, "eventually{i}({inner})"),
            Formula::Until(i, l, r) => write!(f, "({l} until{i} {r})"),
            Formula::Next(inner) => write!(f, "next({inner})"),
            Formula::Since(i, l, r) => write!(f, "({l} since{i} {r})"),
            Formula::Historically(i, inner) => write!(f, "historically{i}({inner})"),
            Formula::Once(i, inner) => write!(f, "once{i}({inner})"),
            Formula::Probabilistic(op, threshold, inner) => write!(f, "P{op}{threshold}({inner})"),
        }
    }
}

impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.lhs, self.op, self.rhs)
    }
}

impl Expr {
    fn write_prec(&self, f: &mut fmt::Formatter<'_>, ctx: u8) -> fmt::Result {
        match self {
            Expr::Binary(op, l, r) => {
                let p = match op {
                    BinaryOp::Add | BinaryOp::Sub => 1,
                    BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 2,
                    BinaryOp::Pow => 3,
                };
                let (lctx, rctx) = match op {
                    BinaryOp::Pow => (p + 1, p),
                    _ => (p, p + 1),
                };
                let wrap = p < ctx;
                if wrap {
                    f.write_str("(")?;
                }
                l.write_prec(f, lctx)?;
                write!(f, " {op} ")?;
                r.write_prec(f, rctx)?;
                if wrap {
                    f.write_str(")")?;
                }
                Ok(())
            }
            Expr::Call(name, args) => {
                write!(f, "{name}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    arg.write_prec(f, 0)?;
                }
                write!(f, ")")
            }
            Expr::Literal(v) => write!(f, "{v}"),
            Expr::Variable(v) => write!(f, "{v}"),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_prec(f, 0)
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.upper {
            Some(u) => write!(f, "[{}, {}]", self.lower, u),
            None => write!(f, "[{}, inf]", self.lower),
        }
    }
}

impl fmt::Display for ComparisonOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ComparisonOp::Less => "<",
            ComparisonOp::LessEqual => "<=",
            ComparisonOp::Greater => ">",
            ComparisonOp::GreaterEqual => ">=",
            ComparisonOp::Equal => "==",
            ComparisonOp::NotEqual => "!=",
        };
        f.write_str(s)
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Pow => "^",
        };
        f.write_str(s)
    }
}

impl fmt::Display for ProbabilityOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ProbabilityOp::GreaterEqual => ">=",
            ProbabilityOp::Greater => ">",
            ProbabilityOp::LessEqual => "<=",
            ProbabilityOp::Less => "<",
        };
        f.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pred(name: &str, op: ComparisonOp, c: f64) -> Formula {
        Formula::Predicate(Predicate {
            lhs: Expr::Variable(name.into()),
            op,
            rhs: Expr::Literal(c),
        })
    }

    #[test]
    fn display_renders_a_bounded_temporal_formula() {
        let f = Formula::Always(
            Interval::bounded(0.0, 10.0).unwrap(),
            Box::new(pred("x", ComparisonOp::Less, 5.0)),
        );
        assert_eq!(f.to_string(), "always[0, 10](x < 5)");
    }

    #[test]
    fn unbounded_interval_prints_inf() {
        let f = Formula::Eventually(
            Interval::unbounded(),
            Box::new(pred("x", ComparisonOp::Greater, 0.0)),
        );
        assert_eq!(f.to_string(), "eventually[0, inf](x > 0)");
    }

    #[test]
    fn variables_are_sorted_and_deduplicated() {
        let f = Formula::And(
            Box::new(pred("speed", ComparisonOp::Greater, 60.0)),
            Box::new(pred("rpm", ComparisonOp::Less, 4000.0)),
        );
        assert_eq!(f.variables(), vec!["rpm".to_string(), "speed".to_string()]);
    }

    #[test]
    fn depth_counts_operator_nesting() {
        let inner = pred("x", ComparisonOp::Less, 5.0);
        let f = Formula::And(
            Box::new(Formula::Always(
                Interval::bounded(0.0, 10.0).unwrap(),
                Box::new(inner.clone()),
            )),
            Box::new(inner),
        );
        assert_eq!(f.depth(), 3);
    }

    #[test]
    fn has_temporal_distinguishes_boolean_from_temporal() {
        let p = pred("x", ComparisonOp::Greater, 0.0);
        assert!(!p.has_temporal());
        assert!(!Formula::Not(Box::new(p.clone())).has_temporal());
        assert!(!Formula::And(Box::new(p.clone()), Box::new(p.clone())).has_temporal());
        let always = Formula::Always(Interval::bounded(0.0, 5.0).unwrap(), Box::new(p.clone()));
        assert!(always.has_temporal());
        assert!(
            Formula::Probabilistic(ProbabilityOp::GreaterEqual, 0.9, Box::new(always))
                .has_temporal()
        );
        assert!(
            !Formula::Probabilistic(ProbabilityOp::GreaterEqual, 0.9, Box::new(p)).has_temporal()
        );
    }

    #[test]
    fn interval_contains_is_inclusive_at_both_ends() {
        let i = Interval::bounded(0.0, 10.0).unwrap();
        assert!(i.contains(0.0) && i.contains(10.0) && i.contains(5.0));
        assert!(!i.contains(-0.1) && !i.contains(10.1));
        assert!(Interval::unbounded().contains(1e9));
        assert!(!Interval::unbounded().contains(-1.0));
    }

    #[test]
    fn probabilistic_operator_renders_threshold() {
        let f = Formula::Probabilistic(
            ProbabilityOp::GreaterEqual,
            0.95,
            Box::new(pred("x", ComparisonOp::Greater, 0.0)),
        );
        assert_eq!(f.to_string(), "P>=0.95(x > 0)");
    }

    #[test]
    fn arithmetic_term_prints_with_minimal_parentheses() {
        let no_parens = Expr::Binary(
            BinaryOp::Add,
            Box::new(Expr::Variable("x".into())),
            Box::new(Expr::Binary(
                BinaryOp::Mul,
                Box::new(Expr::Variable("y".into())),
                Box::new(Expr::Literal(2.0)),
            )),
        );
        assert_eq!(no_parens.to_string(), "x + y * 2");
        assert_eq!(no_parens.depth(), 3);

        let needs_parens = Expr::Binary(
            BinaryOp::Mul,
            Box::new(Expr::Binary(
                BinaryOp::Add,
                Box::new(Expr::Variable("x".into())),
                Box::new(Expr::Variable("y".into())),
            )),
            Box::new(Expr::Literal(2.0)),
        );
        assert_eq!(needs_parens.to_string(), "(x + y) * 2");

        let left_pow = Expr::Binary(
            BinaryOp::Pow,
            Box::new(Expr::Binary(
                BinaryOp::Pow,
                Box::new(Expr::Variable("a".into())),
                Box::new(Expr::Variable("b".into())),
            )),
            Box::new(Expr::Variable("c".into())),
        );
        assert_eq!(left_pow.to_string(), "(a ^ b) ^ c");
    }

    #[test]
    fn invalid_intervals_are_rejected() {
        assert!(Interval::bounded(5.0, 1.0).is_err());
        assert!(Interval::bounded(-1.0, 5.0).is_err());
        assert!(Interval::bounded(f64::NAN, 5.0).is_err());
        assert!(Interval::from_lower(-2.0).is_err());
        assert!(Interval::bounded(0.0, 10.0).is_ok());
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use crate::Formula;

    #[test]
    fn a_formula_round_trips_through_json() {
        let phi =
            Formula::parse("P >= 0.9(always[0, 10]((x + 1 > 5) until[0, 2] (y < 3)))").unwrap();
        let json = serde_json::to_string(&phi).unwrap();
        let back: Formula = serde_json::from_str(&json).unwrap();
        assert_eq!(phi, back);
    }
}