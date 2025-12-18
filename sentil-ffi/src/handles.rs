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

/// # Safety
/// Each `(name, handles, n)` has `n` readable pointers at `handles`, or a null
/// `handles`.
pub(crate) unsafe fn repeated_handle(arrays: &[(&str, *mut *mut c_void, usize)]) -> bool {
    let entries = || {
        arrays.iter().filter(|&&(_, handles, _)| !handles.is_null()).flat_map(
            |&(name, handles, n)| (0..n).map(move |i| (name, i, unsafe { *handles.add(i) })),
        )
    };
    for (k, (name, i, handle)) in entries().enumerate() {
        if handle.is_null() {
            continue;
        }
        if let Some((other, j, _)) = entries().skip(k + 1).find(|&(_, _, h)| h == handle) {
            set_error(
                SentilError::InvalidConfig,
                &format!(
                    "`{name}[{i}]` and `{other}[{j}]` are the same handle, and each position is \
                     consumed separately; pass one handle per position. Nothing was consumed \
                     here, so you still own every handle"
                ),
            );
            return true;
        }
    }
    false
}

/// # Safety
/// Each non-null entry is a live pointer from `into_handle::<T>`.
#[cfg(feature = "gpu")]
pub(crate) unsafe fn take_handle_array<T>(
    what: &str,
    handles: *mut *mut c_void,
    n: usize,
) -> Option<Vec<T>> {
    if n > 0 && handles.is_null() {
        set_error(SentilError::NullPointer, &format!("`{what}` was null but its count is {n}"));
        return None;
    }
    if repeated_handle(&[(what, handles, n)]) {
        return None;
    }
    let mut taken: Vec<Option<T>> = Vec::with_capacity(n);
    for i in 0..n {
        taken.push(take_handle::<T>(*handles.add(i)));
    }
    match taken.iter().position(Option::is_none) {
        Some(i) => {
            set_error(SentilError::NullPointer, &format!("`{what}[{i}]` was null"));
            None
        }
        None => Some(taken.into_iter().flatten().collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversions::{last_error_code, last_error_ptr};
    use std::ffi::CStr;

    fn message() -> String {
        unsafe { CStr::from_ptr(last_error_ptr()) }.to_string_lossy().into_owned()
    }

    #[test]
    fn a_handle_repeated_in_one_array_is_named_by_position() {
        let first = into_handle(1_u32);
        let second = into_handle(2_u32);
        let mut models = [first, second, first];
        assert!(unsafe { repeated_handle(&[("models", models.as_mut_ptr(), 3)]) });
        assert_eq!(last_error_code(), SentilError::InvalidConfig);
        assert!(message().contains("`models[0]` and `models[2]`"), "{}", message());
        unsafe {
            drop_handle::<u32>(first);
            drop_handle::<u32>(second);
        }
    }

    #[test]
    fn a_handle_shared_by_two_arrays_names_both() {
        let shared = into_handle(3_u32);
        let other = into_handle(4_u32);
        let mut init = [shared];
        let mut advance = [other, shared];
        assert!(unsafe {
            repeated_handle(&[("init", init.as_mut_ptr(), 1), ("advance", advance.as_mut_ptr(), 2)])
        });
        assert!(message().contains("`init[0]` and `advance[1]`"), "{}", message());
        unsafe {
            drop_handle::<u32>(shared);
            drop_handle::<u32>(other);
        }
    }

    #[test]
    fn nulls_and_distinct_handles_pass_the_scan() {
        let first = into_handle(5_u32);
        let second = into_handle(6_u32);
        let mut models = [first, std::ptr::null_mut(), std::ptr::null_mut(), second];
        assert!(!unsafe { repeated_handle(&[("models", models.as_mut_ptr(), 4)]) });
        assert!(!unsafe { repeated_handle(&[("noise", std::ptr::null_mut(), 4)]) });
        unsafe {
            drop_handle::<u32>(first);
            drop_handle::<u32>(second);
        }
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn a_repeat_leaves_every_handle_live_for_the_caller() {
        let handle = into_handle(String::from("live"));
        let mut init = [handle, handle];
        assert!(unsafe { take_handle_array::<String>("init", init.as_mut_ptr(), 2) }.is_none());
        assert_eq!(last_error_code(), SentilError::InvalidConfig);
        assert_eq!(unsafe { take_handle::<String>(handle) }.as_deref(), Some("live"));
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn a_null_entry_is_reported_with_its_index() {
        let mut advance = [into_handle(7_u64), std::ptr::null_mut()];
        assert!(unsafe { take_handle_array::<u64>("advance", advance.as_mut_ptr(), 2) }.is_none());
        assert_eq!(last_error_code(), SentilError::NullPointer);
        assert!(message().contains("`advance[1]`"), "{}", message());
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn distinct_entries_are_taken_in_order() {
        let mut init = [into_handle(1_u8), into_handle(2_u8)];
        let taken = unsafe { take_handle_array::<u8>("init", init.as_mut_ptr(), 2) };
        assert_eq!(taken, Some(vec![1, 2]));
    }
}