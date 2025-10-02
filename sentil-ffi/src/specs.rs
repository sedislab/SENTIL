use crate::conversions::{
    c_char_to_string, clear_error, ffi_panic_boundary, into_string_array, set_error, to_c_string,
};
use crate::handles::{drop_handle, into_handle, take_handle};
use crate::SentilError;
use libc::{c_char, c_void, size_t};
use sentil::spec_builder::{SpecBuilder, SpecRegistry};
use std::ptr;

#[no_mangle]
pub extern "C" fn sentil_spec_registry_available(out_count: *mut size_t) -> *mut *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        into_string_array(SpecRegistry::global().available(), out_count)
    })
}

#[no_mangle]
pub extern "C" fn sentil_spec_builder_create(name: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(name) = c_char_to_string(name) else {
            return ptr::null_mut();
        };
        match SpecRegistry::global().builder(&name) {
            Ok(builder) => into_handle(builder),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_spec_builder_from_file(path: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(path) = c_char_to_string(path) else {
            return ptr::null_mut();
        };
        match SpecRegistry::global().load_file(&path) {
            Ok(template) => into_handle(SpecBuilder::new(template)),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_spec_builder_with_variant(
    handle: *mut c_void,
    variant: *const c_char,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Some(builder) = (unsafe { take_handle::<SpecBuilder>(handle) }) else {
            set_error(SentilError::NullPointer, "the builder handle was null");
            return ptr::null_mut();
        };
        let Ok(variant) = c_char_to_string(variant) else {
            return ptr::null_mut();
        };
        match builder.with_variant(&variant) {
            Ok(builder) => into_handle(builder),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_spec_builder_with_param(
    handle: *mut c_void,
    name: *const c_char,
    value: f64,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Some(builder) = (unsafe { take_handle::<SpecBuilder>(handle) }) else {
            set_error(SentilError::NullPointer, "the builder handle was null");
            return ptr::null_mut();
        };
        let Ok(name) = c_char_to_string(name) else {
            return ptr::null_mut();
        };
        match builder.with_param(&name, value) {
            Ok(builder) => into_handle(builder),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_spec_builder_available_variants(
    handle: *mut c_void,
    out_count: *mut size_t,
) -> *mut *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let builder = borrow_handle!(handle, SpecBuilder, ptr::null_mut());
        into_string_array(builder.available_variants().into_iter().map(String::from).collect(), out_count)
    })
}

#[no_mangle]
pub extern "C" fn sentil_spec_builder_build_deterministic(handle: *mut c_void) -> *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let builder = borrow_handle!(handle, SpecBuilder, ptr::null_mut());
        match builder.build_deterministic() {
            Ok(text) => to_c_string(&text),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_spec_builder_build_probabilistic(handle: *mut c_void) -> *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let builder = borrow_handle!(handle, SpecBuilder, ptr::null_mut());
        match builder.build_probabilistic() {
            Ok(text) => to_c_string(&text),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_spec_builder_build_formula(handle: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let builder = borrow_handle!(handle, SpecBuilder, ptr::null_mut());
        match builder.build_formula() {
            Ok(formula) => into_handle(formula),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_spec_builder_build_probabilistic_formula(handle: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let builder = borrow_handle!(handle, SpecBuilder, ptr::null_mut());
        match builder.build_probabilistic_formula() {
            Ok(formula) => into_handle(formula),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_spec_builder_build_lifting_registry(handle: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let builder = borrow_handle!(handle, SpecBuilder, ptr::null_mut());
        match builder.build_lifting_registry() {
            Ok(registry) => into_handle(registry),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_spec_builder_parameters_json(handle: *mut c_void) -> *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let builder = borrow_handle!(handle, SpecBuilder, ptr::null_mut());
        match serde_json::to_string(&builder.parameters()) {
            Ok(text) => to_c_string(&text),
            Err(e) => {
                set_error(SentilError::Json, &e.to_string());
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_spec_builder_into_monitor(handle: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Some(builder) = (unsafe { take_handle::<SpecBuilder>(handle) }) else {
            set_error(SentilError::NullPointer, "the builder handle was null");
            return ptr::null_mut();
        };
        match builder.into_monitor() {
            Ok(monitor) => into_handle(monitor),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_spec_builder_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<SpecBuilder>(handle) });
}