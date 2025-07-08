//! The library's error type.

use core::fmt;

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