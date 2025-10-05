//! The `_sentil` extension module.

use pyo3::prelude::*;

mod config;
mod errors;
mod formula;
mod monitor;
mod signal;
mod stats;
mod synthesis;

use config::{Config, Interval, Robustness, TimeMode};
use errors::{EvaluationError, ParseError, SemanticError, SentilError};
use formula::{Expr, Formula};
use monitor::{FormulaBank, Monitor, MultiMonitor, OnlineMonitor};
use pyo3::wrap_pyfunction;
use signal::{Interpolation, PreparedTrace, RingBuffer, Trace};
use stats::{
    BayesConfig, BayesResult, BayesVerdict, ConfidenceInterval, IntervalMethod, LiftingRegistry,
    NoiseInteraction, NoiseModel, RareEventConfig, RareEventResult, RobustnessDistribution,
    SimExpr, SimModel, SmcConfig, SmcResult, SprtConfig, SprtResult, SprtVerdict, StochasticSystem,
};
use synthesis::{
    Backend, Bounds, ChanceConstraint, ChanceReport, CmaConfig, Controller, SafetyFilter,
    SmoothConfig, SoftKind, SynthesisResult, SystemModel, Witness,
};

fn register_synthesis(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(m.py(), "synthesis")?;
    module.add_function(wrap_pyfunction!(synthesis::soft_min, &module)?)?;
    module.add_function(wrap_pyfunction!(synthesis::soft_max, &module)?)?;
    module.add_function(wrap_pyfunction!(synthesis::solve_qp, &module)?)?;
    module.add_function(wrap_pyfunction!(synthesis::solve_spd, &module)?)?;
    module.add_function(wrap_pyfunction!(synthesis::symmetric_eigen, &module)?)?;
    module.add_function(wrap_pyfunction!(synthesis::synthesize, &module)?)?;
    m.add_submodule(&module)
}

fn register_stats(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(m.py(), "stats")?;
    module.add_function(wrap_pyfunction!(stats::wilson_interval, &module)?)?;
    module.add_function(wrap_pyfunction!(stats::clopper_pearson, &module)?)?;
    module.add_function(wrap_pyfunction!(stats::jeffreys_interval, &module)?)?;
    module.add_function(wrap_pyfunction!(stats::agresti_coull, &module)?)?;
    module.add_function(wrap_pyfunction!(stats::interval, &module)?)?;
    module.add_function(wrap_pyfunction!(stats::z_score, &module)?)?;
    module.add_function(wrap_pyfunction!(stats::chernoff_hoeffding_samples, &module)?)?;
    module.add_function(wrap_pyfunction!(stats::wilson_samples, &module)?)?;
    m.add_submodule(&module)
}

#[pymodule]
fn _sentil(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("SentilError", m.py().get_type::<SentilError>())?;
    m.add("ParseError", m.py().get_type::<ParseError>())?;
    m.add("SemanticError", m.py().get_type::<SemanticError>())?;
    m.add("EvaluationError", m.py().get_type::<EvaluationError>())?;

    m.add_class::<TimeMode>()?;
    m.add_class::<Config>()?;
    m.add_class::<Robustness>()?;
    m.add_class::<Interval>()?;

    m.add_class::<Interpolation>()?;
    m.add_class::<Trace>()?;
    m.add_class::<PreparedTrace>()?;
    m.add_class::<RingBuffer>()?;

    m.add_class::<Expr>()?;
    m.add_class::<Formula>()?;

    m.add_class::<Monitor>()?;
    m.add_class::<OnlineMonitor>()?;
    m.add_class::<MultiMonitor>()?;
    m.add_class::<FormulaBank>()?;

    m.add_class::<ConfidenceInterval>()?;
    m.add_class::<IntervalMethod>()?;
    m.add_class::<NoiseModel>()?;
    m.add_class::<NoiseInteraction>()?;
    m.add_class::<LiftingRegistry>()?;
    m.add_class::<SmcConfig>()?;
    m.add_class::<SmcResult>()?;
    m.add_class::<RobustnessDistribution>()?;
    m.add_class::<SprtConfig>()?;
    m.add_class::<SprtResult>()?;
    m.add_class::<SprtVerdict>()?;
    m.add_class::<BayesConfig>()?;
    m.add_class::<BayesResult>()?;
    m.add_class::<BayesVerdict>()?;
    m.add_class::<SimExpr>()?;
    m.add_class::<SimModel>()?;
    m.add_class::<StochasticSystem>()?;
    m.add_class::<RareEventConfig>()?;
    m.add_class::<RareEventResult>()?;
    register_stats(m)?;

    m.add_class::<SoftKind>()?;
    m.add_class::<SmoothConfig>()?;
    m.add_class::<Backend>()?;
    m.add_class::<Bounds>()?;
    m.add_class::<SystemModel>()?;
    m.add_class::<SynthesisResult>()?;
    m.add_class::<SafetyFilter>()?;
    m.add_class::<ChanceConstraint>()?;
    m.add_class::<ChanceReport>()?;
    m.add_class::<Witness>()?;
    m.add_class::<CmaConfig>()?;
    m.add_class::<Controller>()?;
    register_synthesis(m)?;
    Ok(())
}