use crate::conversions::{c_char_to_string, clear_error, ffi_panic_boundary};
use crate::handles::{drop_handle, into_handle};
use crate::SentilError;
use libc::{c_char, c_void};
use sentil::Formula;
use std::ptr;

/// Parses a PrSTL formula from a null-terminated UTF-8 string. Returns a handle
/// the caller owns and frees with `sentil_formula_destroy`, or null on a parse
/// error whose message names the line and column.
#[no_mangle]
pub extern "C" fn sentil_formula_parse(input: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(text) = c_char_to_string(input) else {
            return ptr::null_mut();
        };
        match Formula::parse(&text) {
            Ok(formula) => into_handle(formula),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

/// Frees a formula handle. Passing null is a no-op.
#[no_mangle]
pub extern "C" fn sentil_formula_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<Formula>(handle) });
}