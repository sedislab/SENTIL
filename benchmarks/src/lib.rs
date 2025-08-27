//! Shared pieces of the SENTIL benchmark suite: the deterministic oracle every
//! tool is measured against, and the helpers the runners share. The runners
//! themselves are separate binaries so each tool emits the same record shape.

pub mod measure;
pub mod oracle;
pub mod schema;