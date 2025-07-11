//! Recursive-descent parser for formula strings.

use super::ast::{BinaryOp, ComparisonOp, Expr, Formula, Interval, Predicate, ProbabilityOp};
use super::lexer::{tokenize, Token, TokenKind};
use crate::error::ParseError;

pub(crate) fn parse(input: &str) -> Result<Formula, ParseError> {
    let mut parser = Parser {
        tokens: tokenize(input)?,
        pos: 0,
    };
    let formula = parser.formula()?;
    if !matches!(parser.peek(), TokenKind::End) {
        let (line, column) = parser.position();
        return Err(ParseError::at(
            format!(
                "unexpected {} after a complete formula",
                parser.peek().describe()
            ),
            line,
            column,
        ));
    }
    Ok(formula)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn position(&self) -> (usize, usize) {
        let t = &self.tokens[self.pos];
        (t.line, t.column)
    }

    fn bump(&mut self) {
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn formula(&mut self) -> Result<Formula, ParseError> {
        self.implies()
    }

    fn implies(&mut self) -> Result<Formula, ParseError> {
        let left = self.or()?;
        if matches!(self.peek(), TokenKind::Implies) {
            self.bump();
            let right = self.implies()?;
            return Ok(Formula::Implies(Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    fn or(&mut self) -> Result<Formula, ParseError> {
        let mut left = self.and()?;
        while matches!(self.peek(), TokenKind::Or) {
            self.bump();
            let right = self.and()?;
            left = Formula::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Formula, ParseError> {
        let mut left = self.until()?;
        while matches!(self.peek(), TokenKind::And) {
            self.bump();
            let right = self.until()?;
            left = Formula::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

}