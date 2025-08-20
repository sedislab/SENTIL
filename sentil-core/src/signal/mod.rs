//! Signals and traces.

mod buffer;
#[cfg(feature = "ingest")]
mod ingest;
mod trace;

pub use buffer::RingBuffer;
pub use trace::Trace;