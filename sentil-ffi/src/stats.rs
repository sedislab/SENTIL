use crate::conversions::{clear_error, ffi_panic_boundary};
use crate::handles::{drop_handle, into_handle};
use crate::{SentilError, SentilIntervalMethod};
use libc::c_void;
use sentil::stats::{
    agresti_coull, chernoff_hoeffding_samples, clopper_pearson, jeffreys_interval, wilson_interval,
    wilson_samples, z_score, ConfidenceInterval, IntervalMethod, NoiseModel,
};
use std::ptr;

fn noise_handle(result: sentil::Result<NoiseModel>) -> *mut c_void {
    match result {
        Ok(model) => into_handle(model),
        Err(e) => {
            let _: SentilError = e.into();
            ptr::null_mut()
        }
    }
}

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

#[no_mangle]
pub extern "C" fn sentil_noise_dirac(value: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::dirac(value)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_gaussian(mean: f64, std_dev: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::gaussian(mean, std_dev)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_uniform(low: f64, high: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::uniform(low, high)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_log_normal(mu: f64, sigma: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::log_normal(mu, sigma)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_exponential(lambda: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::exponential(lambda)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_gamma(shape: f64, scale: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::gamma(shape, scale)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_beta(alpha: f64, beta: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::beta(alpha, beta)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<NoiseModel>(handle) });
}