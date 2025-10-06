//! The specifications library.

use crate::errors::pyerr;
use crate::formula::Formula;
use crate::monitor::Monitor;
use crate::stats::LiftingRegistry;
use pyo3::prelude::*;
use sentil::spec_builder::{SpecBuilder as CoreBuilder, SpecRegistry, SpecTemplate};
use std::collections::HashMap;

/// A premade specification, customized by variant and parameter.
#[pyclass]
#[derive(Clone)]
pub struct SpecBuilder {
    template: SpecTemplate,
    variants: Vec<String>,
    params: Vec<(String, f64)>,
}

impl SpecBuilder {
    fn resolved(&self) -> PyResult<CoreBuilder> {
        let mut builder = CoreBuilder::new(self.template.clone());
        for variant in &self.variants {
            builder = builder.with_variant(variant).map_err(pyerr)?;
        }
        for (name, value) in &self.params {
            builder = builder.with_param(name, *value).map_err(pyerr)?;
        }
        Ok(builder)
    }
}

#[pymethods]
impl SpecBuilder {
    #[new]
    fn new(name: &str) -> PyResult<SpecBuilder> {
        let builder = SpecRegistry::global().builder(name).map_err(pyerr)?;
        Ok(SpecBuilder { template: builder.template().clone(), variants: Vec::new(), params: Vec::new() })
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
        Ok(SpecBuilder { template, variants: Vec::new(), params: Vec::new() })
    }

    /// Select a named variant, returning the customized builder.
    fn with_variant(&self, variant: &str) -> PyResult<SpecBuilder> {
        let mut next = self.clone();
        next.variants.push(variant.to_owned());
        next.resolved()?;
        Ok(next)
    }

    /// Set a parameter value, returning the customized builder.
    fn with_param(&self, name: &str, value: f64) -> PyResult<SpecBuilder> {
        let mut next = self.clone();
        next.params.push((name.to_owned(), value));
        next.resolved()?;
        Ok(next)
    }

    #[getter]
    fn available_variants(&self) -> PyResult<Vec<String>> {
        Ok(self.resolved()?.available_variants().iter().map(|v| (*v).to_owned()).collect())
    }

    /// The resolved parameter values.
    fn parameters(&self) -> PyResult<HashMap<String, f64>> {
        Ok(self.resolved()?.parameters())
    }

    fn build_deterministic(&self) -> PyResult<String> {
        self.resolved()?.build_deterministic().map_err(pyerr)
    }

    fn build_probabilistic(&self) -> PyResult<String> {
        self.resolved()?.build_probabilistic().map_err(pyerr)
    }

    fn build_formula(&self) -> PyResult<Formula> {
        Ok(Formula { inner: self.resolved()?.build_formula().map_err(pyerr)? })
    }

    fn build_probabilistic_formula(&self) -> PyResult<Formula> {
        Ok(Formula { inner: self.resolved()?.build_probabilistic_formula().map_err(pyerr)? })
    }

    fn build_lifting_registry(&self) -> PyResult<LiftingRegistry> {
        Ok(LiftingRegistry { inner: self.resolved()?.build_lifting_registry().map_err(pyerr)? })
    }

    /// Build a monitor from the customized specification.
    fn build_monitor(&self) -> PyResult<Monitor> {
        Ok(Monitor { inner: self.resolved()?.into_monitor().map_err(pyerr)? })
    }

    fn __repr__(&self) -> String {
        format!("SpecBuilder(variants={:?}, params={:?})", self.variants, self.params)
    }
}