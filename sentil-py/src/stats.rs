//! Statistical model checking.

use crate::errors::{pyerr, EvaluationError};
use crate::formula::Formula;
use crate::monitor::Monitor;
use crate::signal::Trace;
use numpy::{IntoPyArray, PyArray1};
use pyo3::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use sentil::stats::{
    agresti_coull as core_agresti, chernoff_hoeffding_samples as core_chernoff,
    clopper_pearson as core_clopper, jeffreys_interval as core_jeffreys,
    wilson_interval as core_wilson, wilson_samples as core_wilson_samples, z_score as core_z,
    BayesConfig as CoreBayes, BayesResult as CoreBayesResult, ConfidenceInterval as CoreInterval,
    IntervalMethod as CoreMethod, LiftingRegistry as CoreLifting, NoiseInteraction as CoreInteraction,
    NoiseModel as CoreNoise, RareEventConfig as CoreRare, RareEventResult as CoreRareResult,
    RobustnessDistribution as CoreDist, SimExpr as CoreSimExpr, SimModel as CoreSimModel,
    SmcConfig as CoreSmc, SmcResult as CoreSmcResult, SprtConfig as CoreSprt,
    SprtResult as CoreSprtResult, StochasticSystem as CoreSystem,
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
    /// A point mass at `value`.
    #[staticmethod]
    fn dirac(value: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::dirac(value))
    }

    /// A normal distribution with the given mean and standard deviation.
    #[staticmethod]
    fn gaussian(mean: f64, std_dev: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::gaussian(mean, std_dev))
    }

    /// A uniform distribution over [low, high].
    #[staticmethod]
    fn uniform(low: f64, high: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::uniform(low, high))
    }

    /// A log-normal distribution from the underlying normal's mu and sigma.
    #[staticmethod]
    fn log_normal(mu: f64, sigma: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::log_normal(mu, sigma))
    }

    /// An exponential distribution with the given rate.
    #[staticmethod]
    fn exponential(rate: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::exponential(rate))
    }

    /// A gamma distribution with the given shape and scale.
    #[staticmethod]
    fn gamma(shape: f64, scale: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::gamma(shape, scale))
    }

    /// A beta distribution with the given alpha and beta shape parameters.
    #[staticmethod]
    fn beta(alpha: f64, beta: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::beta(alpha, beta))
    }

    /// A Weibull distribution with the given shape and scale.
    #[staticmethod]
    fn weibull(shape: f64, scale: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::weibull(shape, scale))
    }

    /// A Rayleigh distribution with the given scale.
    #[staticmethod]
    fn rayleigh(scale: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::rayleigh(scale))
    }

    /// A Gumbel distribution with the given location and scale.
    #[staticmethod]
    fn gumbel(location: f64, scale: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::gumbel(location, scale))
    }

    /// A Cauchy distribution.
    #[staticmethod]
    fn cauchy(location: f64, scale: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::cauchy(location, scale))
    }

    /// A Student's t distribution with `df` degrees of freedom, location, and scale.
    #[staticmethod]
    fn student_t(df: f64, location: f64, scale: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::student_t(df, location, scale))
    }

    /// A normal distribution restricted to [lower, upper].
    #[staticmethod]
    fn truncated_normal(mean: f64, std_dev: f64, lower: f64, upper: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::truncated_normal(mean, std_dev, lower, upper))
    }

    /// A Poisson distribution with the given rate.
    #[staticmethod]
    fn poisson(rate: f64) -> PyResult<NoiseModel> {
        built(CoreNoise::poisson(rate))
    }

    /// A binomial distribution over `trials` with success probability `probability`.
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

    /// Load a model from a JSON file.
    #[staticmethod]
    fn from_file(path: &str) -> PyResult<NoiseModel> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| EvaluationError::new_err(format!("could not read {path}: {e}")))?;
        let inner = serde_json::from_str(&text)
            .map_err(|e| EvaluationError::new_err(format!("invalid noise model JSON in {path}: {e}")))?;
        Ok(NoiseModel { inner })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
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

/// Settings for statistical model checking by sampling.
#[pyclass]
#[derive(Clone)]
pub struct SmcConfig {
    #[pyo3(get, set)]
    pub samples: u64,
    #[pyo3(get, set)]
    pub confidence: f64,
    #[pyo3(get, set)]
    pub seed: u64,
    #[pyo3(get, set)]
    pub method: IntervalMethod,
}

impl SmcConfig {
    fn to_core(&self) -> CoreSmc {
        CoreSmc {
            samples: self.samples,
            confidence: self.confidence,
            seed: self.seed,
            interval_method: self.method.into(),
        }
    }
}

#[pymethods]
impl SmcConfig {
    #[new]
    #[pyo3(signature = (samples=10000, confidence=0.95, seed=42, method=IntervalMethod::Wilson))]
    fn new(samples: u64, confidence: f64, seed: u64, method: IntervalMethod) -> Self {
        Self { samples, confidence, seed, method }
    }

    fn __repr__(&self) -> String {
        format!("SmcConfig(samples={}, confidence={}, seed={})", self.samples, self.confidence, self.seed)
    }
}

/// The outcome of a statistical check.
#[pyclass(frozen)]
pub struct SmcResult {
    #[pyo3(get)]
    pub probability: f64,
    #[pyo3(get)]
    pub interval: Py<ConfidenceInterval>,
    #[pyo3(get)]
    pub satisfactions: u64,
    #[pyo3(get)]
    pub samples: u64,
    #[pyo3(get)]
    pub holds: bool,
}

impl SmcResult {
    fn from_core(py: Python<'_>, result: CoreSmcResult) -> PyResult<Self> {
        Ok(Self {
            probability: result.probability,
            interval: Py::new(py, ConfidenceInterval::from_core(result.interval))?,
            satisfactions: result.satisfactions,
            samples: result.samples,
            holds: result.holds,
        })
    }
}

#[pymethods]
impl SmcResult {
    fn __repr__(&self) -> String {
        format!("SmcResult(probability={}, holds={})", self.probability, self.holds)
    }
}

/// Summary statistics of the robustness values across the sampled ensemble.
#[pyclass(frozen)]
pub struct RobustnessDistribution {
    #[pyo3(get)]
    pub count: u64,
    #[pyo3(get)]
    pub mean: f64,
    #[pyo3(get)]
    pub variance: f64,
    #[pyo3(get)]
    pub min: f64,
    #[pyo3(get)]
    pub max: f64,
}

impl RobustnessDistribution {
    fn from_core(distribution: CoreDist) -> Self {
        Self {
            count: distribution.count,
            mean: distribution.mean,
            variance: distribution.variance,
            min: distribution.min,
            max: distribution.max,
        }
    }
}

#[pymethods]
impl RobustnessDistribution {
    #[getter]
    fn std_dev(&self) -> f64 {
        self.variance.sqrt()
    }

    fn __repr__(&self) -> String {
        format!("RobustnessDistribution(count={}, mean={}, std_dev={})", self.count, self.mean, self.variance.sqrt())
    }
}

#[pymethods]
impl Formula {
    /// Estimate the satisfaction probability by sampling the lifted trace ensemble.
    #[pyo3(signature = (trace, lifting, config=None))]
    fn check(
        &self,
        py: Python<'_>,
        trace: &Trace,
        lifting: &LiftingRegistry,
        config: Option<SmcConfig>,
    ) -> PyResult<SmcResult> {
        let config = config.map(|c| c.to_core()).unwrap_or_default();
        let result = self.inner.check(&trace.inner, &lifting.inner, &config).map_err(pyerr)?;
        SmcResult::from_core(py, result)
    }

    /// The same estimate with the Clopper-Pearson interval.
    #[pyo3(signature = (trace, lifting, config=None))]
    fn check_conservative(
        &self,
        py: Python<'_>,
        trace: &Trace,
        lifting: &LiftingRegistry,
        config: Option<SmcConfig>,
    ) -> PyResult<SmcResult> {
        let config = config.map(|c| c.to_core()).unwrap_or_default();
        let result =
            self.inner.check_conservative(&trace.inner, &lifting.inner, &config).map_err(pyerr)?;
        SmcResult::from_core(py, result)
    }

    /// The estimate together with the robustness distribution across the ensemble.
    #[pyo3(signature = (trace, lifting, config=None))]
    fn check_distribution(
        &self,
        py: Python<'_>,
        trace: &Trace,
        lifting: &LiftingRegistry,
        config: Option<SmcConfig>,
    ) -> PyResult<(SmcResult, RobustnessDistribution)> {
        let config = config.map(|c| c.to_core()).unwrap_or_default();
        let (result, distribution) =
            self.inner.check_distribution(&trace.inner, &lifting.inner, &config).map_err(pyerr)?;
        Ok((SmcResult::from_core(py, result)?, RobustnessDistribution::from_core(distribution)))
    }
}

#[pymethods]
impl Monitor {
    /// Statistically check the monitored formula against the lifted ensemble.
    fn check(&self, py: Python<'_>, trace: &Trace, lifting: &LiftingRegistry) -> PyResult<SmcResult> {
        let result = self.inner.check(&trace.inner, &lifting.inner).map_err(pyerr)?;
        SmcResult::from_core(py, result)
    }
}
/// The verdict of a sequential probability ratio test.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SprtVerdict {
    AcceptH0,
    AcceptH1,
    Inconclusive,
}

/// SPRT settings.
#[pyclass]
#[derive(Clone)]
pub struct SprtConfig {
    #[pyo3(get, set)]
    pub p0: f64,
    #[pyo3(get, set)]
    pub p1: f64,
    #[pyo3(get, set)]
    pub alpha: f64,
    #[pyo3(get, set)]
    pub beta: f64,
    #[pyo3(get, set)]
    pub max_samples: u64,
    #[pyo3(get, set)]
    pub seed: u64,
}

impl SprtConfig {
    fn to_core(&self) -> PyResult<CoreSprt> {
        Ok(CoreSprt::new(self.p0, self.p1, self.alpha, self.beta, self.max_samples)
            .map_err(pyerr)?
            .with_seed(self.seed))
    }
}

#[pymethods]
impl SprtConfig {
    #[new]
    #[pyo3(signature = (p0, p1, alpha=0.05, beta=0.05, max_samples=100000, seed=42))]
    fn new(p0: f64, p1: f64, alpha: f64, beta: f64, max_samples: u64, seed: u64) -> Self {
        Self { p0, p1, alpha, beta, max_samples, seed }
    }
}

/// The result of a sequential test.
#[pyclass(frozen)]
pub struct SprtResult {
    #[pyo3(get)]
    pub verdict: SprtVerdict,
    #[pyo3(get)]
    pub samples: u64,
    #[pyo3(get)]
    pub log_likelihood: f64,
}

impl SprtResult {
    fn from_core(result: CoreSprtResult) -> Self {
        match result {
            CoreSprtResult::AcceptH0 { samples } => {
                Self { verdict: SprtVerdict::AcceptH0, samples, log_likelihood: 0.0 }
            }
            CoreSprtResult::AcceptH1 { samples } => {
                Self { verdict: SprtVerdict::AcceptH1, samples, log_likelihood: 0.0 }
            }
            CoreSprtResult::Inconclusive { samples, log_likelihood } => {
                Self { verdict: SprtVerdict::Inconclusive, samples, log_likelihood }
            }
        }
    }
}

#[pymethods]
impl SprtResult {
    fn __repr__(&self) -> String {
        format!("SprtResult(verdict=SprtVerdict.{:?}, samples={})", self.verdict, self.samples)
    }
}

/// The verdict of a Bayesian sequential test.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BayesVerdict {
    Holds,
    Fails,
    Inconclusive,
}

/// Bayesian sequential test settings.
#[pyclass]
#[derive(Clone)]
pub struct BayesConfig {
    #[pyo3(get, set)]
    pub threshold: f64,
    #[pyo3(get, set)]
    pub bayes_factor: f64,
    #[pyo3(get, set)]
    pub max_samples: u64,
    #[pyo3(get, set)]
    pub seed: u64,
}

impl BayesConfig {
    fn to_core(&self) -> PyResult<CoreBayes> {
        Ok(CoreBayes::new(self.threshold, self.bayes_factor, self.max_samples)
            .map_err(pyerr)?
            .with_seed(self.seed))
    }
}

#[pymethods]
impl BayesConfig {
    #[new]
    #[pyo3(signature = (threshold, bayes_factor=100.0, max_samples=100000, seed=42))]
    fn new(threshold: f64, bayes_factor: f64, max_samples: u64, seed: u64) -> Self {
        Self { threshold, bayes_factor, max_samples, seed }
    }
}

/// The result of a Bayesian test.
#[pyclass(frozen)]
pub struct BayesResult {
    #[pyo3(get)]
    pub verdict: BayesVerdict,
    #[pyo3(get)]
    pub samples: u64,
    #[pyo3(get)]
    pub posterior: f64,
}

impl BayesResult {
    fn from_core(result: CoreBayesResult) -> Self {
        match result {
            CoreBayesResult::Holds { samples, posterior } => {
                Self { verdict: BayesVerdict::Holds, samples, posterior }
            }
            CoreBayesResult::Fails { samples, posterior } => {
                Self { verdict: BayesVerdict::Fails, samples, posterior }
            }
            CoreBayesResult::Inconclusive { samples, posterior } => {
                Self { verdict: BayesVerdict::Inconclusive, samples, posterior }
            }
        }
    }
}

#[pymethods]
impl BayesResult {
    fn __repr__(&self) -> String {
        format!("BayesResult(verdict=BayesVerdict.{:?}, samples={})", self.verdict, self.samples)
    }
}

#[pymethods]
impl Formula {
    /// Decide a probabilistic formula by Wald's sequential probability ratio test.
    fn check_sequential(
        &self,
        trace: &Trace,
        lifting: &LiftingRegistry,
        config: &SprtConfig,
    ) -> PyResult<SprtResult> {
        let result = self.inner.check_sequential(&trace.inner, &lifting.inner, &config.to_core()?);
        Ok(SprtResult::from_core(result.map_err(pyerr)?))
    }

    /// Decide a probabilistic formula by a Bayesian sequential test.
    fn check_bayesian(
        &self,
        trace: &Trace,
        lifting: &LiftingRegistry,
        config: &BayesConfig,
    ) -> PyResult<BayesResult> {
        let result = self.inner.check_bayesian(&trace.inner, &lifting.inner, &config.to_core()?);
        Ok(BayesResult::from_core(result.map_err(pyerr)?))
    }
}

#[pymethods]
impl Monitor {
    /// Decide the monitored formula by Wald's sequential probability ratio test.
    fn check_sequential(
        &self,
        trace: &Trace,
        lifting: &LiftingRegistry,
        config: &SprtConfig,
    ) -> PyResult<SprtResult> {
        let result = self.inner.check_sequential(&trace.inner, &lifting.inner, &config.to_core()?);
        Ok(SprtResult::from_core(result.map_err(pyerr)?))
    }
}
fn to_sim_expr(value: &Bound<'_, PyAny>) -> PyResult<CoreSimExpr> {
    if let Ok(expr) = value.extract::<PyRef<SimExpr>>() {
        Ok(expr.inner.clone())
    } else if let Ok(number) = value.extract::<f64>() {
        Ok(CoreSimExpr::Const(number))
    } else {
        Err(pyo3::exceptions::PyTypeError::new_err("expected a sim expression or a number"))
    }
}

/// A term in a declarative stochastic update.
#[pyclass]
#[derive(Clone)]
pub struct SimExpr {
    pub(crate) inner: CoreSimExpr,
}

impl SimExpr {
    fn combine(
        &self,
        op: fn(Box<CoreSimExpr>, Box<CoreSimExpr>) -> CoreSimExpr,
        other: &Bound<'_, PyAny>,
        reflected: bool,
    ) -> PyResult<SimExpr> {
        let rhs = to_sim_expr(other)?;
        let (left, right) =
            if reflected { (rhs, self.inner.clone()) } else { (self.inner.clone(), rhs) };
        Ok(SimExpr { inner: op(Box::new(left), Box::new(right)) })
    }
}

#[pymethods]
impl SimExpr {
    /// The previous value of the variable at `index`.
    #[staticmethod]
    fn prev(index: usize) -> Self {
        Self { inner: CoreSimExpr::Prev(index) }
    }

    /// The current time.
    #[staticmethod]
    fn time() -> Self {
        Self { inner: CoreSimExpr::Time }
    }

    #[staticmethod]
    fn constant(value: f64) -> Self {
        Self { inner: CoreSimExpr::Const(value) }
    }

    /// A draw from the noise source at `index`.
    #[staticmethod]
    fn noise(index: usize) -> Self {
        Self { inner: CoreSimExpr::Noise(index) }
    }

    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<SimExpr> {
        self.combine(CoreSimExpr::Add, other, false)
    }

    fn __radd__(&self, other: &Bound<'_, PyAny>) -> PyResult<SimExpr> {
        self.combine(CoreSimExpr::Add, other, true)
    }

    fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<SimExpr> {
        self.combine(CoreSimExpr::Sub, other, false)
    }

    fn __rsub__(&self, other: &Bound<'_, PyAny>) -> PyResult<SimExpr> {
        self.combine(CoreSimExpr::Sub, other, true)
    }

    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<SimExpr> {
        self.combine(CoreSimExpr::Mul, other, false)
    }

    fn __rmul__(&self, other: &Bound<'_, PyAny>) -> PyResult<SimExpr> {
        self.combine(CoreSimExpr::Mul, other, true)
    }

    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<SimExpr> {
        self.combine(CoreSimExpr::Div, other, false)
    }

    fn __rtruediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<SimExpr> {
        self.combine(CoreSimExpr::Div, other, true)
    }

    fn __repr__(&self) -> String {
        format!("SimExpr({:?})", self.inner)
    }
}

/// A declarative stochastic model.
#[pyclass]
pub struct SimModel {
    pub(crate) inner: CoreSimModel,
}

#[pymethods]
impl SimModel {
    #[new]
    fn new(
        py: Python<'_>,
        variables: Vec<String>,
        dt: f64,
        horizon: usize,
        init: Vec<Py<SimExpr>>,
        advance: Vec<Py<SimExpr>>,
        noise: Vec<Py<NoiseModel>>,
    ) -> PyResult<Self> {
        let init = init.iter().map(|e| e.borrow(py).inner.clone()).collect();
        let advance = advance.iter().map(|e| e.borrow(py).inner.clone()).collect();
        let noise = noise.iter().map(|n| n.borrow(py).inner.clone()).collect();
        let inner =
            CoreSimModel::new(variables, dt, horizon, init, advance, noise).map_err(pyerr)?;
        Ok(Self { inner })
    }

    /// Draw one realization of the model as a trace.
    #[pyo3(signature = (seed=42))]
    fn simulate(&self, seed: u64) -> PyResult<Trace> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        Ok(Trace { inner: self.inner.simulate(&mut rng).map_err(pyerr)? })
    }

    fn to_stochastic_system(&self) -> PyResult<StochasticSystem> {
        Ok(StochasticSystem { inner: self.inner.to_stochastic_system().map_err(pyerr)? })
    }

    #[getter]
    fn variables(&self) -> Vec<String> {
        self.inner.variables().to_vec()
    }

    #[getter]
    fn dt(&self) -> f64 {
        self.inner.dt()
    }

    #[getter]
    fn horizon(&self) -> usize {
        self.inner.horizon()
    }

    fn __repr__(&self) -> String {
        format!("SimModel(variables={:?}, horizon={})", self.inner.variables(), self.inner.horizon())
    }
}

/// A stochastic system ready for sampling.
#[pyclass(unsendable)]
pub struct StochasticSystem {
    pub(crate) inner: CoreSystem,
}

#[pymethods]
impl StochasticSystem {
    #[pyo3(signature = (seed=42))]
    fn simulate(&self, seed: u64) -> PyResult<Trace> {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        Ok(Trace { inner: self.inner.simulate(&mut rng).map_err(pyerr)? })
    }

    #[getter]
    fn variables(&self) -> Vec<String> {
        self.inner.variables().to_vec()
    }

    #[getter]
    fn dt(&self) -> f64 {
        self.inner.dt()
    }

    #[getter]
    fn horizon(&self) -> usize {
        self.inner.horizon()
    }

    fn __repr__(&self) -> String {
        format!("StochasticSystem(variables={:?})", self.inner.variables())
    }
}

/// Adaptive multilevel splitting settings for rare-event estimation.
#[pyclass]
#[derive(Clone)]
pub struct RareEventConfig {
    #[pyo3(get, set)]
    pub particles: usize,
    #[pyo3(get, set)]
    pub margin: f64,
    #[pyo3(get, set)]
    pub seed: u64,
}

impl RareEventConfig {
    pub(crate) fn to_core(&self) -> CoreRare {
        CoreRare { particles: self.particles, margin: self.margin, seed: self.seed }
    }
}

#[pymethods]
impl RareEventConfig {
    #[new]
    #[pyo3(signature = (particles=4096, margin=0.0, seed=42))]
    fn new(particles: usize, margin: f64, seed: u64) -> Self {
        Self { particles, margin, seed }
    }
}

/// The outcome of a rare-event estimate.
#[pyclass(frozen)]
pub struct RareEventResult {
    #[pyo3(get)]
    pub probability: f64,
    #[pyo3(get)]
    pub violation_probability: f64,
    #[pyo3(get)]
    pub holds: bool,
    #[pyo3(get)]
    pub simulations: u64,
}

impl RareEventResult {
    fn from_core(result: CoreRareResult) -> Self {
        Self {
            probability: result.probability,
            violation_probability: result.violation_probability,
            holds: result.holds,
            simulations: result.simulations,
        }
    }
}

#[pymethods]
impl RareEventResult {
    fn __repr__(&self) -> String {
        format!("RareEventResult(probability={}, holds={})", self.probability, self.holds)
    }
}

#[pymethods]
impl Formula {
    /// Estimate a rare violation probability by adaptive multilevel splitting.
    #[pyo3(signature = (system, config=None))]
    fn check_rare_event(
        &self,
        system: &StochasticSystem,
        config: Option<RareEventConfig>,
    ) -> PyResult<RareEventResult> {
        let config = config.map(|c| c.to_core()).unwrap_or_default();
        let result = self.inner.check_rare_event(&system.inner, &config).map_err(pyerr)?;
        Ok(RareEventResult::from_core(result))
    }
}

#[pymethods]
impl Monitor {
    /// The rare-event estimate under the monitor's splitting settings.
    fn check_rare(&self, system: &StochasticSystem) -> PyResult<RareEventResult> {
        Ok(RareEventResult::from_core(self.inner.check_rare(&system.inner).map_err(pyerr)?))
    }
}