//! GPU availability and the accelerated rare-event entry point.

use crate::errors::pyerr;
use crate::formula::Formula;
use crate::stats::{RareEventConfig, SimModel};
use pyo3::prelude::*;

/// Whether a compatible GPU device is available for the accelerated paths.
#[pyfunction]
pub fn is_available() -> bool {
    sentil::gpu::is_available()
}

/// A fixed-effort multilevel-splitting estimate produced on the GPU.
#[pyclass(frozen)]
pub struct GpuSplittingEstimate {
    #[pyo3(get)]
    pub violation_probability: f64,
    #[pyo3(get)]
    pub particles: usize,
    #[pyo3(get)]
    pub levels: u32,
}

#[pymethods]
impl GpuSplittingEstimate {
    fn __repr__(&self) -> String {
        format!(
            "GpuSplittingEstimate(violation_probability={}, particles={}, levels={})",
            self.violation_probability, self.particles, self.levels
        )
    }
}

#[pymethods]
impl Formula {
    /// Estimate a rare violation probability on the GPU by multilevel splitting.
    #[pyo3(signature = (model, config=None))]
    fn check_rare_event_gpu(
        &self,
        model: PyRef<'_, SimModel>,
        config: Option<RareEventConfig>,
    ) -> PyResult<GpuSplittingEstimate> {
        let config = config.map(|c| c.to_core()).unwrap_or_default();
        let estimate = self.inner.check_rare_event_gpu(&model.inner, &config).map_err(pyerr)?;
        Ok(GpuSplittingEstimate {
            violation_probability: estimate.violation_probability,
            particles: estimate.particles,
            levels: estimate.levels,
        })
    }
}