use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Question {
    FullSignal,
    Monitoring,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Timing {
    pub mean_ms: f64,
    pub std_ms: f64,
    pub min_ms: f64,
    pub p50_ms: f64,
    pub p99_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hardware {
    pub cpu: String,
    pub cores: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub tool: String,
    pub version: String,
    pub language: String,
    pub benchmark: String,
    pub formula: String,
    pub question: Question,
    pub size: u64,
    pub robustness: f64,
    pub timing: Timing,
    pub peak_rss_bytes: Option<u64>,
    pub runs: u64,
    pub hardware: Hardware,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmcRecord {
    pub tool: String,
    pub version: String,
    pub language: String,
    pub device: String,
    pub model: String,
    pub formula: String,
    pub samples: u64,
    pub probability: f64,
    pub ci_lower: f64,
    pub ci_upper: f64,
    pub ground_truth: Option<f64>,
    pub timing: Timing,
    pub throughput_per_s: f64,
    pub runs: u64,
    pub hardware: Hardware,
}