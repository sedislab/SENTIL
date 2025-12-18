use crate::conversions::{
    c_char_to_string, clear_error, collect_strings, ffi_panic_boundary, into_string_array,
    set_error, slice_from, to_c_string,
};
use crate::handles::{drop_handle, into_boxed_array, into_handle, repeated_handle, take_handle};
use crate::{
    SentilBayesVerdict, SentilError, SentilIntervalMethod, SentilNoiseInteraction, SentilSprtVerdict,
};
use libc::{c_char, c_void, size_t};
use sentil::stats::{
    adaptive_multilevel_splitting, agresti_coull, bayes_sequential_test, chernoff_hoeffding_samples,
    clopper_pearson, jeffreys_interval, sequential_test, wilson_interval, wilson_samples, z_score,
    BayesConfig, BayesResult, ConfidenceInterval, IntervalMethod, LiftingRegistry, NoiseModel,
    RareEventConfig, RareEventEstimate, RareEventResult, RareEventSimulator, RobustnessDistribution,
    SmcConfig, SmcResult, SprtConfig, SprtResult, StochasticSystem,
};
use sentil::{Formula, Monitor, Trace};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::ptr;
#[cfg(feature = "gpu")]
use crate::handles::take_handle_array;
#[cfg(feature = "gpu")]
use sentil::stats::{SimExpr, SimModel};

/// Callbacks defining a stochastic system.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SentilSystemCallbacks {
    pub userdata: *mut c_void,
    pub init: Option<unsafe extern "C" fn(*mut c_void, u64, *mut f64, size_t)>,
    pub step: Option<unsafe extern "C" fn(*mut c_void, *const f64, size_t, f64, u64, *mut f64)>,
}

fn rng_from(seed: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(seed)
}

/// Bayesian SMC settings.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SentilBayesConfig {
    pub threshold: f64,
    pub bayes_factor: f64,
    pub max_samples: u64,
    pub seed: u64,
}

impl SentilBayesConfig {
    fn to_core(self) -> sentil::Result<BayesConfig> {
        Ok(BayesConfig::new(self.threshold, self.bayes_factor, self.max_samples)?
            .with_seed(self.seed))
    }
}

/// The result of a Bayesian test.
#[repr(C)]
pub struct SentilBayesResult {
    pub verdict: SentilBayesVerdict,
    pub samples: u64,
    pub posterior: f64,
}

impl From<BayesResult> for SentilBayesResult {
    fn from(r: BayesResult) -> Self {
        match r {
            BayesResult::Holds { samples, posterior } => {
                Self { verdict: SentilBayesVerdict::Holds, samples, posterior }
            }
            BayesResult::Fails { samples, posterior } => {
                Self { verdict: SentilBayesVerdict::Fails, samples, posterior }
            }
            BayesResult::Inconclusive { samples, posterior } => {
                Self { verdict: SentilBayesVerdict::Inconclusive, samples, posterior }
            }
        }
    }
}

/// SPRT settings.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SentilSprtConfig {
    pub p0: f64,
    pub p1: f64,
    pub alpha: f64,
    pub beta: f64,
    pub max_samples: u64,
    pub seed: u64,
}

impl SentilSprtConfig {
    fn to_core(self) -> sentil::Result<SprtConfig> {
        Ok(SprtConfig::new(self.p0, self.p1, self.alpha, self.beta, self.max_samples)?
            .with_seed(self.seed))
    }
}

/// The result of a sequential test.
#[repr(C)]
pub struct SentilSprtResult {
    pub verdict: SentilSprtVerdict,
    pub samples: u64,
    pub log_likelihood: f64,
}

impl From<SprtResult> for SentilSprtResult {
    fn from(r: SprtResult) -> Self {
        match r {
            SprtResult::AcceptH0 { samples } => {
                Self { verdict: SentilSprtVerdict::AcceptH0, samples, log_likelihood: 0.0 }
            }
            SprtResult::AcceptH1 { samples } => {
                Self { verdict: SentilSprtVerdict::AcceptH1, samples, log_likelihood: 0.0 }
            }
            SprtResult::Inconclusive { samples, log_likelihood } => {
                Self { verdict: SentilSprtVerdict::Inconclusive, samples, log_likelihood }
            }
        }
    }
}

/// Spread of robustness across the sampled ensemble.
#[repr(C)]
pub struct SentilRobustnessDistribution {
    pub count: u64,
    pub mean: f64,
    pub variance: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
}

impl From<RobustnessDistribution> for SentilRobustnessDistribution {
    fn from(d: RobustnessDistribution) -> Self {
        Self {
            count: d.count,
            mean: d.mean,
            variance: d.variance,
            std_dev: d.std_dev(),
            min: d.min,
            max: d.max,
        }
    }
}

/// Monte Carlo settings.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SentilSmcConfig {
    pub samples: u64,
    pub confidence: f64,
    pub seed: u64,
    pub interval_method: SentilIntervalMethod,
}

impl From<SentilSmcConfig> for SmcConfig {
    fn from(c: SentilSmcConfig) -> Self {
        SmcConfig {
            samples: c.samples,
            confidence: c.confidence,
            seed: c.seed,
            interval_method: c.interval_method.into(),
        }
    }
}

/// The outcome of a statistical check.
#[repr(C)]
pub struct SentilSmcResult {
    pub probability: f64,
    pub interval: SentilConfidenceInterval,
    pub satisfactions: u64,
    pub samples: u64,
    pub holds: bool,
}

impl From<SmcResult> for SentilSmcResult {
    fn from(r: SmcResult) -> Self {
        Self {
            probability: r.probability,
            interval: r.interval.into(),
            satisfactions: r.satisfactions,
            samples: r.samples,
            holds: r.holds,
        }
    }
}

#[no_mangle]
pub extern "C" fn sentil_smc_config_default() -> SentilSmcConfig {
    let d = SmcConfig::default();
    SentilSmcConfig {
        samples: d.samples,
        confidence: d.confidence,
        seed: d.seed,
        interval_method: d.interval_method.into(),
    }
}

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
#[no_mangle]
pub extern "C" fn sentil_lifting_registry_create() -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || into_handle(LiftingRegistry::new()))
}

#[no_mangle]
pub extern "C" fn sentil_lifting_registry_register(
    handle: *mut c_void,
    variable: *const c_char,
    model: *mut c_void,
    interaction: SentilNoiseInteraction,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        let registry = borrow_handle_mut!(handle, LiftingRegistry, SentilError::NullPointer);
        let variable = match c_char_to_string(variable) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let Some(model) = (unsafe { take_handle::<NoiseModel>(model) }) else {
            set_error(SentilError::NullPointer, "the noise model handle was null");
            return SentilError::NullPointer;
        };
        registry.register(&variable, model, interaction.into());
        SentilError::Ok
    })
}

#[no_mangle]
pub extern "C" fn sentil_lifting_registry_variables(
    handle: *mut c_void,
    out_count: *mut size_t,
) -> *mut *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let registry = borrow_handle!(handle, LiftingRegistry, ptr::null_mut());
        into_string_array(registry.variables().into_iter().map(String::from).collect(), out_count)
    })
}

#[no_mangle]
pub extern "C" fn sentil_lifting_registry_is_empty(handle: *mut c_void) -> bool {
    clear_error();
    ffi_panic_boundary(true, || borrow_handle!(handle, LiftingRegistry, true).is_empty())
}

#[no_mangle]
pub extern "C" fn sentil_lifting_registry_lift(
    handle: *mut c_void,
    trace: *mut c_void,
    seed: u64,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let registry = borrow_handle!(handle, LiftingRegistry, ptr::null_mut());
        let trace = borrow_handle!(trace, Trace, ptr::null_mut());
        match registry.lift(trace, seed) {
            Ok(lifted) => into_handle(lifted),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_lifting_registry_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<LiftingRegistry>(handle) });
}

fn run_check(
    formula: *mut c_void,
    trace: *mut c_void,
    lifting: *mut c_void,
    config: *const SentilSmcConfig,
    out: *mut SentilSmcResult,
    conservative: bool,
) -> SentilError {
    check_ptr!(config, SentilError::NullPointer);
    check_ptr!(out, SentilError::NullPointer);
    let formula = borrow_handle!(formula, Formula, SentilError::NullPointer);
    let trace = borrow_handle!(trace, Trace, SentilError::NullPointer);
    let lifting = borrow_handle!(lifting, LiftingRegistry, SentilError::NullPointer);
    let config: SmcConfig = unsafe { *config }.into();
    let result = if conservative {
        formula.check_conservative(trace, lifting, &config)
    } else {
        formula.check(trace, lifting, &config)
    };
    match result {
        Ok(result) => {
            unsafe { *out = result.into() };
            SentilError::Ok
        }
        Err(e) => e.into(),
    }
}

#[no_mangle]
pub extern "C" fn sentil_formula_check(
    formula: *mut c_void,
    trace: *mut c_void,
    lifting: *mut c_void,
    config: *const SentilSmcConfig,
    out: *mut SentilSmcResult,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        run_check(formula, trace, lifting, config, out, false)
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_check_conservative(
    formula: *mut c_void,
    trace: *mut c_void,
    lifting: *mut c_void,
    config: *const SentilSmcConfig,
    out: *mut SentilSmcResult,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        run_check(formula, trace, lifting, config, out, true)
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_check_distribution(
    formula: *mut c_void,
    trace: *mut c_void,
    lifting: *mut c_void,
    config: *const SentilSmcConfig,
    out_result: *mut SentilSmcResult,
    out_distribution: *mut SentilRobustnessDistribution,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(config, SentilError::NullPointer);
        check_ptr!(out_result, SentilError::NullPointer);
        check_ptr!(out_distribution, SentilError::NullPointer);
        let formula = borrow_handle!(formula, Formula, SentilError::NullPointer);
        let trace = borrow_handle!(trace, Trace, SentilError::NullPointer);
        let lifting = borrow_handle!(lifting, LiftingRegistry, SentilError::NullPointer);
        let config: SmcConfig = unsafe { *config }.into();
        match formula.check_distribution(trace, lifting, &config) {
            Ok((result, distribution)) => {
                unsafe {
                    *out_result = result.into();
                    *out_distribution = distribution.into();
                }
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_check(
    monitor: *mut c_void,
    trace: *mut c_void,
    lifting: *mut c_void,
    out: *mut SentilSmcResult,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let monitor = borrow_handle!(monitor, Monitor, SentilError::NullPointer);
        let trace = borrow_handle!(trace, Trace, SentilError::NullPointer);
        let lifting = borrow_handle!(lifting, LiftingRegistry, SentilError::NullPointer);
        match monitor.check(trace, lifting) {
            Ok(result) => {
                unsafe { *out = result.into() };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_check_sequential(
    formula: *mut c_void,
    trace: *mut c_void,
    lifting: *mut c_void,
    config: *const SentilSprtConfig,
    out: *mut SentilSprtResult,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(config, SentilError::NullPointer);
        check_ptr!(out, SentilError::NullPointer);
        let formula = borrow_handle!(formula, Formula, SentilError::NullPointer);
        let trace = borrow_handle!(trace, Trace, SentilError::NullPointer);
        let lifting = borrow_handle!(lifting, LiftingRegistry, SentilError::NullPointer);
        let config = match unsafe { *config }.to_core() {
            Ok(c) => c,
            Err(e) => return e.into(),
        };
        match formula.check_sequential(trace, lifting, &config) {
            Ok(result) => {
                unsafe { *out = result.into() };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_check_sequential(
    monitor: *mut c_void,
    trace: *mut c_void,
    lifting: *mut c_void,
    config: *const SentilSprtConfig,
    out: *mut SentilSprtResult,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(config, SentilError::NullPointer);
        check_ptr!(out, SentilError::NullPointer);
        let monitor = borrow_handle!(monitor, Monitor, SentilError::NullPointer);
        let trace = borrow_handle!(trace, Trace, SentilError::NullPointer);
        let lifting = borrow_handle!(lifting, LiftingRegistry, SentilError::NullPointer);
        let config = match unsafe { *config }.to_core() {
            Ok(c) => c,
            Err(e) => return e.into(),
        };
        match monitor.check_sequential(trace, lifting, &config) {
            Ok(result) => {
                unsafe { *out = result.into() };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_formula_check_bayesian(
    formula: *mut c_void,
    trace: *mut c_void,
    lifting: *mut c_void,
    config: *const SentilBayesConfig,
    out: *mut SentilBayesResult,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(config, SentilError::NullPointer);
        check_ptr!(out, SentilError::NullPointer);
        let formula = borrow_handle!(formula, Formula, SentilError::NullPointer);
        let trace = borrow_handle!(trace, Trace, SentilError::NullPointer);
        let lifting = borrow_handle!(lifting, LiftingRegistry, SentilError::NullPointer);
        let config = match unsafe { *config }.to_core() {
            Ok(c) => c,
            Err(e) => return e.into(),
        };
        match formula.check_bayesian(trace, lifting, &config) {
            Ok(result) => {
                unsafe { *out = result.into() };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

/// A Bernoulli source.
pub type SentilBernoulliFn = unsafe extern "C" fn(userdata: *mut c_void) -> bool;

#[no_mangle]
pub extern "C" fn sentil_sequential_test(
    config: *const SentilSprtConfig,
    draw: Option<SentilBernoulliFn>,
    userdata: *mut c_void,
    out: *mut SentilSprtResult,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(config, SentilError::NullPointer);
        check_ptr!(out, SentilError::NullPointer);
        let Some(draw) = draw else {
            set_error(SentilError::NullPointer, "the draw callback was null");
            return SentilError::NullPointer;
        };
        let config = match unsafe { *config }.to_core() {
            Ok(c) => c,
            Err(e) => return e.into(),
        };
        match sequential_test(&config, || Ok(unsafe { draw(userdata) })) {
            Ok(result) => {
                unsafe { *out = result.into() };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_bayes_sequential_test(
    config: *const SentilBayesConfig,
    draw: Option<SentilBernoulliFn>,
    userdata: *mut c_void,
    out: *mut SentilBayesResult,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(config, SentilError::NullPointer);
        check_ptr!(out, SentilError::NullPointer);
        let Some(draw) = draw else {
            set_error(SentilError::NullPointer, "the draw callback was null");
            return SentilError::NullPointer;
        };
        let config = match unsafe { *config }.to_core() {
            Ok(c) => c,
            Err(e) => return e.into(),
        };
        match bayes_sequential_test(&config, || Ok(unsafe { draw(userdata) })) {
            Ok(result) => {
                unsafe { *out = result.into() };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[cfg(feature = "gpu")]
fn sim_binary(
    left: *mut c_void,
    right: *mut c_void,
    build: fn(Box<SimExpr>, Box<SimExpr>) -> SimExpr,
) -> *mut c_void {
    if crate::formula::aliased(left, right) {
        return ptr::null_mut();
    }
    let (Some(l), Some(r)) =
        (unsafe { take_handle::<SimExpr>(left) }, unsafe { take_handle::<SimExpr>(right) })
    else {
        set_error(SentilError::NullPointer, "a child expression was null");
        return ptr::null_mut();
    };
    into_handle(build(Box::new(l), Box::new(r)))
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_expr_prev(variable: size_t) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || into_handle(SimExpr::Prev(variable)))
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_expr_time() -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || into_handle(SimExpr::Time))
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_expr_const(value: f64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || into_handle(SimExpr::Const(value)))
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_expr_noise(source: size_t) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || into_handle(SimExpr::Noise(source)))
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_expr_add(left: *mut c_void, right: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || sim_binary(left, right, SimExpr::Add))
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_expr_sub(left: *mut c_void, right: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || sim_binary(left, right, SimExpr::Sub))
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_expr_mul(left: *mut c_void, right: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || sim_binary(left, right, SimExpr::Mul))
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_expr_div(left: *mut c_void, right: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || sim_binary(left, right, SimExpr::Div))
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_expr_call(
    name: *const c_char,
    args: *mut *mut c_void,
    count: size_t,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let Ok(name) = c_char_to_string(name) else {
            return ptr::null_mut();
        };
        if count > 0 {
            check_ptr!(args, ptr::null_mut());
        }
        if unsafe { crate::formula::repeated_arg(args, count) } {
            return ptr::null_mut();
        }
        let mut taken: Vec<Option<SimExpr>> = Vec::with_capacity(count);
        for i in 0..count {
            taken.push(unsafe { take_handle::<SimExpr>(*args.add(i)) });
        }
        if taken.iter().any(Option::is_none) {
            set_error(SentilError::NullPointer, "an argument expression was null");
            return ptr::null_mut();
        }
        into_handle(SimExpr::Call(name, taken.into_iter().flatten().collect()))
    })
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_expr_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<SimExpr>(handle) });
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_model_create(
    variables: *const *const c_char,
    n_vars: size_t,
    dt: f64,
    horizon: size_t,
    init: *mut *mut c_void,
    n_init: size_t,
    advance: *mut *mut c_void,
    n_advance: size_t,
    noise: *mut *mut c_void,
    n_noise: size_t,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        if unsafe {
            repeated_handle(&[
                ("init", init, n_init),
                ("advance", advance, n_advance),
                ("noise", noise, n_noise),
            ])
        } {
            return ptr::null_mut();
        }
        let init = unsafe { take_handle_array::<SimExpr>("init", init, n_init) };
        let advance = unsafe { take_handle_array::<SimExpr>("advance", advance, n_advance) };
        let noise = unsafe { take_handle_array::<NoiseModel>("noise", noise, n_noise) };
        let (Some(init), Some(advance), Some(noise)) = (init, advance, noise) else {
            return ptr::null_mut();
        };
        let variables = match collect_strings(variables, n_vars) {
            Ok(v) => v,
            Err(_) => return ptr::null_mut(),
        };
        match SimModel::new(variables, dt, horizon, init, advance, noise) {
            Ok(model) => into_handle(model),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_model_simulate(handle: *mut c_void, seed: u64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let model = borrow_handle!(handle, SimModel, ptr::null_mut());
        match model.simulate(&mut rng_from(seed)) {
            Ok(trace) => into_handle(trace),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_model_variables(
    handle: *mut c_void,
    out_count: *mut size_t,
) -> *mut *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let model = borrow_handle!(handle, SimModel, ptr::null_mut());
        into_string_array(model.variables().iter().map(String::from).collect(), out_count)
    })
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_model_dt(handle: *mut c_void) -> f64 {
    clear_error();
    ffi_panic_boundary(f64::NAN, || borrow_handle!(handle, SimModel, f64::NAN).dt())
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_model_horizon(handle: *mut c_void) -> size_t {
    clear_error();
    ffi_panic_boundary(0, || borrow_handle!(handle, SimModel, 0).horizon())
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_model_to_stochastic_system(handle: *mut c_void) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let model = borrow_handle!(handle, SimModel, ptr::null_mut());
        match model.to_stochastic_system() {
            Ok(system) => into_handle(system),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[cfg(feature = "gpu")]
#[no_mangle]
pub extern "C" fn sentil_sim_model_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<SimModel>(handle) });
}

#[no_mangle]
pub extern "C" fn sentil_stochastic_system_simulate(handle: *mut c_void, seed: u64) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let system = borrow_handle!(handle, StochasticSystem, ptr::null_mut());
        match system.simulate(&mut rng_from(seed)) {
            Ok(trace) => into_handle(trace),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_stochastic_system_variables(
    handle: *mut c_void,
    out_count: *mut size_t,
) -> *mut *mut c_char {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        check_ptr!(out_count, ptr::null_mut());
        let system = borrow_handle!(handle, StochasticSystem, ptr::null_mut());
        into_string_array(system.variables().iter().map(String::from).collect(), out_count)
    })
}

#[no_mangle]
pub extern "C" fn sentil_stochastic_system_dt(handle: *mut c_void) -> f64 {
    clear_error();
    ffi_panic_boundary(f64::NAN, || borrow_handle!(handle, StochasticSystem, f64::NAN).dt())
}

#[no_mangle]
pub extern "C" fn sentil_stochastic_system_horizon(handle: *mut c_void) -> size_t {
    clear_error();
    ffi_panic_boundary(0, || borrow_handle!(handle, StochasticSystem, 0).horizon())
}

#[no_mangle]
pub extern "C" fn sentil_stochastic_system_create(
    variables: *const *const c_char,
    n_vars: size_t,
    dt: f64,
    horizon: size_t,
    callbacks: SentilSystemCallbacks,
) -> *mut c_void {
    clear_error();
    ffi_panic_boundary(ptr::null_mut(), || {
        let variables = match collect_strings(variables, n_vars) {
            Ok(v) => v,
            Err(_) => return ptr::null_mut(),
        };
        let (Some(init_fn), Some(step_fn)) = (callbacks.init, callbacks.step) else {
            set_error(SentilError::NullPointer, "the init or step callback was null");
            return ptr::null_mut();
        };
        let userdata = callbacks.userdata as usize;
        let init = move |rng: &mut dyn RngCore| {
            let seed = rng.next_u64();
            let mut state = vec![0.0_f64; n_vars];
            unsafe { init_fn(userdata as *mut c_void, seed, state.as_mut_ptr(), n_vars) };
            state
        };
        let step = move |prev: &[f64], time: f64, rng: &mut dyn RngCore| {
            let seed = rng.next_u64();
            let mut state = vec![0.0_f64; n_vars];
            unsafe {
                step_fn(userdata as *mut c_void, prev.as_ptr(), prev.len(), time, seed, state.as_mut_ptr())
            };
            state
        };
        match StochasticSystem::new(variables, dt, horizon, init, step) {
            Ok(system) => into_handle(system.thread_confined()),
            Err(e) => {
                let _: SentilError = e.into();
                ptr::null_mut()
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_stochastic_system_destroy(handle: *mut c_void) {
    clear_error();
    ffi_panic_boundary((), || unsafe { drop_handle::<StochasticSystem>(handle) });
}

/// Rare-event settings.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SentilRareEventConfig {
    pub particles: size_t,
    pub margin: f64,
    pub seed: u64,
}

impl From<SentilRareEventConfig> for RareEventConfig {
    fn from(c: SentilRareEventConfig) -> Self {
        RareEventConfig { particles: c.particles, margin: c.margin, seed: c.seed }
    }
}

/// A rare-event estimate.
#[repr(C)]
pub struct SentilRareEventResult {
    pub probability: f64,
    pub violation_probability: f64,
    pub holds: bool,
    pub simulations: u64,
}

impl From<RareEventResult> for SentilRareEventResult {
    fn from(r: RareEventResult) -> Self {
        Self {
            probability: r.probability,
            violation_probability: r.violation_probability,
            holds: r.holds,
            simulations: r.simulations,
        }
    }
}

#[no_mangle]
pub extern "C" fn sentil_rare_event_config_default() -> SentilRareEventConfig {
    let d = RareEventConfig::default();
    SentilRareEventConfig { particles: d.particles, margin: d.margin, seed: d.seed }
}

#[no_mangle]
pub extern "C" fn sentil_formula_check_rare_event(
    formula: *mut c_void,
    system: *mut c_void,
    config: *const SentilRareEventConfig,
    out: *mut SentilRareEventResult,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(config, SentilError::NullPointer);
        check_ptr!(out, SentilError::NullPointer);
        let formula = borrow_handle!(formula, Formula, SentilError::NullPointer);
        let system = borrow_handle!(system, StochasticSystem, SentilError::NullPointer);
        let config: RareEventConfig = unsafe { *config }.into();
        match formula.check_rare_event(system, &config) {
            Ok(result) => {
                unsafe { *out = result.into() };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[no_mangle]
pub extern "C" fn sentil_monitor_check_rare(
    monitor: *mut c_void,
    system: *mut c_void,
    out: *mut SentilRareEventResult,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let monitor = borrow_handle!(monitor, Monitor, SentilError::NullPointer);
        let system = borrow_handle!(system, StochasticSystem, SentilError::NullPointer);
        match monitor.check_rare(system) {
            Ok(result) => {
                unsafe { *out = result.into() };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

/// A simulator defined by C callbacks over an opaque, fixed-size state.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SentilAmsInterface {
    pub state_size: size_t,
    pub userdata: *mut c_void,
    pub initial_state: Option<unsafe extern "C" fn(*mut c_void, u64, *mut c_void)>,
    pub step: Option<unsafe extern "C" fn(*mut c_void, *const c_void, u64, *mut c_void)>,
    pub is_terminal: Option<unsafe extern "C" fn(*mut c_void, *const c_void, *mut bool) -> bool>,
    pub score: Option<unsafe extern "C" fn(*mut c_void, *const c_void) -> f64>,
}

struct AmsBridge {
    state_size: usize,
    userdata: usize,
    initial_state: unsafe extern "C" fn(*mut c_void, u64, *mut c_void),
    step: unsafe extern "C" fn(*mut c_void, *const c_void, u64, *mut c_void),
    is_terminal: unsafe extern "C" fn(*mut c_void, *const c_void, *mut bool) -> bool,
    score: unsafe extern "C" fn(*mut c_void, *const c_void) -> f64,
}

impl RareEventSimulator for AmsBridge {
    type State = Vec<u8>;

    fn initial_state(&self, rng: &mut dyn RngCore) -> Vec<u8> {
        let mut state = vec![0u8; self.state_size];
        let seed = rng.next_u64();
        unsafe { (self.initial_state)(self.userdata as *mut c_void, seed, state.as_mut_ptr().cast()) };
        state
    }

    fn step(&self, state: &Vec<u8>, rng: &mut dyn RngCore) -> Vec<u8> {
        let mut next = vec![0u8; self.state_size];
        let seed = rng.next_u64();
        unsafe {
            (self.step)(self.userdata as *mut c_void, state.as_ptr().cast(), seed, next.as_mut_ptr().cast())
        };
        next
    }

    fn is_terminal(&self, state: &Vec<u8>) -> (bool, bool) {
        let mut in_rare_event = false;
        let ended = unsafe {
            (self.is_terminal)(self.userdata as *mut c_void, state.as_ptr().cast(), &mut in_rare_event)
        };
        (ended, in_rare_event)
    }

    fn score(&self, state: &Vec<u8>) -> f64 {
        unsafe { (self.score)(self.userdata as *mut c_void, state.as_ptr().cast()) }
    }
}

/// A rare-event estimate from a custom simulator.
#[repr(C)]
pub struct SentilRareEventEstimate {
    pub probability: f64,
    pub simulations: u64,
}

impl From<RareEventEstimate> for SentilRareEventEstimate {
    fn from(e: RareEventEstimate) -> Self {
        Self { probability: e.probability, simulations: e.simulations }
    }
}

#[no_mangle]
pub extern "C" fn sentil_adaptive_multilevel_splitting(
    simulator: SentilAmsInterface,
    particles: size_t,
    target_score: f64,
    max_steps: u64,
    seed: u64,
    out: *mut SentilRareEventEstimate,
) -> SentilError {
    clear_error();
    ffi_panic_boundary(SentilError::Panic, || {
        check_ptr!(out, SentilError::NullPointer);
        let (Some(initial_state), Some(step), Some(is_terminal), Some(score)) =
            (simulator.initial_state, simulator.step, simulator.is_terminal, simulator.score)
        else {
            set_error(SentilError::NullPointer, "an AMS callback was null");
            return SentilError::NullPointer;
        };
        let bridge = AmsBridge {
            state_size: simulator.state_size,
            userdata: simulator.userdata as usize,
            initial_state,
            step,
            is_terminal,
            score,
        };
        match adaptive_multilevel_splitting(&bridge, particles, target_score, max_steps, seed) {
            Ok(estimate) => {
                unsafe { *out = estimate.into() };
                SentilError::Ok
            }
            Err(e) => e.into(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversions::last_error_ptr;
    use std::ffi::CStr;

    fn message() -> String {
        unsafe { CStr::from_ptr(last_error_ptr()) }.to_string_lossy().into_owned()
    }

    #[test]
    fn a_mixture_component_passed_twice_is_rejected_and_stays_live() {
        let component = sentil_noise_gaussian(0.0, 1.0);
        let weights = [0.5, 0.5];
        let mut models = [component, component];
        assert!(sentil_noise_mixture(weights.as_ptr(), models.as_mut_ptr(), 2).is_null());
        assert_eq!(crate::sentil_get_last_error_code(), SentilError::InvalidConfig);
        assert!(message().contains("`models[0]` and `models[1]`"), "{}", message());
        let mut mean = f64::NAN;
        assert!(sentil_noise_mean(component, &mut mean));
        assert_eq!(mean, 0.0);
        sentil_noise_destroy(component);
    }

    #[test]
    fn distinct_mixture_components_are_all_kept() {
        let weights = [0.5, 0.5];
        let mut models = [sentil_noise_gaussian(0.0, 1.0), sentil_noise_gaussian(4.0, 1.0)];
        let mixture = sentil_noise_mixture(weights.as_ptr(), models.as_mut_ptr(), 2);
        assert!(!mixture.is_null());
        let mut mean = f64::NAN;
        assert!(sentil_noise_mean(mixture, &mut mean));
        assert_eq!(mean, 2.0);
        sentil_noise_destroy(mixture);
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn an_expression_shared_by_init_and_advance_is_rejected() {
        let shared = sentil_sim_expr_const(0.0);
        let name = std::ffi::CString::new("x").unwrap();
        let vars = [name.as_ptr()];
        let mut init = [shared];
        let mut advance = [shared];
        let model = sentil_sim_model_create(
            vars.as_ptr(),
            1,
            0.1,
            8,
            init.as_mut_ptr(),
            1,
            advance.as_mut_ptr(),
            1,
            ptr::null_mut(),
            0,
        );
        assert!(model.is_null());
        assert_eq!(crate::sentil_get_last_error_code(), SentilError::InvalidConfig);
        assert!(message().contains("`init[0]` and `advance[0]`"), "{}", message());
        sentil_sim_expr_destroy(shared);
    }
}