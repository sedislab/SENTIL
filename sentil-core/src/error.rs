//! The library's error type.
//!
//! Every fallible operation returns [`Result<T>`], an alias for
//! `Result<T, Error>`. [`Error`] is a single enum: each value says what went
//! wrong, where in the formula or signal it happened, and what a correct input
//! would look like, so a caller never has to guess.

use core::fmt;

use thiserror::Error;

/// The result of any fallible SENTIL operation.
pub type Result<T> = core::result::Result<T, Error>;

/// Everything that can go wrong, as one enum.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum Error {
    /// A formula string could not be parsed.
    #[error(transparent)]
    Parse(#[from] ParseError),

    /// A formula refers to a variable with no value available.
    #[error("no value available for variable `{name}`")]
    UnknownVariable {
        /// The variable as written in the formula.
        name: String,
    },

    /// An arithmetic term divided by zero.
    #[error("division by zero while evaluating `{term}`")]
    DivisionByZero {
        /// The sub-expression that divided by zero.
        term: String,
    },

    /// A predicate called a function the evaluator does not provide.
    #[error(
        "unknown function `{name}` applied to {arity} argument(s); \
         supported functions are abs, sqrt, exp, ln, log, sin, cos, tan, floor, ceil, min, max"
    )]
    UnknownFunction {
        /// The function name as written.
        name: String,
        /// How many arguments it was given.
        arity: usize,
    },

    /// A trace's sample times were not strictly increasing.
    #[error("trace times must strictly increase, but time {time} does not follow {previous}")]
    NonMonotonicTime {
        /// The previous sample time.
        previous: f64,
        /// The offending time that did not increase.
        time: f64,
    },

    /// A trace carried a non-finite time or value.
    #[error("a trace {kind} was not finite: {value}")]
    NonFiniteSample {
        /// Whether the offending number was a `time` or a `value`.
        kind: &'static str,
        /// The non-finite number.
        value: f64,
    },

    /// A signal's sample count did not match the trace's time points.
    #[error("signal `{variable}` has {found} samples but the trace has {expected} time points")]
    SignalLengthMismatch {
        /// The signal whose length was wrong.
        variable: String,
        /// The number of time points the trace has.
        expected: usize,
        /// The number of samples the signal provided.
        found: usize,
    },

    /// Robustness was requested over a trace with no samples.
    #[error("cannot evaluate robustness over an empty trace")]
    EmptyTrace,

    /// A probabilistic operator reached deterministic evaluation.
    #[error(
        "the probabilistic operator `P` needs statistical evaluation; \
         deterministic robustness is undefined for it"
    )]
    ProbabilisticOperator,

    /// The current build, target, or evaluation path does not support a feature.
    #[error("unsupported: {feature}")]
    Unsupported {
        /// What is not supported.
        feature: &'static str,
    },
}

/// A formula failed to parse.
///
/// SENTIL formulas are usually one line, so the column points at the token where
/// parsing stopped. Both coordinates are 1-based.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// What went wrong.
    pub message: String,
    /// The 1-based line of the offending token.
    pub line: usize,
    /// The 1-based column of the offending token.
    pub column: usize,
}

impl ParseError {
    /// Builds a parse error at a 1-based line and column.
    #[must_use]
    pub fn at(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            line,
            column,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "parse error at line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for ParseError {}