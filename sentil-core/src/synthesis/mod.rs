//! Synthesizing inputs, controllers, and witnesses from a specification.
//!
//! Monitoring asks whether a behaviour satisfies a formula. Synthesis turns that
//! around: given a specification, find a behaviour that satisfies it. Everything
//! here rests on a smooth, differentiable robustness over the same formula tree the
//! monitor uses, so an optimizer can push a candidate uphill toward satisfaction.
//!
//! The layers, lightest first. Open-loop [`SynthesisProblem`] and [`Synthesizer`]
//! find an input sequence for a [`SystemModel`] offline, by gradient ascent or
//! [`cma_es`]; an infeasible spec yields the least-violating input rather than
//! nothing. The receding-horizon [`Controller`] runs the same search online,
//! re-planning from the live state within a deadline. The [`SafetyFilter`] shields a
//! nominal controller, overriding it as little as a control barrier allows.
//! [`Formula::find_counterexample`](crate::Formula::find_counterexample) searches for
//! a violating trajectory, and (with the `statistical` feature) a `ChanceConstraint`
//! checks a probabilistic guarantee against a stochastic system.
//!
//! ```
//! use sentil::{Bounds, Formula, LinearModel, SynthesisProblem, Synthesizer};
//!
//! let model = LinearModel::new(vec![vec![1.0]], vec![vec![1.0]], [0.0], ["pos"], 1.0, 5)?;
//! let spec = Formula::parse("eventually[0, 5](pos > 2)")?;
//! let problem = SynthesisProblem::new(&model, &spec)
//!     .with_bounds(Bounds::new(vec![-1.0; 5], vec![1.0; 5])?);
//! let result = Synthesizer::solve(&problem)?;
//! assert!(result.satisfies);
//! # Ok::<(), sentil::Error>(())
//! ```

mod cbf;
#[cfg(feature = "statistical")]
mod chance;
mod cmaes;
mod controller;
mod gradient;
mod model;
mod numerics;
mod pgrad;
mod problem;
mod qp;
mod smooth;
#[cfg(feature = "synthesis-gpu")]
mod synth_gpu;
mod witness;

pub use cbf::SafetyFilter;
#[cfg(feature = "statistical")]
pub use chance::{ChanceConstraint, ChanceReport};
pub use cmaes::{cma_es, cma_es_batched, CmaConfig};
pub use controller::Controller;
pub use model::{Bounds, LinearModel, SystemModel};
pub use numerics::{solve_spd, symmetric_eigen};
pub use pgrad::maximize;
pub use problem::{Backend, SynthesisProblem, SynthesisResult, Synthesizer};
pub use qp::solve_qp;
pub use smooth::{soft_max, soft_min, SmoothConfig, SoftKind};
pub use witness::Witness;