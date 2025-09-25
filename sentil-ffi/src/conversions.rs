use crate::SentilError;
use libc::{c_char, size_t};
use std::cell::RefCell;
use std::ffi::CString;
use std::ptr;

struct ErrorState {
    code: SentilError,
    message: CString,
}

thread_local! {
    static LAST_ERROR: RefCell<ErrorState> = RefCell::new(ErrorState {
        code: SentilError::Ok,
        message: CString::default(),
    });
}

pub(crate) fn set_error(code: SentilError, message: &str) {
    let text = CString::new(message.replace('\0', " ")).unwrap_or_default();
    LAST_ERROR.with(|e| {
        let mut e = e.borrow_mut();
        e.code = code;
        e.message = text;
    });
}

pub(crate) fn last_error_code() -> SentilError {
    LAST_ERROR.with(|e| e.borrow().code)
}

pub(crate) fn last_error_ptr() -> *const c_char {
    LAST_ERROR.with(|e| e.borrow().message.as_ptr())
}

pub(crate) fn clear_error() {
    LAST_ERROR.with(|e| {
        let mut e = e.borrow_mut();
        e.code = SentilError::Ok;
        e.message = CString::default();
    });
}

pub(crate) fn ffi_panic_boundary<T>(default: T, f: impl FnOnce() -> T) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("a panic crossed the C boundary");
            set_error(SentilError::Panic, message);
            default
        }
    }
}

pub(crate) fn last_error_message(buffer: *mut c_char, length: size_t) -> size_t {
    LAST_ERROR.with(|e| {
        let state = e.borrow();
        let bytes = state.message.as_bytes_with_nul();
        let needed = bytes.len();
        if buffer.is_null() || length == 0 {
            return needed;
        }
        let n = needed.min(length);
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), buffer, n);
            *buffer.add(n - 1) = 0;
        }
        needed
    })
}

pub(crate) fn slice_from<'a, T>(ptr: *const T, len: usize) -> Result<&'a [T], SentilError> {
    if len == 0 {
        Ok(&[])
    } else if ptr.is_null() {
        set_error(SentilError::NullPointer, "a required array argument was null");
        Err(SentilError::NullPointer)
    } else {
        Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
    }
}

pub(crate) fn c_char_to_string(ptr: *const c_char) -> Result<String, SentilError> {
    if ptr.is_null() {
        set_error(SentilError::NullPointer, "a required string argument was null");
        return Err(SentilError::NullPointer);
    }
    match unsafe { std::ffi::CStr::from_ptr(ptr) }.to_str() {
        Ok(s) => Ok(s.to_owned()),
        Err(_) => {
            set_error(SentilError::Utf8, "a string argument was not valid UTF-8");
            Err(SentilError::Utf8)
        }
    }
}

pub(crate) fn to_c_string(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => {
            set_error(SentilError::Evaluation, "a returned string held an interior null byte");
            ptr::null_mut()
        }
    }
}

pub(crate) fn into_string_array(items: Vec<String>, out_count: *mut size_t) -> *mut *mut c_char {
    let mut ptrs: Vec<*mut c_char> = Vec::with_capacity(items.len());
    for item in &items {
        match CString::new(item.as_str()) {
            Ok(c) => ptrs.push(c.into_raw()),
            Err(_) => {
                for p in ptrs {
                    unsafe { drop(CString::from_raw(p)) };
                }
                set_error(SentilError::Evaluation, "a name held an interior null byte");
                return ptr::null_mut();
            }
        }
    }
    let len = ptrs.len();
    let raw = Box::into_raw(ptrs.into_boxed_slice());
    unsafe {
        if let Some(c) = out_count.as_mut() {
            *c = len;
        }
    }
    raw.cast::<*mut c_char>()
}

pub(crate) fn code_of(e: &sentil::Error) -> SentilError {
    use sentil::Error as E;
    match e {
        E::Parse(_) => SentilError::Parse,
        E::UnknownVariable { .. } => SentilError::UnknownVariable,
        E::DivisionByZero { .. }
        | E::UnknownFunction { .. }
        | E::ArityMismatch { .. }
        | E::ProbabilisticOperator => SentilError::Evaluation,
        E::NonMonotonicTime { .. }
        | E::NonFiniteSample { .. }
        | E::SignalLengthMismatch { .. }
        | E::EmptyTrace
        | E::PackedLength { .. } => SentilError::Trace,
        E::NotProbabilistic => SentilError::NotProbabilistic,
        E::InvalidNoiseModel { .. } => SentilError::InvalidNoiseModel,
        E::InvalidConfig { .. } => SentilError::InvalidConfig,
        E::Fit { .. } => SentilError::Fit,
        E::Ingest { .. } => SentilError::Ingest,
        E::Splitting { .. } => SentilError::Splitting,
        E::Unsupported { .. } => SentilError::Unsupported,
        _ => SentilError::Evaluation,
    }
}

impl From<sentil::Error> for SentilError {
    fn from(e: sentil::Error) -> Self {
        let code = code_of(&e);
        set_error(code, &e.to_string());
        code
    }
}