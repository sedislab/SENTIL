use crate::conversions::{c_char_to_string, clear_error, ffi_panic_boundary, slice_from};
use crate::handles::{drop_handle, into_handle};
use crate::SentilError;
use libc::{c_char, c_double, c_void, size_t};
use sentil::Trace;
use std::ptr;

#[no_mangle]
pub extern "C" fn sentil_trace_create(times: *const c_double, n: size_t) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(times) = slice_from(times, n) else {
            return ptr::null_mut();
        };
        match Trace::new(times.to_vec()) {
            Ok(trace) => into_handle(trace),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_from_signal(
    times: *const c_double,
    n: size_t,
    name: *const c_char,
    values: *const c_double,
    m: size_t,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(times) = slice_from(times, n) else {
            return ptr::null_mut();
        };
        let Ok(name) = c_char_to_string(name) else {
            return ptr::null_mut();
        };
        let Ok(values) = slice_from(values, m) else {
            return ptr::null_mut();
        };
        match Trace::from_signal(times.to_vec(), &name, values.to_vec()) {
            Ok(trace) => into_handle(trace),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_indexed(len: size_t) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || into_handle(Trace::indexed(len)))
}

#[no_mangle]
pub extern "C" fn sentil_trace_add_signal(
    handle: *mut c_void,
    name: *const c_char,
    values: *const c_double,
    n: size_t,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        let trace = borrow_handle_mut!(handle, Trace, SentilError::NullPointer);
        let name = match c_char_to_string(name) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let values = match slice_from(values, n) {
            Ok(v) => v,
            Err(code) => return code,
        };
        match trace.add_signal(&name, values.to_vec()) {
            Ok(()) => SentilError::Ok,
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<Trace>(handle) });
}