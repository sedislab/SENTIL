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
    #[arg(global = true, short = 'o', long, value_name = "FORMAT", default_value_t = OutputFormat::Text, env = "SENTIL_OUTPUT")]
    pub output: OutputFormat,

    /// A hidden boolean alias for `-o json`.
    #[arg(global = true, long, hide = true)]
    pub json: bool,

    /// When to colorize output.
    #[arg(global = true, long, value_name = "WHEN", default_value_t = ColorWhen::Auto, env = "SENTIL_COLOR")]
    pub color: ColorWhen,

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
    #[command(after_help = "Examples:\n  sentil check -f 'always[0,5] (speed < 30)' -t run.csv\n  sentil check --spec controls/overshoot -t run.csv -o json\n  cat run.csv | sentil check -f 'eventually (x > 2)' -t -")]
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
        /// Bind a formula variable to a dataset column, repeatable, as variable=column.
        #[arg(long, value_name = "VAR=COLUMN")]
        map: Vec<String>,
        /// Dense reads between samples; discrete reads only at them.
        #[arg(long, value_name = "MODE", default_value_t = Semantics::Dense)]
        semantics: Semantics,
        /// Print the robustness at every sample, not just the verdict at t=0.
        #[arg(long)]
        signal: bool,
        /// Print the time intervals where the formula is violated.
        #[arg(long)]
        violations: bool,
        /// Where to run the evaluation.
        #[arg(long, value_name = "BACKEND", default_value_t = Backend::Cpu)]
        backend: Backend,
    },

    /// Monitor a live signal by reading one JSON sample per line and emitting a verdict per line. Watch multiple formulas by separating them with `;`.
    #[command(alias = "stream", after_help = "Examples:\n  sensor | sentil monitor -f 'always (temp < 80)' -o ndjson\n  sentil monitor -f 'x > 0; historically (x > 0)' < samples.ndjson")]
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
        /// Bind a formula variable to an input field, repeatable, as variable=field.
        #[arg(long, value_name = "VAR=FIELD")]
        map: Vec<String>,
    },

    /// Estimate how likely a probabilistic specification holds.
    #[command(alias = "prob", after_help = "Examples:\n  sentil smc -f 'P>=0.95(always[0,10] (x > 0))' -t base.csv --samples 1e5\n  sentil smc --spec controls/overshoot -t base.csv --algo sprt\n  sentil smc -f 'P>=0.9(...)' -t base.csv --algo chernoff --epsilon 0.05")]
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
        /// The half-width of the sprt indifference band around the formula's threshold.
        #[arg(long, default_value_t = 0.05)]
        indifference: f64,
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
        /// A noise model for a signal, repeatable, as signal=distribution:params, e.g. speed=gaussian:0,0.5. Overrides a spec's models.
        #[arg(long, value_name = "SIGNAL=DIST:PARAMS")]
        noise: Vec<String>,
        /// The base trace to lift into an ensemble, or - for standard input.
        #[arg(short, long, value_name = "FILE")]
        trace: String,
        /// Bind a formula variable to a dataset column, repeatable, as variable=column.
        #[arg(long, value_name = "VAR=COLUMN")]
        map: Vec<String>,
    },

    /// Synthesize a control-input sequence that best satisfies a spec on a model.
    #[command(after_help = "Examples:\n  sentil synth -f 'always (x > 0)' --model system.json\n  sentil synth --spec controls/overshoot --model m.json --method cmaes -o json")]
    Synth {
        /// The optimization method.
        #[arg(long, value_name = "METHOD", default_value_t = Method::Gradient)]
        method: Method,
        /// A JSON model file with the fields a, b, x0, variables, dt, horizon, and optional bounds {lower, upper}.
        #[arg(long, value_name = "FILE")]
        model: String,
        /// The spec to satisfy. Use this or --spec.
        #[arg(short, long, value_name = "FORMULA", required_unless_present = "spec")]
        formula: Option<String>,
        /// A premade specification to satisfy instead of a raw formula.
        #[arg(long, value_name = "NAME", required_unless_present = "formula")]
        spec: Option<String>,
        /// A spec variant to apply.
        #[arg(long, value_name = "NAME")]
        variant: Option<String>,
        /// Override a spec parameter, repeatable, as key=value.
        #[arg(short, long, value_name = "KEY=VALUE")]
        param: Vec<String>,
        /// Override the model's horizon.
        #[arg(long, value_name = "N")]
        horizon: Option<usize>,
        /// The optimizer's iteration budget.
        #[arg(long, default_value_t = 200)]
        budget: usize,
    },

    /// Find the tightest value of a spec parameter for which it still holds on a trace.
    #[command(after_help = "Example:\n  sentil mine --spec controls/overshoot --parameter max_overshoot -t run.csv")]
    Mine {
        /// The specification to mine.
        #[arg(long, value_name = "NAME")]
        spec: String,
        /// A spec variant to apply.
        #[arg(long, value_name = "NAME")]
        variant: Option<String>,
        /// Fix another spec parameter, repeatable, as key=value.
        #[arg(short, long, value_name = "KEY=VALUE")]
        param: Vec<String>,
        /// The parameter to mine.
        #[arg(long, value_name = "NAME")]
        parameter: String,
        /// The search range; defaults to the parameter's declared range.
        #[arg(long, value_name = "LO,HI")]
        range: Option<String>,
        /// The trace, or - for standard input.
        #[arg(short, long, value_name = "FILE")]
        trace: String,
        /// Bind a formula variable to a dataset column, repeatable, as variable=column.
        #[arg(long, value_name = "VAR=COLUMN")]
        map: Vec<String>,
    },

    /// Search a model's input space for a trajectory that violates the spec.
    #[command(after_help = "Example:\n  sentil falsify -f 'always (x < 5)' --model system.json")]
    Falsify {
        /// The search method.
        #[arg(long, value_name = "METHOD", default_value_t = Method::CmaEs)]
        method: Method,
        /// A JSON model file with a bounds {lower, upper} block to search within.
        #[arg(long, value_name = "FILE")]
        model: String,
        /// The spec to try to violate. Use this or --spec.
        #[arg(short, long, value_name = "FORMULA", required_unless_present = "spec")]
        formula: Option<String>,
        /// A premade specification to try to violate instead of a raw formula.
        #[arg(long, value_name = "NAME", required_unless_present = "formula")]
        spec: Option<String>,
        /// A spec variant to apply.
        #[arg(long, value_name = "NAME")]
        variant: Option<String>,
        /// Override a spec parameter, repeatable, as key=value.
        #[arg(short, long, value_name = "KEY=VALUE")]
        param: Vec<String>,
        /// Override the model's horizon.
        #[arg(long, value_name = "N")]
        horizon: Option<usize>,
        /// The search iteration budget.
        #[arg(long, default_value_t = 200)]
        budget: usize,
        /// How many times cmaes restarts from a fresh start.
        #[arg(long, default_value_t = 1)]
        restarts: usize,
    },

    /// Apply noise models to a trace, and write the lifted trace as a CSV. Feed the models as a spec or from --noise flags.
    #[command(after_help = "Examples:\n  sentil lift --spec controls/overshoot -t run.csv > lifted.csv\n  sentil lift --noise 'speed=gaussian:0,0.5' -t run.csv > lifted.csv")]
    Lift {
        /// A specification whose noise models to apply.
        #[arg(long, value_name = "NAME")]
        spec: Option<String>,
        /// A spec variant to apply.
        #[arg(long, value_name = "NAME")]
        variant: Option<String>,
        /// Override a spec parameter, repeatable, as key=value.
        #[arg(short, long, value_name = "KEY=VALUE")]
        param: Vec<String>,
        /// A noise model for a signal, repeatable, as signal=distribution:params.
        #[arg(long, value_name = "SIGNAL=DIST:PARAMS")]
        noise: Vec<String>,
        /// The trace to lift, or - for standard input.
        #[arg(short, long, value_name = "FILE")]
        trace: String,
        /// Bind a formula variable to a dataset column, repeatable, as variable=column.
        #[arg(long, value_name = "VAR=COLUMN")]
        map: Vec<String>,
        /// Write this many sampled realizations. If it's above one, a `member` column is added.
        #[arg(long, default_value_t = 1)]
        members: u64,
        /// The base seed, so a lift reproduces exactly.
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },

    /// Build a check interactively, then print and run the equivalent command.
    /// Needs a terminal; in a script use the verbs directly.
    Init,

    /// Show the configuration files that are consulted and the values in effect.
    Config,

    /// Print a shell completion script to stdout.
    Completion {
        /// The shell to generate for.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Print the man page, in roff, to stdout.
    Man,

    /// Explain an operator's robustness semantics, or a verb's output fields.
    Explain {
        /// An operator such as `until`, or with --fields a verb such as `check`.
        /// Omit to list the operators.
        topic: Option<String>,
        /// Describe the JSON fields the named verb emits, not an operator.
        #[arg(long)]
        fields: bool,
    },

    /// An alias from the config file, or an external `sentil-<name>` on the PATH.
    #[command(external_subcommand)]
    // build.rs builds the command for completions and never reads the captured arguments.
    #[allow(dead_code)]
    External(Vec<String>),

    /// List the premade specifications, or inspect one in detail.
    #[command(after_help = "Examples:\n  sentil specs\n  sentil specs --filter aerospace\n  sentil specs controls/overshoot")]
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
    /// Bayesian sequential testing.
    Bayes,
}

impl fmt::Display for Algo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Smc => "smc",
            Self::Sprt => "sprt",
            Self::Chernoff => "chernoff",
            Self::Bayes => "bayes",
        })
    }
}

/// The synthesis optimization method.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// Projected gradient ascent on the smooth robustness, the light default.
    Gradient,
    /// CMA-ES, a black-box search for models gradients do not suit.
    #[value(name = "cmaes", alias = "cma-es")]
    CmaEs,
    /// A complete mixed-integer encoding, for affine models.
    Milp,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Gradient => "gradient",
            Self::CmaEs => "cmaes",
            Self::Milp => "milp",
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