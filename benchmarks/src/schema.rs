//! The one record shape every runner emits, so a result from any tool, in any
//! language, on any machine, lines up with every other for comparison.

use serde::{Deserialize, Serialize};

/// Which question a measurement answers. The two are never mixed in a chart: one
/// is the whole robustness signal, the other is the value at the first sample
/// that an online monitor reports each step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Question {
    /// The robustness at every sample, the whole signal.
    FullSignal,
    /// The robustness at the first sample, the monitoring value now.
    Monitoring,
}

/// Timing summary in milliseconds over the repeated runs of one measurement.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Timing {
    /// Mean wall-clock time per run.
    pub mean_ms: f64,
    /// Sample standard deviation across runs.
    pub std_ms: f64,
    /// Fastest run, the least disturbed by scheduling.
    pub min_ms: f64,
    /// Median run.
    pub p50_ms: f64,
    /// Ninety-ninth percentile, the tail.
    pub p99_ms: f64,
}

/// The machine a measurement ran on, so a number is read in its context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hardware {
    /// The CPU model string.
    pub cpu: String,
    /// Logical cores available.
    pub cores: usize,
}

/// One measurement: a tool running one benchmark, with its timing and footprint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    /// The tool measured, such as `sentil` or `rtamt`.
    pub tool: String,
    /// The tool's version string.
    pub version: String,
    /// The language the runner is written in.
    pub language: String,
    /// The benchmark identifier, such as `deterministic/phi1` or
    /// `scalability/length`.
    pub benchmark: String,
    /// The formula under test.
    pub formula: String,
    /// Which question the timing answers.
    pub question: Question,
    /// The number of samples in the trace, or the input rate for a streaming run.
    pub size: u64,
    /// The robustness the tool computed, recorded so a run doubles as a
    /// correctness check.
    pub robustness: f64,
    /// Timing over the repeated runs.
    pub timing: Timing,
    /// Peak resident memory in bytes, or `None` when not measured in-process.
    pub peak_rss_bytes: Option<u64>,
    /// How many runs the timing summarizes.
    pub runs: u64,
    /// The machine the measurement ran on.
    pub hardware: Hardware,
}