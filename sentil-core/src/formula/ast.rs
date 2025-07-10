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

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Binary(op, l, r) => write!(f, "({l} {op} {r})"),
            Expr::Call(name, args) => {
                write!(f, "{name}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                write!(f, ")")
            }
            Expr::Literal(v) => write!(f, "{v}"),
            Expr::Variable(v) => write!(f, "{v}"),
        }
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