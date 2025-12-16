//! The command-line surface

use clap::{Parser, Subcommand, ValueEnum};
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

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Evaluate a formula's robustness over a recorded trace.
    Check {
        /// The formula to check. Use this or --spec, not both.
        #[arg(short, long, value_name = "FORMULA", required_unless_present = "spec")]
        formula: Option<String>,
        /// A premade specification to check instead of a raw formula.
        #[arg(long, value_name = "NAME", required_unless_present = "formula")]
        spec: Option<String>,
        /// A spec variant to apply.
        #[arg(long, value_name = "NAME")]
        variant: Option<String>,
        /// Override a spec parameter, repeatable, as key=value.
        #[arg(short, long, value_name = "KEY=VALUE")]
        param: Vec<String>,
        /// The trace file, or - for standard input.
        #[arg(short, long, value_name = "FILE")]
        trace: String,
        /// Dense reads between samples; discrete reads only at them.
        #[arg(long, value_name = "MODE", default_value_t = Semantics::Dense)]
        semantics: Semantics,
        /// Where to run the evaluation.
        #[arg(long, value_name = "BACKEND", default_value_t = Backend::Cpu)]
        backend: Backend,
    },

    /// Monitor a live signal by reading one JSON sample per line and emitting a verdict per line. Watch multiple formulas by separating them with `;`.
    #[command(alias = "stream")]
    Monitor {
        /// The formula(s) to monitor. Use this or --spec.
        #[arg(short, long, value_name = "FORMULA", required_unless_present = "spec")]
        formula: Option<String>,
        /// A premade specification to monitor instead of a raw formula.
        #[arg(long, value_name = "NAME", required_unless_present = "formula")]
        spec: Option<String>,
        /// A spec variant to apply.
        #[arg(long, value_name = "NAME")]
        variant: Option<String>,
        /// Override a spec parameter, repeatable, as key=value.
        #[arg(short, long, value_name = "KEY=VALUE")]
        param: Vec<String>,
    },

    /// Estimate how likely a probabilistic specification holds.
    #[command(alias = "prob")]
    Smc {
        /// The estimation algorithm.
        #[arg(long, value_name = "ALGO", default_value_t = Algo::Smc)]
        algo: Algo,
        /// The sample budget. Accepts scientific notation.
        #[arg(long, value_name = "N", default_value = "10000")]
        samples: String,
        /// The confidence level for the reported interval.
        #[arg(long, default_value_t = 0.95)]
        confidence: f64,
        /// Which confidence interval to report.
        #[arg(long, value_name = "METHOD", default_value_t = Interval::Wilson)]
        interval: Interval,
        /// The half-width target the chernoff algorithm sizes the sample count for.
        #[arg(long, default_value_t = 0.05)]
        epsilon: f64,
        /// The base seed, so a run reproduces exactly.
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// The probabilistic formula. Use this or --spec.
        #[arg(short, long, value_name = "FORMULA", required_unless_present = "spec")]
        formula: Option<String>,
        /// A premade specification to check instead of a raw formula.
        #[arg(long, value_name = "NAME", required_unless_present = "formula")]
        spec: Option<String>,
        /// A spec variant to apply.
        #[arg(long, value_name = "NAME")]
        variant: Option<String>,
        /// Override a spec parameter, repeatable, as key=value.
        #[arg(short, long, value_name = "KEY=VALUE")]
        param: Vec<String>,
        /// The base trace to lift into an ensemble, or - for standard input.
        #[arg(short, long, value_name = "FILE")]
        trace: String,
    },

    /// Apply a spec's noise models to a trace, writing the lifted trace as CSV.
    Lift {
        /// The specification whose noise models to apply.
        #[arg(long, value_name = "NAME")]
        spec: String,
        /// A spec variant to apply.
        #[arg(long, value_name = "NAME")]
        variant: Option<String>,
        /// Override a spec parameter, repeatable, as key=value.
        #[arg(short, long, value_name = "KEY=VALUE")]
        param: Vec<String>,
        /// The trace to lift, or - for standard input.
        #[arg(short, long, value_name = "FILE")]
        trace: String,
        /// The base seed, so a lift reproduces exactly.
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },

    /// List the premade specifications, or inspect one in detail.
    Specs {
        /// The specification to inspect, such as `controls/overshoot`. Omit to
        /// list everything.
        name: Option<String>,
        /// List only specifications whose name contains this text.
        #[arg(long, value_name = "TEXT")]
        filter: Option<String>,
    },
}

/// How time between samples is read.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Semantics {
    /// Read the signal continuously, catching an inter-sample crossing.
    Dense,
    /// Read the signal only at the sample points.
    Discrete,
}

impl fmt::Display for Semantics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Dense => "dense",
            Self::Discrete => "discrete",
        })
    }
}

/// The statistical estimation algorithm.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Algo {
    /// Monte Carlo with a confidence interval.
    Smc,
    /// Wald's sequential probability ratio test.
    Sprt,
    /// Monte Carlo with the sample count sized a priori by Chernoff-Hoeffding.
    Chernoff,
    /// Adaptive multilevel splitting, for rare events.
    Ams,
}

impl fmt::Display for Algo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Smc => "smc",
            Self::Sprt => "sprt",
            Self::Chernoff => "chernoff",
            Self::Ams => "ams",
        })
    }
}

/// Which confidence interval to report around an estimate.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interval {
    /// The Wilson score interval, the right default.
    Wilson,
    /// The Clopper-Pearson exact interval, conservative.
    ClopperPearson,
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Wilson => "wilson",
            Self::ClopperPearson => "clopper-pearson",
        })
    }
}

/// Where a computation runs.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    /// The CPU engine.
    Cpu,
    /// The GPU engine, for the simulation-heavy work that supports it.
    Gpu,
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
        })
    }
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