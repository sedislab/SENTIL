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

/// Stable status code returned across the C ABI. `Ok` is zero; every other value
/// signals a failure whose human-readable detail is retrievable through
/// `sentil_get_last_error_message`. The integer values are part of the ABI and do
/// not change between releases.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SentilError {
    Ok = 0,
    NullPointer = 1,
    Utf8 = 2,
    Parse = 3,
    UnknownVariable = 4,
    Evaluation = 5,
    Trace = 6,
    NotProbabilistic = 7,
    InvalidNoiseModel = 8,
    InvalidConfig = 9,
    Fit = 10,
    Ingest = 11,
    Splitting = 12,
    Unsupported = 13,
    Transpilation = 14,
    Gpu = 15,
    Json = 16,
    Panic = 17,
}

#[macro_use]
mod macros;
mod conversions;
mod formula;
mod handles;

use conversions::{clear_error, ffi_panic_boundary, last_error_code, last_error_message};
use libc::{c_char, size_t};

/// Returns the status code of the most recent failed call on this thread, or
/// `SENTIL_OK` when the last call succeeded.
#[no_mangle]
pub extern "C" fn sentil_get_last_error_code() -> SentilError {
    last_error_code()
}

/// Copies the most recent error message on this thread into `buffer`, writing at
/// most `length` bytes and always null terminating when `length` is nonzero.
/// Returns the length the message needs including the terminator, so a caller can
/// pass a null buffer to size an allocation and then call again.
#[no_mangle]
pub extern "C" fn sentil_get_last_error_message(buffer: *mut c_char, length: size_t) -> size_t {
    last_error_message(buffer, length)
}

/// Frees a string this library returned. Passing null is a no-op. Do not pass a
/// pointer the library did not return, and do not free the same pointer twice.
#[no_mangle]
pub extern "C" fn sentil_free_string(s: *mut c_char) {
    clear_error();
    ffi_panic_boundary((), || {
        if !s.is_null() {
            unsafe {
                drop(std::ffi::CString::from_raw(s));
            }
        }
    });
}

/// Frees a string array returned by this library. Passing null is a no-op.
#[no_mangle]
pub extern "C" fn sentil_free_string_array(array: *mut *mut c_char, count: size_t) {
    clear_error();
    ffi_panic_boundary((), || unsafe {
        free_boxed_array_owning(array, count, |&string| string)
    });
}

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