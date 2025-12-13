//! A bank of named formulas evaluated offline over one trace.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use sentil::{FormulaBank, Result, Trace};

use crate::{codec, read_slice, status_of, Status};

/// A formula bank plus the results of its last robustness call.
pub struct EmbeddedBank {
    inner: FormulaBank,
    last: Vec<(String, Result<f64>)>,
}

/// Creates an empty bank.
///
/// # Safety
///
/// `out` must point to a writable handle slot.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_bank_create(out: *mut *mut EmbeddedBank) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    *out = Box::into_raw(Box::new(EmbeddedBank {
        inner: FormulaBank::new(),
        last: Vec::new(),
    }));
    Status::Ok
}

/// Adds a formula under `id`, parsing it from text. Needs the `parser` feature.
///
/// # Safety
///
/// `bank` is a live handle; `id` and `formula` are null-terminated strings.
#[cfg(feature = "parser")]
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_bank_add(
    bank: *mut EmbeddedBank,
    id: *const core::ffi::c_char,
    formula: *const core::ffi::c_char,
) -> Status {
    if bank.is_null() || id.is_null() || formula.is_null() {
        return Status::NullPointer;
    }
    let (Ok(id), Ok(formula)) = (
        core::ffi::CStr::from_ptr(id).to_str(),
        core::ffi::CStr::from_ptr(formula).to_str(),
    ) else {
        return Status::Parse;
    };
    let bank = &mut *bank;
    match bank.inner.add(id, formula) {
        Ok(()) => Status::Ok,
        Err(e) => status_of(&e),
    }
}

/// Adds a formula under `id` from a host-compiled blob.
///
/// # Safety
///
/// `bank` is a live handle, `id` a null-terminated string, and `bytes` points to
/// `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_bank_add_compiled(
    bank: *mut EmbeddedBank,
    id: *const core::ffi::c_char,
    bytes: *const u8,
    len: usize,
) -> Status {
    if bank.is_null() || id.is_null() {
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
    let bank = &mut *bank;
    bank.inner.add_formula(id, &formula);
    Status::Ok
}

/// Evaluates every formula's discrete robustness over `trace`.
///
/// # Safety
///
/// `bank` and `trace` are live handles.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_bank_robustness(
    bank: *mut EmbeddedBank,
    trace: *const Trace,
) -> Status {
    if bank.is_null() || trace.is_null() {
        return Status::NullPointer;
    }
    let (bank, trace) = (&mut *bank, &*trace);
    bank.last = bank.inner.robustness(trace);
    Status::Ok
}

/// Evaluates every formula's dense robustness over `trace`.
///
/// # Safety
///
/// `bank` and `trace` are live handles.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_bank_robustness_dense(
    bank: *mut EmbeddedBank,
    trace: *const Trace,
) -> Status {
    if bank.is_null() || trace.is_null() {
        return Status::NullPointer;
    }
    let (bank, trace) = (&mut *bank, &*trace);
    bank.last = bank.inner.robustness_dense(trace);
    Status::Ok
}

/// The number of results from the last robustness call.
///
/// # Safety
///
/// `bank` is a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_bank_count(bank: *const EmbeddedBank) -> usize {
    if bank.is_null() {
        0
    } else {
        (*bank).last.len()
    }
}

/// The robustness of the formula at `index` from the last call, written to `out`. If
/// that formula could not be evaluated, returns its error status instead.
///
/// # Safety
///
/// `bank` is a live handle and `out` writable.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_bank_result(
    bank: *const EmbeddedBank,
    index: usize,
    out: *mut f64,
) -> Status {
    if bank.is_null() || out.is_null() {
        return Status::NullPointer;
    }
    let bank = &*bank;
    match bank.last.get(index) {
        Some((_, Ok(value))) => {
            *out = *value;
            Status::Ok
        }
        Some((_, Err(e))) => status_of(e),
        None => Status::InvalidConfig,
    }
}

/// Copies the id of the formula at `index` into `buf`, returning the length
/// needed including the terminator.
///
/// # Safety
///
/// `bank` is a live handle; `buf`, if non-null, holds `buf_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_bank_id(
    bank: *const EmbeddedBank,
    index: usize,
    buf: *mut core::ffi::c_char,
    buf_len: usize,
) -> usize {
    if bank.is_null() {
        return 0;
    }
    let bank = &*bank;
    let Some((id, _)) = bank.last.get(index) else {
        return 0;
    };
    crate::copy_into(id.as_bytes(), buf, buf_len)
}

/// The number of formulas held.
///
/// # Safety
///
/// `bank` is a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_bank_len(bank: *const EmbeddedBank) -> usize {
    if bank.is_null() {
        0
    } else {
        (*bank).inner.len()
    }
}

/// Frees a bank. A null pointer is a no-op.
///
/// # Safety
///
/// `bank` is a live handle not already destroyed.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_bank_destroy(bank: *mut EmbeddedBank) {
    if !bank.is_null() {
        drop(Box::from_raw(bank));
    }
}