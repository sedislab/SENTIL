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

    fn primary(&mut self) -> Result<Formula, ParseError> {
        if matches!(self.peek(), TokenKind::LeftParen) {
            self.bump();
            let inner = self.formula()?;
            self.expect(&TokenKind::RightParen, "`)` to close the group")?;
            return Ok(inner);
        }
        self.predicate()
    }

    fn predicate(&mut self) -> Result<Formula, ParseError> {
        let lhs = self.expr()?;
        let op = self.comparison_op()?;
        let rhs = self.expr()?;
        Ok(Formula::Predicate(Predicate { lhs, op, rhs }))
    }

    fn expr(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.factor()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.bump();
            let right = self.factor()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.power()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => break,
            };
            self.bump();
            let right = self.power()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn power(&mut self) -> Result<Expr, ParseError> {
        let base = self.term()?;
        if matches!(self.peek(), TokenKind::Caret) {
            self.bump();
            let exponent = self.power()?;
            return Ok(Expr::Binary(
                BinaryOp::Pow,
                Box::new(base),
                Box::new(exponent),
            ));
        }
        Ok(base)
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            TokenKind::LeftParen => {
                self.bump();
                let inner = self.expr()?;
                self.expect(&TokenKind::RightParen, "`)` to close the parentheses")?;
                Ok(inner)
            }
            TokenKind::Minus => {
                self.bump();
                let operand = self.term()?;
                Ok(Expr::Binary(
                    BinaryOp::Sub,
                    Box::new(Expr::Literal(0.0)),
                    Box::new(operand),
                ))
            }
            TokenKind::Number(n) => {
                let value = *n;
                self.bump();
                Ok(Expr::Literal(value))
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.bump();
                if matches!(self.peek(), TokenKind::LeftParen) {
                    self.bump();
                    Ok(Expr::Call(name, self.call_arguments()?))
                } else {
                    Ok(Expr::Variable(name))
                }
            }
            other => {
                let (line, column) = self.position();
                Err(ParseError::at(
                    format!("expected a value or `(`, found {}", other.describe()),
                    line,
                    column,
                ))
            }
        }
    }

    fn call_arguments(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        if matches!(self.peek(), TokenKind::RightParen) {
            self.bump();
            return Ok(args);
        }
        loop {
            args.push(self.expr()?);
            if matches!(self.peek(), TokenKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.expect(&TokenKind::RightParen, "`)` to close the argument list")?;
        Ok(args)
    }

    fn interval(&mut self) -> Result<Interval, ParseError> {
        if !matches!(self.peek(), TokenKind::LeftBracket) {
            return Ok(Interval::unbounded());
        }
        let (line, column) = self.position();
        self.bump();
        let lower = self.bound()?;
        self.expect(&TokenKind::Comma, "`,` between the interval bounds")?;
        let upper = self.bound()?;
        self.expect(&TokenKind::RightBracket, "`]` to close the interval")?;

        let lower = lower.ok_or_else(|| {
            ParseError::at("an interval's lower bound cannot be `inf`", line, column)
        })?;
        if lower < 0.0 {
            return Err(ParseError::at(
                format!("an interval's lower bound must be at least 0, found {lower}"),
                line,
                column,
            ));
        }
        if let Some(u) = upper {
            if lower > u {
                return Err(ParseError::at(
                    format!("interval lower bound {lower} is greater than upper bound {u}"),
                    line,
                    column,
                ));
            }
        }
        Ok(Interval { lower, upper })
    }

    fn bound(&mut self) -> Result<Option<f64>, ParseError> {
        match self.peek() {
            TokenKind::Infinity => {
                self.bump();
                Ok(None)
            }
            TokenKind::Minus => {
                self.bump();
                Ok(Some(-self.number("an interval bound")?))
            }
            TokenKind::Number(n) => {
                let value = *n;
                self.bump();
                Ok(Some(value))
            }
            other => {
                let (line, column) = self.position();
                Err(ParseError::at(
                    format!("expected a number or `inf`, found {}", other.describe()),
                    line,
                    column,
                ))
            }
        }
    }

    fn number(&mut self, what: &str) -> Result<f64, ParseError> {
        if let TokenKind::Number(n) = self.peek() {
            let value = *n;
            self.bump();
            Ok(value)
        } else {
            let (line, column) = self.position();
            Err(ParseError::at(
                format!("expected {what}, found {}", self.peek().describe()),
                line,
                column,
            ))
        }
    }

    fn signed_number(&mut self, what: &str) -> Result<f64, ParseError> {
        if matches!(self.peek(), TokenKind::Minus) {
            self.bump();
            Ok(-self.number(what)?)
        } else {
            self.number(what)
        }
    }

    fn comparison_op(&mut self) -> Result<ComparisonOp, ParseError> {
        let op = match self.peek() {
            TokenKind::Less => ComparisonOp::Less,
            TokenKind::LessEqual => ComparisonOp::LessEqual,
            TokenKind::Greater => ComparisonOp::Greater,
            TokenKind::GreaterEqual => ComparisonOp::GreaterEqual,
            TokenKind::Equal => ComparisonOp::Equal,
            TokenKind::NotEqual => ComparisonOp::NotEqual,
            other => {
                let (line, column) = self.position();
                return Err(ParseError::at(
                    format!(
                        "expected a comparison (<, <=, >, >=, ==, !=), found {}",
                        other.describe()
                    ),
                    line,
                    column,
                ));
            }
        };
        self.bump();
        Ok(op)
    }

    fn probability_op(&mut self) -> Result<ProbabilityOp, ParseError> {
        let op = match self.peek() {
            TokenKind::GreaterEqual => ProbabilityOp::GreaterEqual,
            TokenKind::Greater => ProbabilityOp::Greater,
            TokenKind::LessEqual => ProbabilityOp::LessEqual,
            TokenKind::Less => ProbabilityOp::Less,
            other => {
                let (line, column) = self.position();
                return Err(ParseError::at(
                    format!(
                        "`P` must be followed by >=, >, <=, or <, found {}",
                        other.describe()
                    ),
                    line,
                    column,
                ));
            }
        };
        self.bump();
        Ok(op)
    }

    fn expect(&mut self, kind: &TokenKind, what: &str) -> Result<(), ParseError> {
        if core::mem::discriminant(self.peek()) == core::mem::discriminant(kind) {
            self.bump();
            Ok(())
        } else {
            let (line, column) = self.position();
            Err(ParseError::at(
                format!("expected {what}, found {}", self.peek().describe()),
                line,
                column,
            ))
        }
    }
}