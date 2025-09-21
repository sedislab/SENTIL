//! The error boundary. Each export reports failure through a per-thread last
//! error in the style of `errno`: a stable code plus the core's message, which a
//! C caller reads back with the two query functions in `lib.rs`.

use crate::SentilError;
use libc::{c_char, size_t};
use std::cell::RefCell;
use std::ptr;

struct ErrorState {
    code: SentilError,
    message: String,
}

thread_local! {
    static LAST_ERROR: RefCell<ErrorState> = const {
        RefCell::new(ErrorState {
            code: SentilError::Ok,
            message: String::new(),
        })
    };
}

pub(crate) fn set_error(code: SentilError, message: &str) {
    LAST_ERROR.with(|e| {
        let mut e = e.borrow_mut();
        e.code = code;
        e.message.clear();
        e.message.push_str(message);
    });
}

pub(crate) fn last_error_code() -> SentilError {
    LAST_ERROR.with(|e| e.borrow().code)
}

/// Resets the thread's last error. Called at the entry of every export so a
/// successful call leaves `Ok` behind.
pub(crate) fn clear_error() {
    LAST_ERROR.with(|e| {
        let mut e = e.borrow_mut();
        e.code = SentilError::Ok;
        e.message.clear();
    });
}

/// Runs `f`, turning a panic into `SentilError::Panic` and the `default` return
/// rather than letting it unwind across the C boundary, which would be undefined.
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
        let bytes = state.message.as_bytes();
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        let needed = end + 1;
        if buffer.is_null() || length == 0 {
            return needed;
        }
        let n = end.min(length - 1);
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), buffer, n);
            *buffer.add(n) = 0;
        }
        needed
    })
}

impl From<sentil::Error> for SentilError {
    fn from(e: sentil::Error) -> Self {
        use sentil::Error as E;
        let code = match &e {
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
        };
        set_error(code, &e.to_string());
        code
    }
}