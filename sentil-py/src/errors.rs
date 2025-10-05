//! The exception hierarchy. `SentilError` is the base so `except SentilError`
//! catches everything; the three subclasses separate the failure kinds.

use pyo3::create_exception;
use pyo3::exceptions::PyException;

create_exception!(_sentil, SentilError, PyException, "Base class for SENTIL errors.");
create_exception!(_sentil, ParseError, SentilError, "A formula failed to parse.");
create_exception!(_sentil, SemanticError, SentilError, "A well-formed formula means something invalid.");
create_exception!(_sentil, EvaluationError, SentilError, "An evaluation, data, or fit error.");