//! The `_sentil` extension module.

use pyo3::prelude::*;

mod config;
mod errors;
mod formula;
mod monitor;
mod signal;
mod stats;

use config::{Config, Interval, Robustness, TimeMode};
use errors::{EvaluationError, ParseError, SemanticError, SentilError};
use formula::{Expr, Formula};
use monitor::{FormulaBank, Monitor, MultiMonitor, OnlineMonitor};
use pyo3::wrap_pyfunction;
use signal::{Interpolation, PreparedTrace, RingBuffer, Trace};
use stats::{
    BayesConfig, BayesResult, BayesVerdict, ConfidenceInterval, IntervalMethod, LiftingRegistry,
    NoiseInteraction, NoiseModel, RobustnessDistribution, SmcConfig, SmcResult, SprtConfig,
    SprtResult, SprtVerdict,
};

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
    register_stats(m)?;
    Ok(())
}