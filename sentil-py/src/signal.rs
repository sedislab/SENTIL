//! Traces, prepared traces, and the streaming ring buffer.

use crate::errors::pyerr;
use numpy::{IntoPyArray, PyArray1};
use pyo3::exceptions::{PyIndexError, PyKeyError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use sentil::{
    Interpolation as CoreInterp, PreparedTrace as CorePrepared, RingBuffer as CoreRing,
    Trace as CoreTrace,
};

/// How a value between two samples is read off a dense-time trace.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    Linear,
    ZeroOrderHold,
    CubicSpline,
}

impl From<Interpolation> for CoreInterp {
    fn from(interp: Interpolation) -> Self {
        match interp {
            Interpolation::Linear => CoreInterp::Linear,
            Interpolation::ZeroOrderHold => CoreInterp::ZeroOrderHold,
            Interpolation::CubicSpline => CoreInterp::CubicSpline,
        }
    }
}

/// A multivariate signal.
#[pyclass]
pub struct Trace {
    pub(crate) inner: CoreTrace,
}

fn add_dict_signals(trace: &mut CoreTrace, signals: &Bound<'_, PyDict>) -> PyResult<()> {
    for (key, value) in signals.iter() {
        let name: String = key.extract()?;
        let values: Vec<f64> = value.extract()?;
        trace.add_signal(&name, values).map_err(pyerr)?;
    }
    Ok(())
}

#[pymethods]
impl Trace {
    /// Build a trace from a time vector and an optional mapping of name to values.
    #[new]
    #[pyo3(signature = (times, signals=None))]
    fn new(times: Vec<f64>, signals: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut inner = CoreTrace::new(times).map_err(pyerr)?;
        if let Some(signals) = signals {
            add_dict_signals(&mut inner, signals)?;
        }
        Ok(Self { inner })
    }

    /// A trace on the integer grid 0, 1, ..., len-1.
    #[staticmethod]
    fn indexed(len: usize) -> Self {
        Self { inner: CoreTrace::indexed(len) }
    }

    /// Parse a trace from CSV held in a string.
    ///
    /// >>> len(Trace.from_csv("t,x\n0,1\n1,2"))
    /// 2
    #[staticmethod]
    fn from_csv(text: &str) -> PyResult<Self> {
        Ok(Self { inner: CoreTrace::from_csv_str(text).map_err(pyerr)? })
    }

    /// Parse a trace from tab-separated text.
    #[staticmethod]
    fn from_tsv(text: &str) -> PyResult<Self> {
        Ok(Self { inner: CoreTrace::from_tsv_str(text).map_err(pyerr)? })
    }

    /// Read a trace from a file, dispatching on the extension.
    #[staticmethod]
    fn from_path(path: &str) -> PyResult<Self> {
        Ok(Self { inner: CoreTrace::from_path(path).map_err(pyerr)? })
    }

    fn add_signal(&mut self, name: &str, values: Vec<f64>) -> PyResult<()> {
        self.inner.add_signal(name, values).map_err(pyerr)
    }

    fn add_signals(&mut self, signals: &Bound<'_, PyDict>) -> PyResult<()> {
        add_dict_signals(&mut self.inner, signals)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    #[getter]
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[getter]
    fn times<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.inner.times().to_vec().into_pyarray(py)
    }

    #[getter]
    fn variables(&self) -> Vec<String> {
        self.inner.variables().into_iter().map(String::from).collect()
    }

    /// The value array for one variable.
    fn __getitem__<'py>(&self, py: Python<'py>, name: &str) -> PyResult<Bound<'py, PyArray1<f64>>> {
        match self.inner.signal(name) {
            Some(values) => Ok(values.to_vec().into_pyarray(py)),
            None => Err(PyKeyError::new_err(name.to_owned())),
        }
    }

    /// The value array for one variable, or `default`.
    #[pyo3(signature = (name, default=None))]
    fn get<'py>(
        &self,
        py: Python<'py>,
        name: &str,
        default: Option<Bound<'py, PyAny>>,
    ) -> Option<Bound<'py, PyAny>> {
        match self.inner.signal(name) {
            Some(values) => Some(values.to_vec().into_pyarray(py).into_any()),
            None => default,
        }
    }

    fn __contains__(&self, name: &str) -> bool {
        self.inner.signal(name).is_some()
    }

    /// Iterate the variable names.
    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let names: Vec<String> = self.inner.variables().into_iter().map(String::from).collect();
        Ok(PyList::new(py, names)?.into_any().call_method0("__iter__")?.unbind())
    }

    fn resample(&self, times: Vec<f64>, interp: Interpolation) -> PyResult<Trace> {
        let inner = self.inner.resample(times, interp.into()).map_err(pyerr)?;
        Ok(Trace { inner })
    }

    /// Fix the interpolation coefficients for repeated resampling.
    fn prepare(&self, interp: Interpolation) -> PreparedTrace {
        PreparedTrace { inner: self.inner.prepare(interp.into()) }
    }

    fn __repr__(&self) -> String {
        format!("Trace(len={}, variables={:?})", self.inner.len(), self.inner.variables())
    }
}

/// A trace with its interpolation coefficients precomputed.
#[pyclass]
pub struct PreparedTrace {
    inner: CorePrepared,
}

#[pymethods]
impl PreparedTrace {
    fn resample(&self, times: Vec<f64>) -> PyResult<Trace> {
        Ok(Trace { inner: self.inner.resample(times).map_err(pyerr)? })
    }
}

/// A fixed-capacity window over the most recent samples, with rolling statistics.
#[pyclass]
pub struct RingBuffer {
    inner: CoreRing,
}

#[pymethods]
impl RingBuffer {
    #[new]
    fn new(capacity: usize) -> PyResult<Self> {
        Ok(Self { inner: CoreRing::new(capacity).map_err(pyerr)? })
    }

    /// Append a sample, returning the evicted oldest sample.
    fn push(&mut self, time: f64, value: f64) -> PyResult<Option<(f64, f64)>> {
        self.inner.push(time, value).map_err(pyerr)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    #[getter]
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    #[getter]
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[getter]
    fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    fn clear(&mut self) {
        self.inner.clear();
    }

    /// The sample at `index`, counting from the oldest, with negative indexing.
    fn __getitem__(&self, index: isize) -> PyResult<(f64, f64)> {
        let resolved = if index < 0 { index + self.inner.len() as isize } else { index };
        usize::try_from(resolved)
            .ok()
            .and_then(|i| self.inner.get(i))
            .ok_or_else(|| PyIndexError::new_err("ring buffer index out of range"))
    }

    fn get(&self, index: usize) -> Option<(f64, f64)> {
        self.inner.get(index)
    }

    fn front(&self) -> Option<(f64, f64)> {
        self.inner.front()
    }

    fn back(&self) -> Option<(f64, f64)> {
        self.inner.back()
    }

    fn pop_front(&mut self) -> Option<(f64, f64)> {
        self.inner.pop_front()
    }

    fn pop_back(&mut self) -> Option<(f64, f64)> {
        self.inner.pop_back()
    }

    fn mean(&self) -> Option<f64> {
        self.inner.mean()
    }

    fn variance(&self) -> Option<f64> {
        self.inner.variance()
    }

    fn std_dev(&self) -> Option<f64> {
        self.inner.std_dev()
    }

    fn min(&self) -> Option<f64> {
        self.inner.min()
    }

    fn max(&self) -> Option<f64> {
        self.inner.max()
    }

    fn recompute_statistics(&mut self) {
        self.inner.recompute_statistics();
    }

    fn time_range(&self) -> Option<(f64, f64)> {
        self.inner.time_range()
    }

    fn at_time(&self, time: f64) -> Option<f64> {
        self.inner.at_time(time)
    }

    fn closest_to_time(&self, time: f64) -> Option<(f64, f64)> {
        self.inner.closest_to_time(time)
    }

    fn between(&self, start: f64, end: f64) -> Vec<(f64, f64)> {
        self.inner.between(start, end).collect()
    }

    fn __repr__(&self) -> String {
        format!("RingBuffer(len={}, capacity={})", self.inner.len(), self.inner.capacity())
    }
}