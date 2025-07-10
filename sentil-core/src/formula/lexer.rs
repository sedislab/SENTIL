//! Tokenizer for formula strings.

use crate::error::ParseError;

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
    pub(crate) fn describe(&self) -> String {
        match self {
            TokenKind::Identifier(s) => format!("variable `{s}`"),
            TokenKind::Number(n) => format!("number {n}"),
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
}