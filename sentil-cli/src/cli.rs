//! The command-line surface

use clap::{Parser, ValueEnum};
use std::fmt;

#[derive(Parser, Debug)]
#[command(
    name = "sentil",
    version,
    about = "Monitor, check, and synthesize against STL and PrSTL specifications",
    propagate_version = true
)]
pub struct Cli {
    /// Output format.
    #[arg(global = true, short = 'o', long, value_name = "FORMAT", default_value_t = OutputFormat::Text)]
    pub output: OutputFormat,

    /// A hidden boolean alias for `-o json`.
    #[arg(global = true, long, hide = true)]
    pub json: bool,

    /// When to colorize output.
    #[arg(global = true, long, value_name = "WHEN", default_value_t = ColorWhen::Auto)]
    pub color: ColorWhen,

    /// Increase logging on stderr; repeat for more.
    #[arg(global = true, short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Print only results and errors.
    #[arg(global = true, short = 'q', long)]
    pub quiet: bool,

    /// Read configuration from this file instead of the discovered locations.
    #[arg(global = true, long, value_name = "FILE")]
    pub config: Option<String>,

    #[arg(global = true, long)]
    pub no_input: bool,
}

/// The machine and human output modes.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    /// A compact, colorized, human-readable rendering.
    Text,
    /// A single self-describing JSON object.
    Json,
    /// One JSON object per line, for streaming.
    Ndjson,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Ndjson => "ndjson",
        })
    }
}

/// The color policy.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorWhen {
    /// Color a terminal, plain text a pipe.
    Auto,
    /// Always color, even through a pipe.
    Always,
    /// Never color.
    Never,
}

impl fmt::Display for ColorWhen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        })
    }
}