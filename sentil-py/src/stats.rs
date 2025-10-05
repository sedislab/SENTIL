//! Statistical model checking.

use crate::errors::{pyerr, EvaluationError};
use crate::signal::Trace;
use numpy::{IntoPyArray, PyArray1};
use pyo3::prelude::*;
use sentil::stats::{
    agresti_coull as core_agresti, chernoff_hoeffding_samples as core_chernoff,
    clopper_pearson as core_clopper, jeffreys_interval as core_jeffreys,
    wilson_interval as core_wilson, wilson_samples as core_wilson_samples, z_score as core_z,
    ConfidenceInterval as CoreInterval, IntervalMethod as CoreMethod, LiftingRegistry as CoreLifting,
    NoiseInteraction as CoreInteraction, NoiseModel as CoreNoise,
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

/// How a noise draw combines with a sensor reading when lifting.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NoiseInteraction {
    Additive,
    Multiplicative,
}

impl From<NoiseInteraction> for CoreInteraction {
    fn from(interaction: NoiseInteraction) -> Self {
        match interaction {
            NoiseInteraction::Additive => CoreInteraction::Additive,
            NoiseInteraction::Multiplicative => CoreInteraction::Multiplicative,
        }
    }
}

/// A fitted or declared sensor noise distribution.
#[pyclass]
#[derive(Clone)]
pub struct NoiseModel {
    pub(crate) inner: CoreNoise,
}

fn built(result: sentil::Result<CoreNoise>) -> PyResult<NoiseModel> {
    Ok(NoiseModel { inner: result.map_err(pyerr)? })
}

#[pymethods]
impl NoiseModel {
    #[staticmethod]
    fn dirac(value: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::dirac(value))
    }

    #[staticmethod]
    fn gaussian(mean: f64, std_dev: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::gaussian(mean, std_dev))
    }

    #[staticmethod]
    fn uniform(low: f64, high: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::uniform(low, high))
    }

    #[staticmethod]
    fn log_normal(mu: f64, sigma: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::log_normal(mu, sigma))
    }

    #[staticmethod]
    fn exponential(rate: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::exponential(rate))
    }

    #[staticmethod]
    fn gamma(shape: f64, scale: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::gamma(shape, scale))
    }

    #[staticmethod]
    fn beta(alpha: f64, beta: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::beta(alpha, beta))
    }

    #[staticmethod]
    fn weibull(shape: f64, scale: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::weibull(shape, scale))
    }

    #[staticmethod]
    fn rayleigh(scale: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::rayleigh(scale))
    }

    #[staticmethod]
    fn gumbel(location: f64, scale: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::gumbel(location, scale))
    }

    #[staticmethod]
    fn cauchy(location: f64, scale: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::cauchy(location, scale))
    }

    #[staticmethod]
    fn student_t(df: f64, location: f64, scale: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::student_t(df, location, scale))
    }

    #[staticmethod]
    fn truncated_normal(mean: f64, std_dev: f64, lower: f64, upper: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::truncated_normal(mean, std_dev, lower, upper))
    }

    #[staticmethod]
    fn poisson(rate: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::poisson(rate))
    }

    #[staticmethod]
    fn binomial(trials: u64, probability: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::binomial(trials, probability))
    }

    /// The empirical distribution of a set of residuals, resampled with replacement.
    #[staticmethod]
    fn bootstrap(residuals: Vec<f64>) -> PyResult<NoiseModel> {
        built(CoreNoise::bootstrap(residuals))
    }

    /// A weighted mixture of component models.
    #[staticmethod]
    fn mixture(
        py: Python<'_>,
        weights: Vec<f64>,
        components: Vec<Py<NoiseModel>>,
    ) -> PyResult<NoiseModel> {
        let models = components.iter().map(|c| c.borrow(py).inner.clone()).collect();
        built(CoreNoise::mixture(weights, models))
    }

    /// Maximum-likelihood Gaussian fit of a sample.
    #[staticmethod]
    fn fit_gaussian(samples: Vec<f64>) -> PyResult<NoiseModel> {
        built(CoreNoise::fit_gaussian(&samples))
    }

    /// The empirical bootstrap of a sample.
    #[staticmethod]
    fn fit_bootstrap(samples: Vec<f64>) -> PyResult<NoiseModel> {
        built(CoreNoise::fit_bootstrap(&samples))
    }

    /// A reservoir-sampled bootstrap that caps the retained residuals.
    #[staticmethod]
    fn fit_bootstrap_reservoir(samples: Vec<f64>, max_samples: usize) -> PyResult<NoiseModel> {
        built(CoreNoise::fit_bootstrap_reservoir(&samples, max_samples))
    }

    /// A Gaussian mixture fit by expectation-maximization.
    #[staticmethod]
    fn fit_gaussian_mixture(
        samples: Vec<f64>,
        components: usize,
        max_iters: usize,
    ) -> PyResult<NoiseModel> {
        built(CoreNoise::fit_gaussian_mixture(&samples, components, max_iters))
    }

    /// The residuals between ground truth and sensor readings under `interaction`.
    #[staticmethod]
    fn residuals<'py>(
        py: Python<'py>,
        truth: Vec<f64>,
        sensor: Vec<f64>,
        interaction: NoiseInteraction,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let residuals = CoreNoise::residuals(&truth, &sensor, interaction.into()).map_err(pyerr)?;
        Ok(residuals.into_pyarray(py))
    }

    /// The mean of the distribution.
    fn mean(&self) -> Option<f64> {
        self.inner.mean()
    }

    /// The variance of the distribution.
    fn variance(&self) -> Option<f64> {
        self.inner.variance()
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| EvaluationError::new_err(format!("could not serialize the noise model: {e}")))
    }

    #[staticmethod]
    fn from_json(text: &str) -> PyResult<NoiseModel> {
        let inner = serde_json::from_str(text)
            .map_err(|e| EvaluationError::new_err(format!("invalid noise model JSON: {e}")))?;
        Ok(NoiseModel { inner })
    }

    fn __repr__(&self) -> String {
        format!("NoiseModel({:?})", self.inner)
    }
}

/// The per-variable noise models used to lift a trace.
#[pyclass]
#[derive(Default)]
pub struct LiftingRegistry {
    pub(crate) inner: CoreLifting,
}

#[pymethods]
impl LiftingRegistry {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    #[pyo3(signature = (variable, model, interaction=NoiseInteraction::Additive))]
    fn register(
        &mut self,
        variable: &str,
        model: PyRef<'_, NoiseModel>,
        interaction: NoiseInteraction,
    ) {
        self.inner.register(variable, model.inner.clone(), interaction.into());
    }

    #[getter]
    fn variables(&self) -> Vec<String> {
        self.inner.variables().into_iter().map(String::from).collect()
    }

    #[getter]
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Draw one stochastic realization of `trace` from the registered models.
    #[pyo3(signature = (trace, seed=42))]
    fn lift(&self, trace: &Trace, seed: u64) -> PyResult<Trace> {
        Ok(Trace { inner: self.inner.lift(&trace.inner, seed).map_err(pyerr)? })
    }

    fn __repr__(&self) -> String {
        format!("LiftingRegistry(variables={:?})", self.inner.variables())
    }
}