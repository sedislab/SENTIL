//! The SENTIL streaming monitor for microcontrollers.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

extern crate alloc;

pub mod bank;
pub mod codec;
pub mod formula;
pub mod multi;
pub mod offline;
pub mod ring_buffer;
#[cfg(feature = "synthesis")]
pub mod synthesis;
#[cfg(test)]
mod abi_tests;

#[cfg(all(feature = "mcu", not(feature = "std")))]
mod mcu {
    use embedded_alloc::LlffHeap as Heap;

    // Links the architecture crate's critical-section provider for the allocator.
    #[cfg(target_arch = "arm")]
    use cortex_m as _;
    #[cfg(target_arch = "riscv32")]
    use riscv as _;

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
/// # Safety
///
/// `heap` must point to `size` writable bytes that outlive every monitor.
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
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    /// A synthesis input was malformed.
    InvalidConfig = 8,
}

/// A short static message for a status code. Never free it.
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
        8 => b"a synthesis input was malformed: shape, dimension, or bounds\0",
        _ => b"internal engine error\0",
    };
    message.as_ptr().cast()
}

/// Writes the library version into the out-pointers. A null pointer is skipped.
#[no_mangle]
pub extern "C" fn sentil_embedded_version(major: *mut u32, minor: *mut u32, patch: *mut u32) {
    let write = |p: *mut u32, v: u32| {
        if !p.is_null() {
            unsafe { *p = v };
        }
    };
    write(major, env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap_or(0));
    write(minor, env!("CARGO_PKG_VERSION_MINOR").parse().unwrap_or(0));
    write(patch, env!("CARGO_PKG_VERSION_PATCH").parse().unwrap_or(0));
}

use sentil::StreamMonitor;

pub(crate) fn status_of(err: &sentil::Error) -> Status {
    use sentil::Error;
    match err {
        Error::Parse(_) => Status::Parse,
        Error::UnknownVariable { .. } => Status::UnknownVariable,
        Error::PackedLength { .. } => Status::PackedLength,
        Error::Unsupported { .. } => Status::Unsupported,
        Error::InvalidConfig { .. } => Status::InvalidConfig,
        _ => Status::Internal,
    }
}

/// Borrows `n` values from a C pointer, empty when `n` is zero.
///
/// # Safety
///
/// `ptr` must point to `n` readable, aligned `T` values when `n` is nonzero.
pub(crate) unsafe fn read_slice<'a, T>(ptr: *const T, n: usize) -> Option<&'a [T]> {
    if n == 0 {
        Some(&[])
    } else if ptr.is_null() {
        None
    } else {
        Some(core::slice::from_raw_parts(ptr, n))
    }
}

/// Copies `bytes` into `buf` null-terminated, returning the length needed
/// including the terminator.
///
/// # Safety
///
/// `buf`, when non-null, must point to `buf_len` writable bytes.
pub(crate) unsafe fn copy_into(bytes: &[u8], buf: *mut core::ffi::c_char, buf_len: usize) -> usize {
    if !buf.is_null() && buf_len > 0 {
        let copy = bytes.len().min(buf_len - 1);
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.cast(), copy);
        *buf.add(copy) = 0;
    }
    bytes.len() + 1
}

/// Builds a streaming monitor from a formula, storing the handle in `*out`.
///
/// # Safety
///
/// `formula` must be a null-terminated UTF-8 string and `out` a writable slot.
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
/// in `*out`.
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
    let Some(blob) = read_slice(bytes, len) else {
        return Status::NullPointer;
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
/// `monitor` must be a live handle from this library that has not been destroyed.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_destroy(monitor: *mut StreamMonitor) {
    if !monitor.is_null() {
        drop(alloc::boxed::Box::from_raw(monitor));
    }
}

/// The robustness after one sample, laid out for C.
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
    pub(crate) fn from_core(r: sentil::Robustness) -> Self {
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
/// # Safety
///
/// `monitor` must be a live handle, `values` points to `n` doubles, and `out`
/// is writable.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_update(
    monitor: *mut StreamMonitor,
    time: f64,
    values: *const f64,
    n: usize,
    out: *mut EmbeddedRobustness,
) -> Status {
    if monitor.is_null() || out.is_null() {
        return Status::NullPointer;
    }
    let Some(slice) = read_slice(values, n) else {
        return Status::NullPointer;
    };
    let monitor = &mut *monitor;
    match monitor.update_packed(time, slice) {
        Ok(robustness) => {
            *out = EmbeddedRobustness::from_core(robustness);
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// The number of variables the formula references.
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

/// The packed-slice position of a named variable.
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

/// Clears the monitor so it can run a fresh stream. A null handle is a no-op.
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