//! SENTIL is a runtime verification engine for Signal Temporal Logic (STL) and
//! its probabilistic extension, PrSTL.
//!
//! It answers one question: does a system's behaviour, recorded as a set of
//! timed signals, satisfy a temporal specification, and by how much. The "how
//! much" is the quantitative robustness: a positive margin means the property
//! holds with room to spare, a negative one says how badly it fails.
//!
//! The library grows from a fast deterministic STL monitor into probabilistic
//! verification over noisy or stochastic systems. This module tree is being
//! built up from that core outward.
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::doc_markdown,
    reason = "domain acronyms like STL, PrSTL, SMC, and SPRT are prose, not code identifiers"
)]
#![allow(
    clippy::must_use_candidate,
    reason = "the public API is value-returning by design; #[must_use] is reserved for values that are genuinely unsafe to ignore"
)]
#![allow(
    clippy::similar_names,
    reason = "the temporal logic uses conventional short names like phi, psi, and rho that are intentionally close"
)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
pub(crate) mod prelude {
    pub(crate) use alloc::borrow::ToOwned;
    pub(crate) use alloc::boxed::Box;
    pub(crate) use alloc::collections::{BTreeMap, VecDeque};
    pub(crate) use alloc::string::String;
    pub(crate) use alloc::sync::Arc;
    pub(crate) use alloc::vec::Vec;
    pub(crate) use alloc::{format, vec};
    pub(crate) use num_traits::Float;
}

pub mod error;
mod expr;
pub mod formula;
#[cfg(feature = "gpu")]
pub mod gpu;
pub mod monitor;
pub mod semantics;
pub mod signal;
#[cfg(feature = "specs")]
pub mod spec_builder;
#[cfg(feature = "statistical")]
pub mod stats;
#[cfg(feature = "synthesis")]
pub mod synthesis;

pub use error::{Error, ParseError, Result};
pub use formula::Formula;
#[cfg(feature = "gpu")]
pub use gpu::GpuSplittingEstimate;
pub use monitor::{Monitor, MonitorConfig, TimeMode};
pub use semantics::{violation_intervals, MultiFormulaMonitor, Robustness, StreamMonitor};
pub use signal::{RingBuffer, Trace};
#[cfg(feature = "specs")]
pub use spec_builder::{SpecBuilder, SpecRegistry, SpecTemplate};
#[cfg(feature = "statistical")]
pub use stats::{
    BayesConfig, BayesResult, ConfidenceInterval, LiftingRegistry, NoiseInteraction, NoiseModel,
    RareEventConfig, RareEventResult, SmcConfig, SmcResult, SprtConfig, SprtResult,
    StochasticSystem,
};
#[cfg(feature = "gpu")]
pub use stats::{SimExpr, SimModel};
#[cfg(feature = "synthesis")]
pub use synthesis::{
    mine_tightest_parameter, Backend, Bounds, CmaConfig, Controller, LinearModel, SafetyFilter,
    SmoothConfig, SoftKind, SynthesisProblem, SynthesisResult, Synthesizer, SystemModel, Witness,
};
#[cfg(all(feature = "synthesis", feature = "statistical"))]
pub use synthesis::{ChanceConstraint, ChanceReport};