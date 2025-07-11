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

    fn until(&mut self) -> Result<Formula, ParseError> {
        let left = self.since()?;
        if matches!(self.peek(), TokenKind::Until) {
            self.bump();
            let interval = self.interval()?;
            let right = self.until()?;
            return Ok(Formula::Until(interval, Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    fn since(&mut self) -> Result<Formula, ParseError> {
        let left = self.temporal()?;
        if matches!(self.peek(), TokenKind::Since) {
            self.bump();
            let interval = self.interval()?;
            let right = self.since()?;
            return Ok(Formula::Since(interval, Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    fn temporal(&mut self) -> Result<Formula, ParseError> {
        match self.peek() {
            TokenKind::Always => {
                self.bump();
                let interval = self.interval()?;
                Ok(Formula::Always(interval, Box::new(self.unary()?)))
            }
            TokenKind::Eventually => {
                self.bump();
                let interval = self.interval()?;
                Ok(Formula::Eventually(interval, Box::new(self.unary()?)))
            }
            TokenKind::Historically => {
                self.bump();
                let interval = self.interval()?;
                Ok(Formula::Historically(interval, Box::new(self.unary()?)))
            }
            TokenKind::Once => {
                self.bump();
                let interval = self.interval()?;
                Ok(Formula::Once(interval, Box::new(self.unary()?)))
            }
            TokenKind::Next => {
                self.bump();
                Ok(Formula::Next(Box::new(self.unary()?)))
            }
            _ => self.unary(),
        }
    }

    fn unary(&mut self) -> Result<Formula, ParseError> {
        if matches!(self.peek(), TokenKind::Not) {
            self.bump();
            return Ok(Formula::Not(Box::new(self.unary()?)));
        }
        self.probabilistic()
    }

    fn probabilistic(&mut self) -> Result<Formula, ParseError> {
        if !matches!(self.peek(), TokenKind::Probability) {
            return self.primary();
        }
        let (line, column) = self.position();
        self.bump();
        let op = self.probability_op()?;
        let threshold = self.signed_number("a probability threshold")?;
        if !(0.0..=1.0).contains(&threshold) {
            return Err(ParseError::at(
                format!("probability threshold {threshold} must lie between 0 and 1"),
                line,
                column,
            ));
        }
        self.expect(&TokenKind::LeftParen, "`(` to open the probabilistic body")?;
        let inner = self.formula()?;
        self.expect(
            &TokenKind::RightParen,
            "`)` to close the probabilistic body",
        )?;
        Ok(Formula::Probabilistic(op, threshold, Box::new(inner)))
    }

}