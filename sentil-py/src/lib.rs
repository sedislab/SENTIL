//! The `_sentil` extension module.

use pyo3::prelude::*;

mod errors;

use errors::{EvaluationError, ParseError, SemanticError, SentilError};

#[pymodule]
fn _sentil(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("SentilError", m.py().get_type::<SentilError>())?;
    m.add("ParseError", m.py().get_type::<ParseError>())?;
    m.add("SemanticError", m.py().get_type::<SemanticError>())?;
    m.add("EvaluationError", m.py().get_type::<EvaluationError>())?;
    Ok(())
}