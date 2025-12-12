//! Offline robustness over a buffered trace.

use alloc::boxed::Box;

use sentil::{violation_intervals, Formula, Trace};

use crate::{read_slice, status_of, Status};

/// Builds a trace from `n` timestamps. Add signals before evaluating a formula.
///
/// # Safety
///
/// `times` points to `n` readable doubles and `out` to a writable slot.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_trace_create(
    times: *const f64,
    n: usize,
    out: *mut *mut Trace,
) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    *out = core::ptr::null_mut();
    let Some(times) = read_slice(times, n) else {
        return Status::NullPointer;
    };
    match Trace::new(times.to_vec()) {
        Ok(trace) => {
            *out = Box::into_raw(Box::new(trace));
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// Builds a trace whose timestamps are 0, 1, ..., `len - 1`.
///
/// # Safety
///
/// `out` must point to a writable slot.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_trace_create_indexed(
    len: usize,
    out: *mut *mut Trace,
) -> Status {
    if out.is_null() {
        return Status::NullPointer;
    }
    *out = Box::into_raw(Box::new(Trace::indexed(len)));
    Status::Ok
}

/// Adds a named signal of `n` values, which must match the trace length.
///
/// # Safety
///
/// `trace` is a live handle, `name` a null-terminated string, and `values` points
/// to `n` readable doubles.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_trace_add_signal(
    trace: *mut Trace,
    name: *const core::ffi::c_char,
    values: *const f64,
    n: usize,
) -> Status {
    if trace.is_null() || name.is_null() {
        return Status::NullPointer;
    }
    let Ok(name) = core::ffi::CStr::from_ptr(name).to_str() else {
        return Status::UnknownVariable;
    };
    let Some(values) = read_slice(values, n) else {
        return Status::NullPointer;
    };
    let trace = &mut *trace;
    match trace.add_signal(name, values.to_vec()) {
        Ok(()) => Status::Ok,
        Err(e) => status_of(&e),
    }
}

/// The number of time points in the trace.
///
/// # Safety
///
/// `trace` is a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_trace_len(trace: *const Trace) -> usize {
    if trace.is_null() {
        0
    } else {
        (*trace).len()
    }
}

/// Frees a trace. A null pointer is a no-op.
///
/// # Safety
///
/// `trace` is a live handle not already destroyed.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_trace_destroy(trace: *mut Trace) {
    if !trace.is_null() {
        drop(Box::from_raw(trace));
    }
}

/// The discrete robustness of `formula` over `trace`, written to `out`.
///
/// # Safety
///
/// `formula` and `trace` are live handles and `out` writable.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_robustness(
    formula: *const Formula,
    trace: *const Trace,
    out: *mut f64,
) -> Status {
    if formula.is_null() || trace.is_null() || out.is_null() {
        return Status::NullPointer;
    }
    let (formula, trace) = (&*formula, &*trace);
    match formula.robustness(trace) {
        Ok(value) => {
            *out = value;
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// The dense robustness of `formula` over `trace`, written to `out`.
///
/// # Safety
///
/// `formula` and `trace` are live handles and `out` writable.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_robustness_dense(
    formula: *const Formula,
    trace: *const Trace,
    out: *mut f64,
) -> Status {
    if formula.is_null() || trace.is_null() || out.is_null() {
        return Status::NullPointer;
    }
    let (formula, trace) = (&*formula, &*trace);
    match formula.robustness_dense(trace) {
        Ok(value) => {
            *out = value;
            Status::Ok
        }
        Err(e) => status_of(&e),
    }
}

/// Writes the per-sample robustness signal into `out`, up to `cap` values, and
/// the full length into `written`.
///
/// # Safety
///
/// `formula` and `trace` are live handles; `out`, if non-null, holds `cap` doubles.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_robustness_signal(
    formula: *const Formula,
    trace: *const Trace,
    out: *mut f64,
    cap: usize,
    written: *mut usize,
) -> Status {
    if formula.is_null() || trace.is_null() {
        return Status::NullPointer;
    }
    let (formula, trace) = (&*formula, &*trace);
    let signal = match formula.robustness_signal(trace) {
        Ok(signal) => signal,
        Err(e) => return status_of(&e),
    };
    write_signal(&signal, out, cap, written)
}

/// The dense per-sample robustness signal, written like
/// [`sentil_embedded_robustness_signal`].
///
/// # Safety
///
/// `formula` and `trace` are live handles; `out`, if non-null, holds `cap` doubles.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_robustness_dense_signal(
    formula: *const Formula,
    trace: *const Trace,
    out: *mut f64,
    cap: usize,
    written: *mut usize,
) -> Status {
    if formula.is_null() || trace.is_null() {
        return Status::NullPointer;
    }
    let (formula, trace) = (&*formula, &*trace);
    let signal = match formula.robustness_dense_signal(trace) {
        Ok(signal) => signal,
        Err(e) => return status_of(&e),
    };
    write_signal(&signal, out, cap, written)
}

unsafe fn write_signal(signal: &[f64], out: *mut f64, cap: usize, written: *mut usize) -> Status {
    let n = signal.len().min(cap);
    if n > 0 {
        if out.is_null() {
            return Status::NullPointer;
        }
        core::ptr::copy_nonoverlapping(signal.as_ptr(), out, n);
    }
    if !written.is_null() {
        *written = signal.len();
    }
    Status::Ok
}

/// Writes the intervals where `formula` is violated as parallel `starts` and
/// `ends` arrays up to `cap` intervals, with the full count in `count`.
///
/// # Safety
///
/// `formula` and `trace` are live handles; `starts` and `ends`, if non-null, hold
/// `cap` doubles each.
#[no_mangle]
pub unsafe extern "C" fn sentil_embedded_violation_intervals(
    formula: *const Formula,
    trace: *const Trace,
    starts: *mut f64,
    ends: *mut f64,
    cap: usize,
    count: *mut usize,
) -> Status {
    if formula.is_null() || trace.is_null() {
        return Status::NullPointer;
    }
    let (formula, trace) = (&*formula, &*trace);
    let signal = match formula.robustness_signal(trace) {
        Ok(signal) => signal,
        Err(e) => return status_of(&e),
    };
    let spans = violation_intervals(trace.times(), &signal);
    let n = spans.len().min(cap);
    if n > 0 {
        if starts.is_null() || ends.is_null() {
            return Status::NullPointer;
        }
        for (i, (start, end)) in spans.iter().take(n).enumerate() {
            *starts.add(i) = *start;
            *ends.add(i) = *end;
        }
    }
    if !count.is_null() {
        *count = spans.len();
    }
    Status::Ok
}