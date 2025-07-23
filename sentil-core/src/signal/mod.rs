//! Signals and traces.

#[cfg(feature = "ingest")]
mod ingest;
mod trace;

pub use trace::Trace;