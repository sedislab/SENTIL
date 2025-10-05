//! Smooth robustness, the numerics, and controller synthesis from a spec.

use crate::errors::pyerr;
use crate::formula::Formula;
use crate::signal::Trace;
use numpy::{IntoPyArray, PyArray1};
use pyo3::prelude::*;
use sentil::synthesis::{
    soft_max as core_soft_max, soft_min as core_soft_min, solve_qp as core_solve_qp,
    solve_spd as core_solve_spd, symmetric_eigen as core_symmetric_eigen, SmoothConfig as CoreSmooth,
};
use sentil::SoftKind as CoreSoftKind;

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

/// The eigenvalues, then the eigenvectors as rows, of a symmetric matrix.
type Eigen<'py> = (Bound<'py, PyArray1<f64>>, Vec<Vec<f64>>);

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