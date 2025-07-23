//! Statistical model checking for the probabilistic operator.

mod confidence;
mod lifting;
mod noise;

pub use confidence::{wilson_interval, z_score, ConfidenceInterval};
pub use lifting::LiftingRegistry;
pub use noise::{NoiseInteraction, NoiseModel};