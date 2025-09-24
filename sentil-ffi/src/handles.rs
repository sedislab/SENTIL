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

/// # Safety
/// `handle` is null or a live pointer from `into_handle::<T>`.
pub(crate) unsafe fn mutate_handle<T>(handle: *mut c_void, apply: impl FnOnce(&mut T)) {
    if handle.is_null() {
        set_error(SentilError::NullPointer, "`handle` was null");
        return;
    }
    apply(&mut *handle.cast::<T>());
}

/// Takes ownership of the value behind `handle`, leaving the caller with nothing
/// to free. Returns `None` for a null handle.
///
/// # Safety
/// `handle` must be null or a pointer returned by `into_handle::<T>` that has not
/// already been freed.
pub(crate) unsafe fn take_handle<T>(handle: *mut c_void) -> Option<T> {
    if handle.is_null() {
        None
    } else {
        Some(*Box::from_raw(handle.cast::<T>()))
    }
}