use crate::conversions::set_error;
use crate::SentilError;
use libc::{c_char, c_void, size_t};
use std::ffi::CString;

pub(crate) fn into_handle<T>(value: T) -> *mut c_void {
    Box::into_raw(Box::new(value)).cast()
}

pub(crate) fn into_boxed_array<T>(items: Vec<T>, out_count: *mut size_t) -> *mut T {
    let len = items.len();
    let raw = Box::into_raw(items.into_boxed_slice());
    unsafe {
        if let Some(c) = out_count.as_mut() {
            *c = len;
        }
    }
    raw.cast::<T>()
}

/// # Safety
/// `array` is null or a pointer from `into_boxed_array::<T>` with this `count`.
pub(crate) unsafe fn free_boxed_array<T>(array: *mut T, count: usize) {
    if !array.is_null() {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(array, count)));
    }
}

/// # Safety
/// `array` is null or a pointer from `into_boxed_array::<T>` with this `count`, and
/// `owned` yields each element's `CString::into_raw` pointer exactly once.
pub(crate) unsafe fn free_boxed_array_owning<T>(
    array: *mut T,
    count: usize,
    owned: impl Fn(&T) -> *mut c_char,
) {
    if array.is_null() {
        return;
    }
    let boxed = Box::from_raw(std::ptr::slice_from_raw_parts_mut(array, count));
    for item in &boxed {
        let string = owned(item);
        if !string.is_null() {
            drop(CString::from_raw(string));
        }
    }
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