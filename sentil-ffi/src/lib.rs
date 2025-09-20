//! Stable C ABI for SENTIL, a runtime verification engine for Signal Temporal
//! Logic and its probabilistic extension PrSTL.
//!
//! The functions here present the Rust crate `sentil` behind a flat C interface:
//! opaque handles, an errno-style last error held per thread, and a panic
//! boundary on every call so a fault becomes an error code rather than aborting
//! the host process. The companion header `include/sentil.h` documents the
//! surface for C and C++ callers.
#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "each export null-checks its pointers and the header states the contract; marking every C entry point unsafe adds noise a C caller cannot observe"
)]

const VERSION_MAJOR: u32 = 1;
const VERSION_MINOR: u32 = 0;
const VERSION_PATCH: u32 = 0;

/// Writes the library version into the out-parameters. Null pointers are skipped.
#[no_mangle]
pub extern "C" fn sentil_version(major: *mut u32, minor: *mut u32, patch: *mut u32) {
    unsafe {
        if let Some(p) = major.as_mut() {
            *p = VERSION_MAJOR;
        }
        if let Some(p) = minor.as_mut() {
            *p = VERSION_MINOR;
        }
        if let Some(p) = patch.as_mut() {
            *p = VERSION_PATCH;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_matches_manifest() {
        let v = format!(
            "{}.{}.{}",
            super::VERSION_MAJOR, super::VERSION_MINOR, super::VERSION_PATCH
        );
        assert_eq!(v, env!("CARGO_PKG_VERSION"));
    }
}