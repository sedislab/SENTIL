//! A parsed formula handle.

use alloc::boxed::Box;

use sentil::Formula;

#[cfg(feature = "parser")]
use crate::status_of;
use crate::{codec, read_slice, Status};

/// Parses a formula, storing the handle in `*out`. Needs the `parser` feature.
///
/// # Safety
///
/// `formula` must be a null-terminated UTF-8 string and `out` a writable slot.
#[cfg(feature = "parser")]
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_formula_create(
    formula: *const core::ffi::c_char,
    out: *mut *mut Formula,
) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    *out = core::ptr::null_mut();
    if formula.is_null() {
        return Status::NullPointer;
    }
    let Ok(text) = core::ffi::CStr::from_ptr(formula).to_str() else {
        return Status::Parse;
    };
    match Formula::parse(text) {
        Ok(formula) => {
            *out = Box::into_raw(Box::new(formula));
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// Rebuilds a formula from a host-compiled blob, storing the handle in `*out`.
///
/// # Safety
///
/// `bytes` must point to `len` readable bytes and `out` to a writable slot.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_formula_create_compiled(
    bytes: *const u8,
    len: usize,
    out: *mut *mut Formula,
) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    *out = core::ptr::null_mut();
    let Some(blob) = read_slice(bytes, len) else {
        return Status::NullPointer;
    };
    match codec::decode(blob) {
        Ok(formula) => {
            *out = Box::into_raw(Box::new(formula));
            Status::Ok
        }
        Err(_) => Status::Decode,
    }
}

/// Frees a formula handle. A null pointer is a no-op.
///
/// # Safety
///
/// `formula` must be a live handle not already destroyed or consumed.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_formula_destroy(formula: *mut Formula) {
    if !formula.is_null() {
        drop(Box::from_raw(formula));
    }
}

/// The formula's nesting depth.
///
/// # Safety
///
/// `formula` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_formula_depth(formula: *const Formula) -> usize {
    if formula.is_null() {
        0
    } else {
        (*formula).depth()
    }
}

/// Whether the formula carries a temporal operator.
///
/// # Safety
///
/// `formula` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_formula_has_temporal(formula: *const Formula) -> bool {
    !formula.is_null() && (*formula).has_temporal()
}

/// Whether the formula is wrapped in the probabilistic operator. A board cannot
/// decide a probabilistic formula, so this is the check before using one.
///
/// # Safety
///
/// `formula` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_formula_is_probabilistic(formula: *const Formula) -> bool {
    !formula.is_null() && matches!(&*formula, Formula::Probabilistic(..))
}

/// The number of distinct variables the formula references.
///
/// # Safety
///
/// `formula` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_formula_variable_count(formula: *const Formula) -> usize {
    if formula.is_null() {
        0
    } else {
        (*formula).variables().len()
    }
}

/// Copies the variable at `index` into `buf`, returning the length needed
/// including the terminator.
///
/// # Safety
///
/// `formula` is a live handle; `buf`, if non-null, holds `buf_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_formula_variable(
    formula: *const Formula,
    index: usize,
    buf: *mut core::ffi::c_char,
    buf_len: usize,
) -> usize {
    if formula.is_null() {
        return 0;
    }
    let variables = (*formula).variables();
    let Some(name) = variables.get(index) else {
        return 0;
    };
    let bytes = name.as_bytes();
    if !buf.is_null() && buf_len > 0 {
        let copy = core::cmp::min(bytes.len(), buf_len - 1);
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.cast(), copy);
        *buf.add(copy) = 0;
    }
    bytes.len() + 1
}