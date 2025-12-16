//! Errors
//!
//! We use [`miette::Diagnostic`] for all error messages.

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// The exit codes, surfaced to the user by `sentil help exit-codes`. The verdict
/// codes (0 and 10) report a successful run; the rest report a failure and follow
/// the sysexits convention so they read the same as other Unix tools.
pub mod code {
    pub const SUCCESS: u8 = 0;
    /// The run succeeded and the specification did not hold.
    pub const VIOLATED: u8 = 10;
    /// Clap emits this itself.
    pub const USAGE: u8 = 2;
    pub const DATA_ERR: u8 = exitcode::DATAERR as u8;
    /// A file was named but could not be opened.
    pub const NO_INPUT: u8 = exitcode::NOINPUT as u8;
    /// A requested backend this build or this machine cannot provide.
    pub const UNAVAILABLE: u8 = exitcode::UNAVAILABLE as u8;
    pub const INTERNAL: u8 = exitcode::SOFTWARE as u8;
}

#[derive(Debug, Error, Diagnostic)]
pub enum CliError {
    #[error("could not parse the formula")]
    #[diagnostic(code(sentil::parse))]
    Parse {
        #[source_code]
        src: String,
        #[label("{label}")]
        span: SourceSpan,
        label: String,
        #[help]
        help: Option<String>,
    },

    /// A bad trace row, a non-numeric value, a parameter that is not `key=value`.
    #[error("{0}")]
    #[diagnostic(code(sentil::input))]
    Input(String, #[help] Option<String>),

    #[error("cannot read {path}")]
    #[diagnostic(code(sentil::not_found), help("check the path, or pass - to read from stdin"))]
    NotFound { path: String },

    #[error("{0}")]
    #[diagnostic(code(sentil::backend))]
    Backend(String, #[help] Option<String>),

    #[error("{0}")]
    #[diagnostic(code(sentil::engine))]
    Engine(String),

    /// A bug
    #[error("{0}")]
    #[diagnostic(code(sentil::internal))]
    Internal(String),
}

impl CliError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Parse { .. } | Self::Input(..) | Self::Engine(_) => code::DATA_ERR,
            Self::NotFound { .. } => code::NO_INPUT,
            Self::Backend(..) => code::UNAVAILABLE,
            Self::Internal(_) => code::INTERNAL,
        }
    }
}

/// Translates the parser's one-based line and column into a byte span
pub fn span_at(src: &str, line: usize, column: usize) -> SourceSpan {
    let mut offset = 0usize;
    for (i, text) in src.lines().enumerate() {
        if i + 1 == line {
            offset += column.saturating_sub(1);
            break;
        }
        offset += text.len() + 1;
    }
    let (start, width) = match src.char_indices().next_back() {
        Some((last, c)) if offset > last => (last, c.len_utf8()),
        _ => (offset, 1),
    };
    SourceSpan::new(start.into(), width)
}

pub type Run = Result<u8, CliError>;