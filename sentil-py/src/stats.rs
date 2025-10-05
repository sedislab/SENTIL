//! Statistical model checking.

use crate::errors::{pyerr, EvaluationError};
use pyo3::prelude::*;
use sentil::stats::{
    agresti_coull as core_agresti, chernoff_hoeffding_samples as core_chernoff,
    clopper_pearson as core_clopper, jeffreys_interval as core_jeffreys,
    wilson_interval as core_wilson, wilson_samples as core_wilson_samples, z_score as core_z,
    ConfidenceInterval as CoreInterval, IntervalMethod as CoreMethod,
};

/// A binomial proportion confidence interval at a stated level.
#[pyclass(frozen)]
pub struct ConfidenceInterval {
    #[pyo3(get)]
    pub lower: f64,
    #[pyo3(get)]
    pub upper: f64,
    #[pyo3(get)]
    pub level: f64,
}

impl ConfidenceInterval {
    fn from_core(ci: CoreInterval) -> Self {
        Self { lower: ci.lower, upper: ci.upper, level: ci.level }
    }
}

#[pymethods]
impl ConfidenceInterval {
    #[getter]
    fn width(&self) -> f64 {
        self.upper - self.lower
    }

    fn __repr__(&self) -> String {
        format!("ConfidenceInterval(lower={}, upper={}, level={})", self.lower, self.upper, self.level)
    }
}

/// The estimator a confidence interval is built with.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IntervalMethod {
    Wilson,
    ClopperPearson,
    Jeffreys,
    AgrestiCoull,
}

impl From<IntervalMethod> for CoreMethod {
    fn from(method: IntervalMethod) -> Self {
        match method {
            IntervalMethod::Wilson => CoreMethod::Wilson,
            IntervalMethod::ClopperPearson => CoreMethod::ClopperPearson,
            IntervalMethod::Jeffreys => CoreMethod::Jeffreys,
            IntervalMethod::AgrestiCoull => CoreMethod::AgrestiCoull,
        }
    }
}

/// The Wilson score interval for `successes` out of `trials` at `level`.
#[pyfunction]
pub fn wilson_interval(successes: u64, trials: u64, level: f64) -> ConfidenceInterval {
    ConfidenceInterval::from_core(core_wilson(successes, trials, level))
}

/// The Clopper-Pearson exact interval.
#[pyfunction]
pub fn clopper_pearson(successes: u64, trials: u64, level: f64) -> ConfidenceInterval {
    ConfidenceInterval::from_core(core_clopper(successes, trials, level))
}

#[pyfunction]
pub fn jeffreys_interval(successes: u64, trials: u64, level: f64) -> ConfidenceInterval {
    ConfidenceInterval::from_core(core_jeffreys(successes, trials, level))
}

#[pyfunction]
pub fn agresti_coull(successes: u64, trials: u64, level: f64) -> ConfidenceInterval {
    ConfidenceInterval::from_core(core_agresti(successes, trials, level))
}

/// A confidence interval by the chosen estimator.
#[pyfunction]
#[pyo3(signature = (successes, trials, level, method=IntervalMethod::Wilson))]
pub fn interval(
    successes: u64,
    trials: u64,
    level: f64,
    method: IntervalMethod,
) -> ConfidenceInterval {
    ConfidenceInterval::from_core(CoreMethod::from(method).interval(successes, trials, level))
}

/// The standard normal quantile at `level`.
#[pyfunction]
pub fn z_score(level: f64) -> f64 {
    core_z(level)
}

/// The Chernoff-Hoeffding sample count for error `epsilon` at confidence `1 - delta`.
#[pyfunction]
pub fn chernoff_hoeffding_samples(epsilon: f64, delta: f64) -> PyResult<u64> {
    core_chernoff(epsilon, delta).map_err(pyerr)
}

/// The Wilson sample count for a half-width `epsilon` at the given level.
#[pyfunction]
pub fn wilson_samples(epsilon: f64, level: f64) -> PyResult<u64> {
    core_wilson_samples(epsilon, level).map_err(pyerr)
}