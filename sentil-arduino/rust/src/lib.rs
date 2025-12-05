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

pub mod codec;

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

use sentil::StreamMonitor;

fn status_of(err: &sentil::Error) -> Status {
    use sentil::Error;
    match err {
        Error::Parse(_) => Status::Parse,
        Error::UnknownVariable { .. } => Status::UnknownVariable,
        Error::PackedLength { .. } => Status::PackedLength,
        Error::Unsupported { .. } => Status::Unsupported,
        _ => Status::Internal,
    }
}

/// Builds a streaming monitor from a formula, storing the handle in `*out`.
///
/// On success returns [`Status::Ok`] and writes an owned monitor to `*out`; on
/// failure returns the reason and writes null. Free the handle with
/// [`sentil_embedded_destroy`]. This entry point exists only when the crate is
/// built with the `parser` feature; the smallest boards drop it and load a
/// host-compiled formula instead.
///
/// # Safety
///
/// `formula` must be a null-terminated UTF-8 string and `out` must point to a
/// writable handle slot.
#[cfg(feature = "parser")]
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_create(
    formula: *const core::ffi::c_char,
    out: *mut *mut StreamMonitor,
) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    *out = core::ptr::null_mut();
    if formula.is_null() {
        return Status::NullPointer;
    }
    let text = match core::ffi::CStr::from_ptr(formula).to_str() {
        Ok(text) => text,
        Err(_) => return Status::Parse,
    };
    match StreamMonitor::new(text) {
        Ok(monitor) => {
            *out = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(monitor));
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// Builds a streaming monitor from a host-compiled formula, storing the handle
/// in `*out`. The blob comes from the `sentil-compile-formula` tool, so the
/// smallest boards can monitor without the parser.
///
/// On success returns [`Status::Ok`] and writes the monitor to `*out`; a
/// malformed blob returns [`Status::Decode`] and writes null.
///
/// # Safety
///
/// `bytes` must point to `len` readable bytes and `out` to a writable slot.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_create_compiled(
    bytes: *const u8,
    len: usize,
    out: *mut *mut StreamMonitor,
) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    *out = core::ptr::null_mut();
    let blob = if len == 0 {
        &[][..]
    } else if bytes.is_null() {
        return Status::NullPointer;
    } else {
        core::slice::from_raw_parts(bytes, len)
    };
    let formula = match codec::decode(blob) {
        Ok(formula) => formula,
        Err(_) => return Status::Decode,
    };
    match StreamMonitor::from_formula(&formula) {
        Ok(monitor) => {
            *out = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(monitor));
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// Frees a monitor from a SENTIL create call. A null pointer is a no-op.
///
/// # Safety
///
/// `monitor` must be a live handle from this library that has not already been
/// destroyed.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_destroy(monitor: *mut StreamMonitor) {
    if !monitor.is_null() {
        drop(alloc::boxed::Box::from_raw(monitor));
    }
}

/// The robustness after one sample, laid out for C.
///
/// `value` is the margin (the interval midpoint while a temporal window is still
/// open), `satisfied` is `value >= 0`, and `resolved` says whether the verdict
/// has settled or still depends on samples not yet seen.
#[repr(C)]
pub struct EmbeddedRobustness {
    /// Whether the verdict has settled to a single value.
    pub resolved: bool,
    /// Whether the property holds at this point.
    pub satisfied: bool,
    /// The robustness margin.
    pub value: f64,
    /// The greatest lower bound on the margin.
    pub lower: f64,
    /// The least upper bound on the margin.
    pub upper: f64,
}

impl EmbeddedRobustness {
    fn from_core(r: sentil::Robustness) -> Self {
        Self {
            resolved: r.is_resolved(),
            satisfied: r.is_satisfied(),
            value: r.value(),
            lower: r.lower(),
            upper: r.upper(),
        }
    }
}

/// Folds one timestamped sample into the monitor and writes the robustness.
///
/// `values` holds the variables in the order [`sentil_embedded_symbol_index`]
/// reports, with `n` entries. Times must strictly increase across calls. On
/// success returns [`Status::Ok`] and fills `*out`; on a missing value or a
/// short slice it returns the reason and leaves `*out` untouched.
///
/// # Safety
///
/// `monitor` must be a live handle, `values` must point to `n` doubles, and
/// `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_update(
    monitor: *mut StreamMonitor,
    time: f64,
    values: *const f64,
    n: usize,
    out: *mut EmbeddedRobustness,
) -> Status {
    if monitor.is_null() || out.is_null() || (values.is_null() && n != 0) {
        return Status::NullPointer;
    }
    let monitor = &mut *monitor;
    let slice = core::slice::from_raw_parts(values, n);
    match monitor.update_packed(time, slice) {
        Ok(robustness) => {
            *out = EmbeddedRobustness::from_core(robustness);
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// The number of variables the formula references, which is the length the
/// packed `values` slice of [`sentil_embedded_update`] must reach. Returns zero
/// for a null handle.
///
/// # Safety
///
/// `monitor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_variable_count(monitor: *const StreamMonitor) -> usize {
    if monitor.is_null() {
        return 0;
    }
    (*monitor).variable_count()
}

/// The packed-slice position of a named variable. Writes the index to `*out` and
/// `true` to `*found` when the formula uses the variable, or `false` when it does
/// not. Resolve each name once, then feed the packed slice in that order.
///
/// # Safety
///
/// `monitor` must be a live handle, `name` a null-terminated string, and `out`
/// and `found` writable.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_symbol_index(
    monitor: *const StreamMonitor,
    name: *const core::ffi::c_char,
    out: *mut usize,
    found: *mut bool,
) -> Status {
    if monitor.is_null() || name.is_null() || out.is_null() || found.is_null() {
        return Status::NullPointer;
    }
    let name = match core::ffi::CStr::from_ptr(name).to_str() {
        Ok(name) => name,
        Err(_) => return Status::UnknownVariable,
    };
    match (*monitor).symbol_index(name) {
        Some(index) => {
            *out = index;
            *found = true;
        }
        None => {
            *out = 0;
            *found = false;
        }
    }
    Status::Ok
}

/// Clears the monitor so it can run again from the start of a fresh stream. A
/// null handle is a no-op.
///
/// # Safety
///
/// `monitor` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_reset(monitor: *mut StreamMonitor) {
    if !monitor.is_null() {
        (*monitor).reset();
    }
}