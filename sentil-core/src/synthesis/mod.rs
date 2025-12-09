//! Synthesizing inputs, controllers, and witnesses from a specification.
//!
//! ```
//! use sentil::{Bounds, Formula, LinearModel, SynthesisProblem, Synthesizer};
//!
//! let model = LinearModel::new(vec![vec![1.0]], vec![vec![1.0]], [0.0], ["pos"], 1.0, 5)?;
//! let spec = Formula::parse("eventually[0, 5](pos > 2)")?;
//! let problem = SynthesisProblem::new(&model, &spec)
//!     .with_bounds(Bounds::new(vec![-1.0; 5], vec![1.0; 5])?);
//! let result = Synthesizer::solve(&problem)?;
//! assert!(result.holds);
//! # Ok::<(), sentil::Error>(())
//! ```

mod autodiff;
mod cbf;
#[cfg(feature = "statistical")]
mod chance;
mod cmaes;
mod controller;
mod convex;
mod gradient;
#[cfg(feature = "std")]
mod milp;
mod mining;
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
#[cfg(feature = "std")]
pub use milp::solve_milp;
pub use mining::mine_tightest_parameter;
pub use model::{AffineForm, Bounds, LinearModel, SystemModel};
pub use numerics::{solve_spd, symmetric_eigen};
pub use pgrad::maximize;
pub use problem::{Backend, SynthesisProblem, SynthesisResult, Synthesizer};
pub use qp::solve_qp;
pub use smooth::{soft_max, soft_min, SmoothConfig, SoftKind};
pub use witness::Witness;