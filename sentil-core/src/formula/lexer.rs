//! Tokenizer for formula strings.

use crate::error::ParseError;
#[cfg(not(feature = "std"))]
use crate::prelude::*;
#[cfg(feature = "std")]
use std::borrow::Cow;
#[cfg(not(feature = "std"))]
use alloc::borrow::Cow;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TokenKind {
    Identifier(String),
    Number(f64),

    Always,
    Eventually,
    Until,
    Next,
    Since,
    Historically,
    Once,

    And,
    Or,
    Not,
    Implies,
    Probability,

    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,

    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Comma,

    Minus,
    Plus,
    Star,
    Slash,
    Percent,
    Caret,

    Infinity,
    End,
}

impl TokenKind {
    pub(crate) fn describe(&self) -> Cow<'static, str> {
        match self {
            TokenKind::Identifier(s) => format!("variable `{s}`").into(),
            TokenKind::Number(n) => format!("number {n}").into(),
            TokenKind::Always => "`always`".into(),
            TokenKind::Eventually => "`eventually`".into(),
            TokenKind::Until => "`until`".into(),
            TokenKind::Next => "`next`".into(),
            TokenKind::Since => "`since`".into(),
            TokenKind::Historically => "`historically`".into(),
            TokenKind::Once => "`once`".into(),
            TokenKind::And => "`and`".into(),
            TokenKind::Or => "`or`".into(),
            TokenKind::Not => "`not`".into(),
            TokenKind::Implies => "`implies`".into(),
            TokenKind::Probability => "`P`".into(),
            TokenKind::Less => "`<`".into(),
            TokenKind::LessEqual => "`<=`".into(),
            TokenKind::Greater => "`>`".into(),
            TokenKind::GreaterEqual => "`>=`".into(),
            TokenKind::Equal => "`==`".into(),
            TokenKind::NotEqual => "`!=`".into(),
            TokenKind::LeftParen => "`(`".into(),
            TokenKind::RightParen => "`)`".into(),
            TokenKind::LeftBracket => "`[`".into(),
            TokenKind::RightBracket => "`]`".into(),
            TokenKind::Comma => "`,`".into(),
            TokenKind::Minus => "`-`".into(),
            TokenKind::Plus => "`+`".into(),
            TokenKind::Star => "`*`".into(),
            TokenKind::Slash => "`/`".into(),
            TokenKind::Percent => "`%`".into(),
            TokenKind::Caret => "`^`".into(),
            TokenKind::Infinity => "`inf`".into(),
            TokenKind::End => "end of input".into(),
        }
    }
}

pub(crate) fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    Lexer::new(input).run()
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn run(mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_trivia();
            if self.at_end() {
                tokens.push(Token {
                    kind: TokenKind::End,
                    line: self.line,
                    column: self.column,
                });
                return Ok(tokens);
            }
            tokens.push(self.next_token()?);
        }
    }

    fn next_token(&mut self) -> Result<Token, ParseError> {
        let line = self.line;
        let column = self.column;
        let ch = self.peek();

        let kind = if ch.is_alphabetic() || ch == '_' {
            self.word()
        } else if ch.is_ascii_digit()
            || (ch == '.' && self.peek_next().is_some_and(|c| c.is_ascii_digit()))
        {
            self.number(line, column)?
        } else {
            self.symbol(line, column)?
        };

        Ok(Token { kind, line, column })
    }

    fn symbol(&mut self, line: usize, column: usize) -> Result<TokenKind, ParseError> {
        let ch = self.peek();
        self.advance();
        let kind = match ch {
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            ',' => TokenKind::Comma,
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '^' => TokenKind::Caret,
            '-' => {
                if self.peek() == '>' {
                    self.advance();
                    TokenKind::Implies
                } else {
                    TokenKind::Minus
                }
            }
            '<' => self.maybe_eq(TokenKind::LessEqual, TokenKind::Less),
            '>' => self.maybe_eq(TokenKind::GreaterEqual, TokenKind::Greater),
            '!' => self.maybe_eq(TokenKind::NotEqual, TokenKind::Not),
            '&' => {
                if self.peek() == '&' {
                    self.advance();
                }
                TokenKind::And
            }
            '|' => {
                if self.peek() == '|' {
                    self.advance();
                }
                TokenKind::Or
            }
            '=' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::Equal
                } else {
                    return Err(ParseError::at(
                        "stray `=`; write `==` to compare for equality",
                        line,
                        column,
                    ));
                }
            }
            other => {
                return Err(ParseError::at(
                    format!("unexpected character `{other}`"),
                    line,
                    column,
                ));
            }
        };
        Ok(kind)
    }

    fn maybe_eq(&mut self, with_eq: TokenKind, without: TokenKind) -> TokenKind {
        if self.peek() == '=' {
            self.advance();
            with_eq
        } else {
            without
        }
    }

    fn word(&mut self) -> TokenKind {
        let start = self.pos;
        while !self.at_end() && (self.peek().is_alphanumeric() || self.peek() == '_') {
            self.advance();
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        match text.as_str() {
            "always" | "globally" | "G" => TokenKind::Always,
            "eventually" | "finally" | "F" => TokenKind::Eventually,
            "until" | "U" => TokenKind::Until,
            "next" | "X" => TokenKind::Next,
            "since" | "S" => TokenKind::Since,
            "historically" | "H" => TokenKind::Historically,
            "once" | "O" => TokenKind::Once,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "implies" => TokenKind::Implies,
            "P" => TokenKind::Probability,
            "inf" => TokenKind::Infinity,
            _ => TokenKind::Identifier(text),
        }
    }

    fn number(&mut self, line: usize, column: usize) -> Result<TokenKind, ParseError> {
        let start = self.pos;
        let mut seen_dot = false;
        while !self.at_end() {
            let ch = self.peek();
            if ch.is_ascii_digit() {
                self.advance();
            } else if ch == '.' && !seen_dot {
                seen_dot = true;
                self.advance();
            } else {
                break;
            }
        }
        if matches!(self.peek(), 'e' | 'E') {
            self.advance();
            if matches!(self.peek(), '+' | '-') {
                self.advance();
            }
            if !self.peek().is_ascii_digit() {
                let text: String = self.chars[start..self.pos].iter().collect();
                return Err(ParseError::at(
                    format!("number `{text}` needs a digit after the exponent"),
                    line,
                    column,
                ));
            }
            while !self.at_end() && self.peek().is_ascii_digit() {
                self.advance();
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        match text.parse::<f64>() {
            Ok(value) if value.is_finite() => Ok(TokenKind::Number(value)),
            Ok(_) => Err(ParseError::at(
                format!("number `{text}` is out of range; use `inf` for an unbounded interval"),
                line,
                column,
            )),
            Err(_) => Err(ParseError::at(
                format!("`{text}` is not a valid number"),
                line,
                column,
            )),
        }
    }

    fn skip_trivia(&mut self) {
        while !self.at_end() {
            let ch = self.peek();
            if ch.is_whitespace() {
                self.advance();
            } else if ch == '#' {
                while !self.at_end() && self.peek() != '\n' {
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> char {
        if self.at_end() {
            '\0'
        } else {
            self.chars[self.pos]
        }
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) {
        if self.at_end() {
            return;
        }
        if self.chars[self.pos] == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        self.pos += 1;
    }

    fn at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(input: &str) -> Vec<TokenKind> {
        tokenize(input)
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn lexes_a_simple_predicate() {
        assert_eq!(
            kinds("x < 5"),
            vec![
                TokenKind::Identifier("x".into()),
                TokenKind::Less,
                TokenKind::Number(5.0),
                TokenKind::End,
            ]
        );
    }

    #[test]
    fn capital_and_word_keywords_alias() {
        assert_eq!(kinds("G")[0], TokenKind::Always);
        assert_eq!(kinds("globally")[0], TokenKind::Always);
        assert_eq!(kinds("finally")[0], TokenKind::Eventually);
        assert_eq!(
            kinds("F U X S H O P")[..7],
            [
                TokenKind::Eventually,
                TokenKind::Until,
                TokenKind::Next,
                TokenKind::Since,
                TokenKind::Historically,
                TokenKind::Once,
                TokenKind::Probability,
            ]
        );
    }

    #[test]
    fn symbolic_logical_operators() {
        assert_eq!(
            kinds("& && | || ! != ->"),
            vec![
                TokenKind::And,
                TokenKind::And,
                TokenKind::Or,
                TokenKind::Or,
                TokenKind::Not,
                TokenKind::NotEqual,
                TokenKind::Implies,
                TokenKind::End,
            ]
        );
    }

    #[test]
    fn numbers_handle_dot_prefix_and_exponents() {
        assert_eq!(kinds(".5")[0], TokenKind::Number(0.5));
        assert_eq!(kinds("1e-6")[0], TokenKind::Number(1e-6));
        assert_eq!(kinds("2.5E3")[0], TokenKind::Number(2500.0));
        assert_eq!(kinds("3.14e+2")[0], TokenKind::Number(314.0));
    }

    #[test]
    fn minus_is_its_own_token() {
        assert_eq!(
            kinds("x < -5"),
            vec![
                TokenKind::Identifier("x".into()),
                TokenKind::Less,
                TokenKind::Minus,
                TokenKind::Number(5.0),
                TokenKind::End,
            ]
        );
    }

    #[test]
    fn comments_and_whitespace_are_skipped_with_line_tracking() {
        let tokens = tokenize("always # a note\n[0, inf]").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Always);
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[1].kind, TokenKind::LeftBracket);
        assert_eq!(tokens[1].line, 2);
    }

    #[test]
    fn stray_equals_is_a_helpful_error() {
        let err = tokenize("x = 5").unwrap_err();
        assert!(err.message.contains("=="));
        assert_eq!(err.column, 3);
    }

    #[test]
    fn unexpected_character_reports_its_column() {
        let err = tokenize("x @ 5").unwrap_err();
        assert!(err.message.contains("unexpected character"));
        assert_eq!(err.column, 3);
    }
}