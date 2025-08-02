//! Synthesizing inputs, controllers, and witnesses from a specification.
//!
//! Monitoring asks whether a behaviour satisfies a formula. Synthesis turns that
//! around: given a specification, find a behaviour that satisfies it. The shared
//! foundation is a smooth, differentiable robustness over the same formula tree
//! the monitor uses, so an optimizer can push a candidate uphill toward
//! satisfaction. The open-loop and receding-horizon layers build on it.

mod model;
mod smooth;

pub use model::Bounds;
pub use smooth::{soft_max, soft_min, SmoothConfig};