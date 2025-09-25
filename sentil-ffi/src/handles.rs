//! Opaque handles: box a value into a `*mut c_void`, then borrow it or take it back.

use libc::c_void;

pub(crate) fn into_handle<T>(value: T) -> *mut c_void {
    Box::into_raw(Box::new(value)).cast()
}

/// # Safety
/// `handle` is null or a live pointer from `into_handle::<T>`.
pub(crate) unsafe fn drop_handle<T>(handle: *mut c_void) {
    if !handle.is_null() {
        drop(Box::from_raw(handle.cast::<T>()));
    }
}

/// # Safety
/// `handle` is null or a live pointer from `into_handle::<T>`.
pub(crate) unsafe fn mutate_handle<T>(handle: *mut c_void, apply: impl FnOnce(&mut T)) {
    if handle.is_null() {
        set_error(SentilError::NullPointer, "`handle` was null");
        return;
    }
    apply(&mut *handle.cast::<T>());
}

/// # Safety
/// `handle` is null or a live pointer from `into_handle::<T>`.
pub(crate) unsafe fn take_handle<T>(handle: *mut c_void) -> Option<T> {
    if handle.is_null() {
        None
    } else {
        Some(*Box::from_raw(handle.cast::<T>()))
    }
}