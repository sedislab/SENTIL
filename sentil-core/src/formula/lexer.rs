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