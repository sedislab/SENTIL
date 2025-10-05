//! The specifications library.

use crate::errors::pyerr;
use crate::formula::Formula;
use crate::monitor::Monitor;
use crate::stats::LiftingRegistry;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use sentil::spec_builder::{SpecBuilder as CoreBuilder, SpecRegistry};
use std::collections::HashMap;

/// A premade specification, customized by variant and parameter, then built into a
/// formula, a lifting registry, or a monitor.
#[pyclass]
pub struct SpecBuilder {
    inner: Option<CoreBuilder>,
}

impl SpecBuilder {
    fn get(&self) -> PyResult<&CoreBuilder> {
        self.inner
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("this spec builder has been turned into a monitor"))
    }

    fn take(&mut self) -> PyResult<CoreBuilder> {
        self.inner
            .take()
            .ok_or_else(|| PyRuntimeError::new_err("this spec builder has been turned into a monitor"))
    }
}

#[pymethods]
impl SpecBuilder {
    #[new]
    fn new(name: &str) -> PyResult<SpecBuilder> {
        Ok(SpecBuilder { inner: Some(SpecRegistry::global().builder(name).map_err(pyerr)?) })
    }

    /// The names of every specification in the library.
    #[staticmethod]
    fn available() -> Vec<String> {
        SpecRegistry::global().available()
    }

    /// Load a specification template from a TOML file.
    #[staticmethod]
    fn from_file(path: &str) -> PyResult<SpecBuilder> {
        let template = SpecRegistry::global().load_file(path).map_err(pyerr)?;
        Ok(SpecBuilder { inner: Some(CoreBuilder::new(template)) })
    }

    /// Select a named variant.
    fn with_variant(&mut self, variant: &str) -> PyResult<()> {
        self.inner = Some(self.take()?.with_variant(variant).map_err(pyerr)?);
        Ok(())
    }

    /// Set a parameter value.
    fn with_param(&mut self, name: &str, value: f64) -> PyResult<()> {
        self.inner = Some(self.take()?.with_param(name, value).map_err(pyerr)?);
        Ok(())
    }

    #[getter]
    fn available_variants(&self) -> PyResult<Vec<String>> {
        Ok(self.get()?.available_variants().iter().map(|v| (*v).to_owned()).collect())
    }

    /// The resolved parameter values.
    fn parameters(&self) -> PyResult<HashMap<String, f64>> {
        Ok(self.get()?.parameters())
    }

    fn build_deterministic(&self) -> PyResult<String> {
        self.get()?.build_deterministic().map_err(pyerr)
    }

    fn build_probabilistic(&self) -> PyResult<String> {
        self.get()?.build_probabilistic().map_err(pyerr)
    }

    fn build_formula(&self) -> PyResult<Formula> {
        Ok(Formula { inner: self.get()?.build_formula().map_err(pyerr)? })
    }

    fn build_probabilistic_formula(&self) -> PyResult<Formula> {
        Ok(Formula { inner: self.get()?.build_probabilistic_formula().map_err(pyerr)? })
    }

    fn build_lifting_registry(&self) -> PyResult<LiftingRegistry> {
        Ok(LiftingRegistry { inner: self.get()?.build_lifting_registry().map_err(pyerr)? })
    }

    /// Build a monitor from the customized specification, consuming the builder.
    fn build_monitor(&mut self) -> PyResult<Monitor> {
        Ok(Monitor { inner: self.take()?.into_monitor().map_err(pyerr)? })
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            Some(builder) => format!("SpecBuilder(variants={:?})", builder.available_variants()),
            None => "SpecBuilder(consumed)".to_owned(),
        }
    }
}