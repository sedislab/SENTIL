//! The STL and PrSTL syntax tree, plus the lexer and parser that build one.

mod ast;
mod lexer;
mod parser;

pub use ast::{BinaryOp, ComparisonOp, Expr, Formula, Interval, Predicate, ProbabilityOp};

impl Formula {
    /// Parses a formula from its textual form.
    ///
    /// ```
    /// use sentil::Formula;
    ///
    /// let phi = Formula::parse("always[0, 10](speed < 5)")?;
    /// assert_eq!(phi.to_string(), "always[0, 10](speed < 5)");
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parse`](crate::Error::Parse) with the line and column of
    /// the offending token.
    pub fn parse(input: &str) -> crate::Result<Self> {
        Ok(parser::parse(input)?)
    }
}