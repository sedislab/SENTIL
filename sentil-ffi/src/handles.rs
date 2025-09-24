//! Opaque handle helpers. A handle is a `*mut c_void` that owns a boxed core
//! value; `into_handle` hands that ownership to the caller and `drop_handle`
//! takes it back.

use libc::c_void;

pub(crate) fn into_handle<T>(value: T) -> *mut c_void {
    Box::into_raw(Box::new(value)).cast()
}

/// # Safety
/// `handle` must be null or a pointer returned by `into_handle::<T>` that has not
/// already been freed.
pub(crate) unsafe fn drop_handle<T>(handle: *mut c_void) {
    if !handle.is_null() {
        drop(Box::from_raw(handle.cast::<T>()));
    }
}