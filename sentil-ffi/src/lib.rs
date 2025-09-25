//! C-ABI for SENTIL, a runtime verification engine for STL and PrSTL.
#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "each export null-checks its pointers and the header states the contract; marking every C entry point unsafe adds noise a C caller cannot observe"
)]

/// Status code returned across the C ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SentilError {
    Ok = 0,
    NullPointer = 1,
    Utf8 = 2,
    Parse = 3,
    UnknownVariable = 4,
    Evaluation = 5,
    Trace = 6,
    NotProbabilistic = 7,
    InvalidNoiseModel = 8,
    InvalidConfig = 9,
    Fit = 10,
    Ingest = 11,
    Splitting = 12,
    Unsupported = 13,
    Transpilation = 14,
    Gpu = 15,
    Json = 16,
    Panic = 17,
}

#[macro_use]
mod macros;
mod conversions;
mod formula;
mod handles;
mod signal;

use conversions::{
    clear_error, ffi_panic_boundary, last_error_code, last_error_message, last_error_ptr,
};
use libc::{c_char, size_t};

#[no_mangle]
pub extern "C" fn sentil_get_last_error_code() -> SentilError {
    last_error_code()
}

#[no_mangle]
pub extern "C" fn sentil_get_last_error() -> *const c_char {
    last_error_ptr()
}

#[no_mangle]
pub extern "C" fn sentil_get_last_error_message(buffer: *mut c_char, length: size_t) -> size_t {
    last_error_message(buffer, length)
}

#[no_mangle]
pub extern "C" fn sentil_free_string(s: *mut c_char) {
    clear_error();
    ffi_panic_boundary((), || {
        if !s.is_null() {
            unsafe {
                drop(std::ffi::CString::from_raw(s));
            }
        }
    });
}

#[no_mangle]
pub extern "C" fn sentil_free_string_array(array: *mut *mut c_char, count: size_t) {
    clear_error();
    ffi_panic_boundary((), || unsafe {
        free_boxed_array_owning(array, count, |&string| string)
    });
}

/// Comparison operator in a predicate.
#[repr(C)]
#[derive(Clone, Copy)]
pub enum SentilComparisonOp {
    Less = 0,
    LessEqual = 1,
    Greater = 2,
    GreaterEqual = 3,
    Equal = 4,
    NotEqual = 5,
}

impl From<SentilComparisonOp> for sentil::formula::ComparisonOp {
    fn from(op: SentilComparisonOp) -> Self {
        use sentil::formula::ComparisonOp as C;
        match op {
            SentilComparisonOp::Less => C::Less,
            SentilComparisonOp::LessEqual => C::LessEqual,
            SentilComparisonOp::Greater => C::Greater,
            SentilComparisonOp::GreaterEqual => C::GreaterEqual,
            SentilComparisonOp::Equal => C::Equal,
            SentilComparisonOp::NotEqual => C::NotEqual,
        }
    }
}

/// Arithmetic operator in an expression.
#[repr(C)]
#[derive(Clone, Copy)]
pub enum SentilBinaryOp {
    Add = 0,
    Sub = 1,
    Mul = 2,
    Div = 3,
    Mod = 4,
    Pow = 5,
}

impl From<SentilBinaryOp> for sentil::formula::BinaryOp {
    fn from(op: SentilBinaryOp) -> Self {
        use sentil::formula::BinaryOp as B;
        match op {
            SentilBinaryOp::Add => B::Add,
            SentilBinaryOp::Sub => B::Sub,
            SentilBinaryOp::Mul => B::Mul,
            SentilBinaryOp::Div => B::Div,
            SentilBinaryOp::Mod => B::Mod,
            SentilBinaryOp::Pow => B::Pow,
        }
    }
}

/// Threshold direction of the probabilistic operator.
#[repr(C)]
#[derive(Clone, Copy)]
pub enum SentilProbabilityOp {
    GreaterEqual = 0,
    Greater = 1,
    LessEqual = 2,
    Less = 3,
}

impl From<SentilProbabilityOp> for sentil::formula::ProbabilityOp {
    fn from(op: SentilProbabilityOp) -> Self {
        use sentil::formula::ProbabilityOp as P;
        match op {
            SentilProbabilityOp::GreaterEqual => P::GreaterEqual,
            SentilProbabilityOp::Greater => P::Greater,
            SentilProbabilityOp::LessEqual => P::LessEqual,
            SentilProbabilityOp::Less => P::Less,
        }
    }
}

const VERSION_MAJOR: u32 = 1;
const VERSION_MINOR: u32 = 0;
const VERSION_PATCH: u32 = 0;

#[no_mangle]
pub extern "C" fn sentil_version(major: *mut u32, minor: *mut u32, patch: *mut u32) {
    unsafe {
        if let Some(p) = major.as_mut() {
            *p = VERSION_MAJOR;
        }
        if let Some(p) = minor.as_mut() {
            *p = VERSION_MINOR;
        }
        if let Some(p) = patch.as_mut() {
            *p = VERSION_PATCH;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_matches_manifest() {
        let v = format!(
            "{}.{}.{}",
            super::VERSION_MAJOR, super::VERSION_MINOR, super::VERSION_PATCH
        );
        assert_eq!(v, env!("CARGO_PKG_VERSION"));
    }
}