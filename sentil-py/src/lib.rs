//! The `_sentil` extension module.

use pyo3::prelude::*;

mod config;
mod errors;
mod formula;
mod signal;

use config::{Config, Interval, Robustness, TimeMode};
use errors::{EvaluationError, ParseError, SemanticError, SentilError};
use formula::{Expr, Formula};
use signal::{Interpolation, PreparedTrace, RingBuffer, Trace};

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
    Ok(())
}