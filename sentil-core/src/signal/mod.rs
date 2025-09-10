//! Signals and traces.

mod buffer;
#[cfg(feature = "ingest")]
mod ingest;
mod interpolation;
mod trace;

pub use buffer::RingBuffer;
pub use interpolation::Interpolation;
pub use trace::{PreparedTrace, Trace};