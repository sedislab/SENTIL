//! Synthesizing inputs, controllers, and witnesses from a specification.
//!
//! Monitoring asks whether a behaviour satisfies a formula. Synthesis turns that
//! around: given a specification, find a behaviour that satisfies it. The shared
//! foundation is a smooth, differentiable robustness over the same formula tree
//! the monitor uses, so an optimizer can push a candidate uphill toward
//! satisfaction. The open-loop and receding-horizon layers build on it.

mod gradient;
mod model;
mod numerics;
mod pgrad;
mod problem;
mod smooth;

pub use model::{Bounds, LinearModel, SystemModel};
pub use numerics::symmetric_eigen;
pub use pgrad::maximize;
pub use problem::{SynthesisProblem, SynthesisResult, Synthesizer};
pub use smooth::{soft_max, soft_min, SmoothConfig};