//! Expressions and formulas.

use crate::config::Interval as PyInterval;
use crate::errors::{pyerr, EvaluationError};
use crate::signal::Trace;
use numpy::{IntoPyArray, PyArray1};
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use sentil::formula::{BinaryOp, ComparisonOp, Expr as CoreExpr, Interval, Predicate, ProbabilityOp};
use sentil::Formula as CoreFormula;

fn to_core_expr(value: &Bound<'_, PyAny>) -> PyResult<CoreExpr> {
    if let Ok(expr) = value.extract::<PyRef<Expr>>() {
        Ok(expr.inner.clone())
    } else if let Ok(number) = value.extract::<f64>() {
        Ok(CoreExpr::Literal(number))
    } else {
        Err(PyTypeError::new_err("expected an expression or a number"))
    }
}

fn make_interval(lower: f64, upper: Option<f64>) -> PyResult<Interval> {
    Interval::new(lower, upper).map_err(pyerr)
}

/// A term in a predicate.
#[pyclass]
#[derive(Clone)]
pub struct Expr {
    pub(crate) inner: CoreExpr,
}

impl Expr {
    fn combine(&self, op: BinaryOp, other: &Bound<'_, PyAny>, reflected: bool) -> PyResult<Expr> {
        let rhs = to_core_expr(other)?;
        let (left, right) =
            if reflected { (rhs, self.inner.clone()) } else { (self.inner.clone(), rhs) };
        Ok(Expr { inner: CoreExpr::Binary(op, Box::new(left), Box::new(right)) })
    }
}

#[pymethods]
impl Expr {
    /// A reference to a trace variable.
    #[staticmethod]
    fn var(name: &str) -> Self {
        Self { inner: CoreExpr::Variable(name.to_owned()) }
    }

    /// A constant value.
    #[staticmethod]
    fn constant(value: f64) -> Self {
        Self { inner: CoreExpr::Literal(value) }
    }

    fn __add__(&self, other: &Bound<'_, PyAny>) -> PyResult<Expr> {
        self.combine(BinaryOp::Add, other, false)
    }

    fn __radd__(&self, other: &Bound<'_, PyAny>) -> PyResult<Expr> {
        self.combine(BinaryOp::Add, other, true)
    }

    fn __sub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Expr> {
        self.combine(BinaryOp::Sub, other, false)
    }

    fn __rsub__(&self, other: &Bound<'_, PyAny>) -> PyResult<Expr> {
        self.combine(BinaryOp::Sub, other, true)
    }

    fn __mul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Expr> {
        self.combine(BinaryOp::Mul, other, false)
    }

    fn __rmul__(&self, other: &Bound<'_, PyAny>) -> PyResult<Expr> {
        self.combine(BinaryOp::Mul, other, true)
    }

    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Expr> {
        self.combine(BinaryOp::Div, other, false)
    }

    fn __rtruediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<Expr> {
        self.combine(BinaryOp::Div, other, true)
    }

    fn __neg__(&self) -> Expr {
        let zero = Box::new(CoreExpr::Literal(0.0));
        Expr { inner: CoreExpr::Binary(BinaryOp::Sub, zero, Box::new(self.inner.clone())) }
    }

    fn __abs__(&self) -> Expr {
        Expr { inner: CoreExpr::Call("abs".to_owned(), vec![self.inner.clone()]) }
    }

    fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> PyResult<Formula> {
        let rhs = to_core_expr(other)?;
        let comparison = match op {
            CompareOp::Lt => ComparisonOp::Less,
            CompareOp::Le => ComparisonOp::LessEqual,
            CompareOp::Gt => ComparisonOp::Greater,
            CompareOp::Ge => ComparisonOp::GreaterEqual,
            CompareOp::Eq => ComparisonOp::Equal,
            CompareOp::Ne => ComparisonOp::NotEqual,
        };
        let predicate = Predicate { lhs: self.inner.clone(), op: comparison, rhs };
        Ok(Formula { inner: CoreFormula::Predicate(predicate) })
    }

    fn __repr__(&self) -> String {
        format!("Expr({})", self.inner)
    }
}

/// A signal temporal logic formula.
#[pyclass]
#[derive(Clone)]
pub struct Formula {
    pub(crate) inner: CoreFormula,
}

#[pymethods]
impl Formula {
    /// Parse a formula from its textual syntax.
    #[staticmethod]
    fn parse(text: &str) -> PyResult<Formula> {
        Ok(Formula { inner: CoreFormula::parse(text).map_err(pyerr)? })
    }

    #[staticmethod]
    fn from_json(text: &str) -> PyResult<Formula> {
        let inner = serde_json::from_str(text)
            .map_err(|e| EvaluationError::new_err(format!("invalid formula JSON: {e}")))?;
        Ok(Formula { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| EvaluationError::new_err(format!("could not serialize the formula: {e}")))
    }

    #[getter]
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    #[getter]
    fn is_temporal(&self) -> bool {
        self.inner.has_temporal()
    }

    #[getter]
    fn variables(&self) -> Vec<String> {
        self.inner.variables()
    }

    fn __invert__(&self) -> Formula {
        Formula { inner: CoreFormula::Not(Box::new(self.inner.clone())) }
    }

    fn __and__(&self, other: &Formula) -> Formula {
        Formula { inner: CoreFormula::And(Box::new(self.inner.clone()), Box::new(other.inner.clone())) }
    }

    fn __or__(&self, other: &Formula) -> Formula {
        Formula { inner: CoreFormula::Or(Box::new(self.inner.clone()), Box::new(other.inner.clone())) }
    }

    fn __rshift__(&self, other: &Formula) -> Formula {
        let inner =
            CoreFormula::Implies(Box::new(self.inner.clone()), Box::new(other.inner.clone()));
        Formula { inner }
    }

    /// Holds at every point of [lower, upper].
    #[pyo3(signature = (lower=0.0, upper=None))]
    fn always(&self, lower: f64, upper: Option<f64>) -> PyResult<Formula> {
        let interval = make_interval(lower, upper)?;
        Ok(Formula { inner: CoreFormula::Always(interval, Box::new(self.inner.clone())) })
    }

    /// Holds at some point of [lower, upper].
    #[pyo3(signature = (lower=0.0, upper=None))]
    fn eventually(&self, lower: f64, upper: Option<f64>) -> PyResult<Formula> {
        let interval = make_interval(lower, upper)?;
        Ok(Formula { inner: CoreFormula::Eventually(interval, Box::new(self.inner.clone())) })
    }

    /// Held at every past point of [lower, upper].
    #[pyo3(signature = (lower=0.0, upper=None))]
    fn historically(&self, lower: f64, upper: Option<f64>) -> PyResult<Formula> {
        let interval = make_interval(lower, upper)?;
        Ok(Formula { inner: CoreFormula::Historically(interval, Box::new(self.inner.clone())) })
    }

    /// Held at some past point of [lower, upper].
    #[pyo3(signature = (lower=0.0, upper=None))]
    fn once(&self, lower: f64, upper: Option<f64>) -> PyResult<Formula> {
        let interval = make_interval(lower, upper)?;
        Ok(Formula { inner: CoreFormula::Once(interval, Box::new(self.inner.clone())) })
    }

    fn next(&self) -> Formula {
        Formula { inner: CoreFormula::Next(Box::new(self.inner.clone())) }
    }

    #[pyo3(signature = (other, lower=0.0, upper=None))]
    fn until(&self, other: &Formula, lower: f64, upper: Option<f64>) -> PyResult<Formula> {
        let interval = make_interval(lower, upper)?;
        let inner = CoreFormula::Until(
            interval,
            Box::new(self.inner.clone()),
            Box::new(other.inner.clone()),
        );
        Ok(Formula { inner })
    }

    #[pyo3(signature = (other, lower=0.0, upper=None))]
    fn since(&self, other: &Formula, lower: f64, upper: Option<f64>) -> PyResult<Formula> {
        let interval = make_interval(lower, upper)?;
        let inner = CoreFormula::Since(
            interval,
            Box::new(self.inner.clone()),
            Box::new(other.inner.clone()),
        );
        Ok(Formula { inner })
    }

    /// Wrap the formula in a probabilistic operator.
    #[pyo3(signature = (threshold, op=">="))]
    fn probability(&self, threshold: f64, op: &str) -> PyResult<Formula> {
        if !(0.0..=1.0).contains(&threshold) {
            return Err(EvaluationError::new_err(format!(
                "probability threshold {threshold} is outside [0, 1]"
            )));
        }
        let op = match op {
            ">=" => ProbabilityOp::GreaterEqual,
            ">" => ProbabilityOp::Greater,
            "<=" => ProbabilityOp::LessEqual,
            "<" => ProbabilityOp::Less,
            other => {
                return Err(EvaluationError::new_err(format!(
                    "probability comparison must be one of >=, >, <=, <, got '{other}'"
                )))
            }
        };
        Ok(Formula { inner: CoreFormula::Probabilistic(op, threshold, Box::new(self.inner.clone())) })
    }

    /// Robustness of the trace under discrete-time semantics.
    fn robustness(&self, trace: &Trace) -> PyResult<f64> {
        self.inner.robustness(&trace.inner).map_err(pyerr)
    }

    /// Robustness under dense-time semantics.
    fn robustness_dense(&self, trace: &Trace) -> PyResult<f64> {
        self.inner.robustness_dense(&trace.inner).map_err(pyerr)
    }

    /// Robustness at every sample.
    fn robustness_signal<'py>(
        &self,
        py: Python<'py>,
        trace: &Trace,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let values = self.inner.robustness_signal(&trace.inner).map_err(pyerr)?;
        Ok(values.into_pyarray(py))
    }

    fn robustness_dense_signal<'py>(
        &self,
        py: Python<'py>,
        trace: &Trace,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let values = self.inner.robustness_dense_signal(&trace.inner).map_err(pyerr)?;
        Ok(values.into_pyarray(py))
    }

    /// The time spans where the formula does not hold.
    fn violations(&self, trace: &Trace) -> PyResult<Vec<PyInterval>> {
        let spans = self.inner.violations(&trace.inner).map_err(pyerr)?;
        Ok(spans.into_iter().map(|(start, end)| PyInterval { start, end }).collect())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("Formula.parse({:?})", self.inner.to_string())
    }
}