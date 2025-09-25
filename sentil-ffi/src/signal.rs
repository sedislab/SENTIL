use crate::conversions::{clear_error, ffi_panic_boundary, slice_from};
use crate::handles::{drop_handle, into_handle};
use crate::SentilError;
use libc::{c_double, c_void, size_t};
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
pub extern "C" fn sentil_trace_indexed(len: size_t) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || into_handle(Trace::indexed(len)))
}

#[no_mangle]
pub extern "C" fn sentil_trace_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<Trace>(handle) });
}