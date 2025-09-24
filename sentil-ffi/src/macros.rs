//! Small macros every export shares: null-check a pointer, or null-check and
//! borrow a handle, recording `NullPointer` and returning the given sentinel on
//! failure.

/// Returns `$ret` after recording a null-pointer error when `$ptr` is null.
macro_rules! check_ptr {
    ($ptr:expr, $ret:expr) => {
        if $ptr.is_null() {
            $crate::conversions::set_error(
                $crate::SentilError::NullPointer,
                concat!("`", stringify!($ptr), "` was null"),
            );
            return $ret;
        }
    };
}

/// Null-checks `$h` and borrows it as `&$ty`, returning `$ret` on null.
macro_rules! borrow_handle {
    ($h:expr, $ty:ty, $ret:expr) => {{
        check_ptr!($h, $ret);
        unsafe { &*$h.cast::<$ty>() }
    }};
}