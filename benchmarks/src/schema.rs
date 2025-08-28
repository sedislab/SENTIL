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