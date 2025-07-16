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

/// Extracts the settled value an immediate-output operator needs from a child.
///
/// Past-time operators and the operands of until and since must see a concrete
/// value each step. A child that still returns an interval is a future-time
/// operator nested where only present data is available; the monitor rejects
/// that composition when it is built, so this is a guard rather than a usual path.
fn extract_concrete(robustness: Robustness) -> Result<f64> {
    match robustness {
        Robustness::Concrete(v) => Ok(v),
        Robustness::Interval(..) => Err(Error::Unsupported {
            feature: "a future-time operator nested where only present data is available",
        }),
    }
}

/// `historically[a, b] phi`: the past-time mirror of `always`.
///
/// A sample only enters the window once it has aged past the lower bound `a`, so
/// it sits in a delay buffer until mature, then joins a monotonic deque that
/// holds the running minimum over the window's far edge `[t - b, t - a]`.
struct HistoricallyNode {
    child: Box<dyn Node>,
    delay: VecDeque<(f64, f64)>,
    window: MonotonicDeque,
    offset_lower: f64,
    width: f64,
}

impl Node for HistoricallyNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let value = extract_concrete(self.child.update(time, state)?)?;
        self.delay.push_back((time, value));
        let maturity = time - self.offset_lower + MATURITY_EPSILON;
        while let Some(&(t, v)) = self.delay.front() {
            if t <= maturity {
                self.window.push_min(t, v);
                self.delay.pop_front();
            } else {
                break;
            }
        }
        self.window.evict_before((time - self.width).max(0.0));
        // Positive infinity reports a property that has never been violated.
        Ok(Robustness::Concrete(
            self.window.front_value().unwrap_or(f64::INFINITY),
        ))
    }

    fn reset(&mut self) {
        self.delay.clear();
        self.window.clear();
        self.child.reset();
    }
}

/// `once[a, b] phi`: the past-time mirror of `eventually`.
struct OnceNode {
    child: Box<dyn Node>,
    delay: VecDeque<(f64, f64)>,
    window: MonotonicDeque,
    offset_lower: f64,
    width: f64,
}

impl Node for OnceNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let value = extract_concrete(self.child.update(time, state)?)?;
        self.delay.push_back((time, value));
        let maturity = time - self.offset_lower + MATURITY_EPSILON;
        while let Some(&(t, v)) = self.delay.front() {
            if t <= maturity {
                self.window.push_max(t, v);
                self.delay.pop_front();
            } else {
                break;
            }
        }
        self.window.evict_before((time - self.width).max(0.0));
        Ok(Robustness::Concrete(
            self.window.front_value().unwrap_or(f64::NEG_INFINITY),
        ))
    }

    fn reset(&mut self) {
        self.delay.clear();
        self.window.clear();
        self.child.reset();
    }
}

/// `phi since[a, b] psi`: the past-time mirror of `until`.
///
/// Each matured candidate carries the value of `psi` at its time and the running
/// minimum of `phi` from then to now; the robustness is the best such pair over
/// the window. Dominated candidates are pruned so the list stays short.
struct SinceNode {
    phi: Box<dyn Node>,
    psi: Box<dyn Node>,
    candidates: VecDeque<(f64, f64, f64)>,
    delay: VecDeque<(f64, f64, f64)>,
    offset_lower: f64,
    width: f64,
}

impl Node for SinceNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let r_phi = extract_concrete(self.phi.update(time, state)?)?;
        let r_psi = extract_concrete(self.psi.update(time, state)?)?;

        for entry in &mut self.delay {
            entry.2 = entry.2.min(r_phi);
        }
        for entry in &mut self.candidates {
            entry.2 = entry.2.min(r_phi);
        }
        self.delay.push_back((time, r_psi, f64::INFINITY));

        let maturity = time - self.offset_lower + MATURITY_EPSILON;
        while let Some(&(t, psi_val, min_phi)) = self.delay.front() {
            if t > maturity {
                break;
            }
            self.delay.pop_front();
            while let Some(&(_, last_psi, last_phi)) = self.candidates.back() {
                let score_last = last_psi.min(last_phi);
                let score_new = psi_val.min(min_phi);
                if score_last <= score_new && last_phi <= min_phi {
                    self.candidates.pop_back();
                } else {
                    break;
                }
            }
            self.candidates.push_back((t, psi_val, min_phi));
        }

        let start = (time - self.width).max(0.0);
        while let Some(&(t, _, _)) = self.candidates.front() {
            if t < start {
                self.candidates.pop_front();
            } else {
                break;
            }
        }

        let best = self
            .candidates
            .iter()
            .map(|&(_, psi, phi)| psi.min(phi))
            .fold(f64::NEG_INFINITY, f64::max);
        Ok(Robustness::Concrete(best))
    }

    fn reset(&mut self) {
        self.candidates.clear();
        self.delay.clear();
        self.phi.reset();
        self.psi.reset();
    }
}