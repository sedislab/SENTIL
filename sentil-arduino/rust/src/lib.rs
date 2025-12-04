//! The SENTIL streaming monitor for microcontrollers.
//!
//! This crate compiles the deterministic STL streaming monitor from the SENTIL
//! core into a `no_std` static library a sketch links directly. It carries the
//! monitor and nothing else: a microcontroller has no room for statistical model
//! checking, synthesis, or a GPU, so those layers are left out by building the
//! core with no default features.
//!
//! The surface is a small C ABI under the `sentil_embedded_` prefix, declared in
//! `src/Sentil.h`. A sketch creates a monitor from a formula, feeds one sample
//! per loop, and reads back the robustness. Errors come back as a status code,
//! never a fault, because the embedded build aborts on panic and so the boundary
//! checks every input before it reaches the engine.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

extern crate alloc;

/// Writes the library version into the out-pointers. A null pointer is skipped.
///
/// The version tracks the SENTIL release this monitor was built from.
#[no_mangle]
pub extern "C" fn sentil_embedded_version(major: *mut u32, minor: *mut u32, patch: *mut u32) {
    let write = |p: *mut u32, v: u32| {
        if !p.is_null() {
            unsafe { *p = v };
        }
    };
    write(major, 1);
    write(minor, 0);
    write(patch, 0);
}