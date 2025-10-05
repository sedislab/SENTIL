//! Offline and streaming monitors, the multi-formula monitor, and the bank.

use crate::config::{Config, Interval as PyInterval, Robustness};
use crate::errors::pyerr;
use crate::formula::Formula;
use crate::signal::Trace;
use numpy::{IntoPyArray, PyArray1};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use sentil::{
    Formula as CoreFormula, FormulaBank as CoreBank, Monitor as CoreMonitor,
    MultiFormulaMonitor as CoreMulti, StreamMonitor as CoreStream,
};

fn formula_arg(value: &Bound<'_, PyAny>) -> PyResult<CoreFormula> {
    if let Ok(formula) = value.extract::<PyRef<Formula>>() {
        Ok(formula.inner.clone())
    } else if let Ok(text) = value.extract::<String>() {
        CoreFormula::parse(&text).map_err(pyerr)
    } else {
        Err(PyTypeError::new_err("expected a Formula or its string"))
    }
}

fn named_pairs(values: &Bound<'_, PyDict>) -> PyResult<Vec<(String, f64)>> {
    let mut pairs = Vec::with_capacity(values.len());
    for (name, value) in values.iter() {
        pairs.push((name.extract::<String>()?, value.extract::<f64>()?));
    }
    Ok(pairs)
}

fn refs(pairs: &[(String, f64)]) -> Vec<(&str, f64)> {
    pairs.iter().map(|(name, value)| (name.as_str(), *value)).collect()
}

/// A monitor over one formula.
#[pyclass(unsendable)]
pub struct Monitor {
    pub(crate) inner: CoreMonitor,
}

#[pymethods]
impl Monitor {
    #[new]
    #[pyo3(signature = (formula, config=None))]
    fn new(formula: &Bound<'_, PyAny>, config: Option<Config>) -> PyResult<Self> {
        let formula = formula_arg(formula)?;
        let config = config.map(|c| c.inner).unwrap_or_default();
        Ok(Self { inner: CoreMonitor::from_formula(formula, config) })
    }

    fn robustness(&self, trace: &Trace) -> PyResult<f64> {
        self.inner.robustness(&trace.inner).map_err(pyerr)
    }

    fn robustness_signal<'py>(
        &self,
        py: Python<'py>,
        trace: &Trace,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        Ok(self.inner.robustness_signal(&trace.inner).map_err(pyerr)?.into_pyarray(py))
    }

    fn violations(&self, trace: &Trace) -> PyResult<Vec<PyInterval>> {
        let spans = self.inner.violations(&trace.inner).map_err(pyerr)?;
        Ok(spans.into_iter().map(|(start, end)| PyInterval { start, end }).collect())
    }

    fn update(&mut self, time: f64, values: &Bound<'_, PyDict>) -> PyResult<Robustness> {
        let pairs = named_pairs(values)?;
        let verdict = self.inner.update(time, &refs(&pairs)).map_err(pyerr)?;
        Ok(Robustness::from_core(verdict))
    }

    fn update_packed(&mut self, time: f64, values: Vec<f64>) -> PyResult<Robustness> {
        Ok(Robustness::from_core(self.inner.update_packed(time, &values).map_err(pyerr)?))
    }

    fn symbol_index(&mut self, name: &str) -> PyResult<Option<usize>> {
        self.inner.symbol_index(name).map_err(pyerr)
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    /// The satisfaction probability estimated at the last update.
    fn last_probability(&self) -> Option<f64> {
        self.inner.last_probability()
    }

    #[getter]
    fn formula(&self) -> Formula {
        Formula { inner: self.inner.formula().clone() }
    }

    #[getter]
    fn config(&self) -> Config {
        Config { inner: self.inner.config().clone() }
    }

    fn __repr__(&self) -> String {
        format!("Monitor({})", self.inner.formula())
    }
}

/// A streaming monitor that folds one timestamped sample at a time.
#[pyclass(unsendable)]
pub struct OnlineMonitor {
    inner: CoreStream,
}

#[pymethods]
impl OnlineMonitor {
    #[new]
    fn new(formula: &Bound<'_, PyAny>) -> PyResult<Self> {
        let formula = formula_arg(formula)?;
        Ok(Self { inner: CoreStream::from_formula(&formula).map_err(pyerr)? })
    }

    fn update(&mut self, time: f64, values: &Bound<'_, PyDict>) -> PyResult<Robustness> {
        let pairs = named_pairs(values)?;
        let verdict = self.inner.update(time, &refs(&pairs)).map_err(pyerr)?;
        Ok(Robustness::from_core(verdict))
    }

    fn update_packed(&mut self, time: f64, values: Vec<f64>) -> PyResult<Robustness> {
        Ok(Robustness::from_core(self.inner.update_packed(time, &values).map_err(pyerr)?))
    }

    /// Replay a whole trace, returning the per-sample verdict.
    fn run(&mut self, trace: &Trace) -> PyResult<Vec<Robustness>> {
        let verdicts = self.inner.run(&trace.inner).map_err(pyerr)?;
        Ok(verdicts.into_iter().map(Robustness::from_core).collect())
    }

    fn symbol_index(&self, name: &str) -> Option<usize> {
        self.inner.symbol_index(name)
    }

    #[getter]
    fn variable_count(&self) -> usize {
        self.inner.variable_count()
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn __repr__(&self) -> String {
        format!("OnlineMonitor(variables={})", self.inner.variable_count())
    }
}

/// Several streaming formulas under one clock.
#[pyclass(unsendable)]
pub struct MultiMonitor {
    inner: CoreMulti,
}

#[pymethods]
impl MultiMonitor {
    #[new]
    fn new() -> Self {
        Self { inner: CoreMulti::new() }
    }

    fn add(&mut self, id: String, formula: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(text) = formula.extract::<String>() {
            self.inner.add(id, &text).map_err(pyerr)
        } else {
            let formula = formula.extract::<PyRef<Formula>>()?;
            self.inner.add_formula(id, &formula.inner).map_err(pyerr)
        }
    }

    fn remove(&mut self, id: &str) -> bool {
        self.inner.remove(id)
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    #[getter]
    fn ids(&self) -> Vec<String> {
        self.inner.ids().map(String::from).collect()
    }

    /// Advance every formula to `time` and return each verdict keyed by id.
    fn update<'py>(
        &mut self,
        py: Python<'py>,
        time: f64,
        values: &Bound<'_, PyDict>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let pairs = named_pairs(values)?;
        let results = self.inner.update(time, &refs(&pairs)).map_err(pyerr)?;
        let out = PyDict::new(py);
        for (id, verdict) in results {
            out.set_item(id, Robustness::from_core(verdict))?;
        }
        Ok(out)
    }

    fn __repr__(&self) -> String {
        format!("MultiMonitor(formulas={})", self.inner.len())
    }
}

/// A batch of named formulas evaluated together over one trace.
#[pyclass]
pub struct FormulaBank {
    inner: CoreBank,
}

fn bank_results(
    py: Python<'_>,
    results: Vec<(String, sentil::Result<f64>)>,
) -> PyResult<Bound<'_, PyDict>> {
    let out = PyDict::new(py);
    for (id, value) in results {
        out.set_item(id, value.map_err(pyerr)?)?;
    }
    Ok(out)
}

#[pymethods]
impl FormulaBank {
    #[new]
    fn new() -> Self {
        Self { inner: CoreBank::new() }
    }

    fn add(&mut self, id: String, formula: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(text) = formula.extract::<String>() {
            self.inner.add(id, &text).map_err(pyerr)
        } else {
            let formula = formula.extract::<PyRef<Formula>>()?;
            self.inner.add_formula(id, &formula.inner);
            Ok(())
        }
    }

    #[getter]
    fn ids(&self) -> Vec<String> {
        self.inner.ids().map(String::from).collect()
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// The robustness of every formula over `trace`, keyed by id.
    fn robustness<'py>(&self, py: Python<'py>, trace: &Trace) -> PyResult<Bound<'py, PyDict>> {
        bank_results(py, self.inner.robustness(&trace.inner))
    }

    fn robustness_dense<'py>(&self, py: Python<'py>, trace: &Trace) -> PyResult<Bound<'py, PyDict>> {
        bank_results(py, self.inner.robustness_dense(&trace.inner))
    }

    fn __repr__(&self) -> String {
        format!("FormulaBank(formulas={})", self.inner.len())
    }
}