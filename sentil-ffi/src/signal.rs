use crate::conversions::{
    c_char_to_string, clear_error, ffi_panic_boundary, into_string_array, set_error, slice_from,
};
use crate::handles::{drop_handle, into_handle};
use crate::{SentilError, SentilInterpolation};
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
pub extern "C" fn sentil_trace_len(handle: *mut c_void) -> size_t {
    clear_error();
    ffi_panic_boundary(0, || borrow_handle!(handle, Trace, 0).len())
}

#[no_mangle]
pub extern "C" fn sentil_trace_is_empty(handle: *mut c_void) -> bool {
    clear_error();
    ffi_panic_boundary(true, || borrow_handle!(handle, Trace, true).is_empty())
}

#[no_mangle]
pub extern "C" fn sentil_trace_times(handle: *mut c_void, out_len: *mut size_t) -> *const c_double {
    clear_error();
    ffi_panic_boundary(ptr::null(), || {
        check_ptr!(out_len, ptr::null());
        let trace = borrow_handle!(handle, Trace, ptr::null());
        let times = trace.times();
        unsafe { *out_len = times.len() };
        times.as_ptr()
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_variables(
    handle: *mut c_void,
    out_count: *mut size_t,
) -> *mut *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let trace = borrow_handle!(handle, Trace, ptr::null_mut());
        let names = trace.variables().into_iter().map(String::from).collect();
        into_string_array(names, out_count)
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_signal(
    handle: *mut c_void,
    name: *const c_char,
    out_len: *mut size_t,
) -> *const c_double {
    clear_error();
    ffi_panic_boundary(ptr::null(), || {
        check_ptr!(out_len, ptr::null());
        let trace = borrow_handle!(handle, Trace, ptr::null());
        let Ok(name) = c_char_to_string(name) else {
            return ptr::null();
        };
        match trace.signal(&name) {
            Some(values) => {
                unsafe { *out_len = values.len() };
                values.as_ptr()
            }
            None => {
                unsafe { *out_len = 0 };
                set_error(SentilError::UnknownVariable, &format!("trace has no signal named `{name}`"));
                ptr::null()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_resample(
    handle: *mut c_void,
    times: *const c_double,
    n: size_t,
    interp: SentilInterpolation,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let trace = borrow_handle!(handle, Trace, ptr::null_mut());
        let Ok(times) = slice_from(times, n) else {
            return ptr::null_mut();
        };
        match trace.resample(times.to_vec(), interp.into()) {
            Ok(resampled) => into_handle(resampled),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

fn trace_from_text(text: *const c_char, parse: fn(&str) -> sentil::Result<Trace>) -> *mut c_void {
    let Ok(text) = c_char_to_string(text) else {
        return ptr::null_mut();
    };
    match parse(&text) {
        Ok(trace) => into_handle(trace),
        Err(e) => {
            let _: SentilError = e.into();
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn sentil_trace_from_csv(text: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || trace_from_text(text, Trace::from_csv_str))
}

#[no_mangle]
pub extern "C" fn sentil_trace_from_tsv(text: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || trace_from_text(text, Trace::from_tsv_str))
}

#[no_mangle]
pub extern "C" fn sentil_trace_from_path(path: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(path) = c_char_to_string(path) else {
            return ptr::null_mut();
        };
        match Trace::from_path(&path) {
            Ok(trace) => into_handle(trace),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_trace_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<Trace>(handle) });
}