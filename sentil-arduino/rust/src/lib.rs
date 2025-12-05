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

// On a board the engine allocates from a fixed region the sketch hands over, and
// a panic halts the core because there is no unwinder and nowhere to print. A
// host build (the `std` feature, used by the oracle test and the formula
// compiler) keeps the system allocator and panic handler instead.
#[cfg(all(feature = "mcu", not(feature = "std")))]
mod mcu {
    use embedded_alloc::LlffHeap as Heap;

    #[global_allocator]
    static HEAP: Heap = Heap::empty();

    pub(crate) unsafe fn init(start: *mut u8, size: usize) {
        HEAP.init(start as usize, size);
    }
}

#[cfg(all(feature = "mcu", not(feature = "std")))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}

/// Hands the monitor a fixed region of memory to allocate from.
///
/// Call this once from `setup()` with a static buffer, before creating any
/// monitor; the region must stay alive for as long as any monitor does. A zero
/// size or null pointer is ignored so the call cannot fault. A host build has a
/// system allocator already, so there this does nothing.
///
/// # Safety
///
/// `heap` must point to `size` writable bytes that outlive every monitor, and
/// this must run before the first allocation on the board.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_init(heap: *mut u8, size: usize) {
    #[cfg(all(feature = "mcu", not(feature = "std")))]
    {
        if size != 0 && !heap.is_null() {
            mcu::init(heap, size);
        }
    }
    #[cfg(not(all(feature = "mcu", not(feature = "std"))))]
    {
        let _ = (heap, size);
    }
}

/// The outcome of an embedded monitor call.
///
/// Zero is success; every other value names one failure so a sketch can branch
/// on it. The functions that build a monitor return this and write the handle
/// through an out-pointer, so there is no error state to read back later.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The call succeeded.
    Ok = 0,
    /// A required pointer argument was null.
    NullPointer = 1,
    /// The formula text could not be parsed.
    Parse = 2,
    /// An update left out a variable the formula needs.
    UnknownVariable = 3,
    /// An update supplied fewer values than the formula has variables.
    PackedLength = 4,
    /// The formula uses an operator the streaming monitor cannot run online.
    Unsupported = 5,
    /// A compiled formula blob was malformed.
    Decode = 6,
    /// The engine reported a failure that the boundary does not map to a code.
    Internal = 7,
}

/// A short, static, human-readable message for a status code. The pointer is to
/// a string literal that lives for the whole program, so never free it.
#[no_mangle]
pub extern "C" fn sentil_embedded_status_message(status: core::ffi::c_int) -> *const core::ffi::c_char {
    let message: &'static [u8] = match status {
        0 => b"ok\0",
        1 => b"a required pointer was null\0",
        2 => b"could not parse the formula\0",
        3 => b"an update left out a variable the formula needs\0",
        4 => b"fewer values than the formula has variables\0",
        5 => b"the formula uses an operator the streaming monitor cannot run online\0",
        6 => b"the compiled formula is malformed\0",
        _ => b"internal engine error\0",
    };
    message.as_ptr().cast()
}

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