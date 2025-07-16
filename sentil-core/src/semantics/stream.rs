//! Online monitoring: monitoring of a formula one timed sample at a time and
//! read its robustness.
//!
//! The monitor compiles the formula into a tree of stateful nodes once, then
//! processes each update by walking the tree. Boolean and atomic structure
//! resolves immediately; the temporal nodes added later keep their own bounded
//! state so the per-sample cost stays flat. Signal values are addressed by a
//! small symbol table, so the hot path reads them from a packed slice.

use std::collections::VecDeque;
use std::sync::Arc;

use super::robustness::Robustness;
use super::window::MonotonicDeque;
use crate::error::{Error, Result};
use crate::expr::Program;
use crate::formula::Formula;

/// The lower-bound delay added when deciding whether a buffered sample has
/// matured into a past-time window, absorbing floating-point rounding.
const MATURITY_EPSILON: f64 = 1e-9;

/// Dense indices for a formula's variables, in sorted order.
#[derive(Debug)]
struct SymbolTable {
    names: Vec<String>,
}

impl SymbolTable {
    fn from_formula(formula: &Formula) -> Self {
        Self {
            names: formula.variables(),
        }
    }

    fn index(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    fn len(&self) -> usize {
        self.names.len()
    }
}

/// One stateful node in the compiled monitor.
trait Node {
    /// Folds in the sample at `time`, whose values are packed in `state` by
    /// symbol index.
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness>;
    fn reset(&mut self);
}

/// A maximal non-temporal subformula, compiled to a flat program.
struct AtomicNode {
    program: Program,
    scratch: Vec<f64>,
}

impl Node for AtomicNode {
    fn update(&mut self, _time: f64, state: &[f64]) -> Result<Robustness> {
        Ok(Robustness::Concrete(
            self.program.eval(&mut self.scratch, state)?,
        ))
    }

    fn reset(&mut self) {}
}

struct NotNode {
    child: Box<dyn Node>,
}

impl Node for NotNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        Ok(self.child.update(time, state)?.negate())
    }

    fn reset(&mut self) {
        self.child.reset();
    }
}

/// Shared shape for the binary boolean operators, differing only in how they
/// combine the two child robustness values.
struct BinaryNode {
    left: Box<dyn Node>,
    right: Box<dyn Node>,
    combine: fn(Robustness, Robustness) -> Robustness,
}

impl Node for BinaryNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let l = self.left.update(time, state)?;
        let r = self.right.update(time, state)?;
        Ok((self.combine)(l, r))
    }

    fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
    }
}