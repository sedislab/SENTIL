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