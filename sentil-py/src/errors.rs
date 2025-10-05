//! Exception types.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use sentil::Error;

create_exception!(_sentil, SentilError, PyException, "Base class for SENTIL errors.");
create_exception!(_sentil, ParseError, SentilError, "A formula failed to parse.");
create_exception!(_sentil, SemanticError, SentilError, "A well-formed formula means something invalid.");
create_exception!(_sentil, EvaluationError, SentilError, "An evaluation, data, or fit error.");

pub(crate) fn pyerr(e: Error) -> PyErr {
    let message = e.to_string();
    match e {
        Error::Parse(_) => ParseError::new_err(message),
        Error::UnknownVariable { .. }
        | Error::NotProbabilistic
        | Error::Unsupported { .. } => SemanticError::new_err(message),
        _ => EvaluationError::new_err(message),
    }
}