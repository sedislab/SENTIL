use crate::conversions::{clear_error, ffi_panic_boundary};
use crate::{SentilError, SentilIntervalMethod};
use sentil::stats::{
    agresti_coull, chernoff_hoeffding_samples, clopper_pearson, jeffreys_interval, wilson_interval,
    wilson_samples, z_score, ConfidenceInterval, IntervalMethod,
};

/// A confidence interval `[lower, upper]` built for `level`.
#[repr(C)]
pub struct SentilConfidenceInterval {
    pub lower: f64,
    pub upper: f64,
    pub level: f64,
}

const NAN_INTERVAL: SentilConfidenceInterval =
    SentilConfidenceInterval { lower: f64::NAN, upper: f64::NAN, level: f64::NAN };

impl From<ConfidenceInterval> for SentilConfidenceInterval {
    fn from(ci: ConfidenceInterval) -> Self {
        Self { lower: ci.lower, upper: ci.upper, level: ci.level }
    }
}

#[no_mangle]
pub extern "C" fn sentil_wilson_interval(
    successes: u64,
    trials: u64,
    level: f64,
) -> SentilConfidenceInterval {
    clear_error();
    ffi_panic_boundary(NAN_INTERVAL, || wilson_interval(successes, trials, level).into())
}

#[no_mangle]
pub extern "C" fn sentil_clopper_pearson(
    successes: u64,
    trials: u64,
    level: f64,
) -> SentilConfidenceInterval {
    clear_error();
    ffi_panic_boundary(NAN_INTERVAL, || clopper_pearson(successes, trials, level).into())
}

#[no_mangle]
pub extern "C" fn sentil_jeffreys_interval(
    successes: u64,
    trials: u64,
    level: f64,
) -> SentilConfidenceInterval {
    clear_error();
    ffi_panic_boundary(NAN_INTERVAL, || jeffreys_interval(successes, trials, level).into())
}

#[no_mangle]
pub extern "C" fn sentil_agresti_coull(
    successes: u64,
    trials: u64,
    level: f64,
) -> SentilConfidenceInterval {
    clear_error();
    ffi_panic_boundary(NAN_INTERVAL, || agresti_coull(successes, trials, level).into())
}

#[no_mangle]
pub extern "C" fn sentil_interval(
    method: SentilIntervalMethod,
    successes: u64,
    trials: u64,
    level: f64,
) -> SentilConfidenceInterval {
    clear_error();
    ffi_panic_boundary(NAN_INTERVAL, || {
        IntervalMethod::from(method).interval(successes, trials, level).into()
    })
}

#[no_mangle]
pub extern "C" fn sentil_z_score(level: f64) -> f64 {
    clear_error();
    ffi_panic_boundary(f64::NAN, || z_score(level))
}

#[no_mangle]
pub extern "C" fn sentil_chernoff_hoeffding_samples(
    epsilon: f64,
    delta: f64,
    out: *mut u64,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        match chernoff_hoeffding_samples(epsilon, delta) {
            Ok(n) => {
                unsafe { *out = n };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_wilson_samples(epsilon: f64, level: f64, out: *mut u64) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        match wilson_samples(epsilon, level) {
            Ok(n) => {
                unsafe { *out = n };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}