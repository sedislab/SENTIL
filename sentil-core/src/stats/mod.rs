//! Statistical model checking for the probabilistic operator.

mod confidence;

pub use confidence::{wilson_interval, z_score, ConfidenceInterval};