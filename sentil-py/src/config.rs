//! Monitor configuration and result types.

use pyo3::prelude::*;
use sentil::{MonitorConfig, TimeMode as CoreTimeMode};

/// How time between samples is read.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimeMode {
    Discrete,
    Dense,
}

impl From<TimeMode> for CoreTimeMode {
    fn from(mode: TimeMode) -> Self {
        match mode {
            TimeMode::Discrete => CoreTimeMode::Discrete,
            TimeMode::Dense => CoreTimeMode::Dense,
        }
    }
}

impl From<CoreTimeMode> for TimeMode {
    fn from(mode: CoreTimeMode) -> Self {
        match mode {
            CoreTimeMode::Discrete => TimeMode::Discrete,
            CoreTimeMode::Dense => TimeMode::Dense,
        }
    }
}

/// Monitor settings.
#[pyclass]
#[derive(Clone)]
pub struct Config {
    pub(crate) inner: MonitorConfig,
}

#[pymethods]
impl Config {
    #[new]
    #[pyo3(signature = (time=TimeMode::Discrete))]
    fn new(time: TimeMode) -> Self {
        Self { inner: MonitorConfig::new().time(time.into()) }
    }

    #[getter]
    fn time(&self) -> TimeMode {
        self.inner.time_mode().into()
    }

    fn __repr__(&self) -> String {
        format!("Config(time=TimeMode.{:?})", self.inner.time_mode())
    }
}

/// A robustness verdict.
#[pyclass(frozen)]
pub struct Robustness {
    #[pyo3(get)]
    pub resolved: bool,
    #[pyo3(get)]
    pub satisfied: bool,
    #[pyo3(get)]
    pub value: f64,
    #[pyo3(get)]
    pub lower: f64,
    #[pyo3(get)]
    pub upper: f64,
}

impl Robustness {
    pub(crate) fn from_core(r: sentil::Robustness) -> Self {
        Self {
            resolved: r.is_resolved(),
            satisfied: r.is_satisfied(),
            value: r.value(),
            lower: r.lower(),
            upper: r.upper(),
        }
    }
}

#[pymethods]
impl Robustness {
    fn __float__(&self) -> f64 {
        self.value
    }

    fn __repr__(&self) -> String {
        let sat = if self.satisfied { "True" } else { "False" };
        if self.resolved {
            format!("Robustness(value={}, satisfied={})", self.value, sat)
        } else {
            format!(
                "Robustness(value={}, lower={}, upper={}, satisfied={})",
                self.value, self.lower, self.upper, sat
            )
        }
    }
}

/// A time span [start, end] where a property does not hold.
#[pyclass(frozen)]
pub struct Interval {
    #[pyo3(get)]
    pub start: f64,
    #[pyo3(get)]
    pub end: f64,
}

#[pymethods]
impl Interval {
    fn __repr__(&self) -> String {
        format!("Interval(start={}, end={})", self.start, self.end)
    }
}