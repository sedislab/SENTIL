//! Recursive-descent parser for formula strings.

use super::ast::{BinaryOp, ComparisonOp, Expr, Formula, Interval, Predicate, ProbabilityOp};
use super::lexer::{tokenize, Token, TokenKind};
use crate::error::ParseError;
#[cfg(not(feature = "std"))]
use crate::prelude::*;

pub(crate) fn parse(input: &str) -> Result<Formula, ParseError> {
    let mut parser = Parser {
        tokens: tokenize(input)?,
        pos: 0,
        depth: 0,
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

const MAX_DEPTH: usize = 256;

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn enter(&mut self) -> Result<(), ParseError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            let (line, column) = self.position();
            return Err(ParseError::at(
                format!("formula nests deeper than the limit of {MAX_DEPTH}"),
                line,
                column,
            ));
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
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
        self.enter()?;
        let left = self.or()?;
        let formula = if matches!(self.peek(), TokenKind::Implies) {
            self.bump();
            let right = self.implies()?;
            Formula::Implies(Box::new(left), Box::new(right))
        } else {
            left
        };
        self.leave();
        Ok(formula)
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
        self.enter()?;
        let left = self.since()?;
        let formula = if matches!(self.peek(), TokenKind::Until) {
            self.bump();
            let interval = self.interval()?;
            let right = self.until()?;
            Formula::Until(interval, Box::new(left), Box::new(right))
        } else {
            left
        };
        self.leave();
        Ok(formula)
    }

    fn since(&mut self) -> Result<Formula, ParseError> {
        self.enter()?;
        let left = self.temporal()?;
        let formula = if matches!(self.peek(), TokenKind::Since) {
            self.bump();
            let interval = self.interval()?;
            let right = self.since()?;
            Formula::Since(interval, Box::new(left), Box::new(right))
        } else {
            left
        };
        self.leave();
        Ok(formula)
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
        self.enter()?;
        let formula = if matches!(self.peek(), TokenKind::Not) {
            self.bump();
            Formula::Not(Box::new(self.unary()?))
        } else {
            self.probabilistic()?
        };
        self.leave();
        Ok(formula)
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
        self.enter()?;
        let base = self.term()?;
        let expr = if matches!(self.peek(), TokenKind::Caret) {
            self.bump();
            let exponent = self.power()?;
            Expr::Binary(BinaryOp::Pow, Box::new(base), Box::new(exponent))
        } else {
            base
        };
        self.leave();
        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        self.enter()?;
        let expr = match self.peek() {
            TokenKind::LeftParen => {
                self.bump();
                let inner = self.expr()?;
                self.expect(&TokenKind::RightParen, "`)` to close the parentheses")?;
                inner
            }
            TokenKind::Minus => {
                self.bump();
                let operand = self.term()?;
                Expr::Binary(BinaryOp::Sub, Box::new(Expr::Literal(0.0)), Box::new(operand))
            }
            TokenKind::Number(n) => {
                let value = *n;
                self.bump();
                Expr::Literal(value)
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.bump();
                if matches!(self.peek(), TokenKind::LeftParen) {
                    self.bump();
                    Expr::Call(name, self.call_arguments()?)
                } else {
                    Expr::Variable(name)
                }
            }
            other => {
                let (line, column) = self.position();
                return Err(ParseError::at(
                    format!("expected a value or `(`, found {}", other.describe()),
                    line,
                    column,
                ));
            }
        };
        self.leave();
        Ok(expr)
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
        Ok(Interval::new_unchecked(lower, upper))
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

#[cfg(test)]
mod tests {
    use super::super::ast::{ComparisonOp, Formula};
    use super::parse;

    fn round_trip(input: &str, canonical: &str) {
        let f = parse(input).expect("should parse");
        assert_eq!(f.to_string(), canonical);
    }

    #[test]
    fn simple_predicate() {
        round_trip("x < 5", "x < 5");
    }

    #[test]
    fn and_binds_tighter_than_implies() {
        round_trip(
            "a > 0 and b > 0 implies c > 0",
            "((a > 0 and b > 0) implies c > 0)",
        );
    }

    #[test]
    fn or_is_left_associative() {
        round_trip("a > 0 or b > 0 or c > 0", "((a > 0 or b > 0) or c > 0)");
    }

    #[test]
    fn until_binds_tighter_than_and() {
        round_trip(
            "a > 0 until b > 0 and c > 0",
            "((a > 0 until[0, inf] b > 0) and c > 0)",
        );
    }

    #[test]
    fn temporal_with_and_without_interval() {
        round_trip("always[0, 10](x < 5)", "always[0, 10](x < 5)");
        round_trip("eventually(x > 0)", "eventually[0, inf](x > 0)");
    }

    #[test]
    fn ltl_shorthands_and_symbols() {
        round_trip(
            "G[0, inf](!(x_1 > 0) && y_2 > 0 -> F(z == 5))",
            "always[0, inf](((not(x_1 > 0) and y_2 > 0) implies eventually[0, inf](z == 5)))",
        );
    }

    #[test]
    fn arithmetic_precedence_and_unary_minus() {
        round_trip("x + y * 2 < 10", "(x + (y * 2)) < 10");
        round_trip("x < -5", "x < (0 - 5)");
        round_trip("2 ^ 3 ^ 2 > 0", "(2 ^ (3 ^ 2)) > 0");
    }

    #[test]
    fn function_calls() {
        round_trip("abs(x - 1) < 2", "abs((x - 1)) < 2");
        round_trip("max(x, y, 0) > 0", "max(x, y, 0) > 0");
    }

    #[test]
    fn probabilistic_with_nested_temporal() {
        round_trip(
            "P>=0.95(always[0, 10](x > 5))",
            "P>=0.95(always[0, 10](x > 5))",
        );
    }

    #[test]
    fn until_is_right_associative() {
        round_trip(
            "a > 0 until b > 0 until c > 0",
            "(a > 0 until[0, inf] (b > 0 until[0, inf] c > 0))",
        );
    }

    #[test]
    fn missing_value_points_at_the_offending_token() {
        let err = parse("always[0, 10](x > )").unwrap_err();
        assert!(err.message.contains("expected a value"));
        assert_eq!(err.column, 19);
    }

    #[test]
    fn infinite_lower_bound_is_rejected() {
        let err = parse("always[inf, 10](x > 0)").unwrap_err();
        assert!(err.message.contains("lower bound cannot be `inf`"));
    }

    #[test]
    fn lower_above_upper_is_rejected() {
        let err = parse("always[10, 5](x > 0)").unwrap_err();
        assert!(err.message.contains("greater than upper bound"));
    }

    #[test]
    fn probability_threshold_out_of_range() {
        let err = parse("P>=1.5(x > 0)").unwrap_err();
        assert!(err.message.contains("between 0 and 1"));
    }

    #[test]
    fn trailing_tokens_are_an_error() {
        let err = parse("x > 0 y > 0").unwrap_err();
        assert!(err.message.contains("after a complete formula"));
    }

    #[test]
    fn negative_interval_bound_parses() {
        let f = parse("historically[0, 5](x > 0)").unwrap();
        assert!(matches!(f, Formula::Historically(..)));
    }

    #[test]
    fn equality_predicate() {
        let f = parse("x == 5").unwrap();
        match f {
            Formula::Predicate(p) => assert_eq!(p.op, ComparisonOp::Equal),
            _ => panic!("expected a predicate"),
        }
    }

    #[test]
    fn moderately_nested_input_still_parses() {
        let nested = format!("{}x > 0{}", "(".repeat(30), ")".repeat(30));
        assert!(parse(&nested).is_ok());
    }

    #[test]
    fn pathological_nesting_reports_an_error_rather_than_overflowing() {
        let bombs = [
            format!("{}x > 0{}", "(".repeat(20_000), ")".repeat(20_000)),
            format!("{}x > 0", "not ".repeat(20_000)),
            format!("x > {}1", "-".repeat(20_000)),
        ];
        for bomb in bombs {
            let err = parse(&bomb).unwrap_err();
            assert!(err.message.contains("deeper than"), "got: {}", err.message);
        }
    }
}