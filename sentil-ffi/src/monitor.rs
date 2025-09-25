use crate::conversions::{clear_error, ffi_panic_boundary};
use crate::handles::{drop_handle, into_handle};
use crate::{SentilError, SentilTimeMode};
use libc::c_void;
use sentil::MonitorConfig;
use std::ptr;

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