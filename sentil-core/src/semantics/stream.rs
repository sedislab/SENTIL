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

/// `always[a, b] phi`, settling the verdict for the time `b` behind the present.
struct FutureAlwaysNode {
    child: Box<dyn Node>,
    buffer: VecDeque<(f64, f64)>,
    window: MonotonicDeque,
    offset_start: f64,
    offset_end: f64,
    bounded: bool,
    first_time: Option<f64>,
    global_min: f64,
}

impl Node for FutureAlwaysNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let child = self.child.update(time, state)?;
        let concrete = matches!(child, Robustness::Concrete(_));
        let (lo, hi) = bounds(child);
        let first = *self.first_time.get_or_insert(time);

        self.buffer.push_back((time, lo));
        self.window.push_min(time, lo);
        self.global_min = self.global_min.min(hi);

        if !self.bounded {
            return Ok(Robustness::Interval(f64::NEG_INFINITY, self.global_min));
        }

        let query_time = time - self.offset_end;
        if query_time < first {
            let partial = self.window.front_value().unwrap_or(f64::INFINITY);
            return Ok(Robustness::Interval(f64::NEG_INFINITY, partial));
        }

        let window_start = query_time + self.offset_start;
        self.window.evict_before(window_start);
        drop_front_before(&mut self.buffer, window_start);
        let min_rob = self.window.front_value().unwrap_or(f64::INFINITY);

        if concrete {
            Ok(Robustness::Concrete(min_rob))
        } else {
            Ok(Robustness::Interval(min_rob, self.global_min))
        }
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.window.clear();
        self.first_time = None;
        self.global_min = f64::INFINITY;
        self.child.reset();
    }
}

/// `eventually[a, b] phi`: the dual of `always`, settling on a supremum.
struct FutureEventuallyNode {
    child: Box<dyn Node>,
    buffer: VecDeque<(f64, f64)>,
    window: MonotonicDeque,
    offset_start: f64,
    offset_end: f64,
    bounded: bool,
    first_time: Option<f64>,
    global_max: f64,
}

impl Node for FutureEventuallyNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let child = self.child.update(time, state)?;
        let concrete = matches!(child, Robustness::Concrete(_));
        let (lo, hi) = bounds(child);
        let first = *self.first_time.get_or_insert(time);

        self.buffer.push_back((time, hi));
        self.window.push_max(time, hi);
        self.global_max = self.global_max.max(lo);

        if !self.bounded {
            return Ok(Robustness::Interval(self.global_max, f64::INFINITY));
        }

        let query_time = time - self.offset_end;
        if query_time < first {
            let partial = self.window.front_value().unwrap_or(f64::NEG_INFINITY);
            return Ok(Robustness::Interval(partial, f64::INFINITY));
        }

        let window_start = query_time + self.offset_start;
        self.window.evict_before(window_start);
        drop_front_before(&mut self.buffer, window_start);
        let max_rob = self.window.front_value().unwrap_or(f64::NEG_INFINITY);

        if concrete {
            Ok(Robustness::Concrete(max_rob))
        } else {
            Ok(Robustness::Interval(self.global_max, max_rob))
        }
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.window.clear();
        self.first_time = None;
        self.global_max = f64::NEG_INFINITY;
        self.child.reset();
    }
}

/// `phi until[a, b] psi`: psi must hold somewhere in the future window with phi
/// holding until then. Unbounded until folds with a constant-space recurrence;
/// the bounded case keeps a short buffer and resolves with a delay of `b`.
struct UntilNode {
    phi: Box<dyn Node>,
    psi: Box<dyn Node>,
    buffer: VecDeque<(f64, f64, f64)>,
    offset_start: f64,
    offset_end: f64,
    bounded: bool,
    first_time: Option<f64>,
    unbounded_dp: f64,
}

impl Node for UntilNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let r_phi = extract_concrete(self.phi.update(time, state)?)?;
        let r_psi = extract_concrete(self.psi.update(time, state)?)?;
        let first = *self.first_time.get_or_insert(time);
        self.buffer.push_back((time, r_phi, r_psi));

        if !self.bounded {
            // Maler-Nickovic recurrence.
            self.unbounded_dp = r_psi.max(r_phi.min(self.unbounded_dp));
            return Ok(Robustness::Interval(self.unbounded_dp, f64::INFINITY));
        }

        let query_time = time - self.offset_end;
        if query_time < first {
            return Ok(Robustness::Interval(
                self.until_over(first, time),
                f64::INFINITY,
            ));
        }

        while let Some(&(t, _, _)) = self.buffer.front() {
            if t < query_time {
                self.buffer.pop_front();
            } else {
                break;
            }
        }
        let start = query_time + self.offset_start;
        let end = query_time + self.offset_end;
        Ok(Robustness::Concrete(self.until_over(start, end)))
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.first_time = None;
        self.unbounded_dp = f64::NEG_INFINITY;
        self.phi.reset();
        self.psi.reset();
    }
}

impl UntilNode {
    /// The until robustness over witness times in `[start, end]`: the best over
    /// those times of `min(psi, inf of phi from the query time up to but not
    /// including the witness)`.
    fn until_over(&self, start: f64, end: f64) -> f64 {
        let mut best = f64::NEG_INFINITY;
        let mut min_phi = f64::INFINITY;
        for &(t, phi, psi) in &self.buffer {
            if t < start {
                continue;
            }
            if t > end {
                break;
            }
            best = best.max(psi.min(min_phi));
            min_phi = min_phi.min(phi);
        }
        best
    }
}

/// `next phi`, reporting the child's value one step behind.
struct NextNode {
    child: Box<dyn Node>,
    initialized: bool,
}

impl Node for NextNode {
    fn update(&mut self, time: f64, state: &[f64]) -> Result<Robustness> {
        let child = extract_concrete(self.child.update(time, state)?)?;
        if self.initialized {
            Ok(Robustness::Concrete(child))
        } else {
            self.initialized = true;
            Ok(Robustness::Interval(f64::NEG_INFINITY, f64::INFINITY))
        }
    }

    fn reset(&mut self) {
        self.initialized = false;
        self.child.reset();
    }
}

fn bounds(robustness: Robustness) -> (f64, f64) {
    (robustness.lower(), robustness.upper())
}

/// Drops buffered `(time, _)` entries whose time is strictly before `limit`.
fn drop_front_before(buffer: &mut VecDeque<(f64, f64)>, limit: f64) {
    while let Some(&(t, _)) = buffer.front() {
        if t < limit {
            buffer.pop_front();
        } else {
            break;
        }
    }
}

/// An online monitor that evaluates a formula incrementally.
pub struct StreamMonitor {
    root: Box<dyn Node>,
    symbols: Arc<SymbolTable>,
    buffer: Vec<f64>,
}

impl StreamMonitor {
    /// Builds a monitor for a formula given as text.
    ///
    /// ```
    /// use sentil::StreamMonitor;
    ///
    /// let mut monitor = StreamMonitor::new("x > 0 and y < 10")?;
    /// let rho = monitor.update(0.0, &[("x", 5.0), ("y", 2.0)])?;
    /// assert_eq!(rho.value(), 5.0);
    /// # Ok::<(), sentil::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a parse error for malformed input, and [`Error::Unsupported`] for an
    /// operator this monitor does not handle.
    pub fn new(formula: &str) -> Result<Self> {
        Self::from_formula(&Formula::parse(formula)?)
    }

    /// Builds a monitor from an already-parsed formula.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unsupported`] for an operator this monitor does not handle.
    pub fn from_formula(formula: &Formula) -> Result<Self> {
        validate_streaming(formula)?;
        let symbols = Arc::new(SymbolTable::from_formula(formula));
        let root = build_node(formula, &symbols)?;
        let buffer = vec![0.0; symbols.len()];
        Ok(Self {
            root,
            symbols,
            buffer,
        })
    }

    /// Folds in one timestep, given the current value of each variable, and
    /// returns the robustness so far.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownVariable`] if a variable the formula needs has no
    /// value in `values`, [`Error::NonFiniteSample`] if `time` is not finite, or
    /// [`Error::NonMonotonicTime`] if it does not follow the previous step.
    pub fn update(&mut self, time: f64, values: &[(&str, f64)]) -> Result<Robustness> {
        for (idx, name) in self.symbols.names.iter().enumerate() {
            match values.iter().find(|(n, _)| n == name) {
                Some((_, v)) => self.buffer[idx] = *v,
                None => return Err(Error::UnknownVariable { name: name.clone() }),
            }
        }
        // Detach the buffer borrow from `self` so the node can borrow it shared.
        let mut buffer = std::mem::take(&mut self.buffer);
        let result = self.root.update(time, &buffer);
        std::mem::swap(&mut self.buffer, &mut buffer);
        result
    }

    /// Folds in one timestep from values packed by [`StreamMonitor::symbol_index`]
    /// order.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownVariable`] if `values` is shorter than the number
    /// of variables the formula needs.
    pub fn update_dense(&mut self, time: f64, values: &[f64]) -> Result<Robustness> {
        if values.len() < self.symbols.len() {
            return Err(Error::UnknownVariable {
                name: self.symbols.names[values.len()].clone(),
            });
        }
        self.root.update(time, values)
    }

    /// The packed-slice index of a variable, for use with
    /// [`StreamMonitor::update_dense`].
    pub fn symbol_index(&self, name: &str) -> Option<usize> {
        self.symbols.index(name)
    }

    /// The number of variables the formula references.
    pub fn variable_count(&self) -> usize {
        self.symbols.len()
    }

    /// Clears all state.
    pub fn reset(&mut self) {
        self.root.reset();
    }
}

/// Whether a formula carries any temporal or probabilistic operator.
fn is_temporal(formula: &Formula) -> bool {
    match formula {
        Formula::Predicate(_) => false,
        Formula::Not(f) => is_temporal(f),
        Formula::And(l, r) | Formula::Or(l, r) | Formula::Implies(l, r) => {
            is_temporal(l) || is_temporal(r)
        }
        _ => true,
    }
}

/// Builds a node for a maximal non-temporal subformula.
fn atomic_node(formula: &Formula, names: &[String]) -> Result<Box<dyn Node>> {
    let program = Program::compile(formula, names)?;
    let scratch = program.scratch();
    Ok(Box::new(AtomicNode { program, scratch }))
}

fn offsets(interval: crate::formula::Interval) -> (f64, f64, bool) {
    match interval.upper {
        Some(b) => (interval.lower, b, true),
        None => (interval.lower, 0.0, false),
    }
}