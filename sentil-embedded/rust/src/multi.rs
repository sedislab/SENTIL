//! Watching several properties at once.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use sentil::{MultiFormulaMonitor, Robustness};

use crate::{codec, read_slice, status_of, EmbeddedRobustness, Status};

/// A multi-formula monitor plus the results of its last update.
pub struct EmbeddedMultiMonitor {
    inner: MultiFormulaMonitor,
    last: Vec<(String, Robustness)>,
}

/// Creates an empty multi-formula monitor.
///
/// # Safety
///
/// `out` must point to a writable handle slot.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_multi_create(out: *mut *mut EmbeddedMultiMonitor) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    *out = Box::into_raw(Box::new(EmbeddedMultiMonitor {
        inner: MultiFormulaMonitor::new(),
        last: Vec::new(),
    }));
    Status::Ok
}

/// Adds a formula under `id`, parsing it from text. Needs the `parser` feature.
///
/// # Safety
///
/// `monitor` is a live handle; `id` and `formula` are null-terminated strings.
#[cfg(feature = "parser")]
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_multi_add(
    monitor: *mut EmbeddedMultiMonitor,
    id: *const core::ffi::c_char,
    formula: *const core::ffi::c_char,
) -> Status {
    if monitor.is_null() || id.is_null() || formula.is_null() {
        return Status::NullPointer;
    }
    let (Ok(id), Ok(formula)) = (
        core::ffi::CStr::from_ptr(id).to_str(),
        core::ffi::CStr::from_ptr(formula).to_str(),
    ) else {
        return Status::Parse;
    };
    match (*monitor).inner.add(id, formula) {
        Ok(()) => Status::Ok,
        Err(e) => status_of(&e),
    }
}

/// Adds a formula under `id` from a host-compiled blob.
///
/// # Safety
///
/// `monitor` is a live handle, `id` a null-terminated string, and `bytes` points
/// to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_multi_add_compiled(
    monitor: *mut EmbeddedMultiMonitor,
    id: *const core::ffi::c_char,
    bytes: *const u8,
    len: usize,
) -> Status {
    if monitor.is_null() || id.is_null() {
        return Status::NullPointer;
    }
    let Ok(id) = core::ffi::CStr::from_ptr(id).to_str() else {
        return Status::Parse;
    };
    let Some(blob) = read_slice(bytes, len) else {
        return Status::NullPointer;
    };
    let Ok(formula) = codec::decode(blob) else {
        return Status::Decode;
    };
    match (*monitor).inner.add_formula(id, &formula) {
        Ok(()) => Status::Ok,
        Err(e) => status_of(&e),
    }
}

/// Folds one timestamped sample into every formula, given the variables as
/// parallel `names` and `values` arrays of length `n`.
///
/// # Safety
///
/// `monitor` is a live handle; `names` holds `n` null-terminated strings and
/// `values` `n` doubles.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_multi_update(
    monitor: *mut EmbeddedMultiMonitor,
    time: f64,
    names: *const *const core::ffi::c_char,
    values: *const f64,
    n: usize,
) -> Status {
    if monitor.is_null() {
        return Status::NullPointer;
    }
    let Some(values) = read_slice(values, n) else {
        return Status::NullPointer;
    };
    if n != 0 && names.is_null() {
        return Status::NullPointer;
    }
    let mut pairs: Vec<(&str, f64)> = Vec::with_capacity(n);
    for (i, &value) in values.iter().enumerate() {
        let name_ptr = *names.add(i);
        if name_ptr.is_null() {
            return Status::NullPointer;
        }
        let Ok(name) = core::ffi::CStr::from_ptr(name_ptr).to_str() else {
            return Status::UnknownVariable;
        };
        pairs.push((name, value));
    }
    let monitor = &mut *monitor;
    match monitor.inner.update(time, &pairs) {
        Ok(results) => {
            monitor.last = results;
            Status::Ok
        }
        Err(e) => {
            monitor.last.clear();
            status_of(&e)
        }
    }
}

/// The number of results from the last update.
///
/// # Safety
///
/// `monitor` is a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_multi_count(monitor: *const EmbeddedMultiMonitor) -> usize {
    if monitor.is_null() {
        0
    } else {
        (*monitor).last.len()
    }
}

/// The robustness of the formula at `index` from the last update, written to `out`.
///
/// # Safety
///
/// `monitor` is a live handle and `out` writable.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_multi_result(
    monitor: *const EmbeddedMultiMonitor,
    index: usize,
    out: *mut EmbeddedRobustness,
) -> Status {
    if monitor.is_null() || out.is_null() {
        return Status::NullPointer;
    }
    let monitor = &*monitor;
    match monitor.last.get(index) {
        Some((_, robustness)) => {
            *out = EmbeddedRobustness::from_core(*robustness);
            Status::Ok
        }
        None => Status::InvalidConfig,
    }
}

/// Copies the id of the formula at `index` into `buf`, returning the length
/// needed including the terminator.
///
/// # Safety
///
/// `monitor` is a live handle; `buf`, if non-null, holds `buf_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_multi_id(
    monitor: *const EmbeddedMultiMonitor,
    index: usize,
    buf: *mut core::ffi::c_char,
    buf_len: usize,
) -> usize {
    if monitor.is_null() {
        return 0;
    }
    let monitor = &*monitor;
    let Some((id, _)) = monitor.last.get(index) else {
        return 0;
    };
    let bytes = id.as_bytes();
    if !buf.is_null() && buf_len > 0 {
        let copy = core::cmp::min(bytes.len(), buf_len - 1);
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.cast(), copy);
        *buf.add(copy) = 0;
    }
    bytes.len() + 1
}

/// The number of formulas held.
///
/// # Safety
///
/// `monitor` is a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_multi_len(monitor: *const EmbeddedMultiMonitor) -> usize {
    if monitor.is_null() {
        0
    } else {
        (*monitor).inner.len()
    }
}

/// Removes the formula under `id`, returning whether one was found.
///
/// # Safety
///
/// `monitor` is a live handle and `id` a null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_multi_remove(
    monitor: *mut EmbeddedMultiMonitor,
    id: *const core::ffi::c_char,
) -> bool {
    if monitor.is_null() || id.is_null() {
        return false;
    }
    let Ok(id) = core::ffi::CStr::from_ptr(id).to_str() else {
        return false;
    };
    (*monitor).inner.remove(id)
}

/// Clears all formula state so the monitor can run a fresh stream.
///
/// # Safety
///
/// `monitor` is a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_multi_reset(monitor: *mut EmbeddedMultiMonitor) {
    if !monitor.is_null() {
        (*monitor).inner.reset();
        (*monitor).last.clear();
    }
}

/// Frees a multi-formula monitor. A null pointer is a no-op.
///
/// # Safety
///
/// `monitor` is a live handle that has not been destroyed.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_multi_destroy(monitor: *mut EmbeddedMultiMonitor) {
    if !monitor.is_null() {
        drop(Box::from_raw(monitor));
    }
}