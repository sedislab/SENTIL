//! Smooth robustness, the numerics, and controller synthesis from a spec.

use crate::errors::pyerr;
use crate::formula::Formula;
use crate::signal::Trace;
use crate::stats::StochasticSystem;
use numpy::{IntoPyArray, PyArray1};
use pyo3::prelude::*;
use sentil::synthesis::{
    soft_max as core_soft_max, soft_min as core_soft_min, solve_qp as core_solve_qp,
    solve_spd as core_solve_spd, symmetric_eigen as core_symmetric_eigen, AffineForm,
    Bounds as CoreBounds, ChanceConstraint as CoreChance, CmaConfig as CoreCma,
    Controller as CoreController, LinearModel, SafetyFilter as CoreFilter, SmoothConfig as CoreSmooth,
    SynthesisProblem, Synthesizer, SystemModel as CoreSystemModel, Witness as CoreWitness,
};
use sentil::{Backend as CoreBackend, SoftKind as CoreSoftKind, Trace as CoreTrace};
use std::sync::Arc;

struct DynModel<'a>(&'a dyn CoreSystemModel);

impl CoreSystemModel for DynModel<'_> {
    fn input_dimension(&self) -> usize {
        self.0.input_dimension()
    }

    fn initial_state(&self) -> &[f64] {
        self.0.initial_state()
    }

    fn rollout_from(&self, initial: &[f64], input: &[f64]) -> sentil::Result<CoreTrace> {
        self.0.rollout_from(initial, input)
    }

    fn affine_form(&self) -> Option<AffineForm> {
        self.0.affine_form()
    }
}

/// The soft minimum and maximum kind used by the smooth interpreter.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoftKind {
    LogSumExp,
    ArithmeticGeometricMean,
}

impl From<SoftKind> for CoreSoftKind {
    fn from(kind: SoftKind) -> Self {
        match kind {
            SoftKind::LogSumExp => CoreSoftKind::LogSumExp,
            SoftKind::ArithmeticGeometricMean => CoreSoftKind::ArithmeticGeometricMean,
        }
    }
}

/// Smooth-robustness settings.
#[pyclass]
#[derive(Clone)]
pub struct SmoothConfig {
    #[pyo3(get, set)]
    pub temperature: f64,
    #[pyo3(get, set)]
    pub kind: SoftKind,
}

impl SmoothConfig {
    fn to_core(&self) -> PyResult<CoreSmooth> {
        Ok(CoreSmooth::new(self.temperature).map_err(pyerr)?.with_kind(self.kind.into()))
    }
}

fn smooth_or_default(config: Option<SmoothConfig>) -> PyResult<CoreSmooth> {
    Ok(config.map(|c| c.to_core()).transpose()?.unwrap_or_default())
}

#[pymethods]
impl SmoothConfig {
    #[new]
    #[pyo3(signature = (temperature=10.0, kind=SoftKind::LogSumExp))]
    fn new(temperature: f64, kind: SoftKind) -> Self {
        Self { temperature, kind }
    }

    fn __repr__(&self) -> String {
        format!("SmoothConfig(temperature={}, kind=SoftKind.{:?})", self.temperature, self.kind)
    }
}

/// The smooth (differentiable) minimum of `values` at `temperature`.
#[pyfunction]
pub fn soft_min(values: Vec<f64>, temperature: f64) -> f64 {
    core_soft_min(&values, temperature)
}

/// The smooth (differentiable) maximum of `values` at `temperature`.
#[pyfunction]
pub fn soft_max(values: Vec<f64>, temperature: f64) -> f64 {
    core_soft_max(&values, temperature)
}

/// Solve the small quadratic program `min 1/2 uPu + qu` subject to `Gu <= h`.
#[pyfunction]
#[pyo3(signature = (p, q, g, h, max_iters=200))]
pub fn solve_qp<'py>(
    py: Python<'py>,
    p: Vec<Vec<f64>>,
    q: Vec<f64>,
    g: Vec<Vec<f64>>,
    h: Vec<f64>,
    max_iters: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let solution = core_solve_qp(&p, &q, &g, &h, max_iters).map_err(pyerr)?;
    Ok(solution.into_pyarray(py))
}

/// Solve `A x = b` for a symmetric positive-definite `A`.
#[pyfunction]
pub fn solve_spd<'py>(
    py: Python<'py>,
    matrix: Vec<Vec<f64>>,
    rhs: Vec<f64>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let solution = core_solve_spd(&matrix, &rhs).map_err(pyerr)?;
    Ok(solution.into_pyarray(py))
}

type Eigen<'py> = (Bound<'py, PyArray1<f64>>, Vec<Vec<f64>>);

/// The eigenvalues, then the eigenvectors as rows, of a symmetric matrix.
#[pyfunction]
pub fn symmetric_eigen(py: Python<'_>, matrix: Vec<Vec<f64>>) -> PyResult<Eigen<'_>> {
    let (values, vectors) = core_symmetric_eigen(&matrix).map_err(pyerr)?;
    Ok((values.into_pyarray(py), vectors))
}

#[pymethods]
impl Formula {
    /// The differentiable robustness used by the synthesis optimizers.
    #[pyo3(signature = (trace, config=None))]
    fn smooth_robustness(&self, trace: &Trace, config: Option<SmoothConfig>) -> PyResult<f64> {
        self.inner.smooth_robustness(&trace.inner, smooth_or_default(config)?).map_err(pyerr)
    }
}

/// The optimization backend the synthesizer uses.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Auto,
    Gradient,
    CmaEs,
    Milp,
}

impl From<Backend> for CoreBackend {
    fn from(backend: Backend) -> Self {
        match backend {
            Backend::Auto => CoreBackend::Auto,
            Backend::Gradient => CoreBackend::Gradient,
            Backend::CmaEs => CoreBackend::CmaEs,
            Backend::Milp => CoreBackend::Milp,
        }
    }
}

impl From<CoreBackend> for Backend {
    fn from(backend: CoreBackend) -> Self {
        match backend {
            CoreBackend::Auto => Backend::Auto,
            CoreBackend::Gradient => Backend::Gradient,
            CoreBackend::CmaEs => Backend::CmaEs,
            CoreBackend::Milp => Backend::Milp,
        }
    }
}

/// Per-coordinate input bounds for synthesis.
#[pyclass]
#[derive(Clone)]
pub struct Bounds {
    lower: Vec<f64>,
    upper: Vec<f64>,
}

impl Bounds {
    fn to_core(&self) -> PyResult<CoreBounds> {
        CoreBounds::new(self.lower.clone(), self.upper.clone()).map_err(pyerr)
    }
}

#[pymethods]
impl Bounds {
    #[new]
    fn new(lower: Vec<f64>, upper: Vec<f64>) -> PyResult<Bounds> {
        CoreBounds::new(lower.clone(), upper.clone()).map_err(pyerr)?;
        Ok(Bounds { lower, upper })
    }

    /// Open bounds over `dimension` coordinates.
    #[staticmethod]
    fn unbounded(dimension: usize) -> Bounds {
        let bounds = CoreBounds::unbounded(dimension);
        Bounds { lower: bounds.lower().to_vec(), upper: bounds.upper().to_vec() }
    }

    #[getter]
    fn lower(&self) -> Vec<f64> {
        self.lower.clone()
    }

    #[getter]
    fn upper(&self) -> Vec<f64> {
        self.upper.clone()
    }

    #[getter]
    fn dimension(&self) -> usize {
        self.lower.len()
    }
}

type SharedModel = Arc<dyn CoreSystemModel + Send + Sync>;

/// A dynamical system the synthesizer drives.
#[pyclass]
pub struct SystemModel {
    inner: SharedModel,
}

#[pymethods]
impl SystemModel {
    /// A linear time-invariant model x_{t+1} = A x_t + B u_t over `horizon` steps.
    #[staticmethod]
    fn linear(
        a: Vec<Vec<f64>>,
        b: Vec<Vec<f64>>,
        x0: Vec<f64>,
        variables: Vec<String>,
        dt: f64,
        horizon: usize,
    ) -> PyResult<SystemModel> {
        let model = LinearModel::new(a, b, x0, variables, dt, horizon).map_err(pyerr)?;
        Ok(SystemModel { inner: Arc::new(model) })
    }

    #[getter]
    fn input_dimension(&self) -> usize {
        self.inner.input_dimension()
    }
}

/// The synthesized input and its robustness.
#[pyclass(frozen)]
pub struct SynthesisResult {
    #[pyo3(get)]
    pub input: Py<PyArray1<f64>>,
    #[pyo3(get)]
    pub robustness: f64,
    #[pyo3(get)]
    pub holds: bool,
    #[pyo3(get)]
    pub backend: Backend,
}

#[pymethods]
impl SynthesisResult {
    fn __repr__(&self) -> String {
        format!("SynthesisResult(robustness={}, holds={})", self.robustness, self.holds)
    }
}

/// Synthesize an input sequence that satisfies `spec` on `model`.
#[pyfunction]
#[pyo3(signature = (model, spec, bounds=None, smooth=None, backend=Backend::Auto, max_iters=0, population=0))]
#[allow(clippy::too_many_arguments)]
pub fn synthesize(
    py: Python<'_>,
    model: PyRef<'_, SystemModel>,
    spec: PyRef<'_, Formula>,
    bounds: Option<PyRef<'_, Bounds>>,
    smooth: Option<SmoothConfig>,
    backend: Backend,
    max_iters: usize,
    population: usize,
) -> PyResult<SynthesisResult> {
    let dyn_model = DynModel(&*model.inner);
    let mut problem = SynthesisProblem::new(&dyn_model, &spec.inner).with_backend(backend.into());
    if max_iters != 0 {
        problem = problem.with_budget(max_iters);
    }
    if population != 0 {
        problem = problem.with_population(population);
    }
    if let Some(bounds) = &bounds {
        problem = problem.with_bounds(bounds.to_core()?);
    }
    if let Some(smooth) = smooth {
        problem = problem.with_smooth(smooth.to_core()?);
    }
    let result = Synthesizer::solve(&problem).map_err(pyerr)?;
    Ok(SynthesisResult {
        input: result.input.into_pyarray(py).unbind(),
        robustness: result.robustness,
        holds: result.holds,
        backend: result.backend.into(),
    })
}

/// A least-restrictive safety shield.
#[pyclass]
pub struct SafetyFilter {
    inner: CoreFilter,
}

#[pymethods]
impl SafetyFilter {
    #[new]
    fn new(bounds: PyRef<'_, Bounds>) -> PyResult<SafetyFilter> {
        Ok(SafetyFilter { inner: CoreFilter::new(bounds.to_core()?) })
    }

    /// Project `nominal` into the bounds and the barriers `a . u >= b`.
    #[pyo3(signature = (nominal, barriers=Vec::new()))]
    fn filter<'py>(
        &self,
        py: Python<'py>,
        nominal: Vec<f64>,
        barriers: Vec<(Vec<f64>, f64)>,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let filtered = self.inner.filter(&nominal, &barriers).map_err(pyerr)?;
        Ok(filtered.into_pyarray(py))
    }
}

/// A chance-constraint report.
#[pyclass(frozen)]
pub struct ChanceReport {
    #[pyo3(get)]
    pub estimate: f64,
    #[pyo3(get)]
    pub lower_bound: f64,
    #[pyo3(get)]
    pub samples: u64,
    #[pyo3(get)]
    pub holds: bool,
}

#[pymethods]
impl ChanceReport {
    fn __repr__(&self) -> String {
        format!("ChanceReport(estimate={}, lower_bound={}, holds={})", self.estimate, self.lower_bound, self.holds)
    }
}

/// A probabilistic satisfaction requirement validated against a stochastic system.
#[pyclass]
pub struct ChanceConstraint {
    inner: CoreChance,
}

#[pymethods]
impl ChanceConstraint {
    #[new]
    #[pyo3(signature = (spec, probability, confidence=0.0, tightening=0.0))]
    fn new(
        spec: PyRef<'_, Formula>,
        probability: f64,
        confidence: f64,
        tightening: f64,
    ) -> PyResult<ChanceConstraint> {
        let mut inner = CoreChance::new(spec.inner.clone(), probability).map_err(pyerr)?;
        if confidence > 0.0 {
            inner = inner.with_confidence(confidence);
        }
        if tightening != 0.0 {
            inner = inner.with_tightening(tightening);
        }
        Ok(ChanceConstraint { inner })
    }

    /// Estimate satisfaction over `samples` realizations and report the bound.
    #[pyo3(signature = (system, samples=1000, seed=42))]
    fn validate(
        &self,
        system: PyRef<'_, StochasticSystem>,
        samples: u64,
        seed: u64,
    ) -> PyResult<ChanceReport> {
        let report = self.inner.validate(&system.inner, samples, seed).map_err(pyerr)?;
        Ok(ChanceReport {
            estimate: report.estimate,
            lower_bound: report.lower_bound,
            samples: report.samples,
            holds: report.holds,
        })
    }
}

/// A witnessing trajectory.
#[pyclass(frozen)]
pub struct Witness {
    #[pyo3(get)]
    pub input: Py<PyArray1<f64>>,
    #[pyo3(get)]
    pub robustness: f64,
    #[pyo3(get)]
    pub trace: Py<Trace>,
}

fn pack_witness(py: Python<'_>, witness: CoreWitness) -> PyResult<Witness> {
    Ok(Witness {
        input: witness.input.into_pyarray(py).unbind(),
        robustness: witness.robustness,
        trace: Py::new(py, Trace { inner: witness.trace })?,
    })
}

/// CMA-ES settings used by the falsifier and the black-box backend.
#[pyclass]
#[derive(Clone)]
pub struct CmaConfig {
    #[pyo3(get, set)]
    pub population: usize,
    #[pyo3(get, set)]
    pub max_generations: usize,
    #[pyo3(get, set)]
    pub initial_step: f64,
    #[pyo3(get, set)]
    pub tol_step: f64,
    #[pyo3(get, set)]
    pub seed: u64,
}

impl CmaConfig {
    fn to_core(&self) -> CoreCma {
        CoreCma {
            population: self.population,
            max_generations: self.max_generations,
            initial_step: self.initial_step,
            tol_step: self.tol_step,
            seed: self.seed,
        }
    }
}

#[pymethods]
impl CmaConfig {
    #[new]
    #[pyo3(signature = (population=32, max_generations=200, initial_step=0.3, tol_step=1e-9, seed=42))]
    fn new(
        population: usize,
        max_generations: usize,
        initial_step: f64,
        tol_step: f64,
        seed: u64,
    ) -> Self {
        Self { population, max_generations, initial_step, tol_step, seed }
    }
}

#[pymethods]
impl Formula {
    /// Search for a violating trajectory by gradient descent on the smooth robustness.
    #[pyo3(signature = (model, bounds, max_iters=200, smooth=None))]
    fn find_counterexample(
        &self,
        py: Python<'_>,
        model: PyRef<'_, SystemModel>,
        bounds: PyRef<'_, Bounds>,
        max_iters: usize,
        smooth: Option<SmoothConfig>,
    ) -> PyResult<Witness> {
        let dyn_model = DynModel(&*model.inner);
        let witness = self
            .inner
            .find_counterexample(&dyn_model, &bounds.to_core()?, max_iters, smooth_or_default(smooth)?)
            .map_err(pyerr)?;
        pack_witness(py, witness)
    }

    /// Search for a violating trajectory with CMA-ES and random restarts.
    #[pyo3(signature = (model, bounds, config=None, restarts=1))]
    fn falsify(
        &self,
        py: Python<'_>,
        model: PyRef<'_, SystemModel>,
        bounds: PyRef<'_, Bounds>,
        config: Option<CmaConfig>,
        restarts: usize,
    ) -> PyResult<Witness> {
        let dyn_model = DynModel(&*model.inner);
        let config = config.map(|c| c.to_core()).unwrap_or_default();
        let witness = self
            .inner
            .falsify(&dyn_model, &bounds.to_core()?, config, restarts)
            .map_err(pyerr)?;
        pack_witness(py, witness)
    }
}

/// A receding-horizon controller that emits a control input within a deadline.
#[pyclass(unsendable)]
pub struct Controller {
    controller: Option<CoreController<'static, DynModel<'static>>>,
    dyn_model: *mut DynModel<'static>,
    spec: *mut Formula,
    model: *const (dyn CoreSystemModel + Send + Sync),
}

impl Drop for Controller {
    fn drop(&mut self) {
        // The controller borrows the leaked boxes, so it drops first.
        self.controller = None;
        unsafe {
            drop(Box::from_raw(self.dyn_model));
            drop(Box::from_raw(self.spec));
            drop(Arc::from_raw(self.model));
        }
    }
}

#[pymethods]
impl Controller {
    #[new]
    #[pyo3(signature = (model, spec, input_width, budget_ns, bounds=None, smooth=None))]
    fn new(
        model: PyRef<'_, SystemModel>,
        spec: PyRef<'_, Formula>,
        input_width: usize,
        budget_ns: u64,
        bounds: Option<PyRef<'_, Bounds>>,
        smooth: Option<SmoothConfig>,
    ) -> PyResult<Controller> {
        let bounds = bounds.map(|b| b.to_core()).transpose()?;
        let smooth = smooth.map(|s| s.to_core()).transpose()?;
        let model_ptr = Arc::into_raw(model.inner.clone());
        let dyn_model = Box::into_raw(Box::new(DynModel(unsafe { &*model_ptr })));
        let spec = Box::into_raw(Box::new(Formula { inner: spec.inner.clone() }));
        let mut controller = CoreController::new(
            unsafe { &(*dyn_model) },
            unsafe { &(*spec).inner },
            input_width,
            std::time::Duration::from_nanos(budget_ns),
        );
        if let Some(bounds) = bounds {
            controller = controller.with_bounds(bounds);
        }
        if let Some(smooth) = smooth {
            controller = controller.with_smooth(smooth);
        }
        Ok(Controller { controller: Some(controller), dyn_model, spec, model: model_ptr })
    }

    /// Emit the next control input for the measured `state`, within the deadline.
    fn control<'py>(
        &mut self,
        py: Python<'py>,
        state: Vec<f64>,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let controller = self.controller.as_mut().expect("controller is live until drop");
        let input = controller.control(&state).map_err(pyerr)?;
        Ok(input.into_pyarray(py))
    }
}