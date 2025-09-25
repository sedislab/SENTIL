use crate::conversions::{c_char_to_string, clear_error, ffi_panic_boundary, set_error};
use crate::handles::{drop_handle, into_handle, take_handle};
use crate::{SentilError, SentilTimeMode};
use libc::{c_char, c_void};
use sentil::{Formula, Monitor, MonitorConfig};
use std::ptr;

fn config_or_default(config: *mut c_void) -> MonitorConfig {
    if config.is_null() {
        MonitorConfig::default()
    } else {
        unsafe { (*config.cast::<MonitorConfig>()).clone() }
    }
}

#[no_mangle]
pub extern "C" fn sentil_monitor_config_create() -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || into_handle(MonitorConfig::new()))
}

#[no_mangle]
pub extern "C" fn sentil_monitor_config_set_time(
    handle: *mut c_void,
    mode: SentilTimeMode,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        let config = borrow_handle_mut!(handle, MonitorConfig, SentilError::NullPointer);
        *config = std::mem::take(config).time(mode.into());
        SentilError::Ok
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_config_time_mode(handle: *mut c_void) -> SentilTimeMode {
    clear_error();
    ffi_panic_boundary(SentilTimeMode::Discrete, || {
        borrow_handle!(handle, MonitorConfig, SentilTimeMode::Discrete).time_mode().into()
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_config_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<MonitorConfig>(handle) });
}

#[no_mangle]
pub extern "C" fn sentil_monitor_create(formula: *mut c_void, config: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Some(formula) = (unsafe { take_handle::<Formula>(formula) }) else {
            set_error(SentilError::NullPointer, "the formula handle was null");
            return ptr::null_mut();
        };
        into_handle(Monitor::from_formula(formula, config_or_default(config)))
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_parse(formula: *const c_char, config: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(text) = c_char_to_string(formula) else {
            return ptr::null_mut();
        };
        match Monitor::new(&text, config_or_default(config)) {
            Ok(monitor) => into_handle(monitor),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_formula(handle: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let monitor = borrow_handle!(handle, Monitor, ptr::null_mut());
        into_handle(monitor.formula().clone())
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_config(handle: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let monitor = borrow_handle!(handle, Monitor, ptr::null_mut());
        into_handle(monitor.config().clone())
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<Monitor>(handle) });
}