//! Robustness semantics: turning a formula and a signal trace into a margin.

mod discrete;
mod eval;
mod robustness;
mod window;

pub use robustness::Robustness;