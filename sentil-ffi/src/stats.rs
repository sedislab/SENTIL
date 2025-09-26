use crate::conversions::{
    c_char_to_string, clear_error, ffi_panic_boundary, set_error, slice_from, to_c_string,
};
use crate::handles::{drop_handle, into_boxed_array, into_handle, take_handle};
use crate::{SentilError, SentilIntervalMethod, SentilNoiseInteraction};
use libc::{c_char, c_void, size_t};
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
pub extern "C" fn sentil_noise_weibull(shape: f64, scale: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::weibull(shape, scale)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_rayleigh(scale: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::rayleigh(scale)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_gumbel(location: f64, scale: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::gumbel(location, scale)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_cauchy(location: f64, scale: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::cauchy(location, scale)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_student_t(df: f64, location: f64, scale: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::student_t(df, location, scale)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_truncated_normal(
    mean: f64,
    std_dev: f64,
    lower: f64,
    upper: f64,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        noise_handle(NoiseModel::truncated_normal(mean, std_dev, lower, upper))
    })
}

#[no_mangle]
pub extern "C" fn sentil_noise_poisson(lambda: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::poisson(lambda)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_binomial(n: u64, p: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || noise_handle(NoiseModel::binomial(n, p)))
}

#[no_mangle]
pub extern "C" fn sentil_noise_bootstrap(residuals: *const f64, n: size_t) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(residuals) = slice_from(residuals, n) else {
            return ptr::null_mut();
        };
        noise_handle(NoiseModel::bootstrap(residuals.to_vec()))
    })
}

#[no_mangle]
pub extern "C" fn sentil_noise_mixture(
    weights: *const f64,
    models: *mut *mut c_void,
    n: size_t,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(weights) = slice_from(weights, n) else {
            return ptr::null_mut();
        };
        if n > 0 {
            check_ptr!(models, ptr::null_mut());
        }
        if unsafe { repeated_handle(&[("models", models, n)]) } {
            return ptr::null_mut();
        }
        let mut components: Vec<Option<NoiseModel>> = Vec::with_capacity(n);
        for i in 0..n {
            components.push(unsafe { take_handle::<NoiseModel>(*models.add(i)) });
        }
        if components.iter().any(Option::is_none) {
            set_error(SentilError::NullPointer, "a mixture component was null");
            return ptr::null_mut();
        }
        let components = components.into_iter().flatten().collect();
        noise_handle(NoiseModel::mixture(weights.to_vec(), components))
    })
}

fn noise_moment(handle: *mut c_void, get: fn(&NoiseModel) -> Option<f64>, out: *mut f64) -> bool {
    check_ptr!(out, false);
    match get(borrow_handle!(handle, NoiseModel, false)) {
        Some(v) => {
            unsafe { *out = v };
            true
        }
        None => false,
    }
}

#[no_mangle]
pub extern "C" fn sentil_noise_mean(handle: *mut c_void, out: *mut f64) -> bool {
    clear_error();
    ffi_panic_boundary(false, || noise_moment(handle, NoiseModel::mean, out))
}

#[no_mangle]
pub extern "C" fn sentil_noise_variance(handle: *mut c_void, out: *mut f64) -> bool {
    clear_error();
    ffi_panic_boundary(false, || noise_moment(handle, NoiseModel::variance, out))
}

#[no_mangle]
pub extern "C" fn sentil_noise_residuals(
    ground_truth: *const f64,
    n: size_t,
    sensor: *const f64,
    m: size_t,
    interaction: SentilNoiseInteraction,
    out_len: *mut size_t,
) -> *mut f64 {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_len, ptr::null_mut());
        let Ok(truth) = slice_from(ground_truth, n) else {
            return ptr::null_mut();
        };
        let Ok(sensor) = slice_from(sensor, m) else {
            return ptr::null_mut();
        };
        match NoiseModel::residuals(truth, sensor, interaction.into()) {
            Ok(residuals) => into_boxed_array(residuals, out_len),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_noise_fit_gaussian(samples: *const f64, n: size_t) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(samples) = slice_from(samples, n) else {
            return ptr::null_mut();
        };
        noise_handle(NoiseModel::fit_gaussian(samples))
    })
}

#[no_mangle]
pub extern "C" fn sentil_noise_fit_bootstrap(samples: *const f64, n: size_t) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(samples) = slice_from(samples, n) else {
            return ptr::null_mut();
        };
        noise_handle(NoiseModel::fit_bootstrap(samples))
    })
}

#[no_mangle]
pub extern "C" fn sentil_noise_fit_bootstrap_reservoir(
    samples: *const f64,
    n: size_t,
    max_samples: size_t,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(samples) = slice_from(samples, n) else {
            return ptr::null_mut();
        };
        noise_handle(NoiseModel::fit_bootstrap_reservoir(samples, max_samples))
    })
}

#[no_mangle]
pub extern "C" fn sentil_noise_fit_gaussian_mixture(
    samples: *const f64,
    n: size_t,
    components: size_t,
    max_iters: size_t,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(samples) = slice_from(samples, n) else {
            return ptr::null_mut();
        };
        noise_handle(NoiseModel::fit_gaussian_mixture(samples, components, max_iters))
    })
}

#[no_mangle]
pub extern "C" fn sentil_noise_to_json(handle: *mut c_void) -> *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let model = borrow_handle!(handle, NoiseModel, ptr::null_mut());
        match serde_json::to_string(model) {
            Ok(text) => to_c_string(&text),
            Err(e) => {
                set_error(SentilError::Json, &e.to_string());
                ptr::null_mut()
            }
        }
    })
}

fn noise_from_str(text: &str) -> *mut c_void {
    match serde_json::from_str::<NoiseModel>(text) {
        Ok(model) => into_handle(model),
        Err(e) => {
            set_error(SentilError::Json, &e.to_string());
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn sentil_noise_from_json(json: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(text) = c_char_to_string(json) else {
            return ptr::null_mut();
        };
        noise_from_str(&text)
    })
}

#[no_mangle]
pub extern "C" fn sentil_noise_from_file(path: *const c_char) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(path) = c_char_to_string(path) else {
            return ptr::null_mut();
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => noise_from_str(&text),
            Err(e) => {
                set_error(SentilError::Ingest, &format!("could not read {path}: {e}"));
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_noise_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<NoiseModel>(handle) });
}