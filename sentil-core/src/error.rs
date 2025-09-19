//! The library's error type.

use core::fmt;

#[cfg(not(feature = "std"))]
use crate::prelude::*;
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
    #[error("no value available for variable `{name}`; add a signal named `{name}` to the trace, or include it in the streaming update")]
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

    /// A known function was called with the wrong number of arguments.
    #[error("function `{name}` takes {expected} argument(s) but got {found}")]
    ArityMismatch {
        /// The function name as written.
        name: String,
        /// How many arguments the function takes.
        expected: usize,
        /// How many arguments it was given.
        found: usize,
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
    #[error("cannot evaluate robustness over an empty trace; add at least one timed sample first")]
    EmptyTrace,

    /// A packed streaming update carried fewer values than the formula needs.
    #[error("the packed update slice has {found} values but the formula needs {expected}")]
    PackedLength {
        /// The number of variables the formula references.
        expected: usize,
        /// The number of values supplied.
        found: usize,
    },

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

    /// A noise model was given parameters outside its valid range.
    #[error("invalid {model} noise model: {reason}")]
    #[non_exhaustive]
    InvalidNoiseModel {
        /// The distribution family, for example `Gaussian`.
        model: &'static str,
        /// What was wrong with the parameters.
        reason: String,
    },

    /// Statistical checking was asked to run on a formula not wrapped in the
    /// probabilistic operator.
    #[error("statistical checking needs a formula wrapped in the probabilistic operator `P`")]
    NotProbabilistic,

    /// A trace could not be read from a file or other source.
    #[error("could not read trace: {message}")]
    #[non_exhaustive]
    Ingest {
        /// The source path, when the data came from a file.
        path: Option<String>,
        /// The 1-based row where reading failed, when known.
        row: Option<usize>,
        /// What went wrong.
        message: String,
    },

    /// Fitting a noise model to sample data failed.
    #[error("could not fit a {method} model: {message}")]
    #[non_exhaustive]
    Fit {
        /// The fitting method, for example `Gaussian MLE`.
        method: &'static str,
        /// What went wrong.
        message: String,
    },

    /// A statistical procedure was handed an invalid configuration.
    #[error("invalid {context} configuration: {message}")]
    #[non_exhaustive]
    InvalidConfig {
        /// The procedure, for example `Chernoff-Hoeffding` or `SPRT`.
        context: &'static str,
        /// What was wrong with the configuration.
        message: String,
    },

    /// A rare-event splitting run hit a numerical problem.
    #[error("adaptive splitting failed at particle {particle}, level {level}: {message}")]
    #[non_exhaustive]
    Splitting {
        /// The particle index where the problem arose.
        particle: usize,
        /// The level at which it arose.
        level: usize,
        /// What went wrong.
        message: String,
    },

    /// A formula could not be lowered to a GPU shader.
    #[cfg(feature = "gpu")]
    #[error("could not transpile to a GPU shader: {message}")]
    #[non_exhaustive]
    Transpilation {
        /// Why the formula cannot run on the GPU.
        message: String,
    },

    /// A GPU run failed after a device was acquired.
    #[cfg(feature = "gpu")]
    #[error("GPU run failed: {message}")]
    #[non_exhaustive]
    Gpu {
        /// What went wrong on the device.
        message: String,
    },
}

/// A formula failed to parse.
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

impl core::error::Error for ParseError {}