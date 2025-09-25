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

macro_rules! borrow_handle {
    ($h:expr, $ty:ty, $ret:expr) => {{
        check_ptr!($h, $ret);
        unsafe { &*$h.cast::<$ty>() }
    }};
}

macro_rules! borrow_handle_mut {
    ($h:expr, $ty:ty, $ret:expr) => {{
        check_ptr!($h, $ret);
        unsafe { &mut *$h.cast::<$ty>() }
    }};
}